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

impl super::PhysicalTopologyParams for FileParams {
    fn valid(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            std::path::Path::new(&self.input_path).exists(),
            "physical topology input file '{}' does not exist",
            self.input_path
        );

        Ok(())
    }

    /// Build a physical topology reading from an input file.
    ///
    /// All the satellite and ground nodes have the same given characteristics
    /// specified by `sat_weight` and `ogs_weight`, respectively.
    fn make_graph(
        &self,
        sat_weight: super::NodeWeight,
        ogs_weight: super::NodeWeight,
        _seed: u64,
    ) -> anyhow::Result<super::Graph> {
        self.valid()?;
        sat_weight.valid()?;
        assert!(sat_weight.node_type == super::NodeType::SAT);
        ogs_weight.valid()?;
        assert!(ogs_weight.node_type == super::NodeType::OGS);

        struct NodeSpec {
            label: String,
            node_type: super::NodeType,
        }
        struct EdgeSpec {
            src: u32,
            dst: u32,
            distance: f64,
            elevation: f64,
        }

        // Read from file.
        let mut nodes = vec![];
        let mut edges = vec![];

        match &self.input_type {
            InputType::LEO => {
                #[derive(Debug)]
                struct Record {
                    node1: u32,
                    node2: u32,
                    distance: f64,
                    is_ground_sat: bool,
                    elevation: f64,
                }
                let mut ogs_nodes = std::collections::HashSet::new();
                let mut all_nodes = std::collections::HashSet::new();
                let mut records = vec![];

                let content = std::fs::read_to_string(&self.input_path)?;
                for (lineno, line) in content.lines().enumerate() {
                    if line.starts_with("#") {
                        continue;
                    }
                    let line = line.replace("\t", " ");
                    let line = line.trim();
                    let tokens: Vec<&str> = line.split(" ").filter(|x| !x.is_empty()).collect();
                    if tokens.len() == 0 {
                        continue;
                    }
                    anyhow::ensure!(
                        tokens.len() == 5,
                        "wrong input at line {} in '{}' ({} fields): {}",
                        lineno + 1,
                        self.input_path,
                        tokens.len(),
                        line
                    );
                    let record = Record {
                        node1: tokens[0].parse::<u32>()?,
                        node2: tokens[1].parse::<u32>()?,
                        distance: tokens[2].parse::<f64>()? * 1000.0_f64,
                        is_ground_sat: if tokens[3].parse::<u32>()? == 0 {
                            false
                        } else {
                            true
                        },
                        elevation: tokens[4].parse::<f64>()?,
                    };
                    all_nodes.insert(record.node1);
                    all_nodes.insert(record.node2);
                    if record.is_ground_sat {
                        ogs_nodes.insert(record.node2);
                    }
                    records.push(record);
                }

                let mut all_nodes: Vec<u32> = all_nodes.iter().cloned().collect();
                all_nodes.sort();

                let mut label_to_id = std::collections::HashMap::new();
                for (id, label) in all_nodes.iter().enumerate() {
                    label_to_id.insert(*label, id as u32);
                    if ogs_nodes.contains(label) {
                        nodes.push(NodeSpec {
                            label: format!("ogs#{}", label),
                            node_type: super::NodeType::OGS,
                        });
                    } else {
                        nodes.push(NodeSpec {
                            label: format!("sat#{}", label),
                            node_type: super::NodeType::SAT,
                        });
                    }
                }

                for record in records {
                    edges.push(EdgeSpec {
                        src: label_to_id[&record.node1],
                        dst: label_to_id[&record.node2],
                        distance: record.distance,
                        elevation: record.elevation,
                    });
                }
            }
        };

        let mut graph = petgraph::Graph::new_undirected();

        // Add nodes.
        for node in nodes {
            match node.node_type {
                super::NodeType::OGS => graph.add_node(ogs_weight.clone_with_label(node.label)),
                super::NodeType::SAT => graph.add_node(sat_weight.clone_with_label(node.label)),
            };
        }

        // Add edges.
        for edge in edges {
            graph.add_edge(
                edge.src.into(),
                edge.dst.into(),
                super::EdgeWeight {
                    distance: edge.distance,
                    elevation: edge.elevation,
                },
            );
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::FileParams;
    use crate::physical_topology::{NodeWeight, PhysicalTopology};
    use std::io::Write;

    #[test]
    fn test_physical_topology_from_file_leo() -> anyhow::Result<()> {
        //
        // topology:
        //
        //       100         200
        //   4 -------- 5 -------- 7
        //   |                     |
        //   | 100                 | 300
        //   |                     |
        //   +--------- 6 ---------+
        //
        // mapping:
        // 4 -> 0, 5 -> 1, 6 -> 2, 7 -> 3
        //

        let remove_me_dir = crate::utils::RemoveMeDir::new("test_physical_topology_from_file_leo")?;

        let mut path = remove_me_dir.dir();
        path.push("topo.txt");

        {
            let mut outfile = std::fs::OpenOptions::new()
                .write(true)
                .append(false)
                .create(true)
                .truncate(true)
                .open(&path)?;

            let _ = writeln!(
                outfile,
                r#"# node1	node2	distance    is_ground_sat   elevation         
4	5	100	0	10

5	7	200	0	20
6	4	100	1	30
6	7	300 1	40
"#
            );
        }

        let file_params = FileParams {
            input_type: super::InputType::LEO,
            input_path: path.to_str().unwrap().to_string(),
        };

        let mut graph = PhysicalTopology::new(
            &file_params,
            NodeWeight::default_sat(),
            NodeWeight::default_ogs(),
            42,
        )
        .unwrap();

        println!("{}", petgraph::dot::Dot::new(&graph.graph));

        assert_eq!(vec![1, 2], graph.sat_indices());
        assert_eq!(vec![0, 3], graph.ogs_indices());
        assert_eq!(4, graph.graph().node_count());
        assert_float_eq::assert_f64_near!(100000.0, graph.distance(0, 1).unwrap());
        assert_float_eq::assert_f64_near!(200000.0, graph.distance(1, 3).unwrap());
        assert_float_eq::assert_f64_near!(300000.0, graph.distance(0, 3).unwrap());
        assert_float_eq::assert_f64_near!(300000.0, graph.distance(2, 3).unwrap());

        Ok(())
    }
}
