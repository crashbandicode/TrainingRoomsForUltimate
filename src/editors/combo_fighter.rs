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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const MARIO: u32 = 0x00;
    const RYU: u32 = 0x3c;

    fn no_question_without_a_recorded_fighter_or_when_it_is_p1s() {
        assert_eq!(plan(None, Some((MARIO, "Mario")), true), Plan::Proceed);
        assert_eq!(plan(Some("Mario"), Some((MARIO, "Mario")), true), Plan::Proceed);
        assert_eq!(plan(Some("  mario "), Some((MARIO, "Mario")), true), Plan::Proceed, "case and spaces");
        // P1 not known yet (no practice entry observed): never a question.
        assert_eq!(plan(Some("Ryu"), None, true), Plan::Proceed);
    }

    fn another_fighter_is_asked_about() {
        // Player Swapping is not an input: a Switch is one explicit load, admitted with it Off (report §2a).
        assert_eq!(plan(Some("Ryu"), Some((MARIO, "Mario")), true), Plan::Ask(TrioFighter::from_kind(RYU).unwrap()));
        assert_eq!(plan(Some("RYU"), Some((MARIO, "Mario")), true), Plan::Ask(TrioFighter::from_kind(RYU).unwrap()));
    }

    fn it_is_kept_without_asking_as_a_named_macro_would_be() {
        // The setting Off: keep, silently.
        assert_eq!(plan(Some("Ryu"), Some((MARIO, "Mario")), false), Plan::Keep(Kept::SettingOff));
        assert_eq!(kept_note(Kept::SettingOff, "Ryu", "Mario"), None);
        // A name no roster fighter has: nothing to switch to, whatever the setting.
        assert_eq!(plan(Some("Ryuu"), Some((MARIO, "Mario")), true), Plan::Keep(Kept::Unknown));
        assert_eq!(plan(Some("Ryuu"), Some((MARIO, "Mario")), false), Plan::Keep(Kept::Unknown));
        assert_eq!(kept_note(Kept::Unknown, " Ryuu ", "Mario").unwrap(), "Ryuu is not a fighter this build can load (P1 stays Mario)");
        // ...unless it is P1's own name (a fighter the roster does not list, written by its label).
        assert_eq!(plan(Some("0x48"), Some((0x48, "0x48")), true), Plan::Proceed);
    }

    /// The shared Fighter Mismatch screen with the `Combo` origin: the same rows and keys, Player 1 in the footer, and
    /// the answer reported to the combos UI (`Effect::ComboFighter`), never applied to a named macro.
    fn the_shared_prompt_asks_about_player_1_and_reports_to_the_combos_ui() {
        use crate::engine::macro_ui::macro_panel::{Effect, FighterOrigin, MacroUi, Pad};
        use crate::model::recording_menu::ViewRole;
        let mut ui = MacroUi::new();
        assert!(!ui.open_fighter_prompt_for(FighterOrigin::Named, ViewRole::Cpu, 1, "Ryu".into(), "Mario".into()), "Named keeps its own path");
        assert!(ui.open_fighter_prompt_for(FighterOrigin::Combo, ViewRole::Player, 4, "Ryu".into(), "Mario".into()));
        assert_eq!(ui.fighter_prompt(), Some((FighterOrigin::Combo, 4)));
        assert!(!ui.open_fighter_prompt(5, "Fox".into(), "Hero".into()), "one prompt at a time");
        let view = ui.view().expect("the prompt is on screen");
        assert_eq!(view.title, "Fighter Mismatch");
        assert_eq!((view.labels[0].as_str(), view.labels[1].as_str(), view.selected), ("Keep current fighter", "Switch to recorded fighter", 0));
        assert!(view.footer.starts_with("Recorded on Ryu · P1 is Mario\n"), "{}", view.footer);
        assert!(!view.footer.contains("CPU") && view.footer.contains("A chooses") && view.footer.contains("B cancels"));
        assert!(matches!(ui.on_pad(Pad::Back), Effect::ComboFighter(None)));
        assert!(!ui.is_open());
        ui.open_fighter_prompt_for(FighterOrigin::Combo, ViewRole::Player, 4, "Ryu".into(), "Mario".into());
        assert!(matches!(ui.on_pad(Pad::Confirm), Effect::ComboFighter(Some(FighterChoice::Keep))), "Keep is the default");
        ui.open_fighter_prompt_for(FighterOrigin::Combo, ViewRole::Player, 4, "Ryu".into(), "Mario".into());
        ui.on_pad(Pad::Down);
        assert!(matches!(ui.on_pad(Pad::Confirm), Effect::ComboFighter(Some(FighterChoice::Switch))));
        assert_eq!(ui.loaded_id(), None, "no named macro was loaded");
        // The replay window's prompt still reports to the replay window.
        ui.open_fighter_prompt(3, "Kirby".into(), "Hero".into());
        assert!(matches!(ui.on_pad(Pad::Back), Effect::ClipFighter(None)));
    }

    fn every_roster_name_is_found_and_the_answers_read_well() {
        for (kind, name) in CHOICES {
            assert_eq!(fighter_named(name).map(TrioFighter::kind), Some(*kind), "{name}");
        }
        let ryu = fighter_named("Ryu").unwrap();
        assert_eq!(answer_note(FighterChoice::Keep, ryu, "Mario"), "practising on Mario");
        assert_eq!(answer_note(FighterChoice::Switch, ryu, "Mario"), "P1 switches to Ryu when the menu closes");
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("no_question_without_a_recorded_fighter_or_when_it_is_p1s", no_question_without_a_recorded_fighter_or_when_it_is_p1s as fn()),
            ("another_fighter_is_asked_about", another_fighter_is_asked_about as fn()),
            ("it_is_kept_without_asking_as_a_named_macro_would_be", it_is_kept_without_asking_as_a_named_macro_would_be as fn()),
            ("the_shared_prompt_asks_about_player_1_and_reports_to_the_combos_ui", the_shared_prompt_asks_about_player_1_and_reports_to_the_combos_ui as fn()),
            ("every_roster_name_is_found_and_the_answers_read_well", every_roster_name_is_found_and_the_answers_read_well as fn()),
        ]);
    }
    }
}
