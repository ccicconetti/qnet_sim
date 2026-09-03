// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::utils::CsvFriend;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// The seed to initialize pseudo-random number generators.
    pub seed: u64,
}

impl CsvFriend for Config {
    fn header(&self) -> String {
        "seed".to_string()
    }
    fn to_csv(&self) -> String {
        format!("{}", self.seed)
    }
}
