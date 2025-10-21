#!/bin/bash
# Start VMM processes on both nodes and register transport endpoints

set -e

# Configuration
NODE_ACCESS="access"
NODE_MO="mo"
PROJECT_DIR="/home/toor/ssi-hv-starter"
COORDINATOR_URL="http://100.86.226.54:8001"
SUDO_PASSWORD="toor"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🚀 Starting VMMs on cluster nodes"
echo "=================================="
echo ""

# Step 0: Sync updated Cargo.toml to remote nodes
echo "Step 0: Syncing code changes to remote nodes"
echo "============================================="
scp "${PROJECT_DIR}/pager/Cargo.toml" "${NODE_ACCESS}:${PROJECT_DIR}/pager/"
scp "${PROJECT_DIR}/pager/Cargo.toml" "${NODE_MO}:${PROJECT_DIR}/pager/"
echo ""

# Helper function to run commands on remote node
run_on_node() {
    local node=$1
    local cmd=$2
    echo "[${node}] ${cmd}"
    ssh -t "${node}" "${cmd}"
}

# Helper function to run commands with sudo (auto-password)
run_on_node_sudo() {
    local node=$1
    local cmd=$2
    echo "[${node}] sudo ${cmd}"
    echo "$SUDO_PASSWORD" | ssh -t "${node}" "sudo -S ${cmd}"
}

# Helper function to wait for endpoint registration
wait_for_endpoint() {
    local node_id=$1
    local max_attempts=15
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "${COORDINATOR_URL}/nodes/${node_id}/endpoint" >/dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} Node ${node_id} endpoint registered"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done
    
    echo -e "${RED}✗${NC} Node ${node_id} endpoint registration timeout"
    return 1
}

# Step 1: Build pager example on both nodes
echo "Step 1: Building pager example on both nodes (clean build)"
echo "==========================================================="
echo "Building on ${NODE_ACCESS}..."
run_on_node "${NODE_ACCESS}" "cd ${PROJECT_DIR} && source ~/.cargo/env && rm -rf target/release/.fingerprint/pager-* target/release/examples/pager_node && cargo build --release --example pager_node"
echo ""

echo "Building on ${NODE_MO}..."
run_on_node "${NODE_MO}" "cd ${PROJECT_DIR} && source ~/.cargo/env && rm -rf target/release/.fingerprint/pager-* target/release/examples/pager_node && cargo build --release --example pager_node"
echo ""

echo "Step 1.5: Enabling userfaultfd on both nodes"
echo "============================================="
echo "Enabling on access..."
echo "$SUDO_PASSWORD" | ssh "$NODE_ACCESS" "sudo -S sysctl -w vm.unprivileged_userfaultfd=1"
echo ""
echo "Enabling on mo..."
echo "$SUDO_PASSWORD" | ssh "$NODE_MO" "sudo -S sysctl -w vm.unprivileged_userfaultfd=1"
echo ""

echo "Step 2: Starting pager on node 0 (access) with sudo"
echo "====================================================="
# Kill any existing pager processes first
echo "$SUDO_PASSWORD" | ssh "$NODE_ACCESS" "sudo -S pkill -9 pager_node 2>/dev/null || true"
sleep 1
# Start pager in background on access (node 0) with sudo - must run the whole nohup under sudo
ssh "$NODE_ACCESS" "cd $PROJECT_DIR && echo $SUDO_PASSWORD | sudo -S sh -c 'nohup ./target/release/examples/pager_node 0 2 $COORDINATOR_URL > pager0.log 2>&1 &'"
echo ""

# Wait a moment for startup
sleep 2

echo "Step 3: Starting pager on node 1 (mo) with sudo"
echo "================================================="
# Kill any existing pager processes first
echo "$SUDO_PASSWORD" | ssh "$NODE_MO" "sudo -S pkill -9 pager_node 2>/dev/null || true"
sleep 1
# Start pager in background on mo (node 1) with sudo - must run the whole nohup under sudo
ssh "$NODE_MO" "cd $PROJECT_DIR && echo $SUDO_PASSWORD | sudo -S sh -c 'nohup ./target/release/examples/pager_node 1 2 $COORDINATOR_URL > pager1.log 2>&1 &'"
echo ""

# Wait for both endpoints to register
sleep 3

echo ""
echo "Step 4: Verifying endpoint registration"
echo "========================================"

if wait_for_endpoint 0 && wait_for_endpoint 1; then
    echo ""
    echo -e "${GREEN}✅ Both VMMs started successfully!${NC}"
    echo ""
    
    # Display registered endpoints
    echo "Registered endpoints:"
    curl -s "${COORDINATOR_URL}/endpoints" | python3 -m json.tool
    
    echo ""
    echo "Next steps:"
    echo "  1. Monitor logs: ssh access 'tail -f ~/ssi-hv-starter/pager0.log'"
    echo "  2. Monitor logs: ssh mo 'tail -f ~/ssi-hv-starter/pager1.log'"
    echo "  3. Check processes: ssh access 'ps aux | grep pager_node'"
    echo "  4. Check processes: ssh mo 'ps aux | grep pager_node'"
    echo ""
    echo "To stop:"
    echo "  ./cleanup.sh"
else
    echo ""
    echo -e "${RED}❌ Failed to register endpoints${NC}"
    echo "Check logs:"
    echo "  ssh access 'cat ~/ssi-hv-starter/pager0.log'"
    echo "  ssh mo 'cat ~/ssi-hv-starter/pager1.log'"
    echo "Check if processes are running:"
    echo "  ssh access 'ps aux | grep pager_node'"
    echo "  ssh mo 'ps aux | grep pager_node'"
    exit 1
fi
