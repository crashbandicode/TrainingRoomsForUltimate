//! Parry practice's saved-practice record (DESIGN §5).

lib_unit_tests! {
use super::*;

fn parry_saved_practice_round_trips_as_references() {
    use crate::engine::library::LoopCount;
    let p = Practice { schema_version: 1, name: "Fox nair OoS".into(), macro_id: "fox-nair-oos".into(), loop_count: LoopCount::Finite(10), set: 1, used: 12 };
    let json = serde_json::to_string(&p).unwrap();
    assert!(json.contains("\"macro\":\"fox-nair-oos\"") && json.contains("\"loop\":10"), "{json}");
    assert_eq!(serde_json::from_str::<Practice>(&json).unwrap(), p);
    assert!(<Practice as crate::model::store::Record>::validate(&p).is_ok());
}

fn parry_practice_file_is_small() {
    use crate::engine::library::LoopCount;
    let p = Practice { schema_version: 1, name: "A practice name of forty-eight characters, long!".into(), macro_id: "x".repeat(48), loop_count: LoopCount::Infinite, set: 2, used: u64::MAX };
    assert!(serde_json::to_vec(&p).unwrap().len() < 400, "references only, no copied frames or saves");
}

fn parry_saved_practice_with_deleted_start_save_is_unusable() {
    use crate::engine::library::StartSaveState;
    assert_eq!(check(true, StartSaveState::Deleted), Usable::StartSaveMissing);
    assert_eq!(check(true, StartSaveState::Linked), Usable::Yes);
    assert_eq!(check(true, StartSaveState::None), Usable::Yes, "no start save: plays where the fighters are (macro-start-save-missing)");
}

fn parry_saved_practice_with_deleted_macro_is_unusable() {
    use crate::engine::library::StartSaveState;
    assert_eq!(check(false, StartSaveState::Linked), Usable::MacroMissing);
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("parry_saved_practice_round_trips_as_references", parry_saved_practice_round_trips_as_references as fn()),
        ("parry_practice_file_is_small", parry_practice_file_is_small as fn()),
        ("parry_saved_practice_with_deleted_start_save_is_unusable", parry_saved_practice_with_deleted_start_save_is_unusable as fn()),
        ("parry_saved_practice_with_deleted_macro_is_unusable", parry_saved_practice_with_deleted_macro_is_unusable as fn()),
    ]);
}
}
