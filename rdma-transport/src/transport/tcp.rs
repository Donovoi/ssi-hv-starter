//! TCP-based page transport for consumer-grade hardware
//!
//! Works on ANY network hardware - standard Ethernet NICs, WiFi, etc.
//! Zero special configuration required.
//!
//! Performance characteristics:
//! - 1 Gbps Ethernet: 500-2000µs per page (Development)
//! - 10 Gbps Ethernet: 200-500µs per page (Standard)
//! - Tuned 10G: 100-300µs per page (Good enough for many workloads)

use super::{MemoryRegion, PageTransport, TransportEndpoint, TransportTier};
use anyhow::{anyhow, Context, Result};
use bincode::{deserialize, serialize};
use log::{debug, info, warn};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

const DEFAULT_PORT: u16 = 50051;
const PORT_RANGE_START: u16 = 50051;
const PORT_RANGE_END: u16 = 50100;
const PAGE_SIZE: usize = 4096;
const PING_SIZE: usize = 8;

/// TCP transport implementation
pub struct TcpTransport {
    local_node_id: u32,
    local_addr: SocketAddr,
    peers: Arc<RwLock<HashMap<u32, SocketAddr>>>,
    runtime: Arc<Runtime>,
    measured_tier: Arc<RwLock<Option<TransportTier>>>,
}

/// TCP memory region (just tracks address, no special registration)
struct TcpMemoryRegion {
    addr: *mut u8,
    length: usize,
}

unsafe impl Send for TcpMemoryRegion {}
unsafe impl Sync for TcpMemoryRegion {}

impl MemoryRegion for TcpMemoryRegion {
    fn lkey(&self) -> u32 {
        0 // Not used for TCP
    }

    fn rkey(&self) -> u32 {
        0 // Not used for TCP
    }

    fn addr(&self) -> *mut u8 {
        self.addr
    }

    fn length(&self) -> usize {
        self.length
    }
}

/// Wire protocol messages
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    /// Fetch a page
    FetchPage { gpa: u64 },
    /// Page data response
    PageData { gpa: u64, data: Vec<u8> },
    /// Send a page (for migration)
    SendPage { gpa: u64, data: Vec<u8> },
    /// Acknowledgment
    Ack,
    /// Ping for latency measurement
    Ping { timestamp: u64 },
    /// Pong response
    Pong { timestamp: u64 },
    /// Error response
    Error { message: String },
}

impl TcpTransport {
    /// Create a new TCP transport
    pub fn new(local_node_id: u32) -> Result<Self> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .context("Failed to create Tokio runtime")?,
        );

        // Try to bind to a port in the range (handle multiple instances)
        let local_addr = runtime.block_on(async {
            for port in PORT_RANGE_START..=PORT_RANGE_END {
                match TcpListener::bind(("0.0.0.0", port)).await {
                    Ok(listener) => {
                        let addr = listener
                            .local_addr()
                            .map_err(|e| anyhow!("Failed to get local address: {}", e))?;
                        // Listener will close here, we'll recreate in the task
                        return Ok(addr);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => continue,
                    Err(e) => return Err(anyhow!("Failed to bind TCP listener: {}", e)),
                }
            }
            Err(anyhow!(
                "No available ports in range {}-{}",
                PORT_RANGE_START,
                PORT_RANGE_END
            ))
        })?;

        info!(
            "TCP transport initialized on {} (node_id={})",
            local_addr, local_node_id
        );
        info!("📡 Ready for consumer-grade hardware connections");

        let peers = Arc::new(RwLock::new(HashMap::new()));
        let measured_tier = Arc::new(RwLock::new(None));

        // Start listener task
        let peers_clone = Arc::clone(&peers);
        let runtime_clone = Arc::clone(&runtime);
        runtime.spawn(async move {
            Self::listener_task(peers_clone, runtime_clone).await;
        });

        Ok(Self {
            local_node_id,
            local_addr,
            peers,
            runtime,
            measured_tier,
        })
    }

    /// Background task to accept incoming connections
    async fn listener_task(_peers: Arc<RwLock<HashMap<u32, SocketAddr>>>, _runtime: Arc<Runtime>) {
        // Try to bind to any available port in range
        let (listener, port) = 'bind: loop {
            for port in PORT_RANGE_START..=PORT_RANGE_END {
                match TcpListener::bind(("0.0.0.0", port)).await {
                    Ok(l) => {
                        info!("TCP listener bound to port {}", port);
                        break 'bind (l, port);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => continue,
                    Err(e) => {
                        warn!("Failed to start TCP listener: {}", e);
                        return;
                    }
                }
            }
            warn!(
                "No ports available in range {}-{}",
                PORT_RANGE_START, PORT_RANGE_END
            );
            return;
        };

        info!("Listening for TCP connections on port {}", port);

        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    debug!("Accepted connection from {}", peer_addr);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(socket).await {
                            warn!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    warn!("Accept error: {}", e);
                }
            }
        }
    }

    /// Handle an incoming connection
    async fn handle_connection(mut socket: TcpStream) -> Result<()> {
        // Set TCP_NODELAY for lower latency
        socket.set_nodelay(true)?;

        loop {
            // Read message length (4 bytes)
            let mut len_buf = [0u8; 4];
            match socket.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(_) => break, // Connection closed
            }
            let msg_len = u32::from_be_bytes(len_buf) as usize;

            if msg_len > 10 * 1024 * 1024 {
                // 10MB max
                return Err(anyhow!("Message too large: {}", msg_len));
            }

            // Read message data
            let mut msg_buf = vec![0u8; msg_len];
            socket.read_exact(&mut msg_buf).await?;

            let msg: Message = deserialize(&msg_buf)?;

            // Handle message
            match msg {
                Message::FetchPage { gpa } => {
                    // In real implementation, look up page from local memory
                    debug!("Received FetchPage request for GPA 0x{:x}", gpa);

                    // For now, return zeros (stub implementation)
                    let response = Message::PageData {
                        gpa,
                        data: vec![0u8; PAGE_SIZE],
                    };

                    Self::send_message(&mut socket, &response).await?;
                }
                Message::SendPage { gpa, data } => {
                    debug!(
                        "Received SendPage for GPA 0x{:x} ({} bytes)",
                        gpa,
                        data.len()
                    );

                    // In real implementation, copy to local memory
                    // For now, just acknowledge
                    let response = Message::Ack;
                    Self::send_message(&mut socket, &response).await?;
                }
                Message::Ping { timestamp } => {
                    let response = Message::Pong { timestamp };
                    Self::send_message(&mut socket, &response).await?;
                }
                _ => {
                    warn!("Unexpected message type in server handler");
                }
            }
        }

        Ok(())
    }

    /// Send a message over TCP
    async fn send_message(socket: &mut TcpStream, msg: &Message) -> Result<()> {
        let msg_data = serialize(msg)?;
        let len = (msg_data.len() as u32).to_be_bytes();

        socket.write_all(&len).await?;
        socket.write_all(&msg_data).await?;
        socket.flush().await?;

        Ok(())
    }

    /// Send a message and wait for response
    async fn send_and_receive(peer_addr: SocketAddr, msg: &Message) -> Result<Message> {
        let mut socket = TcpStream::connect(peer_addr)
            .await
            .context("Failed to connect to peer")?;

        socket.set_nodelay(true)?;

        // Send request
        Self::send_message(&mut socket, msg).await?;

        // Read response length
        let mut len_buf = [0u8; 4];
        socket.read_exact(&mut len_buf).await?;
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        // Read response data
        let mut msg_buf = vec![0u8; msg_len];
        socket.read_exact(&mut msg_buf).await?;

        let response: Message = deserialize(&msg_buf)?;
        Ok(response)
    }

    /// Detect network tier based on measured latency
    fn detect_tier(&self, latency: Duration) -> TransportTier {
        if latency < Duration::from_micros(150) {
            TransportTier::MediumPerformance // Unlikely on TCP, but possible with tuning
        } else if latency < Duration::from_micros(500) {
            TransportTier::Standard // 10G Ethernet
        } else {
            TransportTier::Basic // 1G Ethernet
        }
    }
}

impl PageTransport for TcpTransport {
    fn fetch_page(&self, gpa: u64, remote_node_id: u32) -> Result<Vec<u8>> {
        let peer_addr = {
            let peers = self.peers.read();
            *peers
                .get(&remote_node_id)
                .ok_or_else(|| anyhow!("Node {} not connected", remote_node_id))?
        };

        let msg = Message::FetchPage { gpa };

        let response = self
            .runtime
            .block_on(Self::send_and_receive(peer_addr, &msg))?;

        match response {
            Message::PageData { data, .. } => {
                if data.len() != PAGE_SIZE {
                    return Err(anyhow!(
                        "Invalid page size: expected {}, got {}",
                        PAGE_SIZE,
                        data.len()
                    ));
                }
                Ok(data)
            }
            Message::Error { message } => Err(anyhow!("Remote error: {}", message)),
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    fn send_page(&self, gpa: u64, data: &[u8], remote_node_id: u32) -> Result<()> {
        let peer_addr = {
            let peers = self.peers.read();
            *peers
                .get(&remote_node_id)
                .ok_or_else(|| anyhow!("Node {} not connected", remote_node_id))?
        };

        let msg = Message::SendPage {
            gpa,
            data: data.to_vec(),
        };

        let response = self
            .runtime
            .block_on(Self::send_and_receive(peer_addr, &msg))?;

        match response {
            Message::Ack => Ok(()),
            Message::Error { message } => Err(anyhow!("Remote error: {}", message)),
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    fn register_memory(&self, addr: *mut u8, length: usize) -> Result<Box<dyn MemoryRegion>> {
        // TCP doesn't require special registration
        Ok(Box::new(TcpMemoryRegion { addr, length }))
    }

    fn local_endpoint(&self) -> TransportEndpoint {
        // Try to get a real IP address (not 0.0.0.0)
        let addr = if self.local_addr.ip().is_unspecified() {
            // Use first non-loopback interface
            local_ip_address::local_ip()
                .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)))
                .to_string()
        } else {
            self.local_addr.ip().to_string()
        };

        TransportEndpoint::Tcp {
            addr,
            port: self.local_addr.port(),
        }
    }

    fn connect(&mut self, remote_node_id: u32, remote_endpoint: TransportEndpoint) -> Result<()> {
        match remote_endpoint {
            TransportEndpoint::Tcp { addr, port } => {
                let socket_addr = format!("{}:{}", addr, port)
                    .parse::<SocketAddr>()
                    .context("Invalid socket address")?;

                self.peers.write().insert(remote_node_id, socket_addr);

                info!(
                    "Connected to node {} at {} (TCP)",
                    remote_node_id, socket_addr
                );

                // Measure latency on connect
                if let Ok(latency) = self.measure_latency(remote_node_id) {
                    let tier = self.detect_tier(latency);
                    *self.measured_tier.write() = Some(tier);
                    info!(
                        "Network performance: {} (~{}µs latency)",
                        tier,
                        latency.as_micros()
                    );

                    if matches!(tier, TransportTier::Basic) {
                        warn!("⚠️  Slow network detected (>500µs latency)");
                        warn!(
                            "💡 Consider upgrading to 10G Ethernet or RDMA for better performance"
                        );
                    }
                }

                Ok(())
            }
            #[cfg(feature = "rdma-transport")]
            TransportEndpoint::Rdma { .. } => Err(anyhow!(
                "Cannot connect to RDMA endpoint with TCP transport"
            )),
        }
    }

    fn performance_tier(&self) -> TransportTier {
        self.measured_tier.read().unwrap_or(TransportTier::Standard)
    }

    fn measure_latency(&self, remote_node_id: u32) -> Result<Duration> {
        let peer_addr = {
            let peers = self.peers.read();
            *peers
                .get(&remote_node_id)
                .ok_or_else(|| anyhow!("Node {} not connected", remote_node_id))?
        };

        let start = Instant::now();
        let timestamp = start.elapsed().as_nanos() as u64;

        let msg = Message::Ping { timestamp };

        let response = self
            .runtime
            .block_on(Self::send_and_receive(peer_addr, &msg))?;

        let elapsed = start.elapsed();

        match response {
            Message::Pong { .. } => Ok(elapsed),
            _ => Err(anyhow!("Unexpected response to ping")),
        }
    }
}

impl Drop for TcpTransport {
    fn drop(&mut self) {
        debug!("Shutting down TCP transport");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_transport_creation() {
        let transport = TcpTransport::new(1);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_memory_registration() {
        let transport = TcpTransport::new(1).unwrap();
        let mut buffer = vec![0u8; PAGE_SIZE];
        let mr = transport.register_memory(buffer.as_mut_ptr(), PAGE_SIZE);
        assert!(mr.is_ok());
    }
}
