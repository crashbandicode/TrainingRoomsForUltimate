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

