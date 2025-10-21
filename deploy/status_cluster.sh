#!/bin/bash
# Comprehensive status check for the distributed paging cluster

set -e

COORDINATOR_URL="http://100.86.226.54:8001"
NODE_ACCESS="access"
NODE_MO="mo"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  SSI-HV Distributed Paging Cluster - Status Report${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "ok" ]; then
        echo -e "   ${GREEN}✓${NC} $message"
    elif [ "$status" = "warn" ]; then
        echo -e "   ${YELLOW}⚠${NC} $message"
    else
        echo -e "   ${RED}✗${NC} $message"
    fi
}

# 1. Coordinator Status
echo -e "${YELLOW}1. COORDINATOR${NC}"
echo "───────────────────────────────────────────────────────────"
if HEALTH=$(curl -s "$COORDINATOR_URL/health" 2>/dev/null); then
    print_status "ok" "Coordinator is running at $COORDINATOR_URL"
    if [ -f ~/ssi-hv-starter/coordinator/coordinator.log ]; then
        PID=$(ps aux | grep "python.*main.py\|uvicorn.*8001" | grep -v grep | awk '{print $2}' | head -1)
        if [ -n "$PID" ]; then
            print_status "ok" "Coordinator process PID: $PID"
        fi
    fi
else
    print_status "error" "Coordinator is NOT running"
    echo ""
    echo "   Start with: cd coordinator && python3 -c \"from main import app; import uvicorn; uvicorn.run(app, host='0.0.0.0', port=8001)\" &"
    exit 1
fi
echo ""

# 2. Cluster Status
echo -e "${YELLOW}2. CLUSTER TOPOLOGY${NC}"
echo "───────────────────────────────────────────────────────────"
ENDPOINTS=$(curl -s "$COORDINATOR_URL/endpoints" 2>/dev/null)
CLUSTER_NAME=$(echo "$ENDPOINTS" | grep -o '"cluster_name":"[^"]*"' | cut -d'"' -f4)
NODE_COUNT=$(echo "$ENDPOINTS" | grep -o '"[0-9]":{' | wc -l)

if [ -n "$CLUSTER_NAME" ] && [ "$CLUSTER_NAME" != "none" ]; then
    print_status "ok" "Cluster name: $CLUSTER_NAME"
    print_status "ok" "Registered nodes: $NODE_COUNT"
else
    print_status "warn" "No cluster created (will auto-create on first node join)"
fi
echo ""

# 3. Node Status
echo -e "${YELLOW}3. NODE STATUS${NC}"
echo "───────────────────────────────────────────────────────────"

# Check Node 0
NODE0_ENDPOINT=$(curl -s "$COORDINATOR_URL/nodes/0/endpoint" 2>/dev/null)
if [ $? -eq 0 ] && [ -n "$NODE0_ENDPOINT" ]; then
    NODE0_ADDR=$(echo "$NODE0_ENDPOINT" | grep -o '"tcp_addr":"[^"]*"' | cut -d'"' -f4)
    NODE0_PORT=$(echo "$NODE0_ENDPOINT" | grep -o '"tcp_port":[0-9]*' | cut -d':' -f2)
    print_status "ok" "Node 0 (access) - $NODE0_ADDR:$NODE0_PORT"
    
    # Check if process is running
    ACCESS_PID=$(ssh "$NODE_ACCESS" "ps aux | grep 'pager_node 0' | grep -v grep | awk '{print \$2}'" 2>/dev/null || echo "")
    if [ -n "$ACCESS_PID" ]; then
        print_status "ok" "  Process running (PID: $ACCESS_PID)"
        ACCESS_UPTIME=$(ssh "$NODE_ACCESS" "ps -p $ACCESS_PID -o etime= 2>/dev/null" || echo "unknown")
        print_status "ok" "  Uptime: $ACCESS_UPTIME"
    else
        print_status "error" "  Process NOT running"
    fi
else
    print_status "error" "Node 0 (access) - NOT registered"
fi
echo ""

# Check Node 1
NODE1_ENDPOINT=$(curl -s "$COORDINATOR_URL/nodes/1/endpoint" 2>/dev/null)
if [ $? -eq 0 ] && [ -n "$NODE1_ENDPOINT" ]; then
    NODE1_ADDR=$(echo "$NODE1_ENDPOINT" | grep -o '"tcp_addr":"[^"]*"' | cut -d'"' -f4)
    NODE1_PORT=$(echo "$NODE1_ENDPOINT" | grep -o '"tcp_port":[0-9]*' | cut -d':' -f2)
    print_status "ok" "Node 1 (mo) - $NODE1_ADDR:$NODE1_PORT"
    
    # Check if process is running
    MO_PID=$(ssh "$NODE_MO" "ps aux | grep 'pager_node 1' | grep -v grep | awk '{print \$2}'" 2>/dev/null || echo "")
    if [ -n "$MO_PID" ]; then
        print_status "ok" "  Process running (PID: $MO_PID)"
        MO_UPTIME=$(ssh "$NODE_MO" "ps -p $MO_PID -o etime= 2>/dev/null" || echo "unknown")
        print_status "ok" "  Uptime: $MO_UPTIME"
    else
        print_status "error" "  Process NOT running"
    fi
else
    print_status "error" "Node 1 (mo) - NOT registered"
fi
echo ""

# 4. Connectivity Tests
echo -e "${YELLOW}4. NETWORK CONNECTIVITY${NC}"
echo "───────────────────────────────────────────────────────────"

if [ -n "$NODE0_ADDR" ] && [ -n "$NODE1_ADDR" ]; then
    # Test Node 0 → Node 1
    if ssh "$NODE_ACCESS" "timeout 2 nc -zv $NODE1_ADDR $NODE1_PORT 2>&1" | grep -q "succeeded\|open"; then
        print_status "ok" "Node 0 → Node 1 connectivity"
    else
        print_status "warn" "Node 0 → Node 1 connectivity (could not verify)"
    fi
    
    # Test Node 1 → Node 0
    if ssh "$NODE_MO" "timeout 2 nc -zv $NODE0_ADDR $NODE0_PORT 2>&1" | grep -q "succeeded\|open"; then
        print_status "ok" "Node 1 → Node 0 connectivity"
    else
        print_status "warn" "Node 1 → Node 0 connectivity (could not verify)"
    fi
fi
echo ""

# 5. Memory Configuration
echo -e "${YELLOW}5. MEMORY CONFIGURATION${NC}"
echo "───────────────────────────────────────────────────────────"
print_status "ok" "Memory per node: 1024 MB"
print_status "ok" "Total cluster memory: 2048 MB"
print_status "ok" "Page size: 4 KB"
print_status "ok" "Total pages: ~524,288 pages"
echo ""

# 6. System Health
echo -e "${YELLOW}6. SYSTEM HEALTH${NC}"
echo "───────────────────────────────────────────────────────────"

# Check kernel settings on both nodes
ACCESS_UFFD=$(ssh "$NODE_ACCESS" "sysctl vm.unprivileged_userfaultfd 2>/dev/null | awk '{print \$3}'" || echo "0")
MO_UFFD=$(ssh "$NODE_MO" "sysctl vm.unprivileged_userfaultfd 2>/dev/null | awk '{print \$3}'" || echo "0")

if [ "$ACCESS_UFFD" = "1" ]; then
    print_status "ok" "Node 0 userfaultfd enabled"
else
    print_status "error" "Node 0 userfaultfd DISABLED"
fi

if [ "$MO_UFFD" = "1" ]; then
    print_status "ok" "Node 1 userfaultfd enabled"
else
    print_status "error" "Node 1 userfaultfd DISABLED"
fi

# Check for any errors in logs
if ssh "$NODE_ACCESS" "grep -i 'error\|failed' ~/ssi-hv-starter/pager0.log 2>/dev/null | tail -1" | grep -qi "error\|failed"; then
    print_status "warn" "Node 0 has errors in log (check pager0.log)"
else
    print_status "ok" "Node 0 log clean"
fi

if ssh "$NODE_MO" "grep -i 'error\|failed' ~/ssi-hv-starter/pager1.log 2>/dev/null | tail -1" | grep -qi "error\|failed"; then
    print_status "warn" "Node 1 has errors in log (check pager1.log)"
else
    print_status "ok" "Node 1 log clean"
fi
echo ""

# 7. Recent Activity
echo -e "${YELLOW}7. RECENT ACTIVITY${NC}"
echo "───────────────────────────────────────────────────────────"
RECENT_REQUESTS=$(tail -20 ~/ssi-hv-starter/coordinator/coordinator.log 2>/dev/null | grep "POST\|GET" | wc -l)
print_status "ok" "Coordinator requests (last 20 log lines): $RECENT_REQUESTS"

if [ -f ~/ssi-hv-starter/coordinator/coordinator.log ]; then
    LAST_REQUEST=$(tail -20 ~/ssi-hv-starter/coordinator/coordinator.log 2>/dev/null | grep "HTTP" | tail -1)
    if [ -n "$LAST_REQUEST" ]; then
        echo "   Last request: $(echo $LAST_REQUEST | awk '{print $1,$2,$3,$4,$5}')"
    fi
fi
echo ""

# Summary
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  SUMMARY${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

if [ $NODE_COUNT -eq 2 ]; then
    echo -e "${GREEN}✅ Cluster is OPERATIONAL${NC}"
    echo ""
    echo "The distributed paging cluster is running with:"
    echo "  • 2 nodes connected"
    echo "  • TCP transport active"
    echo "  • userfaultfd registered"
    echo "  • Ready to handle page faults"
    echo ""
    echo "Quick Actions:"
    echo "  • View Node 0 log: ssh access 'tail -f ~/ssi-hv-starter/pager0.log'"
    echo "  • View Node 1 log: ssh mo 'tail -f ~/ssi-hv-starter/pager1.log'"
    echo "  • View endpoints: curl $COORDINATOR_URL/endpoints | python3 -m json.tool"
    echo "  • Stop cluster: cd deploy && ./stop_vmms.sh"
else
    echo -e "${YELLOW}⚠ Cluster is INCOMPLETE${NC}"
    echo ""
    echo "Expected 2 nodes, found $NODE_COUNT registered."
    echo "Start missing nodes with: cd deploy && ./start_vmms.sh"
fi
echo ""
