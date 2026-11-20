use super::{select, Facts, Motion, Move, Sample, Sequence, CANDIDATE_ROWS};
use crate::model::edge_juggle::perfect::{self, Route};

fn facts(fighter: i32, mode: &str) -> Facts {
    Facts {
        fighter,
        corridor: Some([-135.0, -39.0]),
        covered_ledge: matches!(mode, "cross" | "long"),
        player_over_crossing: mode == "center",
        long_approach: mode == "long",
        counter_contact: mode == "counter",
    }
}

fn sample(frame: u32, motion: Motion) -> Sample {
    Sample {
        frame,
        motion,
        can_start: true,
        valid: true,
        at_exit: true,
        can_enter: true,
        approaching: false,
        direction: 1.0,
        aim: [0.6, 0.8],
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
    }

    let mut sequence = Sequence::new(row);
    for (idx, &movement) in steps.iter().enumerate() {
        let frame = idx as u32 * 7;
        assert_eq!(sequence.action(), Some(movement));
        let pad = sequence
            .step(sample(frame, Motion::Free))
            .expect("first real input");
        if movement == Move::Drift {
            assert_eq!((pad.stick, pad.press), (Some([1.0, 0.0]), 0));
            continue;
        }
        let expected = match movement {
            Move::Jump => ([1.0, 0.0], perfect::JUMP),
            Move::Float => ([0.0, -1.0], perfect::JUMP),
            Move::Side | Move::ChargedSide | Move::SonicBlade | Move::Missile => {
                ([1.0, 0.0], perfect::SPECIAL)
            }
            Move::Down | Move::Counter | Move::Bomb => ([0.0, -1.0], perfect::SPECIAL),
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
    verify(0x8, "cross", 1, &[Move::QuickAttack]);
}

#[test]
fn niche_pikachu_thunder_route_and_pad_sequence() {
    verify(0x8, "center", 2, &[Move::Down, Move::QuickAttack]);
}

#[test]
fn niche_pikachu_skull_bash_route_and_pad_sequence() {
    verify(0x8, "long", 3, &[Move::Side, Move::QuickAttack]);
}

#[test]
fn niche_game_watch_fire_route_and_pad_sequence() {
    verify(0x1c, "cross", 4, &[Move::Drift, Move::Jump, Move::Rise]);
}

#[test]
fn niche_diddy_rocketbarrel_route_and_pad_sequence() {
    verify(0x27, "cross", 5, &[Move::Barrel]);
}

#[test]
fn niche_diddy_monkey_flip_route_and_pad_sequence() {
    verify(0x27, "long", 6, &[Move::Side, Move::Barrel]);
}

#[test]
fn niche_rob_burner_route_and_pad_sequence() {
    verify(0x2d, "cross", 7, &[Move::Hover]);
}

#[test]
fn niche_sora_sonic_blade_route_and_pad_sequence() {
    verify(0x5d, "cross", 8, &[Move::SonicBlade, Move::Rise]);
}

#[test]
fn niche_little_mac_haymaker_route_and_pad_sequence() {
    verify(0x34, "near", 9, &[Move::Side, Move::Rise]);
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
    verify(0x10, "cross", 12, &[Move::Travel]);
}

#[test]
fn niche_greninja_shadow_sneak_route_and_pad_sequence() {
    verify(0x35, "cross", 13, &[Move::ChargedSide, Move::Travel]);
}

#[test]
fn niche_ness_pk_thunder_route_and_pad_sequence() {
    verify(0xa, "cross", 14, &[Move::Bolt]);
}

#[test]
fn niche_steve_elytra_route_and_pad_sequence() {
    verify(0x58, "cross", 15, &[Move::Glide]);
}

#[test]
fn niche_pichu_quick_attack_route_and_pad_sequence() {
    verify(0x13, "cross", 16, &[Move::QuickAttack]);
}

#[test]
fn niche_jigglypuff_jumps_route_and_pad_sequence() {
    verify(
        0xc,
        "cross",
        17,
        &[Move::Jump, Move::Jump, Move::Jump, Move::Drift],
    );
}

#[test]
fn niche_meta_knight_drill_rush_route_and_pad_sequence() {
    verify(0x1d, "cross", 18, &[Move::Jump, Move::Jump, Move::Side]);
}

#[test]
fn niche_pit_power_of_flight_route_and_pad_sequence() {
    verify(0x1e, "cross", 19, &[Move::Jump, Move::Travel]);
}

#[test]
fn niche_dark_pit_power_of_flight_route_and_pad_sequence() {
    verify(0x1f, "cross", 20, &[Move::Jump, Move::Travel]);
}

#[test]
fn niche_pac_man_pac_jump_route_and_pad_sequence() {
    verify(0x37, "cross", 21, &[Move::Jump, Move::Travel]);
}

#[test]
fn niche_bayonetta_witch_twist_route_and_pad_sequence() {
    verify(0x40, "cross", 22, &[Move::Travel, Move::Side, Move::Travel]);
}

#[test]
fn niche_sonic_spring_jump_route_and_pad_sequence() {
    verify(0x29, "cross", 23, &[Move::Side, Move::Rise]);
}

#[test]
fn niche_peach_float_route_and_pad_sequence() {
    verify(0xd, "cross", 24, &[Move::Float, Move::Rise]);
}

#[test]
fn niche_daisy_float_route_and_pad_sequence() {
    verify(0xe, "cross", 25, &[Move::Float, Move::Rise]);
}

#[test]
fn niche_snake_cypher_route_and_pad_sequence() {
    verify(0x22, "cross", 26, &[Move::Hover]);
}

#[test]
fn niche_link_remote_bomb_route_and_pad_sequence() {
    verify(0x2, "cross", 27, &[Move::Bomb, Move::Jump, Move::Rise]);
}

#[test]
fn niche_lucas_pk_thunder_route_and_pad_sequence() {
    verify(0x28, "cross", 28, &[Move::Bolt]);
}

#[test]
fn niche_lucario_extreme_speed_route_and_pad_sequence() {
    verify(0x2c, "cross", 29, &[Move::Travel]);
}

#[test]
fn niche_olimar_winged_pikmin_route_and_pad_sequence() {
    verify(0x2b, "cross", 30, &[Move::Hover]);
}

#[test]
fn niche_bowser_jr_clown_kart_route_and_pad_sequence() {
    verify(0x3a, "cross", 31, &[Move::Side, Move::Rise]);
}

#[test]
fn niche_hero_woosh_route_and_pad_sequence() {
    verify(0x53, "cross", 32, &[Move::Rise]);
}

#[test]
fn niche_yoshi_egg_throw_route_and_pad_sequence() {
    verify(0x4, "cross", 33, &[Move::Jump, Move::Rise]);
}

#[test]
fn niche_luigi_green_missile_route_and_pad_sequence() {
    verify(0x9, "near", 34, &[Move::Missile]);
}

#[test]
fn niche_charizard_fly_route_and_pad_sequence() {
    verify(0x26, "cross", 35, &[Move::Jump, Move::Jump, Move::Rise]);
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
    assert_eq!(
        denied.step(Sample {
            can_start: false,
            ..sample(0, Motion::Free)
        }),
        None
    );
    let mut sequence = Sequence::new(row);
    assert_eq!(
        sequence.step(sample(0, Motion::Free)).unwrap().press,
        perfect::SPECIAL
    );
    let pending = sequence.step(sample(1, Motion::Free)).unwrap();
    assert_eq!(pending.press, 0);
    assert_ne!(pending.release & perfect::SPECIAL, 0);
    assert_eq!(sequence.step(sample(2, Motion::Interrupted)), None);
}

#[test]
fn niche_thunder_drifts_into_position_before_spending_down_special() {
    let row = select(facts(0x8, "center"), CANDIDATE_ROWS).unwrap();
    let mut sequence = Sequence::new(row);
    let approach = Sample {
        approaching: true,
        ..sample(0, Motion::Free)
    };
    let pad = sequence.step(approach).unwrap();
    assert_eq!((pad.stick, pad.press), (Some([1.0, 0.0]), 0));
    assert_eq!(
        sequence.step(sample(1, Motion::Free)).unwrap().press,
        perfect::SPECIAL
    );
}

#[test]
fn niche_rocketbarrel_holds_through_native_charge_before_release() {
    let mut sequence = Sequence::new(select(facts(0x27, "cross"), CANDIDATE_ROWS).unwrap());
    sequence.step(sample(0, Motion::Free)).unwrap();
    sequence.step(sample(1, Motion::Starting)).unwrap();
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
