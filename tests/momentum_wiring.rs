//! PLAY-MOMENTUM (primary 2026-11-08 23:03Z, reading (b); cov-rec #70): Play applies a macro's recorded start speeds
//! when the Placement screen's Momentum key reads "Keep recorded speed", and starts at rest with "Start at rest". The
//! start speeds and the start save are taken when Record starts (RECORDING-REDESIGN: "Start save: the save state this
//! macro starts at"). The runtime is Switch-only, so the call-graph scanner (`tests/common/callgraph.rs`) reads it; the
//! decisions are host-tested in `rr_host` (`a_recording_keeps_where_and_how_fast_it_started`,
//! `play_gives_the_recorded_start_speed_only_with_momentum_kept`).

#[path = "common/callgraph.rs"]
mod callgraph;

use std::path::Path;

use callgraph::{Graph, Tok};

fn graph() -> Graph {
    Graph::scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime"), &[])
}

fn one(g: &Graph, key: &str) -> usize {
    let found = g.find(key);
    assert_eq!(found.len(), 1, "{key}: {found:?}");
    found[0]
}

/// Whether `key`'s body names `ident`.
fn names(g: &Graph, key: &str, ident: &str) -> bool {
    g.funcs[one(g, key)].body.iter().any(|t| matches!(t, Tok::Ident(s) if s == ident))
}

/// The record chord that starts a recording posts both fighters' live pose and speeds before the capture begins; the
/// worker's live pose and the start pose are one reading.
#[test]
fn a_record_start_posts_the_start_pose_and_speeds() {
    let g = graph();
    assert!(names(&g, "recording_host::on_chords", "note_record_start"), "the start is posted at the chord");
    assert!(names(&g, "recording_host::on_chords", "capture_role"), "only when a recording starts");
    assert!(names(&g, "recording_host::record_start", "live_pose"));
    assert!(names(&g, "recording_host::live_speed", "get_sum_speed_x") && names(&g, "recording_host::live_speed", "get_sum_speed_y"));
    assert!(names(&g, "recording_host::live_speed", "ready"), "the kinetic rows are the named-macro unit's (R2-1)");
    assert!(names(&g, "states::live_saved", "live_pose"), "the worker's live pose is the same reading");
}

/// The role's first input after the warp gets the start speed, on the game frame that takes it.
#[test]
fn the_first_input_after_the_warp_gets_the_start_speed() {
    let g = graph();
    assert!(names(&g, "recording_host::on_player_poll", "apply_start_speed"));
    assert!(names(&g, "recording_host::take_cpu_frame", "apply_start_speed"));
    for ident in ["take_start_speed", "ready", "fighter_accessor", "set_speed", "get_energy", "lr", "situation_kind"] {
        assert!(names(&g, "recording_host::apply_start_speed", ident), "apply_start_speed names {ident}");
    }
}

/// Every called game function is guarded (playbook): the two `set_speed` thunks are rows of package 4's table, in the
/// named-macro unit whose readiness gates the calls.
#[test]
fn the_set_speed_thunks_are_guarded_rows() {
    let table = include_str!("../src/model/sites_pkg4.rs");
    for (name, offset) in [("kinetic_energy_normal_set_speed", "0x20f8320"), ("kinetic_energy_gravity_set_speed", "0x2120230")] {
        let at = table.find(&format!("name: \"{name}\"")).unwrap_or_else(|| panic!("no row {name}"));
        let row = &table[at..at + 200];
        assert!(row.contains("Feature::NamedMacros") && row.contains(offset), "{name}: {row}");
    }
}
