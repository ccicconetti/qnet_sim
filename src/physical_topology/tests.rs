// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::physical_topology::PhysicalTopology;

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

    PhysicalTopology::from_distances(vec![
        (0, 1, 100.0),
        (1, 2, 100.0),
        (2, 5, 100.0),
        (0, 3, 100.0),
        (3, 4, 100.0),
        (4, 5, 100.0),
        (1, 3, 150.0),
        (2, 4, 150.0),
    ])
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
    let physical_topology = test_graph();
    println!("{}", petgraph::dot::Dot::new(&physical_topology.graph));
}

pub fn test_topo() -> PhysicalTopology {
    use crate::physical_topology::NodeType;

    let mut topo = PhysicalTopology::from_distances(vec![
        (0, 1, 200000.0),
        (0, 2, 200000.0),
        (0, 3, 200000.0),
        (0, 4, 200000.0),
        (4, 5, 200000.0),
    ]);

    topo.graph.node_weight_mut(0.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(1.into()).unwrap().node_type = NodeType::OGS;
    topo.graph.node_weight_mut(2.into()).unwrap().node_type = NodeType::OGS;
    topo.graph.node_weight_mut(3.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(4.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(5.into()).unwrap().node_type = NodeType::SAT;

    topo
}
