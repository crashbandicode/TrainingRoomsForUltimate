//! rc.15 practice menu (`docs/DECISIONS.md`, "rc.15 practice menu", 2026-11-18; the approved mockup, Q1–Q4 A): one row per
//! game on Practice games, a sub-screen each with Start first, Start / Stop as menu actions, no Off choices, CPU during a
//! game in two screens, and old settings files that load as they are.
lib_unit_tests! {
use super::*;
use crate::model::frame::{Frame, Tone};

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn running(game: Game) -> Menu {
    Menu::new(Settings::factory(), Env { running: Some(game), ..Env::default() })
}

fn frame(m: &Menu) -> Frame {
    let mut f = Frame::new();
    crate::model::screens::frame_of(m, &mut f, |_| "Stage");
    f
}

fn row(title: &str, group: u8) -> usize {
    ROWS.iter().position(|r| r.title == title && r.group == group).unwrap_or_else(|| panic!("row {title} in {group}"))
}

/// The Practice games list on Fighters & Stages, with the cursor on `game`'s row.
fn on_list(m: &mut Menu, game: Game) {
    for _ in 0..6 {
        if m.page() == Page::FightersStages {
            break;
        }
        m.press(Button::R);
    }
    let target = row(game.title(), table::GAMES);
    for _ in 0..=m.entry_count(Page::FightersStages) {
        if m.focus() == Some(Entry::Row(target)) {
            return;
        }
        m.press(Button::Down);
    }
    panic!("{} is not on the page", game.title());
}

/// `game`'s sub-screen, opened from the list.
fn opened(game: Game) -> Menu {
    let mut m = menu();
    on_list(&mut m, game);
    m.press(Button::A);
    m
}

fn titles(group: u8) -> Vec<&'static str> {
    ROWS.iter().filter(|r| r.group == group).map(|r| r.title).collect()
}

#[test]
fn games_list_is_one_row_per_game() {
    let list: Vec<_> = ROWS.iter().filter(|r| r.group == table::GAMES).map(|r| r.control).collect();
    assert_eq!(list.len(), Game::ALL.len());
    for (control, game) in list.iter().zip(Game::ALL) {
        assert!(matches!(control, Control::Game(g) if *g == game), "{control:?}");
    }
    assert_eq!(titles(table::GAMES), ["Eggs-ercise", "Break the Targets", "Tech practice", "Mash out", "Parry practice"]);
    // The games' sub groups are not sections of the page: the left column ends with Practice games.
    let m = menu();
    let groups: Vec<&str> = (0..m.entry_count(Page::FightersStages))
        .filter_map(|i| match m.entry(Page::FightersStages, i) {
            Some(Entry::Group(g)) => Some(GROUPS[g].title),
            _ => None,
        })
        .collect();
    // Parry practice (slot 6) left its temporary section for its own sub-screen.
    assert_eq!(groups, ["Player 1 and CPU", "Extras", "Stages", "Loading", "Practice games"]);
    for game in Game::ALL {
        let g = usize::from(table::game_group(game));
        assert!(GROUPS[g].sub && GROUPS[g].page == Page::FightersStages && GROUPS[g].title == game.title());
    }
}

#[test]
fn a_game_row_opens_its_sub_screen_on_start() {
    for game in Game::ALL {
        let mut m = opened(game);
        assert_eq!(m.screen(), Some(&Screen::Rows { group: table::game_group(game), cursor: 0 }), "Q4: the cursor opens on Start");
        assert!(matches!(m.help_row().map(|r| r.control), Some(Control::Start(g)) if g == game));
        let f = frame(&m);
        assert_eq!(f.rows[0].label.string_lossy(), "Start");
        assert_eq!(f.crumb.string_lossy(), format!("Fighters & Stages  >  Practice games  >  {}", game.title()));
        assert_eq!(f.hint.string_lossy(), "L/R page   Up/Down move   ZL+Up/Down jump   Left/Right change   A open   Y reset row   B back");
        assert_eq!(f.group_count, 0, "no left column");
        m.press(Button::B);
        assert_eq!((m.screen(), m.is_closed()), (None, false), "B goes back to the list");
    }
}

/// Today's rows of each game, labels, controls and order, from `c212bc50`'s table: Start, then these.
#[test]
fn each_sub_screen_has_start_first_then_todays_rows() {
    let eggs = ["Start", "Eggs-ercise", "Breaks on", "Target", "Target speed", "CPU during a game"];
    let btt = ["Start", "Projectiles break targets", "Place targets", "Saved layouts", "CPU during a game"];
    let tech = [
        "Start", "Tech practice", "Tech wizard", "Surface", "Tech point", "Damage", "Damage range", "Launch speed",
        "Launch speed range", "Hit damage", "Hit damage range", "Angle", "Angle range", "Start position", "Start range",
        "Time to contact", "Time range", "DI", "DI direction", "Facing", "Delay before launch", "Delay range", "Camera",
        "Skip wizard preview",
    ];
    for (game, want) in [(Game::Eggs, &eggs[..]), (Game::Btt, &btt[..]), (Game::Tech, &tech[..])] {
        let group = table::game_group(game);
        assert_eq!(titles(group), want, "{game:?}");
        let first = ROWS.iter().find(|r| r.group == group).unwrap();
        assert!(matches!(first.control, Control::Start(g) if g == game));
        assert_eq!(menu().sub_rows(group).count(), want.len());
        for r in ROWS.iter().filter(|r| r.group == group) {
            let (a, b) = r.help_lines();
            assert!(!b.is_empty() && a.chars().count() <= 67 && b.chars().count() <= 67, "{}", r.title);
        }
    }
    // The same keys and controls as before the move (the Off/On `btt` row is Start / Stop now).
    assert!(ROWS.iter().all(|r| r.id != "btt"), "the btt row is gone; its key stays in the file");
    let control = |id: &str, group: u8| ROWS.iter().find(|r| r.id == id && r.group == group).map(|r| r.control);
    assert!(matches!(control("eggsercise_break", table::game_group(Game::Eggs)), Some(Control::Dropdown(["Any attack", "Aerials", "Hits of 12%+"]))));
    assert!(matches!(control("btt_place", table::game_group(Game::Btt)), Some(Control::Editor(Editor::PlaceTargets))));
    assert!(matches!(control("tech_wizard", table::game_group(Game::Tech)), Some(Control::Editor(Editor::TechWizard))));
}

#[test]
fn start_closes_the_menu_and_asks_for_that_game() {
    for game in Game::ALL {
        let mut m = opened(game);
        m.press(Button::A);
        assert!(m.is_closed(), "{game:?}");
        let out = m.take_outbox();
        assert_eq!((out.start, out.stop, out.changed), (Some(game), false, false), "Start changes no setting");
    }
}

#[test]
fn closing_the_menu_starts_nothing() {
    let mut m = opened(Game::Eggs);
    // Change the mode and close: the change is committed, nothing starts (Q1).
    m.press(Button::Down);
    m.press(Button::Right);
    m.press(Button::B);
    m.press(Button::B);
    assert!(m.is_closed());
    let out = m.take_outbox();
    assert_eq!((out.start, out.stop, out.changed), (None, false, true));
}

#[test]
fn b_leaves_a_running_game_running() {
    for game in Game::ALL {
        let mut m = running(game);
        on_list(&mut m, game);
        m.press(Button::A);
        m.press(Button::Down);
        m.press(Button::B);
        m.press(Button::B);
        assert!(m.is_closed());
        let out = m.take_outbox();
        assert_eq!((out.start, out.stop), (None, false), "{game:?}");
    }
}

#[test]
fn stop_ends_the_game_and_the_menu_stays_open() {
    let mut m = running(Game::Btt);
    on_list(&mut m, Game::Btt);
    m.press(Button::A);
    assert_eq!(frame(&m).rows[0].label.string_lossy(), "Stop");
    m.press(Button::A);
    assert!(!m.is_closed(), "Stop keeps the menu open");
    assert!(m.take_stop());
    assert!(!m.take_stop(), "taken once");
    assert_eq!((m.running(), frame(&m).rows[0].label.string_lossy()), (None, "Start".to_owned()));
    // Start again from the same row: it closes and starts.
    m.press(Button::A);
    assert_eq!((m.is_closed(), m.take_outbox().start), (true, Some(Game::Btt)));
    // Stopping one game never starts another.
    let mut m = running(Game::Eggs);
    on_list(&mut m, Game::Eggs);
    m.press(Button::A);
    m.press(Button::A);
    assert_eq!(m.take_outbox().start, None);
}

#[test]
fn running_shows_in_green_on_the_list_and_stop_on_the_sub_screen() {
    let mut m = running(Game::Eggs);
    on_list(&mut m, Game::Eggs);
    let f = frame(&m);
    let line = f.focus.map(|i| f.rows[usize::from(i)]).expect("the list row is focused");
    assert_eq!((line.label.string_lossy(), line.value.string_lossy(), line.value_tone), ("Eggs-ercise".into(), "Running  >".into(), Tone::On));
    assert_eq!(f.help[0].string_lossy(), "Eggs-ercise is running. A opens it to stop it or change it.");
    assert_eq!(f.help[1].string_lossy(), "Closing the menu (B) leaves it running.");
    let others: Vec<_> = f.rows.iter().filter(|r| r.label.string_lossy() == "Break the Targets" || r.label.string_lossy() == "Tech practice").map(|r| (r.value.string_lossy(), r.value_tone)).collect();
    assert_eq!(others, [(">".to_owned(), Tone::Normal), (">".to_owned(), Tone::Normal)]);
    m.press(Button::A);
    let f = frame(&m);
    assert_eq!((f.rows[0].label.string_lossy(), f.rows[0].value.string_lossy(), f.rows[0].value_tone), ("Stop".into(), "Running  >".into(), Tone::On));
    assert_eq!(f.help[0].string_lossy(), "Eggs-ercise is running. A stops it, and the row reads Start.");
    assert_eq!(f.help[1].string_lossy(), "Changed settings apply at the next Start. B leaves it running.");
    for game in Game::ALL {
        for line in [format!("{} is running. A opens it to stop it or change it.", game.title()), format!("{} is running. A stops it, and the row reads Start.", game.title())] {
            assert!(line.chars().count() <= 67, "{line}");
        }
    }
    // Not running: Start, ">".
    let f = frame(&opened(Game::Eggs));
    assert_eq!((f.rows[0].label.string_lossy(), f.rows[0].value.string_lossy(), f.rows[0].value_tone), ("Start".into(), ">".into(), Tone::Normal));
}

#[test]
fn mode_rows_have_no_off_and_a_stored_off_reads_as_the_first_mode() {
    let eggs = row("Eggs-ercise", table::game_group(Game::Eggs));
    let tech = row("Tech practice", table::game_group(Game::Tech));
    assert!(matches!(ROWS[eggs].control, Control::Dropdown(["1 minute", "10 targets", "20 targets", "Free practice"])));
    assert!(matches!(ROWS[tech].control, Control::Dropdown(["20 attempts", "10 attempts", "1 minute", "3 minutes", "Free practice"])));
    let mut m = menu();
    // The factory value (and every older file's Off) is index 0: it reads as the first mode.
    assert_eq!((m.get("eggsercise"), m.value(eggs)), (&[1, 0, 0, 0, 0][..], RowValue::Choice(Some(0))));
    assert_eq!((m.get("tech_practice"), m.value(tech)), (&[1, 0, 0, 0, 0, 0][..], RowValue::Choice(Some(0))));
    let mut label = crate::model::frame::Text::<72>::new();
    crate::model::frame::write_value(&m, eggs, &mut label);
    assert_eq!(label.string_lossy(), "<  1 minute  >");
    // Right steps to the second mode, stored one place on; Left from the first wraps to the last, never to Off.
    m.cycle(eggs, true);
    assert_eq!(m.get("eggsercise"), &[0, 0, 1, 0, 0]);
    m.cycle(eggs, false);
    m.cycle(eggs, false);
    assert_eq!(m.get("eggsercise"), &[0, 0, 0, 0, 1]);
    // The list: the current mode is marked, and A on the first stores index 1.
    m.put("eggsercise", &[1, 0, 0, 0, 0]);
    m.activate(eggs);
    assert_eq!(m.screen(), Some(&Screen::List { row: eggs, cursor: 0 }));
    let f = frame(&m);
    assert_eq!((f.row_count, f.rows[0].label.string_lossy(), f.rows[0].value.string_lossy()), (4, "1 minute".into(), "(current)".into()));
    m.press(Button::A);
    assert_eq!(m.get("eggsercise"), &[0, 1, 0, 0, 0]);
    m.activate(tech);
    m.press(Button::Down);
    m.press(Button::A);
    assert_eq!(m.get("tech_practice"), &[0, 0, 1, 0, 0, 0], "10 attempts");
    // One game's mode no longer touches another's (one at a time is the running game, not the rows).
    m.cycle(eggs, true);
    assert_eq!(m.get("tech_practice"), &[0, 0, 1, 0, 0, 0]);
}

#[test]
fn cpu_during_a_game_is_one_setting_in_two_screens() {
    let (in_eggs, in_btt) = (row("CPU during a game", table::game_group(Game::Eggs)), row("CPU during a game", table::game_group(Game::Btt)));
    assert_eq!((ROWS[in_eggs].id, ROWS[in_btt].id), ("practice_cpu", "practice_cpu"));
    let mut m = menu();
    m.cycle(in_eggs, true);
    assert_eq!((m.value(in_eggs), m.value(in_btt)), (RowValue::Choice(Some(1)), RowValue::Choice(Some(1))), "Show in both");
    m.cycle(in_btt, true);
    assert_eq!(m.get("practice_cpu"), &[1, 0]);
    assert_eq!(m.value(in_eggs), RowValue::Choice(Some(0)));
}

/// A settings.json written by `c212bc50` (before this change), every practice key set, including a hand-edited file with
/// all three games on: it loads with every stored value as it was (no migration, nothing normalised away), saves back the
/// same values, and the menu reads the modes from it.
#[test]
fn an_old_settings_file_loads_without_loss() {
    let old = r#"{"menu": {"eggsercise": [0,0,1,0,0], "eggsercise_break": [0,1,0], "eggsercise_target": [0,1], "eggsercise_speed": [0,0,1],
        "practice_cpu": [0,1], "btt": [0,1], "btt_projectiles": [1,0], "tech_practice": [0,0,0,0,1,0], "tech_surface": [0,0,1,0,0],
        "tech_wizard_skip": [1,0], "hide_cpu": [0,1]},
        "defaults_menu": {"eggsercise": [1,0,0,0,0], "btt": [0,1], "tech_practice": [0,1,0,0,0,0]}, "last_page": "fighters_stages"}"#;
    let loaded = settings::parse(old);
    assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
    let s = &loaded.settings;
    let want: [(&str, &[u8]); 9] = [
        ("eggsercise", &[0, 0, 1, 0, 0]), ("eggsercise_break", &[0, 1, 0]), ("eggsercise_target", &[0, 1]), ("eggsercise_speed", &[0, 0, 1]),
        ("practice_cpu", &[0, 1]), ("btt", &[0, 1]), ("btt_projectiles", &[1, 0]), ("tech_practice", &[0, 0, 0, 0, 1, 0]), ("tech_surface", &[0, 0, 1, 0, 0]),
    ];
    for (id, value) in want {
        assert_eq!(s.menu.get(id), Some(value), "{id}");
    }
    assert_eq!((s.defaults.get("btt"), s.defaults.get("tech_practice")), (Some(&[0, 1][..]), Some(&[0, 1, 0, 0, 0, 0][..])));
    // Saved back unchanged.
    let again = settings::parse(&s.to_json()).settings;
    for (id, value) in want {
        assert_eq!(again.menu.get(id), Some(value), "{id} after a save");
    }
    // The menu shows the stored modes: 10 targets and 3 minutes; nothing is running.
    let m = Menu::new(s.clone(), Env::default());
    assert_eq!(m.page(), Page::FightersStages);
    assert_eq!(m.value(row("Eggs-ercise", table::game_group(Game::Eggs))), RowValue::Choice(Some(1)));
    assert_eq!(m.value(row("Tech practice", table::game_group(Game::Tech))), RowValue::Choice(Some(3)));
    assert_eq!(m.running(), None);
    // A file's Off reads as the first mode, and the runtime runs it as that mode.
    let off = settings::parse(r#"{"menu": {"eggsercise": [1,0,0,0,0], "tech_practice": [1,0,0,0,0,0]}}"#).settings;
    let m = Menu::new(off, Env::default());
    assert_eq!(m.value(row("Eggs-ercise", table::game_group(Game::Eggs))), RowValue::Choice(Some(0)));
    assert_eq!(crate::model::practice_games::effective(Game::Eggs, 0, Some(Game::Eggs)), 1);
}

/// The Parry entry (slot 6): a used saved parry practice sets Parry's mode and asks for Parry's Start at the close (its
/// "close the menu and press ZR+Up"); any game's Start replaces that request, and Parry's mode stays as chosen.
#[test]
fn a_used_parry_practice_starts_parry_at_the_close() {
    let mut m = menu();
    m.parry_on(2);
    assert_eq!(m.get("parry_practice"), &[0, 0, 1]);
    assert!(!m.is_closed(), "the player closes the menu");
    let loaded = m.take_outbox();
    assert_eq!(loaded.start, Some(Game::Parry));
    assert!(loaded.resume_parry);
    for game in Game::ALL {
        let mut m = opened(game);
        m.parry_on(2);
        m.press(Button::A);
        let start = m.take_outbox();
        assert_eq!(start.start, Some(game));
        assert!(!start.resume_parry);
        assert_eq!(m.get("parry_practice"), &[0, 0, 1], "Parry's mode stays");
    }
    // The Parry sub-screen: Start, its mode (no Off), its saved practices.
    assert_eq!(titles(table::game_group(Game::Parry)), ["Start", "Parry practice", "Saved parry practices", "Save this parry practice"]);
    let off = settings::parse(r#"{"menu": {"parry_practice": [1,0,0]}}"#).settings;
    let m = Menu::new(off, Env::default());
    assert_eq!(m.value(row("Parry practice", table::game_group(Game::Parry))), RowValue::Choice(Some(0)), "a file's Off reads as 10 attempts");
}

#[test]
fn sub_screen_keys_match_the_page() {
    let mut m = opened(Game::Eggs);
    // Up from Start wraps to the last row; Down comes back.
    m.press(Button::Up);
    assert_eq!(m.help_row().map(|r| r.title), Some("CPU during a game"));
    m.press(Button::Down);
    m.press(Button::Down);
    m.press(Button::Down);
    // Breaks on: Left / Right change it in place, Y resets it, the help panel is its own.
    assert_eq!(m.help_row().map(|r| r.id), Some("eggsercise_break"));
    m.press(Button::Right);
    assert_eq!(m.get("eggsercise_break"), &[0, 1, 0]);
    let f = frame(&m);
    assert_eq!(f.help[0].string_lossy(), "Which of your hits break a target, projectiles included.");
    assert_eq!(f.rows[2].value.string_lossy(), "<  Aerials  >");
    m.press(Button::Y);
    assert_eq!(m.get("eggsercise_break"), &[1, 0, 0]);
    // A opens the row's list on top of the sub-screen; its crumb names the game; B returns to the sub-screen.
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::List { .. })));
    assert!(frame(&m).crumb.string_lossy().ends_with("Eggs-ercise  >  Breaks on"), "{:?}", frame(&m).crumb);
    m.press(Button::B);
    assert!(matches!(m.screen(), Some(Screen::Rows { cursor: 2, .. })));
    // ZL + Up / Down jump a window in the long Tech screen.
    let mut m = opened(Game::Tech);
    assert!(m.takes_jumps());
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::Rows { cursor: 12, .. })));
    assert_eq!(frame(&m).row_count as usize, crate::model::panes::ROWS, "the long screen scrolls in one window");
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::Rows { cursor: 23, .. })), "the window stops on the last row");
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::Rows { cursor: 0, .. })), "then wraps to the first");
    // A greyed row says why.
    let mut m = running(Game::Btt);
    on_list(&mut m, Game::Eggs);
    m.press(Button::A);
    for _ in 0..4 {
        m.press(Button::Down);
    }
    assert_eq!(frame(&m).help[1].string_lossy(), "Targets stay still");
}

#[test]
fn sub_screen_is_remembered_across_a_close() {
    let mut keeper = PlaceKeeper::new();
    let mut held = None;
    let m = keeper.open(&mut held, Settings::factory(), Env::default());
    on_list(m, Game::Tech);
    m.press(Button::A);
    for _ in 0..5 {
        m.press(Button::Down);
    }
    // A list open on top is put down; the sub-screen and its row come back.
    m.press(Button::A);
    keeper.drop_model(&mut held);
    let m = keeper.open(&mut held, Settings::factory(), Env::default());
    assert_eq!(m.screen(), Some(&Screen::Rows { group: table::game_group(Game::Tech), cursor: 5 }));
    // A sub-screen whose list row is not the focused one is not resumed.
    let place = m.place();
    let mut other = menu();
    other.restore_place(Place { screen: Some((Page::FightersStages, Screen::Rows { group: table::game_group(Game::Eggs), cursor: 0 })), ..place });
    assert_eq!(other.screen(), None);
}

#[test]
fn grey_rules_read_the_running_game() {
    let speed = row("Target speed", table::game_group(Game::Eggs));
    let cpu = [row("CPU during a game", table::game_group(Game::Eggs)), row("CPU during a game", table::game_group(Game::Btt))];
    let mut m = menu();
    // The rows of other games no longer grey anything.
    m.put("btt", &[0, 1]);
    m.put("tech_practice", &[0, 1, 0, 0, 0, 0]);
    assert_eq!((m.grey(speed), m.grey(cpu[0]), m.grey(cpu[1])), (None, None, None));
    let m = running(Game::Btt);
    assert_eq!((m.grey(speed), m.grey(cpu[0])), (Some(Grey::TargetsStill), None));
    let m = running(Game::Tech);
    assert_eq!((m.grey(speed), m.grey(cpu[0]), m.grey(cpu[1])), (None, Some(Grey::TechCpu), Some(Grey::TechCpu)));
    let m = running(Game::Eggs);
    assert_eq!((m.grey(speed), m.grey(cpu[0])), (None, None));
}

#[test]
fn place_targets_and_tech_views_start_their_game() {
    for (title, game, flags) in [
        ("Place targets", Game::Btt, (true, false, false)),
        ("Tech point", Game::Tech, (false, true, false)),
        ("Tech wizard", Game::Tech, (false, false, true)),
    ] {
        // From a stored Off the mode is untouched: the row shows the mode that runs.
        let mut m = running(Game::Eggs);
        m.activate(row(title, table::game_group(game)));
        assert!(m.is_closed(), "{title}");
        let out = m.take_outbox();
        assert_eq!(out.start, Some(game), "{title} starts its game (and so stops Eggs-ercise)");
        assert_eq!((out.place_targets, out.tech_point, out.tech_wizard), flags, "{title}");
        assert_eq!(m.get("tech_practice"), &[1, 0, 0, 0, 0, 0], "{title}");
    }
}

#[test]
fn reset_page_covers_the_sub_screen_rows() {
    let mut m = menu();
    m.put("eggsercise", &[0, 0, 0, 1, 0]);
    m.put("practice_cpu", &[0, 1]);
    m.put("tech_camera", &[0, 1]);
    m.page = Page::FightersStages;
    m.reset_page();
    let (_, changed) = m.take_page_reset().unwrap();
    assert_eq!((m.get("eggsercise"), m.get("practice_cpu"), m.get("tech_camera")), (&[1, 0, 0, 0, 0][..], &[1, 0][..], &[1, 0][..]));
    assert_eq!(changed.iter().filter(|id| **id == "practice_cpu").count(), 1, "one key, reset once");
    assert!(changed.contains(&"eggsercise") && changed.contains(&"tech_camera"));
}

/// parry-label-fix (2026-11-19): every practice game's mode row lists exactly the stored options after its retired Off,
/// each under its own name. A display index shows the label of the stored option it maps to, the counts match, the
/// list never offers "Off", and an older file's stored Off shows the first mode. The stored format does not change.
#[test]
fn every_mode_row_labels_the_stored_option_it_writes() {
    use crate::model::frame::{write_value, Text};
    // Each mode row's stored options by stored index, as the engines read them (index 0 is the retired Off).
    let named: [(&str, &[&str]); 3] = [
        ("eggsercise", &["Off", "1 minute", "10 targets", "20 targets", "Free practice"]),
        ("tech_practice", &["Off", "20 attempts", "10 attempts", "1 minute", "3 minutes", "Free practice"]),
        ("parry_practice", &["Off", "10 attempts", "Free practice"]),
    ];
    let rows: Vec<&str> = Game::ALL.iter().filter_map(|g| g.mode_row()).collect();
    assert_eq!(rows, named.map(|(id, _)| id), "every mode row is covered");
    for (id, names) in named {
        let r = ROWS.iter().position(|row| row.id == id).unwrap();
        let skip = crate::model::practice_games::retired(id);
        let stored = Settings::factory().menu.get(id).unwrap().len();
        assert_eq!((skip, stored), (1, names.len()), "{id}: one retired Off, then the modes");
        let m = menu();
        assert_eq!(m.option_count(r), stored - skip, "{id}: one display option per stored mode");
        for shown in 0..m.option_count(r) {
            let value = m.option_value(r, shown).unwrap();
            assert_eq!(value, shown + skip, "{id}");
            assert_eq!(m.dropdown_label(r, shown).map(|(label, _)| label), Some(names[value]), "{id}: display {shown} is stored {value}");
            let mut file = vec![0u8; stored];
            file[value] = 1;
            let mut settings = Settings::factory();
            settings.menu.set(id, &file).unwrap();
            let m = Menu::new(settings, Env::default());
            assert_eq!(m.value(r), RowValue::Choice(Some(shown)), "{id}: stored {value} reads back");
            let mut text = Text::<72>::new();
            write_value(&m, r, &mut text);
            assert_eq!(String::from_utf16(text.units()).unwrap(), format!("<  {}  >", names[value]), "{id}: stored {value}");
        }
        assert!((0..m.option_count(r)).all(|i| m.dropdown_label(r, i).is_some_and(|(label, _)| label != "Off")), "{id}");
        let mut off = vec![0u8; stored];
        off[0] = 1;
        let mut settings = Settings::factory();
        settings.menu.set(id, &off).unwrap();
        let m = Menu::new(settings, Env::default());
        let mut text = Text::<72>::new();
        write_value(&m, r, &mut text);
        assert_eq!(String::from_utf16(text.units()).unwrap(), format!("<  {}  >", names[1]), "{id}: a stored Off shows the first mode");
    }
}
}
