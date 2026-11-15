//! The replay window's clip editor as a frame (MENU-DRAFT Recording > Named macros > Replay window; the layer is package 4's
//! `ClipEditor`). The editor owns Player 1's controls while it is up, so the menu is closed and this frame is drawn alone: the title
//! and breadcrumb, a few lines (fighter, window, playhead, source), the layer's own help texts, and the timeline the view draws
//! from `Frame::clip`.

use core::fmt::Write;

use crate::engine::macro_ui::clip_panel::{clip_help, ClipContext, ClipView, Handle};
use crate::model::frame::{ClipBar, Frame, Tone, View, TITLE};
use crate::model::screens::push;

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
    let _ = write!(frame.crumb, "{}  >  {}", if matches!(view.context, ClipContext::Replay { .. }) { "Replay window" } else { "Crop" }, view.source_name);
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
    if view.context.preview() { push(frame, "Fighter", fighter, Tone::Normal); }
    let mut window = String::new();
    if let ClipContext::Combo { unit } = view.context {
        let _ = write!(window, "{} - {}  ({length} {} of {})", view.start + 1, view.end, if length == 1 { unit.singular() } else { unit.plural() }, view.total);
    } else {
        let _ = write!(window, "{} - {}  ({length} frames, {:.2}s of {})", view.start, view.end, length as f32 / FPS, view.total);
    }
    push(frame, "Crop window", &window, Tone::Normal);
    let mut head = String::new();
    let _ = write!(head, "{}{}", view.cursor, if view.playing { "  playing" } else { "" });
    push(frame, "Playhead", &head, if view.playing { Tone::On } else { Tone::Normal });
    if view.truncated {
        push(frame, "This capture is partial", "", Tone::Dim);
    }
    if view.quantized {
        push(frame, "Frames were quantized by the capture", "", Tone::Dim);
    }
    if view.previous_available {
        push(frame, "Capture", if view.previous_selected { "previous" } else { "latest" }, Tone::Dim);
    }
    let help = clip_help(view, COARSE, CHORD, pace);
    if !help.notice.is_empty() {
        push(frame, &help.notice, "", Tone::Dim);
    }
    let _ = frame.help[0].write_str(&help.primary);
    let _ = frame.help[1].write_str(&help.follow);
    let _ = frame.hint.write_str(&help.controls);
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn view() -> ClipView {
        ClipView {
            context: ClipContext::Replay { fighters: true },
            total: 1800,
            start: 120,
            end: 480,
            cursor: 300,
            handle: Handle::Out,
            playing: false,
            entry: 1,
            fighter_kind: 8,
            previous_selected: false,
            previous_available: true,
            clean_start: true,
            move_start: None,
            quantized: false,
            truncated: true,
            source_name: "Latest Vault capture".into(),
            error: None,
        }
    }

    fn a_playing_preview_and_a_refusal_show_in_the_frame() {
        let mut v = view();
        v.playing = true;
        v.truncated = false;
        v.previous_available = false;
        let mut f = Frame::new();
        clip_frame(&v, "Pikachu", 0, &mut f);
        assert_eq!(f.rows[2].value.string_lossy(), "300  playing");
        assert_eq!(f.rows[2].value_tone, Tone::On);
        assert!(f.clip.playing);
        assert_eq!(f.help[0].string_lossy(), "Previewing the selection.");
        v.playing = false;
        v.error = Some("Native preview is unavailable until the preview restore is provided".into());
        clip_frame(&v, "Pikachu", 0, &mut f);
        assert!(f.help[0].string_lossy().starts_with("Native preview is unavailable"), "{:?}", f.help[0]);
    }

    fn a_held_scrub_names_its_speed() {
        let mut f = Frame::new();
        clip_frame(&view(), "Pikachu", 45, &mut f);
        assert!(f.help[1].string_lossy().contains("45 frames per tick"), "{:?}", f.help[1]);
    }

    /// rc.14 recfix (Eden rc.13 still `p2-replay-editor`): the editor is drawn alone, with no tab row ("the title and breadcrumb,
    /// a few lines, the help texts, the timeline"). It builds on the page's frame, so it must give back its tab slots: the
    /// renderer drew the red active-tab box with no names when a frame still named a tab, or left the last one's box up.
    fn the_clip_editor_names_no_tab_so_no_active_tab_box_is_drawn() {
        use crate::model::menu::{Env, Menu};
        use crate::model::settings::Settings;
        let mut f = Frame::new();
        crate::model::frame::page_frame(&Menu::new(Settings::factory(), Env::default()), &mut f);
        assert!(f.tab_slots().iter().flatten().any(|slot| slot.active), "the page it follows has an active tab");
        clip_frame(&view(), "Pikachu", 0, &mut f);
        assert!(f.tab_slots().iter().all(Option::is_none), "{:?}", f.tab_slots());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_clip_editor_names_no_tab_so_no_active_tab_box_is_drawn", the_clip_editor_names_no_tab_so_no_active_tab_box_is_drawn as fn()),
            ("a_playing_preview_and_a_refusal_show_in_the_frame", a_playing_preview_and_a_refusal_show_in_the_frame as fn()),
            ("a_held_scrub_names_its_speed", a_held_scrub_names_its_speed as fn()),
        ]);
    }
    }
}
