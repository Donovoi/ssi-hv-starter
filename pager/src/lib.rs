//! Userfaultfd pager stub (replace zero-pages with real RDMA fetch later)
use anyhow::Result;
use userfaultfd::{UffdBuilder, RegisterMode};

pub fn start_pager(base: *mut u8, len: usize) -> Result<()> {
    let uffd = UffdBuilder::new().create()?;
    unsafe { uffd.register(base as usize, len, RegisterMode::MISSING)?; }
    println!("pager: registered range {:p}..{:p}", base, unsafe { base.add(len) });
    Ok(())
}
