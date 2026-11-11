//! Diag builds: a game thread that stopped (primary inbox 2026-11-08 20:56Z). Eden covrest-yoga-05: after a plugin menu
//! close following a diag cycle reload, no hook ran again and no diag line came for two minutes. The watchdog thread
//! samples the hooks' total entry count; this decides when the count's silence is a stall and when it is over, so the
//! runtime can then log who holds which lock, the last hook entered and where the menu worker is.

/// No hook entry for longer than this, once armed, is a stall.
pub const STALL_MS: u64 = 2_000;
/// Hook entries before the watch arms: the boot has long stretches without a hook.
pub const ARM_ENTRIES: u64 = 1_000;

/// The menu worker's place in a press and a close (`runtime::stall_watch::worker_step`), by index.
pub const WORKER_STEPS: [&str; 13] = [
    "idle",
    "press",
    "recording_effect",
    "shortcuts",
    "begin_close",
    "post_close",
    "close_commit",
    "publish",
    "closed",
    "sync_panels",
    "apply_kept",
    "setups_close",
    "close_flags",
];

/// The game thread's place in the menu's Mii hand-off (`runtime::stall_watch::game_step`, set by
/// `mii::menu_handoff::service`): the close and open transitions call the saved-Mii adapters, which take the Mii
/// owners' locks and read the native catalog (rc6-bg-qp3's menu had a qualified catalog when it stopped).
pub const GAME_STEPS: [&str; 4] = ["none", "handoff_open", "handoff_close", "handoff_done"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// No hook entered for `quiet_ms`; the count stands at `entries`.
    Stalled { entries: u64, quiet_ms: u64 },
    /// Hooks run again after `quiet_ms` without one.
    Resumed { entries: u64, quiet_ms: u64 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Watch {
    entries: u64,
    since_ms: u64,
    stalled: bool,
}

impl Watch {
    pub const fn new() -> Self {
        Self { entries: 0, since_ms: 0, stalled: false }
    }

    /// One sample: the hooks' total entry count at `now_ms`. A stall is reported once, and its end once.
    pub fn observe(&mut self, entries: u64, now_ms: u64) -> Option<Event> {
        let quiet_ms = now_ms.saturating_sub(self.since_ms);
        if entries != self.entries {
            let was = core::mem::replace(&mut self.stalled, false);
            self.entries = entries;
            self.since_ms = now_ms;
            return was.then_some(Event::Resumed { entries, quiet_ms });
        }
        if !self.stalled && entries >= ARM_ENTRIES && quiet_ms > STALL_MS {
            self.stalled = true;
            return Some(Event::Stalled { entries, quiet_ms });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn a_silent_count_is_a_stall_once_and_its_end_once() {
        let mut watch = Watch::new();
        assert_eq!(watch.observe(5_000, 1_000), None);
        assert_eq!(watch.observe(5_000, 2_500), None, "1.5 s without a hook is not a stall");
        assert_eq!(watch.observe(5_000, 3_001), Some(Event::Stalled { entries: 5_000, quiet_ms: 2_001 }));
        assert_eq!(watch.observe(5_000, 9_000), None, "reported once");
        assert_eq!(watch.observe(5_001, 9_500), Some(Event::Resumed { entries: 5_001, quiet_ms: 8_500 }));
        assert_eq!(watch.observe(5_100, 9_600), None);
    }

    #[test]
    fn the_boot_and_a_running_game_are_not_stalls() {
        let mut watch = Watch::new();
        assert_eq!(watch.observe(10, 1_000), None);
        assert_eq!(watch.observe(10, 60_000), None, "not armed before ARM_ENTRIES");
        for t in 0..100 {
            assert_eq!(watch.observe(2_000 + t * 60, 61_000 + t * 250), None);
        }
    }
    }
}
