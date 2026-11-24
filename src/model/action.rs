//! An action starts at an originating input and owns its later events.
//! Its model is described in `docs/design/action-model.md`.

use super::combo::Kind;

/// Whether this event can start a new action.
pub const fn originates(kind: Kind) -> bool {
    matches!(kind, Kind::Press(_) | Kind::CStick | Kind::Flick(_))
}

/// Extra tolerance for a hold, capped before the next idle-separated step.
pub const fn hold_band(hold: u32) -> u8 {
    if hold / 8 > 15 { 15 } else { (hold / 8) as u8 }
}
