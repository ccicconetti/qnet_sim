// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;

pub fn physical_topology_2_2() -> crate::physical_topology::PhysicalTopology {
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

pub fn logical_topology_2_2() -> (
    crate::physical_topology::PhysicalTopology,
    crate::logical_topology::LogicalTopology,
) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let num_tries = 10;

    for _try in 0..num_tries {
        let physical_topology = physical_topology_2_2();
        let logical_topology = crate::logical_topology::LogicalTopology::from_physical_topology(
            &crate::logical_topology::PhysicalToLogicalPolicy::RandomGreedy,
            &physical_topology,
            &mut rng,
        )
        .expect("could not derive logical graph from physical topology");

        if crate::logical_topology::is_valid(&logical_topology.graph(), &physical_topology).is_ok()
        {
            return (physical_topology, logical_topology);
        }
    }
    panic!(
        "could not find a feasible logical topology in {} tries",
        num_tries
    );
}
