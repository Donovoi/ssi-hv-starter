# Deployment Scripts for access and mo

Scripts for testing SSI-HV on two Ubuntu Server 25.04 nodes connected via Tailscale.

## Prerequisites

- Both nodes accessible via SSH: `access` and `mo`
- Tailscale running on both nodes
- Project cloned to `~/ssi-hv-starter` on both nodes
- Rust toolchain installed on both nodes
- Python 3.10+ installed on both nodes

## Quick Start

### 1. Cleanup Existing Processes

Run this **before every test** to ensure clean state:

```bash
# On access
ssh access "cd ~/ssi-hv-starter && bash deploy/cleanup.sh"

# On mo
ssh mo "cd ~/ssi-hv-starter && bash deploy/cleanup.sh"
```

### 2. Full Two-Node Test

Deploy coordinator, create cluster, and prepare for testing:

```bash
cd ~/ssi-hv-starter
bash deploy/test_two_node.sh
```

This will:
1. ✅ Verify SSH connectivity to both nodes
2. ✅ Get Tailscale IPs
3. ✅ Run cleanup on both nodes
4. ✅ Build project on both nodes
5. ✅ Start coordinator on access
6. ✅ Create 2-node cluster
7. ✅ Show cluster status

### 3. Quick Remote Test

Test on a single node:

```bash
# Test on access
bash deploy/remote_test.sh access

# Test on mo
bash deploy/remote_test.sh mo
```

## Manual Testing

### Start Coordinator (on access)

```bash
ssh access
cd ~/ssi-hv-starter/coordinator
python3 main.py
```

API will be available at `http://<access-tailscale-ip>:8000`

### Check Coordinator Status

```bash
# Get Tailscale IP
ssh access "tailscale ip -4"

# Replace with actual IP
curl http://<ip>:8000/health
curl http://<ip>:8000/cluster
curl http://<ip>:8000/endpoints
```

### Run Tests

```bash
# On access
ssh access "cd ~/ssi-hv-starter && cargo test --release --workspace"

# On mo
ssh mo "cd ~/ssi-hv-starter && cargo test --release --workspace"
```

### Monitor Processes

```bash
# Check what's running
ssh access "ps aux | grep -E '(coordinator|vmm|pager)'"
ssh mo "ps aux | grep -E '(coordinator|vmm|pager)'"

# Check TCP connections
ssh access "ss -tlnp | grep 500"
ssh mo "ss -tlnp | grep 500"

# Check Tailscale status
ssh access "tailscale status"
ssh mo "tailscale status"
```

## Port Usage

- **8000**: Coordinator HTTP API (on access)
- **50051-50100**: Transport layer (TCP, auto-selected)

## Cleanup Checklist

The cleanup script (`cleanup.sh`) handles:

- ✅ Coordinator processes (Python/uvicorn)
- ✅ VMM processes (Rust binaries)
- ✅ Pager processes
- ✅ Test scripts
- ✅ Stale TCP sockets (port 50051-50100)
- ✅ Stale userfaultfd file descriptors
- ✅ Verifies Tailscale is still running

## Troubleshooting

### Cannot connect to node

```bash
# Test SSH
ssh -v access
ssh -v mo

# Test Tailscale
tailscale ping access
tailscale ping mo
```

### Coordinator won't start

```bash
# Check Python
ssh access "python3 --version"

# Check dependencies
ssh access "cd ~/ssi-hv-starter/coordinator && pip list | grep fastapi"

# Check logs
ssh access "cat ~/ssi-hv-starter/coordinator/coordinator.log"
```

### Port already in use

```bash
# Find what's using the port
ssh access "sudo ss -tlnp | grep :8000"

# Run cleanup
ssh access "cd ~/ssi-hv-starter && bash deploy/cleanup.sh"
```

### Build fails

```bash
# Check Rust version
ssh access "rustc --version"
ssh mo "rustc --version"

# Clean build
ssh access "cd ~/ssi-hv-starter && cargo clean && cargo build --release"
```

## Network Topology

```
┌─────────────────┐                    ┌─────────────────┐
│     access      │◄──── Tailscale ───►│       mo        │
│   (Node 0)      │                    │   (Node 1)      │
├─────────────────┤                    ├─────────────────┤
│ Coordinator     │                    │ Worker          │
│ :8000           │                    │                 │
│                 │                    │                 │
│ Transport       │                    │ Transport       │
│ :5005x          │◄──── TCP/RDMA ────►│ :5005x          │
└─────────────────┘                    └─────────────────┘
         │                                      │
         └──────────── Page Transfers ──────────┘
```

## Safety Notes

- **Always run cleanup before testing** to avoid port conflicts
- **Cleanup preserves Tailscale** - won't kill the Tailscale daemon
- **Use CTRL+C to stop coordinator** - cleanup script handles graceful shutdown
- **Check logs if something fails** - logs are in `coordinator.log`

## Example Session

```bash
# From your local machine or either node

# 1. Cleanup
bash deploy/test_two_node.sh

# 2. Wait for setup to complete
# Coordinator will be running on access

# 3. Check cluster
ACCESS_IP=$(ssh access "tailscale ip -4")
curl http://$ACCESS_IP:8000/cluster

# 4. Run integration test
python3 examples/phase6_integration.py

# 5. Cleanup when done
ssh access "cd ~/ssi-hv-starter && bash deploy/cleanup.sh"
ssh mo "cd ~/ssi-hv-starter && bash deploy/cleanup.sh"
```
