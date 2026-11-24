//! The top-left compact HUD row (`TrBtt`: plate, picture chip, one text): Break the Targets' line and, while a take is
//! open, the recording indicator (fix round 2026-11-23, F3). Pure; `runtime::features::hud_draw` writes what this
//! returns.

use core::fmt::Write;

use super::panes;
use super::storage::FixedPath;

/// The fork's role colours (`src/training/ui/roster_grid.rs:9-10`): the roster grid's Player and CPU badges, stripes,
/// Solo Popo markers and cursor tint, and the recording indicator.
pub const PLAYER_COLOR: [u8; 4] = [255, 83, 92, 255];
/// The CPU's roster and indicator colour.
pub const CPU_COLOR: [u8; 4] = [75, 164, 255, 255];
/// The indicator alone: the chip, `REC CPU` and the plate's padding.
pub const REC_W: f32 = 140.0;
/// The row's text pane holds 96 units (`panes::spec(IDX_BTT + 3)`).
pub const TEXT_UNITS: usize = 96;

/// Whose take is open (`rr_host::capture_take`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// Player 1's take.
    Player,
    /// The CPU's take.
    Cpu,
}

impl Take {
    /// The bridge's word, the one `recording.capture.role` uses.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Cpu => "cpu",
        }
    }

    /// The take's label in the compact row.
    fn label(self) -> &'static str {
        match self {
            Self::Player => "REC P1",
            Self::Cpu => "REC CPU",
        }
    }

    /// The take's shared role colour.
    fn color(self) -> [u8; 4] {
        match self {
            Self::Player => PLAYER_COLOR,
            Self::Cpu => CPU_COLOR,
        }
    }
}

/// What the row shows.
pub struct HudRow {
    /// The row's text.
    pub text: FixedPath<{ TEXT_UNITS * 4 }>,
    /// The text's colour.
    pub color: [u8; 4],
    /// The chip's vertex colours: white for Break the Targets' own picture, the role's colour for a take.
    pub chip: [u8; 4],
    /// The take shown, if any.
    pub take: Option<Take>,
    pub playing: [bool; 2],
    /// Break the Targets' line is part of the text.
    pub line: bool,
}

/// `take` is the open take; `line` is Break the Targets' HUD line ("" for none). `None` hides the row.
pub fn hud_row(take: Option<Take>, playing: [bool; 2], line: &str) -> Option<HudRow> {
    if take.is_none() && !playing[0] && !playing[1] && line.is_empty() {
        return None;
    }
    let mut text = FixedPath::new();
    if let Some(take) = take {
        let _ = text.write_str(take.label());
        if playing[0] || playing[1] || !line.is_empty() {
            let _ = text.write_str(" · ");
        }
    }
    let play = match playing {
        [true, false] => "PLAY P1",
        [false, true] => "PLAY CPU",
        [true, true] => "PLAY P1 + CPU",
        [false, false] => "",
    };
    if !play.is_empty() {
        let _ = text.write_str(play);
        if !line.is_empty() {
            let _ = text.write_str(" · ");
        }
    }
    let remaining = if text.as_str().is_empty() { usize::MAX } else { TEXT_UNITS.saturating_sub(text.as_str().chars().count()) };
    for ch in line.chars().take(remaining) {
        if text.write_char(ch).is_err() {
            break;
        }
    }
    let role_color = take.map(Take::color).or_else(|| match playing {
        [true, false] => Some(PLAYER_COLOR),
        [_, true] => Some(CPU_COLOR),
        _ => None,
    });
    Some(HudRow { text, color: role_color.unwrap_or(panes::TEXT), chip: role_color.unwrap_or([255; 4]), take, playing, line: !line.is_empty() })
}

/// `[left, centre y, w, h]`: the indicator alone is `REC_W` wide at the row's corner; anything with the line is
/// `panes::btt_hud_box` of the text.
pub fn hud_row_box(row: &HudRow, strip: bool, bar: bool) -> [f32; 4] {
    if !row.line && row.take.is_some() && row.playing == [false; 2] {
        let [x, y, _, h] = panes::btt_hud_box("", false, false);
        [x, y, REC_W, h]
    } else if !row.line && (row.playing[0] || row.playing[1]) {
        let [x, y, _, h] = panes::btt_hud_box("", false, false);
        let panes::Kind::Text { font, .. } = panes::spec(panes::IDX_BTT + 3).kind else {
            return [x, y, REC_W, h];
        };
        let width = crate::model::notice_box::measure(row.text.as_str().encode_utf16(), font.0).0 / 0.9 + 65.0;
        [x, y, width.max(REC_W), h]
    } else {
        panes::btt_hud_box(row.text.as_str(), strip, bar)
    }
}
