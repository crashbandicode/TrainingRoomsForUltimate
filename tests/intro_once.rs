//! The Training intro notices show once, on the first Training entry of a new install.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;


use model::boot_notices::{self, BootNotices};
use model::chords::ChordSet;
use model::menu::input::Button;
use model::menu::table::Page;
use model::menu::{Entry, Env, Menu};
use model::notify::{Queue, DEFAULT_COLOR};
use model::recording::{Phase, NOTICE_HEADER};
use model::settings::Settings;

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
    ]);
}
