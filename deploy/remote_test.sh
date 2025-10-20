#!/bin/bash
# Quick remote test from any machine
# Usage: ./deploy/remote_test.sh [access|mo]

set -e

TARGET_NODE=${1:-access}
PROJECT_DIR="$HOME/ssi-hv-starter"

echo "🧪 Running remote test on $TARGET_NODE"
echo ""

# Run cleanup
echo "1️⃣  Cleanup..."
ssh "$TARGET_NODE" "cd $PROJECT_DIR && bash deploy/cleanup.sh"

# Build
echo "2️⃣  Build..."
ssh "$TARGET_NODE" "cd $PROJECT_DIR && cargo build --release"

# Run tests
echo "3️⃣  Run tests..."
ssh "$TARGET_NODE" "cd $PROJECT_DIR && cargo test --release --workspace -- --test-threads=1"

echo ""
echo "✅ Remote test complete on $TARGET_NODE"
