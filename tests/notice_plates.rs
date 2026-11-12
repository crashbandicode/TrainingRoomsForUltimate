//! Dark plates behind the plugin's notice texts. The white plate was the light background
//! `0c60459` refused to call a pass.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::panes::{self, Kind, Name};
use model::recording::{Phase, LOCKOUT_COLOR, RECORDING_COLOR, STANDBY_COLOR};

fn channel(v: u8) -> f64 {
    let s = f64::from(v) / 255.0;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}

fn luminance(color: [u8; 4]) -> f64 {
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn over(front: [u8; 4], back: [u8; 4]) -> [u8; 4] {
    let a = f64::from(front[3]) / 255.0;
    let mix = |f: u8, b: u8| (a * f64::from(f) + (1.0 - a) * f64::from(b)).round() as u8;
    [mix(front[0], back[0]), mix(front[1], back[1]), mix(front[2], back[2]), 255]
}

fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Text over the plate, then that result over the stage. An opaque plate hides the stage.
fn composite_contrast(text: [u8; 4], plate: [u8; 4], stage: [u8; 4]) -> f64 {
    let ground = over(plate, stage);
    contrast(over(text, ground), ground)
}

fn text_color(index: usize) -> [u8; 4] {
    let Kind::Text { color, .. } = panes::spec(index).kind else { panic!("text pane") };
    color
}

fn plate_color(index: usize) -> [u8; 4] {
    let Kind::Rect { color } = panes::spec(index).kind else { panic!("plate pane") };
    color
}

const WHITE: [u8; 4] = [255, 255, 255, 255];

const PAIRS: [(usize, usize); 3] = [
    (panes::IDX_NOTICE_HEADER_PLATE, panes::IDX_NOTICE_HEADER),
    (panes::IDX_NOTICE_TEXT_PLATE, panes::IDX_NOTICE_TEXT),
    (panes::IDX_NOTICE_LONG_PLATE, panes::IDX_NOTICE_LONG),
];

fn notice_text_colors() -> Vec<[u8; 4]> {
    let mut colors = vec![
        text_color(panes::IDX_NOTICE_HEADER),
        text_color(panes::IDX_NOTICE_TEXT),
        text_color(panes::IDX_NOTICE_LONG),
        model::notify::DEFAULT_COLOR,
        LOCKOUT_COLOR,
        STANDBY_COLOR,
        RECORDING_COLOR,
    ];
    for phase in [Phase::Lockout, Phase::Standby, Phase::Recording, Phase::Playback] {
        colors.push(phase.notice().expect("phase notice").1);
    }
    colors
}

fn every_notice_text_has_a_plate() {
    let specs = panes::specs();
    let parent = panes::name_of(panes::IDX_NOTICE);
    for (plate_i, text_i) in PAIRS {
        let plate = panes::spec(plate_i);
        let text = panes::spec(text_i);
        assert_eq!(plate.parent, parent);
        assert_eq!(text.parent, parent);
        let plate_at = specs.iter().position(|s| s.name == plate.name).unwrap();
        // Every plate is drawn before every notice text, not only before its own (no_shown_plate_is_drawn_over_…).
        for (_, other_text) in PAIRS {
            let other_at = specs.iter().position(|s| s.name == panes::spec(other_text).name).unwrap();
            assert!(plate_at < other_at, "{:?} is drawn after the text {:?}", plate.name, panes::name_of(other_text));
        }
        assert!(matches!(plate.kind, Kind::Rect { .. }));
        assert!(matches!(text.kind, Kind::Text { .. }));
        assert_eq!(plate.x, text.x);
        assert_eq!(plate.y, text.y);
        assert!((plate.w - text.w - 2.0 * panes::NOTICE_PLATE_PAD).abs() < f32::EPSILON);
        assert!((plate.h - text.h - 2.0 * panes::NOTICE_PLATE_PAD).abs() < f32::EPSILON);
        assert_eq!(plate_color(plate_i), panes::NOTICE_PLATE);
    }
}

/// `[left, bottom, right, top]` of a pane: `x`, `y` are its centre, `w`, `h` its size.
fn bounds(index: usize) -> [f32; 4] {
    let s = panes::spec(index);
    [s.x - s.w / 2.0, s.y - s.h / 2.0, s.x + s.w / 2.0, s.y + s.h / 2.0]
}

fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// A plate drawn after a notice text that is on screen with it covers that text: Eden covrest-02 (2026-11-08) showed
/// the body's plate over the lower half of the "Save State" header. For each notice shape the container can show
/// (`notice_faces`), no visible plate may come after a visible text it overlaps.
fn no_shown_plate_is_drawn_over_a_shown_notice_text() {
    let specs = panes::specs();
    let at = |index: usize| specs.iter().position(|s| s.name == panes::spec(index).name).unwrap();
    for long_body in [false, true] {
        let face = panes::notice_faces(true, long_body);
        let shown = [face.header, face.body, face.long_body];
        for (text_slot, (_, text_i)) in PAIRS.iter().enumerate() {
            for (plate_slot, (plate_i, _)) in PAIRS.iter().enumerate() {
                if !shown[text_slot] || !shown[plate_slot] || plate_slot == text_slot {
                    continue;
                }
                assert!(
                    !(at(*plate_i) > at(*text_i) && overlap(bounds(*plate_i), bounds(*text_i))),
                    "{:?} is drawn after {:?} and covers it (long_body={long_body})",
                    panes::name_of(*plate_i),
                    panes::name_of(*text_i)
                );
            }
        }
    }
}

fn plate_contrast_is_readable_on_white() {
    let plate = panes::NOTICE_PLATE;
    for color in notice_text_colors() {
        let ratio = composite_contrast(color, plate, WHITE);
        assert!(ratio >= 4.5, "{color:?} over {plate:?} over white is {ratio}");
    }
}

fn plate_visible_exactly_when_text_visible() {
    for shown in [false, true] {
        for long_body in [false, true] {
            let face = panes::notice_faces(shown, long_body);
            assert_eq!(face.header, shown);
            assert_eq!(face.body, shown && !long_body);
            assert_eq!(face.long_body, shown && long_body);
            assert!(!(face.body && face.long_body), "one body is shown");
        }
    }
    for (plate_i, text_i) in PAIRS {
        assert_eq!(
            panes::spec(plate_i).visible,
            panes::spec(text_i).visible,
            "baked plate visibility matches its text"
        );
    }
}

fn the_strip_gets_no_second_plate() {
    let specs = panes::specs();
    let strip = panes::name_of(panes::IDX_IV);
    let bar = panes::name_of(panes::IDX_CB);
    let notice = panes::name_of(panes::IDX_NOTICE);
    for (plate_i, _) in PAIRS {
        assert_eq!(panes::spec(plate_i).parent, notice);
    }
    for (i, spec) in specs.iter().enumerate() {
        if !matches!(spec.kind, Kind::Text { .. }) {
            continue;
        }
        if spec.parent != strip && spec.parent != bar {
            continue;
        }
        let Some(prev) = specs.get(i.wrapping_sub(1)) else { continue };
        if prev.parent != spec.parent {
            continue;
        }
        if let Kind::Rect { .. } = prev.kind {
            let sized_to_text = (prev.w - spec.w - 2.0 * panes::NOTICE_PLATE_PAD).abs() < 0.1
                && (prev.h - spec.h - 2.0 * panes::NOTICE_PLATE_PAD).abs() < 0.1
                && prev.x == spec.x
                && prev.y == spec.y;
            assert!(!sized_to_text, "{:?} has a notice plate in front of the strip backdrop", spec.name);
        }
    }
    let plate_names = [
        Name::new("TrModDisp0HdrPlate"),
        Name::new("TrModDisp0TxtPlate"),
        Name::new("TrModDisp0LongPlate"),
    ];
    assert!(specs.iter().filter(|s| plate_names.contains(&s.name)).count() == 3);
}

fn both_layouts_carry_the_plates() {
    let names = ["TrModDisp0HdrPlate", "TrModDisp0TxtPlate", "TrModDisp0LongPlate"];
    for (plate_i, _) in PAIRS {
        assert_eq!(panes::spec(plate_i).parent, panes::name_of(panes::IDX_NOTICE));
        assert_ne!(panes::spec(plate_i).parent, Name::new("RootPane"));
    }
    for recipe in ["assets/info_training.tra1", "assets/info_waiting.tra1"] {
        let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), recipe);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        for name in names {
            assert!(
                bytes.windows(name.len()).any(|w| w == name.as_bytes()),
                "{recipe} is missing {name}"
            );
        }
    }
}

fn known_contrast_limitation_is_not_relabelled_a_pass() {
    for (plate_i, text_i) in PAIRS {
        let plate = plate_color(plate_i);
        assert_eq!(plate[3], 255, "the stage must not show through");
        let lum = luminance(plate);
        assert!(
            lum < 0.05,
            "plate {plate:?} luminance {lum} is the white light background 0c60459 refused to call a pass"
        );
        let ratio = composite_contrast(text_color(text_i), plate, WHITE);
        assert!(ratio >= 4.5, "{:?} over its plate over white is {ratio}", panes::name_of(text_i));
    }
    for color in notice_text_colors() {
        let ratio = composite_contrast(color, panes::NOTICE_PLATE, WHITE);
        assert!(ratio >= 4.5, "{color:?} is the unproven light-background claim");
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("every_notice_text_has_a_plate", every_notice_text_has_a_plate as fn()),
        ("no_shown_plate_is_drawn_over_a_shown_notice_text", no_shown_plate_is_drawn_over_a_shown_notice_text as fn()),
        ("plate_contrast_is_readable_on_white", plate_contrast_is_readable_on_white as fn()),
        ("plate_visible_exactly_when_text_visible", plate_visible_exactly_when_text_visible as fn()),
        ("the_strip_gets_no_second_plate", the_strip_gets_no_second_plate as fn()),
        ("both_layouts_carry_the_plates", both_layouts_carry_the_plates as fn()),
        ("known_contrast_limitation_is_not_relabelled_a_pass", known_contrast_limitation_is_not_relabelled_a_pass as fn()),
    ]);
}
