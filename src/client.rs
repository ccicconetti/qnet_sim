// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;
use rand_distr::Distribution;

use crate::event::*;
use crate::output::Sample;

#[derive(Debug)]
struct EprRequest {
    /// Neighbor node ID, used to identify the NIC, and memory cell index.
    /// If None then the application is still waiting for the OS to indicate
    /// if the EPR was established or not.
    memory_cell: Option<(u32, crate::nic::Role, usize)>,
    /// True if the local operations have been done.
    local_operations_done: bool,
    /// True if the remote operations have been done.
    remote_operations_done: bool,
    /// Timestamp of when the request was created.
    created: u64,
}

/// Client application.
#[derive(Debug)]
pub struct Client {
    /// Source node ID.
    this_node_id: u32,
    /// Source port.
    this_port: u16,
    /// Target node ID.
    peer_node_id: u32,
    /// Target port.
    peer_port: u16,
    /// ID of the next request.
    next_request_id: u64,
    /// R.v. for determine the next EPR request.
    rv_next_epr: rand_distr::Exp<f64>,
    /// R.v. for determine the duration of local operations.
    rv_local_ops: rand_distr::Exp<f64>,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Pending requests.
    pending: std::collections::HashMap<u64, EprRequest>,
}

impl Client {
    /// Create a new client application.
    ///
    /// Parameters:
    /// - `this_node_id`: Source node ID.
    /// - `this_port`: Source port.
    /// - `peer_node_id`: Target node ID.
    /// - `peer_port`: Target port.
    /// - `seed`: Seed to initialize internal RNG.
    /// - `operation_rate`: Rate at which a new EPR is requested, in s^-1.
    /// - `operation_avg_dur`: Average duration of a local operatio, in s.
    fn new(
        this_node_id: u32,
        this_port: u16,
        peer_node_id: u32,
        peer_port: u16,
        seed: u64,
        operation_rate: f64,
        operation_avg_dur: f64,
    ) -> Self {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let rv_next_epr =
            rand_distr::Exp::new(operation_rate).expect("could not create an expo rv");
        let rv_local_ops =
            rand_distr::Exp::new(1.0 / operation_avg_dur).expect("could not create an expo rv");
        Self {
            this_node_id,
            this_port,
            peer_node_id,
            peer_port,
            next_request_id: 0,
            rv_next_epr,
            rv_local_ops,
            rng,
            pending: std::collections::HashMap::new(),
        }
    }

    fn get_request(&mut self, epr: &EprFiveTuple) -> &mut EprRequest {
        assert_eq!(epr.source_node_id, self.this_node_id);
        assert_eq!(epr.source_port, self.this_port);
        assert_eq!(epr.target_node_id, self.peer_node_id);
        assert_eq!(epr.target_port, self.peer_port);

        self.pending
            .get_mut(&epr.request_id)
            .unwrap_or_else(|| panic!("non-existing pending request {}", epr))
    }

    fn remove_request(&mut self, now: u64, request_id: u64) -> Vec<Sample> {
        let epr_request = self.pending.remove(&request_id);

        if let Some(epr_request) = epr_request {
            vec![Sample::Series(
                "latency-node,latency-port".to_string(),
                format!("{},{}", self.this_node_id, self.this_port),
                crate::utils::to_seconds(now - epr_request.created),
            )]
        } else {
            vec![]
        }
    }

    fn handle_epr_request(
        &mut self,
        now: u64,
        node_id: u32,
        port: u16,
    ) -> (Vec<Event>, Vec<Sample>) {
        let mut events = vec![];
        let mut samples = vec![];

        assert!(self.this_node_id == node_id);
        assert!(self.this_port == port);

        // Send the EPR request to the OS.
        events.push(Event::new(
            0.0,
            EventType::OsEvent(OsEventData::EprRequestApp(EprFiveTuple {
                source_node_id: self.this_node_id,
                source_port: self.this_port,
                target_node_id: self.peer_node_id,
                target_port: self.peer_port,
                request_id: self.next_request_id,
            })),
        ));

        self.pending.insert(
            self.next_request_id,
            EprRequest {
                memory_cell: None,
                local_operations_done: false,
                remote_operations_done: false,
                created: now,
            },
        );
        self.next_request_id += 1;

        samples.push(Sample::Series(
            "app_pending_len".to_string(),
            format!("{}:{}", self.this_node_id, self.this_port),
            self.pending.len() as f64,
        ));

        // Generate a new EPR request for the application.
        events.push(Event::new(
            self.rv_next_epr.sample(&mut self.rng),
            EventType::AppEvent(AppEventData::EprRequest(self.this_node_id, self.this_port)),
        ));

        (events, samples)
    }

    fn handle_epr_response(&mut self, data: EprResponseData) -> (Vec<Event>, Vec<Sample>) {
        let mut events = vec![];

        let request = self.get_request(&data.epr);

        assert!(
            request.memory_cell.is_none(),
            "duplicate response received for request {} at application {}:{}",
            data.epr.request_id,
            data.epr.source_port,
            data.epr.request_id,
        );

        if let Some(memory_cell) = data.memory_cell {
            // Request successful.
            request.memory_cell = Some(memory_cell);

            // Start timer for local operations.
            events.push(Event::new(
                self.rv_local_ops.sample(&mut self.rng),
                EventType::AppEvent(AppEventData::LocalComplete(data.epr)),
            ));
        } else {
            self.pending.remove(&data.epr.request_id);
        }

        (events, vec![])
    }

    fn handle_local_complete(&mut self, now: u64, epr: EprFiveTuple) -> (Vec<Event>, Vec<Sample>) {
        let mut events = vec![];

        let this_node_id = self.this_node_id.clone();
        let this_port = self.this_port.clone();

        let request = self.get_request(&epr);

        assert!(
            !request.local_operations_done,
            "duplicate execution of local operations for request {}",
            epr
        );
        request.local_operations_done = true;

        // Compute the fidelity on the local end of this EPR.
        let memory_cell = std::mem::take(&mut request.memory_cell);
        let (neighbor_node_id, role, index) = memory_cell
            .unwrap_or_else(|| panic!("local operation completed on a failed request {}", epr));
        events.push(Event::new(
            0.0,
            EventType::NodeEvent(NodeEventData::EprFidelity(EprFidelityData {
                app_node_id: this_node_id,
                port: this_port,
                consume_node_id: this_node_id,
                neighbor_node_id,
                role,
                index,
            })),
        ));

        if request.remote_operations_done {
            (events, self.remove_request(now, epr.request_id))
        } else {
            (events, vec![])
        }
    }

    fn handle_remote_complete(&mut self, now: u64, epr: EprFiveTuple) -> (Vec<Event>, Vec<Sample>) {
        let request = self.get_request(&epr);

        assert!(
            !request.remote_operations_done,
            "duplicate execution of remote operations for request {}",
            epr
        );
        request.remote_operations_done = true;

        if request.local_operations_done {
            (vec![], self.remove_request(now, epr.request_id))
        } else {
            (vec![], vec![])
        }
    }
}

impl EventHandler for Client {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        match event.event_type {
            EventType::AppEvent(data) => match data {
                AppEventData::EprRequest(node_id, port) => {
                    self.handle_epr_request(now, node_id, port)
                }
                AppEventData::EprResponse(data) => self.handle_epr_response(data),
                AppEventData::LocalComplete(epr) => self.handle_local_complete(now, epr),
                AppEventData::RemoteComplete(epr) => self.handle_remote_complete(now, epr),
            },
            _ => panic!(
                "invalid event {:?} received by a Application object",
                event.event_type
            ),
        }
    }

    fn initial(&mut self) -> Vec<Event> {
        vec![Event::new(
            self.rv_next_epr.sample(&mut self.rng),
            EventType::AppEvent(AppEventData::EprRequest(self.this_node_id, self.this_port)),
        )]
    }
}

#[cfg(test)]
mod tests {

    use crate::event::AppEventData;
    use crate::event::EprFiveTuple;
    use crate::event::EprResponseData;
    use crate::event::Event;
    use crate::event::EventHandler;
    use crate::event::EventType;
    use crate::event::NodeEventData;
    use crate::event::OsEventData;
    use crate::nic;

    use super::Client;

    fn is_os_epr_request(event: &EventType) -> bool {
        if let EventType::OsEvent(data) = event {
            matches!(data, OsEventData::EprRequestApp(_))
        } else {
            false
        }
    }

    fn is_app_epr_request(event: &EventType) -> bool {
        if let EventType::AppEvent(data) = event {
            matches!(data, AppEventData::EprRequest(_, _))
        } else {
            false
        }
    }

    fn is_local_complete(event: &EventType, expected_five_tuple: &EprFiveTuple) -> bool {
        if let EventType::AppEvent(data) = event {
            if let AppEventData::LocalComplete(actual_five_tuple) = data {
                expected_five_tuple == actual_five_tuple
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_node_epr_fidelity(event: &EventType) -> bool {
        if let EventType::NodeEvent(data) = event {
            matches!(data, NodeEventData::EprFidelity(_))
        } else {
            false
        }
    }

    #[test]
    fn test_client() {
        let this_node_id = 0;
        let this_port = 50000;
        let peer_node_id = 1;
        let peer_port = 8080;
        let mut client = Client::new(
            this_node_id,
            this_port,
            peer_node_id,
            peer_port,
            42,
            1.0,
            0.1,
        );

        let events = client.initial();
        assert_eq!(1, events.len());
        assert!(is_app_epr_request(&events[0].event_type));

        let events = client
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::EprRequest(this_node_id, this_port)),
            ))
            .0;
        assert_eq!(2, events.len());
        assert!(is_os_epr_request(&events[0].event_type));
        let five_tuple = if let EventType::OsEvent(data) = &events[0].event_type {
            #[allow(irrefutable_let_patterns)]
            if let OsEventData::EprRequestApp(five_tuple) = data {
                five_tuple
            } else {
                panic!("wrong event sub-type");
            }
        } else {
            panic!("wrong event type")
        };

        assert!(is_app_epr_request(&events[1].event_type));

        let events = client
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::EprResponse(EprResponseData {
                    epr: five_tuple.clone(),
                    memory_cell: Some((2, nic::Role::Master, 0)),
                })),
            ))
            .0;
        assert_eq!(1, events.len());
        assert!(is_local_complete(&events[0].event_type, five_tuple));

        let events = client
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::LocalComplete(five_tuple.clone())),
            ))
            .0;
        assert_eq!(1, events.len());
        assert!(is_node_epr_fidelity(&events[0].event_type));

        let events = client
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::RemoteComplete(five_tuple.clone())),
            ))
            .0;
        assert!(events.is_empty());
    }
}
