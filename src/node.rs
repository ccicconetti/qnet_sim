// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::*;
use crate::output::Sample;

#[derive(Debug, Clone)]
enum Status {
    Queued,
    WaitingForResponse,
}

#[derive(Debug, Clone)]
struct Request {
    /// Time when the request was received.
    received: u64,
    /// EPR five tuple.
    epr: EprFiveTuple,
    /// Status
    status: Status,
    /// Path
    path: Vec<u32>,
}

/// A quantum node.
pub struct Node {
    /// Node's identifier.
    node_id: u32,
    /// Quantum NICs towards logical peers for which this node is master.
    nics_master: std::collections::HashMap<u32, super::nic::Nic>,
    /// Quantum NICs towards logical peers for which this node is slave.
    nics_slave: std::collections::HashMap<u32, super::nic::Nic>,
    /// The applications, identified by their port.
    applications: std::collections::HashMap<u16, Box<dyn crate::event::EventHandler>>,
    /// The logical topology.
    logical_topology: std::rc::Rc<crate::logical_topology::LogicalTopology>,
    /// Pending requests grouped by peer.
    pending_requests: std::collections::HashMap<u32, Vec<Request>>,
}

impl Node {
    /// Create a node with no NICs.
    pub fn new(
        node_id: u32,
        logical_topology: std::rc::Rc<crate::logical_topology::LogicalTopology>,
    ) -> Self {
        Self {
            node_id,
            nics_master: std::collections::HashMap::new(),
            nics_slave: std::collections::HashMap::new(),
            applications: std::collections::HashMap::new(),
            logical_topology,
            pending_requests: std::collections::HashMap::new(),
        }
    }

    /// Retrieve an application running on this node.
    pub fn application(
        &mut self,
        port: u16,
    ) -> anyhow::Result<&mut Box<dyn crate::event::EventHandler>> {
        self.applications.get_mut(&port).ok_or(anyhow::anyhow!(
            "no application at port {} on node {}",
            port,
            self.node_id
        ))
    }

    /// Add an application to this node.
    pub fn add_applicaton(&mut self, application: Box<dyn crate::event::EventHandler>, port: u16) {
        if self.applications.insert(port, application).is_some() {
            panic!(
                "new application with same port {} added at node {}",
                port, self.node_id
            );
        }
    }

    /// Return the next port number available.
    pub fn next_port(&self) -> u16 {
        let mut port = 0;
        loop {
            if !self.applications.contains_key(&port) {
                return port;
            }
            port += 1;
        }
    }

    /// Add a NIC towards a given peer.
    ///
    /// Parameters:
    /// - `peer_node_id`: the identifier of the peer node
    /// - `role`: the role of this node in the logical link
    /// - `num_qubits`: how many quantum memory cells there will be
    ///
    /// Return true if `peer_node_id` was already present with same role for
    /// this node.
    pub fn add_nic(&mut self, peer_node_id: u32, role: super::nic::Role, num_qubits: u32) -> bool {
        self.nics(&role)
            .insert(peer_node_id, super::nic::Nic::new(role, num_qubits))
            .is_none()
    }

    /// Notify that a new EPR has been established. Return the occupancy ratio.
    pub fn epr_established(
        &mut self,
        now: u64,
        peer_node_id: u32,
        role: super::nic::Role,
        epr_pair_id: u64,
    ) -> f64 {
        let nic = self.get_nic(peer_node_id, &role);
        nic.add_epr_pair(now, epr_pair_id);

        // Schedule pending requests for this peer, if any. XXX
        // self.schedule_pending_requests(peer_node_id);

        nic.occupancy()
    }

    /// Consume the qubit of an EPR stored in a memory cell in one of the NICs.
    /// Return the creation time and identifier.
    pub fn consume(
        &mut self,
        peer_node_id: u32,
        role: &super::nic::Role,
        index: usize,
    ) -> Option<crate::nic::MemoryCellData> {
        self.get_nic(peer_node_id, role).consume(index)
    }

    /// Return the right set of NICs depending on the role.
    fn nics(
        &mut self,
        role: &super::nic::Role,
    ) -> &mut std::collections::HashMap<u32, super::nic::Nic> {
        match role {
            super::nic::Role::Master => &mut self.nics_master,
            super::nic::Role::Slave => &mut self.nics_slave,
        }
    }

    /// Return the NIC for a given peer node and role.
    fn get_nic(&mut self, peer_node_id: u32, role: &super::nic::Role) -> &mut super::nic::Nic {
        self.nics(role)
            .get_mut(&peer_node_id)
            .unwrap_or_else(|| panic!("could not find NIC for peer {} ({:?})", peer_node_id, role))
    }

    /// Handle local events.
    fn handle_node_event(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        if let EventType::NodeEvent(data) = event.event_type {
            match data {
                NodeEventData::EprRequestApp(epr) => self.handle_epr_request_app(now, epr),
                NodeEventData::EsRequest(data) => self.handle_es_request(now, data),
            }
        } else {
            panic!(
                "wrong event type received: expected NetworkEvent received {:?}",
                event.event_type
            )
        }
    }

    /// Handle EPR request from an application on this node.
    fn handle_epr_request_app(&mut self, now: u64, epr: EprFiveTuple) -> (Vec<Event>, Vec<Sample>) {
        assert_ne!(
            epr.source_node_id, epr.target_node_id,
            "src and dst nodes must be different"
        );

        // Find the path to go from src to dst in the logical topology.
        assert_eq!(self.node_id, epr.source_node_id);
        let path = self
            .logical_topology
            .path(epr.source_node_id, epr.target_node_id);
        assert!(path.len() >= 2);
        assert_eq!(epr.source_node_id, *path.first().unwrap());
        assert_eq!(epr.target_node_id, *path.last().unwrap());

        if path.len() > 2 {
            todo!(
                "{} {} path {:?}: multi-hop not yet implemented",
                now,
                epr,
                path
            );
        }

        let peer = *path.last().unwrap();
        self.pending_requests
            .entry(peer)
            .or_default()
            .push(Request {
                received: now,
                epr: epr,
                status: Status::Queued,
                path,
            });

        self.schedule_pending_requests(peer)
    }

    /// Handle ES request from another node.
    fn handle_es_request(&mut self, now: u64, data: EsRequestData) -> (Vec<Event>, Vec<Sample>) {
        todo!("{} {:?}: multi-hop not yet implemented", now, data)
        // (vec![], vec![])
    }

    /// Schedule requests pending for a given peer, if possible.
    fn schedule_pending_requests(&mut self, peer: u32) -> (Vec<Event>, Vec<Sample>) {
        let nic = self
            .nics_master
            .get_mut(&peer)
            .expect("no NIC found for peer");

        let mut events = vec![];
        if let Some(requests) = self.pending_requests.get(&peer) {
            for request in requests {
                match request.status {
                    Status::Queued => {
                        if let Some(index) = nic.newest_valid() {
                            let local_pair_id = nic
                                .used(index)
                                .expect("cannot use a memory cell that is assumed to be valid")
                                .identifier;
                            events.push(Event::new(
                                0.0,
                                EventType::NodeEvent(NodeEventData::EsRequest(EsRequestData {
                                    epr: request.epr.clone(),
                                    next_hop: peer,
                                    path: request.path.clone(),
                                    memory_cell: index,
                                    local_pair_id,
                                })),
                            ));
                        } else {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        (events, vec![])
    }
}

impl EventHandler for Node {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        if let Some(transfer) = &event.transfer {
            assert!(
                transfer.done,
                "node {} received an event for which the transfer has not been simulated",
                self.node_id,
            );
        }
        match &event.event_type {
            EventType::AppEvent(data) => {
                // Dispatch event to the correct application.
                let application = self
                    .application(data.port())
                    .expect("unknown target application for an event");
                application.handle(event)
            }
            EventType::NodeEvent(_data) => self.handle_node_event(event),
            _ => panic!(
                "invalid event {:?} received by a Node object",
                event.event_type
            ),
        }
    }

    /// Kick start all the applications.
    fn initial(&mut self) -> Vec<Event> {
        let mut events = vec![];

        for application in self.applications.values_mut() {
            events.append(&mut application.initial());
        }

        events
    }
}
