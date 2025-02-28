// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfGridStatic {
    pub grid_params: crate::physical_topology::GridParams,
    pub sat_weight: crate::physical_topology::NodeWeight,
    pub ogs_weight: crate::physical_topology::NodeWeight,
    pub fidelities: crate::physical_topology::StaticFidelities,
}

impl Default for ConfGridStatic {
    fn default() -> Self {
        Self {
            grid_params: Default::default(),
            sat_weight: crate::physical_topology::NodeWeight {
                node_type: crate::physical_topology::NodeType::SAT,
                memory_qubits: 20,
                decay_rate: 1.0,
                swapping_success_prob: 0.95,
                detectors: 10,
                transmitters: 10,
                capacity: 1000.0,
            },
            ogs_weight: crate::physical_topology::NodeWeight {
                node_type: crate::physical_topology::NodeType::OGS,
                memory_qubits: 100,
                decay_rate: 1.0,
                swapping_success_prob: 0.0,
                detectors: 10,
                transmitters: 0,
                capacity: 0.0,
            },
            fidelities: Default::default(),
        }
    }
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
                    "{},{},{},{}",
                    crate::utils::struct_to_csv_header(&conf.grid_params).unwrap(),
                    crate::utils::struct_to_csv_header(&conf.sat_weight).unwrap(),
                    crate::utils::struct_to_csv_header(&conf.ogs_weight).unwrap(),
                    crate::utils::struct_to_csv_header(&conf.fidelities).unwrap()
                )
            }
        }
    }

    fn to_csv(&self) -> String {
        match &self {
            PhysicalTopology::ConfGridStatic(conf) => {
                format!(
                    "{},{},{},{}",
                    crate::utils::struct_to_csv(&conf.grid_params).unwrap(),
                    crate::utils::struct_to_csv(&conf.sat_weight).unwrap(),
                    crate::utils::struct_to_csv(&conf.ogs_weight).unwrap(),
                    crate::utils::struct_to_csv(&conf.fidelities).unwrap()
                )
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogicalTopology {
    pub physical_to_logical_policy: crate::logical_topology::PhysicalToLogicalPolicy,
}

impl Default for LogicalTopology {
    fn default() -> Self {
        Self {
            physical_to_logical_policy:
                crate::logical_topology::PhysicalToLogicalPolicy::RandomGreedy,
        }
    }
}

impl crate::utils::CsvFriend for LogicalTopology {
    fn header(&self) -> String {
        crate::utils::struct_to_csv_header(self).unwrap()
    }

    fn to_csv(&self) -> String {
        crate::utils::struct_to_csv(self).unwrap()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    /// The duration of the simulation, in s.
    pub duration: f64,
    /// The warm-up period, in s.
    pub warmup_period: f64,
    /// The physical topology configuration.
    pub physical_topology: PhysicalTopology,
    /// The logical topology configuration.
    pub logical_topology: LogicalTopology,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            duration: 10.0,
            warmup_period: 1.0,
            physical_topology: PhysicalTopology::ConfGridStatic(ConfGridStatic::default()),
            logical_topology: LogicalTopology::default(),
        }
    }
}

impl crate::utils::CsvFriend for UserConfig {
    fn header(&self) -> String {
        format!(
            "duration,warmup_period,{},{}",
            self.physical_topology.header(),
            self.logical_topology.header()
        )
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{},{},{}",
            self.duration,
            self.warmup_period,
            self.physical_topology.to_csv(),
            self.logical_topology.to_csv()
        )
    }
}
