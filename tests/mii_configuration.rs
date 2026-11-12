//! Persisted identity and native value ownership survive representation changes.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}
#[allow(dead_code)]
#[path = "../src/model/hook_stats.rs"]
mod hook_stats;

use mii::inventory::MiiClass;
use mii::native_values::RowIds;
use mii::{editor_row as value, inventory};
#[path = "../src/runtime/mii/mii_editor_bridge/draft.rs"]
mod draft;

#[global_allocator]
static ALLOCATOR: hook_stats::CountingAllocator = hook_stats::CountingAllocator;

const ROWS: RowIds = RowIds { outfit: 0x1300070102030405, hat: 0x5a0009060708090a };

fn editor_row() -> value::Row {
    let mut row = [0; 432];
    row[0x114] = 1;
    row[0x118] = b'A';
    row[0x130..0x138].copy_from_slice(&ROWS.outfit.to_le_bytes());
    row[0x138..0x140].copy_from_slice(&ROWS.hat.to_le_bytes());
    row[0x140] = 255;
    row[0x141] = 254;
    row[0x142..0x146].copy_from_slice(&[0, 1, 2, 1]);
    row[0x148..0x1a0].fill(42);
    row[0x1a0..0x1a8].copy_from_slice(&0xc1000013fdfdfb2bu64.to_le_bytes());
    row
}

fn draft_refinement_preserves_error_order_and_independent_copies() {
    use draft::Draft;
    let pending = Draft::native_default(MiiClass::Brawler);
    assert_eq!(pending.row(), None);
    assert_eq!(pending.hash(), 0);
    assert_eq!(pending.validate(), Ok(()));
    let row = editor_row();
    let mut refined = pending;
    let mut identity = row;
    identity[0] = 1;
    assert_eq!(refined.materialize(MiiClass::Gunner, identity), Err("default_intent_changed"));
    assert_eq!(refined, pending);
    assert_eq!(refined.materialize(MiiClass::Brawler, identity), Err("default_identity_not_zero"));
    assert_eq!(refined, pending);
    let mut invalid = row;
    invalid[0x142] = 3;
    assert_eq!(refined.materialize(MiiClass::Brawler, invalid), Err("Invalid edited Mii"));
    assert_eq!(refined, pending);
    let mut other_class = row;
    other_class[0x1a0..0x1a8].copy_from_slice(&0xc100001257ab1d87u64.to_le_bytes());
    assert_eq!(refined.materialize(MiiClass::Brawler, other_class), Err("default_class_mismatch"));
    assert_eq!(refined, pending);
    assert_eq!(refined.materialize(MiiClass::Brawler, row), Ok(()));
    assert_eq!(pending.row(), None);
    assert_eq!(refined.row(), Some(&row));
    assert_eq!(refined.pending_class(), None);
    assert_eq!(refined.validate(), Ok(()));
    assert_eq!(refined, Draft::ready(row));
    let copy = refined;
    assert_eq!(refined.materialize(MiiClass::Brawler, identity), Err("default_intent_changed"));
    assert_eq!(refined, copy);
    let mut changed = row;
    changed[20] = 1; // A byte outside parsed appearance still belongs to the exact row.
    assert_ne!(refined, Draft::ready(changed));
    assert_ne!(pending, Draft::native_default(MiiClass::Gunner));
    assert!(Draft::ready([0; 432]).validate().is_err());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("draft_refinement_preserves_error_order_and_independent_copies", draft_refinement_preserves_error_order_and_independent_copies as fn()),
    ]);
}
