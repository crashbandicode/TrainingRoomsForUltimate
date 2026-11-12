//! Transaction and fail-closed tests for the carried `cycle_preload.ini` reader and writer (written for this repository; the
//! Astra review patterns R1 and R4 of `artifacts/trainingrooms-reviews/R1-astra-pkg1`, docs/PACKAGE-6-PLAN.md (removed after v0.2.0-rc.1; see that tag) section 7).
//!
//! The carried tests drive one load or one store at a time. These run the sequences a reboot makes: load, load again; a store
//! that fails half way and the load after it; the path left missing between the two renames of a replace. Every one goes through
//! the real file system under a fresh temporary directory, never through a hand-supplied result.

lib_unit_tests! {
use std::path::PathBuf;

use crate::engine::compat::menu::BASE_MENU;
use crate::engine::preload::cycle_preload_config::{
    adopt_unset, load_ini, load_ini_unset, render, startup_settings, store_ini, IniLoad, Settings, Unset, BANK_MEMBERS_MIN,
    MAX_BYTES,
};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("tr-preload-ini-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn path(&self, name: &str) -> String {
        self.child(name).to_str().unwrap().to_owned()
    }
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(&self.0).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn saved() -> Settings {
    Settings { slots: 2, fighters: true, hats: false, measurements: false, bank_members: BANK_MEMBERS_MIN, ..Settings::DEFAULT }
}

fn a_stored_choice_survives_two_loads_and_loading_never_writes() {
    let dir = Dir::new("two-load");
    let path = dir.path("cycle_preload.ini");
    store_ini(&path, saved()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()));
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()), "second boot");
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "a load does not rewrite the file");
    assert_eq!(dir.names(), ["cycle_preload.ini"], "no temporary or backup file is left");
}

fn a_first_boot_creates_the_default_once_and_the_second_boot_reads_it() {
    let dir = Dir::new("first-boot");
    let path = dir.path("cycle_preload.ini");
    assert_eq!(load_ini(&path), IniLoad::Created(Settings::DEFAULT));
    assert_eq!(load_ini(&path), IniLoad::Parsed(Settings::DEFAULT));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), render(Settings::DEFAULT));
    assert_eq!(dir.names(), ["cycle_preload.ini"]);
}

fn a_rejected_file_stays_rejected_and_untouched_on_every_boot_until_a_store_replaces_it() {
    let dir = Dir::new("rejected");
    let path = dir.path("cycle_preload.ini");
    let bad = "[cycle_preload]\nslots=9\nsources=both\nmeasurements=on\n";
    std::fs::write(&path, bad).unwrap();
    for boot in 1..=3 {
        let load = load_ini(&path);
        assert!(matches!(load, IniLoad::Rejected(_)), "boot {boot}: {load:?}");
        assert_eq!(startup_settings(load), Settings::FAIL_CLOSED, "boot {boot}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bad, "boot {boot}: the player's bytes are kept");
        assert_eq!(dir.names(), ["cycle_preload.ini"], "boot {boot}");
    }
    store_ini(&path, saved()).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()));
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()));
    assert_eq!(dir.names(), ["cycle_preload.ini"]);
}

fn a_failed_store_leaves_the_live_file_and_the_next_boot_reads_it() {
    let dir = Dir::new("failed-write");
    let path = dir.path("cycle_preload.ini");
    store_ini(&path, saved()).unwrap();
    let before = std::fs::read(&path).unwrap();
    // The temporary name is taken by a directory: opening it for writing fails, so no byte of the new choice is written.
    std::fs::create_dir(dir.child("cycle_preload.ini.tmp")).unwrap();
    let outcome = store_ini(&path, Settings::DEFAULT);
    assert_eq!(outcome, Err("could not write cycle preload ini"));
    assert_eq!(std::fs::read(&path).unwrap(), before, "the live file is the old choice, byte for byte");
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()));
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()), "second boot");
    assert!(!dir.child("cycle_preload.ini.previous").exists());
}

fn a_store_into_a_missing_directory_creates_it_and_a_store_that_cannot_is_an_error_not_a_panic() {
    let dir = Dir::new("directory");
    let nested = dir.path("a/b/cycle_preload.ini");
    store_ini(&nested, saved()).unwrap();
    assert_eq!(load_ini(&nested), IniLoad::Parsed(saved()));
    // A file where the directory should be.
    std::fs::write(dir.child("blocked"), b"x").unwrap();
    assert_eq!(store_ini(&dir.path("blocked/cycle_preload.ini"), saved()), Err("could not create cycle preload directory"));
    assert!(store_ini(&nested, Settings::CLOSED).is_err(), "a closed choice is never written");
    assert_eq!(load_ini(&nested), IniLoad::Parsed(saved()), "and the stored one is untouched");
}

fn a_replace_interrupted_between_its_renames_is_recovered_on_the_next_boot_and_stays_recovered() {
    let dir = Dir::new("interrupted");
    let path = dir.path("cycle_preload.ini");
    // `replace_live` moved the live file to `.previous` and stopped before the temporary file took its place.
    std::fs::write(format!("{path}.previous"), render(saved())).unwrap();
    std::fs::write(format!("{path}.tmp"), render(Settings::DEFAULT)).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()), "the previous choice comes back");
    assert_eq!(load_ini(&path), IniLoad::Parsed(saved()), "and is the active file on the boot after");
    assert!(dir.child("cycle_preload.ini").exists());
    assert!(!dir.child("cycle_preload.ini.previous").exists());
}

fn an_unreadable_previous_copy_fails_closed_on_every_boot() {
    let dir = Dir::new("bad-previous");
    let path = dir.path("cycle_preload.ini");
    std::fs::write(format!("{path}.previous"), b"[cycle_preload]\nslots=2\nslots=3\nsources=both\nmeasurements=on\n").unwrap();
    for boot in 1..=2 {
        let load = load_ini(&path);
        assert!(matches!(load, IniLoad::Rejected(_)), "boot {boot}: {load:?}");
        assert_eq!(startup_settings(load), Settings::FAIL_CLOSED);
    }
}

fn the_read_envelope_rejects_what_the_parser_alone_would_not_see() {
    let dir = Dir::new("envelope");
    let path = dir.path("cycle_preload.ini");
    let valid = render(saved());
    // An accepted prefix followed by an oversized tail (R4: the loader must not stop at the limit and accept the prefix).
    let oversized = format!("{valid}{}", "# padding\n".repeat(MAX_BYTES / 10 + 1));
    assert!(oversized.len() > MAX_BYTES);
    std::fs::write(&path, &oversized).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Rejected("cycle preload ini exceeds 2048 bytes"));
    // An accepted prefix followed by invalid UTF-8.
    let mut tail = valid.clone().into_bytes();
    tail.extend_from_slice(b"# \xff\xfe\n");
    std::fs::write(&path, &tail).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Rejected("cycle preload ini is not utf-8"));
    // A repeated key in either order.
    for body in ["slots=2\nslots=3\n", "slots=3\nslots=2\n"] {
        std::fs::write(&path, format!("[cycle_preload]\n{body}sources=both\nmeasurements=on\n")).unwrap();
        assert_eq!(load_ini(&path), IniLoad::Rejected("duplicate cycle preload key"), "{body:?}");
    }
    // A directory where the file should be, and a missing file under a path that is a file.
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert_eq!(load_ini(&path), IniLoad::Rejected("cycle preload ini is not a file"));
}

fn an_older_file_adopts_the_saved_menu_and_the_completed_file_is_stable_on_the_next_boot() {
    let dir = Dir::new("older");
    let path = dir.path("cycle_preload.ini");
    std::fs::write(&path, "[cycle_preload]\nslots=2\nsources=both\nmeasurements=on\n").unwrap();
    let (load, unset) = load_ini_unset(&path);
    let IniLoad::Parsed(parsed) = load else { panic!("{load:?}") };
    assert!(unset.any(), "an older file leaves the later keys unset");
    let mut menu = BASE_MENU;
    menu.cycle_preload_group_swap = crate::engine::compat::menu::OnOff::OFF;
    let adopted = adopt_unset(parsed, unset, &menu);
    assert!(!adopted.group_swap, "the saved menu's Off is honoured");
    store_ini(&path, adopted).unwrap();
    let (again, unset) = load_ini_unset(&path);
    assert_eq!(again, IniLoad::Parsed(adopted));
    assert_eq!(unset, Unset::NONE, "the completed file has every key");
    let (third, _) = load_ini_unset(&path);
    assert_eq!(third, IniLoad::Parsed(adopted), "third boot");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_stored_choice_survives_two_loads_and_loading_never_writes", a_stored_choice_survives_two_loads_and_loading_never_writes as fn()),
        ("a_first_boot_creates_the_default_once_and_the_second_boot_reads_it", a_first_boot_creates_the_default_once_and_the_second_boot_reads_it as fn()),
        ("a_rejected_file_stays_rejected_and_untouched_on_every_boot_until_a_store_replaces_it", a_rejected_file_stays_rejected_and_untouched_on_every_boot_until_a_store_replaces_it as fn()),
        ("a_failed_store_leaves_the_live_file_and_the_next_boot_reads_it", a_failed_store_leaves_the_live_file_and_the_next_boot_reads_it as fn()),
        ("a_store_into_a_missing_directory_creates_it_and_a_store_that_cannot_is_an_error_not_a_panic", a_store_into_a_missing_directory_creates_it_and_a_store_that_cannot_is_an_error_not_a_panic as fn()),
        ("a_replace_interrupted_between_its_renames_is_recovered_on_the_next_boot_and_stays_recovered", a_replace_interrupted_between_its_renames_is_recovered_on_the_next_boot_and_stays_recovered as fn()),
        ("an_unreadable_previous_copy_fails_closed_on_every_boot", an_unreadable_previous_copy_fails_closed_on_every_boot as fn()),
        ("the_read_envelope_rejects_what_the_parser_alone_would_not_see", the_read_envelope_rejects_what_the_parser_alone_would_not_see as fn()),
        ("an_older_file_adopts_the_saved_menu_and_the_completed_file_is_stable_on_the_next_boot", an_older_file_adopts_the_saved_menu_and_the_completed_file_is_stable_on_the_next_boot as fn()),
    ]);
}
}
