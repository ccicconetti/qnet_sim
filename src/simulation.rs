// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;

use crate::event::Event;
use crate::utils::CsvFriend;
use std::io::Write;

pub struct Simulation {
    // internal data structures
    physical_topology: crate::physical_topology::PhysicalTopology,
    logical_topology: crate::logical_topology::LogicalTopology,

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

        let physical_topology = match &config.user_config.physical_topology {
            crate::user_config::PhysicalTopology::ConfGridStatic(conf) => {
                crate::physical_topology::PhysicalTopology::from_grid_static(
                    conf.grid_params.clone(),
                    conf.sat_weight.clone(),
                    conf.ogs_weight.clone(),
                    conf.fidelities.clone(),
                )?
            }
        };

        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
        let logical_topology = crate::logical_topology::LogicalTopology::from_physical_topology(
            &config
                .user_config
                .logical_topology
                .physical_to_logical_policy,
            &physical_topology,
            &mut rng,
        )?;

        if save_to_dot {
            save_to_dot_file(physical_topology.graph(), "physical_topology.dot")?;
            save_to_dot_file(logical_topology.graph(), "logical_topology.dot")?;
            anyhow::bail!("saved to Dot files");
        }

        Ok(Self {
            physical_topology,
            logical_topology,
            config,
        })
    }

    /// Run a simulation.
    pub fn run(&mut self) -> crate::output::Output {
        let conf = &self.config.user_config;

        // outputs
        let mut single = crate::output::OutputSingle::new();
        let mut series = crate::output::OutputSeries::new();

        // create the event queue and push initial events
        let mut events = crate::event_queue::EventQueue::default();
        events.push(Event::WarmupPeriodEnd(crate::utils::to_nanoseconds(
            conf.warmup_period,
        )));
        events.push(Event::ExperimentEnd(crate::utils::to_nanoseconds(
            conf.duration,
        )));
        for i in 1..100 {
            events.push(Event::Progress(
                crate::utils::to_nanoseconds(i as f64 * conf.duration / 100.0),
                i,
            ));
        }

        // initialize simulated time and ID of the first job
        let mut now;

        // configure random variables

        // metrics
        let mut num_events = 0;

        // simulation loop
        let real_now = std::time::Instant::now();
        let mut last_time = 0;
        'main_loop: loop {
            if let Some(event) = events.pop() {
                now = event.time();

                single.time_avg("event_queue_len", now, events.len() as f64);

                // make sure we never go back in time
                assert!(now >= last_time);
                last_time = now;

                // count the number of events
                num_events += 1;

                // handle the current event
                match event {
                    Event::WarmupPeriodEnd(_) => {
                        log::debug!("W {}", now);
                        single.enable(now);
                        series.enable();
                    }
                    Event::ExperimentEnd(_) => {
                        log::debug!("E {}", now);
                        break 'main_loop;
                    }
                    Event::Progress(_, percentage) => {
                        log::info!("completed {}%", percentage);
                    }
                }
            }
        }

        // save final metrics
        single.one_time("num_events", num_events as f64);
        single.one_time("execution_time", real_now.elapsed().as_secs_f64());

        // return the simulation output
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
