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
    use super::*;

    #[test]
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

    #[test]
    fn training_and_unrelated_scenes_keep_native_requests() {
        for (mode, melee) in [(0x04040000, 0x20), (0x03010101, 0x0d), (0, 0),
            (0x02020000, 0x0d), (0x07010300, 0x0d), (0x07010401, 0x0d), (0x07010400, 0x0e)] {
            assert_eq!(waiting_hazards(true, mode, melee, true, false), None);
            assert_eq!(waiting_hazards(true, mode, melee, true, true), None);
        }
    }
}
