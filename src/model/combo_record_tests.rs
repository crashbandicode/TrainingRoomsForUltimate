//! SPEC-input-07 §4: the record's JSON, its validation, default names, and the store's delete and selection.

lib_unit_tests! {
use super::*;

fn link(from: Move, to: Move, verdict: Verdict) -> LinkRec {
    LinkRec::presses(
        from,
        to,
        verdict,
        Some((0, 4)),
        &[Event::press(Button::Jump, Aim::Neutral, 2), Event::press(Button::Attack, Aim::Neutral, -1)],
    )
}

fn record(name: &str) -> ComboRecord {
    let combo = Combo::from_links([
        link(Move::Aerial, Move::Aerial, Verdict::True(Some(-2))),
        link(Move::Aerial, Move::Aerial, Verdict::True(None)),
        link(Move::Aerial, Move::UpSmash, Verdict::Drop(1)),
    ])
    .unwrap();
    ComboRecord::new(name.to_owned(), combo, None)
}

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tr-combo-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn a_record_round_trips_through_its_json() {
    let rec = record("Nair loop");
    let text = serde_json::to_string(&rec).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["links"][0]["from"], "aerial");
    assert_eq!(value["links"][0]["verdict"], json!({"kind": "true", "link": -2}));
    assert_eq!(value["links"][1]["verdict"], json!({"kind": "true", "link": null}));
    assert_eq!(value["links"][2]["verdict"], json!({"kind": "drop", "link": 1}));
    assert_eq!(value["links"][0]["steps"][0], json!({"button": "jump", "aim": "neutral", "offset": 2}));
    assert_eq!(value["links"][0]["window"], json!([0, 4]));
    let back: ComboRecord = serde_json::from_str(&text).unwrap();
    assert_eq!(back, rec);
}

fn the_library_saves_lists_renames_deletes_and_selects() {
    let dir = scratch("library");
    let library = ComboLibrary::new(&dir);
    assert_eq!(library.selected(), None);
    let id = library.save_new(&record("Nair loop")).unwrap();
    assert_eq!(id, "nair-loop");
    let second = library.save_new(&record("Nair loop")).unwrap();
    assert_eq!(second, "nair-loop-2");
    // Rename rewrites only the name; the id stays.
    let mut renamed = library.load(&second).unwrap();
    renamed.name = "Up air chain".into();
    library.overwrite(&second, &renamed).unwrap();
    let listing = library.list().unwrap();
    assert_eq!(listing.entries.iter().map(|e| (e.id.as_str(), e.name.as_str(), e.links)).collect::<Vec<_>>(),
        [("nair-loop", "Nair loop", 3), ("nair-loop-2", "Up air chain", 3)]);
    library.select(Some(&second)).unwrap();
    assert_eq!(library.selected().as_deref(), Some("nair-loop-2"));
    assert_eq!(std::fs::read_to_string(dir.join("selected.txt")).unwrap(), "nair-loop-2");
    library.delete(&second).unwrap();
    assert!(matches!(library.load(&second), Err(ComboError::Store(StoreError::NotFound(_)))));
    assert!(matches!(library.delete(&second), Err(ComboError::Store(StoreError::NotFound(_)))));
    library.select(None).unwrap();
    library.select(None).unwrap();
    assert_eq!(library.selected(), None);
    assert!(library.select(Some("../escape")).is_err());
    assert_eq!(library.list().unwrap().entries.len(), 1, "selected.txt is not a record");
    let _ = std::fs::remove_dir_all(&dir);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_record_round_trips_through_its_json", a_record_round_trips_through_its_json as fn()),
        ("the_library_saves_lists_renames_deletes_and_selects", the_library_saves_lists_renames_deletes_and_selects as fn()),
    ]);
}
}
