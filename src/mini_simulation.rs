// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::utils::CsvFriend;

pub struct MiniSimulation {
    // internal data structures
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

        let single = crate::output::OutputScalar::default();

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
            single,
            series,
            config,
            mini_config,
        })
    }

    /// Run a simulation.
    pub fn run(&mut self) -> crate::output::Output {
        // let conf = &self.mini_config;
        // let conf_100th = conf.duration / 100.0;

        // initialize simulated time and ID of the first job
        // let mut now;

        // metrics
        // let mut num_events = 0;

        // // simulation loop
        // let real_now = std::time::Instant::now();
        // let mut last_time = 0;
        // 'main_loop: loop {
        //     if let Some(event) = self.events.pop() {
        //         now = event.time();
        //         assert_eq!(now, self.events.last_time());

        //         self.single
        //             .time_avg("event_queue_len", now, self.events.len() as f64);

        //         // make sure we never go back in time
        //         assert!(now >= last_time);
        //         last_time = now;

        //         // count the number of events
        //         num_events += 1;

        //         // handle the current event
        //         let mut transfer_info = String::default();
        //         if let Some(transfer) = &event.transfer {
        //             if !transfer.done {
        //                 transfer_info =
        //                     format!(" ({}->{})", transfer.src_node_id, transfer.dst_node_id);
        //             }
        //         };
        //         let (new_events, new_samples) = match &event.event_type {
        //             EventType::WarmupPeriodEnd => {
        //                 log::debug!("W {}", now);
        //                 self.single.enable(now);
        //                 self.series.enable();
        //                 (vec![], vec![])
        //             }
        //             EventType::ExperimentEnd => {
        //                 log::debug!("E {}", now);
        //                 break 'main_loop;
        //             }
        //             EventType::Progress(percentage) => {
        //                 log::info!("completed {}%", percentage);
        //                 (
        //                     vec![Event::new(conf_100th, EventType::Progress(percentage + 1))],
        //                     vec![],
        //                 )
        //             }
        //             EventType::NetworkEvent(event_data) => {
        //                 log::debug!("X {} {:?}", now, event_data);
        //                 self.network.handle(event)
        //             }
        //             EventType::NodeEvent(event_data) => {
        //                 log::debug!(
        //                     "N {} [node_id {}{}] {:?}",
        //                     now,
        //                     event.target_node_id(),
        //                     transfer_info,
        //                     event_data
        //                 );
        //                 self.network.handle(event)
        //             }
        //             EventType::AppEvent(event_data) => {
        //                 log::debug!(
        //                     "A {} [node_id {}{}] {:?}",
        //                     now,
        //                     event.target_node_id(),
        //                     transfer_info,
        //                     event_data
        //                 );
        //                 self.network.handle(event)
        //             }
        //         };
        //         self.update(new_events, new_samples);
        //     }
        // }

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
