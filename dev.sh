#!/bin/bash
# Development helper script for SSI-HV project

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Command functions
cmd_build() {
    log_info "Building all components..."
    cargo build --workspace --release
    log_success "Build complete!"
}

cmd_test() {
    log_info "Running all tests..."
    
    log_info "Running Rust tests..."
    cargo test --workspace
    
    log_info "Running Python tests..."
    cd coordinator
    if ! command -v pytest &> /dev/null; then
        log_warning "pytest not found, installing..."
        pip install -q pytest httpx fastapi uvicorn pydantic
    fi
    pytest test_coordinator.py -v
    cd ..
    
    log_success "All tests passed!"
}

cmd_test_rust() {
    log_info "Running Rust tests only..."
    cargo test --workspace -- --nocapture
}

cmd_test_python() {
    log_info "Running Python tests only..."
    cd coordinator
    pytest test_coordinator.py -v
    cd ..
}

cmd_coverage() {
    log_info "Generating test coverage report..."
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        log_warning "cargo-tarpaulin not found, installing..."
        cargo install cargo-tarpaulin
    fi
    
    cargo tarpaulin --workspace --out Html --output-dir coverage
    log_success "Coverage report generated in coverage/index.html"
}

cmd_lint() {
    log_info "Running linters..."
    
    log_info "Checking Rust formatting..."
    cargo fmt --all -- --check
    
    log_info "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    
    log_success "Linting complete!"
}

cmd_fix() {
    log_info "Auto-fixing issues..."
    
    log_info "Formatting Rust code..."
    cargo fmt --all
    
    log_info "Applying clippy suggestions..."
    cargo clippy --all-targets --all-features --fix --allow-dirty
    
    log_success "Auto-fix complete!"
}

cmd_clean() {
    log_info "Cleaning build artifacts..."
    cargo clean
    rm -rf coverage/
    rm -rf coordinator/__pycache__
    rm -rf coordinator/.pytest_cache
    log_success "Clean complete!"
}

cmd_check() {
    log_info "Running comprehensive checks..."
    
    log_info "1. Checking compilation..."
    cargo check --workspace
    
    log_info "2. Running tests..."
    cargo test --workspace --quiet
    
    log_info "3. Checking formatting..."
    cargo fmt --all -- --check || log_warning "Code needs formatting (run: ./dev.sh fix)"
    
    log_info "4. Running clippy..."
    cargo clippy --workspace -- -D warnings || log_warning "Clippy warnings found"
    
    log_success "Check complete!"
}

cmd_start_coordinator() {
    log_info "Starting coordinator..."
    cd coordinator
    if ! command -v uvicorn &> /dev/null; then
        log_warning "uvicorn not found, installing dependencies..."
        pip install -q fastapi uvicorn pydantic
    fi
    python main.py
}

cmd_watch() {
    log_info "Starting watch mode for tests..."
    
    if ! command -v cargo-watch &> /dev/null; then
        log_warning "cargo-watch not found, installing..."
        cargo install cargo-watch
    fi
    
    cargo watch -x 'test --workspace'
}

cmd_bench() {
    log_info "Running benchmarks..."
    cargo bench --workspace
}

cmd_doc() {
    log_info "Generating documentation..."
    cargo doc --workspace --no-deps --open
}

cmd_deps() {
    log_info "Checking dependency tree..."
    cargo tree
}

cmd_update() {
    log_info "Updating dependencies..."
    cargo update
    log_success "Dependencies updated!"
}

cmd_stats() {
    log_info "Project statistics:"
    echo ""
    
    echo "Lines of code (Rust):"
    find . -name "*.rs" -not -path "*/target/*" | xargs wc -l | tail -1
    
    echo ""
    echo "Lines of code (Python):"
    find coordinator -name "*.py" | xargs wc -l | tail -1
    
    echo ""
    echo "Test count:"
    echo "  Rust:   $(grep -r "#\[test\]" --include="*.rs" | wc -l)"
    echo "  Python: $(grep -r "def test_" coordinator/test_*.py | wc -l)"
    
    echo ""
    echo "TODO count:"
    grep -r "TODO" --include="*.rs" --include="*.py" | wc -l
    
    echo ""
    log_info "Component breakdown:"
    echo "  acpi-gen:       $(find acpi-gen/src -name "*.rs" | xargs wc -l | tail -1)"
    echo "  pager:          $(find pager/src -name "*.rs" | xargs wc -l | tail -1)"
    echo "  rdma-transport: $(find rdma-transport/src -name "*.rs" | xargs wc -l | tail -1)"
    echo "  vmm:            $(find vmm/src -name "*.rs" | xargs wc -l | tail -1)"
    echo "  coordinator:    $(find coordinator -name "*.py" | xargs wc -l | tail -1)"
}

cmd_help() {
    cat << EOF
SSI-HV Development Helper Script

Usage: ./dev.sh <command>

Build & Test:
  build         Build all components in release mode
  test          Run all tests (Rust + Python)
  test-rust     Run only Rust tests
  test-python   Run only Python tests
  coverage      Generate test coverage report
  check         Run comprehensive checks (compile, test, lint)
  bench         Run benchmarks

Code Quality:
  lint          Run all linters (fmt check, clippy)
  fix           Auto-fix formatting and clippy issues
  clean         Clean build artifacts

Development:
  watch         Watch files and auto-run tests
  start         Start the coordinator API server
  doc           Generate and open documentation

Utilities:
  deps          Show dependency tree
  update        Update all dependencies
  stats         Show project statistics
  help          Show this help message

Examples:
  ./dev.sh build          # Build everything
  ./dev.sh test           # Run all tests
  ./dev.sh watch          # Watch mode for TDD
  ./dev.sh coverage       # Generate coverage report
  ./dev.sh lint           # Check code quality
  ./dev.sh fix            # Auto-fix issues

EOF
}

# Main script
main() {
    case "${1:-help}" in
        build)
            cmd_build
            ;;
        test)
            cmd_test
            ;;
        test-rust)
            cmd_test_rust
            ;;
        test-python)
            cmd_test_python
            ;;
        coverage)
            cmd_coverage
            ;;
        lint)
            cmd_lint
            ;;
        fix)
            cmd_fix
            ;;
        clean)
            cmd_clean
            ;;
        check)
            cmd_check
            ;;
        start)
            cmd_start_coordinator
            ;;
        watch)
            cmd_watch
            ;;
        bench)
            cmd_bench
            ;;
        doc)
            cmd_doc
            ;;
        deps)
            cmd_deps
            ;;
        update)
            cmd_update
            ;;
        stats)
            cmd_stats
            ;;
        help|--help|-h)
            cmd_help
            ;;
        *)
            log_error "Unknown command: $1"
            echo ""
            cmd_help
            exit 1
            ;;
    esac
}

main "$@"
