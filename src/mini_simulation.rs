// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand_distr::num_traits::Inv;

use crate::mini_event::{MiniEvent, MiniEventType};
use crate::output::Sample;
use crate::timed_event::TimedEvent;
use crate::utils::CsvFriend;

pub struct MiniSimulation {
    // internal data structures
    events: crate::event_queue::EventQueue<MiniEvent>,
    single: crate::output::OutputScalar,
    series: crate::output::OutputSeries,

    // configuration
    config: crate::config::Config,
    mini_config: crate::mini_config::MiniConfig,
}

impl MiniSimulation {
    pub fn new(
        config: crate::config::Config,
        mini_config: crate::mini_config::MiniConfig,
        print_metrics: bool,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(mini_config.duration > 0.0, "vanishing duration");

        let mut single = crate::output::OutputScalar::default();
        single.init(
            "time_slot_duration",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new("s", "Time slot duration", true),
        );
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

        let series = crate::output::OutputSeries::new(mini_config.series_ignore.clone());

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
    fn run_sync(&mut self) -> crate::output::Output {
        let conf = &self.mini_config;
        let params = &conf.mini_parameters;
        let conf_100th = conf.duration / 100.0;
        let sync_params = match &params.protocol {
            crate::mini_config::Protocol::Sync(sync_config) => sync_config,
            _ => panic!("wrong simulation protocol"),
        };

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

        // metrics
        let mut num_events = 0;

        //
        // compute the time slot duration
        //

        // target probability that a local entanglement succeeds in a time slot
        let prob_local_entanglement = sync_params
            .prob_local_complete
            .powf(((params.num_repeaters + 1) as f64).inv());

        // maximum time allowed for the local entanglement phase
        let local_entanglement_threshold =
            -params.rate.inv() * (1.0 - prob_local_entanglement).ln();
        assert!(
            local_entanglement_threshold > 0.0,
            "negative max time to wait for local entanglement"
        );

        // maximum time needed for classical signaling
        let max_signalling_time =
            crate::utils::distance_to_latency(params.num_repeaters as f64 * params.distance);

        let time_slot_duration = local_entanglement_threshold + max_signalling_time;
        log::debug!("prob_local_entanglement = {}, local_entanglement_threshold = {}, max_signalling_time = {}, time_slot_duration = {}",prob_local_entanglement, local_entanglement_threshold,max_signalling_time,time_slot_duration);

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
                };
                self.update(new_events, new_samples);
            }
        }

        // save final metrics
        self.single.one_time("num_events", num_events as f64);
        self.single
            .one_time("execution_time", real_now.elapsed().as_secs_f64());
        // self.single.one_time(
        //     "epr_register_final_len",
        //     self.network.epr_register.len() as f64,
        // );

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

    /// Add all the events to the event queue and save metrics.
    fn update(&mut self, events: Vec<MiniEvent>, samples: Vec<Sample>) {
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
        match &self.mini_config.mini_parameters.protocol {
            crate::mini_config::Protocol::Sync(_) => self.run_sync(),
            crate::mini_config::Protocol::Async => todo!(),
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
