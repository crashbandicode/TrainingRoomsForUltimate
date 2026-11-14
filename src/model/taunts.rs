//! Taunts (rc.13, `docs/design/rc13-taunts.md`): Player 1's D-pad taunts can be turned off in practice, pure half.
//!
//! The game opens a taunt only when `WorkModule::is_enable_transition_term` answers yes for one of the three appeal terms
//! (`FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_APPEAL_U`, `_S`, `_LW`). With the row Off, in Training or the waiting-room
//! practice, the runtime's transition-term hook answers no for Player 1's bodies. Kirby with a copy hat keeps Up, so he can
//! drop the hat. Nothing else is touched: the D-pad still reaches the plugin's chords and shortcuts.

use crate::model::cpu::ids::UNRESOLVED;

/// The appeals' bits in a denied mask, in the order of the terms (Up, Side, Down).
pub const UP: u8 = 1;
pub const SIDE: u8 = 2;
pub const DOWN: u8 = 4;

/// The appeals the game may not start for one of Player 1's bodies: none with the row On or outside practice (native
/// Training or the live waiting-room practice; never an online match), Side and Down for Kirby holding a copy hat, else all.
pub const fn denied_mask(on: bool, practice: bool, kirby_with_hat: bool) -> u8 {
    match (on || !practice, kirby_with_hat) {
        (true, _) => 0,
        (false, true) => SIDE | DOWN,
        (false, false) => UP | SIDE | DOWN,
    }
}

/// Whether the transition-term hook answers no to `term`: one of the resolved appeal `terms` (Up, Side, Down) whose bit is
/// in `mask`.
pub fn denies(terms: [i32; 3], mask: u8, term: i32) -> bool {
    term != UNRESOLVED && terms.iter().enumerate().any(|(i, t)| *t == term && mask & (1 << i) != 0)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::cpu::settings::CpuSettings;
    use crate::model::settings::{parse, Snapshot};

    const TERMS: [i32; 3] = [0x41, 0x42, 0x43];

    fn off_denies_every_appeal_in_practice() {
        let mask = denied_mask(false, true, false);
        assert_eq!(mask, UP | SIDE | DOWN);
        for term in TERMS {
            assert!(denies(TERMS, mask, term), "{term:#x}");
        }
    }

    fn kirby_with_a_hat_keeps_up_only() {
        let mask = denied_mask(false, true, true);
        assert!(!denies(TERMS, mask, TERMS[0]), "Up drops the hat");
        assert!(denies(TERMS, mask, TERMS[1]));
        assert!(denies(TERMS, mask, TERMS[2]));
    }

    fn kirby_without_a_hat_is_denied() {
        // Kirby's own taunts, no copy: the same as any fighter.
        assert!(denies(TERMS, denied_mask(false, true, false), TERMS[0]));
    }

    fn outside_practice_nothing_is_denied() {
        // An online match, a local match, the menus: the row does not matter, the hat does not matter.
        for on in [false, true] {
            for kirby_with_hat in [false, true] {
                assert_eq!(denied_mask(on, false, kirby_with_hat), 0, "on={on} hat={kirby_with_hat}");
            }
        }
    }

    fn on_allows_every_appeal() {
        for kirby_with_hat in [false, true] {
            let mask = denied_mask(true, true, kirby_with_hat);
            assert_eq!(mask, 0);
            assert!(TERMS.iter().all(|term| !denies(TERMS, mask, *term)));
        }
    }

    fn only_appeal_terms_are_denied() {
        let mask = denied_mask(false, true, false);
        for term in [0, 1, 0x40, 0x44, -1, i32::MAX] {
            assert!(!denies(TERMS, mask, term), "{term:#x}");
        }
        // An unresolved appeal term matches nothing, even an unresolved query.
        let unresolved = [UNRESOLVED; 3];
        assert!(!denies(unresolved, mask, UNRESOLVED));
        assert!(!denies(unresolved, mask, 0x41));
    }

    fn taunts_row_defaults_on_and_reads_off() {
        let taunts = |snapshot: &Snapshot| CpuSettings::from_snapshot(snapshot).taunts;
        let mut snapshot = Snapshot::factory();
        assert!(taunts(&snapshot));
        snapshot.set("taunts", &[0, 1]).expect("the taunts row exists");
        assert!(!taunts(&snapshot));
        snapshot.set("taunts", &[1, 0]).expect("the taunts row exists");
        assert!(taunts(&snapshot));
        // A settings file from before the row: On.
        assert!(taunts(&parse(r#"{"menu":{"lra_reset":[1,0]}}"#).settings.menu));
    }

    /// This module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("off_denies_every_appeal_in_practice", off_denies_every_appeal_in_practice as fn()),
            ("kirby_with_a_hat_keeps_up_only", kirby_with_a_hat_keeps_up_only as fn()),
            ("kirby_without_a_hat_is_denied", kirby_without_a_hat_is_denied as fn()),
            ("outside_practice_nothing_is_denied", outside_practice_nothing_is_denied as fn()),
            ("on_allows_every_appeal", on_allows_every_appeal as fn()),
            ("only_appeal_terms_are_denied", only_appeal_terms_are_denied as fn()),
            ("taunts_row_defaults_on_and_reads_off", taunts_row_defaults_on_and_reads_off as fn()),
        ]);
    }
    }
}
