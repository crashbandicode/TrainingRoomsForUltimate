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
    /// X or B: stop the player and show the page again.
    Stop,
    /// The take ended and its tail passed: show the page again.
    Done,
}

/// One drill preview after another: the tail restarts whenever the player runs.
#[derive(Default)]
pub struct DrillPreview { tail: PreviewTail }

impl DrillPreview {
    pub const fn new() -> Self { Self { tail: PreviewTail::new() } }

    /// A press while the menu is hidden: X and B stop the preview, as in the Crop editor; nothing else acts.
    pub fn press(&self, button: Button) -> Step {
        if matches!(button, Button::X | Button::B) { Step::Stop } else { Step::Hide }
    }

    /// One worker pass. `running`: the player still has frames (`rr_host::preview_running`). `frame`: the game's frame
    /// number. The page returns `PREVIEW_TAIL_FRAMES` after the last input, as after a Crop preview.
    pub fn pass(&mut self, running: bool, frame: u64) -> Step {
        if self.tail.over(running, frame) { Step::Done } else { Step::Hide }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn only_x_and_b_stop_a_drill_preview_and_the_page_returns_after_the_tail() {
        let mut preview = DrillPreview::new();
        for button in [Button::Up, Button::Down, Button::Left, Button::Right, Button::A, Button::Y,
            Button::L, Button::R, Button::ZL, Button::ZR, Button::JumpUp, Button::JumpDown] {
            assert_eq!(preview.press(button), Step::Hide);
        }
        for button in [Button::X, Button::B] {
            assert_eq!(preview.press(button), Step::Stop);
        }
        assert_eq!(preview.pass(true, 99), Step::Hide);
        for frame in 100..=159 { assert_eq!(preview.pass(false, frame), Step::Hide); }
        assert_eq!(preview.pass(false, 160), Step::Done);
        assert_eq!(preview.pass(true, 161), Step::Hide);
        assert_eq!(preview.pass(false, 162), Step::Hide);
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
        assert!(worker.contains("Effect::PreviewDrill { source, frames } => start_drill_preview(&source, frames)"), "the effect starts the preview");
        let start = body(worker, "fn start_drill_preview(");
        assert!(start.contains("replay_capture::capture_ready()"));
        assert!(start.contains("|| !crate::engine::rr_host::request_drill_preview(source, frames)"));
        assert!(!start.contains("rr_host::request_preview("));
        let request = start.find("request_drill_preview(source, frames)").unwrap();
        let stop = start[request..].find("return;").unwrap() + request;
        let hide = start.find("PREVIEW_HIDES.store(true, Ordering::Release)").unwrap();
        assert!(request < stop && stop < hide);
        let press = body(worker, "fn press(button: Button, s: &mut Session)");
        let hidden = press.find("if PREVIEW_HIDES.load(Ordering::Acquire)").expect("the hidden preview owns presses");
        assert!(hidden < press.find("menu.press_outcome(button)").expect("the menu press"));
        let hidden_press = &press[hidden..press.find("if s.menu.is_none()").unwrap()];
        assert!(hidden_press.contains("s.drill_preview.press(button) == Step::Stop"));
        assert!(hidden_press.contains("end_drill_preview(s, true);") && hidden_press.contains("return;"));
        assert!(worker.contains("sync_drill_preview(&mut session);"));
        let sync = body(worker, "fn sync_drill_preview(");
        assert!(sync.contains("let gone = !(OPEN.load(Ordering::Acquire) || POPUP.load(Ordering::Acquire))"));
        assert!(sync.contains("if gone ||") && sync.contains("end_drill_preview(s, gone);"),
            "the preview stops when neither menu nor popup remains");
        let end = body(worker, "fn end_drill_preview(");
        assert!(end.contains("PREVIEW_HIDES.swap(false, Ordering::AcqRel)") && end.contains("rr_host::stop_preview()"));
        let handle = body(worker, "fn handle(event: Event, s: &mut Session)");
        assert!(handle.find("end_drill_preview(s, true);").unwrap() < handle.find("s.keeper.open(").unwrap());
        let reset = body(worker, "fn sync_input_generation(");
        assert!(reset.find("end_drill_preview(s, true);").unwrap() < reset.find("s.keeper.drop_model(").unwrap());
        assert!(tick.contains("&& !state::PREVIEW_HIDES.load(Ordering::Acquire)"));
        assert!(state.contains("pub static PREVIEW_HIDES: AtomicBool"));
        assert!(!body(state, "pub fn release_menu()").contains("PREVIEW_HIDES"),
            "only the worker clears the preview flag");
    }
    }
}
