// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Graph representing the physical topology of the network.
/// An edge is present if two nodes can establish a quantum/classical link
/// with one another.
///
/// Type: undirected.
/// Node weights: None.
/// Edge weights: distance, in m.
#[derive(Debug, Default)]
pub struct PhysicalTopology {
    pub graph: petgraph::Graph<(), f64, petgraph::Undirected, u32>,
    paths: std::collections::HashMap<
        petgraph::graph::NodeIndex,
        petgraph::algo::bellman_ford::Paths<petgraph::graph::NodeIndex, f64>,
    >,
}

impl PhysicalTopology {
    fn from_vec(edges: Vec<(u32, u32, f64)>) -> Self {
        let mut graph = petgraph::Graph::new_undirected();
        graph.extend_with_edges(edges);
        Self {
            graph,
            paths: std::collections::HashMap::new(),
        }
    }

    /// Return the distance from node u to node v, in m.
    /// The paths are computed in a lazy manner.
    fn distance(
        &mut self,
        u: petgraph::graph::NodeIndex,
        v: petgraph::graph::NodeIndex,
    ) -> anyhow::Result<f64> {
        anyhow::ensure!(
            u.index() < self.graph.node_count(),
            "there's no node {:?} in the graph",
            u
        );
        anyhow::ensure!(
            v.index() < self.graph.node_count(),
            "there's no node {:?} in the graph",
            v
        );
        if let Some(paths) = self.paths.get(&u) {
            if let Some(_pred) = paths.predecessors[v.index()] {
                Ok(paths.distances[v.index()])
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
}

#[cfg(test)]
mod tests {
    use super::PhysicalTopology;

    #[test]
    fn test_physical_topology_distance() -> anyhow::Result<()> {
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

        let mut graph = PhysicalTopology::from_vec(vec![
            (0, 1, 100.0),
            (1, 2, 100.0),
            (2, 5, 100.0),
            (0, 3, 100.0),
            (3, 4, 100.0),
            (4, 5, 100.0),
            (1, 3, 150.0),
            (2, 4, 150.0),
        ]);

        assert_float_eq::assert_f64_near!(graph.distance(0.into(), 1.into()).unwrap(), 100.0);
        assert_float_eq::assert_f64_near!(graph.distance(0.into(), 2.into()).unwrap(), 200.0);
        assert_float_eq::assert_f64_near!(graph.distance(0.into(), 5.into()).unwrap(), 300.0);
        assert_float_eq::assert_f64_near!(graph.distance(1.into(), 3.into()).unwrap(), 150.0);
        assert_float_eq::assert_f64_near!(graph.distance(3.into(), 1.into()).unwrap(), 150.0);

        assert!(graph.distance(0.into(), 99.into()).is_err());
        assert!(graph.distance(99.into(), 0.into()).is_err());
        assert!(graph.distance(99.into(), 99.into()).is_err());

        Ok(())
    }
}
