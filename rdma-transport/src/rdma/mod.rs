//! RDMA subsystem
//!
//! Low-level RDMA device and connection management

// FFI bindings - must be declared first so submodules can use it
#[cfg(not(feature = "stub-rdma"))]
pub(crate) mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/rdma_bindings.rs"));
}

// Stub FFI types when RDMA libraries not available
#[cfg(feature = "stub-rdma")]
pub(crate) mod ffi {
    pub type ibv_device = std::ffi::c_void;
    pub type ibv_context = std::ffi::c_void;
    pub type ibv_pd = std::ffi::c_void;
    pub type ibv_mr = std::ffi::c_void;
    pub type ibv_cq = std::ffi::c_void;
    pub type ibv_qp = std::ffi::c_void;
    pub type ibv_device_attr = std::ffi::c_void;
    pub type ibv_port_attr = std::ffi::c_void;
    pub type ibv_gid = std::ffi::c_void;
    pub type ibv_qp_init_attr = std::ffi::c_void;
    pub type ibv_qp_attr = std::ffi::c_void;
    pub type ibv_send_wr = std::ffi::c_void;
    pub type ibv_sge = std::ffi::c_void;
    pub type ibv_wc = std::ffi::c_void;

    // Stub constants
    pub const IBV_ACCESS_LOCAL_WRITE: u32 = 1;
    pub const IBV_ACCESS_REMOTE_READ: u32 = 2;
    pub const IBV_ACCESS_REMOTE_WRITE: u32 = 4;

    // Stub enums
    #[repr(u32)]
    pub enum ibv_qp_type {
        IBV_QPT_RC = 2,
    }
    #[repr(u32)]
    pub enum ibv_qp_state {
        IBV_QPS_RESET = 0,
        IBV_QPS_INIT = 1,
        IBV_QPS_RTR = 2,
        IBV_QPS_RTS = 3,
    }
    #[repr(u32)]
    pub enum ibv_mtu {
        IBV_MTU_4096 = 5,
    }
    #[repr(u32)]
    pub enum ibv_wr_opcode {
        IBV_WR_RDMA_READ = 2,
        IBV_WR_RDMA_WRITE = 3,
    }
    #[repr(u32)]
    pub enum ibv_wc_status {
        IBV_WC_SUCCESS = 0,
    }

    pub struct ibv_qp_attr_mask(pub i32);
    impl ibv_qp_attr_mask {
        pub const IBV_QP_STATE: Self = Self(1);
        pub const IBV_QP_PKEY_INDEX: Self = Self(2);
        pub const IBV_QP_PORT: Self = Self(4);
        pub const IBV_QP_ACCESS_FLAGS: Self = Self(8);
        pub const IBV_QP_AV: Self = Self(16);
        pub const IBV_QP_PATH_MTU: Self = Self(32);
        pub const IBV_QP_DEST_QPN: Self = Self(64);
        pub const IBV_QP_RQ_PSN: Self = Self(128);
        pub const IBV_QP_MAX_DEST_RD_ATOMIC: Self = Self(256);
        pub const IBV_QP_MIN_RNR_TIMER: Self = Self(512);
        pub const IBV_QP_TIMEOUT: Self = Self(1024);
        pub const IBV_QP_RETRY_CNT: Self = Self(2048);
        pub const IBV_QP_RNR_RETRY: Self = Self(4096);
        pub const IBV_QP_SQ_PSN: Self = Self(8192);
        pub const IBV_QP_MAX_QP_RD_ATOMIC: Self = Self(16384);
    }

    pub struct ibv_send_flags(pub u32);
    impl ibv_send_flags {
        pub const IBV_SEND_SIGNALED: Self = Self(2);
    }
}

pub mod connection;
pub mod device;

pub use connection::{QpEndpoint, RdmaConnection};
pub use device::{DeviceAttributes, PortAttributes, RdmaDevice, RdmaMemoryRegion};
