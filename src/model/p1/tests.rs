lib_unit_tests! {
use super::*;
use std::convert::TryFrom;

// Synthetic rank fixture only; never evidence of a native level/rank mapping.
const RANKS: Option<[u8; 9]> = Some([1; 9]);

fn cfg(trigger: Trigger, command: Command) -> Config {
    Config {
        level: 9,
        trigger,
        command,
        require_contact: true,
        guard_frames: 3600,
    }
}

fn live(game_frame: u64) -> Frame {
    Frame {
        game_frame,
        identity_matches: true,
        active: true,
        ..Frame::default()
    }
}

fn parses_exact_config_examples_and_rounds_seconds_up() {
    let timed =
        config::from_json(include_str!("examples/fight-five-seconds-walk-off.json")).unwrap();
    assert_eq!(timed, cfg(Trigger::AfterGameFrames(300), Command::WalkOff));
    let moves =
        config::from_json(include_str!("examples/completed-attacks-walk-off.json")).unwrap();
    assert_eq!(
        moves,
        cfg(Trigger::AfterCompletedMoves(3), Command::WalkOff)
    );
    let release = config::from_json(include_str!("examples/immediate-release.json")).unwrap();
    assert_eq!(release.level, 1);
    assert_eq!(release.trigger, Trigger::Immediately);
    assert_eq!(release.command, Command::Release);
    let boundary = config::JsonConfig {
        level: 9,
        trigger: config::JsonTrigger::AfterSeconds { seconds: 0.0167 },
        command: config::JsonCommand::WalkOff,
        require_contact: true,
        guard_frames: 60,
    };
    assert_eq!(
        Config::try_from(boundary).unwrap().trigger,
        Trigger::AfterGameFrames(2)
    );
    let defaults = r#"{"level":9,"trigger":{"kind":"after_seconds","seconds":0},"command":"walk_off","guard_frames":60}"#;
    let parsed = config::from_json(defaults).unwrap();
    assert!(parsed.require_contact);
    assert_eq!(parsed.trigger, Trigger::AfterGameFrames(0));
}

fn game_time_uses_active_frame_deltas_and_pauses_return_physical_pad() {
    let mut config = cfg(Trigger::AfterGameFrames(300), Command::WalkOff);
    config.require_contact = false;
    let mut pilot = Pilot::new(config, RANKS).unwrap();
    assert_eq!(pilot.step(live(1000)), Output::Ai);
    assert_eq!(pilot.elapsed_game_frames(), 0);
    assert_eq!(pilot.step(live(1001)), Output::Ai);
    assert_eq!(
        pilot.step(Frame {
            completed_moves: 9,
            ..live(1001)
        }),
        Output::Ai
    );
    assert_eq!(
        (pilot.elapsed_game_frames(), pilot.completed_moves()),
        (1, 0)
    );
    assert_eq!(
        pilot.step(Frame {
            active: false,
            ..live(2000)
        }),
        Output::Pad
    );
    assert_eq!(pilot.step(live(3000)), Output::Ai);
    assert_eq!(pilot.elapsed_game_frames(), 1);
    assert_eq!(pilot.step(live(3298)), Output::Ai);
    assert_eq!(pilot.elapsed_game_frames(), 299);
    assert_eq!(pilot.step(live(3299)), Output::Command(Command::WalkOff));
    assert_eq!(pilot.elapsed_game_frames(), 300);
    assert_eq!(pilot.completed_moves(), 0);
    assert_eq!(
        pilot.step(Frame {
            active: false,
            ..live(4000)
        }),
        Output::Pad
    );
}

fn sustained_infliction_cannot_count_damage_beyond_next_frame() {
    let mut pilot = Pilot::new(cfg(Trigger::Immediately, Command::WalkOff), RANKS).unwrap();
    assert_eq!(pilot.step(live(0)), Output::Ai);
    assert_eq!(
        pilot.step(Frame {
            infliction: true,
            ..live(1)
        }),
        Output::Ai
    );
    assert_eq!(
        pilot.step(Frame {
            infliction: true,
            ..live(2)
        }),
        Output::Ai
    );
    assert_eq!(
        pilot.step(Frame {
            infliction: true,
            target_damage: 7.0,
            ..live(3)
        }),
        Output::Ai
    );
    assert_eq!(pilot.damaging_hits(), 0);
    // A fresh later hit can still establish contact after the stale one expires.
    assert_eq!(
        pilot.step(Frame {
            target_damage: 7.0,
            ..live(4)
        }),
        Output::Ai
    );
    assert_eq!(
        pilot.step(Frame {
            infliction: true,
            target_damage: 10.0,
            ..live(5)
        }),
        Output::Command(Command::WalkOff)
    );
    assert_eq!(pilot.damage_dealt(), 3.0);
}

fn match_end_identity_and_walk_off_completion_release_control() {
    for identity_change in [false, true] {
        let mut pilot =
            Pilot::new(cfg(Trigger::AfterGameFrames(300), Command::Release), RANKS).unwrap();
        assert_eq!(pilot.step(live(10)), Output::Ai);
        let terminal = Frame {
            identity_matches: !identity_change,
            match_ended: !identity_change,
            ..live(10)
        };
        assert_eq!(pilot.step(terminal), Output::Pad);
        assert_eq!(
            pilot.phase(),
            Phase::Done(if identity_change {
                End::IdentityChanged
            } else {
                End::MatchEnded
            })
        );
        assert_eq!(pilot.step(live(11)), Output::Pad);
    }
    let mut config = cfg(Trigger::Immediately, Command::WalkOff);
    config.require_contact = false;
    let mut pilot = Pilot::new(config, RANKS).unwrap();
    assert_eq!(pilot.step(live(10)), Output::Command(Command::WalkOff));
    assert_eq!(
        pilot.step(Frame {
            p1_lost_stock: true,
            ..live(11)
        }),
        Output::Pad
    );
    assert_eq!(pilot.phase(), Phase::Done(End::Completed));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("parses_exact_config_examples_and_rounds_seconds_up", parses_exact_config_examples_and_rounds_seconds_up as fn()),
        ("game_time_uses_active_frame_deltas_and_pauses_return_physical_pad", game_time_uses_active_frame_deltas_and_pauses_return_physical_pad as fn()),
        ("sustained_infliction_cannot_count_damage_beyond_next_frame", sustained_infliction_cannot_count_damage_beyond_next_frame as fn()),
        ("match_end_identity_and_walk_off_completion_release_control", match_end_identity_and_walk_off_completion_release_control as fn()),
    ]);
}
}
