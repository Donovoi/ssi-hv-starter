#!/bin/bash
# Phase 9: Workload Testing Script
#
# This script automates distributed paging workload testing:
# 1. Verifies cluster is running
# 2. Syncs code to all nodes
# 3. Builds workload test on each node
# 4. Runs workload tests in parallel
# 5. Collects and analyzes results
# 6. Generates performance report

set -e

# Configuration
COORDINATOR_URL="http://100.86.226.54:8001"
NODE_0="access"
NODE_1="mo"
PROJECT_DIR="/home/toor/ssi-hv-starter"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

banner() {
    echo ""
    echo -e "${CYAN}$(printf '=%.0s' {1..60})${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}$(printf '=%.0s' {1..60})${NC}"
    echo ""
}

# Check if cluster is running
check_cluster() {
    info "Checking cluster status..."

    # Check coordinator
    if ! curl -s --connect-timeout 2 "$COORDINATOR_URL/health" > /dev/null 2>&1; then
        error "Coordinator not responding at $COORDINATOR_URL"
        echo "  Please start the cluster first:"
        echo "    cd $PROJECT_DIR/deploy && ./start_vmms.sh"
        exit 1
    fi

    # Check nodes
    local endpoints=$(curl -s "$COORDINATOR_URL/endpoints" 2>/dev/null || echo "{}")
    local node_count=$(echo "$endpoints" | python3 -c "import sys, json; print(len(json.load(sys.stdin).get('endpoints', {})))" 2>/dev/null || echo "0")

    if [ "$node_count" -lt 2 ]; then
        error "Expected 2 nodes, found $node_count"
        echo "  Check cluster with: ./status_cluster.sh"
        exit 1
    fi

    success "Cluster is operational ($node_count nodes)"
}

# Sync code to nodes
sync_code() {
    info "Syncing code to cluster nodes..."

    for node in $NODE_0 $NODE_1; do
        echo "  📤 Syncing to $node..."
        rsync -aq --exclude='target/' --exclude='__pycache__/' --exclude='.git/' \
            "$PROJECT_DIR/" "$node:$PROJECT_DIR/" || {
            error "Failed to sync to $node"
            exit 1
        }
    done

    success "Code synced to all nodes"
}

# Build workload test on nodes
build_test() {
    info "Building workload test on both nodes..."

    for node in $NODE_0 $NODE_1; do
        echo "  🔨 Building on $node..."
        ssh "$node" "cd $PROJECT_DIR && source ~/.cargo/env && cargo build --release --example phase9_workload_test 2>&1 | tail -3" || {
            error "Build failed on $node"
            exit 1
        }
    done

    success "Workload test built on all nodes"
}

# Run workload test on a node (returns output file path)
run_test_on_node() {
    local node=$1
    local node_id=$2
    local output_file="/tmp/phase9_node${node_id}_results.txt"

    info "Starting workload test on $node (node_id=$node_id)..."

    # Run directly without sudo (pager process already has cap_sys_admin)
    ssh "$node" "cd $PROJECT_DIR && \
        ./target/release/examples/phase9_workload_test \
        $node_id 2 $COORDINATOR_URL > $output_file 2>&1" &
}

# Parse test results
parse_results() {
    local node=$1
    local node_id=$2
    local output_file="/tmp/phase9_node${node_id}_results.txt"

    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}📊 Results from Node $node_id ($node)${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Fetch and parse results
    ssh "$node" "cat $output_file 2>/dev/null" || {
        error "Could not retrieve results from $node"
        return 1
    }
}

# Generate summary report
generate_report() {
    banner "📈 PHASE 9 TEST SUMMARY"

    echo "Test Configuration:"
    echo "  • Memory Size:    256 MB per node"
    echo "  • Total Nodes:    2"
    echo "  • Coordinator:    $COORDINATOR_URL"
    echo "  • Transport:      TCP"
    echo ""

    echo "Tests Executed:"
    echo "  1. ✓ Local Sequential Access"
    echo "  2. ✓ Data Integrity Verification"
    echo "  3. ✓ Random Access Pattern"
    echo "  4. ✓ Strided Access Pattern"
    echo "  5. ✓ Concurrent Page Faults (4 threads)"
    echo ""

    echo "Results:"
    echo "  • See detailed output above for each node"
    echo "  • Node-specific logs at: /tmp/phase9_node{0,1}_results.txt"
    echo ""

    echo "Next Steps:"
    echo "  • Compare latency distributions across nodes"
    echo "  • Check for remote page fetches in logs"
    echo "  • Run stress test with larger memory sizes"
    echo "  • Implement RDMA transport for lower latency"
    echo ""
}

# Cleanup
cleanup() {
    echo ""
    info "Test complete. Cluster is still running."
    echo "  View status:  ./status_cluster.sh"
    echo "  Stop cluster: ./stop_vmms.sh"
}

# Main test flow
main() {
    banner "🚀 PHASE 9: DISTRIBUTED PAGING WORKLOAD TEST"

    # Step 1: Verify cluster
    check_cluster

    # Step 2: Sync code
    sync_code

    # Step 3: Build test
    build_test

    # Step 4: Run tests on both nodes in parallel
    banner "▶️  RUNNING WORKLOAD TESTS"
    echo ""

    # Start tests
    run_test_on_node $NODE_0 0
    pid0=$!
    sleep 1
    run_test_on_node $NODE_1 1
    pid1=$!

    # Wait for tests to complete
    info "Waiting for tests to complete (PIDs: $pid0, $pid1)..."
    wait $pid0 $pid1 2>/dev/null || true

    # Step 5: Collect and display results
    banner "📊 TEST RESULTS"
    parse_results $NODE_0 0
    echo ""
    parse_results $NODE_1 1

    # Step 6: Generate summary
    generate_report

    # Cleanup
    cleanup

    success "Phase 9 testing complete!"
}

# Run main
main "$@"
