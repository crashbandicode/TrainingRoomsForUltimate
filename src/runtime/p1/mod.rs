//! Package 6 CPU-as-P1 (Q11 fallback): the carried `cpu_pilot.rs`, the thin adapter that answers the names it takes from outside
//! (`compat`), and the boot, frame, input and settings call-ins. Switch only. `compat` and the call-ins are the only code here that
//! is not carried.
//!
//! The fallback runs only while the standalone CPU pilot plugin is ABSENT (decision Q11, the fork's handoff design): with the pilot
//! present nothing is installed and P1 is driven once, by the pilot. The hook is the carried seam row `cpu_pilot.ai_process_begin`
//! (`main+0x30c710`, guarded by `seam_install::gate`); the game calls the carried code makes through `smash::` are guarded by the
//! rows of `sites_pkg6` and of packages 2-4 (`tests/pkg6_smash_calls.rs`).

use smash::app::BattleObjectModuleAccessor;

use crate::model::guard::GuardOutcome;
use crate::model::input_delay::MappedInputs;
use crate::model::install_line::{SiteReport, Written};
use crate::model::settings::Snapshot;
use crate::model::sites_pkg6::SITES;
use crate::runtime::game::{self, natives};
use crate::runtime::ui::state::PILOT_PRESENT;

pub(crate) mod compat;
pub(crate) mod cpu_pilot;

/// Number of rows of `sites_pkg6`.
pub(crate) const N: usize = SITES.len();

/// Compare every row of `sites_pkg6` (before anything is written).
pub(crate) fn check() -> [GuardOutcome; N] {
    std::array::from_fn(|i| SITES[i].outcome(&game::read_window(SITES[i].offset)))
}

/// Install the fallback when its prerequisites passed and the standalone pilot is absent. Returns whether the pilot's hook is
/// installed, why not (the install line's `reason=`), and the per-row install state.
pub(crate) fn install(found: &[GuardOutcome; N], prerequisites: bool) -> (bool, Option<&'static str>, [Written; N]) {
    let mut written = [Written::Skipped; N];
    for (slot, outcome) in written.iter_mut().zip(found) {
        if outcome.usable() {
            *slot = Written::Checked;
        }
    }
    if PILOT_PRESENT.load(std::sync::atomic::Ordering::Acquire) {
        return (false, Some("standalone_pilot_present"), written);
    }
    if !found.iter().all(|f| f.usable()) {
        return (false, Some("site_guard"), written);
    }
    if !prerequisites {
        return (false, Some("prerequisite_feature"), written);
    }
    cpu_pilot::init();
    if cpu_pilot::installed() {
        (true, None, written)
    } else {
        (false, Some("seam_gate"), written)
    }
}

/// The rows of `sites_pkg6` for the install line.
pub(crate) fn reports(found: &[GuardOutcome; N], written: &[Written; N]) -> [SiteReport; N] {
    std::array::from_fn(|i| SiteReport { site: SITES[i].name, found: found[i], written: written[i] })
}

/// The Q11 fallback rows' values, from every settings publish (`p1_cpu_control` On, `p1_cpu_level` 1..=9).
pub(crate) fn publish(snapshot: &Snapshot) {
    let enabled = snapshot.get("p1_cpu_control").is_some_and(|row| row.first().is_some_and(|v| *v != 0));
    let level = snapshot.get("p1_cpu_level").and_then(|row| row.iter().position(|v| *v != 0)).map(|index| (index + 1) as u8);
    cpu_pilot::menu_settings_changed(enabled, level);
}

/// A fighter's frame has run (called from the CPU core's per-fighter callback): the pilot's per-frame step.
///
/// # Safety
/// `fighter` is the fighter object the game just ran a frame of.
pub(crate) unsafe fn frame(fighter: usize) {
    if !cpu_pilot::installed() {
        return;
    }
    let accessor = natives::fighter_accessor(fighter);
    if accessor != 0 {
        cpu_pilot::on_frame(&mut *(accessor as *mut BattleObjectModuleAccessor));
    }
}

/// P1's mapped input record after the delay: the pilot writes its pad here.
///
/// # Safety
/// `out` is the 8-byte record the game wrote for `slot`.
pub(crate) unsafe fn input(slot: i32, out: *mut MappedInputs) {
    if cpu_pilot::installed() {
        cpu_pilot::input(slot, out);
    }
}