//! RDMA transport layer for remote page fetching (M2)
//!
//! Production: Use rdma-core (ibverbs) with RC queue pairs
//! Development fallback: TCP sockets for testing without RDMA hardware
//!
//! Target latency: median <100µs, p99 <500µs (NFR-latency)

use anyhow::{Context, Result, anyhow};
use log::{info, debug, warn};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

const PAGE_SIZE: usize = 4096;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("RDMA operation failed: {0}")]
    RdmaFailed(String),
    
    #[error("Timeout waiting for response")]
    Timeout,
}

/// Node address for cluster communication
#[derive(Debug, Clone)]
pub struct NodeAddr {
    pub node_id: u32,
    pub ip: String,
    pub port: u16,
}

/// RDMA connection to a remote node (stub for M2)
pub struct RdmaConnection {
    remote_node: u32,
    // TODO: Add ibverbs structs:
    // - ibv_context
    // - ibv_pd (protection domain)
    // - ibv_qp (RC queue pair)
    // - ibv_mr (memory region)
    connected: bool,
}

impl RdmaConnection {
    /// Establish RDMA connection to remote node
    pub fn connect(node_id: u32, addr: &str) -> Result<Self> {
        info!("Establishing RDMA connection to node {} at {}", node_id, addr);
        
        // TODO M2: Implement real RDMA connection setup:
        // 1. ibv_open_device() - open RDMA device
        // 2. ibv_alloc_pd() - allocate protection domain
        // 3. ibv_create_cq() - create completion queues
        // 4. ibv_create_qp() - create RC queue pair
        // 5. Exchange QP info with remote (QPN, GID, LID)
        // 6. ibv_modify_qp() - transition to RTS state
        
        warn!("RDMA not yet implemented, using fallback");
        
        Ok(Self {
            remote_node: node_id,
            connected: false,
        })
    }

    /// Perform RDMA READ to fetch remote page
    pub fn rdma_read(&self, remote_addr: u64, buf: &mut [u8]) -> Result<()> {
        debug!("RDMA READ: remote_addr=0x{:x}, len={}", remote_addr, buf.len());
        
        // TODO M2: Implement RDMA READ:
        // 1. Post work request: ibv_post_send() with IBV_WR_RDMA_READ
        // 2. Poll completion queue: ibv_poll_cq()
        // 3. Check for errors in work completion
        
        // Fallback: return zeros
        buf.fill(0);
        
        Ok(())
    }

    /// Perform RDMA WRITE to send page to remote
    pub fn rdma_write(&self, remote_addr: u64, buf: &[u8]) -> Result<()> {
        debug!("RDMA WRITE: remote_addr=0x{:x}, len={}", remote_addr, buf.len());
        
        // TODO M2: Implement RDMA WRITE
        
        Ok(())
    }
}

/// Cluster-wide RDMA transport manager
pub struct TransportManager {
    local_node: u32,
    connections: Arc<RwLock<HashMap<u32, RdmaConnection>>>,
}

impl TransportManager {
    pub fn new(local_node: u32) -> Self {
        Self {
            local_node,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to a remote node
    pub fn connect_node(&self, node_id: u32, addr: &str) -> Result<()> {
        let conn = RdmaConnection::connect(node_id, addr)?;
        self.connections.write().insert(node_id, conn);
        info!("Connected to node {}", node_id);
        Ok(())
    }

    /// Fetch page from remote node
    pub fn fetch_page(&self, node_id: u32, gpa: u64) -> Result<Vec<u8>> {
        let connections = self.connections.read();
        let conn = connections.get(&node_id)
            .ok_or_else(|| anyhow!("No connection to node {}", node_id))?;

        let mut page_buf = vec![0u8; PAGE_SIZE];
        conn.rdma_read(gpa, &mut page_buf)?;

        Ok(page_buf)
    }

    /// Send page to remote node
    pub fn send_page(&self, node_id: u32, gpa: u64, data: &[u8]) -> Result<()> {
        let connections = self.connections.read();
        let conn = connections.get(&node_id)
            .ok_or_else(|| anyhow!("No connection to node {}", node_id))?;

        conn.rdma_write(gpa, data)?;

        Ok(())
    }
}

// Global transport manager (initialized by coordinator)
static mut TRANSPORT: Option<Arc<TransportManager>> = None;

/// Initialize global transport manager
pub fn init_transport(local_node: u32) -> Result<()> {
    unsafe {
        TRANSPORT = Some(Arc::new(TransportManager::new(local_node)));
    }
    Ok(())
}

/// Fetch page from remote node (called by pager)
pub fn fetch_page(node_id: u32, gpa: u64) -> Result<Vec<u8>> {
    unsafe {
        TRANSPORT.as_ref()
            .ok_or_else(|| anyhow!("Transport not initialized"))?
            .fetch_page(node_id, gpa)
    }
}

/// Send page to remote node
pub fn send_page(node_id: u32, gpa: u64, data: &[u8]) -> Result<()> {
    unsafe {
        TRANSPORT.as_ref()
            .ok_or_else(|| anyhow!("Transport not initialized"))?
            .send_page(node_id, gpa, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_init() {
        init_transport(0).unwrap();
        // Basic smoke test
    }
}

