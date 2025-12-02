// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GridParams {
    /// Distance between two neighbor satellites, in m.
    pub orbit_to_orbit_distance: f64,
    /// Distance between an OGS and a satellite, in m.
    pub ground_to_orbit_distance: f64,
    /// Number of orbits.
    pub num_orbits: u32,
    /// Number of satellites in each orbit.
    pub orbit_length: u32,
    /// Minimum eleveation, in degrees.
    pub elevation_min: f64,
    /// Maximum elevation, in degrees.
    pub elevation_max: f64,
}

impl Default for GridParams {
    fn default() -> Self {
        Self {
            orbit_to_orbit_distance: 3000000.0,
            ground_to_orbit_distance: 1000000.0,
            num_orbits: 3,
            orbit_length: 4,
            elevation_min: 10.0,
            elevation_max: 60.0,
        }
    }
}

fn err_if_not_empty(errors: &[String]) -> anyhow::Result<()> {
    if !errors.is_empty() {
        anyhow::bail!(
            "invalid physical topology grid parameters: {}",
            errors.join(",")
        )
    }
    Ok(())
}

impl GridParams {
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
        if self.num_orbits == 0 {
            errors.push(String::from("vanishing number of orbits"));
        }
        if self.orbit_length == 0 {
            errors.push(String::from("vanishing orbit length"));
        }
        err_if_not_empty(&errors)
    }
}
