// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use petgraph::visit::EdgeRef;
use rand::SeedableRng;
use std::io::Write;

use crate::event::{Event, EventHandler, EventType};
use crate::physical_topology;
use crate::{output::Sample, utils::CsvFriend};

pub struct Simulation {
    // internal data structures
    network: crate::network::Network,
    events: crate::event_queue::EventQueue,
    single: crate::output::OutputScalar,
    series: crate::output::OutputSeries,

    // configuration
    config: crate::config::Config,
    user_config: crate::user_config::UserConfig,
}

fn save_to_dot_file<
    T: petgraph::visit::Data
        + petgraph::visit::IntoNodeReferences
        + petgraph::visit::IntoEdgeReferences
        + petgraph::visit::NodeIndexable
        + petgraph::visit::GraphProp,
>(
    graph: T,
    full_path: &str,
) -> anyhow::Result<()>
where
    <T as petgraph::visit::Data>::EdgeWeight: std::fmt::Display,
    <T as petgraph::visit::Data>::NodeWeight: std::fmt::Display,
{
    let mut dotfile = std::fs::OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .truncate(true)
        .open(full_path)?;
    let _ = writeln!(
        dotfile,
        "{}",
        petgraph::dot::Dot::with_config(&graph, &[petgraph::dot::Config::NodeIndexLabel])
    );
    Ok(())
}

impl Simulation {
    fn create_network(
        config: &crate::config::Config,
        user_config: &crate::user_config::UserConfig,
        physical_topology: crate::physical_topology::PhysicalTopology,
        save_to_dot: bool,
    ) -> crate::network::Network {
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

        let rate_computer = crate::user_config::RateComputer::make(&user_config.rate_computer);

        let logical_topology =
            match crate::logical_topology::LogicalTopology::from_physical_topology(
                &user_config.logical_topology.physical_to_logical_policy,
                &physical_topology,
                rate_computer.as_ref(),
                &mut rng,
            ) {
                Ok(logical_topology) => {
                    log::info!(
                        "logical topology found with {} edges (out of {} possible)",
                        logical_topology.graph().edge_count(),
                        logical_topology.num_possible_logical_edges
                    );
                    if crate::logical_topology::is_valid(
                        logical_topology.graph(),
                        &physical_topology,
                    )
                    .is_ok()
                    {
                        log::debug!("{:#?}", logical_topology.graph());

                        if save_to_dot {
                            let _ =
                                save_to_dot_file(logical_topology.graph(), "logical_topology.dot");
                        }

                        logical_topology
                    } else {
                        crate::logical_topology::LogicalTopology::default()
                    }
                }
                Err(err) => {
                    log::info!("logical topology not found: {}", err);
                    crate::logical_topology::LogicalTopology::default()
                }
            };
        crate::network::Network::new(
            physical_topology,
            crate::user_config::FidelityComputer::make(&user_config.fidelity_computer),
            rate_computer,
            std::rc::Rc::new(logical_topology),
            config.seed,
        )
    }

    pub fn new(
        config: crate::config::Config,
        user_config: crate::user_config::UserConfig,
        save_to_dot: bool,
        print_metrics: bool,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(user_config.duration > 0.0, "vanishing duration");

        let physical_topology = user_config.physical_topology.make(config.seed)?;

        if save_to_dot {
            save_to_dot_file(physical_topology.graph(), "physical_topology.dot")?;
        }

        let network = Self::create_network(&config, &user_config, physical_topology, save_to_dot);

        // Terminate immediately if the user requested to save to Dot.
        anyhow::ensure!(!save_to_dot, "saved to Dot files");

        // Create data structure for scalar values.
        let mut single = crate::output::OutputScalar::default();
        single.init("bsm_prob", crate::output::ScalarMetricType::Avg, crate::output::MetricMetadata::new("probability", "Probability of a successful Bell State Measurement performed during an Entanglement Swapping operation", false));
        single.init(
            "bsm_tot",
            crate::output::ScalarMetricType::Count,
            crate::output::MetricMetadata::new(
                "operations",
                "Total number of Entanglement Swapping operations performed",
                false,
            ),
        );
        single.init(
            "event_queue_len",
            crate::output::ScalarMetricType::TimeAvg,
            crate::output::MetricMetadata::new(
                "events",
                "Average number of events in the queue",
                false,
            ),
        );
        single.init(
            "local_epr_misses",
            crate::output::ScalarMetricType::Count,
            crate::output::MetricMetadata::new("misses", "Total number of EPR misses, i.e., events when the master requests to use a given memory cell, which is, however, not available on the slave node, resulting in an ES failure",false)
        );
        single.init(
            "epr_frees",
            crate::output::ScalarMetricType::Count,
            crate::output::MetricMetadata::new(
                "frees",
                "Total number of EPR free operations",
                false,
            ),
        );
        single.init(
            "logical_topology_found",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new("{0,1}", "Value that is equal to 1 if a logical topology has been found for the given physical topology, 0 otherwise",true)
        );
        single.init(
            "logical_topology_num_edges",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new(
                "edges",
                "Total number of edges in the logical topology",
                true,
            ),
        );
        single.init(
            "logical_topology_possible_edges",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new(
                "edges",
                "Total number of possible edges in the logical topology",
                true,
            ),
        );
        single.init(
            "num_events",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new(
                "events",
                "Total number of events in the simulation",
                true,
            ),
        );
        single.init(
            "execution_time",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new("s", "Simulation real-time duration", true),
        );
        single.init(
            "epr_register_final_len",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new(
                "EPR pairs",
                "Total number of residual EPR pairs in the register",
                true,
            ),
        );

        // Create data structure for time series, also setting the headers
        let mut series = crate::output::OutputSeries::new(user_config.series_ignore.clone());
        series.init(
            "gen-fidelity",
            &["node_id"],
            crate::output::MetricMetadata::new("[0,1]", "Generation fidelity", false),
        );
        series.init(
            "fidelity",
            &["node_id", "port"],
            crate::output::MetricMetadata::new("[0,1]", "Measured fidelity", false),
        );
        series.init(
            "occupancy",
            &["node_id", "peer_node_id"],
            crate::output::MetricMetadata::new("memory cells", "Occupancy at a given NIC", false),
        );
        series.init(
            "app-net-latency",
            &["node_id", "port"],
            crate::output::MetricMetadata::new("s", "Application network latency, i.e., the time between when an EPR is requested from an application and when it is fully established, i.e., it can proceed", false),
        );
        series.init(
            "client-latency",
            &["node_id", "port"],
            crate::output::MetricMetadata::new(
                "s",
                "In a client-server application, the waiting time of an EPR request",
                false,
            ),
        );
        series.init(
            "client-queue-len",
            &["node_id", "port"],
            crate::output::MetricMetadata::new(
                "requests",
                "In a client-server application, the number of queued requests at a client",
                false,
            ),
        );
        series.init(
            "ping-latency",
            &["node_id", "peer_node_id"],
            crate::output::MetricMetadata::new(
                "s",
                "In a ping application, the latency to establish an end-to-end entanglement",
                false,
            ),
        );
        series.init(
            "server-queue-len",
            &["node_id", "port"],
            crate::output::MetricMetadata::new(
                "responses",
                "In a client-server application, the number of queue responses at a server",
                false,
            ),
        );
        series.init(
            "app-path-len",
            &["node_id", "port"],
            crate::output::MetricMetadata::new("hops", "The path length for an application", true),
        );
        series.init(
            "app-tries",
            &["node_id", "port"],
            crate::output::MetricMetadata::new("tries", "The number of times an EPR has to be requested before an end-to-end entanglement is established",false)
        );
        series.init(
            "physical-distance",
            &["source", "target", "node_type"],
            crate::output::MetricMetadata::new(
                "m",
                "The physical distance between two nodes in the physical topology",
                true,
            ),
        );
        series.init(
            "physical-elevation",
            &["source", "target", "node_type"],
            crate::output::MetricMetadata::new(
                "degrees",
                "The elevation between two nodes in the physical topology",
                true,
            ),
        );

        // Save physical topology metrics.
        let physical_topology = &network.physical_topology;
        for e in physical_topology.graph().edge_references() {
            let is_ogs = physical_topology
                .graph()
                .node_weight(e.source())
                .unwrap()
                .node_type
                == physical_topology::NodeType::OGS
                || physical_topology
                    .graph()
                    .node_weight(e.target())
                    .unwrap()
                    .node_type
                    == physical_topology::NodeType::OGS;
            let is_ogs_to_str = |is_ogs: bool| {
                if is_ogs {
                    String::from("ogs")
                } else {
                    String::from("sat")
                }
            };

            series.add(
                "physical-distance",
                vec![
                    format!("{}", e.source().index()),
                    format!("{}", e.target().index()),
                    is_ogs_to_str(is_ogs),
                ],
                0.0,
                e.weight().distance,
            );
            series.add(
                "physical-elevation",
                vec![
                    format!("{}", e.source().index()),
                    format!("{}", e.target().index()),
                    is_ogs_to_str(is_ogs),
                ],
                0.0,
                e.weight().elevation,
            );
        }

        // Terminate immediately if the user requested to print metrics.
        if print_metrics {
            println!(
                "# Scalar metrics\n{}\n# Series\n{}",
                markdown_tables::as_table(&single.to_markdown_table()),
                markdown_tables::as_table(&series.to_markdown_table())
            );
            panic!("quitting after printing the metrics");
        }

        Ok(Self {
            network,
            config,
            user_config,
            events: crate::event_queue::EventQueue::default(),
            single,
            series,
        })
    }

    /// Return the path on the logical topology from `src` to `dst`.
    pub fn logical_path(&self, src: u32, dst: u32) -> Vec<u32> {
        self.network.logical_topology.path(src, dst)
    }

    /// Add all the events to the event queue and save metrics.
    fn update(&mut self, events: Vec<Event>, samples: Vec<Sample>) {
        for event in events {
            self.events.push(event);
        }
        let now = self.events.last_time();
        for sample in samples {
            match sample {
                Sample::ScalarOneTime(name, value) => self.single.one_time(&name, value),
                Sample::ScalarAvg(name, value) => self.single.avg(&name, value),
                Sample::ScalarTimeAvg(name, value) => self.single.time_avg(&name, now, value),
                Sample::ScalarCount(name) => self.single.count(&name),
                Sample::Series(name, labels, value) => {
                    self.series
                        .add(&name, labels, crate::utils::to_seconds(now), value)
                }
            }
        }
    }

    /// Run a simulation.
    pub fn run(&mut self) -> crate::output::Output {
        let conf = &self.user_config;
        let conf_100th = conf.duration / 100.0;

        // create the applications (if a logical topology has been found)
        if self.network.logical_topology.graph().node_count() > 0 {
            create_applications(
                self.config.seed,
                &self.user_config.applications,
                &mut self.network,
            );
        }

        // push initial events
        self.events
            .push(Event::new(conf.warmup_period, EventType::WarmupPeriodEnd));
        self.events
            .push(Event::new(conf.duration, EventType::ExperimentEnd));
        self.events.push(Event::new(0.0, EventType::Progress(0)));
        let initial_network_events = self.network.initial();
        let logical_topology_found = if initial_network_events.is_empty() {
            0.0_f64
        } else {
            1.0_f64
        };
        self.update(initial_network_events, vec![]);

        // initialize simulated time and ID of the first job
        let mut now;

        // metrics
        let mut num_events = 0;

        // simulation loop
        let real_now = std::time::Instant::now();
        let mut last_time = 0;
        'main_loop: loop {
            if let Some(event) = self.events.pop() {
                now = event.time();
                assert_eq!(now, self.events.last_time());

                self.single
                    .time_avg("event_queue_len", now, self.events.len() as f64);

                // make sure we never go back in time
                assert!(now >= last_time);
                last_time = now;

                // count the number of events
                num_events += 1;

                // handle the current event
                let mut transfer_info = String::default();
                if let Some(transfer) = &event.transfer {
                    if !transfer.done {
                        transfer_info =
                            format!(" ({}->{})", transfer.src_node_id, transfer.dst_node_id);
                    }
                };
                let (new_events, new_samples) = match &event.event_type {
                    EventType::WarmupPeriodEnd => {
                        log::debug!("W {}", now);
                        self.single.enable(now);
                        self.series.enable();
                        (vec![], vec![])
                    }
                    EventType::ExperimentEnd => {
                        log::debug!("E {}", now);
                        break 'main_loop;
                    }
                    EventType::Progress(percentage) => {
                        log::info!("completed {}%", percentage);
                        (
                            vec![Event::new(conf_100th, EventType::Progress(percentage + 1))],
                            vec![],
                        )
                    }
                    EventType::NetworkEvent(event_data) => {
                        log::debug!("X {} {:?}", now, event_data);
                        self.network.handle(event)
                    }
                    EventType::NodeEvent(event_data) => {
                        log::debug!(
                            "N {} [node_id {}{}] {:?}",
                            now,
                            event.target_node_id(),
                            transfer_info,
                            event_data
                        );
                        self.network.handle(event)
                    }
                    EventType::AppEvent(event_data) => {
                        log::debug!(
                            "A {} [node_id {}{}] {:?}",
                            now,
                            event.target_node_id(),
                            transfer_info,
                            event_data
                        );
                        self.network.handle(event)
                    }
                };
                self.update(new_events, new_samples);
            }
        }

        // save final metrics
        self.single
            .one_time("logical_topology_found", logical_topology_found);
        self.single.one_time(
            "logical_topology_num_edges",
            self.network.logical_topology.graph().edge_count() as f64,
        );
        self.single.one_time(
            "logical_topology_possible_edges",
            self.network.logical_topology.num_possible_logical_edges as f64,
        );
        self.single.one_time("num_events", num_events as f64);
        self.single
            .one_time("execution_time", real_now.elapsed().as_secs_f64());
        self.single.one_time(
            "epr_register_final_len",
            self.network.epr_register.len() as f64,
        );
        if log::log_enabled!(log::Level::Debug) {
            self.network.epr_register.dump();
        }

        // return the simulation output
        let single = std::mem::take(&mut self.single);
        let series = std::mem::take(&mut self.series);
        crate::output::Output {
            scalar: single,
            series,
            config_csv: self.config.to_csv(),
            user_config_csv: self.user_config.to_csv(),
        }
    }
}

fn create_applications(
    seed: u64,
    conf: &crate::user_config::Applications,
    network: &mut crate::network::Network,
) {
    let ogs_indices = network.physical_topology.ogs_indices();
    assert!(!ogs_indices.is_empty(), "no OGS nodes");
    assert!(ogs_indices.len() > 1, "there's a single OGS node");
    match &conf {
        crate::user_config::Applications::ConfPing(conf_ping) => {
            let max_requests = conf_ping.max_requests;
            for (this_node_id, peer_node_id) in
                conf_ping.source_dest_pairs.make_pairs(ogs_indices, seed)
            {
                let this_port = network.nodes[this_node_id as usize].next_port();
                let peer_port = network.nodes[peer_node_id as usize].next_port();

                log::debug!(
                    "creating ping/pong between {}:{} and {}:{} (max requests {})",
                    this_node_id,
                    this_port,
                    peer_node_id,
                    peer_port,
                    max_requests
                );

                let pinger = Box::new(crate::apps::pinger::Pinger::new(
                    this_node_id,
                    this_port,
                    peer_node_id,
                    peer_port,
                    max_requests,
                ));
                network.nodes[this_node_id as usize].add_applicaton(pinger, this_port);

                let ponger = Box::new(crate::apps::ponger::Ponger::new(peer_node_id, peer_port));
                network.nodes[peer_node_id as usize].add_applicaton(ponger, peer_port);
            }
        }
        crate::user_config::Applications::ConfClientServer(conf_client_server) => {
            for (this_node_id, peer_node_id) in conf_client_server
                .source_dest_pairs
                .make_pairs(ogs_indices, seed)
            {
                let this_port = network.nodes[this_node_id as usize].next_port();
                let peer_port = network.nodes[peer_node_id as usize].next_port();

                log::debug!(
                    "creating client/server between {}:{} and {}:{} (rate {} s^1, avg dur client {} s, avg dur server {})",
                    this_node_id,
                    this_port,
                    peer_node_id,
                    peer_port,
                    conf_client_server.operation_rate,conf_client_server.operation_avg_dur_client, conf_client_server.operation_avg_dur_server
                );

                let client = Box::new(crate::apps::client::Client::new(
                    this_node_id,
                    this_port,
                    peer_node_id,
                    peer_port,
                    seed,
                    conf_client_server.operation_rate,
                    conf_client_server.operation_avg_dur_client,
                ));
                network.nodes[this_node_id as usize].add_applicaton(client, this_port);

                let server = Box::new(crate::apps::server::Server::new(
                    peer_node_id,
                    peer_port,
                    seed,
                    conf_client_server.operation_avg_dur_server,
                ));
                network.nodes[peer_node_id as usize].add_applicaton(server, peer_port);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_simulation_run() -> anyhow::Result<()> {
        Ok(())
    }
}
