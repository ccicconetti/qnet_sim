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
            swapping_duration: 0.001,
            correction_duration: 0.0,
            detectors: 10,
            transmitters: 10,
            capacity: 1.0,
        },
        crate::physical_topology::NodeWeight {
            node_type: crate::physical_topology::NodeType::OGS,
            memory_qubits: 20,
            decay_rate: 1.0,
            swapping_success_prob: 0.0,
            swapping_duration: 0.0,
            correction_duration: 0.001,
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
        if let Ok(logical_topology) =
            crate::logical_topology::LogicalTopology::from_physical_topology(
                &crate::logical_topology::PhysicalToLogicalPolicy::RandomGreedy,
                &physical_topology,
                &mut rng,
            )
        {
            if crate::logical_topology::is_valid(&logical_topology.graph(), &physical_topology)
                .is_ok()
            {
                return (physical_topology, logical_topology);
            }
        }
    }
    panic!(
        "could not find a feasible logical topology in {} tries",
        num_tries
    );
}

#[cfg(test)]
mod tests {
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

    fn make_config(seed: u64) -> Config {
        let user_config = UserConfig {
            duration: 10.0,
            warmup_period: 0.0,
            series_ignore: std::collections::HashSet::new(),
            physical_topology: PhysicalTopology::ConfChainStatic(ConfChainStatic {
                chain_params: crate::physical_topology::ChainParams::default(),
                sat_weight: default_sat_weight(),
                ogs_weight: default_ogs_weight(),
                fidelities: crate::physical_topology::StaticFidelities {
                    f_o: 0.99,
                    f_g: 0.98,
                    f_oo: 0.97,
                    f_og: 0.96,
                    f_gg: 0.95,
                },
            }),
            logical_topology: LogicalTopology::default(),
            applications: Applications::ConfPing(ConfPing {
                source_dest_pairs: SourceDestPairs::List(vec![(0, 1)]),
                max_requests: 100,
            }),
        };
        Config { seed, user_config }
    }

    #[ignore]
    #[test]
    fn test_config_to_json() {
        let config = make_config(42);
        println!("{}", serde_json::to_string_pretty(&config).unwrap());
    }

    #[test]
    fn test_sim_single_logical_hop() -> anyhow::Result<()> {
        // env_logger::init();

        let mut sim = None;
        for seed in 0..100 {
            let cand_sim = Simulation::new(make_config(seed), false)?;
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

        let series = output.series.series;
        for (metric, values) in &series {
            println!(
                "series {} [#{}]: {:?}",
                metric,
                values.values.len(),
                values.values
            );
        }
        assert_eq!(100, series.get("epr-request-latency").unwrap().values.len());
        assert_eq!(100, series.get("ping-latency").unwrap().values.len());
        assert_eq!(200, series.get("fidelity").unwrap().values.len());
        check_interval("fidelity", &series, 0.92, 1.0);
        check_interval("gen_fidelity", &series, 0.95, 0.99);

        Ok(())
    }
}
