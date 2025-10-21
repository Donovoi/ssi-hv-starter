# SSI-HV Cluster Management - Command Reference

## Quick Start

### Deploy Cluster
```bash
# 1. Start coordinator (on local machine)
cd coordinator
python3 -c "from main import app; import uvicorn; uvicorn.run(app, host='0.0.0.0', port=8001)" &

# 2. Deploy pager nodes (access + mo)
cd ../deploy
./start_vmms.sh
```

### Check Status
```bash
cd deploy
./status_cluster.sh
```

### Stop Cluster
```bash
cd deploy
./stop_vmms.sh
```

## Available Scripts

### `deploy/start_vmms.sh`
**Purpose:** Deploy the distributed paging cluster to both nodes

**What it does:**
1. Syncs Cargo.toml to remote nodes
2. Performs clean builds with userfaultfd 0.9
3. Enables vm.unprivileged_userfaultfd=1
4. Starts pager_node processes with sudo
5. Registers endpoints with coordinator
6. Verifies endpoint registration

**Usage:**
```bash
cd deploy
./start_vmms.sh
```

### `deploy/stop_vmms.sh`
**Purpose:** Clean shutdown of all pager processes

**What it does:**
1. Kills pager_node processes on both nodes
2. Verifies processes stopped
3. Preserves logs for debugging

**Usage:**
```bash
cd deploy
./stop_vmms.sh
```

### `deploy/status_cluster.sh`
**Purpose:** Comprehensive cluster health check

**What it displays:**
- Coordinator status and PID
- Cluster topology (name, node count)
- Node status (endpoints, PIDs, uptime)
- Network connectivity tests
- Memory configuration
- System health (kernel settings, log errors)
- Recent coordinator activity

**Usage:**
```bash
cd deploy
./status_cluster.sh
```

**Example Output:**
```
═══════════════════════════════════════════════════════════
  SSI-HV Distributed Paging Cluster - Status Report
═══════════════════════════════════════════════════════════

1. COORDINATOR
───────────────────────────────────────────────────────────
   ✓ Coordinator is running at http://100.86.226.54:8001
   ✓ Coordinator process PID: 41978
...
```

### `deploy/test_distributed_paging.sh`
**Purpose:** Integration test for the distributed paging infrastructure

**What it checks:**
- Coordinator availability
- Node registration
- Process health
- TCP connectivity between nodes
- Endpoint discovery

**Usage:**
```bash
cd deploy
./test_distributed_paging.sh
```

## Coordinator API

### Base URL
```
http://100.86.226.54:8001
```

### Endpoints

#### Health Check
```bash
curl http://100.86.226.54:8001/health
```
**Response:**
```json
{"status":"healthy","cluster_active":true}
```

#### List All Endpoints
```bash
curl http://100.86.226.54:8001/endpoints
```
**Response:**
```json
{
  "cluster_name": "auto-cluster",
  "endpoints": {
    "0": {
      "transport_type": "tcp",
      "tcp_addr": "192.168.53.94",
      "tcp_port": 50051,
      ...
    },
    "1": {
      "transport_type": "tcp",
      "tcp_addr": "192.168.53.31",
      "tcp_port": 50051,
      ...
    }
  }
}
```

#### Get Specific Node Endpoint
```bash
curl http://100.86.226.54:8001/nodes/0/endpoint
curl http://100.86.226.54:8001/nodes/1/endpoint
```

#### Pretty Print (with Python)
```bash
curl -s http://100.86.226.54:8001/endpoints | python3 -m json.tool
```

## Log Access

### Node Logs

#### View Node 0 Log
```bash
ssh access 'cat ~/ssi-hv-starter/pager0.log'
ssh access 'tail -f ~/ssi-hv-starter/pager0.log'  # Follow
```

#### View Node 1 Log
```bash
ssh mo 'cat ~/ssi-hv-starter/pager1.log'
ssh mo 'tail -f ~/ssi-hv-starter/pager1.log'  # Follow
```

### Coordinator Log

#### View Coordinator Log
```bash
cat ~/ssi-hv-starter/coordinator/coordinator.log
tail -f ~/ssi-hv-starter/coordinator/coordinator.log  # Follow
```

#### View Last N Lines
```bash
tail -20 ~/ssi-hv-starter/coordinator/coordinator.log
```

#### Search for Errors
```bash
grep -i error ~/ssi-hv-starter/coordinator/coordinator.log
```

## Process Management

### Check Running Processes

#### Local Coordinator
```bash
ps aux | grep "python.*main.py\|uvicorn.*8001" | grep -v grep
```

#### Remote Nodes
```bash
ssh access "ps aux | grep pager_node | grep -v grep"
ssh mo "ps aux | grep pager_node | grep -v grep"
```

### Manual Process Management

#### Kill Coordinator
```bash
pkill -f "python.*main.py\|uvicorn.*8001"
```

#### Kill Node Processes (with sudo)
```bash
ssh access "echo toor | sudo -S pkill -9 pager_node"
ssh mo "echo toor | sudo -S pkill -9 pager_node"
```

## Troubleshooting

### Coordinator Won't Start (Port 8000 in use)
**Solution:** Use port 8001 or wait 60 seconds for TIME_WAIT to expire
```bash
# Check what's using port 8000
sudo lsof -i :8000
sudo fuser 8000/tcp

# Use port 8001 instead
cd coordinator
python3 -c "from main import app; import uvicorn; uvicorn.run(app, host='0.0.0.0', port=8001)" &
```

### Nodes Can't Connect to Coordinator
**Problem:** Wrong IP address
**Solution:** Use Tailscale IP of WSL2 instance (100.86.226.54)
```bash
# Check Tailscale status
tailscale status | grep commando

# Test connectivity from nodes
ssh access "curl http://100.86.226.54:8001/health"
ssh mo "curl http://100.86.226.54:8001/health"
```

### userfaultfd Registration Fails
**Problem:** Old userfaultfd version (0.7) incompatible with kernel 6.14
**Solution:** Upgrade to userfaultfd 0.9
```bash
# Update pager/Cargo.toml
userfaultfd = "0.9"

# Force rebuild on nodes
ssh access "cd ~/ssi-hv-starter && cargo clean && cargo build --release --example pager_node"
ssh mo "cd ~/ssi-hv-starter && cargo clean && cargo build --release --example pager_node"
```

### Kernel Setting Not Enabled
**Problem:** vm.unprivileged_userfaultfd = 0
**Solution:** Enable with sysctl
```bash
ssh access "echo toor | sudo -S sysctl -w vm.unprivileged_userfaultfd=1"
ssh mo "echo toor | sudo -S sysctl -w vm.unprivileged_userfaultfd=1"

# Verify
ssh access "sysctl vm.unprivileged_userfaultfd"
ssh mo "sysctl vm.unprivileged_userfaultfd"
```

### Build Fails on Remote Nodes
**Problem:** Cargo cache or wrong userfaultfd version
**Solution:** Force clean rebuild
```bash
ssh access "cd ~/ssi-hv-starter && rm -rf target && cargo build --release --example pager_node"
```

## Monitoring

### Watch Endpoints in Real-Time
```bash
watch -n 1 'curl -s http://100.86.226.54:8001/endpoints | python3 -m json.tool'
```

### Monitor Coordinator Logs
```bash
tail -f ~/ssi-hv-starter/coordinator/coordinator.log | grep -E "POST|GET|error"
```

### Check Node Resource Usage
```bash
ssh access "top -b -n 1 | grep pager_node"
ssh mo "top -b -n 1 | grep pager_node"
```

### Network Latency Test
```bash
# From access to mo
ssh access "ping -c 5 192.168.53.31"

# From mo to access
ssh mo "ping -c 5 192.168.53.94"
```

## Development

### Build Locally
```bash
cd /home/toor/ssi-hv-starter
cargo build --release --example pager_node
```

### Test Locally (with sudo)
```bash
sudo ./target/release/examples/pager_node 0 2 http://100.86.226.54:8001
```

### Sync Changes to Nodes
```bash
cd /home/toor/ssi-hv-starter
rsync -av --exclude='target/' --exclude='__pycache__/' --exclude='.git/' . access:/home/toor/ssi-hv-starter/
rsync -av --exclude='target/' --exclude='__pycache__/' --exclude='.git/' . mo:/home/toor/ssi-hv-starter/
```

### Force Rebuild on Nodes
```bash
ssh access "cd ~/ssi-hv-starter && cargo clean -p pager && cargo build --release --example pager_node"
ssh mo "cd ~/ssi-hv-starter && cargo clean -p pager && cargo build --release --example pager_node"
```

## Configuration Files

### Deployment Script
- **Location:** `deploy/start_vmms.sh`
- **Key Variables:**
  - `COORDINATOR_URL`: Coordinator endpoint
  - `SUDO_PASSWORD`: Sudo password for remote nodes
  - `NODE_ACCESS`, `NODE_MO`: Node hostnames

### Coordinator
- **Location:** `coordinator/main.py`
- **Port:** 8001 (default 8000 has TIME_WAIT issues)
- **Features:** Auto-cluster creation, endpoint discovery

### Pager Library
- **Location:** `pager/Cargo.toml`
- **Key Dependency:** `userfaultfd = "0.9"` (critical!)

## Performance Tips

### Reduce Log Verbosity
Edit log levels in source files or use `RUST_LOG` environment variable

### Monitor Network Usage
```bash
ssh access "iftop -i eth0"
```

### Check Page Fault Activity
```bash
ssh access "cat /proc/vmstat | grep pgfault"
```

## Safety

### Backup Logs Before Cleanup
```bash
ssh access "cp ~/ssi-hv-starter/pager0.log ~/pager0.log.$(date +%Y%m%d_%H%M%S)"
ssh mo "cp ~/ssi-hv-starter/pager1.log ~/pager1.log.$(date +%Y%m%d_%H%M%S)"
```

### Graceful Shutdown
Always use `./stop_vmms.sh` instead of killing processes manually

### Verify Cleanup
```bash
./status_cluster.sh
```

## Next Steps

For workload testing:
1. Integrate with VMM
2. Deploy guest VMs
3. Measure page fault latency
4. Stress test under load
5. Implement RDMA transport

For more information, see:
- `PHASE8_SUCCESS.md` - Detailed phase 8 completion report
- `CLUSTER_STATUS.md` - Current cluster status and metrics
