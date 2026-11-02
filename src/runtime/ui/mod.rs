//! Package 5: the menu on the game's own layout engine (SPEC-core-03, -05, -06, -07).
//!
//! `archive` substitutes the plugin's `info_training` layout archive when the game decompresses the
//! stock one (and, in the diag build, dumps the stock bytes for the host builder). The per-layout tick
//! anchors, the layout registry and the pane writes come in the later steps of the package.
//!
//! NOT YET SWITCH-COMPILED.

pub mod archive;

use crate::model::guard::GuardOutcome;
use crate::model::sites::Feature;
use crate::model::sites_ui::UI_SITES;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game;

/// What `install` found and did, per site, for the install line.
pub struct UiInstall {
    pub found: [GuardOutcome; UI_SITES.len()],
    pub archive_hook: bool,
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
                _ if install.found[i].usable() => Written::Checked,
                _ => Written::Skipped,
            },
        })
        .collect()
}

/// Compare the package's sites and install what is usable.
pub fn install() -> UiInstall {
    let mut found = [GuardOutcome::Stock; UI_SITES.len()];
    for (slot, site) in found.iter_mut().zip(UI_SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let usable = |name: &str| UI_SITES.iter().position(|s| s.name == name).is_some_and(|i| found[i].usable());
    let archive_hook = usable("ui_archive_alloc");
    if archive_hook {
        archive::install();
    } else {
        diagnostic(format_args!("[TrainingRooms] ui archive hook skipped: site not usable"));
    }
    let _ = Feature::Ui;
    UiInstall { found, archive_hook }
}