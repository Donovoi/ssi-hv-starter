#!/bin/bash
# Idempotent cluster setup script
# Sets up both access and mo nodes with identical environments

set -e

# Configuration
NODE_ACCESS="access"
NODE_MO="mo"
PROJECT_DIR="$HOME/ssi-hv-starter"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🚀 SSI-HV Cluster Setup (Idempotent)"
echo "===================================="
echo ""
echo "Setting up nodes: $NODE_ACCESS, $NODE_MO"
echo ""

# Function to setup a remote node
setup_node() {
    local node=$1
    echo -e "${YELLOW}Setting up $node...${NC}"
    
    # Copy setup script to node
    echo "  Copying setup script..."
    rsync -az "$PROJECT_DIR/deploy/setup_node.sh" "$node:/tmp/" || {
        echo -e "${RED}  ERROR: Cannot copy to $node${NC}"
        return 1
    }
    
    # Run setup script on remote node with pseudo-terminal for sudo
    echo "  Running setup script on $node (may prompt for sudo password)..."
    ssh -t "$node" "bash /tmp/setup_node.sh" || {
        echo -e "${RED}  ERROR: Setup failed on $node${NC}"
        return 1
    }
    
    echo -e "${GREEN}✓ $node setup complete${NC}"
    echo ""
}

# 1. Verify local project exists
echo -e "${YELLOW}[1/5]${NC} Verifying local project..."
if [ ! -d "$PROJECT_DIR" ]; then
    echo -e "${RED}ERROR: Project directory not found: $PROJECT_DIR${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Local project found${NC}"
echo ""

# 2. Setup access node
echo -e "${YELLOW}[2/5]${NC} Setting up $NODE_ACCESS..."
setup_node "$NODE_ACCESS" || exit 1

# 3. Setup mo node
echo -e "${YELLOW}[3/5]${NC} Setting up $NODE_MO..."
setup_node "$NODE_MO" || exit 1

# 4. Sync project to both nodes
echo -e "${YELLOW}[4/5]${NC} Syncing project files..."
echo "  Syncing to $NODE_ACCESS..."
rsync -avz --exclude 'target' --exclude '__pycache__' --exclude '*.pyc' \
    "$PROJECT_DIR/" "$NODE_ACCESS:$PROJECT_DIR/" || {
    echo -e "${RED}ERROR: Cannot sync to $NODE_ACCESS${NC}"
    exit 1
}
echo -e "${GREEN}  ✓ $NODE_ACCESS synced${NC}"

echo "  Syncing to $NODE_MO..."
rsync -avz --exclude 'target' --exclude '__pycache__' --exclude '*.pyc' \
    "$PROJECT_DIR/" "$NODE_MO:$PROJECT_DIR/" || {
    echo -e "${RED}ERROR: Cannot sync to $NODE_MO${NC}"
    exit 1
}
echo -e "${GREEN}  ✓ $NODE_MO synced${NC}"
echo ""

# 5. Verify environments
echo -e "${YELLOW}[5/5]${NC} Verifying environments..."
echo ""
echo "Access node:"
ssh "$NODE_ACCESS" 'bash -c "source ~/.cargo/env 2>/dev/null; echo \"  Rust: \$(rustc --version 2>/dev/null || echo not found)\"; echo \"  GCC: \$(gcc --version 2>/dev/null | head -1 | cut -d\" \" -f3- || echo not found)\"; echo \"  Python: \$(python3 --version 2>/dev/null | cut -d\" \" -f2 || echo not found)\"; echo \"  Tailscale: \$(tailscale ip -4 2>/dev/null || echo not connected)\""'
echo ""
echo "Mo node:"
ssh "$NODE_MO" 'bash -c "source ~/.cargo/env 2>/dev/null; echo \"  Rust: \$(rustc --version 2>/dev/null || echo not found)\"; echo \"  GCC: \$(gcc --version 2>/dev/null | head -1 | cut -d\" \" -f3- || echo not found)\"; echo \"  Python: \$(python3 --version 2>/dev/null | cut -d\" \" -f2 || echo not found)\"; echo \"  Tailscale: \$(tailscale ip -4 2>/dev/null || echo not connected)\""'

echo ""
echo "===================================="
echo -e "${GREEN}✅ Cluster setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Run: ./test_two_node.sh"
echo "  2. Or manually start coordinator on $NODE_ACCESS"
echo ""
