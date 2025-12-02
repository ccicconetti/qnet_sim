// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainParams {
    /// Distance between two neighbor satellites, in m.
    pub orbit_to_orbit_distance: f64,
    /// Distance between an OGS and a satellite, in m.
    pub ground_to_orbit_distance: f64,
    /// Number of satellite repeaters.
    pub num_repeaters: u32,
    /// Minimum eleveation, in degrees.
    pub elevation_min: f64,
    /// Maximum elevation, in degrees.
    pub elevation_max: f64,
}

impl Default for ChainParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000000.0,
            ground_to_orbit_distance: 1000000.0,
            num_repeaters: 1,
            elevation_min: 10.0,
            elevation_max: 60.0,
        }
    }
}

fn err_if_not_empty(errors: &[String]) -> anyhow::Result<()> {
    if !errors.is_empty() {
        anyhow::bail!(
            "invalid physical topology chain parameters: {}",
            errors.join(",")
        )
    }
    Ok(())
}
impl ChainParams {
    pub fn valid(&self) -> anyhow::Result<()> {
        let mut errors = vec![];
        if self.orbit_to_orbit_distance < 0.0 {
            errors.push(format!(
                "orbit-to-orbit distance ({}) < 0",
                self.orbit_to_orbit_distance
            ))
        }
        if self.ground_to_orbit_distance < 0.0 {
            errors.push(format!(
                "ground-to-orbit distance ({}) < 0",
                self.ground_to_orbit_distance
            ))
        }
        if self.num_repeaters == 0 {
            errors.push(String::from("vanishing number of repeaters"));
        }
        err_if_not_empty(&errors)
    }
}
