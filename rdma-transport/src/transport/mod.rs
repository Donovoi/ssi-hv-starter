//! Transport layer abstraction for page transfers
//!
//! Supports multiple transport backends:
//! - TCP: Default, works on any network hardware (consumer-grade)
//! - RDMA: Optional, requires InfiniBand/RoCE NICs (high-performance)
//!
//! The system automatically selects the best available transport.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

#[cfg(feature = "tcp-transport")]
pub mod tcp;

#[cfg(feature = "rdma-transport")]
pub mod rdma;

/// Transport-agnostic endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportEndpoint {
    /// TCP endpoint (IP:port)
    Tcp { addr: String, port: u16 },
    /// RDMA endpoint (QP info)
    #[cfg(feature = "rdma-transport")]
    Rdma {
        qpn: u32,
        lid: u16,
        gid: [u8; 16],
        psn: u32,
    },
}

/// Transport performance characteristics
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportTier {
    /// <100µs latency (RDMA InfiniBand)
    HighPerformance,
    /// 100-300µs latency (RDMA over Ethernet - RoCE)
    MediumPerformance,
    /// 200-500µs latency (10G Ethernet with TCP)
    Standard,
    /// >500µs latency (1G Ethernet with TCP)
    Basic,
}

impl TransportTier {
    pub fn expected_latency(&self) -> Duration {
        match self {
            Self::HighPerformance => Duration::from_micros(50),
            Self::MediumPerformance => Duration::from_micros(150),
            Self::Standard => Duration::from_micros(350),
            Self::Basic => Duration::from_millis(1),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::HighPerformance => "RDMA InfiniBand (Production)",
            Self::MediumPerformance => "RDMA over Ethernet (Prosumer)",
            Self::Standard => "10 Gbps Ethernet (Standard)",
            Self::Basic => "1 Gbps Ethernet (Development)",
        }
    }
}

impl fmt::Display for TransportTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Page transport abstraction
///
/// Implementations handle the network-specific details of fetching/sending pages.
/// The pager uses this trait and doesn't care about the underlying transport.
pub trait PageTransport: Send + Sync {
    /// Fetch a page from a remote node
    ///
    /// # Arguments
    /// * `gpa` - Guest physical address of the page
    /// * `remote_node_id` - ID of the node that owns the page
    ///
    /// # Returns
    /// Page data (4KB or 2MB)
    fn fetch_page(&self, gpa: u64, remote_node_id: u32) -> Result<Vec<u8>>;

    /// Send a page to a remote node
    ///
    /// # Arguments
    /// * `gpa` - Guest physical address of the page
    /// * `data` - Page data to send
    /// * `remote_node_id` - ID of the destination node
    fn send_page(&self, gpa: u64, data: &[u8], remote_node_id: u32) -> Result<()>;

    /// Register a memory region for efficient transfers
    ///
    /// # Arguments
    /// * `addr` - Base address of the memory region
    /// * `length` - Size of the region in bytes
    ///
    /// # Returns
    /// Region handle that can be used for zero-copy transfers (transport-specific)
    fn register_memory(&self, addr: *mut u8, length: usize) -> Result<Box<dyn MemoryRegion>>;

    /// Get local endpoint information for sharing with peers
    fn local_endpoint(&self) -> TransportEndpoint;

    /// Connect to a remote peer
    fn connect(&mut self, remote_node_id: u32, remote_endpoint: TransportEndpoint) -> Result<()>;

    /// Get the performance tier of this transport
    fn performance_tier(&self) -> TransportTier;

    /// Measure actual round-trip latency to a peer
    fn measure_latency(&self, remote_node_id: u32) -> Result<Duration>;
}

/// Memory region handle for zero-copy transfers
pub trait MemoryRegion: Send + Sync {
    /// Get the local key for this region (for RDMA or DMA)
    fn lkey(&self) -> u32;

    /// Get the remote key for this region (for RDMA READ/WRITE)
    fn rkey(&self) -> u32;

    /// Get the base address
    fn addr(&self) -> *mut u8;

    /// Get the length in bytes
    fn length(&self) -> usize;
}

/// Auto-detect and create the best available transport
pub fn create_transport(local_node_id: u32) -> Result<Box<dyn PageTransport>> {
    // Try RDMA first if compiled in
    #[cfg(feature = "rdma-transport")]
    {
        if let Ok(transport) = rdma::RdmaTransport::new(local_node_id) {
            log::info!("🚀 Using RDMA transport (high-performance mode)");
            return Ok(Box::new(transport));
        }
        log::warn!("RDMA not available, falling back to TCP");
    }

    // Fall back to TCP (always available)
    #[cfg(feature = "tcp-transport")]
    {
        log::info!("📡 Using TCP transport (consumer hardware mode)");
        log::info!("💡 Tip: Add RDMA NICs (Mellanox ConnectX) for 10× faster page transfers");
        let transport = tcp::TcpTransport::new(local_node_id)?;
        return Ok(Box::new(transport));
    }

    #[cfg(feature = "stub-rdma")]
    {
        anyhow::bail!("No transport available (stub mode)")
    }

    #[cfg(not(any(feature = "tcp-transport", feature = "rdma-transport")))]
    {
        anyhow::bail!("No transport compiled in. Enable tcp-transport or rdma-transport feature.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_tier_ordering() {
        assert!(
            TransportTier::HighPerformance.expected_latency()
                < TransportTier::Standard.expected_latency()
        );
        assert!(
            TransportTier::Standard.expected_latency() < TransportTier::Basic.expected_latency()
        );
    }

    #[test]
    fn test_create_transport() {
        // Should create TCP transport by default
        let transport = create_transport(1);
        assert!(transport.is_ok());
    }
}
