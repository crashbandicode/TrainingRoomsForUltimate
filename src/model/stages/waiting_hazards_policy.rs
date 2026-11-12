//! Per-request waiting hazards, with no persistent global-rule ownership.

/// Apply only to the current stage request. Matching mode words cannot qualify
/// Quickplay or LDN matches: their melee byte differs from waiting practice.
pub fn waiting_hazards(initialized: bool, mode: u32, melee: u8,
    practice_enabled: bool, hazards_enabled: bool) -> Option<u8> {
    use crate::engine::mii::backend::Layout;
    if !practice_enabled { return None; }
    Layout::scene(initialized, melee, mode, true)
        .filter(|layout| layout.waiting())
        .map(|_| u8::from(hazards_enabled))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn waiting_families_accept_both_settings_but_not_their_matches() {
        for (mode, match_melee) in [(0x02010000, 1), (0x07010101, 0x0e), (0x07010102, 0x0e), (0x07010400, 0x10), (0x87010400, 0x10)] {
            for enabled in [false, true] {
                assert_eq!(waiting_hazards(true, mode, 0x0d, true, enabled), Some(u8::from(enabled)));
                assert_eq!(waiting_hazards(true, mode, match_melee, true, enabled), None);
                assert_eq!(waiting_hazards(false, mode, 0x0d, true, enabled), None);
                assert_eq!(waiting_hazards(true, mode, 0x0d, false, enabled), None);
            }
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("waiting_families_accept_both_settings_but_not_their_matches", waiting_families_accept_both_settings_but_not_their_matches as fn()),
        ]);
    }
    }
}
