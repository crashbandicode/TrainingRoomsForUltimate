//! rc15/submenu-page-switch: L / R step the page from a submenu as from the page itself (popups and editors keep their
//! own buttons).
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
}
