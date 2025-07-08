// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::*;
use crate::output::Sample;

/// Ponger application, that simply replies to any requesting an EPR and
/// measures the fidelity immediately.
#[derive(Debug)]
pub struct Ponger {
    /// Source node ID.
    this_node_id: u32,
    /// Source port.
    this_port: u16,
}

impl Ponger {
    /// Create a new ponger application.
    ///
    /// Parameters:
    /// - `this_node_id`: Source node ID.
    /// - `this_port`: Source port.
    pub fn new(this_node_id: u32, this_port: u16) -> Self {
        Self {
            this_node_id,
            this_port,
        }
    }

    fn handle_epr_response(&mut self, data: EprResponseData) -> (Vec<Event>, Vec<Sample>) {
        assert!(
            !data.is_source,
            "received EPR response addressed to the pinger at a ponger {}:{}",
            self.this_node_id, self.this_port
        );
        assert!(
            data.epr.target_node_id == self.this_node_id,
            "received EPR response not addressed to this node"
        );
        assert!(
            data.epr.target_port == self.this_port,
            "received EPR response not addressed to this port"
        );

        if let Some(memory_cell_id) = data.memory_cell {
            (
                vec![Event::new(
                    0.0,
                    EventType::NetworkEvent(NetworkEventData::EprConsume(EprConsumeData {
                        req_app_node_id: data.epr.source_node_id,
                        req_app_port: data.epr.source_port,
                        consume_node_id: self.this_node_id.clone(),
                        memory_cell_id,
                    })),
                )],
                vec![],
            )
        } else {
            panic!(
                "received an EPR with empty memory cell indication at server {}:{}",
                self.this_node_id, self.this_port
            );
        }
    }
}

impl EventHandler for Ponger {
    fn handle(&mut self, event: Event) -> (Vec<Event>, Vec<Sample>) {
        match event.event_type {
            EventType::AppEvent(data) => match data {
                AppEventData::EprResponse(data) => self.handle_epr_response(data),
                _ => panic!("invalid event received by a ponger: {:?}", data),
            },
            _ => panic!("invalid event {:?} received by a ponger", event.event_type),
        }
    }

    fn initial(&mut self) -> Vec<Event> {
        vec![]
    }
}
