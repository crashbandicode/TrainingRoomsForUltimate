//! The Training intro notices show once, on the first Training entry of a new install.

// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
use crate::model;


use model::boot_notices::{self, BootNotices};
use model::chords::ChordSet;
use model::menu::input::Button;
use model::menu::table::Page;
use model::menu::{Entry, Env, Menu};
use model::notify::{Queue, DEFAULT_COLOR};
use model::recording::{Phase, NOTICE_HEADER};
use model::settings::Settings;
use std::path::Path;

fn src(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

fn item<'a>(text: &'a str, head: &str) -> &'a str {
    let at = text.find(head).unwrap_or_else(|| panic!("{head} not found"));
    let end = text[at..].find("\n}\n").map_or(text.len(), |e| at + e + 3);
    &text[at..end]
}

fn the_waiting_rooms_hud_feeds_the_first_install_notices() {
    let hud = src("src/runtime/features/hud_draw.rs");
    let tick = hud.find("notices::hud_tick(").expect("HUD posts notices");
    let tick = &hud[tick..hud[tick..].find(");").expect("end of HUD tick") + tick + 2];
    assert!(tick.contains("boot_notices::hud_ready(!notice.is_null(), waiting,"), "HUD uses the practice readiness rule");
    assert!(tick.contains("training::session_open()"), "HUD checks the opened room session");
    assert!(!hud.contains("hud_tick(!waiting"), "waiting-room HUD is not excluded");

    let training = src("src/runtime/lobby/training/mod.rs");
    let clear = item(&training, "pub(crate) fn clear_session_state()");
    let recorder = clear.find("recorder::session_clear();").expect("recorder clears the notices");
    let store = clear.find("SESSION_OPEN.store(crate::runtime::scene::lobby_active(),").expect("session follows lobby flag");
    assert!(recorder < store, "the session flag is published after the recorder clears");
    let end_store = store + clear[store..].find(';').expect("end of session store") + 1;
    assert_eq!(clear[end_store..].trim(), "}", "the session store is the clear's last statement");
    assert!(item(&training, "pub(crate) fn session_open()").contains("SESSION_OPEN.load"));
    assert_eq!(training.matches("SESSION_OPEN.store").count(), 1);

    let lobby = src("src/runtime/lobby/training/lobby.rs");
    let update = item(&lobby, "pub fn update()");
    let edge = &update[update.find("if now != previous {").expect("activation edge")..];
    let edge = &edge[..edge.find("\n    }\n").expect("end of activation edge")];
    let set_lobby = edge.find("set_lobby(now)").expect("activation updates CPU scene");
    let clear = edge.find("super::clear_session_state();").expect("activation clears the session");
    assert!(set_lobby < clear, "set_lobby precedes the session clear");
    assert!(!lobby.contains("SESSION_OPEN"), "the carried file does not own the session flag");

    let notices = src("src/runtime/features/notices.rs");
    assert!(notices.contains("pub fn hud_tick(hud_ready: bool)"));
    assert!(!notices.contains("native_hud_ready"));
}

fn the_first_install_notices_wait_for_the_rooms_session() {
    assert!(boot_notices::hud_ready(true, false, false));
    assert!(boot_notices::hud_ready(true, false, true));
    assert!(!boot_notices::hud_ready(true, true, false));
    assert!(boot_notices::hud_ready(true, true, true));
    for waiting in [false, true] {
        for room_session_open in [false, true] {
            assert!(!boot_notices::hud_ready(false, waiting, room_session_open));
        }
    }

    let mut notices = BootNotices::for_install(false, &Settings::factory());
    let chords = Some(ChordSet::factory());
    let mut queue = Queue::new();
    assert!(!notices.post_once(boot_notices::hud_ready(true, true, false), chords, &mut queue));
    assert_eq!(queue.len(), 0);
    queue.clear();
    assert!(notices.post_once(boot_notices::hud_ready(true, true, true), chords, &mut queue));
    assert_eq!(queue.len(), 6);
    let draw = queue.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("TrainingRooms", "Welcome!"));
    assert!(!notices.post_once(boot_notices::hud_ready(true, true, true), chords, &mut queue));

    let mut already_seen = BootNotices::for_install(true, &Settings::factory());
    let mut empty = Queue::new();
    assert!(!already_seen.post_once(boot_notices::hud_ready(true, true, true), chords, &mut empty));
    assert_eq!(empty.len(), 0);
}

fn fresh_notices(file_existed: bool, settings: &Settings) -> BootNotices {
    BootNotices::for_install(file_existed, settings)
}

fn the_intro_seen_flag_survives_a_settings_round_trip() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let text = settings.to_json();
    assert!(text.contains("\"intro_seen\": true") || text.contains("\"intro_seen\":true"));
    let loaded = model::settings::parse(&text).settings;
    assert!(loaded.intro_seen());
    assert!(loaded.to_json().contains("intro_seen"));
    let again = model::settings::parse(&loaded.to_json()).settings;
    assert!(again.intro_seen());
    assert!(boot_notices::intro_already_seen(true, &again));
}

fn save_and_other_notices_still_post() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let mut notices = fresh_notices(true, &settings);
    let mut queue = Queue::new();
    assert!(!notices.post_once(true, Some(ChordSet::factory()), &mut queue));
    boot_notices::saved(&mut queue, 0);
    let draw = queue.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Save State", "Saved"));
    queue.clear();
    let (body, color) = Phase::Recording.notice().unwrap();
    queue.post_static(NOTICE_HEADER, body, 60, color);
    let draw = queue.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), (NOTICE_HEADER, "Recording"));
    assert_ne!(draw.color, DEFAULT_COLOR);

    let mut menu = Menu::new(settings, Env::default());
    to_page(&mut menu, Page::FightersStages);
    goto_reset(&mut menu);
    menu.press(Button::A);
    menu.press(Button::A);
    assert!(menu.take_page_reset().is_some());
}

fn to_page(menu: &mut Menu, page: Page) {
    for _ in 0..6 {
        if menu.page() == page {
            return;
        }
        menu.press(Button::R);
    }
    panic!("page {page:?} not reached");
}

fn goto_reset(menu: &mut Menu) {
    let last = menu.entry_count(menu.page()) - 1;
    assert!(matches!(menu.entry(menu.page(), last), Some(Entry::ResetPage(_))));
    while menu.cursor() != last {
        menu.press(Button::Down);
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_intro_seen_flag_survives_a_settings_round_trip", the_intro_seen_flag_survives_a_settings_round_trip as fn()),
        ("save_and_other_notices_still_post", save_and_other_notices_still_post as fn()),
        ("the_waiting_rooms_hud_feeds_the_first_install_notices", the_waiting_rooms_hud_feeds_the_first_install_notices as fn()),
        ("the_first_install_notices_wait_for_the_rooms_session", the_first_install_notices_wait_for_the_rooms_session as fn()),
    ]);
}
