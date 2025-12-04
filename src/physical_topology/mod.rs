// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod chain_params;
pub mod grid_params;
pub mod static_fidelities;
#[cfg(test)]
pub mod tests;

pub use crate::physical_topology::chain_params::ChainParams;
pub use crate::physical_topology::grid_params::GridParams;
pub use crate::physical_topology::static_fidelities::StaticFidelities;

use rand::Rng;
use rand::SeedableRng;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    /// Satellite node.
    SAT,
    /// On ground station.
    OGS,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NodeType::SAT => "SAT",
                NodeType::OGS => "OGS",
            }
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeWeight {
    /// Node label.
    pub label: String,
    /// Node type.
    pub node_type: NodeType,
    /// Number of memory qubits.
    pub memory_qubits: u32,
    /// Fidelity decay rate of a qubit in memory.
    pub decay_rate: f64,
    /// Entanglement swapping success probability.
    pub swapping_success_prob: f64,
    /// Entanglement swapping duration, in s.
    pub swapping_duration: f64,
    /// Duration of the local operations to correct end-to-end pairs, in s.
    pub correction_duration: f64,
    /// Number of detectors.
    pub detectors: u32,
    /// Number of transmitters, i.e., entangled photon source generators.
    pub transmitters: u32,
    /// Capacity of transmitters, i.e., rate at which they generate
    /// EPR pairs.
    pub capacity: f64,
}

impl std::fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

impl Default for NodeWeight {
    fn default() -> Self {
        NodeWeight::default_sat()
    }
}

impl NodeWeight {
    pub fn default_sat() -> Self {
        Self {
            label: String::default(),
            node_type: NodeType::SAT,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            swapping_duration: 0.001,
            correction_duration: 0.0,
            detectors: 1,
            transmitters: 1,
            capacity: 1.0,
        }
    }

    pub fn default_ogs() -> Self {
        Self {
            label: String::default(),
            node_type: NodeType::OGS,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            swapping_duration: 0.0,
            correction_duration: 0.001,
            detectors: 1,
            transmitters: 0,
            capacity: 0.0,
        }
    }

    pub fn clone_with_label(&self, label: String) -> Self {
        let mut clone = self.clone();
        clone.label = label;
        clone
    }

    fn valid(&self) -> anyhow::Result<()> {
        let mut errors = vec![];
        if self.memory_qubits == 0 && self.detectors > 0 {
            errors.push(format!(
                "vanishing memory qubits with {} detectors",
                self.detectors
            ))
        }
        if self.memory_qubits > 0 && self.detectors == 0 {
            errors.push(format!(
                "vanishing detectors with {} memory qubits",
                self.memory_qubits
            ))
        }
        if self.decay_rate < 0.0 {
            errors.push(format!("decay rate ({}) < 0", self.decay_rate))
        }
        if self.swapping_success_prob < 0.0 || self.swapping_success_prob > 1.0 {
            errors.push(format!(
                "invalid swapping success probability ({})",
                self.swapping_success_prob
            ))
        }
        if self.capacity < 0.0 {
            errors.push(format!("capacity ({}) < 0", self.capacity))
        }

        if !errors.is_empty() {
            anyhow::bail!(
                "invalid physical topology grid parameters: {}",
                errors.join(",")
            )
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct EdgeWeight {
    /// Distance between two nodes, in m.
    distance: f64,
    /// Elevation angle, in degrees.
    elevation: f64,
}

impl std::fmt::Display for EdgeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.distance)
    }
}

impl petgraph::algo::FloatMeasure for EdgeWeight {
    fn zero() -> Self {
        Self {
            distance: f64::zero(),
            elevation: f64::zero(),
        }
    }

    fn infinite() -> Self {
        Self {
            distance: f64::infinite(),
            elevation: f64::zero(),
        }
    }
}

impl std::ops::Add for EdgeWeight {
    type Output = EdgeWeight;

    fn add(self, rhs: Self) -> Self::Output {
        EdgeWeight {
            distance: self.distance + rhs.distance,
            elevation: 0.0,
        }
    }
}

macro_rules! valid_node {
    ($node:expr, $graph:expr) => {
        anyhow::ensure!(
            ($node as usize) < $graph.node_count(),
            "there's no node {:?} in the graph",
            $node
        );
        anyhow::ensure!(
            $graph.node_weight($node.into()).is_some(),
            "there's no node weight associated with {:?} in the graph",
            $node
        );
    };
}

type Graph = petgraph::Graph<NodeWeight, EdgeWeight, petgraph::Undirected, u32>;

/// Undirected graph representing the physical topology of the network.
///
/// An edge is present if two nodes can establish a quantum/classical link
/// with one another.
///
/// A simple fidelity model for the EPR pairs generated is used, with fixed
/// values depending only on whether the generation is one or two hops and
/// if it is STA-STA or STA-OGS.
#[derive(Debug, Default)]
pub struct PhysicalTopology {
    graph: Graph,
    fidelities: StaticFidelities,
    paths: std::collections::HashMap<
        u32,
        petgraph::algo::bellman_ford::Paths<petgraph::graph::NodeIndex, EdgeWeight>,
    >,
}

impl PhysicalTopology {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Build a physical topology consisting of a grid representing a number of
    /// parallel orbits, with inter-orbit communications. The grid wraps around
    /// at the orbits' end.
    ///
    /// Exactly one station is assigned to each square of 4 satellites (if in
    /// the middle) or pair of satellites (if at the top/bottom).
    ///
    /// All the satellite and ground nodes have the same given characteristics.
    /// and static fidelities.
    pub fn from_grid_static(
        grid_params: GridParams,
        sat_weight: NodeWeight,
        ogs_weight: NodeWeight,
        fidelities: StaticFidelities,
        seed: u64,
    ) -> anyhow::Result<Self> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        grid_params.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == NodeType::OGS);
        fidelities.valid()?;

        let mut graph = petgraph::Graph::new_undirected();

        // Add SAT nodes.
        let num_sat = grid_params.orbit_length * grid_params.num_orbits;
        for cnt in 0..num_sat {
            graph.add_node(sat_weight.clone_with_label(format!("sat#{}", cnt)));
        }

        // Add OGS nodes.
        let num_ogs = grid_params.orbit_length * (1 + grid_params.num_orbits);
        for cnt in 0..num_ogs {
            graph.add_node(ogs_weight.clone_with_label(format!("ogs#{}", cnt)));
        }

        // Add orbit-to-orbit edges.

        let orbit_weight = EdgeWeight {
            distance: grid_params.orbit_to_orbit_distance,
            elevation: rng.gen_range(grid_params.elevation_min..=grid_params.elevation_max),
        };
        for i in 0..grid_params.num_orbits {
            for j in 0..grid_params.orbit_length {
                let ndx = j + i * grid_params.orbit_length;
                assert!(ndx < num_sat);
                let mut others = std::collections::HashSet::new();
                // Right
                others.insert(i * grid_params.orbit_length + (j + 1) % grid_params.orbit_length);
                // Left
                others.insert(
                    i * grid_params.orbit_length
                        + (grid_params.orbit_length + j - 1) % grid_params.orbit_length,
                );
                // Up
                if i != 0 {
                    others.insert(ndx - grid_params.orbit_length);
                }
                // Down
                if i != (grid_params.num_orbits - 1) {
                    others.insert(ndx + grid_params.orbit_length);
                }
                for other_ndx in others {
                    assert!(other_ndx < num_sat);
                    if !graph.contains_edge(other_ndx.into(), ndx.into()) {
                        graph.add_edge(ndx.into(), other_ndx.into(), orbit_weight);
                    }
                }
            }
        }

        // Add ground-to-orbit edges.
        let ground_weight = EdgeWeight {
            distance: grid_params.ground_to_orbit_distance,
            elevation: rng.gen_range(grid_params.elevation_min..=grid_params.elevation_max),
        };
        for i in 0..=grid_params.num_orbits {
            for j in 0..grid_params.orbit_length {
                let ndx = num_sat + j + i * grid_params.orbit_length;
                assert!(ndx < num_sat + num_ogs);
                let mut sats = std::collections::HashSet::new();
                // Up
                if i != 0 {
                    sats.insert((i - 1) * grid_params.orbit_length + j);
                    sats.insert(
                        (i - 1) * grid_params.orbit_length
                            + (grid_params.orbit_length + j - 1) % grid_params.orbit_length,
                    );
                }
                // Down
                if i != grid_params.num_orbits {
                    sats.insert(i * grid_params.orbit_length + j);
                    sats.insert(
                        i * grid_params.orbit_length
                            + (grid_params.orbit_length + j - 1) % grid_params.orbit_length,
                    );
                }
                for sat_ndx in sats {
                    assert!(sat_ndx < num_sat);
                    if !graph.contains_edge(sat_ndx.into(), ndx.into()) {
                        graph.add_edge(ndx.into(), sat_ndx.into(), ground_weight);
                    }
                }
            }
        }

        Ok(Self {
            graph,
            fidelities,
            paths: std::collections::HashMap::new(),
        })
    }

    /// Build a physical topology consisting of an linear chain of repeaters,
    /// with one OGS at each end.
    ///
    /// All the satellite and ground nodes have the same given characteristics.
    /// and static fidelities.
    pub fn from_chain_static(
        chain_params: ChainParams,
        sat_weight: NodeWeight,
        ogs_weight: NodeWeight,
        fidelities: StaticFidelities,
        seed: u64,
    ) -> anyhow::Result<Self> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        chain_params.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == NodeType::OGS);
        fidelities.valid()?;

        let mut graph = petgraph::Graph::new_undirected();

        // Add OGS nodes.
        graph.add_node(ogs_weight.clone_with_label(String::from("alice")));
        graph.add_node(ogs_weight.clone_with_label(String::from("bob")));

        // Add SAT nodes.
        for cnt in 0..chain_params.num_repeaters {
            graph.add_node(sat_weight.clone_with_label(format!("rep#{}", cnt)));
        }

        // Add edges.
        for i in 0..chain_params.num_repeaters {
            let ndx = 2 + i;
            assert!(ndx < graph.node_count() as u32);

            // Left-most satellite: connect to left-side OGS.
            if i == 0 {
                graph.add_edge(
                    ndx.into(),
                    0.into(),
                    EdgeWeight {
                        distance: chain_params.ground_to_orbit_distance,
                        elevation: rng
                            .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
                    },
                );
            }

            // Right-most satellite.
            if i == (chain_params.num_repeaters - 1) {
                // Connect to right-side OGS.
                graph.add_edge(
                    ndx.into(),
                    1.into(),
                    EdgeWeight {
                        distance: chain_params.ground_to_orbit_distance,
                        elevation: rng
                            .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
                    },
                );
            } else {
                // Connect to right-hand satellite.
                graph.add_edge(
                    ndx.into(),
                    (ndx + 1).into(),
                    EdgeWeight {
                        distance: chain_params.orbit_to_orbit_distance,
                        elevation: rng
                            .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
                    },
                );
            }
        }

        Ok(Self {
            graph,
            fidelities,
            paths: std::collections::HashMap::new(),
        })
    }

    /// Return the indices of the in-orbit satelites.
    pub fn sat_indices(&self) -> Vec<u32> {
        self.node_indices(NodeType::SAT)
    }

    /// Return the indices of the on-ground stations.
    pub fn ogs_indices(&self) -> Vec<u32> {
        self.node_indices(NodeType::OGS)
    }

    fn node_indices(&self, node_type: NodeType) -> Vec<u32> {
        let mut ret = vec![];
        for (ndx, w) in self.graph.node_weights().enumerate() {
            if w.node_type == node_type {
                ret.push(ndx as u32);
            }
        }
        ret
    }

    /// Return the distance from node u to node v, in m.
    /// The paths are computed in a lazy manner.
    pub fn distance(&mut self, u: u32, v: u32) -> anyhow::Result<f64> {
        valid_node!(u, self.graph);
        valid_node!(v, self.graph);
        if let Some(paths) = self.paths.get(&u) {
            if let Some(_pred) = paths.predecessors[v as usize] {
                Ok(paths.distances[v as usize].distance)
            } else {
                anyhow::bail!("no connection between {:?} and {:?}", u, v);
            }
        } else {
            match petgraph::algo::bellman_ford(&self.graph, u.into()) {
                Ok(paths) => {
                    self.paths.insert(u, paths);
                    self.distance(u, v)
                }
                Err(_err) => anyhow::bail!(
                    "cannot compute distance from {:?} to {:?}: negative cycle",
                    u,
                    v
                ),
            }
        }
    }

    /// Return the initial fidelity of the EPR pairs generated by the given
    /// transmitter towards the two nodes specified. Return error if `tx` does not
    /// have a transmitter or there is no edge between `tx` and `u` or `v`.
    ///
    /// Parameters:
    /// - `tx`: the node that generates EPR pairs
    /// - `u`: one of the nodes that receives one photon of the EPR pairs
    /// - `v`: the other one
    pub fn fidelity(&self, tx: u32, u: u32, v: u32) -> anyhow::Result<f64> {
        valid_node!(tx, self.graph);
        valid_node!(u, self.graph);
        valid_node!(v, self.graph);
        let tx = petgraph::graph::NodeIndex::from(tx);
        let u = petgraph::graph::NodeIndex::from(u);
        let v = petgraph::graph::NodeIndex::from(v);
        anyhow::ensure!(
            self.graph.node_weight(tx).unwrap().transmitters > 0,
            "there are no transmitters on board of {}",
            tx.index()
        );
        anyhow::ensure!(
            u != v,
            "rx nodes are the same: {} = {}",
            u.index(),
            v.index()
        );
        anyhow::ensure!(
            matches!(self.graph.node_weight(tx).unwrap().node_type, NodeType::SAT),
            "node is an OGS and cannot be a transmitter: {}",
            tx.index()
        );

        if tx == u {
            anyhow::ensure!(
                self.graph.find_edge(tx, v).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                v.index()
            );
            match self.graph.node_weight(v).unwrap().node_type {
                NodeType::SAT => Ok(self.fidelities.f_o),
                NodeType::OGS => Ok(self.fidelities.f_g),
            }
        } else if tx == v {
            anyhow::ensure!(
                self.graph.find_edge(tx, u).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                u.index()
            );
            match self.graph.node_weight(u).unwrap().node_type {
                NodeType::SAT => Ok(self.fidelities.f_o),
                NodeType::OGS => Ok(self.fidelities.f_g),
            }
        } else {
            anyhow::ensure!(
                self.graph.find_edge(tx, u).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                u.index()
            );
            anyhow::ensure!(
                self.graph.find_edge(tx, v).is_some(),
                "there is no edge between nodes {} and {}",
                tx.index(),
                v.index()
            );
            match self.graph.node_weight(u).unwrap().node_type {
                NodeType::SAT => match self.graph.node_weight(v).unwrap().node_type {
                    NodeType::SAT => Ok(self.fidelities.f_oo),
                    NodeType::OGS => Ok(self.fidelities.f_og),
                },
                NodeType::OGS => match self.graph.node_weight(v).unwrap().node_type {
                    NodeType::SAT => Ok(self.fidelities.f_og),
                    NodeType::OGS => Ok(self.fidelities.f_gg),
                },
            }
        }
    }

    /// Create a topology of default nodes with given distances.
    #[cfg(test)]
    fn from_distances(edges: Vec<(u32, u32, f64)>, fidelities: StaticFidelities) -> Self {
        let mut graph = petgraph::Graph::new_undirected();

        graph.extend_with_edges(edges.iter().map(|(u, v, distance)| {
            (
                *u,
                *v,
                EdgeWeight {
                    distance: *distance,
                    elevation: 42.0,
                },
            )
        }));
        Self {
            graph,
            fidelities,
            paths: std::collections::HashMap::new(),
        }
    }
}
