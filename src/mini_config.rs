// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncConfig {
    /// Target probability that all the local entanglements are generated
    /// on time within the slot. Used to compute the slot duration.
    pub prob_local_complete: f64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            prob_local_complete: 0.95,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Protocol {
    Sync(SyncConfig),
    Async,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiniParameters {
    /// Protocol type.
    pub protocol: Protocol,
    /// Number of repeaters. The number of entangled source generators is
    /// the number of repeaters + 1.
    pub num_repeaters: u32,
    /// Physical distance between repeaters/end-nodes, in m.
    pub distance: f64,
    /// Generation rate of EPR pairs, in Hz.
    pub rate: f64,
    /// Initial local fidelity.
    pub fidelity_init: f64,
    /// Fidelity decay rate of a qubit in memory.
    pub decay_rate: f64,
    /// Entanglement swapping success probability.
    pub swapping_success_prob: f64,
    /// Entanglement swapping duration, in s.
    pub swapping_duration: f64,
    /// Duration of the local operations to correct end-to-end pairs, in s.
    pub correction_duration: f64,
}

impl Default for MiniParameters {
    fn default() -> Self {
        Self {
            protocol: Protocol::Sync(SyncConfig::default()),
            num_repeaters: 1,
            distance: 1000000.0,
            rate: 100.0,
            fidelity_init: 0.95,
            decay_rate: 1.0,
            swapping_success_prob: 0.95,
            swapping_duration: 0.001,
            correction_duration: 0.001,
        }
    }
}

impl crate::utils::CsvFriend for MiniParameters {
    fn header(&self) -> String {
        crate::utils::struct_to_csv_header(self).unwrap()
    }

    fn to_csv(&self) -> String {
        crate::utils::struct_to_csv(self).unwrap()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiniConfig {
    /// The duration of the simulation, in s.
    pub duration: f64,
    /// The warm-up period, in s.
    pub warmup_period: f64,
    /// Time series metrics to ignore.
    pub series_ignore: std::collections::HashSet<String>,
    /// Sections that are not serialized.
    pub sections_not_serialized: std::collections::HashSet<String>,
    /// Simulation parameters.
    pub mini_parameters: MiniParameters,
}

impl Default for MiniConfig {
    fn default() -> Self {
        Self {
            duration: 1.0,
            warmup_period: 0.0,
            series_ignore: std::collections::HashSet::new(),
            sections_not_serialized: std::collections::HashSet::new(),
            mini_parameters: MiniParameters::default(),
        }
    }
}

impl crate::utils::CsvFriend for MiniConfig {
    fn header(&self) -> String {
        format!("duration,warmup_period,{}", self.mini_parameters.header())
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{},{}",
            self.duration,
            self.warmup_period,
            self.mini_parameters.to_csv()
        )
    }
}
