//! SPEC-input-07 §3 on the strip's own `Link` records: scripted frames step a real `Tracker`, and the helper steps
//! after it as the runtime does. Frame numbers are the tracker's (the first `step` is frame 1).

lib_unit_tests! {
use super::*;
use crate::model::drill::{Advance, SequenceRun, SequenceWalk, Stepped};
use crate::model::frame_strip::{Sample, K};
use crate::model::input_delay::MappedInputs;

const ATTACK: i32 = 0x27;
const ATTACK_HI3: i32 = 0x2b;
const ATTACK_LW3: i32 = 0x2c;
const A: MappedInputs = MappedInputs {
    buttons: 0x1,
    lstick_x: 0,
    lstick_y: 0,
    rstick_x: 0,
    rstick_y: 0,
};
const UP_A: MappedInputs = MappedInputs {
    buttons: 0x1,
    lstick_x: 0,
    lstick_y: 100,
    rstick_x: 0,
    rstick_y: 0,
};
const DOWN_A: MappedInputs = MappedInputs {
    buttons: 0x1,
    lstick_x: 0,
    lstick_y: -100,
    rstick_x: 0,
    rstick_y: 0,
};
const NONE: MappedInputs = MappedInputs::NEUTRAL;
/// Frames of hitlag, and of hitstun after it: the CPU is free `STOP + STUN` frames after a hit.
const STOP: u32 = 4;
const STUN: f32 = 18.0;
/// P1 can act this many frames after its own hit (4 hitlag, 1 active, 8 recovery).
const RECOVERY: u32 = 13;
const STARTUP: u32 = 5;

fn idle() -> Sample {
    Sample {
        actionable: true,
        ..Sample::default()
    }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample {
        attack: Some(mv),
        status,
        motion_frame,
        ..Sample::default()
    }
}

/// The CPU after its last hit on `hit`: hitlag, then hitstun counting down, then free.
fn cpu_after(hit: Option<u32>, frame: u32) -> Sample {
    let Some(hit) = hit else { return idle() };
    let first = hit + STOP;
    if frame < first {
        return Sample {
            stop: true,
            hitstun: true,
            reaction: Some(STUN),
            ..Sample::default()
        };
    }
    let r = STUN - (frame - first) as f32;
    if r + K as f32 > 0.0 {
        Sample {
            hitstun: true,
            reaction: Some(r.max(0.0)),
            ..Sample::default()
        }
    } else {
        idle()
    }
}

/// A follow-up: the move, its status and its input.
#[derive(Clone, Copy)]
struct Hit {
    mv: Move,
    status: i32,
    input: MappedInputs,
}

const JAB: Hit = Hit {
    mv: Move::Jab,
    status: ATTACK,
    input: A,
};
const UP_TILT: Hit = Hit {
    mv: Move::UpTilt,
    status: ATTACK_HI3,
    input: UP_A,
};
const DOWN_TILT: Hit = Hit {
    mv: Move::DownTilt,
    status: ATTACK_LW3,
    input: DOWN_A,
};

/// Runs P1 through `moves`, each pressed on its frame (the first is the opener), on the tracker and the helper; every
/// move connects `STARTUP` frames after its press. `after` idle frames end the script. Returns what each step reported.
struct Run {
    tracker: Tracker,
    helper: Helper,
    outs: Vec<Out>,
    frame: u32,
    last_hit: Option<u32>,
    /// A drilled sequence and its combos: the helper steps through `SequenceRun`, as the runtime does.
    sequence: Option<(SequenceRun, Vec<Combo>)>,
    stepped: Vec<Stepped>,
}

impl Run {
    fn new(helper: Helper) -> Self {
        Self {
            tracker: Tracker::new(),
            helper,
            outs: Vec::new(),
            frame: 0,
            last_hit: None,
            sequence: None,
            stepped: Vec::new(),
        }
    }

    /// A run that drills `steps` as a sequence, going on from `streak` and `best`.
    fn drilling(steps: &[Combo], streak: u32, best: u32) -> Self {
        let mut run = Self::new(Helper::new());
        let walk = SequenceRun::new(SequenceWalk::resume(steps.len(), streak, best));
        walk.start(&mut run.helper, steps);
        run.sequence = Some((walk, steps.to_vec()));
        run
    }

    fn step(&mut self, p1: Sample, inputs: MappedInputs, settings: Settings) {
        self.frame += 1;
        let cpu = cpu_after(self.last_hit, self.frame);
        self.tracker.step(p1, cpu, inputs);
        self.advance(settings);
    }

    fn advance(&mut self, settings: Settings) {
        match self.sequence.as_mut() {
            Some((run, steps)) => {
                let (out, stepped) = run.step(&mut self.helper, steps, &self.tracker, settings);
                self.outs.push(out);
                self.stepped.extend(stepped);
            }
            None => self.outs.push(self.helper.step(&self.tracker, settings)),
        }
    }

    /// The strip's session clear (a save-state load, a scene change).
    fn clear(&mut self) {
        match self.sequence.as_mut() {
            Some((run, steps)) => {
                let (out, stepped) = run.clear(&mut self.helper, steps);
                self.outs.push(out);
                self.stepped.extend(stepped);
            }
            None => self.outs.push(self.helper.clear()),
        }
    }

    fn walk(&self) -> SequenceWalk {
        self.sequence.as_ref().expect("a sequence run").0.walk()
    }

    fn idle_until(&mut self, frame: u32, settings: Settings) {
        while self.frame + 1 < frame {
            self.step(idle(), NONE, settings);
        }
    }

    /// One move pressed on `press`: startup, its hit (hitlag), one active frame and the recovery.
    fn hit(&mut self, hit: Hit, press: u32, settings: Settings) -> u32 {
        self.idle_until(press, settings);
        for f in 0..STARTUP {
            self.step(
                attack(hit.mv, hit.status, f as f32),
                if f == 0 { hit.input } else { NONE },
                settings,
            );
        }
        let at = self.frame + 1;
        self.last_hit = Some(at);
        for _ in 0..STOP {
            self.step(
                Sample {
                    stop: true,
                    active: true,
                    ..attack(hit.mv, hit.status, STARTUP as f32)
                },
                NONE,
                settings,
            );
        }
        self.step(
            Sample {
                active: true,
                ..attack(hit.mv, hit.status, STARTUP as f32 + 1.0)
            },
            NONE,
            settings,
        );
        for f in 0..(RECOVERY - STOP - 1) {
            self.step(
                attack(hit.mv, hit.status, STARTUP as f32 + 2.0 + f as f32),
                NONE,
                settings,
            );
        }
        at
    }

    /// The opener, then each follow-up pressed `delays[i]` frames after P1 could act; `delays` 1 is a TRUE −3 link.
    fn combo(&mut self, opener: Hit, follow: &[(Hit, i32)], settings: Settings) {
        let start = self.frame + 3;
        let mut hit = self.hit(opener, start, settings);
        for (h, delay) in follow {
            let a1 = hit + RECOVERY;
            hit = self.hit(*h, (a1 as i32 + delay) as u32, settings);
        }
    }

    /// Idle until the exchange has closed (the CPU free for longer than the strip's `DROP_MAX`).
    fn settle(&mut self, settings: Settings) {
        let until = self.frame + 60;
        self.idle_until(until, settings);
    }

    /// One frame with an explicit CPU sample. `step` derives the CPU from the last hit and cannot hit a shield.
    fn step_with(&mut self, p1: Sample, cpu: Sample, inputs: MappedInputs) {
        self.frame += 1;
        self.tracker.step(p1, cpu, inputs);
        self.advance(ON);
    }

    /// A follow-up that hits the CPU's shield: the exchange ends and the hit is not a link.
    fn shield_hit(&mut self) {
        for f in 0..STARTUP {
            let cpu = cpu_after(self.last_hit, self.frame + 1);
            self.step_with(
                attack(Move::UpTilt, ATTACK_HI3, f as f32),
                cpu,
                if f == 0 { UP_A } else { NONE },
            );
        }
        let cpu = Sample {
            stop: true,
            shielding: true,
            ..Sample::default()
        };
        self.step_with(
            Sample {
                stop: true,
                active: true,
                ..attack(Move::UpTilt, ATTACK_HI3, STARTUP as f32)
            },
            cpu,
            NONE,
        );
    }

    fn resolved(&self) -> Vec<Summary> {
        self.outs.iter().filter_map(|o| o.resolved).collect()
    }

}

const ON: Settings = Settings {
    on: true,
    tolerance: 1,
    load_after: false,
    match_spot: false,
    match_stick: false,
    move_names: false,
};

/// The reference: a jab and three up tilts, each pressed one frame after P1 could act.
fn recorded() -> Combo {
    let mut run = Run::new(Helper::new());
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    run.settle(ON);
    run.helper.last_combo().expect("the exchange is kept")
}

fn with_target() -> Helper {
    let mut helper = Helper::new();
    helper.set_target(Some(recorded()));
    helper
}

#[test]
fn a_hit_combo_load_is_not_deferred() {
    let mut helper = with_target();
    let settings = Settings { load_after: true, ..ON };
    helper.settings = settings;
    let summary = Summary { outcome: Outcome::Completed, graded: 1, true_links: 1,
        total: 1, clean: true, fault: None };
    helper.record(summary, &mut Out::default());
    assert!(helper.pending_load);
    let tracker = Tracker::new();
    assert!(helper.step(&tracker, settings).load);
    assert!(!helper.step(&tracker, settings).load);
}

#[test]
fn two_provisional_results_restore_a_distinct_full_history_in_order() {
    let mut helper = Helper::new();
    let mut target = Combo::EMPTY;
    target.input_only = true;
    helper.set_target(Some(target));
    let history = core::array::from_fn(|i| Some(Summary {
        outcome: Outcome::Completed, graded: RESULTS as u8, true_links: i as u8,
        total: RESULTS as u8, clean: true, fault: None,
    }));
    helper.results = history;
    helper.streak = RESULTS as u16;
    helper.best = (RESULTS - 1) as u8;
    let failed = Summary { outcome: Outcome::Completed, graded: 1, true_links: 0,
        total: 1, clean: false, fault: Some((0, Reason::Missing(Button::Special))) };
    helper.input.step(Snap { buttons: 0x8, ..Snap::empty(1) });
    helper.record(failed, &mut Out::default());
    helper.input.restart();
    helper.input.step(Snap { buttons: 0x9, ..Snap::empty(30) });
    helper.record(failed, &mut Out::default());
    assert_eq!(helper.results[0..2], [Some(failed); 2]);
    helper.clear();
    assert_eq!(helper.results, history);
    assert_eq!((helper.streak(), helper.best()), (RESULTS as u16, (RESULTS - 1) as u8));
}

#[test]
fn a_release_finalizes_a_result_after_capture_overflows() {
    let mut reference = LinkRec::INPUT;
    reference.events[0] = Some(Event { kind: Kind::Press(Button::Special), dir: Dir::Neutral, offset: 0 });
    let mut target = Combo::from_links([reference]).unwrap();
    target.input_only = true;
    let mut helper = Helper::new();
    helper.set_target(Some(target));
    let mut tracker = Tracker::new();
    let settings = Settings { load_after: true, ..ON };
    for _ in 0..2 {
        tracker.step(idle(), idle(), MappedInputs { buttons: 0x8, ..NONE });
        assert!(!helper.step(&tracker, settings).load);
    }
    assert_eq!(helper.results().count(), 1);
    for i in 0..200 {
        let buttons = 0x8 | if i % 2 == 0 { 1 } else { 0 };
        tracker.step(idle(), idle(), MappedInputs { buttons, ..NONE });
        assert!(!helper.step(&tracker, settings).load);
    }
    assert!(helper.input.combo.overflow);
    tracker.step(idle(), idle(), NONE);
    assert!(helper.step(&tracker, settings).load);
    helper.clear();
    assert_eq!(helper.results().count(), 1);
}

#[test]
fn a_new_final_result_keeps_an_older_deferred_load() {
    let mut helper = Helper::new();
    let mut target = Combo::EMPTY;
    target.input_only = true;
    helper.set_target(Some(target));
    let failed = Summary { outcome: Outcome::Completed, graded: 1, true_links: 0,
        total: 1, clean: false, fault: Some((0, Reason::Missing(Button::Special))) };
    helper.settings.load_after = true;
    helper.input.step(Snap { buttons: 0x8, ..Snap::empty(1) });
    helper.record(failed, &mut Out::default());
    assert!(!helper.pending_load);
    helper.input.restart();
    helper.input.step(Snap { buttons: 0x9, ..Snap::empty(30) });
    helper.input.step(Snap { buttons: 0x8, ..Snap::empty(31) });
    helper.settings.load_after = false;
    helper.record(failed, &mut Out::default());
    assert!(helper.pending_load);
    assert!(helper.provisional.iter().all(Option::is_none));
}

fn a_save_state_load_clears_the_attempt_and_the_next_one_starts_fresh() {
    let mut run = Run::new(with_target());
    let before = run.helper.seen();
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    let original = CaptureOwner::new(8, Some(crate::model::move_names::FighterId {
        chara: crate::model::hash40::hash40("ui_chara_pikachu"), specials: [0; 4],
    }));
    run.helper.retain_owner(before, || original);
    let before = run.helper.seen();
    // The strip clears while the load runs and starts counting again.
    run.tracker.clear();
    run.last_hit = None;
    run.frame = 0;
    run.step(idle(), NONE, ON);
    run.helper.retain_owner(before, || panic!("a no-link frame cannot replace the retained fighter"));
    assert_eq!(run.helper.retained_owner, original);
    assert_eq!(
        run.resolved().last().map(|s| s.outcome),
        Some(Outcome::Cleared)
    );
    assert_eq!(run.helper.results().count(), 1);
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    run.helper.retain_owner(before, || CaptureOwner::UNKNOWN);
    assert_eq!(run.helper.retained_owner, CaptureOwner::UNKNOWN, "a new unidentified take cannot borrow the previous owner");
    let last = *run.resolved().last().unwrap();
    assert!(last.clean, "{last:?}");
    assert_eq!(
        run.helper.results().map(|s| s.outcome).collect::<Vec<_>>(),
        [Outcome::Completed, Outcome::Cleared]
    );
}

fn aims_merge_left_and_right() {
    assert_eq!(Aim::of(None), Aim::Neutral);
    assert_eq!([0, 4].map(|b| Aim::of(Some(b))), [Aim::Side; 2]);
    assert_eq!([1, 2, 3].map(|b| Aim::of(Some(b))), [Aim::Up; 3]);
    assert_eq!([5, 6, 7].map(|b| Aim::of(Some(b))), [Aim::Down; 3]);
}

fn settings_read_the_three_rows() {
    let mut snapshot = crate::model::settings::Snapshot::factory();
    assert_eq!(Settings::from_snapshot(&snapshot), Settings::factory());
    snapshot.set(ROW_ON, &[1, 0]).unwrap();
    snapshot.set(ROW_TOLERANCE, &[0, 0, 1]).unwrap();
    snapshot.set(ROW_AFTER, &[0, 1]).unwrap();
    assert_eq!(
        Settings::from_snapshot(&snapshot),
        Settings {
            on: true,
            tolerance: 2,
            load_after: true,
            match_spot: false,
            match_stick: false,
            move_names: false
        }
    );
    snapshot.set(crate::model::spots::ROW_COMBO, &[1, 0]).unwrap();
    assert!(Settings::from_snapshot(&snapshot).match_spot);
}

fn link_with(verdict: Verdict, to: Move, offset: i32) -> LinkRec {
    LinkRec::presses(
        Move::Aerial,
        to,
        verdict,
        None,
        &[Event::press(Button::Attack, Aim::Up, offset)],
    )
}

fn summarized(reference: &LinkRec, attempt_link: &LinkRec, outcome: Outcome) -> Summary {
    summarized_with(reference, attempt_link, outcome, false)
}

fn summarized_with(reference: &LinkRec, attempt_link: &LinkRec, outcome: Outcome, match_spot: bool) -> Summary {
    let g = grade(reference, attempt_link);
    let mut grades = [None; LINKS];
    grades[0] = Some(g);
    let attempt = Attempt {
        exchange: 1,
        grades,
        len: 1,
        summary: None,
        junction: false,
    };
    summarize(&attempt, 1, outcome, 1, match_spot)
}

/// The second hit of `gap` frames after the CPU can act. `gap` 20 still links; 21 is a new exchange.
fn play_gap(helper: Helper, gap: u32) -> Run {
    let mut run = Run::new(helper);
    let first = run.hit(JAB, 3, ON);
    run.hit(UP_TILT, first + 22 + gap - STARTUP, ON);
    run.settle(ON);
    run
}

/// A true follow-up, then one more hit `gap` frames after that hit's free frame.
fn play_true_then_gap(helper: Helper, gap: u32) -> Run {
    let mut run = Run::new(helper);
    let first = run.hit(JAB, 3, ON);
    let second = run.hit(UP_TILT, first + RECOVERY + 1, ON);
    run.hit(UP_TILT, second + 22 + gap - STARTUP, ON);
    run.settle(ON);
    run
}

/// rc.12 (operator 2026-11-13, "yeah fix it"; docs lane boot docs-op3-0c628724, `out/op-combo-root`): the follow-up's events end at
/// its hit, so a button still held when the hit lands has no release in the attempt. The same jab > jab with A let go 1-2
/// frames after the hit was graded "L1 Attack release missing", not clean. Holding a button past the hit does not change
/// the move that came out: a reference release with no counterpart is not graded; the presses decide.
fn a_release_after_the_hit_is_not_missing() {
    let dir = Dir::from_aim(Aim::Neutral);
    let press = Event { kind: Kind::Press(Button::Attack), dir, offset: 0 };
    let release = Event { kind: Kind::Release(Button::Attack), dir, offset: 2 };
    let reference = LinkRec { events: retain([press, release]), presses_only: false, ..link_with(Verdict::True(Some(0)), Move::Jab, 0) };
    let held = LinkRec { events: retain([press]), ..reference };
    let graded = grade(&reference, &held);
    assert_eq!(graded.marks[0], Some(Mark::At(0)));
    assert_ne!(graded.marks[1], Some(Mark::Missing), "a release after the hit is not missing");
    assert!(graded.clean(1));
    assert_eq!(summarized(&reference, &held, Outcome::Completed).fault, None);
    // A missing press still is.
    let no_press = LinkRec { events: retain([release]), ..reference };
    assert_eq!(grade(&reference, &no_press).marks[0], Some(Mark::Missing));
}

fn a_step_drops_on_a_missing_press_a_late_press_or_the_exchange_end() {
    let reference = link_with(Verdict::True(None), Move::Jab, 0);
    let missing = LinkRec {
        events: [None; STEPS],
        ..reference
    };
    let missed = grade(&reference, &missing);
    assert_eq!(missed.marks[0], Some(Mark::Missing));
    assert!(!missed.clean(1));
    assert_eq!(
        summarized(&reference, &missing, Outcome::Completed).fault,
        Some((0, Reason::Missing(Button::Attack)))
    );

    let late = link_with(Verdict::True(None), Move::Jab, 3);
    let late_grade = grade(&reference, &late);
    assert_eq!(late_grade.worst(1), Some((0, 3)));
    assert!(!late_grade.clean(1) && late_grade.clean(3));
    assert_eq!(
        summarized(&reference, &late, Outcome::Completed).fault,
        Some((0, Reason::Off(Button::Attack, 3)))
    );

    let mut early = Run::new(with_target());
    early.combo(JAB, &[(UP_TILT, 1)], ON);
    assert!(early.resolved().is_empty(), "still in the exchange");
    early.settle(ON);
    let stopped = early.resolved()[0];
    assert_eq!(stopped.outcome, Outcome::Stopped);
    assert!(!stopped.clean);

    let linked = play_gap(Helper::new(), 20);
    assert_eq!(
        linked.tracker.exchange().0,
        1,
        "20 frames after the CPU can act stays one exchange"
    );
    assert!(matches!(
        linked.tracker.latest_link().map(|l| l.verdict),
        Some(Verdict::Drop(_))
    ));
    let counted = {
        let combo = linked
            .helper
            .last_combo()
            .expect("the 20-frame follow-up is a link");
        let mut helper = Helper::new();
        helper.set_target(Some(combo));
        play_gap(helper, 20)
    };
    let counted_summary = counted.resolved()[0];
    assert!(
        counted_summary.clean,
        "the same 20-frame gap counts: {counted_summary:?}"
    );
    assert_eq!(counted_summary.true_links, 0);

    let broken = play_gap(Helper::new(), 21);
    assert_eq!(
        broken.tracker.exchange().0,
        2,
        "21 frames opens a new exchange"
    );
    assert!(broken.tracker.latest_link().is_none());

    let recorded_gap = play_true_then_gap(Helper::new(), 20);
    let combo = recorded_gap.helper.last_combo().expect("two links");
    assert_eq!(combo.len(), 2);
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let still = play_true_then_gap(helper, 20);
    assert!(still.resolved()[0].clean, "{:?}", still.resolved());
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let dropped = play_true_then_gap(helper, 21);
    let dropped_summary = dropped.resolved()[0];
    assert_eq!(dropped_summary.outcome, Outcome::Stopped);
    assert!(!dropped_summary.clean);

    let mut shielded = Run::new(with_target());
    shielded.combo(JAB, &[(UP_TILT, 1)], ON);
    let links_before = shielded.tracker.history().count();
    shielded.shield_hit();
    assert_eq!(
        shielded.tracker.history().count(),
        links_before,
        "a shielded hit does not link"
    );
    let shield_summary = shielded.resolved()[0];
    assert_eq!(shield_summary.outcome, Outcome::Stopped);
    assert!(!shield_summary.clean);
}

fn inputs(buttons: u32, x: i8, y: i8) -> MappedInputs {
    MappedInputs {
        buttons,
        lstick_x: x,
        lstick_y: y,
        rstick_x: 0,
        rstick_y: 0,
    }
}

fn faced(facing: i8, airborne: bool) -> Sample {
    Sample {
        facing,
        airborne,
        ..idle()
    }
}

/// The events `LinkRec::of` keeps from these frames. Offsets are the tracker's frame numbers.
fn snap_link(frames: &[(MappedInputs, Sample)]) -> LinkRec {
    let mut tracker = Tracker::new();
    for &(input, sample) in frames {
        tracker.step(sample, idle(), input);
    }
    let (events, drift) = events_between(&tracker, 0, tracker.frame(), 0, false);
    LinkRec {
        from: Move::Aerial,
        to: Move::Aerial,
        verdict: Verdict::True(None),
        window: None,
        events,
        presses_only: false,
        drift,
        spot: None,
        shown: [0; 3],
    }
}

fn fault_text(
    reference: &LinkRec,
    attempt: &LinkRec,
) -> (Summary, crate::model::frame_strip_plan::Text) {
    let summary = summarized(reference, attempt, Outcome::Completed);
    (summary, crate::model::combo_plan::reason_text(&summary))
}

fn a_short_hop_link_drops_on_a_full_hop() {
    let jump = inputs(0x4, 0, 0);
    let mut early = vec![(jump, idle())];
    early.push((jump, idle()));
    early.push((NONE, idle()));
    let mut late = vec![(jump, idle())];
    for _ in 0..3 {
        late.push((jump, idle()));
    }
    late.push((NONE, idle()));
    let reference = snap_link(&early);
    let attempt = snap_link(&late);
    assert!(reference
        .events()
        .any(|event| event.kind == Kind::Release(Button::Jump)));
    let g = grade(&reference, &attempt);
    assert!(
        !g.clean(1),
        "the release is 2 frames later than the recorded one, and the tolerance is 1"
    );
    let (summary, text) = fault_text(&reference, &attempt);
    assert_eq!(
        summary.fault,
        Some((0, Reason::ReleaseOff(Button::Jump, 2)))
    );
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 Jump release 2f late")
    );
}

fn a_dash_flick_and_a_tap_jump_are_graded() {
    use crate::model::frame_strip::{flick_fresh, DASH_FLICK_X, DIVE_FLICK, JUMP_FLICK_Y};
    assert!(flick_fresh(0, DASH_FLICK_X) && !flick_fresh(DASH_FLICK_X, DASH_FLICK_X));
    assert!(flick_fresh(0, JUMP_FLICK_Y) && !flick_fresh(JUMP_FLICK_Y, JUMP_FLICK_Y));
    assert!(flick_fresh(0, DIVE_FLICK) && !flick_fresh(DIVE_FLICK, DIVE_FLICK));
    let right = faced(1, false);
    let dash = inputs(0, 120, 0);
    let tap = inputs(0x8000, 0, 100);
    let reference = snap_link(&[(dash, right), (tap, right)]);
    assert!(reference
        .events()
        .any(|event| event.kind == Kind::Flick(Flick::Dash)));
    assert!(reference
        .events()
        .any(|event| event.kind == Kind::Flick(Flick::TapJump)));
    assert!(grade(&reference, &reference).clean(1));
    let held = snap_link(&[(dash, right), (dash, right), (tap, right)]);
    assert_eq!(
        held.events()
            .filter(|event| event.kind == Kind::Flick(Flick::Dash))
            .count(),
        1,
        "a held stick is one dash"
    );
    let no_dash = snap_link(&[(tap, right)]);
    let (summary, text) = fault_text(&reference, &no_dash);
    assert_eq!(summary.fault, Some((0, Reason::MissingFlick(Flick::Dash))));
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 dash missing")
    );
    let button_jump = snap_link(&[(dash, right), (inputs(0x4, 0, 100), right)]);
    let (summary, text) = fault_text(&reference, &button_jump);
    assert_eq!(
        summary.fault,
        Some((0, Reason::MissingFlick(Flick::TapJump)))
    );
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 tap-jump missing")
    );
}

fn a_c_stick_event_is_graded_with_its_direction() {
    let right = faced(1, false);
    let c_forward = MappedInputs {
        buttons: 0x80,
        rstick_x: 120,
        ..NONE
    };
    let c_up = MappedInputs {
        buttons: 0x80,
        rstick_y: 120,
        ..NONE
    };
    let reference = snap_link(&[(c_forward, right)]);
    assert_eq!(
        reference.events().next(),
        Some(Event {
            kind: Kind::CStick,
            dir: Dir::Forward,
            offset: 1
        })
    );
    assert!(grade(&reference, &reference).clean(1));
    let wrong = snap_link(&[(c_up, right)]);
    let g = grade(&reference, &wrong);
    assert_eq!(g.marks[0], Some(Mark::WrongDir));
    assert!(!g.clean(1));
    let (summary, text) = fault_text(&reference, &wrong);
    assert_eq!(summary.fault, Some((0, Reason::WrongDir(Kind::CStick))));
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 C-stick wrong direction")
    );
    let (summary, text) = fault_text(&reference, &snap_link(&[(NONE, right)]));
    assert_eq!(summary.fault, Some((0, Reason::MissingCStick)));
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 C-stick missing")
    );
}

fn drift_is_shown_and_never_graded() {
    let hold = |x: i8| {
        snap_link(&[
            (inputs(0x1, 0, 0), idle()),
            (NONE, idle()),
            (inputs(0, x, 0), idle()),
            (inputs(0, x, 0), idle()),
        ])
    };
    let forward = hold(40);
    let back = hold(-40);
    assert_eq!(forward.events, back.events);
    assert_eq!(
        (forward.drift, back.drift),
        (Some(Dir::Forward), Some(Dir::Back))
    );
    let same = grade(&forward, &forward);
    let other = grade(&forward, &back);
    assert!(same.clean(1) && other.clean(1));
    assert_eq!(same.marks, other.marks);
    assert_eq!(
        crate::model::combo_plan::drift_text(Dir::Forward),
        crate::model::frame_strip_plan::Text::new("drift >")
    );
    assert_eq!(
        crate::model::combo_plan::drift_text(Dir::Back),
        crate::model::frame_strip_plan::Text::new("drift <")
    );
}

fn rename_keeps_the_events() {
    use crate::model::combo_record::{ComboLibrary, ComboRecord};
    let link = snap_link(&[
        (inputs(0x4, 0, 0), faced(1, true)),
        (NONE, faced(1, true)),
        (inputs(0, 0, -100), faced(1, true)),
    ]);
    assert!(link
        .events()
        .any(|event| matches!(event.kind, Kind::Release(_) | Kind::Flick(_))));
    let combo = Combo::from_links([link]).unwrap();
    let events = link.events;
    let dir = std::env::temp_dir().join(format!("tr-rr12-rename-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let library = ComboLibrary::new(&dir);
    let id = library
        .save_new(&ComboRecord::new("Nair".into(), combo, None))
        .unwrap();
    let mut loaded = library.load(&id).unwrap();
    assert_eq!(loaded.combo.get(0).unwrap().events, events);
    loaded.name = "Renamed".into();
    library.overwrite(&id, &loaded).unwrap();
    let again = library.load(&id).unwrap();
    assert_eq!(again.name, "Renamed");
    assert_eq!(again.combo.get(0).unwrap().events, events);
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- sequences (docs/RECORDING.md § Sequences and the one drill; RECORDING-REDESIGN Q8 A) --------------------------

/// A combo recorded from `opener` and `follow` (each follow-up pressed `delay` frames after P1 could act).
fn record(opener: Hit, follow: &[(Hit, i32)]) -> Combo {
    let mut run = Run::new(Helper::new());
    run.combo(opener, follow, ON);
    run.settle(ON);
    run.helper.last_combo().expect("the exchange is kept")
}

/// Combo 1: jab, up tilt (one link). Combo 2: down tilt, up tilt, up tilt (two links).
fn two_combos() -> Vec<Combo> {
    vec![
        record(JAB, &[(UP_TILT, 1)]),
        record(DOWN_TILT, &[(UP_TILT, 1), (UP_TILT, 1)]),
    ]
}

fn advances(run: &Run) -> Vec<(usize, Advance)> {
    run.stepped.iter().map(|s| (s.step, s.advance)).collect()
}

fn a_drop_in_combo_two_restarts_at_combo_one_with_the_streak_at_zero_and_best_kept() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 4, 9);
    run.combo(
        JAB,
        &[(UP_TILT, 1), (DOWN_TILT, 1), (UP_TILT, 1), (UP_TILT, 4)],
        ON,
    );
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Dropped)]
    );
    assert_eq!(
        run.stepped[1].summary.fault,
        Some((1, Reason::Off(Button::Attack, 3)))
    );
    let walk = run.walk();
    assert_eq!((walk.cursor(), walk.streak(), walk.best()), (0, 0, 9));
    assert_eq!(run.helper.target(), Some(&steps[0]));
    // Combo 1 starts again on the first link of a later exchange.
    run.settle(ON);
    run.combo(
        JAB,
        &[(UP_TILT, 1), (DOWN_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)],
        ON,
    );
    assert_eq!(
        &advances(&run)[2..],
        [(0, Advance::Next(1)), (1, Advance::Finished)]
    );
    assert_eq!((run.walk().streak(), run.walk().best()), (1, 9));
}

fn a_different_first_move_for_the_next_combo_is_a_wrong_move() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 0, 0);
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Dropped)]
    );
    assert_eq!(
        run.stepped[1].summary.fault,
        Some((0, Reason::WrongMove(Move::UpTilt)))
    );
    assert_eq!(run.helper.target(), Some(&steps[0]));
}

fn a_cleared_strip_between_the_combos_is_a_drop() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 3, 3);
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    run.clear();
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Dropped)]
    );
    assert_eq!(run.stepped[1].summary.outcome, Outcome::Cleared);
    assert_eq!((run.walk().streak(), run.walk().best()), (0, 3));
}

/// Decision SP4: with Match sweet/sour spot On a link counts only if it lands the saved link's spot; a link saved with no spot
/// always matches; Off compares nothing.
fn match_sweet_sour_spot_grades_the_spot_a_link_landed() {
    use crate::model::spots::Spot::{Sour, Sweet};
    let link = |spot| LinkRec { spot, ..link_with(Verdict::True(Some(0)), Move::Aerial, 0) };
    for (saved, got, passes) in [
        (Some(Sweet), Some(Sweet), true),
        (Some(Sweet), Some(Sour), false),
        (Some(Sour), Some(Sour), true),
        (Some(Sour), Some(Sweet), false),
        (None, Some(Sweet), true),
        (None, None, true),
        (Some(Sweet), None, false),
    ] {
        let (reference, attempt) = (link(saved), link(got));
        assert_eq!(grade(&reference, &attempt).wrong_spot, !passes, "{saved:?} / {got:?}");
        let on = summarized_with(&reference, &attempt, Outcome::Completed, true);
        assert_eq!(on.clean, passes, "{saved:?} / {got:?}");
        assert_eq!(on.fault, saved.filter(|_| !passes).map(|s| (0, Reason::WrongSpot(got, s))));
        let off = summarized_with(&reference, &attempt, Outcome::Completed, false);
        assert!(off.clean && off.fault.is_none(), "Off compares no spot");
    }
    // The wrong move is still named first.
    let wrong_move = LinkRec { to: Move::Jab, ..link(Some(Sour)) };
    assert_eq!(summarized_with(&link(Some(Sweet)), &wrong_move, Outcome::Completed, true).fault, Some((0, Reason::WrongMove(Move::Jab))));
}

fn direction_order_matches_original_mappings_and_ties() {
    fn original_facing_dir(bin: Option<u8>, facing: i8) -> Dir {
        let Some(bin) = bin else { return Dir::Neutral };
        let bin = if facing < 0 {
            match bin {
                0 => 4,
                1 => 3,
                3 => 1,
                4 => 0,
                5 => 7,
                7 => 5,
                other => other,
            }
        } else {
            bin
        };
        match bin {
            0 => Dir::Forward,
            1 => Dir::ForwardUp,
            2 => Dir::Up,
            3 => Dir::BackUp,
            4 => Dir::Back,
            5 => Dir::BackDown,
            6 => Dir::Down,
            _ => Dir::ForwardDown,
        }
    }

    fn original_drift_index(dir: Dir) -> Option<usize> {
        Some(match dir {
            Dir::Forward => 0,
            Dir::ForwardUp => 1,
            Dir::Up => 2,
            Dir::BackUp => 3,
            Dir::Back => 4,
            Dir::BackDown => 5,
            Dir::Down => 6,
            Dir::ForwardDown => 7,
            Dir::Neutral | Dir::Side => return None,
        })
    }

    fn original_modal_drift(counts: &[u8; 8], order: &[u8; 8], seen: usize) -> Option<Dir> {
        let dirs = [
            Dir::Forward,
            Dir::ForwardUp,
            Dir::Up,
            Dir::BackUp,
            Dir::Back,
            Dir::BackDown,
            Dir::Down,
            Dir::ForwardDown,
        ];
        let mut best: Option<Dir> = None;
        let mut best_count = 0u8;
        for index in order.iter().take(seen).map(|index| usize::from(*index)) {
            if counts[index] > best_count {
                best_count = counts[index];
                best = Some(dirs[index]);
            }
        }
        best
    }

    for facing in i8::MIN..=i8::MAX {
        assert_eq!(facing_dir(None, facing), original_facing_dir(None, facing));
        for bin in u8::MIN..=u8::MAX {
            assert_eq!(facing_dir(Some(bin), facing), original_facing_dir(Some(bin), facing),
                "bin={bin} facing={facing}");
        }
    }
    let dirs = [Dir::Forward, Dir::ForwardUp, Dir::Up, Dir::BackUp,
        Dir::Back, Dir::BackDown, Dir::Down, Dir::ForwardDown, Dir::Neutral, Dir::Side];
    for dir in dirs {
        assert_eq!(drift_index(dir), original_drift_index(dir));
    }
    // Every tie winner and runner-up, both encounter orders, zero and saturated counts.
    for first in 0..8 {
        for second in 0..8 {
            if first == second { continue; }
            let mut order = [0; 8];
            order[0] = first as u8;
            order[1] = second as u8;
            for count in [0, 1, 2, 254, 255] {
                let mut counts = [0; 8];
                counts[first] = count;
                counts[second] = count;
                for seen in 0..=2 {
                    assert_eq!(modal_drift(&counts, &order, seen),
                        original_modal_drift(&counts, &order, seen));
                }
                assert_eq!(modal_drift(&counts, &order, 2),
                    (count != 0).then_some(dirs[first]));
                if count < 255 {
                    counts[second] = count + 1;
                    assert_eq!(modal_drift(&counts, &order, 2), Some(dirs[second]));
                    assert_eq!(modal_drift(&counts, &order, 2),
                        original_modal_drift(&counts, &order, 2));
                }
            }
        }
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("direction_order_matches_original_mappings_and_ties", direction_order_matches_original_mappings_and_ties as fn()),
        ("a_save_state_load_clears_the_attempt_and_the_next_one_starts_fresh", a_save_state_load_clears_the_attempt_and_the_next_one_starts_fresh as fn()),
        ("aims_merge_left_and_right", aims_merge_left_and_right as fn()),
        ("settings_read_the_three_rows", settings_read_the_three_rows as fn()),
        ("match_sweet_sour_spot_grades_the_spot_a_link_landed", match_sweet_sour_spot_grades_the_spot_a_link_landed as fn()),
        ("a_step_drops_on_a_missing_press_a_late_press_or_the_exchange_end", a_step_drops_on_a_missing_press_a_late_press_or_the_exchange_end as fn()),
        ("a_release_after_the_hit_is_not_missing", a_release_after_the_hit_is_not_missing as fn()),
        ("a_short_hop_link_drops_on_a_full_hop", a_short_hop_link_drops_on_a_full_hop as fn()),
        ("a_dash_flick_and_a_tap_jump_are_graded", a_dash_flick_and_a_tap_jump_are_graded as fn()),
        ("a_c_stick_event_is_graded_with_its_direction", a_c_stick_event_is_graded_with_its_direction as fn()),
        ("drift_is_shown_and_never_graded", drift_is_shown_and_never_graded as fn()),
        ("rename_keeps_the_events", rename_keeps_the_events as fn()),
        ("a_drop_in_combo_two_restarts_at_combo_one_with_the_streak_at_zero_and_best_kept", a_drop_in_combo_two_restarts_at_combo_one_with_the_streak_at_zero_and_best_kept as fn()),
        ("a_different_first_move_for_the_next_combo_is_a_wrong_move", a_different_first_move_for_the_next_combo_is_a_wrong_move as fn()),
        ("a_cleared_strip_between_the_combos_is_a_drop", a_cleared_strip_between_the_combos_is_a_drop as fn()),
    ]);
}
}
