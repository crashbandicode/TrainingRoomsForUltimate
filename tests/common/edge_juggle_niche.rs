use super::{
    select, Budget, Facts, Motion, Move, NearReach, Sample, Sequence, CANDIDATE_ROWS, EXTENSION_ROWS,
    ROWS,
};
use crate::model::edge_juggle::perfect::recovery::Crossing;
use crate::model::edge_juggle::perfect::{self, AirPhysics, Route, SpecialReach};

fn tested_row(id: u8) -> &'static super::Row {
    super::measured_row(id, CANDIDATE_ROWS).expect("row in rc.15 registry")
}

#[test]
fn niche_roster_kinds_match_the_approved_plan() {
    use crate::model::fighter_choices::CHOICES;

    // NICHE-PLAN §2 and NICHE-PLAN-ADDENDUM §C in candidate row ID order.
    let fighters = [
        "Pikachu",
        "Pikachu",
        "Pikachu",
        "Mr. Game & Watch",
        "Diddy Kong",
        "Diddy Kong",
        "R.O.B.",
        "Sora",
        "Little Mac",
        "Little Mac",
        "Sheik",
        "Sheik",
        "Greninja",
        "Ness",
        "Steve",
        "Pichu",
        "Jigglypuff",
        "Meta Knight",
        "Pit",
        "Dark Pit",
        "PAC-MAN",
        "Bayonetta",
        "Sonic",
        "Peach",
        "Daisy",
        "Snake",
        "Link",
        "Lucas",
        "Lucario",
        "Olimar",
        "Bowser Jr.",
        "Hero",
        "Yoshi",
        "Luigi",
        "Charizard",
        "Incineroar",
        "Incineroar",
        "Pyra",
        "Mythra",
        "Mythra",
        "Pikachu",
        "Mr. Game & Watch",
        "Pichu",
        "Link",
    ];
    assert_eq!(super::ROWS.len(), fighters.len());
    for (index, (row, fighter)) in super::ROWS.iter().zip(fighters).enumerate() {
        assert_eq!(usize::from(row.id), index + 1);
        let kind = CHOICES
            .iter()
            .find_map(|&(kind, name)| (name == fighter).then_some(kind))
            .expect("fighter from the plan must be in the roster");
        assert_eq!(row.fighter as u32, kind, "row {} belongs to {fighter}", row.id);
    }
}

#[test]
fn ness_pk_thunder_has_a_row_without_joining_the_unmeasured_extension_pool() {
    let row = tested_row(14);
    assert_eq!(row.moves, [Move::Hold, Move::Bolt]);
    assert_eq!(row.route, Route::Opposite);
    assert!(select(facts(0x0a, "cross"), EXTENSION_ROWS).is_none());
    assert_eq!(tested_row(28).fighter, 0x28, "Lucas has its own PK Thunder row");
}

/// A slow faller with five jumps and a strong up special: every row's crossing fits.
const ROOMY: Budget = Budget {
    air: AirPhysics {
        horizontal_speed: 1.3,
        horizontal_acceleration: 0.08,
        gravity: 0.064,
        terminal_speed: 1.0,
        jump_speed: 1.6,
    },
    jumps: 5,
    special: Some(SpecialReach {
        horizontal: 60.0,
        rise: 90.0,
    }),
    crossing: Crossing {
        span: 100.0,
        band: [-135.0, -39.0],
        foot: -45.0,
        ledge: 0.0,
    },
};

/// A fast faller with one jump on a long stage: no row's crossing fits.
const TIGHT: Budget = Budget {
    air: AirPhysics {
        horizontal_speed: 1.0,
        horizontal_acceleration: 0.05,
        gravity: 0.1,
        terminal_speed: 1.9,
        jump_speed: 2.6,
    },
    jumps: 1,
    special: Some(SpecialReach {
        horizontal: 60.0,
        rise: 70.0,
    }),
    crossing: Crossing {
        span: 170.0,
        band: [-90.0, -45.0],
        foot: -50.0,
        ledge: 0.0,
    },
};

fn facts(fighter: i32, mode: &str) -> Facts {
    Facts {
        fighter,
        corridor: Some([-135.0, -39.0]),
        covered_ledge: matches!(mode, "cross" | "long"),
        player_over_crossing: mode == "center",
        long_approach: mode == "long",
        counter_contact: mode == "counter",
        near: Some(NearReach {
            gap: if mode == "long" { 45.0 } else { 10.0 },
            climb: if mode == "long" { 0.0 } else { 20.0 },
            body: 8.0,
            special: perfect::SpecialReach { horizontal: 14.0, rise: 38.0 },
            ledge_covered: false,
        }),
        short: mode == "near",
        climb: 0.0,
        budget: Some(ROOMY),
    }
}

#[test]
fn hard_niche_crossings_need_a_measured_route_even_with_spare_budget() {
    for (fighter, row, name) in [(0x22, 26, "Snake"), (0x26, 35, "Charizard")] {
        let row = super::measured_row(row, CANDIDATE_ROWS).expect("registered candidate row");
        let input = facts(fighter, "cross");
        assert_eq!(row.route, Route::Opposite, "{name}'s route can be tested by a dedicated model");
        assert!(row.margin(input).is_some(), "{name}: synthetic budget permits a crossing");
        assert!(select(input, CANDIDATE_ROWS).is_none(), "{name}: no estimated crossing");
    }
}

#[test]
fn little_mac_and_incineroar_only_have_near_niche_rows() {
    use crate::model::fighter_choices::CHOICES;

    for (kind, name) in CHOICES.iter().copied().filter(|&(_, name)| matches!(name, "Little Mac" | "Incineroar")) {
        let mut rows = super::ROWS.iter().filter(|row| row.fighter == kind as i32).peekable();
        assert!(rows.peek().is_some(), "{name} needs a non-crossing technique");
        assert!(rows.all(|row| row.route == Route::Near), "{name}");
    }
    assert_eq!(select(facts(0x34, "counter"), CANDIDATE_ROWS).map(|row| row.id), Some(10));
    let mut whip = facts(0x47, "near");
    let near = whip.near.as_mut().unwrap();
    near.gap = 20.0;
    near.climb = 5.0;
    assert_eq!(select(whip, CANDIDATE_ROWS).map(|row| row.id), Some(36));
}

fn sample(frame: u32, motion: Motion) -> Sample {
    Sample {
        frame,
        motion,
        can_start: true,
        valid: true,
        at_exit: true,
        can_enter: true,
        low: false,
        hold_over: true,
        direction: 1.0,
        aim: [0.6, 0.8],
    }
}

/// A glide frame under the stage, away from the exit.
fn gliding(frame: u32, motion: Motion, low: bool) -> Sample {
    Sample {
        at_exit: false,
        hold_over: false,
        low,
        ..sample(frame, motion)
    }
}

fn verify(fighter: i32, mode: &str, id: u8, steps: &[Move]) {
    let input = facts(fighter, mode);
    let row = select(input, CANDIDATE_ROWS).expect("eligible candidate row");
    assert_eq!(row.id, id);
    let route = if matches!(mode, "near" | "counter") {
        Route::Near
    } else {
        Route::Opposite
    };
    assert_eq!(row.route, route);
    assert!(
        select(input, 0).is_none(),
        "disabled rows cannot be selected"
    );
    if route == Route::Opposite {
        let deep_stage = Facts {
            corridor: None,
            ..input
        };
        assert!(
            select(deep_stage, 1 << (id - 1)).is_none(),
            "the same row must decline a closed crossing"
        );
        for budget in [Some(TIGHT), None] {
            assert!(
                select(Facts { budget, ..input }, 1 << (id - 1)).is_none(),
                "the same row must decline a crossing its budget does not cover"
            );
        }
    }

    verify_pad_sequence(row, steps);
}

fn verify_pad_sequence(row: &'static super::Row, steps: &[Move]) {
    let mut sequence = Sequence::new(row);
    for (idx, &movement) in steps.iter().enumerate() {
        let frame = idx as u32 * 7;
        assert_eq!(sequence.action(), Some(movement));
        let pad = sequence
            .step(sample(frame, Motion::Free))
            .expect("first real input");
        if matches!(movement, Move::Drift | Move::Hold) {
            assert_eq!((pad.stick, pad.press), (Some([1.0, 0.0]), 0));
            continue;
        }
        let expected = match movement {
            Move::Jump => ([1.0, 0.0], perfect::JUMP),
            Move::Float => ([0.0, -1.0], perfect::JUMP),
            Move::Side | Move::ChargedSide | Move::Missile => {
                ([1.0, 0.0], perfect::SPECIAL)
            }
            Move::Down | Move::Counter | Move::Bomb | Move::Swap => ([0.0, -1.0], perfect::SPECIAL),
            _ => ([0.0, 1.0], perfect::SPECIAL),
        };
        assert_eq!((pad.stick, pad.press), (Some(expected.0), expected.1));
        assert_eq!(
            sequence.step(sample(frame, Motion::Free)),
            Some(pad),
            "duplicate think uses the same pad"
        );
        let start = sequence
            .step(sample(frame + 1, Motion::Starting))
            .expect("observed native startup");
        let charge = matches!(movement, Move::ChargedSide | Move::Barrel | Move::Missile);
        let hold = charge || movement == Move::Float;
        assert_eq!(start.press, if hold { expected.1 } else { 0 });
        sequence
            .step(sample(frame + 2, Motion::Travel))
            .expect("observed travel");
        let finish = sequence
            .step(sample(frame + 3, Motion::Finish))
            .expect("explicit release");
        assert_eq!(finish.press, 0);
        assert!(finish.release & expected.1 != 0);
        assert_eq!(
            sequence.action(),
            Some(movement),
            "native end phase still belongs to this move"
        );
        assert_eq!(
            sequence.step(sample(frame + 4, Motion::Finish)),
            Some(finish)
        );
        let resumed = sequence
            .step(sample(frame + 5, Motion::Free))
            .expect("native actionable frame");
        assert_eq!(resumed.press, 0, "advance with a released pad");
    }
    assert_eq!(sequence.action(), None);
}

#[test]
fn niche_pikachu_quick_attack_route_and_pad_sequence() {
    verify(0x8, "cross", 1, &[Move::Hold, Move::QuickAttack]);
}

#[test]
fn niche_quick_attack_aims_during_the_pause_before_the_second_warp() {
    let mut sequence = Sequence::new(select(facts(0x8, "cross"), CANDIDATE_ROWS).unwrap());
    sequence.step(sample(0, Motion::Free)).unwrap(); // Hold ends at the exit.
    sequence.step(sample(1, Motion::Free)).unwrap();
    let start = sequence.step(sample(2, Motion::Starting)).unwrap();
    assert_eq!(start.stick, Some([1.0, 0.0]));
    sequence.step(sample(3, Motion::Travel)).unwrap();

    // 0x1e2 (SPECIAL_HI_END) separates the two warps.
    let aim = [-0.1, 0.99];
    let pause = Sample {
        direction: -1.0,
        aim,
        ..sample(4, Motion::Finish)
    };
    let second = sequence.step(pause).unwrap();
    assert_eq!(second.stick, Some(aim));
    assert_eq!(second.press, 0, "the second warp takes the stick in this window");
}

#[test]
fn niche_pikachu_thunder_follows_skull_bash_and_awaits_its_measured_finish() {
    let thunder = tested_row(2);
    assert_eq!(thunder.moves, &[Move::Side, Move::Down, Move::QuickAttack]);
    assert!(
        select(facts(0x8, "center"), CANDIDATE_ROWS).is_none(),
        "The cancel, bolt hit and far ledge need a native witness"
    );
}

#[test]
fn niche_pikachu_skull_bash_route_and_pad_sequence() {
    verify(0x8, "long", 3, &[Move::Side, Move::Jump, Move::QuickAttack]);
}

#[test]
fn niche_game_watch_fire_route_and_pad_sequence() {
    verify_pad_sequence(super::measured_row(4, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::Rise]);
}

#[test]
fn niche_diddy_rocketbarrel_route_and_pad_sequence() {
    verify(0x27, "cross", 5, &[Move::Hold, Move::Barrel]);
}

#[test]
fn niche_diddy_monkey_flip_route_and_pad_sequence() {
    verify(0x27, "long", 6, &[Move::Side, Move::Hold, Move::Barrel]);
}

#[test]
fn niche_rob_burner_route_and_pad_sequence() {
    verify(0x2d, "cross", 7, &[Move::Hold, Move::Hover]);
}

#[test]
fn niche_sora_sonic_blade_route_and_pad_sequence() {
    // Sonic Blade ends in special fall (SmashWiki "Helpless"): the glide, then Aerial Sweep into the Blade.
    verify_pad_sequence(super::measured_row(8, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::SoraChain]);
}

#[test]
fn niche_sora_opens_with_aerial_sweep_and_presses_once_per_blade_window() {
    let row = super::measured_row(8, CANDIDATE_ROWS).unwrap();
    let mut sequence = Sequence::new(row);
    sequence.step(sample(0, Motion::Free)).expect("the glide ends at the exit");
    let frames = [
        (Motion::Free, [0.0, 1.0], perfect::SPECIAL),
        (Motion::Starting, [1.0, 0.0], 0),
        (Motion::AimAgain, [1.0, 0.0], perfect::SPECIAL),
        (Motion::AimAgain, [1.0, 0.0], 0),
        (Motion::Travel, [1.0, 0.0], 0),
        (Motion::AimAgain, [1.0, 0.0], perfect::SPECIAL),
    ];
    for (frame, (motion, stick, press)) in frames.into_iter().enumerate() {
        let input = sequence.step(sample(frame as u32 + 1, motion)).unwrap();
        assert_eq!((input.stick, input.press), (Some(stick), press), "frame {frame}");
    }
}

#[test]
fn niche_little_mac_haymaker_route_and_pad_sequence() {
    verify(0x34, "near", 9, &[Move::Side, Move::Rise]);
}

#[test]
fn niche_haymaker_does_not_spend_side_special_when_the_ledge_is_below_its_safe_approach() {
    let input = facts(0x34, "near");
    let check = |gap, climb| {
        select(
            Facts {
                climb,
                near: input.near.map(|near| NearReach { gap, climb, ..near }),
                ..input
            },
            CANDIDATE_ROWS,
        )
        .map(|row| row.id)
    };
    // 3a29e6af fd-3 made the ledge from this release.
    assert_eq!(check(20.0, 28.0), Some(9));
    // LM-1 asks for an extension at this same-stage gap and height.
    assert_eq!(check(35.0, 10.0), Some(9));
    // fd-1 was still at y=-73 as it reached the ledge column, then fell past reach.
    assert_eq!(check(56.0, 58.0), None);
}

#[test]
fn niche_haymaker_can_take_the_proven_approach_to_a_covered_ledge() {
    let input = facts(0x34, "near");
    let check = |gap, climb, ledge_covered| {
        select(
            Facts {
                short: false,
                climb,
                near: input.near.map(|near| NearReach { gap, climb, ledge_covered, ..near }),
                ..input
            },
            CANDIDATE_ROWS,
        )
        .map(|row| row.id)
    };
    // fd-3: (-99.993, -27.595), ledge near x=-81. P1 guarded from the inside.
    assert_eq!(check(19.0, 27.595, true), Some(9));
    assert_eq!(check(19.0, 27.595, false), None);
    assert_eq!(check(19.0, -20.0, true), None);
    // fd-1: outside Haymaker's measured useful arc even if P1 is now nearby.
    assert_eq!(check(56.0, 58.0, true), None);
    assert_eq!(check(25.0, 58.0, true), None);
}

#[test]
fn niche_little_mac_counter_route_and_pad_sequence() {
    verify(0x34, "counter", 10, &[Move::Counter, Move::Rise]);
}

#[test]
fn niche_sheik_bouncing_fish_route_and_pad_sequence() {
    verify(0x10, "near", 11, &[Move::Down, Move::Rise]);
}

#[test]
fn niche_sheik_vanish_route_and_pad_sequence() {
    verify(0x10, "cross", 12, &[Move::Down, Move::Jump, Move::Travel]);
}

#[test]
fn niche_greninja_shadow_sneak_route_and_pad_sequence() {
    verify(
        0x35,
        "cross",
        13,
        &[Move::ChargedSide, Move::Hold, Move::Travel],
    );
}

#[test]
fn niche_steve_elytra_route_and_pad_sequence() {
    verify_pad_sequence(super::measured_row(15, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::Glide]);
}

#[test]
fn niche_pichu_quick_attack_route_and_pad_sequence() {
    verify(0x13, "cross", 16, &[Move::Side, Move::Jump, Move::QuickAttack]);
}

#[test]
fn niche_jigglypuff_can_cross_when_the_snapshot_reports_no_up_special() {
    let jumps_only = Facts {
        near: None,
        budget: Some(Budget { special: None, ..ROOMY }),
        ..facts(0xc, "cross")
    };
    assert_eq!(select(jumps_only, CANDIDATE_ROWS).map(|row| row.id), Some(17));
    assert!(
        select(Facts { fighter: 0x2d, ..jumps_only }, CANDIDATE_ROWS).is_none(),
        "the same jumps cannot admit a row that requires a native burner"
    );
}

#[test]
fn niche_jigglypuff_jumps_route_and_pad_sequence() {
    verify(0xc, "cross", 17, &[Move::Hold]);
}

#[test]
fn niche_meta_knight_shuttle_loop_route_and_pad_sequence() {
    verify(0x1d, "cross", 18, &[Move::Hold, Move::Rise]);
}

#[test]
fn niche_pit_power_of_flight_route_and_pad_sequence() {
    verify(0x1e, "cross", 19, &[Move::Hold, Move::Travel]);
}

#[test]
fn niche_dark_pit_power_of_flight_route_and_pad_sequence() {
    verify(0x1f, "cross", 20, &[Move::Hold, Move::Travel]);
}

#[test]
fn niche_pac_man_pac_jump_route_and_pad_sequence() {
    verify(0x37, "cross", 21, &[Move::Side, Move::Jump, Move::Rise]);
}

#[test]
fn niche_bayonetta_twists_once_per_jump() {
    // One Witch Twist per jump (SmashWiki "Bayonetta (SSBU)/Up special"): the double jump sits between the two.
    let twists = [
        Move::Hold,
        Move::Travel,
        Move::Jump,
        Move::Travel,
        Move::Side,
    ];
    verify(0x40, "cross", 22, &twists);
    assert!(tested_row(22)
        .moves
        .windows(2)
        .all(|pair| pair != [Move::Travel, Move::Travel] && pair != [Move::Side, Move::Travel]));
}

#[test]
fn niche_sonic_spring_jump_route_and_pad_sequence() {
    verify(0x29, "cross", 23, &[Move::Side, Move::Hold, Move::Rise]);
}

#[test]
fn niche_peach_float_route_and_pad_sequence() {
    verify(0xd, "cross", 24, &[Move::Float, Move::Hold, Move::Rise]);
}

#[test]
fn niche_daisy_float_route_and_pad_sequence() {
    verify(0xe, "cross", 25, &[Move::Float, Move::Hold, Move::Rise]);
}

#[test]
fn niche_snake_cypher_route_and_pad_sequence() {
    verify_pad_sequence(super::measured_row(26, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::Hover]);
}

#[test]
fn niche_link_remote_bomb_route_and_pad_sequence() {
    let row = select(facts(0x2, "near"), CANDIDATE_ROWS).expect("Link has a horizontal shortfall");
    assert_eq!((row.id, row.route, row.moves), (27, Route::Near, &[Move::Bomb, Move::Jump, Move::Rise][..]));
    assert!(select(facts(0x2, "cross"), 1 << (27 - 1)).is_none());
    let mut sequence = Sequence::new(row);
    let pull = sequence.step(sample(0, Motion::Free)).unwrap();
    assert_eq!((pull.press, pull.stick), (perfect::SPECIAL, Some([0.0, -1.0])));
    assert_eq!(sequence.step(sample(1, Motion::Starting)).unwrap().press, 0);
    let drop = sequence.step(sample(8, Motion::ItemHeld)).unwrap();
    assert_eq!((drop.press, drop.stick), (super::CATCH, Some([0.0; 2])));
    let align = Sample { can_start: false, ..sample(9, Motion::ItemLoose) };
    assert_eq!(sequence.step(align).unwrap().press, 0);
    let blast = sequence.step(sample(10, Motion::ItemLoose)).unwrap();
    assert_eq!((blast.press, blast.stick), (perfect::SPECIAL, Some([0.0, -1.0])));
    assert_ne!(blast.release & super::CATCH, 0);
    sequence.step(sample(11, Motion::BombBlast)).unwrap();
    assert_eq!(sequence.step(sample(12, Motion::BombHit)), Some(perfect::Correction::default()));
    sequence.step(sample(24, Motion::Free)).unwrap();
    assert_eq!(sequence.step(sample(25, Motion::Free)).unwrap().press, perfect::JUMP);
    sequence.step(sample(26, Motion::Travel)).unwrap();
    sequence.step(sample(45, Motion::Free)).unwrap();
    let finish = sequence.step(sample(46, Motion::Free)).unwrap();
    assert_eq!((finish.press, finish.stick), (perfect::SPECIAL, Some([0.0, 1.0])));
}

#[test]
fn niche_lucas_pk_thunder_route_and_pad_sequence() {
    verify(0x28, "cross", 28, &[Move::Hold, Move::Bolt]);
}

#[test]
fn niche_lucario_extreme_speed_route_and_pad_sequence() {
    verify(0x2c, "cross", 29, &[Move::Hold, Move::Travel]);
}

#[test]
fn niche_olimar_winged_pikmin_route_and_pad_sequence() {
    verify(0x2b, "cross", 30, &[Move::Hold, Move::Hover]);
}

#[test]
fn niche_bowser_jr_clown_kart_route_and_pad_sequence() {
    verify(0x3a, "cross", 31, &[Move::Side, Move::Jump, Move::Jump, Move::Rise]);
}

#[test]
fn niche_hero_woosh_route_and_pad_sequence() {
    verify_pad_sequence(super::measured_row(32, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::Rise]);
}

#[test]
fn niche_yoshi_egg_throw_route_and_pad_sequence() {
    // Yoshi's climb is the double jump: it waits for the exit.
    verify(0x5, "cross", 33, &[Move::Hold, Move::Jump, Move::Rise]);
    assert!(
        select(facts(0x4, "cross"), CANDIDATE_ROWS).is_none(),
        "Dark Samus does not own Yoshi's recovery"
    );
    let mut spent = Sequence::new(tested_row(33));
    spent.step(sample(0, Motion::Free)).expect("the glide ends");
    let no_jump = Sample {
        can_start: false,
        ..sample(1, Motion::Free)
    };
    assert_eq!(
        spent.step(no_jump).map(|pad| pad.press),
        Some(0),
        "a jump the glide already spent is skipped, not fatal"
    );
    assert_eq!(spent.action(), Some(Move::Rise));
}

#[test]
fn niche_luigi_green_missile_route_and_pad_sequence() {
    verify(0x9, "near", 34, &[Move::Missile]);
}

#[test]
fn niche_charizard_fly_route_and_pad_sequence() {
    verify_pad_sequence(super::measured_row(35, CANDIDATE_ROWS).unwrap(), &[Move::Hold, Move::Rise]);
}

#[test]
fn niche_near_rows_wait_for_an_extension_need() {
    for (fighter, id) in [(0x34, 9), (0x10, 11), (0x9, 34)] {
        let need = facts(fighter, "near");
        assert_eq!(select(need, CANDIDATE_ROWS).map(|row| row.id), Some(id));
        let plain = Facts {
            short: false,
            ..need
        };
        assert!(
            select(plain, CANDIDATE_ROWS).is_none(),
            "row {id}: the plain route returns, so the extension stays unspent"
        );
    }
    let below = Facts {
        climb: 30.0,
        ..facts(0x9, "near")
    };
    assert!(
        select(below, CANDIDATE_ROWS).is_none(),
        "a Green Missile from under the ledge meets the wall"
    );
    let mac_below = Facts {
        climb: 30.0,
        ..facts(0x34, "near")
    };
    assert_eq!(
        select(mac_below, CANDIDATE_ROWS).map(|row| row.id),
        Some(9),
        "Jolt Haymaker still has the uppercut for the climb"
    );
}

#[test]
fn niche_hold_jumps_only_below_the_hold_line() {
    let mut glide = Sequence::new(tested_row(17));
    let level = glide.step(gliding(0, Motion::Free, false)).unwrap();
    assert_eq!(
        (level.stick, level.press),
        (Some([1.0, 0.0]), 0),
        "no jump is burst at the entry"
    );
    assert_eq!(
        glide.step(gliding(1, Motion::Free, true)).unwrap().press,
        perfect::JUMP
    );
    assert_eq!(
        glide.step(gliding(2, Motion::Free, true)).unwrap().press,
        0,
        "one press per jump"
    );
    let rising = glide.step(gliding(3, Motion::Travel, true)).unwrap();
    assert_eq!(rising.press, 0);
    assert_ne!(rising.release & perfect::JUMP, 0);
    assert_eq!(
        glide.step(gliding(30, Motion::Free, false)).unwrap().press,
        0
    );
    assert_eq!(
        glide.step(gliding(31, Motion::Free, true)).unwrap().press,
        perfect::JUMP,
        "the next jump waits for the line again"
    );
    glide.step(gliding(32, Motion::Travel, false)).unwrap();
    let spent = Sample {
        can_start: false,
        ..gliding(60, Motion::Free, true)
    };
    assert_eq!(
        glide.step(spent).map(|pad| pad.press),
        Some(0),
        "no jump left: keep gliding"
    );
    assert_eq!(glide.action(), Some(Move::Hold));
    glide
        .step(sample(61, Motion::Free))
        .expect("the exit ends the glide");
    assert_eq!(glide.action(), None);
}

#[test]
fn niche_crossing_is_checked_again_before_the_entry() {
    // Pit's glide: the plan fitted, then the fighter was hit lower or lost its jumps outside the ledge.
    let glide = Sequence::new(tested_row(19));
    assert!(glide.still_crosses(ROOMY));
    assert!(
        !glide.still_crosses(TIGHT),
        "the rest of the trip no longer fits: stay on the near side"
    );
    let below = Budget {
        crossing: Crossing {
            foot: -140.0,
            ..ROOMY.crossing
        },
        ..ROOMY
    };
    assert!(!glide.still_crosses(below), "already under the band");
}

#[test]
fn niche_crossing_rows_decline_an_infeasible_budget() {
    let every = |fighter, budget| Facts {
        fighter,
        corridor: Some([-135.0, -39.0]),
        covered_ledge: true,
        player_over_crossing: false,
        long_approach: false,
        counter_contact: false,
        near: None,
        short: false,
        climb: 0.0,
        budget,
    };
    let mut crossings = 0;
    for row in super::ROWS
        .iter()
        .filter(|row| row.route == Route::Opposite)
    {
        let only = 1 << (row.id - 1);
        let roomy = every(row.fighter, Some(ROOMY));
        if [2, 3, 6].contains(&row.id) {
            assert!(select(roomy, only).is_none(), "row {} needs a different setup", row.id);
            continue;
        }
        if [4, 8, 14, 15, 26, 32, 35, perfect::link::ROW].contains(&row.id) {
            assert!(select(roomy, only).is_none(), "row {} needs a dedicated model", row.id);
            continue;
        }
        let chosen = select(roomy, only).expect("covered ledge and spare crossing budget");
        crossings += 1;
        let margin = chosen.margin(roomy).expect("a selected row has a margin");
        assert!((0.0..=1.0).contains(&margin), "row {}", row.id);
        for budget in [Some(TIGHT), None] {
            assert!(
                select(every(row.fighter, budget), only).is_none(),
                "row {} must not commit to a crossing it cannot finish",
                row.id
            );
        }
    }
    assert_eq!(crossings, 20);
}

#[test]
fn niche_invalid_native_facts_fail_closed_and_a_mirrored_pad_is_real() {
    let row = select(facts(0x27, "long"), CANDIDATE_ROWS).unwrap();
    let mut sequence = Sequence::new(row);
    let left = Sample {
        direction: -1.0,
        ..sample(0, Motion::Free)
    };
    assert_eq!(
        sequence.step(left).and_then(|pad| pad.stick),
        Some([-1.0, 0.0])
    );
    let invalid = Sample {
        valid: false,
        ..sample(1, Motion::Starting)
    };
    assert_eq!(sequence.step(invalid), None);
    assert_eq!(sequence.action(), None);
}

#[test]
fn niche_requires_stock_transition_and_does_not_spend_an_unaccepted_input_twice() {
    let row = select(facts(0x2d, "cross"), CANDIDATE_ROWS).unwrap();
    let mut denied = Sequence::new(row);
    denied.step(sample(0, Motion::Free)).expect("the glide ends");
    assert_eq!(
        denied.step(Sample {
            can_start: false,
            ..sample(1, Motion::Free)
        }),
        None
    );
    let mut sequence = Sequence::new(row);
    sequence.step(sample(0, Motion::Free)).expect("the glide ends");
    assert_eq!(
        sequence.step(sample(1, Motion::Free)).unwrap().press,
        perfect::SPECIAL
    );
    let pending = sequence.step(sample(2, Motion::Free)).unwrap();
    assert_eq!(pending.press, 0);
    assert_ne!(pending.release & perfect::SPECIAL, 0);
    assert_eq!(sequence.step(sample(3, Motion::Interrupted)), None);
}

#[test]
fn niche_thunder_releases_bash_before_the_down_special_press() {
    // Synthetic transitions check button edges only. M9 measures the native cancel.
    let mut sequence = Sequence::new(tested_row(2));
    let bash = sequence.step(sample(0, Motion::Free)).unwrap();
    assert_eq!((bash.stick, bash.press), (Some([1.0, 0.0]), perfect::SPECIAL));
    for (frame, phase) in [(1, Motion::Starting), (2, Motion::Travel), (3, Motion::Finish)] {
        let pad = sequence.step(sample(frame, phase)).unwrap();
        assert_eq!(pad.press, 0);
        assert_ne!(pad.release & perfect::SPECIAL, 0);
    }
    assert_eq!(sequence.step(sample(4, Motion::Free)).unwrap().press, 0);
    let thunder = sequence.step(sample(5, Motion::Free)).unwrap();
    assert_eq!((thunder.stick, thunder.press), (Some([0.0, -1.0]), perfect::SPECIAL));
    let startup = sequence.step(sample(6, Motion::Starting)).unwrap();
    assert_eq!((startup.stick, startup.press), (Some([0.0, -1.0]), 0));
}

#[test]
fn niche_rocketbarrel_holds_through_native_charge_before_release() {
    let mut sequence = Sequence::new(select(facts(0x27, "cross"), CANDIDATE_ROWS).unwrap());
    sequence.step(sample(0, Motion::Free)).expect("the glide ends");
    sequence.step(sample(1, Motion::Free)).unwrap();
    sequence.step(sample(2, Motion::Starting)).unwrap();
    for frame in 8..28 {
        let edit = sequence.step(sample(frame, Motion::Charging)).unwrap();
        assert_eq!(edit.press, perfect::SPECIAL, "charge frame {frame}");
    }
    let release = sequence.step(sample(28, Motion::Charging)).unwrap();
    assert_eq!(release.press, 0);
    assert_ne!(release.release & perfect::SPECIAL, 0);
}

#[test]
fn niche_float_releases_to_spend_parasol_after_exit() {
    let mut sequence = Sequence::new(select(facts(0xd, "cross"), CANDIDATE_ROWS).unwrap());
    sequence.step(sample(0, Motion::Free)).unwrap();
    sequence.step(sample(1, Motion::Starting)).unwrap();
    let crossing = Sample {
        at_exit: false,
        ..sample(2, Motion::Travel)
    };
    assert_eq!(sequence.step(crossing).unwrap().press, perfect::JUMP);
    let exit = sequence.step(sample(3, Motion::Travel)).unwrap();
    assert_eq!(exit.press, 0);
    assert_ne!(exit.release & perfect::JUMP, 0);
}

#[test]
fn niche_plain_cross_chop_and_ray_leave_timing_to_the_generic_policy() {
    for fighter in [0x47, 0x5b] {
        assert!(
            select(facts(fighter, "near"), CANDIDATE_ROWS).is_none(),
            "fighter {fighter:#x}: direct up special retains the generic timing policy"
        );
    }
}

#[test]
fn niche_mythra_needs_a_measured_photon_edge_before_choosing_a_terminal_special() {
    let input = Facts { short: true, ..facts(0x5b, "long") };
    assert!(
        select(input, CANDIDATE_ROWS).is_none(),
        "a level gap alone cannot prove Photon Edge fits before special fall"
    );
}

#[test]
fn niche_incineroar_adds_whip_only_outside_the_cross_chop_column() {
    let input = Facts { short: true, ..facts(0x47, "long") };
    let far = select(input, CANDIDATE_ROWS).expect("IN-1 for a horizontal shortfall");
    assert_eq!((far.id, far.moves), (36, &[Move::Side, Move::Rise][..]));
    let mut far = Sequence::new(far);
    assert_eq!(
        far.step(sample(0, Motion::Free)).map(|pad| (pad.stick, pad.press)),
        Some((Some([1.0, 0.0]), perfect::SPECIAL))
    );
    far.step(sample(1, Motion::Starting)).expect("stock accepts Alolan Whip");
    far.step(sample(40, Motion::Finish)).expect("no up special during Whip endlag");
    far.step(sample(41, Motion::Free)).expect("one released frame after Whip");
    assert_eq!(
        far.step(sample(42, Motion::Free)).map(|pad| (pad.stick, pad.press)),
        Some((Some([0.0, 1.0]), perfect::SPECIAL))
    );
}

#[test]
fn niche_pyra_starts_the_authored_native_swap_on_horizontal_shortfall() {
    assert!(select(facts(0x5a, "near"), CANDIDATE_ROWS).is_none());
    let input = Facts { short: true, ..facts(0x5a, "long") };
    let row = select(input, CANDIDATE_ROWS).expect("PY-1 for a horizontal shortfall");
    assert_eq!(row.id, 38);
    let mut sequence = Sequence::new(row);
    assert_eq!(
        sequence.step(sample(0, Motion::Free)).map(|pad| (pad.stick, pad.press)),
        Some((Some([0.0, -1.0]), perfect::SPECIAL))
    );
    assert!(!sequence.accepts_form(None));
    assert!(!sequence.accepts_form(Some(0x5b)));
    let startup = sequence.step(sample(1, Motion::Starting)).expect("native swap status");
    assert_eq!(startup.press, 0);
    assert!(sequence.accepts_form(None));
    assert!(sequence.accepts_form(Some(0x5b)));
    assert!(!sequence.accepts_form(Some(0x5a)));
    assert!(!sequence.accepts_form(Some(0x47)));
    sequence.step(sample(2, Motion::Interrupted));
    assert!(!sequence.accepts_form(Some(0x5b)));
}

#[test]
fn niche_mythra_commits_to_only_one_special() {
    let row = tested_row(39);
    assert_eq!((row.id, row.moves), (39, &[Move::Side][..]));
    let mut level = Sequence::new(row);
    assert_eq!(
        level.step(sample(0, Motion::Free)).map(|pad| (pad.stick, pad.press)),
        Some((Some([1.0, 0.0]), perfect::SPECIAL))
    );
    level.step(sample(1, Motion::Starting)).unwrap();
    assert!(level.step(sample(2, Motion::Interrupted)).is_none());
    assert!(level.action().is_none());
}

#[test]
fn niche_near_extensions_require_valid_reach_and_a_plausible_entry() {
    let mut incineroar = Facts { short: true, ..facts(0x47, "long") };
    let reach = incineroar.near.unwrap();
    incineroar.near = Some(NearReach { climb: reach.body + 1.0, ..reach });
    assert!(select(incineroar, CANDIDATE_ROWS).is_none());
    let mut pyra = Facts { short: true, ..facts(0x5a, "long") };
    pyra.near = Some(NearReach {
        special: perfect::SpecialReach { horizontal: f32::NAN, ..reach.special },
        ..reach
    });
    assert!(select(pyra, CANDIDATE_ROWS).is_none());
}

#[test]
fn niche_skull_bash_budget_counts_the_traced_drop_before_the_glide() {
    let start = Crossing {
        band: [-110.0, -39.0],
        foot: -49.72,
        ..ROOMY.crossing
    };
    let input = Facts {
        budget: Some(Budget { crossing: start, ..ROOMY }),
        corridor: Some(start.band),
        ..facts(0x8, "long")
    };
    let skull_bash_only = 1 << (3 - 1);
    assert!(select(input, skull_bash_only).is_none());
    assert_eq!(select(facts(0x8, "long"), skull_bash_only).unwrap().id, 3);
}

#[test]
fn niche_link_does_not_finish_the_bomb_move_after_only_pulling_it() {
    let mut sequence = Sequence::new(tested_row(27));
    sequence.step(sample(0, Motion::Free)).expect("pull bomb");
    sequence.step(sample(1, Motion::Starting)).expect("native pull");
    sequence.step(sample(8, Motion::Free));
    assert_eq!(sequence.action(), None, "no follow-up without a drop and blast");
}

#[test]
fn niche_near_gap_extensions_use_the_full_plain_return() {
    for (fighter, row) in [(0x47, 36), (0x5a, 38)] {
        let input = Facts { short: false, ..facts(fighter, "long") };
        let only = 1 << (row - 1);
        assert!(select(input, only).is_none(), "plain return can reach for row {row}");
        assert_eq!(select(Facts { short: true, ..input }, only).unwrap().id, row);
    }
}

#[test]
fn niche_rob_budget_and_takeoff_allow_the_burner_startup_rise() {
    let rob = tested_row(7);
    let ceiling = -40.0;
    assert!(rob.launch_clears(-65.0, ceiling));
    assert!(!rob.launch_clears(-64.0, ceiling));
    assert!(tested_row(5).launch_clears(-64.0, ceiling));
    let tight_roof = Crossing {
        band: [-63.0, ceiling],
        foot: -58.0,
        ..ROOMY.crossing
    };
    let facts = Facts {
        budget: Some(Budget { crossing: tight_roof, ..ROOMY }),
        ..facts(0x2d, "cross")
    };
    assert!(rob.margin(facts).is_none());
}

#[test]
fn live_sequence_rows_include_near_extensions_and_exclude_crossing_estimates() {
    let cross = facts(0x2d, "cross");
    assert_eq!(select(cross, CANDIDATE_ROWS).map(|row| row.id), Some(7));
    assert!(select(cross, EXTENSION_ROWS).is_none());
    assert_eq!(select(facts(0x34, "counter"), EXTENSION_ROWS).map(|row| row.id), Some(10));
    assert!(ROWS.iter().filter(|row| EXTENSION_ROWS & (1 << (row.id - 1)) != 0)
        .all(|row| row.route == Route::Near));
}

#[test]
fn bowser_jr_crosses_with_the_kart_jump_before_the_double_jump() {
    let route = tested_row(31);
    assert_eq!(route.route, Route::Opposite);
    assert_eq!(route.moves, &[Move::Side, Move::Jump, Move::Jump, Move::Rise]);
}
