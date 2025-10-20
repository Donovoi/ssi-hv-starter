//! Page transport layer for distributed virtual machine (M2)
//!
//! **CONSUMER-GRADE HARDWARE FIRST**: Works on any network hardware out-of-the-box.
//!
//! ## Quick Start (Zero Configuration)
//!
//! ```rust,no_run
//! use rdma_transport::{TransportManager, TransportEndpoint};
//!
//! // Works on ANY hardware - automatically selects best available transport
//! let mut transport = TransportManager::new(1).expect("Failed to create");
//!
//! // Connect to peer
//! let endpoint = TransportEndpoint::Tcp {
//!     addr: "192.168.1.100".to_string(),
//!     port: 50051,
//! };
//! transport.connect_peer(2, endpoint).expect("Failed to connect");
//!
//! // Fetch page (works the same whether TCP or RDMA)
//! let page_data = transport.fetch_page(0x1000, 2).expect("Failed to fetch");
//! ```
//!
//! ## Supported Transports
//!
//! - **TCP** (default): Works on ANY network hardware - Ethernet, WiFi, etc.
//!   - Latency: 200-500µs (10G), 500-2000µs (1G)
//!   - Zero configuration required
//!   - Perfect for development and small deployments
//!
//! - **RDMA** (optional): High-performance mode  
//!   - Requires InfiniBand or RoCE NICs
//!   - Latency: <100µs median, <500µs p99
//!   - Enable with `--features rdma-transport`
//!
//! The system automatically uses the best available transport.

pub mod transport;

#[cfg(feature = "rdma-transport")]
mod rdma;

use anyhow::{anyhow, Result};
use log::{info, warn};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use transport::{PageTransport, TransportEndpoint};

pub const PAGE_SIZE: usize = 4096;

// Re-exports
pub use transport::{TransportEndpoint as Endpoint, TransportTier};

#[cfg(feature = "rdma-transport")]
pub use rdma::QpEndpoint as RdmaEndpoint;

/// Transport manager - unified API for all transport types
pub struct TransportManager {
    local_node_id: u32,
    transport: Box<dyn PageTransport>,
    peer_endpoints: Arc<RwLock<HashMap<u32, TransportEndpoint>>>,
}

impl TransportManager {
    /// Create transport manager with auto-detected best transport
    ///
    /// Tries RDMA first (if compiled in), falls back to TCP.
    /// **Always works** - no special hardware required.
    pub fn new(local_node_id: u32) -> Result<Self> {
        info!("🚀 Initializing transport for node {}", local_node_id);
        info!("💡 Consumer-grade hardware support enabled (plug-and-play)");

        let transport = transport::create_transport(local_node_id)?;
        let tier = transport.performance_tier();

        info!("📊 Network tier: {}", tier);
        info!(
            "⏱️  Expected latency: ~{}µs",
            tier.expected_latency().as_micros()
        );

        if matches!(tier, TransportTier::Basic) {
            warn!("⚠️  1 Gbps network detected - consider upgrading to 10G for better performance");
        }

        Ok(Self {
            local_node_id,
            transport,
            peer_endpoints: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get local endpoint to share with peers
    pub fn local_endpoint(&self) -> TransportEndpoint {
        self.transport.local_endpoint()
    }

    /// Connect to a peer node
    ///
    /// Endpoint can be:
    /// - TCP: "192.168.1.100:50051" or TransportEndpoint::Tcp
    /// - RDMA: QP endpoint info as TransportEndpoint::Rdma
    pub fn connect_peer(&mut self, remote_node_id: u32, endpoint: TransportEndpoint) -> Result<()> {
        info!("🔗 Connecting to node {}", remote_node_id);

        self.transport.connect(remote_node_id, endpoint.clone())?;
        self.peer_endpoints.write().insert(remote_node_id, endpoint);

        // Measure latency
        if let Ok(latency) = self.transport.measure_latency(remote_node_id) {
            info!(
                "✅ Connected to node {} (latency: {}µs)",
                remote_node_id,
                latency.as_micros()
            );
        }

        Ok(())
    }

    /// Fetch a page from remote node
    ///
    /// # Arguments
    /// * `gpa` - Guest physical address
    /// * `remote_node_id` - Node that owns the page
    ///
    /// # Returns
    /// Page data (4KB)
    pub fn fetch_page(&self, gpa: u64, remote_node_id: u32) -> Result<Vec<u8>> {
        self.transport.fetch_page(gpa, remote_node_id)
    }

    /// Send a page to remote node (for migration)
    pub fn send_page(&self, gpa: u64, data: &[u8], remote_node_id: u32) -> Result<()> {
        self.transport.send_page(gpa, data, remote_node_id)
    }

    /// Get current performance tier
    pub fn performance_tier(&self) -> TransportTier {
        self.transport.performance_tier()
    }

    /// Register memory region (for zero-copy if supported)
    pub fn register_memory(
        &self,
        addr: *mut u8,
        length: usize,
    ) -> Result<Box<dyn transport::MemoryRegion>> {
        self.transport.register_memory(addr, length)
    }
}

// Global transport manager (initialized by init_transport)
static mut GLOBAL_TRANSPORT: Option<TransportManager> = None;
static INIT_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// Initialize global transport (call once at startup)
pub fn init_transport(local_node_id: u32) -> Result<()> {
    let _lock = INIT_LOCK.lock();

    unsafe {
        if GLOBAL_TRANSPORT.is_some() {
            return Err(anyhow!("Transport already initialized"));
        }

        GLOBAL_TRANSPORT = Some(TransportManager::new(local_node_id)?);
    }

    Ok(())
}

/// Get global transport manager
pub fn get_transport() -> Result<&'static mut TransportManager> {
    unsafe {
        GLOBAL_TRANSPORT
            .as_mut()
            .ok_or_else(|| anyhow!("Transport not initialized. Call init_transport() first."))
    }
}

/// Connect to remote node (convenience function)
pub fn connect_node(remote_node_id: u32, endpoint: TransportEndpoint) -> Result<()> {
    get_transport()?.connect_peer(remote_node_id, endpoint)
}

/// Fetch page from remote node (convenience function)
pub fn fetch_page(gpa: u64, remote_node_id: u32) -> Result<Vec<u8>> {
    get_transport()?.fetch_page(gpa, remote_node_id)
}

/// Send page to remote node (convenience function)
pub fn send_page(gpa: u64, data: &[u8], remote_node_id: u32) -> Result<()> {
    get_transport()?.send_page(gpa, data, remote_node_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_creation() {
        // Should always work (TCP fallback)
        let transport = TransportManager::new(1);
        if let Err(e) = &transport {
            eprintln!("Transport creation failed: {:?}", e);
        }
        assert!(transport.is_ok());
    }

    #[test]
    fn test_global_init() {
        // Test is isolated, so we can init here
        let result = init_transport(99);
        assert!(result.is_ok());
    }

    #[test]
    fn test_page_size_constant() {
        assert_eq!(PAGE_SIZE, 4096);
    }
}
