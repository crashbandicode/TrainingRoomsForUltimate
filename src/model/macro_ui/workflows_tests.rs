//! The release's recording workflows (operator 2026-11-05: the replay window's retry and exact-Mii named macros) through
//! the carried models: the replay track's target before and after the retried import, and a named macro that carries a
//! Mii from the clip that made it to the CPU request that restores it.

use super::*;
use crate::engine::macro_ui::macro_panel::{Effect, FighterOrigin, LibraryRow, LoadContext, MacroUi, Pad};
use crate::engine::macros::{InputFrame, MacroRecording, StartState};
use crate::engine::mii::native_values::RowIds;
use crate::engine::mii::replay::ReplayMiis;

const BRAWLER: i32 = 0x48;
const MARIO: i32 = 0x00;

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

fn frames() -> Vec<InputFrame> {
    vec![InputFrame { buttons: 0x1, lstick_x: 0, lstick_y: 40, rstick_x: 0, rstick_y: 0 }]
}

fn context(cpu_kind: i32, matches: bool, can_switch: bool) -> LoadContext {
    LoadContext {
        cpu_kind: Some(cpu_kind),
        matches,
        prompt_enabled: true,
        can_switch,
        revision: 3,
        recorded_label: "Mii Brawler (recorded Mii)".into(),
        current_label: "Mario".into(),
    }
}

fn pick(ui: &mut MacroUi) {
    let rows = ["id0", "id1"].map(|id| LibraryRow { id: id.into(), label: id.into() });
    ui.open_library_titled(rows.to_vec(), 0, "Named Library", Some("Record Slot 1".into()));
    ui.on_pad(Pad::Right);
    let Effect::Choose(id) = ui.on_pad(Pad::Confirm) else { panic!("expected choose") };
    assert_eq!(id, "id1");
}

#[test]
fn the_release_ships_the_replay_window_and_exact_mii_named_macros_only() {
    assert_eq!(
        RELEASE,
        Workflows { replay_window: true, exact_mii_named: true, fighter_switch: false, setups: false, placement: false }
    );
    use SelectionSource::{Clip, NamedMacro};
    for source in [Clip, NamedMacro] {
        assert!(may_load_on_cpu(RELEASE, source, true, true), "{source:?}: an exact Mii loads on the CPU");
        assert!(!may_load_on_cpu(RELEASE, source, true, false), "{source:?}: not without the Mii loader");
        assert!(!may_load_on_cpu(RELEASE, source, false, true), "{source:?}: no switch to an ordinary fighter");
    }
    let closed = Workflows { replay_window: false, exact_mii_named: false, ..RELEASE };
    assert!(!may_load_on_cpu(closed, Clip, true, true) && !may_load_on_cpu(closed, NamedMacro, true, true));
    let open = Workflows { fighter_switch: true, ..RELEASE };
    assert!(may_load_on_cpu(open, Clip, false, false));
}

/// The replay window's retry: a Mii track whose import was missed is `Ok(None)` (the runtime resolves it before it asks),
/// then the exact Mii; a failed import refuses the switch but not the inputs; an ordinary track is its fighter, which
/// this release does not switch to.
#[test]
fn a_mii_track_is_retried_then_loads_that_exact_mii() {
    let replay = capture();
    let raw = replay.for_track(0, BRAWLER as u32).unwrap();
    let missed = clip_target(BRAWLER, Ok(raw), None, false, true);
    assert_eq!(missed, Ok(None), "the missed import is retried");
    assert!(clip_switchable(RELEASE, &missed, true), "the track still offers a Switch while it resolves");
    let configuration = raw.resolve(rows()).unwrap();
    let resolved = clip_target(BRAWLER, Ok(raw), Some(configuration), false, true).unwrap().unwrap();
    assert_eq!(resolved, Target::mii(configuration));
    assert_eq!(resolved.configuration(), Some(configuration), "that exact Mii, not its class");
    assert!(resolved.ready(BRAWLER as u32, Some(configuration)));
    assert!(!resolved.ready(BRAWLER as u32, None), "a Brawler that is not this Mii is not ready");
    assert!(clip_switchable(RELEASE, &Ok(Some(resolved)), true));
    // The retry failed: no Switch, the inputs still preview on the current CPU.
    let failed = clip_target(BRAWLER, Ok(raw), None, true, true);
    assert_eq!(failed, Err("Recorded Mii metadata is unavailable".into()));
    assert!(!clip_switchable(RELEASE, &failed, true));
    // Without a recorded Mii for the track, or for another class.
    assert!(clip_target(BRAWLER, Err("Capture entry has no matching Mii configuration"), None, false, true).is_err());
    assert_eq!(clip_target(0x49, Ok(raw), None, false, true), Err("Capture fighter identity changed".into()));
    assert!(clip_target(BRAWLER, Ok(raw), Some(configuration), false, false).is_err(), "no Mii loader in this build");
    // An ordinary track: its fighter, kept on the current CPU in this release.
    let mario = clip_target(MARIO, Err("unused"), None, false, true);
    assert_eq!(mario, Ok(Some(Target::fighter(MARIO as u32))));
    assert!(!clip_switchable(RELEASE, &mario, true));
    assert!(clip_target(0x7fff, Err("unused"), None, false, true).is_err(), "a fighter the trio cannot load");
}

/// A named macro carrying a Mii, from the clip's export to the CPU request that restores the Mii: the file keeps the
/// exact configuration through JSON; the release loads it, asks, and Switch requests that Mii's fighter; the runtime's
/// request is eligible; Keep plays on the current CPU.
#[test]
fn a_named_macro_carries_its_mii_and_restores_it() {
    let replay = capture();
    let configuration = replay.for_track(0, BRAWLER as u32).unwrap().resolve(rows()).unwrap();
    let mut recording = MacroRecording::new("Brawler combo", "13.0.5", StartState::neutral(BRAWLER, 0, 1.0), frames()).unwrap();
    recording.set_mii(configuration).unwrap();
    let file = serde_json::to_string(&recording).unwrap();
    let loaded: MacroRecording = serde_json::from_str(&file).unwrap();
    let target = loaded.playback_target().unwrap();
    assert_eq!(target.configuration(), Some(configuration), "the file keeps the exact Mii");
    assert_eq!(named_load_refusal(RELEASE, target, false, true), None, "a Mii macro loads on another CPU");
    let can_switch = may_load_on_cpu(RELEASE, SelectionSource::NamedMacro, target.configuration().is_some(), true);
    assert!(can_switch);

    // Switch: the recorded Mii's fighter is requested; playback waits for that exact Mii.
    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), loaded.clone(), context(MARIO, false, can_switch)).unwrap();
    assert_eq!(ui.fighter_prompt(), Some((FighterOrigin::Named, 3)));
    ui.on_pad(Pad::Down);
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::NeedCharacter(BRAWLER)));
    assert_eq!(ui.claim_character_request(), Some(BRAWLER));
    assert_eq!(ui.loaded_target(), Some(Target::mii(configuration)));
    assert!(!ui.plays_current());
    assert!(!ui.playback_allowed(), "nothing plays before the Mii is on the CPU");
    assert!(may_load_on_cpu(RELEASE, SelectionSource::NamedMacro, true, true), "the runtime's request is eligible");

    // Keep: the inputs play on the current CPU; the file's Mii is untouched.
    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), loaded, context(MARIO, false, can_switch)).unwrap();
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::None));
    assert!(ui.plays_current());
    assert!(keep_allowed(RELEASE, ui.loaded_target()));
    assert_eq!(ui.loaded_target().unwrap().configuration(), Some(configuration));
    assert_eq!(ui.claim_character_request(), None);
}

/// The Eden fixture of `artifacts/phase23/rc4-combined/rows-replay`: a replay capture whose entry 1 is a Mii Gunner with
/// REPLAY185's real companion (outfit 75, hat 125, colour 0, voice 37, specials 2221; `tests/mii_replay185_headgear.rs`)
/// beside a Mario on entry 0, and a named macro carrying that Gunner's exact Mii. Written only on request:
/// `RW_FIXTURE_DIR=<dir> cargo test --lib write_replay_window_fixture -- --ignored` makes `<dir>/replay_captures/` and
/// `<dir>/macros/`, copied into the profile's `sd:/ultimate/TrainingRooms/`.
#[test]
#[ignore]
fn write_replay_window_fixture() {
    use crate::engine::macros::{CaptureArchive, Library, ReplayFrame, ReplaySession, ReplayTrack};
    let dir = std::path::PathBuf::from(std::env::var("RW_FIXTURE_DIR").expect("RW_FIXTURE_DIR"));
    let mut companion = vec![1, 1, 1, 0x4a, 1, 0];
    companion.extend_from_slice(&[0; 0x26]);
    companion.extend_from_slice(&[
        0, 11, 0, 64, 64, 0, 0, 5, 9, 0, 0, 75, 8, 0, 10, 8, 7, 6, 1, 0, 18, 12, 8, 7, 6, 1, 1, 12, 2, 4, 9, 13, 8, 8, 6, 1, 8, 5,
        1, 8, 5, 17, 20, 5, 4, 1, 4, 2, 20, 0,
    ]);
    companion.extend_from_slice(&[75, 0, 125, 0, 0, 0, 37, 0, 2, 2, 2, 1]);
    let miis = ReplayMiis::from_bytes(&companion).unwrap();
    let rows = RowIds { outfit: 0x1300_4b14_67a2_d93a, hat: 0x5a00_7d15_3956_e6b8 };
    let gunner = miis.for_track(1, 0x4a).unwrap().resolve(rows).unwrap();
    // Three seconds standing (status 0, WAIT: a clean start), with an A press every second.
    let track = |entry: i32, kind: i32| ReplayTrack {
        entry,
        frames: (0..180)
            .map(|f| ReplayFrame {
                input: InputFrame { buttons: u32::from(f % 60 == 30), ..InputFrame::default() },
                start: StartState::neutral(kind, 0, if entry == 0 { 1.0 } else { -1.0 }),
                x: if entry == 0 { -30.0 } else { 30.0 },
                y: 0.0,
            })
            .collect(),
        quantized_analog: false,
    };
    let session = ReplaySession { tracks: vec![track(0, MARIO), track(1, 0x4a)], truncated: false };
    let saved = CaptureArchive::new(dir.join("replay_captures"))
        .save_with_miis("rc4-replay-window-gunner", "RC4 replay window: Mario vs Mii Gunner", "13.0.5", &session, Some(&miis))
        .unwrap();
    let mut macro_file = MacroRecording::new("RC4 Gunner Mii", "13.0.5", StartState::neutral(0x4a, 0, -1.0), frames()).unwrap();
    macro_file.set_mii(gunner).unwrap();
    let id = Library::new(dir.join("macros")).save_new(&macro_file).unwrap();
    std::fs::write(
        dir.join("fixture.json"),
        serde_json::json!({ "capture_id": saved.id, "macro_id": id, "gunner_canonical": gunner.canonical().to_hex() }).to_string(),
    )
    .unwrap();
}

/// What stays closed: an ordinary named macro recorded with another fighter is refused while a CPU exists (the RC rule),
/// and plays only on its own fighter; with no CPU yet (boot restore) it waits.
#[test]
fn an_ordinary_named_macro_keeps_the_rc_rule() {
    let recording = MacroRecording::new("Mario drill", "13.0.5", StartState::neutral(MARIO, 0, 1.0), frames()).unwrap();
    let target = recording.playback_target().unwrap();
    assert_eq!(
        named_load_refusal(RELEASE, target, false, true),
        Some("Choose the recorded CPU fighter before loading this macro")
    );
    assert_eq!(named_load_refusal(RELEASE, target, true, true), None, "on its own fighter it loads");
    assert_eq!(named_load_refusal(RELEASE, target, false, false), None, "before a CPU exists it waits");
    assert!(!keep_allowed(RELEASE, Some(target)));
    assert!(!keep_allowed(RELEASE, None), "an older Mii file without its exact Mii does not play elsewhere");
    assert!(!may_load_on_cpu(RELEASE, SelectionSource::NamedMacro, false, true));
    let closed = Workflows { exact_mii_named: false, ..RELEASE };
    let mii = Target::mii(capture().for_track(0, BRAWLER as u32).unwrap().resolve(rows()).unwrap());
    assert_eq!(named_load_refusal(closed, mii, false, true), Some("Exact Mii named loading is unavailable in this release"));
    assert!(!keep_allowed(closed, Some(mii)));
}
