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

fn scratch(tag: &str) -> crate::test_scratch::Scratch {
    crate::test_scratch::Scratch::new(&format!("tr-combo-{tag}"))
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

fn a_takes_name_is_the_prefix_and_sixteen_hex_digits() {
    let name = drill_take_name(0x1f);
    assert_eq!(name, "Drill take 000000000000001f");
    assert!(is_drill_take_name(&name));
    for invalid in ["Drill take", "Drill take 1f", "Drill take 000000000000001F",
        "Drill take 000000000000001g", "Macro 1"] {
        assert!(!is_drill_take_name(invalid));
    }
}

fn a_drills_take_is_kept_by_a_rename_and_dropped_by_a_crop() {
    let mut step = LinkRec::INPUT;
    step.events[0] = Some(Event::press(Button::Jump, Aim::Neutral, 0));
    let mut combo = Combo::from_links([step]).unwrap();
    combo.input_only = true;
    let drill = ComboRecord::new("Drill 1".into(), combo, None).with_take("drill-take");
    let value = drill.to_value();
    assert_eq!(value["take"], "drill-take");
    let mut renamed = ComboRecord::from_value(&value).unwrap();
    assert_eq!(renamed.take.as_deref(), Some("drill-take"));
    renamed.name = "Renamed".into();
    assert_eq!(renamed.to_value()["take"], "drill-take");
    assert_eq!(renamed.crop_units(0, 1).unwrap().take, None);
    assert!(record("Hit combo").to_value().get("take").is_none());
    for bad in [json!("../other"), json!(42), json!("")] {
        let mut invalid = value.clone();
        invalid["take"] = bad;
        assert_eq!(ComboRecord::from_value(&invalid).unwrap().take, None);
    }
}

/// Decision SP4: a link's spot is one optional field; an older file, and a word this build does not know, read as no spot.
fn a_links_spot_is_saved_and_an_older_file_reads_as_none() {
    use crate::model::spots::Spot;
    let plain = record("Nair loop");
    let mut links: Vec<LinkRec> = plain.combo.links().copied().collect();
    links[0].spot = Some(Spot::Sweet);
    links[1].spot = Some(Spot::Late);
    let rec = ComboRecord::new("Nair loop".to_owned(), Combo::from_links(links).unwrap(), None);
    let text = serde_json::to_string(&rec).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!((&value["links"][0]["spot"], &value["links"][1]["spot"]), (&json!("sweet"), &json!("late")));
    assert!(value["links"][2].get("spot").is_none(), "no spot, no field");
    let back: ComboRecord = serde_json::from_str(&text).unwrap();
    assert_eq!(back.combo.links().map(|l| l.spot).collect::<Vec<_>>(), [Some(Spot::Sweet), Some(Spot::Late), None]);
    let old: Value = serde_json::from_str(&serde_json::to_string(&plain).unwrap()).unwrap();
    assert!(old["links"].as_array().unwrap().iter().all(|l| l.get("spot").is_none()), "a combo saved without spots writes none");
    let back: ComboRecord = serde_json::from_value(old).unwrap();
    assert!(back.combo.links().all(|l| l.spot.is_none()), "an older file has none");
    let mut odd = value;
    odd["links"][0]["spot"] = json!("tip");
    let back: ComboRecord = serde_json::from_value(odd).unwrap();
    assert_eq!(back.combo.get(0).unwrap().spot, None, "an unknown word reads as none");
}

fn the_library_saves_lists_renames_deletes_and_selects() {
    let scratch = scratch("library");
    let dir = scratch.join("library");
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

fn the_links_are_read_straight_into_the_record() {
    let source = include_str!("combo_record.rs");
    let body = source.split_once("fn from_value(").unwrap().1.split_once("\nfn recorded_fighter(").unwrap().0;
    assert!(body.contains("combo.push(link)"));
    assert!(!body.contains("from_links("));
    assert!(!body.contains("collect::<"));
}

fn a_combo_file_keeps_its_load_rules() {
    let template = record("Rules").to_value()["links"][0].clone();
    let links: Vec<_> = (0_i32..16).map(|n| {
        let mut link = template.clone();
        link["window"] = json!([n, n + 1]);
        link
    }).collect();
    let combo_file = json!({"schema_version": 1, "name": "Rules", "links": links});
    let loaded: ComboRecord = serde_json::from_value(combo_file.clone()).unwrap();
    assert_eq!(loaded.combo.len(), 16);
    assert_eq!(loaded.combo.links().map(|link| link.window).collect::<Vec<_>>(),
        (0_i32..16).map(|n| Some((n, n + 1))).collect::<Vec<_>>());
    let round_trip: ComboRecord = serde_json::from_value(serde_json::to_value(&loaded).unwrap()).unwrap();
    assert_eq!(round_trip, loaded);

    let error = |value: Value| serde_json::from_value::<ComboRecord>(value).unwrap_err().to_string();
    let mut invalid = combo_file.clone();
    invalid["links"] = json!([]);
    assert!(error(invalid).contains("bad combo field: links"));
    let mut invalid = combo_file.clone();
    invalid["links"] = json!(vec![template.clone(); 17]);
    assert!(error(invalid.clone()).contains("bad combo field: links"));
    invalid["links"][0]["to"] = json!("not-a-move");
    assert!(error(invalid).contains("bad combo field: links"));
    for index in [0, 15] {
        let mut invalid = combo_file.clone();
        invalid["links"][index]["to"] = json!("not-a-move");
        assert!(error(invalid).contains("bad combo field: to"));
    }

    let press = |button: &str, offset: i32| json!({"kind": "press", "button": button, "dir": "neutral", "offset": offset});
    let drill = json!({
        "schema_version": 2, "kind": "input_drill", "name": "Rules drill",
        "input_steps": [
            {"events": [press("attack", 0), press("special", 5)]},
            {"events": [press("jump", 7)]}
        ]
    });
    let mut invalid = drill.clone();
    invalid["input_steps"] = json!([]);
    assert!(error(invalid).contains("bad combo field: input_steps"));
    let mut invalid = drill.clone();
    invalid["input_steps"][0]["events"] = json!([]);
    assert!(error(invalid).contains("bad combo field: events"));
    let mut invalid = drill.clone();
    invalid["input_steps"][0]["events"] = json!(vec![press("attack", 0); 13]);
    assert!(error(invalid).contains("bad combo field: events"));
    let mut invalid = drill.clone();
    invalid["input_steps"][1]["events"][0]["offset"] = json!(4);
    assert!(error(invalid).contains("bad combo field: offset"));
    let mut invalid = drill.clone();
    invalid["input_steps"][0]["events"][0]["offset"] = json!(1);
    assert!(error(invalid).contains("bad combo field: offset"));
    let loaded: ComboRecord = serde_json::from_value(drill).unwrap();
    assert!(loaded.combo.input_only);
    assert_eq!(loaded.combo.len(), 2);
    assert_eq!(loaded.combo.get(0).unwrap().events().map(|event| event.offset).collect::<Vec<_>>(), [0, 5]);
    assert_eq!(loaded.combo.get(1).unwrap().events().map(|event| event.offset).collect::<Vec<_>>(), [7]);
    let round_trip: ComboRecord = serde_json::from_value(serde_json::to_value(&loaded).unwrap()).unwrap();
    assert_eq!(round_trip, loaded);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_record_round_trips_through_its_json", a_record_round_trips_through_its_json as fn()),
        ("a_takes_name_is_the_prefix_and_sixteen_hex_digits", a_takes_name_is_the_prefix_and_sixteen_hex_digits as fn()),
        ("a_drills_take_is_kept_by_a_rename_and_dropped_by_a_crop", a_drills_take_is_kept_by_a_rename_and_dropped_by_a_crop as fn()),
        ("a_links_spot_is_saved_and_an_older_file_reads_as_none", a_links_spot_is_saved_and_an_older_file_reads_as_none as fn()),
        ("the_library_saves_lists_renames_deletes_and_selects", the_library_saves_lists_renames_deletes_and_selects as fn()),
        ("the_links_are_read_straight_into_the_record", the_links_are_read_straight_into_the_record as fn()),
        ("a_combo_file_keeps_its_load_rules", a_combo_file_keeps_its_load_rules as fn()),
    ]);
}
}
