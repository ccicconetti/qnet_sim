// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::physical_topology::{ChainParams, FidelityComputer, GridParams, NodeWeight};

use crate::physical_topology::{
    NodeType, PhysicalTopology, PhysicalTopologyParams, StaticFidelities,
};

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
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
    .is_err());
    assert!(GridParams {
        orbit_to_orbit_distance: 3000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 1,
        orbit_length: 0,
        elevation_min: 10.0,
        elevation_max: 60.0
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
    .is_err());
    assert!(GridParams {
        orbit_to_orbit_distance: -1.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 1,
        orbit_length: 1,
        elevation_min: 10.0,
        elevation_max: 60.0
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
    .is_err());
    assert!(GridParams {
        orbit_to_orbit_distance: 1000.0,
        ground_to_orbit_distance: -1.0,
        num_orbits: 1,
        orbit_length: 1,
        elevation_min: 10.0,
        elevation_max: 60.0
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
    .is_err());

    // Valid 1x1 grid
    let graph = GridParams {
        orbit_to_orbit_distance: 1000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 1,
        orbit_length: 1,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
    .unwrap();
    assert_eq!((0..1).collect::<Vec<u32>>(), graph.sat_indices());
    assert_eq!((1..3).collect::<Vec<u32>>(), graph.ogs_indices());

    // Valid 1x2 grid
    let graph = GridParams {
        orbit_to_orbit_distance: 1000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 1,
        orbit_length: 2,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
    .unwrap();
    assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
    assert_eq!((2..6).collect::<Vec<u32>>(), graph.ogs_indices());

    // Valid 2x1 grid
    let graph = GridParams {
        orbit_to_orbit_distance: 1000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 2,
        orbit_length: 1,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
    .unwrap();
    assert_eq!((0..2).collect::<Vec<u32>>(), graph.sat_indices());
    assert_eq!((2..5).collect::<Vec<u32>>(), graph.ogs_indices());

    // Valid 2x2 grid
    let graph = GridParams {
        orbit_to_orbit_distance: 1000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 2,
        orbit_length: 2,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
    .unwrap();
    assert_eq!((0..4).collect::<Vec<u32>>(), graph.sat_indices());
    assert_eq!((4..10).collect::<Vec<u32>>(), graph.ogs_indices());

    // Valid 4x3 grid
    let mut graph = GridParams {
        orbit_to_orbit_distance: 3000.0,
        ground_to_orbit_distance: 1000.0,
        num_orbits: 3,
        orbit_length: 4,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
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

#[test]
fn test_physical_topology_from_chain() {
    // Invalid params.
    assert!(ChainParams {
        orbit_to_orbit_distance: 3000.0,
        ground_to_orbit_distance: 1000.0,
        num_repeaters: 0,
        elevation_min: 10.0,
        elevation_max: 60.0
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42,)
    .is_err());

    // Valid 4-satellite chain.
    let mut graph = ChainParams {
        orbit_to_orbit_distance: 3000.0,
        ground_to_orbit_distance: 1000.0,
        num_repeaters: 4,
        elevation_min: 10.0,
        elevation_max: 60.0,
    }
    .make_topology(NodeWeight::default_sat(), NodeWeight::default_ogs(), 42)
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
fn test_static_fidelities() {
    let fidelities = StaticFidelities {
        f_o: 0.6,
        f_g: 0.7,
        f_oo: 0.8,
        f_og: 0.9,
        f_gg: 1.0,
    };

    let mut topo = PhysicalTopology::from_distances(vec![
        (0, 1, 1.0),
        (0, 2, 1.0),
        (0, 3, 1.0),
        (0, 4, 1.0),
        (4, 5, 1.0),
    ]);

    topo.graph.node_weight_mut(0.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(1.into()).unwrap().node_type = NodeType::OGS;
    topo.graph.node_weight_mut(2.into()).unwrap().node_type = NodeType::OGS;
    topo.graph.node_weight_mut(3.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(4.into()).unwrap().node_type = NodeType::SAT;
    topo.graph.node_weight_mut(5.into()).unwrap().node_type = NodeType::SAT;

    assert_eq!(fidelities.f_o, fidelities.fidelity(&topo, 0, 0, 3).unwrap());
    assert_eq!(fidelities.f_o, fidelities.fidelity(&topo, 0, 3, 0).unwrap());
    assert_eq!(fidelities.f_g, fidelities.fidelity(&topo, 0, 0, 1).unwrap());
    assert_eq!(fidelities.f_g, fidelities.fidelity(&topo, 0, 1, 0).unwrap());
    assert_eq!(
        fidelities.f_oo,
        fidelities.fidelity(&topo, 0, 3, 4).unwrap()
    );
    assert_eq!(
        fidelities.f_og,
        fidelities.fidelity(&topo, 0, 1, 3).unwrap()
    );
    assert_eq!(
        fidelities.f_gg,
        fidelities.fidelity(&topo, 0, 1, 2).unwrap()
    );

    assert!(fidelities.fidelity(&topo, 0, 0, 5).is_err());
    assert!(fidelities.fidelity(&topo, 0, 5, 0).is_err());
    assert!(fidelities.fidelity(&topo, 0, 1, 5).is_err());
    assert!(fidelities.fidelity(&topo, 0, 1, 1).is_err());
    assert!(fidelities.fidelity(&topo, 0, 0, 0).is_err());
    assert!(fidelities.fidelity(&topo, 0, 99, 1).is_err());
    assert!(fidelities.fidelity(&topo, 99, 1, 2).is_err());
}
