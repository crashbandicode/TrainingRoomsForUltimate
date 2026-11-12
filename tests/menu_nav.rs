//! Menu navigation (operator request 2026-11-05, branch `p23/menu-nav`): ZL + Up / Down jumps between the sections of a
//! page and by a window in the popup lists, and the menu comes back where it was when it opens again (`Menu::place` /
//! `Menu::restore_place`; the page part extends Q5, "the menu opens on the last page used", 2026-11-01).
//!
//! Decided here and documented in the model: the section jump does not wrap (Up / Down still do); the place is
//! kept for the boot by the runtime and clamped to the table as it stands at the opening.

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

use model::menu::input::Button;
use model::menu::table::{Control, Editor, Page, ROWS};
use model::menu::{Entry, Env, Menu, Screen, LIST_JUMP};
use model::settings::Settings;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn to_page(m: &mut Menu, page: Page) {
    for _ in 0..6 {
        if m.page() == page {
            return;
        }
        m.press(Button::R);
    }
    panic!("page {page:?} not reached");
}

fn group_of(entry: Entry) -> usize {
    match entry {
        Entry::Group(g) | Entry::ResetPage(g) => g,
        Entry::Row(r) => usize::from(ROWS[r].group),
    }
}

/// The groups of `page` with a visible row, in order, as the menu lists them.
fn sections(m: &Menu, page: Page) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..m.entry_count(page) {
        if let Some(Entry::Group(g)) = m.entry(page, i) {
            out.push(g);
        }
    }
    out
}

fn section(m: &Menu) -> usize {
    group_of(m.focus().expect("a focused entry"))
}

fn row_of(id: &str) -> usize {
    ROWS.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("no row {id}"))
}

fn goto_row(m: &mut Menu, row: usize) {
    to_page(m, ROWS[row].page());
    for _ in 0..=m.entry_count(m.page()) {
        if m.focused_row() == Some(row) {
            return;
        }
        m.press(Button::Down);
    }
    panic!("row {} not reached", ROWS[row].title);
}

// ---- section jump on a page ------------------------------------------------------------------------------------

fn the_jump_works_on_every_page_and_never_leaves_it() {
    let mut m = menu();
    for page in [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls] {
        to_page(&mut m, page);
        for _ in 0..12 {
            m.press(Button::JumpDown);
            assert_eq!(m.page(), page);
        }
        // On the Recording page ZL+Up/Down pages the library section by 12. It does not walk the left column.
        if page != Page::Recording {
            assert_eq!(section(&m), *sections(&m, page).last().unwrap());
        }
        for _ in 0..12 {
            m.press(Button::JumpUp);
        }
        assert_eq!(section(&m), sections(&m, page)[0]);
    }
}

// ---- jump in the popup lists -----------------------------------------------------------------------------------

fn zl_jumps_in_a_dropdown_list_and_a_delay_popup() {
    let mut m = menu();
    let delay = row_of("input_delay");
    goto_row(&mut m, delay);
    m.press(Button::A);
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::List { cursor, .. }) if *cursor == LIST_JUMP));
    m.press(Button::B);
    let ledge = row_of("ledge_delay");
    goto_row(&mut m, ledge);
    m.press(Button::A);
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::Delay { cursor, editing: false, .. }) if *cursor == LIST_JUMP));
}

// ---- the remembered place -------------------------------------------------------------------------------------


// ---- Reset page to defaults (operator 2026-11-05: no hidden ZR reset; an explicit, confirmed line) ---------------

fn zr_on_a_page_changes_nothing() {
    // Rows changed on three pages: the old ZR would have put each back.
    let mut m = menu();
    goto_row(&mut m, row_of("sdi_strength"));
    m.press(Button::Right);
    for id in ["trio_practice", "hud"] {
        goto_row(&mut m, row_of(id));
        m.press(Button::A);
    }
    for page in [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls] {
        to_page(&mut m, page);
        let _ = m.take_outbox();
        let (settings, cursor) = (m.settings().clone(), m.cursor());
        m.press(Button::ZR);
        assert_eq!(m.settings(), &settings, "{page:?}");
        assert_eq!((m.cursor(), m.screen().copied()), (cursor, None));
        assert!(!m.take_outbox().changed);
        assert!(m.take_page_reset().is_none());
    }
}

// ---- where ZL + Up / Down are jumps (review MN-1) ------------------------------------------------------------------

// ---- the worker's keeper (review MN-4) -------------------------------------------------------------------------------

/// The settings the worker reads at an opening: the committed file, whose `last_page` a close wrote.
fn committed(m: &Menu) -> Settings {
    let mut s = m.settings().clone();
    s.set_last_page(m.page().id());
    s
}

/// The worker's path, as `runtime::ui::worker` runs it: `drop_model` whenever the input generation moves (a close, a
/// scene change, a Training reset, a lobby change), `open` at the next opening.
fn the_keeper_brings_the_menu_back_across_closes_scenes_and_resets() {
    use model::menu::PlaceKeeper;
    let mut keeper = PlaceKeeper::new();
    let mut slot: Option<Menu> = None;
    // First opening of the boot: no place yet, the factory page.
    let m = keeper.open(&mut slot, Settings::factory(), Env::default());
    assert_eq!(m.page(), Page::Cpu);
    to_page(m, Page::FightersStages);
    m.press(Button::JumpDown);
    m.press(Button::Down);
    let (page, cursor) = (m.page(), m.cursor());
    // A close: the model is dropped, the place kept.
    m.press(Button::B);
    let settings = committed(slot.as_ref().unwrap());
    keeper.drop_model(&mut slot);
    assert!(slot.is_none());
    let m = keeper.open(&mut slot, settings.clone(), Env::default());
    assert_eq!((m.page(), m.cursor()), (page, cursor), "reopened where it closed");
    // A scene change with a checklist open (no close, no last_page commit): the checklist comes back.
    goto_row(m, row_of("mash_state"));
    m.press(Button::A);
    m.press(Button::JumpDown);
    let open = *m.screen().unwrap();
    keeper.drop_model(&mut slot);
    // A Training reset and a lobby change move the generation again with no model: the place stays.
    keeper.drop_model(&mut slot);
    keeper.drop_model(&mut slot);
    let m = keeper.open(&mut slot, settings.clone(), Env::default());
    assert_eq!(m.page(), Page::Cpu, "the page of the dropped model, not the file's last_page");
    assert_eq!(m.screen(), Some(&open));
    assert_eq!(m.focused_row(), Some(row_of("mash_state")));
    // A grid open at a drop is not resumed: its row is focused.
    m.press(Button::B);
    let items = ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::GiveItem))).unwrap();
    goto_row(m, items);
    m.press(Button::A);
    keeper.drop_model(&mut slot);
    let m = keeper.open(&mut slot, settings.clone(), Env::default());
    assert_eq!((m.screen(), m.focused_row()), (None, Some(items)));
    // A model still held at an opening is reopened with the new settings and brought back to the kept place.
    let kept = keeper.place();
    let mut fresh = settings.clone();
    fresh.set_last_page(Page::Saves.id());
    let m = keeper.open(&mut slot, fresh, Env::default());
    assert_eq!(m.focused_row(), Some(items));
    assert_eq!(keeper.place(), kept, "opening keeps the place; only a drop replaces it");
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_jump_works_on_every_page_and_never_leaves_it", the_jump_works_on_every_page_and_never_leaves_it as fn()),
        ("zl_jumps_in_a_dropdown_list_and_a_delay_popup", zl_jumps_in_a_dropdown_list_and_a_delay_popup as fn()),
        ("zr_on_a_page_changes_nothing", zr_on_a_page_changes_nothing as fn()),
        ("the_keeper_brings_the_menu_back_across_closes_scenes_and_resets", the_keeper_brings_the_menu_back_across_closes_scenes_and_resets as fn()),
    ]);
}
