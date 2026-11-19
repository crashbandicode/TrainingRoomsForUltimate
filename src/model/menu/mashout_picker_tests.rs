//! rc15/mashout-grid-picker: Practice games > Mash out picks its fighter and move on the portrait grid. A on a chip opens
//! a popup over it with only that fighter's valid options from the reviewed move table (`moves`); the picker writes the
//! same three keys and values the State / Attacker move / Grab type dropdowns wrote.
lib_unit_tests! {
use super::*;
use crate::editors::roster::RosterEditor;
use crate::model::frame::{Frame, Text};
use crate::model::mashout::moves::{self, Config, MoveAvailability, State};
use crate::model::mashout::picker::{self, Chip};
use crate::model::screens::TILE_GREYED;
use crate::engine::swap::roster_grid::{CellContent, Role, RosterGrid, CELLS};

const MARIO: u32 = 0x00;
const STEVE: u32 = 0x58;
const RANDOM_CELL: usize = CELLS - 1;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn with(state: usize, attacker: usize, grab: usize) -> Menu {
    let mut s = Settings::factory();
    let hot = |len: usize, i: usize| { let mut v = vec![0u8; len]; v[i] = 1; v };
    s.menu.set(moves::STATE_ROW, &hot(moves::STATE_LABELS.len(), state)).unwrap();
    s.menu.set(moves::ATTACKER_ROW, &hot(moves::ATTACKER_LABELS.len(), attacker)).unwrap();
    s.menu.set(moves::GRAB_ROW, &hot(moves::GRAB_LABELS.len(), grab)).unwrap();
    Menu::new(s, Env::default())
}

fn frame(m: &Menu) -> Frame {
    let mut f = Frame::new();
    crate::model::screens::frame_of(m, &mut f, |_| "Stage");
    f
}

fn text<const N: usize>(t: &Text<N>) -> String {
    String::from_utf16(t.units()).unwrap()
}

fn one(m: &Menu, id: &str) -> Option<usize> {
    m.settings().menu.get(id).and_then(|row| row.iter().position(|v| *v != 0))
}

/// Open the picker as the worker does: the open asks the roster editor (MASHOUT_SEEK) to put the cursor on the saved chip.
fn open(m: &mut Menu) {
    m.open_editor(Editor::MashoutPicker);
    let (tile, button) = m.take_roster_press().expect("the seek");
    assert_eq!(tile, MASHOUT_SEEK);
    RosterEditor::new(RosterUse::Mashout).apply(m, usize::from(tile), button);
}

fn picker_row() -> usize {
    ROWS.iter().position(|r| matches!(r.control, Control::Editor(Editor::MashoutPicker))).expect("the picker row")
}

/// The cell whose chip holds `kind` (a shared slot holds several forms).
fn home(kind: u32) -> usize {
    let mut grid = RosterGrid::new(Role::Cpu);
    grid.focus_kind(kind);
    (0..CELLS).find(|c| grid.info(*c).and_then(|i| i.kind()) == Some(kind)).expect("a cell")
}

/// A on `kind`'s chip, previewing its form with X first as the player would.
fn a_on(m: &mut Menu, kind: u32) -> usize {
    let cell = home(kind);
    let mut editor = RosterEditor::new(RosterUse::Mashout);
    for _ in 0..3 {
        let mut f = Frame::new();
        editor.decorate_mashout(&mut f, m);
        if u32::from(f.tile_n[cell]) == kind { break; }
        editor.apply(m, cell, Button::X);
    }
    editor.apply(m, cell, Button::A);
    cell
}

fn popup_lines(m: &Menu) -> Vec<String> {
    let f = frame(m);
    (0..usize::from(f.pop.count)).map(|i| text(&f.pop.lines[i])).collect()
}

/// Every row of `kind` in the catalog whose (state, persisted index) the dropdown model accepts, in catalog order.
fn table(kind: u32) -> Vec<(usize, usize)> {
    moves::CATALOG.iter().enumerate()
        .filter(|(_, s)| s.attacker_kind == kind)
        .map(|(row, s)| (State::ALL.iter().position(|x| *x == s.state).unwrap(), if row < moves::RANDOM_ATTACKER { row } else { row + 1 }))
        .filter(|(state, attacker)| moves::attacker_valid(*state, *attacker))
        .collect()
}

#[test]
fn mashout_row_opens_the_portrait_grid() {
    let group = table::game_group(Game::Mashout);
    let titles: Vec<_> = ROWS.iter().filter(|r| r.group == group).map(|r| r.title).collect();
    assert_eq!(titles, ["Start", "Fighter and move"]);
    let mut m = menu();
    open(&mut m);
    let Some(Screen::Grid { layout, page, .. }) = m.screen().copied() else { panic!("a grid") };
    assert_eq!((layout.kind, page), (GridKind::Roster(RosterUse::Mashout), 0));
    // The character select grid of the Player / CPU Character picks: the same layout, cells and stepping.
    let pick = GridLayout::new(GridKind::Roster(RosterUse::PlayerPick), m.env().console);
    assert_eq!((layout.cols, layout.rows, layout.tiles()), (pick.cols, pick.rows, pick.tiles()));
    for cell in 0..layout.tiles() {
        for dir in [Move::Up, Move::Down, Move::Left, Move::Right] {
            assert_eq!(layout.step(0, cell, dir), pick.step(0, cell, dir));
        }
    }
    m.press(Button::B);
    assert!(matches!(m.screen(), Some(Screen::Rows { .. }) | None));
}

#[test]
fn mashout_popup_options_equal_the_move_table() {
    for (kind, _) in moves::ATTACKERS {
        let got: Vec<_> = picker::choices(Chip::Fighter(*kind)).map(|c| (c.state, c.attacker)).collect();
        assert_eq!(got, table(*kind), "kind {kind:#x}");
        assert!(got.len() <= picker::MAX);
        for c in picker::choices(Chip::Fighter(*kind)) {
            let s = moves::scenario(c.attacker).unwrap();
            assert_eq!(moves::availability(s.id), MoveAvailability::Available, "{} listed", s.name);
            assert_eq!(c.grab, s.plan == moves::Plan::Grab);
        }
    }
    let names = |kind| picker::choices(Chip::Fighter(kind)).map(|c| moves::scenario(c.attacker).unwrap().name).collect::<Vec<_>>();
    assert_eq!(names(MARIO), ["Grab"], "Mario has only Grab");
    assert_eq!(names(STEVE), ["Grab", "Empty minecart"], "Steve has the minecart");
}

#[test]
fn mashout_popup_opens_over_the_chip() {
    let mut m = menu();
    open(&mut m);
    let steve = a_on(&mut m, STEVE);
    let pop = m.mashout_pop().expect("Steve's popup");
    assert_eq!((pop.chip, pop.cell), (Chip::Fighter(STEVE), steve));
    let f = frame(&m);
    assert_eq!(f.pop.anchor, Some(steve as u8), "over Steve's chip");
    assert_eq!(popup_lines(&m), ["Grab  <  Standing  >", "Empty minecart"]);
    m.press(Button::B);
    assert!(m.mashout_pop().is_none());
    assert!(matches!(m.screen(), Some(Screen::Grid { .. })), "B closes the popup back to the grid");
    a_on(&mut m, MARIO);
    assert_eq!(popup_lines(&m), ["Grab  <  Standing  >"]);
}

/// Operator review of the preview (primary 15:49Z): the popup is sized to its content, its widest line plus the padding.
#[test]
fn mashout_popup_box_fits_its_widest_line() {
    use crate::model::frame::{PopView, POP_PAD};
    let measured = |pop: &PopView| (0..usize::from(pop.count))
        .map(|i| 1.2 * crate::model::notice_box::measure(pop.lines[i].units().iter().copied(), crate::model::panes::FONT_SMALL.0).0)
        .fold(0.0, f32::max);
    let mut widths = Vec::new();
    for kind in [MARIO, STEVE] {
        let mut m = menu();
        open(&mut m);
        a_on(&mut m, kind);
        let pop = frame(&m).pop;
        assert_eq!(pop.width(), measured(&pop) + 2.0 * POP_PAD, "{kind:#x}");
        assert!(pop.width() < 400.0, "{kind:#x}: {} is not the old 520-wide box", pop.width());
        widths.push(pop.width());
    }
    // Steve's lines are Mario's Grab line and "Empty minecart": the same widest line, the same box.
    assert_eq!(widths[0], widths[1]);
    assert_eq!(PopView::EMPTY.width(), 2.0 * POP_PAD);
}

/// Mash out's sub-screen from the Practice games list, the cursor on "Fighter and move".
fn on_picker_row(m: &mut Menu) {
    for _ in 0..6 {
        if m.page() == Page::FightersStages { break; }
        m.press(Button::R);
    }
    let list = ROWS.iter().position(|r| matches!(r.control, Control::Game(Game::Mashout))).unwrap();
    for _ in 0..=m.entry_count(Page::FightersStages) {
        if m.focus() == Some(Entry::Row(list)) { break; }
        m.press(Button::Down);
    }
    m.press(Button::A);
    m.press(Button::Down);
    assert!(m.help_row().is_some_and(|r| std::ptr::eq(r, &ROWS[picker_row()])), "on the picker row");
}

/// Steve → Empty minecart through the picker, back on the picker row.
fn pick_steve_minecart(m: &mut Menu) {
    open(m);
    a_on(m, STEVE);
    m.press(Button::Down);
    m.press(Button::A);
    assert_eq!(one(m, moves::ATTACKER_ROW), Some(98));
}

fn chosen(m: &Menu) -> (Option<usize>, Option<usize>, Option<usize>) {
    (one(m, moves::STATE_ROW), one(m, moves::ATTACKER_ROW), one(m, moves::GRAB_ROW))
}

/// The factory, and a saved default that is not the factory (Random Bury, Pivot grab type).
fn reset_cases() -> [(Menu, (Option<usize>, Option<usize>, Option<usize>), &'static str); 2] {
    let mut saved = Settings::factory();
    let hot = |len: usize, i: usize| { let mut v = vec![0u8; len]; v[i] = 1; v };
    saved.defaults.set(moves::STATE_ROW, &hot(moves::STATE_LABELS.len(), 1)).unwrap();
    saved.defaults.set(moves::ATTACKER_ROW, &hot(moves::ATTACKER_LABELS.len(), moves::RANDOM_ATTACKER)).unwrap();
    saved.defaults.set(moves::GRAB_ROW, &hot(moves::GRAB_LABELS.len(), 2)).unwrap();
    [(menu(), (Some(0), Some(0), Some(0)), "Mario · Grab (Standing)  >"),
     (Menu::new(saved, Env::default()), (Some(1), Some(moves::RANDOM_ATTACKER), Some(2)), "Random Bury  >")]
}

fn row_value(m: &Menu) -> String {
    let mut t = Text::<72>::new();
    crate::model::frame::write_value(m, picker_row(), &mut t);
    text(&t)
}

/// Review 17:01Z #1: Y on "Fighter and move" restores the picker's three keys from the saved defaults.
#[test]
fn mashout_y_resets_the_picker_to_its_saved_default() {
    for (mut m, want, shown) in reset_cases() {
        on_picker_row(&mut m);
        pick_steve_minecart(&mut m);
        assert!(m.help_row().is_some_and(|r| std::ptr::eq(r, &ROWS[picker_row()])), "back on the picker row");
        m.press(Button::Y);
        assert_eq!(chosen(&m), want);
        assert_eq!(row_value(&m), shown);
    }
}

/// Review 17:01Z #1: the confirmed Reset page to defaults restores the picker too, and reports the row as changed.
#[test]
fn mashout_page_reset_restores_the_picker_to_its_saved_default() {
    for (mut m, want, shown) in reset_cases() {
        on_picker_row(&mut m);
        pick_steve_minecart(&mut m);
        m.press(Button::B);
        for _ in 0..=m.entry_count(Page::FightersStages) {
            if matches!(m.focus(), Some(Entry::ResetPage(_))) { break; }
            m.press(Button::Down);
        }
        assert!(matches!(m.focus(), Some(Entry::ResetPage(_))));
        m.press(Button::A);
        assert!(matches!(m.screen(), Some(Screen::ConfirmReset)));
        m.press(Button::A);
        assert_eq!(chosen(&m), want);
        assert_eq!(row_value(&m), shown);
        let (_, changed) = m.take_page_reset().expect("a page reset");
        assert!(changed.contains(&"Fighter and move"), "{changed:?}");
    }
}

#[test]
fn mashout_greyed_fighters_cannot_be_picked() {
    let m = menu();
    let mut f = Frame::new();
    RosterEditor::new(RosterUse::Mashout).decorate_mashout(&mut f, &m);
    let grid = RosterGrid::new(Role::Cpu);
    for cell in 0..CELLS {
        let pickable = match grid.info(cell).unwrap().content {
            CellContent::Fighter(kind) => picker::pickable(Chip::Fighter(kind)),
            CellContent::All => true,
            CellContent::Unsupported => false,
        };
        assert_eq!(f.tile_state[cell] == TILE_GREYED, !pickable, "cell {cell}");
        if !pickable {
            let mut m = menu();
            open(&mut m);
            RosterEditor::new(RosterUse::Mashout).apply(&mut m, cell, Button::A);
            assert!(m.mashout_pop().is_none(), "a greyed chip opens nothing");
        }
    }
    // A fighter the move table does not list has no option, so its chip is greyed and refuses A.
    assert!(!picker::pickable(Chip::Fighter(0x7f)));
    assert_eq!(picker::choices(Chip::Fighter(0x7f)).count(), 0);
    let mut m = menu();
    let before = m.settings().menu.clone();
    m.mashout_open(Chip::Fighter(0x7f), 0);
    assert!(m.mashout_pop().is_none());
    assert_eq!(m.settings().menu, before);
}

#[test]
fn mashout_grab_line_keeps_the_grab_type() {
    let mut m = menu();
    open(&mut m);
    a_on(&mut m, MARIO);
    m.press(Button::Right);
    assert_eq!(popup_lines(&m), ["Grab  <  Dash  >"]);
    m.press(Button::Right);
    m.press(Button::Right);
    assert_eq!(popup_lines(&m), ["Grab  <  Random  >"]);
    m.press(Button::Right);
    assert_eq!(popup_lines(&m), ["Grab  <  Standing  >"], "the four grab types wrap");
    m.press(Button::Left);
    m.press(Button::Left);
    assert_eq!(popup_lines(&m), ["Grab  <  Pivot  >"]);
    m.press(Button::A);
    assert!(m.mashout_pop().is_none());
    assert!(!matches!(m.screen(), Some(Screen::Grid { .. })), "A confirms and leaves the grid");
    assert_eq!((one(&m, moves::STATE_ROW), one(&m, moves::ATTACKER_ROW), one(&m, moves::GRAB_ROW)), (Some(0), Some(0), Some(2)));
    // Left / Right on a line without a grab type change nothing.
    let mut m = menu();
    open(&mut m);
    a_on(&mut m, STEVE);
    m.press(Button::Down);
    m.press(Button::Right);
    assert_eq!(popup_lines(&m)[1], "Empty minecart");
    m.press(Button::A);
    assert_eq!((one(&m, moves::STATE_ROW), one(&m, moves::GRAB_ROW)), (Some(0), Some(0)));
    assert_eq!(moves::scenario(one(&m, moves::ATTACKER_ROW).unwrap()).unwrap().name, "Empty minecart");
}

#[test]
fn mashout_random_chip_keeps_every_random_choice() {
    let random: Vec<_> = picker::choices(Chip::Random).map(|c| (c.state, c.attacker, c.grab)).collect();
    let r = moves::RANDOM_ATTACKER;
    assert_eq!(random, [(moves::RANDOM_STATE, r, true), (0, r, true), (1, r, false), (2, r, false), (3, r, false), (4, r, false)]);
    // Exactly the Random choices of the dropdowns: Random state, and Random within each state.
    for state in 0..moves::STATE_LABELS.len() {
        assert!(moves::attacker_valid(state, r));
        assert!(random.iter().any(|(s, _, _)| *s == state));
    }
    let mut m = menu();
    open(&mut m);
    let mut f = Frame::new();
    RosterEditor::new(RosterUse::Mashout).decorate_mashout(&mut f, &m);
    assert_eq!(text(&f.tiles[RANDOM_CELL]), "Random");
    RosterEditor::new(RosterUse::Mashout).apply(&mut m, RANDOM_CELL, Button::A);
    assert_eq!(m.mashout_pop().map(|p| p.chip), Some(Chip::Random));
    assert_eq!(popup_lines(&m), ["Random state  <  Standing  >", "Random Grab  <  Standing  >", "Random Bury", "Random Frozen", "Random Egg", "Random Inhale"]);
    m.press(Button::Up);
    m.press(Button::A);
    assert_eq!((one(&m, moves::STATE_ROW), one(&m, moves::ATTACKER_ROW)), (Some(4), Some(r)));
}

#[test]
fn mashout_row_shows_the_chosen_fighter_and_move() {
    let value = |m: &Menu| { let mut t = Text::<72>::new(); crate::model::frame::write_value(m, picker_row(), &mut t); text(&t) };
    assert_eq!(value(&menu()), "Mario · Grab (Standing)  >");
    assert_eq!(value(&with(0, 98, 0)), "Steve · Empty minecart  >");
    assert_eq!(value(&with(0, 0, 1)), "Mario · Grab (Dash)  >");
    assert_eq!(value(&with(moves::RANDOM_STATE, moves::RANDOM_ATTACKER, 3)), "Random state (Random)  >");
    assert_eq!(value(&with(1, moves::RANDOM_ATTACKER, 0)), "Random Bury  >");
    // An edited conflicting pair (Bury with Mario's grab) is invalid, as the dropdowns showed it.
    assert_eq!(value(&with(1, 0, 0)), "-  >");
}

#[test]
fn mashout_picker_writes_the_same_keys_as_the_dropdowns() {
    // Parity: every configuration the dropdowns could write is reachable on the grid and written with the same three
    // keys and values; the grid writes nothing else.
    let mut cases = Vec::new();
    for state in 0..moves::STATE_LABELS.len() {
        for option in (0..moves::attacker_count(state)).filter(|o| moves::attacker_offered(state, *o)) {
            let attacker = moves::attacker_index(state, option).unwrap();
            let grabs = if moves::uses_grab(Config { state, attacker, grab: 0 }) { 0..moves::GRAB_LABELS.len() } else { 0..1 };
            cases.extend(grabs.map(|grab| Config { state, attacker, grab }));
        }
    }
    let picker_total: usize = moves::ATTACKERS.iter().map(|(k, _)| Chip::Fighter(*k)).chain([Chip::Random])
        .flat_map(picker::choices).map(|c| if c.grab { moves::GRAB_LABELS.len() } else { 1 }).sum();
    assert_eq!(picker_total, cases.len(), "the grid offers exactly the dropdowns' choices");
    for want in cases {
        let mut m = menu();
        let factory = m.settings().menu.clone();
        open(&mut m);
        match picker::chip_of(want).expect("a chip") {
            Chip::Random => RosterEditor::new(RosterUse::Mashout).apply(&mut m, RANDOM_CELL, Button::A),
            Chip::Fighter(kind) => _ = a_on(&mut m, kind),
        }
        let line = picker::line_of(want).expect("a line");
        let pop = m.mashout_pop().expect("a popup");
        for _ in pop.cursor..line { m.press(Button::Down); }
        for _ in line..pop.cursor { m.press(Button::Up); }
        for _ in 0..want.grab { m.press(Button::Right); }
        m.press(Button::A);
        let mut old = factory.clone();
        let hot = |len: usize, i: usize| { let mut v = vec![0u8; len]; v[i] = 1; v };
        old.set(moves::STATE_ROW, &hot(moves::STATE_LABELS.len(), want.state)).unwrap();
        old.set(moves::ATTACKER_ROW, &hot(moves::ATTACKER_LABELS.len(), want.attacker)).unwrap();
        if moves::uses_grab(want) { old.set(moves::GRAB_ROW, &hot(moves::GRAB_LABELS.len(), want.grab)).unwrap(); }
        assert_eq!(m.settings().menu, old, "{want:?}");
        assert_eq!(Config::from_snapshot(&m.settings().menu), Some(Config { grab: if moves::uses_grab(want) { want.grab } else { 0 }, ..want }));
    }
}

#[test]
fn mashout_old_setting_opens_on_its_chip_and_option() {
    // A saved Steve minecart with a Pivot grab type: the grid opens on Steve, his popup on the minecart line.
    let mut m = with(0, 98, 2);
    open(&mut m);
    assert!(matches!(m.screen(), Some(Screen::Grid { cell, .. }) if *cell == home(STEVE)));
    a_on(&mut m, STEVE);
    let pop = m.mashout_pop().unwrap();
    assert_eq!((pop.cursor, pop.grab), (1, 2));
    // Mario's grab with the Dash type.
    let mut m = with(0, 0, 1);
    open(&mut m);
    assert!(matches!(m.screen(), Some(Screen::Grid { cell, .. }) if *cell == home(MARIO)));
    a_on(&mut m, MARIO);
    assert_eq!(popup_lines(&m), ["Grab  <  Dash  >"]);
    // Random within Bury: the grid opens on the Random chip, its popup on Random Bury.
    let mut m = with(1, moves::RANDOM_ATTACKER, 0);
    open(&mut m);
    assert!(matches!(m.screen(), Some(Screen::Grid { cell, .. }) if *cell == RANDOM_CELL));
    RosterEditor::new(RosterUse::Mashout).apply(&mut m, RANDOM_CELL, Button::A);
    assert_eq!(m.mashout_pop().unwrap().cursor, 2, "Random state, Random Grab, then Random Bury");
    // Every valid saved setting maps back to the chip and line that write it.
    for (kind, _) in moves::ATTACKERS {
        for (line, c) in picker::choices(Chip::Fighter(*kind)).enumerate() {
            let config = Config { state: c.state, attacker: c.attacker, grab: 0 };
            assert_eq!((picker::chip_of(config), picker::line_of(config)), (Some(Chip::Fighter(*kind)), Some(line)));
        }
    }
    // The chip of the saved fighter is marked on the grid.
    let mut f = Frame::new();
    RosterEditor::new(RosterUse::Mashout).decorate_mashout(&mut f, &with(0, 98, 0));
    assert_eq!(text(&f.tiles[home(STEVE)]), "> Steve");
    assert_eq!(text(&f.tiles[home(MARIO)]), "Mario");
}
}
