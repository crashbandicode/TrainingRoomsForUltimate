//! The compact notice box (artifacts/phase23/notice-ui-devin/DESIGN.md): fit, padding, wrap, hard bounds, anchor,
//! stacking, and a required notice drawn with Combo Display Off.

lib_unit_tests! {
use super::*;
use crate::model::notify::{Body, Queue, BODY_MAX, DEFAULT_COLOR};
use crate::model::panes::{self as p, cb, iv};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn wrapped(text: &str) -> String {
    let mut out = units(text);
    let len = out.len();
    let len = wrap(&mut out, len, MAX_TEXT_W, NOTICE_BODY_FONT.0, MAX_LINES);
    String::from_utf16(&out[..len]).unwrap()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// The boxes, the wrapped body's line count and the wrapped body, through the HUD writer's own buffer size.
fn boxed(header: &str, body: &str) -> (NoticeBox, usize, Vec<u16>) {
    let mut buffer = [0u16; BODY_MAX];
    let len = crate::model::boot_notices::notice_units(body, &mut buffer);
    let mut heading = [0u16; HEADER_UNITS];
    let (placed, _, len) = layout(header, &mut heading, &mut buffer, len);
    let text = buffer[..len].to_vec();
    (placed, measure(text.iter().copied(), NOTICE_BODY_FONT.0).1, text)
}

/// The hard bounds every notice keeps: the plate inside the left gutter (8 px short of the frame strip at 720p and 1080p,
/// the same canvas), every line within `MAX_TEXT_W`, and above the player HUD even under the BTT chip.
fn assert_bounded(name: &str, b: &NoticeBox, text: &[u16]) {
    assert!(b.plate[0] + b.plate[2] / 2.0 <= -iv::STRIP_W / 2.0 - 12.0 + 1e-3, "{name}: right edge {:?}", b.plate);
    assert!(b.plate[2] <= p::NOTICE_MAX_W + 1e-3, "{name}: width {}", b.plate[2]);
    for line in text.split(|&u| u == u16::from(b'\n')) {
        let (width, _) = measure(line.iter().copied(), NOTICE_BODY_FONT.0);
        assert!(width <= MAX_TEXT_W + 1e-3, "{name}: line {} is {width}", String::from_utf16_lossy(line));
    }
    let (compact, wide) = ("10/10 0:07.12", "3/10   A place  B undo  X start");
    for (body, strip, bar) in [(compact, true, true), (wide, false, false), (wide, true, false), (wide, true, true)] {
        let chip = p::btt_hud_box(body, strip, bar);
        let bottom = b.plate[1] - b.plate[3] / 2.0 + stack(b.plate[3], Some(chip));
        assert!(bottom > -250.0, "{name}: bottom {bottom} reaches the player HUD");
    }
}

/// The notices the operator named, and the longest bodies the plugin posts.
const NOTICES: [(&str, &str); 8] = [
    ("Tech Practice", "Choose a surface · A sets · X starts"),
    ("Tech Practice", "Perfect · 3f · Roll left · 4/20 · streak 2 · Auto approximate"),
    ("Tech Practice", "Done! 20: 12 Perfect · 3 Held · 2 Early · 1 Late · 2 Missed · best 5 · avg 2.1f · fighter best 7"),
    ("Mii Fighter", "CPU fighter applied"),
    ("Fighters", "Loading 7/8"),
    ("Save State", "Saved"),
    ("CPU level", "Lv 7 from TrainingRooms (Training menu: Lv 3)"),
    ("Open Menu", "B + D-pad Up or ZL + R + D-pad Down or L + ZR + Y + D-pad Left"),
];

/// The estimate against the rendered `nintendo64` widths it was calibrated on (ems of the font's x size, Eden stills).
fn notice_box_calibration() {
    for (text, rendered) in [
        ("Tech Practice", 6.40),
        ("CPU level", 4.80),
        ("Choose a surface · A sets · X starts", 17.16),
        ("Lv 7 from TrainingRooms (Training menu: Lv 3)", 22.98),
    ] {
        let (estimate, lines) = measure(units(text), 1.0);
        assert_eq!(lines, 1);
        assert!((estimate / rendered - 1.0).abs() < 0.05, "{text}: {estimate} em against {rendered}");
    }
    assert_eq!(measure(units(""), 17.0), (0.0, 0));
    assert_eq!(advance(0xe0a0), 1.0, "a button glyph counts a full em");
}

/// The plate is the wider text plus the padding on each side, and its lines plus the padding top and bottom; both texts
/// start one padding in from its left edge, and the header one padding under its top.
fn notice_box_fits_text_with_padding() {
    for (header, body) in NOTICES {
        let (b, lines, text) = boxed(header, body);
        let widest = measure(units(header), NOTICE_HEADER_FONT.0).0.max(measure(text.iter().copied(), NOTICE_BODY_FONT.0).0);
        assert_eq!(b.plate[2], widest + 2.0 * PAD_X, "{body}");
        assert_eq!(b.plate[3], 2.0 * PAD_Y + NOTICE_HEADER_FONT.1 + LINE_GAP + lines as f32 * NOTICE_BODY_FONT.1, "{body}");
        let [left, top] = [b.plate[0] - b.plate[2] / 2.0, b.plate[1] + b.plate[3] / 2.0];
        assert!(close(left, p::NOTICE_LEFT) && top == p::NOTICE_TOP, "{body}: the anchor {left} {top}");
        for text in [b.header, b.body] {
            assert!(close(text[0] - text[2] / 2.0, left + PAD_X), "{body}: left-aligned one padding in");
            assert_eq!(text[2], widest);
        }
        assert_eq!(b.header[1] + b.header[3] / 2.0, top - PAD_Y);
        assert_eq!(b.body[1] - b.body[3] / 2.0, top - b.plate[3] + PAD_Y, "{body}: one padding over the bottom");
        assert_eq!(b.body[3], lines as f32 * NOTICE_BODY_FONT.1);
    }
    let (empty, lines, _) = boxed("Fighters", "");
    assert_eq!((lines, empty.plate[3]), (0, 2.0 * PAD_Y + NOTICE_HEADER_FONT.1), "a header alone has no body line");
}

/// The Tech picker: two short lines in a box under a third of the old 992-unit plate's width and about a fifth of its area.
fn notice_box_tech_picker_is_compact() {
    assert_eq!(wrapped(NOTICES[0].1), "Choose a surface · A sets\n· X starts");
    let (b, lines, _) = boxed(NOTICES[0].0, NOTICES[0].1);
    assert_eq!(lines, 2);
    assert!(b.plate[2] < 992.0 / 3.0, "{}", b.plate[2]);
    assert!(b.plate[2] * b.plate[3] < 0.2 * 992.0 * (82.0 + 102.0), "{:?}", b.plate);
    let (short, _, _) = boxed("Mii Fighter", "CPU fighter applied");
    assert!(short.plate[2] < 200.0 && short.plate[3] < 80.0, "{:?}", short.plate);
}

/// Every line fits the box; the plate stays inside the left gutter at every length up to the body limit.
fn notice_box_wraps_within_max_width() {
    for (header, body) in NOTICES {
        let (b, _, text) = boxed(header, body);
        assert_bounded(body, &b, &text);
    }
    let long = "Lv 7 from TrainingRooms · ".repeat(12);
    let (b, _, text) = boxed("Tech Practice", &long[..BODY_MAX]);
    assert_bounded("long", &b, &text);
}

/// A break goes before a `·` separator when the line has one, else at the last space that fits.
fn notice_box_wrap_prefers_separators() {
    assert_eq!(wrapped(NOTICES[1].1), "Perfect · 3f · Roll left · 4/20\n· streak 2 · Auto approximate");
    assert_eq!(wrapped("Same as Mario Mario Mario, nothing new saved"), "Same as Mario Mario Mario,\nnothing new saved");
    assert_eq!(wrapped("Saved"), "Saved", "a short body is not touched");
}

/// Wrapping that needs no hard break only turns spaces into newlines: the text, its length and existing newlines stay.
fn notice_box_wrap_keeps_text_and_length() {
    for (_, body) in NOTICES {
        let before = units(body);
        let after = units(&wrapped(body));
        assert_eq!(after.len(), before.len());
        for (a, b) in after.iter().zip(&before) {
            assert!(a == b || (*b == u16::from(b' ') && *a == u16::from(b'\n')), "{body}");
        }
    }
    assert_eq!(wrapped("one\ntwo"), "one\ntwo");
    let mut empty: [u16; 0] = [];
    assert_eq!(wrap(&mut empty, 0, MAX_TEXT_W, NOTICE_BODY_FONT.0, MAX_LINES), 0);
}

/// Review R1: a word wider than the box (a saved layout's 48-character name) breaks inside the word and keeps every
/// character; Rename, Load and Delete (`runtime/ui/layouts.rs`) all stay inside the gutter.
fn notice_box_long_name_is_hard_broken_within_bounds() {
    let name = "W".repeat(48);
    for body in [format!("Renamed to {name}"), format!("Loaded {name}: close the menu to start"), format!("Deleted {name}")] {
        let (b, lines, text) = boxed("Saved layouts", &body);
        assert_bounded(&body, &b, &text);
        assert!(lines >= 3, "{body}: {lines} lines");
        let kept: String = String::from_utf16(&text).unwrap().chars().filter(|&c| c != '\n' && c != ' ').collect();
        assert_eq!(kept, body.replace(' ', ""), "every character is kept");
    }
    let word = "W".repeat(40);
    assert_eq!(wrapped(&word).replace('\n', ""), word);
    assert!(wrapped(&word).contains('\n'));
}

/// Review R1: no room to insert a break, or more than `MAX_LINES` lines: the text ends in `…`, still within bounds; a
/// header wider than the box ends in `…` on its one line.
fn notice_box_overflow_ends_in_an_ellipsis_within_bounds() {
    let full = "W".repeat(BODY_MAX);
    let (b, lines, text) = boxed("Saved layouts", &full);
    assert_bounded("320 W", &b, &text);
    assert!(lines <= MAX_LINES && text.last() == Some(&0x2026), "{lines} lines");
    let glyphs: String = core::iter::repeat_n('\u{e0a0}', BODY_MAX / 3).collect();
    let (b, lines, text) = boxed("Open Menu", &glyphs);
    assert_bounded("button glyphs", &b, &text);
    assert!(lines <= MAX_LINES);
    let words = format!("{} ", "W".repeat(16)).repeat(BODY_MAX / 17);
    let (b, lines, text) = boxed("Saved layouts", &words);
    assert_bounded("a line per word", &b, &text);
    assert!(lines == MAX_LINES && text.last() == Some(&0x2026), "{lines} lines");
    let mut heading = [0u16; HEADER_UNITS];
    let mut body = [0u16; 4];
    let (b, header_len, _) = layout(&"M".repeat(24), &mut heading, &mut body, 0);
    assert!(b.plate[2] <= p::NOTICE_MAX_W + 1e-3 && heading[header_len - 1] == 0x2026, "{:?}", b.plate);
}

/// Review R3 (CRASH-0900): a required notice survives Combo Display Off and the compact writer's path draws it bounded.
fn notice_box_required_notice_draws_with_combo_display_off() {
    let mut queue = Queue::new();
    queue.post_static("Save State", "Saved", 120, DEFAULT_COLOR);
    let (header, body) = (crate::model::lra_reset::DEFERRED_HEADER, crate::model::lra_reset::DEFERRED_BODY);
    queue.post_required(header, Body::copied(body), 120, DEFAULT_COLOR);
    let out = queue.tick(false, false);
    let draw = out.draw.expect("the required notice draws with Combo Display Off");
    assert!(out.visible && draw.header == header && draw.body.as_str() == body);
    let (b, _, text) = boxed(draw.header, draw.body.as_str());
    assert_bounded(body, &b, &text);
    assert_eq!(String::from_utf16(&text).unwrap().replace('\n', " "), body);
}

/// Spot 1: the plate's corner is the BTT chip's (16 px, 8 px), left of the frame strip and of the combo and sequence bars;
/// Training's damage panels are on the right.
fn notice_box_anchors_top_left_clear_of_hud() {
    assert_eq!([p::NOTICE_LEFT, p::NOTICE_TOP], [-960.0 + 24.0, 540.0 - 12.0]);
    assert_eq!(p::btt_hud_box("10/10 0:07.12", false, false)[0], p::NOTICE_LEFT, "the chip's left edge");
    const RIGHT: f32 = p::NOTICE_LEFT + p::NOTICE_MAX_W;
    const { assert!(RIGHT <= -iv::STRIP_W / 2.0 - 12.0, "8 px left of the frame strip") };
    const { assert!(RIGHT <= -cb::BAR_W / 2.0, "left of the combo bar") };
    const { assert!(RIGHT <= -cb::SEQ_W / 2.0, "left of the sequence bar") };
    const { assert!(RIGHT < 0.0, "Total Damage, Combo and Damage are on the right") };
}

/// The BTT chip shows: the box goes 8 px under it when they would meet, and stays at the anchor when they would not.
fn notice_box_stacks_below_btt_chip() {
    let (b, _, _) = boxed("Tech Practice", NOTICES[0].1);
    let h = b.plate[3];
    assert_eq!(stack(h, None), 0.0);
    for (strip, bar) in [(false, false), (true, false), (true, true)] {
        let chip = p::btt_hud_box("10/10 0:07.12", strip, bar);
        let top = p::NOTICE_TOP + stack(h, Some(chip));
        assert_eq!(top, chip[1] - chip[3] / 2.0 - GAP, "under the compact chip");
        let placing = p::btt_hud_box("3/10   A place  B undo  X start", strip, bar);
        let at = stack(h, Some(placing));
        if strip {
            assert_eq!(at, 0.0, "the wide chip under the strip leaves the corner free");
        } else {
            assert_eq!(p::NOTICE_TOP + at, placing[1] - placing[3] / 2.0 - GAP);
        }
    }
}

/// Review R2: a dialog's background is sized to its crumb, content, help and hint (inside each), much smaller than the
/// 1840 × 1000 page panel; a page keeps the panel.
fn dialog_box_fits_its_content() {
    use crate::model::frame::{Frame, View};
    use core::fmt::Write;
    let inside = |frame: &Frame, right: f32, bottom: f32| {
        let d = frame.dialog_box().expect("a dialog");
        let [x, y, w, h] = d.bg;
        assert!(x - w / 2.0 <= -880.0 && x + w / 2.0 >= right && y + h / 2.0 >= p::CRUMB_Y + 28.0, "{:?}", d);
        assert!(y - h / 2.0 <= d.hint_y - 22.0 && d.hint_y < bottom && d.help_y[0] < bottom, "{:?}", d);
        assert!(w < 0.7 * p::PANEL_W && h < 0.5 * p::PANEL_H, "{:?} is not compact", d);
    };
    let mut delete = Frame::new();
    (delete.view, delete.dialog, delete.tile_cols, delete.tile_rows, delete.tile_count) = (View::Tiles, true, 4, 8, 2);
    for (tile, label) in delete.tiles.iter_mut().zip(["Keep it", "Delete it"]) {
        let _ = tile.write_str(label);
    }
    let _ = delete.crumb.write_str("Fighters & Stages  >  Delete Projectile probe?");
    let _ = delete.help[0].write_str("The layout file is removed from the card.");
    let _ = delete.hint.write_str("D-pad move   A confirm   Y other action   B back");
    let cell_w = (p::ROW_RIGHT - p::ROW_LEFT) / 4.0;
    inside(&delete, p::ROW_LEFT + 2.0 * cell_w, 320.0 - p::TILE_AREA_H / 8.0);
    let mut reset = Frame::new();
    (reset.dialog, reset.row_count) = (true, 3);
    for (row, label) in reset.rows.iter_mut().zip(["Reset this page to your saved defaults?", "A  reset", "B  cancel"]) {
        let _ = row.label.write_str(label);
    }
    let _ = reset.help[0].write_str("Every setting on CPU goes back to your saved defaults.");
    let _ = reset.help[1].write_str("Other pages are not changed.");
    let _ = reset.hint.write_str("A reset   B cancel");
    inside(&reset, p::row_label_box(false).0, p::row_y(2) - p::ROW_H / 2.0);
    let mut popup = Frame::new();
    (popup.popup, popup.row_count, popup.conn_count) = (true, 4, 3);
    let d = popup.dialog_box().expect("the recording pop-up");
    assert!(d.bg[0] + d.bg[2] / 2.0 >= p::CONN_RIGHT && d.bg[3] < p::PANEL_H, "{d:?}");
    assert_eq!(Frame::new().dialog_box(), None, "a page keeps the full panel");
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("notice_box_calibration", notice_box_calibration as fn()),
        ("notice_box_fits_text_with_padding", notice_box_fits_text_with_padding as fn()),
        ("notice_box_tech_picker_is_compact", notice_box_tech_picker_is_compact as fn()),
        ("notice_box_wraps_within_max_width", notice_box_wraps_within_max_width as fn()),
        ("notice_box_wrap_prefers_separators", notice_box_wrap_prefers_separators as fn()),
        ("notice_box_wrap_keeps_text_and_length", notice_box_wrap_keeps_text_and_length as fn()),
        ("notice_box_long_name_is_hard_broken_within_bounds", notice_box_long_name_is_hard_broken_within_bounds as fn()),
        ("notice_box_overflow_ends_in_an_ellipsis_within_bounds", notice_box_overflow_ends_in_an_ellipsis_within_bounds as fn()),
        ("notice_box_required_notice_draws_with_combo_display_off", notice_box_required_notice_draws_with_combo_display_off as fn()),
        ("notice_box_anchors_top_left_clear_of_hud", notice_box_anchors_top_left_clear_of_hud as fn()),
        ("notice_box_stacks_below_btt_chip", notice_box_stacks_below_btt_chip as fn()),
        ("dialog_box_fits_its_content", dialog_box_fits_its_content as fn()),
    ]);
}
}
