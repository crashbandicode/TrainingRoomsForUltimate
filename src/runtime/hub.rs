//! The Training reset hub (SPEC-state-06 §5.2): clears every per-session plugin state when the game
//! resets Training. Called by the fighter on-init wrapper, and by the save-state load on every
//! PosMove frame (package 3, SPEC-state-06 §5.3). Each package adds its own step here; the order is
//! `lra_reset::HUB_ORDER`.

use crate::model::input_delay::Clear;
use crate::model::lra_reset::{self, HubStep};
use crate::runtime::features::{cpu, input_delay};

pub fn run(cause: Clear) {
    // How many queued input-delay records the hub dropped.
    let mut dropped = 0;
    lra_reset::run_hub(|step| {
        // The CPU behaviours clear their own state for every step they own (mash, SDI, counters,
        // ledge); steps whose features are not built yet (combo, throw, shield tilt) have nothing to
        // clear.
        cpu::hub_step(step);
        if step == HubStep::InputDelay {
            dropped = input_delay::clear(cause);
        }
    });
    report(cause, dropped);
}

/// The diag build writes one line per hub run (decision D4: the tester can prove the B8 clear); the
/// release build writes nothing.
fn report(cause: Clear, dropped: usize) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[TrainingRooms] hub_run cause={cause:?} input_delay_cleared={dropped}"
    ));
    #[cfg(not(feature = "diag"))]
    let _ = (cause, dropped);
}
