#!/bin/bash
# Stop VMM processes on both nodes

set -e

# Configuration
NODE_ACCESS="access"
NODE_MO="mo"
SUDO_PASSWORD="toor"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🛑 Stopping VMM processes on cluster nodes"
echo "==========================================="
echo ""

echo "Stopping pager on node 0 (access)..."
echo "$SUDO_PASSWORD" | ssh "$NODE_ACCESS" "sudo -S pkill -9 pager_node 2>/dev/null || true"
echo -e "${GREEN}✓${NC} Stopped pager on access"

echo ""
echo "Stopping pager on node 1 (mo)..."
echo "$SUDO_PASSWORD" | ssh "$NODE_MO" "sudo -S pkill -9 pager_node 2>/dev/null || true"
echo -e "${GREEN}✓${NC} Stopped pager on mo"

echo ""
echo "Verifying processes stopped..."
ACCESS_PROCS=$(ssh "$NODE_ACCESS" "ps aux | grep pager_node | grep -v grep" || echo "")
MO_PROCS=$(ssh "$NODE_MO" "ps aux | grep pager_node | grep -v grep" || echo "")

if [ -z "$ACCESS_PROCS" ] && [ -z "$MO_PROCS" ]; then
    echo -e "${GREEN}✅ All VMM processes stopped successfully${NC}"
else
    echo -e "${RED}⚠ Some processes may still be running${NC}"
    if [ -n "$ACCESS_PROCS" ]; then
        echo "  access: $ACCESS_PROCS"
    fi
    if [ -n "$MO_PROCS" ]; then
        echo "  mo: $MO_PROCS"
    fi
fi

echo ""
echo "Logs preserved at:"
echo "  access: ~/ssi-hv-starter/pager0.log"
echo "  mo: ~/ssi-hv-starter/pager1.log"
