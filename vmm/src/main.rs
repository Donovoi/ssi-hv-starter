use anyhow::Result;
use kvm_ioctls::Kvm;
use vm_memory::{GuestAddress, GuestMemoryMmap};
use std::os::fd::AsRawFd;

fn main() -> Result<()> {
    env_logger::init();
    let kvm = Kvm::new()?;
    let vm = kvm.create_vm()?;

    // Minimal 1 GiB guest memory (not yet registered with KVM slots)
    let _gm = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), 1<<30)])?;

    println!("SSI-HV VMM bootstrap created VM: fd={}", vm.as_raw_fd());
    Ok(())
}
