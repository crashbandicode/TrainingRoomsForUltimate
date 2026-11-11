//! The replay window's clip editor as a frame (MENU-DRAFT Recording > Named macros > Replay window; the layer is package 4's
//! `ClipEditor`). The editor owns Player 1's controls while it is up, so the menu is closed and this frame is drawn alone: the title
//! and breadcrumb, a few lines (fighter, window, playhead, source), the layer's own help texts, and the timeline the view draws
//! from `Frame::clip`.

use core::fmt::Write;

use crate::engine::macro_ui::clip_panel::{clip_help, ClipView, Handle};
use crate::model::frame::{ClipBar, Frame, Tone, View, TITLE};

/// Frames per second of the game: the length of a window in seconds.
const FPS: f32 = 60.0;

/// The modifier the editor reads for fast scrubbing and for the lead-in chord (`replay_capture::on_input`: ZR on a GameCube
/// controller, ZL or ZR on the others).
const COARSE: &str = "ZL or ZR";
const CHORD: &str = "ZL";

fn handle_code(handle: Handle) -> u8 {
    match handle {
        Handle::Playhead => 0,
        Handle::In => 1,
        Handle::Out => 2,
    }
}

/// Build the clip editor's frame. `fighter` is the label of the track's fighter, `pace` the frames per tick of the held scrub
/// (`replay_capture::scrub_pace`).
pub fn clip_frame(view: &ClipView, fighter: &str, pace: i32, frame: &mut Frame) {
    frame.reset();
    let _ = frame.title.write_str(TITLE);
    let _ = write!(frame.crumb, "Replay window  >  {}", view.source_name);
    frame.view = View::Clip;
    frame.clip = ClipBar {
        total: view.total,
        start: view.start,
        end: view.end,
        cursor: view.cursor,
        handle: handle_code(view.handle),
        playing: view.playing,
    };
    let length = view.end.saturating_sub(view.start);
    let mut line = |label: &str, value: &str, tone: Tone| {
        let at = usize::from(frame.row_count);
        if at < frame.rows.len() {
            let row = &mut frame.rows[at];
            let _ = row.label.write_str(label);
            let _ = row.value.write_str(value);
            row.value_tone = tone;
            frame.row_count += 1;
        }
    };
    line("Fighter", fighter, Tone::Normal);
    let mut window = String::new();
    let _ = write!(window, "{} - {}  ({} frames, {:.2}s of {})", view.start, view.end, length, length as f32 / FPS, view.total);
    line("Crop window", &window, Tone::Normal);
    let mut head = String::new();
    let _ = write!(head, "{}{}", view.cursor, if view.playing { "  playing" } else { "" });
    line("Playhead", &head, if view.playing { Tone::On } else { Tone::Normal });
    if view.truncated {
        line("This capture is partial", "", Tone::Dim);
    }
    if view.quantized {
        line("Frames were quantized by the capture", "", Tone::Dim);
    }
    if view.previous_available {
        line("Capture", if view.previous_selected { "previous" } else { "latest" }, Tone::Dim);
    }
    let help = clip_help(view, COARSE, CHORD, pace);
    if !help.notice.is_empty() {
        line(&help.notice, "", Tone::Dim);
    }
    let _ = frame.help[0].write_str(&help.primary);
    let _ = frame.help[1].write_str(&help.follow);
    let _ = frame.hint.write_str(&help.controls);
}

