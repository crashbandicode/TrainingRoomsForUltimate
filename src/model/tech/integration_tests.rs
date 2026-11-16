use super::*;
use crate::model::tech_launch::Surface;
use controller::{Controller, Event, Outcome, Setup};
use session::{Decision, Direction, Grade, Length, Phase};
use surface::{Kind, Line, Source, Surfaces};

struct Fake {
    attempt: Attempt,
    placements: u32,
    launches: u32,
    restores: u32,
    busy: bool,
    reject_launch: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            attempt: Attempt::IDLE,
            placements: 0,
            launches: 0,
            restores: 0,
            busy: false,
            reject_launch: false,
        }
    }
}
impl Launcher for Fake {
    type Error = ();
    fn place(&mut self, _: Launch) -> Result<(), ()> {
        self.placements += 1;
        self.attempt = Attempt::held(u64::from(self.placements));
        Ok(())
    }
    fn launch(&mut self) -> Result<(), ()> {
        if self.reject_launch {
            return Err(());
        }
        self.launches += 1;
        self.attempt.launched();
        Ok(())
    }
    fn attempt(&self) -> Result<Attempt, ()> {
        if self.busy {
            Err(())
        } else {
            Ok(self.attempt)
        }
    }
    fn restore(&mut self) {
        self.restores += 1;
    }
}
fn setup() -> Setup {
    Setup {
        launch: Launch {
            start: Vec2 { x: 0., y: 30. },
            target: Vec2::default(),
            angle: 270.,
            knockback: 100.,
            percent: 40.,
            hit_damage: 12.,
            height: HitHeight::Middle,
            facing: 1.,
            hit_from_behind: false,
            grounded_hit: false,
            di: Some(Vec2::default()),
        },
        delay: 2,
    }
}
fn decision() -> Decision {
    Decision {
        tech: Some(Direction::WallJump),
        count: 4,
        previous: 60,
        window: 12.0,
        lockout: 28,
        reaction: 20.,
        damage_fall: false,
        enabled: true,
        press_frame_offset: Some(0),
    }
}
#[test]
fn launcher_receipt_busy_never_fabricates_contact_or_grade() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    assert_eq!(
        c.start(Length::Attempts(10)),
        Event::NeedSetup { reroll: true }
    );
    assert_eq!(c.place(&mut f, setup()), Ok(Event::Placed));
    f.busy = true;
    assert_eq!(c.tick(&mut f, false, Some(false)), Err(()));
    assert_eq!(c.session.phase, Phase::Delay(2));
    assert_eq!(c.session.summary.attempts, 0);
    f.busy = false;
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::None));
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::Launched));
    f.attempt.record_contact(ContactReceipt {
        surface: Surface::Wall,
        requested_status: 999,
        passive_enabled: true,
        position: Vec2::default(),
    });
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::None));
    assert!(c.awaiting_outcome());
    assert_eq!(
        c.session.summary.attempts, 0,
        "contact alone is not a timing grade"
    );
    assert_eq!(
        c.outcome(Outcome {
            attempt: 1,
            decision: decision()
        }),
        Event::Graded
    );
    assert_eq!(c.direction, Some(Direction::WallJump));
    assert_eq!(c.session.summary.attempts, 1);
    assert_eq!(
        c.outcome(Outcome {
            attempt: 1,
            decision: decision()
        }),
        Event::None
    );
    assert_eq!(c.session.summary.attempts, 1);
}
#[test]
fn adapter_failures_do_not_reroll_or_duplicate_launch() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    f.reject_launch = true;
    assert_eq!(c.tick(&mut f, false, Some(false)), Err(()));
    f.reject_launch = false;
    assert_eq!(c.tick(&mut f, true, Some(false)), Ok(Event::None));
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::Launched));
    c.tick(&mut f, false, Some(false)).unwrap();
    assert_eq!((f.placements, f.launches), (1, 1));
    assert_eq!(c.sampled(), Some(setup()));
    assert_eq!(
        c.outcome(Outcome {
            attempt: 999,
            decision: decision()
        }),
        Event::None
    );
    c.stop(&mut f);
    assert_eq!(f.restores, 1);
}
#[test]
fn receipt_driven_respawn_rerolls_only_counted_attempts() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    assert_eq!(c.abandon(), Event::NeedSetup { reroll: false });
    assert_eq!(c.session.summary.attempts, 0);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    c.outcome(Outcome {
        attempt: 2,
        decision: decision(),
    });
    assert_eq!(c.tick(&mut f, false, Some(true)), Ok(Event::None));
    assert_eq!(
        c.session.phase,
        Phase::Hold(0),
        "the decision frame is not late frame one"
    );
    for _ in 0..44 {
        assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::None));
    }
    assert_eq!(
        c.tick(&mut f, false, Some(false)),
        Ok(Event::NeedSetup { reroll: true })
    );
}
struct Lines(Vec<Line>);
impl Source for Lines {
    fn read(&mut self, id: usize) -> Option<Line> {
        self.0.iter().copied().find(|l| l.id == id)
    }
    fn next(&mut self, _: Line, _: usize) -> Option<(usize, usize)> {
        None
    }
}
fn ceiling(id: usize, y: f32) -> Line {
    Line {
        id,
        ends: [Vec2 { x: -30., y }, Vec2 { x: 30., y }],
        normal: Vec2 { x: 0., y: -1. },
        floor: false,
        wall: false,
        passable: false,
    }
}
#[test]
fn point_picker_reaches_underside_and_remembers_stage() {
    use crate::model::{
        btt::Pad,
        chords::logical::{A, X},
    };
    let mut surfaces = Surfaces::EMPTY;
    let line = ceiling(1, -40.);
    surfaces.push(line);
    let mut picker = picker::Picker::EMPTY;
    picker.enter(3, Vec2 { x: 0., y: -39. });
    let pad = |pressed| Pad {
        stick: [0.; 2],
        held: 0,
        pressed,
        save: false,
        load: false,
    };
    assert_eq!(
        picker.feed(pad(1 << A), &surfaces, [-100., 100., 100., -100.], None),
        picker::Pick::Saved
    );
    assert_eq!(picker.selected.unwrap().point.y, -40.);
    assert_eq!(
        picker.feed(pad(1 << X), &surfaces, [-100., 100., 100., -100.], None),
        picker::Pick::Start
    );
    let arrow = picker.normal_arrow().unwrap();
    assert_eq!(arrow[1].y, -45.);
    picker.enter(4, Vec2::default());
    assert!(picker.selected.is_none());
    assert!(picker.hovered.is_none());
    picker.enter(3, Vec2::default());
    assert!(
        picker.selected.is_none(),
        "discovery must rebind the saved point"
    );
    assert_eq!(picker.cursor, Vec2 { x: 0., y: -40. });
    let mut reloaded = Surfaces::EMPTY;
    reloaded.push(ceiling(101, -40.));
    assert!(picker.restore(&reloaded));
    assert_eq!(picker.selected.unwrap().point.y, -40.);
    assert_eq!(picker.selected.unwrap().line.id, 101);
    assert_ne!(picker.selected.unwrap().line.id, line.id);
}
#[test]
fn random_spot_refreshes_lines_and_clips_camera_before_sampling() {
    let mut surfaces = Surfaces::EMPTY;
    let still = ceiling(1, -40.);
    let moved = ceiling(2, -20.);
    surfaces.push(still);
    surfaces.push(moved);
    let mut source = Lines(vec![still, ceiling(2, -21.)]);
    let spot = picker::random_spot(
        &surfaces,
        &mut source,
        Some(Kind::Ceiling),
        [-10., 10., 0., -100.],
        &mut |_| 0,
    )
    .unwrap();
    assert_eq!(spot.line.id, 1);
    assert!(spot.point.x >= -10. && spot.point.x <= 10.);
    assert!(spot.current(&mut source));
    source.0[0].ends[0].x += 0.001;
    assert!(!spot.current(&mut source));
    assert!(picker::random_spot(
        &surfaces,
        &mut source,
        None,
        [-100., 100., 0., -100.],
        &mut |_| 0
    )
    .is_none());
}
#[test]
fn all_fixed_attempt_values_stay_and_random_rerolls_only_on_request() {
    use crate::model::{
        settings::Settings,
        tech_settings::{self, Mode, Numeric},
    };
    let snapshot = Settings::factory().menu;
    let mut config = tech_settings::read(&snapshot);
    let set = |lo| Numeric {
        mode: Mode::Set,
        lo,
        hi: lo,
    };
    config.speed = set(100.);
    config.damage = set(40.);
    config.hit_damage = set(12.);
    config.angle = set(270.);
    config.start = set(30.);
    config.time = set(20.);
    config.delay = set(30.);
    config.facing = 2;
    config.di = 0;
    let line = Line {
        id: 9,
        ends: [Vec2 { x: -50., y: 0. }, Vec2 { x: 50., y: 0. }],
        normal: Vec2 { x: 0., y: 1. },
        floor: true,
        wall: false,
        passable: false,
    };
    let mut surfaces = Surfaces::EMPTY;
    surfaces.push(line);
    let bounds = solver::Box2 {
        min: Vec2 { x: -200., y: -200. },
        max: Vec2 { x: 200., y: 200. },
    };
    let context = || planner::Context {
        surfaces: &surfaces,
        spot: picker::Spot {
            line,
            point: Vec2::default(),
        },
        params: crate::model::tech_launch::FlightParams {
            brake: 0.051,
            gravity: 0.087,
            fall_speed_cap: 1.5,
        },
        blast: bounds,
        view: None,
    };
    let a = planner::plan(config, context(), &mut |_| {
        panic!("fixed values must not draw randomness")
    })
    .unwrap();
    let b = planner::plan(config, context(), &mut |_| {
        panic!("fixed values must not draw randomness")
    })
    .unwrap();
    assert_eq!(a.setup, b.setup);
    assert_eq!(a.setup.launch.start, Vec2 { x: 0., y: 30. });
    assert_eq!(a.setup.launch.percent, 40.);
    assert_eq!(a.setup.launch.hit_damage, 12.);
    assert_eq!(a.setup.launch.di, Some(Vec2::default()));
    assert!(!a.approximate);
    config.speed = Numeric {
        mode: Mode::Random,
        lo: 80.,
        hi: 250.,
    };
    let low = planner::plan(config, context(), &mut |_| 0).unwrap();
    let high = planner::plan(config, context(), &mut |n| n - 1).unwrap();
    assert_eq!(
        (low.setup.launch.knockback, high.setup.launch.knockback),
        (80., 250.)
    );
    assert_eq!(low.setup.launch.percent, high.setup.launch.percent);
    assert_eq!(low.setup.launch.start, high.setup.launch.start);
}
#[test]
fn native_abandonment_while_held_retries_same_sample_uncounted() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    f.attempt.mark_abandoned();
    assert_eq!(
        c.tick(&mut f, false, Some(false)),
        Ok(Event::NeedSetup { reroll: false })
    );
    assert_eq!(c.session.summary.attempts, 0);
    assert_eq!(c.session.phase, Phase::Place);
    assert_eq!(c.sampled(), Some(setup()));
}

#[test]
fn session_progress_and_per_fighter_bests_survive_new_runs() {
    let mut session = session::Session::IDLE;
    session.start(Length::Frames(3600));
    session.elapsed = 61;
    let mut text = String::new();
    session.write_progress(&mut text).unwrap();
    assert_eq!(text, "0:59 left · streak 0");
    let mut bests = session::Bests::EMPTY;
    assert_eq!(bests.record(0, 7), 7);
    assert_eq!(bests.record(1, 3), 3);
    assert_eq!(bests.record(0, 2), 7);
    assert_eq!(bests.record(0, 8), 8);
    assert_eq!(Direction::Ceiling.label(), "Ceiling tech");
}
#[test]
fn placement_query_busy_then_abandoned_retries_without_counting_or_launching() {
    let mut controller = Controller::IDLE;
    let mut launcher = Fake::new();
    controller.start(Length::Free);
    launcher.busy = true;
    assert_eq!(controller.place(&mut launcher, setup()), Ok(Event::Placed));
    assert_eq!(launcher.placements, 1);
    launcher.attempt.mark_abandoned();
    launcher.busy = false;
    assert_eq!(
        controller.tick(&mut launcher, false, Some(false)),
        Ok(Event::NeedSetup { reroll: false })
    );
    assert_eq!(controller.session.phase, Phase::Place);
    assert_eq!(controller.session.summary.attempts, 0);
    assert_eq!(launcher.launches, 0);
    assert_eq!(controller.sampled(), Some(setup()));
}

#[test]
fn abandoned_attempt_cancels_pending_launch_retry() {
    let mut controller = Controller::IDLE;
    let mut launcher = Fake::new();
    controller.start(Length::Free);
    controller.place(&mut launcher, setup()).unwrap();
    controller.tick(&mut launcher, false, Some(false)).unwrap();
    launcher.reject_launch = true;
    assert_eq!(controller.tick(&mut launcher, false, Some(false)), Err(()));
    launcher.reject_launch = false;
    launcher.attempt.mark_abandoned();
    assert_eq!(
        controller.tick(&mut launcher, false, Some(false)),
        Ok(Event::NeedSetup { reroll: false })
    );
    assert_eq!(
        launcher.launches, 0,
        "a stale pending launch cannot wake an abandoned attempt"
    );
    assert_eq!(controller.session.summary.attempts, 0);
    controller.place(&mut launcher, setup()).unwrap();
    controller.tick(&mut launcher, false, Some(false)).unwrap();
    assert_eq!(
        controller.tick(&mut launcher, false, Some(false)),
        Ok(Event::Launched)
    );
    assert_eq!(launcher.launches, 1);
}

#[test]
fn stage_point_cache_returns_the_latest_choice() {
    let mut picker = picker::Picker::EMPTY;
    let first = picker::Spot {
        line: ceiling(1, -40.),
        point: Vec2 { x: 0., y: -40. },
    };
    let second = picker::Spot {
        line: ceiling(2, -20.),
        point: Vec2 { x: 10., y: -20. },
    };
    picker.enter(3, Vec2::default());
    picker.selected = Some(first);
    picker.enter(4, Vec2::default());
    let other = picker::Spot {
        line: ceiling(9, -60.),
        point: Vec2 { x: -5., y: -60. },
    };
    picker.selected = Some(other);
    picker.enter(3, Vec2::default());
    assert_eq!(picker.cursor, first.point);
    let mut first_return = Surfaces::EMPTY;
    first_return.push(ceiling(101, -40.));
    assert!(picker.restore(&first_return));
    picker.selected = Some(second);
    picker.enter(4, Vec2::default());
    assert_eq!(
        picker.cursor, other.point,
        "each stage keeps its own latest point"
    );
    assert!(picker.selected.is_none());
    picker.enter(3, Vec2::default());
    assert_eq!(picker.cursor, second.point);
    assert!(picker.selected.is_none());
    let mut second_return = Surfaces::EMPTY;
    second_return.push(ceiling(201, -40.));
    second_return.push(ceiling(202, -20.));
    assert!(picker.restore(&second_return));
    let restored = picker.selected.unwrap();
    assert_eq!(restored.point, second.point);
    assert_eq!(restored.line.id, 202);
    assert_ne!(restored.line.id, second.line.id);
}

#[test]
fn fractional_native_window_is_not_truncated() {
    use session::{grade, Grade};
    let mut native = decision();
    native.tech = None;
    native.window = 12.5;
    native.count = 12;
    native.previous = 27;
    assert_eq!(
        grade(native),
        Some(Grade::Locked(27)),
        "12 is inside the native 12.5-frame comparison"
    );
    native.previous = 28;
    assert_eq!(grade(native), None);
    native.count = 13;
    assert_eq!(grade(native), Some(Grade::Early(1)));
    native.count = 40;
    assert_eq!(grade(native), Some(Grade::Early(28)));
    native.count = 41;
    assert_eq!(
        grade(native),
        None,
        "40.5 is the untouched window plus lockout bound"
    );
}

#[test]
fn unavailable_press_calibration_never_becomes_zero_or_an_average_sample() {
    use session::{grade, Grade, Summary};
    let mut native = decision();
    native.press_frame_offset = None;
    assert_eq!(grade(native), Some(Grade::Perfect(None)));
    let mut summary = Summary::default();
    summary.record(grade(native).unwrap());
    assert_eq!(
        summary.attempts, 1,
        "accepted native tech is still a success"
    );
    assert_eq!(summary.streak, 1);
    assert_eq!(summary.average(), None);
    summary.record(Grade::Perfect(Some(4)));
    assert_eq!(summary.average(), Some(4.));
    let mut label = String::new();
    Grade::Perfect(None).write(&mut label).unwrap();
    assert_eq!(label, "Perfect · timing unavailable");
}

#[test]
fn same_stage_rediscovery_rebinds_once_and_preserves_moved_line_refusal() {
    let mut picker = picker::Picker::EMPTY;
    let old = picker::Spot {
        line: ceiling(7, -40.),
        point: Vec2 { x: 10., y: -40. },
    };
    picker.enter(3, Vec2::default());
    picker.selected = Some(old);
    picker.hovered = Some(old);

    // A new native scene may use the same stage id and completely new line ids.
    picker.enter(3, Vec2 { x: 90., y: 90. });
    assert_eq!(picker.cursor, old.point);
    assert!(picker.selected.is_none());
    assert!(picker.hovered.is_none());
    let fresh = ceiling(8, -40.);
    let mut surfaces = Surfaces::EMPTY;
    surfaces.push(fresh);
    assert!(picker.restore(&surfaces));
    let restored = picker.selected.unwrap();
    assert_eq!(restored.line, fresh);
    assert_eq!(restored.point, old.point);
    let mut source = Lines(vec![fresh]);
    assert!(restored.current(&mut source));

    source.0[0].ends[0].x += 0.001;
    assert!(!restored.current(&mut source));
    let mut moved_surfaces = Surfaces::EMPTY;
    moved_surfaces.push(source.0[0]);
    assert!(!picker.restore(&moved_surfaces), "restoration is one shot");
    assert_eq!(
        picker.selected,
        Some(restored),
        "never silently rebind a moved line"
    );
    assert!(!picker.selected.unwrap().current(&mut source));
}

#[test]
fn restoring_without_a_saved_point_does_not_choose_a_surface() {
    let mut picker = picker::Picker::EMPTY;
    let here = Vec2 { x: 5., y: -39. };
    picker.enter(3, here);
    let mut surfaces = Surfaces::EMPTY;
    surfaces.push(ceiling(1, -40.));
    assert!(!picker.restore(&surfaces));
    assert_eq!(picker.cursor, here);
    assert!(picker.selected.is_none());
}

fn snapshot(attempt: u64) -> crate::model::tech_launch::DecisionSnapshot {
    crate::model::tech_launch::DecisionSnapshot {
        attempt,
        contact: ContactReceipt {
            surface: Surface::Wall,
            requested_status: 999,
            passive_enabled: true,
            position: Vec2::default(),
        },
        source_status: Some(123),
        entered_status: None,
        guard_count: Some(254),
        previous_guard_gap: Some(254),
        guard_event: None,
        window_base: Some(12),
        window_multiplier: Some(1.0),
        lockout: Some(28),
        reaction: Some(20.),
        damage_fall: Some(false),
        stick_x: None,
        press_frame_offset: None,
    }
}

#[test]
fn snapshot_requires_acceptance_and_rejects_stale_duplicate_delivery() {
    use session::{Grade, NativeOutcome};
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, None).unwrap();
    c.tick(&mut f, false, None).unwrap();
    let mut s = snapshot(f.attempt.id);
    assert_eq!(c.snapshot(s, None), Event::None);
    assert_eq!(c.session.phase, Phase::Flight);
    s.attempt += 1;
    assert_eq!(
        c.snapshot(s, Some(NativeOutcome::Tech(Some(Direction::Wall)))),
        Event::None
    );
    s.attempt -= 1;
    assert_eq!(
        c.snapshot(s, Some(NativeOutcome::Tech(Some(Direction::Wall)))),
        Event::Graded
    );
    assert_eq!(c.session.result, Some(Grade::Perfect(None)));
    assert_eq!(c.direction, Some(Direction::Wall));
    assert_eq!(c.session.summary.attempts, 1);
    assert_eq!(c.session.summary.average(), None);
    assert_eq!(c.snapshot(s, Some(NativeOutcome::Miss)), Event::None);
    assert_eq!(c.session.summary.attempts, 1);
}

#[test]
fn snapshot_missing_inputs_stay_unavailable_without_substitutions() {
    use session::{map_snapshot, Grade, NativeOutcome};
    let s = snapshot(1);
    let mut incomplete = Vec::new();
    let mut v = s;
    v.guard_count = None;
    incomplete.push(v);
    let mut v = s;
    v.previous_guard_gap = None;
    incomplete.push(v);
    let mut v = s;
    v.window_base = None;
    incomplete.push(v);
    let mut v = s;
    v.window_multiplier = None;
    incomplete.push(v);
    let mut v = s;
    v.lockout = None;
    incomplete.push(v);
    let mut v = s;
    v.reaction = None;
    incomplete.push(v);
    let mut v = s;
    v.damage_fall = None;
    incomplete.push(v);
    let mut v = s;
    v.window_multiplier = Some(f32::NAN);
    incomplete.push(v);
    let mut v = s;
    v.reaction = Some(f32::NAN);
    incomplete.push(v);
    let mut v = s;
    v.lockout = Some(-1);
    incomplete.push(v);
    for v in incomplete {
        assert_eq!(
            map_snapshot(v, NativeOutcome::Miss),
            Some(Grade::Unavailable)
        );
    }
    assert_eq!(map_snapshot(s, NativeOutcome::Miss), None);
    let mut held = s;
    held.damage_fall = Some(true);
    held.reaction = None;
    assert_eq!(
        map_snapshot(held, NativeOutcome::Tech(Some(Direction::Wall))),
        Some(Grade::Held)
    );
    assert_eq!(
        map_snapshot(held, NativeOutcome::Miss),
        Some(Grade::AfterHitstun)
    );
    held.contact.passive_enabled = false;
    assert_eq!(
        map_snapshot(held, NativeOutcome::Miss),
        Some(Grade::Untechable)
    );
}

#[test]
fn snapshot_fractional_window_and_calibration_are_preserved() {
    use session::{map_snapshot, Grade, NativeOutcome};
    let mut s = snapshot(1);
    s.window_multiplier = Some(1.0416666);
    s.guard_count = Some(12);
    s.previous_guard_gap = Some(27);
    assert_eq!(
        map_snapshot(s, NativeOutcome::Miss),
        Some(Grade::Locked(27))
    );
    s.guard_count = Some(13);
    assert_eq!(map_snapshot(s, NativeOutcome::Miss), Some(Grade::Early(1)));
    s.guard_count = Some(4);
    assert_eq!(
        map_snapshot(s, NativeOutcome::Tech(Some(Direction::Wall))),
        Some(Grade::Perfect(None))
    );
    s.press_frame_offset = Some(1);
    assert_eq!(
        map_snapshot(s, NativeOutcome::Tech(Some(Direction::Wall))),
        Some(Grade::Perfect(Some(5)))
    );
    s.guard_count = None;
    assert_eq!(
        map_snapshot(s, NativeOutcome::Tech(Some(Direction::Wall))),
        Some(Grade::Perfect(None))
    );
    assert_eq!(
        map_snapshot(s, NativeOutcome::Tech(None)),
        Some(Grade::Perfect(None)),
        "unavailable roll direction cannot erase a proven success"
    );
}

#[test]
fn unavailable_guard_window_never_fabricates_late_or_no_press() {
    use session::{Action, Grade, Session};
    for gap in [1, 15, 30] {
        for press in [None, Some(30)] {
            let mut s = Session::IDLE;
            s.start(Length::Attempts(10));
            s.placed(1);
            s.tick(false, None);
            s.contact_grade(None);
            for age in 1..=30 {
                let event = if age == gap {
                    None
                } else {
                    Some(press == Some(age))
                };
                s.tick(false, event);
            }
            assert_eq!(s.result, Some(Grade::Unavailable));
            assert_eq!(s.summary.attempts, 0);
            for _ in 31..45 {
                assert_eq!(s.tick(false, Some(false)), Action::None);
            }
            assert_eq!(s.tick(false, Some(false)), Action::Place { reroll: false });
        }
    }
}

#[test]
fn unavailable_contact_holds_exactly_45_frames_and_reuses_sample() {
    use session::{Grade, NativeOutcome};
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Attempts(10));
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, None).unwrap();
    c.tick(&mut f, false, None).unwrap();
    let mut s = snapshot(f.attempt.id);
    s.guard_count = None;
    assert_eq!(c.snapshot(s, Some(NativeOutcome::Miss)), Event::Graded);
    assert_eq!(c.session.result, Some(Grade::Unavailable));
    c.tick(&mut f, false, None).unwrap();
    assert_eq!(
        c.session.phase,
        Phase::Hold(0),
        "decision frame does not consume hold"
    );
    for _ in 0..5 {
        c.tick(&mut f, true, None).unwrap();
    }
    for _ in 1..45 {
        assert_eq!(c.tick(&mut f, false, None), Ok(Event::None));
    }
    assert_eq!(
        c.tick(&mut f, false, None),
        Ok(Event::NeedSetup { reroll: false })
    );
    assert_eq!(c.session.summary.attempts, 0);
    assert_eq!(c.sampled(), Some(setup()));
    assert_eq!(f.restores, 0);
}

#[test]
fn paused_unavailable_guard_does_not_poison_proven_window() {
    use session::{Grade, Session};
    let mut s = Session::IDLE;
    s.start(Length::Free);
    s.placed(1);
    s.tick(false, None);
    s.contact_grade(None);
    for _ in 0..40 {
        s.tick(true, None);
    }
    for _ in 1..30 {
        s.tick(false, Some(false));
    }
    s.tick(false, Some(true));
    assert_eq!(s.result, Some(Grade::Late(30)));
    assert_eq!(s.summary.attempts, 1);
}

#[test]
fn unproven_acceptance_consumes_only_exact_snapshot_as_unavailable() {
    use session::{Grade, NativeOutcome};
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, None).unwrap();
    c.tick(&mut f, false, None).unwrap();
    let mut s = snapshot(f.attempt.id + 1);
    assert_eq!(c.snapshot_unavailable(s, 45), Event::None);
    assert_eq!(c.session.phase, Phase::Flight);
    s.attempt -= 1;
    assert_eq!(c.snapshot(s, None), Event::None);
    assert_eq!(c.snapshot_unavailable(s, 45), Event::Graded);
    assert_eq!(c.session.result, Some(Grade::Unavailable));
    assert_eq!(c.session.summary.attempts, 0);
    assert_eq!(c.direction, None);
    assert_eq!(c.snapshot_unavailable(s, 45), Event::None);
    assert_eq!(c.snapshot(s, Some(NativeOutcome::Miss)), Event::None);
    for _ in 0..10 {
        assert_eq!(c.tick(&mut f, true, None), Ok(Event::None));
    }
    assert_eq!(c.session.phase, Phase::Hold(44));
    assert_eq!(
        c.tick(&mut f, false, None),
        Ok(Event::NeedSetup { reroll: false })
    );
}

#[test]
fn delayed_accepted_snapshot_keeps_original_contact_respawn_boundary() {
    use session::NativeOutcome;
    for age in [1, 17, 45] {
        let mut c = Controller::IDLE;
        let mut f = Fake::new();
        c.start(Length::Free);
        c.place(&mut f, setup()).unwrap();
        c.tick(&mut f, false, None).unwrap();
        c.tick(&mut f, false, None).unwrap();
        let s = snapshot(f.attempt.id);
        for _ in 1..age {
            c.tick(&mut f, false, None).unwrap();
        }
        assert_eq!(
            c.snapshot_at(s, Some(session::NativeOutcome::Tech(None)), age),
            Event::Graded
        );
        let now = c.session.elapsed;
        assert_eq!(c.tick(&mut f, true, None), Ok(Event::None));
        assert_eq!(c.session.elapsed, now);
        for _ in age..45 {
            assert_eq!(c.tick(&mut f, false, None), Ok(Event::None));
        }
        assert_eq!(
            c.tick(&mut f, false, None),
            Ok(Event::NeedSetup { reroll: true })
        );
        assert_eq!(c.session.summary.attempts, 1);
        assert_eq!(
            c.snapshot_at(s, Some(NativeOutcome::Miss), age),
            Event::None
        );
    }
}

#[test]
fn delayed_miss_delivery_does_not_invent_past_guard_observations() {
    use session::{Grade, NativeOutcome};
    for age in [2, 30, 31, 45] {
        let mut c = Controller::IDLE;
        let mut f = Fake::new();
        c.start(Length::Free);
        c.place(&mut f, setup()).unwrap();
        c.tick(&mut f, false, None).unwrap();
        c.tick(&mut f, false, None).unwrap();
        let s = snapshot(f.attempt.id);
        assert_eq!(
            c.snapshot_at(s, Some(NativeOutcome::Miss), age),
            Event::Graded
        );
        c.tick(&mut f, false, Some(true)).unwrap();
        assert_eq!(c.session.result, Some(Grade::Unavailable));
        assert_eq!(c.session.summary.attempts, 0);
    }
}

#[test]
fn native_status_acceptance_requires_retained_actual_transition() {
    use session::{accepted_status, NativeOutcome};
    let ids = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110];
    for (index, expected) in [
        NativeOutcome::Tech(Some(Direction::InPlace)),
        NativeOutcome::Tech(None),
        NativeOutcome::Miss,
        NativeOutcome::Miss,
        NativeOutcome::Tech(Some(Direction::Ceiling)),
        NativeOutcome::Miss,
        NativeOutcome::Miss,
        NativeOutcome::Tech(Some(Direction::Wall)),
        NativeOutcome::Tech(Some(Direction::WallJump)),
        NativeOutcome::Miss,
        NativeOutcome::Miss,
    ]
    .into_iter()
    .enumerate()
    {
        let mut s = snapshot(1);
        s.source_status = Some(90);
        s.contact.requested_status = ids[index];
        assert_eq!(
            accepted_status(s, ids),
            None,
            "request alone is not acceptance"
        );
        s.entered_status = Some(999);
        assert_eq!(
            accepted_status(s, ids),
            None,
            "unrecognized actual status stays unavailable"
        );
        s.entered_status = Some(ids[index]);
        assert_eq!(accepted_status(s, ids), Some(expected));
        s.contact.requested_status = 998;
        assert_eq!(
            accepted_status(s, ids),
            Some(expected),
            "actual status owns the outcome even when the request differs"
        );
        s.source_status = Some(ids[index]);
        assert_eq!(
            accepted_status(s, ids),
            None,
            "unchanged source is not a transition"
        );
        s.source_status = None;
        assert_eq!(
            accepted_status(s, ids),
            None,
            "missing source is unavailable"
        );
    }
}

#[test]
fn snapshot_admission_rejects_core_ack_retry_then_allows_next_attempt() {
    use session::NativeOutcome;
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    let first = f.attempt.id;
    assert!(
        !c.accepts_snapshot(first),
        "held/delayed receipt is not a decision"
    );
    c.tick(&mut f, false, None).unwrap();
    c.tick(&mut f, false, None).unwrap();
    assert!(c.accepts_snapshot(first));
    assert!(!c.accepts_snapshot(first + 1));
    assert_eq!(
        c.snapshot(snapshot(first), Some(NativeOutcome::Tech(None))),
        Event::Graded
    );
    // The core can retry its already-posted bytes when its later ack lock was Busy.
    assert!(
        !c.accepts_snapshot(first),
        "ack retry must not occupy runtime pending inbox"
    );
    c.tick(&mut f, false, None).unwrap();
    for _ in 1..45 {
        c.tick(&mut f, false, None).unwrap();
    }
    assert_eq!(
        c.tick(&mut f, false, None),
        Ok(Event::NeedSetup { reroll: true })
    );
    assert!(!c.accepts_snapshot(first));
    c.place(&mut f, setup()).unwrap();
    let next = f.attempt.id;
    assert_ne!(next, first);
    assert!(!c.accepts_snapshot(first));
    assert!(!c.accepts_snapshot(next));
    c.tick(&mut f, false, None).unwrap();
    c.tick(&mut f, false, None).unwrap();
    assert!(!c.accepts_snapshot(first));
    assert!(
        c.accepts_snapshot(next),
        "retry rejection must not starve the next original delivery"
    );
    assert_eq!(
        c.snapshot(snapshot(next), Some(NativeOutcome::Tech(None))),
        Event::Graded
    );
    assert_eq!(c.session.summary.attempts, 2);
}

#[test]
fn abandonment_without_a_decision_never_invents_no_contact() {
    use super::session::{Grade, Length};
    let mut c = super::controller::Controller::IDLE;
    let mut launcher = Fake::new();
    c.start(Length::Free);
    c.place(&mut launcher, setup()).unwrap();
    while c.session.phase != super::session::Phase::Flight {
        c.tick(&mut launcher, false, None).unwrap();
    }
    assert_eq!(
        c.abandon(),
        super::controller::Event::NeedSetup { reroll: false }
    );
    assert_eq!(c.session.result, Some(Grade::Unavailable));
    assert_eq!(c.session.summary.attempts, 0);
}

#[test]
fn retained_delivery_grades_actual_tech_roll_and_miss() {
    use crate::model::tech_launch::DecisionDelivery;
    let ids = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110];
    for (actual, stick, success, direction) in [
        (100, 0., true, Some(Direction::InPlace)),
        (101, -1., true, None),
        (101, 1., true, None),
        (102, 0., false, None),
    ] {
        let mut c = Controller::IDLE;
        let mut f = Fake::new();
        c.start(Length::Attempts(10));
        c.place(&mut f, setup()).unwrap();
        while c.session.phase != Phase::Flight {
            c.tick(&mut f, false, None).unwrap();
        }
        let mut raw = snapshot(f.attempt.id);
        raw.source_status = Some(90);
        raw.contact.requested_status = 109; // actual outcome deliberately differs
        raw.stick_x = Some(stick);
        raw.guard_count = Some(13);
        let mut delivery = DecisionDelivery::EMPTY;
        assert!(delivery.capture_first(raw));
        assert!(delivery.confirmed_decision().is_none());
        assert!(delivery.observe_entered_status(raw.attempt, Some(actual), &ids));
        let delivered = delivery.confirmed_decision().unwrap();
        assert_eq!(delivered.contact, raw.contact);
        assert_eq!(delivered.stick_x.unwrap().to_bits(), stick.to_bits());
        assert_eq!(c.retained_snapshot(delivered, ids, 0), Event::Graded);
        assert_eq!(c.session.summary.attempts, 1);
        assert_eq!(
            c.session.summary.perfect + c.session.summary.held,
            u32::from(success)
        );
        assert_eq!(c.direction, direction);
        assert_eq!(
            c.session.result,
            Some(if success {
                Grade::Perfect(None)
            } else {
                Grade::Early(2)
            })
        );
    }
}

#[test]
fn retained_unconfirmed_query_expires_uncounted_at_45_and_continues() {
    use crate::model::tech_launch::DecisionDelivery;
    let ids = [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110];
    // An ungranted read, an unknown source, an unchanged source, and an
    // unrelated status all remain queryable without becoming a false miss.
    for (source, actual) in [
        (Some(90), None),
        (None, Some(100)),
        (Some(90), Some(90)),
        (Some(90), Some(999)),
    ] {
        let mut c = Controller::IDLE;
        let mut f = Fake::new();
        let fixed = setup();
        c.start(Length::Attempts(10));
        c.place(&mut f, fixed).unwrap();
        while c.session.phase != Phase::Flight {
            c.tick(&mut f, false, None).unwrap();
        }
        let mut raw = snapshot(f.attempt.id);
        raw.source_status = source;
        let mut delivery = DecisionDelivery::EMPTY;
        delivery.capture_first(raw);
        assert!(!delivery.observe_entered_status(raw.attempt, actual, &ids));
        assert!(delivery.confirmed_decision().is_none());
        for age in 0..=session::HOLD {
            let queried = delivery
                .pending_decision()
                .expect("unconfirmed snapshot is retained");
            assert_eq!(queried.entered_status, None);
            let event = c.retained_snapshot(queried, ids, age);
            assert_eq!(
                event,
                if age == session::HOLD {
                    Event::Graded
                } else {
                    Event::None
                }
            );
            let tick = c.tick(&mut f, false, None).unwrap();
            assert_eq!(
                tick,
                if age == session::HOLD {
                    Event::NeedSetup { reroll: false }
                } else {
                    Event::None
                }
            );
        }
        assert_eq!(c.session.result, Some(Grade::Unavailable));
        assert_eq!(c.session.summary.attempts, 0);
        assert_eq!(c.sampled(), Some(fixed));
        c.place(&mut f, fixed).unwrap();
        assert_ne!(f.attempt.id, raw.attempt);
    }
}

#[test]
fn retained_confirmation_preserves_first_decision_and_contact_age() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    while c.session.phase != Phase::Flight {
        c.tick(&mut f, false, None).unwrap();
    }
    let original = snapshot(f.attempt.id);
    let mut pending = None;
    assert!(c.retain_snapshot(&mut pending, original));
    pending.as_mut().unwrap().1 = 17;
    let mut confirmed = original;
    confirmed.entered_status = Some(100);
    confirmed.guard_count = Some(1); // cannot resample the decision-frame counters
    assert!(c.retain_snapshot(&mut pending, confirmed));
    let (saved, age) = pending.unwrap();
    assert_eq!(age, 17);
    assert_eq!(saved.guard_count, original.guard_count);
    assert_eq!(saved.entered_status, Some(100));
    confirmed.entered_status = Some(101);
    c.retain_snapshot(&mut pending, confirmed);
    assert_eq!(pending.unwrap().0.entered_status, Some(100));
}
