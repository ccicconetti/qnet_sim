// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand::SeedableRng;
use rand_distr::Distribution;

use crate::event::*;
use crate::output::Sample;

#[derive(Debug)]
struct EprResponse {
    /// Neighbor node ID, used to identify the NIC, and memory cell index.
    /// If None then the application is still waiting for the OS to indicate
    /// if the EPR was established or not.
    memory_cell: (u32, crate::nic::Role, usize),
    /// Client node ID.
    client_node_id: u32,
    /// Client port number.
    client_port: u16,
}

/// Serverapplication.
#[derive(Debug)]
pub struct Server {
    /// Source node ID.
    this_node_id: u32,
    /// Source port.
    this_port: u16,
    /// R.v. for determine the duration of local operations.
    rv_local_ops: rand_distr::Exp<f64>,
    /// Pseudo-random number generator.
    rng: rand::rngs::StdRng,
    /// Pending requests.
    pending: std::collections::HashMap<u64, EprResponse>,
}

impl Server {
    /// Create a new server application.
    ///
    /// Parameters:
    /// - `this_node_id`: Source node ID.
    /// - `this_port`: Source port.
    /// - `seed`: Seed to initialize internal RNG.
    /// - `operation_avg_dur`: Average duration of a local operation, in s.
    fn new(this_node_id: u32, this_port: u16, seed: u64, operation_avg_dur: f64) -> Self {
        let rng = rand::rngs::StdRng::seed_from_u64(seed);
        let rv_local_ops =
            rand_distr::Exp::new(1.0 / operation_avg_dur).expect("could not create an expo rv");
        Self {
            this_node_id,
            this_port,
            rv_local_ops,
            rng,
            pending: std::collections::HashMap::new(),
        }
    }

    fn pending_len_trace(&self) -> Sample {
        Sample::Series(
            "server_pending_len".to_string(),
            format!("{}:{}", self.this_node_id, self.this_port),
            self.pending.len() as f64,
        )
    }

    fn handle_epr_response(&mut self, data: EprResponseData) -> (Vec<Event>, Vec<Sample>) {
        assert!(
            data.epr.target_node_id == self.this_node_id,
            "received EPR response not addressed to this node"
        );
        assert!(
            data.epr.target_port == self.this_port,
            "received EPR response not addressed to this port"
        );

        if let Some(memory_cell) = data.memory_cell {
            self.pending.insert(
                data.epr.request_id,
                EprResponse {
                    memory_cell,
                    client_node_id: data.epr.source_node_id,
                    client_port: data.epr.source_port,
                },
            );
        } else {
            panic!(
                "received an EPR with empty memory cell indication at server {}:{}",
                self.this_node_id, self.this_port
            );
        }

        (
            // Start timer for local operations.
            vec![Event::new(
                self.rv_local_ops.sample(&mut self.rng),
                EventType::AppEvent(AppEventData::LocalComplete(data.epr)),
            )],
            // Trace the queue of local operations.
            vec![self.pending_len_trace()],
        )
    }

    fn handle_local_complete(&mut self, now: u64, epr: EprFiveTuple) -> (Vec<Event>, Vec<Sample>) {
        let mut events = vec![];

        assert_eq!(epr.target_node_id, self.this_node_id);
        assert_eq!(epr.target_port, self.this_port);

        let request = self
            .pending
            .remove(&epr.request_id)
            .unwrap_or_else(|| panic!("non-existing pending request {}", epr));

        assert_eq!(epr.source_node_id, request.client_node_id);
        assert_eq!(epr.source_port, request.client_port);

        // Compute the fidelity on the local end of this EPR.
        let (neighbor_node_id, role, index) = request.memory_cell;
        events.push(Event::new(
            0.0,
            EventType::NodeEvent(NodeEventData::EprFidelity(EprFidelityData {
                app_node_id: request.client_node_id,
                port: request.client_port,
                consume_node_id: self.this_node_id.clone(),
                neighbor_node_id,
                role,
                index,
            })),
        ));

        (events, vec![self.pending_len_trace()])
    }
}

impl EventHandler for Server {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        match event.event_type {
            EventType::AppEvent(data) => match data {
                AppEventData::EprResponse(data) => self.handle_epr_response(data),
                AppEventData::LocalComplete(epr) => self.handle_local_complete(now, epr),
                _ => panic!("invalid event received by a server: {:?}", data),
            },
            _ => panic!(
                "invalid event {:?} received by a Application object",
                event.event_type
            ),
        }
    }

    fn initial(&mut self) -> Vec<Event> {
        vec![]
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

    use super::Server;

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
    fn test_server() {
        let this_node_id = 0;
        let this_port = 50000;
        let peer_node_id = 1;
        let peer_port = 8080;
        let request_id = 999;
        let mut server = Server::new(this_node_id, this_port, 42, 0.1);

        assert!(server.initial().is_empty());

        let five_tuple = EprFiveTuple {
            source_node_id: peer_node_id.clone(),
            source_port: peer_port.clone(),
            target_node_id: this_node_id.clone(),
            target_port: this_port.clone(),
            request_id,
        };
        let events = server
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::EprResponse(EprResponseData {
                    epr: five_tuple.clone(),
                    memory_cell: Some((2, nic::Role::Master, 0)),
                })),
            ))
            .0;
        assert_eq!(1, events.len());
        assert!(is_local_complete(&events[0].event_type, &five_tuple));

        let events = server
            .handle(Event::new(
                1.0,
                EventType::AppEvent(AppEventData::LocalComplete(five_tuple.clone())),
            ))
            .0;
        assert_eq!(1, events.len());
        assert!(is_node_epr_fidelity(&events[0].event_type));
    }
}
