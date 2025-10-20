#!/usr/bin/env python3
"""
SSI-HV Coordinator Control Plane (M3)

Manages cluster formation, node join/leave, and orchestration.
Exposes REST API for cluster management and metrics.
"""

import asyncio
import logging
from typing import Dict, List, Optional
from dataclasses import dataclass, field
from datetime import datetime
import json

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import uvicorn

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("ssi-hv-coordinator")

# ============================================================================
# Data Models
# ============================================================================


class TransportEndpoint(BaseModel):
    """Transport endpoint information (TCP or RDMA)"""
    transport_type: str  # "tcp" or "rdma"
    # TCP fields
    tcp_addr: Optional[str] = None
    tcp_port: Optional[int] = None
    # RDMA fields (optional)
    rdma_qpn: Optional[int] = None
    rdma_lid: Optional[int] = None
    rdma_gid: Optional[str] = None
    rdma_psn: Optional[int] = None


class NodeInfo(BaseModel):
    """Node information for cluster membership"""
    node_id: int
    hostname: str
    ip_address: str
    rdma_gid: Optional[str] = None  # Deprecated, use endpoint.rdma_gid
    cpu_count: int
    memory_mb: int
    status: str = "joining"  # joining, active, leaving, failed
    endpoint: Optional[TransportEndpoint] = None  # Transport endpoint


class ClusterConfig(BaseModel):
    """Cluster configuration"""
    name: str
    total_memory_mb: int
    total_vcpus: int
    nodes: List[NodeInfo]


class ClusterCreateRequest(BaseModel):
    """Request to create a new cluster"""
    name: str
    nodes: List[NodeInfo]


class MetricsResponse(BaseModel):
    """Cluster metrics response"""
    cluster_name: str
    total_nodes: int
    active_nodes: int
    total_memory_mb: int
    total_vcpus: int
    remote_fault_rate: float
    remote_miss_ratio: float
    avg_fault_latency_us: float


# ============================================================================
# Cluster State
# ============================================================================

@dataclass
class ClusterState:
    """In-memory cluster state"""
    name: str
    nodes: Dict[int, NodeInfo] = field(default_factory=dict)
    endpoints: Dict[int, TransportEndpoint] = field(
        default_factory=dict)  # node_id -> endpoint
    created_at: datetime = field(default_factory=datetime.now)
    vm_running: bool = False

    def add_node(self, node: NodeInfo):
        """Add node to cluster"""
        self.nodes[node.node_id] = node
        if node.endpoint:
            self.endpoints[node.node_id] = node.endpoint
            logger.info(
                f"Node {node.node_id} ({node.hostname}) joined cluster with {node.endpoint.transport_type.upper()} transport")
        else:
            logger.info(
                f"Node {node.node_id} ({node.hostname}) joined cluster")

    def remove_node(self, node_id: int):
        """Remove node from cluster"""
        if node_id in self.nodes:
            node = self.nodes.pop(node_id)
            logger.info(f"Node {node_id} ({node.hostname}) left cluster")

    def get_active_nodes(self) -> List[NodeInfo]:
        """Get list of active nodes"""
        return [n for n in self.nodes.values() if n.status == "active"]

    def total_memory_mb(self) -> int:
        """Calculate total cluster memory"""
        return sum(n.memory_mb for n in self.get_active_nodes())

    def total_vcpus(self) -> int:
        """Calculate total cluster vCPUs"""
        return sum(n.cpu_count for n in self.get_active_nodes())


# ============================================================================
# FastAPI Application
# ============================================================================

app = FastAPI(
    title="SSI-HV Coordinator",
    description="Control plane for Single-System-Image Hypervisor cluster",
    version="0.1.0"
)

# Global cluster state
current_cluster: Optional[ClusterState] = None


@app.post("/cluster", status_code=201)
async def create_cluster(request: ClusterCreateRequest) -> dict:
    """
    Create a new SSI-HV cluster.

    This initializes the cluster and prepares nodes for VM deployment.
    """
    global current_cluster

    if current_cluster is not None:
        raise HTTPException(status_code=400, detail="Cluster already exists")

    logger.info(
        f"Creating cluster '{request.name}' with {len(request.nodes)} nodes")

    # Create cluster state
    current_cluster = ClusterState(name=request.name)

    # Add all nodes
    for node in request.nodes:
        current_cluster.add_node(node)

    # TODO: Initialize RDMA connections between nodes
    # TODO: Distribute address space allocation
    # TODO: Start VMM processes on each node

    return {
        "status": "created",
        "cluster_name": request.name,
        "nodes": len(request.nodes),
        "total_memory_mb": current_cluster.total_memory_mb(),
        "total_vcpus": current_cluster.total_vcpus(),
    }


@app.delete("/cluster")
async def destroy_cluster() -> dict:
    """
    Destroy the current cluster and clean up resources.
    """
    global current_cluster

    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    logger.info(f"Destroying cluster '{current_cluster.name}'")

    # TODO: Stop VMM processes
    # TODO: Tear down RDMA connections
    # TODO: Clean up resources

    cluster_name = current_cluster.name
    current_cluster = None

    return {
        "status": "destroyed",
        "cluster_name": cluster_name,
    }


@app.get("/cluster")
async def get_cluster_info() -> dict:
    """
    Get current cluster information.
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    return {
        "name": current_cluster.name,
        "nodes": len(current_cluster.nodes),
        "active_nodes": len(current_cluster.get_active_nodes()),
        "total_memory_mb": current_cluster.total_memory_mb(),
        "total_vcpus": current_cluster.total_vcpus(),
        "vm_running": current_cluster.vm_running,
        "created_at": current_cluster.created_at.isoformat(),
    }


@app.post("/nodes")
async def add_node(node: NodeInfo) -> dict:
    """
    Add a node to the existing cluster (dynamic join).
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    if node.node_id in current_cluster.nodes:
        raise HTTPException(
            status_code=400, detail=f"Node {node.node_id} already exists")

    logger.info(f"Adding node {node.node_id} to cluster")

    current_cluster.add_node(node)

    # TODO: Establish RDMA connections
    # TODO: Redistribute memory allocation
    # TODO: Start VMM on new node

    return {
        "status": "joined",
        "node_id": node.node_id,
        "cluster_nodes": len(current_cluster.nodes),
    }


@app.delete("/nodes/{node_id}")
async def remove_node(node_id: int) -> dict:
    """
    Remove a node from the cluster (graceful leave).
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    if node_id not in current_cluster.nodes:
        raise HTTPException(
            status_code=404, detail=f"Node {node_id} not found")

    logger.info(f"Removing node {node_id} from cluster")

    # TODO: Migrate pages from leaving node
    # TODO: Close RDMA connections
    # TODO: Stop VMM on node

    current_cluster.remove_node(node_id)

    return {
        "status": "removed",
        "node_id": node_id,
        "remaining_nodes": len(current_cluster.nodes),
    }


@app.get("/metrics")
async def get_metrics() -> MetricsResponse:
    """
    Get cluster metrics (Prometheus-compatible format).

    Metrics include:
    - Remote fault rate (faults/s)
    - Remote miss ratio (%)
    - Average fault service latency (µs)
    - Migration traffic (bytes/s)
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    # TODO M6: Collect real metrics from VMM/pager processes
    # For now, return mock data

    return MetricsResponse(
        cluster_name=current_cluster.name,
        total_nodes=len(current_cluster.nodes),
        active_nodes=len(current_cluster.get_active_nodes()),
        total_memory_mb=current_cluster.total_memory_mb(),
        total_vcpus=current_cluster.total_vcpus(),
        remote_fault_rate=0.0,
        remote_miss_ratio=0.0,
        avg_fault_latency_us=0.0,
    )


@app.get("/pages/{gpa:path}")
async def get_page_info(gpa: str) -> dict:
    """
    Get information about a specific guest physical page.

    Returns:
    - Owner node
    - Heat (access frequency)
    - Migration history
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    # Parse GPA (hex string)
    try:
        gpa_int = int(gpa, 16) if gpa.startswith('0x') else int(gpa)
    except ValueError:
        raise HTTPException(status_code=400, detail="Invalid GPA format")

    # TODO M6: Query page directory for ownership and heat

    return {
        "gpa": hex(gpa_int),
        "owner_node": 0,
        "heat": 0,
        "access_count": 0,
        "migration_count": 0,
    }


@app.post("/nodes/{node_id}/endpoint", status_code=201)
async def register_endpoint(node_id: int, endpoint: TransportEndpoint) -> dict:
    """
    Register or update transport endpoint for a node.

    This allows nodes to advertise their TCP or RDMA endpoints
    for other nodes to discover and connect to.
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    if node_id not in current_cluster.nodes:
        raise HTTPException(
            status_code=404, detail=f"Node {node_id} not found")

    # Store endpoint
    current_cluster.endpoints[node_id] = endpoint
    current_cluster.nodes[node_id].endpoint = endpoint

    logger.info(
        f"Node {node_id} registered {endpoint.transport_type.upper()} endpoint: "
        f"{endpoint.tcp_addr}:{endpoint.tcp_port}" if endpoint.transport_type == "tcp"
        else f"QPN={endpoint.rdma_qpn}"
    )

    return {
        "status": "registered",
        "node_id": node_id,
        "transport_type": endpoint.transport_type,
    }


@app.get("/nodes/{node_id}/endpoint")
async def get_endpoint(node_id: int) -> TransportEndpoint:
    """
    Get transport endpoint for a specific node.

    Other nodes use this to discover connection information
    for remote page fetches.
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    if node_id not in current_cluster.endpoints:
        raise HTTPException(
            status_code=404,
            detail=f"No endpoint registered for node {node_id}"
        )

    return current_cluster.endpoints[node_id]


@app.get("/endpoints")
async def list_all_endpoints() -> dict:
    """
    Get all registered transport endpoints in the cluster.

    Useful for nodes to discover all peers at once.
    """
    if current_cluster is None:
        raise HTTPException(status_code=404, detail="No active cluster")

    return {
        "cluster_name": current_cluster.name,
        "endpoints": {
            str(node_id): endpoint.model_dump()
            for node_id, endpoint in current_cluster.endpoints.items()
        },
    }


@app.get("/health")
async def health_check() -> dict:
    """Health check endpoint"""
    return {
        "status": "healthy",
        "cluster_active": current_cluster is not None,
    }


# ============================================================================
# Main
# ============================================================================

def main():
    """Start the coordinator API server"""
    logger.info("Starting SSI-HV Coordinator (M3)")
    logger.info("API documentation: http://0.0.0.0:8000/docs")

    uvicorn.run(
        app,
        host="0.0.0.0",
        port=8000,
        log_level="info",
    )


if __name__ == "__main__":
    main()
