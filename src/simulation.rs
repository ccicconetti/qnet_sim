// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;

use crate::{output::Sample, utils::CsvFriend};
use std::io::Write;

use crate::event::{Event, EventHandler, EventType};

pub struct Simulation {
    // internal data structures
    logical_topology: crate::logical_topology::LogicalTopology,
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
    pub fn new(config: crate::config::Config, save_to_dot: bool) -> anyhow::Result<Self> {
        anyhow::ensure!(config.user_config.duration > 0.0, "vanishing duration");

        let physical_topology = config
            .user_config
            .physical_topology
            .to_physical_topology()?;

        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
        let logical_topology = crate::logical_topology::LogicalTopology::from_physical_topology(
            &config
                .user_config
                .logical_topology
                .physical_to_logical_policy,
            &physical_topology,
            &mut rng,
        )?;

        crate::logical_topology::is_valid(logical_topology.graph(), &physical_topology)?;

        if save_to_dot {
            save_to_dot_file(physical_topology.graph(), "physical_topology.dot")?;
            save_to_dot_file(logical_topology.graph(), "logical_topology.dot")?;
            anyhow::bail!("saved to Dot files");
        }

        let network =
            crate::network::Network::new(&logical_topology, physical_topology, config.seed);
        let series = crate::output::OutputSeries::new(config.user_config.series_ignore.clone());

        Ok(Self {
            logical_topology,
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

        log::debug!("{:#?}", self.logical_topology.graph());

        // push initial events
        self.events
            .push(Event::new(conf.warmup_period, EventType::WarmupPeriodEnd));
        self.events
            .push(Event::new(conf.duration, EventType::ExperimentEnd));
        for i in 1..100 {
            self.events.push(Event::new(
                i as f64 * conf.duration / 100.0,
                EventType::Progress(i),
            ));
        }
        let initial_network_events = self.network.initial();
        self.update(initial_network_events, vec![]);

        // initialize simulated time and ID of the first job
        let mut now;

        // configure random variables

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
                match &event.event_type {
                    EventType::WarmupPeriodEnd => {
                        log::debug!("W {}", now);
                        self.single.enable(now);
                        self.series.enable();
                    }
                    EventType::ExperimentEnd => {
                        log::debug!("E {}", now);
                        break 'main_loop;
                    }
                    EventType::Progress(percentage) => {
                        log::info!("completed {}%", percentage);
                    }
                    EventType::EprGenerated(event_data) => {
                        log::debug!("G {} {:?}", now, event_data);
                        let (new_events, new_samples) = self.network.handle(event);
                        self.update(new_events, new_samples);
                    }
                    EventType::EprNotified(event_data) => {
                        log::debug!("N {} {:?}", now, event_data);
                        let (new_events, new_samples) = self.network.handle(event);
                        self.update(new_events, new_samples);
                    }
                }
            }
        }

        // save final metrics
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
