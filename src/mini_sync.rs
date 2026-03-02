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
        let max_signalling_time =
            crate::utils::distance_to_latency(params.num_repeaters as f64 * params.distance);

        let time_slot_duration = local_entanglement_threshold + max_signalling_time;
        log::debug!("prob_local_entanglement = {}, local_entanglement_threshold = {}, max_signalling_time = {}, time_slot_duration = {}",prob_local_entanglement, local_entanglement_threshold,max_signalling_time,time_slot_duration);

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
        }
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
            return samples;
        }

        // At this point, we are sure that the ebit succeeds.
        samples.push(Sample::ScalarAvg("ebit_prob".to_string(), 1.0));

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
            for end_node in end_nodes {
                let latency = latencies.get_mut(&end_node).unwrap();
                let num_hops = if end_node == alice_id {
                    bsm_candidate.repeater + 1
                } else {
                    n - bsm_candidate.repeater
                };
                let new_latency = crate::utils::distance_to_latency(
                    num_hops as f64 * self.mini_config.mini_parameters.distance,
                ) + self.mini_config.mini_parameters.swapping_duration
                    + self.mini_config.mini_parameters.correction_duration;
                if new_latency > *latency {
                    *latency = new_latency;
                }
            }
            last_es = last_es.max(bsm_candidate.es_time);

            epr_pairs[bsm_candidate.left].epr_pair_id = epr_pair_id_new;
            epr_pairs[bsm_candidate.right].epr_pair_id = epr_pair_id_new;
            last_epr_pair_id = Some(epr_pair_id_new);
        }

        let last_epr_pair_id = last_epr_pair_id.unwrap();
        let mut fidelities = vec![];
        for end_node in end_nodes {
            let (updated, fidelity) = self
                .epr_register
                .consume(last_epr_pair_id, end_node as u32)
                .unwrap();

            assert!(updated >= now);
            let elapsed_since_es = latencies[&end_node] - crate::utils::to_seconds(updated - now);
            let end_fidelity = crate::utils::fidelity_decay(
                fidelity,
                self.mini_config.mini_parameters.decay_rate,
                elapsed_since_es,
            );
            fidelities.push(end_fidelity);
        }

        for id in 0..fidelities.len() {
            samples.push(Sample::Series(
                "fidelity".to_string(),
                vec![id.to_string()],
                fidelities[id],
            ));
        }

        log::debug!(
            "new time slot: end-to-end entanglement succeeded with latencies {:?} fidelities {:?}",
            latencies.values(),
            fidelities
        );

        samples
    }
}
