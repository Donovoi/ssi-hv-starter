# SSI-HV Quick Reference Card

## 🚀 Common Commands

```bash
# Development
./dev.sh help           # Show all available commands
./dev.sh build          # Build all components
./dev.sh test           # Run all tests (Rust + Python)
./dev.sh watch          # Auto-run tests on file changes

# Testing
./dev.sh test-rust      # Rust tests only
./dev.sh test-python    # Python tests only
./dev.sh coverage       # Generate coverage report

# Code Quality
./dev.sh lint           # Run all linters
./dev.sh fix            # Auto-fix formatting issues
./dev.sh check          # Comprehensive checks

# Running
./dev.sh start          # Start coordinator API
cargo run --bin vmm     # Run VMM
cargo run --bin acpi-gen  # Generate ACPI tables

# Utilities
./dev.sh stats          # Project statistics
./dev.sh clean          # Clean build artifacts
./dev.sh doc            # Generate documentation
```

## 📊 Test Status

| Component | Tests | Status |
|-----------|-------|--------|
| acpi-gen | 7 | ✅ |
| pager | 17 | ✅ |
| rdma-transport | 13 | ✅ |
| vmm | 4 | ✅ |
| coordinator | 22 | ✅ |
| **TOTAL** | **63** | **✅** |

Pass Rate: **100%** | Coverage: **~85%**

## 📂 Project Structure

```
ssi-hv-starter/
├── vmm/                    # Virtual Machine Monitor
│   ├── src/main.rs        # KVM integration (4 tests)
│   └── src/vcpu.rs        # vCPU manager (1 test)
├── pager/                 # Distributed memory manager
│   └── src/lib.rs         # Userfaultfd pager (17 tests)
├── rdma-transport/        # RDMA communication
│   └── src/lib.rs         # Transport layer (13 tests)
├── acpi-gen/              # ACPI table generator
│   └── src/main.rs        # NUMA topology (7 tests)
├── coordinator/           # Control plane API
│   ├── main.py            # FastAPI server
│   └── test_coordinator.py  # API tests (22 tests)
├── docs/                  # Requirements & design
├── .github/workflows/     # CI/CD pipelines
└── .vscode/               # Editor config
```

## 📖 Documentation

- **README.md** - Project overview & quick start
- **DEVELOPMENT.md** - Development guide
- **STATUS.md** - Milestone tracking
- **TESTING.md** - Testing guide
- **TEST_COVERAGE.md** - Coverage report
- **TDD_SUMMARY.md** - TDD implementation
- **SESSION_COMPLETE.md** - Session summary

## 🔧 VS Code Shortcuts

- `Ctrl+Shift+B` - Build All
- `Ctrl+Shift+T` - Test All
- `Terminal > Run Task...` - Access all tasks

## 🐛 Debugging

```bash
# Rust with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --exact --nocapture

# Python with debugger
pytest test_coordinator.py::test_name -v --pdb

# Enable verbose logging
RUST_LOG=debug cargo run --bin vmm
```

## 🎯 Milestones

- ✅ M0: VMM Skeleton
- ✅ M1: Userfaultfd Pager
- 🚧 M2: RDMA Transport (20%)
- 🚧 M3: Two-Node Bring-Up (40%)
- 📋 M4: ACPI NUMA (10%)
- 📋 M5: Windows Boot (0%)
- 📋 M6: Telemetry (5%)
- 📋 M7: Hardening (0%)

## 🎓 TDD Workflow

1. **Write test first** - Define expected behavior
2. **Run test** - Should fail (red)
3. **Write code** - Minimal implementation
4. **Run test** - Should pass (green)
5. **Refactor** - Improve code quality
6. **Repeat** - For next feature

## 📈 Performance Targets

| Metric | Target | Status |
|--------|--------|--------|
| Remote fault latency (median) | <100µs | 🚧 M2 |
| Remote fault latency (p99) | <500µs | 🚧 M2 |
| Remote miss ratio | <5% | 🚧 M6 |
| RDMA bandwidth | >10 GB/s | 🚧 M2 |

## 🔗 Quick Links

- [Problem Statement](docs/01_problem_statement.md)
- [System Requirements](docs/02_system_requirements.md)
- [API Documentation](http://localhost:8000/docs) (when coordinator running)

---

**Last Updated:** October 20, 2025  
**Test Status:** ✅ 63/63 passing  
**Ready For:** M2 RDMA Implementation
