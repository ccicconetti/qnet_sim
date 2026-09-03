// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::output::Sample;
use rand::SeedableRng;
use rand_distr::{num_traits::Inv, Distribution};

pub struct MiniSync {
    /// Maximum time allowed for the local entanglement phase, in ns.
    local_entanglement_threshold: u64,
    // Time slot duration, in s.
    pub time_slot_duration: f64,
    /// Exponentially distributed r.v. to generate the inter-arrival times.
    rv_gen: rand_distr::Exp<f64>,
    /// Uniform r.v. to determine if an entanglement swapping succeeds.
    rv_bsm: rand_distr::Uniform<f64>,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Simulation configuration.
    mini_config: crate::mini_config::MiniConfig,
    /// EPR register.
    pub epr_register: crate::epr_register::EprRegister,
    /// Consecutive time slots without a successful ebit.
    pub consecutive_failed_time_slots: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct BsmCandidate {
    pub left: usize,
    pub right: usize,
    pub repeater: usize,
    pub es_time: u64,
}
impl Ord for BsmCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .es_time
            .cmp(&self.es_time)
            .then_with(|| other.left.cmp(&self.left))
            .then_with(|| other.right.cmp(&self.right))
            .then_with(|| other.repeater.cmp(&self.repeater))
    }
}

impl PartialOrd for BsmCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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
        let max_signalling_time = crate::utils::distance_to_latency(
            (1 + 2 * params.num_repeaters) as f64 * params.distance,
        );

        // time to create the path, if specified
        let path_creation_latency = if params.create_path {
            crate::utils::distance_to_latency(params.num_repeaters as f64 * params.distance)
        } else {
            0.0
        };

        let time_slot_duration =
            local_entanglement_threshold + max_signalling_time + path_creation_latency;
        log::debug!("prob_local_entanglement = {}, local_entanglement_threshold = {}, max_signalling_time = {}, path_creation_latency = {}, time_slot_duration = {}",prob_local_entanglement, local_entanglement_threshold,max_signalling_time,path_creation_latency,time_slot_duration);

        Self {
            time_slot_duration,
            local_entanglement_threshold: crate::utils::to_nanoseconds(
                local_entanglement_threshold,
            ),
            rv_gen: rand_distr::Exp::new(mini_config.mini_parameters.rate).unwrap(),
            rv_bsm: rand_distr::Uniform::new(0.0, 1.0),
            rng: rand::rngs::StdRng::seed_from_u64(config.seed),
            mini_config,
            epr_register: Default::default(),
            consecutive_failed_time_slots: 0,
        }
    }

    pub fn get_metrics(
        series_ignore: std::collections::HashSet<String>,
    ) -> (crate::output::OutputScalar, crate::output::OutputSeries) {
        let mut single = crate::output::OutputScalar::default();
        single.init(
            "time_slot_duration",
            crate::output::ScalarMetricType::OneTime,
            crate::output::MetricMetadata::new("s", "Time slot duration", true),
        );
        single.init("bsm_prob", crate::output::ScalarMetricType::Avg, crate::output::MetricMetadata::new("probability", "Probability of a successful Bell State Measurement performed during an Entanglement Swapping operation", false));
        single.init(
            "ebit_prob",
            crate::output::ScalarMetricType::Avg,
            crate::output::MetricMetadata::new(
                "probability",
                "Probability that an end-to-end entanglement is established",
                false,
            ),
        );
        single.init(
            "ebit_tot",
            crate::output::ScalarMetricType::Count,
            crate::output::MetricMetadata::new(
                "ebits",
                "Number of successful end-to-end entanglements",
                false,
            ),
        );
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
        single.init(
            "fidelity",
            crate::output::ScalarMetricType::Avg,
            crate::output::MetricMetadata::new(
                "[0,1]",
                "End-to-end fidelity of successful ebits",
                false,
            ),
        );
        single.init(
            "latency",
            crate::output::ScalarMetricType::Avg,
            crate::output::MetricMetadata::new(
                "s",
                "End-to-end latency of successful ebits",
                false,
            ),
        );

        let mut series = crate::output::OutputSeries::new(series_ignore);
        series.init(
            "fidelity",
            &["node_id"],
            crate::output::MetricMetadata::new("[0,1]", "End-to-end fidelity", false),
        );
        series.init(
            "latency",
            &["node_id"],
            crate::output::MetricMetadata::new("s", "End-to-end latency", false),
        );

        (single, series)
    }

    pub fn handle_time_slot(&mut self, now: u64) -> Vec<Sample> {
        let mut samples = vec![];

        // Generate the EPR pairs, one per each pair of adjacent nodes.
        //
        //              R    R    R
        // (alice) 0 -- 1 -- 2 -- 3 -- 4 (bob)

        let n = self.mini_config.mini_parameters.num_repeaters as usize;
        let mut epr_pairs = vec![];
        #[derive(Debug)]
        struct EprPair {
            pub left: usize,
            pub right: usize,
            pub epr_pair_id: u64,
            pub generation_rel_time: u64,
        }
        for left in 0..=n {
            let generation_rel_time =
                crate::utils::to_nanoseconds(self.rv_gen.sample(&mut self.rng));
            let epr_pair_id = self.epr_register.new_epr_pair(
                left as u32,
                left as u32 + 1,
                now + generation_rel_time,
                self.mini_config.mini_parameters.fidelity_init,
            );
            epr_pairs.push(EprPair {
                left,
                right: left + 1,
                epr_pair_id,
                generation_rel_time,
            });
        }

        let mut bsm_success = true;
        for _ in 0..n {
            let single_bsm_success = self.rv_bsm.sample(&mut self.rng)
                < self.mini_config.mini_parameters.swapping_success_prob;
            bsm_success &= single_bsm_success;
            samples.push(Sample::ScalarAvg(
                "bsm_prob".to_string(),
                single_bsm_success as u32 as f64,
            ));
            samples.push(Sample::ScalarCount("bsm_tot".to_string()));
        }

        // If there is at least pair that was generated after the time horizon
        // for local entanglement in this time slot, the end-to-end
        // entanglement fails and return immediately.
        // Same if any of the repeaters fails the BSM.
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
            || !bsm_success
        {
            for epr_pair in epr_pairs {
                self.epr_register
                    .consume(epr_pair.epr_pair_id, epr_pair.left as u32);
                self.epr_register
                    .consume(epr_pair.epr_pair_id, epr_pair.right as u32);
            }
            samples.push(Sample::ScalarAvg("ebit_prob".to_string(), 0.0));
            log::debug!("new time slot: end-to-end entanglement failed");
            self.consecutive_failed_time_slots += 1;
            return samples;
        }

        // At this point, we are sure that the ebit succeeds.
        samples.push(Sample::ScalarAvg("ebit_prob".to_string(), 1.0));
        samples.push(Sample::ScalarCount("ebit_tot".to_string()));

        let mut bsm_candidates = std::collections::BinaryHeap::new();
        for repeater in 0..n {
            let left = repeater;
            let right = repeater + 1;
            let es_time = epr_pairs[left]
                .generation_rel_time
                .max(epr_pairs[right].generation_rel_time);
            bsm_candidates.push(BsmCandidate {
                left,
                right,
                repeater,
                es_time,
            });
        }

        let alice_id = 0_usize;
        let bob_id = n + 1;
        let end_nodes = [alice_id, bob_id];
        let mut latencies = std::collections::HashMap::from([(alice_id, 0.0), (bob_id, 0.0)]);
        let mut last_es = 0;
        let mut last_epr_pair_id = Some(epr_pairs.first().unwrap().epr_pair_id);
        while let Some(bsm_candidate) = bsm_candidates.pop() {
            let epr_pair_id_new = self.epr_register.entanglement_swapping(
                now + bsm_candidate.es_time,
                self.mini_config.mini_parameters.decay_rate,
                epr_pairs[bsm_candidate.left].epr_pair_id,
                epr_pairs[bsm_candidate.right].epr_pair_id,
                bsm_candidate.repeater as u32 + 1,
            );

            // All the corrections from the repeaters go (arbitrarily) to Bob.
            let latency = latencies.get_mut(&bob_id).unwrap();
            let num_hops = n - bsm_candidate.repeater;
            let new_latency = crate::utils::distance_to_latency(
                num_hops as f64 * self.mini_config.mini_parameters.distance,
            ) + self.mini_config.mini_parameters.swapping_duration
                + self.mini_config.mini_parameters.correction_duration;
            if new_latency > *latency {
                *latency = new_latency;
            }

            last_es = last_es.max(bsm_candidate.es_time);

            epr_pairs[bsm_candidate.left].epr_pair_id = epr_pair_id_new;
            epr_pairs[bsm_candidate.right].epr_pair_id = epr_pair_id_new;
            last_epr_pair_id = Some(epr_pair_id_new);
        }
        let bob_latency = latencies[&bob_id];
        *latencies.get_mut(&alice_id).unwrap() = bob_latency
            + crate::utils::distance_to_latency(
                (n as f64 + 1.0) * self.mini_config.mini_parameters.distance,
            );

        let last_epr_pair_id = last_epr_pair_id.unwrap();
        let mut fidelities = vec![];
        for end_node in end_nodes {
            let (_updated, fidelity) = self
                .epr_register
                .consume(last_epr_pair_id, end_node as u32)
                .unwrap();

            let end_fidelity = crate::utils::fidelity_decay(
                fidelity,
                self.mini_config.mini_parameters.decay_rate,
                latencies[&end_node],
            );
            fidelities.push(end_fidelity);
        }

        for (id, fidelity) in fidelities.iter().enumerate() {
            samples.push(Sample::Series(
                "fidelity".to_string(),
                vec![id.to_string()],
                *fidelity,
            ));
            samples.push(Sample::ScalarAvg("fidelity".to_string(), *fidelity));
        }
        for (node_id, latency) in &latencies {
            let latency =
                *latency + self.time_slot_duration * self.consecutive_failed_time_slots as f64;
            let id = if *node_id == alice_id { 0 } else { 1 };
            samples.push(Sample::Series(
                "latency".to_string(),
                vec![id.to_string()],
                latency,
            ));
            samples.push(Sample::ScalarAvg("latency".to_string(), latency));
        }

        log::debug!(
            "new time slot: end-to-end entanglement succeeded with latencies {:?} fidelities {:?}",
            latencies.values(),
            fidelities
        );
        self.consecutive_failed_time_slots = 0;

        samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::mini_config::MiniConfig;

    #[test]
    fn test_mini_sync() {
        let num_repeaters_values = [1, 2, 3, 5];
        let mut ebit_probs = vec![];
        let mut fidelities = vec![];
        for num_repeaters in num_repeaters_values {
            let mut mini_config = MiniConfig::default();
            mini_config.mini_parameters.num_repeaters = num_repeaters;
            let mut mini_sync = MiniSync::new(&Config { seed: 42 }, mini_config);

            let mut samples = vec![];
            let time_slot_duration = 100000000_u64;
            let num_time_slots = 1000;
            for i in 0..num_time_slots {
                samples.append(&mut mini_sync.handle_time_slot(i * time_slot_duration));
            }

            let (mut single, mut series) = MiniSync::get_metrics(std::collections::HashSet::new());
            single.enable(0);
            series.enable();

            crate::output::add_all_samples(
                num_time_slots * time_slot_duration,
                samples,
                &mut single,
                &mut series,
            );

            let bsm_prob = *single.values().get("bsm_prob").unwrap();
            let ebit_prob = *single.values().get("ebit_prob").unwrap();
            let ebit_tot = *single.values().get("ebit_tot").unwrap();
            println!(
                "{} repeaters: {} {} {}",
                num_repeaters, bsm_prob, ebit_prob, ebit_tot
            );
            assert_eq!(95, (bsm_prob * 100.0).round() as usize);
            assert_eq!((ebit_prob * 1000.0).round() as usize, ebit_tot as usize);
            ebit_probs.push(ebit_prob);

            let fidelity = series.series.get(&String::from("fidelity")).unwrap();
            let (_min, avg, _max, cnt) = fidelity.stats();
            println!("{} repeaters: {} {}", num_repeaters, avg, cnt);
            assert_eq!(cnt, ebit_tot as usize * 2);
            fidelities.push(avg);
        }

        for i in 1..ebit_probs.len() {
            assert!(ebit_probs[i] < ebit_probs[i - 1]);
            assert!(fidelities[i] < fidelities[i - 1]);
        }
    }
}
