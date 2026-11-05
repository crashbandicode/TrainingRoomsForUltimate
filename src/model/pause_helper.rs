//! PAUSE-HELPER, pure half (operator decision 2026-11-05, option (a) "cancel at pause-open"; `docs/DECISIONS.md` on
//! `review/menu-proposal` `deccd62`).
//!
//! The game's Training pause state (`main+0x1558720`, installed at `0x1558658-0x1558668`) runs an exit block on resume
//! and quit (`0x1558f4c-0x1559118`). For each FighterManager entry 1-7 that is present, not at outer phase 3
//! (`[e+0x5920]`) and carries the live mark `[e+0x5aa0] == 0x5000_0000`, it calls the AI control-type setter
//! `0x6b82b0([e+0x4178], 1)`, which dereferences the actor alias `[e+0x4178]`. A helper entry the plugin created
//! (`entry_summon_fighter`) has no actor there until its load installs one, and a pause that opens inside that window
//! freezes it for the whole pause, so the exit block writes through a null actor (Surface lane `tr-p23-surface-b4t`).
//!
//! The runtime half (`runtime::roster::native_source_capacity`, `runtime::roster::character_sources`) retires such an
//! entry at pause-open through the native single-slot retire the game itself runs (FighterManager teardown per slot,
//! `0x66e914-0x66e94c`; the summon removal mid-battle, `0x4eb00c-0x4eb04c`), holds creation while the pause state is
//! installed, and lets the slot be created again once both native completions of the retire arrived (the entry
//! destructor's callback and the owner cleanup, in either order). This module decides and sequences; it never touches
//! memory.

/// What the runtime read from one present FighterManager entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryFacts {
    /// `[e+0x5920]`, the outer phase. The exit loop skips phase 3 (a Reload owns the entry).
    pub phase: u8,
    /// `[e+0x5aa0]`. The exit loop only walks `0x5000_0000`.
    pub mark: u32,
    /// `[e+0x4178]`, the actor alias the exit loop dereferences.
    pub actor: u64,
}

impl EntryFacts {
    /// Whether the pause state's exit loop calls the setter for this entry (`0x1558f84-0x1558fac` per slot).
    pub const fn walked(self) -> bool {
        self.phase != 3 && self.mark == 0x5000_0000
    }

    /// Walked with no actor: the setter would write through null.
    pub const fn exposed(self) -> bool {
        self.walked() && self.actor == 0
    }
}

/// Per capacity slot: `Some((entry, destroyed, owner_retired))` while a pause retire waits for its two completions;
/// `paused` while the pause state is installed (creation is held then: "created again after the pause").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ledger<const N: usize> {
    retiring: [Option<(usize, bool, bool)>; N],
    pub paused: bool,
}

impl<const N: usize> Ledger<N> {
    pub const fn new() -> Self {
        Self { retiring: [None; N], paused: false }
    }

    /// The runtime is about to retire `entry` in slot index `index`; false (do not retire) when that slot is already
    /// mid-retire, out of range, or `entry` is null.
    pub fn begin(&mut self, index: usize, entry: usize) -> bool {
        match self.retiring.get_mut(index) {
            Some(slot @ None) if entry != 0 => {
                *slot = Some((entry, false, false));
                true
            }
            _ => false,
        }
    }

    /// The native retire refused (the slot no longer held the entry): undo `begin`.
    pub fn refused(&mut self, index: usize) {
        if let Some(slot) = self.retiring.get_mut(index) {
            *slot = None;
        }
    }

    /// The entry destructor's callback for `entry`; true when the retire is complete (create the slot again).
    pub fn destroyed(&mut self, index: usize, entry: usize) -> bool {
        self.complete(index, Some(entry))
    }

    /// The owner cleanup of slot index `index`; true when the retire is complete (create the slot again).
    pub fn owner_retired(&mut self, index: usize) -> bool {
        self.complete(index, None)
    }

    fn complete(&mut self, index: usize, entry: Option<usize>) -> bool {
        let Some(Some((retired, destroyed, owner))) = self.retiring.get_mut(index) else {
            return false;
        };
        match entry {
            Some(entry) if entry != *retired => return false,
            Some(_) => *destroyed = true,
            None => *owner = true,
        }
        let done = *destroyed && *owner;
        if done {
            self.retiring[index] = None;
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(phase: u8, mark: u32, actor: u64) -> EntryFacts {
        EntryFacts { phase, mark, actor }
    }

    #[test]
    fn the_exit_loop_walks_exactly_what_0x1558f84_walks() {
        assert!(facts(0, 0x5000_0000, 0).walked(), "a fresh created entry (phase 0) is walked");
        assert!(facts(6, 0x5000_0000, 0x1234).walked(), "an active entry (phase 6) is walked");
        assert!(!facts(3, 0x5000_0000, 0).walked(), "phase 3 (Reload) is skipped by the loop");
        assert!(!facts(0, 0, 0).walked(), "no live mark: skipped");
        assert!(!facts(0, 0x4000_0000, 0).walked(), "another mark: skipped");
    }

    #[test]
    fn only_a_walked_entry_without_an_actor_is_exposed() {
        assert!(facts(0, 0x5000_0000, 0).exposed(), "the b4t case: created, walked, no actor");
        assert!(!facts(0, 0x5000_0000, 0x11a7108600).exposed(), "its actor is installed");
        assert!(!facts(3, 0x5000_0000, 0).exposed(), "a hat Reload at phase 3 is not walked");
        assert!(!facts(0, 0, 0).exposed());
    }

    #[test]
    fn a_retire_completes_once_both_native_callbacks_arrived_in_either_order() {
        let mut ledger = Ledger::<6>::new();
        assert!(ledger.begin(1, 0x11c60aa000));
        assert!(!ledger.destroyed(1, 0x11c60aa000), "owner cleanup still to come");
        assert!(ledger.owner_retired(1), "both arrived: create it again");
        assert!(!ledger.owner_retired(1), "already complete");
        assert!(ledger.begin(0, 0x11c60a4000));
        assert!(!ledger.owner_retired(0), "the owner can retire first");
        assert!(ledger.destroyed(0, 0x11c60a4000));
    }

    #[test]
    fn callbacks_for_other_entries_and_idle_slots_change_nothing() {
        let mut ledger = Ledger::<6>::new();
        assert!(!ledger.destroyed(1, 0x10), "an idle slot: an ordinary destruction");
        assert!(!ledger.owner_retired(1), "an idle slot: an ordinary owner cleanup");
        assert!(ledger.begin(1, 0x20));
        assert!(!ledger.destroyed(1, 0x30), "another entry in that slot");
        assert!(!ledger.owner_retired(1));
        assert!(ledger.destroyed(1, 0x20), "still waiting for 0x20, then complete");
        assert!(!ledger.destroyed(9, 0x20) && !ledger.owner_retired(9), "out of range never panics");
    }

    #[test]
    fn a_slot_mid_retire_is_not_retired_again_and_a_refusal_undoes_the_begin() {
        let mut ledger = Ledger::<6>::new();
        assert!(ledger.begin(2, 0x40));
        assert!(!ledger.begin(2, 0x40), "two pauses before the callbacks: one native retire");
        assert!(!ledger.begin(2, 0x50));
        assert!(!ledger.begin(6, 0x40), "out of range");
        assert!(!ledger.begin(3, 0), "a null entry is never retired");
        ledger.refused(2);
        ledger.refused(9);
        assert!(ledger.begin(2, 0x40), "a later pause can retire it");
    }

    /// The b4t sequence end to end: a helper is created, a pause opens inside its no-actor window, the entry is
    /// retired and creation held; after the exit block and both native callbacks the slot is created again and its
    /// actor arrives later.
    #[test]
    fn the_b4t_window_is_retired_at_pause_open_and_recreated_after_the_pause() {
        let mut ledger = Ledger::<6>::new();
        ledger.paused = true;
        assert!(facts(0, 0x5000_0000, 0).exposed());
        assert!(ledger.begin(1, 0x11c60aa000));
        assert!(!ledger.destroyed(1, 0x11c60aa000), "the destructor runs inside the retire");
        assert!(ledger.paused, "no creation until the exit block");
        ledger.paused = false;
        assert!(ledger.owner_retired(1), "owner cleanup after the pause: create again");
        assert!(!facts(0, 0x5000_0000, 0x11a7108600).exposed(), "the recreated helper's actor arrives");
    }
}
