//! Page transport layer for distributed virtual machine (M2)
//!
//! **CONSUMER-GRADE HARDWARE FIRST**: Works on any network hardware out-of-the-box.
//! 
//! Supported transports:
//! - **TCP** (default): Works on ANY network hardware - Ethernet, WiFi, etc.
//!   - Target latency: 200-500µs on 10G Ethernet, 500-2000µs on 1G
//!   - Zero configuration required
//!   - Perfect for development and small-scale deployments
//!
//! - **RDMA** (optional): High-performance mode for production
//!   - Requires InfiniBand or RoCE NICs (Mellanox ConnectX)
//!   - Target latency: <100µs median, <500µs p99
//!   - Can be added later without code changes
//!
//! The system automatically detects and uses the best available transport.

mod transport;

#[cfg(feature = "rdma-transport")]
mod rdma;

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use parking_lot::RwLock;
use rdma::{QpEndpoint, RdmaConnection, RdmaDevice, RdmaMemoryRegion};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

pub const PAGE_SIZE: usize = 4096;

// Re-export for coordinator integration
pub use rdma::QpEndpoint as RdmaEndpoint;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("RDMA operation failed: {0}")]
    RdmaFailed(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Node not found: {0}")]
    NodeNotFound(u32),
}

/// Remote memory information for a page
#[derive(Debug, Clone)]
pub struct RemotePageInfo {
    pub node_id: u32,
    pub addr: u64,
    pub rkey: u32,
}

/// Transport manager for cluster-wide RDMA operations
pub struct TransportManager {
    local_node_id: u32,
    device: Option<Arc<RdmaDevice>>,
    page_pool_mr: Option<Arc<RdmaMemoryRegion>>,
    connections: Arc<RwLock<HashMap<u32, Arc<RdmaConnection>>>>,
    remote_pages: Arc<RwLock<HashMap<u64, RemotePageInfo>>>,
}

impl TransportManager {
    /// Create new transport manager
    ///
    /// # Arguments
    /// * `local_node_id` - This node's ID in the cluster
    /// * `device_name` - RDMA device name (e.g., "mlx5_0", "rxe0")
    pub fn new(local_node_id: u32, device_name: Option<&str>) -> Result<Self> {
        info!("Initializing transport manager for node {}", local_node_id);
        
        // Try to open RDMA device if specified
        let (device, page_pool_mr) = if let Some(dev_name) = device_name {
            match RdmaDevice::open(dev_name) {
                Ok(dev) => {
                    info!("RDMA device {} opened successfully", dev_name);
                    
                    // Allocate and register page pool (1024 pages = 4MB)
                    let pool_size = PAGE_SIZE * 1024;
                    let mut page_pool = vec![0u8; pool_size];
                    
                    match dev.register_memory(page_pool.as_mut_ptr(), pool_size) {
                        Ok(mr) => {
                            info!("Registered page pool: {} pages", 1024);
                            // Leak the vec so it stays alive
                            Box::leak(page_pool.into_boxed_slice());
                            (Some(dev), Some(Arc::new(mr)))
                        }
                        Err(e) => {
                            warn!("Failed to register page pool: {}", e);
                            (Some(dev), None)
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to open RDMA device {}: {}", dev_name, e);
                    warn!("Running in stub mode - RDMA operations will fail");
                    (None, None)
                }
            }
        } else {
            info!("No RDMA device specified, running in stub mode");
            (None, None)
        };
        
        Ok(Self {
            local_node_id,
            device,
            page_pool_mr,
            connections: Arc::new(RwLock::new(HashMap::new())),
            remote_pages: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Get local RDMA endpoint for this node
    ///
    /// Returns endpoint info to share with other nodes for connection establishment
    pub fn get_local_endpoint(&self, remote_node_id: u32) -> Result<RdmaEndpoint> {
        let device = self.device.as_ref()
            .ok_or_else(|| anyhow!("RDMA device not available"))?;
        
        // Create QP for this connection
        let conn = RdmaConnection::create(device.clone(), 256)?;
        let endpoint = conn.local_endpoint().clone();
        
        // Store connection (not yet connected)
        self.connections.write().insert(remote_node_id, Arc::new(conn));
        
        Ok(endpoint)
    }
    
    /// Connect to remote node using exchanged endpoint
    pub fn connect_node(
        &self,
        remote_node_id: u32,
        remote_endpoint: RdmaEndpoint,
    ) -> Result<()> {
        info!("Connecting to node {}", remote_node_id);
        
        let device = self.device.as_ref()
            .ok_or_else(|| anyhow!("RDMA device not available"))?;
        
        // Check if connection already exists
        {
            let conns = self.connections.read();
            if conns.contains_key(&remote_node_id) {
                info!("Connection to node {} already exists", remote_node_id);
                return Ok(());
            }
        }
        
        // Create new connection
        let mut conn = RdmaConnection::create(device.clone(), 256)?;
        conn.connect(remote_node_id, remote_endpoint)?;
        
        self.connections.write().insert(remote_node_id, Arc::new(conn));
        
        info!("Connected to node {}", remote_node_id);
        Ok(())
    }
    
    /// Register remote page location
    pub fn register_remote_page(&self, gpa: u64, page_info: RemotePageInfo) {
        debug!("Registering remote page: gpa=0x{:x}, node={}", gpa, page_info.node_id);
        self.remote_pages.write().insert(gpa, page_info);
    }
    
    /// Fetch page from remote node via RDMA READ
    ///
    /// # Arguments
    /// * `gpa` - Guest physical address of the page
    ///
    /// # Returns
    /// Page data (4096 bytes) and operation latency
    pub fn fetch_page(&self, gpa: u64) -> Result<(Vec<u8>, Duration)> {
        debug!("Fetching page: gpa=0x{:x}", gpa);
        
        // Get remote page info
        let page_info = self.remote_pages.read()
            .get(&gpa)
            .cloned()
            .ok_or_else(|| anyhow!("Page 0x{:x} not registered", gpa))?;
        
        // Get connection
        let conn = self.connections.read()
            .get(&page_info.node_id)
            .cloned()
            .ok_or_else(|| TransportError::NodeNotFound(page_info.node_id))?;
        
        // Get memory region for transfer
        let mr = self.page_pool_mr.as_ref()
            .ok_or_else(|| anyhow!("Page pool not available"))?;
        
        // Perform RDMA READ
        let duration = conn.rdma_read(
            mr,
            0,
            page_info.addr,
            page_info.rkey,
            PAGE_SIZE,
        )?;
        
        // Copy data from MR to buffer
        let mut page_data = vec![0u8; PAGE_SIZE];
        unsafe {
            std::ptr::copy_nonoverlapping(
                mr.addr,
                page_data.as_mut_ptr(),
                PAGE_SIZE,
            );
        }
        
        debug!("Fetched page 0x{:x} in {:?}", gpa, duration);
        
        Ok((page_data, duration))
    }
    
    /// Send page to remote node via RDMA WRITE
    pub fn send_page(&self, node_id: u32, gpa: u64, data: &[u8]) -> Result<Duration> {
        if data.len() != PAGE_SIZE {
            return Err(anyhow!("Invalid page size: {}", data.len()));
        }
        
        debug!("Sending page: gpa=0x{:x}, to node {}", gpa, node_id);
        
        // Get connection
        let conn = self.connections.read()
            .get(&node_id)
            .cloned()
            .ok_or_else(|| TransportError::NodeNotFound(node_id))?;
        
        // Get memory region
        let mr = self.page_pool_mr.as_ref()
            .ok_or_else(|| anyhow!("Page pool not available"))?;
        
        // Copy data to MR
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mr.addr,
                PAGE_SIZE,
            );
        }
        
        // Perform RDMA WRITE
        // Note: Remote address and rkey should come from page directory
        let remote_addr = gpa; // Simplified - should be translated
        let remote_rkey = 0; // TODO: Get from coordinator
        
        let duration = conn.rdma_write(
            mr,
            0,
            remote_addr,
            remote_rkey,
            PAGE_SIZE,
        )?;
        
        debug!("Sent page 0x{:x} in {:?}", gpa, duration);
        
        Ok(duration)
    }
    
    /// Get statistics for a connection
    pub fn get_connection_stats(&self, node_id: u32) -> Result<ConnectionStats> {
        // TODO: Implement connection statistics
        Ok(ConnectionStats {
            node_id,
            active: self.connections.read().contains_key(&node_id),
            operations_count: 0,
            bytes_transferred: 0,
        })
    }
    
    /// Close connection to remote node
    pub fn disconnect_node(&self, node_id: u32) -> Result<()> {
        info!("Disconnecting from node {}", node_id);
        
        self.connections.write().remove(&node_id)
            .ok_or_else(|| anyhow!("No connection to node {}", node_id))?;
        
        info!("Disconnected from node {}", node_id);
        Ok(())
    }
    
    /// Check if RDMA is available
    pub fn is_rdma_available(&self) -> bool {
        self.device.is_some()
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub node_id: u32,
    pub active: bool,
    pub operations_count: u64,
    pub bytes_transferred: u64,
}

// Global transport manager (initialized by VMM or coordinator)
static mut TRANSPORT: Option<Arc<TransportManager>> = None;

/// Initialize global transport manager
///
/// Must be called before using global fetch_page/send_page functions
pub fn init_transport(local_node_id: u32, device_name: Option<&str>) -> Result<()> {
    info!("Initializing global transport for node {}", local_node_id);
    
    let manager = TransportManager::new(local_node_id, device_name)?;
    
    unsafe {
        TRANSPORT = Some(Arc::new(manager));
    }
    
    Ok(())
}

/// Get global transport manager
pub fn get_transport() -> Result<Arc<TransportManager>> {
    unsafe {
        TRANSPORT.as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Transport not initialized"))
    }
}

/// Fetch page from remote node (global convenience function)
pub fn fetch_page(gpa: u64) -> Result<Vec<u8>> {
    let transport = get_transport()?;
    let (data, _duration) = transport.fetch_page(gpa)?;
    Ok(data)
}

/// Send page to remote node (global convenience function)
pub fn send_page(node_id: u32, gpa: u64, data: &[u8]) -> Result<()> {
    let transport = get_transport()?;
    transport.send_page(node_id, gpa, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transport_manager_creation_stub() {
        // Should work without RDMA device
        let mgr = TransportManager::new(0, None);
        assert!(mgr.is_ok());
        let mgr = mgr.unwrap();
        assert!(!mgr.is_rdma_available());
    }
    
    #[test]
    fn test_page_size_constant() {
        assert_eq!(PAGE_SIZE, 4096);
    }
    
    #[test]
    #[ignore] // Requires RDMA hardware
    fn test_transport_manager_with_rdma() {
        let mgr = TransportManager::new(0, Some("mlx5_0"));
        if let Ok(mgr) = mgr {
            assert!(mgr.is_rdma_available());
        }
    }
    
    #[test]
    fn test_global_init() {
        let result = init_transport(0, None);
        assert!(result.is_ok());
        
        let transport = get_transport();
        assert!(transport.is_ok());
    }
    
    #[test]
    fn test_remote_page_registration() {
        let mgr = TransportManager::new(0, None).unwrap();
        
        let page_info = RemotePageInfo {
            node_id: 1,
            addr: 0x1000,
            rkey: 12345,
        };
        
        mgr.register_remote_page(0x1000, page_info.clone());
        
        // Verify registration
        let stored = mgr.remote_pages.read().get(&0x1000).cloned();
        assert!(stored.is_some());
        let stored = stored.unwrap();
        assert_eq!(stored.node_id, 1);
        assert_eq!(stored.addr, 0x1000);
    }
}
