//! Resolved status handoff from the game resolver to the non-hook service.
use super::ids::{Table, C, UNRESOLVED};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Exact UI initializer order; no native lookup occurs in the consumer.
pub const STATUS_IDS: [C; 7] = [
    C::StatusDamageFly,
    C::StatusDamageFlyRoll,
    C::StatusDamageFlyMeteor,
    C::StatusDamageFlyReflectD,
    C::StatusDamageFlyReflectU,
    C::StatusDamageFlyReflectLr,
    C::StatusDamageFall,
];

pub struct Init {
    ids: [AtomicI32; 7],
    available: AtomicBool,
    attempted: AtomicBool,
    ready: AtomicBool,
}
impl Default for Init {
    fn default() -> Self {
        Self::new()
    }
}
impl Init {
    pub const fn new() -> Self {
        Self {
            ids: [const { AtomicI32::new(UNRESOLVED) }; 7],
            available: AtomicBool::new(false),
            attempted: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }
    /// Single resolver producer: publish nothing until all seven names resolved.
    pub fn publish(&self, table: &Table) -> bool {
        let ids = STATUS_IDS.map(|c| table.get(c));
        if ids.contains(&UNRESOLVED) {
            return false;
        }
        for (slot, id) in self.ids.iter().zip(ids) {
            slot.store(id, Ordering::Relaxed);
        }
        self.available.store(true, Ordering::Release);
        true
    }
    /// Only the existing non-hook heartbeat calls this. Failure stays unavailable.
    pub fn service(&self, init: impl FnOnce([i32; 7]) -> Option<()>) {
        if !self.available.load(Ordering::Acquire) || self.attempted.swap(true, Ordering::AcqRel) {
            return;
        }
        let ids = std::array::from_fn(|i| self.ids[i].load(Ordering::Relaxed));
        if init(ids).is_some() {
            self.ready.store(true, Ordering::Release);
        }
    }
    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}
