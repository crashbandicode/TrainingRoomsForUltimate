//! SPEC-state-04 (as the fork's item engine does it): the item of a snapshot's choice, the source fighter
//! it needs, the turnip face override and the timers rule.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::state::held_item::{HeldItem, ITEM_NAMES};
use model::state::ids::{Table, C};
use model::state::items::{self, Restore, Timers, Turnip, Variation};

#[test]
fn state04_table_lookup() {
    // Every choice of the fork's list has a spec; a choice outside it has none.
    for choice in 0..ITEM_NAMES.len() as u8 {
        assert!(items::spec_for(choice).is_some(), "{}", ITEM_NAMES[usize::from(choice)]);
    }
    assert!(items::spec_for(38).is_none());
    // The names agree with the sources (a spot check against the list).
    assert_eq!(items::spec_for(2).unwrap().turnip, Some(Turnip::Daisy));
    assert_eq!(items::spec_for(25).unwrap().turnip, Some(Turnip::Peach));
    assert_eq!(items::spec_for(29).unwrap().source, Some(C::KindRob));
    assert_eq!(items::spec_for(11).unwrap().kind, C::ItemLinkBomb);
}

#[test]
fn state04_common_items_need_no_source_fighter() {
    for choice in [6, 7, 8, 26, 27] {
        assert_eq!(items::spec_for(choice).unwrap().source, None, "{}", ITEM_NAMES[usize::from(choice)]);
    }
    for choice in [0, 1, 9, 10, 11, 12, 13, 14, 21, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37] {
        assert!(items::spec_for(choice).unwrap().source.is_some(), "{}", ITEM_NAMES[usize::from(choice)]);
    }
    assert_eq!(items::spec_for(7).unwrap().variation, Variation::Constant(C::ItemBombheiNormal));
}

#[test]
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

#[test]
fn state04_timers_only_when_the_maxima_match() {
    let t = Timers { lifetime: 500, lifetime_max: 1400, fuse: -1, fuse_max: 0 };
    assert!(items::timers_apply(&t, 1400, 0));
    assert!(!items::timers_apply(&t, 1300, 0));
    assert!(!items::timers_apply(&t, 1400, 5));
}

#[test]
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

#[test]
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

#[test]
fn state04_no_menu_item_applied_on_load() {
    // The machine hands the player's snapshot to the restore; a record with none carries no item.
    use model::state::record::SavedState;
    assert!(SavedState::never_saved().held_item.is_none());
}
