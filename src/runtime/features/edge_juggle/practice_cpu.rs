//! Temporary practice CPU selection, serviced by the game frame's roster owner.
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::model::edge_juggle::picker;
use crate::runtime::{roster::character_roster as roster, scene};

const NONE: u32 = u32::MAX;
static ORIGINAL: AtomicU32 = AtomicU32::new(NONE);
static TEMPORARY: AtomicU32 = AtomicU32::new(NONE);
static SWITCHING: AtomicBool = AtomicBool::new(false);
/// The selection that has been live in the slot since it became the selection (model picker::Settled); NONE: none.
static SETTLED: AtomicU32 = AtomicU32::new(NONE);

fn settled() -> picker::Settled {
    let kind = SETTLED.load(Ordering::Acquire);
    picker::Settled::new((kind != NONE).then_some(kind))
}

fn see(target: Option<u32>, live: Option<u32>) -> picker::Settled {
    let next = settled().see(target, live);
    SETTLED.store(next.kind().unwrap_or(NONE), Ordering::Release);
    next
}

/// The CPU's fighter entry was destroyed (leaving practice, or changing fighters): the
/// settlement ends with it. Generic session clears and Player 1 reloads do not come here.
pub(in crate::runtime) fn cpu_entry_departed() {
    SETTLED.store(NONE, Ordering::Release);
}

/// The picker gate's question for the live CPU kind; also notes the selection as settled when it is live.
pub(super) fn answers(wanted: u32, live: u32) -> bool {
    picker::answers(wanted, live, see(Some(wanted), Some(live)).is(wanted))
}

pub(in crate::runtime) fn temporary(kind: u32) -> bool {
    TEMPORARY.load(Ordering::Acquire) == kind
}

pub(super) fn switching() -> bool {
    SWITCHING.load(Ordering::Acquire)
}

pub(in crate::runtime) fn restoring() -> bool {
    ORIGINAL.load(Ordering::Acquire) != NONE
}

/// Only the game frame calls the roster. The picker publishes its selection through the active drill.
pub(in crate::runtime) fn service_practice_cpu_selection() {
    let original = ORIGINAL.load(Ordering::Relaxed);
    let original = (original != NONE).then_some(original);
    let selected = super::cpu_selection();
    let (next, tick) = picker::tick(settled(), selected, original, scene::practice_scene().is_some(),
        || super::super::practice::active().is_some() && !super::running(),
        || (!roster::busy() && roster::cpu_reload_ready()).then(roster::cpu_kind).flatten(),
    );
    SETTLED.store(next.kind().unwrap_or(NONE), Ordering::Release);
    match tick {
        picker::Tick::Idle => return,
        picker::Tick::Left => {
            ORIGINAL.store(NONE, Ordering::Relaxed);
            TEMPORARY.store(NONE, Ordering::Release);
            SWITCHING.store(false, Ordering::Release);
        }
        picker::Tick::OtherGame => SWITCHING.store(false, Ordering::Release),
        picker::Tick::Serve { target, observed, step } => {
            SWITCHING.store(step.switching, Ordering::Release);
            if step.request {
                if let Some(observed) = observed {
                    let previous = TEMPORARY.swap(target, Ordering::AcqRel);
                    if roster::request_practice_cpu_kind(target) {
                        if selected.is_some() && original.is_none() {
                            ORIGINAL.store(observed, Ordering::Relaxed);
                        }
                    } else {
                        TEMPORARY.store(previous, Ordering::Release);
                    }
                }
            } else if observed.is_some() && selected.is_none() {
                ORIGINAL.store(NONE, Ordering::Relaxed);
                TEMPORARY.store(NONE, Ordering::Release);
            }
        }
    }
}
