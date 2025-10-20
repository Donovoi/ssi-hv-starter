use anyhow::{Context, Result};
use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuFd, VmFd};
use log::{info, warn};
use std::os::fd::AsRawFd;
use std::thread;
use vm_memory::{Address, GuestAddress, GuestMemory, GuestMemoryMmap, GuestMemoryRegion};

mod vcpu;

/// SSI-HV VMM Configuration
#[derive(Debug)]
struct VmmConfig {
    /// Guest physical memory size in bytes
    mem_size: usize,
    /// Number of vCPUs
    num_vcpus: u32,
    /// Node ID in the cluster (0 for local-only mode)
    node_id: u32,
    /// Total nodes in cluster
    total_nodes: u32,
}

impl Default for VmmConfig {
    fn default() -> Self {
        Self {
            mem_size: 1 << 30, // 1 GiB
            num_vcpus: 2,
            node_id: 0,
            total_nodes: 1,
        }
    }
}

/// Main VMM structure managing the guest VM
struct SsiVmm {
    kvm: Kvm,
    vm: VmFd,
    guest_memory: GuestMemoryMmap<()>,
    config: VmmConfig,
}

impl SsiVmm {
    fn new(config: VmmConfig) -> Result<Self> {
        let kvm = Kvm::new().context("Failed to open /dev/kvm")?;
        let vm = kvm.create_vm().context("Failed to create VM")?;

        info!("Created KVM VM: fd={}", vm.as_raw_fd());
        info!(
            "Config: mem_size={}MB, vcpus={}, node={}/{}",
            config.mem_size >> 20,
            config.num_vcpus,
            config.node_id,
            config.total_nodes
        );

        // Create guest memory
        let guest_memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), config.mem_size)])
            .context("Failed to create guest memory")?;

        Ok(Self {
            kvm,
            vm,
            guest_memory,
            config,
        })
    }

    /// Setup KVM memory slots
    fn setup_memory(&mut self) -> Result<()> {
        info!("Setting up KVM memory slots");

        for (slot, region) in self.guest_memory.iter().enumerate() {
            let mem_region = kvm_userspace_memory_region {
                slot: slot as u32,
                flags: 0,
                guest_phys_addr: region.start_addr().raw_value(),
                memory_size: region.len() as u64,
                userspace_addr: region.as_ptr() as u64,
            };

            unsafe {
                self.vm
                    .set_user_memory_region(mem_region)
                    .context("Failed to set KVM memory region")?;
            }

            info!(
                "Mapped slot {}: GPA 0x{:x}, size 0x{:x}",
                slot, mem_region.guest_phys_addr, mem_region.memory_size
            );
        }

        Ok(())
    }

    /// Initialize userfaultfd pager for distributed memory
    fn setup_pager(&self) -> Result<()> {
        info!("Initializing userfaultfd pager");

        // Get the first memory region
        let region = self
            .guest_memory
            .iter()
            .next()
            .context("No memory regions available")?;

        let base = region.as_ptr();
        let len = region.len() as usize;

        // Start pager with RDMA transport info
        pager::start_pager(base, len, self.config.node_id, self.config.total_nodes)
            .context("Failed to start pager")?;

        info!("Pager registered: base={:p}, len=0x{:x}", base, len);
        Ok(())
    }

    /// Create and configure vCPUs
    fn create_vcpus(&self) -> Result<Vec<VcpuFd>> {
        let mut vcpus = Vec::new();

        for i in 0..self.config.num_vcpus {
            let vcpu = self
                .vm
                .create_vcpu(i as u64)
                .context(format!("Failed to create vCPU {}", i))?;

            // Setup CPUID (use reasonable default of 256 entries)
            let cpuid = self
                .kvm
                .get_supported_cpuid(256)
                .context("Failed to get supported CPUID")?;
            vcpu.set_cpuid2(&cpuid).context("Failed to set CPUID")?;

            info!("Created vCPU {}", i);
            vcpus.push(vcpu);
        }

        Ok(vcpus)
    }

    fn run(&mut self) -> Result<()> {
        // Setup memory slots in KVM
        self.setup_memory()?;

        // Initialize pager for distributed memory
        self.setup_pager()?;

        // Create vCPUs
        let vcpus = self.create_vcpus()?;

        info!("SSI-HV VMM initialized successfully");
        info!(
            "VM fd={}, vCPUs={}, memory={}MB",
            self.vm.as_raw_fd(),
            vcpus.len(),
            self.config.mem_size >> 20
        );

        // TODO: Setup serial console, load OVMF, start vCPU run loops
        warn!("vCPU run loops not yet implemented - VM created but not running");

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("SSI-HV VMM starting (M0/M1 implementation)");

    let config = VmmConfig::default();
    let mut vmm = SsiVmm::new(config)?;
    vmm.run()?;

    info!("VMM initialization complete");

    // Keep running for testing
    info!("Press Ctrl+C to exit");
    thread::park();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmm_config_default() {
        let config = VmmConfig::default();
        assert_eq!(config.mem_size, 1 << 30);
        assert_eq!(config.num_vcpus, 2);
        assert_eq!(config.node_id, 0);
        assert_eq!(config.total_nodes, 1);
    }

    #[test]
    fn test_vmm_config_custom() {
        let config = VmmConfig {
            mem_size: 2 << 30,
            num_vcpus: 4,
            node_id: 1,
            total_nodes: 2,
        };
        assert_eq!(config.mem_size, 2 << 30);
        assert_eq!(config.num_vcpus, 4);
        assert_eq!(config.node_id, 1);
        assert_eq!(config.total_nodes, 2);
    }

    #[test]
    fn test_vmm_config_memory_sizes() {
        let config_1gb = VmmConfig {
            mem_size: 1 << 30,
            ..Default::default()
        };
        assert_eq!(config_1gb.mem_size, 1_073_741_824);

        let config_4gb = VmmConfig {
            mem_size: 4 << 30,
            ..Default::default()
        };
        assert_eq!(config_4gb.mem_size, 4_294_967_296);
    }
}
