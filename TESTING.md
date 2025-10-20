# Quick Test Guide

## Running All Tests

```bash
# Run everything
make test

# Or manually:
cargo test --workspace && cd coordinator && pytest test_coordinator.py -v
```

## Rust Tests

### Run All Rust Tests
```bash
cargo test --workspace
```

### Run Specific Component
```bash
cargo test -p acpi-gen
cargo test -p pager
cargo test -p rdma-transport
cargo test -p vmm
```

### Run Single Test
```bash
cargo test test_page_directory_claim
cargo test test_pager_stats_median_latency
```

### Run with Output
```bash
cargo test --workspace -- --nocapture
```

### Run with Specific Threads
```bash
cargo test --workspace -- --test-threads=1
```

### Show Test Output
```bash
cargo test --workspace -- --show-output
```

## Python Tests

### Run All Python Tests
```bash
cd coordinator
pytest test_coordinator.py -v
```

### Run Specific Test Class
```bash
pytest test_coordinator.py::TestClusterManagement -v
pytest test_coordinator.py::TestNodeManagement -v
```

### Run Single Test
```bash
pytest test_coordinator.py::TestClusterManagement::test_create_cluster -v
```

### Run with Coverage
```bash
pytest test_coordinator.py --cov=main --cov-report=html
open htmlcov/index.html
```

### Run Quietly
```bash
pytest test_coordinator.py -q
```

## Test Summary

### Quick Stats
```bash
# Count passing tests
cargo test --workspace 2>&1 | grep "test result:"
cd coordinator && pytest test_coordinator.py -q 2>&1 | tail -1
```

### Component Breakdown
```bash
# acpi-gen
cargo test -p acpi-gen -- --format=terse

# pager
cargo test -p pager -- --format=terse

# rdma-transport
cargo test -p rdma-transport -- --format=terse

# vmm
cargo test -p vmm -- --format=terse
```

## Test Development

### Watch Mode (Auto-rerun on changes)
```bash
# Install cargo-watch
cargo install cargo-watch

# Watch Rust tests
cargo watch -x 'test --workspace'

# Watch specific component
cargo watch -x 'test -p pager'

# Watch Python tests
cd coordinator
pytest-watch test_coordinator.py
```

### Add New Test (Rust)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        // Arrange
        let input = setup();
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Add New Test (Python)
```python
class TestMyFeature:
    def test_basic_functionality(self):
        # Arrange
        setup()
        
        # Act
        result = my_function()
        
        # Assert
        assert result == expected
```

## Debugging Tests

### Run Single Test with Backtrace
```bash
RUST_BACKTRACE=1 cargo test test_name -- --exact --nocapture
```

### Python Test with Debugger
```bash
pytest test_coordinator.py::test_name -v --pdb
```

### Show Test Names Only
```bash
cargo test --workspace -- --list
pytest test_coordinator.py --collect-only
```

## CI/CD Integration

### GitHub Actions (example)
```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --workspace
      - run: cd coordinator && pip install -r requirements.txt && pytest
```

## Test Results

### Current Status
- **Total Tests:** 63
- **Passing:** 63 (100%)
- **Failing:** 0
- **Coverage:** ~85%

### Component Breakdown
| Component | Tests | Status |
|-----------|-------|--------|
| acpi-gen | 7 | ✅ |
| pager | 17 | ✅ |
| rdma-transport | 13 | ✅ |
| vmm | 4 | ✅ |
| coordinator | 22 | ✅ |

## Troubleshooting

### Tests Hang
```bash
# Kill stuck processes
pkill -9 cargo
pkill -9 pytest

# Clean and rebuild
cargo clean
cargo test --workspace
```

### Dependency Issues
```bash
# Update Rust dependencies
cargo update

# Update Python dependencies
pip install --upgrade -r coordinator/requirements.txt
```

### Test Isolation Issues
```bash
# Run tests sequentially
cargo test --workspace -- --test-threads=1
pytest test_coordinator.py -v -x
```

## Performance Testing (Future)

### Benchmark Tests
```bash
cargo bench --workspace
```

### Load Testing
```bash
# Install hey
go install github.com/rakyll/hey@latest

# Load test coordinator
hey -n 1000 -c 10 http://localhost:8000/health
```

## Documentation

- [TEST_COVERAGE.md](TEST_COVERAGE.md) - Detailed coverage report
- [TDD_SUMMARY.md](TDD_SUMMARY.md) - TDD methodology and implementation
- [STATUS.md](STATUS.md) - Current project status with test section
