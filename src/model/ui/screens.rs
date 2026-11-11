//! The sub-screens of the menu as frames (SPEC-core-03 §2.2, §2.5): the popup lists, the delay popup, the
//! slider, the direction grid's list form, the swap groups, the situation overrides, the button chords, the
//! chord editor and the editors whose content belongs to a later package. Every one is drawn in the same
//! panes as the page (the right-hand rows, the breadcrumb, the help panel); the grids with pictures
//! (`Screen::Directions` chips, `Screen::Grid`) have their own panes and frames in `grids`.

use core::fmt::Write;

use super::frame::{page_frame, ChipVisual, RESET_LABEL, Frame, RangeView, RowLine, Text, Tone, View};
use super::menu::chord::Chord;
use super::menu::options::{ACTIONS, CHORD_ROWS, DIRECTIONS, SITUATIONS, SWAP_GROUPS, YES_NO_RANDOM};
use super::catalog::ITEMS;
use super::menu::grid::{GridKind, GridLayout, RosterUse};
use super::menu::table::{Control, Editor, Row, GROUPS, ROWS};
use super::menu::{ChordCapture, ChordTarget, Grey, Menu, Screen, SliderState, DELAY_ROWS, OVERRIDE_COLUMN};
use super::panes::ROWS as ROW_LINES;

const HINT_LIST: &str = "Up/Down choose   ZL+Up/Down jump   A set   B back";
const HINT_CHECKLIST: &str = "Up/Down move   ZL+Up/Down jump   A tick   ZL all or none   B back";
const HINT_DELAY: &str = "Up/Down move   ZL+Up/Down jump   A tick or edit the handle   Y clear   ZL all   B back";
const HINT_SLIDER: &str = "Left/Right other handle   A pick up or put down   Up/Down/Left/Right change   B back";
const HINT_DIRECTIONS: &str = "Up/Down move   A add one weight   Y clear   ZL all or none   B back";
const HINT_CHORD: &str = "X capture   A confirm   Y empty   B cancel";

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
        Screen::ConfirmReset => write!(out, "  >  {RESET_LABEL}"),
        Screen::Grid { .. } => Ok(()),
        _ => Ok(()),
    };
}

fn base(menu: &Menu, screen: &Screen, frame: &mut Frame) {
    page_frame(menu, frame);
    // The sub-screen owns the right-hand side: no groups, no rows of the page, its own crumb.
    frame.group_count = 0;
    frame.rows.fill(RowLine::default());
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
        GridKind::Stages | GridKind::StageChange => stage(n.saturating_sub(1)),
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
    frame.tile_stage_grid = matches!(layout.kind, GridKind::Stages | GridKind::StageChange);
    frame.tile_item_grid = layout.kind == GridKind::Items;
    frame.tile_centre_last = matches!(layout.kind, GridKind::Roster(_));
    frame.tile_sig = ((page as u32) << 8) | u32::from(layout.cols) | (u32::from(layout.rows) << 4);
    frame.tile_focus = cell as u8;
    let title = match layout.kind {
        GridKind::Items => "Give item",
        GridKind::Stages => "Stage grid",
        GridKind::StageChange => "Change Stage",
        GridKind::Roster(RosterUse::PlayerPick) => "Player Character",
        GridKind::Roster(RosterUse::CpuPick) => "CPU Character",
        GridKind::Roster(RosterUse::Cycles) => "Character Cycles",
        GridKind::Roster(RosterUse::Hats) => "Kirby Hats",
    };
    let _ = write!(frame.crumb, "  >  {title}   page {} of {}", page + 1, layout.pages());
    let _ = frame.help[0].write_str(match layout.kind {
        GridKind::Items => "A gives Player 1 this item now.",
        GridKind::Stages => "Stages the lobby cycles through; All takes every stage.",
        GridKind::StageChange => "The stage to go to now, in Training or the waiting room.",
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

/// `tile_state` of a greyed cell: a delay box while a range is set, or a roster tile the editing role cannot choose.
pub const TILE_GREYED: u8 = 2;

const HINT_PANEL: &str = "D-pad move   A confirm   Y delete or other   B back";

/// A panel of the named-macro / replay layer (package 4): its title, the labels of its 4 x 8 page (an empty one draws nothing),
/// the focused cell and a footer of at most two lines. Drawn in the tile panes, the same ones the item grid uses, under the
/// page's tabs; the footer is the two-line help panel.
pub fn panel_frame<S: AsRef<str>>(menu: &Menu, title: &str, footer: &str, labels: &[S], selected: usize, frame: &mut Frame) {
    page_frame(menu, frame);
    frame.group_count = 0;
    frame.rows.fill(RowLine::default());
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

const HINT_FIGHTER_QUESTION: &str = "D-pad move   A choose   B cancel";

/// The shared Fighter Mismatch question: a panel whose keys are the question's own (it has no Y action).
pub fn fighter_question_frame<S: AsRef<str>>(menu: &Menu, title: &str, footer: &str, labels: &[S], selected: usize, frame: &mut Frame) {
    panel_frame(menu, title, footer, labels, selected, frame);
    frame.hint.clear();
    let _ = frame.hint.write_str(HINT_FIGHTER_QUESTION);
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
    let _ = frame.hint.write_str("Up/Down move   ZL+Up/Down jump   A switch   B back");
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
    let _ = frame.hint.write_str("Left/Right situation or column   Up/Down move   ZL+Up/Down jump   A tick   ZL all or none   B back");
}
fn chords_frame(menu: &Menu, screen: &Screen, cursor: usize, frame: &mut Frame) {
    base(menu, screen, frame);
    lines_window(frame, cursor, CHORD_ROWS.len(), |i, line| {
        let (name, id) = CHORD_ROWS[i];
        let _ = line.label.write_str(name);
        let chord = Chord::from_row(menu.settings().menu.get(id).unwrap_or(&[]));
        let _ = write!(line.value, "{chord}  >");
    });
    let _ = frame.help[0].write_str("Choose a row; X in its editor starts physical chord capture.");
    let _ = frame.hint.write_str("Up/Down move   ZL+Up/Down jump   A change the chord   B back");
}

fn chord_entry_frame(menu: &Menu, screen: &Screen, captured: Chord, frame: &mut Frame) {
    base(menu, screen, frame);
    push(frame, "Chord", &format!("{captured}"), Tone::On);
    frame.focus = Some(0);
    let _ = frame.help[0].write_str("X starts capture: release all, hold the chord, then release all to preview.");
    let _ = frame.help[1].write_str("An empty chord never passes (a menu shortcut with none is disabled).");
    let hint = match menu.chord_capture() {
        ChordCapture::Preview => HINT_CHORD,
        ChordCapture::Release => "Release all buttons to start capture",
        ChordCapture::Buttons(_) => "Hold one button, press the others; release all to preview",
    };
    let _ = frame.hint.write_str(hint);
    if let Some(why) = menu.chord_refusal() {
        frame.help[1].clear();
        let _ = frame.help[1].write_str(why.text());
    }
}

fn external_frame(menu: &Menu, screen: &Screen, frame: &mut Frame) {
    base(menu, screen, frame);
    push(frame, "This editor is not part of this build yet", "", Tone::Dim);
    let _ = frame.help[0].write_str("Its content arrives with the package that owns it.");
    let _ = frame.hint.write_str("B back");
}

fn confirm_reset_frame(menu: &Menu, screen: &Screen, frame: &mut Frame) {
    base(menu, screen, frame);
    push(frame, "Reset this page to your saved defaults?", "", Tone::Normal);
    push(frame, "A  reset", "", Tone::On);
    push(frame, "B  cancel", "", Tone::Normal);
    frame.focus = Some(1);
    let _ = write!(frame.help[0], "Every setting on {} goes back to your saved defaults.", menu.page().title());
    // The menu shortcuts are not settings of the page (they live in the shortcuts file), so the reset leaves them.
    let shortcuts = ROWS.iter().any(|r| {
        r.page() == menu.page() && r.visible(menu.env().pilot_present) && matches!(r.control, Control::Editor(Editor::ShortcutSlot(_)))
    });
    let _ = frame.help[1].write_str(if shortcuts {
        "The menu shortcuts stay as they are. Other pages are not changed."
    } else {
        "Other pages are not changed."
    });
    let _ = frame.hint.write_str("A reset   B cancel");
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
        Screen::ConfirmReset => confirm_reset_frame(menu, &screen, frame),
    }
}
