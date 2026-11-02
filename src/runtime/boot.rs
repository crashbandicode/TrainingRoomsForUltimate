//! Boot (SPEC-core-01 §5.1), package 1: version guard, panic hook, data folder, `developer.ini`,
//! settings, a comparison of every site window, then the features whose sites passed, then the one
//! install line. NOT YET SWITCH-COMPILED.
//!
//! Not in package 1: the first-boot import runtime (SPEC-core-08; its plan is in `model::migration`),
//! the provider plugins, the menu, the notifications.

use std::fs;
use std::path::Path;

use crate::model::cpu::rng::Rng;
use crate::model::cpu::settings::CpuSettings;
use crate::model::developer_ini;
use crate::model::guard::GuardOutcome;
use crate::model::install_line::{DeveloperReport, Extra, FeatureReport, InstallLine, SettingsReport, SiteReport, Written};
use crate::model::settings::{self, Settings};
use crate::model::settings_file;
use crate::model::sites::{self, Feature, SITES};
use crate::model::stage_hazards;
use crate::model::storage::{self, FixedPath};
use crate::model::version::{self, Verdict};
use crate::runtime::alloc::hook_allocations;
use crate::runtime::diag_log::{self, diagnostic};
use crate::model::{sites_pkg3, sites_pkg4, sites_pkg6};
use crate::model::state::file::SaveFile;
use crate::model::state::settings::StateSettings;
use crate::runtime::features::{
    chords, cpu, cpu_common, hud_draw, input_delay, lra_reset, notices, recorder, reset_hub, stage_hazards as hazards, states, writer,
};
use crate::runtime::{lobby, mii, p1, roster};
use crate::runtime::{clock, game, import, install, published, scene, state_boot, ui};

const SITE_COUNT: usize = SITES.len();

pub fn run() {
    diag_log::init_diagnostics();
    diag_log::set_tick(crate::runtime::seams::seam_install::report_late);
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
            seams: None,
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
    #[cfg(feature = "diag")]
    crate::runtime::sync_probe::run();
    let developer = read_developer_ini();
    published::set_developer_page(developer.developer_page);
    // SPEC-core-08: the fork's data is imported before the settings load (the captures go on in the
    // background).
    let migration = import::start();
    let loaded = load_settings();
    // The CPU behaviours' state exists before the first publish and before any hook is installed. The
    // generator is the plugin's own (never the game's): seeded from the system tick and an address.
    cpu::init(
        CpuSettings::from_snapshot(&loaded.menu),
        Rng::seed_from(clock::uptime_ms(), (run as fn() as usize) as u64),
    );
    // Native Training until the waiting room's practice says otherwise (package 6 calls this again).
    cpu::set_lobby(false);
    scene::set_lobby_active(false);
    // Package 3: the save-state slots (read once, here, on a thread with an explicit stack; the writer
    // thread keeps the file current).
    states::init(
        StateSettings::from_snapshot(&loaded.menu),
        load_save_states(),
        Rng::seed_from(clock::uptime_ms().rotate_left(7), ((run as fn() as usize) as u64) ^ 0x5a7e_57a7),
    );
    states::start_writer();
    #[cfg(feature = "diag")]
    states::start_diag_triggers();
    // Package 4: the shortcuts file (fail-closed, created when absent) and the recorder's generator.
    chords::load_ini();
    writer::load_replay_capture();
    writer::start();
    recorder::init(Rng::seed_from(clock::uptime_ms().wrapping_add(1), (run as fn() as usize) as u64));
    published::publish(&loaded.menu, 0);
    // Package 6: the typed view the carried engines read (decision D1); the settings are loaded and published first.
    crate::runtime::compat::menu::apply(&loaded);
    #[cfg(feature = "diag")]
    crate::runtime::diag_reload::start();

    // 4. Compare every site window before anything is written (SPEC-core-01 §5.1 step 5b).
    let state_found = state_boot::check();
    let mut found = [GuardOutcome::Stock; SITE_COUNT];
    for (slot, site) in found.iter_mut().zip(SITES.iter()) {
        *slot = if site.compared() {
            site.outcome(&game::read_window(site.offset))
        } else {
            site.slot_outcome(game::read_slot_target(site.offset))
        };
    }
    let mut found4 = [GuardOutcome::Stock; sites_pkg4::SITES.len()];
    for (slot, site) in found4.iter_mut().zip(sites_pkg4::SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let usable = |site: &sites::Site| {
        SITES.iter().position(|s| s.name == site.name).is_some_and(|i| found[i].usable())
    };
    // The shared fighter natives of `game::natives`, each group once the rows it calls passed their guards.
    let row_usable = |name: &str| sites::find(name).is_some_and(usable);
    let row4_usable = |name: &str| sites_pkg4::SITES.iter().position(|s| s.name == name).is_some_and(|i| found4[i].usable());
    if row4_usable("fighter_manager_get_entry") {
        game::allow_natives(game::NativeGroup::Entry);
    }
    if row_usable("fighter_information") && row_usable("is_operation_cpu") {
        game::allow_natives(game::NativeGroup::Operator);
    }
    if row_usable("utility_get_category") && row_usable("utility_get_kind") {
        game::allow_natives(game::NativeGroup::Category);
    }
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
    let (cpu_core_on, cpu_defence_on, cpu_ledge_on, cpu_tech_on, cpu_shield_on, cpu_common_reason) = install_cpu(&usable, &mut mark);
    let cpu_mash_on = cpu_core_on && install_cpu_mash(&usable, &mut mark);
    // Package 3: the save states ride on the CPU core's per-frame wrapper and constant lookup.
    let mut state_written = [Written::Skipped; state_boot::N];
    let state_on = state_boot::install(&state_found, cpu_core_on, &mut state_written);
    let state_reports = state_boot::reports(&state_found, &state_written);
    // Package 4: the recorder, the chords' poll and the notices.
    let mut written4 = [Written::Skipped; sites_pkg4::SITES.len()];
    // The recorder calls package 2's `work_get_int` row (the ledge rest timer): its own guard gates the recorder.
    let get_int_ok = sites::find("work_get_int").is_some_and(usable);
    let (recording_on, notices_on) = install_pkg4(input_delay_on && get_int_ok, cpu_core_on, &found4, &mut written4);

    // Package 5: the menu's layout archive hook, UI anchors and input callers.
    let ui_install = ui::install(&loaded, input_delay_on);
    // Package 4's notices and input log draw from the layout tick package 5 just installed.
    if !ui::hud::register(hud_draw::draw) {
        diagnostic(format_args!("[TrainingRooms] hud draw not registered: every slot taken"));
    }

    // Package 6: the CPU-as-P1 fallback (Q11). It installs only while the standalone CPU pilot is absent (`ui::install` found out
    // above) and only when the CPU core, the input hooks, the save-state natives and every game function it calls passed their guards.
    let found6 = p1::check();
    let p1_prerequisites = cpu_core_on
        && input_delay_on
        && state_on
        && get_int_ok
        && ["status_kind", "prev_status_kind", "pos_x", "pos_y", "damage_of", "utility_get_kind"].iter().all(|name| row_usable(name))
        && row4_usable("fighter_manager_get_entry");
    let (p1_on, p1_reason, written6) = p1::install(&found6, p1_prerequisites);
    // Package 6: the roster cycle (hot swap, preload, Kirby hats, items), the Mii files and the waiting-room practice engine. The cycle
    // starts only when every game function its code calls through `smash::` passed its guard (`roster::CALLED_ROWS`); each of its
    // files then checks its own seam rows and installs nothing when they are refused (the seam fields of the install line say which).
    let state_row_usable = |name: &str| sites_pkg3::STATE_SITES.iter().position(|s| s.name == name).is_some_and(|i| state_found[i].usable());
    let own_row_usable = |name: &str| sites_pkg6::SITES.iter().position(|s| s.name == name).is_some_and(|i| found6[i].usable());
    let roster_on = roster::CALLED_ROWS.iter().all(|name| row_usable(name) || row4_usable(name) || state_row_usable(name) || own_row_usable(name));
    if roster_on {
        roster::init();
        mii::init();
        lobby::init();
    } else {
        diagnostic(format_args!("[TrainingRooms] roster cycle not started: a game function it calls failed its site guard"));
    }
    // 6. The install line: the base sites, then package 3's, package 4's and package 5's.
    const STATE_END: usize = SITE_COUNT + state_boot::N;
    let base_reports: [SiteReport; STATE_END + sites_pkg4::SITES.len()] = std::array::from_fn(|i| {
        if i < SITE_COUNT {
            SiteReport { site: SITES[i].name, found: found[i], written: written[i] }
        } else if i < STATE_END {
            state_reports[i - SITE_COUNT]
        } else {
            let j = i - STATE_END;
            SiteReport { site: sites_pkg4::SITES[j].name, found: found4[j], written: written4[j] }
        }
    });
    let mut reports: Vec<SiteReport> = base_reports.to_vec();
    reports.extend(ui::reports(&ui_install));
    reports.extend(p1::reports(&found6, &written6));
    let features = [
        FeatureReport { feature: Feature::InputDelay.name(), installed: input_delay_on, reason: None },
        FeatureReport { feature: Feature::LraReset.name(), installed: lra_on, reason: None },
        FeatureReport { feature: Feature::StageHazards.name(), installed: hazards_on, reason: None },
        FeatureReport { feature: Feature::ResetHub.name(), installed: hub_on, reason: None },
        FeatureReport { feature: Feature::CpuCore.name(), installed: cpu_core_on, reason: None },
        FeatureReport { feature: Feature::CpuDefence.name(), installed: cpu_defence_on, reason: None },
        FeatureReport { feature: Feature::CpuLedge.name(), installed: cpu_ledge_on, reason: None },
        FeatureReport { feature: Feature::CpuTech.name(), installed: cpu_tech_on, reason: None },
        FeatureReport { feature: Feature::CpuShield.name(), installed: cpu_shield_on, reason: None },
        FeatureReport { feature: Feature::CpuMash.name(), installed: cpu_mash_on, reason: None },
        FeatureReport { feature: Feature::CpuCommon.name(), installed: false, reason: cpu_common_reason },
        FeatureReport { feature: Feature::StateCore.name(), installed: state_on, reason: None },
        FeatureReport { feature: "chords", installed: input_delay_on, reason: None },
        FeatureReport { feature: Feature::Recording.name(), installed: recording_on, reason: None },
        FeatureReport { feature: Feature::Notices.name(), installed: notices_on, reason: None },
        FeatureReport { feature: Feature::Ui.name(), installed: ui_install.anchors, reason: None },
        FeatureReport { feature: Feature::P1Cpu.name(), installed: p1_on, reason: p1_reason },
        FeatureReport { feature: Feature::Roster.name(), installed: roster_on, reason: (!roster_on).then_some("site_guard") },
    ];
    let seam_fields = crate::runtime::seams::seam_install::InstallFields;
    let line = InstallLine {
        tick: clock::uptime_ms(),
        version: env!("CARGO_PKG_VERSION"),
        display,
        build_id,
        verdict,
        sites: &reports,
        features: &features,
        seams: Some(Extra(&seam_fields)),
        developer: Some(DeveloperReport { on: developer.developer_page, reason: developer.reason.text() }),
        settings: Some(SettingsReport {
            input_delay: loaded.menu.input_delay(),
            lra_reset: loaded.menu.lra_reset(),
            stage_hazards: loaded.menu.stage_hazards(),
        }),
        migration: Some(migration.as_str()),
        heap_allocations: Some(hook_allocations()),
    };
    install::write(&line);
    diag_log::flush();
}

/// Read `save_states.json` on a thread with a 128 KiB stack, joined here (the settings reader's rule).
fn load_save_states() -> SaveFile {
    let spawned = std::thread::Builder::new().name("tr-state-load".into()).stack_size(128 * 1024).spawn(states::load_file);
    match spawned {
        Ok(handle) => handle.join().unwrap_or_default(),
        Err(_) => states::load_file(),
    }
}

/// The CPU behaviours (package 2): the per-frame wrapper first (the facts every other piece reads),
/// then each feature's hooks when all of its sites (and the core's) are usable. Returns whether the
/// core, SPEC-cpu-02's hooks, SPEC-cpu-03's hook and SPEC-cpu-04's hooks and SPEC-cpu-05's hooks were installed. The DI hook lives in the `common`
/// script module and is not installed (its 64-byte guard cannot be taken yet, see the report).
fn install_cpu(usable: &dyn Fn(&sites::Site) -> bool, mark: &mut dyn FnMut(&str, Written)) -> (bool, bool, bool, bool, bool, Option<&'static str>) {
    if !sites::feature_usable(Feature::CpuCore, usable) {
        return (false, false, false, false, false, Some("cpu_core_not_installed"));
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
    // The hooks in the `common` script module install when that module loads, through the nro-load
    // provider (`libnro_hook`); the install line names why they are not installed yet, or at all.
    let common = if sites::feature_usable(Feature::CpuCommon, usable) {
        cpu_common::register()
    } else {
        Some("site_guard_failed")
    };
    (true, defence, ledge, tech, shield, common)
}

/// SPEC-cpu-01 and SPEC-cpu-06: the command-flag, aerial-kind and attack-angle hooks. The frame wrapper
/// reads the fall facts only once `set_mash_on` is set, before any of these hooks can fire.
fn install_cpu_mash(usable: &dyn Fn(&sites::Site) -> bool, mark: &mut dyn FnMut(&str, Written)) -> bool {
    if !sites::feature_usable(Feature::CpuMash, usable) {
        return false;
    }
    cpu::set_mash_on(true);
    skyline::install_hook!(cpu::cpu_command_flag_hook);
    skyline::install_hook!(cpu::cpu_attack_air_kind_hook);
    skyline::install_hook!(cpu::cpu_stick_dir_hook);
    for name in ["command_flag_hook", "attack_air_kind_hook", "stick_dir_hook"] {
        mark(name, Written::Far);
    }
    true
}

/// Package 4: the recorder's hook at the AI control function's epilogue (it also needs the poll hooks
/// that carry the chords and the P1 stage, and the CPU core's per-frame facts), and the notice
/// observers. Every row of a feature must have passed its guard. Returns whether the recorder and the
/// notice hooks were installed.
fn install_pkg4(poll_ok: bool, cpu_core_on: bool, found4: &[GuardOutcome], written4: &mut [Written]) -> (bool, bool) {
    let usable4 = |name: &str| sites_pkg4::SITES.iter().position(|s| s.name == name).is_some_and(|i| found4[i].usable());
    for (slot, site) in written4.iter_mut().zip(sites_pkg4::SITES.iter()) {
        if usable4(site.name) {
            *slot = Written::Checked;
        }
    }
    let mut mark = |name: &str, how: Written| {
        if let Some(i) = sites_pkg4::SITES.iter().position(|s| s.name == name) {
            written4[i] = how;
        }
    };
    let recording = poll_ok && cpu_core_on && sites_pkg4::sites_of(Feature::Recording).all(|s| usable4(s.name));
    if recording {
        skyline::install_hook!(recorder::ai_epilogue);
        mark("ai_epilogue", Written::Near);
    }
    // Each observer is its own site and its own pin: a mismatch leaves notices visible (SPEC-input-05 §4).
    let combo_a = usable4("combo_display_a_pin") && usable4("combo_display_a");
    let combo_b = usable4("combo_display_b_pin") && usable4("combo_display_b");
    let stale = usable4("stale_moves_toggle");
    if combo_a {
        skyline::install_hook!(notices::combo_display_a);
        mark("combo_display_a", Written::Near);
    }
    if combo_b {
        skyline::install_hook!(notices::combo_display_b);
        mark("combo_display_b", Written::Near);
    }
    if stale {
        skyline::install_hook!(notices::stale_moves_toggle);
        mark("stale_moves_toggle", Written::Near);
    }
    (recording, combo_a && combo_b && stale)
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
/// (SPEC-core-04 §5). All of the file logic, including the recovery of a file that does not read
/// cleanly (the original kept as `settings.invalid-<n>.json`, the sanitized settings written to the
/// active path before the original is retired), is `model::settings_file::load`; this only runs it and
/// logs its notes.
fn load_settings() -> Settings {
    let spawned = std::thread::Builder::new().name("tr-settings-load".into()).stack_size(settings::LOADER_STACK_BYTES).spawn(load_settings_inner);
    match spawned {
        Ok(handle) => handle.join().unwrap_or_else(|_| Settings::factory()),
        Err(_) => load_settings_inner(),
    }
}

fn load_settings_inner() -> Settings {
    let boot = settings_file::load(Path::new(storage::ROOT));
    for note in &boot.notes {
        diagnostic(format_args!("{note}"));
    }
    boot.settings
}