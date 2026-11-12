//! The fork's recording workflows as they ship again (operator 2026-11-05), through the carried models: a replay track's
//! target before and after its retried Mii import, and a Switch to the recorded identity (an exact Mii or an ordinary
//! fighter) from the replay window and from a named macro, with Keep beside it.

lib_unit_tests! {
use super::*;
use crate::engine::mii::native_values::RowIds;
use crate::engine::mii::replay::{ReplayMiis, Target};

const BRAWLER: i32 = 0x48;
const MARIO: i32 = 0x00;
const HERO: i32 = 0x12;

/// A replay header with one Mii Brawler on entry 0 (the native layout `mii::replay` parses).
fn capture() -> ReplayMiis {
    let mut bytes = vec![0u8; crate::engine::mii::replay::HEADER_LEN];
    bytes[4..8].copy_from_slice(b"CFMK");
    bytes[8..12].copy_from_slice(&crate::engine::mii::replay::VERSION.to_le_bytes());
    bytes[0x10..0x12].copy_from_slice(&(crate::engine::mii::replay::HEADER_LEN as u16).to_le_bytes());
    for index in 0..8 {
        bytes[0x468 + index * 0x260 + 0xa] = 2;
    }
    let at = 0x468;
    bytes[at + 8] = BRAWLER as u8;
    bytes[at + 0xa] = 0;
    for n in 0..88 {
        bytes[at + 0x150 + n] = n as u8;
    }
    bytes[at + 0x1a8..at + 0x1b6].copy_from_slice(&[65, 1, 1, 0, 0, 0, 4, 0, 37, 0, 1, 0, 0, 0]);
    ReplayMiis::parse_inner(&bytes).unwrap()
}

/// The qualified native row ids the runtime reads when it resolves (imports) the recorded Mii.
fn rows() -> RowIds {
    RowIds { outfit: 0x1300_011b_fef7_a5f7, hat: 0x5a00_000f_a9ef_a5ea }
}

/// The replay window's retry: a Mii track whose import was missed is `Ok(None)` (the window resolves it before it asks),
/// then that exact Mii; a failed import refuses the Switch but not the inputs.
fn a_mii_track_is_retried_then_loads_that_exact_mii() {
    let replay = capture();
    let raw = replay.for_track(0, BRAWLER as u32).unwrap();
    let missed = clip_target(BRAWLER, Ok(raw), None, false, true);
    assert_eq!(missed, Ok(None), "the missed import is retried");
    assert!(missed.is_ok(), "the track still offers a Switch while it resolves (the window's gate: target.is_ok())");
    let configuration = raw.resolve(rows()).unwrap();
    let resolved = clip_target(BRAWLER, Ok(raw), Some(configuration), false, true).unwrap().unwrap();
    assert_eq!(resolved, Target::mii(configuration));
    assert_eq!(resolved.configuration(), Some(configuration), "that exact Mii, not its class");
    assert!(resolved.ready(BRAWLER as u32, Some(configuration)));
    assert!(!resolved.ready(BRAWLER as u32, None), "a Brawler that is not this Mii is not ready");
    // The retry failed: no Switch, the inputs still preview on the current CPU.
    assert_eq!(clip_target(BRAWLER, Ok(raw), None, true, true), Err("Recorded Mii metadata is unavailable".into()));
    assert!(clip_target(BRAWLER, Err("Capture entry has no matching Mii configuration"), None, false, true).is_err());
    assert_eq!(clip_target(0x49, Ok(raw), None, false, true), Err("Capture fighter identity changed".into()));
    assert!(clip_target(BRAWLER, Ok(raw), Some(configuration), false, false).is_err(), "no Mii loader in this build");
}

/// An ordinary track's Switch loads its recorded fighter, as the fork did; a fighter the trio cannot load is refused.
fn an_ordinary_track_switches_to_its_recorded_fighter() {
    assert_eq!(clip_target(MARIO, Err("an ordinary track has no Mii"), None, false, true), Ok(Some(Target::fighter(MARIO as u32))));
    assert_eq!(clip_target(HERO, Err("unused"), None, false, false), Ok(Some(Target::fighter(HERO as u32))), "no Mii loader needed");
    assert!(Target::fighter(MARIO as u32).ready(MARIO as u32, None));
    assert!(clip_target(0x7fff, Err("unused"), None, false, true).is_err());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_mii_track_is_retried_then_loads_that_exact_mii", a_mii_track_is_retried_then_loads_that_exact_mii as fn()),
        ("an_ordinary_track_switches_to_its_recorded_fighter", an_ordinary_track_switches_to_its_recorded_fighter as fn()),
    ]);
}
}
