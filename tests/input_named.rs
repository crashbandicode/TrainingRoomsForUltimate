//! SPEC-input-01 section 9 host rows for the named layer's frame source: the named lockout, the capture that possesses the CPU
//! from frame 0, the layer-owned buffer read back by the control stage, the stop chord, named playback and its end.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
use crate::model;

use std::cell::RefCell;

use model::input_delay::MappedInputs;
use model::recording::*;

const CTX: ControlCtx = ControlCtx { shulk_dial: false };
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
    let out = input;
    let control = r.control_named(&CTX, &|i| layer.at(i));
    let post = r.post_frame(&POST);
    (out, control, post)
}

fn a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    layer.store(0, MappedInputs { lstick_x: 120, ..MappedInputs::NEUTRAL });
    r.start_named_playback(1, -1, 0, 1).unwrap();
    let (_, control, _) = frame(&mut r, &layer, MappedInputs::NEUTRAL);
    assert_eq!(control.write.map(|w| w.stick_x), Some(-1.0), "recorded facing -1 against the CPU's +1 mirrors");

    assert_eq!(Recorder::new().start_named_playback(0, 1, 0, 1), Err(PlayRefusal::NothingLoaded));
    assert_eq!(Recorder::new().start_named_playback(NAMED_FRAMES + 1, 1, 0, 1), Err(PlayRefusal::NothingLoaded));
}

fn f4_named_playback_keeps_start_facing_until_the_restore_updates_it() {
    let mut r = Recorder::new();
    let layer = Layer::default();
    for i in 0..4 {
        layer.store(i, MappedInputs { lstick_x: 80, ..MappedInputs::NEUTRAL });
    }
    r.start_named_playback(4, -1, 0, -1).unwrap();
    for _ in 0..2 {
        let write = r.control_named(&ControlCtx { shulk_dial: false }, &|i| layer.at(i)).write.unwrap();
        assert_eq!(write.stick_x, 80.0 / 120.0);
    }
    r.set_current_facing(1);
    let write = r.control_named(&ControlCtx { shulk_dial: false }, &|i| layer.at(i)).write.unwrap();
    assert_eq!(write.stick_x, -80.0 / 120.0);
}

#[test]
fn restored_named_playback_finishes_while_drill_is_selected() {
    use model::edge_juggle::drill_owns_pad;

    let mut r = Recorder::new();
    let layer = Layer::default();
    layer.store(0, MappedInputs { lstick_x: 120, ..MappedInputs::NEUTRAL });
    r.start_named_playback(1, 1, 0, 1).unwrap();
    assert!(!drill_owns_pad(true, r.phase() != Phase::Idle));
    let (_, control, post) = frame(&mut r, &layer, MappedInputs::NEUTRAL);
    assert_eq!(control.write.map(|pad| pad.stick_x), Some(1.0));
    assert_eq!(post.ended, Some(Ended::NamedPlayed));
    assert!(drill_owns_pad(true, r.phase() != Phase::Idle));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy", a_named_playback_is_mirrored_by_the_recorded_facing_and_refused_while_busy as fn()),
        ("f4_named_playback_keeps_start_facing_until_the_restore_updates_it", f4_named_playback_keeps_start_facing_until_the_restore_updates_it as fn()),
    ]);
}
