use super::{
    crossing_margin, di_anchor, level_run, mix, niche_plan, planned_delay, short, zone, Crossing,
    Guarder, Hold, HoldFacts, LedgeWatch, Measured, Reach, Zone,
};
use crate::model::edge_juggle::perfect::{
    hit_recovery, AirPhysics, Correction, Plan, Point, Route, SpecialReach, GUARD, JUMP, SPECIAL,
};

/// The Guard the Edge DI: the shipped survival rule against the point opposite the stage.
fn edge_di(cpu: Point, ledge: Point, speed: Point, hitstun: f32, blast: [f32; 4]) -> Option<Point> {
    hit_recovery(cpu, di_anchor(cpu, ledge), speed, hitstun, blast)
}

const AIR: AirPhysics = AirPhysics {
    horizontal_speed: 2.0,
    horizontal_acceleration: 0.15,
    gravity: 0.12,
    terminal_speed: 2.1,
    jump_speed: 3.5,
};
const UP: SpecialReach = SpecialReach {
    horizontal: 20.0,
    rise: 45.0,
};
const LEDGE: Point = [-70.0, 0.0];
const BOTTOM: f32 = -95.0;
const ROUTES: [Route; 4] = [Route::Near, Route::High, Route::LateLow, Route::Opposite];

fn plan(route: Route) -> Option<Plan> {
    Some(Plan {
        route,
        waypoints: [[0.0; 2]; 3],
        count: 1,
        weight: 8,
    })
}

fn only(route: Route) -> [Option<Plan>; 4] {
    ROUTES.map(|candidate| plan(candidate).filter(|_| candidate == route))
}

/// How often each route is rolled over an even sweep of the draw range.
fn shares(options: [Option<Plan>; 4], zone: Zone, last: Option<Route>) -> [u32; 4] {
    let mut counts = [0; 4];
    for step in 0..1000u32 {
        let route = mix(options, zone, last, step * (u32::MAX / 1000))
            .expect("a feasible route")
            .route;
        counts[ROUTES.iter().position(|&known| known == route).unwrap()] += 1;
    }
    counts
}

#[test]
fn replanning_weights_against_the_preceding_completed_attempt() {
    use super::RouteHistory;

    let mut history = RouteHistory::default();
    history.start();
    history.planned(Route::High);
    history.complete();
    assert_eq!(history.previous(), Some(Route::High));

    history.start();
    history.planned(Route::Near);
    assert_eq!(history.previous(), Some(Route::High));
    history.planned(Route::Opposite);
    assert_eq!(history.previous(), Some(Route::High), "a replan is provisional");

    // Discard the interrupted attempt. The next result publishes its final choice.
    history.start();
    assert_eq!(history.previous(), Some(Route::High));
    history.planned(Route::Near);
    history.planned(Route::LateLow);
    assert_eq!(history.previous(), Some(Route::High));
    history.complete();
    assert_eq!(history.previous(), Some(Route::LateLow));

    history.start();
    history.complete();
    assert_eq!(history.previous(), None, "stock completed the preceding attempt");
}

#[test]
fn horizontal_limit_jumps_before_stock_waits_one_frame_too_long() {
    let reach = Reach::trusted(AIR, true, None);
    let velocity = [0.0, -1.4];
    for inward in [-1.0, 1.0] {
        let start = [LEDGE[0] - 68.0 * inward, -45.0];
        assert!(reach.returns(start, velocity, LEDGE, 0, BOTTOM));
        assert!(!reach.returns(start, velocity, LEDGE, 1, BOTTOM));
        assert!(reach.returns([LEDGE[0], start[1]], velocity, LEDGE, 1, BOTTOM));
        assert_eq!(
            reach.jump_at_horizontal_limit(start, velocity, LEDGE, BOTTOM, 0),
            Correction { press: JUMP, stick: Some([inward, 0.0]), release: 0 },
        );
        for action in [GUARD, JUMP, SPECIAL, 0x1] {
            assert_eq!(
                reach.jump_at_horizontal_limit(start, velocity, LEDGE, BOTTOM, action),
                Correction::default(),
            );
        }
        assert_eq!(
            Reach::trusted(AIR, false, None)
                .jump_at_horizontal_limit(start, velocity, LEDGE, BOTTOM, 0),
            Correction::default(),
        );
        for other in [[start[0], -43.0], [start[0], -52.0], [f32::NAN, start[1]]] {
            assert_eq!(
                reach.jump_at_horizontal_limit(other, velocity, LEDGE, BOTTOM, 0),
                Correction::default(),
            );
        }
    }
}

#[test]
fn native_params_give_the_traced_double_jump_and_drift() {
    use crate::model::edge_juggle::perfect::NativeAir;
    // fighter_param 13.0.5: (air_accel_y, jump_aerial_y, air_accel_x_add, air_accel_x_mul), then the
    // first double-jump frame and the full-stick drift step of the exact trace edge-niche-cand-fd-3.
    let traced = [
        ("Pikachu", [0.095, 35.5, 0.01, 0.09], 2.550, 0.100),
        ("Sheik", [0.15, 40.0, 0.01, 0.08], 3.389, 0.090),
        ("Mr. Game & Watch", [0.08, 27.51, 0.07, 0.03], 2.058, 0.100),
        ("Diddy Kong", [0.125, 41.21, 0.0, 0.05], 3.147, 0.050),
        ("R.O.B.", [0.09, 38.0, 0.04, 0.045], 2.570, 0.085),
        ("Little Mac", [0.09, 26.0, 0.01, 0.03], 2.118, 0.040),
    ];
    for (fighter, [gravity, height, add, mul], launch, drift) in traced {
        let air = AirPhysics::from_params(NativeAir {
            air_speed_x_stable: 1.0,
            air_accel_x_add: add,
            air_accel_x_mul: mul,
            air_accel_y: gravity,
            air_speed_y_stable: 1.5,
            jump_aerial_y: height,
        });
        assert!(air.valid(), "{fighter}: a zero `add` term is a real fighter");
        assert!(
            (air.jump_speed - launch).abs() < 0.002,
            "{fighter}: the param is a height of {height}, the jump starts at {launch}, not {}",
            air.jump_speed
        );
        assert!(
            (air.horizontal_acceleration - drift).abs() < 0.0005,
            "{fighter}: full-stick drift is add + mul = {drift}, not {}",
            air.horizontal_acceleration
        );
        let (mut rise, mut speed) = (0.0, air.jump_speed);
        while speed > 0.0 {
            rise += speed;
            speed -= air.gravity;
        }
        assert!(
            (rise - height).abs() < 0.5,
            "{fighter}: the simulated apex {rise} is the param {height}"
        );
    }
}

#[test]
fn zone_classifies_the_guarder_against_the_near_ledge() {
    let cpu = [-95.0, -14.0];
    let guard = |position: Point, hanging: bool, airborne: bool| Guarder {
        position,
        hanging,
        airborne,
        reach: 20.0,
    };
    let at = |position, hanging, airborne| zone(guard(position, hanging, airborne), LEDGE, cpu);
    assert_eq!(at([-74.0, -6.0], true, false), Zone::Hang);
    assert_eq!(at([-60.0, 0.0], false, false), Zone::Ledge);
    assert_eq!(at([-30.0, 0.0], false, false), Zone::Mid);
    assert_eq!(at([20.0, 0.0], false, false), Zone::Far);
    assert_eq!(at([-90.0, 5.0], false, true), Zone::OffHigh);
    assert_eq!(at([-90.0, -40.0], false, true), Zone::OffLow);
    assert_eq!(
        zone(guard([60.0, 0.0], false, false), [70.0, 0.0], [95.0, -14.0]),
        Zone::Ledge,
        "the right ledge mirrors the left"
    );
    assert_eq!(
        at([74.0, -6.0], true, false),
        Zone::Far,
        "hanging on the far ledge does not guard this one"
    );
    assert_eq!(
        at([f32::NAN, 0.0], false, false),
        Zone::Far,
        "an unreadable guarder is no guarder"
    );
}

#[test]
fn mix_never_takes_the_ledge_from_a_hanging_guarder() {
    let [near, high, _, _] = shares(ROUTES.map(plan), Zone::Hang, None);
    assert_eq!(near, 0, "a held ledge is not a recovery");
    assert!(high > 500, "land on the stage behind the guarder: {high}");
}

#[test]
fn mix_goes_low_under_an_offstage_guarder_above_and_high_over_one_below() {
    let [near, high, low, across] = shares(ROUTES.map(plan), Zone::OffHigh, None);
    assert_eq!(high, 0);
    assert!(low > near && low > across);
    let [near, high, low, across] = shares(ROUTES.map(plan), Zone::OffLow, None);
    assert_eq!(low, 0);
    assert!(high > near && high > across);
}

#[test]
fn mix_rolls_a_feasible_crossing_as_one_option_and_penalises_a_repeat() {
    let fresh = shares(ROUTES.map(plan), Zone::Ledge, None)[3];
    assert!(
        0 < fresh && fresh < 1000,
        "a crossing is one option: {fresh}"
    );
    let repeat = shares(ROUTES.map(plan), Zone::Ledge, Some(Route::Opposite))[3];
    assert!(
        repeat * 10 < fresh * 7,
        "a repeat is rarer: {repeat} of {fresh}"
    );
    assert_eq!(
        shares(ROUTES.map(plan), Zone::Far, None)[3],
        0,
        "no crossing unguarded"
    );
}

#[test]
fn mix_keeps_the_only_feasible_non_crossing_route_whatever_the_zone() {
    assert_eq!(shares(only(Route::Near), Zone::Hang, None), [1000, 0, 0, 0]);
    assert_eq!(shares(only(Route::High), Zone::OffHigh, None), [0, 1000, 0, 0]);
    assert_eq!(mix([None; 4], Zone::Ledge, None, 7), None);
}

#[test]
fn a_crossing_never_becomes_the_default_when_all_other_models_decline() {
    let crossing = only(Route::Opposite);
    assert_eq!(mix(crossing, Zone::Far, None, 0), None);
    assert!(mix(crossing, Zone::Ledge, None, 0).is_some());
    assert_eq!(mix(crossing, Zone::Ledge, None, u32::MAX), None);

    let [_, high, _, opposite] = ROUTES.map(plan);
    let alternatives = [None, high, None, opposite];
    // The P1 zone weighs this particular High candidate at zero.
    assert_eq!(mix(alternatives, Zone::OffHigh, None, u32::MAX), None);
}

#[test]
fn reach_counts_jump_and_up_special_together() {
    let from = [-95.0, -57.0];
    let fall = [0.0, -1.4];
    let reach = |jump, special| Reach { air: AIR, jump, special };
    assert!(
        !AIR.reaches_with_jump(from, fall, LEDGE, 0, BOTTOM),
        "the jump alone is short"
    );
    assert!(!reach(true, None).returns(from, fall, LEDGE, 0, BOTTOM));
    assert!(
        !reach(false, Some(UP)).returns(from, fall, LEDGE, 0, BOTTOM),
        "the special alone is short"
    );
    assert!(reach(true, Some(UP)).returns(from, fall, LEDGE, 0, BOTTOM));
    assert!(
        reach(true, None).returns([-95.0, -14.0], fall, LEDGE, 4, BOTTOM),
        "the shipped jump-only case is unchanged"
    );
    assert!(!reach(true, Some(UP)).returns([f32::NAN, 0.0], fall, LEDGE, 0, BOTTOM));
    let edge = [-88.0, -44.0];
    assert!(reach(false, Some(UP)).returns(edge, fall, LEDGE, 0, BOTTOM));
    assert!(
        !Reach::trusted(AIR, false, Some(UP)).returns(edge, fall, LEDGE, 0, BOTTOM),
        "timing keeps a margin inside the stock table"
    );
}

#[test]
fn deadline_is_the_last_frame_that_still_returns() {
    let reach = Reach {
        air: AIR,
        jump: true,
        special: Some(UP),
    };
    let from = [-95.0, -14.0];
    let fall = [0.0, -1.4];
    let last = reach
        .deadline(from, fall, LEDGE, BOTTOM)
        .expect("returns now");
    assert!(reach.returns(from, fall, LEDGE, last, BOTTOM));
    assert!(!reach.returns(from, fall, LEDGE, last + 1, BOTTOM));
    assert!(
        last > 8,
        "the low route is deeper than the old 8-frame delay: {last}"
    );
    assert_eq!(
        reach.deadline([-95.0, -93.0], fall, LEDGE, BOTTOM),
        Some(1),
        "one more frame before the fall reaches the blast boundary"
    );
    assert_eq!(reach.deadline([-95.0, BOTTOM], fall, LEDGE, BOTTOM), None);
}

#[test]
fn low_route_waits_for_the_deadline_and_near_stays_early() {
    assert_eq!(
        planned_delay(Route::LateLow, Zone::Ledge, Some(30), 12345),
        27
    );
    for draw in [0, 7, 10, 11, 999, u32::MAX] {
        assert!(planned_delay(Route::Near, Zone::Mid, Some(30), draw) <= 10);
    }
    assert!((0..40).any(|draw| planned_delay(Route::Near, Zone::Mid, Some(30), draw) > 0));
    assert_eq!(planned_delay(Route::Near, Zone::Far, Some(30), 9), 0);
    assert_eq!(planned_delay(Route::High, Zone::Hang, Some(30), 9), 0);
    assert_eq!(planned_delay(Route::LateLow, Zone::Ledge, None, 9), 0);
    assert_eq!(planned_delay(Route::LateLow, Zone::Ledge, Some(2), 9), 0);
}

fn held(button: u32, planned: u8, can_wait: bool, buttons: u32) -> HoldFacts {
    HoldFacts {
        frame: 0,
        buttons,
        button,
        jumps: 1,
        planned,
        can_wait,
        hot: false,
    }
}

#[test]
fn robot_stock_burner_stays_available_at_the_first_press() {
    use crate::model::edge_juggle::perfect::rob;

    // A rectangular table cannot predict R.O.B.'s fuel-dependent climb or
    // a stock sequence that presses special again to sustain the burner.
    assert!(super::stock_timing_reach(rob::KIND, Some(UP)).is_none());
    assert!(super::stock_timing_reach(0, Some(UP)).is_some());
    assert!(super::stock_timing_reach(0, None).is_none());
}

#[test]
fn bowser_jr_preserves_the_first_stock_abandon_ship_press() {
    use crate::model::edge_juggle::perfect::kart;

    // The same stock press recovers at frame 44 outside the measured Kart Dash route.
    let stock = 0x8802;
    let timing = super::stock_timing_reach(kart::KIND, Some(UP));
    let edit = timing.map_or_else(Correction::default, |_| {
        Hold::default().step(held(SPECIAL, 38, true, stock))
    });
    assert_eq!(stock & !edit.release | edit.press, stock);
}

#[test]
fn pichu_ordinary_jump_timing_declines_when_agility_is_unavailable() {
    const PICHU: i32 = 0x13;
    let timing = super::stock_timing_reach(PICHU, None);
    assert!(!super::generic_timing_allowed(PICHU, timing, true));
    assert!(super::generic_timing_allowed(0, None, true));
    assert!(!super::generic_timing_allowed(0, None, false));
}

#[test]
fn pichu_ordinary_return_preserves_stock_jump_and_agility_timing() {
    const PICHU: i32 = 0x13;
    const PIKACHU: i32 = 0x08;
    let timing = super::stock_timing_reach(PICHU, Some(UP));
    for (button, planned, stock) in [(JUMP, 46, 0x4), (SPECIAL, 7, 0x802)] {
        // Exact 133979fc trace f13-pichu-6, input sequences 14977 and 15168.
        let edit = if super::generic_timing_allowed(PICHU, timing, false) {
            Hold::default().step(held(button, planned, true, stock))
        } else {
            Correction::default()
        };
        assert_eq!(stock & !edit.release | edit.press, stock, "{button:#x}");
    }
    assert!(timing.is_none(), "the generic projection cannot aim both bursts");
    assert!(super::stock_timing_reach(PIKACHU, Some(UP)).is_some());
}

/// A hold stepped once per game frame.
#[derive(Default)]
struct Ticking(Hold, u32);

impl Ticking {
    fn step(&mut self, facts: HoldFacts) -> Correction {
        self.1 += 1;
        self.0.step(HoldFacts {
            frame: self.1,
            ..facts
        })
    }

    fn waiting(&self) -> bool {
        self.0.waiting()
    }
}

#[test]
fn hold_counts_game_frames_not_think_calls() {
    let mut hold = Hold::default();
    let at = |frame, buttons| HoldFacts {
        frame,
        ..held(JUMP, 3, true, buttons)
    };
    let first = hold.step(at(10, 0x8004));
    assert_eq!(first.release, JUMP);
    for _ in 0..5 {
        assert_eq!(
            hold.step(at(10, 0x8004)),
            first,
            "a second think on one frame repeats the pad"
        );
    }
    assert_eq!(hold.step(at(11, 0x8000)), Correction::default());
    assert_eq!(hold.step(at(12, 0x8000)), Correction::default());
    let spent = hold.step(at(13, 0x8000));
    assert_eq!(spent.press, JUMP, "three game frames after the plan");
    assert_eq!(hold.step(at(13, 0x8000)), spent);
    assert!(!hold.waiting());
}

#[test]
fn a_new_plan_keeps_the_press_the_hold_still_owes() {
    let at = |frame, button, planned, buttons| HoldFacts {
        frame,
        ..held(button, planned, true, buttons)
    };
    let mut hold = Hold::default();
    assert_eq!(hold.step(at(10, JUMP, 20, 0x8004)).release, JUMP);
    assert_eq!(hold.step(at(11, JUMP, 20, 0x8000)), Correction::default());
    // The guarder changed zone: the route is rolled again with a new delay.
    hold.replan();
    assert!(hold.waiting(), "the swallowed stock jump is still owed");
    assert_eq!(
        hold.step(at(12, JUMP, 2, 0x8000)),
        Correction::default(),
        "the new plan restarts the clock"
    );
    assert_eq!(hold.step(at(13, JUMP, 2, 0x8000)), Correction::default());
    assert_eq!(hold.step(at(14, JUMP, 2, 0x8000)).press, JUMP);
    assert!(!hold.waiting());

    let mut at_once = Hold::default();
    at_once.step(at(10, JUMP, 20, 0x8004));
    at_once.replan();
    assert_eq!(
        at_once.step(at(11, JUMP, 0, 0x8000)).press,
        JUMP,
        "a plan with no delay spends the owed jump on its first frame"
    );

    let mut spent = Hold::default();
    spent.step(at(10, JUMP, 20, 0x8004));
    spent.replan();
    assert_eq!(
        spent.step(at(11, SPECIAL, 0, 0x8000)),
        Correction::default(),
        "an owed jump is never paid with the up special"
    );
    assert!(!spent.waiting());

    let mut elsewhere = Hold::default();
    elsewhere.step(HoldFacts {
        jumps: 3,
        ..at(10, JUMP, 20, 0x8004)
    });
    elsewhere.replan();
    assert_eq!(
        elsewhere.step(HoldFacts {
            jumps: 2,
            ..at(11, JUMP, 0, 0x8000)
        }),
        Correction::default(),
        "a jump spent meanwhile settles the debt"
    );
    assert!(!elsewhere.waiting());
}

#[test]
fn jump_is_held_in_reserve_while_the_special_alone_returns() {
    let mut hold = Ticking::default();
    let first = hold.step(held(JUMP, u8::MAX, true, 0x8004));
    assert_eq!((first.press, first.release), (0, JUMP));
    assert!(hold.waiting());
    for _ in 0..11 {
        assert_eq!(
            hold.step(held(JUMP, u8::MAX, true, 0x8000)),
            Correction::default()
        );
    }
    let spent = hold.step(held(JUMP, u8::MAX, false, 0x8000));
    assert_eq!(
        spent.press, JUMP,
        "the special-only reach ends: jump now"
    );
    assert!(!hold.waiting());

    let mut unguarded = Ticking::default();
    assert_eq!(
        unguarded.step(held(JUMP, 0, true, 0x8004)),
        Correction::default(),
        "no reserve without a guarder off-stage"
    );
    assert!(!unguarded.waiting());
}

#[test]
fn bowser_jr_held_jump_brakes_before_the_near_ledge() {
    // Exact de242a91 attempt 9, at the first R2 hold and two thinks later.
    // Full inward drift took Jr. under FD before he could recover.
    let air = AirPhysics {
        horizontal_speed: 1.134,
        horizontal_acceleration: 0.08,
        gravity: 0.092,
        terminal_speed: 1.65,
        jump_speed: 2.47,
    };
    let reach = Reach {
        air,
        jump: true,
        special: Some(SpecialReach { horizontal: 42.0, rise: 65.0 }),
    };
    let velocity = [1.134, -1.65];
    let ledge = [-79.9856, 0.00026];
    let initial = [-90.1836, -56.0758];
    let brake = [-87.9156, -59.3758];
    let past_braking_point = [-84.5136, -64.3258];
    assert_eq!(reach.hold_stick(initial, velocity, ledge), Some(1.0));
    assert_eq!(reach.hold_stick(brake, velocity, ledge), Some(-1.0));
    assert_eq!(reach.hold_stick(past_braking_point, velocity, ledge), None);
    assert!(!reach.returns(past_braking_point, velocity, ledge, 1, -145.0));
    assert_eq!(reach.hold_stick([87.9156, -59.3758], [-1.134, -1.65], [79.9856, ledge[1]]), Some(1.0));
}


#[test]
fn bowser_jr_keeps_the_stock_up_special_while_delaying_the_jump() {
    // Exact d1889000: attempts 88154/89842 died after R2 withheld 0x8802.
    // At matched Level 9 release 102193 the same pad reached the ledge in 45f.
    let mut hold = Ticking::default();
    hold.step(held(JUMP, 20, true, 0x8000));
    assert_eq!(hold.step(held(JUMP, 20, true, 0x8802)), Correction::default());
    assert!(!hold.waiting(), "an up special never leaves a jump owed");
    assert_eq!(hold.step(held(JUMP, 20, false, 0x8000)), Correction::default());
}

#[test]
fn a_stock_up_special_settles_any_withheld_jump_on_that_frame() {
    let mut hold = Ticking::default();
    assert_eq!(hold.step(held(JUMP, 20, true, 0x8004)).release, JUMP);
    assert!(hold.waiting());
    assert_eq!(hold.step(held(JUMP, 20, false, 0x8802)), Correction::default());
    assert!(!hold.waiting());
    assert_eq!(hold.step(held(JUMP, 20, false, 0x8000)), Correction::default());
}

#[test]
fn stock_up_special_is_held_until_the_sweet_spot_frame() {
    let mut hold = Ticking::default();
    for frame in 0..5 {
        let edit = hold.step(held(SPECIAL, 5, true, 0x8002));
        assert_eq!((edit.press, edit.release), (0, SPECIAL), "frame {frame}");
    }
    assert_eq!(hold.step(held(SPECIAL, 5, true, 0x8000)).press, SPECIAL);
    assert!(!hold.waiting());

    let mut last_frame = Ticking::default();
    last_frame.step(held(SPECIAL, 30, true, 0x8002));
    assert_eq!(
        last_frame.step(held(SPECIAL, 30, false, 0x8002)).press,
        SPECIAL,
        "the envelope ends next frame"
    );

    let mut unguarded = Ticking::default();
    assert_eq!(
        unguarded.step(held(SPECIAL, 0, true, 0x8002)),
        Correction::default()
    );

    let mut defended = Ticking::default();
    defended.step(held(SPECIAL, 30, true, 0x8002));
    assert_eq!(
        defended.step(held(SPECIAL, 30, true, 0x8000 | GUARD)),
        Correction::default(),
        "a stock air dodge passes and ends the hold"
    );
    assert!(!defended.waiting());
}

#[test]
fn hot_ledge_delays_the_arrival_only_inside_the_deadline() {
    let facts = |can_wait, hot| HoldFacts {
        frame: 0,
        buttons: 0x8000,
        button: JUMP,
        jumps: 1,
        planned: 1,
        can_wait,
        hot,
    };
    let mut hold = Ticking::default();
    hold.step(held(JUMP, 1, true, 0x8004));
    assert_eq!(
        hold.step(facts(true, true)),
        Correction::default(),
        "wait out the hit"
    );
    assert!(hold.waiting());
    assert_eq!(
        hold.step(facts(false, true)).press,
        JUMP,
        "no frame left to wait"
    );

    let mut watch = LedgeWatch::default();
    assert!(!watch.hot(None, false, 99, 12));
    for frame in 100..112 {
        assert!(
            !watch.hot(Some(1), false, frame, 12),
            "inside the reaction time the attack is not seen: {frame}"
        );
    }
    assert!(watch.hot(Some(1), false, 112, 12), "start-up: the hit is coming");
    assert!(watch.hot(Some(1), true, 115, 12), "active");
    assert!(
        !watch.hot(Some(1), false, 118, 12),
        "end lag is the time to arrive"
    );
    assert!(!watch.hot(Some(2), false, 130, 12), "a new attack restarts the clock");
    assert!(watch.hot(Some(2), false, 130, 0), "frame-perfect sees it at once");
}

#[test]
fn an_aimed_special_shares_its_travel_with_the_climb() {
    let special = SpecialReach {
        horizontal: 60.0,
        rise: 40.0,
    };
    assert_eq!(level_run(special, 0.0), 60.0);
    assert_eq!(level_run(special, 20.0), 30.0);
    assert_eq!(level_run(special, 45.0), 0.0);
    assert_eq!(level_run(special, -10.0), 60.0);
    assert_eq!(level_run(special, f32::NAN), 0.0);
    let strong = niche_plan(Route::Opposite, 1.0, [75.0, 0.0]);
    let weak = niche_plan(Route::Opposite, 0.0, [75.0, 0.0]);
    assert_eq!((strong.route, strong.waypoints[0]), (Route::Opposite, [75.0, 0.0]));
    let rolled =
        |row| shares([None, None, plan(Route::LateLow), Some(row)], Zone::Ledge, None)[3];
    assert!(
        rolled(strong) > rolled(weak) && rolled(weak) > 0,
        "a thin margin is rolled less often, never dropped"
    );
}

#[test]
fn extension_is_used_only_for_a_horizontal_shortfall() {
    let slow = AirPhysics {
        horizontal_speed: 1.0,
        horizontal_acceleration: 0.05,
        gravity: 0.11,
        terminal_speed: 1.8,
        jump_speed: 2.6,
    };
    let reach = Reach {
        air: slow,
        jump: true,
        special: Some(SpecialReach {
            horizontal: 8.0,
            rise: 25.0,
        }),
    };
    let fall = [0.0, -1.0];
    assert!(
        !short(reach, [-82.0, -10.0], fall, LEDGE, -140.0),
        "under the ledge the plain route returns"
    );
    assert!(
        short(reach, [-150.0, 0.0], fall, LEDGE, -140.0),
        "far out and level: distance is missing"
    );
    assert!(
        !short(reach, [-150.0, -120.0], fall, LEDGE, -140.0),
        "too deep: a level extension does not help"
    );
}

const FLOATY: AirPhysics = AirPhysics {
    horizontal_speed: 1.3,
    horizontal_acceleration: 0.08,
    gravity: 0.064,
    terminal_speed: 1.0,
    jump_speed: 1.6,
};
const HEAVY: AirPhysics = AirPhysics {
    horizontal_speed: 1.0,
    horizontal_acceleration: 0.05,
    gravity: 0.1,
    terminal_speed: 1.9,
    jump_speed: 2.6,
};
const LONG: Crossing = Crossing {
    span: 170.0,
    band: [-90.0, -45.0],
    foot: -50.0,
    ledge: 0.0,
};

#[test]
fn crossing_budget_refuses_a_span_beyond_glide_and_special() {
    let special = Some(SpecialReach {
        horizontal: 60.0,
        rise: 70.0,
    });
    let none = Measured::default();
    assert_eq!(crossing_margin(HEAVY, 1, special, true, none, LONG), None);
    let side = Measured {
        run: 120.0,
        rise: 0.0,
    };
    assert!(
        crossing_margin(HEAVY, 1, special, true, side, LONG).is_some(),
        "a measured side special closes the gap"
    );
    assert_eq!(
        crossing_margin(HEAVY, 1, None, true, side, LONG),
        None,
        "nothing left to climb with"
    );
    let deep = Crossing {
        span: 170.0,
        band: [-130.0, -70.0],
        foot: -75.0,
        ledge: 0.0,
    };
    assert_eq!(
        crossing_margin(FLOATY, 5, None, false, none, deep),
        None,
        "jumps alone do not cross and climb a long deep stage"
    );
    for band in [[-45.0, -90.0], [f32::NAN, -45.0]] {
        assert_eq!(
            crossing_margin(HEAVY, 1, special, true, side, Crossing { band, ..LONG }),
            None
        );
    }
    assert_eq!(
        crossing_margin(
            HEAVY,
            1,
            special,
            true,
            side,
            Crossing {
                foot: -95.0,
                ..LONG
            }
        ),
        None,
        "already below the band"
    );
}

#[test]
fn crossing_budget_accepts_a_floaty_multi_jumper_on_a_short_stage() {
    let short_stage = Crossing {
        span: 100.0,
        band: [-60.0, -30.0],
        foot: -35.0,
        ledge: 0.0,
    };
    let margin = crossing_margin(FLOATY, 5, None, false, Measured::default(), short_stage)
        .expect("glide and jumps cross");
    assert!((0.0..=1.0).contains(&margin));
    let fewer = crossing_margin(FLOATY, 3, None, false, Measured::default(), short_stage);
    assert!(
        fewer.is_none_or(|less| less < margin),
        "fewer jumps leave less slack"
    );
    let lifted = crossing_margin(
        FLOATY,
        3,
        Some(SpecialReach {
            horizontal: 0.0,
            rise: 60.0,
        }),
        false,
        Measured::default(),
        short_stage,
    );
    assert!(
        lifted.is_some(),
        "an unaimed up special is lift at the exit"
    );
}

#[test]
fn edge_di_points_at_the_stage_unless_a_blast_zone_is_closer() {
    let blast = [-240.0, 240.0, 180.0, -140.0];
    assert_eq!(
        edge_di([-120.0, -10.0], LEDGE, [-1.0, 0.8], 10.0, blast),
        Some([1.0, 0.0]),
        "toward the stage, wherever the guarder is"
    );
    assert_eq!(
        edge_di([120.0, -10.0], [70.0, 0.0], [1.0, 0.8], 10.0, blast),
        Some([-1.0, 0.0])
    );
    assert_eq!(
        edge_di([-225.0, 0.0], LEDGE, [-3.0, 0.5], 8.0, blast),
        Some([1.0, 0.0]),
        "survive the side blast zone"
    );
    assert_eq!(
        edge_di([-120.0, 170.0], LEDGE, [0.2, 3.0], 8.0, blast),
        Some([0.0, -1.0]),
        "survive the top blast zone first"
    );
    assert_eq!(
        edge_di([f32::NAN, 0.0], LEDGE, [1.0, 1.0], 8.0, blast),
        None
    );
}

#[test]
fn reach_can_rise_to_the_ledge_after_braking_outside_its_column() {
    let reach = Reach { air: AIR, jump: true, special: None };
    let from = [LEDGE[0] - 4.0, -12.0];
    let speed = [1.0, 0.0];
    assert_eq!(reach.hold_stick(from, speed, LEDGE), Some(-1.0));
    assert!(reach.returns(from, speed, LEDGE, 0, BOTTOM));
    assert!(reach.returns(from, speed, LEDGE, 3, BOTTOM));
    assert!(!short(reach, from, speed, LEDGE, BOTTOM));
    assert!(!Reach { jump: false, ..reach }.returns(from, speed, LEDGE, 0, BOTTOM));

    // At 1.0 units out, the same speed takes 2.85 units to stop.
    assert!(!reach.returns([LEDGE[0] - 1.0, from[1]], speed, LEDGE, 0, BOTTOM));
}

#[test]
fn an_enabled_jump_transition_does_not_hide_an_exhausted_jump_budget() {
    use super::recovery_jump_available;
    assert!(recovery_jump_available(Some(1), true));
    assert!(!recovery_jump_available(Some(1), false));
    assert!(!recovery_jump_available(Some(0), true));
    assert!(!recovery_jump_available(None, true));
}


#[test]
fn measured_crossing_reads_the_race_to_the_far_ledge() {
    use super::CrossingRace;
    let race = CrossingRace {
        near_x: -80.0,
        far_x: 80.0,
        guarder_x: -79.0,
        guarder_run_speed: 1.5,
        guarder_facing: 1.0,
        crossing_frames: 158,
    };
    assert_eq!(race.crossing_share(Zone::Ledge), Some(6));
    assert_eq!(race.crossing_share(Zone::Hang), Some(20));
    assert_eq!(race.crossing_share(Zone::OffLow), Some(45));
    assert_eq!(CrossingRace { guarder_run_speed: 0.0, ..race }.crossing_share(Zone::Hang), None);
    assert_eq!(CrossingRace { guarder_x: f32::NAN, ..race }.crossing_share(Zone::Hang), None);
    let mirrored = CrossingRace {
        near_x: -race.near_x,
        far_x: -race.far_x,
        guarder_x: -race.guarder_x,
        guarder_facing: -race.guarder_facing,
        ..race
    };
    assert_eq!(mirrored.crossing_share(Zone::Ledge), race.crossing_share(Zone::Ledge));
    let turning = CrossingRace { guarder_run_speed: 1.30, ..race };
    assert_eq!(turning.crossing_share(Zone::Ledge), Some(6));
    assert_eq!(CrossingRace { guarder_facing: -1.0, ..turning }.crossing_share(Zone::Ledge), Some(20));
}

#[test]
fn measured_crossing_is_a_mixup_with_or_without_modeled_alternatives() {
    use super::{mix_measured_crossing, CrossingRace};
    let race = CrossingRace {
        near_x: -80.0,
        far_x: 80.0,
        guarder_x: -79.0,
        guarder_run_speed: 1.5,
        guarder_facing: 1.0,
        crossing_frames: 158,
    };
    let options = ROUTES.map(plan);
    let route = |draw, last| {
        mix_measured_crossing(options, Zone::Ledge, last, draw, race).unwrap().route
    };
    let percent = |p: u64| ((u64::from(u32::MAX) * p) / 100) as u32;
    assert_ne!(route(percent(92), None), Route::Opposite);
    assert_eq!(route(percent(95), None), Route::Opposite);
    assert_ne!(route(percent(95), Some(Route::Opposite)), Route::Opposite);
    assert_eq!(route(percent(99), Some(Route::Opposite)), Route::Opposite);
    assert_eq!(
        mix_measured_crossing(only(Route::Opposite), Zone::Hang, None, 0, race),
        None,
    );
    assert_eq!(
        mix_measured_crossing(only(Route::Opposite), Zone::Hang, None, u32::MAX, race).unwrap().route,
        Route::Opposite,
    );
    let invalid = CrossingRace { guarder_run_speed: f32::NAN, ..race };
    assert_ne!(
        mix_measured_crossing(options, Zone::Ledge, None, u32::MAX, invalid).unwrap().route,
        Route::Opposite,
    );
}

#[test]
fn seeded_crossing_roll_stays_a_minority_even_with_a_high_spare_climb() {
    use crate::model::cpu::rng::Rng;

    let [near, _, _, opposite] = ROUTES.map(plan);
    let near = near.map(|p| Plan { weight: 1, ..p });
    let opposite = opposite.map(|p| Plan { weight: 16, ..p });
    let mut rng = Rng::new(0x7352_D194);
    let mut crossings = 0;
    for _ in 0..10_000 {
        let draw = rng.next_u32();
        let chosen = mix([near, None, None, opposite], Zone::Ledge, None, draw).unwrap();
        assert_eq!(mix([near, None, None, None], Zone::Ledge, None, draw), near);
        crossings += u32::from(chosen.route == Route::Opposite);
    }
    assert!((4300..=4700).contains(&crossings), "crossing cap is 45%: {crossings}/10000");
}

#[test]
fn seeded_measured_roll_keeps_the_uncovered_normal_route_and_respects_live_reach() {
    use super::{mix_measured_crossing, CrossingRace};
    use crate::model::cpu::rng::Rng;

    let race = CrossingRace {
        near_x: -80.0, far_x: 80.0, guarder_x: 65.0, guarder_run_speed: 1.5,
        guarder_facing: -1.0, crossing_frames: 158,
    };
    let mut rng = Rng::new(0x7483_A5D6);
    let mut counts = [0u32; 3];
    let [near, high, low, opposite] = ROUTES.map(plan);
    for _ in 0..10_000 {
        let draw = rng.next_u32();
        for (i, (zone, guarder_x, cross)) in [
            (Zone::Far, 65.0, opposite),
            (Zone::Hang, -79.0, opposite),
            (Zone::Hang, -79.0, None),
        ].into_iter().enumerate() {
            let chosen = mix_measured_crossing(
                [near, high, low, cross], zone, None, draw, CrossingRace { guarder_x, ..race },
            ).unwrap();
            counts[i] += u32::from(chosen.route == Route::Opposite);
        }
    }
    assert_eq!(counts[0], 0, "leave the uncovered normal ledge to the fighter");
    assert!((1850..=2150).contains(&counts[1]), "covered ledge and close race: {counts:?}");
    assert_eq!(counts[2], 0, "a live reach veto cannot enter the roll");
}
