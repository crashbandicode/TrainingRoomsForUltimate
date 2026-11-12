//! Shareable combo files (`docs/COMBOS.md`, operator 2026-11-05): files people copy in, hand edits, unknown fields, bad
//! files, the order of the list, the recorded fighter, and the files rc.4 writes.

lib_unit_tests! {
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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_hand_written_file_without_version_or_name_is_listed_by_its_file_name", a_hand_written_file_without_version_or_name_is_listed_by_its_file_name as fn()),
        ("a_bad_file_is_skipped_with_its_name_and_reason_and_the_others_stay", a_bad_file_is_skipped_with_its_name_and_reason_and_the_others_stay as fn()),
    ]);
}
}
