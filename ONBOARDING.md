# SSI-HV Developer Onboarding Guide

Welcome to the SSI-HV (Single-System-Image Hypervisor) project! This guide will help you get started quickly.

## 🎯 Project Overview

SSI-HV is a research prototype that aggregates multiple x86_64 machines into one large NUMA system presented to a guest OS via UEFI/ACPI. Think of it as creating a single massive computer from multiple physical machines.

**Current Status:** M0 and M1 complete, 63 tests passing, ready for M2 development

## 🚀 Quick Start (5 Minutes)

### 1. Prerequisites

```bash
# Check you have the essentials
rustc --version    # Need Rust stable
python3 --version  # Need Python 3.10+
lsmod | grep kvm   # Need KVM support (for running VMM)
```

### 2. Clone & Build

```bash
git clone <your-repo-url>
cd ssi-hv-starter
make build         # Or: cargo build --workspace --release
```

### 3. Run Tests

```bash
make test          # Runs all 63 tests
# Should see: 63 passed, 0 failed ✅
```

### 4. Explore

```bash
./dev.sh help      # See all available commands
make help          # See Makefile targets
```

## 📚 Essential Reading (30 Minutes)

Read these documents in order:

1. **[README.md](README.md)** (5 min)
   - Project overview and quick start
   - Current status and milestones

2. **[DEVELOPMENT.md](DEVELOPMENT.md)** (15 min)
   - Architecture overview
   - Component responsibilities
   - Development workflow

3. **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** (5 min)
   - Commands cheat sheet
   - Project structure
   - Common tasks

4. **[TESTING.md](TESTING.md)** (5 min)
   - How to run tests
   - TDD workflow
   - Adding new tests

## 🏗️ Architecture (10 Minutes)

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Coordinator (Python)                   │
│                   REST API Control Plane                    │
└───────────────────┬─────────────────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
┌───────▼──────┐        ┌──────▼────────┐
│   VMM Node 0 │        │  VMM Node 1   │
│  (Rust/KVM)  │◄──────►│  (Rust/KVM)   │
└──────┬───────┘  RDMA  └───────┬───────┘
       │                        │
   ┌───▼────┐              ┌────▼───┐
   │ Pager  │              │ Pager  │
   │(uffd)  │              │(uffd)  │
   └────────┘              └────────┘
```

**VMM (Virtual Machine Monitor)** - `vmm/`
- Creates and manages KVM virtual machines
- Allocates guest physical memory
- Creates vCPUs

**Pager (Memory Manager)** - `pager/`
- Handles page faults using userfaultfd
- Implements first-touch allocation policy
- Fetches remote pages via RDMA

**RDMA Transport** - `rdma-transport/`
- Low-latency page transfers between nodes
- Target: <100µs median latency

**ACPI Generator** - `acpi-gen/`
- Generates NUMA topology tables (SRAT, SLIT, HMAT)
- Tells guest OS about the distributed memory layout

**Coordinator** - `coordinator/`
- FastAPI REST API for cluster management
- Node join/leave orchestration
- Metrics collection

## 🔧 Development Workflow

### Your First Task: Run Everything

```bash
# 1. Build all components
make build

# 2. Run all tests (should pass!)
make test

# 3. Start the coordinator in one terminal
make run-coordinator
# Access API docs at: http://localhost:8000/docs

# 4. Check project stats
make stats
# Or: ./dev.sh stats
```

### TDD Workflow (Recommended)

We follow Test-Driven Development:

```bash
# 1. Start watch mode
./dev.sh watch

# 2. Write a test (it will fail)
# Edit: pager/src/lib.rs, add #[test] function

# 3. Watch tests auto-run and fail (RED)

# 4. Implement the feature (GREEN)

# 5. Refactor if needed

# 6. Repeat!
```

### Adding a New Test

**Rust Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_new_feature() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

**Python Example:**
```python
def test_my_new_endpoint():
    response = client.get("/new-endpoint")
    assert response.status_code == 200
    assert response.json()["key"] == "value"
```

### Running Specific Tests

```bash
# Single Rust test
cargo test test_name -- --exact

# Single component
cargo test -p pager

# Single Python test
cd coordinator
pytest test_coordinator.py::TestClass::test_method -v

# With debugging
RUST_BACKTRACE=1 cargo test test_name -- --nocapture
pytest test_coordinator.py::test_name --pdb
```

## 🎓 Learning Path

### Week 1: Understanding
- ✅ Complete this onboarding guide
- ✅ Run all tests successfully
- ✅ Read architecture docs
- ✅ Explore the codebase
- ✅ Run each component locally

### Week 2: First Contributions
- 🎯 Fix a small bug or TODO
- 🎯 Add a test for existing code
- 🎯 Improve documentation
- 🎯 Review test coverage report

### Week 3: Feature Development
- 🎯 Pick a small feature from M2 backlog
- 🎯 Write tests first (TDD)
- 🎯 Implement the feature
- 🎯 Submit a PR

## 🐛 Common Issues

### "cargo test fails with KVM error"

Some tests might require KVM. If you don't have it:
```bash
# Ubuntu/Debian
sudo apt-get install qemu-kvm
sudo usermod -aG kvm $USER
# Log out and back in
```

### "pytest not found"

```bash
cd coordinator
pip install pytest httpx fastapi uvicorn pydantic
```

### "Permission denied on /dev/kvm"

```bash
sudo chmod 666 /dev/kvm
# Or add yourself to kvm group:
sudo usermod -aG kvm $USER
```

### "RDMA tests fail"

RDMA tests use a TCP fallback, so they should pass. Real RDMA needs hardware:
```bash
# Check RDMA devices
make check-rdma
```

## 📖 Reference Documents

### For Development
- [DEVELOPMENT.md](DEVELOPMENT.md) - Complete dev guide
- [TESTING.md](TESTING.md) - Testing guide
- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Command cheat sheet

### For Understanding
- [docs/01_problem_statement.md](docs/01_problem_statement.md) - Why this exists
- [docs/02_system_requirements.md](docs/02_system_requirements.md) - What we're building
- [STATUS.md](STATUS.md) - Current progress

### For Testing
- [TEST_COVERAGE.md](TEST_COVERAGE.md) - What's tested
- [TDD_SUMMARY.md](TDD_SUMMARY.md) - How we test

## 🎯 Milestones & Roadmap

| Milestone | Status | Description |
|-----------|--------|-------------|
| M0: VMM Skeleton | ✅ Done | KVM VM management |
| M1: Userfaultfd Pager | ✅ Done | Page fault handling |
| M2: RDMA Transport | 🚧 20% | Low-latency page transfers |
| M3: Two-Node Bring-Up | 📋 Next | Cluster coordination |
| M4: ACPI NUMA | 📋 Soon | Guest topology |
| M5: Windows Boot | 📋 Later | Windows support |
| M6: Telemetry | 📋 Later | Monitoring & optimization |
| M7: Hardening | 📋 Later | Production readiness |

**Current Focus:** M2 RDMA Transport implementation

## 🤝 Contributing

### Before You Start
1. Read this onboarding guide
2. Run all tests and ensure they pass
3. Read [DEVELOPMENT.md](DEVELOPMENT.md)
4. Pick an issue or discuss with the team

### Making Changes
1. Create a feature branch: `git checkout -b feature/awesome`
2. Write tests first (TDD)
3. Implement the feature
4. Ensure all tests pass: `make test`
5. Run linters: `make lint` or `./dev.sh lint`
6. Format code: `make fmt` or `./dev.sh fix`
7. Submit a PR with clear description

### PR Checklist
- [ ] Tests added/updated
- [ ] All tests pass (`make test`)
- [ ] Code formatted (`make fmt`)
- [ ] Linters happy (`make lint`)
- [ ] Documentation updated if needed
- [ ] PR description explains what and why

## 🔗 Useful Commands Summary

```bash
# Development
./dev.sh help           # All commands
./dev.sh test           # Run all tests
./dev.sh build          # Build everything
./dev.sh watch          # TDD watch mode
./dev.sh stats          # Project stats

# Make targets
make help               # Show all targets
make test               # Run all tests
make build              # Build release
make run-coordinator    # Start API server

# Testing
cargo test --workspace  # Rust tests
pytest coordinator/     # Python tests
./dev.sh coverage       # Coverage report

# Quality
./dev.sh lint           # Check code quality
./dev.sh fix            # Auto-fix issues
```

## 🎉 You're Ready!

You now know enough to start contributing! Remember:

1. **Tests first** - We follow TDD
2. **Ask questions** - No question is too small
3. **Small PRs** - Easier to review
4. **Have fun** - This is research code!

Welcome to the team! 🚀

---

**Need Help?**
- Check documentation in this repo
- Review test files for examples
- Look at recent commits for patterns

**Test Status:** ✅ 63/63 passing (100%)  
**Ready For:** Your first contribution!
