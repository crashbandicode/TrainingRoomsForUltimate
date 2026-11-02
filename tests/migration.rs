//! SPEC-core-08 (first-boot import), the pure plan. Test paths are the spec's §9 rows
//! (`migration::…`). Not here: the item-on-load conversion (N6), receipt hashes, the copy loop.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

mod migration {
    use super::model::migration::{self as m, Action, ImportResult, Outcome, Part, Plan, Skip, Source};
    use super::model::settings::{self, Settings};
    use super::model::storage;

    fn src(rel: &'static str, bytes: u64) -> Source<'static> {
        Source { rel, bytes }
    }

    fn listing() -> Vec<Source<'static>> {
        vec![
            src("training_modpack_menu.conf", 22_000),
            src("training_modpack.json", 72),
            src("button_config.ini", 40),
            src("replay_capture.ini", 20),
            src("save_states.json", 5_000),
            src("mii-presets.json", 900),
            src("setups/a.json", 300),
            src("setups/b.json", 300),
            src("macros/m1.json", 1_000),
            src("replay_captures/c1.smd", 3_000),
            src("cycle_preload.ini", 100),
            src("practice-diagnostics.log", 100_000),
            src("practice-diagnostics.previous.log", 100_000),
            src("version.txt", 5),
            src("dev.json", 2),
            src("layout.arc", 1_000),
            src("training_modpack_menu_defaults.conf", 10),
            src("setups/nested/x.json", 1),
        ]
    }

    fn copies<'a>(plan: &'a Plan<'a>) -> &'a [Action<'a>] {
        match plan {
            Plan::Copy(actions) => actions,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plan_copies_listed_files_only() {
        let sources = listing();
        let plan = m::plan(false, Some(&sources), |_| false);
        let actions = copies(&plan);
        let pairs: Vec<_> = actions.iter().map(|a| (a.from, a.to)).collect();
        assert_eq!(
            pairs,
            [
                ("training_modpack_menu.conf", "settings.json"),
                ("button_config.ini", "button_config.ini"),
                ("replay_capture.ini", "replay_capture.ini"),
                ("save_states.json", "save_states.json"),
                ("mii-presets.json", "mii-presets.json"),
                ("setups/a.json", "setups/a.json"),
                ("setups/b.json", "setups/b.json"),
                ("macros/m1.json", "macros/m1.json"),
                ("replay_captures/c1.smd", "replay_captures/c1.smd"),
            ]
        );
        assert!(actions.iter().all(|a| a.skip.is_none()));
    }

    #[test]
    fn plan_skips_existing_and_excluded() {
        let sources = listing();
        let plan = m::plan(false, Some(&sources), |to| to == "settings.json" || to == "setups/b.json");
        let skipped: Vec<_> = copies(&plan).iter().filter(|a| a.skip == Some(Skip::Exists)).map(|a| a.to).collect();
        assert_eq!(skipped, ["settings.json", "setups/b.json"]);
        // Never imported: the fork's updater record, diagnostics, version stamp, dev inputs, caches,
        // the legacy formats, nested folders.
        for excluded in [
            "training_modpack.json",
            "cycle_preload.ini",
            "practice-diagnostics.log",
            "practice-diagnostics.previous.log",
            "version.txt",
            "dev.json",
            "layout.arc",
            "training_modpack_menu_defaults.conf",
            "setups/nested/x.json",
            "developer.ini",
            "migration.json",
        ] {
            assert!(m::route(excluded).is_none(), "{excluded}");
        }
    }

    #[test]
    fn marker_blocks_rerun() {
        let sources = listing();
        assert_eq!(m::plan(true, Some(&sources), |_| false), Plan::MarkerPresent);
    }

    #[test]
    fn only_the_marker_means_already_imported() {
        // Operator rule 2026-11-02: a pre-installed developer.ini (or any other file in the new
        // root) must not stop the import. The plan sees only the marker flag and the destinations of
        // routed files, and `developer.ini` is neither.
        let sources = listing();
        let present = |name: &str| name == storage::file::DEVELOPER_INI;
        let plan = m::plan(present(storage::file::MIGRATION), Some(&sources), present);
        assert_eq!(copies(&plan).len(), 9);
        assert!(copies(&plan).iter().all(|a| a.skip.is_none()));
        assert!(m::route(storage::file::DEVELOPER_INI).is_none());
    }

    #[test]
    fn no_source_writes_none_marker() {
        assert_eq!(m::plan(false, None, |_| false), Plan::NoSource);
        assert_eq!(m::result(&[]).word(), "none");
    }

    #[test]
    fn io_error_is_partial() {
        let outcomes = [Outcome::Copied, Outcome::IoError, Outcome::Skipped(Skip::Exists)];
        assert_eq!(m::result(&outcomes), ImportResult::Partial);
        assert_eq!(m::result(&[Outcome::Copied, Outcome::Skipped(Skip::Exists)]), ImportResult::Done);
        assert_eq!(m::result(&[Outcome::Skipped(Skip::Exists)]), ImportResult::Nothing);
    }

    #[test]
    fn too_large_skipped() {
        let sources = [src("save_states.json", m::MAX_COPY_BYTES + 1), src("macros/ok.json", m::MAX_COPY_BYTES)];
        let plan = m::plan(false, Some(&sources), |_| false);
        let actions = copies(&plan);
        assert_eq!(actions[0].skip, Some(Skip::TooLarge));
        assert_eq!(actions[1].skip, None);
        assert_eq!(Skip::TooLarge.word(), "too_large");
    }

    #[test]
    fn settings_renamed_on_copy() {
        // The settings file of the fork is training_modpack_menu.conf (its training_modpack.json is
        // the updater record, which is never copied).
        assert_eq!(m::route("training_modpack_menu.conf").map(|r| r.to), Some("settings.json"));
        assert!(m::route("training_modpack.json").is_none());
        assert_eq!(m::SETTINGS_SOURCE, "training_modpack_menu.conf");
    }

    #[test]
    fn ini_files_copied() {
        for ini in ["button_config.ini", "replay_capture.ini"] {
            let r = m::route(ini).unwrap();
            assert_eq!((r.to, r.part), (ini, Part::Blocking));
        }
    }

    #[test]
    fn loader_accepts_old_name_once() {
        use storage::SettingsSource::*;
        assert_eq!(storage::pick_settings_source(false, true), LegacyName);
        assert_eq!(storage::pick_settings_source(true, true), Current);
    }

    #[test]
    fn character_item_ignored_after_import() {
        let text = r#"{"menu": {"character_item": [0,1,0], "lra_reset": [0,1]}, "defaults_menu": {"character_item": [1]}}"#;
        let loaded = settings::parse(text);
        assert!(!loaded.corrupt());
        assert!(!loaded.settings.to_json().contains("character_item"));
        assert!(!loaded.settings.menu.lra_reset());
    }

    #[test]
    fn captures_have_no_size_cap() {
        let sources = [src("replay_captures/big.smd", 900 * 1024 * 1024)];
        let plan = m::plan(false, Some(&sources), |_| false);
        assert_eq!(copies(&plan)[0].skip, None);
        let r = m::route("replay_captures/big.smd").unwrap();
        assert!(!r.capped);
    }

    #[test]
    fn captures_part_then_rename() {
        let mut name = String::new();
        m::part_name("replay_captures/c1.smd", &mut name).unwrap();
        assert_eq!(name, "replay_captures/c1.smd.part");
        // A leftover .part in the source folder is not a capture to copy.
        let sources = [src("replay_captures/c1.smd", 1)];
        let plan = m::plan(false, Some(&sources), |to| to.ends_with(".part"));
        assert_eq!(copies(&plan)[0].skip, None);
    }

    #[test]
    fn plan_splits_blocking_and_background() {
        let sources = listing();
        let plan = m::plan(false, Some(&sources), |_| false);
        for a in copies(&plan) {
            let expected = if a.from.starts_with("replay_captures/") { Part::Background } else { Part::Blocking };
            assert_eq!(a.part, expected, "{}", a.from);
        }
    }

    #[test]
    fn marker_after_background_part() {
        assert!(!m::marker_written_after(true, false));
        assert!(!m::marker_written_after(false, true));
        assert!(m::marker_written_after(true, true));
    }

    #[test]
    fn captures_originals_untouched() {
        // The plan only ever names a destination under the new root and never a source write.
        let sources = listing();
        let plan = m::plan(false, Some(&sources), |_| false);
        for a in copies(&plan) {
            assert!(!a.to.starts_with("../") && !a.to.starts_with("sd:"), "{}", a.to);
        }
        let _ = Settings::factory();
    }
}
