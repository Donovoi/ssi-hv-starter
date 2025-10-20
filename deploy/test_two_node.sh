#!/bin/bash
# Two-node deployment and testing script for access and mo
# This script orchestrates the complete end-to-end test

set -e

# Configuration
NODE_ACCESS="access"  # Hostname of first node
NODE_MO="mo"          # Hostname of second node
COORDINATOR_PORT=8000
PROJECT_DIR="$HOME/ssi-hv-starter"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Detect current node
CURRENT_NODE=$(hostname)

echo "🚀 SSI-HV Two-Node Test Deployment"
echo "==================================="
echo ""
echo "Current node: $CURRENT_NODE"
echo "Node 1 (coordinator): $NODE_ACCESS"
echo "Node 2 (worker): $NODE_MO"
echo ""
echo "NOTE: Run ./setup_cluster.sh first to ensure all dependencies are installed!"
echo ""

# Function to run command on remote node via SSH
run_on_node() {
    local node=$1
    shift
    echo -e "${BLUE}[$node]${NC} $@"
    ssh -o ConnectTimeout=5 "$node" "$@"
}

# Function to get Tailscale IP
get_tailscale_ip() {
    local node=$1
    if [ "$node" = "$CURRENT_NODE" ]; then
        tailscale ip -4
    else
        ssh "$node" "tailscale ip -4" 2>/dev/null || echo "unknown"
    fi
}

# Step 0: Verify connectivity
echo -e "${YELLOW}Step 0: Verifying connectivity${NC}"
echo "Checking SSH access to both nodes..."

if ! ssh -o ConnectTimeout=5 "$NODE_ACCESS" "echo 'Connection OK'" &>/dev/null; then
    echo -e "${RED}❌ Cannot connect to $NODE_ACCESS${NC}"
    exit 1
fi
echo -e "${GREEN}✓ $NODE_ACCESS reachable${NC}"

if ! ssh -o ConnectTimeout=5 "$NODE_MO" "echo 'Connection OK'" &>/dev/null; then
    echo -e "${RED}❌ Cannot connect to $NODE_MO${NC}"
    exit 1
fi
echo -e "${GREEN}✓ $NODE_MO reachable${NC}"

# Get Tailscale IPs
echo ""
echo "Tailscale IPs:"
ACCESS_IP=$(get_tailscale_ip "$NODE_ACCESS")
MO_IP=$(get_tailscale_ip "$NODE_MO")
echo "  $NODE_ACCESS: $ACCESS_IP"
echo "  $NODE_MO: $MO_IP"
echo ""

# Step 1: Cleanup both nodes
echo -e "${YELLOW}Step 1: Cleaning up existing processes${NC}"
echo "Running cleanup on $NODE_ACCESS..."
run_on_node "$NODE_ACCESS" "cd $PROJECT_DIR && bash deploy/cleanup.sh"
echo ""
echo "Running cleanup on $NODE_MO..."
run_on_node "$NODE_MO" "cd $PROJECT_DIR && bash deploy/cleanup.sh"
echo ""

# Step 2: Build on both nodes
echo -e "${YELLOW}Step 2: Building project on both nodes${NC}"
echo "Building on $NODE_ACCESS..."
run_on_node "$NODE_ACCESS" "source \$HOME/.cargo/env && cd $PROJECT_DIR && cargo build --release --workspace"
echo ""
echo "Building on $NODE_MO..."
run_on_node "$NODE_MO" "source \$HOME/.cargo/env && cd $PROJECT_DIR && cargo build --release --workspace"
echo ""

# Step 3: Start coordinator on access
echo -e "${YELLOW}Step 3: Starting coordinator on $NODE_ACCESS${NC}"
run_on_node "$NODE_ACCESS" "cd $PROJECT_DIR/coordinator && nohup ~/.local/bin/uvicorn main:app --host 0.0.0.0 --port 8000 > coordinator.log 2>&1 &"
sleep 3

# Verify coordinator is running
COORDINATOR_URL="http://${ACCESS_IP}:${COORDINATOR_PORT}"
echo "Checking coordinator at $COORDINATOR_URL/health..."
if curl -s "$COORDINATOR_URL/health" | grep -q "healthy"; then
    echo -e "${GREEN}✓ Coordinator is running${NC}"
else
    echo -e "${RED}❌ Coordinator failed to start${NC}"
    echo "Checking logs:"
    run_on_node "$NODE_ACCESS" "tail -20 $PROJECT_DIR/coordinator/coordinator.log"
    exit 1
fi
echo ""

# Step 4: Create cluster
echo -e "${YELLOW}Step 4: Creating cluster${NC}"
curl -X POST "$COORDINATOR_URL/cluster" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"test-cluster\",
        \"nodes\": [
            {
                \"node_id\": 0,
                \"hostname\": \"$NODE_ACCESS\",
                \"ip_address\": \"$ACCESS_IP\",
                \"cpu_count\": $(run_on_node "$NODE_ACCESS" "nproc"),
                \"memory_mb\": 16384,
                \"status\": \"active\"
            },
            {
                \"node_id\": 1,
                \"hostname\": \"$NODE_MO\",
                \"ip_address\": \"$MO_IP\",
                \"cpu_count\": $(run_on_node "$NODE_MO" "nproc"),
                \"memory_mb\": 16384,
                \"status\": \"active\"
            }
        ]
    }"
echo ""
echo -e "${GREEN}✓ Cluster created${NC}"
echo ""

# Step 5: Show cluster status
echo -e "${YELLOW}Step 5: Cluster Status${NC}"
curl -s "$COORDINATOR_URL/cluster" | python3 -m json.tool
echo ""

echo -e "${GREEN}✅ Two-node test environment ready!${NC}"
echo ""
echo "Next steps:"
echo "  1. Start VMM on $NODE_ACCESS (node_id=0)"
echo "  2. Start VMM on $NODE_MO (node_id=1)"
echo "  3. Monitor logs for page faults and transfers"
echo ""
echo "Coordinator API: $COORDINATOR_URL/docs"
echo "View endpoints: curl $COORDINATOR_URL/endpoints"
echo ""
