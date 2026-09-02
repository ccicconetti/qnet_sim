// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod chain_params;
pub mod fidelity_computer;
pub mod file_params;
pub mod fixed_rate;
pub mod grid_params;
pub mod leo_fidelities;
pub mod leo_rates;
pub mod rate_computer;
pub mod static_fidelities;
#[cfg(test)]
pub mod tests;

pub use crate::physical_topology::chain_params::ChainParams;
pub use crate::physical_topology::fidelity_computer::FidelityComputer;
pub use crate::physical_topology::file_params::FileParams;
pub use crate::physical_topology::fixed_rate::FixedRate;
pub use crate::physical_topology::grid_params::GridParams;
pub use crate::physical_topology::leo_fidelities::LeoFidelities;
pub use crate::physical_topology::leo_rates::LeoRates;
pub use crate::physical_topology::rate_computer::RateComputer;
pub use crate::physical_topology::static_fidelities::StaticFidelities;

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
    pub label: Option<String>,
    /// Node type.
    pub node_type: NodeType,
    /// True if this node can perform entanglement swapping.
    pub is_repeater: bool,
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
}

impl std::fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(label) = &self.label {
            write!(f, "{}", label)
        } else {
            write!(f, "{}", self.node_type)
        }
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
            label: None,
            node_type: NodeType::SAT,
            is_repeater: true,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            swapping_duration: 0.001,
            correction_duration: 0.0,
            detectors: 1,
            transmitters: 1,
        }
    }

    pub fn default_ogs() -> Self {
        Self {
            label: None,
            node_type: NodeType::OGS,
            is_repeater: false,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            swapping_duration: 0.0,
            correction_duration: 0.001,
            detectors: 1,
            transmitters: 0,
        }
    }

    pub fn clone_with_label(&self, label: String) -> Self {
        let mut clone = self.clone();
        clone.label = Some(label);
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
    pub distance: f64,
    /// Elevation angle, in degrees.
    pub elevation: f64,
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
    paths: std::collections::HashMap<
        u32,
        petgraph::algo::bellman_ford::Paths<petgraph::graph::NodeIndex, EdgeWeight>,
    >,
}

impl PhysicalTopology {
    pub fn new(
        params: &dyn PhysicalTopologyParams,
        sat_weight: NodeWeight,
        ogs_weight: NodeWeight,
        seed: u64,
    ) -> anyhow::Result<Self> {
        Ok(PhysicalTopology {
            graph: params.make_graph(sat_weight, ogs_weight, seed)?,
            paths: std::collections::HashMap::new(),
        })
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
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

    /// Return the label of a node.
    pub fn node_label(&self, node: u32) -> anyhow::Result<String> {
        self.node_valid(node)?;
        Ok(self.graph.node_weight(node.into()).unwrap().to_string())
    }

    /// Return the id of a node given its label.
    pub fn node_id_by_label(&self, label: &str) -> anyhow::Result<u32> {
        for (ndx, w) in self.graph.node_weights().enumerate() {
            if w.to_string() == label {
                return Ok(ndx as u32);
            }
        }
        anyhow::bail!("node with label {} not found in the graph", label)
    }

    /// Check if a node is valid.
    fn node_valid(&self, node: u32) -> anyhow::Result<()> {
        anyhow::ensure!(
            (node as usize) < self.graph.node_count(),
            "there's no node {:?} in the graph",
            node
        );
        anyhow::ensure!(
            self.graph.node_weight(node.into()).is_some(),
            "there's no node weight associated with {:?} in the graph",
            node
        );
        Ok(())
    }

    /// Return the distance from node u to node v, in m.
    /// The paths are computed in a lazy manner.
    pub fn distance(&mut self, u: u32, v: u32) -> anyhow::Result<f64> {
        self.node_valid(u)?;
        self.node_valid(v)?;
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

    /// Create a topology of default nodes with given distances.
    #[cfg(test)]
    fn from_distances(edges: Vec<(u32, u32, f64)>) -> Self {
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
            paths: std::collections::HashMap::new(),
        }
    }
}

pub trait PhysicalTopologyParams {
    /// Check if the physical topology configuration is valid.
    fn valid(&self) -> anyhow::Result<()>;

    /// Build a physical topology with all the satellite and ground nodes
    /// having the same given characteristics.
    fn make_graph(
        &self,
        sat_weight: NodeWeight,
        ogs_weight: NodeWeight,
        seed: u64,
    ) -> anyhow::Result<Graph>;
}
