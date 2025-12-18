// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixedRate {
    /// Fixed generation rate of EPR pairs.
    pub rate: f64,
}

impl Default for FixedRate {
    fn default() -> Self {
        Self { rate: 1000.0 }
    }
}

impl super::rate_computer::RateComputer for FixedRate {
    fn rate(
        &self,
        topology: &super::PhysicalTopology,
        tx: u32,
        u: u32,
        v: u32,
    ) -> anyhow::Result<f64> {
        topology.node_valid(tx)?;
        topology.node_valid(u)?;
        topology.node_valid(v)?;
        Ok(self.rate)
    }

    fn valid(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.rate >= 0.0,
            "negative fixed rate of EPR generation: {}",
            self.rate
        );
        Ok(())
    }
}
