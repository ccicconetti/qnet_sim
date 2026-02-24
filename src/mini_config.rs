// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiniParameters {
    /// Initial local fidelity.
    pub fidelity_init: f64,
}

impl Default for MiniParameters {
    fn default() -> Self {
        Self {
            fidelity_init: 0.95,
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
