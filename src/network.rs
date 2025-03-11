// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::EdgeRef;
use rand::SeedableRng;
use rand_distr::Distribution;

use crate::event::EventHandler;
use crate::event::{EprGeneratedData, Event, EventType};

#[derive(Debug)]
pub struct EprGenerator {
    tx_node_id: u32,
    master_node_id: u32,
    slave_node_id: u32,
    rv: rand_distr::Exp<f64>,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
}

impl EprGenerator {
    fn handle(&mut self) -> Vec<crate::event::Event> {
        let mut events = vec![];

        // Notify peer nodes of the new EPR available.

        // Schedule the next EPR generation.
        let next_epr_generation = self.rv.sample(&mut self.rng);
        events.push(Event::new(
            next_epr_generation,
            EventType::EprGenerated(EprGeneratedData {
                tx_node_id: self.tx_node_id,
                master_node_id: self.master_node_id,
                slave_node_id: self.slave_node_id,
            }),
        ));

        events
    }
}

/// A quantum network is made of a collection of nodes.
#[derive(Debug)]
pub struct Network {
    /// The network nodes, with compact identifiers from 0.
    nodes: Vec<super::node::Node>,
    /// The EPR pair generators, indexed by the ID of the tx node.
    epr_generators: std::collections::HashMap<u32, Vec<EprGenerator>>,
}

impl Network {
    /// Create a network from the logical topology.
    pub fn new(
        logical_topology: &super::logical_topology::LogicalTopology,
        init_seed: u64,
    ) -> Self {
        // Create the nodes.
        let mut nodes = vec![];
        for node_id in 0..logical_topology.graph().node_count() {
            nodes.push(super::node::Node::new(node_id as u32));
        }

        // Add the NICs and EPR generators.
        let mut epr_generators: std::collections::HashMap<u32, Vec<EprGenerator>> =
            std::collections::HashMap::new();
        for (cnt, edge) in logical_topology.graph().edge_references().enumerate() {
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

            let master_node_id = master_node_id as u32;
            let slave_node_id = slave_node_id as u32;
            epr_generators
                .entry(edge.weight().tx)
                .or_default()
                .push(EprGenerator {
                    tx_node_id: edge.weight().tx,
                    master_node_id,
                    slave_node_id,
                    rv: rand_distr::Exp::new(edge.weight().capacity)
                        .expect("could not create an expo rv"),
                    rng: rand::rngs::StdRng::seed_from_u64(init_seed + cnt as u64),
                });
        }

        Self {
            nodes,
            epr_generators,
        }
    }
}

impl crate::event::EventHandler for Network {
    fn handle(&mut self, event: Event) -> Vec<Event> {
        match &event.event_type {
            EventType::EprGenerated(data) => {
                for generator in self
                    .epr_generators
                    .get_mut(&data.tx_node_id)
                    .expect("unknown tx node id")
                {
                    if generator.master_node_id == data.master_node_id
                        && generator.slave_node_id == data.slave_node_id
                    {
                        return generator.handle();
                    }
                }
                panic!(
                    "could not find generator for tx_node_id {} master_node_id {} slave_node_id {}",
                    data.tx_node_id, data.master_node_id, data.slave_node_id
                );
            }
            _ => panic!(
                "invalid event {:?} received by a Network object",
                event.event_type
            ),
        }
    }

    fn initial(&mut self) -> Vec<Event> {
        let mut events = vec![];

        for generators in self.epr_generators.values_mut() {
            for generator in generators {
                for event in generator.handle() {
                    match &event.event_type {
                        EventType::EprGenerated(data) => events.push(event),
                        _ => {}
                    }
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_distr::Distribution;

    use super::Network;

    #[test]
    fn test_network_from_logical_topology() {
        let logical_topology = crate::tests::logical_topology_2_2();
        let network = Network::new(&logical_topology, 42);
        assert_eq!(10, network.nodes.len());
    }

    #[test]
    fn test_expo_rv() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let rv = rand_distr::Exp::new(10.0).unwrap();
        for _ in 0..10 {
            let x = rv.sample(&mut rng);
            println!("{}", x);
        }
    }
}
