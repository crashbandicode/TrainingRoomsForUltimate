//! Boot's share of package 3: compare the save-state site windows with everything else, install the
//! hooks when every site is usable and the CPU core's per-frame wrapper (which calls the save-state step)
//! is installed, and report the rows. NOT YET SWITCH-COMPILED.

use crate::model::guard::GuardOutcome;
use crate::model::install_line::{SiteReport, Written};
use crate::model::sites_pkg3::STATE_SITES;
use crate::runtime::features::states;
use crate::runtime::game;

/// Number of rows.
pub const N: usize = STATE_SITES.len();

/// Compare every window (phase one: before anything is written).
pub fn check() -> [GuardOutcome; N] {
    let mut found = [GuardOutcome::Stock; N];
    for (slot, site) in found.iter_mut().zip(STATE_SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    found
}

/// Install the hooks when every row is usable and `frame_wrapper` (the CPU core's per-frame hook) is in.
/// Returns whether the feature is installed; marks the rows in `written`.
pub fn install(found: &[GuardOutcome; N], frame_wrapper: bool, written: &mut [Written; N]) -> bool {
    for (slot, outcome) in written.iter_mut().zip(found) {
        if outcome.usable() {
            *slot = Written::Checked;
        }
    }
    if !frame_wrapper || !found.iter().all(|f| f.usable()) {
        return false;
    }
    skyline::install_hook!(states::state_get_param_int_hook);
    skyline::install_hook!(states::state_set_dead_rumble_hook);
    skyline::install_hook!(states::state_req_quake_hook);
    skyline::install_hook!(states::state_play_se_hook);
    skyline::install_hook!(states::state_effect_req_hook);
    skyline::install_hook!(states::state_effect_req_follow_hook);
    skyline::install_hook!(states::state_no_star_ko_hook);
    skyline::install_hook!(states::state_joker_summon_hook);
    skyline::install_hook!(states::state_mac_meter_hook);
    skyline::install_hook!(states::state_wiifit_heal_hook);
    skyline::install_hook!(states::state_req_screen_hook);
    skyline::install_hook!(states::state_fighter_play_se_hook);
    for name in [
        "state_get_param_int",
        "state_set_dead_rumble",
        "state_req_quake",
        "state_play_se",
        "state_effect_req",
        "state_effect_req_follow",
        "state_no_star_ko",
        "state_joker_summon",
        "state_mac_meter",
        "state_wiifit_heal",
        "state_req_screen",
        "state_fighter_play_se",
    ] {
        if let Some(i) = STATE_SITES.iter().position(|s| s.name == name) {
            written[i] = Written::Far;
        }
    }
    true
}

/// The rows' install-line entries.
pub fn reports(found: &[GuardOutcome; N], written: &[Written; N]) -> [SiteReport; N] {
    std::array::from_fn(|i| SiteReport { site: STATE_SITES[i].name, found: found[i], written: written[i] })
}
