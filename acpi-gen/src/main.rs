use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};

/// Cluster topology configuration for ACPI generation
#[derive(Debug, Serialize, Deserialize)]
struct ClusterTopology {
    nodes: Vec<NodeConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeConfig {
    node_id: u32,
    /// CPUs assigned to this node
    cpu_start: u32,
    cpu_count: u32,
    /// Memory assigned to this node (in bytes)
    mem_start: u64,
    mem_size: u64,
    /// Estimated latency to other nodes (in 10ns units for SLIT)
    latencies: Vec<u32>,
}

/// Generate ACPI SRAT (System Resource Affinity Table)
fn generate_srat(topology: &ClusterTopology) -> Result<Vec<u8>> {
    info!("Generating ACPI SRAT for {} nodes", topology.nodes.len());

    // TODO M4: Implement ACPI SRAT generation using acpi_tables crate
    // SRAT contains:
    // - Processor Local APIC/SAPIC Affinity Structure (for each CPU)
    // - Memory Affinity Structure (for each memory range)

    // Structure:
    // - Header (signature "SRAT", length, revision, checksum, etc.)
    // - Reserved fields
    // - Affinity structures for CPUs and memory

    let mut srat_data = Vec::new();

    for node in &topology.nodes {
        info!(
            "  Node {}: CPUs {}-{}, Memory 0x{:x}-0x{:x}",
            node.node_id,
            node.cpu_start,
            node.cpu_start + node.cpu_count - 1,
            node.mem_start,
            node.mem_start + node.mem_size
        );

        // Add processor affinity structures
        for cpu in node.cpu_start..(node.cpu_start + node.cpu_count) {
            // Type 0: Processor Local APIC/SAPIC Affinity
            // TODO: Add proper ACPI structure
        }

        // Add memory affinity structure
        // Type 1: Memory Affinity
        // TODO: Add proper ACPI structure
    }

    info!("SRAT generation complete (stub)");
    Ok(srat_data)
}

/// Generate ACPI SLIT (System Locality Information Table)
fn generate_slit(topology: &ClusterTopology) -> Result<Vec<u8>> {
    info!("Generating ACPI SLIT for {} nodes", topology.nodes.len());

    // SLIT contains a matrix of relative distances between nodes
    // - Distance from node to itself = 10
    // - Distance to remote nodes based on latency measurements

    let num_nodes = topology.nodes.len() as u64;

    // TODO M4: Implement SLIT generation
    // Structure:
    // - Header
    // - Number of System Localities (u64)
    // - Matrix of distances (num_nodes x num_nodes, each u8)

    info!("SLIT matrix ({}x{}):", num_nodes, num_nodes);
    for i in 0..num_nodes {
        let mut row = String::new();
        for j in 0..num_nodes {
            let distance = if i == j {
                10 // Local
            } else {
                // Use configured latency or default to 20 for remote
                topology.nodes[i as usize]
                    .latencies
                    .get(j as usize)
                    .copied()
                    .unwrap_or(20)
            };
            row.push_str(&format!("{:3} ", distance));
        }
        info!("  [{}]", row);
    }

    let slit_data = Vec::new();
    info!("SLIT generation complete (stub)");
    Ok(slit_data)
}

/// Generate ACPI HMAT (Heterogeneous Memory Attribute Table)
fn generate_hmat(topology: &ClusterTopology) -> Result<Vec<u8>> {
    info!("Generating ACPI HMAT for {} nodes", topology.nodes.len());

    // HMAT provides detailed memory characteristics:
    // - Latency (read/write)
    // - Bandwidth (read/write)
    // - Memory side cache information

    // TODO M4: Implement HMAT generation (optional for MVP)
    // This is more detailed than SLIT and provides bandwidth info

    let hmat_data = Vec::new();
    info!("HMAT generation complete (stub)");
    Ok(hmat_data)
}

/// Generate all ACPI tables for SSI-HV cluster
fn generate_acpi_tables(topology: &ClusterTopology) -> Result<()> {
    info!("=== ACPI Table Generation (M4) ===");

    let srat = generate_srat(topology)?;
    let slit = generate_slit(topology)?;
    let hmat = generate_hmat(topology)?;

    // TODO M4: Write tables to files or integrate with OVMF
    // - Tables should be loaded by UEFI firmware
    // - Guest OS will parse these to understand NUMA topology

    info!("ACPI tables generated successfully");
    info!("Next: Integrate with OVMF and test guest NUMA recognition");

    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("SSI-HV ACPI Generator (M4)");

    // Example 2-node cluster topology
    let topology = ClusterTopology {
        nodes: vec![
            NodeConfig {
                node_id: 0,
                cpu_start: 0,
                cpu_count: 4,
                mem_start: 0,
                mem_size: 2 << 30,       // 2 GiB
                latencies: vec![10, 20], // Local=10, Remote=20
            },
            NodeConfig {
                node_id: 1,
                cpu_start: 4,
                cpu_count: 4,
                mem_start: 2 << 30,
                mem_size: 2 << 30, // 2 GiB
                latencies: vec![20, 10],
            },
        ],
    };

    generate_acpi_tables(&topology)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_config_creation() {
        let node = NodeConfig {
            node_id: 0,
            cpu_start: 0,
            cpu_count: 4,
            mem_start: 0,
            mem_size: 2 << 30,
            latencies: vec![10, 20],
        };
        assert_eq!(node.node_id, 0);
        assert_eq!(node.cpu_count, 4);
    }

    #[test]
    fn test_cluster_topology() {
        let topology = ClusterTopology {
            nodes: vec![
                NodeConfig {
                    node_id: 0,
                    cpu_start: 0,
                    cpu_count: 4,
                    mem_start: 0,
                    mem_size: 2 << 30,
                    latencies: vec![10, 20],
                },
                NodeConfig {
                    node_id: 1,
                    cpu_start: 4,
                    cpu_count: 4,
                    mem_start: 2 << 30,
                    mem_size: 2 << 30,
                    latencies: vec![20, 10],
                },
            ],
        };
        assert_eq!(topology.nodes.len(), 2);
        assert_eq!(topology.nodes[0].node_id, 0);
        assert_eq!(topology.nodes[1].node_id, 1);
    }

    #[test]
    fn test_generate_srat() {
        let topology = ClusterTopology {
            nodes: vec![NodeConfig {
                node_id: 0,
                cpu_start: 0,
                cpu_count: 2,
                mem_start: 0,
                mem_size: 1 << 30,
                latencies: vec![10],
            }],
        };
        let result = generate_srat(&topology);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_slit() {
        let topology = ClusterTopology {
            nodes: vec![NodeConfig {
                node_id: 0,
                cpu_start: 0,
                cpu_count: 2,
                mem_start: 0,
                mem_size: 1 << 30,
                latencies: vec![10],
            }],
        };
        let result = generate_slit(&topology);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_hmat() {
        let topology = ClusterTopology {
            nodes: vec![NodeConfig {
                node_id: 0,
                cpu_start: 0,
                cpu_count: 2,
                mem_start: 0,
                mem_size: 1 << 30,
                latencies: vec![10],
            }],
        };
        let result = generate_hmat(&topology);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_acpi_tables() {
        let topology = ClusterTopology {
            nodes: vec![NodeConfig {
                node_id: 0,
                cpu_start: 0,
                cpu_count: 2,
                mem_start: 0,
                mem_size: 1 << 30,
                latencies: vec![10],
            }],
        };
        let result = generate_acpi_tables(&topology);
        assert!(result.is_ok());
    }

    #[test]
    fn test_two_node_topology() {
        let topology = ClusterTopology {
            nodes: vec![
                NodeConfig {
                    node_id: 0,
                    cpu_start: 0,
                    cpu_count: 4,
                    mem_start: 0,
                    mem_size: 2 << 30,
                    latencies: vec![10, 20],
                },
                NodeConfig {
                    node_id: 1,
                    cpu_start: 4,
                    cpu_count: 4,
                    mem_start: 2 << 30,
                    mem_size: 2 << 30,
                    latencies: vec![20, 10],
                },
            ],
        };
        assert_eq!(topology.nodes.len(), 2);
        assert_eq!(topology.nodes[0].latencies.len(), 2);
        assert_eq!(topology.nodes[1].latencies.len(), 2);
    }
}
