//! Userfaultfd-based pager for distributed memory (M1)
//!
//! Handles page faults in guest memory by:
//! 1. Receiving fault notifications via userfaultfd
//! 2. Checking local page directory for ownership
//! 3. Fetching from remote node via RDMA if needed
//! 4. Resolving fault with UFFDIO_COPY/WAKE

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, warn};
use parking_lot::RwLock;
use rdma_transport::{Endpoint as TransportEndpoint, TransportManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use userfaultfd::{Event, RegisterMode, Uffd, UffdBuilder};

const PAGE_SIZE: usize = 4096;

/// Coordinator endpoint model (matches Python API)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoordinatorEndpoint {
    transport_type: String,
    tcp_addr: Option<String>,
    tcp_port: Option<u16>,
    rdma_qpn: Option<u32>,
    rdma_lid: Option<u16>,
    rdma_gid: Option<String>,
    rdma_psn: Option<u32>,
}

impl CoordinatorEndpoint {
    /// Convert to TransportEndpoint for use with TransportManager
    fn to_transport_endpoint(&self) -> Result<TransportEndpoint> {
        match self.transport_type.as_str() {
            "tcp" => {
                let addr = self
                    .tcp_addr
                    .clone()
                    .ok_or_else(|| anyhow!("Missing tcp_addr"))?;
                let port = self.tcp_port.ok_or_else(|| anyhow!("Missing tcp_port"))?;
                Ok(TransportEndpoint::Tcp { addr, port })
            }
            "rdma" => {
                #[cfg(feature = "rdma-transport")]
                {
                    let qpn = self.rdma_qpn.ok_or_else(|| anyhow!("Missing rdma_qpn"))?;
                    let lid = self.rdma_lid.ok_or_else(|| anyhow!("Missing rdma_lid"))?;
                    let gid_str = self
                        .rdma_gid
                        .clone()
                        .ok_or_else(|| anyhow!("Missing rdma_gid"))?;
                    let psn = self.rdma_psn.ok_or_else(|| anyhow!("Missing rdma_psn"))?;

                    // Parse GID from hex string
                    let gid = hex::decode(gid_str.trim_start_matches("0x"))
                        .map_err(|e| anyhow!("Invalid GID format: {}", e))?;
                    if gid.len() != 16 {
                        return Err(anyhow!("GID must be 16 bytes"));
                    }
                    let mut gid_arr = [0u8; 16];
                    gid_arr.copy_from_slice(&gid);

                    Ok(TransportEndpoint::Rdma {
                        qpn,
                        lid,
                        gid: gid_arr,
                        psn,
                    })
                }
                #[cfg(not(feature = "rdma-transport"))]
                {
                    Err(anyhow!("RDMA transport not compiled in"))
                }
            }
            _ => Err(anyhow!("Unknown transport type: {}", self.transport_type)),
        }
    }
}

/// Page ownership state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOwner {
    Local,
    Remote(u32), // node_id
    Unknown,
}

/// Page directory tracking ownership across the cluster
pub struct PageDirectory {
    /// Map guest physical page number to owner node
    ownership: RwLock<HashMap<u64, PageOwner>>,
    local_node: u32,
}

impl PageDirectory {
    fn new(local_node: u32) -> Self {
        Self {
            ownership: RwLock::new(HashMap::new()),
            local_node,
        }
    }

    /// Get page owner (first-touch policy for M3)
    fn get_owner(&self, page_num: u64) -> PageOwner {
        self.ownership
            .read()
            .get(&page_num)
            .copied()
            .unwrap_or(PageOwner::Unknown)
    }

    /// Claim ownership of a page (first touch)
    pub fn claim_page(&self, page_num: u64) {
        self.ownership.write().insert(page_num, PageOwner::Local);
    }

    /// Set page owner explicitly (for testing and migration)
    pub fn set_owner(&self, page_num: u64, owner: PageOwner) {
        self.ownership.write().insert(page_num, owner);
    }

    /// Get total pages tracked
    pub fn page_count(&self) -> usize {
        self.ownership.read().len()
    }
}

/// Statistics for observability (NFR-observability)
#[derive(Debug, Default, Clone)]
pub struct PagerStats {
    pub local_faults: u64,
    pub remote_faults: u64,
    pub fault_service_time_us: Vec<u64>,
}

impl PagerStats {
    /// Calculate median fault service time
    pub fn median_latency_us(&self) -> Option<u64> {
        if self.fault_service_time_us.is_empty() {
            return None;
        }
        let mut sorted = self.fault_service_time_us.clone();
        sorted.sort_unstable();
        let len = sorted.len();
        if len % 2 == 0 {
            // Even number of elements: average the two middle values
            Some((sorted[len / 2 - 1] + sorted[len / 2]) / 2)
        } else {
            // Odd number: take the middle element
            Some(sorted[len / 2])
        }
    }

    /// Calculate p99 fault service time
    pub fn p99_latency_us(&self) -> Option<u64> {
        if self.fault_service_time_us.is_empty() {
            return None;
        }
        let mut sorted = self.fault_service_time_us.clone();
        sorted.sort_unstable();
        let idx = (sorted.len() as f64 * 0.99) as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    /// Calculate remote miss ratio
    pub fn remote_miss_ratio(&self) -> f64 {
        let total = self.local_faults + self.remote_faults;
        if total == 0 {
            0.0
        } else {
            self.remote_faults as f64 / total as f64
        }
    }
}

/// Main pager structure
pub struct Pager {
    uffd: Uffd,
    base: u64,
    len: usize,
    directory: Arc<PageDirectory>,
    stats: Arc<RwLock<PagerStats>>,
    node_id: u32,
    total_nodes: u32,
    transport: Arc<RwLock<TransportManager>>,
    coordinator_url: String,
}

impl Pager {
    fn new(
        base: *mut u8,
        len: usize,
        node_id: u32,
        total_nodes: u32,
        coordinator_url: &str,
    ) -> Result<Self> {
        let uffd = UffdBuilder::new()
            .close_on_exec(true)
            .non_blocking(false)
            .create()
            .context("Failed to create userfaultfd")?;

        // Register memory region for MISSING mode (page faults)
        info!(
            "Attempting to register memory: base={:p}, len=0x{:x}",
            base, len
        );
        match unsafe { uffd.register(base as *mut libc::c_void, len) } {
            Ok(_) => info!("Successfully registered memory with userfaultfd"),
            Err(e) => {
                eprintln!("Failed to register userfaultfd: {:?}", e);
                return Err(anyhow::anyhow!("Failed to register userfaultfd: {:?}", e));
            }
        }

        info!("Userfaultfd registered: base={:p}, len=0x{:x}", base, len);

        // Initialize transport manager
        info!("Initializing transport layer for node {}...", node_id);
        let mut transport =
            TransportManager::new(node_id).context("Failed to create transport manager")?;

        // Register endpoint with coordinator
        let local_endpoint = transport.local_endpoint();
        Self::register_with_coordinator(coordinator_url, node_id, &local_endpoint)
            .context("Failed to register with coordinator")?;

        // Discover and connect to all peer nodes
        Self::discover_and_connect_peers(coordinator_url, node_id, &mut transport)
            .context("Failed to discover peers")?;

        Ok(Self {
            uffd,
            base: base as u64,
            len,
            directory: Arc::new(PageDirectory::new(node_id)),
            stats: Arc::new(RwLock::new(PagerStats::default())),
            node_id,
            total_nodes,
            transport: Arc::new(RwLock::new(transport)),
            coordinator_url: coordinator_url.to_string(),
        })
    }

    /// Register local endpoint with coordinator
    fn register_with_coordinator(
        coordinator_url: &str,
        node_id: u32,
        endpoint: &TransportEndpoint,
    ) -> Result<()> {
        let client = reqwest::blocking::Client::new();

        let endpoint_json = match endpoint {
            TransportEndpoint::Tcp { addr, port } => serde_json::json!({
                "transport_type": "tcp",
                "tcp_addr": addr,
                "tcp_port": port,
            }),
            #[cfg(feature = "rdma-transport")]
            TransportEndpoint::Rdma { qpn, lid, gid, psn } => serde_json::json!({
                "transport_type": "rdma",
                "rdma_qpn": qpn,
                "rdma_lid": lid,
                "rdma_gid": format!("0x{}", hex::encode(gid)),
                "rdma_psn": psn,
            }),
        };

        let url = format!("{}/nodes/{}/endpoint", coordinator_url, node_id);
        let response = client
            .post(&url)
            .json(&endpoint_json)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .context("Failed to send endpoint registration")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to register endpoint: {}",
                response.status()
            ));
        }

        info!("✅ Registered endpoint with coordinator: {:?}", endpoint);
        Ok(())
    }

    /// Discover peer endpoints from coordinator and connect
    fn discover_and_connect_peers(
        coordinator_url: &str,
        local_node_id: u32,
        transport: &mut TransportManager,
    ) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/endpoints", coordinator_url);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .context("Failed to fetch endpoints")?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch endpoints: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct EndpointsResponse {
            endpoints: HashMap<String, CoordinatorEndpoint>,
        }

        let endpoints_resp: EndpointsResponse = response
            .json()
            .context("Failed to parse endpoints response")?;

        info!(
            "📋 Discovered {} peer nodes",
            endpoints_resp.endpoints.len()
        );

        // Connect to all peers except self
        for (node_id_str, coord_endpoint) in endpoints_resp.endpoints {
            let peer_node_id: u32 = node_id_str.parse().context("Invalid node ID in response")?;

            if peer_node_id == local_node_id {
                continue; // Skip self
            }

            let transport_endpoint = coord_endpoint
                .to_transport_endpoint()
                .context("Failed to convert endpoint")?;

            transport
                .connect_peer(peer_node_id, transport_endpoint)
                .context(format!("Failed to connect to node {}", peer_node_id))?;
        }

        Ok(())
    }

    /// Main fault handling loop
    fn handle_faults(self) -> Result<()> {
        info!(
            "Pager: fault handling loop started on node {}",
            self.node_id
        );

        loop {
            // Read fault event (blocking)
            let event = match self.uffd.read_event() {
                Ok(Some(event)) => event,
                Ok(None) => continue,
                Err(e) => {
                    warn!("Failed to read uffd event: {}", e);
                    continue;
                }
            };

            match event {
                Event::Pagefault { addr, .. } => {
                    let start = std::time::Instant::now();
                    let fault_addr = addr as u64;

                    if let Err(e) = self.handle_pagefault(fault_addr) {
                        warn!("Failed to handle page fault at 0x{:x}: {}", fault_addr, e);
                    }

                    let elapsed = start.elapsed().as_micros() as u64;
                    self.stats.write().fault_service_time_us.push(elapsed);

                    debug!(
                        "Fault serviced: addr=0x{:x}, time={}µs",
                        fault_addr, elapsed
                    );
                }
                Event::Fork { .. } => {
                    info!("Fork event (unhandled)");
                }
                Event::Remap { .. } => {
                    info!("Remap event (unhandled)");
                }
                Event::Remove { .. } => {
                    info!("Remove event (unhandled)");
                }
                Event::Unmap { .. } => {
                    info!("Unmap event (unhandled)");
                }
            }
        }
    }

    /// Handle a single page fault
    fn handle_pagefault(&self, fault_addr: u64) -> Result<()> {
        let page_num = (fault_addr - self.base) / PAGE_SIZE as u64;

        debug!("Page fault: addr=0x{:x}, page_num={}", fault_addr, page_num);

        // Check ownership
        let owner = self.directory.get_owner(page_num);

        match owner {
            PageOwner::Local => {
                // Already local, just zero-fill (shouldn't happen in normal operation)
                self.resolve_with_zeros(fault_addr)?;
                self.stats.write().local_faults += 1;
            }
            PageOwner::Remote(node) => {
                // Fetch from remote node via RDMA
                self.fetch_remote_page(fault_addr, node)?;
                self.stats.write().remote_faults += 1;
            }
            PageOwner::Unknown => {
                // First touch - claim ownership and zero-fill
                self.directory.claim_page(page_num);
                self.resolve_with_zeros(fault_addr)?;
                self.stats.write().local_faults += 1;
            }
        }

        Ok(())
    }

    /// Resolve fault with zero-filled page (local allocation)
    fn resolve_with_zeros(&self, addr: u64) -> Result<()> {
        let zero_page = vec![0u8; PAGE_SIZE];

        unsafe {
            self.uffd
                .copy(
                    zero_page.as_ptr() as *const libc::c_void,
                    addr as *mut libc::c_void,
                    PAGE_SIZE,
                    true,
                )
                .context("Failed to copy zero page")?;
        }

        debug!("Resolved with zeros: addr=0x{:x}", addr);
        Ok(())
    }

    /// Fetch page from remote node via transport layer
    fn fetch_remote_page(&self, addr: u64, remote_node: u32) -> Result<()> {
        debug!(
            "Fetching remote page: addr=0x{:x}, from node {}",
            addr, remote_node
        );

        // Use TransportManager to fetch page (works with TCP or RDMA)
        let transport = self.transport.read();
        let page_data = transport
            .fetch_page(addr, remote_node)
            .context("Failed to fetch page via transport")?;

        if page_data.len() != PAGE_SIZE {
            return Err(anyhow!(
                "Invalid page size: expected {}, got {}",
                PAGE_SIZE,
                page_data.len()
            ));
        }

        unsafe {
            self.uffd
                .copy(
                    page_data.as_ptr() as *const libc::c_void,
                    addr as *mut libc::c_void,
                    PAGE_SIZE,
                    true,
                )
                .context("Failed to copy remote page")?;
        }

        Ok(())
    }

    /// Get statistics for observability
    pub fn get_stats(&self) -> PagerStats {
        let stats = self.stats.read();
        PagerStats {
            local_faults: stats.local_faults,
            remote_faults: stats.remote_faults,
            fault_service_time_us: stats.fault_service_time_us.clone(),
        }
    }

    /// Get page directory for testing
    pub fn directory(&self) -> &Arc<PageDirectory> {
        &self.directory
    }

    /// Get transport manager for testing
    pub fn transport(&self) -> Arc<RwLock<TransportManager>> {
        Arc::clone(&self.transport)
    }
}

/// Start pager in background thread
///
/// # Arguments
/// * `base` - Base address of guest memory region
/// * `len` - Length of memory region
/// * `node_id` - Local node identifier
/// * `total_nodes` - Total nodes in cluster
/// * `coordinator_url` - Coordinator URL (e.g., "http://localhost:8000")
pub fn start_pager(
    base: *mut u8,
    len: usize,
    node_id: u32,
    total_nodes: u32,
    coordinator_url: &str,
) -> Result<JoinHandle<Result<()>>> {
    info!(
        "Starting pager: base={:p}, len=0x{:x}, node={}/{}",
        base, len, node_id, total_nodes
    );
    info!("Coordinator: {}", coordinator_url);

    let pager = Pager::new(base, len, node_id, total_nodes, coordinator_url)?;

    let handle = thread::Builder::new()
        .name(format!("pager-node{}", node_id))
        .spawn(move || pager.handle_faults())
        .context("Failed to spawn pager thread")?;

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_directory_new() {
        let dir = PageDirectory::new(0);
        assert_eq!(dir.local_node, 0);
        assert_eq!(dir.page_count(), 0);
    }

    #[test]
    fn test_page_directory_claim() {
        let dir = PageDirectory::new(0);

        // Initially unknown
        assert_eq!(dir.get_owner(0), PageOwner::Unknown);

        // Claim page
        dir.claim_page(0);
        assert_eq!(dir.get_owner(0), PageOwner::Local);
        assert_eq!(dir.page_count(), 1);
    }

    #[test]
    fn test_page_directory_set_owner() {
        let dir = PageDirectory::new(0);

        // Set remote owner
        dir.set_owner(0, PageOwner::Remote(1));
        assert_eq!(dir.get_owner(0), PageOwner::Remote(1));

        // Change to local
        dir.set_owner(0, PageOwner::Local);
        assert_eq!(dir.get_owner(0), PageOwner::Local);
    }

    #[test]
    fn test_page_directory_multiple_pages() {
        let dir = PageDirectory::new(0);

        dir.claim_page(0);
        dir.set_owner(1, PageOwner::Remote(1));
        dir.set_owner(2, PageOwner::Remote(2));

        assert_eq!(dir.get_owner(0), PageOwner::Local);
        assert_eq!(dir.get_owner(1), PageOwner::Remote(1));
        assert_eq!(dir.get_owner(2), PageOwner::Remote(2));
        assert_eq!(dir.get_owner(3), PageOwner::Unknown);
        assert_eq!(dir.page_count(), 3);
    }

    #[test]
    fn test_pager_stats_default() {
        let stats = PagerStats::default();
        assert_eq!(stats.local_faults, 0);
        assert_eq!(stats.remote_faults, 0);
        assert!(stats.fault_service_time_us.is_empty());
        assert_eq!(stats.remote_miss_ratio(), 0.0);
    }

    #[test]
    fn test_pager_stats_remote_miss_ratio() {
        let mut stats = PagerStats::default();
        stats.local_faults = 95;
        stats.remote_faults = 5;

        assert_eq!(stats.remote_miss_ratio(), 0.05);
    }

    #[test]
    fn test_pager_stats_remote_miss_ratio_zero() {
        let mut stats = PagerStats::default();
        stats.local_faults = 100;
        stats.remote_faults = 0;

        assert_eq!(stats.remote_miss_ratio(), 0.0);
    }

    #[test]
    fn test_pager_stats_remote_miss_ratio_all_remote() {
        let mut stats = PagerStats::default();
        stats.local_faults = 0;
        stats.remote_faults = 100;

        assert_eq!(stats.remote_miss_ratio(), 1.0);
    }

    #[test]
    fn test_pager_stats_median_latency() {
        let mut stats = PagerStats::default();
        stats.fault_service_time_us = vec![10, 20, 30, 40, 50];

        assert_eq!(stats.median_latency_us(), Some(30));
    }

    #[test]
    fn test_pager_stats_median_latency_even_count() {
        let mut stats = PagerStats::default();
        stats.fault_service_time_us = vec![10, 20, 30, 40];

        // Median of even count averages the two middle elements: (20 + 30) / 2 = 25
        assert_eq!(stats.median_latency_us(), Some(25));
    }

    #[test]
    fn test_pager_stats_p99_latency() {
        let mut stats = PagerStats::default();
        stats.fault_service_time_us = (1..=100).collect();

        let p99 = stats.p99_latency_us().unwrap();
        assert!(p99 >= 99);
    }

    #[test]
    fn test_pager_stats_p99_latency_small_sample() {
        let mut stats = PagerStats::default();
        stats.fault_service_time_us = vec![100, 200, 500];

        // p99 with 3 samples should return highest
        assert_eq!(stats.p99_latency_us(), Some(500));
    }

    #[test]
    fn test_pager_stats_empty_latency() {
        let stats = PagerStats::default();
        assert_eq!(stats.median_latency_us(), None);
        assert_eq!(stats.p99_latency_us(), None);
    }

    #[test]
    fn test_page_owner_equality() {
        assert_eq!(PageOwner::Local, PageOwner::Local);
        assert_eq!(PageOwner::Remote(1), PageOwner::Remote(1));
        assert_ne!(PageOwner::Local, PageOwner::Remote(1));
        assert_ne!(PageOwner::Remote(1), PageOwner::Remote(2));
        assert_ne!(PageOwner::Unknown, PageOwner::Local);
    }

    #[test]
    fn test_page_size_constant() {
        assert_eq!(PAGE_SIZE, 4096);
    }

    #[test]
    fn test_page_owner_clone() {
        let owner = PageOwner::Remote(5);
        let cloned = owner.clone();
        assert_eq!(owner, cloned);
    }

    #[test]
    fn test_pager_stats_clone() {
        let mut stats = PagerStats::default();
        stats.local_faults = 10;
        stats.remote_faults = 5;
        stats.fault_service_time_us = vec![100, 200];

        let cloned = stats.clone();
        assert_eq!(cloned.local_faults, 10);
        assert_eq!(cloned.remote_faults, 5);
        assert_eq!(cloned.fault_service_time_us.len(), 2);
    }
}
