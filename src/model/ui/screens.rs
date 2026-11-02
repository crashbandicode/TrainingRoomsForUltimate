//! The sub-screens of the menu as frames (SPEC-core-03 §2.2, §2.5): the popup lists, the delay popup, the
//! slider, the direction grid's list form, the swap groups, the situation overrides, the button chords, the
//! chord editor and the editors whose content belongs to a later package. Every one is drawn in the same
//! panes as the page (the right-hand rows, the breadcrumb, the help panel); the grids with pictures
//! (`Screen::Directions` chips, `Screen::Grid`) have their own panes and frames in `grids`.

use core::fmt::Write;

use super::frame::{page_frame, Frame, RowLine, Text, Tone, View};
use super::menu::chord::Chord;
use super::menu::options::{ACTIONS, CHORD_ROWS, DIRECTIONS, SITUATIONS, SWAP_GROUPS, YES_NO_RANDOM};
use super::catalog::{ITEMS, STAGES};
use super::menu::grid::{GridKind, GridLayout};
use super::menu::table::{Control, Row, GROUPS, ROWS};
use super::menu::{ChordTarget, Grey, Menu, Screen, SliderState};
use super::panes::ROWS as ROW_LINES;

const HINT_LIST: &str = "Up/Down choose   A set   B back";
const HINT_CHECKLIST: &str = "Up/Down move   A tick   ZL all or none   B back";
const HINT_DELAY: &str = "Up/Down move   A tick or edit the handle   Y clear   ZL all   B back";
const HINT_SLIDER: &str = "Left/Right other handle   A pick up or put down   Up/Down/Left/Right change   B back";
const HINT_DIRECTIONS: &str = "Up/Down move   A add one weight   Y clear   ZL all or none   B back";
const HINT_CHORD: &str = "Hold one button and press the others   A confirm   Y empty   B back";

/// First line of a window of `ROW_LINES` rows over `total` that keeps `cursor` in view.
pub fn window_start(cursor: usize, total: usize) -> usize {
    if total <= ROW_LINES || cursor < ROW_LINES / 2 {
        0
    } else {
        (cursor + 1 + ROW_LINES / 2).min(total).saturating_sub(ROW_LINES)
    }
}

fn row_of(screen: &Screen) -> Option<&'static Row> {
    match screen {
        Screen::List { row, .. }
        | Screen::Checklist { row, .. }
        | Screen::Delay { row, .. }
        | Screen::Slider { row, .. }
        | Screen::Directions { row, .. } => Some(&ROWS[*row]),
        _ => None,
    }
}

/// The breadcrumb: page, section, row.
fn crumb(menu: &Menu, screen: &Screen, out: &mut Text<120>) {
    let _ = write!(out, "{}", menu.page().title());
    if let Some(row) = row_of(screen) {
        let _ = write!(out, "  >  {}  >  {}", GROUPS[usize::from(row.group)].title, row.title);
        return;
    }
    let _ = match screen {
        Screen::SwapGroups { .. } => write!(out, "  >  Swap groups"),
        Screen::Overrides { .. } => write!(out, "  >  Situation overrides"),
        Screen::Chords { .. } => write!(out, "  >  Button chords"),
        Screen::ChordEntry { target: ChordTarget::Shortcut(slot), .. } => write!(out, "  >  Menu shortcut {}", slot + 1),
        Screen::ChordEntry { target: ChordTarget::Row(i), .. } => write!(out, "  >  {}", CHORD_ROWS[usize::from(*i)].0),
        Screen::External(editor) => write!(out, "  >  {editor:?}"),
        Screen::Grid { .. } => Ok(()),
        _ => Ok(()),
    };
}

fn base(menu: &Menu, screen: &Screen, frame: &mut Frame) {
    page_frame(menu, frame);
    // The sub-screen owns the right-hand side: no groups, no rows of the page, its own crumb.
    frame.group_count = 0;
    frame.rows = Frame::new().rows;
    frame.row_count = 0;
    frame.focus = None;
    frame.help = [Text::new(); 2];
    frame.hint.clear();
    crumb(menu, screen, &mut frame.crumb);
    if let Some(row) = menu.help_row() {
        let (one, two) = row.help_lines();
        let _ = frame.help[0].write_str(one);
        let _ = frame.help[1].write_str(two);
    }
}

fn push(frame: &mut Frame, label: &str, value: &str, tone: Tone) {
    let at = usize::from(frame.row_count);
    if at >= ROW_LINES {
        return;
    }
    frame.row_count += 1;
    let line = &mut frame.rows[at];
    let _ = line.label.write_str(label);
    let _ = line.value.write_str(value);
    line.value_tone = tone;
    line.label_tone = if tone == Tone::Off { Tone::Off } else { Tone::Normal };
}

fn mark(on: bool) -> (&'static str, Tone) {
    if on {
        ("[x]", Tone::On)
    } else {
        ("[ ]", Tone::Normal)
    }
}

/// Options of a list-like row as one window of lines.
fn option_label(row: &Row, i: usize, first: u16, step: u16) -> Text<80> {
    let mut label = Text::new();
    let _ = match row.control {
        Control::Dropdown(labels) | Control::MultiSelect(labels) => label.write_str(labels.get(i).copied().unwrap_or("-")),
        Control::YesNoRandom => label.write_str(YES_NO_RANDOM.get(i).copied().unwrap_or("-")),
        Control::Number { .. } => write!(label, "{}", first + step * i as u16),
        Control::MultiSelectRange { .. } => write!(label, "{}", first + step * i as u16),
        _ => Ok(()),
    };
    label
}

fn lines_window(frame: &mut Frame, cursor: usize, total: usize, mut line: impl FnMut(usize, &mut RowLine)) {
    let first = window_start(cursor, total);
    for i in first..total.min(first + ROW_LINES) {
        let slot = i - first;
        line(i, &mut frame.rows[slot]);
        frame.row_count += 1;
    }
    frame.focus = Some((cursor - first) as u8);
}

fn list_frame(menu: &Menu, screen: &Screen, row: usize, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    let r = &ROWS[row];
    let total = menu.option_count(row);
    let current = menu.settings().menu.get(r.id).map(<[u8]>::to_vec).unwrap_or_default();
    let (first, step) = match r.control {
        Control::Number { first, step } => (first, step),
        _ => (0, 1),
    };
    let yes_no = matches!(r.control, Control::YesNoRandom);
    lines_window(frame, cursor, total, |i, line| {
        let label = option_label(r, i, first, step);
        line.label = label;
        let chosen = if yes_no {
            // Yes = [1,0], No = [0,1], Random = [1,1].
            matches!((i, current.first().copied().unwrap_or(0) != 0, current.get(1).copied().unwrap_or(0) != 0), (0, true, false) | (1, false, true) | (2, true, true))
        } else {
            current.get(i).copied().unwrap_or(0) != 0
        };
        if chosen {
            let _ = line.value.write_str("(current)");
            line.value_tone = Tone::On;
        }
    });
    let _ = frame.hint.write_str(HINT_LIST);
}

fn checklist_frame(menu: &Menu, screen: &Screen, row: usize, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    let r = &ROWS[row];
    let total = menu.option_count(row);
    let current = menu.settings().menu.get(r.id).map(<[u8]>::to_vec).unwrap_or_default();
    lines_window(frame, cursor, total, |i, line| {
        line.label = option_label(r, i, 0, 1);
        let weight = current.get(i).copied().unwrap_or(0);
        let (text, tone) = mark(weight != 0);
        let _ = line.value.write_str(text);
        line.value_tone = tone;
    });
    let _ = frame.hint.write_str(HINT_CHECKLIST);
}

fn delay_frame(menu: &Menu, screen: &Screen, row: usize, cursor: usize, editing: bool, frame: &mut Frame) {
    base(menu, screen, frame);
    let r = &ROWS[row];
    let Control::MultiSelectRange { first, step, range_id } = r.control else { return };
    let settings = &menu.settings().menu;
    let ticked = settings.get(r.id).map(<[u8]>::to_vec).unwrap_or_default();
    let range = settings.get(range_id).map(<[u8]>::to_vec).unwrap_or_default();
    let (check_grey, range_grey) = menu.delay_grey(row);
    let total = ticked.len() + 2;
    lines_window(frame, cursor, total, |i, line| {
        if i < ticked.len() {
            let _ = write!(line.label, "{}", first + step * i as u16);
            let (text, tone) = mark(ticked[i] != 0);
            let _ = line.value.write_str(text);
            line.value_tone = if check_grey.is_some() { Tone::Off } else { tone };
            if check_grey.is_some() {
                line.label_tone = Tone::Off;
            }
        } else {
            let handle = i - ticked.len();
            let _ = line.label.write_str(if handle == 0 { "Range: Min" } else { "Range: Max" });
            match range.get(handle) {
                Some(index) => {
                    let _ = write!(line.value, "{}", first + step * u16::from(*index));
                    if editing && cursor == i {
                        let _ = line.value.write_str("  (editing)");
                    }
                }
                None => {
                    let _ = line.value.write_str("not set");
                    line.value_tone = Tone::Dim;
                }
            }
            if range_grey.is_some() {
                line.label_tone = Tone::Off;
                line.value_tone = Tone::Off;
            }
        }
    });
    // A greyed control says why on the help panel's second line.
    let reason = if cursor < ticked.len() { check_grey } else { range_grey };
    if let Some(grey) = reason {
        frame.help[1].clear();
        let _ = frame.help[1].write_str(Grey::text(grey));
    }
    let _ = frame.hint.write_str(HINT_DELAY);
}

fn slider_frame(menu: &Menu, screen: &Screen, row: usize, state: SliderState, frame: &mut Frame) {
    base(menu, screen, frame);
    let values = menu.settings().menu.get(ROWS[row].id).map(<[u8]>::to_vec).unwrap_or_default();
    let selected = matches!(state, SliderState::LowerSelected | SliderState::UpperSelected);
    let lower = matches!(state, SliderState::LowerHover | SliderState::LowerSelected);
    for (i, name) in ["Min", "Max"].into_iter().enumerate() {
        let value = values.get(i).copied().unwrap_or(0);
        push(frame, name, &format!("{value}%"), if selected && (i == 0) == lower { Tone::On } else { Tone::Normal });
    }
    frame.focus = Some(u8::from(!lower));
    let _ = frame.hint.write_str(HINT_SLIDER);
}

fn directions_frame(menu: &Menu, screen: &Screen, row: usize, chip: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    frame.view = View::Chips;
    let weights = menu.settings().menu.get(ROWS[row].id).map(<[u8]>::to_vec).unwrap_or_default();
    for (slot, weight) in frame.chip_weights.iter_mut().zip(weights.iter()) {
        *slot = *weight;
    }
    frame.chip_focus = chip as u8;
    // The two help lines say what is chosen, in words (the chips show it as badges).
    frame.help[1].clear();
    let mut first = true;
    for (i, weight) in weights.iter().enumerate().take(DIRECTIONS.len()) {
        if *weight != 0 {
            let _ = write!(frame.help[1], "{}{} x{}", if first { "Selected: " } else { ", " }, DIRECTIONS[i], weight);
            first = false;
        }
    }
    if first {
        let _ = frame.help[1].write_str("Nothing selected.");
    }
    let _ = frame.hint.write_str(HINT_DIRECTIONS);
}

/// The label of tile `n` of a grid. The stage grid's first tile is All; the model counts 118 stage tiles (All and
/// 117), the carried catalog lists 115 stages, so the tiles after them are numbered until the catalog is whole.
fn tile_label(kind: GridKind, n: usize, short: bool, out: &mut Text<28>) {
    let name: &str = match kind {
        GridKind::Items => ITEMS.get(n).copied().unwrap_or("-"),
        GridKind::Stages if n == 0 => "All",
        GridKind::Stages => STAGES.get(n - 1).copied().unwrap_or("-"),
    };
    if short {
        // Docked tiles are narrow: the first 14 characters, cut at a space where there is one.
        let mut cut = name.char_indices().nth(14).map_or(name.len(), |(i, _)| i);
        if cut < name.len() {
            if let Some(space) = name[..cut].rfind(' ') {
                if space >= 6 {
                    cut = space;
                }
            }
        }
        let _ = out.write_str(&name[..cut]);
    } else {
        let _ = out.write_str(name);
    }
}

fn grid_frame(menu: &Menu, screen: &Screen, layout: GridLayout, page: usize, cell: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    frame.view = View::Tiles;
    frame.tile_cols = layout.cols;
    frame.tile_rows = layout.rows;
    let len = layout.page_len(page).min(frame.tiles.len());
    for i in 0..len {
        if let Some(n) = layout.tile(page, i) {
            tile_label(layout.kind, n, layout.short_names, &mut frame.tiles[i]);
        }
    }
    frame.tile_count = len as u8;
    frame.tile_focus = cell as u8;
    let title = match layout.kind {
        GridKind::Items => "Give item",
        GridKind::Stages => "Stage grid",
    };
    let _ = write!(frame.crumb, "  >  {title}   page {} of {}", page + 1, layout.pages());
    let _ = frame.help[0].write_str(match layout.kind {
        GridKind::Items => "Pick the item the CPU or you carry on load.",
        GridKind::Stages => "Stages the lobby cycles through; All takes every stage.",
    });
    let _ = frame.hint.write_str("D-pad move   L/R page   B back");
}
fn swap_frame(menu: &Menu, screen: &Screen, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    lines_window(frame, cursor, SWAP_GROUPS.len(), |i, line| {
        let _ = line.label.write_str(SWAP_GROUPS[i]);
        let on = menu.settings().menu.get(super::menu::options::SWAP_GROUP_IDS[i]).and_then(|v| v.first().copied()).unwrap_or(0) != 0;
        match menu.swap_entry_grey(i) {
            Some(grey) => {
                let _ = line.value.write_str(Grey::text(grey));
                line.value_tone = Tone::Off;
                line.label_tone = Tone::Off;
            }
            None => {
                let _ = line.value.write_str(if on { "[ON]" } else { "[OFF]" });
                line.value_tone = if on { Tone::On } else { Tone::Off };
            }
        }
    });
    if let Some(grey) = menu.swap_entry_grey(cursor) {
        frame.help[1].clear();
        let _ = frame.help[1].write_str(Grey::text(grey));
    }
    let _ = frame.hint.write_str("Up/Down move   A switch   B back");
}

fn overrides_frame(menu: &Menu, screen: &Screen, situation: usize, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    let (name, id) = SITUATIONS[situation];
    let current = menu.settings().menu.get(id).map(<[u8]>::to_vec).unwrap_or_default();
    lines_window(frame, cursor, ACTIONS.len() + 1, |i, line| {
        if i == 0 {
            let _ = write!(line.label, "Situation  ({} of {})", situation + 1, SITUATIONS.len());
            let _ = write!(line.value, "<  {name}  >");
        } else {
            let _ = line.label.write_str(ACTIONS[i - 1]);
            let (text, tone) = mark(current.get(i - 1).copied().unwrap_or(0) != 0);
            let _ = line.value.write_str(text);
            line.value_tone = tone;
        }
    });
    let _ = frame.help[0].write_str("The actions the CPU may take in this situation (one at random).");
    frame.help[1].clear();
    let _ = frame.hint.write_str("Left/Right situation   Up/Down move   A tick   ZL all or none   B back");
}

fn chords_frame(menu: &Menu, screen: &Screen, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    lines_window(frame, cursor, CHORD_ROWS.len(), |i, line| {
        let (name, id) = CHORD_ROWS[i];
        let _ = line.label.write_str(name);
        let chord = Chord::from_row(menu.settings().menu.get(id).unwrap_or(&[]));
        let _ = write!(line.value, "{chord}  >");
    });
    let _ = frame.help[0].write_str("Hold one button of the chord and press the others.");
    let _ = frame.hint.write_str("Up/Down move   A change the chord   B back");
}

fn chord_entry_frame(menu: &Menu, screen: &Screen, captured: Chord, frame: &mut Frame) {
    base(menu, screen, frame);
    push(frame, "Chord", &format!("{captured}"), Tone::On);
    frame.focus = Some(0);
    let _ = frame.help[0].write_str("Hold one button and press the others to set the chord.");
    let _ = frame.help[1].write_str("An empty chord never passes (a menu shortcut with none is disabled).");
    let _ = frame.hint.write_str(HINT_CHORD);
}

fn external_frame(menu: &Menu, screen: &Screen, frame: &mut Frame) {
    base(menu, screen, frame);
    push(frame, "This editor is not part of this build yet", "", Tone::Dim);
    let _ = frame.help[0].write_str("Its content arrives with the package that owns it.");
    let _ = frame.hint.write_str("B back");
}

/// The frame of whatever the menu shows now. Grid screens (`Screen::Grid`) are drawn by `grids`.
pub fn frame_of(menu: &Menu, frame: &mut Frame) {
    let Some(screen) = menu.screen().copied() else {
        page_frame(menu, frame);
        return;
    };
    match screen {
        Screen::List { row, cursor } => list_frame(menu, &screen, row, cursor, frame),
        Screen::Checklist { row, cursor } => checklist_frame(menu, &screen, row, cursor, frame),
        Screen::Delay { row, cursor, editing } => delay_frame(menu, &screen, row, cursor, editing, frame),
        Screen::Slider { row, state } => slider_frame(menu, &screen, row, state, frame),
        Screen::Directions { row, chip } => directions_frame(menu, &screen, row, chip, frame),
        Screen::SwapGroups { cursor } => swap_frame(menu, &screen, cursor, frame),
        Screen::Overrides { situation, cursor } => overrides_frame(menu, &screen, situation, cursor, frame),
        Screen::Chords { cursor } => chords_frame(menu, &screen, cursor, frame),
        Screen::ChordEntry { captured, .. } => chord_entry_frame(menu, &screen, captured, frame),
        Screen::Grid { layout, page, cell } => grid_frame(menu, &screen, layout, page, cell, frame),
        Screen::External(_) => external_frame(menu, &screen, frame),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::menu::input::Button;
    use crate::model::menu::table::{Editor, Page};
    use crate::model::menu::{Entry, Env};
    use crate::model::settings::Settings;

    fn menu() -> Menu {
        Menu::new(Settings::factory(), Env::default())
    }

    fn frame(m: &Menu) -> Frame {
        let mut f = Frame::new();
        frame_of(m, &mut f);
        f
    }

    /// Move to the row called `title` on its page and press A.
    fn open(m: &mut Menu, title: &str) {
        let row = ROWS.iter().position(|r| r.title == title).unwrap_or_else(|| panic!("no row {title}"));
        let page = ROWS[row].page();
        for _ in 0..6 {
            if m.page() == page {
                break;
            }
            m.press(Button::R);
        }
        for _ in 0..=m.entry_count(page) {
            if m.focus() == Some(Entry::Row(row)) {
                m.press(Button::A);
                return;
            }
            m.press(Button::Down);
        }
        panic!("could not reach {title}");
    }

    #[test]
    fn a_dropdown_popup_lists_its_options_and_marks_the_current_one() {
        let mut m = menu();
        open(&mut m, "SDI Strength");
        let f = frame(&m);
        assert_eq!(f.row_count, 4);
        assert_eq!(f.rows[3].label.to_string_lossy(), "High");
        assert!(f.rows.iter().take(4).any(|r| r.value.to_string_lossy() == "(current)"));
        assert!(f.crumb.to_string_lossy().contains("SDI Strength"));
        assert!(f.hint.to_string_lossy().contains("A set"));
        assert_eq!(f.group_count, 0);
    }

    #[test]
    fn a_checklist_popup_shows_ticks_and_scrolls_with_the_cursor() {
        let mut m = menu();
        open(&mut m, "Mash");
        for _ in 0..20 {
            m.press(Button::Down);
        }
        let f = frame(&m);
        assert_eq!(f.row_count as usize, ROW_LINES);
        assert!(f.focus.is_some());
        assert_eq!(window_start(20, 30), 20 + 1 + ROW_LINES / 2 - ROW_LINES);
        m.press(Button::A);
        let f = frame(&m);
        assert!(f.rows[f.focus.unwrap() as usize].value.to_string_lossy() == "[x]");
    }

    #[test]
    fn the_delay_popup_greys_the_checklist_once_a_range_is_set_and_says_why() {
        let mut m = menu();
        open(&mut m, "Ledge Delay");
        let f = frame(&m);
        // 31 values plus the Min and Max handles, in a window.
        assert_eq!(f.row_count as usize, ROW_LINES);
        // Down to Min (index 31), A creates the range [0, 0] and starts editing.
        for _ in 0..31 {
            m.press(Button::Down);
        }
        m.press(Button::A);
        m.press(Button::B);
        let f = frame(&m);
        let first = f.rows[0];
        assert_eq!(first.value_tone, Tone::Off, "a ticked entry greys while a range is set");
        // Back on the first entry the help panel names the reason.
        for _ in 0..40 {
            m.press(Button::Up);
        }
        let f = frame(&m);
        assert!(f.help[1].to_string_lossy().contains("range is set"), "{:?}", f.help[1]);
    }

    #[test]
    fn the_swap_groups_overrides_chords_and_chord_editor_have_frames() {
        let mut m = menu();
        open(&mut m, "Situation overrides");
        let f = frame(&m);
        assert!(f.rows[0].label.to_string_lossy().starts_with("Situation"));
        assert_eq!(f.rows[0].value.to_string_lossy(), "<  Ledge Neutral Getup  >");
        let mut m = menu();
        m.press(Button::R);
        m.press(Button::R);
        open(&mut m, "Button chords");
        let f = frame(&m);
        assert_eq!(f.row_count, 6);
        m.press(Button::A);
        let f = frame(&m);
        assert!(f.crumb.to_string_lossy().contains("Save State Save"));
        assert!(f.hint.to_string_lossy().contains("press the others"));
        let _ = (Page::Cpu, Editor::GiveItem);
    }

    #[test]
    fn a_direction_grid_shows_the_chips_with_their_weights_and_says_the_choice_in_words() {
        let mut m = menu();
        open(&mut m, "DI Direction");
        m.press(Button::A);
        let f = frame(&m);
        assert_eq!(f.view, View::Chips);
        assert_eq!(f.chip_focus, 8, "the focus starts on Neutral");
        assert_eq!(f.chip_weights[8], 1);
        assert!(f.help[1].to_string_lossy().contains("Neutral x1"), "{:?}", f.help[1]);
    }

    fn open_grid(mode: crate::model::menu::grid::ConsoleMode, title: &str) -> Menu {
        let mut m = Menu::new(Settings::factory(), Env { console: mode, ..Env::default() });
        open(&mut m, title);
        m
    }

    #[test]
    fn the_item_grid_is_one_docked_page_of_38_short_names_and_three_handheld_pages() {
        use crate::model::menu::grid::ConsoleMode;
        let m = open_grid(ConsoleMode::Docked, "Give item");
        let f = frame(&m);
        assert_eq!((f.view, f.tile_cols, f.tile_rows, f.tile_count), (View::Tiles, 5, 8, 38));
        assert!(f.tiles[..38].iter().all(|t| !t.is_empty() && t.units().len() <= 14));
        let mut m = open_grid(ConsoleMode::Handheld, "Give item");
        let f = frame(&m);
        assert_eq!((f.tile_cols, f.tile_rows, f.tile_count), (4, 4, 16));
        m.press(Button::R);
        m.press(Button::R);
        assert_eq!(frame(&m).tile_count, 6, "the third page holds the last 6 items");
        assert!(frame(&m).crumb.to_string_lossy().contains("page 3 of 3"));
    }

    #[test]
    fn the_stage_grid_pages_follow_the_catalog_docked_and_handheld() {
        use crate::model::menu::grid::ConsoleMode;
        let mut m = open_grid(ConsoleMode::Docked, "Lobby Stages");
        let mut counts = Vec::new();
        for _ in 0..3 {
            counts.push(frame(&m).tile_count);
            m.press(Button::R);
        }
        assert_eq!(counts, [40, 40, (crate::model::menu::grid::STAGE_TILES - 80) as u8]);
        assert_eq!(frame(&m).tiles[0].to_string_lossy(), "All");
        let m = open_grid(ConsoleMode::Handheld, "Lobby Stages");
        let pages = crate::model::menu::grid::STAGE_TILES.div_ceil(16);
        assert!(frame(&m).crumb.to_string_lossy().contains(&format!("page 1 of {pages}")));
    }
    #[test]
    fn every_sub_screen_of_every_row_draws_within_the_budget() {
        // Open the screen of every row that has one and check the frame stays inside the layout's lines.
        for (i, row) in ROWS.iter().enumerate() {
            if matches!(row.control, Control::Toggle | Control::ReplayCapture | Control::Info(_)) {
                continue;
            }
            let mut m = Menu::new(Settings::factory(), Env { developer_page: true, pilot_present: false, ..Env::default() });
            let page = row.page();
            for _ in 0..6 {
                if m.page() == page {
                    break;
                }
                m.press(Button::R);
            }
            let mut reached = false;
            for _ in 0..=m.entry_count(page) {
                if m.focus() == Some(Entry::Row(i)) {
                    reached = true;
                    break;
                }
                m.press(Button::Down);
                if let Some(Entry::Group(g)) = m.focus() {
                    m.press(Button::A);
                    let _ = g;
                }
            }
            if !reached {
                continue;
            }
            m.press(Button::A);
            let f = frame(&m);
            assert!(f.row_count as usize <= ROW_LINES, "{}", row.title);
        }
    }
}