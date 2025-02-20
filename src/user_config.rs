// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ConfGridStatic {
    pub grid_params: crate::physical_topology::GridParams,
    pub node_weight: crate::physical_topology::NodeWeight,
    pub fidelities: crate::physical_topology::StaticFidelities,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicalTopology {
    ConfGridStatic(ConfGridStatic),
}

impl crate::utils::CsvFriend for PhysicalTopology {
    fn header(&self) -> String {
        match &self {
            PhysicalTopology::ConfGridStatic(conf) => {
                format!(
                    "{},{},{}",
                    crate::utils::struct_to_csv_header(&conf.grid_params).unwrap(),
                    crate::utils::struct_to_csv_header(&conf.node_weight).unwrap(),
                    crate::utils::struct_to_csv_header(&conf.fidelities).unwrap()
                )
            }
        }
    }

    fn to_csv(&self) -> String {
        match &self {
            PhysicalTopology::ConfGridStatic(conf) => {
                format!(
                    "{},{},{}",
                    crate::utils::struct_to_csv(&conf.grid_params).unwrap(),
                    crate::utils::struct_to_csv(&conf.node_weight).unwrap(),
                    crate::utils::struct_to_csv(&conf.fidelities).unwrap()
                )
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    /// The duration of the simulation, in s.
    pub duration: f64,
    /// The warm-up period, in s.
    pub warmup_period: f64,
    /// The physical topology.
    pub physical_topology: PhysicalTopology,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            duration: 10.0,
            warmup_period: 1.0,
            physical_topology: PhysicalTopology::ConfGridStatic(ConfGridStatic::default()),
        }
    }
}

impl crate::utils::CsvFriend for UserConfig {
    fn header(&self) -> String {
        format!("duration,warmup_period,{}", self.physical_topology.header())
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{},{}",
            self.duration,
            self.warmup_period,
            self.physical_topology.to_csv()
        )
    }
}
