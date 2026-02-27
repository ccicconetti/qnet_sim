// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::timed_event::TimedEvent;

pub struct EventQueue<E: TimedEvent> {
    queue: std::collections::BinaryHeap<E>,
    last_time: u64,
}

impl<E: TimedEvent> Default for EventQueue<E> {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            last_time: Default::default(),
        }
    }
}

impl<E: TimedEvent> EventQueue<E> {
    /// Add an event with relative time to the event queue.
    pub fn push(&mut self, mut event: E) {
        event.advance(self.last_time);
        self.queue.push(event);
    }

    /// Return the next event, if any.
    pub fn pop(&mut self) -> Option<E> {
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
