//! RDMA device management
//!
//! Handles RDMA device discovery, opening, and resource allocation.

use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use std::ffi::CStr;
use std::ptr;
use std::sync::Arc;

#[cfg(not(feature = "stub-rdma"))]
mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/rdma_bindings.rs"));
}

// Stub implementation when RDMA libraries not available
#[cfg(feature = "stub-rdma")]
mod ffi {
    pub type ibv_device = std::ffi::c_void;
    pub type ibv_context = std::ffi::c_void;
    pub type ibv_pd = std::ffi::c_void;
    pub type ibv_mr = std::ffi::c_void;
}

use ffi::*;

/// RDMA device handle with protection domain
pub struct RdmaDevice {
    context: *mut ibv_context,
    pd: *mut ibv_pd,
    device_name: String,
}

unsafe impl Send for RdmaDevice {}
unsafe impl Sync for RdmaDevice {}

impl RdmaDevice {
    /// Open RDMA device by name
    ///
    /// # Arguments
    /// * `device_name` - Name of RDMA device (e.g., "mlx5_0", "rxe0")
    ///
    /// # Returns
    /// Initialized device with protection domain
    pub fn open(device_name: &str) -> Result<Arc<Self>> {
        #[cfg(feature = "stub-rdma")]
        {
            warn!("RDMA stub mode - no real device");
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            info!("Opening RDMA device: {}", device_name);

            // Get list of available devices
            let mut num_devices = 0i32;
            let device_list = unsafe { ibv_get_device_list(&mut num_devices) };

            if device_list.is_null() {
                return Err(anyhow!("No RDMA devices found"));
            }

            // Find device by name
            let mut target_device: *mut ibv_device = ptr::null_mut();
            for i in 0..num_devices {
                let device = unsafe { *device_list.offset(i as isize) };
                let name = unsafe {
                    CStr::from_ptr(ibv_get_device_name(device))
                        .to_string_lossy()
                        .into_owned()
                };

                debug!("Found RDMA device: {}", name);

                if name == device_name {
                    target_device = device;
                    break;
                }
            }

            if target_device.is_null() {
                unsafe { ibv_free_device_list(device_list) };
                return Err(anyhow!("Device {} not found", device_name));
            }

            // Open device context
            let context = unsafe { ibv_open_device(target_device) };
            unsafe { ibv_free_device_list(device_list) };

            if context.is_null() {
                return Err(anyhow!("Failed to open device {}", device_name));
            }

            info!("Opened RDMA device: {}", device_name);

            // Allocate protection domain
            let pd = unsafe { ibv_alloc_pd(context) };
            if pd.is_null() {
                unsafe { ibv_close_device(context) };
                return Err(anyhow!("Failed to allocate protection domain"));
            }

            info!("Allocated protection domain");

            Ok(Arc::new(Self {
                context,
                pd,
                device_name: device_name.to_string(),
            }))
        }
    }

    /// Query device attributes
    pub fn query_attributes(&self) -> Result<DeviceAttributes> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            let mut attr: ibv_device_attr = unsafe { std::mem::zeroed() };
            let ret = unsafe { ibv_query_device(self.context, &mut attr) };

            if ret != 0 {
                return Err(anyhow!("Failed to query device attributes"));
            }

            Ok(DeviceAttributes {
                max_qp: attr.max_qp,
                max_cq: attr.max_cq,
                max_mr: attr.max_mr,
                max_mr_size: attr.max_mr_size,
            })
        }
    }

    /// Query port attributes
    pub fn query_port(&self, port_num: u8) -> Result<PortAttributes> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            let mut attr: ibv_port_attr = unsafe { std::mem::zeroed() };
            let ret = unsafe { ibv_query_port(self.context, port_num, &mut attr) };

            if ret != 0 {
                return Err(anyhow!("Failed to query port {}", port_num));
            }

            // Query GID
            let mut gid: ibv_gid = unsafe { std::mem::zeroed() };
            let ret = unsafe { ibv_query_gid(self.context, port_num, 0, &mut gid) };

            if ret != 0 {
                warn!("Failed to query GID for port {}", port_num);
            }

            Ok(PortAttributes {
                state: attr.state,
                lid: attr.lid,
                gid: gid.raw,
            })
        }
    }

    /// Register memory region for RDMA access
    ///
    /// # Arguments
    /// * `addr` - Virtual address of memory region
    /// * `length` - Size of memory region in bytes
    ///
    /// # Returns
    /// Memory region handle with lkey and rkey
    pub fn register_memory(&self, addr: *mut u8, length: usize) -> Result<RdmaMemoryRegion> {
        #[cfg(feature = "stub-rdma")]
        {
            return Err(anyhow!("RDMA not available (stub mode)"));
        }

        #[cfg(not(feature = "stub-rdma"))]
        {
            debug!("Registering memory: addr={:?}, len={}", addr, length);

            let access_flags =
                IBV_ACCESS_LOCAL_WRITE | IBV_ACCESS_REMOTE_READ | IBV_ACCESS_REMOTE_WRITE;

            let mr = unsafe {
                ibv_reg_mr(
                    self.pd,
                    addr as *mut libc::c_void,
                    length,
                    access_flags as i32,
                )
            };

            if mr.is_null() {
                return Err(anyhow!("Failed to register memory region"));
            }

            let lkey = unsafe { (*mr).lkey };
            let rkey = unsafe { (*mr).rkey };

            debug!("Registered MR: lkey=0x{:x}, rkey=0x{:x}", lkey, rkey);

            Ok(RdmaMemoryRegion {
                mr,
                addr,
                length,
                lkey,
                rkey,
            })
        }
    }

    /// Get raw context pointer (for QP creation)
    pub(crate) fn context(&self) -> *mut ibv_context {
        self.context
    }

    /// Get raw protection domain pointer (for QP creation)
    pub(crate) fn pd(&self) -> *mut ibv_pd {
        self.pd
    }

    /// Get device name
    pub fn name(&self) -> &str {
        &self.device_name
    }
}

impl Drop for RdmaDevice {
    fn drop(&mut self) {
        #[cfg(not(feature = "stub-rdma"))]
        {
            debug!("Closing RDMA device: {}", self.device_name);

            if !self.pd.is_null() {
                unsafe { ibv_dealloc_pd(self.pd) };
            }

            if !self.context.is_null() {
                unsafe { ibv_close_device(self.context) };
            }
        }
    }
}

/// RDMA memory region
pub struct RdmaMemoryRegion {
    mr: *mut ibv_mr,
    pub addr: *mut u8,
    pub length: usize,
    pub lkey: u32,
    pub rkey: u32,
}

unsafe impl Send for RdmaMemoryRegion {}
unsafe impl Sync for RdmaMemoryRegion {}

impl Drop for RdmaMemoryRegion {
    fn drop(&mut self) {
        #[cfg(not(feature = "stub-rdma"))]
        {
            if !self.mr.is_null() {
                unsafe { ibv_dereg_mr(self.mr) };
            }
        }
    }
}

/// Device attributes
#[derive(Debug, Clone)]
pub struct DeviceAttributes {
    pub max_qp: i32,
    pub max_cq: i32,
    pub max_mr: i32,
    pub max_mr_size: u64,
}

/// Port attributes
#[derive(Debug, Clone)]
pub struct PortAttributes {
    pub state: u32,
    pub lid: u16,
    pub gid: [u8; 16],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires RDMA hardware
    fn test_device_open() {
        let device = RdmaDevice::open("mlx5_0");
        // May fail if no hardware, that's expected
        if let Ok(dev) = device {
            assert!(!dev.name().is_empty());
        }
    }

    #[test]
    #[ignore] // Requires RDMA hardware
    fn test_query_attributes() {
        if let Ok(device) = RdmaDevice::open("mlx5_0") {
            let attr = device.query_attributes();
            assert!(attr.is_ok());
            let attr = attr.unwrap();
            assert!(attr.max_qp > 0);
        }
    }

    #[test]
    #[ignore] // Requires RDMA hardware
    fn test_memory_registration() {
        if let Ok(device) = RdmaDevice::open("mlx5_0") {
            let mut buffer = vec![0u8; 4096];
            let mr = device.register_memory(buffer.as_mut_ptr(), buffer.len());
            assert!(mr.is_ok());
            let mr = mr.unwrap();
            assert_eq!(mr.length, 4096);
            assert!(mr.lkey != 0);
            assert!(mr.rkey != 0);
        }
    }
}
