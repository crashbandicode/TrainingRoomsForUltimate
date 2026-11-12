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

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_stop_chord_ends_the_capture_and_reports_the_frames", the_stop_chord_ends_the_capture_and_reports_the_frames as fn()),
        ("the_stop_chord_does_nothing_during_a_numbered_recording", the_stop_chord_does_nothing_during_a_numbered_recording as fn()),
        ("a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy", a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy as fn()),
    ]);
}
