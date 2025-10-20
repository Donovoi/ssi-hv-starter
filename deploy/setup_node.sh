#!/bin/bash
# Idempotent node setup script
# Ensures all dependencies are installed and environment is configured identically
# Handles sudo automatically

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "🔧 SSI-HV Node Setup (Idempotent)"
echo "=================================="
echo ""

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if package is installed (Debian/Ubuntu)
package_installed() {
    dpkg -l "$1" 2>/dev/null | grep -q "^ii"
}

# Function to run command with sudo if available
run_with_sudo() {
    if command_exists sudo; then
        sudo "$@"
    else
        "$@"
    fi
}

# 1. Install build-essential (gcc, make, etc.)
echo -e "${YELLOW}[1/8]${NC} Checking build-essential..."
if ! command_exists gcc; then
    echo "  Installing build-essential..."
    run_with_sudo apt-get update -qq
    run_with_sudo apt-get install -y build-essential
    echo -e "${GREEN}  ✓ build-essential installed${NC}"
else
    echo -e "${GREEN}  ✓ gcc already installed: $(gcc --version | head -1)${NC}"
fi

# 2. Install pkg-config (required by some Rust dependencies)
echo -e "${YELLOW}[2/8]${NC} Checking pkg-config..."
if ! command_exists pkg-config; then
    echo "  Installing pkg-config..."
    run_with_sudo apt-get install -y pkg-config
    echo -e "${GREEN}  ✓ pkg-config installed${NC}"
else
    echo -e "${GREEN}  ✓ pkg-config already installed${NC}"
fi

# 3. Install libssl-dev (required by openssl-sys crate)
echo -e "${YELLOW}[3/8]${NC} Checking libssl-dev..."
if ! package_installed libssl-dev; then
    echo "  Installing libssl-dev..."
    run_with_sudo apt-get install -y libssl-dev
    echo -e "${GREEN}  ✓ libssl-dev installed${NC}"
else
    echo -e "${GREEN}  ✓ libssl-dev already installed${NC}"
fi

# 4. Install Rust (idempotent - won't reinstall if present)
echo -e "${YELLOW}[4/8]${NC} Checking Rust toolchain..."
if ! command_exists rustc; then
    echo "  Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}  ✓ Rust installed: $(rustc --version)${NC}"
else
    source "$HOME/.cargo/env" 2>/dev/null || true
    echo -e "${GREEN}  ✓ Rust already installed: $(rustc --version)${NC}"
fi

# 5. Ensure Rust is in PATH for future sessions
echo -e "${YELLOW}[5/8]${NC} Configuring Rust environment..."
if ! grep -q "source.*\.cargo/env" "$HOME/.bashrc" 2>/dev/null; then
    echo 'source "$HOME/.cargo/env"' >> "$HOME/.bashrc"
    echo -e "${GREEN}  ✓ Added Rust to .bashrc${NC}"
else
    echo -e "${GREEN}  ✓ Rust already in .bashrc${NC}"
fi

# 6. Install Python 3 and pip (usually pre-installed on Ubuntu)
echo -e "${YELLOW}[6/8]${NC} Checking Python 3..."
if ! command_exists python3; then
    echo "  Installing Python 3..."
    run_with_sudo apt-get install -y python3 python3-pip
    echo -e "${GREEN}  ✓ Python 3 installed${NC}"
else
    echo -e "${GREEN}  ✓ Python 3 already installed: $(python3 --version)${NC}"
fi

# Ensure pip is available
if ! command_exists pip && ! command_exists pip3 && ! python3 -m pip --version >/dev/null 2>&1; then
    echo "  Installing pip..."
    run_with_sudo apt-get install -y python3-pip
    echo -e "${GREEN}  ✓ pip installed${NC}"
fi

# 7. Install Python dependencies (FastAPI, uvicorn)
echo -e "${YELLOW}[7/8]${NC} Checking Python dependencies..."
if ! python3 -c "import fastapi" 2>/dev/null; then
    echo "  Installing FastAPI and uvicorn..."
    python3 -m pip install --user --break-system-packages fastapi uvicorn 2>/dev/null || \
        python3 -m pip install --user fastapi uvicorn || \
        pip install --user --break-system-packages fastapi uvicorn 2>/dev/null || \
        pip install --user fastapi uvicorn
    echo -e "${GREEN}  ✓ Python dependencies installed${NC}"
else
    echo -e "${GREEN}  ✓ FastAPI already installed${NC}"
fi

# 8. Verify Tailscale is running (don't install, just check)
echo -e "${YELLOW}[8/8]${NC} Checking Tailscale..."
if command_exists tailscale; then
    if systemctl is-active --quiet tailscaled 2>/dev/null || pgrep -x tailscaled >/dev/null; then
        TAILSCALE_IP=$(tailscale ip -4 2>/dev/null || echo "unknown")
        echo -e "${GREEN}  ✓ Tailscale running: $TAILSCALE_IP${NC}"
    else
        echo -e "${YELLOW}  ⚠ Tailscale installed but not running${NC}"
    fi
else
    echo -e "${YELLOW}  ⚠ Tailscale not installed (optional)${NC}"
fi

echo ""
echo "=================================="
echo -e "${GREEN}✅ Node setup complete!${NC}"
echo ""
echo "Environment summary:"
echo "  - GCC:        $(gcc --version | head -1 | cut -d' ' -f3-)"
echo "  - Rust:       $(rustc --version | cut -d' ' -f2-)"
echo "  - Python:     $(python3 --version | cut -d' ' -f2)"
echo "  - FastAPI:    $(python3 -c "import fastapi; print(fastapi.__version__)" 2>/dev/null || echo 'not found')"
echo "  - pkg-config: $(pkg-config --version 2>/dev/null || echo 'not found')"
echo "  - OpenSSL:    $(pkg-config --modversion openssl 2>/dev/null || echo 'not found')"
echo ""
