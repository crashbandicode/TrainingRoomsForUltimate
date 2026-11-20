use super::{
    crossing_margin, edge_di, hot, mix, planned_delay, short, zone, Crossing, Guarder, Hold,
    HoldFacts, Measured, Reach, Zone,
};
use crate::model::edge_juggle::perfect::{
    AirPhysics, Correction, Plan, Point, Route, SpecialReach, GUARD, JUMP, SPECIAL,
};

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
fn mix_keeps_the_only_feasible_route_whatever_the_zone() {
    assert_eq!(shares(only(Route::Near), Zone::Hang, None), [1000, 0, 0, 0]);
    assert_eq!(
        shares(only(Route::Opposite), Zone::Far, None),
        [0, 0, 0, 1000]
    );
    assert_eq!(shares(only(Route::High), Zone::OffHigh, None), [0, 1000, 0, 0]);
    assert_eq!(mix([None; 4], Zone::Ledge, None, 7), None);
}

#[test]
fn reach_counts_jump_and_up_special_together() {
    let from = [-95.0, -50.0];
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
        None,
        "too deep: act now"
    );
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
        buttons,
        button,
        planned,
        can_wait,
        hot: false,
    }
}

#[test]
fn jump_is_held_in_reserve_while_the_special_alone_returns() {
    let mut hold = Hold::default();
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

    let mut unguarded = Hold::default();
    assert_eq!(
        unguarded.step(held(JUMP, 0, true, 0x8004)),
        Correction::default(),
        "no reserve without a guarder off-stage"
    );
    assert!(!unguarded.waiting());
}

#[test]
fn stock_up_special_is_held_until_the_sweet_spot_frame() {
    let mut with_jump = Hold::default();
    with_jump.step(held(JUMP, 20, true, 0x8004));
    let both = with_jump.step(held(JUMP, 20, true, 0x8002));
    assert_eq!(
        (both.press, both.release),
        (0, SPECIAL),
        "no up special while the jump is unspent"
    );

    let mut hold = Hold::default();
    for frame in 0..5 {
        let edit = hold.step(held(SPECIAL, 5, true, 0x8002));
        assert_eq!((edit.press, edit.release), (0, SPECIAL), "frame {frame}");
    }
    assert_eq!(hold.step(held(SPECIAL, 5, true, 0x8000)).press, SPECIAL);
    assert!(!hold.waiting());

    let mut last_frame = Hold::default();
    last_frame.step(held(SPECIAL, 30, true, 0x8002));
    assert_eq!(
        last_frame.step(held(SPECIAL, 30, false, 0x8002)).press,
        SPECIAL,
        "the envelope ends next frame"
    );

    let mut unguarded = Hold::default();
    assert_eq!(
        unguarded.step(held(SPECIAL, 0, true, 0x8002)),
        Correction::default()
    );

    let mut defended = Hold::default();
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
        buttons: 0x8000,
        button: JUMP,
        planned: 1,
        can_wait,
        hot,
    };
    let mut hold = Hold::default();
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

    assert!(hot(Some(12), 12, true));
    assert!(
        !hot(Some(11), 12, true),
        "inside the reaction time the attack is not seen"
    );
    assert!(!hot(Some(40), 12, false));
    assert!(!hot(None, 0, true));
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
