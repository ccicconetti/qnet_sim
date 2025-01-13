// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    /// The duration of the simulation, in s.
    pub duration: f64,
    /// The warm-up period, in s.
    pub warmup_period: f64,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            duration: 10.0,
            warmup_period: 1.0,
        }
    }
}

impl UserConfig {
    pub fn header() -> String {
        String::from("duration,warmup_period")
    }
    pub fn to_csv(&self) -> String {
        format!("{},{}", self.duration, self.warmup_period)
    }
}
