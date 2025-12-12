// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
enum InputType {
    #[default]
    LEO = 0,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileParams {
    /// Input file type.
    input_type: InputType,
    /// Input file path.
    input_path: String,
}

impl FileParams {
    pub fn valid(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            std::path::Path::new(&self.input_path).exists(),
            "physical topology input file '{}' does not exist",
            self.input_path
        );

        Ok(())
    }

    /// Build a physical topology reading from an input file.
    ///
    /// All the satellite and ground nodes have the same given characteristics.
    /// and static fidelities.
    pub fn make_topology(
        file_params: FileParams,
        sat_weight: super::NodeWeight,
        ogs_weight: super::NodeWeight,
        fidelities: super::StaticFidelities,
        seed: u64,
    ) -> anyhow::Result<super::PhysicalTopology> {
        // chain_params.valid()?;
        // sat_weight.valid()?;
        // assert!(sat_weight.node_type == super::NodeType::SAT);
        // ogs_weight.valid()?;
        // assert!(ogs_weight.node_type == super::NodeType::OGS);
        // fidelities.valid()?;

        // let mut graph = petgraph::Graph::new_undirected();

        // // Add OGS nodes.
        // graph.add_node(ogs_weight.clone_with_label(String::from("alice")));
        // graph.add_node(ogs_weight.clone_with_label(String::from("bob")));

        // // Add SAT nodes.
        // for cnt in 0..chain_params.num_repeaters {
        //     graph.add_node(sat_weight.clone_with_label(format!("rep#{}", cnt)));
        // }

        // // Add edges.
        // for i in 0..chain_params.num_repeaters {
        //     let ndx = 2 + i;
        //     assert!(ndx < graph.node_count() as u32);

        //     // Left-most satellite: connect to left-side OGS.
        //     if i == 0 {
        //         graph.add_edge(
        //             ndx.into(),
        //             0.into(),
        //             super::EdgeWeight {
        //                 distance: chain_params.ground_to_orbit_distance,
        //                 elevation: rng
        //                     .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
        //             },
        //         );
        //     }

        //     // Right-most satellite.
        //     if i == (chain_params.num_repeaters - 1) {
        //         // Connect to right-side OGS.
        //         graph.add_edge(
        //             ndx.into(),
        //             1.into(),
        //             super::EdgeWeight {
        //                 distance: chain_params.ground_to_orbit_distance,
        //                 elevation: rng
        //                     .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
        //             },
        //         );
        //     } else {
        //         // Connect to right-hand satellite.
        //         graph.add_edge(
        //             ndx.into(),
        //             (ndx + 1).into(),
        //             super::EdgeWeight {
        //                 distance: chain_params.orbit_to_orbit_distance,
        //                 elevation: rng
        //                     .gen_range(chain_params.elevation_min..=chain_params.elevation_max),
        //             },
        //         );
        //     }
        // }

        // Ok(super::PhysicalTopology {
        //     graph,
        //     fidelities,
        //     paths: std::collections::HashMap::new(),
        // })
        todo!()
    }
}
