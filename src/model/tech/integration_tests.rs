use super::*;
use crate::model::tech_launch::Surface;
use controller::{Controller, Event, Outcome, Setup};
use session::{Decision, Direction, Length, Phase};
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
    assert_eq!(c.tick(&mut f, false, false), Err(()));
    assert_eq!(c.session.phase, Phase::Delay(2));
    assert_eq!(c.session.summary.attempts, 0);
    f.busy = false;
    assert_eq!(c.tick(&mut f, false, false), Ok(Event::None));
    assert_eq!(c.tick(&mut f, false, false), Ok(Event::Launched));
    f.attempt.record_contact(ContactReceipt {
        surface: Surface::Wall,
        requested_status: 999,
        passive_enabled: true,
        position: Vec2::default(),
    });
    assert_eq!(c.tick(&mut f, false, false), Ok(Event::None));
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
    c.tick(&mut f, false, false).unwrap();
    f.reject_launch = true;
    assert_eq!(c.tick(&mut f, false, false), Err(()));
    f.reject_launch = false;
    assert_eq!(c.tick(&mut f, true, false), Ok(Event::None));
    assert_eq!(c.tick(&mut f, false, false), Ok(Event::Launched));
    c.tick(&mut f, false, false).unwrap();
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
    c.tick(&mut f, false, false).unwrap();
    c.tick(&mut f, false, false).unwrap();
    assert_eq!(c.abandon(), Event::NeedSetup { reroll: false });
    assert_eq!(c.session.summary.attempts, 0);
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, false).unwrap();
    c.tick(&mut f, false, false).unwrap();
    c.outcome(Outcome {
        attempt: 2,
        decision: decision(),
    });
    assert_eq!(c.tick(&mut f, false, true), Ok(Event::None));
    assert_eq!(
        c.session.phase,
        Phase::Hold(0),
        "the decision frame is not late frame one"
    );
    for _ in 0..44 {
        assert_eq!(c.tick(&mut f, false, false), Ok(Event::None));
    }
    assert_eq!(
        c.tick(&mut f, false, false),
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
        c.tick(&mut f, false, false),
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
        controller.tick(&mut launcher, false, false),
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
    controller.tick(&mut launcher, false, false).unwrap();
    launcher.reject_launch = true;
    assert_eq!(controller.tick(&mut launcher, false, false), Err(()));
    launcher.reject_launch = false;
    launcher.attempt.mark_abandoned();
    assert_eq!(
        controller.tick(&mut launcher, false, false),
        Ok(Event::NeedSetup { reroll: false })
    );
    assert_eq!(
        launcher.launches, 0,
        "a stale pending launch cannot wake an abandoned attempt"
    );
    assert_eq!(controller.session.summary.attempts, 0);
    controller.place(&mut launcher, setup()).unwrap();
    controller.tick(&mut launcher, false, false).unwrap();
    assert_eq!(
        controller.tick(&mut launcher, false, false),
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
