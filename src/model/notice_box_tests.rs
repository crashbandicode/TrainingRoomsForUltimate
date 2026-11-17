//! The compact notice box (artifacts/phase23/notice-ui-devin/DESIGN.md): fit, padding, wrap, anchor and stacking.

lib_unit_tests! {
use super::*;
use crate::model::panes::{self as p, cb, iv};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn wrapped(text: &str) -> String {
    let mut out = units(text);
    wrap(&mut out, MAX_TEXT_W, NOTICE_BODY_FONT.0);
    String::from_utf16(&out).unwrap()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn boxed(header: &str, body: &str) -> (NoticeBox, usize) {
    let mut out = units(body);
    let placed = layout(header, &mut out);
    (placed, measure(out, NOTICE_BODY_FONT.0).1)
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
        let (b, lines) = boxed(header, body);
        let mut text = units(body);
        wrap(&mut text, MAX_TEXT_W, NOTICE_BODY_FONT.0);
        let widest = measure(units(header), NOTICE_HEADER_FONT.0).0.max(measure(text, NOTICE_BODY_FONT.0).0);
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
    let (empty, lines) = boxed("Fighters", "");
    assert_eq!((lines, empty.plate[3]), (0, 2.0 * PAD_Y + NOTICE_HEADER_FONT.1), "a header alone has no body line");
}

/// The Tech picker: two short lines in a box under a third of the old 992-unit plate's width and about a fifth of its area.
fn notice_box_tech_picker_is_compact() {
    assert_eq!(wrapped(NOTICES[0].1), "Choose a surface · A sets\n· X starts");
    let (b, lines) = boxed(NOTICES[0].0, NOTICES[0].1);
    assert_eq!(lines, 2);
    assert!(b.plate[2] < 992.0 / 3.0, "{}", b.plate[2]);
    assert!(b.plate[2] * b.plate[3] < 0.2 * 992.0 * (82.0 + 102.0), "{:?}", b.plate);
    let (short, _) = boxed("Mii Fighter", "CPU fighter applied");
    assert!(short.plate[2] < 200.0 && short.plate[3] < 80.0, "{:?}", short.plate);
}

/// Every line fits the box; the plate stays inside the left gutter at every length up to the body limit.
fn notice_box_wraps_within_max_width() {
    let check = |body: &str| {
        let mut text = units(body);
        wrap(&mut text, MAX_TEXT_W, NOTICE_BODY_FONT.0);
        for line in text.split(|&u| u == u16::from(b'\n')) {
            assert!(measure(line.iter().copied(), NOTICE_BODY_FONT.0).0 <= MAX_TEXT_W, "{}", String::from_utf16_lossy(line));
        }
        let (b, _) = boxed("Tech Practice", body);
        assert!(b.plate[0] + b.plate[2] / 2.0 <= -iv::STRIP_W / 2.0 - 12.0, "{body}: 8 px left of the frame strip");
    };
    for (_, body) in NOTICES {
        check(body);
    }
    let long = "Lv 7 from TrainingRooms · ".repeat(12);
    check(&long[..crate::model::notify::BODY_MAX]);
}

/// A break goes before a `·` separator when the line has one, else at the last space that fits.
fn notice_box_wrap_prefers_separators() {
    assert_eq!(wrapped(NOTICES[1].1), "Perfect · 3f · Roll left · 4/20\n· streak 2 · Auto approximate");
    assert_eq!(wrapped("Same as Mario Mario Mario, nothing new saved"), "Same as Mario Mario Mario,\nnothing new saved");
    assert_eq!(wrapped("Saved"), "Saved", "a short body is not touched");
}

/// Wrapping only turns spaces into newlines: the text, its length and existing newlines stay; a word too wide stays whole.
fn notice_box_wrap_keeps_text_and_length() {
    for (_, body) in NOTICES {
        let before = units(body);
        let mut after = before.clone();
        wrap(&mut after, MAX_TEXT_W, NOTICE_BODY_FONT.0);
        assert_eq!(after.len(), before.len());
        for (a, b) in after.iter().zip(&before) {
            assert!(a == b || (*b == u16::from(b' ') && *a == u16::from(b'\n')), "{body}");
        }
    }
    let word = "W".repeat(40);
    assert_eq!(wrapped(&word), word);
    assert_eq!(wrapped(&format!("{word} a")), format!("{word}\na"));
    assert_eq!(wrapped("one\ntwo"), "one\ntwo");
    let mut empty: [u16; 0] = [];
    wrap(&mut empty, MAX_TEXT_W, NOTICE_BODY_FONT.0);
}

/// Spot 1: the plate's corner is the BTT chip's (16 px, 8 px), left of the frame strip and of the combo and sequence bars,
/// and even the tallest notice under the chip ends above the player HUD; Training's damage panels are on the right.
fn notice_box_anchors_top_left_clear_of_hud() {
    assert_eq!([p::NOTICE_LEFT, p::NOTICE_TOP], [-960.0 + 24.0, 540.0 - 12.0]);
    assert_eq!(p::btt_hud_box("10/10 0:07.12", false, false)[0], p::NOTICE_LEFT, "the chip's left edge");
    const RIGHT: f32 = p::NOTICE_LEFT + p::NOTICE_MAX_W;
    const { assert!(RIGHT <= -iv::STRIP_W / 2.0 - 12.0, "8 px left of the frame strip") };
    const { assert!(RIGHT <= -cb::BAR_W / 2.0, "left of the combo bar") };
    const { assert!(RIGHT <= -cb::SEQ_W / 2.0, "left of the sequence bar") };
    const { assert!(RIGHT < 0.0, "Total Damage, Combo and Damage are on the right") };
    let tallest = "a ".repeat(crate::model::notify::BODY_MAX / 2);
    let (b, _) = boxed("Tech Practice", &tallest);
    let chip = p::btt_hud_box("10/10 0:07.12", true, true);
    let bottom = b.plate[1] - b.plate[3] / 2.0 + stack(b.plate[3], Some(chip));
    assert!(bottom > -250.0, "{bottom}: above the player HUD");
}

/// The BTT chip shows: the box goes 8 px under it when they would meet, and stays at the anchor when they would not.
fn notice_box_stacks_below_btt_chip() {
    let (b, _) = boxed("Tech Practice", NOTICES[0].1);
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

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("notice_box_calibration", notice_box_calibration as fn()),
        ("notice_box_fits_text_with_padding", notice_box_fits_text_with_padding as fn()),
        ("notice_box_tech_picker_is_compact", notice_box_tech_picker_is_compact as fn()),
        ("notice_box_wraps_within_max_width", notice_box_wraps_within_max_width as fn()),
        ("notice_box_wrap_prefers_separators", notice_box_wrap_prefers_separators as fn()),
        ("notice_box_wrap_keeps_text_and_length", notice_box_wrap_keeps_text_and_length as fn()),
        ("notice_box_anchors_top_left_clear_of_hud", notice_box_anchors_top_left_clear_of_hud as fn()),
        ("notice_box_stacks_below_btt_chip", notice_box_stacks_below_btt_chip as fn()),
    ]);
}
}
