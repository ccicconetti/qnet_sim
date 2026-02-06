// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;

pub fn physical_topology_2_2() -> crate::physical_topology::PhysicalTopology {
    crate::physical_topology::PhysicalTopology::new(
        &crate::physical_topology::GridParams {
            orbit_to_orbit_distance: 1.0,
            ground_to_orbit_distance: 1.0,
            num_orbits: 2,
            orbit_length: 2,
            elevation_min: 10.0,
            elevation_max: 60.0,
        },
        crate::physical_topology::NodeWeight {
            label: None,
            node_type: crate::physical_topology::NodeType::SAT,
            is_repeater: true,
            memory_qubits: 20,
            decay_rate: 1.0,
            swapping_success_prob: 0.5,
            swapping_duration: 0.001,
            correction_duration: 0.0,
            detectors: 20,
            transmitters: 20,
        },
        crate::physical_topology::NodeWeight {
            label: None,
            node_type: crate::physical_topology::NodeType::OGS,
            is_repeater: false,
            memory_qubits: 10,
            decay_rate: 1.0,
            swapping_success_prob: 0.0,
            swapping_duration: 0.0,
            correction_duration: 0.001,
            detectors: 10,
            transmitters: 0,
        },
        42,
    )
    .expect("invalid physical topology")
}

pub fn logical_topology_2_2() -> (
    crate::physical_topology::PhysicalTopology,
    crate::logical_topology::LogicalTopology,
) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let physical_topology = physical_topology_2_2();
    if let Ok(logical_topology) = crate::logical_topology::LogicalTopology::from_physical_topology(
        &crate::logical_topology::PhysicalToLogicalPolicy::RandomGreedy,
        &physical_topology,
        &crate::physical_topology::FixedRate { rate: 1.0 },
        &mut rng,
    ) {
        if crate::logical_topology::is_valid(&logical_topology.graph(), &physical_topology).is_ok()
        {
            return (physical_topology, logical_topology);
        }
    }

    panic!(
        "could not find a feasible logical topology for physical topology {:?}",
        physical_topology
    );
}

#[cfg(test)]
mod tests {
    use core::f64;
    use std::io::Write;

    use crate::config::*;
    use crate::output::OutputSeriesSingle;
    use crate::simulation::Simulation;
    use crate::user_config::*;

    fn check_interval(
        metric: &str,
        series: &std::collections::HashMap<String, OutputSeriesSingle>,
        min: f64,
        max: f64,
    ) {
        if let Some(series) = series.get(metric) {
            for (labels, time, val) in &series.values {
                assert!(
                    *val >= min,
                    "metric {metric}, labels {labels:?}, time {time}: val {val} < min {min}"
                );
                assert!(
                    *val <= max,
                    "metric {metric}, labels {labels:?}, time {time}: val {val} > max {max}"
                );
            }
        }
    }

    fn make_config(duration: f64, memory_qubits: u32, seed: u64, num_repeaters: u32) -> Config {
        let user_config = UserConfig {
            duration,
            warmup_period: 0.0,
            series_ignore: std::collections::HashSet::new(),
            sections_not_serialized: std::collections::HashSet::new(),
            physical_topology: PhysicalTopology::Chain(ConfChain {
                chain_params: crate::physical_topology::ChainParams {
                    orbit_to_orbit_distance: 3000000.0,
                    ground_to_orbit_distance: 1000000.0,
                    num_repeaters,
                    elevation_min: 10.0,
                    elevation_max: 60.0,
                },
                sat_weight: crate::physical_topology::NodeWeight {
                    label: None,
                    node_type: crate::physical_topology::NodeType::SAT,
                    is_repeater: true,
                    memory_qubits,
                    decay_rate: 1.0,
                    swapping_success_prob: 0.95,
                    swapping_duration: 0.001,
                    correction_duration: 0.0,
                    detectors: 10,
                    transmitters: 10,
                },
                ogs_weight: crate::physical_topology::NodeWeight {
                    label: None,
                    node_type: crate::physical_topology::NodeType::OGS,
                    is_repeater: false,
                    memory_qubits,
                    decay_rate: 1.0,
                    swapping_success_prob: 0.0,
                    swapping_duration: 0.0,
                    correction_duration: 0.001,
                    detectors: 10,
                    transmitters: 0,
                },
            }),
            fidelity_computer: crate::user_config::FidelityComputer::StaticFidelities(
                crate::physical_topology::StaticFidelities {
                    f_o: 0.99,
                    f_g: 0.98,
                    f_oo: 0.97,
                    f_og: 0.96,
                    f_gg: 0.95,
                },
            ),
            rate_computer: crate::user_config::RateComputer::FixedRate(
                crate::physical_topology::FixedRate { rate: 1000.0 },
            ),
            logical_topology: LogicalTopology::default(),
            applications: Applications::ConfPing(ConfPing {
                source_dest_pairs: SourceDestPairs::List(vec![(0, 1)]),
                max_requests: 100,
            }),
        };
        Config { seed, user_config }
    }

    fn make_grid_config(seed: u64, file_topo: Option<String>, grid_size: usize) -> Config {
        let sat_weight = crate::physical_topology::NodeWeight {
            label: None,
            node_type: crate::physical_topology::NodeType::SAT,
            is_repeater: true,
            memory_qubits: 100,
            decay_rate: 1.0,
            swapping_success_prob: 0.95,
            swapping_duration: 0.001,
            correction_duration: 0.0,
            detectors: 10,
            transmitters: 20,
        };
        let ogs_weight = crate::physical_topology::NodeWeight {
            label: None,
            node_type: crate::physical_topology::NodeType::OGS,
            is_repeater: false,
            memory_qubits: 100,
            decay_rate: 1.0,
            swapping_success_prob: 0.0,
            swapping_duration: 0.0,
            correction_duration: 0.001,
            detectors: 10,
            transmitters: 0,
        };
        let physical_topology = if let Some(input_path) = file_topo {
            {
                let mut outfile = std::fs::OpenOptions::new()
                    .write(true)
                    .append(false)
                    .create(true)
                    .truncate(true)
                    .open(&input_path)
                    .unwrap();

                let _ = writeln!(outfile, "# node1 node2 distance is_ground_sat elevation");

                //
                // SAT nodes:         OGS nodes:
                //
                //                     9, 10, 11
                // 0 -- 1 -- 2
                // |    |    |        12, 13, 14
                // 3 -- 4 -- 5
                // |    |    |        15, 16, 17
                // 6 -- 7 -- 8
                //                    18, 19, 20
                //

                for i in 0..grid_size {
                    for j in 0..grid_size {
                        let cur_node = i + j * grid_size;
                        if i < (grid_size - 1) {
                            let _ = writeln!(outfile, "{} {} 300 0 42", cur_node, cur_node + 1);
                        } else {
                            let _ = writeln!(outfile, "{} {} 300 0 42", cur_node, j * grid_size);
                        }
                        if j < (grid_size - 1) {
                            let _ =
                                writeln!(outfile, "{} {} 300 0 42", cur_node, cur_node + grid_size);
                        }
                    }
                }

                // OGS nodes
                let nsq = grid_size * grid_size;
                for i in 0..grid_size {
                    for j in 0..grid_size {
                        let cur_node = i + j * grid_size;
                        let _ = writeln!(outfile, "{} {} 100 1 42", cur_node, nsq + cur_node);
                        let _ = writeln!(
                            outfile,
                            "{} {} 100 1 42",
                            cur_node,
                            nsq + j * grid_size + (i + 1) % grid_size
                        );
                        let _ = writeln!(
                            outfile,
                            "{} {} 100 1 42",
                            cur_node,
                            nsq + cur_node + grid_size
                        );
                        let _ = writeln!(
                            outfile,
                            "{} {} 100 1 42",
                            cur_node,
                            nsq + j * grid_size + (i + 1) % grid_size + grid_size
                        );
                    }
                }
            }

            PhysicalTopology::File(ConfFile {
                file_params: crate::physical_topology::FileParams {
                    input_type: crate::physical_topology::file_params::InputType::Leo,
                    input_path,
                },
                sat_weight,
                ogs_weight,
            })
        } else {
            PhysicalTopology::Grid(ConfGrid {
                grid_params: crate::physical_topology::GridParams {
                    orbit_to_orbit_distance: 3000000.0,
                    ground_to_orbit_distance: 1000000.0,
                    num_orbits: grid_size as u32,
                    orbit_length: grid_size as u32,
                    elevation_min: 42.0,
                    elevation_max: 42.0,
                },
                sat_weight,
                ogs_weight,
            })
        };
        let user_config = UserConfig {
            duration: 2.0,
            warmup_period: 0.0,
            series_ignore: std::collections::HashSet::new(),
            sections_not_serialized: std::collections::HashSet::new(),
            physical_topology,
            fidelity_computer: crate::user_config::FidelityComputer::StaticFidelities(
                crate::physical_topology::StaticFidelities {
                    f_o: 0.99,
                    f_g: 0.98,
                    f_oo: 0.97,
                    f_og: 0.96,
                    f_gg: 0.95,
                },
            ),
            rate_computer: crate::user_config::RateComputer::FixedRate(
                crate::physical_topology::FixedRate { rate: 100.0 },
            ),
            logical_topology: LogicalTopology::default(),
            applications: Applications::ConfPing(ConfPing {
                source_dest_pairs: SourceDestPairs::AllToAll,
                max_requests: 1,
            }),
        };
        Config { seed, user_config }
    }

    #[ignore]
    #[test]
    fn test_config_to_json() {
        let config = make_config(1.0, 10, 42, 1);
        println!("{}", serde_json::to_string_pretty(&config).unwrap());
    }

    #[test]
    fn test_sim_single_logical_hop() -> anyhow::Result<()> {
        // env_logger::init();

        let mut sim = None;
        for seed in 0..100 {
            let cand_sim = Simulation::new(make_config(10.0, 20, seed, 1), false, false)?;
            if cand_sim.logical_path(0, 1).len() == 2 {
                sim = Some(cand_sim);
                break;
            } else {
                println!("seed {seed} does not have a 1-hop path from 0 to 1: skip")
            }
        }
        let mut sim = sim.unwrap();

        let output = sim.run();

        let scalar = output.scalar.values();
        println!("scalar: {:?}", scalar);
        assert_float_eq::assert_f64_near!(1.0, *scalar.get("logical_topology_found").unwrap());
        assert!(*scalar.get("event_queue_len").unwrap() > 0.0);
        assert!(*scalar.get("num_events").unwrap() > 0.0);
        assert!(scalar.get("bsm_prob").unwrap().is_nan());
        assert_float_eq::assert_f64_near!(0.0, *scalar.get("bsm_tot").unwrap());

        let series = output.series.series;
        for (metric, values) in &series {
            println!(
                "series {} stats (min/avg/max/len) {:?}",
                metric,
                values.stats()
            );
        }
        assert_eq!(100, series.get("app-net-latency").unwrap().values.len());
        assert_eq!(100, series.get("ping-latency").unwrap().values.len());
        assert_eq!(200, series.get("fidelity").unwrap().values.len());
        check_interval("fidelity", &series, 0.91, 1.0);
        check_interval("gen-fidelity", &series, 0.95, 0.99);
        check_interval("app-tries", &series, 1.0, 21.0);
        check_interval("occupancy", &series, 0.0, 1.0);
        check_interval("app-path-len", &series, 1.0, 2.0);

        Ok(())
    }

    #[test]
    fn test_sim_multiple_logical_hops() -> anyhow::Result<()> {
        // env_logger::init();

        let mut sim = None;
        for seed in 0..100 {
            let cand_sim = Simulation::new(make_config(20.0, 100, seed, 3), false, false)?;
            if cand_sim.logical_path(0, 1).len() == 3 {
                sim = Some(cand_sim);
                break;
            } else {
                println!("seed {seed} does not have a 3-hop path from 0 to 1: skip")
            }
        }
        let mut sim = sim.unwrap();

        let output = sim.run();

        let scalar = output.scalar.values();
        println!("scalar: {:?}", scalar);
        assert_float_eq::assert_f64_near!(1.0, *scalar.get("logical_topology_found").unwrap());
        assert!(*scalar.get("event_queue_len").unwrap() > 0.0);
        assert!(*scalar.get("num_events").unwrap() > 0.0);
        assert!(*scalar.get("bsm_prob").unwrap() > 0.9);
        assert!(*scalar.get("bsm_tot").unwrap() > 100.0);

        let series = output.series.series;
        for (metric, values) in &series {
            println!(
                "series {} stats (min/avg/max/len) {:?}",
                metric,
                values.stats()
            );
        }
        assert_eq!(100, series.get("app-net-latency").unwrap().values.len());
        assert_eq!(100, series.get("ping-latency").unwrap().values.len());
        assert_eq!(200, series.get("fidelity").unwrap().values.len());
        check_interval("fidelity", &series, 0.8, 1.0);
        check_interval("gen-fidelity", &series, 0.95, 0.99);
        check_interval("app-tries", &series, 1.0, 50.0);
        check_interval("occupancy", &series, 0.0, 1.0);
        check_interval("app-path-len", &series, 1.0, 3.0);

        Ok(())
    }

    #[test]
    fn test_sim_compare_grids() -> anyhow::Result<()> {
        // env_logger::init();
        let seed = 4;

        let remove_me_dir = crate::utils::RemoveMeDir::new("test_sim_compare_grids")?;

        let mut path = remove_me_dir.dir();
        path.push("topo.txt");

        let mut sim = Simulation::new(
            make_grid_config(seed, Some(path.to_str().unwrap().to_string()), 3),
            false,
            false,
        )?;
        let output_file = sim.run();
        println!("{:?}", output_file.scalar.values());

        let mut sim = Simulation::new(make_grid_config(seed, None, 3), false, false)?;
        let output_grid = sim.run();
        println!("{:?}", output_grid.scalar.values());

        let scalar_file = output_file.scalar.values();
        let scalar_grid = output_grid.scalar.values();
        assert_float_eq::assert_f64_near!(1.0, *scalar_file.get("logical_topology_found").unwrap());
        assert_float_eq::assert_f64_near!(1.0, *scalar_grid.get("logical_topology_found").unwrap());
        assert_eq!(
            *scalar_file.get("logical_topology_num_edges").unwrap(),
            *scalar_grid.get("logical_topology_num_edges").unwrap()
        );
        assert_eq!(
            *scalar_file.get("logical_topology_possible_edges").unwrap(),
            *scalar_grid.get("logical_topology_possible_edges").unwrap()
        );

        assert_eq!(
            output_file
                .series
                .series
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<String>>(),
            output_grid
                .series
                .series
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<String>>()
        );

        Ok(())
    }
}
