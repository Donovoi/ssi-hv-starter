.PHONY: all build test clean run-coordinator run-vmm run-acpi-gen help check fmt

# Default target
all: build

# Build all Rust components
build:
	@echo "Building SSI-HV components..."
	cargo build --release --workspace

# Build in debug mode
build-debug:
	@echo "Building SSI-HV (debug)..."
	cargo build --workspace

# Run tests
test:
	@echo "Running unit tests..."
	cargo test --workspace

# Run tests with coverage
test-coverage:
	@echo "Running tests with coverage..."
	cargo tarpaulin --workspace --out Html --output-dir coverage

# Check code without building
check:
	@echo "Checking code..."
	cargo check --workspace

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt --all

# Lint code
lint:
	@echo "Linting code..."
	cargo clippy --workspace -- -D warnings

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf coordinator/__pycache__
	rm -rf coverage/

# Install Python dependencies
install-py:
	@echo "Installing Python coordinator dependencies..."
	cd coordinator && pip install -e .

# Run coordinator
run-coordinator: install-py
	@echo "Starting SSI-HV Coordinator..."
	@echo "API docs: http://localhost:8000/docs"
	cd coordinator && python main.py

# Run VMM (requires KVM)
run-vmm: build
	@echo "Starting SSI-HV VMM..."
	@echo "Note: Requires /dev/kvm access"
	RUST_LOG=info ./target/release/vmm

# Run ACPI generator
run-acpi-gen: build
	@echo "Generating ACPI tables..."
	RUST_LOG=info ./target/release/acpi-gen

# Run integration tests
integration-test:
	@echo "Running integration tests..."
	@chmod +x tests/integration/test_cluster.sh
	@./tests/integration/test_cluster.sh

# Check KVM availability
check-kvm:
	@echo "Checking KVM availability..."
	@if [ -e /dev/kvm ]; then \
		echo "✓ /dev/kvm is available"; \
		ls -l /dev/kvm; \
	else \
		echo "✗ /dev/kvm not found"; \
		echo "  Run: sudo modprobe kvm_intel  (or kvm_amd)"; \
		exit 1; \
	fi

# Check RDMA devices
check-rdma:
	@echo "Checking RDMA devices..."
	@if command -v ibv_devices >/dev/null 2>&1; then \
		ibv_devices; \
	else \
		echo "RDMA tools not installed"; \
		echo "  Ubuntu/Debian: sudo apt-get install rdma-core libibverbs-dev"; \
		echo "  RHEL/CentOS: sudo yum install rdma-core-devel"; \
	fi

# Development environment setup
dev-setup:
	@echo "Setting up development environment..."
	@echo "1. Installing Rust components..."
	rustup component add rustfmt clippy
	@echo "2. Installing Python dependencies..."
	$(MAKE) install-py
	@echo "3. Checking KVM..."
	$(MAKE) check-kvm || true
	@echo "4. Development setup complete!"

# Run all pre-commit checks
pre-commit: fmt lint test
	@echo "✓ All pre-commit checks passed!"

# Show help
help:
	@echo "SSI-HV Makefile targets:"
	@echo ""
	@echo "Build & Development:"
	@echo "  make build           - Build all components (release)"
	@echo "  make build-debug     - Build all components (debug)"
	@echo "  make test            - Run unit tests"
	@echo "  make check           - Check code without building"
	@echo "  make fmt             - Format code"
	@echo "  make lint            - Run clippy linter"
	@echo "  make clean           - Clean build artifacts"
	@echo ""
	@echo "Running Components:"
	@echo "  make run-coordinator - Start control plane (port 8000)"
	@echo "  make run-vmm         - Start VMM (requires KVM)"
	@echo "  make run-acpi-gen    - Generate ACPI tables"
	@echo ""
	@echo "Testing:"
	@echo "  make integration-test - Run integration tests"
	@echo "  make check-kvm        - Verify KVM availability"
	@echo "  make check-rdma       - Check RDMA devices"
	@echo ""
	@echo "Setup:"
	@echo "  make dev-setup       - Setup development environment"
	@echo "  make install-py      - Install Python dependencies"
	@echo "  make pre-commit      - Run all pre-commit checks"
	@echo ""
	@echo "Current milestone: M0/M1 complete"
	@echo "Next: M2 (RDMA implementation)"
