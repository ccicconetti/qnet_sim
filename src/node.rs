// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::*;
use crate::output::Sample;
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone)]
struct EsRequestStatusWaitingData {
    pred_memory_cell: Option<MemoryCellId>,
    succ_memory_cell: MemoryCellId,
}

#[derive(Debug, Clone)]
enum EsRequestStatus {
    QueuedSource,
    QueuedIntermediate(MemoryCellId), // memory cell of the predecessor
    Waiting(EsRequestStatusWaitingData),
}

#[derive(Debug, Clone)]
struct EsRequest {
    /// EPR five tuple.
    epr: EprFiveTuple,
    /// Status
    status: EsRequestStatus,
    /// Path
    path: Vec<u32>,
}

#[derive(Debug, Clone)]
struct AppRequest {
    /// Time when the request was received.
    received: u64,
    /// EPR five tuple.
    epr: EprFiveTuple,
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
    /// Pending ES requests grouped by peer.
    pending_es_requests: std::collections::HashMap<u32, Vec<EsRequest>>,
    /// Pending application requests.
    pending_app_requests: Vec<AppRequest>,
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
        for (peer, requests) in &self.pending_es_requests {
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
            pending_es_requests: std::collections::HashMap::new(),
            pending_app_requests: vec![],
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
        let mut events = vec![];
        let this_node_id = self.node_id;
        let occupancy = {
            let nic = self.get_nic(peer_node_id, &role);
            let (epr_pair_id, _added) = nic.add_epr_pair(now, epr_pair_id);
            if let Some(epr_pair_id) = epr_pair_id {
                events.push(Event::new(
                    0.0,
                    EventType::NetworkEvent(NetworkEventData::EprFree(EprFreeData {
                        epr_pair_id,
                        node_id: this_node_id,
                    })),
                ));
            }
            nic.occupancy()
        };

        // Schedule pending requests for this peer, if any.
        let (mut new_events, mut samples) = self.schedule_pending_es_requests(peer_node_id);
        events.append(&mut new_events);

        samples.push(Sample::Series(
            "occupancy".to_string(),
            vec![self.node_id.to_string(), peer_node_id.to_string()],
            occupancy,
        ));

        (events, samples)
    }

    /// Consume the qubit of an EPR stored in a memory cell in one of the NICs
    /// and return it, if found.
    pub fn consume(
        &mut self,
        peer_node_id: u32,
        role: &super::nic::Role,
        epr_pair_id: u64,
    ) -> Option<crate::nic::MemoryCellData> {
        self.get_nic(peer_node_id, role).consume(epr_pair_id)
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
                NodeEventData::EsFreeMemoryCell(data) => self.handle_es_free_memory_cell(now, data),
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
        _now: u64,
        received: u64,
        epr: EprFiveTuple,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_ne!(
            epr.source_node_id, epr.target_node_id,
            "src and dst nodes must be different"
        );

        self.pending_app_requests.push(AppRequest {
            received,
            epr: epr.clone(),
        });

        log::debug!(
            "node {} pending app requests:\n{}",
            self.node_id,
            self.pending_app_requests
                .iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<String>>()
                .join("\n")
        );

        // Find the path to go from src to dst in the logical topology.
        assert_eq!(self.node_id, epr.source_node_id);
        let path = self
            .logical_topology
            .path(epr.source_node_id, epr.target_node_id);
        assert!(path.len() >= 2);
        assert_eq!(epr.source_node_id, *path.first().unwrap());
        assert_eq!(epr.target_node_id, *path.last().unwrap());

        let successor = self.successor(&path);
        self.pending_es_requests
            .entry(successor)
            .or_default()
            .push(EsRequest {
                epr,
                status: EsRequestStatus::QueuedSource,
                path,
            });

        self.schedule_pending_es_requests(successor)
    }

    /// Handle ES request from another node.
    ///
    /// Try to lock the EPR pair in the specified slave position.
    ///
    /// If the operation succeeds, then:
    ///
    /// - If this is the target node, we schedule a EsLocalComplete to perform
    ///   X/Z corrections.
    /// - Else, if this is an intermediate node, we schedule a request to lock
    ///   an EPR pair (master) that will be needed for the BSM.
    ///
    /// If the memory cell does not contain what the master expects, then
    /// send an EsFailure to the predecessor to free the EPR pair and a
    /// EsRemoteFailed to the source node, so that it can try again.
    fn handle_es_request(&mut self, _now: u64, data: EsRequestData) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, data.target);

        let predecessor = self.predecessor(&data.path);
        let source = *data.path.first().unwrap();

        let mut events = vec![];
        let mut samples = vec![];

        // Check if there is a valid and unused EPR pair in the memory cell
        // indicated in the request.
        let nic = self
            .nics_slave
            .get_mut(&predecessor)
            .expect("received an EsRequest from an unknown peer");

        if nic.used(data.epr_pair_id) {
            // We just locked the memory cell so that it cannot be modified.
            // We now schedule an event for when the local operations (Bell-state
            // measurement or X/Z corrections) need to be done.

            if data.epr.target_node_id == self.node_id {
                // This is the final target node.
                //
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
                // This is an intermediate node, which has to perform ES.
                let successor = self.successor(&data.path);
                let pred_memory_cell = MemoryCellId {
                    neighbor_node_id: predecessor,
                    role: crate::nic::Role::Slave,
                    epr_pair_id: data.epr_pair_id,
                };
                self.pending_es_requests
                    .entry(successor)
                    .or_default()
                    .push(EsRequest {
                        epr: data.epr,
                        status: EsRequestStatus::QueuedIntermediate(pred_memory_cell),
                        path: data.path,
                    });

                let (mut new_events, mut new_samples) =
                    self.schedule_pending_es_requests(successor);
                events.append(&mut new_events);
                samples.append(&mut new_samples);
            }
        } else {
            // The memory cell does not contain what the master expects.
            log::debug!(
                "node {} memory cells:\n{}",
                self.node_id,
                nic.dump().join("\n")
            );

            // Notify the source node that the end-to-end entanglement failed.
            let epr = data.epr.clone();
            events.push(Event::new_transfer(
                EventType::NodeEvent(NodeEventData::EsRemoteFailed(EsRemoteFailedData {
                    epr,
                    sender: self.node_id,
                })),
                self.node_id,
                source,
            ));

            samples.push(Sample::ScalarCount("local_epr_misses".to_string()))
        }

        (events, samples)
    }

    /// Handle completion of local operations for an ES.
    ///
    /// If the operation was a correction:
    /// - Send `EsRemoteComplete` to source node.
    /// - Notify `EprResponse` (is_source = false) to the local app.
    ///
    /// If the operation was a BSM, decide (randomly) if successful:
    /// - Success: send `EsSuccess` to the previous hop and send a new
    ///   `EsRequest` to the next hop.
    /// - Failure: send `EsFailure` to the previous hop and free the local EPR
    ///   pair (slave).
    fn handle_es_local_complete(
        &mut self,
        _now: u64,
        data: EsRequestData,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(self.node_id, data.target);
        assert!(data.path.len() >= 2);

        let mut events = vec![];
        let mut samples = vec![];
        let pos = data.path.iter().position(|x| *x == self.node_id).unwrap();
        assert!(
            pos >= 1,
            "EsLocalComplete received at node {} for EPR {}, with path {:?}",
            self.node_id,
            data.epr,
            data.path
        );
        let source = *data.path.first().unwrap();
        let predecessor = data.path[pos - 1];
        if self.node_id == *data.path.last().unwrap() {
            assert!(pos == data.path.len() - 1);
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
                neighbor_node_id: predecessor,
                role: super::nic::Role::Slave,
                epr_pair_id: data.epr_pair_id,
            });
            events.push(Event::immediate(EventType::AppEvent(
                AppEventData::EprResponse(EprResponseData {
                    epr,
                    is_source: false,
                    memory_cell,
                }),
            )));
        } else {
            // This is an intermediate node.
            assert!(pos < data.path.len() - 1);
            let successor = data.path[pos + 1];

            let succ_memory_cell = MemoryCellId {
                neighbor_node_id: successor,
                role: crate::nic::Role::Master,
                epr_pair_id: data.epr_pair_id,
            };

            let pred_memory_cell = self.pop_es_request_by_memory_cell(&succ_memory_cell).unwrap_or_else(|| {
                    panic!(
                        "found at intermediate node {} a pending request without predecessor memory cell for request {:?}",
                        self.node_id, data
                    );
                }
                );
            assert_eq!(pred_memory_cell.neighbor_node_id, predecessor);
            assert_eq!(pred_memory_cell.role, crate::nic::Role::Slave);

            if self.rng.gen_bool(self.properties.swapping_success_prob) {
                // Successful Bell-state measurement.
                samples.push(Sample::ScalarAvg("bsm_prob".to_string(), 1.0));

                // Send an EsRequest message to the successor.
                events.push(Event::new_transfer(
                    EventType::NodeEvent(NodeEventData::EsRequest(EsRequestData {
                        epr: data.epr,
                        target: successor,
                        path: data.path.clone(),
                        epr_pair_id: succ_memory_cell.epr_pair_id,
                    })),
                    self.node_id,
                    successor,
                ));
            } else {
                // Failed Bell-state measurement.
                samples.push(Sample::ScalarAvg("bsm_prob".to_string(), 0.0));

                // Free the memory cell of the successor.
                assert!(matches!(succ_memory_cell.role, crate::nic::Role::Master));
                let succ_memory_cell_reverse = MemoryCellId {
                    neighbor_node_id: self.node_id,
                    role: crate::nic::Role::Slave,
                    epr_pair_id: succ_memory_cell.epr_pair_id,
                };
                events.push(Event::new_transfer(
                    EventType::NodeEvent(NodeEventData::EsFreeMemoryCell(EsFreeMemoryCellData {
                        memory_cell: succ_memory_cell_reverse,
                        target: successor,
                    })),
                    self.node_id,
                    successor,
                ));

                // Notify the source that the remote entanglement has failed.
                events.push(Event::new_transfer(
                    EventType::NodeEvent(NodeEventData::EsRemoteFailed(EsRemoteFailedData {
                        epr: data.epr,
                        sender: self.node_id,
                    })),
                    self.node_id,
                    source,
                ));
            }

            // Irrespective of the success/failure of the BSM, we free the
            // predecessor and successor local EPR pairs.
            if !self.local_free_by_memory_cell(&pred_memory_cell) {
                panic!(
                    "failed at node {} to free predecessor memory cell {:?}",
                    self.node_id, pred_memory_cell
                );
            }
            if !self.local_free_by_memory_cell(&succ_memory_cell) {
                panic!(
                    "failed at node {} to free successor memory cell {:?}",
                    self.node_id, succ_memory_cell
                );
            }

            // Notify the network to perform ES.
            events.push(Event::immediate(EventType::NetworkEvent(
                NetworkEventData::EprEntanglementSwapping(EprEntangleSwappingData {
                    epr_pair_id_pred: pred_memory_cell.epr_pair_id,
                    epr_pair_id_succ: succ_memory_cell.epr_pair_id,
                    bsm_node_id: self.node_id,
                }),
            )));
        }

        (events, samples)
    }

    /// Free the local memory cell corresponding to the given EPR.
    fn handle_es_free_memory_cell(
        &mut self,
        _now: u64,
        data: EsFreeMemoryCellData,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert_eq!(data.target, self.node_id);
        assert_eq!(crate::nic::Role::Slave, data.memory_cell.role);

        if !self.local_free_by_memory_cell(&data.memory_cell) {
            panic!(
                "failed at node {} to free memory cell {:?}:\n{}",
                self.node_id, data.memory_cell, self
            );
        }

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

        let app_request = self.pop_app_request(&epr);
        let (_peer, es_request) = self.pop_es_request_by_epr(&epr);

        if let EsRequestStatus::Waiting(waiting_data) = es_request.status {
            assert!(
                waiting_data.pred_memory_cell.is_none(),
                "incorrect pending request at node {}: {:?}",
                self.node_id,
                waiting_data
            );
            let events = vec![Event::immediate(EventType::AppEvent(
                AppEventData::EprResponse(EprResponseData {
                    epr,
                    is_source: true,
                    memory_cell: Some(waiting_data.succ_memory_cell),
                }),
            ))];
            (
                events,
                vec![Sample::Series(
                    "epr-request-latency".to_string(),
                    vec![
                        self.node_id.to_string(),
                        (es_request.path.len() - 1).to_string(),
                    ],
                    crate::utils::to_seconds(now - app_request.received),
                )],
            )
        } else {
            panic!("invalid status of ES request at node {} for EPR {}: expected WaitingForResponse, found {:?}", self.node_id, epr, es_request.status)
        }
    }

    /// Handle indication at the source node that a remote entanglement
    /// swapping procedure has failed.
    ///
    /// Search for a pending request with matching `EprFiveTuple` and, if found,
    /// free the local EPR pair and reschedule the end-to-end request.
    fn handle_es_remote_failed(
        &mut self,
        now: u64,
        data: EsRemoteFailedData,
    ) -> (Vec<Event>, Vec<Sample>) {
        let epr = data.epr;
        assert_eq!(self.node_id, epr.source_node_id);

        // Free the local memory cell (master).
        let memory_cell = self.local_free_by_epr(&epr).unwrap_or_else(|| {
            panic!(
                "could not find at node {} memory cell for EPR {}",
                self.node_id, epr
            );
        });
        assert!(matches!(memory_cell.role, crate::nic::Role::Master));

        // Send request to free the peer memory cell (slave), unless the
        // EsRemoteFailed message came from that node.
        let mut events = vec![];
        if data.sender != memory_cell.neighbor_node_id {
            let memory_cell_reverse = MemoryCellId {
                neighbor_node_id: self.node_id,
                role: crate::nic::Role::Slave,
                epr_pair_id: memory_cell.epr_pair_id,
            };
            let target = memory_cell.neighbor_node_id;
            events.push(Event::new_transfer(
                EventType::NodeEvent(NodeEventData::EsFreeMemoryCell(EsFreeMemoryCellData {
                    memory_cell: memory_cell_reverse,
                    target,
                })),
                self.node_id,
                target,
            ));
        }

        // Reschedule the failed app request.
        let app_request = self.pop_app_request(&epr);
        let (mut new_events, _new_samples) =
            self.handle_epr_request_app(now, app_request.received, app_request.epr);
        events.append(&mut new_events);

        (events, vec![])
    }

    /// Return the predecessor of this node in a path.
    fn predecessor(&mut self, path: &Vec<u32>) -> u32 {
        let pos = path
            .iter()
            .position(|x| *x == self.node_id)
            .unwrap_or_else(|| panic!("cannot find node {} in path {:?}", self.node_id, path));
        assert!(
            pos >= 1,
            "cannot find predecessor of node {} in path {:?}",
            self.node_id,
            path
        );
        path[pos - 1]
    }

    /// Return the successor of this node in a path.
    fn successor(&mut self, path: &Vec<u32>) -> u32 {
        let pos = path
            .iter()
            .position(|x| *x == self.node_id)
            .unwrap_or_else(|| panic!("cannot find node {} in path {:?}", self.node_id, path));
        assert!(
            pos < path.len() - 1,
            "cannot find successor of node {} in path {:?}",
            self.node_id,
            path
        );
        path[pos + 1]
    }

    /// Free a local memory cell. Return true if consumed.
    fn local_free_by_memory_cell(&mut self, memory_cell: &MemoryCellId) -> bool {
        if let Some(nic) = self
            .nics(&memory_cell.role)
            .get_mut(&memory_cell.neighbor_node_id)
        {
            nic.consume(memory_cell.epr_pair_id).is_some()
        } else {
            false
        }
    }

    /// Free a local memory cell by EPR and remove any pending associated
    /// request. Return memory cell if consumed.
    fn local_free_by_epr(&mut self, epr: &EprFiveTuple) -> Option<MemoryCellId> {
        for (peer, es_requests) in &mut self.pending_es_requests {
            if let Some(pos) = es_requests.iter().position(|x| x.epr == *epr) {
                let es_request = es_requests.remove(pos);
                if let EsRequestStatus::Waiting(waiting_data) = &es_request.status {
                    assert!(
                        waiting_data.pred_memory_cell.is_none(),
                        "incorrect pending request at node {}: {:?}",
                        self.node_id,
                        waiting_data
                    );
                    let memory_cell = &waiting_data.succ_memory_cell;
                    assert_eq!(*peer, memory_cell.neighbor_node_id);
                    if let Some(nic) = self.nics_master.get_mut(&memory_cell.neighbor_node_id) {
                        if nic.consume(memory_cell.epr_pair_id).is_some() {
                            return Some(memory_cell.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Remove an app request from the queue.
    fn pop_app_request(&mut self, epr: &EprFiveTuple) -> AppRequest {
        if let Some(app_ndx) = self.pending_app_requests.iter().position(|x| x.epr == *epr) {
            self.pending_app_requests.remove(app_ndx)
        } else {
            panic!(
                "could not find queued app request at node {} for EPR {}",
                self.node_id, epr
            )
        }
    }

    /// Remove an ES request from the queue with matching EPR.
    fn pop_es_request_by_epr(&mut self, epr: &EprFiveTuple) -> (u32, EsRequest) {
        for (peer, es_requests) in &mut self.pending_es_requests {
            if let Some(epr_ndx) = es_requests.iter().position(|x| x.epr == *epr) {
                return (*peer, es_requests.swap_remove(epr_ndx));
            }
        }
        panic!(
            "could not find queued ES request at node {} for EPR {}",
            self.node_id, epr
        );
    }

    /// Remove an ES request in status WaitingForResponse from the queue
    /// with matching successor memory cell.
    ///
    /// Return the predecessor memory cell if found.
    fn pop_es_request_by_memory_cell(
        &mut self,
        memory_cell: &MemoryCellId,
    ) -> Option<MemoryCellId> {
        if let Some(es_requests) = &mut self
            .pending_es_requests
            .get_mut(&memory_cell.neighbor_node_id)
        {
            if let Some(pos) = es_requests.iter().position(|x| {
                if let EsRequestStatus::Waiting(waiting_data) = &x.status {
                    waiting_data.succ_memory_cell == *memory_cell
                } else {
                    false
                }
            }) {
                if let EsRequestStatus::Waiting(waiting_data) = es_requests.remove(pos).status {
                    return waiting_data.pred_memory_cell;
                }
            }
        }

        None
    }

    /// Schedule requests pending for a given peer, if possible.
    ///
    /// For all requests in a queued status perform the following procedure.
    ///  
    /// Mark the memory cell as used, so that it cannot be overwritten.
    ///
    /// Generate a new event depending on the request:
    /// - If we are the source node, then an EsRequest is sent to
    ///   the next hop.
    /// - Otherwise, we schedule a local EsLocalComplete event that simulates
    ///   the execution of the BSM operation.
    ///
    /// Finally, the request status is changed to WaitingForResponse.
    fn schedule_pending_es_requests(&mut self, peer: u32) -> (Vec<Event>, Vec<Sample>) {
        let log_status = format!("{self}");
        let mut events = vec![];
        if let Some(nic) = self.nics_master.get_mut(&peer) {
            if let Some(requests) = &mut self.pending_es_requests.get_mut(&peer) {
                if !requests.is_empty() {
                    log::debug!("{log_status}");
                }
                for request in requests.iter_mut() {
                    if matches!(request.status, EsRequestStatus::Waiting(_)) {
                        continue;
                    }

                    if let Some(epr_pair_id) = nic.newest_valid() {
                        nic.used(epr_pair_id);

                        let pred_memory_cell = match &request.status {
                            EsRequestStatus::QueuedSource => {
                                events.push(Event::new_transfer(
                                    EventType::NodeEvent(NodeEventData::EsRequest(EsRequestData {
                                        epr: request.epr.clone(),
                                        target: peer,
                                        path: request.path.clone(),
                                        epr_pair_id,
                                    })),
                                    self.node_id,
                                    peer,
                                ));
                                None
                            }
                            EsRequestStatus::QueuedIntermediate(pred_epr_pair_id) => {
                                events.push(Event::new(
                                    self.properties.swapping_duration,
                                    EventType::NodeEvent(NodeEventData::EsLocalComplete(
                                        EsRequestData {
                                            epr: request.epr.clone(),
                                            target: self.node_id,
                                            path: request.path.clone(),
                                            epr_pair_id,
                                        },
                                    )),
                                ));
                                Some(pred_epr_pair_id.clone())
                            }
                            EsRequestStatus::Waiting(_) => {
                                panic!("unreachable statement");
                            }
                        };

                        let waiting_data = EsRequestStatusWaitingData {
                            pred_memory_cell,
                            succ_memory_cell: MemoryCellId {
                                neighbor_node_id: peer,
                                role: crate::nic::Role::Master,
                                epr_pair_id,
                            },
                        };
                        request.status = EsRequestStatus::Waiting(waiting_data);
                    } else {
                        break;
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
