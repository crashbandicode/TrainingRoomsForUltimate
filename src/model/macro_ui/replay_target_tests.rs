//! The fork's recording workflows as they ship again (operator 2026-11-05), through the carried models: a replay track's
//! target before and after its retried Mii import, and a Switch to the recorded identity (an exact Mii or an ordinary
//! fighter) from the replay window and from a named macro, with Keep beside it.

use super::*;
use crate::engine::macro_ui::macro_panel::{Effect, FighterGate, FighterOrigin, LibraryRow, LoadContext, MacroUi, Pad};
use crate::engine::macros::{InputFrame, MacroRecording, StartState};
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

fn frames() -> Vec<InputFrame> {
    vec![InputFrame { buttons: 0x1, lstick_x: 0, lstick_y: 40, rstick_x: 0, rstick_y: 0 }]
}

/// What the runtime's `load_context` gives in this build: the Mii loader is compiled in, so every exact target can be
/// switched to (`can_switch`, the fork's `cfg!(mii-ordinary-cpu-alpha) || configuration().is_none()`).
fn context(cpu_kind: i32, matches: bool) -> LoadContext {
    LoadContext {
        cpu_kind: Some(cpu_kind),
        matches,
        prompt_enabled: true,
        can_switch: true,
        revision: 3,
        recorded_label: "recorded".into(),
        current_label: "current".into(),
    }
}

fn pick(ui: &mut MacroUi) {
    let rows = ["id0", "id1"].map(|id| LibraryRow { id: id.into(), label: id.into() });
    ui.open_library_titled(rows.to_vec(), 0, "Named Library", Some("Record Slot 1".into()));
    ui.on_pad(Pad::Right);
    let Effect::Choose(id) = ui.on_pad(Pad::Confirm) else { panic!("expected choose") };
    assert_eq!(id, "id1");
}

/// The replay window's retry: a Mii track whose import was missed is `Ok(None)` (the window resolves it before it asks),
/// then that exact Mii; a failed import refuses the Switch but not the inputs.
#[test]
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
#[test]
fn an_ordinary_track_switches_to_its_recorded_fighter() {
    assert_eq!(clip_target(MARIO, Err("an ordinary track has no Mii"), None, false, true), Ok(Some(Target::fighter(MARIO as u32))));
    assert_eq!(clip_target(HERO, Err("unused"), None, false, false), Ok(Some(Target::fighter(HERO as u32))), "no Mii loader needed");
    assert!(Target::fighter(MARIO as u32).ready(MARIO as u32, None));
    assert!(clip_target(0x7fff, Err("unused"), None, false, true).is_err());
}

/// A named macro carrying a Mii: the clip's resolved Mii goes into the recording, through JSON; loading it on another
/// CPU asks; Switch requests the Mii's fighter and playback waits for that exact Mii; Keep plays on the current CPU and
/// leaves the file's Mii.
#[test]
fn a_named_macro_carries_its_mii_and_restores_it() {
    let configuration = capture().for_track(0, BRAWLER as u32).unwrap().resolve(rows()).unwrap();
    let mut recording = MacroRecording::new("Brawler combo", "13.0.5", StartState::neutral(BRAWLER, 0, 1.0), frames()).unwrap();
    recording.set_mii(configuration).unwrap();
    let loaded: MacroRecording = serde_json::from_str(&serde_json::to_string(&recording).unwrap()).unwrap();
    assert_eq!(loaded.playback_target().unwrap().configuration(), Some(configuration), "the file keeps the exact Mii");
    assert_eq!(context(MARIO, false).gate(&loaded), FighterGate::Prompt);

    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), loaded.clone(), context(MARIO, false)).unwrap();
    assert_eq!(ui.fighter_prompt(), Some((FighterOrigin::Named, 3)));
    ui.on_pad(Pad::Down);
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::NeedCharacter(BRAWLER)));
    assert_eq!(ui.claim_character_request(), Some(BRAWLER));
    assert_eq!(ui.loaded_target(), Some(Target::mii(configuration)));
    assert!(!ui.plays_current());
    assert!(!ui.playback_allowed(), "nothing plays before the Mii is on the CPU");

    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), loaded, context(MARIO, false)).unwrap();
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::None));
    assert!(ui.plays_current());
    assert_eq!(ui.loaded_target().unwrap().configuration(), Some(configuration));
    assert_eq!(ui.claim_character_request(), None);
}

/// An ordinary named macro recorded with another fighter offers Switch (load the recorded fighter) or Keep (play on the
/// current CPU), as the fork did, instead of waiting for that fighter (the RC rule, gone).
#[test]
fn an_ordinary_named_macro_offers_switch_or_keep() {
    let recording = MacroRecording::new("Mario drill", "13.0.5", StartState::neutral(MARIO, 0, 1.0), frames()).unwrap();
    assert_eq!(recording.playback_target().unwrap(), Target::fighter(MARIO as u32));
    assert_eq!(context(HERO, false).gate(&recording), FighterGate::Prompt);
    assert_eq!(context(MARIO, true).gate(&recording), FighterGate::Proceed, "on its own fighter nothing is asked");

    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), recording.clone(), context(HERO, false)).unwrap();
    assert_eq!(ui.fighter_prompt(), Some((FighterOrigin::Named, 3)));
    ui.on_pad(Pad::Down);
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::NeedCharacter(MARIO)), "Switch loads Mario on the CPU");
    assert_eq!(ui.claim_character_request(), Some(MARIO));
    assert!(!ui.playback_allowed(), "playback waits for Mario");
    ui.observe_cpu(Some(MARIO), false);
    assert!(ui.playback_allowed());

    let mut ui = MacroUi::new();
    pick(&mut ui);
    ui.offer_load("id1".into(), recording, context(HERO, false)).unwrap();
    assert!(matches!(ui.on_pad(Pad::Confirm), Effect::None), "Keep");
    assert!(ui.plays_current());
    assert_eq!(ui.claim_character_request(), None);
    ui.observe_cpu(Some(HERO), false);
    assert!(ui.playback_allowed(), "Mario's inputs play on Hero");
}

/// The Eden fixture of `artifacts/phase23/rc5-combined/rows-replay`: a replay capture whose entry 1 is a Mii Gunner with
/// REPLAY185's real companion (outfit 75, hat 125, colour 0, voice 37, specials 2221; `tests/mii_replay185_headgear.rs`)
/// beside a Mario on entry 0, a named macro carrying that Gunner's exact Mii, and an ordinary named macro recorded with
/// Mario. Written only on request: `RW_FIXTURE_DIR=<dir> cargo test --lib write_replay_window_fixture -- --ignored` makes
/// `<dir>/replay_captures/` and `<dir>/macros/`, copied into the profile's `sd:/ultimate/TrainingRooms/`.
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
    let session = ReplaySession { tracks: vec![track(0, MARIO), track(1, 0x4a)], truncated: false, breaks: Vec::new() };
    let saved = CaptureArchive::new(dir.join("replay_captures"))
        .save_with_miis("rc4-replay-window-gunner", "RC4 replay window: Mario vs Mii Gunner", "13.0.5", &session, Some(&miis))
        .unwrap();
    let library = Library::new(dir.join("macros"));
    let mut mii_macro = MacroRecording::new("RC4 Gunner Mii", "13.0.5", StartState::neutral(0x4a, 0, -1.0), frames()).unwrap();
    mii_macro.set_mii(gunner).unwrap();
    let mii_id = library.save_new(&mii_macro).unwrap();
    let mario_macro = MacroRecording::new("RC5 Mario drill", "13.0.5", StartState::neutral(MARIO, 0, 1.0), frames()).unwrap();
    let mario_id = library.save_new(&mario_macro).unwrap();
    // A saved Training setup: P1 Mario at 42 % on the left, the CPU Mario at 80 % on the right, both standing.
    use crate::model::state::setup::{FighterSetup, SetupLibrary, SetupRecord};
    let fighter = |x: f32, percent: f32, lr: f32| FighterSetup {
        fighter_kind: MARIO, x, y: 0.0, percent, lr, situation_kind: 0, charge: Default::default(), steve_state: None,
        held_item: None, mii: None,
    };
    let setup = SetupRecord::new("RC5 Mario corners", "13.0.5", None, fighter(-60.0, 42.0, 1.0), fighter(60.0, 80.0, -1.0)).unwrap();
    let setup_id = SetupLibrary::new(dir.join("setups")).save_new(&setup).unwrap();
    std::fs::write(
        dir.join("fixture.json"),
        serde_json::json!({ "capture_id": saved.id, "macro_id": mii_id, "ordinary_macro_id": mario_id, "setup_id": setup_id,
                            "gunner_canonical": gunner.canonical().to_hex() })
        .to_string(),
    )
    .unwrap();
}
