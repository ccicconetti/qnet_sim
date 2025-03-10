// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::EdgeRef;

/// A quantum network is made of a collection of nodes.
#[derive(Debug)]
struct Network {
    /// The network nodes, with compact identifiers from 0.
    nodes: Vec<super::node::Node>,
}

impl Network {
    /// Create a network from the logical topology.
    pub fn new(logical_topology: &super::logical_topology::LogicalTopology) -> Self {
        // Create the nodes.
        let mut nodes = vec![];
        for node_id in 0..logical_topology.graph().node_count() {
            nodes.push(super::node::Node::new(node_id as u32));
        }

        // Add the NICs.
        for edge in logical_topology.graph().edge_references() {
            let master_node_id = edge.source().index();
            let slave_node_id = edge.target().index();
            let num_qubits = edge.weight().memory_qubits;

            nodes[master_node_id].add_nic(
                slave_node_id as u32,
                super::nic::Role::Master,
                num_qubits,
            );
            nodes[slave_node_id].add_nic(
                master_node_id as u32,
                super::nic::Role::Slave,
                num_qubits,
            );
        }

        Self { nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::Network;

    #[test]
    fn test_network_from_logical_topology() {
        let logical_topology = crate::tests::logical_topology_2_2();
        let network = Network::new(&logical_topology);
        assert_eq!(10, network.nodes.len());
    }
}
