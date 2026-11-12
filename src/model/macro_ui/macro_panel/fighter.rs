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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn gate_asks_only_for_a_real_mismatch_that_can_be_switched() {
        // Same fighter: never a prompt, whatever the setting.
        assert_eq!(gate(true, true, true), Gate::Proceed);
        assert_eq!(gate(false, true, true), Gate::Proceed);
        // Mismatch with the setting on.
        assert_eq!(gate(true, true, false), Gate::Prompt);
        // Setting off keeps the current CPU without asking.
        assert_eq!(gate(false, true, false), Gate::Keep);
        // Nothing to switch to (older Mii file): keep, never prompt.
        assert_eq!(gate(true, false, false), Gate::Keep);
    }

    fn prompt_defaults_to_keep_and_cancel_is_distinct_from_both_choices() {
        let mut prompt = Prompt::new(Origin::Named, ViewRole::Cpu, 7, "Kirby".into(), "Hero".into());
        assert_eq!(prompt.choice(), Choice::Keep);
        assert_eq!(prompt.view().selected, 0);
        assert_eq!(prompt.on_pad(Pad::Confirm), Outcome::Chose(Choice::Keep));
        assert_eq!(prompt.on_pad(Pad::Back), Outcome::Cancelled);
        assert_eq!(prompt.on_pad(Pad::Extra), Outcome::None);
        assert_eq!(prompt.on_pad(Pad::Down), Outcome::None);
        assert_eq!(prompt.choice(), Choice::Switch);
        assert_eq!(prompt.on_pad(Pad::Confirm), Outcome::Chose(Choice::Switch));
        // The cursor clamps; it never wraps past either row.
        prompt.on_pad(Pad::Right);
        assert_eq!(prompt.choice(), Choice::Switch);
        prompt.on_pad(Pad::Up);
        prompt.on_pad(Pad::Left);
        assert_eq!(prompt.choice(), Choice::Keep);
        assert_eq!((prompt.origin(), prompt.revision()), (Origin::Named, 7));
    }

    fn prompt_view_names_both_fighters_and_both_actions() {
        let view = Prompt::new(Origin::Clip, ViewRole::Cpu, 1, "Kirby".into(), "Hero".into()).view();
        assert_eq!(view.labels.len(), PAGE);
        assert_eq!(view.labels[0], "Keep current fighter");
        assert_eq!(view.labels[1], "Switch to recorded fighter");
        assert!(view.labels[2..].iter().all(String::is_empty));
        assert!(view.footer.contains("Kirby") && view.footer.contains("Hero"));
        assert!(view.footer.contains("A chooses") && view.footer.contains("B cancels"));
    }

    fn a_recording_prompt_uses_the_same_rows_and_starts_on_keep() {
        let prompt = Prompt::new(Origin::Recording, ViewRole::Player, 0, "Pikachu".into(), "Mario".into());
        let view = prompt.view();
        assert_eq!(view.title, "Fighter Mismatch");
        assert_eq!(view.labels[0], "Keep current fighter");
        assert_eq!(view.labels[1], "Switch to recorded fighter");
        assert_eq!(view.selected, 0);
        assert_eq!(prompt.choice(), Choice::Keep);
        assert!(view.footer.contains("Pikachu") && view.footer.contains("Mario"));
        assert!(view.footer.contains("B cancels"));
    }

    fn the_footer_names_the_side_that_plays() {
        // hot-slots 2026-11-10 18:58Z (`cov-mismatch-cancel`): Play on Player 1 (Donkey Kong) with the CPU on Mario read
        // "Recorded Mario · current CPU Donkey Kong". The role word is the playing side's, in the menu's names.
        let first = |origin, side| {
            let footer = Prompt::new(origin, side, 0, "Mario".into(), "Donkey Kong".into()).view().footer;
            footer.lines().next().unwrap_or_default().to_owned()
        };
        for origin in [Origin::Named, Origin::Clip, Origin::Recording] {
            assert_eq!(first(origin, ViewRole::Player), "Recorded Mario · current Player 1 Donkey Kong", "{origin:?}");
            assert_eq!(first(origin, ViewRole::Cpu), "Recorded Mario · current CPU Donkey Kong", "{origin:?}");
        }
        // A combo's question is about Player 1's fighter: its wording is the same whichever side is passed.
        assert_eq!(first(Origin::Combo, ViewRole::Player), "Recorded on Mario · P1 is Donkey Kong");
        assert_eq!(first(Origin::Combo, ViewRole::Cpu), "Recorded on Mario · P1 is Donkey Kong");
        // The second line is unchanged for every origin.
        let view = Prompt::new(Origin::Recording, ViewRole::Player, 0, "Mario".into(), "Donkey Kong".into()).view();
        assert_eq!(view.footer.lines().nth(1), Some("Inputs are the same either way. A chooses, B cancels."));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("gate_asks_only_for_a_real_mismatch_that_can_be_switched", gate_asks_only_for_a_real_mismatch_that_can_be_switched as fn()),
            ("prompt_defaults_to_keep_and_cancel_is_distinct_from_both_choices", prompt_defaults_to_keep_and_cancel_is_distinct_from_both_choices as fn()),
            ("prompt_view_names_both_fighters_and_both_actions", prompt_view_names_both_fighters_and_both_actions as fn()),
            ("a_recording_prompt_uses_the_same_rows_and_starts_on_keep", a_recording_prompt_uses_the_same_rows_and_starts_on_keep as fn()),
            ("the_footer_names_the_side_that_plays", the_footer_names_the_side_that_plays as fn()),
        ]);
    }
    }
}
