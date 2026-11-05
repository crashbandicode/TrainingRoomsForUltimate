//! The identity a replay track's Switch loads on the CPU (the replay window, `runtime::macros::replay_capture`): the
//! fork's `Editor::target_for`, moved here unchanged so its decisions are host-tested (operator 2026-11-05: "yes port
//! enable the fork's Replay Window retry / exact-Mii named macros", then "Switch CPU to recorded fighter, Setup library,
//! Edit placement").
//!
//! `Ok(None)` is a Mii track whose exact configuration is not resolved yet (its import was missed, or not done in this
//! window): the window retries it (`mii_replay::resolve`) before it asks. An ordinary fighter is its kind, which a Switch
//! loads on the CPU as the fork did.

#[cfg(any(test, target_os = "switch"))]
use crate::engine::mii::{lifetime::Configuration, replay::{RawMii, Target}};

/// The identity a Switch would load. `raw` is the track's recorded Mii (an error when the capture has none for it);
/// `resolved` is its exact configuration once resolved; `resolve_failed`: resolving it failed; `mii_loader`: the build
/// carries the Mii loader (`mii-ordinary-cpu-alpha`). Preview and Keep never need it. (`engine::swap`, the trio, exists
/// for the Switch and the tests only, as the window does.)
#[cfg(any(test, target_os = "switch"))]
pub fn clip_target(
    kind: i32,
    raw: Result<&RawMii, &str>,
    resolved: Option<Configuration>,
    resolve_failed: bool,
    mii_loader: bool,
) -> Result<Option<Target>, String> {
    if crate::engine::mii::inventory::MiiClass::from_kind(kind as u32).is_none() {
        if crate::engine::swap::trio::TrioFighter::from_kind(kind as u32).is_none() {
            return Err("Preview fighter is not available".into()); // restriction: replay-preview-fighter
        }
        return Ok(Some(Target::fighter(kind as u32)));
    }
    let raw = raw.map_err(str::to_owned)?;
    if raw.class().kind() != kind as u32 {
        return Err("Capture fighter identity changed".into());
    }
    if resolve_failed {
        return Err("Recorded Mii metadata is unavailable".into()); // restriction: replay-mii-metadata
    }
    if !mii_loader {
        return Err("Replay Mii loading is unavailable in this build".into()); // restriction: replay-mii-loader-build
    }
    Ok(resolved.map(Target::mii))
}

#[cfg(test)]
#[path = "replay_target_tests.rs"]
mod tests;
