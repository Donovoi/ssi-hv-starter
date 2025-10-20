//! RDMA transport stub — fill with verbs bindings
use anyhow::Result;

pub struct RdmaClient;
impl RdmaClient {
    pub fn connect(_addr: &str) -> Result<Self> { Ok(Self) }
}
