//! Lock-free, process-lifetime counts of Perfect's changed pads.

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum PerfectRule {
    Route = 1,
    Jump = 2,
    DistanceDodge = 3,
    Reach = 4,
    Detour = 5,
    Niche = 6,
    Landing = 11,
    ReactDodge = 12,
    HitRecovery = 13,
    Meteor = 14,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RuleCounts {
    /// R1 through R5, in order.
    pub edge: [u32; 5],
    /// J1 through J4, in order.
    pub juggler: [u32; 4],
    /// Native trace rule ID. Zero means no Perfect correction has been applied.
    pub last_rule: u32,
}

#[derive(Default)]
pub struct Counters {
    counts: [AtomicU32; PerfectRule::Meteor as usize + 1],
    last: AtomicU32,
}

impl Counters {
    pub const fn new() -> Self {
        Self {
            counts: [const { AtomicU32::new(0) }; PerfectRule::Meteor as usize + 1],
            last: AtomicU32::new(0),
        }
    }

    pub fn record(&self, rule: PerfectRule) {
        self.counts[rule as usize].fetch_add(1, Ordering::Relaxed);
        self.last.store(rule as u32, Ordering::Release);
    }

    pub fn snapshot(&self) -> RuleCounts {
        let last_rule = self.last.load(Ordering::Acquire);
        let load = |rule: usize| self.counts[rule].load(Ordering::Relaxed);
        RuleCounts {
            edge: std::array::from_fn(|i| load(PerfectRule::Route as usize + i)),
            juggler: std::array::from_fn(|i| load(PerfectRule::Landing as usize + i)),
            last_rule,
        }
    }
}
