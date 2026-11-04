//! Boot's share of package 3: compare the save-state site windows with everything else, install the
//! hooks when every site is usable and the CPU core's per-frame wrapper (which calls the save-state step)
//! is installed, and report the rows. NOT YET SWITCH-COMPILED.

use crate::model::guard::GuardOutcome;
use crate::model::install_line::{SiteReport, Written};
use crate::model::sites_pkg3::STATE_SITES;
use crate::runtime::features::{state_restore, states};
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
/// Returns whether the feature is installed; marks the rows in `written`. When boot has set
/// `states::KIRBY_HATS` (the roster cycle started and the `state_kirby_opff` row is usable), Kirby's copy
/// routine hook installs on its own: it carries the Kirby hat step.
pub fn install(found: &[GuardOutcome; N], frame_wrapper: bool, written: &mut [Written; N]) -> bool {
    states::set_active(false);
    for (slot, outcome) in written.iter_mut().zip(found) {
        if outcome.usable() {
            *slot = Written::Checked;
        }
    }
    let mut mark = |name: &str, form: Written| {
        if let Some(i) = STATE_SITES.iter().position(|s| s.name == name) {
            written[i] = form;
        }
    };
    // The rows are installed together: they share the frame step and the constant table, and the
    // frame step calls the natives unguarded once it runs. The one exception is Kirby's copy routine:
    // its hook is the Kirby hats' per-Kirby call site (the fork's `SITE_KIRBY_OPFF`), which must not
    // depend on the other save-state rows; its own row still guards it.
    let all = frame_wrapper && found.iter().all(|f| f.usable());
    if all || states::KIRBY_HATS.load(std::sync::atomic::Ordering::Acquire) {
        skyline::install_hook!(states::state_kirby_opff_hook);
        mark("state_kirby_opff", Written::Far);
    }
    if !all {
        return false;
    }
    // Each hook with the row it writes, in install order (a replace hook is Skyline's far jump).
    macro_rules! far {
        ($($($hook:ident)::+ => $row:literal,)+) => { $( skyline::install_hook!($($hook)::+); mark($row, Written::Far); )+ };
    }
    far! {
        states::state_get_param_int_hook => "state_get_param_int",
        states::state_set_dead_rumble_hook => "state_set_dead_rumble",
        states::state_req_quake_hook => "state_req_quake",
        states::state_play_se_hook => "state_play_se",
        states::state_effect_req_hook => "state_effect_req",
        states::state_effect_req_follow_hook => "state_effect_req_follow",
        states::state_no_star_ko_hook => "state_no_star_ko",
        states::state_joker_summon_hook => "state_joker_summon",
        states::state_mac_meter_hook => "state_mac_meter",
        states::state_wiifit_heal_hook => "state_wiifit_heal",
        states::state_req_screen_hook => "state_req_screen",
        states::state_fighter_play_se_hook => "state_fighter_play_se",
    }
    state_restore::install_probability_hooks();
    for site in STATE_SITES.iter().filter(|s| s.name.contains("_prob_")) {
        mark(site.name, Written::Far);
    }
    far! {
        state_restore::state_pikmin_autonomy_hook => "state_pikmin_autonomy",
        states::state_effect_req_on_joint_hook => "state_effect_req_on_joint_hook",
    }
    // The inline hook at the Pokemon decide store is a one-word branch.
    skyline::install_hook!(states::state_pokemon_decide_hook);
    mark("state_pokemon_decide", Written::Near);
    states::set_active(true);
    true
}

/// The rows' install-line entries.
pub fn reports(found: &[GuardOutcome; N], written: &[Written; N]) -> [SiteReport; N] {
    std::array::from_fn(|i| SiteReport { site: STATE_SITES[i].name, found: found[i], written: written[i] })
}
