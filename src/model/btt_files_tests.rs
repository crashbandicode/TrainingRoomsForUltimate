//! rc.14 Break the Targets, the saved layouts' files (btt-5, `artifacts/rc14-btt-layouts/DESIGN.md`): one file each in
//! `layouts/`, this stage's in Recording's order (the hot five, then by name), Save always a new file, the use stamps, the
//! Saved layouts list's frame and footer.

lib_unit_tests! {
use super::*;
use crate::model::frame::Frame;
use crate::engine::library::{Kind, RecordingIndex, Use};
use crate::model::menu::{Env, Menu};
use crate::model::recording_menu::SaveFile;
use crate::model::screens::hot_list_frame;
use crate::model::settings::Settings;

/// A fresh data folder for one case.
fn dir(case: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tr-btt5-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// This stage's listed names, in order, and how many are hot.
fn names(root: &Path, stage: i32) -> (Vec<String>, usize) {
    let (rows, hot) = listed(&all(root).0, stage);
    (rows.into_iter().map(|r| r.1.name).collect(), hot)
}

/// Save writes a new file every time, named `Layout N · <stage>` with the lowest free N on that stage; the file holds the
/// stage, the targets and its use; nothing already there is replaced.
fn layout_save_makes_a_new_file_never_overwrites() {
    let root = dir("save");
    assert_eq!(all(&root), (vec![], 0), "no folder: no layouts, nothing wrong");
    let first = save_new(&root, 0, "Battlefield", &[[-20.0, 15.0], [0.0, 15.0]]).unwrap();
    let bytes = std::fs::read(root.join(DIR).join(format!("{first}.json"))).unwrap();
    let second = save_new(&root, 0, "Battlefield", &[[-20.0, 15.0], [0.0, 15.0]]).unwrap();
    assert_ne!(first, second, "the same targets again: a second file");
    assert_eq!(std::fs::read(root.join(DIR).join(format!("{first}.json"))).unwrap(), bytes, "the first file is untouched");
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!((json["schema_version"].as_u64(), json["name"].as_str(), json["stage"].as_i64()), (Some(1), Some("Layout 1 · Battlefield"), Some(0)));
    assert_eq!((json["used"].as_u64(), json["targets"].to_string()), (Some(1), "[[-20.0,15.0],[0.0,15.0]]".to_owned()));
    assert_eq!(names(&root, 0), (vec!["Layout 2 · Battlefield".to_owned(), "Layout 1 · Battlefield".to_owned()], 2), "the newest at the top");
    let _ = std::fs::remove_dir_all(root);
}

/// The default name takes the lowest number no layout of that stage uses (as "Combo N"); other stages' names don't count.
fn layout_default_name_takes_the_lowest_free_number() {
    let root = dir("name");
    for _ in 0..3 {
        save_new(&root, 0, "Battlefield", &[[0.0, 15.0]]).unwrap();
    }
    let two = all(&root).0.into_iter().find(|r| r.1.name == "Layout 2 · Battlefield").unwrap().0;
    library(&root).delete(&two).unwrap();
    save_new(&root, 2, "Big Battlefield", &[[0.0, 15.0]]).unwrap();
    save_new(&root, 0, "Battlefield", &[[0.0, 15.0]]).unwrap();
    assert_eq!(names(&root, 0).0[0], "Layout 2 · Battlefield", "2 was free again");
    assert_eq!(names(&root, 2).0, ["Layout 1 · Big Battlefield"], "Big Battlefield counts its own");
    let _ = std::fs::remove_dir_all(root);
}

/// The list: the five used last, newest first (save, load, rename, duplicate are each a use), then the rest by name.
fn layouts_hot_five_newest_first_then_by_name() {
    let root = dir("hot");
    for _ in 0..7 {
        save_new(&root, 0, "Battlefield", &[[0.0, 15.0]]).unwrap();
    }
    let id = |name: &str| all(&root).0.into_iter().find(|r| r.1.name == name).unwrap().0;
    let n = |k: u32| format!("Layout {k} · Battlefield");
    assert_eq!(names(&root, 0), ([7, 6, 5, 4, 3, 1, 2].map(n).to_vec(), 5));
    use_file(&root, &id(&n(1)), None).unwrap();
    use_file(&root, &id(&n(2)), Some("Wide line")).unwrap();
    duplicate(&root, &id(&n(7)), "Battlefield").unwrap();
    assert_eq!(names(&root, 0).0, [n(2), "Wide line".into(), n(1), n(7), n(6), n(3), n(4), n(5)], "the copy took the free Layout 2");
    let _ = std::fs::remove_dir_all(root);
}

/// A sixth use pushes the oldest out of the five; its file stays, below the line, by name.
fn layouts_sixth_use_pushes_the_oldest_out_the_file_stays() {
    let root = dir("sixth");
    for _ in 0..6 {
        save_new(&root, 0, "Battlefield", &[[0.0, 15.0]]).unwrap();
    }
    let (rows, hot) = names(&root, 0);
    assert_eq!((hot, rows.len(), rows[5].as_str()), (5, 6, "Layout 1 · Battlefield"), "Layout 1, the oldest, left the five");
    assert_eq!(all(&root).0.len(), 6, "and its file stays");
    let _ = std::fs::remove_dir_all(root);
}

/// Only this stage's layouts are listed (and reach the chords' five); a file that can't be read is skipped and counted, and
/// the old per-stage slot files (`targets/<stage>.json`, never shipped) are neither read nor written.
fn layouts_list_only_this_stage() {
    let root = dir("stage");
    save_new(&root, 0, "Battlefield", &[[0.0, 15.0]]).unwrap();
    save_new(&root, 2, "Big Battlefield", &[[1.0, 15.0], [9.0, 15.0]]).unwrap();
    std::fs::create_dir_all(root.join("targets")).unwrap();
    std::fs::write(root.join("targets/0.json"), "{\"version\":1,\"stage\":0,\"slots\":[null,null,null,null,null]}").unwrap();
    std::fs::write(root.join(DIR).join("broken.json"), "{ not json").unwrap();
    let (every, skipped) = all(&root);
    assert_eq!((every.len(), skipped), (2, 1));
    assert_eq!(names(&root, 0).0, ["Layout 1 · Battlefield"]);
    let (rows, hot) = listed(&every, 2);
    let f = five(&rows, hot);
    assert_eq!((f[0].map(|h| (h.name.as_str().to_owned(), h.n)), f[1].is_none()), (Some(("Layout 1 · Big Battlefield".to_owned(), 2)), true));
    assert_eq!(f[0].unwrap().at[..2], [[1.0, 15.0], [9.0, 15.0]]);
    assert_eq!(std::fs::read_to_string(root.join("targets/0.json")).unwrap().len(), 58, "the old file is left alone");
    let _ = std::fs::remove_dir_all(root);
}

/// Rename keeps the file (its id) and moves it to the top; Duplicate makes a new file with the same targets; Delete removes the
/// file and the list closes up. A bad name or too many targets is refused before anything is written.
fn layout_rename_keeps_the_file_duplicate_makes_a_copy_delete_removes() {
    let root = dir("files");
    let id = save_new(&root, 0, "Battlefield", &[[3.0, 4.0]]).unwrap();
    save_new(&root, 0, "Battlefield", &[[5.0, 6.0]]).unwrap();
    use_file(&root, &id, Some("  Ledge trap ")).unwrap();
    let after = all(&root).0;
    assert!(after.iter().any(|r| r.0 == id && r.1.name == "Ledge trap"), "renamed in its own file");
    assert_eq!((after.len(), names(&root, 0).0[0].as_str()), (2, "Ledge trap"));
    let copy = duplicate(&root, &id, "Battlefield").unwrap();
    let copied = library(&root).load(&copy).unwrap();
    assert_eq!((copied.name.as_str(), copied.targets.clone(), copy != id), ("Layout 1 · Battlefield", vec![[3.0, 4.0]], true));
    library(&root).delete(&id).unwrap();
    assert_eq!(names(&root, 0).0, ["Layout 1 · Battlefield", "Layout 2 · Battlefield"]);
    assert!(use_file(&root, &copy, Some("")).is_err() && use_file(&root, &copy, Some(&"x".repeat(49))).is_err());
    assert!(library(&root).save_new(&Layout { schema_version: 1, name: "Many".into(), stage: 0, used: 1, targets: vec![[0.0, 0.0]; 11] }).is_err());
    assert_eq!(library(&root).load(&copy).unwrap().name, "Layout 1 · Battlefield", "a refused rename wrote nothing");
    let _ = std::fs::remove_dir_all(root);
}

/// "The same convention as before": the saved layouts and Recording's save states order through one table of use sequences
/// the same way (Recording's index and `ordered` against the layouts' use stamps and `listed`).
fn layouts_hot_order_is_recordings() {
    let ids = ["a-one", "b-two", "c-three", "d-four", "e-five", "f-six", "g-seven"];
    let cases: [&[usize]; 4] = [&[0, 1, 2], &[0, 1, 2, 3, 4, 5, 6], &[6, 5, 4, 3, 2, 1, 0, 3, 0], &[2, 2, 2, 4, 1, 6, 5, 0]];
    for uses in cases {
        let mut index = RecordingIndex::fresh();
        let mut layouts: Vec<(String, Layout)> = vec![];
        for (at, i) in uses.iter().enumerate() {
            index.note_use(Kind::Save, ids[*i], Use::Bound, at as u64 + 1).unwrap();
            let used = at as u64 + 1;
            match layouts.iter_mut().find(|l| l.0 == ids[*i]) {
                Some(l) => l.1.used = used,
                None => layouts.push((ids[*i].into(), Layout { schema_version: 1, name: ids[*i].into(), stage: 0, used, targets: vec![[0.0, 0.0]] })),
            }
        }
        let saves: Vec<SaveFile> = layouts.iter().map(|l| SaveFile { id: l.0.clone(), name: l.0.clone(), fighters: String::new() }).collect();
        let recording: Vec<String> = ordered(&saves, index.hot(Kind::Save), |f| &f.id, |f| &f.name).into_iter().map(|f| f.id.clone()).collect();
        let (rows, hot) = listed(&layouts, 0);
        assert_eq!(rows.into_iter().map(|r| r.0).collect::<Vec<_>>(), recording, "{uses:?}");
        assert_eq!(hot, index.hot(Kind::Save).len(), "{uses:?}");
    }
}

/// The list is drawn like the Recording lists: the hot five in the green outline, the hairline under the fifth when more
/// follow, the white cursor; scrolled past the five, neither shows. The footer names the layout under the cursor.
fn layouts_list_paints_the_outline_and_hairline() {
    let menu = Menu::new(Settings::factory(), Env::default());
    let rows = |n: usize| (0..n).map(|i| (format!("Layout {i}"), targets_label(i % 3 + 1))).collect::<Vec<_>>();
    let mut frame = Frame::new();
    hot_list_frame(&menu, "Saved layouts · Battlefield", "Layout 0 · 1 target\nA open   B back", &rows(7), 0, 5, &mut frame);
    assert_eq!((frame.outline[0], frame.hairline, frame.white_cursor, frame.focus, frame.row_count), (Some((0, 5)), Some(4), true, Some(0), 7));
    assert_eq!((frame.rows[1].label.string_lossy(), frame.rows[1].value.string_lossy()), ("Layout 1".to_owned(), "2 targets".to_owned()));
    assert_eq!((frame.help[0].string_lossy(), frame.help[1].string_lossy()), ("Layout 0 · 1 target".to_owned(), "A open   B back".to_owned()));
    assert!(frame.crumb.string_lossy().ends_with("Saved layouts · Battlefield"), "{}", frame.crumb.string_lossy());
    frame.reset();
    hot_list_frame(&menu, "Saved layouts", "", &rows(2), 1, 2, &mut frame);
    assert_eq!((frame.outline[0], frame.hairline, frame.focus), (Some((0, 2)), None, Some(1)), "two files: both hot, no line");
    frame.reset();
    hot_list_frame(&menu, "Saved layouts", "", &rows(30), 25, 5, &mut frame);
    assert_eq!((frame.outline[0], frame.hairline), (None, None), "scrolled past the five");
    frame.reset();
    hot_list_frame(&menu, "Saved layouts", "", &[], 0, 0, &mut frame);
    assert_eq!((frame.outline[0], frame.row_count, frame.focus), (None, 0, None));
}

/// The list's footer: the layout under the cursor and the keys; the files skipped, if any; with none saved, how to save one.
fn layouts_list_footer() {
    let layout = |name: &str, n: usize| (name.to_owned(), Layout { schema_version: 1, name: name.into(), stage: 0, used: 1, targets: vec![[0.0, 0.0]; n] });
    let rows = [layout("Ledge trap", 1), layout("Layout 1 · Battlefield", 6)];
    assert_eq!(list_footer(&rows, 1, 0), "Layout 1 · Battlefield · 6 targets\nA open   B back");
    assert_eq!(list_footer(&rows, 0, 2), "Ledge trap · 1 target\nA open   B back   2 files skipped");
    assert_eq!(list_footer(&rows, 0, 1), "Ledge trap · 1 target\nA open   B back   1 file skipped", "one file, one noun");
    assert_eq!(list_footer(&[], 0, 0), "No saved layouts on this stage yet. ZL + Down saves one while placing\nA open   B back");
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("layout_save_makes_a_new_file_never_overwrites", layout_save_makes_a_new_file_never_overwrites as fn()),
        ("layout_default_name_takes_the_lowest_free_number", layout_default_name_takes_the_lowest_free_number as fn()),
        ("layouts_hot_five_newest_first_then_by_name", layouts_hot_five_newest_first_then_by_name as fn()),
        ("layouts_sixth_use_pushes_the_oldest_out_the_file_stays", layouts_sixth_use_pushes_the_oldest_out_the_file_stays as fn()),
        ("layouts_list_only_this_stage", layouts_list_only_this_stage as fn()),
        ("layout_rename_keeps_the_file_duplicate_makes_a_copy_delete_removes", layout_rename_keeps_the_file_duplicate_makes_a_copy_delete_removes as fn()),
        ("layouts_hot_order_is_recordings", layouts_hot_order_is_recordings as fn()),
        ("layouts_list_paints_the_outline_and_hairline", layouts_list_paints_the_outline_and_hairline as fn()),
        ("layouts_list_footer", layouts_list_footer as fn()),
    ]);
}
}
