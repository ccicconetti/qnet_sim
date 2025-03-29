// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::output::Sample;

#[derive(Debug, PartialEq, Eq)]
pub struct EprGeneratedData {
    pub tx_node_id: u32,
    pub master_node_id: u32,
    pub slave_node_id: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EprNotifiedData {
    pub this_node_id: u32,
    pub peer_node_id: u32,
    pub role: crate::nic::Role,
    pub epr_pair_id: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EprFidelityData {
    /// ID of the node where the application runs.
    pub app_node_id: u32,
    /// Port where the application runes.
    pub port: u16,
    /// ID of the node that consumes the EPR.
    pub consume_node_id: u32,
    /// ID of the neighbor to identify the NIC.
    pub neighbor_node_id: u32,
    /// Role of the consuming node.
    pub role: crate::nic::Role,
    /// Index of the memory cell in the NIC.
    pub index: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeEventData {
    /// New EPR generated by a tx.
    EprGenerated(EprGeneratedData),
    /// EPR pair notified at a node.
    EprNotified(EprNotifiedData),
    /// Measure fidelity of a given EPR pair.
    EprFidelity(EprFidelityData),
}

/// Every EPR request is uniquely identified by the five-tuple:
/// - source node ID and port
/// - target node ID and port
/// - request ID
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EprFiveTuple {
    /// Source node ID.
    pub source_node_id: u32,
    /// Source port.
    pub source_port: u16,
    /// Target node ID.
    pub target_node_id: u32,
    /// Target port.
    pub target_port: u16,
    /// Request ID
    pub request_id: u64,
}

impl EprFiveTuple {
    pub fn new(
        source_node_id: u32,
        source_port: u16,
        target_node_id: u32,
        target_port: u16,
        request_id: u64,
    ) -> Self {
        Self {
            source_node_id,
            source_port,
            target_node_id,
            target_port,
            request_id,
        }
    }
}

impl std::fmt::Display for EprFiveTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "src {}:{} dst {}:{} id {}",
            self.source_node_id,
            self.source_port,
            self.target_node_id,
            self.target_port,
            self.request_id
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EprResponseData {
    /// Five-tuple associated with this EPR.
    pub epr: EprFiveTuple,
    /// Neighbor node ID, used to identify the NIC, and memory cell index.
    /// If None then the request failed.
    pub memory_cell: Option<(u32, crate::nic::Role, usize)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OsEventData {
    /// New EPR request requested by an app, identified by the five tuple
    EprRequestApp(EprFiveTuple),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppEventData {
    /// New EPR request needed by an app, identified by node ID and port.
    EprRequest(u32, u16),
    /// EPR request response from the OS.
    EprResponse(EprResponseData),
    /// Local operations complete for a given EPR request.
    LocalComplete(EprFiveTuple),
    /// Remote operations complete for a given EPR request.
    RemoteComplete(EprFiveTuple),
}

#[derive(Debug, PartialEq, Eq)]
pub enum EventType {
    /// The warm-up period expires.
    WarmupPeriodEnd,
    /// The simulation ends.
    ExperimentEnd,
    /// Print progress.
    Progress(u16),

    /// Node-related event.
    NodeEvent(NodeEventData),
    /// OS-related event.
    OsEvent(OsEventData),
    /// Application-related event.
    AppEvent(AppEventData),
}

/// A simulation event.
#[derive(PartialEq, Eq)]
pub struct Event {
    time: u64,
    pub event_type: EventType,
}

impl Event {
    pub fn new(time: f64, event_type: EventType) -> Self {
        Self {
            time: crate::utils::to_nanoseconds(time),
            event_type,
        }
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn advance(&mut self, advance_time: u64) {
        self.time += advance_time
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.time().partial_cmp(&self.time())
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub trait EventHandler {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>);
    fn initial(&mut self) -> Vec<Event>;
}
