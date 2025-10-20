"""
Unit tests for SSI-HV Coordinator
Following TDD principles with comprehensive coverage
"""

from main import app, ClusterState, NodeInfo
import pytest
from fastapi.testclient import TestClient
import sys
import os

# Add coordinator to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


client = TestClient(app)


class TestHealthCheck:
    """Test health check endpoint"""

    def test_health_check(self):
        response = client.get("/health")
        assert response.status_code == 200
        data = response.json()
        assert "status" in data
        assert data["status"] == "healthy"
        assert "cluster_active" in data


class TestClusterManagement:
    """Test cluster lifecycle operations"""

    def test_create_cluster(self):
        response = client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "rdma_gid": "fe80::1",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )
        assert response.status_code == 201
        data = response.json()
        assert data["status"] == "created"
        assert data["cluster_name"] == "test-cluster"
        assert data["nodes"] == 1

        # Cleanup
        client.delete("/cluster")

    def test_create_cluster_duplicate(self):
        # Create first cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Try to create another - should fail
        response = client.post(
            "/cluster",
            json={
                "name": "test-cluster-2",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )
        assert response.status_code == 400

        # Cleanup
        client.delete("/cluster")

    def test_get_cluster_info(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Get info
        response = client.get("/cluster")
        assert response.status_code == 200
        data = response.json()
        assert data["name"] == "test-cluster"
        assert data["nodes"] == 1
        assert data["active_nodes"] == 1
        assert data["total_memory_mb"] == 8192
        assert data["total_vcpus"] == 4

        # Cleanup
        client.delete("/cluster")

    def test_get_cluster_info_no_cluster(self):
        response = client.get("/cluster")
        assert response.status_code == 404

    def test_destroy_cluster(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Destroy
        response = client.delete("/cluster")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "destroyed"
        assert data["cluster_name"] == "test-cluster"

    def test_destroy_cluster_none_exists(self):
        response = client.delete("/cluster")
        assert response.status_code == 404


class TestNodeManagement:
    """Test node operations"""

    def test_add_node(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Add another node
        response = client.post(
            "/nodes",
            json={
                "node_id": 1,
                "hostname": "node1",
                "ip_address": "192.168.1.11",
                "cpu_count": 4,
                "memory_mb": 8192,
                "status": "active",
            },
        )
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "joined"
        assert data["node_id"] == 1
        assert data["cluster_nodes"] == 2

        # Cleanup
        client.delete("/cluster")

    def test_add_node_duplicate(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Try to add same node again
        response = client.post(
            "/nodes",
            json={
                "node_id": 0,
                "hostname": "node0",
                "ip_address": "192.168.1.10",
                "cpu_count": 4,
                "memory_mb": 8192,
                "status": "active",
            },
        )
        assert response.status_code == 400

        # Cleanup
        client.delete("/cluster")

    def test_remove_node(self):
        # Create cluster with 2 nodes
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    },
                    {
                        "node_id": 1,
                        "hostname": "node1",
                        "ip_address": "192.168.1.11",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    },
                ],
            },
        )

        # Remove node
        response = client.delete("/nodes/1")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "removed"
        assert data["node_id"] == 1
        assert data["remaining_nodes"] == 1

        # Cleanup
        client.delete("/cluster")

    def test_remove_node_not_found(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Try to remove non-existent node
        response = client.delete("/nodes/999")
        assert response.status_code == 404

        # Cleanup
        client.delete("/cluster")


class TestMetrics:
    """Test metrics endpoint"""

    def test_get_metrics(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Get metrics
        response = client.get("/metrics")
        assert response.status_code == 200
        data = response.json()
        assert "cluster_name" in data
        assert "total_nodes" in data
        assert "active_nodes" in data
        assert "remote_fault_rate" in data
        assert "remote_miss_ratio" in data

        # Cleanup
        client.delete("/cluster")

    def test_get_metrics_no_cluster(self):
        response = client.get("/metrics")
        assert response.status_code == 404


class TestPageInfo:
    """Test page ownership queries"""

    def test_get_page_info_hex(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Query page
        response = client.get("/pages/0x1000")
        assert response.status_code == 200
        data = response.json()
        assert "gpa" in data
        assert "owner_node" in data
        assert data["gpa"] == "0x1000"

        # Cleanup
        client.delete("/cluster")

    def test_get_page_info_decimal(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Query page
        response = client.get("/pages/4096")
        assert response.status_code == 200
        data = response.json()
        assert data["gpa"] == "0x1000"

        # Cleanup
        client.delete("/cluster")

    def test_get_page_info_invalid(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Query with invalid address
        response = client.get("/pages/invalid")
        assert response.status_code == 400

        # Cleanup
        client.delete("/cluster")


class TestClusterState:
    """Test ClusterState internal logic"""

    def test_cluster_state_creation(self):
        state = ClusterState(name="test")
        assert state.name == "test"
        assert len(state.nodes) == 0
        assert state.vm_running is False

    def test_cluster_state_add_node(self):
        state = ClusterState(name="test")
        node = NodeInfo(
            node_id=0,
            hostname="node0",
            ip_address="192.168.1.10",
            cpu_count=4,
            memory_mb=8192,
        )
        state.add_node(node)
        assert len(state.nodes) == 1
        assert 0 in state.nodes

    def test_cluster_state_remove_node(self):
        state = ClusterState(name="test")
        node = NodeInfo(
            node_id=0,
            hostname="node0",
            ip_address="192.168.1.10",
            cpu_count=4,
            memory_mb=8192,
        )
        state.add_node(node)
        state.remove_node(0)
        assert len(state.nodes) == 0

    def test_cluster_state_get_active_nodes(self):
        state = ClusterState(name="test")
        node1 = NodeInfo(
            node_id=0,
            hostname="node0",
            ip_address="192.168.1.10",
            cpu_count=4,
            memory_mb=8192,
            status="active",
        )
        node2 = NodeInfo(
            node_id=1,
            hostname="node1",
            ip_address="192.168.1.11",
            cpu_count=4,
            memory_mb=8192,
            status="joining",
        )
        state.add_node(node1)
        state.add_node(node2)

        active = state.get_active_nodes()
        assert len(active) == 1
        assert active[0].node_id == 0

    def test_cluster_state_total_memory(self):
        state = ClusterState(name="test")
        node1 = NodeInfo(
            node_id=0,
            hostname="node0",
            ip_address="192.168.1.10",
            cpu_count=4,
            memory_mb=8192,
            status="active",
        )
        node2 = NodeInfo(
            node_id=1,
            hostname="node1",
            ip_address="192.168.1.11",
            cpu_count=4,
            memory_mb=16384,
            status="active",
        )
        state.add_node(node1)
        state.add_node(node2)

        assert state.total_memory_mb() == 24576

    def test_cluster_state_total_vcpus(self):
        state = ClusterState(name="test")
        node1 = NodeInfo(
            node_id=0,
            hostname="node0",
            ip_address="192.168.1.10",
            cpu_count=4,
            memory_mb=8192,
            status="active",
        )
        node2 = NodeInfo(
            node_id=1,
            hostname="node1",
            ip_address="192.168.1.11",
            cpu_count=8,
            memory_mb=16384,
            status="active",
        )
        state.add_node(node1)
        state.add_node(node2)

        assert state.total_vcpus() == 12


class TestTransportEndpoints:
    """Test transport endpoint registration and discovery"""

    def test_register_tcp_endpoint(self):
        # Create cluster with node
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Register TCP endpoint
        response = client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )
        assert response.status_code == 201
        data = response.json()
        assert data["status"] == "registered"
        assert data["node_id"] == 0
        assert data["transport_type"] == "tcp"

        # Cleanup
        client.delete("/cluster")

    def test_register_rdma_endpoint(self):
        # Create cluster with node
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Register RDMA endpoint
        response = client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "rdma",
                "rdma_qpn": 12345,
                "rdma_lid": 1,
                "rdma_gid": "fe80::1",
                "rdma_psn": 100,
            },
        )
        assert response.status_code == 201
        data = response.json()
        assert data["status"] == "registered"
        assert data["transport_type"] == "rdma"

        # Cleanup
        client.delete("/cluster")

    def test_get_endpoint(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Register endpoint
        client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )

        # Get endpoint
        response = client.get("/nodes/0/endpoint")
        assert response.status_code == 200
        data = response.json()
        assert data["transport_type"] == "tcp"
        assert data["tcp_addr"] == "192.168.1.10"
        assert data["tcp_port"] == 50051

        # Cleanup
        client.delete("/cluster")

    def test_get_endpoint_not_found(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Try to get endpoint for node without registered endpoint
        response = client.get("/nodes/0/endpoint")
        assert response.status_code == 404

        # Cleanup
        client.delete("/cluster")

    def test_list_all_endpoints(self):
        # Create cluster with multiple nodes
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    },
                    {
                        "node_id": 1,
                        "hostname": "node1",
                        "ip_address": "192.168.1.11",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    },
                ],
            },
        )

        # Register endpoints
        client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )
        client.post(
            "/nodes/1/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.11",
                "tcp_port": 50051,
            },
        )

        # List all endpoints
        response = client.get("/endpoints")
        assert response.status_code == 200
        data = response.json()
        assert data["cluster_name"] == "test-cluster"
        assert "endpoints" in data
        assert "0" in data["endpoints"]
        assert "1" in data["endpoints"]
        assert data["endpoints"]["0"]["transport_type"] == "tcp"
        assert data["endpoints"]["1"]["transport_type"] == "tcp"

        # Cleanup
        client.delete("/cluster")

    def test_update_endpoint(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Register TCP endpoint
        client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )

        # Update to different port
        response = client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50052,
            },
        )
        assert response.status_code == 201

        # Verify update
        response = client.get("/nodes/0/endpoint")
        data = response.json()
        assert data["tcp_port"] == 50052

        # Cleanup
        client.delete("/cluster")

    def test_register_endpoint_no_cluster(self):
        response = client.post(
            "/nodes/0/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )
        assert response.status_code == 404

    def test_register_endpoint_node_not_found(self):
        # Create cluster
        client.post(
            "/cluster",
            json={
                "name": "test-cluster",
                "nodes": [
                    {
                        "node_id": 0,
                        "hostname": "node0",
                        "ip_address": "192.168.1.10",
                        "cpu_count": 4,
                        "memory_mb": 8192,
                        "status": "active",
                    }
                ],
            },
        )

        # Try to register endpoint for non-existent node
        response = client.post(
            "/nodes/999/endpoint",
            json={
                "transport_type": "tcp",
                "tcp_addr": "192.168.1.10",
                "tcp_port": 50051,
            },
        )
        assert response.status_code == 404

        # Cleanup
        client.delete("/cluster")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
