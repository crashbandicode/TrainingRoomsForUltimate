//! F9 (fix ledger; lead ruling 2026-11-02): Training's parked CPU entries and the roster cycle's helper sources.
//!
//! Native Training keeps CPU entries 2 and 3 parked beside the visible 0 and 1; the cycle's source book admits a helper fighter into
//! a parked entry only while the pause menu's No. of CPUs is 1 (the fork's rule, `qualified_backend`). Raising the count makes the
//! game activate entries 2..=count with whatever body they hold, so a helper left there showed as the extra CPU (Eden s6-04). This
//! is the decision the source book takes on each entry's native loader callback: an owned helper in an entry the count makes live is
//! given back (the entry is reinstalled with the native fighter it held before the helper, captured at admission), unless a consumer
//! still leases it (then it waits for the release). Pure and host-tested; the runtime half is in `runtime::roster::character_sources`.

/// The Training entries the cycle may use as helpers (the fork's `CALLBACK_SLOTS` for Training).
pub const PARKED: [u32; 2] = [2, 3];

/// What the loader callback of a parked entry does with an owned helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryAction {
    /// Nothing to do: not Training, not a parked entry, no helper, or the entry stays parked.
    Keep,
    /// The count made this entry live: reinstall the native fighter before the game shows the helper.
    GiveBack,
    /// The entry is live but the helper is leased (a Kirby hat or an item uses it) or its native fighter is unknown: retry on the
    /// next callback.
    Wait,
}

/// Whether the source book may admit a new helper in Training: exactly one visible CPU (the fork's rule; an unknown count, e.g. its
/// guards failed, admits nothing: the preload-off path).
pub fn admits(quantity: Option<u32>) -> bool {
    quantity == Some(1)
}

/// Whether the count makes `slot` a live CPU entry (CPU entries are 1..=count).
pub fn live(quantity: Option<u32>, slot: u32) -> bool {
    quantity.is_some_and(|count| slot >= 1 && slot <= count)
}

/// The decision for `slot` on its loader callback.
pub fn action(training: bool, quantity: Option<u32>, slot: u32, owned: bool, leased: bool, native_kind: Option<u32>) -> EntryAction {
    if !training || !PARKED.contains(&slot) || !owned || !live(quantity, slot) {
        return EntryAction::Keep;
    }
    if leased || native_kind.is_none() {
        return EntryAction::Wait;
    }
    EntryAction::GiveBack
}

#[cfg(test)]
mod tests {
    use super::*;

    const MYTHRA: Option<u32> = Some(0x5b);

    #[test]
    fn a_helper_admitted_at_one_is_given_back_when_the_count_makes_its_entry_live() {
        // Admitted at 1 (entry 2 holds the helper, the native kind was captured).
        assert!(admits(Some(1)));
        assert_eq!(action(true, Some(1), 2, true, false, MYTHRA), EntryAction::Keep);
        // Count 2: entry 2 goes live, entry 3 stays parked.
        assert_eq!(action(true, Some(2), 2, true, false, MYTHRA), EntryAction::GiveBack);
        assert_eq!(action(true, Some(2), 3, true, false, MYTHRA), EntryAction::Keep);
        assert!(!admits(Some(2)), "no new helper while more than one CPU is visible");
        // Count 3: both parked entries are live.
        assert_eq!(action(true, Some(3), 3, true, false, MYTHRA), EntryAction::GiveBack);
        // Back to 1: nothing owned any more, and admission is allowed again.
        assert_eq!(action(true, Some(1), 2, false, false, None), EntryAction::Keep);
        assert!(admits(Some(1)));
    }

    #[test]
    fn a_leased_helper_or_an_unknown_native_fighter_waits() {
        assert_eq!(action(true, Some(2), 2, true, true, MYTHRA), EntryAction::Wait);
        assert_eq!(action(true, Some(2), 2, true, false, None), EntryAction::Wait);
    }

    #[test]
    fn the_maximum_count_leaves_no_helper_entry_and_falls_back_to_preload_off() {
        // Count 3 (the pause menu's maximum): every parked entry is live and nothing may be admitted: the source book is empty,
        // swaps load synchronously (the shipped preload-off path) and hats/items keep the engine's existing refusal notice.
        assert!(PARKED.iter().all(|slot| live(Some(3), *slot)));
        assert!(!admits(Some(3)));
        // An unknown count (the F9 guards failed) admits nothing either.
        assert!(!admits(None));
        assert_eq!(action(true, None, 2, true, false, MYTHRA), EntryAction::Keep);
    }

    #[test]
    fn outside_training_and_outside_the_parked_entries_nothing_changes() {
        assert_eq!(action(false, Some(3), 2, true, false, MYTHRA), EntryAction::Keep);
        assert_eq!(action(true, Some(3), 5, true, false, MYTHRA), EntryAction::Keep);
        assert_eq!(action(true, Some(3), 1, true, false, MYTHRA), EntryAction::Keep);
    }
}
