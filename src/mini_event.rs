// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::timed_event::TimedEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum MiniEventType {
    /// The warm-up period expires.
    WarmupPeriodEnd,
    /// The simulation ends.
    ExperimentEnd,
    /// Print progress.
    Progress(u16),
}

/// A mini simulation event.
#[derive(PartialEq, Eq)]
pub struct MiniEvent {
    /// The simulated time associated with the event.
    time: u64,
    /// The event type.
    pub event_type: MiniEventType,
}

impl MiniEvent {
    /// Create a new event to be executed at the specified relative time, in s.
    pub fn new(time_relative: f64, event_type: MiniEventType) -> Self {
        Self {
            time: crate::utils::to_nanoseconds(time_relative),
            event_type,
        }
    }

    /// Create a new event to be executed right now..
    pub fn immediate(event_type: MiniEventType) -> Self {
        Self {
            time: 0,
            event_type,
        }
    }

    /// Reset the event time, in s.
    pub fn reset(&mut self, time_relative: f64) {
        self.time = crate::utils::to_nanoseconds(time_relative);
    }
}

impl crate::timed_event::TimedEvent for MiniEvent {
    /// Return the time of the event, in ns.
    fn time(&self) -> u64 {
        self.time
    }

    /// Advance the event time by the specified period, in ns.
    fn advance(&mut self, advance_time: u64) {
        self.time += advance_time
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for MiniEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.time().partial_cmp(&self.time())
    }
}

impl Ord for MiniEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
