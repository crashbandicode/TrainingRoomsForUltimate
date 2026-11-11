//! Menu/request ownership arbitration only; no native calls or pilot timers.
//! A changed setting is the only way to lift a completed request's inhibition.

pub(crate) const fn level(enabled: bool, selected: Option<u8>) -> u8 {
    match (enabled, selected) {
        (true, Some(level @ 1..=9)) => level,
        _ => 0,
    }
}

/// Low four bits hold Off (0) or level 1..9; the rest is a change revision.
/// Publishing the same effective setting is deliberately idempotent.
pub(crate) fn update(previous: u64, level: u8) -> u64 {
    if previous & 15 == u64::from(level) {
        previous
    } else {
        (previous & !15).wrapping_add(16) | u64::from(level)
    }
}

pub(crate) struct Arbitration {
    observed: u64,
    inhibited: bool,
}

impl Arbitration {
    pub(crate) const fn new() -> Self {
        Self {
            observed: 0,
            inhibited: false,
        }
    }

    pub(crate) fn request_started(&mut self) {
        self.inhibited = true;
    }

    /// Call only after menu capture ends. A request owns the whole session,
    /// including menu pauses: commits during it cannot queue menu AI afterward.
    pub(crate) fn observe(&mut self, published: u64, request_active: bool) -> Option<u8> {
        if self.observed != published {
            self.observed = published;
            if !request_active {
                self.inhibited = false;
            }
        }
        if request_active {
            self.request_started();
        }
        let level = (published & 15) as u8;
        (!self.inhibited && (1..=9).contains(&level)).then_some(level)
    }
}

