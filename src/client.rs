// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

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
    fn get_request(&mut self, epr: &EprFiveTuple) -> &mut EprRequest {
        assert_eq!(epr.source_node_id, self.this_node_id);
        assert_eq!(epr.source_port, self.this_port);
        assert_eq!(epr.target_node_id, self.peer_node_id);
        assert_eq!(epr.target_port, self.peer_port);

        self.pending
            .get_mut(&epr.request_id)
            .unwrap_or_else(|| panic!("non-existing pending request {}", epr))
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
        let mut samples = vec![];

        let request = self.get_request(&epr);

        assert!(
            request.local_operations_done,
            "duplicate execution of local operations for request {}",
            epr
        );
        request.local_operations_done = true;

        if request.remote_operations_done {
            let memory_cell = std::mem::take(&mut request.memory_cell);
            let (neighbor_node_id, role, index) = memory_cell
                .unwrap_or_else(|| panic!("local operation completed on a failed request {}", epr));
            events.push(Event::new(
                0.0,
                EventType::NodeEvent(NodeEventData::EprFidelity(EprFidelityData {
                    app_node_id: self.this_node_id,
                    port: self.this_port,
                    consume_node_id: self.this_node_id,
                    neighbor_node_id,
                    role,
                    index,
                })),
            ));

            let epr_request = self.pending.remove(&epr.request_id);

            if let Some(epr_request) = epr_request {
                samples.push(Sample::Series(
                    "latency-node,latency-port".to_string(),
                    format!("{},{}", self.this_node_id, self.this_port),
                    crate::utils::to_seconds(now - epr_request.created),
                ));
            }
        }

        (events, samples)
    }

    fn handle_remote_complete(&mut self, now: u64, epr: EprFiveTuple) -> (Vec<Event>, Vec<Sample>) {
        let mut samples = vec![];

        let request = self.get_request(&epr);

        assert!(
            request.remote_operations_done,
            "duplicate execution of remote operations for request {}",
            epr
        );
        request.remote_operations_done = true;

        if request.local_operations_done {
            let epr_request = self.pending.remove(&epr.request_id);

            if let Some(epr_request) = epr_request {
                samples.push(Sample::Series(
                    "latency-node,latency-port".to_string(),
                    format!("{},{}", self.this_node_id, self.this_port),
                    crate::utils::to_seconds(now - epr_request.created),
                ));
            }
        }

        (vec![], samples)
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

    #[test]
    fn test_client() {
        // XXX
    }
}
