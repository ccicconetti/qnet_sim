// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum NodeType {
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
    /// Node type.
    pub node_type: NodeType,
    /// Number of memory qubits.
    pub memory_qubits: u32,
    /// Fidelity decay rate of a qubit in memory.
    pub decay_rate: f64,
    /// Entanglement swapping success probability.
    pub swapping_success_prob: f64,
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
        write!(f, "{}", self.node_type)
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
            node_type: NodeType::SAT,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            detectors: 1,
            transmitters: 1,
            capacity: 1.0,
        }
    }

    pub fn default_ogs() -> Self {
        Self {
            node_type: NodeType::OGS,
            memory_qubits: 1,
            decay_rate: 0.0,
            swapping_success_prob: 1.0,
            detectors: 1,
            transmitters: 0,
            capacity: 0.0,
        }
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
        }
    }

    fn infinite() -> Self {
        Self {
            distance: f64::infinite(),
        }
    }
}

impl std::ops::Add for EdgeWeight {
    type Output = EdgeWeight;

    fn add(self, rhs: Self) -> Self::Output {
        EdgeWeight {
            distance: self.distance + rhs.distance,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticFidelities {
    /// One hop, orbit-to-orbit.
    pub f_o: f64,
    /// One hop, orbit-to-ground.
    pub f_g: f64,
    /// Two hops, orbit-to-orbit.
    pub f_oo: f64,
    /// Two hops, orbit-to-ground.
    pub f_og: f64,
    /// Two hops, ground-to-ground.
    pub f_gg: f64,
}

impl Default for StaticFidelities {
    fn default() -> Self {
        Self {
            f_o: 1.0,
            f_g: 1.0,
            f_oo: 1.0,
            f_og: 1.0,
            f_gg: 1.0,
        }
    }
}

impl StaticFidelities {
    fn valid(&self) -> anyhow::Result<()> {
        let fidelities = vec![
            (self.f_o, "one-hop, orbit-to-orbit"),
            (self.f_g, "one-hop, orbit-to-ground"),
            (self.f_oo, "two-hops, orbit-to-orbit"),
            (self.f_og, "two-hops, orbit-to-ground"),
            (self.f_gg, "two-hops, ground-to-ground"),
        ];
        let mut errors = vec![];
        for (fidelity, name) in fidelities {
            if fidelity < 0.0 {
                errors.push(format!("{} fidelity ({}) is < 0", fidelity, name));
            } else if fidelity > 1.0 {
                errors.push(format!("{} fidelity ({}) is > 1", fidelity, name));
            }
        }
        if !errors.is_empty() {
            anyhow::bail!("invalid static fidelities: {}", errors.join(","))
        }
        Ok(())
    }
}

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
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000.0,
            ground_to_orbit_distance: 1000.0,
            num_orbits: 3,
            orbit_length: 4,
        }
    }
}

impl GridParams {
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
        if !errors.is_empty() {
            anyhow::bail!(
                "invalid physical topology grid parameters: {}",
                errors.join(",")
            )
        }
        Ok(())
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
    ) -> anyhow::Result<Self> {
        grid_params.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == NodeType::OGS);
        fidelities.valid()?;

        let mut graph = petgraph::Graph::new_undirected();

        // Add SAT nodes.
        let num_sat = grid_params.orbit_length * grid_params.num_orbits;
        for _ in 0..num_sat {
            graph.add_node(sat_weight.clone());
        }

        // Add OGS nodes.
        let num_ogs = grid_params.orbit_length * (1 + grid_params.num_orbits);
        for _ in 0..num_ogs {
            graph.add_node(ogs_weight.clone());
        }

        // Add orbit-to-orbit edges.
        let orbit_weight = EdgeWeight {
            distance: grid_params.orbit_to_orbit_distance,
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
                    println!("{} {}", ndx, other_ndx);
                    if !graph.contains_edge(other_ndx.into(), ndx.into()) {
                        graph.add_edge(ndx.into(), other_ndx.into(), orbit_weight.clone());
                    }
                }
            }
        }

        // Add ground-to-orbit edges.
        let ground_weight = EdgeWeight {
            distance: grid_params.ground_to_orbit_distance,
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
                        graph.add_edge(ndx.into(), sat_ndx.into(), ground_weight.clone());
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
    fn distance(&mut self, u: u32, v: u32) -> anyhow::Result<f64> {
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
    fn fidelity(&mut self, tx: u32, u: u32, v: u32) -> anyhow::Result<f64> {
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

    fn to_dot(&self) -> String {
        format!("{}", petgraph::dot::Dot::new(&self.graph))
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

#[cfg(test)]
mod tests {
    use crate::physical_topology::{GridParams, NodeWeight};

    use super::{NodeType, PhysicalTopology, StaticFidelities};

    fn test_graph() -> PhysicalTopology {
        //
        //                ┌───┐        ┌───┐
        //         100    │   │  100   │   │   100
        //       ┌────────┤ 1 ├────────┤ 2 ├────────┐
        //       │        │   │        │   │        │
        //       │        └─┬─┘        └─┬─┘        │
        //     ┌─┴─┐        │            │        ┌─┴─┐
        //     │   │        │            │        │   │
        //     │ 0 │        │150         │150     │ 5 │
        //     │   │        │            │        │   │
        //     └─┬─┘        │            │        └─┬─┘
        //       │        ┌─┴─┐        ┌─┴─┐        │
        //       │  100   │   │  100   │   │  100   │
        //       └────────┤ 3 ├────────┤ 4 │────────┘
        //                │   │        │   │
        //                └───┘        └───┘
        //

        PhysicalTopology::from_distances(
            vec![
                (0, 1, 100.0),
                (1, 2, 100.0),
                (2, 5, 100.0),
                (0, 3, 100.0),
                (3, 4, 100.0),
                (4, 5, 100.0),
                (1, 3, 150.0),
                (2, 4, 150.0),
            ],
            StaticFidelities::default(),
        )
    }

    #[test]
    fn test_physical_topology_distance() -> anyhow::Result<()> {
        let mut graph = test_graph();

        assert_float_eq::assert_f64_near!(graph.distance(0, 1).unwrap(), 100.0);
        assert_float_eq::assert_f64_near!(graph.distance(0, 2).unwrap(), 200.0);
        assert_float_eq::assert_f64_near!(graph.distance(0, 5).unwrap(), 300.0);
        assert_float_eq::assert_f64_near!(graph.distance(1, 3).unwrap(), 150.0);
        assert_float_eq::assert_f64_near!(graph.distance(3, 1).unwrap(), 150.0);

        assert!(graph.distance(0, 99).is_err());
        assert!(graph.distance(99, 0).is_err());
        assert!(graph.distance(99, 99).is_err());

        Ok(())
    }

    #[test]
    fn test_physical_topology_dot() {
        let graph = test_graph();
        println!("{}", graph.to_dot());
    }

    #[test]
    fn test_physical_topology_from_grid() {
        // Invalid params
        assert!(PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 0,
                orbit_length: 1,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .is_err());
        assert!(PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 0,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .is_err());
        assert!(PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: -1.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 1,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .is_err());
        assert!(PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: -1.0,
                num_orbits: 1,
                orbit_length: 1,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .is_err());

        // Valid 1x1 grid
        let graph = PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 1,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .unwrap();
        assert_eq!((0..1).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((1..3).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 1x2 grid
        let graph = PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 1,
                orbit_length: 2,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .unwrap();
        assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((2..6).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 2x1 grid
        let graph = PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 2,
                orbit_length: 1,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .unwrap();
        assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((2..5).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 2x2 grid
        let graph = PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 1000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 2,
                orbit_length: 2,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .unwrap();
        assert_eq!((0..4).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((4..10).collect::<Vec<u32>>(), graph.ogs_indices());

        // Valid 4x3 grid
        let mut graph = PhysicalTopology::from_grid_static(
            GridParams {
                orbit_to_orbit_distance: 3000.0,
                ground_to_orbit_distance: 1000.0,
                num_orbits: 3,
                orbit_length: 4,
            },
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            StaticFidelities::default(),
        )
        .unwrap();

        assert_eq!((0..12).collect::<Vec<u32>>(), graph.sat_indices());
        assert_eq!((12..28).collect::<Vec<u32>>(), graph.ogs_indices());
        assert_eq!(28, graph.graph().node_count());
        println!("{}", graph.to_dot());
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 1).unwrap());
        assert_float_eq::assert_f64_near!(4000.0, graph.distance(0, 2).unwrap());
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 3).unwrap());
        assert_float_eq::assert_f64_near!(2000.0, graph.distance(0, 4).unwrap());
        assert_float_eq::assert_f64_near!(4000.0, graph.distance(0, 11).unwrap());
        assert_float_eq::assert_f64_near!(6000.0, graph.distance(12, 26).unwrap());
    }

    #[test]
    fn test_physical_topology_fidelities() {
        let fidelities = StaticFidelities {
            f_o: 0.6,
            f_g: 0.7,
            f_oo: 0.8,
            f_og: 0.9,
            f_gg: 1.0,
        };

        let mut topo = PhysicalTopology::from_distances(
            vec![
                (0, 1, 1.0),
                (0, 2, 1.0),
                (0, 3, 1.0),
                (0, 4, 1.0),
                (4, 5, 1.0),
            ],
            fidelities.clone(),
        );

        topo.graph.node_weight_mut(0.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(1.into()).unwrap().node_type = NodeType::OGS;
        topo.graph.node_weight_mut(2.into()).unwrap().node_type = NodeType::OGS;
        topo.graph.node_weight_mut(3.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(4.into()).unwrap().node_type = NodeType::SAT;
        topo.graph.node_weight_mut(5.into()).unwrap().node_type = NodeType::SAT;

        assert_eq!(fidelities.f_o, topo.fidelity(0, 0, 3).unwrap());
        assert_eq!(fidelities.f_o, topo.fidelity(0, 3, 0).unwrap());
        assert_eq!(fidelities.f_g, topo.fidelity(0, 0, 1).unwrap());
        assert_eq!(fidelities.f_g, topo.fidelity(0, 1, 0).unwrap());
        assert_eq!(fidelities.f_oo, topo.fidelity(0, 3, 4).unwrap());
        assert_eq!(fidelities.f_og, topo.fidelity(0, 1, 3).unwrap());
        assert_eq!(fidelities.f_gg, topo.fidelity(0, 1, 2).unwrap());

        assert!(topo.fidelity(0, 0, 5).is_err());
        assert!(topo.fidelity(0, 5, 0).is_err());
        assert!(topo.fidelity(0, 1, 5).is_err());
        assert!(topo.fidelity(0, 1, 1).is_err());
        assert!(topo.fidelity(0, 0, 0).is_err());
        assert!(topo.fidelity(0, 99, 1).is_err());
        assert!(topo.fidelity(99, 1, 2).is_err());
    }
}
