//! The menu's hidden state while an input drill's take previews (fix round 2026-11-23, F10). The Recording player
//! plays the take (`rr_host::request_preview`); this decides what the hidden menu does with a press and when the page
//! returns. Pure: `runtime::ui::worker` owns one.

use super::clip_panel::PreviewTail;
use crate::model::menu::input::Button;

/// What the worker does after a press or a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Keep the menu hidden.
    Hide,
    /// X or B: ask the player to stop. The menu stays hidden until a pass says `Done`.
    Stop,
    /// Show the page again: the take ended and its tail passed, or the player has stopped after X or B.
    Done,
}

/// One drill preview. The tail restarts whenever the player runs; after X or B the page waits only for the player
/// to stop.
#[derive(Default)]
pub struct DrillPreview { tail: PreviewTail, stopping: bool }

impl DrillPreview {
    pub const fn new() -> Self { Self { tail: PreviewTail::new(), stopping: false } }

    /// A press while the menu is hidden: the first X or B stops the preview, as in the Crop editor; nothing else acts.
    pub fn press(&mut self, button: Button) -> Step {
        if matches!(button, Button::X | Button::B) && !self.stopping {
            self.stopping = true;
            Step::Stop
        } else {
            Step::Hide
        }
    }

    /// One worker pass. `running`: the player still has frames (`rr_host::preview_running`). `frame`: the game's frame
    /// number. After X or B the page returns on the first pass with no run left, so no frame of the take is handed to
    /// Player 1 behind a shown page. Otherwise it returns `PREVIEW_TAIL_FRAMES` after the last input, as after a
    /// Crop preview.
    pub fn pass(&mut self, running: bool, frame: u64) -> Step {
        let over = if self.stopping { !running } else { self.tail.over(running, frame) };
        if over { Step::Done } else { Step::Hide }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn a_playing_preview_returns_before_the_editor_keys() {
        let source = include_str!("../../runtime/macros/replay_capture.rs");
        let input = source.split("pub fn on_input(").nth(1).unwrap();
        let zr = input.find("if TARGET_TAP.lock().unwrap().zr_tap(held)").unwrap();
        let playing = input.find("if clip_view().is_some_and(|v| v.playing)").unwrap();
        let mapping = input.find("let action = if p.b() { Some(ClipAction::Cancel) }").unwrap();
        assert!(zr < playing && playing < mapping);
        assert!(input[zr..playing].contains("on_action(ClipAction::SwapTarget);"));
        let branch = input[playing..mapping].split("return;").next().unwrap();
        assert!(branch.contains("controller.just_down.x() || controller.just_down.b()"));
        assert!(branch.contains("on_action(ClipAction::TogglePlay);"));
        assert_eq!(branch.matches("on_action(").count(), 1);
        assert!(input[playing..mapping].contains("return;"));
    }

    #[test]
    fn drill_preview_b_stops_and_keeps_the_page() {
        let mut preview = DrillPreview::new();
        assert_eq!(preview.press(Button::B), Step::Stop);
        assert_eq!(preview.press(Button::B), Step::Hide);
        assert_eq!(preview.pass(true, 1), Step::Hide);
        assert_eq!(preview.pass(false, 2), Step::Done);
        let worker = include_str!("../../runtime/ui/worker.rs");
        let press = worker.split("fn press(button: Button, s: &mut Session)").nth(1).unwrap();
        let hidden = press.split("if PREVIEW_HIDES.load(Ordering::Acquire)").nth(1).unwrap()
            .split("if s.menu.is_none()").next().unwrap();
        assert!(hidden.contains("s.drill_preview.press(button) == Step::Stop"));
        assert!(hidden.contains("rr_host::stop_preview();") && hidden.contains("return;"));
        assert!(!hidden.contains("end_drill_preview(") && !hidden.contains("PREVIEW_HIDES.store"));
    }

    #[test]
    fn the_drill_strip_shows_the_same_stop_hint() {
        assert_eq!(crate::editors::clip::PREVIEW_STOP_HINT, "B stop preview");
        let worker = include_str!("../../runtime/ui/worker.rs");
        let publish = worker.split("fn publish_frame(s: &Session)").nth(1).unwrap();
        let strip = publish.split("if !flow::frame_needs_menu(s.phase)").next().unwrap();
        let editor = publish.split("if !flow::frame_needs_menu(s.phase)").nth(1).unwrap()
            .split("let Some(menu)").next().unwrap();
        assert!(strip.contains("stop_hint(&mut next, true,") && strip.contains("preview_running()"));
        assert!(editor.contains("stop_hint(&mut next, view.playing,") && editor.contains("preview_running()"));
        assert!(!worker.contains(concat!("X or B:", " stop.")));
    }

    #[test]
    fn the_editor_redraws_when_the_preview_ends() {
        let mut tail = PreviewTail::new();
        assert!(!tail.over(true, 10));
        assert!(!tail.over(false, 11));
        let end = 11 + crate::engine::macro_ui::clip_panel::PREVIEW_TAIL_FRAMES;
        assert!(!tail.over(false, end - 1));
        assert!(tail.over(false, end));
        tail.restart();
        assert!(!tail.over(false, end));
        assert!(tail.over(false, end + crate::engine::macro_ui::clip_panel::PREVIEW_TAIL_FRAMES));
        let source = include_str!("../../runtime/macros/replay_capture.rs");
        let service = source.split("pub fn service_reload()").nth(1).unwrap()
            .split("pub fn preview_failed(").next().unwrap();
        assert!(service.contains("TAIL.lock().unwrap().over(rr_host::preview_running()"));
        let worker = include_str!("../../runtime/ui/worker.rs");
        let watching = worker.split("fn watching(&self)").nth(1).unwrap().split("\n    }").next().unwrap();
        assert!(watching.contains("self.phase != Phase::Menu"));
        let run = worker.split("fn run()").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(run.contains("sync_panels(&mut session);"));
        assert!(run.contains("session.watching()") && run.contains("WAKE.wait(Some(POLL))"));
        let sync = worker.split("fn sync_panels(s: &mut Session)").nth(1).unwrap().split("\n}\n").next().unwrap();
        assert!(sync.contains("if open || s.phase == Phase::Clip || s.phase == Phase::Prompt"));
        assert!(sync.contains("publish_frame(s);"));
    }

    #[test]
    fn only_x_and_b_stop_a_drill_preview_and_the_page_returns_after_the_tail() {
        let mut preview = DrillPreview::new();
        for button in [Button::Up, Button::Down, Button::Left, Button::Right, Button::A, Button::Y,
            Button::L, Button::R, Button::ZL, Button::ZR, Button::JumpUp, Button::JumpDown] {
            assert_eq!(preview.press(button), Step::Hide);
        }
        assert_eq!(preview.pass(true, 99), Step::Hide);
        for frame in 100..=159 { assert_eq!(preview.pass(false, frame), Step::Hide); }
        assert_eq!(preview.pass(false, 160), Step::Done);
        assert_eq!(preview.pass(true, 161), Step::Hide);
        assert_eq!(preview.pass(false, 162), Step::Hide);
        for button in [Button::X, Button::B] {
            assert_eq!(DrillPreview::new().press(button), Step::Stop);
        }
    }

    #[test]
    fn after_x_or_b_the_page_waits_for_the_player_to_stop_and_skips_the_tail() {
        for button in [Button::X, Button::B] {
            let mut preview = DrillPreview::new();
            assert_eq!(preview.press(button), Step::Stop);
            assert_eq!(preview.press(Button::X), Step::Hide);
            assert_eq!(preview.press(Button::B), Step::Hide);
            for frame in 0..5 { assert_eq!(preview.pass(true, frame), Step::Hide); }
            assert_eq!(preview.pass(false, 5), Step::Done);
        }
    }

    #[test]
    fn a_drill_preview_hides_the_open_menu_and_keeps_its_pad() {
        let worker = include_str!("../../runtime/ui/worker.rs");
        let tick = include_str!("../../runtime/ui/tick.rs");
        let state = include_str!("../../runtime/ui/state.rs");
        let body = |source: &'static str, name: &str| {
            let start = source.find(name).unwrap_or_else(|| panic!("missing {name}"));
            &source[start..start + source[start..].find("\n}\n").expect("end of function")]
        };
        assert!(worker.contains("Effect::PreviewDrill { source, frames, trim } => start_drill_preview(&source, frames, trim)"), "the effect starts the preview");
        let start = body(worker, "fn start_drill_preview(");
        assert!(start.contains("replay_capture::capture_ready()"));
        assert!(start.contains("|| !crate::engine::rr_host::request_drill_preview(source, frames, trim)"));
        assert!(!start.contains("rr_host::request_preview("));
        let request = start.find("request_drill_preview(source, frames, trim)").unwrap();
        let stop = start[request..].find("return;").unwrap() + request;
        let hide = start.find("PREVIEW_HIDES.store(true, Ordering::Release)").unwrap();
        assert!(request < stop && stop < hide);
        let press = body(worker, "fn press(button: Button, s: &mut Session)");
        let hidden = press.find("if PREVIEW_HIDES.load(Ordering::Acquire)").expect("the hidden preview owns presses");
        assert!(hidden < press.find("menu.press_outcome(button)").expect("the menu press"));
        let hidden_press = &press[hidden..press.find("if s.menu.is_none()").unwrap()];
        assert!(hidden_press.contains("s.drill_preview.press(button) == Step::Stop"));
        assert!(hidden_press.contains("rr_host::stop_preview();") && hidden_press.contains("return;"));
        assert!(!hidden_press.contains("end_drill_preview(") && !hidden_press.contains("PREVIEW_HIDES.s"));
        assert!(worker.contains("sync_drill_preview(&mut session);"));
        let sync = body(worker, "fn sync_drill_preview(");
        assert!(sync.contains("let gone = !(OPEN.load(Ordering::Acquire) || POPUP.load(Ordering::Acquire))"));
        assert!(sync.contains("if gone ||") && sync.contains("end_drill_preview(s, gone);"),
            "the preview stops when neither menu nor popup remains");
        let end = body(worker, "fn end_drill_preview(");
        assert!(end.contains("PREVIEW_HIDES.swap(false, Ordering::AcqRel)") && end.contains("rr_host::stop_preview()"));
        assert!(end.contains("s.drill_preview = DrillPreview::new();"));
        let handle = body(worker, "fn handle(event: Event, s: &mut Session)");
        assert!(handle.find("end_drill_preview(s, true);").unwrap() < handle.find("s.keeper.open(").unwrap());
        let reset = body(worker, "fn sync_input_generation(");
        assert!(reset.find("end_drill_preview(s, true);").unwrap() < reset.find("s.keeper.drop_model(").unwrap());
        assert!(tick.contains("let hidden = state::PREVIEW_HIDES.load(Ordering::Acquire) && !state::PREVIEW_STRIP.load(Ordering::Acquire);"));
        assert!(tick.contains("&& !hidden;"));
        assert!(state.contains("pub static PREVIEW_HIDES: AtomicBool"));
        assert!(!body(state, "pub fn release_menu()").contains("PREVIEW_HIDES"),
            "only the worker clears the preview flag");
    }
    }
}
