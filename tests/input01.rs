//! SPEC-input-01 §10 host rows: the recorder's Playback Slots (filled from library macros since rc.12 TR1; nothing records into
//! one since TR2), its buffers, mirroring, the transition-term table, mash start and interrupt, the settings rows and the
//! named-layer contract.

use crate::model;

use model::cpu::ids::{Table, C, UNRESOLVED};
use model::input_delay::MappedInputs;
use model::recording::*;
use model::settings::Snapshot;

fn press(buttons: u32) -> MappedInputs {
    MappedInputs { buttons, ..MappedInputs::NEUTRAL }
}

fn stick(x: i8, y: i8) -> MappedInputs {
    MappedInputs { lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL }
}

fn frame(run: &mut SlotRun, frames: &[MappedInputs]) -> (Option<Inject>, Option<u8>) {
    let control = run.control(false, |i| frames.get(usize::from(i)).copied());
    (control, run.post())
}

/// 60 frames: `head` and then neutral.
fn take(head: &[MappedInputs]) -> Vec<MappedInputs> {
    let mut frames = head.to_vec();
    frames.resize(60, MappedInputs::NEUTRAL);
    frames
}

#[test]
fn place_row_is_in_recording_options_off_by_default() {
    use model::menu::table::{Control, Page, ROWS};
    let crop = ROWS.iter().position(|row| row.id == "recording_crop").unwrap();
    let row = &ROWS[crop + 1];
    assert_eq!(row.id, "recording_place");
    assert_eq!(row.title, "Place fighters before Record");
    assert_eq!(row.page(), Page::Recording);
    assert_eq!(row.group, ROWS[crop].group);
    assert!(matches!(row.control, Control::Toggle));
    assert_eq!(row.help, "On: Record first opens a wizard to place both fighters\nand pick their state. Off: Record starts at once.");
    let mut snapshot = Snapshot::factory();
    assert_eq!(snapshot.get("recording_place"), Some(&[0, 1][..]));
    assert!(!RecorderSettings::factory().place);
    assert!(!RecorderSettings::from_snapshot(&snapshot).place);
    snapshot.set("recording_place", &[1, 0]).unwrap();
    assert!(RecorderSettings::from_snapshot(&snapshot).place);
    snapshot.set("recording_place", &[0, 1]).unwrap();
    assert!(!RecorderSettings::from_snapshot(&snapshot).place);
}

#[test]
fn menu_defaults_match_spec() {
    let s = RecorderSettings::from_snapshot(&Snapshot::factory());
    assert_eq!(s, RecorderSettings::factory());
    assert_eq!((s.trim, s.mash_interrupt), (model::trim::Trim::End, true));
    assert_eq!(s.hitstun, HitstunTiming::HitstunEnds);
}

#[test]
fn settings_rows_are_read() {
    let mut snap = Snapshot::factory();
    snap.set("hitstun_playback", &[0, 0, 1]).unwrap();
    let s = RecorderSettings::from_snapshot(&snap);
    assert_eq!(s.hitstun, HitstunTiming::HitstopBegins);
}

/// Operator 2026-11-25, PC5: "advanced settings allow you to always copy or always change for all. default is prompt".
#[test]
fn the_shared_start_save_setting_is_prompt_by_default() {
    use model::menu::table::ROWS;
    use model::record_place::SharedStart;
    let mut snap = Snapshot::factory();
    assert_eq!(RecorderSettings::factory().shared, SharedStart::Prompt);
    assert_eq!(RecorderSettings::from_snapshot(&snap).shared, SharedStart::Prompt);
    for (row, rule) in [([0, 1, 0], SharedStart::Change), ([0, 0, 1], SharedStart::Copy), ([1, 0, 0], SharedStart::Prompt)] {
        snap.set("shared_start_save", &row).unwrap();
        assert_eq!(RecorderSettings::from_snapshot(&snap).shared, rule);
    }
    // An Advanced row of Recording › Options, right after the other one there.
    let at = ROWS.iter().position(|row| row.id == "shared_start_save").expect("the row");
    assert_eq!((ROWS[at].title, ROWS[at].advanced), ("Shared start save", true));
    assert_eq!((ROWS[at - 1].id, ROWS[at - 1].group), ("settings_override_playback", ROWS[at].group));
    assert_eq!(model::menu::options::SHARED_START, ["Prompt", "Always change", "Always copy"]);
}

#[test]
fn neutral_frame_rules() {
    assert!(is_neutral(MappedInputs::NEUTRAL));
    assert!(is_neutral(press(FLICK_JUMP)));
    assert!(!is_neutral(press(FLICK_JUMP | 1)));
    assert!(is_neutral(stick(23, -23)));
    assert!(!is_neutral(stick(24, 0)));
    assert!(!is_neutral(stick(0, -24)));
    assert!(!is_neutral(MappedInputs { rstick_x: 30, ..MappedInputs::NEUTRAL }));
}

#[test]
fn a_slot_plays_every_frame_once() {
    // Frame k is distinguishable: the jump bit and the stick x.
    let frames: Vec<_> = (0..60).map(|k| MappedInputs { buttons: 0x4, lstick_x: k + 1, ..MappedInputs::NEUTRAL }).collect();
    let mut r = SlotRun::start(0, 60, 1, 1, None, 7);
    let mut played = Vec::new();
    for _ in 0..60 {
        let (control, post) = frame(&mut r, &frames);
        played.push(control.unwrap());
        if post.is_some() {
            assert_eq!(post, Some(0));
            break;
        }
    }
    assert_eq!(played.len(), 60);
    assert!(!r.active());
    assert_eq!(played[59].stick_x, 60.0 / 120.0);
}

#[test]
fn playback_refuses_empty_slot() {
    use crate::engine::rr_host;
    rr_host::set_slot(4, None).unwrap();
    assert_eq!(rr_host::slot_start_mash(4, rr_host::slot_take(4).unwrap(), 1), Err(PlayRefusal::SlotEmpty));
    assert_eq!(rr_host::slot_start_mash(5, rr_host::slot_take(5).unwrap(), 1), Err(PlayRefusal::SlotOutOfRange));
}

#[test]
fn mirror_multiplier_and_shulk_dial() {
    let f = stick(100, -60);
    assert_eq!(inject_values(f, 1).stick_x, 100.0 / 120.0);
    assert_eq!(inject_values(f, -1).stick_x, -100.0 / 120.0);
    // The vertical axis is never mirrored.
    assert_eq!(inject_values(f, -1).stick_y, -60.0 / 120.0);
    // Through the recorder: recorded facing 1, the CPU now faces left.
    let mut r = SlotRun::start(0, 60, 1, -1, None, 0);
    let left = r.control(false, |_| Some(stick(100, 0))).unwrap();
    assert!(left.stick_x < 0.0, "mirrored");
    r = SlotRun::start(0, 60, 1, -1, None, 0);
    let dial = r.control(true, |_| Some(stick(100, 0))).unwrap();
    assert!(dial.stick_x > 0.0, "Shulk's dial: never mirrored");
    // Recorded facing left, the CPU faces left again: no mirroring.
    r = SlotRun::start(1, 60, -1, -1, None, 0);
    assert!(r.control(false, |_| Some(stick(100, 0))).unwrap().stick_x > 0.0);
}

#[test]
fn f4_playback_same_start_facing_survives_a_turn() {
    let mut r = SlotRun::start(0, 3, -1, -1, None, 0);
    for _ in 0..3 {
        let write = r.control(false, |_| Some(stick(80, 0))).unwrap();
        assert_eq!(write.stick_x, 80.0 / 120.0);
    }
}

#[test]
fn f4_playback_opposite_start_facing_stays_mirrored_after_a_turn() {
    let mut r = SlotRun::start(0, 3, -1, 1, None, 0);
    for _ in 0..3 {
        let write = r.control(false, |_| Some(stick(80, 0))).unwrap();
        assert_eq!(write.stick_x, -80.0 / 120.0);
    }
}

#[test]
fn f4_playback_turns_only_when_the_recorded_direction_changes() {
    let frames: Vec<_> = [-80, 80, -80, 80].into_iter().flat_map(|x| [stick(x, 0); 60]).collect();
    let mut r = SlotRun::start(0, frames.len() as u16, -1, -1, None, 0);
    let mut facing = -1;
    let mut turns = 0;
    let mut mismatches = 0;
    for hold in [-80i8, 80, -80, 80] {
        for _ in 0..60 {
            let write = r.control(false, |i| frames.get(usize::from(i)).copied()).unwrap();
            if write.stick_x != f32::from(hold) / 120.0 {
                mismatches += 1;
            }
            let next = if write.stick_x > 0.0 { 1 } else { -1 };
            if next != facing {
                facing = next;
                turns += 1;
            }
        }
    }
    assert_eq!(turns, 3);
    assert_eq!(mismatches, 0);
    assert_eq!(r.frame(), 240);
    assert_eq!(r.post(), Some(0));
}

#[test]
fn f4_playback_shulk_dial_uses_the_recorded_sign() {
    let mut r = SlotRun::start(0, 3, -1, 1, None, 0);
    for _ in 0..3 {
        let write = r.control(true, |_| Some(stick(80, 0))).unwrap();
        assert_eq!(write.stick_x, 80.0 / 120.0);
    }
}

#[test]
fn injected_stick_values() {
    let i = inject_values(stick(127, -127), 1);
    assert_eq!((i.stick_x, i.stick_y), (1.0, -1.0), "clamped to full deflection");
    let small = inject_values(stick(20, 0), 1);
    assert_eq!(small.stick_x, 20.0 / 120.0);
    assert_eq!(small.clamped_x, 0.0, "below 0.2 the clamped field is zero");
    let big = inject_values(stick(60, 30), 1);
    assert_eq!((big.clamped_x, big.clamped_y), (0.5, 0.25));
    assert_eq!(inject_values(press(0x2a5), 1).buttons, 0x2a5);
}

#[test]
fn control_write_touches_only_verified_fields() {
    // The injected set has no right-stick or rumble member (rule R6): the type is the proof.
    let i = inject_values(MappedInputs { buttons: 1, lstick_x: 1, lstick_y: 2, rstick_x: 120, rstick_y: 120 }, 1);
    let Inject { buttons, stick_x, stick_y, clamped_x, clamped_y } = i;
    assert_eq!(buttons, 1);
    assert!(stick_x < 0.1 && stick_y < 0.1 && clamped_x == 0.0 && clamped_y == 0.0);
}

#[test]
fn ledge_buffer_schedule() {
    let frames = take(&[press(0x10), press(0x20)]);
    let mut r = SlotRun::start(0, 60, 1, 1, Some(false), 0);
    let mut seq = Vec::new();
    for _ in 0..9 {
        seq.push(r.control(false, |i| frames.get(usize::from(i)).copied()).unwrap().buttons);
    }
    // Frame 0 twice, neutral three times, frame 0 once more, then frame 1 onward.
    assert_eq!(seq, [0x10, 0x10, 0, 0, 0, 0x10, 0x20, 0, 0]);
    r = SlotRun::start(0, 60, 1, 1, Some(true), 0);
    let mut seq = Vec::new();
    for _ in 0..7 {
        seq.push(r.control(false, |i| frames.get(usize::from(i)).copied()).unwrap().buttons);
    }
    assert_eq!(seq, [0x10, 0, 0, 0, 0x10, 0x20, 0]);
}

fn table() -> Table {
    // Ascending ids in the order the game declares them is not assumed: only what the rules use.
    Table::with(&[
        (C::StatusAttackAir, 40),
        (C::StatusEscapeAir, 50),
        (C::StatusEscapeAirSlide, 51),
        (C::StatusSpecialHi, 60),
        (C::StatusJumpSquat, 70),
        (C::StatusJumpAerial, 71),
        (C::StatusEscape, 80),
        (C::StatusEscapeF, 81),
        (C::StatusEscapeB, 82),
        (C::StatusCatch, 90),
    ])
}

#[test]
fn starting_status_to_term() {
    let t = table();
    let cases = [
        (40, Some(C::TermContAttackAir)),
        (50, Some(C::TermContEscapeAir)),
        (51, Some(C::TermContEscapeAir)),
        (60, Some(C::TermContSpecialHi)),
        (70, Some(C::TermContJumpSquat)),
        (71, Some(C::TermContJumpAerial)),
        (80, Some(C::TermContEscape)),
        (81, Some(C::TermContEscapeF)),
        (82, Some(C::TermContEscapeF)),
        (90, Some(C::TermContCatch)),
        (1234, None),
        (UNRESOLVED, None),
    ];
    for (status, term) in cases {
        assert_eq!(term_for_status(&t, status), term, "status {status}");
    }
}

#[test]
fn a_start_from_any_other_status_waits_for_the_fighter_to_act() {
    let t = table();
    for status in [UNRESOLVED, 0, 1, 39, 41, 49, 52, 61, 79, 83, 91, 100_000] {
        assert_eq!(term_for_status(&t, status), None, "status {status}");
    }
}

#[test]
fn hitstun_timing_modes() {
    let mut s = RecorderSettings::factory();
    let ctx = |head: Option<u8>, hitstun: bool, hitstop: bool, term: bool| MashCtx { head_slot: head, in_hitstun: hitstun, in_hitstop: hitstop, term_enabled: term };
    // No queued playback: nothing starts.
    assert_eq!(mash_should_start(true, &s, &ctx(None, false, false, true)), None);
    // Not in hitstun: the derived term decides.
    assert_eq!(mash_should_start(true, &s, &ctx(Some(2), false, false, true)), Some(2));
    assert_eq!(mash_should_start(true, &s, &ctx(Some(2), false, false, false)), None);
    // In hitstun, As Hitstun Ends: the term.
    assert_eq!(mash_should_start(true, &s, &ctx(Some(1), true, true, false)), None);
    assert_eq!(mash_should_start(true, &s, &ctx(Some(1), true, true, true)), Some(1));
    // As Hitstop Ends: when not in hitstop.
    s.hitstun = HitstunTiming::HitstopEnds;
    assert_eq!(mash_should_start(true, &s, &ctx(Some(1), true, true, true)), None);
    assert_eq!(mash_should_start(true, &s, &ctx(Some(1), true, false, false)), Some(1));
    // As Hitstop Begins: at once.
    s.hitstun = HitstunTiming::HitstopBegins;
    assert_eq!(mash_should_start(true, &s, &ctx(Some(1), true, true, false)), Some(1));
    // The term is asked only where it decides.
    assert!(!mash_needs_term(true, &s, Some(1), true));
    assert!(mash_needs_term(true, &s, Some(1), false));
    s.hitstun = HitstunTiming::HitstunEnds;
    assert!(mash_needs_term(true, &s, Some(1), true));
    assert!(!mash_needs_term(true, &s, None, true));
    assert!(!mash_needs_term(false, &s, Some(1), true));
    assert_eq!(mash_should_start(false, &s, &ctx(Some(1), true, true, true)), None);
}

/// rc.12 TR1: a Playback Slot (copied from a library macro) plays once per trigger. The old loop read `playback_loop`,
/// a row the Loop list no longer writes (#15), so an old On replayed every mash and ledge playback for ever.
#[test]
fn a_playback_slot_plays_once() {
    let frames = [press(1); 3];
    let mut r = SlotRun::start(0, 3, 1, 1, None, 0);
    for _ in 0..3 {
        assert!(r.active());
        frame(&mut r, &frames);
    }
    for _ in 0..6 {
        assert_eq!(frame(&mut r, &frames), (None, None));
        assert!(!r.active(), "played once");
    }
}

#[test]
fn mash_interrupt_stops() {
    let mut s = RecorderSettings::factory();
    let frames = [press(1); 30];
    let mut r = SlotRun::start(0, 30, 1, 1, None, 0);
    frame(&mut r, &frames);
    s.mash_interrupt = false;
    if s.mash_interrupt { r.stop(); }
    assert!(r.active());
    s.mash_interrupt = true;
    if s.mash_interrupt { r.stop(); }
    assert!(!r.active());
    r = SlotRun::start(0, 30, 1, 1, None, 0);
    let mut popped = false;
    for _ in 0..40 {
        popped |= frame(&mut r, &frames).1 == Some(0);
    }
    assert!(popped);
}

#[test]
fn session_clear_stops() {
    let mut r = SlotRun::start(0, 60, 1, 1, None, 0);
    r.stop();
    assert!(!r.active());
}

#[test]
fn phase_notifications() {
    assert_eq!(Phase::Lockout.tint(), Some([1.0, 1.0, 0.0]));
    assert_eq!(Phase::Standby.tint(), Some([1.0, 0.0, 1.0]));
    assert_eq!(Phase::Recording.tint(), Some([0.0, 0.0, 1.0]));
    assert_eq!(Phase::Playback.tint(), None);
    assert_eq!(Phase::Idle.tint(), None);
    assert_eq!(Phase::Lockout.notice(), Some(("Lockout", [255, 224, 112, 255])));
    assert_eq!(Phase::Standby.notice(), Some(("Standby", [255, 156, 255, 255])));
    assert_eq!(Phase::Recording.notice(), Some(("Recording", [148, 176, 255, 255])));
    assert_eq!(Phase::Playback.notice(), Some(("Playback", [236, 238, 242, 255])));
    assert_eq!(Phase::Idle.notice(), None);
    assert_eq!(NOTICE_HEADER, "Input Recording");
}

/// Fix round 2026-11-23, F3: while its take records, a fighter has its role's hue (Player 1 red, the CPU blue); the
/// CPU's lockout and standby keep hues of their own, and each phase notice has its tint's hue.
#[test]
fn a_recorded_fighter_has_its_roles_tint() {
    assert_eq!((PLAYER_TINT, CPU_TINT), ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]));
    assert_eq!(Phase::Recording.tint(), Some(CPU_TINT));
    assert_eq!((Phase::Lockout.tint(), Phase::Standby.tint()), (Some(LOCKOUT_TINT), Some(STANDBY_TINT)));
    let tints = [PLAYER_TINT, CPU_TINT, LOCKOUT_TINT, STANDBY_TINT];
    for (i, tint) in tints.iter().enumerate() {
        assert!(!tints[i + 1..].contains(tint), "{tint:?} is used twice");
    }
    for phase in [Phase::Lockout, Phase::Standby, Phase::Recording] {
        let (tint, (_, colour)) = (phase.tint().unwrap(), phase.notice().unwrap());
        let off = (0..3).filter(|&i| tint[i] == 0.0).map(|i| colour[i]).max().unwrap();
        let on = (0..3).filter(|&i| tint[i] == 1.0).map(|i| colour[i]).min().unwrap();
        assert!(off < on, "{phase:?}: the notice has its tint's hue");
    }
}

/// The CPU's phase tint and Player 1's take tint go through one writer; Player 1's is written to entry 0's active body
/// from Player 1's own frame in a practice scene (Training and the waiting room).
#[test]
fn the_runtime_tints_each_recorded_fighter() {
    let recorder = include_str!("../src/runtime/features/recorder.rs");
    let body = |source: &'static str, name: &str| {
        let start = source.find(name).unwrap_or_else(|| panic!("missing {name}"));
        &source[start..start + source[start..].find("\n}\n").expect("end of function")]
    };
    let writer = body(recorder, "unsafe fn tint(accessor: usize");
    assert!(writer.contains("MODEL_COLOR_BLEND") && writer.contains("natives::set_color_rgb(accessor, r, g, b, blend)"));
    assert_eq!(recorder.matches("natives::set_color_rgb(").count(), 1, "one writer");
    assert!(body(recorder, "unsafe fn on_ai_frame(").contains("tint(body.accessor, rgb)"), "the CPU's phase tint");
    let player = body(recorder, "pub(crate) unsafe fn tint_player(");
    assert!(player.contains("recording_host::player_tint()"), "only during Player 1's take");
    assert!(player.contains("ENABLED.load(") && player.contains("load_consts()"), "behind the recorder's guards");
    assert!(player.contains("natives::module_accessor(0) == Some(accessor)"), "entry 0's active body, never Nana");
    assert!(player.contains("tint(accessor, rgb)"));
    let frame = body(include_str!("../src/runtime/features/cpu.rs"), "unsafe fn fighter_frame(");
    let scene = frame.find("if training && current_frame {").expect("Player 1's practice-scene frame");
    let call = frame.find("recorder::tint_player(accessor);").expect("Player 1's tint");
    assert!(scene < call && call < frame.find("if !training {").expect("the CPU's part"), "in Player 1's block");
}

#[test]
fn named_contract_export_import_refusals() {
    use crate::engine::macros::{InputFrame, MacroRecording, StartState};
    use crate::engine::rr_host;
    let long = MacroRecording::new("Long", "13.0.5", StartState::neutral(7, 0, 1.0),
        vec![InputFrame::default(); 601]).unwrap();
    assert_eq!(rr_host::set_slot(9, Some(&long)).unwrap_err().message(), "No such Playback Slot. Nothing was changed.");
    assert_eq!(
        rr_host::set_slot(3, Some(&long)).unwrap_err().message(),
        "This macro is 601 frames; a Playback Slot holds 600. Crop it first."
    );
}

/// The transition-term ranges are inclusive at both ends; other statuses use the actionable check.
#[test]
fn other_status_has_no_specific_transition_term() {
    let t = table();
    for status in [0, 1, 39, 41, 49, 52, 61, 79, 83, 91, 100_000] {
        assert_eq!(term_for_status(&t, status), None, "status {status}");
    }
    // Both ends of the two ranges are inside them (the half-open upstream ranges left these out).
    assert_eq!(term_for_status(&t, 51), Some(C::TermContEscapeAir));
    assert_eq!(term_for_status(&t, 82), Some(C::TermContEscapeF));
}

/// R2-5 (review R2): a running playback stops on a mash interrupt only when the row is On, and the CPU's view of
/// the recorder (`playback_state`) follows at once.
#[test]
fn r2_5_mash_interrupt_stops_a_running_playback_only_when_enabled() {
    let mut s = RecorderSettings::factory();
    let frames = take(&[press(1); 30]);
    let mut r = SlotRun::start(0, 60, 1, 1, None, 0);
    frame(&mut r, &frames);
    assert!(r.active());
    s.mash_interrupt = false;
    if s.mash_interrupt { r.stop(); }
    assert!(r.active(), "continues while the row is Off");
    frame(&mut r, &frames);
    assert!(r.active());
    s.mash_interrupt = true;
    if s.mash_interrupt { r.stop(); }
    assert!(!r.active(), "the CPU no longer sees a playback");
    let (control, _) = frame(&mut r, &frames);
    assert!(control.is_none());
}

/// B4 (decided 2026-11-01, SPEC-input-01 question 7 / SPEC-cpu-01 question 7): while an AI — the pilot or
/// the fallback — drives P1, Record is refused with a notice and nothing is recorded. The AI fact is
/// checked first, ahead of the missing-fighter and save-state reasons, and the notice names it.
#[test]
fn record_refused_when_p1_ai() {
    assert_eq!(record_refusal(true, true, false), Some(RecordRefusal::P1DrivenByAi));
    assert_eq!(
        record_refusal(true, false, true),
        Some(RecordRefusal::P1DrivenByAi),
        "the AI fact is checked first"
    );
    assert_eq!(record_refusal(false, true, true), Some(RecordRefusal::SaveStateLoading));
    assert_eq!(record_refusal(false, false, false), Some(RecordRefusal::FightersMissing));
    assert_eq!(record_refusal(false, true, false), None, "a human-driven P1 records");
    assert_eq!(RecordRefusal::P1DrivenByAi.text(), "P1 is controlled by an AI: recording is off");
}

/// Stand-ins for the standalone pilot's `ssbu_cpu_pilot_state_v1` (ssbu-cpu-pilot `src/runtime.rs:526-538`): the same
/// signature and the same write, with `armed_level` 3 (the Surface lane's `p1=cpu / level=3`) or 0.
unsafe extern "C" fn pilot_armed_l3(out: *mut PilotStateV1) -> bool {
    out.write(PilotStateV1 { abi_version: 1, settings: 0x31, armed_level: 3, reserved: [0; 7], last_input: 0 });
    true
}
unsafe extern "C" fn pilot_unarmed(out: *mut PilotStateV1) -> bool {
    out.write(PilotStateV1 { abi_version: 1, settings: 0x31, ..PilotStateV1::default() });
    true
}
unsafe extern "C" fn pilot_inactive(_: *mut PilotStateV1) -> bool {
    false
}

/// B4-1 (`artifacts/phase23/LANE-SURFACE-REPORT.md` "B4-1 root cause"): the standalone pilot arms P1 without flipping
/// `is_operation_cpu(0)`, so the refusal reads its export. Armed: refused with the existing notice. Unarmed, inactive or
/// absent: P1 records.
#[test]
fn record_refused_while_the_standalone_pilot_drives_p1() {
    let refusal = |export: Option<PilotStateExport>| record_refusal(unsafe { p1_driven_by_ai(false, false, export) }, true, false);
    assert_eq!(refusal(Some(pilot_armed_l3)), Some(RecordRefusal::P1DrivenByAi));
    assert_eq!(refusal(Some(pilot_armed_l3)).map(RecordRefusal::text), Some("P1 is controlled by an AI: recording is off"));
    assert_eq!(refusal(Some(pilot_unarmed)), None, "an unarmed pilot leaves P1 to the player: it records");
    assert_eq!(refusal(Some(pilot_inactive)), None, "an inactive export (false) is not an AI");
    assert_eq!(refusal(None), None, "no pilot on the card: it records");
    // The game's own operator flag still refuses on its own (a CPU-operated entry 0).
    assert!(unsafe { p1_driven_by_ai(true, false, None) });
    // The declared ABI: 24 bytes, `armed_level` at offset 8, `last_input` at 16 (the pilot's #[repr(C)] struct).
    assert_eq!(std::mem::size_of::<PilotStateV1>(), 24);
    assert_eq!(std::mem::offset_of!(PilotStateV1, armed_level), 8);
    assert_eq!(std::mem::offset_of!(PilotStateV1, last_input), 16);
}

/// R7 (rc.8): since `0bb2bea3` the Record chord goes to the recording worker, which toggles a capture, and nothing asked B4
/// any more. A chord that would start a capture now asks the same refusals as every other record start; a chord that stops a
/// running capture (started before the pilot armed) still stops it, without asking.
#[test]
fn a_record_chord_that_starts_a_capture_is_refused_while_an_ai_drives_p1() {
    let pilot = |export: PilotStateExport| move || record_refusal(unsafe { p1_driven_by_ai(false, false, Some(export)) }, true, false);
    assert_eq!(record_chord(true, false, pilot(pilot_armed_l3)), Err(RecordRefusal::P1DrivenByAi));
    assert_eq!(
        record_chord(true, false, pilot(pilot_armed_l3)).map_err(RecordRefusal::text),
        Err("P1 is controlled by an AI: recording is off")
    );
    assert_eq!(record_chord(true, false, pilot(pilot_unarmed)), Ok(true), "an unarmed pilot leaves P1 to the player");
    assert_eq!(record_chord(true, false, pilot(pilot_inactive)), Ok(true), "an inactive export is not an AI");
    assert_eq!(record_chord(true, true, || panic!("a stop asks no refusal")), Ok(true), "a running capture stops");
    assert_eq!(record_chord(false, false, || panic!("no chord asks no refusal")), Ok(false));
    assert_eq!(record_chord(false, true, || panic!("no chord asks no refusal")), Ok(false));
}

/// rc.9 C14 (`PLUGIN-SHORTCUT-CONFLICTS.md`): Play on Player 1 writes P1's input, as the standalone pilot does while it
/// drives P1, so its chord is refused then, with R7's check and notice style. Unarmed, inactive or absent: it plays.
#[test]
fn play_on_player_1_is_refused_while_an_ai_drives_p1() {
    let pilot = |export: PilotStateExport| move || unsafe { p1_driven_by_ai(false, false, Some(export)) };
    assert_eq!(play_player_chord(true, pilot(pilot_armed_l3)), Err("P1 is controlled by an AI: Play on Player 1 is off"));
    assert_eq!(play_player_chord(true, pilot(pilot_unarmed)), Ok(true));
    assert_eq!(play_player_chord(true, pilot(pilot_inactive)), Ok(true));
    assert_eq!(play_player_chord(true, || unsafe { p1_driven_by_ai(false, false, None) }), Ok(true), "no pilot: it plays");
    assert!(play_player_chord(true, || unsafe { p1_driven_by_ai(true, false, None) }).is_err(), "the operator flag too");
    assert_eq!(play_player_chord(false, || panic!("no chord asks nothing")), Ok(false));
}

/// rc.14 recfix (Eden rc.13 `g5-rc13-xps-1`, gif-lane-5): TR's own P1 CPU Control (the fallback pilot) arms the AI and hands
/// back its decision as P1's mapped input, leaving `is_operation_cpu(0)` false ("No operation-CPU change persists",
/// `cpu_pilot.rs`), so B4 never saw it: Record started and Play on Player 1 played while it drove P1. The fallback's own
/// answer (it is emitting P1's input now) refuses both, with the standalone pilot's notices. Paused for a menu or a recording,
/// or Off, it is not emitting: P1 records and plays. A running capture still stops without asking.
#[test]
fn record_and_play_on_player_1_are_refused_while_the_fallback_drives_p1() {
    let driven = |fallback: bool| move || unsafe { p1_driven_by_ai(false, fallback, None) };
    let record = |fallback: bool| record_chord(true, false, || record_refusal(driven(fallback)(), true, false));
    assert_eq!(record(true), Err(RecordRefusal::P1DrivenByAi));
    assert_eq!(record(true).map_err(RecordRefusal::text), Err("P1 is controlled by an AI: recording is off"));
    assert_eq!(record(false), Ok(true), "the fallback Off or paused: P1 records");
    assert_eq!(play_player_chord(true, driven(true)), Err("P1 is controlled by an AI: Play on Player 1 is off"));
    assert_eq!(play_player_chord(true, driven(false)), Ok(true), "the fallback Off or paused: Play on Player 1 plays");
    assert_eq!(record_chord(true, true, || panic!("a stop asks no refusal")), Ok(true), "a running capture stops");
    // The other two answers still refuse on their own, whatever the fallback says.
    assert!(unsafe { p1_driven_by_ai(true, false, None) });
    assert!(unsafe { p1_driven_by_ai(false, false, Some(pilot_armed_l3)) });
    assert!(!unsafe { p1_driven_by_ai(false, false, Some(pilot_unarmed)) });
}

/// R7: a starting Record chord answers exactly as `record_refusal` does, over every combination of its live facts (the
/// numbered and named starts ask the same function).
#[test]
fn a_starting_record_chord_answers_as_record_refusal() {
    for ai in [false, true] {
        for present in [false, true] {
            for loading in [false, true] {
                let expected = record_refusal(ai, present, loading);
                let chord = record_chord(true, false, || record_refusal(ai, present, loading));
                assert_eq!(chord.err(), expected, "ai={ai} present={present} loading={loading}");
                assert_eq!(chord.ok(), expected.is_none().then_some(true));
            }
        }
    }
}