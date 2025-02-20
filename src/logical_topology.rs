// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

struct LogicalTopology {}

impl LogicalTopology {}

fn find_possible_logical_edges(
    physical_topology: &crate::physical_topology::PhysicalTopology,
) -> Vec<(u32, u32, u32)> {
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

            // The same node may be a rx, too
            if u_w.detectors > 0 {
                rx_candidates.push(u.index());
            }

            // Add all possibile combinations (quadratic).
            for i in 0..rx_candidates.len() {
                for j in 0..i {
                    assert_ne!(rx_candidates[i], rx_candidates[j]);
                    ret.push((
                        u.index() as u32,
                        rx_candidates[i] as u32,
                        rx_candidates[j] as u32,
                    ));
                }
            }
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::find_possible_logical_edges;

    #[test]
    fn test_logical_topology_find_possible_logical_edges() {
        let physical_topology = crate::physical_topology::PhysicalTopology::from_grid_static(
            crate::physical_topology::GridParams {
                orbit_to_orbit_distance: 1.0,
                ground_to_orbit_distance: 1.0,
                num_orbits: 2,
                orbit_length: 2,
            },
            crate::physical_topology::NodeWeight::default_sat(),
            crate::physical_topology::NodeWeight::default_ogs(),
            crate::physical_topology::StaticFidelities::default(),
        )
        .expect("invalid physical topology");

        let res = find_possible_logical_edges(&physical_topology);

        println!("{:?}", res);
    }
}
