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
pub enum Effect { None, Open(String), Close, TimedOut, Notify(String) }

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

