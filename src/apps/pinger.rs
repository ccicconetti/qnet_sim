// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::*;
use crate::output::Sample;

/// Application that requests an EPR towards another node and measures the
/// fidelity immediately after it is established.
/// The requests are issued back-to-back until a maximum is reached.
#[derive(Debug)]
pub struct Pinger {
    /// Source node ID.
    this_node_id: u32,
    /// Source port.
    this_port: u16,
    /// Target node ID.
    peer_node_id: u32,
    /// Target port.
    peer_port: u16,
    /// Maximum number of requests.
    max_requests: u64,
    /// ID of the next request.
    next_request_id: u64,
    /// Timestamp of when the last request was created.
    created: u64,
}

impl Pinger {
    /// Create a new client application.
    ///
    /// Parameters:
    /// - `this_node_id`: Source node ID.
    /// - `this_port`: Source port.
    /// - `peer_node_id`: Target node ID.
    /// - `peer_port`: Target port.
    /// - `max_requests`: Maximum number of requests.
    fn new(
        this_node_id: u32,
        this_port: u16,
        peer_node_id: u32,
        peer_port: u16,
        max_requests: u64,
    ) -> Self {
        Self {
            this_node_id,
            this_port,
            peer_node_id,
            peer_port,
            next_request_id: 0,
            max_requests,
            created: 0,
        }
    }

    fn handle_epr_request(
        &mut self,
        now: u64,
        node_id: u32,
        port: u16,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert!(self.this_node_id == node_id);
        assert!(self.this_port == port);

        self.created = now;
        self.next_request_id += 1;

        // Send the EPR request to the OS.
        (
            vec![Event::new(
                0.0,
                EventType::OsEvent(OsEventData::EprRequestApp(EprFiveTuple {
                    source_node_id: self.this_node_id,
                    source_port: self.this_port,
                    target_node_id: self.peer_node_id,
                    target_port: self.peer_port,
                    request_id: self.next_request_id,
                })),
            )],
            vec![],
        )
    }

    fn handle_epr_response(
        &mut self,
        now: u64,
        data: EprResponseData,
    ) -> (Vec<Event>, Vec<Sample>) {
        assert!(
            data.is_source,
            "received EPR response addressed to the ponger at pinger {}:{}",
            self.this_node_id, self.this_port
        );

        let mut events = vec![];

        let mut data = data;

        // Compute the fidelity on the local end of this EPR.
        let memory_cell = std::mem::take(&mut data.memory_cell);
        let (neighbor_node_id, role, index) = memory_cell.expect("empty memory cell received");
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

        // Terminate if the maximum number of requests was exceeded.
        if self.next_request_id < self.max_requests {
            events.push(Event::new(
                0.0,
                EventType::AppEvent(AppEventData::EprRequest(self.this_node_id, self.this_port)),
            ));
        }

        (
            events,
            vec![Sample::Series(
                "pinger-this-node,pinger-peer-node".to_string(),
                format!("{},{}", self.this_node_id, self.peer_node_id),
                crate::utils::to_seconds(now - self.created),
            )],
        )
    }
}

impl EventHandler for Pinger {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        let now = event.time();
        match event.event_type {
            EventType::AppEvent(data) => match data {
                AppEventData::EprRequest(node_id, port) => {
                    self.handle_epr_request(now, node_id, port)
                }
                AppEventData::EprResponse(data) => self.handle_epr_response(now, data),
                _ => panic!("invalid application event received by a pinger: {:?}", data),
            },
            _ => panic!("invalid event {:?} received by a pinger", event.event_type),
        }
    }

    fn initial(&mut self) -> Vec<Event> {
        vec![Event::new(
            0.0,
            EventType::AppEvent(AppEventData::EprRequest(self.this_node_id, self.this_port)),
        )]
    }
}
