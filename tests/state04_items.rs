//! SPEC-state-04 (as the fork's item engine does it): the item of a snapshot's choice, the source fighter
//! it needs, the turnip face override and the timers rule.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::state::held_item::HeldItem;
use model::state::ids::{Table, C};
use model::state::items::{self, Restore, Timers, Turnip};

fn state04_snapshot_precedence() {
    let item = HeldItem::Item { choice: 25, variation: 7, lifetime: 100, lifetime_max: 600, fuse: -1, fuse_max: -1 };
    match items::restore_for(&item) {
        Restore::Grant { spec, variation, timers } => {
            assert_eq!(spec.turnip, Some(Turnip::Peach));
            assert_eq!(variation, 7);
            assert_eq!(timers, Timers { lifetime: 100, lifetime_max: 600, fuse: -1, fuse_max: -1 });
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(items::restore_for(&HeldItem::Empty), Restore::Clear);
    assert_eq!(items::restore_for(&HeldItem::Unsupported), Restore::Nothing);
    // A snapshot that fails validation grants nothing.
    let bad = HeldItem::Item { choice: 11, variation: 3, lifetime: 0, lifetime_max: 0, fuse: -1, fuse_max: -1 };
    assert_eq!(items::restore_for(&bad), Restore::Nothing);
    // No snapshot at all (a save without one) means no item is applied: there is nothing to call.
}

fn state04_face_probability_hooks() {
    // 58.0 for the chosen face, 0.0 for every other, pass-through when nothing is set.
    for n in 1..=8u8 {
        assert_eq!(items::face_probability(n, None), None);
        for face in 1..=8u32 {
            let p = items::face_probability(n, Some(face)).unwrap();
            assert_eq!(p, if face == u32::from(n) { 58.0 } else { 0.0 });
        }
    }
    // The chosen face is the snapshot's variation plus one.
    assert_eq!(items::turnip_face(0), 1);
    assert_eq!(items::turnip_face(7), 8);
}

fn state04_choice_of_a_held_item() {
    let ids = Table::sequential(5000);
    let kind_of = |c: C| ids.get(c);
    // Link's Remote Bomb with variation 0 is choice 11; a kind nobody lists is none.
    assert_eq!(items::choice_of(ids.get(C::ItemLinkBomb), 0, kind_of), Some(11));
    assert_eq!(items::choice_of(ids.get(C::ItemGyro), 5, kind_of), Some(29), "R.O.B.'s gyro keeps its colour");
    assert_eq!(items::choice_of(ids.get(C::ItemGyro), 9, kind_of), None);
    assert_eq!(items::choice_of(1, 0, kind_of), None);
    // A turnip with the winky face (5) is the Daisy winky choice (3).
    assert_eq!(items::choice_of(ids.get(C::ItemDaisyTurnip), 5, kind_of), Some(3));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("state04_snapshot_precedence", state04_snapshot_precedence as fn()),
        ("state04_face_probability_hooks", state04_face_probability_hooks as fn()),
        ("state04_choice_of_a_held_item", state04_choice_of_a_held_item as fn()),
    ]);
}
