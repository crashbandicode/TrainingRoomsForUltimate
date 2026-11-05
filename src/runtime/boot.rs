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
use crate::model::capabilities::{self, Granted};
use crate::model::guard::GuardOutcome;
use crate::model::install_line::{DeveloperReport, Extra, FeatureReport, InstallLine, SettingsReport, SiteReport, Written};
use crate::model::settings::Settings;
use crate::model::settings_file;
use crate::model::sites::{self, Feature, SITES};
use crate::model::stage_hazards;
use crate::model::storage::{self, FixedPath};
use crate::model::thread_stacks::ThreadId;
use crate::model::version::{self, Verdict};
use crate::runtime::alloc::hook_allocations;
use crate::runtime::diag_log::{self, diagnostic};
use crate::model::sites_pkg3::{self, STATE_SITES};
use crate::model::sites_pkg4;
use crate::model::sites_hitbox;
use crate::model::sites_pkg6;
use crate::model::sites_u4;
use crate::model::sites_u4b;
use crate::model::sites_ui::UI_SITES;
use crate::model::state::file::SaveFile;
use crate::model::state::settings::StateSettings;
use crate::runtime::features::{
    chords, cpu, cpu_common, hitbox, hud_draw, input_delay, input_display, lra_reset, notices, recorder, reset_hub, stage_hazards as hazards, states, writer,
};
use crate::runtime::{lobby, mii, p1, roster};
use crate::runtime::{clock, game, import, install, published, scene, state_boot, ui};

const SITE_COUNT: usize = SITES.len();

pub fn run() {
    // The diag build measures the boot thread's stack across the whole boot (stream/stack); the file must be open first.
    #[cfg(feature = "diag")]
    {
        diag_log::init_diagnostics();
        crate::runtime::threads::boot_begin();
    }
    run_inner();
    #[cfg(feature = "diag")]
    {
        crate::runtime::threads::boot_end();
        diag_log::flush();
    }
}

fn run_inner() {
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
    // Counter storage must exist before ANY hook, including seam-drill hooks.
    // Allocation before this point reads unavailable TLS without initializing
    // it. Reservation/rollback calls only native scalar-slot SDK functions.
    if let Err(error) = crate::runtime::hook_tls::init() {
        diagnostic(format_args!("[TrainingRooms] install_refused reason=hook_counter_native_tls_unavailable result={error:#x}"));
        diag_log::flush();
        return;
    }
    // The hook-sharing drill (feature `seam-drill`): with `foreign=` in diag.txt it hooks its sites as a plugin that does not share
    // would, before any hook of ours (the default order) or, with `,after`, just before the install line.
    #[cfg(feature = "seam-drill")]
    crate::runtime::seams::seam_drill::before();

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
    // Diag build: the kernel arbiter the plugin's locks wait in answers as its source says (hos-lock README).
    #[cfg(feature = "diag")]
    diagnostic(format_args!("[Sync] arbiter self_test {}", hos_lock::horizon::self_test()));
    let developer = read_developer_ini();
    published::set_developer_page(developer.developer_page);
    // SPEC-core-08: the fork's data is imported before the settings load (the captures go on in the
    // background).
    let (migration, captures) = import::start();
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
    crate::runtime::compat::menu::apply_kept(&*loaded);
    #[cfg(feature = "diag")]
    crate::runtime::diag_reload::start();

    // 4. Compare every site window before anything is written (SPEC-core-01 §5.1 step 5b).
    let state_found = state_boot::check();
    // Only the diag build changes it (`apply_diag_poison` below).
    #[cfg(feature = "diag")]
    let mut state_found = state_found;
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
    // Package 5's rows are compared here too, before anything is written: a feature's closure (R2-1) can hold
    // a row of any table.
    let mut ui_found = [GuardOutcome::Stock; UI_SITES.len()];
    for (slot, site) in ui_found.iter_mut().zip(UI_SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    // Eden proof of R2-1 (diag only): rows named in `diag-poison.txt` are treated as foreign, as if another plugin
    // or an unknown patch had changed those bytes, so the install decisions run on a rejected row.
    #[cfg(feature = "diag")]
    apply_diag_poison(&mut found, &mut state_found, &mut found4, &mut ui_found);
    // Package 6's CPU-as-P1 rows and the run-time texture helper's rows are compared here too (R2-1): both are capabilities.
    let found6 = p1::check();
    // SPEC-core-06's rows: compared before anything is written (the rebound thunk's window holds its neighbours).
    // (Diag builds: the rows named in `diag-poison.txt` read as foreign, as for the other tables.)
    #[cfg(feature = "diag")]
    let poison_text = diag_poison_text();
    #[cfg(not(feature = "diag"))]
    let poison_text = String::new();
    let hb_found = hitbox::check(&|name| poison_text.split(|c: char| c.is_whitespace() || c == ',').any(|n| n == name));
    let developer_found = crate::runtime::developer::check();
    let mut u4_found = [GuardOutcome::Stock; sites_u4::SITES.len()];
    for (slot, site) in u4_found.iter_mut().zip(sites_u4::SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let mut u4b_found = [GuardOutcome::Stock; sites_u4b::SITES.len()];
    for (slot, site) in u4b_found.iter_mut().zip(sites_u4b::SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    // A row of any table by name (names are unique across the tables, `tests/capabilities.rs`).
    let usable = |site: &sites::Site| {
        let by_name = |rows: &[sites::Site], outcomes: &[GuardOutcome]| rows.iter().position(|s| s.name == site.name).map(|i| outcomes[i].usable());
        by_name(&SITES, &found)
            .or_else(|| by_name(&STATE_SITES, &state_found))
            .or_else(|| by_name(&sites_pkg4::SITES, &found4))
            .or_else(|| by_name(&UI_SITES, &ui_found))
            .or_else(|| by_name(&sites_pkg6::SITES, &found6))
            .or_else(|| by_name(&sites_u4::SITES, &u4_found))
            .or_else(|| by_name(&sites_u4b::SITES, &u4b_found))
            .or_else(|| by_name(&sites_hitbox::SITES, &hb_found))
            .or_else(|| by_name(&crate::model::developer_debug::SITES, &developer_found))
            .unwrap_or(false)
    };
    // The native-call layer calls only what passed (`game::natives` and `state_natives` refuse the rest).
    game::grant(Granted::from_usable(&usable));
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

    // Reserve every scalar TLS key before installing any input callback.
    // A partial failure rolls back all keys and reports an unavailable feature.
    // R3-3: pause ownership is installed on its minimal guarded callbacks, even
    // when the renderer's independent archive/text/manager closure is unavailable.
    let native_pause_on = ui::anchors::install(&usable);
    let (input_delay_on, input_delay_reason) = if !native_pause_on {
        (false, Some("native_pause_observer_unavailable"))
    } else if sites::feature_usable(Feature::InputDelay, usable) {
        match input_delay::init() {
            Ok(()) => (true, None),
            Err(result) => {
                diagnostic(format_args!("[TrainingRooms] input TLS reservation failed: result={result:#x}"));
                (false, Some("native_tls_unavailable"))
            }
        }
    } else {
        (false, None)
    };
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
    let (cpu_core_on, cpu_defence_on, cpu_ledge_on, cpu_tech_on, cpu_shield_on, cpu_common_reason, cpu_shield_damage_on, cpu_camera_on) = install_cpu(&usable, &mut mark);
    let cpu_mash_on = cpu_core_on && install_cpu_mash(&usable, &mut mark);
    // SPEC-core-06: the hitbox visualizer rides on the CPU core's per-fighter frame wrapper; its closure (own rows, the Training
    // predicate, the shared natives' rows) must have passed as a whole.
    let (hitbox_on, hb_written) = hitbox::install(&hb_found, cpu_core_on && capabilities::unit_usable(Feature::Hitbox, &usable));
    // Package 6: the roster cycle (hot swap, preload, Kirby hats, items), the Mii files and the waiting-room practice engine. The cycle
    // starts only when every game function its code calls through `smash::` passed its guard (`roster::CALLED_ROWS`); each of its
    // files then checks its own seam rows and installs nothing when they are refused (the seam fields of the install line say which).
    let state_row_usable = |name: &str| sites_pkg3::STATE_SITES.iter().position(|s| s.name == name).is_some_and(|i| state_found[i].usable());
    let own_row_usable = |name: &str| sites_pkg6::SITES.iter().position(|s| s.name == name).is_some_and(|i| found6[i].usable());
    let roster_on = roster::CALLED_ROWS.iter().all(|name| row_usable(name) || row4_usable(name) || state_row_usable(name) || own_row_usable(name));
    // Package 3: the save states ride on the CPU core's per-frame wrapper and constant lookup. The Kirby copy routine's hook also
    // carries the roster cycle's Kirby hat step, so it installs on its own row when the cycle starts.
    let mut state_written = [Written::Skipped; state_boot::N];
    let kirby_hats = roster_on && state_row_usable("state_kirby_opff");
    states::KIRBY_HATS.store(kirby_hats, std::sync::atomic::Ordering::Release);
    let state_on = state_boot::install(&state_found, cpu_core_on && capabilities::unit_usable(Feature::StateCore, &usable), &mut state_written);
    let state_reports = state_boot::reports(&state_found, &state_written);
    // Package 4: the recorder, the chords' poll and the notices.
    let mut written4 = [Written::Skipped; sites_pkg4::SITES.len()];
    // The recorder calls package 2's `work_get_int` row (the ledge rest timer): its own guard gates the recorder.
    // R2-1: the recorder's closure (its own rows and the other features' rows its code calls) is checked as a
    // whole; the notices keep their own per-observer rows and only need the cross-feature part here.
    // The input display's reads in the input-delay hook have their own flag, so Input Delay does not depend on them.
    let display_on = capabilities::unit_usable(Feature::InputDisplay, &usable);
    input_display::set_enabled(display_on);
    let recording_closure = capabilities::unit_usable(Feature::Recording, &usable);
    let notices_closure = capabilities::cross_blocked_by(Feature::Notices, &usable).is_none();
    let (recording_on, notices_on) = install_pkg4(input_delay_on && recording_closure, cpu_core_on, notices_closure, &found4, &mut written4);
    recorder::set_enabled(recording_on);

    // Package 5: the menu's layout archive hook, UI anchors and input callers (only when the closure holds).
    let ui_install = if capabilities::unit_usable(Feature::Ui, &usable) {
        ui::install(&*loaded, input_delay_on, ui_found)
    } else {
        diagnostic(format_args!("[Ui] not installed: a row of its closure was rejected"));
        ui::UiInstall { found: ui_found, archive_hook: false, anchors: false, texture_found: u4_found, effect_found: u4b_found }
    };
    // Package 4's notices and input log draw from the layout tick package 5 just installed.
    if !ui::hud::register(hud_draw::draw) {
        diagnostic(format_args!("[TrainingRooms] hud draw not registered: every slot taken"));
    }
    // HB1 route A4 prototype (diag only): the hitbox visualizer's own orbs draw from the same tick.
    #[cfg(feature = "diag")]
    if !ui::hud::register(hitbox::draw_orbs) {
        diagnostic(format_args!("[TrainingRooms] orb draw not registered: every slot taken"));
    }

    // Package 6: the CPU-as-P1 fallback (Q11). It installs only while the standalone CPU pilot is absent (`ui::install` found out
    // above) and only when the CPU core, the input hooks, the save-state natives and every game function it calls passed their guards.
    // The natives it calls are its closure (R2-1, `capabilities`): the package 2-4 rows its code reaches, with its own rows.
    let p1_prerequisites = cpu_core_on && input_delay_on && state_on && capabilities::unit_usable(Feature::P1Cpu, &usable);
    let (p1_on, p1_reason, written6) = p1::install(&found6, p1_prerequisites);
    // Package 4, S4-4: the named-macro layer and the live replay capture (its guards and hooks are the seam installer's: hooks only when
    // replay_capture.ini says on). The roster cycle and the menu's panel screens are behind the layer's port, named in the install line.
    let named_source = loaded.menu.get("macro_source").is_some_and(|row| row.get(1).is_some_and(|v| *v != 0));
    // R2-1: the layer's whole closure (its own rows and the cross-feature rows its code calls) must have passed.
    let macros_rows_ok = recording_on && capabilities::unit_usable(Feature::NamedMacros, &usable);
    let (macros_on, capture_on) = crate::runtime::macros::install(named_source, macros_rows_ok);
    // F9: the No. of CPUs field the cycle's Training sources depend on is read only when the game's own root load, reader and pause
    // menu writer are the 13.0.5 instructions (the rows name it in the install line otherwise; then Training admits no source: the
    // shipped preload-off path).
    roster::set_cpu_quantity_guarded(roster::CPU_QUANTITY_ROWS.iter().all(|name| own_row_usable(name)));
    // F11: the running body of an entry (natives::entry_active_body) follows the game's active-cell index only when its read is the
    // 13.0.5 one; otherwise every reader gets the first body, as before.
    crate::runtime::game::natives::ACTIVE_CELL_GUARDED.store(own_row_usable("entry_active_cell"), std::sync::atomic::Ordering::Release);
    if roster_on {
        init_roster_cycle();
    } else {
        diagnostic(format_args!("[TrainingRooms] roster cycle not started: a game function it calls failed its site guard"));
    }
    let developer_debug_on = crate::runtime::developer::install(
        developer.developer_page, capabilities::unit_usable(Feature::DeveloperDebug, &usable),
    );

    // 6. The install line: the base sites, then package 3's, package 4's and package 5's.
    const STATE_END: usize = SITE_COUNT + state_boot::N;
    // Construct the already-owned report vector directly: an intermediate full array
    // remains in the boot frame even while the synchronous loader fallback runs.
    let mut reports: Vec<SiteReport> = (0..STATE_END + sites_pkg4::SITES.len()).map(|i| {
        if i < SITE_COUNT {
            SiteReport { site: SITES[i].name, found: found[i], written: written[i] }
        } else if i < STATE_END {
            state_reports[i - SITE_COUNT]
        } else {
            let j = i - STATE_END;
            SiteReport { site: sites_pkg4::SITES[j].name, found: found4[j], written: written4[j] }
        }
    }).collect();
    reports.extend(ui::reports(&ui_install));
    reports.extend(p1::reports(&found6, &written6));
    reports.extend(hitbox::reports(&hb_found, &hb_written));
    reports.extend(crate::runtime::developer::reports(&developer_found, developer_debug_on));
    // A feature that is not installed names the first row of its closure that was rejected, when there is one.
    let blocked = |feature: Feature| capabilities::blocked_by(feature, &usable);
    let report = |feature: Feature, installed: bool, reason: Option<&'static str>| FeatureReport {
        feature: feature.name(),
        installed,
        reason,
        site: if installed || reason.is_some() { None } else { blocked(feature) },
    };
    let common_blocked = cpu_common_reason == Some("site_guard_failed");
    // Q7 / L1: the Replay capture row on the Deluxe Plugins page (registered from the layer install above).
    let (deluxe_on, deluxe_reason) = crate::runtime::deluxe_row::report();
    let features = [
        report(Feature::DeveloperDebug, developer_debug_on, (!developer.developer_page).then_some("page_disabled")),
        report(Feature::InputDelay, input_delay_on, input_delay_reason),
        report(Feature::LraReset, lra_on, None),
        report(Feature::StageHazards, hazards_on, None),
        report(Feature::ResetHub, hub_on, None),
        report(Feature::CpuCore, cpu_core_on, None),
        report(Feature::CpuDefence, cpu_defence_on, None),
        report(Feature::CpuLedge, cpu_ledge_on, None),
        report(Feature::CpuTech, cpu_tech_on, None),
        report(Feature::CpuShield, cpu_shield_on, None),
        report(Feature::CpuMash, cpu_mash_on, None),
        report(Feature::CpuShieldDamage, cpu_shield_damage_on, None),
        report(Feature::CpuCamera, cpu_camera_on, None),
        report(Feature::CpuCommon, false, if common_blocked { None } else { cpu_common_reason }),
        report(Feature::StateCore, state_on, None),
        FeatureReport { feature: Feature::Hitbox.name(), installed: hitbox_on, reason: (!hitbox_on && !cpu_core_on).then_some("cpu_core_not_installed"), site: if hitbox_on || !cpu_core_on { None } else { blocked(Feature::Hitbox) } },
        FeatureReport { feature: "chords", installed: input_delay_on, reason: input_delay_reason, site: None },
        report(Feature::InputDisplay, display_on, None),
        report(Feature::Recording, recording_on, None),
        report(Feature::Notices, notices_on, None),
        report(Feature::Ui, ui_install.anchors, None),
        FeatureReport { feature: Feature::P1Cpu.name(), installed: p1_on, reason: p1_reason, site: if p1_on || p1_reason.is_some_and(|r| r != "prerequisite_feature") { None } else { blocked(Feature::P1Cpu) } },
        report(Feature::UiTexture, capabilities::unit_usable(Feature::UiTexture, &usable), None),
        FeatureReport { feature: "named_macros", installed: macros_on, reason: if macros_on { None } else { Some("row_rejected_or_recorder_off") }, site: None },
        FeatureReport { feature: "live_replay", installed: capture_on, reason: if capture_on { None } else { Some("capture_off_or_guard") }, site: None },
        FeatureReport { feature: "deluxe_row", installed: deluxe_on, reason: deluxe_reason, site: None },
        FeatureReport { feature: "macros_port", installed: false, reason: Some(crate::runtime::macros::port::install_report()), site: None },
        FeatureReport { feature: Feature::Roster.name(), installed: roster_on, reason: (!roster_on).then_some("site_guard"), site: None },
    ];
    feature_reports_fit(&features);
    #[cfg(feature = "seam-drill")]
    crate::runtime::seams::seam_drill::after();
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
    // The import's captures run last, on the boot loaders' static stack (free once they and the install line are done).
    if let Some(pending) = captures {
        import::start_background(pending);
    }
}

/// Diag builds: `diag-poison.txt` (names separated by blanks, commas or lines) marks those rows foreign before any
/// install decision, each name logged (`diag_poison <name> found=<bool>`).
#[cfg(feature = "diag")]
fn diag_poison_text() -> String {
    let Some(path) = FixedPath::<128>::in_root("diag-poison.txt") else { return String::new() };
    fs::read_to_string(path.as_str()).unwrap_or_default()
}

#[cfg(feature = "diag")]
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn apply_diag_poison(found: &mut [GuardOutcome], state: &mut [GuardOutcome], found4: &mut [GuardOutcome], ui: &mut [GuardOutcome]) {
    let text = diag_poison_text();
    let poison = |rows: &[sites::Site], outcomes: &mut [GuardOutcome], name: &str| -> bool {
        match rows.iter().position(|row| row.name == name) {
            Some(i) => {
                outcomes[i] = GuardOutcome::Foreign { first_diff: 0 };
                true
            }
            None => false,
        }
    };
    for name in text.split(|c: char| c.is_whitespace() || c == ',').filter(|n| !n.is_empty()) {
        let hit = poison(&SITES, found, name) || poison(&STATE_SITES, state, name) || poison(&sites_pkg4::SITES, found4, name) || poison(&UI_SITES, ui, name) || sites_hitbox::SITES.iter().any(|row| row.name == name);
        diagnostic(format_args!("[TrainingRooms] diag_poison {name} found={hit}"));
    }
}

/// Read `save_states.json` on its own thread (stack from `model::thread_stacks`), joined here (the settings reader's rule).
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn load_save_states() -> SaveFile {
    crate::runtime::threads::run_joined(ThreadId::StateLoad, states::load_file).unwrap_or_else(|_| states::load_file())
}

/// The CPU behaviours (package 2): the per-frame wrapper first (the facts every other piece reads),
/// then each feature's hooks when all of its sites (and the core's) are usable. Returns whether the
/// core, SPEC-cpu-02's hooks, SPEC-cpu-03's hook and SPEC-cpu-04's hooks and SPEC-cpu-05's hooks were installed. The DI hook lives in the `common`
/// script module and is not installed (its 64-byte guard cannot be taken yet, see the report).
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn install_cpu(usable: &dyn Fn(&sites::Site) -> bool, mark: &mut dyn FnMut(&str, Written)) -> (bool, bool, bool, bool, bool, Option<&'static str>, bool, bool) {
    if !sites::feature_usable(Feature::CpuCore, usable) {
        return (false, false, false, false, false, Some("cpu_core_not_installed"), false, false);
    }
    cpu::install_frame();
    mark("cpu_on_frame_slot", Written::Far);
    let defence = sites::feature_usable(Feature::CpuDefence, usable);
    if defence {
        skyline::install_hook!(cpu::cpu_sdi_hook);
        skyline::install_hook!(cpu::cpu_stick_x_hook);
        skyline::install_hook!(cpu::cpu_stick_y_hook);
        skyline::install_hook!(cpu::cpu_clatter_start_hook);
        cpu::set_defence_on(true);
        for name in ["sdi_hook", "stick_x_hook", "stick_y_hook", "clatter_start_hook"] {
            mark(name, Written::Far);
        }
    }
    let ledge = sites::feature_usable(Feature::CpuLedge, usable);
    if ledge {
        skyline::install_hook!(cpu::cpu_transition_term_hook);
        cpu::set_ledge_on(true);
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
    // The game's CommonParams object, for the shield damage multiplier (SPEC-cpu-05 section 4).
    let shield_damage = sites::feature_usable(Feature::CpuShieldDamage, usable);
    if shield_damage {
        skyline::install_hook!(cpu::cpu_common_params_hook);
        mark("common_params_fill", Written::Near);
    }
    // The fixed camera (SPEC-cpu-04 section 5): the stage id, the stage table and the two camera hooks.
    let camera = sites::feature_usable(Feature::CpuCamera, usable);
    if camera {
        skyline::install_hook!(cpu::cpu_camera_mode_hook);
        skyline::install_hook!(cpu::cpu_camera_setup_hook);
        mark("camera_mode_hook", Written::Near);
        mark("fixed_camera_setup_hook", Written::Far);
    }
    (true, defence, ledge, tech, shield, common, shield_damage, camera)
}

/// SPEC-cpu-01 and SPEC-cpu-06: the command-flag, aerial-kind and attack-angle hooks. The frame wrapper
/// reads the fall facts only once `set_mash_on` is set, before any of these hooks can fire.
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
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
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn install_pkg4(poll_ok: bool, cpu_core_on: bool, notices_closure: bool, found4: &[GuardOutcome], written4: &mut [Written]) -> (bool, bool) {
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
    let combo_a = notices_closure && usable4("combo_display_a_pin") && usable4("combo_display_a");
    let combo_b = notices_closure && usable4("combo_display_b_pin") && usable4("combo_display_b");
    let stale = notices_closure && usable4("stale_moves_toggle");
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
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
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
#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn read_developer_ini() -> developer_ini::DeveloperIni {
    match FixedPath::<128>::in_root(storage::file::DEVELOPER_INI) {
        Some(path) => developer_ini::load(path.as_str()),
        None => developer_ini::DeveloperIni::default(),
    }
}


/// Read `settings.json` (or the fork's file name, once) on its own thread (stack from `model::thread_stacks`), joined here
/// (SPEC-core-04 §5). All of the file logic, including the recovery of a file that does not read
/// cleanly (the original kept as `settings.invalid-<n>.json`, the sanitized settings written to the
/// active path before the original is retired), is `model::settings_file::load`; this only runs it and
/// logs its notes.
/// The settings come back boxed (stream/stack): `Settings` is about 8 KB, and moved by value it took two 8 KB slots of the boot
/// frame on Skyline's 64 KiB thread.
fn load_settings() -> Box<Settings> {
    crate::runtime::threads::run_joined(ThreadId::SettingsLoad, load_settings_boxed).unwrap_or_else(|_| load_settings_boxed())
}

#[inline(never)] // stream/stack: its locals live only while it runs, not in boot's frame on Skyline's 64 KiB thread
fn load_settings_inner() -> Settings {
    let boot = settings_file::load(Path::new(storage::ROOT));
    for note in &boot.notes {
        diagnostic(format_args!("{note}"));
    }
    boot.settings
}

/// The settings boxed where they are read (the loader thread, or this fallback's own frame), never in boot's frame.
#[inline(never)]
fn load_settings_boxed() -> Box<Settings> {
    Box::new(load_settings_inner())
}

/// Package 6's roster cycle, Mii files and waiting-room engine (boot step 5): their inlined init state (two 8,040 B values in
/// 1a14480) stays in this frame while it runs instead of boot's frame on Skyline's 64 KiB thread (stream/stack).
#[inline(never)]
fn init_roster_cycle() {
    roster::init();
    mii::init();
    lobby::init();
}
/// The install line's buffer is sized for `install_capacity::MAX_FEATURE_REPORTS` feature reports: a boot that reports more does not
/// build (`INSTALL_LINE_CAPACITY` would no longer be the worst case).
fn feature_reports_fit<const N: usize>(_: &[FeatureReport; N]) {
    const { assert!(N <= crate::install_capacity::MAX_FEATURE_REPORTS, "raise install_capacity::MAX_FEATURE_REPORTS") }
}
