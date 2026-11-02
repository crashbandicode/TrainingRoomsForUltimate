//! Boot (SPEC-core-01 §5.1), package 1: version guard, panic hook, data folder, `developer.ini`,
//! settings, a comparison of every site window, then the features whose sites passed, then the one
//! install line. NOT YET SWITCH-COMPILED.
//!
//! Not in package 1: the first-boot import runtime (SPEC-core-08; its plan is in `model::migration`),
//! the provider plugins, the menu, the notifications.

use std::fs;

use crate::model::cpu::rng::Rng;
use crate::model::cpu::settings::CpuSettings;
use crate::model::developer_ini;
use crate::model::guard::GuardOutcome;
use crate::model::install_line::{DeveloperReport, FeatureReport, InstallLine, SettingsReport, SiteReport, Written};
use crate::model::settings::{self, Settings};
use crate::model::sites::{self, Feature, SITES};
use crate::model::stage_hazards;
use crate::model::storage::{self, FixedPath};
use crate::model::version::{self, Verdict};
use crate::runtime::alloc::hook_allocations;
use crate::runtime::diag_log::{self, diagnostic};
use crate::runtime::features::{cpu, input_delay, lra_reset, reset_hub, stage_hazards as hazards};
use crate::runtime::{clock, game, install, published};

const SITE_COUNT: usize = SITES.len();

pub fn run() {
    diag_log::init_diagnostics();
    diag_log::start_drain_thread();

    // 1. The version guard: nothing else runs on another game.
    let display = game::display_version();
    let display = version::display_version(&display);
    let build_id = version::find_gnu_build_id(game::rodata_tail()).map(|(_, id)| id);
    let verdict = version::verdict(display, build_id);
    if verdict != Verdict::Supported {
        let line = InstallLine {
            tick: clock::uptime_ms(),
            version: env!("CARGO_PKG_VERSION"),
            display,
            build_id,
            verdict,
            sites: &[],
            features: &[],
            developer: None,
            settings: None,
            migration: None,
            heap_allocations: Some(hook_allocations()),
        };
        install::write(&line);
        diag_log::flush();
        return;
    }

    // 2. Panic policy (decision B2): log it, no pop-up. `panic = "abort"` then ends the process.
    std::panic::set_hook(Box::new(|info| {
        diagnostic(format_args!("[PracticeDiagnostics] rust_panic {info}"));
        diag_log::flush();
    }));

    // 3. Data folder, developer.ini (N1), settings.
    if let Err(error) = fs::create_dir_all(storage::ROOT) {
        diagnostic(format_args!("[TrainingRooms] data folder unavailable: {error}"));
    }
    let developer = read_developer_ini();
    published::set_developer_page(developer.developer_page);
    let loaded = load_settings();
    // The CPU behaviours' state exists before the first publish and before any hook is installed. The
    // generator is the plugin's own (never the game's): seeded from the system tick and an address.
    cpu::init(
        CpuSettings::from_snapshot(&loaded.menu),
        Rng::seed_from(clock::uptime_ms(), (run as fn() as usize) as u64),
    );
    // Native Training until the waiting room's practice says otherwise (package 6 calls this again).
    cpu::set_lobby(false);
    published::publish(&loaded.menu, 0);

    // 4. Compare every site window before anything is written (SPEC-core-01 §5.1 step 5b).
    let mut found = [GuardOutcome::Stock; SITE_COUNT];
    for (slot, site) in found.iter_mut().zip(SITES.iter()) {
        *slot = if site.compared() {
            site.outcome(&game::read_window(site.offset))
        } else {
            site.slot_outcome(game::read_slot_target(site.offset))
        };
    }
    let usable = |site: &sites::Site| {
        SITES.iter().position(|s| s.name == site.name).is_some_and(|i| found[i].usable())
    };
    if sites::find("is_training_mode").is_some_and(usable) {
        game::allow_training_predicate();
    }

    // 5. Install the features whose sites are all usable.
    let mut written = [Written::Skipped; SITE_COUNT];
    for (slot, site) in written.iter_mut().zip(SITES.iter()) {
        if usable(site) {
            *slot = Written::Checked;
        }
    }
    let mut mark = |name: &str, how: Written| {
        if let Some(i) = SITES.iter().position(|s| s.name == name) {
            written[i] = how;
        }
    };

    let input_delay_on = sites::feature_usable(Feature::InputDelay, usable);
    if input_delay_on {
        skyline::install_hook!(input_delay::caller_a_note);
        skyline::install_hook!(input_delay::caller_a_apply);
        skyline::install_hook!(input_delay::caller_b_note);
        skyline::install_hook!(input_delay::caller_b_apply);
        for name in ["input_caller_a_note", "input_caller_a_apply", "input_caller_b_note", "input_caller_b_apply"] {
            mark(name, Written::Near);
        }
    }
    let lra_on = sites::feature_usable(Feature::LraReset, usable);
    if lra_on {
        skyline::install_hook!(lra_reset::reset_check);
        mark("reset_check", Written::Near);
    }
    let hub_on = sites::feature_usable(Feature::ResetHub, usable);
    if hub_on {
        reset_hub::install();
        mark("on_init_slot", Written::Far);
    }
    let hazards_on = install_hazards(&usable, &mut mark);
    let (cpu_core_on, cpu_defence_on, cpu_ledge_on, cpu_tech_on, cpu_shield_on) = install_cpu(&usable, &mut mark);

    // 6. The install line.
    let reports: [SiteReport; SITE_COUNT] =
        std::array::from_fn(|i| SiteReport { site: SITES[i].name, found: found[i], written: written[i] });
    let features = [
        FeatureReport { feature: Feature::InputDelay.name(), installed: input_delay_on },
        FeatureReport { feature: Feature::LraReset.name(), installed: lra_on },
        FeatureReport { feature: Feature::StageHazards.name(), installed: hazards_on },
        FeatureReport { feature: Feature::ResetHub.name(), installed: hub_on },
        FeatureReport { feature: Feature::CpuCore.name(), installed: cpu_core_on },
        FeatureReport { feature: Feature::CpuDefence.name(), installed: cpu_defence_on },
        FeatureReport { feature: Feature::CpuLedge.name(), installed: cpu_ledge_on },
        FeatureReport { feature: Feature::CpuTech.name(), installed: cpu_tech_on },
        FeatureReport { feature: Feature::CpuShield.name(), installed: cpu_shield_on },
    ];
    let line = InstallLine {
        tick: clock::uptime_ms(),
        version: env!("CARGO_PKG_VERSION"),
        display,
        build_id,
        verdict,
        sites: &reports,
        features: &features,
        developer: Some(DeveloperReport { on: developer.developer_page, reason: developer.reason.text() }),
        settings: Some(SettingsReport {
            input_delay: loaded.menu.input_delay(),
            lra_reset: loaded.menu.lra_reset(),
            stage_hazards: loaded.menu.stage_hazards(),
        }),
        migration: None,
        heap_allocations: Some(hook_allocations()),
    };
    install::write(&line);
    diag_log::flush();
}

/// The CPU behaviours (package 2): the per-frame wrapper first (the facts every other piece reads),
/// then each feature's hooks when all of its sites (and the core's) are usable. Returns whether the
/// core, SPEC-cpu-02's hooks, SPEC-cpu-03's hook and SPEC-cpu-04's hooks and SPEC-cpu-05's hooks were installed. The DI hook lives in the `common`
/// script module and is not installed (its 64-byte guard cannot be taken yet, see the report).
fn install_cpu(usable: &dyn Fn(&sites::Site) -> bool, mark: &mut dyn FnMut(&str, Written)) -> (bool, bool, bool, bool, bool) {
    if !sites::feature_usable(Feature::CpuCore, usable) {
        return (false, false, false, false, false);
    }
    cpu::install_frame();
    mark("cpu_on_frame_slot", Written::Far);
    let defence = sites::feature_usable(Feature::CpuDefence, usable);
    if defence {
        skyline::install_hook!(cpu::cpu_sdi_hook);
        skyline::install_hook!(cpu::cpu_stick_x_hook);
        skyline::install_hook!(cpu::cpu_stick_y_hook);
        skyline::install_hook!(cpu::cpu_clatter_start_hook);
        for name in ["sdi_hook", "stick_x_hook", "stick_y_hook", "clatter_start_hook"] {
            mark(name, Written::Far);
        }
    }
    let ledge = sites::feature_usable(Feature::CpuLedge, usable);
    if ledge {
        skyline::install_hook!(cpu::cpu_transition_term_hook);
        mark("transition_term_hook", Written::Far);
    }
    let tech = sites::feature_usable(Feature::CpuTech, usable);
    if tech {
        skyline::install_hook!(cpu::cpu_motion_change_hook);
        skyline::install_hook!(cpu::cpu_quake_pos_hook);
        cpu::set_tech_on(true);
        for name in ["motion_change_hook", "quake_pos_hook"] {
            mark(name, Written::Far);
        }
    }
    let shield = sites::feature_usable(Feature::CpuShield, usable);
    if shield {
        skyline::install_hook!(cpu::cpu_check_button_on_hook);
        skyline::install_hook!(cpu::cpu_check_button_off_hook);
        skyline::install_hook!(cpu::cpu_stick_x_no_clamp_hook);
        skyline::install_hook!(cpu::cpu_stick_y_no_clamp_hook);
        skyline::install_hook!(cpu::cpu_shield_param_hook);
        for name in ["check_button_on_hook", "check_button_off_hook", "stick_x_no_clamp_hook", "stick_y_no_clamp_hook", "shield_param_hook"] {
            mark(name, Written::Far);
        }
    }
    (true, defence, ledge, tech, shield)
}

/// Stage Hazards: both a derived flag address equal to the pinned one and a usable hook site.
fn install_hazards(usable: &dyn Fn(&sites::Site) -> bool, mark: &mut dyn FnMut(&str, Written)) -> bool {
    let sites_ok = sites::feature_usable(Feature::StageHazards, usable);
    let flag = sites::find("hazard_flag").and_then(|site| {
        let window = game::read_window(site.offset);
        let word = |i: usize| u32::from_le_bytes([window[4 * i], window[4 * i + 1], window[4 * i + 2], window[4 * i + 3]]);
        stage_hazards::flag_offset(site.offset, word(0), word(1)).filter(|offset| *offset == stage_hazards::FLAG_OFFSET)
    });
    match stage_hazards::plan(flag, sites_ok) {
        stage_hazards::Plan::Enabled => {
            hazards::set_flag_address(game::text_base() + stage_hazards::FLAG_OFFSET);
            skyline::install_hook!(hazards::hazard_rule);
            mark("hazard_hook", Written::Near);
            true
        }
        stage_hazards::Plan::Disabled(missing) => {
            diagnostic(format_args!("[TrainingRooms] stage_hazards off: {}", missing.text()));
            false
        }
    }
}

/// `developer.ini` through the one validated loader (`developer_ini::load`): it fails closed, and the
/// install line reports the outcome (`developer=on|off reason=…`).
fn read_developer_ini() -> developer_ini::DeveloperIni {
    match FixedPath::<128>::in_root(storage::file::DEVELOPER_INI) {
        Some(path) => developer_ini::load(path.as_str()),
        None => developer_ini::DeveloperIni::default(),
    }
}

/// Read `settings.json` (or the fork's file name, once) on a thread with a 128 KiB stack, joined here
/// (SPEC-core-04 §5). A file that does not read cleanly is kept under `settings.invalid-<n>.json`
/// (decision S2) and what parses is used.
fn load_settings() -> Settings {
    let spawned = std::thread::Builder::new()
        .name("tr-settings-load".into())
        .stack_size(settings::LOADER_STACK_BYTES)
        .spawn(load_settings_inner);
    match spawned {
        Ok(handle) => handle.join().unwrap_or_else(|_| Settings::factory()),
        Err(_) => load_settings_inner(),
    }
}

fn load_settings_inner() -> Settings {
    let exists = |name: &str| FixedPath::<128>::in_root(name).is_some_and(|p| fs::metadata(p.as_str()).is_ok());
    let name = match storage::pick_settings_source(
        exists(storage::file::SETTINGS),
        exists(storage::file::LEGACY_SETTINGS),
    ) {
        storage::SettingsSource::Current => storage::file::SETTINGS,
        storage::SettingsSource::LegacyName => storage::file::LEGACY_SETTINGS,
        storage::SettingsSource::Missing => return Settings::factory(),
    };
    let Some(path) = FixedPath::<128>::in_root(name) else {
        return Settings::factory();
    };
    let bytes = match fs::read(path.as_str()) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostic(format_args!("[TrainingRooms] settings unreadable: {error}; factory settings"));
            return Settings::factory();
        }
    };
    let loaded = settings::parse_bytes(&bytes);
    for issue in &loaded.issues {
        diagnostic(format_args!("[TrainingRooms] settings issue at={} reason={}", issue.at, issue.reason));
    }
    if loaded.corrupt() {
        keep_copy(path.as_str());
    }
    loaded.settings
}

/// Rename the file that did not read cleanly to the first free `settings.invalid-<n>.json`.
fn keep_copy(path: &str) {
    let exists = |name: &str| FixedPath::<128>::in_root(name).is_some_and(|p| fs::metadata(p.as_str()).is_ok());
    let Some(n) = storage::first_free_invalid_copy(exists) else {
        diagnostic(format_args!("[TrainingRooms] no free settings.invalid-<n>.json; the file stays where it is"));
        return;
    };
    let Some(target) = storage::invalid_copy_name(n).and_then(|name| FixedPath::<128>::in_root(name.as_str())) else {
        return;
    };
    match fs::rename(path, target.as_str()) {
        Ok(()) => diagnostic(format_args!("[TrainingRooms] settings kept as {}", target.as_str())),
        Err(error) => diagnostic(format_args!("[TrainingRooms] could not keep a copy: {error}")),
    }
}
