#!/bin/bash
# Integration test for SSI-HV cluster formation
set -euo pipefail

COORDINATOR_URL="${COORDINATOR_URL:-http://localhost:8000}"
CLUSTER_NAME="test-cluster-$(date +%s)"

echo "=== SSI-HV Integration Test ==="
echo "Coordinator: $COORDINATOR_URL"
echo "Cluster: $CLUSTER_NAME"
echo

# Check health
echo "[1/6] Checking coordinator health..."
curl -sf "$COORDINATOR_URL/health" | jq .
echo "✓ Coordinator is healthy"
echo

# Create cluster
echo "[2/6] Creating cluster..."
RESPONSE=$(curl -sf -X POST "$COORDINATOR_URL/cluster" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"$CLUSTER_NAME\",
    \"nodes\": [
      {
        \"node_id\": 0,
        \"hostname\": \"node0\",
        \"ip_address\": \"192.168.1.10\",
        \"rdma_gid\": \"fe80::1\",
        \"cpu_count\": 4,
        \"memory_mb\": 8192,
        \"status\": \"active\"
      },
      {
        \"node_id\": 1,
        \"hostname\": \"node1\",
        \"ip_address\": \"192.168.1.11\",
        \"rdma_gid\": \"fe80::2\",
        \"cpu_count\": 4,
        \"memory_mb\": 8192,
        \"status\": \"active\"
      }
    ]
  }")

echo "$RESPONSE" | jq .
echo "✓ Cluster created"
echo

# Get cluster info
echo "[3/6] Getting cluster info..."
curl -sf "$COORDINATOR_URL/cluster" | jq .
echo "✓ Cluster info retrieved"
echo

# Get metrics
echo "[4/6] Getting metrics..."
curl -sf "$COORDINATOR_URL/metrics" | jq .
echo "✓ Metrics retrieved"
echo

# Query page ownership
echo "[5/6] Querying page ownership..."
curl -sf "$COORDINATOR_URL/pages/0x1000" | jq .
echo "✓ Page info retrieved"
echo

# Destroy cluster
echo "[6/6] Destroying cluster..."
curl -sf -X DELETE "$COORDINATOR_URL/cluster" | jq .
echo "✓ Cluster destroyed"
echo

echo "=== All tests passed! ==="
