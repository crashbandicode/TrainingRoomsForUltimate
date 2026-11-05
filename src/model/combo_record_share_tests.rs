//! Shareable combo files (`docs/COMBOS.md`, operator 2026-11-05): files people copy in, hand edits, unknown fields, bad
//! files, the order of the list, the recorded fighter, and the files rc.4 writes.

use super::*;

/// A folder of its own under the system temp dir, empty.
fn folder(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tr-combo-share-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &std::path::Path, file: &str, text: &str) {
    std::fs::write(dir.join(file), text).unwrap();
}

/// One link as a person would type it: a jab into an up tilt, a true combo, one press.
const LINK: &str = r#"{"from": "jab", "to": "up_tilt", "verdict": {"kind": "true", "link": -3}, "window": [-2, 6],
    "steps": [{"button": "attack", "aim": "up", "offset": 1}]}"#;

/// The exact text rc.4 (`p23/rc4-candidate` `ff69e53`) writes for a saved combo: compact JSON, its keys in its order.
const RC4_FILE: &str = r#"{"links":[{"from":"aerial","steps":[{"aim":"neutral","button":"jump","offset":2},{"aim":"neutral","button":"attack","offset":-1}],"to":"aerial","verdict":{"kind":"true","link":-2},"window":[0,4]},{"from":"aerial","steps":[],"to":"up_smash","verdict":{"kind":"drop","link":1},"window":null}],"name":"Nair loop","schema_version":1}"#;

/// rc.4's reader, kept as it was (`combo_record.rs` at `ff69e53`): `schema_version` exactly 1, `name` a string, 1 to 16
/// links. Every other field it ignores.
fn rc4_reads(value: &Value) -> bool {
    value.get("schema_version").and_then(Value::as_u64) == Some(1)
        && value.get("name").and_then(Value::as_str).is_some()
        && value.get("links").and_then(Value::as_array).is_some_and(|links| (1..=LINKS).contains(&links.len()))
}

#[test]
fn a_file_rc4_wrote_loads_and_a_file_written_now_is_one_rc4_reads() {
    let dir = folder("migration");
    write(&dir, "nair-loop.json", RC4_FILE);
    let library = ComboLibrary::new(&dir);
    let loaded = library.load("nair-loop").unwrap();
    assert_eq!((loaded.name.as_str(), loaded.combo.len(), loaded.recorded_on.as_deref()), ("Nair loop", 2, None));
    // Written now (with the new optional field): still schema 1, and rc.4's reader accepts it.
    let id = library.save_new(&ComboRecord::new("Saved now".into(), loaded.combo, Some("Pikachu".into()))).unwrap();
    let written: Value = serde_json::from_slice(&std::fs::read(dir.join(format!("{id}.json"))).unwrap()).unwrap();
    assert_eq!((written["schema_version"].clone(), written["recorded_on"].clone()), (json!(1), json!("Pikachu")));
    assert!(rc4_reads(&written));
    assert_eq!(library.load(&id).unwrap().combo, loaded.combo, "the links round-trip");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hand_written_file_without_version_or_name_is_listed_by_its_file_name() {
    let dir = folder("hand");
    write(&dir, "Ryu BnB.json", &format!(r#"{{"links": [{LINK}]}}"#));
    let listing = ComboLibrary::new(&dir).list().unwrap();
    assert!(listing.skipped.is_empty(), "{:?}", listing.skipped);
    let [entry] = &listing.entries[..] else { panic!("one entry: {:?}", listing.entries) };
    assert_eq!((entry.id.as_str(), entry.name.as_str(), entry.links), ("Ryu BnB", "Ryu BnB", 1));
    assert!(entry.shared(), "a name the plugin did not choose");
    assert_eq!(entry.label(), "Ryu BnB · 1 links");
    // It can be the target: selected.txt names it.
    let library = ComboLibrary::new(&dir);
    library.select(Some("Ryu BnB")).unwrap();
    assert_eq!(library.selected().as_deref(), Some("Ryu BnB"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_fields_are_ignored_and_a_rename_keeps_them() {
    let dir = folder("unknown");
    let text = format!(
        r#"{{"schema_version": 1, "name": "Shared", "author": "a friend", "recorded_on": "Ryu",
            "links": [{}]}}"#,
        LINK.replace(r#""window""#, r#""note": "buffer the up tilt", "window""#)
    );
    write(&dir, "shared.json", &text);
    let library = ComboLibrary::new(&dir);
    let mut record = library.load("shared").unwrap();
    assert_eq!((record.name.as_str(), record.recorded_on.as_deref()), ("Shared", Some("Ryu")));
    // The user renames it (the list's X): only the name changes in the file.
    record.name = "Ryu jab chain".into();
    library.overwrite("shared", &record).unwrap();
    let after: Value = serde_json::from_slice(&std::fs::read(dir.join("shared.json")).unwrap()).unwrap();
    assert_eq!(after["name"], "Ryu jab chain");
    assert_eq!(after["author"], "a friend");
    assert_eq!(after["recorded_on"], "Ryu");
    assert_eq!(after["links"][0]["note"], "buffer the up tilt");
    assert_eq!(after["schema_version"], 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_bad_file_is_skipped_with_its_name_and_reason_and_the_others_stay() {
    let dir = folder("bad");
    write(&dir, "good.json", &format!(r#"{{"name": "Good", "links": [{LINK}]}}"#));
    write(&dir, "typo.json", &format!(r#"{{"name": "Typo", "links": [{}]}}"#, LINK.replace("up_tilt", "uptilt")));
    write(&dir, "newer.json", &format!(r#"{{"schema_version": 2, "name": "Newer", "links": [{LINK}]}}"#));
    write(&dir, "broken.json", "{\"name\": \"Broken\", \"links\": [");
    write(&dir, "notes.txt", "not a combo");
    write(&dir, "._good.json", "\0\x05\x16\x07 macOS resource fork");
    let listing = ComboLibrary::new(&dir).list().unwrap();
    assert_eq!(listing.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Good"]);
    let reasons: Vec<(&str, &str)> = listing.skipped.iter().map(|s| (s.file.as_str(), s.reason.as_str())).collect();
    assert_eq!(reasons.len(), 3, "{reasons:?}");
    assert!(reasons.contains(&("typo.json", "malformed file: bad combo field: to")), "the text docs/COMBOS.md shows: {reasons:?}");
    assert!(reasons.iter().any(|(f, r)| *f == "newer.json" && r.contains("schema version 2")), "{reasons:?}");
    assert!(reasons.iter().any(|(f, r)| *f == "broken.json" && r.contains("malformed")), "{reasons:?}");
    // One notice: the first file, its reason, and how many more.
    let note = skipped_note(&listing.skipped).unwrap();
    assert!(note.starts_with("broken.json skipped: malformed"), "{note}");
    assert!(note.ends_with(" (+2 more)"), "{note}");
    assert_eq!(skipped_note(&[]), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_of_dropped_in_files_is_listed_in_name_order() {
    let dir = folder("order");
    for (file, name) in [("b.json", "banana"), ("Zed Combo.json", ""), ("combo-1.json", "Combo 1"), ("x.json", "apple")] {
        let name = if name.is_empty() { String::new() } else { format!(r#""name": "{name}", "#) };
        write(&dir, file, &format!(r#"{{{name}"links": [{LINK}]}}"#));
    }
    let listing = ComboLibrary::new(&dir).list().unwrap();
    let rows: Vec<(&str, &str, bool)> = listing.entries.iter().map(|e| (e.id.as_str(), e.name.as_str(), e.shared())).collect();
    assert_eq!(rows, [("x", "apple", false), ("b", "banana", false), ("combo-1", "Combo 1", false), ("Zed Combo", "Zed Combo", true)]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_never_replaces_a_file_someone_copied_in() {
    let dir = folder("never-overwrite");
    write(&dir, "combo-1.json", &format!(r#"{{"name": "Someone's", "links": [{LINK}]}}"#));
    let library = ComboLibrary::new(&dir);
    let combo = library.load("combo-1").unwrap().combo;
    let id = library.save_new(&ComboRecord::new("Combo 1".into(), combo, None)).unwrap();
    assert_eq!(id, "combo-1-2");
    assert_eq!(library.load("combo-1").unwrap().name, "Someone's");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_recorded_fighter_is_written_and_shown_and_names_are_tidied() {
    let combo = Combo::from_links([link_of(&serde_json::from_str(LINK).unwrap()).unwrap()]).unwrap();
    let saved = serde_json::to_value(ComboRecord::new("Jab chain".into(), combo, Some("Mario".into()))).unwrap();
    assert_eq!(saved["recorded_on"], "Mario");
    let entry = serde_json::from_value::<ComboRecord>(saved).unwrap().summary("jab-chain");
    assert_eq!(entry.label(), "Jab chain · 1 links · recorded on Mario");
    // A name with a tab and spaces around it, a too-long one, a fighter that is not text.
    let typed = format!(r#"{{"name": "  Tab\there  ", "recorded_on": 7, "links": [{LINK}]}}"#);
    let record = serde_json::from_str::<ComboRecord>(&typed).unwrap();
    assert_eq!((record.name.as_str(), record.recorded_on), ("Tabhere", None));
    let long = format!(r#"{{"name": "{}", "links": [{LINK}]}}"#, "n".repeat(80));
    assert_eq!(serde_json::from_str::<ComboRecord>(&long).unwrap().name.chars().count(), MAX_NAME_CHARS);
}

#[test]
fn shared_file_names_are_any_safe_name() {
    for ok in ["Ryu BnB", "combo-1", "Pikachu (nair loop)", "é", &"x".repeat(MAX_FILE_NAME_CHARS)] {
        assert!(shareable_id(ok), "{ok}");
    }
    for bad in ["", ".hidden", " padded", "padded ", "a/b", "a\\b", "c:d", "a*b", "a?b", "tab\there", &"x".repeat(MAX_FILE_NAME_CHARS + 1)] {
        assert!(!shareable_id(bad), "{bad:?}");
    }
}
