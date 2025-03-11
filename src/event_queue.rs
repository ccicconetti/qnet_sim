// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::Event;

#[derive(Default)]
pub struct EventQueue {
    queue: std::collections::BinaryHeap<Event>,
    last_time: u64,
}

impl EventQueue {
    /// Add an event with relative time to the event queue.
    pub fn push(&mut self, event: Event) {
        let mut event = event;
        event.advance(self.last_time);
        self.queue.push(event);
    }

    /// Add all the events to the event queue.
    pub fn push_many(&mut self, events: Vec<Event>) {
        for event in events {
            self.push(event);
        }
    }

    /// Return the next event, if any.
    pub fn pop(&mut self) -> Option<Event> {
        let last_event = self.queue.pop();
        if let Some(event) = &last_event {
            self.last_time = event.time();
        }
        last_event
    }

    pub fn last_time(&self) -> u64 {
        self.last_time
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
