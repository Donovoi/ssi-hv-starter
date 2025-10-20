# Consumer Hardware Feasibility Analysis

**Date:** October 20, 2025  
**Question:** Can we run SSI-HV on consumer-grade hardware (normal computers)?

## TL;DR

**Short answer:** 🟡 **Partially Feasible** - You can run it, but won't meet the <100µs latency targets without significant trade-offs.

**Practical answer:** ✅ **YES for development/testing**, ⚠️ **NO for production performance**

---

## Current Architecture Requirements

### What We Built For
- **RDMA NICs**: InfiniBand or RoCEv2-capable adapters (Mellanox ConnectX)
- **Target Latency**: <100µs median, <500µs p99 for remote page faults
- **Bandwidth**: 100-400 Gbps fabrics
- **Cost**: ~$500-2000 per node (NICs + switches)

### Why RDMA?
1. **Kernel bypass**: Direct NIC access, no syscalls
2. **Zero-copy**: DMA directly to/from VM memory
3. **Sub-microsecond latency**: <1µs for small transfers
4. **Hardware offload**: RDMA READ/WRITE in NIC firmware

---

## Consumer Hardware Reality Check

### Typical Consumer Setup
```
Hardware:
- CPU: AMD Ryzen / Intel Core (✅ Good enough)
- RAM: 16-64GB (✅ Sufficient)
- Network: 1-10 Gbps Ethernet (⚠️ Problem area)
- NIC: Realtek/Intel consumer NICs (❌ No RDMA)
- Cost: $1000-2000 total
```

### The Gap
| Metric | RDMA (Target) | Consumer Ethernet | Gap |
|--------|---------------|-------------------|-----|
| **Latency** | <1µs | 50-200µs | 50-200× slower |
| **CPU overhead** | ~1-2% | 20-40% | 10-20× higher |
| **Jitter** | Very low | High (10-100µs) | Unpredictable |
| **Bandwidth** | 100-400 Gbps | 1-10 Gbps | 10-400× slower |

---

## Feasibility Analysis by Use Case

### 1. ✅ **Development & Testing** (HIGHLY FEASIBLE)

**What works:**
- Full codebase compiles and runs
- Functional testing of all components
- End-to-end VM boot and operation
- Multi-node coordination logic
- Page migration algorithms

**What changes:**
```rust
// Use TCP/IP instead of RDMA
// In rdma-transport/src/lib.rs

#[cfg(feature = "tcp-transport")]
pub fn fetch_page(gpa: u64) -> Result<Vec<u8>> {
    // TCP socket to remote node
    let stream = TcpStream::connect(remote_addr)?;
    stream.write_all(&PageRequest { gpa }.serialize())?;
    let mut buf = vec![0u8; 4096];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}
```

**Expected performance:**
- Remote page fault: **200-500µs** (vs 100µs target)
- Throughput: **1-5 GB/s** (vs 100 GB/s target)
- CPU usage: **40-60%** per core (vs 2-5%)

**Verdict:** ✅ **Perfect for development**

---

### 2. 🟡 **Small-Scale Production** (FEASIBLE WITH CAVEATS)

**Scenario:** 2-4 nodes, light workloads, cost-sensitive deployment

**Requirements:**
- **10 Gbps Ethernet** (upgrade from 1 Gbps)
- **Low-latency NICs** (Intel X710, Mellanox ConnectX-4 Lx in Ethernet mode)
- **Kernel tuning** (interrupt affinity, CPU pinning, huge pages)
- **Network isolation** (dedicated VLAN, no traffic mixing)

**What you get:**
- Remote page fault: **100-300µs** (acceptable for many workloads)
- Throughput: **5-8 GB/s** (enough for moderate memory pressure)
- Cost: **~$500/node** (vs $2000 for full RDMA)

**Trade-offs:**
- ❌ Won't hit <100µs p50 target
- ✅ Can hit <500µs p99 with tuning
- ⚠️ Higher CPU overhead (15-25% vs 2-5%)
- ⚠️ More sensitive to network congestion

**Good for:**
- Development environments
- Testing/staging clusters
- Low-memory-pressure workloads
- Budget-constrained deployments

**Not good for:**
- High-frequency trading
- Real-time databases
- Memory-intensive HPC
- Applications sensitive to tail latency

**Verdict:** 🟡 **Usable but not optimal**

---

### 3. ❌ **High-Performance Production** (NOT FEASIBLE)

**Scenario:** Meeting the <100µs median, <500µs p99 targets

**Why consumer hardware fails:**

```
Physics of standard Ethernet:
1. Syscall overhead: ~2-5µs (kernel scheduling, context switch)
2. TCP processing: ~10-20µs (checksum, segmentation, retransmit logic)
3. NIC interrupt: ~5-10µs (IRQ handling, softirq processing)
4. Network transit: ~50-100µs (switches, cable, buffering)
5. Memory copy: ~5-10µs (userspace ↔ kernel buffers)
---
Total: ~72-145µs best case (no congestion)
```

**RDMA avoids all of this:**
```
RDMA path:
1. Userspace → NIC: Direct DMA (~0.3µs)
2. Network transit: ~0.5µs (lossless fabric)
3. NIC → Remote memory: Direct DMA (~0.3µs)
---
Total: ~1-2µs (100× better)
```

**Verdict:** ❌ **Need real RDMA hardware**

---

## Implementation Strategy: Multi-Transport Architecture

### Proposed Solution: Transport Abstraction Layer

```rust
// rdma-transport/src/lib.rs

pub enum TransportMode {
    Rdma,      // Real RDMA (ibverbs)
    Tcp,       // TCP/IP over standard Ethernet
    Udp,       // UDP with reliability layer
    Stub,      // Testing only
}

pub trait PageTransport {
    fn fetch_page(&self, gpa: u64, remote_node: u32) -> Result<Vec<u8>>;
    fn send_page(&self, gpa: u64, data: &[u8], remote_node: u32) -> Result<()>;
    fn latency_estimate(&self) -> Duration; // For migration decisions
}

impl PageTransport for RdmaTransport { /* ... */ }
impl PageTransport for TcpTransport { /* ... */ }
impl PageTransport for UdpTransport { /* ... */ }
```

### Benefits
1. **Same codebase** works on all hardware
2. **Runtime selection** via config
3. **Graceful degradation** with performance warnings
4. **Easy testing** without special hardware

### Configuration
```yaml
# config.yaml
transport:
  mode: tcp  # or rdma, udp
  backend:
    rdma:
      device: mlx5_0
      port: 1
    tcp:
      port: 50051
      nodelay: true
      buffer_size: 262144
  
performance:
  target_latency_us: 500  # Adjust based on transport
  migration_threshold: 0.1  # More aggressive on slow transports
```

---

## Consumer Hardware Options (Best → Worst)

### Tier 1: Prosumer RoCE (Best consumer option)
**Hardware:**
- Mellanox ConnectX-4 Lx (~$150 used)
- 10/25 Gbps Ethernet
- RoCE v2 capable (RDMA over Ethernet)

**Performance:**
- Latency: **50-150µs** (3-5× slower than InfiniBand)
- Bandwidth: **10-25 Gbps**
- CPU: **5-10%** overhead

**Cost:** ~$300-500/node (NIC + switch)

**Verdict:** ✅ **Good compromise**

---

### Tier 2: High-end Ethernet (Acceptable)
**Hardware:**
- Intel X710 or E810 (~$100-200)
- 10 Gbps Ethernet
- Low-latency mode, SR-IOV

**Performance:**
- Latency: **100-300µs** (tuned kernel)
- Bandwidth: **8-10 Gbps**
- CPU: **15-25%** overhead

**Cost:** ~$200-400/node

**Verdict:** 🟡 **Workable for many use cases**

---

### Tier 3: Standard Ethernet (Development only)
**Hardware:**
- Intel i350, Realtek 8111
- 1 Gbps Ethernet

**Performance:**
- Latency: **500-2000µs** (highly variable)
- Bandwidth: **1 Gbps**
- CPU: **40-60%** overhead

**Cost:** ~$50/node (or built-in)

**Verdict:** ⚠️ **Development/testing only**

---

## Recommended Approach

### Phase 1: Multi-Transport Support (2-3 days)
```rust
// Add to rdma-transport/Cargo.toml
[features]
default = ["rdma-transport"]
rdma-transport = []
tcp-transport = ["tokio", "serde"]
udp-transport = ["tokio", "quinn"]

// Implement TcpPageTransport
```

**Benefits:**
- Works on ANY hardware
- Easy local testing
- Cloud deployment (no special NICs needed)

---

### Phase 2: Performance Tiers (Document)
```markdown
Hardware Tier Guide:

Production (Meet <100µs target):
- InfiniBand: Mellanox ConnectX-5+ ($500-1000)
- Switch: NVIDIA QM8700 (~$5000)
- Total: ~$1500/node + shared switch

Prosumer (Meet <300µs target):  
- RoCE v2: Mellanox ConnectX-4 Lx ($150)
- Switch: 10G capable with PFC (~$500)
- Total: ~$400/node + shared switch

Development (No target):
- Standard Ethernet: Built-in NICs
- Switch: Any gigabit switch
- Total: $0 additional cost
```

---

### Phase 3: Dynamic Adaptation
```rust
// Automatically adjust policies based on measured latency

impl TransportManager {
    pub fn calibrate(&mut self) -> Result<()> {
        let measured = self.measure_round_trip_latency()?;
        
        if measured < Duration::from_micros(150) {
            self.policy = AggressiveMigration;  // RDMA-like
        } else if measured < Duration::from_micros(500) {
            self.policy = ModerateMigration;    // TCP tuned
        } else {
            self.policy = ConservativeMigration;  // Slow network
        }
        
        Ok(())
    }
}
```

---

## Cost Breakdown

### RDMA Production Cluster (2 nodes)
```
2× Mellanox ConnectX-5: $1000
1× InfiniBand Switch: $5000 (can be shared)
Cables: $200
---
Total: $6,200 (or $1,100/node if switch shared)
```

### RoCE Prosumer Cluster (2 nodes)
```
2× Mellanox ConnectX-4 Lx: $300
1× 10G Ethernet Switch: $500
Cables: $100
---
Total: $900 ($450/node)
```

### Standard Ethernet (2 nodes)
```
Built-in NICs: $0
1G Switch: $50
Cables: $20
---
Total: $70
```

---

## Performance Predictions

### Workload Impact Model

| Workload Type | RDMA | RoCE | 10G Eth | 1G Eth |
|---------------|------|------|---------|--------|
| **Memory streaming** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐ |
| **Random access (hot)** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Random access (cold)** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ |
| **Read-heavy** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Write-heavy** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ |
| **Development/Testing** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

### Expected Remote Miss Penalties

| Transport | Median Latency | P99 Latency | Impact |
|-----------|----------------|-------------|--------|
| **InfiniBand** | 50-100µs | 200-500µs | Transparent |
| **RoCE v2** | 100-200µs | 300-800µs | Noticeable |
| **10G Ethernet (tuned)** | 200-400µs | 500-1500µs | Significant |
| **1G Ethernet** | 500-2000µs | 2-10ms | Severe |

---

## Recommendations by Scenario

### Scenario A: You're building this project (Learning/Research)
**Hardware:** ✅ **Use what you have** (standard Ethernet)
**Approach:** 
- Implement TCP transport first
- Measure baseline performance
- Add RDMA when needed
- Focus on correctness, not speed

**Estimated Dev Time:** +2-3 days for TCP transport

---

### Scenario B: You want to demo this (Show it works)
**Hardware:** 🟡 **Prosumer RoCE** (~$500 total)
**Approach:**
- Buy 2× used ConnectX-4 Lx
- Use 10G switch you might have
- Achieves "fast enough" latency
- Impressive demos

**Cost:** $500-800

---

### Scenario C: You're deploying this (Production)
**Hardware:** ⚠️ **Depends on workload**
**Decision tree:**
```
Memory pressure > 50%? → InfiniBand required
Latency SLA < 200µs? → InfiniBand required  
Budget < $500/node? → RoCE acceptable
Budget < $200/node? → 10G Ethernet (relaxed SLAs)
Just experimenting? → Standard Ethernet fine
```

---

## Action Items

### Immediate (Next Week)
1. ✅ **Add `--features tcp-transport`** to Cargo.toml
2. ✅ **Implement `TcpPageTransport`** (~200 lines)
3. ✅ **Add transport selection** to config
4. ✅ **Document performance expectations**

### Short-term (Next Month)
5. 🔄 **Benchmark on consumer hardware**
6. 🔄 **Add RoCE support** (RDMA over Ethernet)
7. 🔄 **Implement adaptive policies**

### Long-term (Future)
8. ⏳ **Cloud provider support** (AWS EFA, Azure InfiniBand)
9. ⏳ **Zero-copy optimizations** (io_uring, AF_XDP)
10. ⏳ **Hybrid transport** (RDMA for hot pages, TCP for cold)

---

## Bottom Line

### ✅ YES if:
- You're developing/testing
- You accept 3-10× higher latency
- You tune the kernel aggressively  
- You use 10G+ Ethernet
- Your workload tolerates >100µs faults

### ❌ NO if:
- You need <100µs p50 latency
- You have memory-intensive workloads
- You need predictable tail latency
- You're running production services with SLAs

### 🎯 Best Path Forward:
**Implement multi-transport architecture** (2-3 days work) that:
1. Works on any hardware (TCP fallback)
2. Optimizes when RDMA available
3. Warns user about performance tier
4. Adjusts policies dynamically

This way, **anyone can run SSI-HV**, but they get the **performance their hardware supports**.

---

## Technical Effort Estimate

### Adding TCP Transport
```
Files to modify:
- rdma-transport/src/transport/tcp.rs (NEW, ~300 lines)
- rdma-transport/src/lib.rs (add trait abstraction, ~100 lines)
- rdma-transport/Cargo.toml (add features, ~10 lines)
- coordinator/config.yaml (transport selection, ~20 lines)

Estimated time: 2-3 days
Testing time: 1 day
Documentation: 0.5 days

Total: ~4 days
```

**ROI:** Makes project accessible to 100× more users.

---

## Conclusion

**Consumer hardware is FEASIBLE** for SSI-HV, but requires:
1. ✅ Multi-transport abstraction (4 days work)
2. ✅ Adjusted performance expectations
3. ✅ Clear documentation of trade-offs

**You can develop on a laptop, deploy on prosumer hardware, and scale to RDMA when needed.**

The key is **NOT** to force everyone to buy $2000 NICs, but to **support multiple transport tiers** and let users choose their price/performance point.

**Next step:** Should we implement the TCP transport layer? This would make the system universally deployable.
