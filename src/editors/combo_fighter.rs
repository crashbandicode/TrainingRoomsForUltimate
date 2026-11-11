//! Which fighter P1 practises a chosen combo on (SPEC-input-07 §4a; operator 2026-11-05, DECISIONS DEFERRED-LIST): the
//! named macros' Switch/Keep question (`macro_panel::fighter::gate`, unchanged), asked about Player 1 with the combo's
//! `recorded_on`. Values only: the menu worker reads the facts and makes the requests (`runtime::ui::combos`).

use crate::engine::macro_ui::macro_panel::{fighter_gate, FighterChoice, FighterGate};
use crate::engine::swap::trio::{TrioFighter, CHOICES};

/// Why a combo recorded on another fighter is kept on P1's fighter without asking (`tr_combo_fighter_v1` detail).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// *Ask before switching fighter* is Off: as for a named macro, the current fighter is kept without asking.
    SettingOff = 1,
    /// `recorded_on` is not a fighter this build can load.
    Unknown = 2,
}

/// What choosing a combo does about P1's fighter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// Nothing to choose: no recorded fighter, the same fighter, or P1's fighter is not known yet.
    Proceed,
    /// Practise on P1's fighter without asking.
    Keep(Kept),
    /// Ask: keep P1's fighter, or switch P1 to this one.
    Ask(TrioFighter),
}

/// `tr_combo_fighter_v1` events (x0).
pub mod event {
    pub const PROCEED: u32 = 1;
    pub const KEPT: u32 = 2;
    pub const ASKED: u32 = 3;
    pub const KEEP: u32 = 4;
    pub const SWITCH: u32 = 5;
    pub const CANCELLED: u32 = 6;
    pub const REQUESTED: u32 = 7;
}

/// The roster fighter `name` names (the names `recorded_on` is written with), ignoring case and surrounding spaces.
pub fn fighter_named(name: &str) -> Option<TrioFighter> {
    let name = name.trim();
    CHOICES.iter().find(|(_, known)| known.eq_ignore_ascii_case(name)).and_then(|(kind, _)| TrioFighter::from_kind(*kind))
}

/// The plan for a combo recorded on `recorded_on` while P1 is `p1` (its kind and its name as the list writes it).
/// `prompt_enabled`: *Ask before switching fighter*. Player Swapping plays no part: a Switch is one explicit load the
/// user chose (`character_roster::request_player_kind`), which that setting does not refuse.
pub fn plan(recorded_on: Option<&str>, p1: Option<(u32, &str)>, prompt_enabled: bool) -> Plan {
    let (Some(recorded), Some((kind, label))) = (recorded_on, p1) else { return Plan::Proceed };
    let fighter = fighter_named(recorded);
    let matches = match fighter {
        Some(fighter) => fighter.kind() == kind,
        // A fighter the roster does not list is still the same one when the names agree.
        None => recorded.trim().eq_ignore_ascii_case(label),
    };
    match (fighter_gate(prompt_enabled, fighter.is_some(), matches), fighter) {
        (FighterGate::Proceed, _) => Plan::Proceed,
        (FighterGate::Prompt, Some(fighter)) => Plan::Ask(fighter),
        (_, None) => Plan::Keep(Kept::Unknown),
        (_, Some(_)) => Plan::Keep(Kept::SettingOff),
    }
}

/// What the target notice adds for a combo kept on P1's fighter without asking (`None`: nothing, as for a named macro
/// with the setting Off).
pub fn kept_note(kept: Kept, recorded: &str, p1: &str) -> Option<String> {
    match kept {
        Kept::SettingOff => None,
        Kept::Unknown => Some(format!("{} is not a fighter this build can load (P1 stays {p1})", recorded.trim())),
    }
}

/// What the target notice adds for the answer to the question.
pub fn answer_note(choice: FighterChoice, fighter: TrioFighter, p1: &str) -> String {
    match choice {
        FighterChoice::Keep => format!("practising on {p1}"),
        FighterChoice::Switch => format!("P1 switches to {} when the menu closes", fighter.name()),
    }
}

