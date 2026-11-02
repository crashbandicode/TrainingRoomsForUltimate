//! SPEC-core-01 (boot, version guard, site table, install line, storage paths, developer.ini).
//! Test names are the spec's §9 rows. The version guard's own carried tests are in
//! `tests/carried_model_facade.rs`; the 64-byte rule's are in `tests/carried_guard.rs`.

mod common;

#[path = "../src/model/mod.rs"]
#[allow(dead_code)]
mod model;

use model::guard::{GuardOutcome, HookForm, Placement, SeamKind, GUARD_BYTES};
use model::install_line::{FeatureReport, InstallLine, SiteReport, Written};
use model::sites::{self, Feature, Phase, SITES};
use model::storage::{self, FixedPath};
use model::version::{self, Verdict};

fn note(id: &[u8]) -> Vec<u8> {
    let mut n = Vec::new();
    n.extend_from_slice(&4u32.to_le_bytes());
    n.extend_from_slice(&(id.len() as u32).to_le_bytes());
    n.extend_from_slice(&3u32.to_le_bytes());
    n.extend_from_slice(b"GNU\0");
    n.extend_from_slice(id);
    n
}

#[test]
fn core01_verdict_matrix() {
    let id = &version::MAIN_BUILD_ID_BYTES[..];
    assert_eq!(version::verdict(b"13.0.5", Some(id)), Verdict::Supported);
    assert_eq!(version::verdict(b"13.0.4", Some(id)), Verdict::UnsupportedVersion);
    assert_eq!(version::verdict(b"13.0.5", Some(&[7u8; 20])), Verdict::UnsupportedBuild);
    assert_eq!(version::verdict(b"13.0.5", None), Verdict::BuildIdMissing);
    assert_eq!(Verdict::UnsupportedVersion.reason(), "display_version");
    assert_eq!(Verdict::UnsupportedBuild.reason(), "build_id");
    assert_eq!(Verdict::BuildIdMissing.reason(), "build_id_missing");
}

#[test]
fn core01_build_id_scan_tail() {
    // The last 4-aligned note wins over an earlier decoy.
    let mut tail = vec![0u8; 0x1000];
    let decoy = note(&[0x11; 20]);
    tail[0x100..0x100 + decoy.len()].copy_from_slice(&decoy);
    let real = note(&version::MAIN_BUILD_ID_BYTES);
    tail[0xf3c..0xf3c + real.len()].copy_from_slice(&real);
    let (at, id) = version::find_gnu_build_id(&tail).unwrap();
    assert_eq!((at, id), (0xf3c, &version::MAIN_BUILD_ID_BYTES[..]));
}

#[test]
fn core01_unsupported_logs_and_returns() {
    for verdict in [Verdict::UnsupportedVersion, Verdict::UnsupportedBuild, Verdict::BuildIdMissing] {
        assert!(sites::install_plan(verdict).is_empty(), "{verdict:?} installs nothing");
        let line = InstallLine {
            tick: 5,
            version: "0.1.0",
            display: b"13.0.4",
            build_id: None,
            verdict,
            sites: &[],
            features: &[],
            migration: None,
            heap_allocations: Some(0),
        };
        let text = line.to_string();
        assert!(text.contains(" result=unsupported_version reason="), "{text}");
        assert!(text.ends_with(" heap_allocations=0"), "{text}");
        assert!(!text.contains("=stock"), "{text}");
    }
    assert_eq!(sites::install_plan(Verdict::Supported).len(), SITES.len());
}

#[test]
fn core01_install_line_format() {
    let sites = [
        SiteReport { site: "final_input_mapping", found: GuardOutcome::Stock, written: Written::Far },
        SiteReport { site: "hazard_flag", found: GuardOutcome::HookedByOther { at: 4, usable: true }, written: Written::Checked },
        SiteReport { site: "reset_check", found: GuardOutcome::Foreign { first_diff: 8 }, written: Written::Skipped },
    ];
    let features = [
        FeatureReport { feature: "input_delay", on: true },
        FeatureReport { feature: "lra_reset", on: false },
    ];
    let line = InstallLine {
        tick: 123456,
        version: "0.1.0",
        display: b"13.0.5",
        build_id: Some(&version::MAIN_BUILD_ID_BYTES),
        verdict: Verdict::Supported,
        sites: &sites,
        features: &features,
        migration: Some("none"),
        heap_allocations: Some(0),
    };
    assert_eq!(
        line.to_string(),
        "123456 plugin=training_rooms/0.1.0 game=13.0.5 build=21450c647b8c5940ed5b68218ea128fd3c5d597d \
         final_input_mapping=stock/far hazard_flag=other/checked reset_check=foreign/skipped \
         input_delay=on lra_reset=off migration=none result=installed heap_allocations=0"
    );
    let unmetered = InstallLine { heap_allocations: None, ..line };
    assert!(unmetered.to_string().ends_with(" result=installed heap_allocations=unmetered"));
    assert_eq!(storage::file::EVENTS_LOG, "events.log");
    assert_eq!(FixedPath::<128>::in_root(storage::file::EVENTS_LOG).unwrap().as_str(), "sd:/ultimate/TrainingRooms/events.log");
}

#[test]
fn core01_site_table_guards_present() {
    let mut names = std::collections::BTreeSet::new();
    for site in &SITES {
        assert!(names.insert(site.name), "duplicate site {}", site.name);
        assert!(site.guard.iter().any(|b| *b != 0), "{} has no guard bytes", site.name);
        if let SeamKind::Hook { placement: Placement::Entry, .. } = site.kind {
            assert!(!site.note.is_empty(), "{}: an entry hook needs its justification", site.name);
        }
        if !site.compared() {
            assert!(site.expected_slot_target().is_some_and(|t| t != 0));
            assert!(site.guard[8..].iter().all(|b| *b == 0));
        }
    }
    // Every feature has at least one site of its own and the shared predicate exists.
    for feature in [Feature::InputDelay, Feature::LraReset, Feature::ResetHub, Feature::StageHazards, Feature::Training] {
        assert!(sites::sites_of(feature).next().is_some(), "{}", feature.name());
    }
    assert!(sites::find("is_training_mode").is_some());
    // A feature is usable only when all of its sites and the predicate are.
    assert!(sites::feature_usable(Feature::InputDelay, |_| true));
    assert!(!sites::feature_usable(Feature::InputDelay, |s| s.name != "final_input_mapping"));
    assert!(!sites::feature_usable(Feature::InputDelay, |s| s.name != "is_training_mode"));
    assert!(sites::feature_usable(Feature::StageHazards, |s| s.feature != Feature::InputDelay));
}

#[test]
fn core01_site_offsets_named_by_hooks_are_table_rows() {
    for (name, offset) in [
        ("final_input_mapping", sites::FINAL_INPUT_MAPPING_OFFSET),
        ("reset_check", sites::RESET_CHECK_OFFSET),
        ("on_init_slot", sites::ON_INIT_SLOT_OFFSET),
        ("on_init", sites::ON_INIT_OFFSET),
        ("is_training_mode", sites::IS_TRAINING_MODE_OFFSET),
        ("hazard_hook", model::stage_hazards::HOOK_SITE_OFFSET),
        ("hazard_flag", model::stage_hazards::FLAG_SCAN_OFFSET),
    ] {
        assert_eq!(sites::find(name).unwrap().offset, offset, "{name}");
    }
}

#[test]
fn core01_overlapping_windows_checked_first() {
    // The hazard window and hook sit 8 bytes apart: a hook at one changes what the other reads.
    let pairs: Vec<_> = sites::overlapping_pairs().map(|(a, b)| (SITES[a].name, SITES[b].name)).collect();
    assert!(pairs.contains(&("hazard_window", "hazard_hook")), "{pairs:?}");
    // Boot compares every window before it writes anything.
    assert_eq!(sites::BOOT_PHASES, [Phase::CheckEveryWindow, Phase::InstallUsableFeatures]);
    // And the reason: after our own hook is written, the other window no longer reads stock.
    let window = sites::find("hazard_window").unwrap();
    let hook = sites::find("hazard_hook").unwrap();
    let mut after = window.guard;
    let delta = hook.offset - window.offset;
    after[delta..delta + 4].copy_from_slice(&0x1400_0123u32.to_le_bytes());
    assert!(matches!(window.outcome(&after), GuardOutcome::HookedByOther { usable: true, .. }), "relied row tolerates a hook past its first word");
    let mut hook_changed = hook.guard;
    hook_changed[0..4].copy_from_slice(&0x1400_0123u32.to_le_bytes());
    assert!(!hook.outcome(&hook_changed).usable(), "a second hook on our own site is not usable");
}

#[test]
fn core01_chain_entry_classification() {
    // The replace hook at the final input mapping entry: stock, another plugin's one-word branch,
    // and a far jump (Skyline's two forms, HOOK-SHARING).
    let site = sites::find("final_input_mapping").unwrap();
    assert!(matches!(site.kind, SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }));
    assert_eq!(site.outcome(&site.guard), GuardOutcome::Stock);
    let mut branch = site.guard;
    branch[..4].copy_from_slice(&0x1400_0456u32.to_le_bytes());
    assert_eq!(site.outcome(&branch), GuardOutcome::HookedByOther { at: 0, usable: false });
    let mut far = site.guard;
    far[..4].copy_from_slice(&model::guard::LDR_X17_LITERAL_8.to_le_bytes());
    far[4..8].copy_from_slice(&model::guard::BR_X17.to_le_bytes());
    assert_eq!(site.outcome(&far), GuardOutcome::HookedByOther { at: 0, usable: false });
    // The slot row: right target is stock, any other is foreign.
    let slot = sites::find("on_init_slot").unwrap();
    assert_eq!(slot.slot_outcome(0x60dfa0), GuardOutcome::Stock);
    assert!(!slot.slot_outcome(0x123456).usable());
}

#[test]
fn core01_sites_match_owned_elf() {
    let Some(main) = common::owned_main("core01_sites_match_owned_elf") else {
        return;
    };
    for site in SITES.iter().filter(|s| s.compared()) {
        assert_eq!(main.window(site.offset), site.guard, "{} at {:#x}", site.name, site.offset);
    }
    // The vtable slot is in `.data` and holds zero in the ELF (the loader relocates it), so it is
    // checked against the table instead: the slot must point at the `on_init` function row.
    let slot = sites::find("on_init_slot").unwrap();
    let on_init = sites::find("on_init").unwrap();
    assert_eq!(slot.expected_slot_target(), Some(on_init.offset as u64));
    assert_eq!(GUARD_BYTES, 64);
}

#[test]
fn core01_paths() {
    assert_eq!(storage::ROOT, "sd:/ultimate/TrainingRooms");
    assert_eq!(FixedPath::<128>::in_root(storage::file::SETTINGS).unwrap().as_str(), "sd:/ultimate/TrainingRooms/settings.json");
    assert_eq!(FixedPath::<128>::in_root(storage::file::DEVELOPER_INI).unwrap().as_str(), "sd:/ultimate/TrainingRooms/developer.ini");
    // Too long for the buffer: no path, not a truncated one.
    assert!(FixedPath::<16>::in_root("settings.json").is_none());
    // The fork's folder is only ever a source.
    assert_ne!(storage::ROOT, storage::FORK_ROOT);
    // S2: the first free renamed copy.
    assert_eq!(storage::invalid_copy_name(3).unwrap().as_str(), "settings.invalid-3.json");
    let taken = ["settings.invalid-1.json", "settings.invalid-2.json"];
    assert_eq!(storage::first_free_invalid_copy(|n| taken.contains(&n)), Some(3));
    assert_eq!(storage::first_free_invalid_copy(|_| true), None);
    // S1: settings.json, else the fork's name once, else factory.
    use storage::SettingsSource::*;
    assert_eq!(storage::pick_settings_source(true, true), Current);
    assert_eq!(storage::pick_settings_source(false, true), LegacyName);
    assert_eq!(storage::pick_settings_source(false, false), Missing);
    assert_eq!(storage::file::LEGACY_SETTINGS, "training_modpack_menu.conf");
    assert_eq!(FixedPath::<128>::in_root(storage::file::EVENTS_LOG).unwrap().as_str(), storage::EVENTS_LOG_PATH);
    assert_eq!(FixedPath::<128>::in_root(storage::file::DIAGNOSTICS).unwrap().as_str(), storage::DIAGNOSTICS_PATH);
    assert_eq!(FixedPath::<128>::in_root(storage::file::DIAGNOSTICS_PREVIOUS).unwrap().as_str(), storage::DIAGNOSTICS_PREVIOUS_PATH);
}

#[test]
fn core01_diag_ring_rotation() {
    assert!(!storage::should_rotate(0, 100));
    assert!(!storage::should_rotate(storage::DIAGNOSTIC_LIMIT - 100, 100));
    assert!(storage::should_rotate(storage::DIAGNOSTIC_LIMIT - 99, 100));
    assert_eq!(storage::DIAGNOSTIC_LIMIT, 128 * 1024);
}

#[test]
fn core01_diag_ring_admission() {
    use model::diag_ring::shipped;
    assert!(shipped(b"[TrainingRooms] boot"));
    assert!(shipped(b"[PracticeDiagnostics] rust_panic at x"));
    assert!(!shipped(b"[Hud] anything"));
}

#[test]
fn core01_no_network_symbols() {
    // Dropped features stay dropped: no TLS or HTTP crate anywhere in the dependency graph.
    let lock = include_str!("../Cargo.lock");
    for forbidden in ["native-tls", "openssl", "rustls", "minreq", "reqwest", "hyper", "ureq", "curl"] {
        assert!(!lock.contains(&format!("name = \"{forbidden}\"")), "{forbidden} is in Cargo.lock");
    }
}

#[test]
fn core01_developer_ini() {
    use model::developer_ini::parse;
    assert!(parse(b"developer_page=on\n").developer_page);
    assert!(parse(b"  developer_page = on  \r\n").developer_page);
    assert!(parse(b"\xef\xbb\xbfdeveloper_page=on").developer_page);
    assert!(parse(b"; note\n[dev]\n# x\ndeveloper_page=on\nother=1").developer_page);
    // Missing file (empty), other values, other keys, comments: hidden.
    assert!(!parse(b"").developer_page);
    assert!(!parse(b"developer_page=off").developer_page);
    assert!(!parse(b"developer_page=ON").developer_page);
    assert!(!parse(b"developer_page=1").developer_page);
    assert!(!parse(b"developer_page").developer_page);
    assert!(!parse(b"; developer_page=on").developer_page);
    assert!(!parse(b"x_developer_page=on").developer_page);
    assert!(!parse(&[0xff, 0xfe, 0x00]).developer_page);
    // The last line wins.
    assert!(parse(b"developer_page=off\ndeveloper_page=on").developer_page);
    assert!(!parse(b"developer_page=on\ndeveloper_page=off").developer_page);
    // Only the first 8 KiB are read.
    let mut long = vec![b'#'; 9000];
    long.extend_from_slice(b"\ndeveloper_page=on");
    assert!(!parse(&long).developer_page);
}
