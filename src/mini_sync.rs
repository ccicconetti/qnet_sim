// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::output::Sample;
use rand::SeedableRng;
use rand_distr::{num_traits::Inv, Distribution};

pub struct MiniSync {
    /// Maximum time allowed for the local entanglement phase, in s.
    local_entanglement_threshold: f64,
    // Time slot duration, in s.
    pub time_slot_duration: f64,
    /// Exponentially distributed r.v. to generate the inter-arrival times.
    rv: rand_distr::Exp<f64>,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Simulation configuration.
    mini_config: crate::mini_config::MiniConfig,
}

impl MiniSync {
    pub fn new(
        config: &crate::config::Config,
        mini_config: crate::mini_config::MiniConfig,
    ) -> Self {
        let params = &mini_config.mini_parameters;
        let sync_params = match &params.protocol {
            crate::mini_config::Protocol::Sync(sync_config) => sync_config,
            _ => panic!("wrong simulation protocol"),
        };

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

        Self {
            time_slot_duration,
            local_entanglement_threshold,
            rv: rand_distr::Exp::new(mini_config.mini_parameters.rate).unwrap(),
            rng: rand::rngs::StdRng::seed_from_u64(config.seed),
            mini_config,
        }
    }

    pub fn handle_time_slot(&mut self, now: u64) -> Vec<Sample> {
        let mut samples = vec![];
        log::debug!("new time slot");

        // generate the EPR pairs, one per each pair of adjacent nodes
        //
        //              R    R    R
        // (alice) 0 -- 1 -- 2 -- 3 -- 4 (bob)

        let n = self.mini_config.mini_parameters.num_repeaters;
        let mut epr_pairs = vec![];
        struct EprPair {
            left: u32,
            right: u32,
            generation_rel_time: f64,
        }
        for left in 0..=n {
            let generation_rel_time = self.rv.sample(&mut self.rng);
            epr_pairs.push(EprPair {
                left,
                right: left + 1,
                generation_rel_time,
            });
        }
        if epr_pairs
            .iter()
            .max_by(|x, y| {
                x.generation_rel_time
                    .partial_cmp(&y.generation_rel_time)
                    .unwrap()
            })
            .unwrap()
            .generation_rel_time
            > self.local_entanglement_threshold
        {
            samples.push(Sample::ScalarAvg("ebit_prob".to_string(), 0.0));
        } else {
            samples.push(Sample::ScalarAvg("ebit_prob".to_string(), 1.0));
        }

        samples
    }
}
