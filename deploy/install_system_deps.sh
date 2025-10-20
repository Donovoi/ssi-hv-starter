#!/bin/bash
# Install system dependencies on Ubuntu nodes
# This script requires sudo and should be run directly on each node

set -e

echo "🔧 Installing SSI-HV System Dependencies"
echo "========================================="
echo ""

if [ "$EUID" -ne 0 ]; then 
    echo "This script requires sudo. Running with sudo..."
    exec sudo bash "$0" "$@"
fi

echo "Updating package list..."
apt-get update -qq

echo "Installing build-essential (gcc, g++, make)..."
apt-get install -y build-essential

echo "Installing pkg-config..."
apt-get install -y pkg-config

echo "Installing libssl-dev..."
apt-get install -y libssl-dev

echo "Installing Python 3 and pip (if not present)..."
apt-get install -y python3 python3-pip

echo ""
echo "✅ System dependencies installed!"
echo ""
echo "Installed packages:"
echo "  - gcc: $(gcc --version | head -1 | cut -d' ' -f3-)"
echo "  - pkg-config: $(pkg-config --version)"
echo "  - OpenSSL dev: $(pkg-config --modversion openssl)"
echo "  - Python: $(python3 --version | cut -d' ' -f2)"
echo ""
echo "Next: Run ./setup_node.sh as regular user to install Rust and Python packages"
echo ""
