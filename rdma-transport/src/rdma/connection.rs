//! RDMA connection and queue pair management
//!
//! Implements RDMA Reliable Connection (RC) queue pairs for page transfers.

use super::device::{RdmaDevice, RdmaMemoryRegion};
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::ptr;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(not(feature = "stub-rdma"))]
use super::device::ffi::*;

/// RDMA connection endpoint information
///
/// This is exchanged between nodes to establish connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpEndpoint {
    pub qpn: u32,      // Queue Pair Number
    pub lid: u16,      // Local Identifier
    pub gid: [u8; 16], // Global Identifier
    pub psn: u32,      // Packet Sequence Number (for flow control)
}

/// RDMA connection with RC queue pair
pub struct RdmaConnection {
    device: Arc<RdmaDevice>,
    #[cfg(not(feature = "stub-rdma"))]
    qp: *mut ibv_qp,
    #[cfg(not(feature = "stub-rdma"))]
    cq_send: *mut ibv_cq,
    #[cfg(not(feature = "stub-rdma"))]
    cq_recv: *mut ibv_cq,
    local_endpoint: QpEndpoint,
    remote_endpoint: Option<QpEndpoint>,
    pub remote_node_id: u32,
}

unsafe impl Send for RdmaConnection {}
unsafe impl Sync for RdmaConnection {}

impl RdmaConnection {
    /// Create new RDMA connection (QP in RESET state)
    ///
    /// # Arguments
    /// * `device` - RDMA device handle
    /// * `cq_depth` - Completion queue depth (number of outstanding operations)
    pub fn create(device: Arc<RdmaDevice>, cq_depth: u32) -> Result<Self> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            info!("Creating RDMA connection, CQ depth={}", cq_depth);

            // Create completion queues
            let cq_send = unsafe {
                ibv_create_cq(
                    device.context(),
                    cq_depth as i32,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                )
            };

            if cq_send.is_null() {
                return Err(anyhow!("Failed to create send CQ"));
            }

            let cq_recv = unsafe {
                ibv_create_cq(
                    device.context(),
                    cq_depth as i32,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    0,
                )
            };

            if cq_recv.is_null() {
                unsafe { ibv_destroy_cq(cq_send) };
                return Err(anyhow!("Failed to create recv CQ"));
            }

            debug!("Created CQs: send={:?}, recv={:?}", cq_send, cq_recv);

            // Create queue pair
            let mut qp_init_attr: ibv_qp_init_attr = unsafe { std::mem::zeroed() };
            qp_init_attr.send_cq = cq_send;
            qp_init_attr.recv_cq = cq_recv;
            qp_init_attr.qp_type = ibv_qp_type::IBV_QPT_RC;
            qp_init_attr.cap.max_send_wr = cq_depth;
            qp_init_attr.cap.max_recv_wr = cq_depth;
            qp_init_attr.cap.max_send_sge = 1;
            qp_init_attr.cap.max_recv_sge = 1;
            qp_init_attr.cap.max_inline_data = 64; // Small inline data support

            let qp = unsafe { ibv_create_qp(device.pd(), &mut qp_init_attr) };

            if qp.is_null() {
                unsafe {
                    ibv_destroy_cq(cq_send);
                    ibv_destroy_cq(cq_recv);
                }
                return Err(anyhow!("Failed to create QP"));
            }

            let qpn = unsafe { (*qp).qp_num };
            debug!("Created QP: qpn={}", qpn);

            // Query port to get LID and GID
            let port = device.query_port(1)?;

            let local_endpoint = QpEndpoint {
                qpn,
                lid: port.lid,
                gid: port.gid,
                psn: rand::random::<u32>() & 0xffffff, // 24-bit PSN
            };

            debug!(
                "Local endpoint: qpn={}, lid={}",
                local_endpoint.qpn, local_endpoint.lid
            );

            Ok(Self {
                device,
                qp,
                cq_send,
                cq_recv,
                local_endpoint,
                remote_endpoint: None,
                remote_node_id: 0,
            })
        }
    }

    /// Get local endpoint info for exchange
    pub fn local_endpoint(&self) -> &QpEndpoint {
        &self.local_endpoint
    }

    /// Connect to remote node using exchanged endpoint
    ///
    /// Transitions QP: RESET → INIT → RTR → RTS
    pub fn connect(&mut self, remote_node_id: u32, remote_ep: QpEndpoint) -> Result<()> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            info!(
                "Connecting to node {}, remote_qpn={}",
                remote_node_id, remote_ep.qpn
            );

            self.remote_node_id = remote_node_id;
            self.remote_endpoint = Some(remote_ep.clone());

            // Transition to INIT state
            self.qp_to_init()?;

            // Transition to RTR (Ready To Receive)
            self.qp_to_rtr(&remote_ep)?;

            // Transition to RTS (Ready To Send)
            self.qp_to_rts()?;

            info!("QP connection established to node {}", remote_node_id);
            Ok(())
        }
    }

    #[cfg(not(feature = "stub-rdma"))]
    fn qp_to_init(&self) -> Result<()> {
        let mut attr: ibv_qp_attr = unsafe { std::mem::zeroed() };
        attr.qp_state = ibv_qp_state::IBV_QPS_INIT;
        attr.pkey_index = 0;
        attr.port_num = 1;
        attr.qp_access_flags =
            (IBV_ACCESS_REMOTE_READ | IBV_ACCESS_REMOTE_WRITE | IBV_ACCESS_LOCAL_WRITE) as u32;

        let mask = ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_PKEY_INDEX
            | ibv_qp_attr_mask::IBV_QP_PORT
            | ibv_qp_attr_mask::IBV_QP_ACCESS_FLAGS;

        let ret = unsafe { ibv_modify_qp(self.qp, &mut attr, mask.0 as i32) };

        if ret != 0 {
            return Err(anyhow!("Failed to transition QP to INIT"));
        }

        debug!("QP transitioned to INIT");
        Ok(())
    }

    #[cfg(not(feature = "stub-rdma"))]
    fn qp_to_rtr(&self, remote_ep: &QpEndpoint) -> Result<()> {
        let mut attr: ibv_qp_attr = unsafe { std::mem::zeroed() };
        attr.qp_state = ibv_qp_state::IBV_QPS_RTR;
        attr.path_mtu = ibv_mtu::IBV_MTU_4096;
        attr.dest_qp_num = remote_ep.qpn;
        attr.rq_psn = remote_ep.psn;
        attr.max_dest_rd_atomic = 1;
        attr.min_rnr_timer = 12;
        attr.ah_attr.dlid = remote_ep.lid;
        attr.ah_attr.sl = 0;
        attr.ah_attr.src_path_bits = 0;
        attr.ah_attr.port_num = 1;

        // Set GID if available (for RoCE)
        if remote_ep.gid != [0u8; 16] {
            attr.ah_attr.is_global = 1;
            attr.ah_attr.grh.dgid.raw = remote_ep.gid;
            attr.ah_attr.grh.sgid_index = 0;
            attr.ah_attr.grh.hop_limit = 64;
        }

        let mask = ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_AV
            | ibv_qp_attr_mask::IBV_QP_PATH_MTU
            | ibv_qp_attr_mask::IBV_QP_DEST_QPN
            | ibv_qp_attr_mask::IBV_QP_RQ_PSN
            | ibv_qp_attr_mask::IBV_QP_MAX_DEST_RD_ATOMIC
            | ibv_qp_attr_mask::IBV_QP_MIN_RNR_TIMER;

        let ret = unsafe { ibv_modify_qp(self.qp, &mut attr, mask.0 as i32) };

        if ret != 0 {
            return Err(anyhow!("Failed to transition QP to RTR"));
        }

        debug!("QP transitioned to RTR");
        Ok(())
    }

    #[cfg(not(feature = "stub-rdma"))]
    fn qp_to_rts(&self) -> Result<()> {
        let mut attr: ibv_qp_attr = unsafe { std::mem::zeroed() };
        attr.qp_state = ibv_qp_state::IBV_QPS_RTS;
        attr.sq_psn = self.local_endpoint.psn;
        attr.timeout = 14; // ~67ms
        attr.retry_cnt = 7;
        attr.rnr_retry = 7;
        attr.max_rd_atomic = 1;

        let mask = ibv_qp_attr_mask::IBV_QP_STATE
            | ibv_qp_attr_mask::IBV_QP_TIMEOUT
            | ibv_qp_attr_mask::IBV_QP_RETRY_CNT
            | ibv_qp_attr_mask::IBV_QP_RNR_RETRY
            | ibv_qp_attr_mask::IBV_QP_SQ_PSN
            | ibv_qp_attr_mask::IBV_QP_MAX_QP_RD_ATOMIC;

        let ret = unsafe { ibv_modify_qp(self.qp, &mut attr, mask.0 as i32) };

        if ret != 0 {
            return Err(anyhow!("Failed to transition QP to RTS"));
        }

        debug!("QP transitioned to RTS");
        Ok(())
    }

    /// Perform RDMA READ operation
    ///
    /// # Arguments
    /// * `local_mr` - Local memory region to read into
    /// * `local_offset` - Offset within local MR
    /// * `remote_addr` - Remote virtual address
    /// * `remote_rkey` - Remote memory region key
    /// * `length` - Number of bytes to read
    ///
    /// # Returns
    /// Duration of the operation
    pub fn rdma_read(
        &self,
        local_mr: &RdmaMemoryRegion,
        local_offset: usize,
        remote_addr: u64,
        remote_rkey: u32,
        length: usize,
    ) -> Result<Duration> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            let start = Instant::now();

            let wr_id = self.generate_wr_id();

            // Scatter-gather element
            let mut sge = ibv_sge {
                addr: (local_mr.addr as u64) + (local_offset as u64),
                length: length as u32,
                lkey: local_mr.lkey,
            };

            // Work request
            let mut wr: ibv_send_wr = unsafe { std::mem::zeroed() };
            wr.wr_id = wr_id;
            wr.sg_list = &mut sge;
            wr.num_sge = 1;
            wr.opcode = ibv_wr_opcode::IBV_WR_RDMA_READ;
            wr.send_flags = ibv_send_flags::IBV_SEND_SIGNALED.0;
            wr.wr.rdma.remote_addr = remote_addr;
            wr.wr.rdma.rkey = remote_rkey;

            // Post send
            let mut bad_wr: *mut ibv_send_wr = ptr::null_mut();
            let ret = unsafe { ibv_post_send(self.qp, &mut wr, &mut bad_wr) };

            if ret != 0 {
                return Err(anyhow!("Failed to post RDMA READ"));
            }

            // Poll for completion
            self.poll_send_completion(wr_id)?;

            Ok(start.elapsed())
        }
    }

    /// Perform RDMA WRITE operation
    pub fn rdma_write(
        &self,
        local_mr: &RdmaMemoryRegion,
        local_offset: usize,
        remote_addr: u64,
        remote_rkey: u32,
        length: usize,
    ) -> Result<Duration> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            let start = Instant::now();

            let wr_id = self.generate_wr_id();

            let mut sge = ibv_sge {
                addr: (local_mr.addr as u64) + (local_offset as u64),
                length: length as u32,
                lkey: local_mr.lkey,
            };

            let mut wr: ibv_send_wr = unsafe { std::mem::zeroed() };
            wr.wr_id = wr_id;
            wr.sg_list = &mut sge;
            wr.num_sge = 1;
            wr.opcode = ibv_wr_opcode::IBV_WR_RDMA_WRITE;
            wr.send_flags = ibv_send_flags::IBV_SEND_SIGNALED.0;
            wr.wr.rdma.remote_addr = remote_addr;
            wr.wr.rdma.rkey = remote_rkey;

            let mut bad_wr: *mut ibv_send_wr = ptr::null_mut();
            let ret = unsafe { ibv_post_send(self.qp, &mut wr, &mut bad_wr) };

            if ret != 0 {
                return Err(anyhow!("Failed to post RDMA WRITE"));
            }

            self.poll_send_completion(wr_id)?;

            Ok(start.elapsed())
        }
    }

    #[cfg(not(feature = "stub-rdma"))]
    fn poll_send_completion(&self, expected_wr_id: u64) -> Result<()> {
        let mut wc: ibv_wc = unsafe { std::mem::zeroed() };
        let timeout = Instant::now() + Duration::from_secs(5);

        loop {
            let n = unsafe { ibv_poll_cq(self.cq_send, 1, &mut wc) };

            if n < 0 {
                return Err(anyhow!("CQ polling failed"));
            }

            if n > 0 {
                if wc.status != ibv_wc_status::IBV_WC_SUCCESS {
                    return Err(anyhow!("RDMA operation failed: status={:?}", wc.status));
                }

                if wc.wr_id == expected_wr_id {
                    return Ok(());
                }
            }

            if Instant::now() > timeout {
                return Err(anyhow!("Timeout waiting for completion"));
            }

            // Small backoff to avoid spinning
            std::thread::sleep(Duration::from_micros(1));
        }
    }

    fn generate_wr_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static WR_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        WR_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

impl Drop for RdmaConnection {
    fn drop(&mut self) {
        #[cfg(not(feature = "stub-rdma"))]
        {
            unsafe {
                if !self.qp.is_null() {
                    ibv_destroy_qp(self.qp);
                }
                if !self.cq_send.is_null() {
                    ibv_destroy_cq(self.cq_send);
                }
                if !self.cq_recv.is_null() {
                    ibv_destroy_cq(self.cq_recv);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires RDMA hardware
    fn test_connection_creation() {
        if let Ok(device) = RdmaDevice::open("mlx5_0") {
            let conn = RdmaConnection::create(device, 128);
            assert!(conn.is_ok());
            let conn = conn.unwrap();
            assert!(conn.local_endpoint().qpn > 0);
        }
    }
}
