# SSI-HV Development Guide

## Project Status: M0/M1 Implementation Complete ✅

This repository now contains a working foundation for the Single-System-Image Hypervisor:

### ✅ Completed Milestones

**M0 - Local VMM Skeleton**
- KVM VM creation and management
- Guest memory allocation and mapping to KVM slots
- vCPU creation with CPUID setup
- Modular architecture with separate components

**M1 - Userfaultfd Pager**
- Complete userfaultfd registration and fault handling
- Background thread for fault service loop
- Page directory for ownership tracking
- First-touch page allocation policy
- Telemetry and statistics collection
- Integration hooks for RDMA transport

### 🚧 In Progress

**M2 - RDMA Transport** (Stub with TODOs)
- Transport manager structure ready
- Connection management framework
- Needs: Real ibverbs bindings for RDMA READ/WRITE
- Fallback: TCP can be used for testing

**M3 - Control Plane**
- FastAPI coordinator with REST endpoints
- Cluster creation/destruction
- Node join/leave operations
- Metrics exposition
- Needs: Integration with VMM processes

**M4 - ACPI NUMA** (Framework ready)
- SRAT/SLIT/HMAT generation scaffolding
- Topology configuration support
- Needs: Actual ACPI table encoding and OVMF integration

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Python dependencies
cd coordinator && pip install -e .

# Install system dependencies (Ubuntu/Debian)
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libkvm-dev \
    qemu-system-x86 \
    ovmf
```

### Building

```bash
# Build all Rust components
cargo build --release

# The workspace includes:
# - vmm: Main virtual machine monitor
# - pager: Userfaultfd-based memory pager
# - rdma-transport: RDMA communication layer
# - acpi-gen: ACPI table generator
```

### Running Components

**1. Start the Coordinator (Control Plane)**
```bash
cd coordinator
python main.py

# API available at: http://localhost:8000
# Interactive docs: http://localhost:8000/docs
```

**2. Run the VMM (Single Node)**
```bash
# Must have KVM available
./target/release/vmm

# Output will show:
# - VM creation
# - Memory registration
# - Userfaultfd initialization
# - vCPU setup
```

**3. Generate ACPI Tables**
```bash
./target/release/acpi-gen

# Generates SRAT/SLIT tables for cluster topology
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Coordinator (Python)                     │
│  FastAPI REST API for cluster management and orchestration  │
└────────────────┬────────────────────────────────────────────┘
                 │
                 │ Control Channel
                 │
    ┌────────────┴────────────┬──────────────┐
    │                         │              │
┌───▼─────┐             ┌────▼──────┐  ┌───▼──────┐
│  VMM    │◄────RDMA───►│   VMM     │  │   VMM    │
│ Node 0  │             │  Node 1   │  │  Node N  │
├─────────┤             ├───────────┤  ├──────────┤
│ Pager   │             │  Pager    │  │  Pager   │
│ (uffd)  │             │  (uffd)   │  │  (uffd)  │
└─────────┘             └───────────┘  └──────────┘
```

### Component Responsibilities

**VMM (`vmm/`)**
- KVM VM lifecycle management
- vCPU scheduling and execution
- Memory slot configuration
- Integration with pager and RDMA

**Pager (`pager/`)**
- Userfaultfd fault handling
- Page ownership directory
- Local vs remote page resolution
- Statistics collection (latency, fault rate)

**RDMA Transport (`rdma-transport/`)**
- RDMA connection management
- Page fetch/send operations
- Target: <100µs median latency, <500µs p99

**ACPI Generator (`acpi-gen/`)**
- SRAT: CPU and memory affinity
- SLIT: Inter-node latency matrix
- HMAT: Bandwidth and cache characteristics

**Coordinator (`coordinator/`)**
- Cluster formation and teardown
- Node membership management
- Address space allocation
- Metrics aggregation

## API Usage Examples

### Create a 2-Node Cluster

```bash
curl -X POST http://localhost:8000/cluster \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-cluster",
    "nodes": [
      {
        "node_id": 0,
        "hostname": "node0.local",
        "ip_address": "192.168.1.10",
        "rdma_gid": "fe80::1",
        "cpu_count": 8,
        "memory_mb": 16384,
        "status": "active"
      },
      {
        "node_id": 1,
        "hostname": "node1.local",
        "ip_address": "192.168.1.11",
        "rdma_gid": "fe80::2",
        "cpu_count": 8,
        "memory_mb": 16384,
        "status": "active"
      }
    ]
  }'
```

### Get Cluster Status

```bash
curl http://localhost:8000/cluster
```

### Get Metrics

```bash
curl http://localhost:8000/metrics
```

### Query Page Ownership

```bash
curl http://localhost:8000/pages/0x1000000
```

## Next Steps (M2-M7)

### M2: RDMA Transport Implementation
**Priority: HIGH**

1. Add RDMA dependencies:
   ```toml
   # In rdma-transport/Cargo.toml
   rdma-core-sys = "0.1"  # or ibverbs-sys
   ```

2. Implement in `rdma-transport/src/lib.rs`:
   - `ibv_open_device()` - Open RDMA NIC
   - `ibv_create_qp()` - Create RC queue pairs
   - `ibv_post_send()` - Post RDMA READ/WRITE
   - `ibv_poll_cq()` - Poll completion queue

3. Test latency:
   ```bash
   # Measure round-trip time for page fetch
   ./target/release/rdma-benchmark
   # Target: median <100µs, p99 <500µs
   ```

### M3: Two-Node Bring-Up
**Priority: HIGH**

1. Integrate coordinator with VMM
2. Implement page directory service
3. Enable remote page fault resolution via RDMA
4. Boot Linux guest spanning 2 nodes

Success criteria:
- Guest touches >90% of memory
- Remote faults serviced correctly
- No guest crashes or data corruption

### M4: ACPI NUMA Tables
**Priority: MEDIUM**

1. Complete `acpi-gen` with actual table encoding
2. Generate binary ACPI blobs
3. Integrate with OVMF firmware
4. Verify guest OS recognizes NUMA topology

Test:
```bash
# In guest Linux
numactl --hardware
cat /sys/devices/system/node/node*/meminfo
```

### M5: Windows Boot
**Priority: MEDIUM**

1. Validate ACPI compatibility with Windows
2. Ensure SRAT/SLIT meet Windows requirements
3. Test with various Windows versions

### M6: Telemetry & Placement
**Priority: MEDIUM**

1. Implement page heat tracking
2. Calculate remote miss ratio
3. Add migration policies (LRU, affinity-based)
4. Export Prometheus metrics

Target: <5% remote miss ratio after warm-up

### M7: Hardening
**Priority: LOW (Post-MVP)**

1. Add huge page support (2 MiB)
2. Implement backpressure and flow control
3. Optimize tail latency (p95, p99)
4. Add failure recovery

## Testing

### Unit Tests
```bash
cargo test --workspace
```

### Integration Tests
```bash
# Start coordinator
cd coordinator && python main.py &

# Run integration test suite
./tests/integration/test_cluster_formation.sh
```

### Benchmarks
```bash
# Pager latency benchmark
cargo bench --package pager

# RDMA latency benchmark (requires RDMA hardware)
cargo bench --package rdma-transport
```

## Performance Targets (NFR)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Remote fault latency (median) | <100 µs | `pager_stats.fault_service_time_us` |
| Remote fault latency (p99) | <500 µs | Same |
| Remote miss ratio (steady state) | <5% | `remote_faults / total_faults` |
| RDMA bandwidth | >10 GB/s | RDMA perftest |
| Page migration rate | <1% of pages/sec | `migration_count` |

## Debugging

### Enable Debug Logging
```bash
RUST_LOG=debug ./target/release/vmm
```

### Check KVM Support
```bash
# Verify KVM is available
ls -l /dev/kvm
# Should show: crw-rw----+ 1 root kvm

# Check if user is in kvm group
groups | grep kvm
```

### Monitor Userfaultfd Events
```bash
# In one terminal
./target/release/vmm

# In another terminal
sudo bpftrace -e 'tracepoint:syscalls:sys_enter_ioctl /comm == "vmm"/ { @[args->cmd] = count(); }'
```

### RDMA Diagnostics
```bash
# List RDMA devices
ibv_devices

# Check RDMA link status
ibv_devinfo

# Test RDMA performance
ib_write_bw  # On one node
ib_write_bw <remote-ip>  # On other node
```

## Troubleshooting

**Problem: "Failed to open /dev/kvm"**
```bash
sudo chmod 666 /dev/kvm
# Or add user to kvm group
sudo usermod -aG kvm $USER
newgrp kvm
```

**Problem: "Failed to register userfaultfd"**
- Check kernel version (need ≥4.3)
- Enable userfaultfd: `echo 1 | sudo tee /proc/sys/vm/unprivileged_userfaultfd`

**Problem: RDMA connection fails**
- Verify RDMA NICs are connected: `ibstatus`
- Check subnet manager is running (for InfiniBand)
- Verify network connectivity: `ping <remote-ip>`

## Contributing

See individual component READMEs for detailed contribution guidelines:
- `vmm/README.md` - VMM architecture and KVM details
- `pager/README.md` - Userfaultfd and fault handling
- `rdma-transport/README.md` - RDMA programming guide
- `acpi-gen/README.md` - ACPI table specifications

## References

- [Problem Statement](docs/01_problem_statement.md)
- [System Requirements](docs/02_system_requirements.md)
- [KVM API Documentation](https://www.kernel.org/doc/Documentation/virtual/kvm/api.txt)
- [Userfaultfd Documentation](https://docs.kernel.org/admin-guide/mm/userfaultfd.html)
- [ACPI Specification](https://uefi.org/specs/ACPI/6.5/)
- [RDMA Programming Guide](https://github.com/linux-rdma/rdma-core)

## License

Apache-2.0 (see LICENSE)
