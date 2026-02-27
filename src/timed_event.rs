// SPDX-FileCopyrightText: © 2026 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Trait bound required by `EventQueue`.
///
/// - Must be orderable (for `BinaryHeap`).
/// - Must expose its timestamp (`time`).
/// - Must be able to advance from a relative time to an absolute time (`advance`).
pub trait TimedEvent: Ord {
    fn time(&self) -> u64;
    /// Convert a relative-time event into an absolute-time event by shifting it by `base_time`.
    fn advance(&mut self, base_time: u64);
}
