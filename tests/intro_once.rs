//! The Training intro notices show once, on the first Training entry of a new install.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use model::boot_notices::{self, BootNotices};
use model::chords::ChordSet;
use model::menu::input::Button;
use model::menu::table::{Page, ROWS};
use model::menu::{Entry, Env, Menu, Screen};
use model::notify::{Queue, DEFAULT_COLOR};
use model::recording::{Phase, NOTICE_HEADER};
use model::settings::Settings;
use model::settings_file;

const INTRO_HEADERS: [&str; 6] = [
    "TrainingRooms",
    "Open Menu",
    "Save State",
    "Load State",
    "Input Record",
    "Input Playback",
];

fn scratch() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tr-intro-once-{}-{}-{}",
        std::process::id(),
        nanos,
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn headers_of(queue: &mut Queue) -> Vec<String> {
    let mut headers = Vec::new();
    for _ in 0..2000 {
        let out = queue.tick(true, false);
        if let Some(draw) = out.draw {
            headers.push(draw.header.to_string());
        }
        if !out.visible {
            break;
        }
    }
    headers
}

fn fresh_notices(file_existed: bool, settings: &Settings) -> BootNotices {
    BootNotices::for_install(file_existed, settings)
}

#[test]
fn the_first_training_entry_of_a_new_install_posts_the_intro_once() {
    let dir = scratch();
    let boot = settings_file::load(&dir);
    assert!(!boot.file_existed, "a directory with no settings file is a new install");
    assert!(!boot.settings.intro_seen());
    let mut notices = fresh_notices(boot.file_existed, &boot.settings);
    let mut queue = Queue::new();
    let chords = ChordSet::factory();
    assert!(notices.post_once(true, Some(chords), &mut queue));
    let mut saw = queue.tick(true, false);
    assert_eq!(saw.draw.unwrap().body.as_str(), "Welcome!");
    let mut headers = vec!["TrainingRooms".to_string()];
    for _ in 0..2000 {
        saw = queue.tick(true, false);
        if let Some(draw) = saw.draw {
            headers.push(draw.header.to_string());
        }
        if !saw.visible {
            break;
        }
    }
    assert_eq!(headers, INTRO_HEADERS);
    assert!(!notices.post_once(true, Some(chords), &mut queue));
    assert!(queue.is_empty());
    let mut settings = boot.settings;
    settings.mark_intro_seen();
    assert!(settings.intro_seen());
    let stored = model::settings::parse(&settings.to_json()).settings;
    assert!(stored.intro_seen());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_later_training_entry_posts_no_intro_notices() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let mut notices = fresh_notices(true, &settings);
    let mut queue = Queue::new();
    assert!(!notices.post_once(true, Some(ChordSet::factory()), &mut queue));
    assert!(headers_of(&mut queue).is_empty());
    for header in INTRO_HEADERS {
        assert!(!queue_contains(&queue, header));
    }
}

#[test]
fn the_intro_seen_flag_survives_a_settings_round_trip() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let text = settings.to_json();
    assert!(text.contains("\"intro_seen\": true") || text.contains("\"intro_seen\":true"));
    let loaded = model::settings::parse(&text).settings;
    assert!(loaded.intro_seen());
    assert_eq!(loaded.to_json().contains("intro_seen"), true);
    let again = model::settings::parse(&loaded.to_json()).settings;
    assert!(again.intro_seen());
    assert!(boot_notices::intro_already_seen(true, &again));
}

#[test]
fn an_existing_install_counts_as_seen() {
    let dir = scratch();
    fs::write(dir.join("settings.json"), r#"{"menu":{"hud":[1,0]},"last_page":"cpu"}"#).unwrap();
    let boot = settings_file::load(&dir);
    assert!(boot.file_existed);
    assert!(!boot.settings.intro_seen(), "the file has no intro flag");
    assert!(boot_notices::intro_already_seen(boot.file_existed, &boot.settings));
    let mut notices = fresh_notices(boot.file_existed, &boot.settings);
    let mut queue = Queue::new();
    assert!(!notices.post_once(true, Some(ChordSet::factory()), &mut queue));
    assert!(queue.is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn save_and_other_notices_still_post() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let mut notices = fresh_notices(true, &settings);
    let mut queue = Queue::new();
    assert!(!notices.post_once(true, Some(ChordSet::factory()), &mut queue));
    boot_notices::saved(&mut queue, 0);
    let draw = queue.tick(true, false).draw.unwrap();
    assert_eq!((draw.header, draw.body.as_str()), ("Save State", "Saved Slot 1"));
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

#[test]
fn a_page_reset_does_not_reset_the_intro_flag() {
    let mut settings = Settings::factory();
    settings.mark_intro_seen();
    let mut menu = Menu::new(settings.clone(), Env::default());
    let row = row_of("trio_practice");
    goto_row(&mut menu, row);
    menu.press(Button::A);
    to_page(&mut menu, Page::FightersStages);
    goto_reset(&mut menu);
    menu.press(Button::A);
    assert!(matches!(menu.screen(), Some(&Screen::ConfirmReset)));
    menu.press(Button::A);
    assert!(menu.settings().intro_seen());
    let merged = menu.commit_onto(&settings);
    assert!(merged.intro_seen());
    assert!(merged.to_json().contains("intro_seen"));
    let mut again = fresh_notices(true, &merged);
    let mut queue = Queue::new();
    assert!(!again.post_once(true, Some(ChordSet::factory()), &mut queue));
}

fn queue_contains(queue: &Queue, header: &str) -> bool {
    let mut copy = queue.clone();
    headers_of(&mut copy).iter().any(|h| h == header)
}

fn row_of(id: &str) -> usize {
    ROWS.iter().position(|row| row.id == id).unwrap()
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

fn goto_row(menu: &mut Menu, row: usize) {
    to_page(menu, ROWS[row].page());
    for _ in 0..=menu.entry_count(menu.page()) {
        if menu.focused_row() == Some(row) {
            return;
        }
        menu.press(Button::Down);
    }
    panic!("row {} not reached", ROWS[row].title);
}

fn goto_reset(menu: &mut Menu) {
    let last = menu.entry_count(menu.page()) - 1;
    assert!(matches!(menu.entry(menu.page(), last), Some(Entry::ResetPage(_))));
    while menu.cursor() != last {
        menu.press(Button::Down);
    }
}
