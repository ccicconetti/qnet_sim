// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::*;
use crate::output::Sample;
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone)]
enum Status {
    Queued,
    WaitingForResponse(MemoryCellId),
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

#[derive(Debug, Clone)]
pub struct NodeProperties {
    /// Entanglement swapping success probability.
    pub swapping_success_prob: f64,
    /// Entanglement swapping duration, in s.
    pub swapping_duration: f64,
    /// Duration of the local operations to correct end-to-end pairs, in s.
    pub correction_duration: f64,
}

/// A quantum node.
pub struct Node {
    /// Node's identifier.
    node_id: u32,
    /// Node's properties.
    properties: NodeProperties,
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
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "node_id {}", self.node_id)?;
        for (peer, nic) in &self.nics_master {
            writeln!(f, "NIC peer {peer}: {nic}")?;
        }
        for (peer, nic) in &self.nics_slave {
            writeln!(f, "NIC peer {peer}: {nic}")?;
        }
        writeln!(f, "apps on ports {:?}", self.applications.keys())?;
        for (peer, requests) in &self.pending_requests {
            for request in requests {
                writeln!(f, "REQ peer {peer}: {request:?}")?;
            }
        }
        Ok(())
    }
}

impl Node {
    /// Create a node with no NICs.
    pub fn new(
        node_id: u32,
        properties: NodeProperties,
        logical_topology: std::rc::Rc<crate::logical_topology::LogicalTopology>,
        init_seed: u64,
    ) -> Self {
        Self {
            node_id,
            properties,
            nics_master: std::collections::HashMap::new(),
            nics_slave: std::collections::HashMap::new(),
            applications: std::collections::HashMap::new(),
            logical_topology,
            pending_requests: std::collections::HashMap::new(),
            rng: rand::rngs::StdRng::seed_from_u64(init_seed + node_id as u64),
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
    ) -> (Vec<Event>, Vec<Sample>) {
        let occupancy = {
            let nic = self.get_nic(peer_node_id, &role);
            nic.add_epr_pair(now, epr_pair_id);
            nic.occupancy()
        };

        // Schedule pending requests for this peer, if any.
        let (events, mut samples) = self.schedule_pending_requests(peer_node_id);

        samples.push(Sample::Series(
            "occupancy".to_string(),
            vec![self.node_id.to_string(), peer_node_id.to_string()],
            occupancy,
        ));

        (events, samples)
    }

    /// Consume the qubit of an EPR stored in a memory cell in one of the NICs.
    /// Return the creation time and identifier.
    pub fn consume(
        &mut self,
        peer_node_id: u32,
        role: &super::nic::Role,
        local_pair_id: u64,
    ) -> Option<crate::nic::MemoryCellData> {
        self.get_nic(peer_node_id, role).consume(local_pair_id)
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
        let this_node_id = self.node_id;
        self.nics(role).get_mut(&peer_node_id).unwrap_or_else(|| {
            panic!("node {this_node_id}: could not find NIC for peer {peer_node_id} ({role:?})")
        })
    }

    /// Handle local events.
    fn handle_node_event(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        if let EventType::NodeEvent(data) = event.event_type {
            match data {
                NodeEventData::EprRequestApp(epr) => self.handle_epr_request_app(now, now, epr),
                NodeEventData::EsRequest(data) => self.handle_es_request(now, data),
                NodeEventData::EsLocalComplete(data) => self.handle_es_local_complete(now, data),
                NodeEventData::EsSuccess(data) => self.handle_es_response(now, data, true),
                NodeEventData::EsFailure(data) => self.handle_es_response(now, data, false),
                NodeEventData::EsRemoteComplete(data) => self.handle_es_remote_complete(now, data),
                NodeEventData::EsRemoteFailed(data) => self.handle_es_remote_failed(now, data),
            }
        } else {
            panic!(
                "wrong event type received: expected NetworkEvent received {:?}",
                event.event_type
            )
        }
    }

    /// Handle EPR request from an application on this node.
    ///
    /// - `now`: the current simulated time
    /// - `received`: the time when the request was originally received
    /// - `epr`: the EPR to be established
    fn handle_epr_request_app(
        &mut self,
        now: u64,
        received: u64,
        epr: EprFiveTuple,
    ) -> (Vec<Event>, Vec<Sample>) {
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
                received,
                epr,
                status: Status::Queued,
                path,
            });

        self.schedule_pending_requests(peer)
    }

    /// Handle ES request from another node.
    fn handle_es_request(&mut self, now: u64, data: EsRequestData) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, data.next_hop);

        #[cfg(debug_assertions)]
        {
            let mut prev_hop_pos = data
                .path
                .iter()
                .position(|x| *x == self.node_id)
                .expect("this node not present in the path of an EsRequest")
                as u32;
            assert!(
                prev_hop_pos > 0,
                "the first node ({}) in the path ({:?}) cannot receive an EsRequest",
                self.node_id,
                data.path,
            );
            prev_hop_pos -= 1;
            assert_eq!(data.path[prev_hop_pos as usize], data.prev_hop);
        }

        if data.epr.target_node_id == self.node_id {
            // This is the final target node.
            let mut events = vec![];

            // Check if there is a valid and unused EPR pair in the memory cell
            // indicated in the request.
            let nic = self
                .nics_slave
                .get_mut(&data.prev_hop)
                .expect("received an EsRequest from an unknown peer");

            if nic.used(data.local_pair_id) {
                // We just locked the memory cell so that it cannot be modified.
                // We now schedule an event for when the local operations need
                // to be done.

                // If this is a single hop EPR request, then the EPR pair can
                // be used immediately. Otherwise, X/Z corrections might be
                // necessary dependin on the outcome of the BSM operations
                // along the path.
                let event_delay = if data.path.len() > 2 {
                    let rand = self.rng.gen_range(0..4);
                    if rand == 0 {
                        // no corrections
                        0.0
                    } else if rand == 1 {
                        // both X and Z corrections
                        self.properties.correction_duration * 2.0
                    } else {
                        // only X or Z correction
                        self.properties.correction_duration
                    }
                } else {
                    0.0
                };

                events.push(Event::new(
                    event_delay,
                    EventType::NodeEvent(NodeEventData::EsLocalComplete(data)),
                ));
            } else {
                nic.print_all_cells(); // XXX

                // The memory cell does not contain what the master expects.
                let dst_node_id = data.prev_hop;
                events.push(Event::new_transfer(
                    EventType::NodeEvent(NodeEventData::EsFailure(data)),
                    self.node_id,
                    dst_node_id,
                ));
            }

            (events, vec![])
        } else {
            // This is an intermediate node, which has to perform entanglement
            // swapping.
            todo!("{} {:?}: multi-hop not yet implemented", now, data)
        }
    }

    /// Handle completion of local operations for an ES.
    ///
    /// If the operation was a BSM, decide (randomly) if successful:
    /// - Success: send `EsSuccess` to prev_hop.
    /// - Failure: send `EsFailure` to prev_hop, free local EPR pair (slave).
    ///
    /// If the operation was a correction:
    /// - Send `EsRemoteComplete` to source node.
    /// - Notify `EprResponse` (is_source = false) to the local app.
    fn handle_es_local_complete(
        &mut self,
        now: u64,
        data: EsRequestData,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, data.next_hop);
        assert!(data.path.len() >= 2);

        let mut events = vec![];
        if self.node_id == *data.path.last().unwrap() {
            // This node is the last element in the path, which means that the
            // local operation was an X/Z correction, which never fails.
            let src_node_id = *data.path.first().unwrap();
            let epr = data.epr.clone();
            events.push(Event::new_transfer(
                EventType::NodeEvent(NodeEventData::EsRemoteComplete(data.epr)),
                self.node_id,
                src_node_id,
            ));
            let memory_cell = Some(MemoryCellId {
                neighbor_node_id: data.prev_hop,
                role: super::nic::Role::Slave,
                local_pair_id: data.local_pair_id,
            });
            events.push(Event::new(
                0.0_f64,
                EventType::AppEvent(AppEventData::EprResponse(EprResponseData {
                    epr,
                    is_source: false,
                    memory_cell,
                })),
            ));
        } else {
            // XXX
            todo!("{} {:?}: multi-hop not yet implemented", now, data)
        }

        (events, vec![])
    }

    /// Handle response received for an ES request.
    ///
    /// If failed:
    /// - Communicate failure to the source of the path.
    /// - Free local EPR pair (master).
    ///
    /// If success:
    /// - Free previous EPR pair (if any).
    ///
    /// In both cases remove the request from the pending queue
    fn handle_es_response(
        &mut self,
        now: u64,
        data: EsRequestData,
        success: bool,
    ) -> (Vec<Event>, Vec<Sample>) {
        // XXX
        (vec![], vec![])
    }

    /// Handle indication at the source node that a remote entanglement
    /// swapping procedure is complete (and successful).
    ///
    /// Search for a pending request with matching `EprFiveTuple` and, if found,
    /// notify `EprResponse` (is_source = true) to the application.
    fn handle_es_remote_complete(
        &mut self,
        now: u64,
        epr: EprFiveTuple,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, epr.source_node_id);

        for requests in &mut self.pending_requests.values_mut() {
            if let Some(epr_ndx) = requests.iter().position(|x| x.epr == epr) {
                let request = requests.swap_remove(epr_ndx);
                if let Status::WaitingForResponse(memory_cell) = request.status {
                    let events = vec![Event::new(
                        0.0_f64,
                        EventType::AppEvent(AppEventData::EprResponse(EprResponseData {
                            epr,
                            is_source: true,
                            memory_cell: Some(memory_cell),
                        })),
                    )];
                    return (
                        events,
                        vec![Sample::Series(
                            "epr-request-latency".to_string(),
                            vec![
                                self.node_id.to_string(),
                                (request.path.len() - 1).to_string(),
                            ],
                            crate::utils::to_seconds(now - request.received),
                        )],
                    );
                } else {
                    panic!(
                        "wrong queued request at node {} for EPR {}: {:?}",
                        self.node_id, epr, request
                    );
                }
            }
        }

        panic!(
            "could not find a queued request at node {} for EPR {}",
            self.node_id, epr
        )
    }

    /// Handle indication at the source node that a remote entanglement
    /// swapping procedure has failed.
    ///
    /// Search for a pending request with matching `EprFiveTuple` and, if found,
    /// free the local EPR pair and reschedule the end-to-end request.
    fn handle_es_remote_failed(
        &mut self,
        now: u64,
        epr: EprFiveTuple,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, epr.source_node_id);

        for requests in &mut self.pending_requests.values_mut() {
            if let Some(epr_ndx) = requests.iter().position(|x| x.epr == epr) {
                let request = requests.swap_remove(epr_ndx);
                return self.handle_epr_request_app(now, request.received, request.epr);
            }
        }

        (vec![], vec![])
    }

    /// Schedule requests pending for a given peer, if possible.
    fn schedule_pending_requests(&mut self, peer: u32) -> (Vec<Event>, Vec<Sample>) {
        let log_status = format!("{self}");
        let mut events = vec![];
        if let Some(nic) = self.nics_master.get_mut(&peer) {
            if let Some(requests) = &mut self.pending_requests.get_mut(&peer) {
                if !requests.is_empty() {
                    log::debug!("{log_status}");
                }
                for request in requests.iter_mut() {
                    if let Status::Queued = request.status {
                        if let Some(local_pair_id) = nic.newest_valid() {
                            nic.used(local_pair_id);
                            events.push(Event::new_transfer(
                                EventType::NodeEvent(NodeEventData::EsRequest(EsRequestData {
                                    epr: request.epr.clone(),
                                    prev_hop: self.node_id,
                                    next_hop: peer,
                                    path: request.path.clone(),
                                    local_pair_id,
                                })),
                                self.node_id,
                                peer,
                            ));
                            request.status = Status::WaitingForResponse(MemoryCellId {
                                neighbor_node_id: peer,
                                role: crate::nic::Role::Master,
                                local_pair_id,
                            });
                        } else {
                            break;
                        }
                    }
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
