//! The practice session lease (`EntryLease`, carried `practice_policy`): a replaced entry (a hot swap) retires the session, a later
//! frame acquires a fresh one, and dropping a lease that is not held reports nothing. The runtime callers are
//! `runtime::lobby::{update, frame}` (`forget` at the lobby practice's start and end, `refresh` then `acquire` per frame) and
//! `character_roster`'s destruction path (`entry_departed`). Not carried: written for this repository.

lib_unit_tests! {
use super::practice_policy::{Context, Entries, EntryLease};

fn entries(player: usize, cpu: usize) -> Entries {
    Entries { context: Context::Waiting, player, cpu }
}

fn a_hot_swap_that_replaces_an_entry_retires_the_session_and_the_next_frame_starts_a_new_one() {
    let mut lease = EntryLease::new();
    let first = entries(0x1000, 0x9000);
    assert!(lease.acquire(first), "a session with both entries ready acquires");
    assert!(lease.active() && lease.owns(0x9000));
    // The frame after a hot swap resolved a different CPU entry object.
    let swapped = entries(0x1000, 0xa000);
    assert!(lease.replaced(Some(swapped)), "the replaced entry is seen");
    assert!(lease.retire(), "the session retires (the runtime clears the session state)");
    assert!(!lease.active());
    assert!(lease.acquire(swapped), "the next frame acquires a fresh session");
    assert!(lease.owns(0xa000) && !lease.owns(0x9000));
}

fn an_unreadable_entry_also_retires_and_a_dropped_lease_reports_nothing() {
    let mut lease = EntryLease::new();
    assert!(!lease.retire(), "forgetting an empty lease is silent");
    assert!(lease.acquire(entries(0x1000, 0x9000)));
    assert!(lease.replaced(None), "an entry that cannot be resolved counts as replaced");
    assert!(lease.retire());
    assert!(!lease.retire(), "retired once");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_hot_swap_that_replaces_an_entry_retires_the_session_and_the_next_frame_starts_a_new_one", a_hot_swap_that_replaces_an_entry_retires_the_session_and_the_next_frame_starts_a_new_one as fn()),
        ("an_unreadable_entry_also_retires_and_a_dropped_lease_reports_nothing", an_unreadable_entry_also_retires_and_a_dropped_lease_reports_nothing as fn()),
    ]);
}
}
