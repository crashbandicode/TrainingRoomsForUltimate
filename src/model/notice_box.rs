//! The compact notice box (2026-11-17; it replaces the centred plates and the practice games' top band of
//! `docs/decisions/rc14-targets-preview.md`): every `TrModDisp0` notice is one dark plate sized to its text plus a small
//! padding, at the top left of the screen, wrapped so it stays left of the frame strip, and stacked under the BTT compact
//! chip while that shows. Pure: the HUD draw writes the boxes this returns. Canvas units (1920 × 1080, y up); a box is
//! `[centre x, centre y, w, h]`, as the panes are placed.

use super::panes::{NOTICE_BODY_FONT, NOTICE_HEADER_FONT, NOTICE_LEFT, NOTICE_MAX_W, NOTICE_TOP};

/// Text to plate edge: 10 px left and right, 6 px top and bottom (plus the font's own leading) at 1280 × 720.
pub const PAD_X: f32 = 15.0;
pub const PAD_Y: f32 = 9.0;
/// Between the header line and the body.
pub const LINE_GAP: f32 = 2.0;
/// Between the box and the BTT chip it stacks under (8 px).
pub const GAP: f32 = 12.0;
/// The widest a wrapped line may be.
pub const MAX_TEXT_W: f32 = NOTICE_MAX_W - 2.0 * PAD_X;
/// The most body lines a notice keeps (a longer text ends in `…`): with the wide BTT chip over it, the tallest box still
/// ends above the player HUD.
pub const MAX_LINES: usize = 14;
/// The header pane's capacity (`panes::spec(IDX_NOTICE_HEADER)`).
pub const HEADER_UNITS: usize = 24;
const ELLIPSIS: u16 = 0x2026;
const NEWLINE: u16 = b'\n' as u16;
const SPACE: u16 = b' ' as u16;
const SEPARATOR: u16 = 0xb7;

/// One UTF-16 unit's advance in ems of the font's x size. The classes are calibrated on four `nintendo64` strings
/// measured on Eden stills (design report: within -1.7 % to +4.2 %); any other non-ASCII unit, such as a button
/// glyph, counts a full em, and so do `M` and `W` (a 48-W name renders 1.08 em a W in the menu font, Eden 2026-11-17).
pub fn advance(unit: u16) -> f32 {
    match char::from_u32(u32::from(unit)).unwrap_or('\u{fffd}') {
        ' ' => 0.32,
        'i' | 'l' | 'j' | '.' | ',' | ':' | ';' | '\'' | '!' | '|' | '·' => 0.30,
        'f' | 'r' | 't' | 'I' | '(' | ')' | '[' | ']' | '/' | '-' => 0.38,
        'm' | 'w' => 0.82,
        'A'..='L' | 'N'..='V' | 'X'..='Z' => 0.72,
        '0'..='9' => 0.58,
        c if c.is_ascii() && !matches!(c, 'M' | 'W') => 0.55,
        _ => 1.0,
    }
}

/// The widest line of `units` at x size `font`, and the number of lines (0 for no text).
pub fn measure(units: impl IntoIterator<Item = u16>, font: f32) -> (f32, usize) {
    let (mut widest, mut line, mut lines) = (0.0f32, 0.0, 0);
    for unit in units {
        lines = lines.max(1);
        if unit == NEWLINE {
            lines += 1;
            line = 0.0;
        } else {
            line += advance(unit) * font;
            widest = widest.max(line);
        }
    }
    (widest, lines)
}

/// Wraps `units[..len]` in place so no line is wider than `max` and at most `max_lines` lines remain; returns the new
/// length (at most `units.len()`). A line that would pass `max` breaks at its last space before a `·` separator, else at
/// its last space; a word wider than `max` breaks inside it (a newline is inserted). When the buffer has no room for that
/// newline, or the lines run out, the last line ends in `…` within `max` and the rest is dropped.
pub fn wrap(units: &mut [u16], mut len: usize, max: f32, font: f32, max_lines: usize) -> usize {
    let (mut start, mut width, mut lines, mut i) = (0, 0.0, 1, 0);
    while i < len {
        if units[i] == NEWLINE {
            if lines == max_lines {
                return ellipsis(units, start, i, max, font);
            }
            (start, width, lines, i) = (i + 1, 0.0, lines + 1, i + 1);
            continue;
        }
        let w = advance(units[i]) * font;
        if width + w > max && i > start {
            let space = |sep: bool| {
                (start + 1..=i).rev().find(|&k| units[k] == SPACE && (!sep || (k + 1 < len && units[k + 1] == SEPARATOR)))
            };
            if let Some(at) = space(true).or_else(|| space(false)) {
                (units[at], i) = (NEWLINE, at);
            } else if len < units.len() {
                units.copy_within(i..len, i + 1);
                (units[i], len) = (NEWLINE, len + 1);
            } else {
                return ellipsis(units, start, i, max, font);
            }
            continue;
        }
        width += w;
        i += 1;
    }
    len
}

/// The line `units[start..end]` (no wider than `max`) ends in `…` within `max`; returns the new text length.
fn ellipsis(units: &mut [u16], start: usize, mut end: usize, max: f32, font: f32) -> usize {
    let mut width: f32 = units[start..end].iter().map(|&u| advance(u) * font).sum();
    while end > start && width + advance(ELLIPSIS) * font > max {
        end -= 1;
        width -= advance(units[end]) * font;
    }
    units[end] = ELLIPSIS;
    end + 1
}

/// The panes of one notice at the anchor (the plate's top-left corner at `NOTICE_LEFT`, `NOTICE_TOP`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoticeBox {
    pub plate: [f32; 4],
    pub header: [f32; 4],
    pub body: [f32; 4],
}

/// The notice's header fitted into `heading` (one line) and its UTF-16 body `body[..len]` wrapped in place, both within
/// `MAX_TEXT_W`, so the plate is never wider than `NOTICE_MAX_W`. Returns the boxes and the two texts' lengths.
pub fn layout(header: &str, heading: &mut [u16; HEADER_UNITS], body: &mut [u16], len: usize) -> (NoticeBox, usize, usize) {
    let [head, text] = [NOTICE_HEADER_FONT, NOTICE_BODY_FONT];
    let header_len = heading.iter_mut().zip(header.encode_utf16()).map(|(slot, unit)| *slot = unit).count();
    let header_len = wrap(heading, header_len, MAX_TEXT_W, head.0, 1);
    let len = wrap(body, len, MAX_TEXT_W, text.0, MAX_LINES);
    let (header_w, _) = measure(heading[..header_len].iter().copied(), head.0);
    let (body_w, lines) = measure(body[..len].iter().copied(), text.0);
    let (w, body_h) = (header_w.max(body_w).min(MAX_TEXT_W), lines as f32 * text.1);
    let gap = if lines == 0 { 0.0 } else { LINE_GAP };
    let h = 2.0 * PAD_Y + head.1 + gap + body_h;
    let (x, header_y) = (NOTICE_LEFT + PAD_X + w / 2.0, NOTICE_TOP - PAD_Y - head.1 / 2.0);
    let boxes = NoticeBox {
        plate: [x, NOTICE_TOP - h / 2.0, w + 2.0 * PAD_X, h],
        header: [x, header_y, w, head.1],
        body: [x, header_y - head.1 / 2.0 - gap - body_h / 2.0, w, body_h],
    };
    (boxes, header_len, len)
}

/// The notice container's y offset for a box `h` high: 0 at the anchor, or `GAP` under the BTT compact chip
/// (`panes::btt_hud_box`, `[left, centre y, w, h]`; it starts at the same left edge) while that shows and would meet it.
pub fn stack(h: f32, btt: Option<[f32; 4]>) -> f32 {
    match btt {
        Some([_, y, _, bh]) if NOTICE_TOP - h - GAP < y + bh / 2.0 && y - bh / 2.0 < NOTICE_TOP + GAP => {
            y - bh / 2.0 - GAP - NOTICE_TOP
        }
        _ => 0.0,
    }
}

#[cfg(test)]
#[path = "notice_box_tests.rs"]
mod tests;
