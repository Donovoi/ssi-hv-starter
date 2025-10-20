# SSI-HV (Single‑System‑Image Hypervisor)

[![Tests](https://github.com/Donovoi/ssi-hv-starter/workflows/Tests/badge.svg)](https://github.com/Donovoi/ssi-hv-starter/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

A research prototype that **aggregates multiple x86_64 machines into one large NUMA system** presented to a guest OS via UEFI/ACPI.

## 🎯 Project Status

**Milestones Complete:** M0 (VMM Skeleton) ✅ | M1 (Userfaultfd Pager) ✅  
**Current Phase:** M2 (RDMA Transport) 🚧  
**Test Coverage:** 63 tests, 100% pass rate, ~85% coverage

| Component | Status | Tests | Coverage |
|-----------|--------|-------|----------|
| VMM | ✅ Complete | 4 | ~70% |
| Pager | ✅ Complete | 17 | ~90% |
| RDMA Transport | 🚧 Framework | 13 | ~85% |
| ACPI Generator | 🚧 Framework | 7 | ~80% |
| Coordinator | ✅ Complete | 22 | ~95% |

## 🚀 Quick Start

### Prerequisites

- Rust stable toolchain (2021 edition)
- Python 3.10+
- Linux kernel 6.2+ (for KVM and userfaultfd)
- KVM support (check with `lsmod | grep kvm`)

### Build & Test

```bash
# Build all components
cargo build --workspace --release

# Run all tests
./dev.sh test

# Or use individual commands
cargo test --workspace              # Rust tests
cd coordinator && pytest -v         # Python tests
```

### Run Components

```bash
# Start the coordinator (control plane)
./dev.sh start
# API docs: http://localhost:8000/docs

# Run VMM (single node)
cargo run --bin vmm -- --node-id 0 --total-nodes 1

# Generate ACPI tables
cargo run --bin acpi-gen -- cluster-config.yaml
```

## 📚 Documentation

- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development guide and architecture
- **[STATUS.md](STATUS.md)** - Detailed milestone tracking
- **[TEST_COVERAGE.md](TEST_COVERAGE.md)** - Test inventory and coverage
- **[TDD_SUMMARY.md](TDD_SUMMARY.md)** - TDD methodology and implementation
- **[TESTING.md](TESTING.md)** - Quick testing reference
- **[docs/01_problem_statement.md](docs/01_problem_statement.md)** - Problem statement & scope
- **[docs/02_system_requirements.md](docs/02_system_requirements.md)** - Requirements & milestones

## 🏗️ Architecture

### Components

**VMM (Virtual Machine Monitor)**
- KVM integration for VM lifecycle management
- Guest physical memory allocation and mapping
- vCPU creation and management
- Located in `vmm/`

**Pager (Distributed Memory Manager)**
- Userfaultfd-based fault handling
- First-touch page allocation policy
- Page directory for ownership tracking
- Statistics collection (latency, fault rate)
- Located in `pager/`

**RDMA Transport**
- Connection management framework
- Page fetch/send API
- Ready for ibverbs integration
- Located in `rdma-transport/`

**ACPI Generator**
- NUMA topology table generation (SRAT, SLIT, HMAT)
- Cluster configuration support
- Located in `acpi-gen/`

**Coordinator (Control Plane)**
- FastAPI REST API for cluster management
- Node join/leave orchestration
- Metrics exposition
- Located in `coordinator/`

## 🧪 Testing

All components have comprehensive test coverage following TDD principles:

```bash
# Run all tests
./dev.sh test

# Run specific component tests
cargo test -p pager
cargo test -p rdma-transport
pytest coordinator/test_coordinator.py

# Generate coverage report
./dev.sh coverage

# Watch mode for TDD
./dev.sh watch
```

**Test Summary:**
- ✅ 41 Rust unit tests across 4 components
- ✅ 22 Python tests for REST API
- ✅ 100% pass rate
- ✅ ~85% code coverage

## 🔧 Development

### Helper Script

Use the included `dev.sh` script for common tasks:

```bash
./dev.sh help          # Show all commands
./dev.sh build         # Build all components
./dev.sh test          # Run all tests
./dev.sh lint          # Run linters
./dev.sh fix           # Auto-fix issues
./dev.sh watch         # Watch mode for TDD
./dev.sh coverage      # Generate coverage report
./dev.sh stats         # Show project statistics
```

### VS Code Integration

The project includes `.vscode/tasks.json` for common development tasks:
- `Ctrl+Shift+B` - Build All
- `Ctrl+Shift+T` - Test All
- Access via `Terminal > Run Task...`

### Next Steps (M2-M7)

**M2: RDMA Transport** (Current)
- Implement actual RDMA operations using ibverbs
- Target: <100µs median latency, <500µs p99

**M3: Two-Node Bring-Up**
- Integrate coordinator with VMMs
- Cross-node page fault resolution
- Boot Linux guest spanning 2 nodes

**M4: ACPI NUMA**
- Binary ACPI table encoding
- OVMF firmware integration
- Guest NUMA topology recognition

**M5: Windows Boot**
- Windows-compatible ACPI tables
- VirtIO device support
- Windows guest testing

**M6: Telemetry & Placement**
- Page heat tracking
- Migration policies (LRU, affinity-based)
- Prometheus metrics

**M7: Hardening**
- Huge page support
- Failure recovery
- Performance optimization

## 📊 Performance Targets

| Metric | Target | Status |
|--------|--------|--------|
| Remote fault latency (median) | <100µs | 🚧 M2 |
| Remote fault latency (p99) | <500µs | 🚧 M2 |
| Remote miss ratio | <5% | 🚧 M6 |
| RDMA bandwidth | >10 GB/s | 🚧 M2 |

## 🤝 Contributing

This is a research prototype. Contributions welcome!

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Follow TDD principles (write tests first)
4. Ensure all tests pass (`./dev.sh test`)
5. Run linters (`./dev.sh lint`)
6. Submit a pull request

## 📝 License

Apache-2.0 (see [LICENSE](LICENSE))

## 🔗 References

- [Linux RDMA Programming](https://github.com/linux-rdma/rdma-core)
- [KVM API Documentation](https://www.kernel.org/doc/Documentation/virtual/kvm/api.txt)
- [ACPI 6.5 Specification](https://uefi.org/specs/ACPI/6.5/)
- [Userfaultfd Guide](https://docs.kernel.org/admin-guide/mm/userfaultfd.html)
