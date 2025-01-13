// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::event::Event;

#[derive(Default)]
pub struct EventQueue {
    queue: std::collections::BinaryHeap<Event>,
}

impl EventQueue {
    pub fn push(&mut self, event: Event) {
        self.queue.push(event);
    }
    pub fn pop(&mut self) -> Option<Event> {
        self.queue.pop()
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
