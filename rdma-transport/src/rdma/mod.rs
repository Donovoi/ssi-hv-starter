//! RDMA subsystem
//!
//! Low-level RDMA device and connection management

pub mod connection;
pub mod device;

pub use connection::{QpEndpoint, RdmaConnection};
pub use device::{DeviceAttributes, PortAttributes, RdmaDevice, RdmaMemoryRegion};

// Re-export FFI module for internal use
#[cfg(not(feature = "stub-rdma"))]
pub(crate) mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/rdma_bindings.rs"));
}
