//! Original UI20 dark-panel regression; body colors remain the specified API values.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::panes::{self, Kind};

fn luminance(color: [u8; 4]) -> f64 {
    let c = color.map(|v| {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    });
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn notice_panel_contrasts_with_specified_default_and_phase_colors() {
    let default = model::notify::DEFAULT_COLOR;
    assert_eq!(default, [0, 0, 0, 255], "the specified API color is unchanged");
    assert!(contrast(default, [8, 10, 14, 255]) < 1.1, "UI20 original dark-panel failure");
    let Kind::Rect { color: background } = panes::spec(panes::IDX_NOTICE_BASE).kind else { panic!() };
    let Kind::Text { color: header, .. } = panes::spec(panes::IDX_NOTICE_HEADER).kind else { panic!() };
    assert_eq!(background[3], 255, "contrast must not depend on the scene underneath");
    for color in [default, header, [8, 8, 200, 255], [200, 8, 200, 255], [200, 8, 8, 255]] {
        assert!(contrast(color, background) >= 4.5, "{color:?} on {background:?}");
    }
    assert_eq!(panes::PANEL, [8, 10, 14, 255], "menu palette is independent");
}
