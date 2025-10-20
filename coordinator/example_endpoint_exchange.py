#!/usr/bin/env python3
"""
Example: Transport endpoint exchange via coordinator

Demonstrates how nodes register their TCP/RDMA endpoints
and discover peer endpoints for page transfers.
"""

import requests
import json
from typing import Dict, Optional

COORDINATOR_URL = "http://localhost:8000"


def create_cluster(name: str, nodes: list) -> dict:
    """Create a new cluster"""
    response = requests.post(
        f"{COORDINATOR_URL}/cluster",
        json={"name": name, "nodes": nodes}
    )
    response.raise_for_status()
    return response.json()


def register_tcp_endpoint(node_id: int, addr: str, port: int) -> dict:
    """Register TCP transport endpoint for a node"""
    endpoint = {
        "transport_type": "tcp",
        "tcp_addr": addr,
        "tcp_port": port,
    }
    response = requests.post(
        f"{COORDINATOR_URL}/nodes/{node_id}/endpoint",
        json=endpoint
    )
    response.raise_for_status()
    return response.json()


def register_rdma_endpoint(
    node_id: int,
    qpn: int,
    lid: int,
    gid: str,
    psn: int
) -> dict:
    """Register RDMA transport endpoint for a node"""
    endpoint = {
        "transport_type": "rdma",
        "rdma_qpn": qpn,
        "rdma_lid": lid,
        "rdma_gid": gid,
        "rdma_psn": psn,
    }
    response = requests.post(
        f"{COORDINATOR_URL}/nodes/{node_id}/endpoint",
        json=endpoint
    )
    response.raise_for_status()
    return response.json()


def get_peer_endpoint(peer_node_id: int) -> dict:
    """Get endpoint for a peer node"""
    response = requests.get(
        f"{COORDINATOR_URL}/nodes/{peer_node_id}/endpoint"
    )
    response.raise_for_status()
    return response.json()


def get_all_endpoints() -> dict:
    """Get all registered endpoints in cluster"""
    response = requests.get(f"{COORDINATOR_URL}/endpoints")
    response.raise_for_status()
    return response.json()


def example_tcp_two_node_cluster():
    """Example: 2-node cluster with TCP transport"""
    print("\n=== TCP Transport Example ===\n")

    # Create cluster
    nodes = [
        {
            "node_id": 0,
            "hostname": "node0",
            "ip_address": "192.168.1.10",
            "cpu_count": 8,
            "memory_mb": 16384,
            "status": "active",
        },
        {
            "node_id": 1,
            "hostname": "node1",
            "ip_address": "192.168.1.11",
            "cpu_count": 8,
            "memory_mb": 16384,
            "status": "active",
        },
    ]

    cluster = create_cluster("tcp-cluster", nodes)
    print(f"✅ Created cluster: {cluster['cluster_name']}")
    print(
        f"   Nodes: {cluster['nodes']}, Memory: {cluster['total_memory_mb']} MB\n")

    # Node 0 registers TCP endpoint
    result = register_tcp_endpoint(
        node_id=0,
        addr="192.168.1.10",
        port=50051
    )
    print(f"✅ Node 0 registered TCP endpoint: 192.168.1.10:50051")

    # Node 1 registers TCP endpoint
    result = register_tcp_endpoint(
        node_id=1,
        addr="192.168.1.11",
        port=50051
    )
    print(f"✅ Node 1 registered TCP endpoint: 192.168.1.11:50051\n")

    # Node 0 discovers Node 1's endpoint
    peer_endpoint = get_peer_endpoint(peer_node_id=1)
    print(f"🔍 Node 0 discovered Node 1's endpoint:")
    print(f"   Transport: {peer_endpoint['transport_type'].upper()}")
    print(
        f"   Address: {peer_endpoint['tcp_addr']}:{peer_endpoint['tcp_port']}\n")

    # List all endpoints
    all_endpoints = get_all_endpoints()
    print(f"📋 All cluster endpoints:")
    for node_id, endpoint in all_endpoints["endpoints"].items():
        if endpoint["transport_type"] == "tcp":
            print(
                f"   Node {node_id}: TCP {endpoint['tcp_addr']}:{endpoint['tcp_port']}")

    print("\n✨ Ready for page transfers!")


def example_rdma_upgrade():
    """Example: Upgrade from TCP to RDMA"""
    print("\n=== RDMA Upgrade Example ===\n")

    # Create cluster with TCP initially
    nodes = [
        {
            "node_id": 0,
            "hostname": "node0",
            "ip_address": "192.168.1.10",
            "cpu_count": 8,
            "memory_mb": 16384,
            "status": "active",
        },
    ]

    cluster = create_cluster("upgrade-cluster", nodes)
    print(f"✅ Created cluster: {cluster['cluster_name']}")

    # Initially use TCP
    register_tcp_endpoint(node_id=0, addr="192.168.1.10", port=50051)
    print(f"✅ Node 0 started with TCP transport")

    endpoint = get_peer_endpoint(0)
    print(f"   Initial: TCP {endpoint['tcp_addr']}:{endpoint['tcp_port']}\n")

    # Upgrade to RDMA (after installing RDMA NICs)
    print("🚀 Upgrading to RDMA transport...")
    register_rdma_endpoint(
        node_id=0,
        qpn=12345,
        lid=1,
        gid="fe80::a00:27ff:fe00:0",
        psn=100
    )
    print(f"✅ Node 0 upgraded to RDMA transport")

    endpoint = get_peer_endpoint(0)
    print(
        f"   Upgraded: RDMA QPN={endpoint['rdma_qpn']}, LID={endpoint['rdma_lid']}\n")

    print("✨ Zero-downtime upgrade complete!")


def example_mixed_transport():
    """Example: Mixed TCP and RDMA transports"""
    print("\n=== Mixed Transport Example ===\n")

    # Create 3-node cluster
    nodes = [
        {
            "node_id": 0,
            "hostname": "node0-rdma",
            "ip_address": "192.168.1.10",
            "cpu_count": 16,
            "memory_mb": 32768,
            "status": "active",
        },
        {
            "node_id": 1,
            "hostname": "node1-tcp",
            "ip_address": "192.168.1.11",
            "cpu_count": 8,
            "memory_mb": 16384,
            "status": "active",
        },
        {
            "node_id": 2,
            "hostname": "node2-tcp",
            "ip_address": "192.168.1.12",
            "cpu_count": 8,
            "memory_mb": 16384,
            "status": "active",
        },
    ]

    cluster = create_cluster("mixed-cluster", nodes)
    print(f"✅ Created cluster: {cluster['cluster_name']}")
    print(
        f"   Total: {cluster['total_memory_mb']} MB across {cluster['nodes']} nodes\n")

    # Node 0 has RDMA
    register_rdma_endpoint(
        node_id=0, qpn=12345, lid=1,
        gid="fe80::a00:27ff:fe00:0", psn=100
    )
    print(f"✅ Node 0: RDMA transport (high-performance server)")

    # Nodes 1 and 2 use TCP
    register_tcp_endpoint(node_id=1, addr="192.168.1.11", port=50051)
    register_tcp_endpoint(node_id=2, addr="192.168.1.12", port=50051)
    print(f"✅ Node 1: TCP transport (consumer hardware)")
    print(f"✅ Node 2: TCP transport (consumer hardware)\n")

    # Show all endpoints
    all_endpoints = get_all_endpoints()
    print(f"📋 Cluster transport topology:")
    for node_id, endpoint in all_endpoints["endpoints"].items():
        if endpoint["transport_type"] == "tcp":
            print(
                f"   Node {node_id}: TCP {endpoint['tcp_addr']}:{endpoint['tcp_port']} (200-500µs)")
        else:
            print(
                f"   Node {node_id}: RDMA QPN={endpoint['rdma_qpn']} (<100µs)")

    print("\n✨ Mixed-transport cluster operational!")
    print("   Node 0 <-> Node 1: TCP fallback (auto-detected)")
    print("   Node 1 <-> Node 2: TCP peer-to-peer")


def main():
    """Run all examples"""
    print("\n" + "="*60)
    print("  SSI-HV Transport Endpoint Exchange Examples")
    print("="*60)

    try:
        # Check coordinator is running
        response = requests.get(f"{COORDINATOR_URL}/health")
        response.raise_for_status()
        print("\n✅ Coordinator is running at", COORDINATOR_URL)
    except requests.exceptions.RequestException as e:
        print(f"\n❌ Error: Coordinator not running at {COORDINATOR_URL}")
        print("   Start coordinator with: python coordinator/main.py")
        return 1

    # Run examples
    try:
        example_tcp_two_node_cluster()

        # Clean up between examples
        requests.delete(f"{COORDINATOR_URL}/cluster")

        example_rdma_upgrade()

        # Clean up
        requests.delete(f"{COORDINATOR_URL}/cluster")

        example_mixed_transport()

        # Final cleanup
        requests.delete(f"{COORDINATOR_URL}/cluster")

    except requests.exceptions.RequestException as e:
        print(f"\n❌ Error: {e}")
        return 1

    print("\n" + "="*60)
    print("  ✨ All examples completed successfully!")
    print("="*60 + "\n")

    return 0


if __name__ == "__main__":
    exit(main())
