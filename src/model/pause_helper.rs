//! PAUSE-HELPER, pure half (operator decision 2026-11-05, option (a) "cancel at pause-open"; `docs/DECISIONS.md` on
//! `review/menu-proposal` `deccd62`).
//!
//! Training's running state (`main+0x1558720`, installed at `0x1558658-0x1558668` at each resume) runs its exit block
//! (`0x1558f4c-0x1559118`) when the pause opens. For each FighterManager entry 1-7 that is present, not at outer
//! phase 3 (`[e+0x5920]`) and carries the live mark `[e+0x5aa0] == 0x5000_0000`, it calls the AI control-type setter
//! `0x6b82b0([e+0x4178], 1)`, which dereferences the actor alias `[e+0x4178]`. A helper entry the plugin created
//! (`entry_summon_fighter`) has no actor there until its load installs one, so a pause that opens inside that window
//! writes through a null actor (Surface lane `tr-p23-surface-b4t`).
//!
//! The runtime half (`runtime::roster::native_source_capacity`, `runtime::roster::character_sources`) retires such an
//! entry at the exit block's first word, before the walk, through the native single-slot retire the game itself runs
//! (FighterManager teardown per slot, `0x66e914-0x66e94c`; the summon removal mid-battle, `0x4eb00c-0x4eb04c`), and
//! lets the slot be created again: with its live owner reused, as the game's creation helper `0x4eb6a0` does, once
//! the entry destructor's callback arrived; otherwise once the owner cleanup arrived too. This module decides and
//! sequences; it never touches memory.

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

/// The owner context of a retired slot, read in the entry destructor's callback (Ally Round 8 F1): `resource`
/// `[G+8*slot]`, `alias` `[G+0xe8+8*slot]` (G = `[main+0x5324680]`), the owner's vtable and its header kind `+0x58`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owner {
    pub resource: usize,
    pub alias: usize,
    pub vtable: bool,
    pub header: u32,
}

impl Owner {
    /// The game's single-slot retire leaves the owner, and its creation helper `0x4eb6a0` reuses a live one
    /// (`0x4eb750-0x4eb76c` creates an owner only when `[G+0xe8+8*slot] == 0`). Reusable: still the slot's bound owner
    /// in both roots and as empty as a fresh creation leaves it (no header kind), which the creation check requires.
    pub const fn reusable(self, bound: usize) -> bool {
        bound != 0 && self.resource == bound && self.alias == bound && self.vtable && self.header == u32::MAX
    }
}

/// Per capacity slot: `Some((entry, destroyed, owner_retired))` while a pause retire waits for its completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ledger<const N: usize> {
    retiring: [Option<(usize, bool, bool)>; N],
}

impl<const N: usize> Default for Ledger<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ledger<N> {
    pub const fn new() -> Self {
        Self { retiring: [None; N] }
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

    /// Whether slot index `index` is mid-retire of `entry`.
    pub fn retiring(&self, index: usize, entry: usize) -> bool {
        matches!(self.retiring.get(index), Some(Some((retired, _, _))) if *retired == entry)
    }

    /// The destructor's callback for `entry` with its owner kept for reuse (`Owner::reusable`): the retire is complete
    /// without an owner cleanup, which the game runs only at departure.
    pub fn reused(&mut self, index: usize, entry: usize) -> bool {
        let hit = self.retiring(index, entry);
        if hit {
            self.retiring[index] = None;
        }
        hit
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
    lib_unit_tests! {
    use super::*;

    fn facts(phase: u8, mark: u32, actor: u64) -> EntryFacts {
        EntryFacts { phase, mark, actor }
    }

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
    /// retired before the walk; with its owner kept the slot is created again at once (or, without a reusable owner,
    /// once the owner cleanup arrives) and its actor arrives later.
    fn the_b4t_window_is_retired_at_pause_open_and_recreated() {
        let mut ledger = Ledger::<6>::new();
        assert!(facts(0, 0x5000_0000, 0).exposed());
        assert!(ledger.begin(1, 0x11c60aa000));
        assert!(ledger.reused(1, 0x11c60aa000), "the destructor runs inside the retire; the owner is kept");
        assert!(ledger.begin(1, 0x11c60b0000), "the re-created entry, caught by a later pause");
        assert!(!ledger.destroyed(1, 0x11c60b0000), "no reusable owner: wait for its cleanup");
        assert!(ledger.owner_retired(1), "owner cleanup: create again");
        assert!(!facts(0, 0x5000_0000, 0x11a7108600).exposed(), "the recreated helper's actor arrives");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_slot_mid_retire_is_not_retired_again_and_a_refusal_undoes_the_begin", a_slot_mid_retire_is_not_retired_again_and_a_refusal_undoes_the_begin as fn()),
            ("the_b4t_window_is_retired_at_pause_open_and_recreated", the_b4t_window_is_retired_at_pause_open_and_recreated as fn()),
        ]);
    }
    }
}
