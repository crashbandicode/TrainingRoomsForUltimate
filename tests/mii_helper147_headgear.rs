//! Helper callback admission. Captured hats are the before-value. A changed hat
//! is a fresh-O decision for a helper that belongs to the layout. Training uses
//! slots 2/3. Waiting uses slots 5/6. Synthetic hat ids only; no native fixture.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).

use {crate::engine::mii::backend::Layout, crate::engine::mii::ordinary_cpu as cpu};

fn training_helpers_admit_a_changed_hat_and_an_unequal_saved_pair() {
    let saved = [0u16, 125];
    assert!(cpu::helper_slot_admission(Layout::Training, 2, saved, 125));
    assert!(cpu::helper_slot_admission(Layout::Training, 3, saved, 0));
    assert!(cpu::helper_slot_admission(Layout::Training, 2, saved, 0));
    assert!(cpu::helper_slot_admission(Layout::Training, 3, saved, 125));
    assert_eq!(cpu::headgear_reload(Layout::Training, 2, 0, 125), Ok(true));
    assert_eq!(cpu::headgear_reload(Layout::Training, 3, 125, 0), Ok(true));
    assert_eq!(cpu::headgear_reload(Layout::Training, 2, 0, 0), Ok(false));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("training_helpers_admit_a_changed_hat_and_an_unequal_saved_pair", training_helpers_admit_a_changed_hat_and_an_unequal_saved_pair as fn()),
    ]);
}
