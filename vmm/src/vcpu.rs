/// vCPU management module for SSI-HV
use anyhow::Result;
use kvm_ioctls::VcpuFd;
use log::info;

/// Manages vCPU lifecycle and execution
pub struct VcpuManager {
    vcpu: VcpuFd,
    id: u32,
}

impl VcpuManager {
    pub fn new(vcpu: VcpuFd, id: u32) -> Self {
        Self { vcpu, id }
    }

    /// Run the vCPU in a loop (to be implemented)
    pub fn run(&mut self) -> Result<()> {
        info!("vCPU {} run loop starting", self.id);
        // TODO: Implement KVM_RUN loop with exit handling
        // - KVM_EXIT_IO for serial console
        // - KVM_EXIT_MMIO for device access
        // - KVM_EXIT_HLT
        Ok(())
    }
}
