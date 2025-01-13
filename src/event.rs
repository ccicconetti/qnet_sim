// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// For all the events there is the time when it is scheduled to occur.
#[derive(PartialEq, Eq)]
pub enum Event {
    /// The warm-up period expires.
    WarmupPeriodEnd(u64),
    /// The simulation ends.
    ExperimentEnd(u64),
    /// Print progress.
    Progress(u64, u16),
}

impl Event {
    pub fn time(&self) -> u64 {
        match self {
            Self::WarmupPeriodEnd(t) | Self::ExperimentEnd(t) | Self::Progress(t, _) => *t,
        }
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.time().partial_cmp(&self.time())
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
