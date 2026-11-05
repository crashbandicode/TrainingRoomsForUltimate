//! Which of the fork's recording workflows this release ships, and the decisions they share between the replay window
//! (`runtime::macros::replay_capture`) and the named macros (`runtime::macros::named_macros`). Pure and host-tested; the
//! runtime asks these with `RELEASE`.
//!
//! Operator 2026-11-05: "yes port enable the fork's Replay Window retry / exact-Mii named macros". Enabled:
//! - the replay window (Recording > Named macros > Replay window): choosing a capture's track retries a missed Mii import
//!   (`clip_target` answers `Ok(None)`, the runtime resolves the recorded Mii) and a Switch loads that exact Mii on the CPU;
//! - named macros that carry an exact Mii: loading one may restore that Mii on the CPU (Switch) or play on the current
//!   CPU (Keep), as the fork did.
//!
//! Still closed (`fighter_switch`, `setups`, `placement`), the rest of the RC's frozen workflows: switching the CPU to a
//! recorded ordinary (non-Mii) fighter from either place, the Setup library and Edit placement.

use crate::engine::mii::lifetime::Configuration;
use crate::engine::mii::replay::{RawMii, SelectionSource, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Workflows {
    /// The replay window opens and takes its actions (track choice, preview, crop, export).
    pub replay_window: bool,
    /// A named macro that carries an exact Mii loads, with its Switch / Keep choice.
    pub exact_mii_named: bool,
    /// The CPU may be switched to a recorded ordinary fighter (a replay track, a named macro).
    pub fighter_switch: bool,
    /// The Setup library (save and bind a Training setup).
    pub setups: bool,
    /// Edit placement (nudge a loaded macro's start).
    pub placement: bool,
}

/// What this release ships.
pub const RELEASE: Workflows =
    Workflows { replay_window: true, exact_mii_named: true, fighter_switch: false, setups: false, placement: false };

/// Whether `source` may load its recorded identity on the CPU. An exact Mii needs the Mii loader
/// (`mii-ordinary-cpu-alpha`) and its workflow; an ordinary fighter needs `fighter_switch`.
pub fn may_load_on_cpu(w: Workflows, source: SelectionSource, exact_mii: bool, mii_loader: bool) -> bool {
    if exact_mii {
        mii_loader
            && match source {
                SelectionSource::Clip => w.replay_window,
                SelectionSource::NamedMacro => w.exact_mii_named,
            }
    } else {
        w.fighter_switch
    }
}

/// Why a named macro may not be loaded now, if it may not. `target` is its playback target; `matches`: the recorded
/// identity is already the settled CPU; `cpu_present`: a CPU exists (boot may restore a selection before one does, and
/// its playback then waits for the recorded identity).
pub fn named_load_refusal(w: Workflows, target: Target, matches: bool, cpu_present: bool) -> Option<&'static str> {
    if target.configuration().is_some() {
        return (!w.exact_mii_named).then_some("Exact Mii named loading is unavailable in this release");
    }
    (!w.fighter_switch && cpu_present && !matches).then_some("Choose the recorded CPU fighter before loading this macro")
}

/// Whether a loaded named macro may play on the current CPU though it is not the recorded identity (the Keep answer).
pub fn keep_allowed(w: Workflows, target: Option<Target>) -> bool {
    w.fighter_switch || (w.exact_mii_named && target.is_some_and(|t| t.configuration().is_some()))
}

/// The identity a replay track's Switch would load. `raw` is the track's recorded Mii (an error when the capture has
/// none for it); `resolved` is its exact configuration once resolved; `resolve_failed`: resolving it failed.
/// `Ok(None)` is a Mii whose import was missed or not done yet: the runtime retries it (`mii_replay::resolve`) before
/// it asks or switches. An ordinary fighter is its kind; a fighter the trio cannot load is refused.
pub fn clip_target(
    kind: i32,
    raw: Result<&RawMii, &str>,
    resolved: Option<Configuration>,
    resolve_failed: bool,
    mii_loader: bool,
) -> Result<Option<Target>, String> {
    if crate::engine::mii::inventory::MiiClass::from_kind(kind as u32).is_none() {
        if crate::engine::swap::trio::TrioFighter::from_kind(kind as u32).is_none() {
            return Err("Preview fighter is not available".into());
        }
        return Ok(Some(Target::fighter(kind as u32)));
    }
    let raw = raw.map_err(str::to_owned)?;
    if raw.class().kind() != kind as u32 {
        return Err("Capture fighter identity changed".into());
    }
    if resolve_failed {
        return Err("Recorded Mii metadata is unavailable".into());
    }
    if !mii_loader {
        return Err("Replay Mii loading is unavailable in this build".into());
    }
    Ok(resolved.map(Target::mii))
}

/// Whether a replay track offers a Switch: an unresolved Mii will be an exact Mii once its import is retried.
pub fn clip_switchable(w: Workflows, target: &Result<Option<Target>, String>, mii_loader: bool) -> bool {
    match target {
        Ok(None) => may_load_on_cpu(w, SelectionSource::Clip, true, mii_loader),
        Ok(Some(t)) => may_load_on_cpu(w, SelectionSource::Clip, t.configuration().is_some(), mii_loader),
        Err(_) => false,
    }
}

#[cfg(test)]
#[path = "workflows_tests.rs"]
mod tests;
