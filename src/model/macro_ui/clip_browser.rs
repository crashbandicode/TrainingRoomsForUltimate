//! Capture selection while a waiting-lobby CPU finishes its current native action.
//! No fighter pointers or named-macro state belong to this value-only browser.
use crate::engine::macro_ui::macro_panel::{Effect as MenuEffect, LibraryRow, MacroUi, Pad, View};

const READY_TICKS: u32 = 180;

enum Selection {
    Browsing,
    Settling { id: String, ticks: u32 },
    Opening,
    Closed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Effect { None, Open(String), Close, TimedOut, Notify(String), Delete(String) }

pub struct ClipBrowser { ui: MacroUi, selection: Selection }

impl ClipBrowser {
    pub fn new(rows: Vec<LibraryRow>, skipped: usize) -> Self {
        let mut ui = MacroUi::new();
        ui.open_library(rows, skipped);
        Self { ui, selection: Selection::Browsing }
    }

    pub fn pending(&self) -> bool {
        matches!(self.selection, Selection::Settling { .. } | Selection::Opening)
    }

    pub fn opening(&self) -> bool { matches!(self.selection, Selection::Opening) }

    /// Only a nonclean CPU may advance. Hold it again as soon as it is clean,
    /// including between the readiness tick and the native adapter's publication.
    pub fn release_cpu(&self, clean: bool) -> bool {
        matches!(self.selection, Selection::Settling { .. }) && !clean
    }

    pub fn view(&self) -> Option<View> {
        let mut view = match self.selection {
            Selection::Browsing => self.ui.view()?,
            Selection::Settling { .. } | Selection::Opening => View {
                title: String::new(), selected: 0,
                labels: vec![if self.opening() { "Opening capture..." }
                    else { "Waiting for CPU idle or fall..." }.into()],
                footer: "B: cancel".into(),
            },
            Selection::Closed => return None,
        };
        view.title = "Replay Captures".into();
        Some(view)
    }

    /// After a delete: the list as storage has it now. False when nothing is left and the window closed.
    pub fn relist(&mut self, _rows: Vec<LibraryRow>, _skipped: usize) -> bool {
        true
    }

    pub fn on_pad(&mut self, pad: Pad, waiting: bool) -> Effect {
        if matches!(pad, Pad::Back) {
            self.selection = Selection::Closed;
            return Effect::Close;
        }
        // Repeated Confirm and navigation cannot replace an admitted ID or renew
        // its deadline. Training continues to use immediate, validated opening.
        if !matches!(self.selection, Selection::Browsing) { return Effect::None; }
        match self.ui.on_pad(pad) {
            MenuEffect::Choose(id) if waiting => {
                self.selection = Selection::Settling { id, ticks: 0 };
                Effect::None
            }
            MenuEffect::Choose(id) => {
                self.selection = Selection::Opening;
                Effect::Open(id)
            }
            MenuEffect::Notify(error) => Effect::Notify(error),
            _ => Effect::None,
        }
    }

    /// Once per active practice layout, not per fighter or input poll. Departure
    /// wins even when CPU readiness arrives in the same tick. No request resumes
    /// itself after Cancel, timeout or an interruption.
    pub fn tick(&mut self, eligible: bool, clean: bool) -> Effect {
        // Opening is already in the native adapter (including synchronous
        // Training). Its revision/context fence, not this waiter, publishes it.
        if !matches!(self.selection, Selection::Settling { .. }) { return Effect::None; }
        if !eligible {
            self.selection = Selection::Closed;
            return Effect::Close;
        }
        let Selection::Settling { id, ticks } = &mut self.selection else { return Effect::None; };
        if clean {
            let id = std::mem::take(id);
            self.selection = Selection::Opening;
            return Effect::Open(id);
        }
        *ticks += 1;
        if *ticks >= READY_TICKS {
            self.selection = Selection::Closed;
            Effect::TimedOut
        } else { Effect::None }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn browser() -> ClipBrowser {
        ClipBrowser::new(vec![
            LibraryRow { id: "saved:500".into(), label: "Replay500f".into() },
            LibraryRow { id: "saved:other".into(), label: "Other capture".into() },
        ], 0)
    }

    fn active_waiting_cpu_can_finish_before_the_exact_capture_opens() {
        let mut browser = browser();
        assert_eq!(browser.on_pad(Pad::Confirm, true), Effect::None);
        assert!(browser.pending());
        // Counterexample: Mario is attacking, rather than an idle Training CPU.
        // The browser must release its panel stop and wait, not reopen/refreeze.
        assert!(browser.release_cpu(false));
        for _ in 0..45 {
            assert_eq!(browser.tick(true, false), Effect::None);
            assert_eq!(browser.on_pad(Pad::Confirm, true), Effect::None);
            assert_eq!(browser.on_pad(Pad::Right, true), Effect::None);
        }
        // WAIT or either admitted fall status is sampled by the native adapter.
        assert!(!browser.release_cpu(true));
        assert_eq!(browser.tick(true, true), Effect::Open("saved:500".into()));
        assert!(browser.opening());
        assert!(!browser.release_cpu(false));
        assert_eq!(browser.tick(true, true), Effect::None); // One load only.
    }

    fn idle_training_stays_immediate_and_waiting_can_choose_another_capture() {
        let mut training = browser();
        assert_eq!(training.on_pad(Pad::Confirm, false), Effect::Open("saved:500".into()));
        assert!(!training.release_cpu(false));
        // The waiting-only layout clock must not cancel a synchronous Training load.
        assert_eq!(training.tick(false, true), Effect::None);
        assert!(training.opening());

        let mut waiting = browser();
        assert_eq!(waiting.on_pad(Pad::Right, true), Effect::None);
        assert_eq!(waiting.on_pad(Pad::Confirm, true), Effect::None);
        assert_eq!(waiting.tick(true, true), Effect::Open("saved:other".into()));
    }

    fn y_deletes_the_highlighted_capture_and_keeps_the_list_open() {
        let mut browser = browser();
        assert_eq!(browser.view().unwrap().footer, "A loads, Y deletes, B closes.");
        assert_eq!(browser.on_pad(Pad::Right, false), Effect::None);
        // The effect carries the highlighted row's own id. The list is rebuilt from what storage holds afterwards.
        assert_eq!(browser.on_pad(Pad::Extra, false), Effect::Delete("saved:other".into()));
        assert!(browser.relist(vec![LibraryRow { id: "saved:500".into(), label: "Replay500f".into() }], 0));
        let view = browser.view().expect("the list stays open");
        assert_eq!(view.title, "Replay Captures");
        assert!(view.labels.iter().any(|label| label.contains("Replay500f")));
        assert!(!view.labels.iter().any(|label| label.contains("Other capture")));
        assert_eq!(view.footer, "A loads, Y deletes, B closes.");
        assert_eq!(browser.on_pad(Pad::Confirm, false), Effect::Open("saved:500".into()));
    }

    fn a_delete_that_fails_leaves_the_capture_in_the_replay_window() {
        let mut browser = browser();
        assert_eq!(browser.on_pad(Pad::Extra, false), Effect::Delete("saved:500".into()));
        // Storage still has both.
        assert!(browser.relist(vec![
            LibraryRow { id: "saved:500".into(), label: "Replay500f".into() },
            LibraryRow { id: "saved:other".into(), label: "Other capture".into() },
        ], 0));
        let view = browser.view().unwrap();
        assert!(view.labels.iter().any(|label| label.contains("Replay500f")));
        assert!(view.labels.iter().any(|label| label.contains("Other capture")));
        assert_eq!(browser.on_pad(Pad::Confirm, false), Effect::Open("saved:500".into()));
    }

    fn deleting_the_last_capture_closes_the_replay_window() {
        let mut browser = ClipBrowser::new(vec![LibraryRow { id: "latest".into(), label: "Latest Vault capture".into() }], 0);
        assert_eq!(browser.on_pad(Pad::Extra, false), Effect::Delete("latest".into()));
        assert!(!browser.relist(Vec::new(), 0));
        assert!(browser.view().is_none());
        assert_eq!(browser.on_pad(Pad::Confirm, false), Effect::None);
        assert_eq!(browser.on_pad(Pad::Extra, false), Effect::None);
    }

    fn y_deletes_in_training_and_in_a_waiting_room() {
        for waiting in [false, true] {
            let mut browser = browser();
            assert_eq!(browser.on_pad(Pad::Extra, waiting), Effect::Delete("saved:500".into()));
            assert!(!browser.pending());
        }
        // A capture already chosen in a waiting room is opening: Y does nothing until B.
        let mut chosen = browser();
        assert_eq!(chosen.on_pad(Pad::Confirm, true), Effect::None);
        assert_eq!(chosen.on_pad(Pad::Extra, true), Effect::None);
        assert!(chosen.pending());
        assert_eq!(chosen.tick(true, true), Effect::Open("saved:500".into()));
        assert_eq!(chosen.on_pad(Pad::Extra, true), Effect::None, "nor while it opens");
        // B closes and deletes nothing.
        assert_eq!(browser().on_pad(Pad::Back, false), Effect::Close);
    }

    fn explicit_cancel_before_or_after_readiness_cannot_open_on_return() {
        for ready in [false, true] {
            let mut browser = browser();
            browser.on_pad(Pad::Confirm, true);
            browser.tick(true, ready);
            assert_eq!(browser.on_pad(Pad::Back, true), Effect::Close);
            assert!(!browser.pending());
            assert!(!browser.opening()); // Native publication rejects this state.
            assert!(!browser.release_cpu(false));
            assert!(browser.view().is_none());
            assert_eq!(browser.tick(true, true), Effect::None);
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("y_deletes_the_highlighted_capture_and_keeps_the_list_open", y_deletes_the_highlighted_capture_and_keeps_the_list_open as fn()),
            ("a_delete_that_fails_leaves_the_capture_in_the_replay_window", a_delete_that_fails_leaves_the_capture_in_the_replay_window as fn()),
            ("deleting_the_last_capture_closes_the_replay_window", deleting_the_last_capture_closes_the_replay_window as fn()),
            ("y_deletes_in_training_and_in_a_waiting_room", y_deletes_in_training_and_in_a_waiting_room as fn()),
            ("active_waiting_cpu_can_finish_before_the_exact_capture_opens", active_waiting_cpu_can_finish_before_the_exact_capture_opens as fn()),
            ("idle_training_stays_immediate_and_waiting_can_choose_another_capture", idle_training_stays_immediate_and_waiting_can_choose_another_capture as fn()),
            ("explicit_cancel_before_or_after_readiness_cannot_open_on_return", explicit_cancel_before_or_after_readiness_cannot_open_on_return as fn()),
        ]);
    }
    }
}
