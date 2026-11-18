//! Regression for Loop list confirmation, cancellation and its existing shared cycling setter.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "support/playback_engine.rs"]
#[allow(dead_code)]
pub mod engine;

use engine::library::{LoopCount, RecordingIndex, Role};
use engine::macros::{InputFrame, MacroRecording, StartState};
use engine::playback::{Book, Playback};
use model::menu::input::Button;
use model::menu::table::ROWS;
use model::menu::{Entry, Env, Menu, Screen, LOOP_ROW};
use model::recording_menu::{LoopMark, ViewRole};
use model::settings::Settings;
use std::path::PathBuf;

const COUNTS: [(LoopMark, Option<usize>); 6] = [
    (LoopMark::Infinite, None),
    (LoopMark::Finite(1), Some(1)),
    (LoopMark::Finite(2), Some(2)),
    (LoopMark::Finite(3), Some(3)),
    (LoopMark::Finite(5), Some(5)),
    (LoopMark::Finite(10), Some(10)),
];

fn focus(menu: &mut Menu, target: Entry) {
    for _ in 0..=menu.entry_count(menu.page()) {
        if menu.focus() == Some(target) { return; }
        menu.press(Button::Down);
    }
    panic!("cannot focus {target:?}");
}

fn menu(role: ViewRole) -> Menu {
    let mut menu = Menu::new(Settings::factory(), Env::default());
    let row = ROWS.iter().position(|row| row.id == LOOP_ROW).unwrap();
    for _ in 0..6 {
        if menu.page() == ROWS[row].page() { break; }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), ROWS[row].page());
    menu.recording_mut().adopt(
        None,
        [Some("macro-a".into()), Some("macro-a".into())],
        [LoopMark::Finite(3), LoopMark::Finite(5)],
        None,
    );
    if role == ViewRole::Cpu { menu.press(Button::ZR); }
    let group = usize::from(ROWS[row].group);
    if !menu.group_is_open(group) {
        focus(&mut menu, Entry::Group(group));
        menu.press(Button::A);
    }
    focus(&mut menu, Entry::Row(row));
    assert_eq!(menu.recording().page_role(), role);
    menu
}

fn pick(menu: &mut Menu, index: usize) {
    menu.press(Button::A);
    for _ in 0..6 {
        if matches!(menu.screen(), Some(Screen::List { cursor, .. }) if *cursor == index) { break; }
        menu.press(Button::Down);
    }
    assert!(matches!(menu.screen(), Some(Screen::List { cursor, .. }) if *cursor == index));
    menu.press(Button::A);
    assert!(menu.screen().is_none());
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("tr-loop-picker-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

/// Use the worker's real publication seam, reload its index, then execute the real playback machine.
fn assert_playback(menu: &Menu, role: ViewRole, expected: Option<usize>) {
    let root = Scratch::new();
    engine::rr_host::publish_selection(&root.0, &menu.recording().selection(), None);
    let index = RecordingIndex::load(&root.0).unwrap();
    let role = if role == ViewRole::Player { Role::Player } else { Role::Cpu };
    assert_eq!(
        index.loop_count(role),
        expected.map_or(LoopCount::Infinite, |n| LoopCount::Finite(n as u8)),
    );
    let other = if role == Role::Player { Role::Cpu } else { Role::Player };
    let other_view = if role == Role::Player { ViewRole::Cpu } else { ViewRole::Player };
    let other_count = match menu.recording().loop_of(other_view) {
        LoopMark::Infinite => LoopCount::Infinite,
        LoopMark::Finite(n) => LoopCount::Finite(n),
    };
    assert_eq!(index.loop_count(other), other_count);
    let mut book = Book::new(index);
    let frames = [0x10, 0x20].map(|buttons| InputFrame { buttons, ..InputFrame::default() }).to_vec();
    book.add_recording("macro-a", MacroRecording::new("Two frames", "13.0.5", StartState::neutral(0, 0, 1.0), frames).unwrap());
    let mut playback = Playback::new();
    assert!(playback.play_chord(role, &book).started);
    for i in 0..expected.map_or(24, |n| n * 2) {
        assert_eq!(playback.step(role, &book).expect("selected pass").frame.buttons, [0x10, 0x20][i % 2]);
    }
    if expected.is_none() {
        assert!(playback.playing(role), "infinite continues beyond the finite test window");
        assert!(playback.play_chord(role, &book).stopped);
    }
    assert!(playback.step(role, &book).is_none(), "no extra pass after a finite count or explicit stop");
    assert!(!playback.playing(role));
}

#[test]
fn loop_list_selection_is_published_and_used_by_playback() {
    for role in [ViewRole::Player, ViewRole::Cpu] {
        for (index, (mark, passes)) in COUNTS.into_iter().enumerate() {
            let mut menu = menu(role);
            let other = menu.recording().loop_of(role.other());
            pick(&mut menu, index);
            assert_eq!(menu.recording().loop_of(role), mark);
            assert_eq!(menu.recording().loop_of(role.other()), other);
            assert_playback(&menu, role, passes);
        }
    }
}

#[test]
fn loop_list_cancel_keeps_the_previous_playback_count() {
    for (role, passes) in [(ViewRole::Player, 3), (ViewRole::Cpu, 5)] {
        let mut menu = menu(role);
        let before = menu.recording().selection();
        menu.press(Button::A);
        menu.press(Button::Down);
        menu.press(Button::B);
        assert!(menu.screen().is_none());
        assert_eq!(menu.recording().selection(), before);
        assert_playback(&menu, role, Some(passes));
    }
}

#[test]
fn loop_list_and_left_right_choose_the_same_counts() {
    for role in [ViewRole::Player, ViewRole::Cpu] {
        for button in [Button::Left, Button::Right] {
            for (target, (mark, passes)) in COUNTS.into_iter().enumerate() {
                let mut listed = menu(role);
                let mut cycled = menu(role);
                pick(&mut listed, target);
                for _ in 0..6 {
                    if cycled.recording().loop_row_index() == target { break; }
                    cycled.press(button);
                }
                assert_eq!(cycled.recording().loop_of(role), mark);
                assert_eq!(cycled.recording().selection(), listed.recording().selection());
                assert_playback(&cycled, role, passes);
            }
        }
    }
}
