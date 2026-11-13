//! SPEC-input-01 §10 host rows: the recorder's state machine (lockout, standby, recording, playback,
//! crop, loop, buffers, mirroring, the transition-term table, mash start and interrupt), the settings
//! rows and the named-layer contract.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::ids::{Table, C, UNRESOLVED};
use model::cpu::rng::Rng;
use model::input_delay::MappedInputs;
use model::recording::*;
use model::settings::Snapshot;

fn press(buttons: u32) -> MappedInputs {
    MappedInputs { buttons, ..MappedInputs::NEUTRAL }
}

fn stick(x: i8, y: i8) -> MappedInputs {
    MappedInputs { lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL }
}

const CTX: ControlCtx = ControlCtx { facing: 1, shulk_dial: false };
const POST: PostCtx = PostCtx { facing: 1, queue_head_is_playback: false };
const FACTS: PollFacts = PollFacts { cpu_status: 7 };

/// One game frame: poll P1 with `input`, then the CPU control frame, then the post-frame stage.
fn frame(r: &mut Recorder, s: &RecorderSettings, input: MappedInputs) -> (MappedInputs, ControlOut, PostOut) {
    let mut out = input;
    r.poll(&mut out, FACTS);
    let control = r.control(&CTX);
    let post = r.post_frame(s, &POST);
    (out, control, post)
}

fn settings(duration: u16, crop: bool) -> RecorderSettings {
    RecorderSettings { duration, crop, ..RecorderSettings::factory() }
}

/// Start a recording and run the lockout out: afterwards the recorder stands by.
fn locked_out(r: &mut Recorder, s: &RecorderSettings) {
    r.start_recording(s, 1, StartSnapshot::NONE);
    for _ in 0..LOCKOUT_FRAMES {
        frame(r, s, press(1));
    }
}

#[test]
fn menu_defaults_match_spec() {
    let s = RecorderSettings::from_snapshot(&Snapshot::factory());
    assert_eq!(s, RecorderSettings::factory());
    assert_eq!((s.slot, s.duration, s.crop, s.mash_interrupt, s.override_playback), (0, 150, true, true, false));
    assert_eq!(s.hitstun, HitstunTiming::HitstunEnds);
    assert_eq!(s.playback_slots.0, [1, 0, 0, 0, 0]);
    assert!(s.save_state_slots.is_empty());
}

#[test]
fn settings_rows_are_read() {
    let mut snap = Snapshot::factory();
    snap.set("recording_slot", &[0, 0, 1, 0, 0]).unwrap();
    snap.set("recording_duration", &[0; 19]).unwrap();
    let mut dur = [0u8; 19];
    dur[18] = 1;
    snap.set("recording_duration", &dur).unwrap();
    snap.set("hitstun_playback", &[0, 0, 1]).unwrap();
    snap.set("settings_override_playback", &[1, 0]).unwrap();
    snap.set("save_state_playback", &[0, 3, 0, 0, 2]).unwrap();
    let s = RecorderSettings::from_snapshot(&snap);
    assert_eq!((s.slot, s.duration), (2, 600));
    assert_eq!(s.hitstun, HitstunTiming::HitstopBegins);
    assert!(s.override_playback);
    assert_eq!(s.save_state_slots.0, [0, 3, 0, 0, 2]);
}

#[test]
fn override_playback_default_off() {
    assert!(!RecorderSettings::factory().override_playback);
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
fn recorder_lockout_then_standby_then_cpu() {
    let s = settings(60, true);
    let mut r = Recorder::new();
    r.start_recording(&s, 1, StartSnapshot::NONE);
    assert_eq!((r.mode(), r.possession(), r.phase()), (Mode::Pause, Possession::Lockout, Phase::Lockout));
    // P1's output is neutral during the lockout, and the CPU is not written.
    for n in 0..LOCKOUT_FRAMES {
        let (out, control, _) = frame(&mut r, &s, press(1));
        assert_eq!(out, MappedInputs::NEUTRAL, "lockout frame {n}");
        assert!(control.write.is_none());
        assert!(control.hold_cliff_timer);
    }
    assert_eq!((r.mode(), r.possession(), r.phase()), (Mode::Record, Possession::Standby, Phase::Standby));
}

#[test]
fn standby_ends_on_non_neutral() {
    let s = settings(60, true);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    // Neutral polls keep the standby and inject neutral into the CPU without advancing.
    for _ in 0..10 {
        let (out, control, _) = frame(&mut r, &s, MappedInputs::NEUTRAL);
        assert_eq!(out, MappedInputs::NEUTRAL);
        assert_eq!(control.write, Some(Inject::NEUTRAL));
        assert_eq!((r.possession(), r.frame()), (Possession::Standby, 0));
    }
    // The first non-neutral poll: P1 is neutral again, the CPU gets the frame at once and advances.
    let (out, control, _) = frame(&mut r, &s, press(0x1));
    assert_eq!(out, MappedInputs::NEUTRAL);
    assert_eq!(control.write.map(|w| w.buttons), Some(0x1));
    assert_eq!((r.possession(), r.frame(), r.phase()), (Possession::Cpu, 1, Phase::Recording));
}

#[test]
fn starting_status_captured_frame_one() {
    let s = settings(60, true);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    frame(&mut r, &s, press(0x1));
    assert!(r.wants_cpu_status());
    let mut out = press(0);
    r.poll(&mut out, PollFacts { cpu_status: 42 });
    assert_eq!(r.origin(0).unwrap().status, 42);
    assert!(!r.wants_cpu_status());
    // A later poll never overwrites it.
    r.control(&CTX);
    let mut out = press(0);
    r.poll(&mut out, PollFacts { cpu_status: 99 });
    assert_eq!(r.origin(0).unwrap().status, 42);
}

#[test]
fn frame_bounds_store_and_play_full_length() {
    let s = settings(60, false);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    // Frame k is distinguishable: the jump bit and the stick x.
    let mut ended = None;
    let mut sent = Vec::new();
    for k in 0..60u32 {
        let (_, control, post) = frame(&mut r, &s, MappedInputs { buttons: 0x4, lstick_x: (k + 1) as i8, ..MappedInputs::NEUTRAL });
        sent.push(control.write.unwrap());
        if post.ended.is_some() {
            ended = post.ended;
            assert_eq!(k, 59);
        }
    }
    assert_eq!(ended, Some(Ended::Recorded { slot: 0, frames: 60 }));
    assert_eq!(r.mode(), Mode::None);
    assert_eq!(sent.len(), 60);
    let frames = r.frames(0).unwrap();
    assert_eq!(frames.len(), 60);
    assert_eq!(frames[0].lstick_x, 1);
    assert_eq!(frames[59].lstick_x, 60, "the last frame is stored");
    // Playback plays every stored frame, one per control frame.
    r.start_playback(0, 1).unwrap();
    let mut played = Vec::new();
    for _ in 0..60 {
        let (_, control, post) = frame(&mut r, &s, MappedInputs::NEUTRAL);
        played.push(control.write.unwrap());
        if post.ended.is_some() {
            break;
        }
    }
    assert_eq!(played.len(), 60);
    assert_eq!(r.mode(), Mode::None);
    assert_eq!(played[59].stick_x, 60.0 / 120.0);
}

#[test]
fn crop_trims_trailing_neutral() {
    let s = settings(60, true);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    for k in 0..60 {
        let input = if k < 7 { press(0x1) } else { MappedInputs::NEUTRAL };
        frame(&mut r, &s, input);
    }
    assert_eq!(r.slot_len(0), 7);
    assert!(r.is_recorded(0));
    let off = settings(60, false);
    locked_out(&mut r, &off);
    for k in 0..60 {
        frame(&mut r, &off, if k < 7 { press(0x1) } else { MappedInputs::NEUTRAL });
    }
    assert_eq!(r.slot_len(0), 60);
}

#[test]
fn playback_refuses_empty_slot() {
    let mut r = Recorder::new();
    assert_eq!(r.start_playback(0, 1), Err(PlayRefusal::SlotEmpty), "never recorded");
    assert_eq!(r.start_playback(5, 1), Err(PlayRefusal::SlotOutOfRange));
    let s = settings(60, true);
    r.start_recording(&s, 1, StartSnapshot::NONE);
    assert_eq!(r.start_playback(1, 1), Err(PlayRefusal::Lockout));
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &s, MappedInputs::NEUTRAL);
    }
    assert_eq!(r.start_playback(1, 1), Err(PlayRefusal::Recording), "B8: refused while recording");
    r.stop();
    assert_eq!(r.start_playback(0, 1), Err(PlayRefusal::SlotEmpty), "the abandoned recording never began");
}

#[test]
fn playback_slot_pick_and_lockout_refusal() {
    let mut s = RecorderSettings::factory();
    s.playback_slots = model::cpu::settings::Weights([0, 3, 0, 0, 1]);
    let mut rng = Rng::new(11);
    let mut seen = [0u32; 5];
    for _ in 0..4000 {
        seen[Recorder::pick_chord_slot(&s, &mut rng).unwrap() as usize] += 1;
    }
    assert_eq!(seen[0] + seen[2] + seen[3], 0);
    assert!(seen[1] > 2 * seen[4], "weight 3 against weight 1: {seen:?}");
    s.playback_slots = model::cpu::settings::Weights([0; 5]);
    assert_eq!(Recorder::pick_chord_slot(&s, &mut rng), None);
    assert_eq!(Recorder::resolve_chords(true, true), ChordAction::Playback);
    assert_eq!(Recorder::resolve_chords(true, false), ChordAction::Record);
    assert_eq!(Recorder::resolve_chords(false, false), ChordAction::None);
}

#[test]
fn mirror_multiplier_and_shulk_dial() {
    let f = stick(100, -60);
    assert_eq!(inject_values(f, 1).stick_x, 100.0 / 120.0);
    assert_eq!(inject_values(f, -1).stick_x, -100.0 / 120.0);
    // The vertical axis is never mirrored.
    assert_eq!(inject_values(f, -1).stick_y, -60.0 / 120.0);
    // Through the recorder: recorded facing 1, the CPU now faces left.
    let s = settings(60, false);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    frame(&mut r, &s, stick(100, 0));
    for _ in 0..59 {
        frame(&mut r, &s, MappedInputs::NEUTRAL);
    }
    r.start_playback(0, -1).unwrap();
    let mut out = MappedInputs::NEUTRAL;
    r.poll(&mut out, FACTS);
    let left = r.control(&ControlCtx { facing: -1, shulk_dial: false }).write.unwrap();
    assert!(left.stick_x < 0.0, "mirrored");
    r.start_playback(0, -1).unwrap();
    let dial = r.control(&ControlCtx { facing: -1, shulk_dial: true }).write.unwrap();
    assert!(dial.stick_x > 0.0, "Shulk's dial: never mirrored");
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
    let s = settings(60, false);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    for k in 0..60 {
        frame(&mut r, &s, if k == 0 { press(0x10) } else if k == 1 { press(0x20) } else { MappedInputs::NEUTRAL });
    }
    r.playback_ledge(0, 1, false).unwrap();
    let mut seq = Vec::new();
    for _ in 0..9 {
        let mut out = MappedInputs::NEUTRAL;
        r.poll(&mut out, FACTS);
        seq.push(r.control(&CTX).write.unwrap().buttons);
    }
    // Frame 0 twice, neutral three times, frame 0 once more, then frame 1 onward.
    assert_eq!(seq, [0x10, 0x10, 0, 0, 0, 0x10, 0x20, 0, 0]);
    r.stop();
    r.playback_ledge(0, 1, true).unwrap();
    let mut seq = Vec::new();
    for _ in 0..7 {
        let mut out = MappedInputs::NEUTRAL;
        r.poll(&mut out, FACTS);
        seq.push(r.control(&CTX).write.unwrap().buttons);
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
        (40, C::TermContAttackAir),
        (50, C::TermContEscapeAir),
        (51, C::TermContEscapeAir),
        (60, C::TermContSpecialHi),
        (70, C::TermContJumpSquat),
        (71, C::TermContJumpAerial),
        (80, C::TermContEscape),
        (81, C::TermContEscapeF),
        (82, C::TermContEscapeF),
        (90, C::TermContCatch),
        (1234, C::TermContSpecialS),
        (UNRESOLVED, C::TermContSpecialS),
    ];
    for (status, term) in cases {
        assert_eq!(Recorder::term_for_status(&t, status), term, "status {status}");
    }
}

#[test]
fn hitstun_timing_modes() {
    let r = Recorder::new();
    let mut s = RecorderSettings::factory();
    let ctx = |head: Option<u8>, hitstun: bool, hitstop: bool, term: bool| MashCtx { head_slot: head, in_hitstun: hitstun, in_hitstop: hitstop, term_enabled: term };
    // No queued playback: nothing starts.
    assert_eq!(r.mash_should_start(&s, &ctx(None, false, false, true)), None);
    // Not in hitstun: the derived term decides.
    assert_eq!(r.mash_should_start(&s, &ctx(Some(2), false, false, true)), Some(2));
    assert_eq!(r.mash_should_start(&s, &ctx(Some(2), false, false, false)), None);
    // In hitstun, As Hitstun Ends: the term.
    assert_eq!(r.mash_should_start(&s, &ctx(Some(1), true, true, false)), None);
    assert_eq!(r.mash_should_start(&s, &ctx(Some(1), true, true, true)), Some(1));
    // As Hitstop Ends: when not in hitstop.
    s.hitstun = HitstunTiming::HitstopEnds;
    assert_eq!(r.mash_should_start(&s, &ctx(Some(1), true, true, true)), None);
    assert_eq!(r.mash_should_start(&s, &ctx(Some(1), true, false, false)), Some(1));
    // As Hitstop Begins: at once.
    s.hitstun = HitstunTiming::HitstopBegins;
    assert_eq!(r.mash_should_start(&s, &ctx(Some(1), true, true, false)), Some(1));
    // The term is asked only where it decides.
    assert!(!r.mash_needs_term(&s, Some(1), true));
    assert!(r.mash_needs_term(&s, Some(1), false));
    s.hitstun = HitstunTiming::HitstunEnds;
    assert!(r.mash_needs_term(&s, Some(1), true));
    assert!(!r.mash_needs_term(&s, None, true));
}

/// rc.12 TR1: a Playback Slot (copied from a library macro) plays once per trigger. The old loop read `playback_loop`,
/// a row the Loop list no longer writes (#15), so an old On replayed every mash and ledge playback for ever.
#[test]
fn a_playback_slot_plays_once() {
    let mut snap = Snapshot::factory();
    snap.set("playback_loop", &[1, 0]).unwrap();
    let s = RecorderSettings::from_snapshot(&snap);
    let mut r = Recorder::new();
    r.import_slot(0, &[press(1); 3], model::recording::Origin::NONE).unwrap();
    r.start_playback(0, 1).unwrap();
    for _ in 0..3 {
        assert_eq!(r.mode(), Mode::Playback);
        frame(&mut r, &s, MappedInputs::NEUTRAL);
    }
    for _ in 0..6 {
        frame(&mut r, &s, MappedInputs::NEUTRAL);
        assert_eq!(r.mode(), Mode::None, "played once");
    }
    // An empty copy (its macro left the slot) empties it.
    r.import_slot(0, &[], model::recording::Origin::NONE).unwrap();
    assert_eq!(r.start_playback(0, 1), Err(PlayRefusal::SlotEmpty));
}

#[test]
fn mash_interrupt_stops() {
    let mut s = settings(60, true);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    for k in 0..60 {
        frame(&mut r, &s, if k < 30 { press(1) } else { MappedInputs::NEUTRAL });
    }
    r.start_playback(0, 1).unwrap();
    frame(&mut r, &s, MappedInputs::NEUTRAL);
    s.mash_interrupt = false;
    assert!(!r.interrupt_on_mash(&s));
    assert_eq!(r.mode(), Mode::Playback);
    s.mash_interrupt = true;
    assert!(r.interrupt_on_mash(&s));
    assert_eq!((r.mode(), r.possession(), r.frame()), (Mode::None, Possession::Player, 0));
    // The mash queue head is popped when a run ends.
    r.start_playback(0, 1).unwrap();
    let mut popped = false;
    for _ in 0..40 {
        let mut out = MappedInputs::NEUTRAL;
        r.poll(&mut out, FACTS);
        r.control(&CTX);
        popped |= r.post_frame(&s, &PostCtx { facing: 1, queue_head_is_playback: true }).pop_mash;
    }
    assert!(popped);
}

#[test]
fn save_state_triggers_once() {
    let mut rng = Rng::new(5);
    let mut s = RecorderSettings::factory();
    // Revision 8.2: a load starts neither recording nor slot playback. The trace is
    // `load_starts_no_playback` on the per-role machine.
    assert_eq!(Recorder::on_state_restored(&s, false, &mut rng), LoadAction::None);
    s.save_state_slots = model::cpu::settings::Weights([0, 0, 2, 0, 0]);
    assert_eq!(Recorder::on_state_restored(&s, false, &mut rng), LoadAction::None);
    assert_eq!(Recorder::on_state_restored(&s, true, &mut rng), LoadAction::None);
}

#[test]
fn per_slot_origin() {
    let s0 = RecorderSettings { slot: 0, ..settings(60, true) };
    let s1 = RecorderSettings { slot: 1, ..settings(60, true) };
    let mut r = Recorder::new();
    r.start_recording(&s0, 1, StartSnapshot::NONE);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &s0, MappedInputs::NEUTRAL);
    }
    for k in 0..60 {
        frame(&mut r, &s0, if k < 5 { press(1) } else { MappedInputs::NEUTRAL });
    }
    r.start_recording(&s1, -1, StartSnapshot::NONE);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &s1, MappedInputs::NEUTRAL);
    }
    for k in 0..60 {
        frame(&mut r, &s1, if k < 5 { stick(100, 0) } else { MappedInputs::NEUTRAL });
    }
    assert_eq!((r.origin(0).unwrap().facing, r.origin(1).unwrap().facing), (1, -1));
    // Slot 1 was recorded facing left; the CPU faces left again: no mirroring.
    r.start_playback(1, -1).unwrap();
    let w = r.control(&ControlCtx { facing: -1, shulk_dial: false }).write.unwrap();
    assert!(w.stick_x > 0.0);
    r.stop();
    // Slot 0 faces right at recording; the CPU faces left now: mirrored.
    r.start_playback(0, -1).unwrap();
    assert_eq!(r.starting_status_of(0), r.origin(0).unwrap().status);
}

#[test]
fn session_clear_stops() {
    let s = settings(60, true);
    let mut r = Recorder::new();
    r.start_recording(&s, 1, StartSnapshot::NONE);
    r.stop();
    assert_eq!((r.mode(), r.possession(), r.phase()), (Mode::None, Possession::Player, Phase::Idle));
    let ps = r.playback_state(false);
    assert!(!ps.running && !ps.standby);
}

#[test]
fn phase_notifications() {
    assert_eq!(Phase::Lockout.tint(), Some([0.0, 0.0, 1.0]));
    assert_eq!(Phase::Standby.tint(), Some([1.0, 0.0, 1.0]));
    assert_eq!(Phase::Recording.tint(), Some([1.0, 0.0, 0.0]));
    assert_eq!(Phase::Playback.tint(), None);
    assert_eq!(Phase::Lockout.notice(), Some(("Lockout", [148, 176, 255, 255])));
    assert_eq!(Phase::Standby.notice(), Some(("Standby", [255, 156, 255, 255])));
    assert_eq!(Phase::Recording.notice(), Some(("Recording", [255, 132, 124, 255])));
    assert_eq!(Phase::Playback.notice(), Some(("Playback", [236, 238, 242, 255])));
    assert_eq!(Phase::Idle.notice(), None);
    assert_eq!(NOTICE_HEADER, "Input Recording");
}

#[test]
fn named_contract_export_import_refusals() {
    let mut r = Recorder::new();
    assert_eq!(r.export_slot(0).unwrap_err().message(), "Record into this slot first");
    assert_eq!(r.export_slot(9).unwrap_err().message(), "Record Slot is out of range");
    assert_eq!(r.import_slot(9, &[press(1)], Origin::NONE).unwrap_err().message(), "No such Playback Slot. Nothing was changed.");
    let long = vec![press(1); 601];
    assert_eq!(
        r.import_slot(0, &long, Origin::NONE).unwrap_err().message(),
        "This macro is 601 frames; a Playback Slot holds 600. Crop it first."
    );
    assert!(!r.is_recorded(0), "nothing changed");
    r.import_slot(2, &[press(1), press(2)], Origin { facing: -1, status: 3, start: StartSnapshot::NONE }).unwrap();
    let (frames, origin) = r.export_slot(2).unwrap();
    assert_eq!((frames.len(), origin.facing, origin.status), (2, -1, 3));
    assert!(r.start_playback(2, 1).is_ok());
}

#[test]
fn recorder_state_is_the_24000_byte_slot_table_and_a_little_more() {
    // 5 slots x 600 frames x 8 bytes, statically allocated (SPEC-input-01 §3.2, §8).
    assert_eq!(SLOTS * SLOT_FRAMES * std::mem::size_of::<MappedInputs>(), 24_000);
    assert!(std::mem::size_of::<Recorder>() < 24_000 + 512);
}

/// Lead decision (2026-11-02): the transition-term ranges are inclusive at both ends (question 3), and
/// every other starting status keeps the described behaviour, the side-special term, for parity with the
/// fork. A later change to either is deliberate: this test and docs/test-provenance/package-4.md say so.
#[test]
fn other_status_keeps_the_placeholder_term() {
    let t = table();
    for status in [0, 1, 39, 41, 49, 52, 61, 79, 83, 91, 100_000] {
        assert_eq!(Recorder::term_for_status(&t, status), C::TermContSpecialS, "status {status}");
    }
    // Both ends of the two ranges are inside them (the half-open upstream ranges left these out).
    assert_eq!(Recorder::term_for_status(&t, 51), C::TermContEscapeAir);
    assert_eq!(Recorder::term_for_status(&t, 82), C::TermContEscapeF);
}

/// R2-5 (review R2): a running playback stops on a mash interrupt only when the row is On, and the CPU's view of
/// the recorder (`playback_state`) follows at once.
#[test]
fn r2_5_mash_interrupt_stops_a_running_playback_only_when_enabled() {
    let mut s = settings(60, true);
    let mut r = Recorder::new();
    locked_out(&mut r, &s);
    for k in 0..60 {
        frame(&mut r, &s, if k < 30 { press(1) } else { MappedInputs::NEUTRAL });
    }
    r.start_playback(0, 1).unwrap();
    frame(&mut r, &s, MappedInputs::NEUTRAL);
    assert!(r.playback_state(false).running);
    s.mash_interrupt = false;
    assert!(!r.interrupt_on_mash(&s));
    assert!(r.playback_state(false).running, "continues while the row is Off");
    let (out, ..) = frame(&mut r, &s, MappedInputs::NEUTRAL);
    let _ = out;
    assert_eq!(r.mode(), Mode::Playback);
    s.mash_interrupt = true;
    assert!(r.interrupt_on_mash(&s));
    assert!(!r.playback_state(false).running, "the CPU no longer sees a playback");
    // Nothing is injected on the next frame: the recorder is idle.
    let (_, control, _) = frame(&mut r, &s, MappedInputs::NEUTRAL);
    assert!(control.write.is_none());
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
    let refusal = |export: Option<PilotStateExport>| record_refusal(unsafe { p1_driven_by_ai(false, export) }, true, false);
    assert_eq!(refusal(Some(pilot_armed_l3)), Some(RecordRefusal::P1DrivenByAi));
    assert_eq!(refusal(Some(pilot_armed_l3)).map(RecordRefusal::text), Some("P1 is controlled by an AI: recording is off"));
    assert_eq!(refusal(Some(pilot_unarmed)), None, "an unarmed pilot leaves P1 to the player: it records");
    assert_eq!(refusal(Some(pilot_inactive)), None, "an inactive export (false) is not an AI");
    assert_eq!(refusal(None), None, "no pilot on the card: it records");
    // The game's own operator flag still refuses on its own (the fallback / a CPU-operated entry 0).
    assert!(unsafe { p1_driven_by_ai(true, None) });
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
    let pilot = |export: PilotStateExport| move || record_refusal(unsafe { p1_driven_by_ai(false, Some(export)) }, true, false);
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
    let pilot = |export: PilotStateExport| move || unsafe { p1_driven_by_ai(false, Some(export)) };
    assert_eq!(play_player_chord(true, pilot(pilot_armed_l3)), Err("P1 is controlled by an AI: Play on Player 1 is off"));
    assert_eq!(play_player_chord(true, pilot(pilot_unarmed)), Ok(true));
    assert_eq!(play_player_chord(true, pilot(pilot_inactive)), Ok(true));
    assert_eq!(play_player_chord(true, || unsafe { p1_driven_by_ai(false, None) }), Ok(true), "no pilot: it plays");
    assert!(play_player_chord(true, || unsafe { p1_driven_by_ai(true, None) }).is_err(), "the operator flag too");
    assert_eq!(play_player_chord(false, || panic!("no chord asks nothing")), Ok(false));
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