//! Review R1 of package 1: recovery of a partially bad `settings.json` is a boot-time storage
//! transaction. These tests run `model::settings_file::load` (the code the runtime calls) on real
//! files in a temporary directory: two boots in a row, the backup, the write-failure paths and the
//! fork's old file name.

use std::fs;
use std::path::{Path, PathBuf};

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::settings::{self, Settings};
use model::settings_file::{self, BACKUP_NAME, TEMP_NAME};
use model::storage::{self, file};
use serde_json::{json, Value};

/// A fresh empty folder for one test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tr-recovery-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn one_hot(len: usize, at: usize) -> Vec<u8> {
    (0..len).map(|i| u8::from(i == at)).collect()
}

/// The player's settings: Input Delay 5 and Hazards On in the menu, Input Delay 3 in the saved
/// defaults, rows and top-level keys this build does not know.
fn players_settings() -> Settings {
    let mut s = Settings::factory();
    s.menu.set("input_delay", &one_hot(31, 5)).unwrap();
    s.menu.set("stage_hazards", &[1, 0]).unwrap();
    s.defaults.set("input_delay", &one_hot(31, 3)).unwrap();
    s.extra.insert("future_top_level_key".to_owned(), json!({"kept": [1, 2, 3]}));
    s.set_last_page("saves");
    s
}

/// `players_settings` as a file in which two things were damaged: `lra_reset` is not an array, and
/// an unknown row inside `menu` is present and must survive.
fn damaged_file() -> Vec<u8> {
    let mut value: Value = serde_json::from_str(&players_settings().to_json()).unwrap();
    value["menu"]["lra_reset"] = json!("bad");
    value["menu"]["future_row_of_a_later_build"] = json!([9, 9]);
    serde_json::to_vec_pretty(&value).unwrap()
}

fn read(dir: &Path, name: &str) -> Vec<u8> {
    fs::read(dir.join(name)).unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut all: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    all.sort();
    all
}

/// What boot 1 recovers: the valid nondefault fields, the saved defaults and the unknown keys, with
/// the bad field at its factory value.
fn check_recovered(s: &Settings) {
    assert_eq!(s.menu.input_delay(), 5);
    assert!(s.menu.stage_hazards());
    assert_eq!(s.menu.get("lra_reset"), Settings::factory().menu.get("lra_reset"), "the bad field is at its factory value");
    assert_eq!(s.defaults.input_delay(), 3, "saved defaults survive");
    assert_eq!(s.menu.extra().get("future_row_of_a_later_build"), Some(&json!([9, 9])));
    assert_eq!(s.extra.get("future_top_level_key"), Some(&json!({"kept": [1, 2, 3]})));
    assert_eq!(s.last_page(), Some("saves"));
}

#[test]
fn core04_recovery_two_boots_keep_the_recovered_settings_and_the_backup() {
    let dir = folder("two-boots");
    let original = damaged_file();
    fs::write(dir.join(file::SETTINGS), &original).unwrap();

    // Boot 1: recovers field by field, keeps the original untouched, publishes the recovery.
    let boot1 = settings_file::load(&dir);
    check_recovered(&boot1.settings);
    assert!(boot1.notes.iter().any(|n| n.contains("at=menu.lra_reset")), "{:?}", boot1.notes);
    assert!(boot1.notes.iter().any(|n| n.contains("settings recovered")), "{:?}", boot1.notes);
    assert_eq!(read(&dir, "settings.invalid-1.json"), original, "the backup is the untouched original");
    assert_eq!(names(&dir), ["settings.invalid-1.json", "settings.json"], "no temp file is left");

    // Boot 2: the active file now reads cleanly and gives the same settings; nothing new is kept.
    let boot2 = settings::parse_bytes(&read(&dir, file::SETTINGS));
    assert!(boot2.issues.is_empty(), "{:?}", boot2.issues);
    let again = settings_file::load(&dir);
    assert!(again.notes.is_empty(), "{:?}", again.notes);
    assert_eq!(again.settings, boot1.settings);
    check_recovered(&again.settings);
    assert_eq!(read(&dir, "settings.invalid-1.json"), original);
    assert_eq!(names(&dir), ["settings.invalid-1.json", "settings.json"]);

    // Boot 3 (still clean): the same again.
    assert_eq!(settings_file::load(&dir).settings, boot1.settings);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_recovery_of_a_file_that_is_not_json_keeps_it_and_writes_a_valid_one() {
    let dir = folder("not-json");
    fs::write(dir.join(file::SETTINGS), b"{ not json").unwrap();
    let boot = settings_file::load(&dir);
    assert_eq!(boot.settings, Settings::factory());
    assert_eq!(read(&dir, "settings.invalid-1.json"), b"{ not json");
    assert!(settings::parse_bytes(&read(&dir, file::SETTINGS)).issues.is_empty());
    // A second damaged file gets the next number, never the first one's.
    fs::write(dir.join(file::SETTINGS), [0xff, 0xfe, 0x00]).unwrap();
    settings_file::load(&dir);
    assert_eq!(read(&dir, "settings.invalid-1.json"), b"{ not json");
    assert_eq!(read(&dir, "settings.invalid-2.json"), [0xff, 0xfe, 0x00]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_recovery_write_failure_leaves_the_original_active_and_logs_it() {
    let dir = folder("write-failure");
    let original = damaged_file();
    fs::write(dir.join(file::SETTINGS), &original).unwrap();
    // Make the sanitized file impossible to write: its temp name is a directory.
    fs::create_dir(dir.join(TEMP_NAME)).unwrap();

    let boot = settings_file::load(&dir);
    check_recovered(&boot.settings); // this boot still uses the recovered settings
    assert!(boot.notes.iter().any(|n| n.contains("recovery abandoned") && n.contains("could not write settings.json")), "{:?}", boot.notes);
    assert_eq!(read(&dir, file::SETTINGS), original, "the original stays active, byte for byte");
    assert!(!names(&dir).iter().any(|n| n.starts_with("settings.invalid-")), "no backup without a replacement");

    // Next boot with the obstacle gone: the recovery completes, one backup only.
    fs::remove_dir(dir.join(TEMP_NAME)).unwrap();
    let boot = settings_file::load(&dir);
    check_recovered(&boot.settings);
    assert_eq!(read(&dir, "settings.invalid-1.json"), original);
    assert_eq!(settings_file::load(&dir).settings, boot.settings);
    assert_eq!(names(&dir), ["settings.invalid-1.json", "settings.json"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_recovery_without_a_free_backup_name_keeps_the_original_active() {
    let dir = folder("no-backup");
    let original = damaged_file();
    fs::write(dir.join(file::SETTINGS), &original).unwrap();
    for n in 1..=storage::INVALID_COPIES_MAX {
        fs::write(dir.join(storage::invalid_copy_name(n).unwrap().as_str()), format!("old {n}")).unwrap();
    }
    let boot = settings_file::load(&dir);
    check_recovered(&boot.settings);
    assert!(boot.notes.iter().any(|n| n.contains("recovery abandoned")), "{:?}", boot.notes);
    assert_eq!(read(&dir, file::SETTINGS), original);
    assert!(!dir.join(TEMP_NAME).exists(), "the temp file is cleaned up");
    assert_eq!(read(&dir, "settings.invalid-7.json"), b"old 7", "existing backups are never overwritten");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_recovery_of_the_forks_old_file_name_retires_it_after_the_write() {
    let dir = folder("legacy");
    let original = damaged_file();
    fs::write(dir.join(file::LEGACY_SETTINGS), &original).unwrap();
    let boot = settings_file::load(&dir);
    check_recovered(&boot.settings);
    assert!(!dir.join(file::LEGACY_SETTINGS).exists(), "retired after settings.json was written");
    assert_eq!(read(&dir, "settings.invalid-1.json"), original);
    assert!(settings::parse_bytes(&read(&dir, file::SETTINGS)).issues.is_empty());
    assert_eq!(settings_file::load(&dir).settings, boot.settings);

    // A clean old-name file is read, not rewritten or retired (the next save writes settings.json).
    let dir2 = folder("legacy-clean");
    fs::write(dir2.join(file::LEGACY_SETTINGS), players_settings().to_json()).unwrap();
    let boot = settings_file::load(&dir2);
    assert!(boot.notes.is_empty());
    assert_eq!(boot.settings, players_settings());
    assert_eq!(names(&dir2), [file::LEGACY_SETTINGS]);
    fs::remove_dir_all(&dir).unwrap();
    fs::remove_dir_all(&dir2).unwrap();
}

#[test]
fn core04_a_clean_file_and_a_missing_file_are_left_alone() {
    let dir = folder("clean");
    assert_eq!(settings_file::load(&dir).settings, Settings::factory());
    assert!(names(&dir).is_empty());
    fs::write(dir.join(file::SETTINGS), players_settings().to_json()).unwrap();
    let boot = settings_file::load(&dir);
    assert!(boot.notes.is_empty());
    assert_eq!(boot.settings, players_settings());
    assert_eq!(names(&dir), [file::SETTINGS]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_an_interrupted_replace_is_restored_before_loading() {
    // A crash inside the replace fallback leaves settings.json.bak and no settings.json.
    let dir = folder("orphan");
    fs::write(dir.join(BACKUP_NAME), players_settings().to_json()).unwrap();
    let boot = settings_file::load(&dir);
    assert_eq!(boot.settings, players_settings());
    assert!(dir.join(file::SETTINGS).exists() && !dir.join(BACKUP_NAME).exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn core04_save_publishes_through_the_same_replace() {
    let dir = folder("save");
    settings_file::save(&dir, &players_settings()).unwrap();
    settings_file::save(&dir, &players_settings()).unwrap();
    assert_eq!(names(&dir), [file::SETTINGS]);
    assert_eq!(settings_file::load(&dir).settings, players_settings());
    fs::remove_dir_all(&dir).unwrap();
}

/// Invalid settings must still boot when the directory denies deletion and recovery writes.
/// This exercises the production filesystem path, not a mocked successful cleanup.
#[cfg(unix)]
#[test]
fn core04_invalid_undeletable_settings_defaults_once_without_writing_elsewhere() {
    use std::os::unix::fs::PermissionsExt;
    struct WritableOnDrop(PathBuf, fs::Permissions);
    impl Drop for WritableOnDrop {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, self.1.clone());
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let parent = folder("invalid-undeletable");
    let dir = parent.join("data");
    fs::create_dir(&dir).unwrap();
    let original = b"{ invalid settings";
    let active = dir.join(file::SETTINGS);
    fs::write(&active, original).unwrap();
    let restore = WritableOnDrop(dir.clone(), fs::metadata(&dir).unwrap().permissions());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let error = fs::remove_file(&active).expect_err("fixture must actually deny deletion");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    for _ in 0..2 {
        let boot = settings_file::load(&dir);
        assert_eq!(boot.settings, Settings::factory());
        assert_eq!(boot.notes.len(), 1, "one boot diagnostic, no cleanup retry spam: {:?}", boot.notes);
        assert!(boot.notes[0].contains("recovery abandoned"));
        assert!(boot.notes[0].contains("using factory settings"));
        assert_eq!(read(&dir, file::SETTINGS), original);
        assert_eq!(names(&dir), [file::SETTINGS]);
        assert_eq!(names(&parent), ["data"], "never redirect recovery to a writable parent");
    }
    drop(restore);
    fs::remove_dir(parent).unwrap();
}
