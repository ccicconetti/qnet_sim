// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::EdgeRef;
use rand::SeedableRng;
use rand_distr::Distribution;

use crate::event::*;
use crate::output::Sample;

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
    /// Schedule the next EPR generation.
    fn handle(&mut self) -> Event {
        let next_epr_generation = self.rv.sample(&mut self.rng);
        Event::new(
            next_epr_generation,
            EventType::NetworkEvent(NetworkEventData::EprGenerated(EprGeneratedData {
                tx_node_id: self.tx_node_id,
                master_node_id: self.master_node_id,
                slave_node_id: self.slave_node_id,
            })),
        )
    }
}

/// A quantum network is made of a collection of nodes.
pub struct Network {
    /// The network nodes, with compact identifiers from 0.
    pub nodes: Vec<super::node::Node>,
    /// The EPR pair generators, indexed by the ID of the tx node.
    pub epr_generators: std::collections::HashMap<u32, Vec<EprGenerator>>,
    /// The EPR register.
    pub epr_register: crate::epr_register::EprRegister,
    /// The physical topology.
    pub physical_topology: crate::physical_topology::PhysicalTopology,
}

impl Network {
    /// Create a network from the logical topology.
    pub fn new(
        logical_topology: &super::logical_topology::LogicalTopology,
        physical_topology: crate::physical_topology::PhysicalTopology,
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

        let epr_register = crate::epr_register::EprRegister::default();
        Self {
            nodes,
            epr_generators,
            epr_register,
            physical_topology,
        }
    }

    fn handle_app_event(&mut self, now: u64, data: AppEventData) -> (Vec<Event>, Vec<Sample>) {
        let mut data = data;
        if data.needs_latency() {
            data.clear_latency();
            let distance = self
                .physical_topology
                .distance(
                    data.node_id(),
                    data.peer_node_id().expect("empty peer node_id"),
                )
                .expect("cannot compute distance between two nodes");

            let latency = crate::utils::distance_to_latency(distance);
            (vec![Event::new(latency, EventType::AppEvent(data))], vec![])
        } else {
            let node_id = data.node_id();
            assert!((node_id as usize) < self.nodes.len(), "invalid application event received by a Network object: node_id = {}, number of nodes = {}", node_id, self.nodes.len());
            let node = &mut self.nodes[node_id as usize];
            node.handle(Event::new_ns(now, EventType::AppEvent(data)))
        }
    }

    fn handle_epr_generated(
        &mut self,
        now: u64,
        data: EprGeneratedData,
    ) -> (Vec<Event>, Vec<Sample>) {
        for generator in self
            .epr_generators
            .get_mut(&data.tx_node_id)
            .expect("unknown tx node id")
        {
            if generator.master_node_id == data.master_node_id
                && generator.slave_node_id == data.slave_node_id
            {
                let mut events = vec![];
                let mut samples = vec![];

                // Create a new EPR pair.
                if let Ok(fidelity) = self.physical_topology.fidelity(
                    data.tx_node_id,
                    data.master_node_id,
                    data.slave_node_id,
                ) {
                    samples.push(Sample::Series(
                        "gen_fidelity".to_string(),
                        data.tx_node_id.to_string(),
                        fidelity,
                    ));

                    let epr_pair_id = self.epr_register.new_epr_pair(
                        data.master_node_id,
                        data.slave_node_id,
                        now,
                        fidelity,
                    );

                    // Add events notifying the creation of the EPR pair
                    // on the master/slave nodes.
                    events.push(Event::new(
                        0.0_f64,
                        EventType::NetworkEvent(NetworkEventData::EprNotified(EprNotifiedData {
                            this_node_id: data.master_node_id,
                            peer_node_id: data.slave_node_id,
                            role: crate::nic::Role::Master,
                            epr_pair_id,
                        })),
                    ));
                    events.push(Event::new(
                        0.0_f64,
                        EventType::NetworkEvent(NetworkEventData::EprNotified(EprNotifiedData {
                            this_node_id: data.slave_node_id,
                            peer_node_id: data.master_node_id,
                            role: crate::nic::Role::Slave,
                            epr_pair_id,
                        })),
                    ));
                }

                // Add event to generate another EPR pair in the future.
                events.push(generator.handle());

                return (events, samples);
            }
        }
        panic!(
            "could not find generator for tx_node_id {} master_node_id {} slave_node_id {}",
            data.tx_node_id, data.master_node_id, data.slave_node_id
        );
    }

    fn handle_epr_notified(
        &mut self,
        now: u64,
        data: EprNotifiedData,
    ) -> (Vec<Event>, Vec<Sample>) {
        // Check consistency.
        assert!(
            data.this_node_id < self.nodes.len() as u32,
            "invalid node identifier {} with {} nodes",
            data.this_node_id,
            self.nodes.len()
        );
        assert!(
            data.peer_node_id < self.nodes.len() as u32,
            "invalid node identifier {} with {} nodes",
            data.peer_node_id,
            self.nodes.len()
        );

        let occupancy = self.nodes[data.this_node_id as usize].epr_established(
            now,
            data.peer_node_id,
            data.role,
            data.epr_pair_id,
        );

        (
            vec![],
            vec![Sample::Series(
                "occupancy".to_string(),
                format!(
                    "{}-{}",
                    data.this_node_id.to_string(),
                    data.peer_node_id.to_string()
                ),
                occupancy,
            )],
        )
    }

    fn handle_epr_fidelity(
        &mut self,
        now: u64,
        data: EprFidelityData,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert!(data.consume_node_id <= self.nodes.len() as u32);
        let fidelity = if let Some((_creation_time, epr_pair_id)) = self.nodes
            [data.consume_node_id as usize]
            .consume(data.consume_node_id, &data.role, data.index)
        {
            if let Some(weight) = self
                .physical_topology
                .graph()
                .node_weight(data.consume_node_id.into())
            {
                if let Some((updated, fidelity)) =
                    self.epr_register.consume(epr_pair_id, data.consume_node_id)
                {
                    assert!(now >= updated);
                    crate::utils::fidelity(
                        fidelity,
                        weight.decay_rate,
                        crate::utils::to_seconds(now - updated),
                    )
                } else {
                    panic!("EPR pair not found {:?}", data);
                }
            } else {
                panic!("no such node {:?}", data);
            }
        } else {
            panic!("no EPR found at {:?}", data);
        };

        (
            vec![],
            vec![Sample::Series(
                "fidelity-node,fidelity-port".to_string(),
                format!("{},{}", data.app_node_id, data.port),
                fidelity,
            )],
        )
    }
}

impl EventHandler for Network {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        match event.event_type {
            EventType::AppEvent(data) => self.handle_app_event(now, data),
            // EventType::NodeEvent(data) => self.handle_os_event(now, data),
            EventType::NetworkEvent(data) => match data {
                NetworkEventData::EprGenerated(data) => self.handle_epr_generated(now, data),
                NetworkEventData::EprNotified(data) => self.handle_epr_notified(now, data),
                NetworkEventData::EprFidelity(data) => self.handle_epr_fidelity(now, data),
            },
            _ => panic!(
                "invalid event {:?} received by a Network object",
                event.event_type
            ),
        }
    }

    /// Kick start all the EPR generators and other nodes' initial events.
    fn initial(&mut self) -> Vec<Event> {
        let mut events = vec![];

        for generators in self.epr_generators.values_mut() {
            for generator in generators {
                events.push(generator.handle());
            }
        }

        for node in &mut self.nodes {
            events.append(&mut node.initial());
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
        let (physical_topology, logical_topology) = crate::tests::logical_topology_2_2();
        let network = Network::new(&logical_topology, physical_topology, 42);
        assert_eq!(10, network.nodes.len());
    }

    #[test]
    fn test_expo_rv() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let rv = rand_distr::Exp::new(10.0).unwrap();
        let mut sum = 0.0;
        for i in 0..100 {
            let x = rv.sample(&mut rng);
            sum += x;
            if i % 5 == 4 {
                println!("\t{}", x);
            } else {
                print!("\t{}", x);
            }
        }
        assert_float_eq::assert_f64_near!(0.1, (((sum / 100.0) * 10.0) as f64).round() / 10.0);
    }
}
