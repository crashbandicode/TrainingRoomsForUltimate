//! rc.14 Break the Targets, the menu's side (`artifacts/rc14-eggsercise/SPEC.md` §V2.5, V2.7, acceptance row 33): the two
//! practice games exclude each other, Place targets closes the menu with a request, and Target speed greys while the
//! targets stay still.

lib_unit_tests! {
use super::*;

fn row(id: &str) -> usize {
    ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("row {id}"))
}

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

/// Row 33: turning one practice game On sets the other Off, from every way the menu changes a row.
fn btt_and_eggsercise_exclude_each_other() {
    let mut m = menu();
    let (eggs, btt) = (row("eggsercise"), row("btt"));
    m.cycle(eggs, true);
    assert_eq!((m.get("eggsercise"), m.get("btt")), (&[0, 1, 0, 0, 0][..], &[1, 0][..]));
    // Left / Right on Break the Targets: Eggs-ercise goes Off.
    m.cycle(btt, true);
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[0, 1][..], &[1, 0, 0, 0, 0][..]));
    // And back: the one turned On last wins.
    m.cycle(eggs, true);
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[1, 0][..], &[0, 1, 0, 0, 0][..]));
    // A list's pick and a bulk put take the same path; turning a game Off touches nothing else.
    m.put("btt", &[0, 1]);
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[0, 1][..], &[1, 0, 0, 0, 0][..]));
    m.put("btt", &[1, 0]);
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[1, 0][..], &[1, 0, 0, 0, 0][..]));
    // The close commits both rows the player's change touched, over the live settings.
    let mut live = Settings::factory();
    live.menu.set("eggsercise", &[0, 0, 1, 0, 0]).unwrap();
    let mut m = Menu::new(live.clone(), Env::default());
    m.cycle(btt, true);
    let merged = m.commit_onto(&live);
    assert_eq!((merged.menu.get("btt"), merged.menu.get("eggsercise")), (Some(&[0, 1][..]), Some(&[1, 0, 0, 0, 0][..])));
}

/// A settings file with both On (hand-edited) keeps Eggs-ercise, in the menu rows and in the saved defaults.
fn a_file_with_both_practice_games_on_keeps_eggsercise() {
    let both = r#"{"menu": {"eggsercise": [0,1,0,0,0], "btt": [0,1]}, "defaults_menu": {"eggsercise": [0,0,0,1,0], "btt": [0,1]}}"#;
    let loaded = settings::parse(both);
    let s = &loaded.settings;
    assert_eq!((s.menu.get("eggsercise"), s.menu.get("btt")), (Some(&[0, 1, 0, 0, 0][..]), Some(&[1, 0][..])));
    assert_eq!((s.defaults.get("eggsercise"), s.defaults.get("btt")), (Some(&[0, 0, 0, 1, 0][..]), Some(&[1, 0][..])));
    let one = settings::parse(r#"{"menu": {"btt": [0,1]}}"#).settings;
    assert_eq!((one.menu.get("eggsercise"), one.menu.get("btt")), (Some(&[1, 0, 0, 0, 0][..]), Some(&[0, 1][..])), "Break the Targets alone stays On");
}

/// Place targets turns Break the Targets On (and Eggs-ercise Off), closes the menu and asks the runtime for the placing view, once.
fn place_targets_closes_the_menu_with_a_request() {
    let mut m = menu();
    m.put("eggsercise", &[0, 1, 0, 0, 0]);
    assert!(!m.is_closed());
    m.activate(row("btt_place"));
    assert!(m.is_closed());
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[0, 1][..], &[1, 0, 0, 0, 0][..]), "no prior visit to the Break the Targets row");
    assert!(m.take_outbox().place_targets);
    assert!(!m.take_outbox().place_targets, "taken once");
}

/// SPEC V2.5: Target speed is greyed with the reason "Targets stay still" while Break the Targets is On.
fn target_speed_greys_while_the_targets_stay_still() {
    let mut m = menu();
    let speed = row("eggsercise_speed");
    assert_eq!(m.grey(speed), None);
    m.put("btt", &[0, 1]);
    assert_eq!(m.grey(speed), Some(Grey::TargetsStill));
    assert_eq!(Grey::TargetsStill.text(), "Targets stay still");
    let depth = m.depth();
    m.activate(speed);
    m.cycle(speed, true);
    assert_eq!((m.depth(), m.get("eggsercise_speed")), (depth, &[1, 0, 0][..]), "A and Left / Right do nothing while greyed");
    m.put("btt", &[1, 0]);
    assert_eq!(m.grey(speed), None);
}

/// Exercise every available registered pair in both directions, through real row writes and
/// tracked commits. Stage tech joins this table when its settings/menu row is integrated.
fn all_available_practice_games_follow_last_on() {
    let games: Vec<_> = super::super::practice_games::ROWS.iter().copied()
        .filter(|id| settings::def(id).is_some()).collect();
    for &first in &games {
        for &last in &games {
            if first == last {
                continue;
            }
            let mut m = menu();
            m.put(first, &[0, 1]);
            m.put(last, &[0, 1]);
            assert!(super::super::practice_games::is_on(&m.settings.menu, last), "{first} then {last}");
            for &other in &games {
                if other != last {
                    assert_eq!(m.get(other)[0], 1, "{other} Off after {last}");
                    assert!(m.get(other).iter().skip(1).all(|value| *value == 0));
                }
            }
            // Turning an already-Off peer Off again leaves the selected game alone.
            m.put_one_hot(first, 0);
            assert!(super::super::practice_games::is_on(&m.settings.menu, last));
            m.save_defaults();
            let merged = m.commit_onto(&Settings::factory());
            assert!(super::super::practice_games::is_on(&merged.menu, last));
            assert!(super::super::practice_games::is_on(&merged.defaults, last));
            assert!(merged.menu.get(first).unwrap().iter().skip(1).all(|value| *value == 0));
        }
    }
    // All enabled peers in a direct snapshot must be cleared even if no prior menu edit saw them.
    let mut dirty = Settings::factory();
    for &id in &games {
        dirty.menu.set(id, &[0, 1]).unwrap();
    }
    let mut m = Menu::new(dirty, Env::default());
    // Change Eggs-ercise to a different On choice so this is a real edit.
    m.put("eggsercise", &[0, 0, 1, 0, 0]);
    for &id in &games {
        assert_eq!(super::super::practice_games::is_on(&m.settings.menu, id), id == "eggsercise");
        if id != "eggsercise" {
            assert!(m.get(id).iter().skip(1).all(|value| *value == 0));
        }
    }
}

fn btt_projectiles_default_and_round_trip() {
    let id = "btt_projectiles";
    let mut m = menu();
    assert_eq!(m.get(id), &[0, 1], "default On");
    let old = settings::parse(r#"{"menu":{"btt":[0,1]}}"#).settings;
    assert_eq!(old.menu.get(id), Some(&[0, 1][..]), "old files acquire the On default");
    m.cycle(row(id), true);
    assert_eq!(m.get(id), &[1, 0]);
    let saved = m.commit_onto(&Settings::factory());
    let json = saved.to_json();
    let loaded = settings::parse(&json).settings;
    assert_eq!(loaded.menu.get(id), Some(&[1, 0][..]));
}

fn unavailable_projectiles_leave_melee_and_placement_usable() {
    let mut m = Menu::new(Settings::factory(), Env { btt_projectiles_ready: false, ..Env::default() });
    let projectile = row("btt_projectiles");
    assert_eq!(m.grey(projectile), Some(Grey::ProjectilesUnavailable));
    let depth = m.depth();
    m.activate(projectile);
    m.cycle(projectile, true);
    m.cycle(projectile, false);
    assert_eq!((m.depth(), m.get("btt_projectiles")), (depth, &[0, 1][..]));
    assert_eq!(m.grey(row("btt")), None);
    assert_eq!(m.grey(row("btt_place")), None);
    m.cycle(row("btt"), true);
    assert_eq!(m.get("btt"), &[0, 1]);
    m.activate(row("btt_place"));
    assert!(m.take_outbox().place_targets);
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("btt_and_eggsercise_exclude_each_other", btt_and_eggsercise_exclude_each_other as fn()),
        ("a_file_with_both_practice_games_on_keeps_eggsercise", a_file_with_both_practice_games_on_keeps_eggsercise as fn()),
        ("place_targets_closes_the_menu_with_a_request", place_targets_closes_the_menu_with_a_request as fn()),
        ("unavailable_projectiles_leave_melee_and_placement_usable", unavailable_projectiles_leave_melee_and_placement_usable as fn()),
        ("btt_projectiles_default_and_round_trip", btt_projectiles_default_and_round_trip as fn()),
        ("target_speed_greys_while_the_targets_stay_still", target_speed_greys_while_the_targets_stay_still as fn()),
        ("all_available_practice_games_follow_last_on", all_available_practice_games_follow_last_on as fn()),
    ]);
}
}
