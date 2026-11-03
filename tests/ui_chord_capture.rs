//! N7 original physical-input regression: never injects Menu::chord_entry.
//! Baseline delivery is the actual press-only input.rs -> worker.rs boundary.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::adapter::{bit, Adapter, Style};
use model::menu::chord::Chord;
use model::menu::input::Button;
use model::menu::table::ROWS;
use model::menu::{Entry, Env, Menu, Screen};

fn shortcut_editor() -> Menu {
    let mut menu = Menu::new(model::settings::Settings::factory(), Env::default());
    let row = ROWS.iter().position(|row| row.title == "Menu shortcut 1").unwrap();
    for _ in 0..6 {
        if menu.page() == ROWS[row].page() { break; }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), ROWS[row].page());
    for _ in 0..menu.entry_count(menu.page()) {
        if menu.focus() == Some(Entry::Row(row)) { break; }
        menu.press(Button::Down);
    }
    assert_eq!(menu.focus(), Some(Entry::Row(row)));
    menu.press(Button::A);
    assert!(matches!(menu.screen(), Some(Screen::ChordEntry { .. })));
    menu
}

fn physical_poll(menu: &mut Menu, adapter: &mut Adapter, raw: u32) {
    let step = adapter.poll(raw, Style::Pro, true, false, menu.env().shortcuts);
    for button in step.presses() { menu.press(button); }
}

#[test]
fn n7_physical_zl_up_is_captured_without_direct_model_injection() {
    let mut menu = shortcut_editor();
    let mut adapter = Adapter::new();
    for raw in [1 << bit::X, 0, 1 << bit::ZL,
            (1 << bit::ZL) | (1 << bit::DPAD_UP), 0] {
        physical_poll(&mut menu, &mut adapter, raw);
    }
    // Independent spec wire-order oracle: logical ZL bit6, D-pad Up bit8.
    let expected = Chord((1 << 6) | (1 << 8));
    assert!(matches!(menu.screen(), Some(Screen::ChordEntry { captured, .. }) if *captured == expected),
        "physical polls must replace the preview, not just navigate: {:?}", menu.screen());
    assert_eq!(menu.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1,
        "capture alone must not confirm an edit");
    physical_poll(&mut menu, &mut adapter, 1 << bit::A);
    assert_eq!(menu.env().shortcuts[0], expected);
    assert_eq!(menu.take_outbox().button_config, Some([expected, Chord::NONE]));
}
