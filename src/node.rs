// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// A quantum node.
#[derive(Debug)]
pub struct Node {
    /// Node's identifier.
    node_id: u32,
    /// Quantum NICs towards logical peers for which this node is master.
    nics_master: std::collections::HashMap<u32, super::nic::Nic>,
    /// Quantum NICs towards logical peers for which this node is slave.
    nics_slave: std::collections::HashMap<u32, super::nic::Nic>,
}

impl Node {
    /// Create a node with no NICs.
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            nics_master: std::collections::HashMap::new(),
            nics_slave: std::collections::HashMap::new(),
        }
    }

    /// Add a NIC towards a given peer.
    ///
    /// Parameters:
    /// - `peer_node_id`: the identifier of the peer node
    /// - `role`: the role of this node in the logical link
    /// - `num_qubits`: how many quantum memory cells there will be
    ///
    /// Return true if `peer_node_id` was already present with same role for
    /// this node.
    pub fn add_nic(&mut self, peer_node_id: u32, role: super::nic::Role, num_qubits: u32) -> bool {
        self.nics(&role)
            .insert(peer_node_id, super::nic::Nic::new(role, num_qubits))
            .is_none()
    }

    /// Notify that a new EPR has been established.
    pub fn epr_established(
        &mut self,
        now: u64,
        peer_node_id: u32,
        role: super::nic::Role,
        epr_pair_id: u64,
    ) {
        self.nics(&role)
            .get_mut(&peer_node_id)
            .unwrap_or_else(|| panic!("could not find NIC for peer {}", peer_node_id))
            .add_epr_pair(now, epr_pair_id);
    }

    /// Return the right set of NICs depending on the role.
    fn nics(
        &mut self,
        role: &super::nic::Role,
    ) -> &mut std::collections::HashMap<u32, super::nic::Nic> {
        match role {
            super::nic::Role::Master => &mut self.nics_master,
            super::nic::Role::Slave => &mut self.nics_slave,
        }
    }
}
