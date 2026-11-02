//! The Training reset hub (SPEC-state-06 §5.2): clears every per-session plugin state when the game
//! resets Training. Called by the fighter on-init wrapper, and by the save-state load on every
//! PosMove frame (package 3, SPEC-state-06 §5.3). Each package adds its own step here; the order is
//! `lra_reset::HUB_ORDER`. NOT YET SWITCH-COMPILED.

use crate::model::input_delay::Clear;
use crate::model::lra_reset::{self, HubStep};
use crate::runtime::features::input_delay;

pub fn run(cause: Clear) {
    lra_reset::run_hub(|step| {
        // Steps whose features are not built yet (mash, SDI, counters, combo, ledge, throw, shield
        // tilt) have nothing to clear.
        if step == HubStep::InputDelay {
            input_delay::clear(cause);
        }
    });
}
