//! SPEC-input-01 section 9 host rows for the named layer's frame source: the named lockout, the capture that possesses the CPU
//! from frame 0, the layer-owned buffer read back by the control stage, the stop chord, named playback and its end.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::cell::RefCell;

use model::input_delay::MappedInputs;
use model::recording::*;

fn press(buttons: u32) -> MappedInputs {
    MappedInputs { buttons, ..MappedInputs::NEUTRAL }
}

const CTX: ControlCtx = ControlCtx { facing: 1, shulk_dial: false };
const POST: PostCtx = PostCtx { facing: 1, queue_head_is_playback: false };

/// The layer's buffer, as `named_macros` keeps it.
#[derive(Default)]
struct Layer {
    frames: RefCell<Vec<MappedInputs>>,
}

impl Layer {
    fn store(&self, at: u16, frame: MappedInputs) {
        let mut frames = self.frames.borrow_mut();
        if frames.len() <= usize::from(at) {
            frames.resize(usize::from(at) + 1, MappedInputs::NEUTRAL);
        }
        frames[usize::from(at)] = frame;
    }

    fn at(&self, at: u16) -> Option<MappedInputs> {
        self.frames.borrow().get(usize::from(at)).copied()
    }
}

/// One game frame of a named run.
fn frame(r: &mut Recorder, layer: &Layer, input: MappedInputs) -> (MappedInputs, ControlOut, PostOut) {
    let mut out = input;
    if let Some((at, stored)) = r.poll_named(&mut out) {
        layer.store(at, stored);
    }
    let control = r.control_named(&CTX, &|i| layer.at(i));
    let post = r.post_frame(&RecorderSettings::factory(), &POST);
    (out, control, post)
}

fn the_named_lockout_runs_thirty_frames_neutral_then_possesses_the_cpu_at_once() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    r.start_named_lockout(1);
    assert!(r.is_named());
    assert_eq!(r.phase(), Phase::Lockout);
    for _ in 0..LOCKOUT_FRAMES {
        let (out, control, _) = frame(&mut r, &layer, press(1));
        assert_eq!(out, MappedInputs::NEUTRAL, "P1 is neutral during the lockout");
        assert!(control.write.is_none());
    }
    assert!(layer.frames.borrow().is_empty(), "nothing is stored during the lockout");
    // No standby: the capture is already recording with the CPU possessed.
    assert_eq!((r.mode(), r.possession()), (Mode::Record, Possession::Cpu));
    assert_eq!(r.length(), NAMED_FRAMES);
}

fn capture_stores_from_frame_zero_and_the_cpu_performs_what_was_just_stored() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    r.start_named_lockout(1);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &layer, MappedInputs::NEUTRAL);
    }
    let (out, control, _) = frame(&mut r, &layer, press(4));
    assert_eq!(out, MappedInputs::NEUTRAL, "P1's output is neutral while recording");
    assert_eq!(layer.at(0), Some(press(4)));
    assert_eq!(control.write.map(|w| w.buttons), Some(4));
    let (_, control, _) = frame(&mut r, &layer, press(8));
    assert_eq!(layer.at(1), Some(press(8)));
    assert_eq!(control.write.map(|w| w.buttons), Some(8));
    assert_eq!(r.frame(), 2);
}

fn before_the_first_poll_there_is_nothing_to_perform_and_the_clock_waits() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    r.start_named_lockout(1);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &layer, MappedInputs::NEUTRAL);
    }
    // A control frame with no stored frame 0: neutral controls, no advance.
    let control = r.control_named(&CTX, &|_| None);
    assert_eq!(control.write, Some(Inject::NEUTRAL));
    assert_eq!(r.frame(), 0);
}

fn the_stop_chord_ends_the_capture_and_reports_the_frames() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    r.start_named_lockout(1);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &layer, MappedInputs::NEUTRAL);
    }
    for _ in 0..7 {
        frame(&mut r, &layer, press(1));
    }
    assert_eq!(r.finish_named_capture(), Some(7));
    assert_eq!((r.mode(), r.possession(), r.is_named()), (Mode::None, Possession::Player, false));
    assert_eq!(r.finish_named_capture(), None, "a second stop has nothing to end");
}

fn the_stop_chord_does_nothing_during_a_numbered_recording() {
    let mut r = Recorder::new();
    r.start_recording(&RecorderSettings::factory(), 1, StartSnapshot::NONE);
    assert_eq!(r.finish_named_capture(), None);
    assert_eq!(r.mode(), Mode::Pause);
}

fn a_named_playback_reads_the_layer_and_ends_after_its_length() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    for i in 0..5u32 {
        layer.store(i as u16, press(1 << i));
    }
    let started = r.start_named_playback(5, 1, 42, 1).expect("started");
    assert_eq!(started.frames, 5);
    assert_eq!(r.phase(), Phase::Playback);
    let mut written = Vec::new();
    let mut ended = None;
    for _ in 0..5 {
        let (_, control, post) = frame(&mut r, &layer, MappedInputs::NEUTRAL);
        written.push(control.write.map(|w| w.buttons));
        ended = post.ended.or(ended);
    }
    assert_eq!(written, [Some(1), Some(2), Some(4), Some(8), Some(16)]);
    assert_eq!(ended, Some(Ended::NamedPlayed));
    assert_eq!(r.mode(), Mode::None);
    assert!(!r.is_named());
}

fn a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    layer.store(0, MappedInputs { lstick_x: 120, ..MappedInputs::NEUTRAL });
    r.start_named_playback(1, -1, 0, 1).unwrap();
    let (_, control, _) = frame(&mut r, &layer, MappedInputs::NEUTRAL);
    assert_eq!(control.write.map(|w| w.stick_x), Some(-1.0), "recorded facing -1 against the CPU's +1 mirrors");

    let mut r = Recorder::new();
    r.start_named_lockout(1);
    assert_eq!(r.start_named_playback(3, 1, 0, 1), Err(PlayRefusal::Lockout));
    assert_eq!(Recorder::new().start_named_playback(0, 1, 0, 1), Err(PlayRefusal::SlotEmpty));
    assert_eq!(Recorder::new().start_named_playback(NAMED_FRAMES + 1, 1, 0, 1), Err(PlayRefusal::SlotEmpty));
}

fn a_capture_that_reaches_the_maximum_ends_like_a_stop_and_numbered_runs_are_not_named() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    r.start_named_lockout(1);
    for _ in 0..LOCKOUT_FRAMES {
        frame(&mut r, &layer, MappedInputs::NEUTRAL);
    }
    let mut ended = None;
    for _ in 0..NAMED_FRAMES {
        let (_, _, post) = frame(&mut r, &layer, press(1));
        ended = post.ended.or(ended);
    }
    assert_eq!(ended, Some(Ended::NamedCaptured { frames: NAMED_FRAMES }));
    assert_eq!(r.mode(), Mode::None);

    let s = RecorderSettings::factory();
    r.start_recording(&s, 1, StartSnapshot::NONE);
    assert!(!r.is_named());
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_named_lockout_runs_thirty_frames_neutral_then_possesses_the_cpu_at_once", the_named_lockout_runs_thirty_frames_neutral_then_possesses_the_cpu_at_once as fn()),
        ("capture_stores_from_frame_zero_and_the_cpu_performs_what_was_just_stored", capture_stores_from_frame_zero_and_the_cpu_performs_what_was_just_stored as fn()),
        ("before_the_first_poll_there_is_nothing_to_perform_and_the_clock_waits", before_the_first_poll_there_is_nothing_to_perform_and_the_clock_waits as fn()),
        ("the_stop_chord_ends_the_capture_and_reports_the_frames", the_stop_chord_ends_the_capture_and_reports_the_frames as fn()),
        ("the_stop_chord_does_nothing_during_a_numbered_recording", the_stop_chord_does_nothing_during_a_numbered_recording as fn()),
        ("a_named_playback_reads_the_layer_and_ends_after_its_length", a_named_playback_reads_the_layer_and_ends_after_its_length as fn()),
        ("a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy", a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy as fn()),
        ("a_capture_that_reaches_the_maximum_ends_like_a_stop_and_numbered_runs_are_not_named", a_capture_that_reaches_the_maximum_ends_like_a_stop_and_numbered_runs_are_not_named as fn()),
    ]);
}
