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
    /// Minimum eleveation, in degrees.
    pub elevation_min: f64,
    /// Maximum elevation, in degrees.
    pub elevation_max: f64,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000000.0,
            ground_to_orbit_distance: 1000000.0,
            num_repeaters: 1,
            elevation_min: 10.0,
            elevation_max: 60.0,
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
    fn make_topology(
        &self,
        sat_weight: super::NodeWeight,
        ogs_weight: super::NodeWeight,
        seed: u64,
    ) -> anyhow::Result<super::PhysicalTopology> {
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
            graph.add_node(sat_weight.clone_with_label(format!("rep#{}", cnt)));
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

        Ok(super::PhysicalTopology {
            graph,
            paths: std::collections::HashMap::new(),
        })
    }
}
