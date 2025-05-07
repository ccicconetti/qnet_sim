// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// A quantum node.
pub struct Node {
    /// Node's identifier.
    node_id: u32,
    /// Quantum NICs towards logical peers for which this node is master.
    nics_master: std::collections::HashMap<u32, super::nic::Nic>,
    /// Quantum NICs towards logical peers for which this node is slave.
    nics_slave: std::collections::HashMap<u32, super::nic::Nic>,
    /// The applications, identified by their port.
    applications: std::collections::HashMap<u16, Box<dyn crate::event::EventHandler>>,
}

impl Node {
    /// Create a node with no NICs.
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            nics_master: std::collections::HashMap::new(),
            nics_slave: std::collections::HashMap::new(),
            applications: std::collections::HashMap::new(),
        }
    }

    /// Retrieve an application running on this node.
    pub fn application(
        &mut self,
        port: u16,
    ) -> anyhow::Result<&mut Box<dyn crate::event::EventHandler>> {
        self.applications.get_mut(&port).ok_or(anyhow::anyhow!(
            "no application at port {} on node {}",
            port,
            self.node_id
        ))
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

    /// Notify that a new EPR has been established. Return the occupancy ratio.
    pub fn epr_established(
        &mut self,
        now: u64,
        peer_node_id: u32,
        role: super::nic::Role,
        epr_pair_id: u64,
    ) -> f64 {
        let nic = self.get_nic(peer_node_id, &role);
        nic.add_epr_pair(now, epr_pair_id);
        nic.occupancy()
    }

    /// Consume the qubit of an EPR stored in a memory cell in one of the NICs.
    /// Return the creation time and identifier.
    pub fn consume(
        &mut self,
        peer_node_id: u32,
        role: &super::nic::Role,
        index: usize,
    ) -> Option<(u64, u64)> {
        self.get_nic(peer_node_id, role).consume(index)
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

    /// Return the NIC for a given peer node and role.
    fn get_nic(&mut self, peer_node_id: u32, role: &super::nic::Role) -> &mut super::nic::Nic {
        self.nics(role)
            .get_mut(&peer_node_id)
            .unwrap_or_else(|| panic!("could not find NIC for peer {} ({:?})", peer_node_id, role))
    }
}
