// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use rand_distr::Distribution;

use crate::event::*;
use crate::output::Sample;

#[derive(Debug, Default)]
struct EprRequest {
    /// Neighbor node ID, used to identify the NIC, and memory cell index.
    /// If None then the application is still waiting for the OS to indicate
    /// if the EPR was established or not.
    memory_cell: Option<(u32, usize)>,
    /// True if the local operations have been done.
    local_operations_done: bool,
    /// True if the remote operations have been done.
    remote_operations_done: bool,
}

/// Every EPR request is uniquely identified by the five-tuple:
/// source node ID and port
/// target node ID and port
/// request ID
#[derive(Debug)]
pub struct Application {
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

impl Application {
    fn get_request(
        &mut self,
        source_node_id: u32,
        source_port: u16,
        request_id: u64,
    ) -> &mut EprRequest {
        assert_eq!(source_node_id, self.this_node_id);
        assert_eq!(source_port, self.this_port);

        self.pending.get_mut(&request_id).unwrap_or_else(|| {
            panic!(
                "non-existing pending request {} at application {}:{}",
                request_id, self.this_node_id, self.this_port
            )
        })
    }
}

impl EventHandler for Application {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let mut events = vec![];
        let mut samples = vec![];

        match event.event_type {
            EventType::AppEvent(data) => match data {
                AppEventData::EprRequest(node_id, port) => {
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

                    self.pending
                        .insert(self.next_request_id, Default::default());
                    self.next_request_id += 1;

                    samples.push(Sample::Series(
                        "app_pending_len".to_string(),
                        format!("{}:{}", self.this_node_id, self.this_port),
                        self.pending.len() as f64,
                    ));

                    // Generate a new EPR request for the application.
                    events.push(Event::new(
                        self.rv_next_epr.sample(&mut self.rng),
                        EventType::AppEvent(AppEventData::EprRequest(
                            self.this_node_id,
                            self.this_port,
                        )),
                    ));
                }
                AppEventData::EprResponse(data) => {
                    let request =
                        self.get_request(data.source_node_id, data.source_port, data.request_id);

                    assert!(
                        request.memory_cell.is_none(),
                        "duplicate response received for request {} at application {}:{}",
                        data.request_id,
                        self.this_node_id,
                        self.this_port
                    );

                    if let Some(memory_cell) = data.memory_cell {
                        // Request successful.
                        request.memory_cell = Some(memory_cell);

                        // Start timer for local operations.
                        events.push(Event::new(
                            self.rv_local_ops.sample(&mut self.rng), // XXX
                            EventType::AppEvent(AppEventData::LocalComplete(
                                self.this_node_id,
                                self.this_port,
                                data.request_id,
                            )),
                        ));
                    } else {
                        self.pending.remove(&data.request_id);
                    }
                }
                AppEventData::LocalComplete(source_node_id, source_port, request_id) => {
                    let request = self.get_request(source_node_id, source_port, request_id);

                    assert!(request.local_operations_done, "duplicate execution of local operations for request {} at application {}:{}",request_id, self.this_node_id, self.this_port);
                    request.local_operations_done = true;

                    if request.remote_operations_done {
                        // Compute fidelity XXX
                        self.pending.remove(&request_id);
                    }
                }
                AppEventData::RemoteComplete(source_node_id, source_port, request_id) => {
                    let request = self.get_request(source_node_id, source_port, request_id);

                    assert!(request.remote_operations_done, "duplicate execution of remote operations for request {} at application {}:{}",request_id, self.this_node_id, self.this_port);
                    request.remote_operations_done = true;

                    if request.local_operations_done {
                        // Compute fidelity XXX
                        self.pending.remove(&request_id);
                    }
                }
            },
            _ => panic!(
                "invalid event {:?} received by a Application object",
                event.event_type
            ),
        }

        (events, samples)
    }

    fn initial(&mut self) -> Vec<Event> {
        vec![Event::new(
            self.rv_next_epr.sample(&mut self.rng),
            EventType::AppEvent(AppEventData::EprRequest(self.this_node_id, self.this_port)),
        )]
    }
}
