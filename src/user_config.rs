// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

fn default_sat_weight() -> crate::physical_topology::NodeWeight {
    crate::physical_topology::NodeWeight {
        node_type: crate::physical_topology::NodeType::SAT,
        memory_qubits: 20,
        decay_rate: 1.0,
        swapping_success_prob: 0.95,
        detectors: 10,
        transmitters: 10,
        capacity: 1000.0,
    }
}

fn default_ogs_weight() -> crate::physical_topology::NodeWeight {
    crate::physical_topology::NodeWeight {
        node_type: crate::physical_topology::NodeType::OGS,
        memory_qubits: 100,
        decay_rate: 1.0,
        swapping_success_prob: 0.0,
        detectors: 10,
        transmitters: 0,
        capacity: 0.0,
    }
}

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
            sat_weight: default_sat_weight(),
            ogs_weight: default_ogs_weight(),
            fidelities: Default::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfChainStatic {
    pub chain_params: crate::physical_topology::ChainParams,
    pub sat_weight: crate::physical_topology::NodeWeight,
    pub ogs_weight: crate::physical_topology::NodeWeight,
    pub fidelities: crate::physical_topology::StaticFidelities,
}

impl Default for ConfChainStatic {
    fn default() -> Self {
        Self {
            chain_params: Default::default(),
            sat_weight: default_sat_weight(),
            ogs_weight: default_ogs_weight(),
            fidelities: Default::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicalTopology {
    ConfGridStatic(ConfGridStatic),
    ConfChainStatic(ConfChainStatic),
}

impl PhysicalTopology {
    pub fn to_physical_topology(
        &self,
    ) -> anyhow::Result<crate::physical_topology::PhysicalTopology> {
        match self {
            PhysicalTopology::ConfGridStatic(conf) => {
                crate::physical_topology::PhysicalTopology::from_grid_static(
                    conf.grid_params.clone(),
                    conf.sat_weight.clone(),
                    conf.ogs_weight.clone(),
                    conf.fidelities.clone(),
                )
            }
            PhysicalTopology::ConfChainStatic(conf) => {
                crate::physical_topology::PhysicalTopology::from_chain_static(
                    conf.chain_params.clone(),
                    conf.sat_weight.clone(),
                    conf.ogs_weight.clone(),
                    conf.fidelities.clone(),
                )
            }
        }
    }
}

impl crate::utils::CsvFriend for PhysicalTopology {
    fn header(&self) -> String {
        match &self {
            PhysicalTopology::ConfGridStatic(conf) => format!(
                "{},{},{},{}",
                crate::utils::struct_to_csv_header(&conf.grid_params).unwrap(),
                crate::utils::struct_to_csv_header(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.ogs_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.fidelities).unwrap()
            ),
            PhysicalTopology::ConfChainStatic(conf) => format!(
                "{},{},{},{}",
                crate::utils::struct_to_csv_header(&conf.chain_params).unwrap(),
                crate::utils::struct_to_csv_header(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.ogs_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.fidelities).unwrap()
            ),
        }
    }

    fn to_csv(&self) -> String {
        match &self {
            PhysicalTopology::ConfGridStatic(conf) => format!(
                "{},{},{},{}",
                crate::utils::struct_to_csv(&conf.grid_params).unwrap(),
                crate::utils::struct_to_csv(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.ogs_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.fidelities).unwrap()
            ),
            PhysicalTopology::ConfChainStatic(conf) => format!(
                "{},{},{},{}",
                crate::utils::struct_to_csv(&conf.chain_params).unwrap(),
                crate::utils::struct_to_csv(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.ogs_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.fidelities).unwrap()
            ),
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
pub enum SourceDestPairs {
    Random(usize),
    AllToAll,
}

impl Default for SourceDestPairs {
    fn default() -> Self {
        Self::Random(1)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfPing {
    pub source_dest_pairs: SourceDestPairs,
    pub max_requests: u64,
}

impl Default for ConfPing {
    fn default() -> Self {
        Self {
            source_dest_pairs: SourceDestPairs::default(),
            max_requests: 1,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfClientServer {
    pub source_dest_pairs: SourceDestPairs,
    pub operation_rate: f64,
    pub operation_avg_dur_client: f64,
    pub operation_avg_dur_server: f64,
}

impl Default for ConfClientServer {
    fn default() -> Self {
        Self {
            source_dest_pairs: SourceDestPairs::default(),
            operation_rate: 1.0,
            operation_avg_dur_client: 0.1,
            operation_avg_dur_server: 0.1,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Applications {
    ConfPing(ConfPing),
    ConfClientServer(ConfClientServer),
}

impl Default for Applications {
    fn default() -> Self {
        Self::ConfPing(ConfPing::default())
    }
}

impl crate::utils::CsvFriend for Applications {
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
    /// Time series metrics to ignore.
    pub series_ignore: std::collections::HashSet<String>,
    /// The physical topology configuration.
    pub physical_topology: PhysicalTopology,
    /// The logical topology configuration.
    pub logical_topology: LogicalTopology,
    /// The applications.
    pub applications: Applications,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            duration: 10.0,
            warmup_period: 1.0,
            series_ignore: std::collections::HashSet::new(),
            physical_topology: PhysicalTopology::ConfGridStatic(ConfGridStatic::default()),
            logical_topology: LogicalTopology::default(),
            applications: Applications::default(),
        }
    }
}

impl crate::utils::CsvFriend for UserConfig {
    fn header(&self) -> String {
        format!(
            "duration,warmup_period,{},{},{}",
            self.physical_topology.header(),
            self.logical_topology.header(),
            self.applications.header()
        )
    }
    fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.duration,
            self.warmup_period,
            self.physical_topology.to_csv(),
            self.logical_topology.to_csv(),
            self.applications.to_csv()
        )
    }
}
