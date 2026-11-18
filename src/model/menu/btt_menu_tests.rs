//! rc.14 Break the Targets, the menu's side (`artifacts/rc14-eggsercise/SPEC.md` §V2.5, V2.7, acceptance row 33): the two
//! practice games exclude each other, Place targets closes the menu with a request, and Target speed greys while the
//! targets stay still. rc.15 practice menu: one game at a time is the running game (`practice_games::effective`), no longer
//! a rewrite of the other games' rows; the rows only hold modes.

lib_unit_tests! {
use super::*;

fn row(id: &str) -> usize {
    ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("row {id}"))
}

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

/// Row 33, rc.15: starting one game stops the other, because one value names the running game; a game's mode row no
/// longer turns the other's Off, and the close commits only the row the player changed.
fn btt_and_eggsercise_exclude_each_other() {
    use crate::model::practice_games::effective;
    for (active, eggs, btt) in [(Some(Game::Eggs), 1, 0), (Some(Game::Btt), 0, 1), (None, 0, 0)] {
        assert_eq!((effective(Game::Eggs, 0, active), effective(Game::Btt, 1, active)), (eggs, btt), "{active:?}");
    }
    let mut m = menu();
    let eggs = row("eggsercise");
    m.cycle(eggs, true);
    m.put("btt", &[0, 1]);
    assert_eq!((m.get("eggsercise"), m.get("btt")), (&[0, 0, 1, 0, 0][..], &[0, 1][..]), "the rows no longer exclude each other");
    let mut live = Settings::factory();
    live.menu.set("btt", &[0, 1]).unwrap();
    let mut m = Menu::new(live.clone(), Env::default());
    m.cycle(eggs, true);
    let merged = m.commit_onto(&live);
    assert_eq!((merged.menu.get("btt"), merged.menu.get("eggsercise")), (Some(&[0, 1][..]), Some(&[0, 0, 1, 0, 0][..])));
}

/// A settings file with both On (hand-edited) loads as it is: nothing is rewritten, in the menu rows or in the saved
/// defaults (rc.15: no migration loss).
fn a_file_with_both_practice_games_on_keeps_eggsercise() {
    let both = r#"{"menu": {"eggsercise": [0,1,0,0,0], "btt": [0,1]}, "defaults_menu": {"eggsercise": [0,0,0,1,0], "btt": [0,1]}}"#;
    let loaded = settings::parse(both);
    let s = &loaded.settings;
    assert_eq!((s.menu.get("eggsercise"), s.menu.get("btt")), (Some(&[0, 1, 0, 0, 0][..]), Some(&[0, 1][..])));
    assert_eq!((s.defaults.get("eggsercise"), s.defaults.get("btt")), (Some(&[0, 0, 0, 1, 0][..]), Some(&[0, 1][..])));
    let one = settings::parse(r#"{"menu": {"btt": [0,1]}}"#).settings;
    assert_eq!((one.menu.get("eggsercise"), one.menu.get("btt")), (Some(&[1, 0, 0, 0, 0][..]), Some(&[0, 1][..])));
}

/// Place targets starts Break the Targets (so any other game stops), closes the menu and asks the runtime for the placing
/// view, once. It writes no row.
fn place_targets_closes_the_menu_with_a_request() {
    let mut m = menu();
    m.put("eggsercise", &[0, 1, 0, 0, 0]);
    let _ = m.take_outbox();
    assert!(!m.is_closed());
    m.activate(row("btt_place"));
    assert!(m.is_closed());
    assert_eq!((m.get("btt"), m.get("eggsercise")), (&[1, 0][..], &[0, 1, 0, 0, 0][..]), "no row is rewritten");
    let out = m.take_outbox();
    assert!(out.place_targets && out.start == Some(Game::Btt) && !out.changed);
    assert!(!m.take_outbox().place_targets, "taken once");
}

/// SPEC V2.5: Target speed is greyed with the reason "Targets stay still" while Break the Targets runs (rc.15: the running
/// game, no longer the `btt` row).
fn target_speed_greys_while_the_targets_stay_still() {
    let speed = row("eggsercise_speed");
    let mut m = menu();
    m.put("btt", &[0, 1]);
    assert_eq!(m.grey(speed), None);
    let mut m = Menu::new(Settings::factory(), Env { running: Some(Game::Btt), ..Env::default() });
    assert_eq!(m.grey(speed), Some(Grey::TargetsStill));
    assert_eq!(Grey::TargetsStill.text(), "Targets stay still");
    let depth = m.depth();
    m.activate(speed);
    m.cycle(speed, true);
    assert_eq!((m.depth(), m.get("eggsercise_speed")), (depth, &[1, 0, 0][..]), "A and Left / Right do nothing while greyed");
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
    let start = ROWS.iter().position(|r| matches!(r.control, Control::Start(Game::Btt))).unwrap();
    assert_eq!((m.grey(start), m.grey(row("btt_place"))), (None, None));
    m.activate(start);
    assert_eq!(m.take_outbox().start, Some(Game::Btt));
    let mut m = Menu::new(Settings::factory(), Env { btt_projectiles_ready: false, ..Env::default() });
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
    ]);
}
}
