//! Userfaultfd-based pager for distributed memory (M1)
//!
//! Handles page faults in guest memory by:
//! 1. Receiving fault notifications via userfaultfd
//! 2. Checking local page directory for ownership
//! 3. Fetching from remote node via RDMA if needed
//! 4. Resolving fault with UFFDIO_COPY/WAKE

use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, warn};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use userfaultfd::{Event, RegisterMode, Uffd, UffdBuilder};

const PAGE_SIZE: usize = 4096;

/// Page ownership state
#[derive(Debug, Clone, Copy, PartialEq)]
enum PageOwner {
    Local,
    Remote(u32), // node_id
    Unknown,
}

/// Page directory tracking ownership across the cluster
struct PageDirectory {
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
    fn claim_page(&self, page_num: u64) {
        self.ownership.write().insert(page_num, PageOwner::Local);
    }
}

/// Statistics for observability (NFR-observability)
#[derive(Debug, Default)]
pub struct PagerStats {
    pub local_faults: u64,
    pub remote_faults: u64,
    pub fault_service_time_us: Vec<u64>,
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
}

impl Pager {
    fn new(base: *mut u8, len: usize, node_id: u32, total_nodes: u32) -> Result<Self> {
        let uffd = UffdBuilder::new()
            .close_on_exec(true)
            .non_blocking(false)
            .create()
            .context("Failed to create userfaultfd")?;

        // Register memory region for MISSING mode (page faults)
        unsafe {
            uffd.register(base as *mut libc::c_void, len)
                .context("Failed to register userfaultfd")?;
        }

        info!("Userfaultfd registered: base={:p}, len=0x{:x}", base, len);

        Ok(Self {
            uffd,
            base: base as u64,
            len,
            directory: Arc::new(PageDirectory::new(node_id)),
            stats: Arc::new(RwLock::new(PagerStats::default())),
            node_id,
            total_nodes,
        })
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

    /// Fetch page from remote node (M2/M3 - RDMA integration)
    fn fetch_remote_page(&self, addr: u64, remote_node: u32) -> Result<()> {
        debug!(
            "Fetching remote page: addr=0x{:x}, from node {}",
            addr, remote_node
        );

        // TODO M2: Use RDMA transport to fetch page
        // For now, use zero page as fallback
        let page_data =
            rdma_transport::fetch_page(remote_node, addr).unwrap_or_else(|_| vec![0u8; PAGE_SIZE]);

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
}

/// Start pager in background thread
pub fn start_pager(
    base: *mut u8,
    len: usize,
    node_id: u32,
    total_nodes: u32,
) -> Result<JoinHandle<Result<()>>> {
    info!(
        "Starting pager: base={:p}, len=0x{:x}, node={}/{}",
        base, len, node_id, total_nodes
    );

    let pager = Pager::new(base, len, node_id, total_nodes)?;

    let handle = thread::Builder::new()
        .name(format!("pager-node{}", node_id))
        .spawn(move || pager.handle_faults())
        .context("Failed to spawn pager thread")?;

    Ok(handle)
}
