//! The sub-screens of the menu as frames (SPEC-core-03 §2.2, §2.5): the popup lists, the delay popup, the
//! slider, the direction grid's list form, the swap groups, the situation overrides, the button chords, the
//! chord editor and the editors whose content belongs to a later package. Every one is drawn in the same
//! panes as the page (the right-hand rows, the breadcrumb, the help panel); the grids with pictures
//! (`Screen::Directions` chips, `Screen::Grid`) have their own panes and frames in `grids`.

use core::fmt::Write;

use super::frame::{page_frame, ChipVisual, Frame, RangeView, RowLine, Text, Tone, View};
use super::menu::chord::Chord;
use super::menu::options::{ACTIONS, CHORD_ROWS, DIRECTIONS, SITUATIONS, SWAP_GROUPS, YES_NO_RANDOM};
use super::catalog::ITEMS;
use super::menu::grid::{GridKind, GridLayout, RosterUse};
use super::menu::table::{Control, Row, GROUPS, ROWS};
use super::menu::{ChordTarget, Grey, Menu, Screen, SliderState, DELAY_ROWS, OVERRIDE_COLUMN};
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

/// The delay popup (N3, mockup 19): the checklist as a grid of boxes (a ticked value on the accent) over the min-max
/// range control. One of the two is greyed while the other holds a value; the help panel says why on the greyed one.
fn delay_frame(menu: &Menu, screen: &Screen, row: usize, cursor: usize, editing: bool, frame: &mut Frame) {
    base(menu, screen, frame);
    let r = &ROWS[row];
    let Control::MultiSelectRange { first, step, range_id } = r.control else { return };
    let settings = &menu.settings().menu;
    let ticked = settings.get(r.id).map(<[u8]>::to_vec).unwrap_or_default();
    let range = settings.get(range_id).map(<[u8]>::to_vec).unwrap_or_default();
    let (check_grey, range_grey) = menu.delay_grey(row);
    let count = ticked.len().min(DELAY_TILES);
    frame.view = View::Delay;
    frame.tile_cols = DELAY_TILES.div_ceil(DELAY_ROWS) as u8;
    frame.tile_rows = DELAY_ROWS as u8;
    frame.tile_count = count as u8;
    for (i, mark) in ticked.iter().enumerate().take(count) {
        let _ = write!(frame.tiles[i], "{}", first + step * i as u16);
        frame.tile_state[i] = if check_grey.is_some() { 2 } else { u8::from(*mark != 0) };
    }
    let on_handle = cursor >= ticked.len();
    frame.tile_focus = if on_handle { 255 } else { cursor as u8 };
    let top = ticked.len().saturating_sub(1) as u8;
    frame.range = RangeView {
        set: range.len() == 2,
        min: range.first().copied().unwrap_or(0),
        max: range.get(1).copied().unwrap_or(0),
        top,
        grey: range_grey.is_some(),
        focus: if on_handle { (cursor - ticked.len() + 1) as u8 } else { 0 },
        editing,
    };
    // Four text lines the view places: the checklist's header and count, the range's header and value, then the
    // two handle labels.
    let n_ticked = ticked.iter().filter(|v| **v != 0).count();
    let rows = &mut frame.rows;
    let _ = rows[0].label.write_str("Checklist");
    let _ = if n_ticked == 0 { rows[0].value.write_str("none ticked") } else { write!(rows[0].value, "{n_ticked} ticked") };
    let _ = rows[1].label.write_str("Range (min - max)");
    match (frame.range.set, range.as_slice()) {
        (true, [lo, hi]) => {
            let _ = write!(rows[1].value, "{} - {}", first + step * u16::from(*lo), first + step * u16::from(*hi));
            let _ = write!(rows[2].label, "min {}", first + step * u16::from(*lo));
            let _ = write!(rows[3].label, "max {}", first + step * u16::from(*hi));
        }
        _ => {
            let _ = rows[1].value.write_str("not set");
            rows[1].value_tone = Tone::Dim;
        }
    }
    if check_grey.is_some() {
        rows[0].label_tone = Tone::Off;
        rows[0].value_tone = Tone::Off;
    }
    if range_grey.is_some() {
        rows[1].label_tone = Tone::Off;
        rows[1].value_tone = Tone::Off;
    }
    frame.row_count = 4;
    // A greyed control says why on the help panel's second line.
    let reason = if on_handle { range_grey } else { check_grey };
    if let Some(grey) = reason {
        frame.help[1].clear();
        let _ = frame.help[1].write_str(Grey::text(grey));
    }
    let _ = frame.hint.write_str(HINT_DELAY);
}

/// The delay popup's checklist grid: columns of DELAY_ROWS boxes, up to 32 boxes (the longest value set has 31).
pub const DELAY_TILES: usize = 32;
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
    frame.chip_visual = match ROWS[row].id {
        "di_state" => ChipVisual::Di,
        "sdi_state" => ChipVisual::Sdi,
        _ => ChipVisual::None,
    };
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
/// The label of stage `i` of the catalog (package 6's list, handed in by the runtime: this module is also compiled by
/// the by-path test crates, which do not have the engine).
pub type StageLabel = fn(usize) -> &'static str;

fn tile_label(kind: GridKind, n: usize, short: bool, stage: StageLabel, out: &mut Text<28>) {
    let name: &str = match kind {
        GridKind::Items => ITEMS.get(n).copied().unwrap_or("-"),
        GridKind::Stages if n == 0 => "All",
        GridKind::Stages => stage(n - 1),
        // The roster editor (engine::swap::roster_grid) writes the names, ranks and picks: this module has no engine.
        GridKind::Roster(_) => "",
    };
    if short {
        // Docked tiles are narrow (14 characters at the small font). A longer name is cut to 12 characters and ends in
        // ".." so a cut is visible and two names with the same first word (Mushroom Kingdom, Mushroomy Kingdom) stay apart.
        if name.chars().count() <= 14 {
            let _ = out.write_str(name);
        } else {
            let cut = name.char_indices().nth(12).map_or(name.len(), |(i, _)| i);
            let _ = out.write_str(name[..cut].trim_end());
            let _ = out.write_str("..");
        }
    } else {
        let _ = out.write_str(name);
    }
}

fn grid_frame(menu: &Menu, screen: &Screen, layout: GridLayout, page: usize, cell: usize, stage: StageLabel, frame: &mut Frame) {
    base(menu, screen, frame);
    frame.view = View::Tiles;
    frame.tile_cols = layout.cols;
    frame.tile_rows = layout.rows;
    let len = layout.page_len(page).min(frame.tiles.len());
    for i in 0..len {
        if let Some(n) = layout.tile(page, i) {
            tile_label(layout.kind, n, layout.short_names, stage, &mut frame.tiles[i]);
            frame.tile_n[i] = n as u16;
        }
    }
    frame.tile_count = len as u8;
    frame.tile_stage_grid = layout.kind == GridKind::Stages;
    frame.tile_centre_last = matches!(layout.kind, GridKind::Roster(_));
    frame.tile_sig = ((page as u32) << 8) | u32::from(layout.cols) | (u32::from(layout.rows) << 4);
    frame.tile_focus = cell as u8;
    let title = match layout.kind {
        GridKind::Items => "Give item",
        GridKind::Stages => "Stage grid",
        GridKind::Roster(RosterUse::PlayerPick) => "Player Character",
        GridKind::Roster(RosterUse::CpuPick) => "CPU Character",
        GridKind::Roster(RosterUse::Cycles) => "Character Cycles",
        GridKind::Roster(RosterUse::Hats) => "Kirby Hats",
    };
    let _ = write!(frame.crumb, "  >  {title}   page {} of {}", page + 1, layout.pages());
    let _ = frame.help[0].write_str(match layout.kind {
        GridKind::Items => "A gives Player 1 this item now.",
        GridKind::Stages => "Stages the lobby cycles through; All takes every stage.",
        GridKind::Roster(RosterUse::PlayerPick | RosterUse::CpuPick) => "A picks the fighter; X shows the next form of a shared slot.",
        GridKind::Roster(RosterUse::Cycles) => "A adds in order, R removes, Y marks Solo Popo on Ice Climbers, X previews forms, ZR switches role.",
        GridKind::Roster(RosterUse::Hats) => "A adds in order (All takes everyone), R removes, X previews forms, ZR switches role.",
    });
    let _ = frame.hint.write_str(match layout.kind {
        GridKind::Roster(RosterUse::PlayerPick | RosterUse::CpuPick) => "D-pad move   A pick   X form   B back",
        GridKind::Roster(RosterUse::Cycles) => "D-pad move   A pick   R remove   Y Solo Popo   X form   ZR role   B back",
        GridKind::Roster(RosterUse::Hats) => "D-pad move   A pick   R remove   X form   ZR role   B back",
        _ => "D-pad move   L/R page   B back",
    });
}
/// Cells of a panel of the named-macro / replay layer: 4 columns by 8 rows (the layer's own page, `NX_SUBMENU_*`).
pub const PANEL_COLS: usize = 4;
pub const PANEL_ROWS: usize = 8;
pub const PANEL_CELLS: usize = PANEL_COLS * PANEL_ROWS;

/// `tile_state` of a cell that draws nothing (an empty label of the layer's page).
pub const TILE_HIDDEN: u8 = 255;

const HINT_PANEL: &str = "D-pad move   A confirm   Y delete or other   B back";

/// A panel of the named-macro / replay layer (package 4): its title, the labels of its 4 x 8 page (an empty one draws nothing),
/// the focused cell and a footer of at most two lines. Drawn in the tile panes, the same ones the item grid uses, under the
/// page's tabs; the footer is the two-line help panel.
pub fn panel_frame<S: AsRef<str>>(menu: &Menu, title: &str, footer: &str, labels: &[S], selected: usize, frame: &mut Frame) {
    page_frame(menu, frame);
    frame.group_count = 0;
    frame.rows = Frame::new().rows;
    frame.row_count = 0;
    frame.focus = None;
    frame.help = [Text::new(); 2];
    frame.hint.clear();
    let _ = write!(frame.crumb, "{}  >  {}", menu.page().title(), title);
    frame.view = View::Tiles;
    frame.tile_cols = PANEL_COLS as u8;
    frame.tile_rows = PANEL_ROWS as u8;
    let count = labels.len().min(PANEL_CELLS);
    frame.tile_count = count as u8;
    for (i, label) in labels.iter().take(count).enumerate() {
        let _ = frame.tiles[i].write_str(label.as_ref());
        if label.as_ref().is_empty() {
            frame.tile_state[i] = TILE_HIDDEN;
        }
    }
    frame.tile_focus = selected.min(count.saturating_sub(1)) as u8;
    for (line, text) in footer.split('\n').take(2).enumerate() {
        let _ = frame.help[line].write_str(text);
    }
    let _ = frame.hint.write_str(HINT_PANEL);
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

/// Situation overrides (mockup 06): the situation dropdown line, then the actions as boxes in columns of 10;
/// a ticked action is on the accent. Left / Right on the situation line change it, on a box they step a column.
fn overrides_frame(menu: &Menu, screen: &Screen, situation: usize, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    let (name, id) = SITUATIONS[situation];
    let current = menu.settings().menu.get(id).map(<[u8]>::to_vec).unwrap_or_default();
    frame.view = View::Overrides;
    frame.tile_cols = ACTIONS.len().div_ceil(OVERRIDE_COLUMN) as u8;
    frame.tile_rows = OVERRIDE_COLUMN as u8;
    frame.tile_count = ACTIONS.len() as u8;
    for (i, action) in ACTIONS.iter().enumerate() {
        let _ = frame.tiles[i].write_str(action);
        frame.tile_state[i] = u8::from(current.get(i).copied().unwrap_or(0) != 0);
    }
    frame.tile_focus = if cursor == 0 { 255 } else { (cursor - 1) as u8 };
    let _ = write!(frame.rows[0].label, "Situation  ({} of {})", situation + 1, SITUATIONS.len());
    let _ = write!(frame.rows[0].value, "<  {name}  >");
    frame.row_count = 1;
    frame.focus = (cursor == 0).then_some(0);
    let _ = frame.help[0].write_str("The actions the CPU may take in this situation (one at random).");
    frame.help[1].clear();
    let _ = frame.hint.write_str("Left/Right situation or column   Up/Down move   A tick   ZL all or none   B back");
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
pub fn frame_of(menu: &Menu, frame: &mut Frame, stage: StageLabel) {
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
        Screen::Grid { layout, page, cell } => grid_frame(menu, &screen, layout, page, cell, stage, frame),
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
        frame_of(m, &mut f, |i| if i == 0 { "Battlefield" } else { "Stage" });
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
        assert_eq!(f.rows[3].label.string_lossy(), "High");
        assert!(f.rows.iter().take(4).any(|r| r.value.string_lossy() == "(current)"));
        assert!(f.crumb.string_lossy().contains("SDI Strength"));
        assert!(f.hint.string_lossy().contains("A set"));
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
        assert!(f.rows[f.focus.unwrap() as usize].value.string_lossy() == "[x]");
    }

    #[test]
    fn the_delay_popup_greys_the_checklist_once_a_range_is_set_and_says_why() {
        let mut m = menu();
        open(&mut m, "Ledge Delay");
        let f = frame(&m);
        // The popup is the delay view: 31 boxes in 4 rows of columns, then the range control (none set yet).
        assert_eq!(f.view, View::Delay);
        assert_eq!((f.tile_count, f.tile_rows, f.tile_cols), (31, 4, 8));
        assert_eq!(f.tiles[1].string_lossy(), "10");
        assert!(!f.range.set && !f.range.grey && f.range.top == 30);
        // Down to Min (index 31), A creates the range [0, 0] and starts editing.
        for _ in 0..31 {
            m.press(Button::Down);
        }
        m.press(Button::A);
        m.press(Button::B);
        let f = frame(&m);
        assert!(f.range.set && f.range.focus == 1 && !f.range.editing);
        assert_eq!(f.tile_state[0], 2, "a box greys while a range is set");
        assert_eq!(f.rows[1].value.string_lossy(), "0 - 0");
        // Back on the first box the help panel names the reason.
        for _ in 0..40 {
            m.press(Button::Up);
        }
        let f = frame(&m);
        assert!(f.help[1].string_lossy().contains("range is set"), "{:?}", f.help[1]);
    }

    #[test]
    fn left_and_right_step_a_column_of_the_delay_checklist_and_stop_at_the_edge() {
        let mut m = menu();
        open(&mut m, "Ledge Delay");
        m.press(Button::Right);
        assert_eq!(frame(&m).tile_focus, 4);
        m.press(Button::Left);
        m.press(Button::Left);
        assert_eq!(frame(&m).tile_focus, 0, "no step past the first column");
        for _ in 0..9 {
            m.press(Button::Right);
        }
        assert_eq!(frame(&m).tile_focus, 28, "the last full column; 32 would be past the 31 boxes");
        m.press(Button::A);
        assert_eq!(frame(&m).tile_state[28], 1, "a tick shows");
    }
    #[test]
    fn the_overrides_screen_is_columns_of_boxes_and_left_right_step_a_column() {
        let mut m = menu();
        open(&mut m, "Situation overrides");
        let f = frame(&m);
        assert_eq!((f.view, f.tile_count, f.tile_rows, f.tile_cols), (View::Overrides, 30, 10, 3));
        assert_eq!(f.tile_focus, 255, "the cursor starts on the situation line");
        assert_eq!(f.focus, Some(0));
        m.press(Button::Down);
        assert_eq!(frame(&m).tile_focus, 0);
        m.press(Button::Right);
        m.press(Button::Right);
        assert_eq!(frame(&m).tile_focus, 20, "two columns right, same row");
        m.press(Button::Right);
        assert_eq!(frame(&m).tile_focus, 20, "no step past the last column");
        m.press(Button::Left);
        m.press(Button::Left);
        m.press(Button::Left);
        assert_eq!(frame(&m).tile_focus, 0, "no step past the first column");
        m.press(Button::A);
        assert_eq!(frame(&m).tile_state[0], 1);
    }
    #[test]
    fn the_swap_groups_overrides_chords_and_chord_editor_have_frames() {
        let mut m = menu();
        open(&mut m, "Situation overrides");
        let f = frame(&m);
        assert!(f.rows[0].label.string_lossy().starts_with("Situation"));
        assert_eq!(f.rows[0].value.string_lossy(), "<  Ledge Neutral Getup  >");
        let mut m = menu();
        m.press(Button::R);
        m.press(Button::R);
        open(&mut m, "Button chords");
        let f = frame(&m);
        assert_eq!(f.row_count, 6);
        m.press(Button::A);
        let f = frame(&m);
        assert!(f.crumb.string_lossy().contains("Save State Save"));
        assert!(f.hint.string_lossy().contains("press the others"));
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
        assert!(f.help[1].string_lossy().contains("Neutral x1"), "{:?}", f.help[1]);
        assert_eq!(f.chip_visual, ChipVisual::Di, "the DI grid draws the game's DI visual");
    }

    #[test]
    fn only_the_di_and_sdi_grids_ask_for_the_games_visual() {
        for (title, want) in [("DI Direction", ChipVisual::Di), ("SDI Direction", ChipVisual::Sdi), ("Airdodge Direction", ChipVisual::None)] {
            let mut m = menu();
            open(&mut m, title);
            m.press(Button::A);
            let f = frame(&m);
            assert_eq!(f.view, View::Chips, "{title}");
            assert_eq!(f.chip_visual, want, "{title}");
        }
    }

    fn open_grid(mode: crate::model::menu::grid::ConsoleMode, title: &str) -> Menu {
        let mut m = Menu::new(Settings::factory(), Env { console: mode, ..Env::default() });
        open(&mut m, title);
        m
    }

    #[test]
    fn a_on_the_give_item_grid_asks_for_that_item_every_time_and_the_stage_grid_asks_for_nothing() {
        use crate::model::menu::grid::ConsoleMode;
        let mut m = open_grid(ConsoleMode::Docked, "Give item");
        m.press(Button::Right);
        m.press(Button::Right);
        m.press(Button::A);
        assert_eq!(m.take_give_item(), Some(2));
        assert_eq!(m.take_give_item(), None, "taken once");
        m.press(Button::A);
        assert_eq!(m.take_give_item(), Some(2), "the already-chosen item is asked for again");
        // Handheld: the third page starts at item 32.
        let mut m = open_grid(ConsoleMode::Handheld, "Give item");
        m.press(Button::R);
        m.press(Button::R);
        m.press(Button::A);
        assert_eq!(m.take_give_item(), Some(32));
        let mut m = open_grid(ConsoleMode::Docked, "Lobby Stages");
        m.press(Button::A);
        assert_eq!(m.take_give_item(), None);
    }

    #[test]
    fn a_docked_label_that_is_cut_ends_in_two_dots_and_names_with_one_first_word_stay_apart() {
        let mut a = Text::<28>::new();
        let mut b = Text::<28>::new();
        let mut c = Text::<28>::new();
        tile_label(GridKind::Items, 0, true, |_| "", &mut a);
        assert!(a.units().len() <= 14);
        fn stage(n: usize) -> &'static str {
            ["Mushroom Kingdom", "Mushroomy Kingdom", "Final Destination"][n]
        }
        tile_label(GridKind::Stages, 1, true, stage, &mut b);
        tile_label(GridKind::Stages, 2, true, stage, &mut c);
        assert_eq!(b.string_lossy(), "Mushroom Kin..");
        assert_eq!(c.string_lossy(), "Mushroomy Ki..");
        let mut d = Text::<28>::new();
        tile_label(GridKind::Stages, 3, false, stage, &mut d);
        assert_eq!(d.string_lossy(), "Final Destination");
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
        assert!(frame(&m).crumb.string_lossy().contains("page 3 of 3"));
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
        assert_eq!(frame(&m).tiles[0].string_lossy(), "All");
        let m = open_grid(ConsoleMode::Handheld, "Lobby Stages");
        let pages = crate::model::menu::grid::STAGE_TILES.div_ceil(16);
        assert!(frame(&m).crumb.string_lossy().contains(&format!("page 1 of {pages}")));
    }
    #[test]
    fn a_panel_of_the_macro_layer_is_a_4_by_8_tile_page_with_a_two_line_footer() {
        let m = menu();
        let mut labels = vec![String::new(); PANEL_CELLS];
        labels[0] = "Jab string · Mario".into();
        labels[1] = "Ledge trap · Pikachu".into();
        let mut f = Frame::new();
        panel_frame(&m, "Named Library", "Ledge trap · Pikachu\npage 1/1  skipped 0    A loads, B closes.", &labels, 1, &mut f);
        assert_eq!((f.view, f.tile_cols, f.tile_rows, f.tile_count), (View::Tiles, 4, 8, 32));
        assert_eq!(f.tiles[0].string_lossy(), "Jab string · Mario");
        assert_eq!((f.tile_state[0], f.tile_state[2], f.tile_state[31]), (0, TILE_HIDDEN, TILE_HIDDEN));
        assert_eq!(f.tile_focus, 1);
        assert_eq!(f.crumb.string_lossy(), "CPU  >  Named Library");
        assert_eq!(f.help[0].string_lossy(), "Ledge trap · Pikachu");
        assert!(f.help[1].string_lossy().starts_with("page 1/1"));
        assert_eq!(f.group_count, 0);
        assert_eq!(f.row_count, 0);
        // A one-label panel (the system keyboard's "Name and save") and a focus past the labels are clamped.
        let mut f = Frame::new();
        panel_frame(&m, "Name macro", "Recording · 12f · 0.20s\nA: system keyboard   B: return", &["Name and save"], 9, &mut f);
        assert_eq!((f.tile_count, f.tile_focus), (1, 0));
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
