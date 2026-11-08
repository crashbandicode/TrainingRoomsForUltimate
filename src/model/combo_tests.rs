//! SPEC-input-07 §3 on the strip's own `Link` records: scripted frames step a real `Tracker`, and the helper steps
//! after it as the runtime does. Frame numbers are the tracker's (the first `step` is frame 1).

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

    fn graded(&self) -> Vec<(usize, Grade)> {
        self.outs.iter().filter_map(|o| o.link).collect()
    }
}

const ON: Settings = Settings {
    on: true,
    tolerance: 1,
    load_after: false,
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
fn the_capture_keeps_each_link_with_its_follow_up_presses() {
    let combo = recorded();
    assert_eq!(combo.len(), 3);
    let first = combo.get(0).unwrap();
    assert_eq!((first.from, first.to), (Move::Jab, Move::UpTilt));
    assert_eq!(
        first.verdict,
        Verdict::True(Some(-3)),
        "hit 3 frames before the CPU was free"
    );
    assert_eq!(
        first.events().collect::<Vec<_>>(),
        [
            Event {
                kind: Kind::Press(Button::Attack),
                dir: Dir::Up,
                offset: 1
            },
            Event {
                kind: Kind::Release(Button::Attack),
                dir: Dir::Neutral,
                offset: 2
            }
        ]
    );
    assert!(
        first.drift.is_none(),
        "the stick is neutral on every frame that is not an event"
    );
    assert_eq!(
        first.window.map(|(a, _)| a),
        Some(0),
        "the window starts at A1"
    );
    let rest = combo.get(1).unwrap();
    assert_eq!((rest.from, rest.to), (Move::UpTilt, Move::UpTilt));
}

#[test]
fn a_clean_attempt_completes_with_every_link_true_and_on_time() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    let graded = run.graded();
    assert_eq!(
        graded.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert!(graded
        .iter()
        .all(|(_, g)| g.clean(1) && g.marks[0] == Some(Mark::At(0))));
    let [summary] = run.resolved()[..] else {
        panic!("one result: {:?}", run.resolved())
    };
    assert_eq!(
        summary,
        Summary {
            outcome: Outcome::Completed,
            graded: 3,
            true_links: 3,
            total: 3,
            clean: true,
            fault: None
        }
    );
    assert_eq!((run.helper.streak(), run.helper.best()), (1, 3));
}

#[test]
fn a_late_press_inside_the_tolerance_is_on_time_and_outside_it_is_named() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 2), (UP_TILT, 1), (UP_TILT, 3)], ON);
    let summary = run.resolved()[0];
    assert_eq!(
        summary.true_links, 3,
        "still true combos (the CPU is free 9 frames after A1)"
    );
    assert!(!summary.clean);
    assert_eq!(
        summary.fault,
        Some((2, Reason::Off(Button::Attack, 2))),
        "link 1 is +1 (inside), link 3 is +2"
    );
    let wide = Settings { tolerance: 2, ..ON };
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 2), (UP_TILT, 1), (UP_TILT, 3)], wide);
    assert!(run.resolved()[0].clean, "within 2 frames everywhere");
}

#[test]
fn a_dropped_link_is_named_with_its_frames() {
    let mut run = Run::new(with_target());
    // A1 + 5: the hit lands on A1 + 10, one frame after the CPU was free (hit + 22 = A1 + 9).
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 5), (UP_TILT, 1)], ON);
    let summary = run.resolved()[0];
    assert_eq!(
        (summary.outcome, summary.true_links, summary.total),
        (Outcome::Completed, 2, 3)
    );
    // The hit is still a DROP on the grade. It is also four frames late, and that press is the fault:
    // TRUE is not required (operator 2026-11-06, answer 1).
    let (_, dropped) = run.graded()[1];
    assert_eq!(dropped.link.verdict, Verdict::Drop(1));
    assert!(!summary.clean);
    assert_eq!(summary.fault, Some((1, Reason::Off(Button::Attack, 4))));
    assert_eq!(run.helper.streak(), 0);
}

#[test]
fn a_different_follow_up_is_a_wrong_move() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1), (DOWN_TILT, 1), (UP_TILT, 1)], ON);
    let graded = run.graded();
    let (_, second) = graded[1];
    assert!(second.wrong_move && !second.is_true());
    assert_eq!(
        second.marks[0],
        Some(Mark::WrongDir),
        "the attack is down, and the recorded one is up"
    );
    assert_eq!(second.extra, 0, "the release is the same event");
    assert_eq!(
        run.resolved()[0].fault,
        Some((1, Reason::WrongMove(Move::DownTilt)))
    );
}

#[test]
fn an_exchange_that_ends_early_stops_the_attempt() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    assert!(run.resolved().is_empty(), "the CPU is still in hitstun");
    run.settle(ON);
    let summary = run.resolved()[0];
    assert_eq!(
        (
            summary.outcome,
            summary.graded,
            summary.true_links,
            summary.total
        ),
        (Outcome::Stopped, 1, 1, 3)
    );
    assert!(!summary.clean);
}

#[test]
fn an_attempt_starts_only_on_the_first_link_of_an_exchange_with_the_reference_opener() {
    let mut run = Run::new(with_target());
    // An up tilt opener: the reference opens with a jab.
    run.combo(UP_TILT, &[(UP_TILT, 1), (UP_TILT, 1)], ON);
    run.settle(ON);
    assert!(run.graded().is_empty() && run.resolved().is_empty());
    // Turned On mid-combo: the open exchange's later links start nothing.
    let mut run = Run::new(with_target());
    let off = Settings { on: false, ..ON };
    let start = run.frame + 3;
    let hit = run.hit(JAB, start, off);
    let hit = run.hit(UP_TILT, hit + RECOVERY + 1, off);
    run.hit(UP_TILT, hit + RECOVERY + 1, ON);
    assert!(run.graded().is_empty());
}

#[test]
fn off_or_without_a_target_nothing_is_graded_but_the_combo_is_kept() {
    let mut run = Run::new(Helper::new());
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    assert!(run.graded().is_empty());
    assert_eq!(run.helper.last_combo().map(|c| c.len()), Some(1));
    let mut run = Run::new(with_target());
    let off = Settings { on: false, ..ON };
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], off);
    assert!(run.graded().is_empty() && run.resolved().is_empty());
}

#[test]
fn a_multi_hit_opener_is_one_link() {
    // The opener connects twice in one action: the strip links only the follow-up.
    let mut run = Run::new(Helper::new());
    run.idle_until(3, ON);
    for f in 0..STARTUP {
        run.step(
            attack(Move::Aerial, 0x50, f as f32),
            if f == 0 { A } else { NONE },
            ON,
        );
    }
    let mut motion = STARTUP as f32;
    for hit in 0..2 {
        run.last_hit = Some(run.frame + 1);
        for _ in 0..STOP {
            run.step(
                Sample {
                    stop: true,
                    active: true,
                    ..attack(Move::Aerial, 0x50, motion)
                },
                NONE,
                ON,
            );
        }
        motion += 1.0;
        run.step(
            Sample {
                active: true,
                ..attack(Move::Aerial, 0x50, motion)
            },
            NONE,
            ON,
        );
        motion += 1.0;
        if hit == 0 {
            run.step(attack(Move::Aerial, 0x50, motion), NONE, ON);
            motion += 1.0;
        }
    }
    for _ in 0..6 {
        run.step(attack(Move::Aerial, 0x50, motion), NONE, ON);
        motion += 1.0;
    }
    let (exchange, _) = run.tracker.exchange();
    assert_eq!(exchange, 1, "both hits are one exchange");
    let a1 = run.frame + 1;
    run.hit(UP_TILT, a1 + 1, ON);
    run.settle(ON);
    let combo = run.helper.last_combo().unwrap();
    assert_eq!(combo.len(), 1);
    assert_eq!(
        (combo.get(0).unwrap().from, combo.get(0).unwrap().to),
        (Move::Aerial, Move::UpTilt)
    );
}

#[test]
fn a_save_state_load_clears_the_attempt_and_the_next_one_starts_fresh() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    // The strip clears while the load runs and starts counting again.
    run.tracker.clear();
    run.last_hit = None;
    run.frame = 0;
    run.step(idle(), NONE, ON);
    assert_eq!(
        run.resolved().last().map(|s| s.outcome),
        Some(Outcome::Cleared)
    );
    assert_eq!(run.helper.results().count(), 1);
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    let last = *run.resolved().last().unwrap();
    assert!(last.clean, "{last:?}");
    assert_eq!(
        run.helper.results().map(|s| s.outcome).collect::<Vec<_>>(),
        [Outcome::Completed, Outcome::Cleared]
    );
}

#[test]
fn load_after_an_attempt_waits_for_the_exchange_to_close() {
    let load = Settings {
        load_after: true,
        ..ON
    };
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], load);
    assert!(run.resolved()[0].clean);
    assert!(
        !run.outs.iter().any(|o| o.load),
        "the CPU is still in the last hit's hitstun"
    );
    run.settle(load);
    assert_eq!(
        run.outs.iter().filter(|o| o.load).count(),
        1,
        "one load request"
    );
}

#[test]
fn a_new_target_clears_the_history() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    assert_eq!(run.helper.results().count(), 1);
    run.helper.set_target(Some(recorded()));
    assert_eq!(
        (
            run.helper.results().count(),
            run.helper.streak(),
            run.helper.best()
        ),
        (0, 0, 0)
    );
    assert!(run.helper.attempt().is_none());
}

#[test]
fn the_history_keeps_the_last_eight_newest_first() {
    let mut run = Run::new(with_target());
    for i in 0..10 {
        let delay = if i == 9 { 5 } else { 1 };
        run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, delay), (UP_TILT, 1)], ON);
        run.settle(ON);
    }
    let results: Vec<_> = run.helper.results().collect();
    assert_eq!(results.len(), RESULTS);
    assert!(!results[0].clean && results[1].clean, "newest first");
    assert_eq!((run.helper.streak(), run.helper.best()), (0, 3));
}

#[test]
fn grading_matches_steps_in_order_by_button_and_aim() {
    let press = |button, aim, offset| Event::press(button, aim, offset);
    let link = |presses: &[Event]| {
        LinkRec::presses(
            Move::Aerial,
            Move::Aerial,
            Verdict::True(None),
            None,
            presses,
        )
    };
    let reference = link(&[
        press(Button::Jump, Aim::Neutral, 2),
        press(Button::Attack, Aim::Neutral, 5),
    ]);
    // A short hop 2 frames late and the aerial on time.
    let g = grade(
        &reference,
        &link(&[
            press(Button::Jump, Aim::Neutral, 4),
            press(Button::Attack, Aim::Neutral, 5),
        ]),
    );
    assert_eq!(g.marks[0], Some(Mark::At(2)));
    assert_eq!(g.marks[1], Some(Mark::At(0)));
    assert!(g.marks[2..].iter().all(|mark| mark.is_none()));
    assert_eq!(g.worst(1), Some((0, 2)));
    assert!(g.is_true() && !g.clean(1) && g.clean(2));
    // A forward air: the attack's aim differs, so it is missing and the press is extra.
    let g = grade(
        &reference,
        &link(&[
            press(Button::Jump, Aim::Neutral, 2),
            press(Button::Attack, Aim::Side, 5),
        ]),
    );
    assert_eq!(g.marks[1], Some(Mark::Missing));
    assert_eq!((g.missing(), g.extra), (Some(1), 1));
    // Order: an attack before the jump does not match the reference's attack, which comes after the jump.
    let g = grade(
        &reference,
        &link(&[
            press(Button::Attack, Aim::Neutral, 0),
            press(Button::Jump, Aim::Neutral, 2),
        ]),
    );
    assert_eq!(g.marks[..2], [Some(Mark::At(0)), Some(Mark::Missing)]);
}

#[test]
fn aims_merge_left_and_right() {
    assert_eq!(Aim::of(None), Aim::Neutral);
    assert_eq!([0, 4].map(|b| Aim::of(Some(b))), [Aim::Side; 2]);
    assert_eq!([1, 2, 3].map(|b| Aim::of(Some(b))), [Aim::Up; 3]);
    assert_eq!([5, 6, 7].map(|b| Aim::of(Some(b))), [Aim::Down; 3]);
}

#[test]
fn a_fresh_profile_grades_a_drill_without_touching_options() {
    let fresh = Settings::factory();
    assert!(fresh.on, "the factory helper is On");
    assert_eq!(Settings::from_snapshot(&crate::model::settings::Snapshot::factory()), fresh);
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1)], fresh);
    run.settle(fresh);
    let summary = run.resolved()[0];
    assert_eq!((summary.graded, summary.true_links), (1, 1));
}

#[test]
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
            load_after: true
        }
    );
}

#[test]
fn a_combo_recorded_on_another_fighter_grades_whatever_fighter_p1_is() {
    use crate::model::combo_record::{ComboLibrary, ComboRecord};
    // Saved while P1 was Ryu, read back from its file. The helper's input (`Sample`) carries no fighter: it grades the
    // moves and timing of whatever fighter P1 is now.
    let dir = std::env::temp_dir().join(format!("tr-combo-fighter-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let library = ComboLibrary::new(&dir);
    let id = library
        .save_new(&ComboRecord::new(
            "Ryu jabs".into(),
            recorded(),
            Some("Ryu".into()),
        ))
        .unwrap();
    let back = library.load(&id).unwrap();
    assert_eq!(back.recorded_on.as_deref(), Some("Ryu"));
    let mut helper = Helper::new();
    helper.set_target(Some(back.combo));
    let mut run = Run::new(helper);
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    let [summary] = run.resolved()[..] else {
        panic!("one result: {:?}", run.resolved())
    };
    assert_eq!(
        (
            summary.outcome,
            summary.true_links,
            summary.total,
            summary.clean
        ),
        (Outcome::Completed, 3, 3, true)
    );
    let _ = std::fs::remove_dir_all(&dir);
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
    summarize(&attempt, 1, outcome, 1)
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

#[test]
fn clean_passes_a_non_true_link_inside_a_combo_and_a_sequence() {
    let reference = link_with(Verdict::True(Some(-3)), Move::Aerial, 1);
    let attempt = link_with(Verdict::Drop(4), Move::Aerial, 1);
    let g = grade(&reference, &attempt);
    assert!(!g.is_true() && !g.wrong_move);
    assert_eq!(
        g.marks[0],
        Some(Mark::At(0)),
        "same button, direction and order"
    );
    assert!(g.clean(1));
    let summary = summarized(&reference, &attempt, Outcome::Completed);
    assert!(summary.clean && summary.fault.is_none() && summary.true_links == 0);

    let mut walk = crate::model::drill::SequenceWalk::new(2);
    walk.note_summary(&summary);
    assert_eq!(walk.cursor(), 1, "the sequence counts the non-true link");
    assert_eq!((walk.streak(), walk.best()), (0, 0));
}

#[test]
fn a_different_move_drops_even_when_the_presses_match() {
    let reference = link_with(Verdict::True(None), Move::UpTilt, 1);
    let attempt = link_with(Verdict::True(None), Move::DownTilt, 1);
    let g = grade(&reference, &attempt);
    assert!(g.wrong_move);
    assert_eq!(g.marks[0], Some(Mark::At(0)));
    assert_eq!(g.extra, 0);
    assert!(!g.clean(1));
    let summary = summarized(&reference, &attempt, Outcome::Completed);
    assert!(!summary.clean);
    assert_eq!(summary.fault, Some((0, Reason::WrongMove(Move::DownTilt))));
}

#[test]
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

#[test]
fn a_single_combo_keeps_todays_best() {
    let mut run = Run::new(with_target());
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)], ON);
    run.settle(ON);
    assert_eq!((run.helper.streak(), run.helper.best()), (1, 3));
    run.combo(JAB, &[(UP_TILT, 1), (UP_TILT, 5), (UP_TILT, 1)], ON);
    run.settle(ON);
    assert_eq!(
        (run.helper.streak(), run.helper.best()),
        (0, 3),
        "a miss clears the streak and keeps best"
    );

    let captured = play_gap(Helper::new(), 20);
    let combo = captured.helper.last_combo().expect("linked");
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let once = play_gap(helper, 20);
    assert!(once.resolved()[0].clean);
    assert_eq!(once.resolved()[0].true_links, 0);
    assert_eq!((once.helper.streak(), once.helper.best()), (1, 0));
    let twice = play_gap(once.helper, 20);
    assert_eq!(
        (twice.helper.streak(), twice.helper.best()),
        (2, 0),
        "best stays the true-link count, not the streak"
    );
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
    let (events, drift) = events_between(&tracker, 0, tracker.frame(), 0);
    LinkRec {
        from: Move::Aerial,
        to: Move::Aerial,
        verdict: Verdict::True(None),
        window: None,
        events,
        presses_only: false,
        drift,
    }
}

fn fault_text(
    reference: &LinkRec,
    attempt: &LinkRec,
) -> (Summary, crate::model::frame_strip_plan::Text) {
    let summary = summarized(reference, attempt, Outcome::Completed);
    (summary, crate::model::combo_plan::reason_text(&summary))
}

#[test]
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

#[test]
fn a_missing_fast_fall_flick_drops() {
    let down = inputs(0, 0, -100);
    let air = faced(1, true);
    let reference = snap_link(&[(NONE, air), (down, air)]);
    assert!(reference
        .events()
        .any(|event| event.kind == Kind::Flick(Flick::FastFall)));
    let grounded = snap_link(&[(NONE, idle()), (down, idle())]);
    assert!(
        grounded
            .events()
            .all(|event| event.kind != Kind::Flick(Flick::FastFall)),
        "a crouch uses the same stick line"
    );
    let g = grade(&reference, &grounded);
    assert_eq!(g.marks[0], Some(Mark::Missing));
    assert!(!g.clean(1));
    let (summary, text) = fault_text(&reference, &grounded);
    assert_eq!(
        summary.fault,
        Some((0, Reason::MissingFlick(Flick::FastFall)))
    );
    assert_eq!(
        text,
        crate::model::frame_strip_plan::Text::new("L1 fast-fall missing")
    );
}

#[test]
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

#[test]
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

#[test]
fn directions_are_relative_to_facing() {
    let toward = |facing: i8, x: i8| snap_link(&[(inputs(0x1, x, 0), faced(facing, false))]);
    let right = toward(1, 120);
    let left = toward(-1, -120);
    assert_eq!(
        right.events, left.events,
        "forward is forward either way the fighter faces"
    );
    assert!(grade(&right, &left).clean(1));
    let back = toward(1, -120);
    let g = grade(&right, &back);
    assert_eq!(g.marks[0], Some(Mark::WrongDir));
    assert!(!g.clean(1));
}

#[test]
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

#[test]
fn an_old_presses_only_combo_still_grades_as_before() {
    let text = r#"{"schema_version":1,"name":"Old","links":[{"from":"aerial","to":"aerial","verdict":{"kind":"true","link":null},"window":null,"steps":[{"button":"attack","aim":"side","offset":1}]}]}"#;
    let record: crate::model::combo_record::ComboRecord = serde_json::from_str(text).unwrap();
    let reference = *record.combo.get(0).unwrap();
    assert!(reference.presses_only);
    assert!(record.combo.links().all(|link| link
        .events()
        .all(|event| matches!(event.kind, Kind::Press(_)))));
    let side = LinkRec::presses(
        Move::Aerial,
        Move::Aerial,
        Verdict::True(None),
        None,
        &[Event::press(Button::Attack, Aim::Side, 1)],
    );
    assert!(grade(&reference, &side).clean(1));
    let right = snap_link(&[(inputs(0x1, 50, 0), faced(1, false))]);
    let left = snap_link(&[(inputs(0x1, -50, 0), faced(1, false))]);
    assert!(
        grade(&reference, &right).clean(1) && grade(&reference, &left).clean(1),
        "left and right are still one aim"
    );
    let up = LinkRec::presses(
        Move::Aerial,
        Move::Aerial,
        Verdict::True(None),
        None,
        &[Event::press(Button::Attack, Aim::Up, 1)],
    );
    let g = grade(&reference, &up);
    assert_eq!(g.marks[0], Some(Mark::Missing));
    assert!(
        g.wrong_dir().is_none(),
        "a different aim stays a missing press"
    );
}

#[test]
fn a_link_holds_twelve_events() {
    let mut frames = Vec::new();
    for i in 0..13 {
        let buttons = if i % 2 == 0 { 0x1 } else { 0 };
        frames.push((inputs(buttons, 0, 0), idle()));
    }
    let link = snap_link(&frames);
    let kept: Vec<_> = link.events().collect();
    assert_eq!(kept.len(), 12);
    assert!(
        kept.iter().all(|event| event.offset != 13),
        "the 13th event, the press on frame 13, is dropped"
    );
    assert_eq!(kept[0].offset, 1);
    assert_eq!(kept[11].offset, 12);
    let extra = retain((0..13).map(|offset| Event::press(Button::Attack, Aim::Neutral, offset)));
    assert_eq!(extra.iter().flatten().count(), 12);
    assert!(extra.iter().flatten().all(|event| event.offset != 12));
}

#[test]
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

#[test]
fn save_combo_from_live_play_records_events() {
    // Save last combo and Make combo both read this capture. Make combo stores the id only (`Drill::make_combo`).
    let mut run = Run::new(Helper::new());
    let opener = run.hit(JAB, 3, ON);
    let air = faced(1, true);
    let jump = inputs(0x4, 0, 0);
    run.step(air, jump, ON);
    run.step(air, jump, ON);
    run.step(air, NONE, ON);
    run.step(air, inputs(0, 0, -100), ON);
    let follow = (opener + RECOVERY + 1).max(run.frame + STARTUP + 1);
    run.hit(UP_TILT, follow, ON);
    run.settle(ON);
    let combo = run
        .helper
        .last_combo()
        .expect("the capture Save last combo writes");
    let kinds: Vec<_> = combo
        .links()
        .flat_map(|link| link.events())
        .map(|event| event.kind)
        .collect();
    assert!(kinds.contains(&Kind::Press(Button::Jump)), "{kinds:?}");
    assert!(kinds.contains(&Kind::Release(Button::Jump)), "{kinds:?}");
    assert!(kinds.contains(&Kind::Flick(Flick::FastFall)), "{kinds:?}");
    let record = crate::model::combo_record::ComboRecord::new("Live".into(), combo, None);
    let value = serde_json::to_value(&record).unwrap();
    let events = value["links"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|link| link["events"].as_array().unwrap().iter());
    let events: Vec<_> = events.collect();
    assert!(events
        .iter()
        .any(|event| event["kind"] == "press" && event["button"] == "jump"));
    assert!(events
        .iter()
        .any(|event| event["kind"] == "release" && event["button"] == "jump"));
    assert!(events
        .iter()
        .any(|event| event["kind"] == "flick" && event["flick"] == "fast_fall"));
    let mut drill = crate::model::drill::OneDrill::new();
    assert!(drill.current().is_none());
    assert_eq!(drill.make_combo("live").as_str(), "live");
    assert!(
        drill.current().is_none(),
        "Make combo stores the id and leaves the drill"
    );
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

#[test]
fn a_sequence_goes_from_combo_one_to_combo_two_inside_one_exchange() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 0, 0);
    // Jab > up tilt is combo 1; up tilt > down tilt joins them and is not graded; then combo 2.
    run.combo(
        JAB,
        &[(UP_TILT, 1), (DOWN_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)],
        ON,
    );
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Finished)]
    );
    assert!(run
        .stepped
        .iter()
        .all(|s| s.summary.clean && s.summary.outcome == Outcome::Completed));
    assert_eq!(
        run.stepped[1].summary.graded, 2,
        "combo 2's own two links, not the link between the combos"
    );
    assert_eq!(
        run.graded()
            .iter()
            .map(|(i, g)| (*i, g.reference.from))
            .collect::<Vec<_>>(),
        [(0, Move::Jab), (0, Move::DownTilt), (1, Move::UpTilt)],
        "the link between the combos is not graded"
    );
    let walk = run.walk();
    assert_eq!(
        (walk.cursor(), walk.streak(), walk.best()),
        (0, 1, 1),
        "a finished sequence adds one to the streak"
    );
    assert_eq!(run.helper.target(), Some(&steps[0]), "back at combo 1");
}

#[test]
fn the_link_between_the_combos_need_not_be_true() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 0, 0);
    // Down tilt pressed 12 frames after P1 could act: the CPU was free first, a DROP link inside the strip's 20 frames.
    run.combo(
        JAB,
        &[(UP_TILT, 1), (DOWN_TILT, 12), (UP_TILT, 1), (UP_TILT, 1)],
        ON,
    );
    let joining = run
        .tracker
        .history()
        .find(|link| link.to == Move::DownTilt)
        .expect("the link between the combos");
    assert!(
        matches!(joining.verdict, Verdict::Drop(_)),
        "{:?}",
        joining.verdict
    );
    assert_eq!(
        run.graded().len(),
        3,
        "combo 1's link and combo 2's two links are graded"
    );
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Finished)]
    );
}

#[test]
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

#[test]
fn the_exchange_ending_before_the_next_combo_is_a_drop() {
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 2, 2);
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    run.settle(ON);
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Dropped)]
    );
    let gap = run.stepped[1].summary;
    assert_eq!(
        (gap.outcome, gap.graded, gap.clean),
        (Outcome::Stopped, 0, false)
    );
    let walk = run.walk();
    assert_eq!((walk.cursor(), walk.streak(), walk.best()), (0, 0, 2));
}

#[test]
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

#[test]
fn a_one_combo_sequence_counts_each_clean_attempt() {
    let steps = vec![record(JAB, &[(UP_TILT, 1)])];
    let mut run = Run::drilling(&steps, 0, 0);
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    run.settle(ON);
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    run.settle(ON);
    assert_eq!(
        advances(&run),
        [(0, Advance::Finished), (0, Advance::Finished)]
    );
    assert_eq!((run.walk().streak(), run.walk().best()), (2, 2));
}

#[test]
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

#[test]
fn load_after_a_sequence_waits_for_its_exchange_to_close() {
    let load = Settings {
        load_after: true,
        ..ON
    };
    let steps = two_combos();
    let mut run = Run::drilling(&steps, 0, 0);
    run.combo(
        JAB,
        &[(UP_TILT, 1), (DOWN_TILT, 1), (UP_TILT, 1), (UP_TILT, 1)],
        load,
    );
    assert!(
        !run.outs.iter().any(|o| o.load),
        "combo 2 followed in the same exchange"
    );
    run.settle(load);
    assert_eq!(
        run.outs.iter().filter(|o| o.load).count(),
        1,
        "one load, after the sequence"
    );
    assert_eq!(
        advances(&run),
        [(0, Advance::Next(1)), (1, Advance::Finished)]
    );
}

#[test]
fn an_empty_sequence_drills_nothing() {
    let mut run = Run::drilling(&[], 0, 0);
    run.combo(JAB, &[(UP_TILT, 1)], ON);
    run.settle(ON);
    assert!(run.helper.target().is_none());
    assert!(run.stepped.is_empty() && run.graded().is_empty());
}
