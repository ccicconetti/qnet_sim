// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::user_config::UserConfig;
use crate::utils::CsvFriend;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// The seed to initialize pseudo-random number generators.
    pub seed: u64,
    /// The user-specified configuration.
    pub user_config: UserConfig,
}

impl CsvFriend for Config {
    fn header(&self) -> String {
        format!("seed,{}", self.user_config.header())
    }
    fn to_csv(&self) -> String {
        format!("{},{}", self.seed, self.user_config.to_csv())
    }
}
