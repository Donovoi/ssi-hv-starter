#!/bin/bash
# Pre-flight check before running tests on access and mo
# Verifies environment is ready for two-node testing

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

echo "🔍 SSI-HV Pre-Flight Check"
echo "=========================="
echo ""

check_pass() {
    echo -e "${GREEN}✓ $1${NC}"
}

check_fail() {
    echo -e "${RED}✗ $1${NC}"
    ERRORS=$((ERRORS + 1))
}

check_warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Check 1: SSH connectivity
echo "Checking SSH connectivity..."
if ssh -o ConnectTimeout=5 node0 "echo ok" &>/dev/null; then
    check_pass "Can connect to 'node0'"
else
    check_fail "Cannot connect to 'node0' - check SSH config or hostname"
fi

if ssh -o ConnectTimeout=5 node1 "echo ok" &>/dev/null; then
    check_pass "Can connect to 'node1'"
else
    check_fail "Cannot connect to 'node1' - check SSH config or hostname"
fi

# Check 2: Tailscale
echo ""
echo "Checking Tailscale..."
if command -v tailscale &>/dev/null; then
    check_pass "Tailscale CLI installed locally"
    if tailscale status &>/dev/null; then
        check_pass "Tailscale is running locally"
    else
        check_warn "Tailscale may not be running locally"
    fi
else
    check_warn "Tailscale CLI not found locally (may be OK if on remote node)"
fi

# Check on remote nodes
if ssh node0 "tailscale status" &>/dev/null; then
    check_pass "Tailscale running on 'node0'"
    ACCESS_IP=$(ssh node0 "tailscale ip -4")
    echo "  Access Tailscale IP: $ACCESS_IP"
else
    check_fail "Tailscale not running on 'node0'"
fi

if ssh node1 "tailscale status" &>/dev/null; then
    check_pass "Tailscale running on 'node1'"
    MO_IP=$(ssh node1 "tailscale ip -4")
    echo "  Mo Tailscale IP: $MO_IP"
else
    check_fail "Tailscale not running on 'node1'"
fi

# Check 3: Project directory
echo ""
echo "Checking project directory..."
PROJECT_DIR="$HOME/ssi-hv-starter"

if ssh node0 "test -d $PROJECT_DIR"; then
    check_pass "Project exists on 'node0': $PROJECT_DIR"
else
    check_fail "Project not found on 'node0': $PROJECT_DIR"
fi

if ssh node1 "test -d $PROJECT_DIR"; then
    check_pass "Project exists on 'node1': $PROJECT_DIR"
else
    check_fail "Project not found on 'node1': $PROJECT_DIR"
fi

# Check 4: Rust toolchain
echo ""
echo "Checking Rust toolchain..."
ACCESS_RUST=$(ssh node0 "rustc --version 2>/dev/null || echo 'NOT FOUND'")
MO_RUST=$(ssh node1 "rustc --version 2>/dev/null || echo 'NOT FOUND'")

if [[ "$ACCESS_RUST" != "NOT FOUND" ]]; then
    check_pass "Rust on 'node0': $ACCESS_RUST"
else
    check_fail "Rust not found on 'node0'"
fi

if [[ "$MO_RUST" != "NOT FOUND" ]]; then
    check_pass "Rust on 'node1': $MO_RUST"
else
    check_fail "Rust not found on 'node1'"
fi

# Check 5: Python
echo ""
echo "Checking Python..."
ACCESS_PYTHON=$(ssh node0 "python3 --version 2>/dev/null || echo 'NOT FOUND'")
MO_PYTHON=$(ssh node1 "python3 --version 2>/dev/null || echo 'NOT FOUND'")

if [[ "$ACCESS_PYTHON" != "NOT FOUND" ]]; then
    check_pass "Python on 'node0': $ACCESS_PYTHON"
else
    check_fail "Python3 not found on 'node0'"
fi

if [[ "$MO_PYTHON" != "NOT FOUND" ]]; then
    check_pass "Python on 'node1': $MO_PYTHON"
else
    check_fail "Python3 not found on 'node1'"
fi

# Check 6: Required Python packages (on access - coordinator node)
echo ""
echo "Checking Python dependencies on 'node0'..."
if ssh node0 "python3 -c 'import fastapi' 2>/dev/null"; then
    check_pass "FastAPI installed on 'node0'"
else
    check_fail "FastAPI not installed on 'node0' - run: pip install fastapi uvicorn"
fi

# Check 7: Available memory
echo ""
echo "Checking available resources..."
ACCESS_MEM=$(ssh node0 "free -m | grep Mem | awk '{print \$7}'")
MO_MEM=$(ssh node1 "free -m | grep Mem | awk '{print \$7}'")

if [ "$ACCESS_MEM" -gt 1000 ]; then
    check_pass "Available memory on 'node0': ${ACCESS_MEM}MB"
else
    check_warn "Low memory on 'node0': ${ACCESS_MEM}MB"
fi

if [ "$MO_MEM" -gt 1000 ]; then
    check_pass "Available memory on 'node1': ${MO_MEM}MB"
else
    check_warn "Low memory on 'node1': ${MO_MEM}MB"
fi

# Check 8: CPU cores
ACCESS_CPU=$(ssh node0 "nproc")
MO_CPU=$(ssh node1 "nproc")
echo "  CPUs on 'node0': $ACCESS_CPU cores"
echo "  CPUs on 'node1': $MO_CPU cores"

# Check 9: Kernel version (for KVM and userfaultfd)
echo ""
echo "Checking kernel versions..."
ACCESS_KERNEL=$(ssh node0 "uname -r")
MO_KERNEL=$(ssh node1 "uname -r")
echo "  'node0': $ACCESS_KERNEL"
echo "  'node1': $MO_KERNEL"

# Check 10: KVM support
echo ""
echo "Checking KVM support..."
if ssh node0 "test -e /dev/kvm"; then
    check_pass "KVM device exists on 'node0'"
else
    check_warn "KVM device not found on 'node0' (/dev/kvm)"
fi

if ssh node1 "test -e /dev/kvm"; then
    check_pass "KVM device exists on 'node1'"
else
    check_warn "KVM device not found on 'node1' (/dev/kvm)"
fi

# Check 11: No existing processes
echo ""
echo "Checking for running SSI-HV processes..."
ACCESS_PROCS=$(ssh node0 "pgrep -f 'coordinator|vmm' || true")
MO_PROCS=$(ssh node1 "pgrep -f 'coordinator|vmm' || true")

if [ -z "$ACCESS_PROCS" ]; then
    check_pass "No SSI-HV processes on 'node0'"
else
    check_warn "Found processes on 'node0' - run cleanup.sh first"
    echo "  PIDs: $ACCESS_PROCS"
fi

if [ -z "$MO_PROCS" ]; then
    check_pass "No SSI-HV processes on 'node1'"
else
    check_warn "Found processes on 'node1' - run cleanup.sh first"
    echo "  PIDs: $MO_PROCS"
fi

# Summary
echo ""
echo "=========================="
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ Pre-flight check PASSED${NC}"
    echo ""
    echo "Ready to test! Run:"
    echo "  bash deploy/test_two_node.sh"
    exit 0
else
    echo -e "${RED}❌ Pre-flight check FAILED with $ERRORS errors${NC}"
    echo ""
    echo "Fix the errors above before testing."
    exit 1
fi
