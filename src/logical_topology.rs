// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::{EdgeRef, IntoNodeReferences};
use shuffle::shuffler::Shuffler;

const NEGLIGIBLE_AMOUNT: f64 = 1e-5;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicalToLogicalPolicy {
    RandomGreedy,
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct NodeWeight {}

impl std::fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct EdgeWeight {
    /// Index of the node that generates the EPR pair.
    pub tx: u32,
    /// Number of memory qubits reserved for this link.
    pub memory_qubits: u32,
    /// Capacity of tx, i.e., rate at which it generates EPR pairs.
    pub capacity: f64,
    /// Cost of the edge, to compute shortest distance.
    pub cost: usize,
}

impl std::fmt::Display for EdgeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tx {}, mem {}, cap {}",
            self.tx, self.memory_qubits, self.capacity
        )
    }
}

impl petgraph::algo::FloatMeasure for EdgeWeight {
    fn zero() -> Self {
        Self {
            tx: 0,
            memory_qubits: 0,
            capacity: 0.0,
            cost: 0,
        }
    }

    fn infinite() -> Self {
        Self {
            tx: 0,
            memory_qubits: 0,
            capacity: 0.0,
            cost: usize::MAX / 2,
        }
    }
}

impl std::ops::Add for EdgeWeight {
    type Output = EdgeWeight;

    fn add(self, rhs: Self) -> Self::Output {
        EdgeWeight {
            tx: 0,
            memory_qubits: 0,
            capacity: 0.0,
            cost: self.cost + rhs.cost,
        }
    }
}

type Graph = petgraph::Graph<NodeWeight, EdgeWeight, petgraph::Directed, u32>;
type Paths = std::collections::HashMap<
    u32,
    petgraph::algo::bellman_ford::Paths<petgraph::graph::NodeIndex, EdgeWeight>,
>;

/// Undirected graph representing the logical topology of the network.
///
/// An edge is present if two nodes are receiving EPR pairs by an entangled
/// source generator with some non-zero capacity.
/// Both the receiving nodes consume one detector for this purpose and
/// a number of memory qubits.
/// The egress node of the edge is the master, the ingress one is the slave.
///
#[derive(Debug, Default)]
pub struct LogicalTopology {
    graph: Graph,
    paths: Paths,
}

impl LogicalTopology {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn from_physical_topology(
        policy: &PhysicalToLogicalPolicy,
        physical_topology: &crate::physical_topology::PhysicalTopology,
        rng: &mut rand::rngs::StdRng,
    ) -> anyhow::Result<Self> {
        let graph = match policy {
            PhysicalToLogicalPolicy::RandomGreedy => {
                physical_to_logical_random_greedy(physical_topology, rng)?
            }
        };
        let paths = find_paths(&graph)?;
        Ok(Self { graph, paths })
    }
}

#[derive(Debug, Default, Clone)]
struct LogicalEdge {
    pub tx: u32,
    pub master: u32,
    pub slave: u32,
}

impl LogicalEdge {
    fn swap_master_slave(&self) -> Self {
        Self {
            tx: self.tx,
            master: self.slave,
            slave: self.master,
        }
    }
}

impl std::fmt::Display for LogicalEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}->{} ({})", self.master, self.slave, self.tx)
    }
}

macro_rules! weight {
    ($node:expr,$physical_graph:expr) => {
        $physical_graph.node_weight_mut($node.into()).unwrap()
    };
}

fn physical_to_logical_random_greedy(
    physical_topology: &crate::physical_topology::PhysicalTopology,
    rng: &mut rand::rngs::StdRng,
) -> anyhow::Result<Graph> {
    let mut possible_logical_edges = find_possible_logical_edges(physical_topology);
    let mut irs = shuffle::irs::Irs::default();
    let _ = irs.shuffle(&mut possible_logical_edges, rng);

    let mut physical_graph = physical_topology.graph().clone();

    let mut logical_graph = Graph::new();

    // Add all nodes from the physical topology.
    for _ in 0..physical_graph.node_count() {
        logical_graph.add_node(NodeWeight {});
    }

    // Save OGS nodes.
    let ogs_nodes = physical_topology.ogs_indices();

    for logical_edge in possible_logical_edges {
        // Skip if master and slave are already connected by an edge.
        if logical_graph
            .find_edge(logical_edge.master.into(), logical_edge.slave.into())
            .is_some()
        {
            continue;
        }

        // Skip if end-points do not have each at least one  memory qubit.
        if weight!(logical_edge.master, physical_graph).memory_qubits == 0
            || weight!(logical_edge.slave, physical_graph).memory_qubits == 0
        {
            continue;
        }

        // Skip if end-points do not have each an available detector.
        if weight!(logical_edge.master, physical_graph).detectors == 0
            || weight!(logical_edge.slave, physical_graph).detectors == 0
        {
            continue;
        }

        // Skip if tx does not have a transmitter available.
        if weight!(logical_edge.tx, physical_graph).transmitters == 0 {
            continue;
        }

        // Reserve one memory qubit and one detector in the master/slave nodes
        // and a transmitter in the tx node.
        weight!(logical_edge.master, physical_graph).memory_qubits -= 1;
        weight!(logical_edge.slave, physical_graph).memory_qubits -= 1;
        weight!(logical_edge.master, physical_graph).detectors -= 1;
        weight!(logical_edge.slave, physical_graph).detectors -= 1;
        weight!(logical_edge.tx, physical_graph).transmitters -= 1;

        // Add the edge to the logical topology.
        logical_graph.add_edge(
            logical_edge.master.into(),
            logical_edge.slave.into(),
            EdgeWeight {
                tx: logical_edge.tx,
                memory_qubits: 1,
                capacity: 0.0,
                cost: 1,
            },
        );

        // Break as soon as all the OGS nodes can reach one another.
        if reachable(&logical_graph, &ogs_nodes) {
            break;
        }
    }

    anyhow::ensure!(
        reachable(&logical_graph, &ogs_nodes),
        "could not find a logical topology for the given physical topology"
    );

    // Assign residual memory qubits as possible, one at a time.
    let mut candidate_edges = vec![];
    for edge in logical_graph.edge_references() {
        candidate_edges.push((edge.source(), edge.target()));
    }
    let mut irs = shuffle::irs::Irs::default();
    let _ = irs.shuffle(&mut candidate_edges, rng);

    while !candidate_edges.is_empty() {
        let mut candidate_edges_new = vec![];
        while let Some((u, v)) = candidate_edges.pop() {
            if physical_graph.node_weight(u).unwrap().memory_qubits > 0
                && physical_graph.node_weight(v).unwrap().memory_qubits > 0
            {
                logical_graph
                    .edge_weight_mut(logical_graph.find_edge(u, v).unwrap())
                    .unwrap()
                    .memory_qubits += 1;
                physical_graph.node_weight_mut(u).unwrap().memory_qubits -= 1;
                physical_graph.node_weight_mut(v).unwrap().memory_qubits -= 1;
                candidate_edges_new.push((u, v));
            }
        }
        std::mem::swap(&mut candidate_edges, &mut candidate_edges_new);
    }

    // Assign logical edge capacities, by dividing evenly for each node
    // between the number of logical edges crossing that node.
    for (u, w) in physical_graph.node_references() {
        let u_ndx = u.index() as u32;

        // Count how many logical edges are served by this node as the tx.
        let num_served = logical_graph
            .edge_references()
            .filter(|e| e.weight().tx == u_ndx)
            .count();

        // Skip edges that do not serve as tx.
        if num_served == 0 {
            continue;
        }

        // Divide equally the capacity between logical edges.
        let even_capacity = w.capacity / num_served as f64;

        // Assign it to all the logical edges.
        for w in logical_graph.edge_weights_mut() {
            if w.tx == u_ndx {
                w.capacity = even_capacity;
            }
        }
    }

    Ok(logical_graph)
}

/// Return all possible paths on the logical topology graph from any source node
/// to all others.
fn find_paths(logical_graph: &Graph) -> anyhow::Result<Paths> {
    let mut all_paths = std::collections::HashMap::new();
    for source in logical_graph.node_indices() {
        match petgraph::algo::bellman_ford(&logical_graph, source) {
            Ok(local_paths) => {
                all_paths.insert(source.index() as u32, local_paths);
            }
            Err(_err) => anyhow::bail!(
                "cannot compute path from {}: negative cycle",
                source.index()
            ),
        }
    }
    Ok(all_paths)
}

/// Return Ok() if the logical topology is valid.
///
/// A logical topology is valid if:
///
/// - any OGS node can reach any other
/// - each edge appears at most once between any two nodes
/// - each edge has non-vanishing memory qubits and capacity
/// - the sum of the capacity of transmitters is not exceeded
/// - the cumulative number of memory qubits of physical nodes is not exceeded
/// - the number of tx per node is not exceeded
/// - the number of rx per node is not exceeded
///
/// Parameters:
/// - `logical_topology`: the logical topology to validate.
/// - `physical_topology`: the underlying physical topology.
///
fn is_valid(
    logical_topology: &Graph,
    physical_topology: &crate::physical_topology::PhysicalTopology,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        reachable(logical_topology, &physical_topology.ogs_indices()),
        "there is some OGS that cannot be reached by another OGS"
    );
    for e in logical_topology.edge_references() {
        anyhow::ensure!(
            logical_topology
                .edges_connecting(e.source(), e.target())
                .count()
                == 1,
            "wrong number of edges {}->{}",
            e.source().index(),
            e.target().index()
        );
        anyhow::ensure!(
            e.weight().capacity > 0.0,
            "vanishing capacity for edge {:?}",
            e
        );
        anyhow::ensure!(
            e.weight().memory_qubits > 0,
            "vanishing number of qubits for edge {:?}",
            e
        );
    }
    for (u, w) in physical_topology.graph().node_references() {
        let u_ndx = u.index() as u32;
        let sum_capacity: f64 = logical_topology
            .edge_weights()
            .filter(|e| e.tx == u_ndx)
            .map(|e| e.capacity)
            .sum();
        anyhow::ensure!(
            w.capacity >= sum_capacity || (sum_capacity - w.capacity) < NEGLIGIBLE_AMOUNT,
            "tx capacity of node {} exceeded: {} > {}",
            u_ndx,
            sum_capacity,
            w.capacity
        );

        let sum_memory_qubits: u32 = logical_topology
            .edge_references()
            .filter(|e| e.source() == u || e.target() == u)
            .map(|e| e.weight().memory_qubits)
            .sum();
        anyhow::ensure!(
            w.memory_qubits >= sum_memory_qubits,
            "memory qubits of node {} exceeded: {} > {}",
            u_ndx,
            sum_memory_qubits,
            w.memory_qubits
        );

        let sum_detectors: u32 = logical_topology
            .edge_references()
            .filter(|e| e.source() == u || e.target() == u)
            .count() as u32;
        anyhow::ensure!(
            w.detectors >= sum_detectors,
            "detectors of node {} exceeded: {} > {}",
            u_ndx,
            sum_detectors,
            w.detectors
        );

        let sum_transmitters: u32 = logical_topology
            .edge_references()
            .filter(|e| e.weight().tx == u_ndx)
            .count() as u32;
        anyhow::ensure!(
            w.transmitters >= sum_transmitters,
            "transmitters of node {} exceeded: {} > {}",
            u_ndx,
            sum_transmitters,
            w.detectors
        );
    }
    Ok(())
}

/// Return true if any node can reach any other via the given graph.
fn reachable(graph: &Graph, nodes: &Vec<u32>) -> bool {
    for u in nodes {
        match petgraph::algo::bellman_ford(&graph, (*u).into()) {
            Ok(paths) => {
                for v in nodes {
                    if *u == *v {
                        continue;
                    }
                    if paths.predecessors[*v as usize].is_none() {
                        return false;
                    }
                }
            }
            Err(_err) => return false,
        }
    }
    true
}

/// Find all possible logical edges in a given physical topology.
///
/// Add two edges for each pair of nodes (u,v) that have at least one detector
/// and can be reached by a transmitter tx with non-zero capacity.
///
/// Return a vector of tuples (tx,u,v).
fn find_possible_logical_edges(
    physical_topology: &crate::physical_topology::PhysicalTopology,
) -> Vec<LogicalEdge> {
    let mut ret = vec![];
    let graph = physical_topology.graph();

    for u in graph.node_indices() {
        let u_w = graph.node_weight(u).unwrap();
        if u_w.transmitters > 0 {
            let mut rx_candidates = vec![];
            // Find all neighbors that can be an rx
            for v in graph.neighbors(u) {
                if graph.node_weight(v).unwrap().detectors > 0 {
                    rx_candidates.push(v.index());
                }
            }

            // The same node may be an rx, too
            if u_w.detectors > 0 {
                rx_candidates.push(u.index());
            }

            // Add all possibile combinations (quadratic).
            for i in 0..rx_candidates.len() {
                for j in 0..i {
                    assert_ne!(rx_candidates[i], rx_candidates[j]);
                    let logical_edge = LogicalEdge {
                        tx: u.index() as u32,
                        master: rx_candidates[i] as u32,
                        slave: rx_candidates[j] as u32,
                    };
                    let logical_edge_swapped = logical_edge.swap_master_slave();
                    ret.push(logical_edge);
                    ret.push(logical_edge_swapped);
                }
            }
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use petgraph::visit::EdgeRef;
    use rand::SeedableRng;

    use crate::logical_topology::is_valid;

    use super::{find_paths, find_possible_logical_edges, physical_to_logical_random_greedy};

    fn physical_topology_2_2() -> crate::physical_topology::PhysicalTopology {
        crate::physical_topology::PhysicalTopology::from_grid_static(
            crate::physical_topology::GridParams {
                orbit_to_orbit_distance: 1.0,
                ground_to_orbit_distance: 1.0,
                num_orbits: 2,
                orbit_length: 2,
            },
            crate::physical_topology::NodeWeight {
                node_type: crate::physical_topology::NodeType::SAT,
                memory_qubits: 10,
                decay_rate: 1.0,
                swapping_success_prob: 0.5,
                detectors: 10,
                transmitters: 10,
                capacity: 1.0,
            },
            crate::physical_topology::NodeWeight {
                node_type: crate::physical_topology::NodeType::OGS,
                memory_qubits: 20,
                decay_rate: 1.0,
                swapping_success_prob: 0.0,
                detectors: 10,
                transmitters: 0,
                capacity: 0.0,
            },
            crate::physical_topology::StaticFidelities::default(),
        )
        .expect("invalid physical topology")
    }

    #[test]
    fn test_logical_topology_find_possible_logical_edges() {
        let physical_topology = physical_topology_2_2();
        let res = find_possible_logical_edges(&physical_topology);

        assert_eq!(168, res.len());

        let sat_indices: std::collections::HashSet<u32> =
            std::collections::HashSet::from_iter(physical_topology.sat_indices().iter().cloned());
        let ogs_indices: std::collections::HashSet<u32> =
            std::collections::HashSet::from_iter(physical_topology.ogs_indices().iter().cloned());
        for e in &res {
            assert!(sat_indices.contains(&e.tx));
            assert!(sat_indices.contains(&e.master) || ogs_indices.contains(&e.master));
            assert!(sat_indices.contains(&e.slave) || ogs_indices.contains(&e.slave));
        }
    }

    #[test]
    fn test_logical_topology_physical_to_logical_random_greedy() -> anyhow::Result<()> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for _try in 0..10 {
            let physical_topology = physical_topology_2_2();
            let logical_graph = physical_to_logical_random_greedy(&physical_topology, &mut rng)?;

            for e in logical_graph.edge_references() {
                println!(
                    "{} -> {}, {:?}",
                    e.source().index(),
                    e.target().index(),
                    e.weight()
                );
            }

            if is_valid(&logical_graph, &physical_topology).is_err() {
                continue;
            }

            let all_paths = find_paths(&logical_graph)?;

            for (source, paths) in all_paths {
                assert!(paths.distances.iter().map(|x| x.cost).max().unwrap() <= 4);
                println!(
                    "distances of {}: {}",
                    source,
                    paths
                        .distances
                        .iter()
                        .map(|x| format!("{}", x.cost))
                        .collect::<Vec<String>>()
                        .join(",")
                );
            }

            return Ok(());
        }

        anyhow::bail!("test failed");
    }
}
