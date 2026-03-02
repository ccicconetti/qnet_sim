// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::mini_event::{MiniEvent, MiniEventType};
use crate::output::Sample;
use crate::timed_event::TimedEvent;
use crate::utils::CsvFriend;

pub struct MiniSimulation {
    // Event queue.
    events: crate::event_queue::EventQueue<MiniEvent>,
    /// Output data (scalar values).
    single: crate::output::OutputScalar,
    /// Output data (time series).
    series: crate::output::OutputSeries,

    /// Simulation configuration (generic).
    config: crate::config::Config,
    /// Simulation configuration (specific for mini simulations).
    mini_config: crate::mini_config::MiniConfig,
}

impl MiniSimulation {
    pub fn new(
        config: crate::config::Config,
        mini_config: crate::mini_config::MiniConfig,
        print_metrics: bool,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(mini_config.duration > 0.0, "vanishing duration");

        let (single, series) =
            crate::mini_sync::MiniSync::get_metrics(mini_config.series_ignore.clone());

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
            events: Default::default(),
            single,
            series,
            config,
            mini_config,
        })
    }

    /// Run a simulation with a sync protocol.
    fn run_sync(&mut self) {
        let conf = &self.mini_config;
        let conf_100th = conf.duration / 100.0;

        // initialize simulated time
        let mut now;

        // push initial events
        self.events.push(MiniEvent::new(
            conf.warmup_period,
            MiniEventType::WarmupPeriodEnd,
        ));
        self.events
            .push(MiniEvent::new(conf.duration, MiniEventType::ExperimentEnd));
        self.events
            .push(MiniEvent::new(0.0, MiniEventType::Progress(0)));
        self.events
            .push(MiniEvent::new(0.0, MiniEventType::TimeSlot));

        // metrics
        let mut num_events = 0;

        // compute the time slot duration
        let mut mini_sync = crate::mini_sync::MiniSync::new(&self.config, self.mini_config.clone());
        let time_slot_duration = mini_sync.time_slot_duration;
        self.single
            .one_time("time_slot_duration", time_slot_duration);

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
                    MiniEventType::WarmupPeriodEnd => {
                        log::debug!("W {}", now);
                        self.single.enable(now);
                        self.series.enable();
                        (vec![], vec![])
                    }
                    MiniEventType::ExperimentEnd => {
                        log::debug!("E {}", now);
                        break 'main_loop;
                    }
                    MiniEventType::Progress(percentage) => {
                        log::info!("completed {}%", percentage);
                        (
                            vec![MiniEvent::new(
                                conf_100th,
                                MiniEventType::Progress(percentage + 1),
                            )],
                            vec![],
                        )
                    }
                    MiniEventType::TimeSlot => (
                        vec![MiniEvent::new(time_slot_duration, MiniEventType::TimeSlot)],
                        mini_sync.handle_time_slot(now),
                    ),
                };
                self.update(new_events, new_samples);
            }
        }

        // save final metrics
        self.single.one_time("num_events", num_events as f64);
        self.single
            .one_time("execution_time", real_now.elapsed().as_secs_f64());
        self.single.one_time(
            "epr_register_final_len",
            mini_sync.epr_register.len() as f64,
        );
    }

    /// Add all the events to the event queue and save metrics.
    fn update(&mut self, events: Vec<MiniEvent>, samples: Vec<Sample>) {
        for event in events {
            self.events.push(event);
        }
        let now = self.events.last_time();
        crate::output::add_all_samples(now, samples, &mut self.single, &mut self.series);
    }

    /// Run a simulation.
    pub fn run(&mut self) -> crate::output::Output {
        match &self.mini_config.mini_parameters.protocol {
            crate::mini_config::Protocol::Sync(_) => self.run_sync(),
            crate::mini_config::Protocol::Async => todo!(),
        }

        // return the simulation output
        let single = std::mem::take(&mut self.single);
        let series = std::mem::take(&mut self.series);
        crate::output::Output {
            scalar: single,
            series,
            config_csv: self.config.to_csv(),
            user_config_csv: self.mini_config.to_csv(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mini_simulation_run() -> anyhow::Result<()> {
        Ok(())
    }
}
