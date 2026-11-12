//! Which fighter plays a recording's inputs: the one it was recorded with, or the
//! CPU already on stage.
//!
//! Recordings carry inputs, so any fighter can play them. When the recorded
//! fighter differs from the settled CPU the user chooses once per explicit
//! selection. This module is value-only: the runtime computes `matches` with its
//! exact readiness predicate (a same-class Mii with different exact
//! configuration is not a match) and owns every native request.

use super::{Pad, View, PAGE};
use crate::model::recording_menu::ViewRole;

/// What a selection must do before it may play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// The recorded identity is already the settled CPU. Nothing to choose.
    Proceed,
    /// Play on the current CPU. No prompt, no CPU change.
    Keep,
    /// Ask once for this selection.
    Prompt,
}

/// `matches`: the recorded identity is already the settled CPU.
/// `switchable`: an exact recorded target exists to switch to; an older Mii file
/// without its exact Mii has none, so there is nothing to offer.
/// `prompt_enabled`: the persistent Fighter Prompt setting.
pub fn gate(prompt_enabled: bool, switchable: bool, matches: bool) -> Gate {
    if matches {
        Gate::Proceed
    } else if !prompt_enabled || !switchable {
        Gate::Keep
    } else {
        Gate::Prompt
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Safe default: inputs play on the current CPU, which is never reloaded.
    Keep,
    /// Load the recorded fighter through the existing exact-fighter route.
    Switch,
}

/// Which selection owns the prompt. The runtime routes the result by origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Named library load. The UI keeps the candidate and applies the choice.
    Named,
    /// Replay clip/track selection. The clip editor keeps the candidate.
    Clip,
    /// A chosen combo (SPEC-input-07 §4a): the question is about Player 1's
    /// fighter, and the combos UI keeps the candidate.
    Combo,
    /// A recording, a combo drill, or a chain's macro 1. The recording worker keeps the candidate.
    Recording,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    None,
    Chose(Choice),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    origin: Origin,
    /// The side the inputs play on; `current` is its fighter.
    side: ViewRole,
    revision: u64,
    recorded: String,
    current: String,
    cursor: usize,
}

const ROWS: [(&str, Choice); 2] = [
    ("Keep current fighter", Choice::Keep),
    ("Switch to recorded fighter", Choice::Switch),
];

impl Prompt {
    /// `revision` is the owning selection's revision. The caller drops a prompt
    /// whose revision is no longer current. The cursor starts on Keep.
    pub fn new(origin: Origin, side: ViewRole, revision: u64, recorded: String, current: String) -> Self {
        Self { origin, side, revision, recorded, current, cursor: 0 }
    }

    pub fn origin(&self) -> Origin {
        self.origin
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn choice(&self) -> Choice {
        ROWS[self.cursor].1
    }

    pub fn on_pad(&mut self, pad: Pad) -> Outcome {
        match pad {
            Pad::Left | Pad::Up => self.cursor = self.cursor.saturating_sub(1),
            Pad::Right | Pad::Down => self.cursor = (self.cursor + 1).min(ROWS.len() - 1),
            Pad::Confirm => return Outcome::Chose(self.choice()),
            Pad::Back => return Outcome::Cancelled,
            Pad::Extra | Pad::Rename => {}
        }
        Outcome::None
    }

    pub fn view(&self) -> View {
        let mut labels = vec![String::new(); PAGE];
        for (label, (text, _)) in labels.iter_mut().zip(ROWS) {
            *label = text.to_owned();
        }
        let footer = match self.origin {
            Origin::Combo => format!(
                "Recorded on {} · P1 is {}\nThe combo is graded the same on either. A chooses, B cancels.",
                self.recorded, self.current
            ),
            Origin::Named | Origin::Clip | Origin::Recording => format!(
                "Recorded {} · current {} {}\nInputs are the same either way. A chooses, B cancels.",
                self.recorded, self.side.label(), self.current
            ),
        };
        View {
            title: "Fighter Mismatch".to_owned(),
            footer,
            labels,
            selected: self.cursor,
        }
    }
}

