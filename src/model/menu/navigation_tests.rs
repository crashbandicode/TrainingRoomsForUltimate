//! rc15/submenu-page-switch: L / R step the page from a submenu as from the page itself (popups and editors keep their
//! own buttons), and ZL + Up / Down wrap at the first and last section or entry, as Up / Down do.
lib_unit_tests! {
use super::*;
use core::mem::discriminant;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn on_page(page: Page) -> Menu {
    let mut m = menu();
    for _ in 0..6 {
        if m.page() == page { break; }
        m.press(Button::R);
    }
    assert_eq!(m.page(), page);
    m
}

fn row_where(f: impl Fn(&Row) -> bool) -> usize {
    ROWS.iter().position(f).expect("a row")
}

/// The page L or R reaches from the page itself (the existing top-level path).
fn top_level(page: Page, button: Button) -> Page {
    let mut m = on_page(page);
    m.press(button);
    m.page()
}

/// The screens L / R leave for the next page: a game's sub-screen, the pickers, the editor screens and lists.
fn submenus(m: &Menu) -> Vec<Screen> {
    let direction = row_where(|r| matches!(r.control, Control::DirectionGrid { .. }));
    vec![
        Screen::Rows { group: table::game_group(Game::Mashout), cursor: 0 },
        m.roster_screen(RosterUse::PlayerPick),
        m.roster_screen(RosterUse::CpuPick),
        m.roster_screen(RosterUse::Mashout),
        Screen::Directions { row: direction, chip: DIRECTION_START },
        Screen::Overrides { situation: 0, cursor: 0 },
        Screen::Chords { cursor: 0 },
    ]
}

#[test]
fn page_switch_leaves_a_submenu() {
    for button in [Button::L, Button::R] {
        let want = top_level(Page::FightersStages, button);
        assert_ne!(want, Page::FightersStages);
        let probe = menu();
        for screen in submenus(&probe) {
            let mut m = on_page(Page::FightersStages);
            m.stack.push(screen);
            m.press(button);
            assert_eq!((m.page(), m.screen().copied()), (want, None), "{button:?} from {screen:?}");
        }
        // Reached by the player's own path: Practice games > Mash out, then "Fighter and move" open on the grid.
        let mut m = on_page(Page::FightersStages);
        let list = row_where(|r| matches!(r.control, Control::Game(Game::Mashout)));
        for _ in 0..=m.entry_count(Page::FightersStages) {
            if m.focus() == Some(Entry::Row(list)) { break; }
            m.press(Button::Down);
        }
        m.press(Button::A);
        m.press(Button::Down);
        m.press(Button::A);
        assert!(matches!(m.screen(), Some(Screen::Grid { .. })), "the Mash out grid over its sub-screen");
        assert_eq!(m.stack.len(), 2);
        m.press(button);
        assert_eq!((m.page(), m.stack.len()), (want, 0), "{button:?} leaves both sub-screens");
        assert!(m.mashout_pop().is_none());
    }
}

#[test]
fn page_switch_keeps_popup_and_editor_buttons() {
    let dropdown = row_where(|r| matches!(r.control, Control::Dropdown(_)));
    let checklist = row_where(|r| matches!(r.control, Control::MultiSelect(_)));
    let delay = row_where(|r| matches!(r.control, Control::MultiSelectRange { .. }));
    let slider = row_where(|r| matches!(r.control, Control::Slider));
    let probe = menu();
    let grid = |kind| Screen::Grid { layout: GridLayout::new(kind, probe.env().console), page: 0, cell: 0 };
    let own = [
        Screen::List { row: dropdown, cursor: 0 },
        Screen::Checklist { row: checklist, cursor: 0 },
        Screen::Delay { row: delay, cursor: 0, editing: false },
        Screen::Slider { row: slider, state: SliderState::LowerHover },
        Screen::SwapGroups { cursor: 0 },
        Screen::ChordEntry { target: ChordTarget::Row(0), captured: Chord::NONE },
        Screen::ConfirmReset,
        grid(GridKind::Items),
        grid(GridKind::Stages),
        grid(GridKind::StageChange),
        probe.roster_screen(RosterUse::Cycles),
        probe.roster_screen(RosterUse::Hats),
        Screen::External(Editor::SetupLibrary),
    ];
    for button in [Button::L, Button::R] {
        for screen in own {
            let mut m = on_page(Page::FightersStages);
            m.stack.push(screen);
            m.press(button);
            assert_eq!(m.page(), Page::FightersStages, "{button:?} on {screen:?}");
            assert_eq!(m.stack.len(), 1, "{button:?} on {screen:?}");
            assert_eq!(discriminant(m.screen().unwrap()), discriminant(&screen), "{button:?} on {screen:?}");
        }
        // Mash out's popup over its chip keeps its keys too.
        let mut m = on_page(Page::FightersStages);
        m.stack.push(m.roster_screen(RosterUse::Mashout));
        m.mashout_open(super::super::mashout::picker::Chip::Fighter(0x58), 78);
        m.press(button);
        assert_eq!(m.page(), Page::FightersStages);
        assert!(m.mashout_pop().is_some() && matches!(m.screen(), Some(Screen::Grid { .. })));
    }
}

/// One press as the worker runs it: the model, then the roster editor for a grid press, then the editor's lifetime rule.
fn worker_press(m: &mut Menu, worker: &mut Option<crate::editors::roster::RosterEditor>, button: Button) {
    m.press(button);
    if let Some((tile, b)) = m.take_roster_press() {
        let Some(Screen::Grid { layout: GridLayout { kind: GridKind::Roster(use_), .. }, .. }) = m.screen().copied() else { return };
        worker.get_or_insert_with(|| crate::editors::roster::RosterEditor::new(use_)).apply(m, usize::from(tile), b);
    }
    crate::editors::roster::retain(worker, m);
}

/// The label the worker draws on `cell` of the open roster grid (a fresh editor when it holds none, as `publish_frame`).
fn drawn(m: &Menu, worker: &Option<crate::editors::roster::RosterEditor>, cell: usize) -> String {
    let editor = worker.unwrap_or_else(|| crate::editors::roster::RosterEditor::new(RosterUse::PlayerPick));
    let mut f = crate::model::frame::Frame::new();
    editor.decorate(&mut f, &editor.values(m.kept(editor.key())));
    String::from_utf16(f.tiles[cell].units()).unwrap()
}

/// Review 23:07Z #1: X previews Mythra in Player Character; R leaves for the next page, L comes back and the grid is
/// opened again. The worker's roster editor did not outlive its grid: the chip shows the saved form (Pyra), the saved
/// pick is unchanged, and A picks the form shown.
#[test]
fn page_switch_drops_the_worker_roster_preview() {
    use crate::engine::swap::roster_grid::{Role, RosterGrid, CELLS};
    use crate::engine::swap::trio::CHOICES;
    let grid = RosterGrid::new(Role::Player);
    let aegis = (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(0x5a)).expect("the Aegis cell");
    let open = |m: &mut Menu| {
        m.open_editor(Editor::PortraitPlayer);
        let Some(Screen::Grid { layout, page, .. }) = m.screen().copied() else { panic!("the grid") };
        m.top(Screen::Grid { layout, page, cell: aegis });
    };
    let mut m = on_page(Page::FightersStages);
    let saved = m.kept("player_character").cloned();
    let mut worker = None;
    open(&mut m);
    assert_eq!(drawn(&m, &worker, aegis), "Pyra");
    worker_press(&mut m, &mut worker, Button::X);
    assert_eq!(drawn(&m, &worker, aegis), "Mythra", "X previews the next form");
    worker_press(&mut m, &mut worker, Button::R);
    assert_eq!((m.page(), m.screen().copied()), (top_level(Page::FightersStages, Button::R), None));
    worker_press(&mut m, &mut worker, Button::L);
    assert_eq!(m.kept("player_character").cloned(), saved, "the saved pick is preserved");
    open(&mut m);
    assert_eq!(drawn(&m, &worker, aegis), "Pyra", "the grid opens on the saved form");
    worker_press(&mut m, &mut worker, Button::A);
    let pyra = CHOICES.iter().position(|(k, _)| *k == 0x5a).unwrap();
    let picked = m.kept("player_character").and_then(|v| v.as_array()).and_then(|a| a.iter().position(|x| x.as_u64() == Some(1)));
    assert_eq!(picked, Some(pyra), "A picks the form shown, not the stale preview");
}

/// The first focusable entry of each section of `page`, in order.
fn section_starts(m: &Menu, page: Page) -> Vec<usize> {
    let mut starts: Vec<usize> = Vec::new();
    let mut last = None;
    for i in 0..m.entry_count(page) {
        let Some(e) = m.entry(page, i) else { continue };
        if m.focusable(e) && last != Some(group_of(e)) {
            last = Some(group_of(e));
            starts.push(i);
        }
    }
    starts
}

#[test]
fn a_list_jump_inside_a_list_stops_at_its_ends() {
    assert_eq!(jumped_within(0, 0, false), None);
    assert_eq!(jumped_within(0, 0, true), None);
    assert_eq!(jumped_within(0, 1, false), None);
    assert_eq!(jumped_within(0, 1, true), None);
    assert_eq!(jumped_within(0, 30, false), None);
    assert_eq!(jumped_within(0, 30, true), Some(12));
    assert_eq!(jumped_within(15, 30, false), Some(3));
    assert_eq!(jumped_within(15, 30, true), Some(27));
    assert_eq!(jumped_within(29, 30, false), Some(17));
    assert_eq!(jumped_within(29, 30, true), None);
    assert_eq!(jumped_within(99, 30, false), Some(17));
    assert_eq!(jumped_within(99, 30, true), None);
    for len in 0usize..=30 {
        for cursor in 0..=len + 2 {
            for forward in [false, true] {
                let wrap = if forward { 0 } else { len.saturating_sub(1) };
                assert_eq!(jumped(cursor, len, forward), jumped_within(cursor, len, forward).unwrap_or(wrap));
            }
        }
    }
}

#[test]
fn page_jump_wraps_at_the_edges() {
    // A page: ZL + Up from the first section goes to the last, ZL + Down from the last to the first.
    let mut m = on_page(Page::FightersStages);
    let starts = section_starts(&m, Page::FightersStages);
    assert!(starts.len() > 2);
    m.cursor[Page::FightersStages.index()] = starts[0];
    m.press(Button::JumpUp);
    assert_eq!(m.cursor[Page::FightersStages.index()], *starts.last().unwrap(), "ZL + Up wraps to the last section");
    m.press(Button::JumpDown);
    assert_eq!(m.cursor[Page::FightersStages.index()], starts[0], "ZL + Down wraps to the first section");
    m.press(Button::JumpDown);
    assert_eq!(m.cursor[Page::FightersStages.index()], starts[1], "inside the page a jump is one section");
    // A game's sub-screen (Tech practice, longer than a window): the window stops on the last row, then wraps.
    let tech = table::game_group(Game::Tech);
    let len = menu().sub_rows(tech).count();
    assert!(len > LIST_JUMP);
    let mut m = on_page(Page::FightersStages);
    m.stack.push(Screen::Rows { group: tech, cursor: len - 1 });
    m.press(Button::JumpDown);
    assert_eq!(m.screen().copied(), Some(Screen::Rows { group: tech, cursor: 0 }), "ZL + Down wraps to the first row");
    m.press(Button::JumpUp);
    assert_eq!(m.screen().copied(), Some(Screen::Rows { group: tech, cursor: len - 1 }), "ZL + Up wraps to the last row");
    m.press(Button::JumpUp);
    assert_eq!(m.screen().copied(), Some(Screen::Rows { group: tech, cursor: len - 1 - LIST_JUMP }), "one window up");
    // A dropdown list: the same at both ends.
    let row = row_where(|r| matches!(r.control, Control::Dropdown(labels) if labels.len() > 2));
    let n = menu().option_count(row);
    let mut m = on_page(Page::FightersStages);
    m.stack.push(Screen::List { row, cursor: n - 1 });
    m.press(Button::JumpDown);
    assert_eq!(m.screen().copied(), Some(Screen::List { row, cursor: 0 }));
    m.press(Button::JumpUp);
    assert_eq!(m.screen().copied(), Some(Screen::List { row, cursor: n - 1 }));
}
}
