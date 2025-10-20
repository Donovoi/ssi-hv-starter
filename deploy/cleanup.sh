#!/bin/bash
# Cleanup script for SSI-HV processes on access and mo
# Kills all existing SSI-HV related processes before new deployment

set -e

echo "🧹 SSI-HV Cleanup Script"
echo "========================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to kill processes by name pattern
kill_processes() {
    local pattern=$1
    local name=$2
    
    echo -e "${YELLOW}Checking for $name processes...${NC}"
    
    # Find PIDs matching pattern
    pids=$(pgrep -f "$pattern" 2>/dev/null || true)
    
    if [ -z "$pids" ]; then
        echo -e "${GREEN}✓ No $name processes found${NC}"
        return 0
    fi
    
    echo -e "${RED}Found $name processes: $pids${NC}"
    
    # Try graceful shutdown first (SIGTERM)
    echo "  Sending SIGTERM..."
    echo "$pids" | xargs -r kill -TERM 2>/dev/null || true
    sleep 2
    
    # Check if still running
    still_running=$(echo "$pids" | xargs -r ps -p 2>/dev/null | grep -v PID || true)
    
    if [ -n "$still_running" ]; then
        echo "  Sending SIGKILL..."
        echo "$pids" | xargs -r kill -KILL 2>/dev/null || true
        sleep 1
    fi
    
    echo -e "${GREEN}✓ $name processes killed${NC}"
}

# Kill coordinator (Python FastAPI)
kill_processes "coordinator.*main.py" "Coordinator"
kill_processes "uvicorn.*coordinator" "Coordinator (uvicorn)"

# Kill VMM processes
kill_processes "target/.*vmm" "VMM"
kill_processes "ssi-hv.*vmm" "VMM"

# Kill any Python test scripts
kill_processes "phase.*_integration.py" "Integration tests"
kill_processes "example_endpoint_exchange.py" "Example scripts"

# Kill any lingering Rust processes (pager, etc)
kill_processes "target/debug/pager" "Pager (debug)"
kill_processes "target/release/pager" "Pager (release)"

# Clean up any stale TCP sockets in our port range (50051-50100)
echo -e "${YELLOW}Checking for stale TCP sockets in port range 50051-50100...${NC}"
for port in $(seq 50051 50100); do
    if ss -tlnp | grep -q ":$port "; then
        echo -e "${RED}  Port $port is in use${NC}"
        # Find and kill process using this port
        pid=$(ss -tlnp | grep ":$port " | sed 's/.*pid=\([0-9]*\).*/\1/' | head -1)
        if [ -n "$pid" ]; then
            echo "  Killing process $pid on port $port"
            kill -9 "$pid" 2>/dev/null || true
        fi
    fi
done
echo -e "${GREEN}✓ Port range cleaned${NC}"

# Clean up any userfaultfd registrations (if any processes crashed)
echo -e "${YELLOW}Checking for stale userfaultfd file descriptors...${NC}"
stale_uffd=$(lsof -t /dev/userfaultfd 2>/dev/null || true)
if [ -n "$stale_uffd" ]; then
    echo -e "${RED}Found stale userfaultfd users: $stale_uffd${NC}"
    echo "$stale_uffd" | xargs -r kill -9 2>/dev/null || true
else
    echo -e "${GREEN}✓ No stale userfaultfd registrations${NC}"
fi

# Verify Tailscale is still running
echo -e "${YELLOW}Verifying Tailscale connection...${NC}"
if systemctl is-active --quiet tailscaled; then
    echo -e "${GREEN}✓ Tailscale is running${NC}"
    tailscale status --self | head -3
else
    echo -e "${RED}⚠️  Tailscale may not be running${NC}"
fi

echo ""
echo -e "${GREEN}✅ Cleanup complete!${NC}"
echo ""
