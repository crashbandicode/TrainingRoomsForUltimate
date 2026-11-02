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
pub mod hud;
pub mod input;
pub mod pane;
pub mod registry;
pub mod state;
pub mod tick;
pub mod view;
pub mod worker;

use crate::model::guard::GuardOutcome;
use crate::model::settings::Settings;
use crate::model::sites_ui::{self, UI_SITES};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game;

/// What `install` found and did, per site, for the install line.
pub struct UiInstall {
    pub found: [GuardOutcome; UI_SITES.len()],
    pub archive_hook: bool,
    pub anchors: bool,
}

/// The install-line rows of the package's sites.
pub fn reports(install: &UiInstall) -> Vec<crate::model::install_line::SiteReport> {
    use crate::model::install_line::{SiteReport, Written};
    UI_SITES
        .iter()
        .enumerate()
        .map(|(i, site)| SiteReport {
            site: site.name,
            found: install.found[i],
            written: match site.name {
                "ui_archive_alloc" if install.archive_hook => Written::Near,
                "ui_pass_calculate" | "ui_register_calculate" if install.anchors => Written::Near,
                _ if install.found[i].usable() => Written::Checked,
                _ => Written::Skipped,
            },
        })
        .collect()
}

/// Compare the package's sites and install what is usable. `input_ready`: the final-input-mapping caller
/// hooks (Input Delay's four sites) are installed, which is where the menu reads Player 1.
pub fn install(settings: &Settings, input_ready: bool) -> UiInstall {
    let mut found = [GuardOutcome::Stock; UI_SITES.len()];
    for (slot, site) in found.iter_mut().zip(UI_SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let usable = |name: &str| UI_SITES.iter().position(|s| s.name == name).is_some_and(|i| found[i].usable());
    if usable("ui_set_text") {
        pane::set_text_setter(game::text_base() + sites_ui::UI_SET_TEXT_OFFSET);
    }
    let archive_hook = usable("ui_archive_alloc");
    if archive_hook {
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
    let anchors = archive_hook
        && input_ready
        && usable("ui_pass_calculate")
        && usable("ui_register_calculate")
        && usable("ui_manager_global")
        && usable("ui_set_text")
        && slot.is_some();
    if anchors {
        registry::set_manager_slot(slot.unwrap_or_default() as usize);
        worker::start(settings.clone());
        anchors::install();
        diagnostic(format_args!("[Ui] anchors installed manager_slot=main+{:#x}", slot.unwrap_or_default() - game::text_base() as u64));
    } else {
        diagnostic(format_args!(
            "[Ui] anchors skipped archive={archive_hook} input={input_ready} slot={}",
            slot.is_some()
        ));
    }
    UiInstall { found, archive_hook, anchors }
}