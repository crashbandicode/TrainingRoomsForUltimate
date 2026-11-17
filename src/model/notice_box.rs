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
const NEWLINE: u16 = b'\n' as u16;
const SPACE: u16 = b' ' as u16;
const SEPARATOR: u16 = 0xb7;

/// One UTF-16 unit's advance in ems of the font's x size. The classes are calibrated on four `nintendo64` strings
/// measured on Eden stills (design report: within -1.7 % to +4.2 %); any other non-ASCII unit, such as a button
/// glyph, counts a full em.
pub fn advance(unit: u16) -> f32 {
    match char::from_u32(u32::from(unit)).unwrap_or('\u{fffd}') {
        ' ' => 0.32,
        'i' | 'l' | 'j' | '.' | ',' | ':' | ';' | '\'' | '!' | '|' | '·' => 0.30,
        'f' | 'r' | 't' | 'I' | '(' | ')' | '[' | ']' | '/' | '-' => 0.38,
        'm' | 'w' | 'M' | 'W' => 0.82,
        'A'..='Z' => 0.72,
        '0'..='9' => 0.58,
        c if c.is_ascii() => 0.55,
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

/// Wraps `units` in place at `max`: a line that would pass it breaks at its last space before a `·` separator, else at
/// its last space. Only spaces become newlines, so the text and its length stay; a word wider than `max` stays whole.
pub fn wrap(units: &mut [u16], max: f32, font: f32) {
    let (mut start, mut width, mut i) = (0, 0.0, 0);
    while i < units.len() {
        if units[i] == NEWLINE {
            (start, width) = (i + 1, 0.0);
        } else {
            width += advance(units[i]) * font;
            let space = |sep: bool| (start + 1..i).rev().find(|&k| units[k] == SPACE && (!sep || units[k + 1] == SEPARATOR));
            if let Some(at) = (width > max).then(|| space(true).or_else(|| space(false))).flatten() {
                units[at] = NEWLINE;
                i = at;
                continue;
            }
        }
        i += 1;
    }
}

/// The panes of one notice at the anchor (the plate's top-left corner at `NOTICE_LEFT`, `NOTICE_TOP`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoticeBox {
    pub plate: [f32; 4],
    pub header: [f32; 4],
    pub body: [f32; 4],
}

/// `body` (the UTF-16 notice body) is wrapped in place at `MAX_TEXT_W`; the plate fits the wider of the two texts.
pub fn layout(header: &str, body: &mut [u16]) -> NoticeBox {
    let [head, text] = [NOTICE_HEADER_FONT, NOTICE_BODY_FONT];
    wrap(body, MAX_TEXT_W, text.0);
    let (header_w, _) = measure(header.encode_utf16(), head.0);
    let (body_w, lines) = measure(body.iter().copied(), text.0);
    let (w, body_h) = (header_w.max(body_w), lines as f32 * text.1);
    let gap = if lines == 0 { 0.0 } else { LINE_GAP };
    let h = 2.0 * PAD_Y + head.1 + gap + body_h;
    let (x, header_y) = (NOTICE_LEFT + PAD_X + w / 2.0, NOTICE_TOP - PAD_Y - head.1 / 2.0);
    NoticeBox {
        plate: [x, NOTICE_TOP - h / 2.0, w + 2.0 * PAD_X, h],
        header: [x, header_y, w, head.1],
        body: [x, header_y - head.1 / 2.0 - gap - body_h / 2.0, w, body_h],
    }
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
