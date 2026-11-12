//! `cycle_preload.ini` as a boot cache of `settings.json` (decision D7 and the lead's ruling of 2026-11-02); written for this
//! repository, not carried.
//!
//! The fork kept the 13 `cycle_preload_*` rows in two places and let the INI overlay the menu at startup. Here `settings.json`
//! (through the typed view of `compat::menu`) is the only source of the active choice. The INI is regenerated from it: written when
//! it is missing, stale, unreadable or rejected, left alone when it already says the same, and never read back to change the
//! active choice. A first boot therefore writes the player's imported (or the factory) settings, not the engine's own defaults, and
//! a rejected or failed cache cannot turn the preload on or off by itself. The carried reader and the transactional writer
//! (`store_ini`: temporary file, rename, `.previous` recovery) do the file work.

use super::cycle_preload_config::{parse_unset, settings_from_menu, store_ini, Settings, MAX_BYTES};
use crate::engine::compat::menu::TrainingModpackMenu;

/// What the cache did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cache {
    /// The file already said the active choice.
    Unchanged,
    /// The file was missing, stale or rejected and now says the active choice.
    Written,
    /// The file could not be written (the reason); the active choice is unaffected and the next boot tries again.
    Failed(&'static str),
}

/// The active choice and what happened to the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Synced {
    pub active: Settings,
    pub cache: Cache,
}

/// The choice `settings.json` asks for. Rows that do not read as one choice each fail closed (preload off), the carried
/// `Settings::FAIL_CLOSED`, never the engine default.
pub fn desired(menu: &TrainingModpackMenu) -> Settings {
    settings_from_menu(menu).unwrap_or(Settings::FAIL_CLOSED)
}

/// Boot, and a menu apply: make the INI say what the settings say.
pub fn sync(path: &str, menu: &TrainingModpackMenu) -> Synced {
    let active = desired(menu);
    // Never invoke the legacy default-creating reader: even the first write must reflect JSON.
    // Invalid menu rows fail closed without touching the cache.
    let cache = if settings_from_menu(menu).is_none() || cached(path) == Some(active) {
        Cache::Unchanged
    } else {
        match store_ini(path, active) {
            Ok(()) => Cache::Written,
            Err(reason) => Cache::Failed(reason),
        }
    };
    Synced { active, cache }
}

fn cached(path: &str) -> Option<Settings> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    file.take((MAX_BYTES + 1) as u64).read_to_string(&mut text).ok()?;
    if text.len() > MAX_BYTES {
        return None;
    }
    let (settings, unset) = parse_unset(&text).ok()?;
    (!unset.any()).then_some(settings)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::compat::menu::{OnOff, BASE_MENU};
    use crate::engine::preload::cycle_preload_config::{load_ini, render, CyclePreloadSlots, IniLoad};
    use crate::model::settings::Snapshot;
    use std::path::PathBuf;

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("tr-preload-boot-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn path(&self) -> String {
            self.0.join("cycle_preload.ini").to_str().unwrap().to_owned()
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn imported() -> TrainingModpackMenu {
        let mut menu = BASE_MENU;
        menu.cycle_preload_slots = CyclePreloadSlots::TWO;
        menu.cycle_preload_group_swap = OnOff::OFF;
        menu.cycle_preload_measurements = OnOff::ON;
        menu
    }

    fn the_factory_menu_is_the_factory_snapshot_and_the_engine_default() {
        // The approved fresh-install defaults (factory table, MENU-DRAFT) are the engine's defaults: Solo Popo Off, Preload Samples Off.
        assert_eq!(BASE_MENU, TrainingModpackMenu::from_snapshot(&Snapshot::factory()));
        assert_eq!(desired(&BASE_MENU), Settings::DEFAULT);
        assert_eq!(BASE_MENU.solo_popo_original, OnOff::OFF);
        assert_eq!(BASE_MENU.cycle_preload_measurements, OnOff::OFF);
        let samples = Settings::DEFAULT.measurements;
        assert!(!samples);
    }

    fn a_first_boot_writes_the_players_settings_and_the_second_boot_leaves_the_file_alone() {
        let dir = Dir::new("first");
        let path = dir.path();
        let first = sync(&path, &imported());
        assert_eq!(first.active, desired(&imported()));
        assert_ne!(first.active, Settings::DEFAULT);
        assert_eq!(first.cache, Cache::Written);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), render(first.active), "written from settings.json, not the engine default");
        let bytes = std::fs::read(&path).unwrap();
        let second = sync(&path, &imported());
        assert_eq!((second.active, second.cache), (first.active, Cache::Unchanged));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    fn a_stale_cache_never_overrides_settings_json() {
        let dir = Dir::new("stale");
        let path = dir.path();
        // The cache says preload 5 with samples on; settings.json says 2 with group swap off.
        store_ini(&path, Settings { measurements: true, ..Settings::DEFAULT }).unwrap();
        let boot = sync(&path, &imported());
        assert_eq!(boot.active.slots, 2);
        assert!(!boot.active.group_swap);
        assert_eq!(boot.cache, Cache::Written);
        assert_eq!(load_ini(&path), IniLoad::Parsed(boot.active), "the cache now follows the settings");
    }

    fn missing_factory_cache_is_written_and_invalid_menu_does_not_create_one() {
        let dir = Dir::new("missing-factory");
        let path = dir.path();
        let mut invalid = BASE_MENU;
        invalid.cycle_preload_slots = CyclePreloadSlots::empty();
        assert_eq!(sync(&path, &invalid).active, Settings::FAIL_CLOSED);
        assert!(!std::path::Path::new(&path).exists());
        assert_eq!(sync(&path, &BASE_MENU).cache, Cache::Written);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), render(desired(&BASE_MENU)));
    }

    fn off_settings_survive_missing_stale_rejected_and_unwritable_caches() {
        let dir = Dir::new("off");
        let path = dir.path();
        let mut menu = BASE_MENU;
        super::super::cycle_preload_config::apply_settings(&mut menu, Settings::FAIL_CLOSED);
        for old in [None, Some(render(Settings::DEFAULT)), Some("invalid".into())] {
            let _ = std::fs::remove_file(&path);
            if let Some(old) = old { std::fs::write(&path, old).unwrap(); }
            assert_eq!(sync(&path, &menu).active, Settings::FAIL_CLOSED);
            assert_eq!(load_ini(&path), IniLoad::Parsed(Settings::FAIL_CLOSED));
        }
        store_ini(&path, Settings::DEFAULT).unwrap();
        std::fs::create_dir(format!("{path}.tmp")).unwrap();
        let result = sync(&path, &menu);
        assert_eq!(result.active, Settings::FAIL_CLOSED);
        assert!(matches!(result.cache, Cache::Failed(_)));
        assert_eq!(load_ini(&path), IniLoad::Parsed(Settings::DEFAULT));
    }

    fn a_rejected_cache_is_regenerated_and_cannot_change_the_active_choice() {
        let dir = Dir::new("rejected");
        let path = dir.path();
        std::fs::write(&path, "[cycle_preload]\nslots=9\n").unwrap();
        let boot = sync(&path, &BASE_MENU);
        assert_eq!(boot.active, Settings::DEFAULT, "not FAIL_CLOSED: the bad file is not the source");
        assert_eq!(boot.cache, Cache::Written);
        assert_eq!(load_ini(&path), IniLoad::Parsed(Settings::DEFAULT));
        assert_eq!(sync(&path, &BASE_MENU).cache, Cache::Unchanged);
    }

    fn a_cache_that_cannot_be_written_leaves_the_active_choice_and_the_old_file() {
        let dir = Dir::new("failed");
        let path = dir.path();
        store_ini(&path, Settings::DEFAULT).unwrap();
        let before = std::fs::read(&path).unwrap();
        std::fs::create_dir(format!("{path}.tmp")).unwrap();
        let boot = sync(&path, &imported());
        assert_eq!(boot.active, desired(&imported()));
        assert_eq!(boot.cache, Cache::Failed("could not write cycle preload ini"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    fn rows_that_are_not_one_choice_fail_closed_and_do_not_touch_the_file() {
        let dir = Dir::new("malformed");
        let path = dir.path();
        store_ini(&path, Settings::DEFAULT).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut menu = BASE_MENU;
        menu.cycle_preload_slots = CyclePreloadSlots::empty();
        let boot = sync(&path, &menu);
        assert_eq!(boot.active, Settings::FAIL_CLOSED);
        assert_eq!(boot.active.slots, 0);
        assert_eq!(boot.cache, Cache::Unchanged);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_factory_menu_is_the_factory_snapshot_and_the_engine_default", the_factory_menu_is_the_factory_snapshot_and_the_engine_default as fn()),
            ("a_first_boot_writes_the_players_settings_and_the_second_boot_leaves_the_file_alone", a_first_boot_writes_the_players_settings_and_the_second_boot_leaves_the_file_alone as fn()),
            ("a_stale_cache_never_overrides_settings_json", a_stale_cache_never_overrides_settings_json as fn()),
            ("missing_factory_cache_is_written_and_invalid_menu_does_not_create_one", missing_factory_cache_is_written_and_invalid_menu_does_not_create_one as fn()),
            ("off_settings_survive_missing_stale_rejected_and_unwritable_caches", off_settings_survive_missing_stale_rejected_and_unwritable_caches as fn()),
            ("a_rejected_cache_is_regenerated_and_cannot_change_the_active_choice", a_rejected_cache_is_regenerated_and_cannot_change_the_active_choice as fn()),
            ("a_cache_that_cannot_be_written_leaves_the_active_choice_and_the_old_file", a_cache_that_cannot_be_written_leaves_the_active_choice_and_the_old_file as fn()),
            ("rows_that_are_not_one_choice_fail_closed_and_do_not_touch_the_file", rows_that_are_not_one_choice_fail_closed_and_do_not_touch_the_file as fn()),
        ]);
    }
    }
}
