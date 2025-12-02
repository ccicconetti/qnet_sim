// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StaticFidelities {
    /// One hop, orbit-to-orbit.
    pub f_o: f64,
    /// One hop, orbit-to-ground.
    pub f_g: f64,
    /// Two hops, orbit-to-orbit.
    pub f_oo: f64,
    /// Two hops, orbit-to-ground.
    pub f_og: f64,
    /// Two hops, ground-to-ground.
    pub f_gg: f64,
}

impl Default for StaticFidelities {
    fn default() -> Self {
        Self {
            f_o: 1.0,
            f_g: 1.0,
            f_oo: 1.0,
            f_og: 1.0,
            f_gg: 1.0,
        }
    }
}

impl StaticFidelities {
    pub fn valid(&self) -> anyhow::Result<()> {
        let fidelities = vec![
            (self.f_o, "one-hop, orbit-to-orbit"),
            (self.f_g, "one-hop, orbit-to-ground"),
            (self.f_oo, "two-hops, orbit-to-orbit"),
            (self.f_og, "two-hops, orbit-to-ground"),
            (self.f_gg, "two-hops, ground-to-ground"),
        ];
        let mut errors = vec![];
        for (fidelity, name) in fidelities {
            if fidelity < 0.0 {
                errors.push(format!("{fidelity} fidelity ({name}) is < 0"));
            } else if fidelity > 1.0 {
                errors.push(format!("{fidelity} fidelity ({name}) is > 1"));
            }
        }
        if !errors.is_empty() {
            anyhow::bail!("invalid static fidelities: {}", errors.join(","))
        }
        Ok(())
    }
}
