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
        &mut |_, _| {},
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
        &mut |_| 0,
        &mut |_, _| {},
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
        damage_multiplier: Some(1.),
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
    assert_eq!(a.setup.launch.percent, 52.);
    assert_eq!(a.setup.launch.hit_damage, 12.);
    assert_eq!(a.setup.launch.di, Some(Vec2::default()));
    assert!(!a.approximate);
    let converted = planner::plan(
        config,
        planner::Context {
            damage_multiplier: Some(1.2),
            ..context()
        },
        &mut |_| panic!("fixed values must not draw randomness"),
    )
    .unwrap();
    assert_eq!(converted.sampled_percent, [40., 12.]);
    assert_eq!(converted.setup.launch.hit_damage, 10.);
    for damage_multiplier in [
        None,
        Some(0.),
        Some(-1.),
        Some(f32::NAN),
        Some(f32::INFINITY),
    ] {
        assert!(planner::plan(
            config,
            planner::Context {
                damage_multiplier,
                ..context()
            },
            &mut |_| panic!("unavailable multiplier must not sample an attempt")
        )
        .is_none());
    }
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
fn launch_percent_is_pre_hit_plus_native_forward_hit() {
    use crate::model::tech_settings::{Mode, Numeric};
    let speed = Numeric {
        mode: Mode::Set,
        lo: 100.,
        hi: 100.,
    };
    for (pre, visible, expected) in [
        (0., 15., 0x416fffff),
        (0., 27., 0x41d80001),
        (0., 30., 0x41efffff),
        (20., 16.38, 0x4211851e),
        (20., f32::from_bits(0x41830a3e), 0x4211851f),
    ] {
        let plan = percent_fixture(pre, visible, 1.2, speed, 0);
        assert_eq!(plan.setup.launch.percent.to_bits(), expected);
        assert_eq!(plan.sampled_percent, [pre, visible]);
    }
    // Recompute from retained original samples under a fresh multiplier;
    // neither a prior quotient nor a prior rounded post-percent is reused.
    let first = percent_fixture(20., 15., 1.2, speed, 0);
    let [pre, visible] = first.sampled_percent;
    let retry = percent_fixture(pre, visible, 1., speed, 0);
    assert_eq!(retry.setup.launch.hit_damage, 15.);
    assert_eq!(retry.setup.launch.percent, 35.);
    assert_eq!(
        crate::model::tech_hitlag::frames(
            planner::base_hit_damage(12., Some(1.2)).unwrap(),
            1.,
            hitlag_parameters(1., 1.2)
        ),
        Some(12)
    );
}
#[test]
fn knockback_range_stays_in_tumble() {
    use crate::model::tech_settings::{self, Mode, Numeric};
    let range = tech_settings::scale("tech_speed_range").unwrap();
    assert_eq!((range.value(0), range.value(range.max)), (80., 250.));
    // Checks the configured tumble-floor domain, independently of percent.
    // Actual native tumble/contact acceptance remains a separate runtime gate.
    for pre in (0..=150).step_by(5) {
        for index in 0..=range.max {
            for multiplier in [1., 1.2] {
                for mode in [Mode::Set, Mode::Random] {
                    let kb = range.value(index);
                    let speed = if mode == Mode::Set {
                        Numeric {
                            mode,
                            lo: kb,
                            hi: kb,
                        }
                    } else {
                        Numeric {
                            mode,
                            lo: range.value(0),
                            hi: range.value(range.max),
                        }
                    };
                    let plan =
                        percent_fixture(pre as f32, 12., multiplier, speed, u32::from(index));
                    assert_eq!(plan.setup.launch.knockback, kb);
                    assert!(plan.setup.launch.knockback >= 80.);
                }
            }
        }
    }
}
fn percent_fixture(
    pre: f32,
    visible: f32,
    multiplier: f32,
    speed: crate::model::tech_settings::Numeric,
    draw: u32,
) -> planner::Plan {
    use crate::model::{
        settings::Settings,
        tech_settings::{self, Mode, Numeric},
    };
    let set = |n| Numeric {
        mode: Mode::Set,
        lo: n,
        hi: n,
    };
    let mut config = tech_settings::read(&Settings::factory().menu);
    config.speed = speed;
    config.damage = set(pre);
    config.hit_damage = set(visible);
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
    planner::plan(
        config,
        planner::Context {
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
            blast: solver::Box2 {
                min: Vec2 { x: -200., y: -200. },
                max: Vec2 { x: 200., y: 200. },
            },
            view: None,
            damage_multiplier: Some(multiplier),
        },
        &mut |n| draw % n,
    )
    .unwrap()
}
#[test]
fn visible_hit_damage_uses_only_the_supplied_live_multiplier() {
    let base = planner::base_hit_damage(16.38, Some(1.2)).unwrap();
    // Decimal input and the observed native percent delta differ by one ULP.
    // Keep the requested f32 division, without fitting either result.
    assert_eq!(base.to_bits(), 0x415a6665);
    assert_eq!(format!("{base:.2}"), "13.65");
    assert_eq!(
        planner::base_hit_damage(f32::from_bits(0x41830a3e), Some(1.2))
            .unwrap()
            .to_bits(),
        0x415a6667
    );
    assert_eq!(planner::base_hit_damage(16.38, Some(1.)), Some(16.38));
    assert_eq!(planner::base_hit_damage(1., Some(1.2)), Some(1. / 1.2));
    assert_eq!(planner::base_hit_damage(30., Some(1.)), Some(30.));
    for multiplier in [
        None,
        Some(0.),
        Some(-1.),
        Some(f32::NAN),
        Some(f32::INFINITY),
        Some(f32::MIN_POSITIVE),
    ] {
        assert_eq!(planner::base_hit_damage(16.38, multiplier), None);
    }
    for visible in [0., 30.01, -1., f32::NAN, f32::INFINITY] {
        assert_eq!(planner::base_hit_damage(visible, Some(1.2)), None);
    }
    for visible in [16.38, f32::from_bits(0x41830a3e)] {
        let base = planner::base_hit_damage(visible, Some(1.2)).unwrap();
        // The product defines a generic attack factor of 1, not Marth's move.
        assert_eq!(
            crate::model::tech_hitlag::frames(base, 1., hitlag_parameters(1., 1.2)),
            Some(14)
        );
    }
}
fn hitlag_parameters(k: f32, percent_multiplier: f32) -> crate::model::tech_hitlag::Parameters {
    crate::model::tech_hitlag::Parameters {
        maximum: 30.,
        multiplier: 0.65,
        add: 6.,
        threshold: 10,
        object_multiplier: k,
        percent_multiplier,
    }
}
#[test]
fn inverse_roundtrip_can_cross_a_native_hitlag_floor() {
    use crate::model::tech_hitlag::frames;
    let p = hitlag_parameters(1., 1.2);
    // A hypothetical other hit is not an original base for a player-set p.
    let other_hit = f32::from_bits(0x415d89d9);
    let visible = other_hit * p.percent_multiplier;
    assert_eq!(visible.to_bits(), 0x4184ec4f);
    let inverse = planner::base_hit_damage(visible, Some(p.percent_multiplier)).unwrap();
    assert_eq!(inverse.to_bits(), 0x415d89d8);
    assert_eq!(frames(other_hit, 1., p), Some(15));
    assert_eq!(frames(inverse, 1., p), Some(14));
    for (bits, expected) in [(0x4184ec4e, 14), (0x4184ec4f, 14), (0x4184ec50, 15)] {
        let base =
            planner::base_hit_damage(f32::from_bits(bits), Some(p.percent_multiplier)).unwrap();
        assert_eq!(frames(base, 1., p), Some(expected));
    }
}
#[test]
fn canonical_visible_damage_is_deterministic_for_the_same_live_inputs() {
    use crate::model::tech_hitlag::frames;
    let mut visible: Vec<f32> = (10..=300).map(|tenths| tenths as f32 / 10.).collect();
    visible.extend([
        16.38,
        f32::from_bits(0x4184ec4e),
        f32::from_bits(0x4184ec4f),
        f32::from_bits(0x4184ec50),
    ]);
    // Determinism includes the live multiplier and native hitlag parameters;
    // changing either intentionally changes the hit rather than recovering one.
    for multiplier in [1., 1.2] {
        for k in [1., 0.925] {
            let parameters = hitlag_parameters(k, multiplier);
            for &p in &visible {
                let d = planner::base_hit_damage(p, Some(multiplier)).unwrap();
                let expected = frames(d, 1., parameters).unwrap();
                for _ in 0..8 {
                    let again = planner::base_hit_damage(p, Some(multiplier)).unwrap();
                    assert_eq!(again.to_bits(), d.to_bits());
                    assert_eq!(frames(again, 1., parameters), Some(expected));
                }
            }
        }
    }
}

#[test]
fn canonical_native_forward_preserves_tenth_percent_display() {
    let visible = (10..=300).map(|tenths| tenths as f32 / 10.).chain([
        16.38,
        f32::from_bits(0x4184ec4e),
        f32::from_bits(0x4184ec4f),
        f32::from_bits(0x4184ec50),
    ]);
    for p in visible {
        for multiplier in [1., 1.2] {
            let speed = crate::model::tech_settings::Numeric {
                mode: crate::model::tech_settings::Mode::Set,
                lo: 100.,
                hi: 100.,
            };
            let launch = percent_fixture(0., p, multiplier, speed, 0).setup.launch;
            let d = planner::base_hit_damage(p, Some(multiplier)).unwrap();
            assert_eq!(launch.hit_damage.to_bits(), d.to_bits());
            let native_added = launch.hit_damage * multiplier;
            assert_eq!(launch.percent.to_bits(), native_added.to_bits());
            // Display projection only; no rounding is added to product arithmetic.
            assert_eq!(
                format!("{native_added:.1}"),
                format!("{p:.1}"),
                "p={p:?}, multiplier={multiplier:?}, d={d:?}"
            );
        }
    }
    let d = planner::base_hit_damage(15., Some(1.2)).unwrap();
    assert_eq!((d * 1.2).to_bits(), 0x416fffff);
    assert_eq!(format!("{:.1}", d * 1.2), "15.0");
}

#[test]
fn native_reference_hitlag_uses_its_own_base_damage() {
    // R1 captured base damage, not the inverse of its displayed percent delta.
    let reference_d = f32::from_bits(0x415a6666);
    assert_eq!(
        crate::model::tech_hitlag::frames(reference_d, 0.7, hitlag_parameters(1., 1.2)),
        Some(10)
    );
    assert_ne!(
        planner::base_hit_damage(16.38, Some(1.2))
            .unwrap()
            .to_bits(),
        reference_d.to_bits()
    );
}

#[test]
fn pre_hit_twenty_plus_visible_hit_displays_thirty_six_point_three_eight() {
    use crate::model::tech_settings::{Mode, Numeric};
    let speed = Numeric {
        mode: Mode::Set,
        lo: 100.,
        hi: 100.,
    };
    let launch = percent_fixture(20., 16.38, 1.2, speed, 0).setup.launch;
    assert_eq!(launch.hit_damage.to_bits(), 0x415a6665);
    assert_eq!(launch.percent.to_bits(), 0x4211851e);
    assert_eq!(format!("{:.2}", launch.percent), "36.38");
    // Generic product AttackData is 1.0. Do not fit it to Marth reference hitlag.
    assert_eq!(
        crate::model::tech_hitlag::frames(launch.hit_damage, 1., hitlag_parameters(1., 1.2)),
        Some(14)
    );

    // R1 uses its OWN captured base and AttackData, never inverse(visible p).
    let reference_d = f32::from_bits(0x415a6666);
    let reference_post = 20.0f32 + reference_d * 1.2f32;
    assert_eq!(reference_post.to_bits(), 0x4211851f);
    assert_eq!(format!("{reference_post:.2}"), "36.38");
    assert_eq!(
        crate::model::tech_hitlag::frames(reference_d, 0.7, hitlag_parameters(1., 1.2)),
        Some(10)
    );
}

#[test]
fn integer_visible_ui_grid_preimages_keep_the_native_hitlag_floor() {
    use crate::model::tech_hitlag::frames;
    // Current visible slider emits integers only. This is a bounded domain
    // check, not a stability claim for arbitrary fractional input or parameters.
    for multiplier in [1., 1.2] {
        for k in [1., 0.925] {
            let p = hitlag_parameters(k, multiplier);
            for visible in 1..=30 {
                let visible = visible as f32;
                let base = planner::base_hit_damage(visible, Some(multiplier)).unwrap();
                for bits in base.to_bits() - 2..=base.to_bits() + 2 {
                    let candidate = f32::from_bits(bits);
                    if (candidate * multiplier).to_bits() == visible.to_bits() {
                        assert_eq!(
                            frames(candidate, 1., p),
                            frames(base, 1., p),
                            "visible={visible}, multiplier={multiplier}, k={k}"
                        );
                    }
                }
            }
        }
    }
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
fn known_miss_with_unavailable_guard_window_never_fabricates_a_timing_reason() {
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
            assert_eq!(s.result, Some(Grade::Missed));
            assert_eq!(s.summary.attempts, 1);
            for _ in 31..45 {
                assert_eq!(s.tick(false, Some(false)), Action::None);
            }
            assert_eq!(s.tick(false, Some(false)), Action::Place { reroll: true });
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
        assert_eq!(c.session.result, Some(Grade::Missed));
        assert_eq!(c.session.summary.attempts, 1);
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

// Bootstrap regression: this harness drives production Bootstrap/Rays/Surfaces.
// Only the stage's native ray/read/neighbour observations are faked.
struct BootstrapStage {
    lines: Vec<Line>,
    ray_calls: usize,
    hits: Vec<usize>,
}
impl BootstrapStage {
    fn fixture() -> Self {
        let p = |x, y| Vec2 { x, y };
        let line = |id, a, b, normal, floor, wall, passable| Line {
            id,
            ends: [a, b],
            normal,
            floor,
            wall,
            passable,
        };
        Self {
            lines: vec![
                line(100, p(-80., 0.), p(80., 0.), p(0., 1.), true, false, false),
                line(101, p(80., 0.), p(80., -30.), p(1., 0.), false, true, false),
                line(
                    102,
                    p(80., -30.),
                    p(-80., -30.),
                    p(0., -1.),
                    false,
                    false,
                    false,
                ),
                line(
                    103,
                    p(-80., -30.),
                    p(-80., 0.),
                    p(-1., 0.),
                    false,
                    true,
                    false,
                ),
                line(
                    104,
                    p(-60., 24.129),
                    p(60., 24.129),
                    p(0., 1.),
                    true,
                    false,
                    true,
                ),
            ],
            ray_calls: 0,
            hits: Vec::new(),
        }
    }
    /// Native vertical intersections, including its shortened/origin output.
    /// This contains no seed-selection, bootstrap state or continuation policy.
    fn ray(&mut self, ray: discovery::Ray) -> Option<(Line, Vec2)> {
        self.ray_calls += 1;
        assert_eq!(ray.delta.x, 0.);
        assert!(ray.delta.y < 0.);
        let mut nearest = f32::INFINITY;
        let mut chosen = None;
        for line in &self.lines {
            let dx = line.ends[1].x - line.ends[0].x;
            if dx == 0. {
                continue;
            }
            let u = (ray.origin.x - line.ends[0].x) / dx;
            let y = line.ends[0].y + u * (line.ends[1].y - line.ends[0].y);
            let t = (y - ray.origin.y) / ray.delta.y;
            if !(-1e-5..=1.00001).contains(&u) || !(-1e-5..=1.00001).contains(&t) {
                continue;
            }
            if t < nearest {
                nearest = t;
                let fraction = if t < 0. { 0. } else { t.min(1.) * 0.99 };
                chosen = Some((
                    *line,
                    Vec2 {
                        x: ray.origin.x,
                        y: ray.origin.y + ray.delta.y * fraction,
                    },
                ));
            }
        }
        if let Some((line, _)) = chosen {
            self.hits.push(line.id);
        }
        chosen
    }
}
impl Source for BootstrapStage {
    fn read(&mut self, id: usize) -> Option<Line> {
        self.lines.iter().copied().find(|line| line.id == id)
    }
    fn next(&mut self, line: Line, end: usize) -> Option<(usize, usize)> {
        let shared = *line.ends.get(end)?;
        self.lines
            .iter()
            .copied()
            .filter(|next| next.id != line.id && !next.passable)
            .find_map(|next| discovery::far_end(shared, next).map(|far| (next.id, far)))
    }
}
fn bootstrap_closed_outline(touched: Option<usize>) -> (Surfaces, BootstrapStage) {
    let mut stage = BootstrapStage::fixture();
    let touch = touched.and_then(|id| stage.read(id));
    let mut bootstrap = discovery::Bootstrap::new([-40., 40., 70., -60.], touch).unwrap();
    for _ in 0..32 {
        if bootstrap.seed().is_some() || bootstrap.finished() {
            break;
        }
        let ray = bootstrap
            .current()
            .expect("unfinished bootstrap must offer one ray");
        match stage.ray(ray) {
            Some((line, point)) => bootstrap.hit(line, point).unwrap(),
            None => bootstrap.miss(),
        }
    }
    let (seed, end) = bootstrap
        .seed()
        .expect("a solid main floor must be found without a grounded solid P1 touch");
    let mut surfaces = Surfaces::EMPTY;
    surfaces.begin(seed, end);
    for _ in 0..4 {
        surfaces.step(&mut stage);
    }
    assert_eq!(surfaces.state, surface::Walk::Closed);
    assert_eq!(surfaces.lines().count(), 4);
    assert_eq!(
        surfaces
            .lines()
            .filter(|line| line.kind() == Some(Kind::Wall))
            .count(),
        2
    );
    assert_eq!(
        surfaces
            .lines()
            .filter(|line| line.kind() == Some(Kind::Ceiling))
            .count(),
        1
    );
    assert!(surfaces.lines().all(|line| !line.passable));
    // Complete the same per-column production schedule used for platforms.
    let mut rays = discovery::Rays::new([-40., 40., 70., -60.]).unwrap();
    for _ in 0..40 {
        let Some(ray) = rays.current() else { break };
        match stage.ray(ray) {
            Some((line, point)) => {
                rays.hit(line, point).unwrap();
                if line.passable {
                    assert!(surfaces.push(line));
                }
            }
            None => rays.miss(),
        }
    }
    assert!(
        rays.current().is_none(),
        "platform discovery must terminate"
    );
    assert_eq!(surfaces.lines().filter(|line| line.passable).count(), 1);
    (surfaces, stage)
}
#[test]
fn bootstrap_from_passable_p1_touch_discovers_main_outline_and_platform() {
    // Loop1: Mario is at (-40,24.129) on the separate Battlefield platform.
    let (_, stage) = bootstrap_closed_outline(Some(104));
    assert_eq!(
        &stage.hits[..2],
        &[104, 100],
        "bootstrap must progress past the biased passable hit to the solid floor"
    );
    assert!(stage.ray_calls <= 38);
}
#[test]
fn bootstrap_airborne_without_p1_touch_completes_discovery() {
    let (_, stage) = bootstrap_closed_outline(None);
    assert_eq!(&stage.hits[..2], &[104, 100]);
    assert!(stage.ray_calls <= 38);
}
#[test]
fn bootstrap_main_floor_touch_preserves_immediate_seed() {
    let stage = BootstrapStage::fixture();
    let bootstrap =
        discovery::Bootstrap::new([-40., 40., 70., -60.], Some(stage.lines[0])).unwrap();
    assert_eq!(bootstrap.seed(), Some((100, 1)));
    assert!(
        bootstrap.current().is_none(),
        "solid touch needs no bootstrap ray"
    );
    bootstrap_closed_outline(Some(100));
}
#[test]
fn bootstrap_empty_stage_finishes_with_bounded_native_work() {
    let mut stage = BootstrapStage::fixture();
    stage.lines.clear();
    let mut bootstrap = discovery::Bootstrap::new([-20., 20., 50., -50.], None).unwrap();
    for _ in 0..6 {
        if bootstrap.finished() {
            break;
        }
        let ray = bootstrap
            .current()
            .expect("unfinished bootstrap must offer a ray");
        assert!(stage.ray(ray).is_none());
        bootstrap.miss();
    }
    assert!(
        bootstrap.finished(),
        "five empty columns must end bootstrap"
    );
    assert_eq!(bootstrap.seed(), None);
    assert!(bootstrap.current().is_none());
    assert_eq!(
        stage.ray_calls, 5,
        "one ray per ten-unit column, including both edges"
    );
    for _ in 0..3 {
        bootstrap.miss();
        assert!(bootstrap.finished());
        assert!(bootstrap.current().is_none());
    }
    assert_eq!(stage.ray_calls, 5);
}

#[test]
fn bootstrap_operation_budget_is_exact_finite_and_sticky() {
    // Five ten-unit columns, at most two visits per retained line plus miss,
    // and the existing maximum outline-walk work. This is an operation bound,
    // not an elapsed-frame deadline or a larger native geometry tolerance.
    assert_eq!(surface::MAX_LINES, 64);
    // The second width needs ceil(41/10), not truncation to four intervals.
    for (right, operations) in [(20., 714), (21., 844)] {
        let mut bootstrap = discovery::Bootstrap::new([-20., right, 50., -50.], None).unwrap();
        for _ in 0..operations {
            assert!(
                bootstrap.tick(),
                "budget expired before its exact work allowance"
            );
        }
        assert!(
            !bootstrap.tick(),
            "a stalled source cannot get unbounded native work"
        );
        for _ in 0..8 {
            assert!(!bootstrap.tick(), "budget exhaustion is terminal");
        }
    }
}

#[test]
fn bootstrap_operation_budget_rejects_invalid_or_overflowing_camera_ranges() {
    for range in [
        [f32::NAN, 20., 50., -50.],
        [20., -20., 50., -50.],
        [-20., 20., -50., -50.],
        [-f32::MAX, f32::MAX, 50., -50.],
        [0., f32::MAX, 50., -50.],
    ] {
        assert!(
            discovery::Bootstrap::new(range, None).is_none(),
            "{range:?}"
        );
    }
}

#[test]
fn bootstrap_ray_can_seed_an_unflagged_solid_outline_line() {
    let mut line = BootstrapStage::fixture().lines[0];
    line.floor = false;
    line.normal = Vec2 { x: 0.8, y: 0.6 };
    assert!(line.valid());
    assert_eq!(line.kind(), None);
    let mut bootstrap = discovery::Bootstrap::new([-20., 20., 50., -50.], None).unwrap();
    bootstrap.hit(line, Vec2 { x: -20., y: 0.5 }).unwrap();
    assert_eq!(bootstrap.seed(), Some((line.id, 1)));
    assert!(bootstrap.current().is_none());
}

#[test]
fn recovery_discards_same_epoch_dead_ids_then_rebinds_and_allows_launch() {
    use crate::model::cpu::role::{RoleTable, Scene};
    use surface::{RememberedSpot, StageEpoch, Walk};
    let mut roles = RoleTable::new();
    roles.observe_scene(Some(Scene::Training));
    let epoch = StageEpoch {
        stage: 3,
        form: 0,
        training: roles.receipt(0).unwrap(),
    };
    let mut source = BootstrapStage::fixture();
    let mut surfaces = Surfaces::EMPTY;
    surfaces.bind(epoch);
    surfaces.begin(100, 1);
    for _ in 0..4 {
        surfaces.step(&mut source);
    }
    assert_eq!(surfaces.state, Walk::Closed);
    let old = picker::Spot {
        line: source.read(101).unwrap(),
        point: Vec2 { x: 80., y: -10. },
    };
    let saved = RememberedSpot::capture(old.line, old.point).unwrap();
    for line in &mut source.lines {
        line.id += 1000;
    }
    // Source::read(None) is the adapter's unchanged result for inactive raw lines.
    assert!(!old.current(&mut source));
    assert!(
        !surfaces.bind(epoch),
        "Training reset can keep the same epoch"
    );
    assert_eq!(surfaces.lines().next().unwrap().id, 100);
    surfaces.discard();
    assert_eq!(surfaces.state, Walk::Broken);
    assert_eq!(surfaces.lines().count(), 0);
    assert!(
        !surfaces.bind(epoch),
        "discard must preserve the current epoch"
    );
    assert_eq!(surfaces.epoch(), Some(epoch));
    assert_eq!(surfaces.step(&mut Lines(vec![])), Walk::Broken);
    assert_eq!(
        saved.resolve(&surfaces),
        Err(surface::RecoverError::Incomplete)
    );
    surfaces.begin(1100, 1);
    for _ in 0..4 {
        surfaces.step(&mut source);
    }
    assert_eq!(surfaces.state, Walk::Closed);
    let (line, point) = saved.resolve(&surfaces).unwrap();
    assert_eq!(line.id, 1101);
    assert_eq!(point, old.point);
    assert!(picker::Spot { line, point }.current(&mut source));
    assert!(source.read(old.line.id).is_none());
    struct Guarded<'a> {
        native: Fake,
        spot: picker::Spot,
        source: &'a mut BootstrapStage,
        placed_id: Option<usize>,
    }
    impl Launcher for Guarded<'_> {
        type Error = ();
        fn place(&mut self, launch: Launch) -> Result<(), ()> {
            if !self.spot.current(self.source) {
                return Err(());
            }
            assert_eq!(launch.target, self.spot.point);
            self.placed_id = Some(self.spot.line.id);
            self.native.place(launch)
        }
        fn launch(&mut self) -> Result<(), ()> {
            self.native.launch()
        }
        fn attempt(&self) -> Result<Attempt, ()> {
            self.native.attempt()
        }
        fn restore(&mut self) {
            self.native.restore();
        }
    }
    let mut controller = Controller::IDLE;
    controller.start(Length::Free);
    let mut accepted = setup();
    accepted.launch.target = point;
    {
        let mut retired = Guarded {
            native: Fake::new(),
            spot: old,
            source: &mut source,
            placed_id: None,
        };
        assert_eq!(controller.place(&mut retired, accepted), Err(()));
        assert_eq!(retired.placed_id, None);
        assert_eq!(retired.native.placements, 0);
    }
    let mut launcher = Guarded {
        native: Fake::new(),
        spot: picker::Spot { line, point },
        source: &mut source,
        placed_id: None,
    };
    assert_eq!(controller.place(&mut launcher, accepted), Ok(Event::Placed));
    assert_eq!(launcher.placed_id, Some(1101));
    assert_eq!(
        controller.tick(&mut launcher, false, Some(false)),
        Ok(Event::None)
    );
    assert_eq!(
        controller.tick(&mut launcher, false, Some(false)),
        Ok(Event::Launched)
    );
    assert_eq!(
        (launcher.native.placements, launcher.native.launches),
        (1, 1)
    );
}

#[test]
fn recovery_missing_geometry_reports_specific_failure_without_nearest_substitution() {
    use surface::{RecoverError, RememberedSpot, Walk};
    let old = ceiling(1, -40.);
    let saved = RememberedSpot::capture(old, Vec2 { x: 0., y: -40. }).unwrap();
    let mut fresh = Surfaces::EMPTY;
    fresh.push(ceiling(2, -39.));
    fresh.state = Walk::Closed;
    assert!(fresh
        .nearest(Vec2 { x: 0., y: -40. }, Some(Kind::Ceiling))
        .is_some());
    assert_eq!(saved.resolve(&fresh), Err(RecoverError::MissingSurface));
    assert_eq!(
        RecoverError::MissingSurface.message(),
        "Selected surface no longer exists"
    );
    assert_eq!(
        RecoverError::Incomplete.message(),
        "Surface discovery incomplete"
    );
}

#[test]
fn recovery_requires_exact_normal_flags_and_unmoved_endpoints() {
    use surface::{RecoverError, RememberedSpot, Walk};
    let old = ceiling(1, -40.);
    let saved = RememberedSpot::capture(old, Vec2 { x: 0., y: -40. }).unwrap();
    let mut shifted = old;
    shifted.ends[0].x = f32::from_bits(old.ends[0].x.to_bits() + 1);
    let mut normal = old;
    normal.normal.x = 0.01;
    let mut floor = old;
    floor.floor = true;
    let mut wall = old;
    wall.wall = true;
    let mut passable = old;
    passable.passable = true;
    for mut changed in [shifted, normal, floor, wall, passable] {
        changed.id = 99;
        let mut fresh = Surfaces::EMPTY;
        assert!(fresh.push(changed));
        fresh.state = Walk::Closed;
        assert_eq!(
            saved.resolve(&fresh),
            Err(RecoverError::MissingSurface),
            "{changed:?}"
        );
    }
}

#[test]
fn recovery_accepts_reversed_endpoints_with_same_normal_kind_and_point() {
    use surface::{RememberedSpot, Walk};
    let old = ceiling(1, -40.);
    let point = Vec2 { x: 7., y: -40. };
    let saved = RememberedSpot::capture(old, point).unwrap();
    let mut reversed = old;
    reversed.id = 7;
    reversed.ends.swap(0, 1);
    let mut fresh = Surfaces::EMPTY;
    fresh.push(reversed);
    fresh.state = Walk::Closed;
    assert_eq!(saved.resolve(&fresh), Ok((reversed, point)));
}

#[test]
fn recovery_capture_rejects_invalid_or_untargetable_world_data() {
    use surface::RememberedSpot;
    let old = ceiling(1, -40.);
    assert!(RememberedSpot::capture(
        old,
        Vec2 {
            x: f32::NAN,
            y: -40.
        }
    )
    .is_none());
    let mut invalid = old;
    invalid.ends[1] = invalid.ends[0];
    assert!(RememberedSpot::capture(invalid, old.ends[0]).is_none());
    let mut unflagged = old;
    unflagged.normal = Vec2 { x: 0.8, y: 0.6 };
    assert_eq!(unflagged.kind(), None);
    assert!(RememberedSpot::capture(unflagged, old.ends[0]).is_none());
}

#[test]
fn native_timing_grade_labels_state_before_without_inventing_press_coverage() {
    for (grade, expected) in [
        (Grade::Perfect(Some(0)), "Perfect · 0f before"),
        (Grade::Perfect(Some(7)), "Perfect · 7f before"),
        (Grade::Perfect(None), "Perfect · timing unavailable"),
        (Grade::Locked(9), "Locked out (2nd press 10f after 1st)"),
        (Grade::Missed, "Missed"),
        (Grade::Unavailable, "Grading unavailable (not counted)"),
    ] {
        let mut label = String::new();
        grade.write(&mut label).unwrap();
        assert_eq!(label, expected);
    }
}

#[test]
fn native_timing_summary_averages_only_known_perfect_samples() {
    let mut summary = session::Summary::default();
    for grade in [
        Grade::Perfect(None),
        Grade::Perfect(Some(2)),
        Grade::Perfect(Some(5)),
        Grade::Held,
        Grade::Early(1),
        Grade::Late(1),
        Grade::Locked(2),
        Grade::Missed,
        Grade::Unavailable,
    ] {
        summary.record(grade);
    }
    assert_eq!(summary.attempts, 8);
    assert_eq!(summary.timed_perfect, 2);
    assert_eq!(summary.average(), Some(3.5));
    let mut label = String::new();
    summary.write(&mut label).unwrap();
    assert_eq!(
        label,
        "Done! 8: 3 Perfect · 1 Held · 1 Early · 1 Late · 2 Missed · best 4 · avg 3.5f before"
    );
}

#[test]
fn native_timing_summary_without_calibrated_samples_omits_average() {
    let mut summary = session::Summary::default();
    for grade in [Grade::Perfect(None), Grade::Held, Grade::Unavailable] {
        summary.record(grade);
    }
    assert_eq!(summary.attempts, 2);
    assert_eq!(summary.timed_perfect, 0);
    assert_eq!(summary.average(), None);
    let mut label = String::new();
    summary.write(&mut label).unwrap();
    assert_eq!(
        label,
        "Done! 2: 1 Perfect · 1 Held · 0 Early · 0 Late · 0 Missed · best 2"
    );
}

fn assert_native_no_tech_counted(surface: Surface, entered: i32) {
    // Native constant order used by the guarded adapter; requested status alone is insufficient.
    let ids = [103, 104, 80, 78, 107, 109, 77, 105, 106, 108, 76];
    let mut controller = Controller::IDLE;
    let mut launcher = Fake::new();
    controller.start(Length::Free);
    controller.place(&mut launcher, setup()).unwrap();
    controller.tick(&mut launcher, false, None).unwrap();
    controller.tick(&mut launcher, false, None).unwrap();
    controller.session.summary.record(Grade::Perfect(Some(3)));
    let mut observed = snapshot(launcher.attempt.id);
    observed.contact.surface = surface;
    observed.contact.requested_status = entered;
    observed.source_status = Some(73);
    observed.entered_status = Some(entered);
    assert_eq!(
        session::accepted_status(observed, ids),
        Some(session::NativeOutcome::Miss)
    );
    assert_eq!(controller.retained_snapshot(observed, ids, 0), Event::Graded);
    for _ in 0..=session::LATE {
        controller.tick(&mut launcher, false, None).unwrap();
    }
    assert_eq!(controller.session.result, Some(Grade::Missed));
    assert_eq!(controller.session.summary.attempts, 2);
    assert_eq!(controller.session.summary.missed, 1);
    assert_eq!(controller.session.summary.streak, 0);
    // An unreadable late-input reason cannot erase a proven native no-tech result.
}

#[test]
fn native_floor_no_tech_is_counted_even_without_a_late_press_reason() {
    assert_native_no_tech_counted(Surface::Floor, 78);
}
#[test]
fn native_wall_no_tech_is_counted_even_without_a_late_press_reason() {
    assert_native_no_tech_counted(Surface::Wall, 76);
}
#[test]
fn native_ceiling_no_tech_is_counted_even_without_a_late_press_reason() {
    assert_native_no_tech_counted(Surface::Ceiling, 77);
}

#[test]
fn ten_native_misses_advance_and_finish_the_session() {
    let mut session = session::Session::IDLE;
    session.start(Length::Attempts(10));
    for attempt in 1..=10 {
        session.placed(1);
        assert_eq!(session.tick(false, Some(false)), session::Action::Launch);
        session.contact_grade(None);
        for _ in 0..session::LATE {
            session.tick(false, Some(false));
        }
        assert_eq!(session.result, Some(Grade::Missed));
        assert_eq!(session.summary.attempts, attempt);
        assert_eq!(session.summary.streak, 0);
        let mut action = session::Action::None;
        for _ in session::LATE..session::HOLD {
            action = session.tick(false, Some(false));
        }
        assert_eq!(action, if attempt == 10 { session::Action::Summary } else {
            session::Action::Place { reroll: true }
        });
    }
    assert_eq!(session.phase, session::Phase::Done);
    assert_eq!(session.summary.missed, 10);
}

/// rc.15 practice menu (tr-review-astra-3 16:37Z, finding 1): a menu visit during a running Tech practice suspends the
/// attempt in flight and keeps the session. The count, the streak, the clock and the length survive; B resumes it with a
/// new attempt (the same setup when the visit cut it before its grade).
#[test]
fn a_menu_visit_suspends_the_session_and_keeps_its_progress() {
    let mut c = Controller::IDLE;
    let mut f = Fake::new();
    let contact = ContactReceipt { surface: Surface::Wall, requested_status: 999, passive_enabled: true, position: Vec2::default() };
    c.start(Length::Attempts(20));
    c.place(&mut f, setup()).unwrap();
    c.tick(&mut f, false, Some(false)).unwrap();
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::Launched));
    f.attempt.record_contact(contact);
    c.tick(&mut f, false, Some(false)).unwrap();
    assert_eq!(c.outcome(Outcome { attempt: 1, decision: decision() }), Event::Graded);
    assert_eq!((c.session.summary.attempts, c.session.phase), (1, Phase::Hold(0)));
    let elapsed = c.session.elapsed;
    // The menu opens while the graded attempt holds: the next one is a new setup.
    assert_eq!(c.pause_for_menu(&mut f), Event::NeedSetup { reroll: true });
    assert_eq!((c.session.phase, c.session.summary.attempts, c.session.length), (Phase::Place, 1, Length::Attempts(20)));
    assert_eq!((c.session.elapsed, f.restores), (elapsed, 1), "the clock stops and the fighter is released");
    assert_eq!(c.pause_for_menu(&mut f), Event::None, "once per visit");
    // B: the session goes on. The next attempt is placed and the menu opens again in its delay: the same setup repeats.
    c.place(&mut f, setup()).unwrap();
    assert_eq!(c.session.phase, Phase::Delay(2));
    assert_eq!(c.pause_for_menu(&mut f), Event::NeedSetup { reroll: false });
    assert_eq!((c.session.phase, c.session.summary.attempts, f.restores), (Phase::Place, 1, 2));
    assert_eq!(c.tick(&mut f, false, Some(false)), Ok(Event::None));
    assert_eq!(c.place(&mut f, setup()), Ok(Event::Placed), "the resumed session places its next attempt");
    assert_eq!(c.session.summary.attempts, 1);
    // Nothing to suspend before a start or after the summary.
    let mut idle = Controller::IDLE;
    assert_eq!((idle.pause_for_menu(&mut f), f.restores), (Event::None, 2));
}


fn tech_f006_fixture() -> (Surfaces, picker::Spot, crate::model::tech_settings::Config) {
    let v = |x, y| Vec2 { x, y };
    // Training stage 308, stock 13.0.5 main outline and passable platforms.
    let vertices = [v(620., 0.), v(-80., 0.), v(-80., -340.), v(620., -340.)];
    let normals = [v(0., 1.), v(-1., 0.), v(0., -1.), v(1., 0.)];
    let mut surfaces = Surfaces::EMPTY;
    for i in 0..4 {
        assert!(surfaces.push(Line {
            id: i + 1,
            ends: [vertices[i], vertices[(i + 1) % 4]],
            normal: normals[i],
            floor: i == 0,
            wall: i == 1 || i == 3,
            passable: false,
        }));
    }
    for (i, (left, right, y)) in [
        (-309.422, -274.789, 24.119),
        (-267.3165, -232.6835, 47.189003),
        (-225.211, -190.578, 24.119),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(surfaces.push(Line {
            id: i + 5,
            ends: [v(left, y), v(right, y)],
            normal: v(0., 1.),
            floor: true,
            wall: false,
            passable: true,
        }));
    }
    surfaces.state = surface::Walk::Closed;
    let (line, point) = surfaces.nearest(v(-63.36, 0.), Some(Kind::Floor)).unwrap();
    assert_eq!((line.id, point.y), (1, 0.));

    let snapshot = crate::model::settings::Settings::factory().menu;
    let mut config = crate::model::tech_settings::read(&snapshot);
    config.surface = 4;
    config.camera = 1;
    (surfaces, picker::Spot { line, point }, config)
}

fn tech_f006_context(surfaces: &Surfaces, spot: picker::Spot) -> planner::Context<'_> {
    planner::Context {
        surfaces,
        spot,
        params: crate::model::tech_launch::FlightParams {
            brake: 0.051,
            gravity: 0.085,
            fall_speed_cap: 1.85,
        },
        blast: solver::editor_bounds([-589.9998, 500., 360.00015, -137.85207]),
        view: Some(solver::editor_bounds([-509.99976, 420., 280.00003, -39.864185])),
        damage_multiplier: Some(1.2),
    }
}

#[test]
fn damage_surface_hint_reports_fixed_and_random_targets() {
    use crate::model::tech_settings::{Mode, Numeric};
    use surface::{damaging_material, MaterialDraws, MaterialHint};

    let (_, spot, mut config) = tech_f006_fixture();
    let floor = spot.line.kind();
    config.time.mode = Mode::Set;
    let mut draws = MaterialDraws::default();
    for material in [0x12, 0x13, 0x14, 0x1f, 0x24, 0x25] {
        assert!(damaging_material(floor, 0, material));
        assert!(!damaging_material(floor, 0x10, material));
        assert!(!damaging_material(Some(Kind::Ceiling), 0, material));
        draws.observe(damaging_material(floor, 0, material));
    }
    assert_eq!(draws.hint(config).map(MaterialHint::text), Some("Damaging surface: tech fails"));
    config.speed = Numeric { mode: Mode::Random, lo: 100., hi: 250. };
    assert_eq!(draws.hint(config).map(MaterialHint::text), Some("All draws: tech fails on damaging surfaces"));
    config.surface = 1;
    assert_eq!(draws.hint(config), Some(MaterialHint::All));
    assert!(!damaging_material(floor, 0, 1));
    draws.observe(false);
    assert_eq!(draws.hint(config).map(MaterialHint::text), Some("Some draws: tech fails on damaging surfaces"));
    let mut ordinary_or_unknown = MaterialDraws::default();
    ordinary_or_unknown.observe(false);
    assert_eq!(ordinary_or_unknown.hint(config), None);
}

#[test]
fn damage_surface_hint_observes_only_eligible_targets_without_changing_draws() {
    use surface::{MaterialDraws, MaterialHint};

    let (surfaces, _, mut config) = tech_f006_fixture();
    let floor: Vec<_> = surfaces
        .lines()
        .filter(|line| line.kind() == Some(Kind::Floor))
        .collect();
    let mut source = Lines(floor);
    let mut draws = MaterialDraws::default();
    let mut bins = Vec::new();
    let mut observed = Vec::new();
    let picked = picker::random_spot(
        &surfaces,
        &mut source,
        Some(Kind::Floor),
        [-240., 10., 30., -5.],
        &mut |n| {
            bins.push(n);
            n - 1
        },
        &mut |_, line| {
            observed.push(line.id);
            draws.observe(line.id == 7);
        },
    )
    .unwrap();
    assert_eq!((picked.line.id, picked.point.y), (1, 0.));
    assert_eq!(bins, [1, 2, 65536]);
    assert_eq!(observed, [1, 7], "other passable platforms are outside this camera");
    config.surface = 1;
    assert_eq!(draws.hint(config), Some(MaterialHint::Some));
}

#[test]
fn tech_f006_largest_random_draw_still_finds_a_feasible_contact() {
    let (surfaces, spot, config) = tech_f006_fixture();
    let plan = planner::plan(config, tech_f006_context(&surfaces, spot), &mut |bins| bins - 1)
        .expect("Auto should offer a feasible contact for the default random range");
    assert_eq!((plan.contact_frames, plan.setup.launch.angle), (38, 270.));
}

#[test]
fn tech_f006_every_default_auto_draw_has_a_solution() {
    let (surfaces, spot, config) = tech_f006_fixture();
    for frame in 20..=38 {
        let mut draws = 0;
        let plan = planner::plan(config, tech_f006_context(&surfaces, spot), &mut |bins| {
            draws += 1;
            assert_eq!(bins, 19, "the original 39f and 40f draws must be absent");
            u32::from(frame - 20)
        })
        .expect("each admissible default contact time has a floor solution");
        assert_eq!((draws, plan.contact_frames), (1, frame));
        assert_eq!(plan.setup.launch.angle, 270.);
    }
}

#[test]
fn tech_f006_auto_clamp_uses_sampled_speed_and_rejects_empty_intersections() {
    use crate::model::tech_settings::{Mode, Numeric};

    let (surfaces, spot, mut config) = tech_f006_fixture();
    let context = || tech_f006_context(&surfaces, spot);
    config.speed = Numeric { mode: Mode::Random, lo: 80., hi: 100. };
    for (speed_draw, last_frame) in [(0, 30), (1, 34), (2, 38)] {
        let mut choices = [(3, speed_draw), (last_frame - 19, last_frame - 20)].into_iter();
        let plan = planner::plan(config, context(), &mut |bins| {
            let (expected, index) = choices.next().expect("only speed and time draw");
            assert_eq!(bins, expected);
            index
        })
        .unwrap();
        assert!(choices.next().is_none());
        assert_eq!(u32::from(plan.contact_frames), last_frame);
    }
    config.speed.mode = Mode::Set;
    config.time.lo = 31.;
    assert!(planner::plan(config, context(), &mut |_| panic!("the time interval is empty")).is_none());
}

#[test]
fn tech_f006_set_time_and_non_auto_start_keep_their_sampling() {
    use crate::model::tech_settings::{Mode, Numeric};

    let (surfaces, spot, mut config) = tech_f006_fixture();
    let context = || tech_f006_context(&surfaces, spot);
    config.time = Numeric { mode: Mode::Set, lo: 39., hi: 40. };
    assert!(planner::plan(config, context(), &mut |_| panic!("Set time never draws")).is_none());
    config.time.lo = 38.;
    let fixed = planner::plan(config, context(), &mut |_| panic!("Set time never draws")).unwrap();
    assert_eq!(fixed.contact_frames, 38);

    config.time = Numeric { mode: Mode::Random, lo: 20., hi: 40. };
    for (start_mode, tail) in [(Mode::Set, &[][..]), (Mode::Random, &[(1, 0), (360, 90)][..])] {
        config.start = Numeric { mode: start_mode, lo: 30., hi: 30. };
        let mut choices = std::iter::once((21, 20)).chain(tail.iter().copied());
        let plan = planner::plan(config, context(), &mut |bins| {
            let (expected, index) = choices.next().expect("only configured random values draw");
            assert_eq!(bins, expected);
            index
        })
        .unwrap();
        assert!(choices.next().is_none());
        assert_eq!(plan.contact_frames, 40);
        assert_eq!(plan.setup.launch.start, Vec2 { x: spot.point.x, y: 30. });
    }
}
