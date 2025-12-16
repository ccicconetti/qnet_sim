// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::Rng;
use rand::SeedableRng;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GridParams {
    /// Distance between two neighbor satellites, in m.
    pub orbit_to_orbit_distance: f64,
    /// Distance between an OGS and a satellite, in m.
    pub ground_to_orbit_distance: f64,
    /// Number of orbits.
    pub num_orbits: u32,
    /// Number of satellites in each orbit.
    pub orbit_length: u32,
    /// Minimum eleveation, in degrees.
    pub elevation_min: f64,
    /// Maximum elevation, in degrees.
    pub elevation_max: f64,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000000.0,
            ground_to_orbit_distance: 1000000.0,
            num_orbits: 3,
            orbit_length: 4,
            elevation_min: 10.0,
            elevation_max: 60.0,
        }
    }
}

fn err_if_not_empty(errors: &[String]) -> anyhow::Result<()> {
    if !errors.is_empty() {
        anyhow::bail!(
            "invalid physical topology grid parameters: {}",
            errors.join(",")
        )
    }
    Ok(())
}

impl super::PhysicalTopologyParams for GridParams {
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
        if self.num_orbits == 0 {
            errors.push(String::from("vanishing number of orbits"));
        }
        if self.orbit_length == 0 {
            errors.push(String::from("vanishing orbit length"));
        }
        err_if_not_empty(&errors)
    }

    /// Build a physical topology consisting of a grid representing a number of
    /// parallel orbits, with inter-orbit communications. The grid wraps around
    /// at the orbits' end.
    ///
    /// Exactly one station is assigned to each square of 4 satellites (if in
    /// the middle) or pair of satellites (if at the top/bottom).
    fn make_graph(
        &self,
        sat_weight: super::NodeWeight,
        ogs_weight: super::NodeWeight,
        seed: u64,
    ) -> anyhow::Result<super::Graph> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        self.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == super::NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == super::NodeType::OGS);

        let mut graph = petgraph::Graph::new_undirected();

        // Add SAT nodes.
        let num_sat = self.orbit_length * self.num_orbits;
        for cnt in 0..num_sat {
            graph.add_node(sat_weight.clone_with_label(format!("sat#{}", cnt)));
        }

        // Add OGS nodes.
        let num_ogs = self.orbit_length * (1 + self.num_orbits);
        for cnt in 0..num_ogs {
            graph.add_node(ogs_weight.clone_with_label(format!("ogs#{}", cnt)));
        }

        // Add orbit-to-orbit edges.

        let orbit_weight = super::EdgeWeight {
            distance: self.orbit_to_orbit_distance,
            elevation: 0.0,
        };
        for i in 0..self.num_orbits {
            for j in 0..self.orbit_length {
                let ndx = j + i * self.orbit_length;
                assert!(ndx < num_sat);
                let mut others = std::collections::HashSet::new();
                // Right
                others.insert(i * self.orbit_length + (j + 1) % self.orbit_length);
                // Left
                others.insert(
                    i * self.orbit_length + (self.orbit_length + j - 1) % self.orbit_length,
                );
                // Up
                if i != 0 {
                    others.insert(ndx - self.orbit_length);
                }
                // Down
                if i != (self.num_orbits - 1) {
                    others.insert(ndx + self.orbit_length);
                }
                for other_ndx in others {
                    assert!(other_ndx < num_sat);
                    if !graph.contains_edge(other_ndx.into(), ndx.into()) {
                        let mut orbit_weight = orbit_weight.clone();
                        orbit_weight.elevation =
                            rng.gen_range(self.elevation_min..=self.elevation_max);
                        graph.add_edge(ndx.into(), other_ndx.into(), orbit_weight);
                    }
                }
            }
        }

        // Add ground-to-orbit edges.
        let ground_weight = super::EdgeWeight {
            distance: self.ground_to_orbit_distance,
            elevation: 0.0,
        };
        for i in 0..=self.num_orbits {
            for j in 0..self.orbit_length {
                let ndx = num_sat + j + i * self.orbit_length;
                assert!(ndx < num_sat + num_ogs);
                let mut sats = std::collections::HashSet::new();
                // Up
                if i != 0 {
                    sats.insert((i - 1) * self.orbit_length + j);
                    sats.insert(
                        (i - 1) * self.orbit_length
                            + (self.orbit_length + j - 1) % self.orbit_length,
                    );
                }
                // Down
                if i != self.num_orbits {
                    sats.insert(i * self.orbit_length + j);
                    sats.insert(
                        i * self.orbit_length + (self.orbit_length + j - 1) % self.orbit_length,
                    );
                }
                for sat_ndx in sats {
                    assert!(sat_ndx < num_sat);
                    if !graph.contains_edge(sat_ndx.into(), ndx.into()) {
                        let mut ground_weight = ground_weight.clone();
                        ground_weight.elevation =
                            rng.gen_range(self.elevation_min..=self.elevation_max);
                        graph.add_edge(ndx.into(), sat_ndx.into(), ground_weight);
                    }
                }
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use crate::physical_topology::{GridParams, NodeWeight, PhysicalTopologyParams};

    #[test]
    fn test_physical_topology_from_grid() {
        // Invalid params
        assert!(GridParams {
            orbit_to_orbit_distance: 3000.0,
            ground_to_orbit_distance: 1000.0,
            num_orbits: 0,
            orbit_length: 1,
            elevation_min: 10.0,
            elevation_max: 60.0
        }
        .make_graph(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
        .is_err());
        assert!(GridParams {
            orbit_to_orbit_distance: 3000.0,
            ground_to_orbit_distance: 1000.0,
            num_orbits: 1,
            orbit_length: 0,
            elevation_min: 10.0,
            elevation_max: 60.0
        }
        .make_graph(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
        .is_err());
        assert!(GridParams {
            orbit_to_orbit_distance: -1.0,
            ground_to_orbit_distance: 1000.0,
            num_orbits: 1,
            orbit_length: 1,
            elevation_min: 10.0,
            elevation_max: 60.0
        }
        .make_graph(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
        .is_err());
        assert!(GridParams {
            orbit_to_orbit_distance: 1000.0,
            ground_to_orbit_distance: -1.0,
            num_orbits: 1,
            orbit_length: 1,
            elevation_min: 10.0,
            elevation_max: 60.0
        }
        .make_graph(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
        .is_err());

        // Valid 1x1 grid

        let graph = crate::physical_topology::PhysicalTopology::new(
            &GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 1,
                elevation_min: 10.0,
                elevation_max: 60.0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();
        assert_eq!((0..1).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((1..3).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 1x2 grid
        let graph = crate::physical_topology::PhysicalTopology::new(
            &GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 2,
                elevation_min: 10.0,
                elevation_max: 60.0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();
        assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((2..6).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 2x1 grid
        let graph = crate::physical_topology::PhysicalTopology::new(
            &GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 2,
                orbit_length: 1,
                elevation_min: 10.0,
                elevation_max: 60.0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();
        assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((2..5).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 2x2 grid
        let graph = crate::physical_topology::PhysicalTopology::new(
            &GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 2,
                orbit_length: 2,
                elevation_min: 10.0,
                elevation_max: 60.0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();
        assert_eq!((0..4).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((4..10).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 4x3 grid
        let mut graph = crate::physical_topology::PhysicalTopology::new(
            &GridParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 3,
                orbit_length: 4,
                elevation_min: 10.0,
                elevation_max: 60.0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();

        assert_eq!((0..12).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((12..28).collect::<Vec<u32>>(), graph.ogs_indices());
        assert_eq!(28, graph.graph().node_count());
        println!("{}", petgraph::dot::Dot::new(&graph.graph));
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 1).unwrap());
        assert_float_eq::assert_f64_near!(4000.0, graph.distance(0, 2).unwrap());
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 3).unwrap());
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 4).unwrap());
        assert_float_eq::assert_f64_near!(4000.0, graph.distance(0, 11).unwrap());
        assert_float_eq::assert_f64_near!(6000.0, graph.distance(12, 26).unwrap());
    }
}
