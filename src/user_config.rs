// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::physical_topology::{PhysicalTopologyParams, StaticFidelities};
use rand::SeedableRng;
use rand_distr::Distribution;

pub fn default_sat_weight() -> crate::physical_topology::NodeWeight {
    crate::physical_topology::NodeWeight {
        label: None,
        node_type: crate::physical_topology::NodeType::SAT,
        is_repeater: true,
        memory_qubits: 20,
        decay_rate: 1.0,
        swapping_success_prob: 0.95,
        swapping_duration: 0.001,
        correction_duration: 0.0,
        detectors: 10,
        transmitters: 10,
        capacity: 1000.0,
    }
}

pub fn default_ogs_weight() -> crate::physical_topology::NodeWeight {
    crate::physical_topology::NodeWeight {
        label: None,
        node_type: crate::physical_topology::NodeType::OGS,
        is_repeater: false,
        memory_qubits: 100,
        decay_rate: 1.0,
        swapping_success_prob: 0.0,
        swapping_duration: 0.0,
        correction_duration: 0.001,
        detectors: 10,
        transmitters: 0,
        capacity: 0.0,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfGrid {
    pub grid_params: crate::physical_topology::GridParams,
    pub sat_weight: crate::physical_topology::NodeWeight,
    pub ogs_weight: crate::physical_topology::NodeWeight,
}

impl Default for ConfGrid {
    fn default() -> Self {
        Self {
            grid_params: Default::default(),
            sat_weight: default_sat_weight(),
            ogs_weight: default_ogs_weight(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfChain {
    pub chain_params: crate::physical_topology::ChainParams,
    pub sat_weight: crate::physical_topology::NodeWeight,
    pub ogs_weight: crate::physical_topology::NodeWeight,
}

impl Default for ConfChain {
    fn default() -> Self {
        Self {
            chain_params: Default::default(),
            sat_weight: default_sat_weight(),
            ogs_weight: default_ogs_weight(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PhysicalTopology {
    Grid(ConfGrid),
    Chain(ConfChain),
}

impl PhysicalTopology {
    pub fn make(&self, seed: u64) -> anyhow::Result<crate::physical_topology::PhysicalTopology> {
        match self {
            PhysicalTopology::Grid(conf) => conf.grid_params.make_topology(
                conf.sat_weight.clone(),
                conf.ogs_weight.clone(),
                seed,
            ),
            PhysicalTopology::Chain(conf) => conf.chain_params.make_topology(
                conf.sat_weight.clone(),
                conf.ogs_weight.clone(),
                seed,
            ),
        }
    }
}

impl crate::utils::CsvFriend for PhysicalTopology {
    fn header(&self) -> String {
        match &self {
            PhysicalTopology::Grid(conf) => format!(
                "{},{},{}",
                crate::utils::struct_to_csv_header(&conf.grid_params).unwrap(),
                crate::utils::struct_to_csv_header(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.ogs_weight).unwrap(),
            ),
            PhysicalTopology::Chain(conf) => format!(
                "{},{},{}",
                crate::utils::struct_to_csv_header(&conf.chain_params).unwrap(),
                crate::utils::struct_to_csv_header(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv_header(&conf.ogs_weight).unwrap(),
            ),
        }
    }

    fn to_csv(&self) -> String {
        match &self {
            PhysicalTopology::Grid(conf) => format!(
                "{},{},{}",
                crate::utils::struct_to_csv(&conf.grid_params).unwrap(),
                crate::utils::struct_to_csv(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.ogs_weight).unwrap(),
            ),
            PhysicalTopology::Chain(conf) => format!(
                "{},{},{}",
                crate::utils::struct_to_csv(&conf.chain_params).unwrap(),
                crate::utils::struct_to_csv(&conf.sat_weight).unwrap(),
                crate::utils::struct_to_csv(&conf.ogs_weight).unwrap(),
            ),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FidelityComputer {
    StaticFidelities(crate::physical_topology::StaticFidelities),
}

impl FidelityComputer {
    pub fn make(&self) -> Box<dyn crate::physical_topology::FidelityComputer> {
        match self {
            Self::StaticFidelities(conf) => Box::new(conf.clone()),
        }
    }
}

impl crate::utils::CsvFriend for FidelityComputer {
    fn header(&self) -> String {
        match &self {
            Self::StaticFidelities(conf) => {
                format!("{}", crate::utils::struct_to_csv_header(&conf).unwrap(),)
            }
        }
    }

    fn to_csv(&self) -> String {
        match &self {
            Self::StaticFidelities(conf) => {
                format!("{}", crate::utils::struct_to_csv(&conf).unwrap(),)
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
pub enum SourceDestPairs {
    Random(usize),
    AllToAll,
    List(Vec<(u32, u32)>),
}

impl Default for SourceDestPairs {
    fn default() -> Self {
        Self::Random(1)
    }
}

impl SourceDestPairs {
    /// Make source/destination pairs.
    pub fn make_pairs(&self, end_nodes: Vec<u32>, seed: u64) -> Vec<(u32, u32)> {
        let mut source_dest_pairs = vec![];
        match self {
            Self::Random(num_applications) => {
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
                let uniform = rand_distr::Uniform::new(0, end_nodes.len());
                for _ in 0..*num_applications {
                    let this_node_ndx = uniform.sample(&mut rng) as u32;
                    let peer_node_ndx = loop {
                        let candidate = uniform.sample(&mut rng) as u32;
                        if candidate != this_node_ndx {
                            break candidate;
                        }
                    };
                    source_dest_pairs.push((
                        end_nodes[this_node_ndx as usize],
                        end_nodes[peer_node_ndx as usize],
                    ));
                }
            }
            Self::AllToAll => {
                for this_node_id in &end_nodes {
                    for peer_node_id in &end_nodes {
                        if this_node_id == peer_node_id {
                            continue;
                        }
                        source_dest_pairs.push((*this_node_id, *peer_node_id));
                    }
                }
            }
            Self::List(pairs) => {
                for (this_node_id, peer_node_id) in pairs {
                    assert!(end_nodes.iter().find(|x| *x == this_node_id).is_some());
                    assert!(end_nodes.iter().find(|x| *x == peer_node_id).is_some());
                    source_dest_pairs.push((*this_node_id, *peer_node_id));
                }
            }
        }
        source_dest_pairs
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
    /// Sections that are not serialized.
    pub sections_not_serialized: std::collections::HashSet<String>,
    /// The physical topology configuration.
    pub physical_topology: PhysicalTopology,
    /// The fidelity computer.
    pub fidelity_computer: FidelityComputer,
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
            sections_not_serialized: std::collections::HashSet::new(),
            physical_topology: PhysicalTopology::Grid(ConfGrid::default()),
            fidelity_computer: FidelityComputer::StaticFidelities(StaticFidelities::default()),
            logical_topology: LogicalTopology::default(),
            applications: Applications::default(),
        }
    }
}

impl crate::utils::CsvFriend for UserConfig {
    fn header(&self) -> String {
        let mut ret = format!("duration,warmup_period");
        if !self.sections_not_serialized.contains("physical_topology") {
            ret += ",";
            ret += &self.physical_topology.header();
        }
        if !self.sections_not_serialized.contains("fidelity_computer") {
            ret += ",";
            ret += &self.fidelity_computer.header();
        }
        if !self.sections_not_serialized.contains("logical_topology") {
            ret += ",";
            ret += &self.logical_topology.header();
        }
        if !self.sections_not_serialized.contains("applications") {
            ret += ",";
            ret += &self.applications.header();
        }
        ret
    }
    fn to_csv(&self) -> String {
        let mut ret = format!("{},{}", self.duration, self.warmup_period);
        if !self.sections_not_serialized.contains("physical_topology") {
            ret += ",";
            ret += &self.physical_topology.to_csv();
        }
        if !self.sections_not_serialized.contains("fidelity_computer") {
            ret += ",";
            ret += &self.fidelity_computer.to_csv();
        }
        if !self.sections_not_serialized.contains("logical_topology") {
            ret += ",";
            ret += &self.logical_topology.to_csv();
        }
        if !self.sections_not_serialized.contains("applications") {
            ret += ",";
            ret += &self.applications.to_csv();
        }
        ret
    }
}
