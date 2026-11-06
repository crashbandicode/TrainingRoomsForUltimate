//! UI20 put notice text on a dark panel and black body text failed (~1.06:1). The white plate
//! that followed disappeared on a bright stage. Body text is now the light colour that clears
//! 4.5:1 on the dark plate. Black on that plate remains the original failure.
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
    assert_eq!(default, panes::TEXT, "the default body colour is the light notice text");
    assert!(contrast([0, 0, 0, 255], panes::PANEL) < 1.1, "UI20 original dark-panel failure");
    assert!(contrast(default, panes::NOTICE_PLATE) >= 4.5);
    let Kind::Rect { color: background } = panes::spec(panes::IDX_NOTICE_HEADER_PLATE).kind else { panic!() };
    assert_eq!(background, panes::NOTICE_PLATE);
    assert_eq!(background[3], 255, "contrast must not depend on the scene underneath");
    assert_eq!(panes::PANEL, [8, 10, 14, 255], "menu palette is independent");
}
