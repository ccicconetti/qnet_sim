// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;
use std::io::Write;

use crate::event::{Event, EventHandler, EventType};
use crate::{output::Sample, utils::CsvFriend};

pub struct Simulation {
    // internal data structures
    network: crate::network::Network,
    events: crate::event_queue::EventQueue,
    single: crate::output::OutputSingle,
    series: crate::output::OutputSeries,

    // configuration
    config: crate::config::Config,
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
        physical_topology: crate::physical_topology::PhysicalTopology,
        save_to_dot: bool,
    ) -> crate::network::Network {
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

        let logical_topology = if let Ok(logical_topology) =
            crate::logical_topology::LogicalTopology::from_physical_topology(
                &config
                    .user_config
                    .logical_topology
                    .physical_to_logical_policy,
                &physical_topology,
                &mut rng,
            ) {
            if crate::logical_topology::is_valid(logical_topology.graph(), &physical_topology)
                .is_ok()
            {
                log::debug!("{:#?}", logical_topology.graph());

                if save_to_dot {
                    let _ = save_to_dot_file(logical_topology.graph(), "logical_topology.dot");
                }

                logical_topology
            } else {
                crate::logical_topology::LogicalTopology::default()
            }
        } else {
            crate::logical_topology::LogicalTopology::default()
        };
        crate::network::Network::new(&logical_topology, physical_topology, config.seed)
    }

    pub fn new(config: crate::config::Config, save_to_dot: bool) -> anyhow::Result<Self> {
        anyhow::ensure!(config.user_config.duration > 0.0, "vanishing duration");

        let physical_topology = config
            .user_config
            .physical_topology
            .to_physical_topology()?;

        if save_to_dot {
            save_to_dot_file(physical_topology.graph(), "physical_topology.dot")?;
        }

        let network = Self::create_network(&config, physical_topology, save_to_dot);

        // Save to Graphviz files and terminate immediately.
        anyhow::ensure!(!save_to_dot, "saved to Dot files");

        let series = crate::output::OutputSeries::new(config.user_config.series_ignore.clone());

        Ok(Self {
            network,
            config,
            events: crate::event_queue::EventQueue::default(),
            single: crate::output::OutputSingle::default(),
            series,
        })
    }

    /// Add all the events to the event queue and save metrics.
    fn update(&mut self, events: Vec<Event>, samples: Vec<Sample>) {
        for event in events {
            self.events.push(event);
        }
        let now = self.events.last_time();
        for sample in samples {
            match sample {
                Sample::SingleOneTime(name, value) => self.single.one_time(&name, value),
                Sample::SingleTimeAvg(name, value) => self.single.time_avg(&name, now, value),
                Sample::Series(name, label, value) => {
                    self.series
                        .add(&name, &label, crate::utils::to_seconds(now), value)
                }
            }
        }
    }

    /// Run a simulation.
    pub fn run(&mut self) -> crate::output::Output {
        let conf = &self.config.user_config;
        let conf_100th = conf.duration / 100.0;

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
                    EventType::NodeEvent(event_data) => {
                        log::debug!("N {} {:?}", now, event_data);
                        self.network.handle(event)
                    }
                    EventType::OsEvent(event_data) => {
                        log::debug!("O {} {:?}", now, event_data);
                        self.network.handle(event)
                    }
                    EventType::AppEvent(event_data) => {
                        log::debug!("A {} {:?}", now, event_data);
                        self.network.handle(event)
                    }
                };
                self.update(new_events, new_samples);
            }
        }

        // save final metrics
        self.single
            .one_time("logical_topology_found", logical_topology_found);
        self.single.one_time("num_events", num_events as f64);
        self.single
            .one_time("execution_time", real_now.elapsed().as_secs_f64());

        // return the simulation output
        let single = std::mem::take(&mut self.single);
        let series = std::mem::take(&mut self.series);
        crate::output::Output {
            single,
            series,
            config_csv: self.config.to_csv(),
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
