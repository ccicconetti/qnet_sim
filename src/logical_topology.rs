// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::{EdgeRef, IntoNodeReferences};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicalToLogicalPolicy {
    RandomGreedy,
}

#[derive(Debug, Clone, Default, PartialOrd, PartialEq)]
pub struct NodeWeight {
    /// Node label.
    pub label: String,
    /// True if the node is a possible end-node.
    pub is_endnode: bool,
    /// True if the node is a repeater.
    pub is_repeater: bool,
}

impl std::fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialOrd, PartialEq)]
pub struct EdgeWeight {
    /// Index of the node that generates the EPR pair.
    pub tx: u32,
    /// Number of memory qubits reserved for this link.
    pub memory_qubits: u32,
    /// Transmittion rate at which a node generates EPR pairs.
    pub rate: f64,
    /// Cost of the edge, to compute shortest distance.
    pub cost: usize,
}

impl std::fmt::Display for EdgeWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "tx {}, mem {}, cap {}",
            self.tx, self.memory_qubits, self.rate
        )
    }
}

impl petgraph::algo::FloatMeasure for EdgeWeight {
    fn zero() -> Self {
        Self {
            tx: 0,
            memory_qubits: 0,
            rate: 0.0,
            cost: 0,
        }
    }

    fn infinite() -> Self {
        Self {
            tx: 0,
            memory_qubits: 0,
            rate: 0.0,
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
            rate: 0.0,
            cost: self.cost + rhs.cost,
        }
    }
}

type Graph = petgraph::Graph<NodeWeight, EdgeWeight, petgraph::Directed, u32>;
type Paths = std::collections::HashMap<(u32, u32), Vec<u32>>; // s,d -> path

/// Undirected graph representing the logical topology of the network.
///
/// An edge is present if two nodes are receiving EPR pairs by an entangled
/// source generator with some non-zero rate.
/// Both the receiving nodes consume one detector for this purpose and
/// a number of memory qubits.
/// The egress node of the edge is the master, the ingress one is the slave.
///
#[derive(Debug, Default)]
pub struct LogicalTopology {
    graph: Graph,
    paths: Paths,
    pub num_possible_logical_edges: usize,
}

impl LogicalTopology {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Return the label of a given node, from its index.
    pub fn node_label(&self, node_ndx: u32) -> &str {
        &self.graph[petgraph::graph::NodeIndex::from(node_ndx)].label
    }

    /// Return the path between `src` and `dst` in the logical topology.
    ///
    /// Do not use nodes that are not repeaters for intermediate hops.
    ///
    /// Subsequent calls always return the same path for the same
    /// source/destination pair.
    ///
    /// Panic if there is no such path.
    pub fn path(&self, src: u32, dst: u32) -> Vec<u32> {
        assert!(
            src < self.graph.node_count() as u32,
            "invalid src node index {} in the logical topology (count is {})",
            src,
            self.graph.node_count()
        );
        assert!(
            dst < self.graph.node_count() as u32,
            "invalid dst node index {} in the logical topology (count is {})",
            dst,
            self.graph.node_count()
        );

        if let Some(path) = self.paths.get(&(src, dst)) {
            path.clone()
        } else {
            panic!("could not find path from {src} to {dst} in the logical topology");
        }
    }

    /// Create the logical topology from a physical topology using algorithm
    /// specified in `policy`.
    pub fn from_physical_topology(
        policy: &PhysicalToLogicalPolicy,
        physical_topology: &crate::physical_topology::PhysicalTopology,
        rate_computer: &dyn crate::physical_topology::RateComputer,
        rng: &mut rand::rngs::StdRng,
    ) -> anyhow::Result<Self> {
        let possible_logical_edges = find_possible_logical_edges(physical_topology);
        let num_possible_logical_edges = possible_logical_edges.len();
        let graph = match policy {
            PhysicalToLogicalPolicy::RandomGreedy => physical_to_logical_random_greedy(
                physical_topology,
                rate_computer,
                possible_logical_edges,
                rng,
            )?,
        };
        let paths = find_paths(&graph)?;
        Ok(Self {
            graph,
            paths,
            num_possible_logical_edges,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq)]
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

/// Create a logical topology from a physical one using a greedy approach.
///
/// Return the logical graph.
fn physical_to_logical_random_greedy(
    physical_topology: &crate::physical_topology::PhysicalTopology,
    rate_computer: &dyn crate::physical_topology::RateComputer,
    possible_logical_edges: Vec<LogicalEdge>,
    rng: &mut rand::rngs::StdRng,
) -> anyhow::Result<Graph> {
    let mut possible_logical_edges = possible_logical_edges;
    possible_logical_edges.sort();
    crate::utils::shuffle(&mut possible_logical_edges, rng);

    let mut physical_graph = physical_topology.graph().clone();
    let mut logical_graph = Graph::new();

    // Add all nodes from the physical topology.
    for node_weight in physical_graph.node_weights() {
        logical_graph.add_node(NodeWeight {
            label: node_weight.label.clone().unwrap_or_default(),
            is_endnode: matches!(
                node_weight.node_type,
                super::physical_topology::NodeType::OGS
            ),
            is_repeater: node_weight.is_repeater,
        });
    }

    // Keep track of which paths have been found.
    let mut paths_not_found = std::collections::HashSet::new();
    for s in physical_topology.ogs_indices() {
        for d in physical_topology.ogs_indices() {
            paths_not_found.insert((s, d));
        }
    }

    // The loop below terminates when either there are no more possible
    // logical edges to be added or all the paths have been found.
    for logical_edge in possible_logical_edges {
        // Break if all the paths have been found.
        if paths_not_found.is_empty() {
            break;
        }

        // Skip if master and slave are already connected by an edge.
        if logical_graph
            .find_edge(logical_edge.master.into(), logical_edge.slave.into())
            .is_some()
        {
            continue;
        }

        // Skip if end-points do not have each at least one memory qubit.
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
                rate: 0.0,
                cost: 1,
            },
        );

        // Remove any new paths found, if any.
        let repeater_graph = make_repeater_graph(&logical_graph);
        let mut new_paths_found = vec![];
        for (s, d) in &paths_not_found {
            if !find_path(&logical_graph, &repeater_graph, s, d).is_empty() {
                new_paths_found.push((*s, *d));
            }
        }
        for new_path_found in new_paths_found {
            paths_not_found.remove(&new_path_found);
        }
    }

    anyhow::ensure!(
        paths_not_found.is_empty(),
        "could not find logical paths for the following pairs ({} out of {}) for the physical topology below:{:?}\n{:?}\n{:?}",
        paths_not_found.len(),
        physical_topology.ogs_indices().len().pow(2),
        paths_not_found,
        physical_topology,
        logical_graph
    );

    // Assign residual memory qubits as possible, one at a time.
    let mut candidate_edges = vec![];
    for edge in logical_graph.edge_references() {
        candidate_edges.push((edge.source(), edge.target()));
    }
    candidate_edges.sort();
    crate::utils::shuffle(&mut candidate_edges, rng);

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

    // Assign logical edge rates.
    let mut rates = vec![];
    for e in logical_graph.edge_references() {
        assert_eq!(f64::default(), e.weight().rate);

        rates.push((
            e.id(),
            rate_computer.rate(
                &physical_topology,
                e.weight().tx,
                e.source().index() as u32,
                e.target().index() as u32,
            )?,
        ));
    }
    for (e_id, rate) in rates {
        logical_graph.edge_weight_mut(e_id).unwrap().rate = rate;
    }

    Ok(logical_graph)
}

/// Return all possible paths on the logical topology graph from any end node
/// to any other, only crossing repeater nodes.
fn find_paths(logical_graph: &Graph) -> anyhow::Result<Paths> {
    let mut all_paths = std::collections::HashMap::new();

    let repeater_graph = make_repeater_graph(&logical_graph);

    // Collect all the possible end nodes.
    let endnodes: Vec<u32> = logical_graph
        .node_weights()
        .enumerate()
        .filter_map(|(u, w)| if w.is_endnode { Some(u as u32) } else { None })
        .collect();

    // Compute the paths.
    for s in &endnodes {
        for d in &endnodes {
            let path = find_path(logical_graph, &repeater_graph, s, d);
            anyhow::ensure!(
                !path.is_empty(),
                "cannot compute path from {} to {} in the logical graph",
                s,
                d
            );
            all_paths.insert((*s, *d), path);
        }
    }
    Ok(all_paths)
}

/// Return the repeater graph for a logical topology.
///
/// Create a copy of the logical graph containing all the nodes in the
/// original graph, but only the edges that connect any two nodes that are
/// both repeaters.
fn make_repeater_graph(logical_graph: &Graph) -> petgraph::Graph<(), f64> {
    let mut repeater_graph = petgraph::Graph::new();
    for _ in logical_graph.node_indices() {
        repeater_graph.add_node(());
    }
    for e in logical_graph.edge_references() {
        if logical_graph[e.source()].is_repeater && logical_graph[e.target()].is_repeater {
            repeater_graph.add_edge(e.source(), e.target(), 1.0);
        }
    }
    repeater_graph
}

/// Return the path from `s` to `d` in the logical graph, if available,
/// otherwise return an empty vector.
///
/// The `graph_repeater` is a graph containing the same nodes as `logical_graph`
/// but only the edges connecting two repeater nodes.
fn find_path(
    logical_graph: &Graph,
    repeater_graph: &petgraph::Graph<(), f64>,
    s: &u32,
    d: &u32,
) -> Vec<u32> {
    // Add the edges for the source and destination nodes.
    let mut repeater_graph = repeater_graph.clone();
    for e in logical_graph.edges_directed((*s).into(), petgraph::Incoming) {
        repeater_graph.add_edge(e.source(), e.target(), 1.0);
    }
    for e in logical_graph.edges_directed((*s).into(), petgraph::Outgoing) {
        repeater_graph.add_edge(e.source(), e.target(), 1.0);
    }
    for e in logical_graph.edges_directed((*d).into(), petgraph::Incoming) {
        repeater_graph.add_edge(e.source(), e.target(), 1.0);
    }
    for e in logical_graph.edges_directed((*d).into(), petgraph::Outgoing) {
        repeater_graph.add_edge(e.source(), e.target(), 1.0);
    }

    // Find the shortest path from s to d.
    match petgraph::algo::bellman_ford(&repeater_graph, (*s).into()) {
        Ok(paths) => {
            assert!(paths.predecessors.len() == repeater_graph.node_count());
            let mut path = vec![*d];

            let mut cur = *d as usize;
            while cur != *s as usize {
                assert!(cur < paths.predecessors.len());
                if let Some(prev) = paths.predecessors[cur] {
                    cur = prev.index();
                    path.push(cur as u32);
                } else {
                    return vec![];
                }
            }

            path.reverse();

            path
        }
        Err(_err) => {
            panic!(
                "negative cycle detected when finding the path from {} to {}",
                s, d
            );
        }
    }
}

/// Return Ok() if the logical topology is valid.
///
/// A logical topology is valid if:
///
/// - each edge appears at most once between any two nodes
/// - each edge has non-vanishing memory qubits and rate
/// - the cumulative number of memory qubits of physical nodes is not exceeded
/// - the number of tx per node is not exceeded
/// - the number of rx per node is not exceeded
///
/// Parameters:
/// - `logical_topology`: the logical topology to validate.
/// - `physical_topology`: the underlying physical topology.
///
pub fn is_valid(
    logical_topology: &Graph,
    physical_topology: &crate::physical_topology::PhysicalTopology,
) -> anyhow::Result<()> {
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
        anyhow::ensure!(e.weight().rate > 0.0, "vanishing rate for edge {:?}", e);
        anyhow::ensure!(
            e.weight().memory_qubits > 0,
            "vanishing number of qubits for edge {:?}",
            e
        );
    }
    for (u, w) in physical_topology.graph().node_references() {
        let u_ndx = u.index() as u32;

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

/// Find all possible logical edges in a given physical topology.
///
/// Add two edges for each pair of nodes (u,v) that have at least one detector
/// and can be reached by a transmitter tx.
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

    use crate::logical_topology::{is_valid, LogicalTopology, PhysicalToLogicalPolicy};
    use crate::physical_topology::PhysicalTopology;

    use super::{find_paths, find_possible_logical_edges, physical_to_logical_random_greedy};
    use crate::tests::physical_topology_2_2;

    fn get_physical_logical_topology_2_2() -> (PhysicalTopology, LogicalTopology) {
        let physical_topology = physical_topology_2_2();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let logical_topology = LogicalTopology::from_physical_topology(
            &PhysicalToLogicalPolicy::RandomGreedy,
            &physical_topology,
            &crate::physical_topology::FixedRate { rate: 1.0 },
            &mut rng,
        )
        .expect("could not create the logical topology");
        (physical_topology, logical_topology)
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
    fn test_logical_topology_path() {
        let (physical_topology, logical_topology) = get_physical_logical_topology_2_2();
        for src in physical_topology.ogs_indices() {
            for dst in physical_topology.ogs_indices() {
                let path = logical_topology.path(src as u32, dst as u32);
                println!("src {} dst {} path {:?}", src, dst, path);
                assert!(!path.is_empty());

                if src == dst {
                    assert_eq!(src, path[0]);
                } else {
                    assert_eq!(src, *path.first().unwrap());
                    assert_eq!(dst, *path.last().unwrap());
                }
            }
        }
    }

    #[test]
    fn test_logical_topology_node_label() {
        let (_physical_topology, logical_topology) = get_physical_logical_topology_2_2();
        let expected_labels = vec![
            "sat#0", "sat#1", "sat#2", "sat#3", "ogs#0", "ogs#1", "ogs#2", "ogs#3", "ogs#4",
            "ogs#5",
        ];
        let mut actual_labels = vec![];
        for node_ndx in 0..logical_topology.graph.node_count() as u32 {
            actual_labels.push(logical_topology.node_label(node_ndx));
        }
        assert_eq!(expected_labels, actual_labels);
    }

    #[test]
    fn test_logical_topology_physical_to_logical_random_greedy() -> anyhow::Result<()> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let physical_topology = physical_topology_2_2();
        let possible_logical_edges = find_possible_logical_edges(&physical_topology);
        assert_eq!(168, possible_logical_edges.len());
        if let Ok(logical_graph) = physical_to_logical_random_greedy(
            &physical_topology,
            &crate::physical_topology::FixedRate { rate: 1.0 },
            possible_logical_edges,
            &mut rng,
        ) {
            for e in logical_graph.edge_references() {
                println!(
                    "{} -> {}, {:?}",
                    e.source().index(),
                    e.target().index(),
                    e.weight()
                );
            }

            assert!(is_valid(&logical_graph, &physical_topology).is_ok());

            let all_paths = find_paths(&logical_graph)?;

            let ogs_node_ids: std::collections::HashSet<u32> = std::collections::HashSet::from_iter(
                physical_topology.ogs_indices().iter().cloned(),
            );

            for ((s, d), path) in all_paths {
                // Skip non-OGS nodes.
                if !ogs_node_ids.contains(&s) {
                    continue;
                }

                println!(
                    "path from {} to {}: {}",
                    s,
                    d,
                    path.iter()
                        .map(|x| format!("{:?}", x))
                        .collect::<Vec<String>>()
                        .join(",")
                );

                assert!(path.len() <= 9);
            }

            assert_eq!(45, logical_graph.edge_count());
        }

        Ok(())
    }
}
