//! Package 5: the menu on the game's own layout engine (SPEC-core-03, -05, -06, -07).
//!
//! `archive` substitutes the plugin's `info_training` layout archive when the game decompresses the
//! stock one; `anchors` runs the per-layout tick (`tick`), which resolves the menu's panes (`pane`,
//! `registry`) and draws the worker's frame (`view`); `input` turns Player 1's controller into events for
//! the menu worker (`worker`), which owns the model. `hud` lets the other packages draw their panels from
//! the same tick.
//!
//! NOT YET SWITCH-COMPILED.

pub mod anchors;
pub mod archive;
pub mod chips;
pub mod combos;
pub mod effect_texture;
pub mod external;
pub mod facts;
pub mod hud;
pub mod input;
pub mod menu_app;
pub mod native_pause;
pub mod pane;
#[cfg(feature = "diag")]
pub mod probe;
pub mod registry;
pub mod requests;
pub mod setups;
pub mod state;
pub mod texture;
pub mod thumbs;
#[cfg(feature = "texture-probe")]
pub mod texture_probe;
pub mod tick;
pub mod view;
pub mod worker;

use crate::model::guard::GuardOutcome;
use crate::model::recording::PilotStateExport;
use crate::model::settings::Settings;
use crate::model::sites_ui::{self, UI_SITES};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game;

/// What `install` found and did, per site, for the install line.
pub struct UiInstall {
    pub found: [GuardOutcome; UI_SITES.len()],
    pub archive_hook: bool,
    pub anchors: bool,
    /// The u4 texture helper's guards (`sites_u4::SITES`).
    pub texture_found: [GuardOutcome; crate::model::sites_u4::SITES.len()],
    /// The u4b effect-texture helper's guards (`sites_u4b::SITES`).
    pub effect_found: [GuardOutcome; crate::model::sites_u4b::SITES.len()],
}

/// The install-line rows of the package's sites.
pub fn reports(install: &UiInstall) -> Vec<crate::model::install_line::SiteReport> {
    use crate::model::install_line::{SiteReport, Written};
    let texture = crate::model::sites_u4::SITES.iter().enumerate().map(|(i, site)| SiteReport {
        site: site.name,
        found: install.texture_found[i],
        written: if install.texture_found[i].usable() { Written::Checked } else { Written::Skipped },
    });
    UI_SITES
        .iter()
        .enumerate()
        .map(|(i, site)| SiteReport {
            site: site.name,
            found: install.found[i],
            written: match site.name {
                "ui_archive_alloc" if install.archive_hook => Written::Near,
                "ui_pass_calculate" | "ui_register_calculate" if anchors::installed() => Written::Near,
                _ if install.found[i].usable() => Written::Checked,
                _ => Written::Skipped,
            },
        })
        .chain(texture)
        .chain(crate::model::sites_u4b::SITES.iter().enumerate().map(|(i, site)| SiteReport {
            site: site.name,
            found: install.effect_found[i],
            written: if install.effect_found[i].usable() { Written::Checked } else { Written::Skipped },
        }))
        .collect()
}

/// Compare the package's sites and install what is usable. `input_ready`: the final-input-mapping caller
/// hooks (Input Delay's four sites) are installed, which is where the menu reads Player 1.
/// The standalone CPU pilot plugin's state export ssbu_cpu_pilot_state_v1, when that plugin is loaded (Skyline loads every
/// plugin before any main runs, so presence is final at boot).
pub(crate) fn pilot_export() -> Option<PilotStateExport> {
    let mut address = 0usize;
    // SAFETY: a plain symbol lookup with a NUL-terminated name; a found address is that export, whose ABI
    // `PilotStateExport` declares (ssbu-cpu-pilot `src/runtime.rs:510-538`).
    unsafe { skyline::nn::ro::LookupSymbol(&mut address, b"ssbu_cpu_pilot_state_v1\0".as_ptr()) };
    (address != 0).then(|| unsafe { core::mem::transmute::<usize, PilotStateExport>(address) })
}

/// The menu's session ends with Training (the fork's `clear_session_state`): the menu is closed without a
/// commit, its close fade and the pause flag are forgotten, the `Owners::MENU` bit is released, the input
/// adapter is reset and the worker drops its model. Called where the CPU runtime clears the other
/// session state (`features::recorder::session_clear`).
pub fn clear_session_state() {
    use std::sync::atomic::Ordering;
    state::OPEN.store(false, Ordering::Release);
    state::CLOSING.store(false, Ordering::Release);
    state::CLOSE_COUNT.store(0, Ordering::Release);
    state::MENU_SHOWN.store(false, Ordering::Release);
    crate::runtime::features::input_delay::retire_native_pause();
    menu_app::note_open();
    crate::runtime::published::INPUT_OWNERS.fetch_and(!crate::model::input_delay::Owners::MENU.bits(), Ordering::AcqRel);
    input::reset_session();
    facts::forget_frame_facts();
    worker::push_reset();
    state::FRAME_SEQ.fetch_add(1, Ordering::Release);
}

pub fn install(settings: &Settings, input_ready: bool, found: [GuardOutcome; UI_SITES.len()]) -> UiInstall {
    worker::remember(settings.clone());
    state::PILOT_PRESENT.store(pilot_export().is_some(), std::sync::atomic::Ordering::Release);
    #[cfg(feature = "diag")]
    probe::install();
    // Authoritative pre-write outcomes from boot. The native observer has now
    // written our own anchor hooks; rereading those as pristine self-rejects UI.
    let usable = |name: &str| UI_SITES.iter().position(|s| s.name == name).is_some_and(|i| found[i].usable());
    if usable("ui_set_text") {
        pane::set_text_setter(game::text_base() + sites_ui::UI_SET_TEXT_OFFSET);
    }
    if usable("ui_is_ready_go") {
        game::allow_ready_go();
    }
    let archive_hook = usable("ui_archive_alloc");
    if archive_hook {
        // R2-7: everything a hand-out would otherwise allocate in the hook (the retire thread, the parked list) is made here.
        archive::prepare();
        archive::install();
    } else {
        diagnostic(format_args!("[TrainingRooms] ui archive hook skipped: site not usable"));
    }
    let word = |offset: usize| unsafe { ((game::text_base() + offset) as *const u32).read_volatile() };
    let slot = sites_ui::decode_adrp_ldr(
        (game::text_base() + sites_ui::UI_MANAGER_GLOBAL_OFFSET) as u64,
        word(sites_ui::UI_MANAGER_GLOBAL_OFFSET),
        word(sites_ui::UI_MANAGER_GLOBAL_OFFSET + 4),
    );
    // Both anchors, the manager global and the input callers, or none: a menu that draws without input (or
    // the other way round) is worse than no menu (SPEC-core-05 §6: the anchors are skipped together).
    let anchors = crate::model::readiness::renderer_anchors_ready(
        usable, input_ready, anchors::installed(), slot.is_some());
    if anchors {
        registry::set_manager_slot(slot.unwrap_or_default() as usize);
        worker::start(settings.clone());
        state::INSTALLED.store(true, std::sync::atomic::Ordering::Release);
        diagnostic(format_args!("[Ui] anchors installed manager_slot=main+{:#x} pilot_present={}", slot.unwrap_or_default() - game::text_base() as u64, state::PILOT_PRESENT.load(std::sync::atomic::Ordering::Acquire)));
    } else {
        diagnostic(format_args!(
            "[Ui] anchors skipped archive={archive_hook} input={input_ready} slot={}",
            slot.is_some()
        ));
    }
    // The game-file texture helper (u4): the stage grid's thumbnails ride on it; without it the tiles keep their text.
    let texture_found = texture::install();
    // The resident effect-texture helper (u4b): the DI / SDI chips ride on it; without it they keep the clean room's arrows.
    let effect_found = effect_texture::install();
    UiInstall { found, archive_hook, anchors, texture_found, effect_found }
}
