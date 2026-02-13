// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::Rng;
use rand::SeedableRng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainParams {
    /// Distance between two neighbor satellites, in m.
    pub orbit_to_orbit_distance: f64,
    /// Distance between an OGS and a satellite, in m.
    pub ground_to_orbit_distance: f64,
    /// Number of satellite repeaters.
    pub num_repeaters: u32,
    /// Minimum elevation, in degrees.
    pub elevation_min: f64,
    /// Maximum elevation, in degrees.
    pub elevation_max: f64,
    /// Perfect flag. A "perfect" chain is one with an odd number of
    /// satellite repeaters alternating entangled photon generators and
    /// quantum repeaters, so that there's a single possible logical topology
    /// that interconnects the two on-ground stations at the chain end.
    pub perfect: bool,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000000.0,
            ground_to_orbit_distance: 1000000.0,
            num_repeaters: 1,
            elevation_min: 10.0,
            elevation_max: 60.0,
            perfect: false,
        }
    }
}

fn err_if_not_empty(errors: &[String]) -> anyhow::Result<()> {
    if !errors.is_empty() {
        anyhow::bail!(
            "invalid physical topology chain parameters: {}",
            errors.join(",")
        )
    }
    Ok(())
}
impl super::PhysicalTopologyParams for ChainParams {
    fn valid(&self) -> anyhow::Result<()> {
        let mut errors = vec![];
        if self.orbit_to_orbit_distance < 0.0 {
            errors.push(format!(
                "orbit-to-orbit distance ({}) < 0",
                self.orbit_to_orbit_distance
            ))
        }
        if self.ground_to_orbit_distance < 0.0 {
            errors.push(format!(
                "ground-to-orbit distance ({}) < 0",
                self.ground_to_orbit_distance
            ))
        }
        if self.num_repeaters == 0 {
            errors.push(String::from("vanishing number of repeaters"));
        }
        err_if_not_empty(&errors)
    }

    /// Build a physical topology consisting of an linear chain of repeaters,
    /// with one OGS at each end.
    fn make_graph(
        &self,
        sat_weight: super::NodeWeight,
        ogs_weight: super::NodeWeight,
        seed: u64,
    ) -> anyhow::Result<super::Graph> {
        anyhow::ensure!(
            !self.perfect || self.num_repeaters % 2 == 1,
            "a perfect chain requires an odd number of repeaters"
        );

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        self.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == super::NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == super::NodeType::OGS);

        let mut graph = petgraph::Graph::new_undirected();

        // Add OGS nodes.
        graph.add_node(ogs_weight.clone_with_label(String::from("alice")));
        graph.add_node(ogs_weight.clone_with_label(String::from("bob")));

        // Add SAT nodes.
        for cnt in 0..self.num_repeaters {
            let weight = if self.perfect {
                let mut weight = sat_weight.clone();
                if cnt % 2 == 0 {
                    weight.is_repeater = false;
                    weight.memory_qubits = 0;
                    weight.detectors = 0;
                    weight.clone_with_label(format!("gen#{}", cnt / 2))
                } else {
                    assert!(weight.is_repeater);
                    weight.transmitters = 0;
                    weight.clone_with_label(format!("rep#{}", cnt / 2))
                }
            } else {
                sat_weight.clone_with_label(format!("rep#{}", cnt))
            };

            graph.add_node(weight);
        }

        // Add edges.
        for i in 0..self.num_repeaters {
            let ndx = 2 + i;
            assert!(ndx < graph.node_count() as u32);

            // Left-most satellite: connect to left-side OGS.
            if i == 0 {
                graph.add_edge(
                    ndx.into(),
                    0.into(),
                    super::EdgeWeight {
                        distance: self.ground_to_orbit_distance,
                        elevation: rng.gen_range(self.elevation_min..=self.elevation_max),
                    },
                );
            }

            // Right-most satellite.
            if i == (self.num_repeaters - 1) {
                // Connect to right-side OGS.
                graph.add_edge(
                    ndx.into(),
                    1.into(),
                    super::EdgeWeight {
                        distance: self.ground_to_orbit_distance,
                        elevation: rng.gen_range(self.elevation_min..=self.elevation_max),
                    },
                );
            } else {
                // Connect to right-hand satellite.
                graph.add_edge(
                    ndx.into(),
                    (ndx + 1).into(),
                    super::EdgeWeight {
                        distance: self.orbit_to_orbit_distance,
                        elevation: rng.gen_range(self.elevation_min..=self.elevation_max),
                    },
                );
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use crate::physical_topology::{ChainParams, NodeWeight, PhysicalTopology};

    #[test]
    fn test_physical_topology_from_chain() {
        // Invalid params.
        assert!(PhysicalTopology::new(
            &ChainParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_repeaters: 0,
                elevation_min: 10.0,
                elevation_max: 60.0,
                perfect: false
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .is_err());

        // Valid 4-satellite chain.
        let mut graph = PhysicalTopology::new(
            &ChainParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_repeaters: 4,
                elevation_min: 10.0,
                elevation_max: 60.0,
                perfect: false,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();

        assert_eq!((2..6).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((0..2).collect::<Vec<u32>>(), graph.ogs_indices());
        assert_eq!(6, graph.graph().node_count());
        println!("{}", petgraph::dot::Dot::new(&graph.graph));
        assert_float_eq::assert_f64_near!(11000.0, graph.distance(0, 1).unwrap());
        assert_float_eq::assert_f64_near!(1000.0, graph.distance(0, 2).unwrap());
        assert_float_eq::assert_f64_near!(4000.0, graph.distance(0, 3).unwrap());
        assert_float_eq::assert_f64_near!(7000.0, graph.distance(0, 4).unwrap());
        assert_float_eq::assert_f64_near!(10000.0, graph.distance(0, 5).unwrap());
    }

    #[test]
    fn test_physical_topology_from_perfect_chain() {
        // Invalid params.
        for n in 0..10 {
            assert!(PhysicalTopology::new(
                &ChainParams {
                    orbit_to_orbit_distance: 3000.0,
                    ground_to_orbit_distance: 1000.0,
                    num_repeaters: n * 2,
                    elevation_min: 10.0,
                    elevation_max: 60.0,
                    perfect: true
                },
                NodeWeight::default_sat(),
                NodeWeight::default_ogs(),
                42,
            )
            .is_err());
        }

        // Valid perfect chains.
        for n in 0..10 {
            let graph = PhysicalTopology::new(
                &ChainParams {
                    orbit_to_orbit_distance: 3000.0,
                    ground_to_orbit_distance: 1000.0,
                    num_repeaters: n * 2 + 1,
                    elevation_min: 10.0,
                    elevation_max: 60.0,
                    perfect: true,
                },
                NodeWeight::default_sat(),
                NodeWeight::default_ogs(),
                42,
            )
            .unwrap();

            assert_eq!(
                (2..(2 + n * 2 + 1)).collect::<Vec<u32>>(),
                graph.sat_indices()
            );
            assert_eq!((0..2).collect::<Vec<u32>>(), graph.ogs_indices());
            assert_eq!(n * 2 + 1 + 2, graph.graph().node_count() as u32);
            println!("{}", petgraph::dot::Dot::new(&graph.graph));

            for i in 2..(2 + n * 2 + 1) {
                let w = graph.graph().node_weight(i.into()).unwrap();
                if i % 2 == 0 {
                    assert!(false == w.is_repeater);
                    assert_eq!(0, w.memory_qubits);
                    assert_eq!(0, w.detectors);
                } else {
                    assert!(true == w.is_repeater);
                    assert_eq!(0, w.transmitters);
                }
            }
        }
    }
}
