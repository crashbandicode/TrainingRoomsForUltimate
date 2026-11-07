//! Menu navigation (operator request 2026-11-05, branch `p23/menu-nav`): ZL + Up / Down jumps between the sections of a
//! page and by a window in the popup lists, and the menu comes back where it was when it opens again (`Menu::place` /
//! `Menu::restore_place`; the page part extends Q5, "the menu opens on the last page used", 2026-11-01).
//!
//! Decided here and documented in the model: the section jump does not wrap (Up / Down still do); the place is
//! kept for the boot by the runtime and clamped to the table as it stands at the opening.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::menu::input::Button;
use model::menu::table::{Control, Editor, Page, GROUPS, ROWS};
use model::menu::{Entry, Env, Menu, Screen, LIST_JUMP};
use model::settings::Settings;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

/// The model the runtime builds at the next opening: the old model is dropped at every close.
fn reopened(old: &Menu, env: Env) -> Menu {
    let place = old.place();
    let mut settings = old.settings().clone();
    settings.set_last_page(old.page().id());
    let mut m = Menu::new(settings, env);
    m.restore_place(place);
    m
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

#[test]
fn zl_down_walks_every_section_of_the_cpu_page_and_stops_at_the_last() {
    let mut m = menu();
    let order = sections(&m, Page::Cpu);
    assert!(order.len() >= 5, "the CPU page has its sections: {order:?}");
    assert_eq!(section(&m), order[0]);
    for want in &order[1..] {
        m.press(Button::JumpDown);
        assert_eq!(section(&m), *want);
        // It lands on the first row of the section (the group header is not focusable there).
        let first_row = ROWS.iter().position(|r| usize::from(r.group) == *want && r.visible(false)).unwrap();
        assert_eq!(m.focused_row(), Some(first_row), "{}", GROUPS[*want].title);
    }
    // No wrap: on the last section the cursor stays.
    let last = m.cursor();
    m.press(Button::JumpDown);
    assert_eq!(m.cursor(), last);
}

#[test]
fn zl_up_walks_back_and_stops_at_the_first_section() {
    let mut m = menu();
    let order = sections(&m, Page::Cpu);
    for _ in 1..order.len() {
        m.press(Button::JumpDown);
    }
    // From the middle of the last section, ZL + Up goes to the previous section's first row.
    m.press(Button::Down);
    for want in order[..order.len() - 1].iter().rev() {
        m.press(Button::JumpUp);
        assert_eq!(section(&m), *want);
    }
    let first = m.cursor();
    m.press(Button::JumpUp);
    assert_eq!(m.cursor(), first, "no wrap at the first section");
    // Up / Down keep wrapping as before.
    m.press(Button::Up);
    assert_eq!(section(&m), *order.last().unwrap());
}

#[test]
fn a_collapsed_advanced_group_is_a_section_and_its_header_takes_the_jump() {
    let mut m = menu();
    to_page(&mut m, Page::FightersStages);
    let order = sections(&m, Page::FightersStages);
    let loading = *order.last().unwrap();
    assert!(GROUPS[loading].advanced, "Loading is the page's advanced group");
    for _ in 1..order.len() {
        m.press(Button::JumpDown);
    }
    assert_eq!(m.focus(), Some(Entry::Group(loading)));
    // Opened, the jump still lands on the header first (the first focusable entry of the section).
    m.press(Button::A);
    m.press(Button::JumpUp);
    m.press(Button::JumpDown);
    assert_eq!(m.focus(), Some(Entry::Group(loading)));
}

#[test]
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

#[test]
fn zl_jumps_a_window_in_a_checklist_and_stops_at_its_ends() {
    let mut m = menu();
    goto_row(&mut m, row_of("mash_state"));
    m.press(Button::A);
    let len = m.option_count(row_of("mash_state"));
    assert!(len > 2 * LIST_JUMP, "Mash lists {len} actions");
    m.press(Button::JumpDown);
    assert!(matches!(m.screen(), Some(Screen::Checklist { cursor, .. }) if *cursor == LIST_JUMP));
    for _ in 0..5 {
        m.press(Button::JumpDown);
    }
    assert!(matches!(m.screen(), Some(Screen::Checklist { cursor, .. }) if *cursor == len - 1));
    for _ in 0..5 {
        m.press(Button::JumpUp);
    }
    assert!(matches!(m.screen(), Some(Screen::Checklist { cursor: 0, .. })));
}

#[test]
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

#[test]
fn the_list_jump_is_one_window_of_the_drawn_lines() {
    assert_eq!(LIST_JUMP, model::panes::ROWS);
}

// ---- the remembered place -------------------------------------------------------------------------------------

#[test]
fn the_menu_reopens_on_the_same_page_and_row() {
    let mut m = menu();
    to_page(&mut m, Page::FightersStages);
    m.press(Button::JumpDown);
    m.press(Button::JumpDown);
    m.press(Button::Down);
    let (page, cursor) = (m.page(), m.cursor());
    m.press(Button::B);
    assert!(m.is_closed());
    let m = reopened(&m, Env::default());
    assert_eq!((m.page(), m.cursor()), (page, cursor));
}

#[test]
fn every_page_keeps_its_own_row_across_a_close() {
    let mut m = menu();
    m.press(Button::JumpDown);
    let cpu = m.cursor();
    to_page(&mut m, Page::Saves);
    m.press(Button::Down);
    m.press(Button::Down);
    let saves = m.cursor();
    m.press(Button::B);
    let mut m = reopened(&m, Env::default());
    assert_eq!((m.page(), m.cursor()), (Page::Saves, saves));
    // L goes back to the CPU page, which kept its own row while another page was open.
    m.press(Button::L);
    assert_eq!((m.page(), m.cursor()), (Page::Cpu, cpu));
}

#[test]
fn an_open_advanced_group_stays_open() {
    let mut m = menu();
    to_page(&mut m, Page::FightersStages);
    let order = sections(&m, Page::FightersStages);
    for _ in 1..order.len() {
        m.press(Button::JumpDown);
    }
    m.press(Button::A);
    m.press(Button::Down);
    let row = m.focused_row().expect("a Loading row");
    let m = reopened(&m, Env::default());
    assert!(m.group_is_open(*order.last().unwrap()));
    assert_eq!(m.focused_row(), Some(row));
}

#[test]
fn a_model_owned_sub_screen_is_resumed_with_its_cursor() {
    let mut m = menu();
    goto_row(&mut m, row_of("mash_state"));
    m.press(Button::A);
    m.press(Button::JumpDown);
    let open = *m.screen().unwrap();
    // A session that ends with the screen still open (a scene change): the next opening shows it again.
    let m = reopened(&m, Env::default());
    assert_eq!(m.screen(), Some(&open));
    assert_eq!(m.focused_row(), Some(row_of("mash_state")));
}

#[test]
fn a_runtime_owned_screen_falls_back_to_its_parent_row() {
    let mut m = menu();
    let items = ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::GiveItem))).unwrap();
    goto_row(&mut m, items);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Grid { .. })));
    let m = reopened(&m, Env::default());
    assert_eq!(m.screen(), None);
    assert_eq!(m.focused_row(), Some(items));
    // The chord editor (a capture in progress) is not resumed either.
    let mut m = menu();
    let shortcut = ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::ShortcutSlot(0)))).unwrap();
    goto_row(&mut m, shortcut);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::ChordEntry { .. })));
    let m = reopened(&m, Env::default());
    assert_eq!((m.screen(), m.focused_row()), (None, Some(shortcut)));
}

#[test]
fn a_stale_place_is_clamped_when_the_table_changed() {
    // The CPU page's last row is a fallback row that exists only without the pilot plugin (Q11).
    let mut m = Menu::new(Settings::factory(), Env { pilot_present: false, ..Env::default() });
    // Up wraps to the reset line, then to the last row.
    m.press(Button::Up);
    m.press(Button::Up);
    let without = m.cursor();
    let fallback = m.focused_row().unwrap();
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::List { row, .. }) if *row == fallback), "its dropdown is open at the close");
    let m = reopened(&m, Env { pilot_present: true, ..Env::default() });
    // With the pilot present the fallback row is gone, so the page has one entry fewer: the kept index now names the
    // page's last entry, the reset line (review MN-4: the old check, "the hidden row is not focused", could not fail).
    assert_eq!(m.entry_count(Page::Cpu), without + 1);
    assert_eq!(m.cursor(), without, "the index is kept where it still names an entry");
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))), "the entry at that index now: {:?}", m.focus());
    assert_eq!(m.screen(), None, "the dropdown of a row that is gone is not resumed on whatever is focused now");
}


// ---- Reset page to defaults (operator 2026-11-05: no hidden ZR reset; an explicit, confirmed line) ---------------

/// Focus the page's last line, the reset line.
fn goto_reset(m: &mut Menu) {
    let last = m.entry_count(m.page()) - 1;
    assert!(matches!(m.entry(m.page(), last), Some(Entry::ResetPage(_))));
    while m.cursor() != last {
        m.press(Button::Down);
    }
}

#[test]
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

#[test]
fn every_page_ends_with_the_reset_line_in_its_last_section() {
    let env = Env { developer_page: true, ..Env::default() };
    let mut m = Menu::new(Settings::factory(), env);
    for page in [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls, Page::Developer] {
        to_page(&mut m, page);
        let last = m.entry(page, m.entry_count(page) - 1);
        assert_eq!(last, Some(Entry::ResetPage(*sections(&m, page).last().unwrap())), "{page:?}");
        // Up from the first section's first entry wraps to it.
        for _ in 0..12 {
            m.press(Button::JumpUp);
        }
        m.press(Button::Up);
        assert_eq!(m.focus(), last);
    }
}

#[test]
fn the_reset_line_restores_the_page_after_a_confirm_and_reports_the_rows() {
    let mut m = menu();
    let swapping = row_of("trio_practice");
    goto_row(&mut m, swapping);
    let before = m.settings().menu.get("trio_practice").unwrap().to_vec();
    m.press(Button::A);
    assert_ne!(m.settings().menu.get("trio_practice").unwrap(), &before[..]);
    // A row of another page keeps its change.
    to_page(&mut m, Page::DisplayControls);
    goto_row(&mut m, row_of("hud"));
    m.press(Button::A);
    let hud = m.settings().menu.get("hud").unwrap().to_vec();
    to_page(&mut m, Page::FightersStages);
    goto_reset(&mut m);
    m.press(Button::A);
    assert_eq!(m.screen(), Some(&Screen::ConfirmReset));
    assert_ne!(m.settings().menu.get("trio_practice").unwrap(), &before[..], "nothing changes before the confirm");
    m.press(Button::A);
    assert_eq!(m.screen(), None);
    assert_eq!(m.settings().menu.get("trio_practice").unwrap(), &before[..]);
    assert_eq!(m.settings().menu.get("hud").unwrap(), &hud[..]);
    assert_eq!(m.take_page_reset(), Some((Page::FightersStages, vec!["trio_practice"])));
    // A reset that changes nothing still runs and reports no rows.
    goto_reset(&mut m);
    m.press(Button::A);
    m.press(Button::A);
    assert_eq!(m.take_page_reset(), Some((Page::FightersStages, vec![])));
}

#[test]
fn b_cancels_the_reset() {
    let mut m = menu();
    goto_row(&mut m, row_of("trio_practice"));
    m.press(Button::A);
    let changed = m.settings().clone();
    goto_reset(&mut m);
    m.press(Button::A);
    m.press(Button::B);
    assert_eq!((m.screen(), m.is_closed()), (None, false), "B leaves the confirmation, not the menu");
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))));
    assert_eq!(m.settings(), &changed);
    assert!(m.take_page_reset().is_none());
    // The confirmation is not resumed at the next opening: its line is focused instead.
    m.press(Button::A);
    let m = reopened(&m, Env::default());
    assert_eq!(m.screen(), None);
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))));
}

// ---- where ZL + Up / Down are jumps (review MN-1) ------------------------------------------------------------------

#[test]
fn only_pages_and_popup_lists_take_jumps() {
    let mut m = menu();
    assert!(m.takes_jumps(), "a page");
    goto_row(&mut m, row_of("mash_state"));
    m.press(Button::A);
    assert!(m.takes_jumps(), "a checklist");
    m.press(Button::B);
    goto_row(&mut m, row_of("sdi_strength"));
    m.press(Button::A);
    assert!(m.takes_jumps(), "a dropdown list");
    m.press(Button::B);
    goto_row(&mut m, row_of("air_dodge_dir"));
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Directions { .. })));
    assert!(!m.takes_jumps(), "a direction grid");
    m.press(Button::B);
    let items = ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::GiveItem))).unwrap();
    goto_row(&mut m, items);
    m.press(Button::A);
    assert!(matches!(m.screen(), Some(Screen::Grid { .. })));
    assert!(!m.takes_jumps(), "a grid");
}

/// The adapter sends plain Up / Down on a direction grid (`Adapter::set_jumps(false)`); should a jump still arrive (the
/// screen changed under a held ZL), the grid moves as for the plain direction and ZL's all-or-none still works.
#[test]
fn a_direction_grid_moves_on_a_jump_and_zl_still_fills_it() {
    let mut m = menu();
    let row = row_of("air_dodge_dir");
    goto_row(&mut m, row);
    m.press(Button::A);
    let chip = |m: &Menu| match m.screen() { Some(Screen::Directions { chip, .. }) => *chip, other => panic!("{other:?}") };
    let start = chip(&m);
    m.press(Button::Up);
    let by_up = chip(&m);
    m.press(Button::Down);
    assert_eq!(chip(&m), start);
    m.press(Button::JumpUp);
    assert_eq!(chip(&m), by_up, "a jump on a direction grid is the plain Up");
    m.press(Button::JumpDown);
    assert_eq!(chip(&m), start);
    let before = m.settings().menu.get("air_dodge_dir").unwrap().to_vec();
    m.press(Button::ZL);
    let after = m.settings().menu.get("air_dodge_dir").unwrap().to_vec();
    assert_ne!(after, before, "ZL fills (or clears) every direction");
    assert!(after.iter().all(|v| *v != 0) || after.iter().all(|v| *v == 0));
}

#[test]
fn a_checklist_takes_the_zl_tap_as_all_or_none() {
    let mut m = menu();
    let mash = row_of("mash_state");
    goto_row(&mut m, mash);
    m.press(Button::A);
    let before = m.settings().menu.get("mash_state").unwrap().to_vec();
    m.press(Button::ZL);
    let after = m.settings().menu.get("mash_state").unwrap().to_vec();
    assert_ne!(after, before);
    assert!(after.iter().all(|v| *v != 0) || after.iter().all(|v| *v == 0), "all or none: {after:?}");
}

// ---- the worker's keeper (review MN-4) -------------------------------------------------------------------------------

/// The settings the worker reads at an opening: the committed file, whose `last_page` a close wrote.
fn committed(m: &Menu) -> Settings {
    let mut s = m.settings().clone();
    s.set_last_page(m.page().id());
    s
}

/// The worker's path, as `runtime::ui::worker` runs it: `drop_model` whenever the input generation moves (a close, a
/// scene change, a Training reset, a lobby change), `open` at the next opening.
#[test]
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

#[test]
fn the_developer_page_gone_falls_back_to_last_page_rules() {
    let env = Env { developer_page: true, ..Env::default() };
    let mut m = Menu::new(Settings::factory(), env);
    to_page(&mut m, Page::Developer);
    m.press(Button::Down);
    let m = reopened(&m, Env::default());
    assert_ne!(m.page(), Page::Developer);
    assert!(m.cursor() < m.entry_count(m.page()));
}
