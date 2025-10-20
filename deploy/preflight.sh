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
if ssh -o ConnectTimeout=5 access "echo ok" &>/dev/null; then
    check_pass "Can connect to 'access'"
else
    check_fail "Cannot connect to 'access' - check SSH config or hostname"
fi

if ssh -o ConnectTimeout=5 mo "echo ok" &>/dev/null; then
    check_pass "Can connect to 'mo'"
else
    check_fail "Cannot connect to 'mo' - check SSH config or hostname"
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
if ssh access "tailscale status" &>/dev/null; then
    check_pass "Tailscale running on 'access'"
    ACCESS_IP=$(ssh access "tailscale ip -4")
    echo "  Access Tailscale IP: $ACCESS_IP"
else
    check_fail "Tailscale not running on 'access'"
fi

if ssh mo "tailscale status" &>/dev/null; then
    check_pass "Tailscale running on 'mo'"
    MO_IP=$(ssh mo "tailscale ip -4")
    echo "  Mo Tailscale IP: $MO_IP"
else
    check_fail "Tailscale not running on 'mo'"
fi

# Check 3: Project directory
echo ""
echo "Checking project directory..."
PROJECT_DIR="$HOME/ssi-hv-starter"

if ssh access "test -d $PROJECT_DIR"; then
    check_pass "Project exists on 'access': $PROJECT_DIR"
else
    check_fail "Project not found on 'access': $PROJECT_DIR"
fi

if ssh mo "test -d $PROJECT_DIR"; then
    check_pass "Project exists on 'mo': $PROJECT_DIR"
else
    check_fail "Project not found on 'mo': $PROJECT_DIR"
fi

# Check 4: Rust toolchain
echo ""
echo "Checking Rust toolchain..."
ACCESS_RUST=$(ssh access "rustc --version 2>/dev/null || echo 'NOT FOUND'")
MO_RUST=$(ssh mo "rustc --version 2>/dev/null || echo 'NOT FOUND'")

if [[ "$ACCESS_RUST" != "NOT FOUND" ]]; then
    check_pass "Rust on 'access': $ACCESS_RUST"
else
    check_fail "Rust not found on 'access'"
fi

if [[ "$MO_RUST" != "NOT FOUND" ]]; then
    check_pass "Rust on 'mo': $MO_RUST"
else
    check_fail "Rust not found on 'mo'"
fi

# Check 5: Python
echo ""
echo "Checking Python..."
ACCESS_PYTHON=$(ssh access "python3 --version 2>/dev/null || echo 'NOT FOUND'")
MO_PYTHON=$(ssh mo "python3 --version 2>/dev/null || echo 'NOT FOUND'")

if [[ "$ACCESS_PYTHON" != "NOT FOUND" ]]; then
    check_pass "Python on 'access': $ACCESS_PYTHON"
else
    check_fail "Python3 not found on 'access'"
fi

if [[ "$MO_PYTHON" != "NOT FOUND" ]]; then
    check_pass "Python on 'mo': $MO_PYTHON"
else
    check_fail "Python3 not found on 'mo'"
fi

# Check 6: Required Python packages (on access - coordinator node)
echo ""
echo "Checking Python dependencies on 'access'..."
if ssh access "python3 -c 'import fastapi' 2>/dev/null"; then
    check_pass "FastAPI installed on 'access'"
else
    check_fail "FastAPI not installed on 'access' - run: pip install fastapi uvicorn"
fi

# Check 7: Available memory
echo ""
echo "Checking available resources..."
ACCESS_MEM=$(ssh access "free -m | grep Mem | awk '{print \$7}'")
MO_MEM=$(ssh mo "free -m | grep Mem | awk '{print \$7}'")

if [ "$ACCESS_MEM" -gt 1000 ]; then
    check_pass "Available memory on 'access': ${ACCESS_MEM}MB"
else
    check_warn "Low memory on 'access': ${ACCESS_MEM}MB"
fi

if [ "$MO_MEM" -gt 1000 ]; then
    check_pass "Available memory on 'mo': ${MO_MEM}MB"
else
    check_warn "Low memory on 'mo': ${MO_MEM}MB"
fi

# Check 8: CPU cores
ACCESS_CPU=$(ssh access "nproc")
MO_CPU=$(ssh mo "nproc")
echo "  CPUs on 'access': $ACCESS_CPU cores"
echo "  CPUs on 'mo': $MO_CPU cores"

# Check 9: Kernel version (for KVM and userfaultfd)
echo ""
echo "Checking kernel versions..."
ACCESS_KERNEL=$(ssh access "uname -r")
MO_KERNEL=$(ssh mo "uname -r")
echo "  'access': $ACCESS_KERNEL"
echo "  'mo': $MO_KERNEL"

# Check 10: KVM support
echo ""
echo "Checking KVM support..."
if ssh access "test -e /dev/kvm"; then
    check_pass "KVM device exists on 'access'"
else
    check_warn "KVM device not found on 'access' (/dev/kvm)"
fi

if ssh mo "test -e /dev/kvm"; then
    check_pass "KVM device exists on 'mo'"
else
    check_warn "KVM device not found on 'mo' (/dev/kvm)"
fi

# Check 11: No existing processes
echo ""
echo "Checking for running SSI-HV processes..."
ACCESS_PROCS=$(ssh access "pgrep -f 'coordinator|vmm' || true")
MO_PROCS=$(ssh mo "pgrep -f 'coordinator|vmm' || true")

if [ -z "$ACCESS_PROCS" ]; then
    check_pass "No SSI-HV processes on 'access'"
else
    check_warn "Found processes on 'access' - run cleanup.sh first"
    echo "  PIDs: $ACCESS_PROCS"
fi

if [ -z "$MO_PROCS" ]; then
    check_pass "No SSI-HV processes on 'mo'"
else
    check_warn "Found processes on 'mo' - run cleanup.sh first"
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
