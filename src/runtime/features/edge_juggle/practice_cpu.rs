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

/// Only the game frame calls the roster. The picker publishes its selection through the active drill.
pub(in crate::runtime) fn service_practice_cpu_selection() {
    let original = ORIGINAL.load(Ordering::Relaxed);
    let selected = super::cpu_selection();
    let Some(target) = selected.or(Some(original).filter(|kind| *kind != NONE)) else {
        see(None, None);
        return;
    };
    if !scene::is_native_training() {
        see(None, None);
        ORIGINAL.store(NONE, Ordering::Relaxed);
        TEMPORARY.store(NONE, Ordering::Release);
        SWITCHING.store(false, Ordering::Release);
        return;
    }
    if selected.is_none() && super::super::practice::active().is_some() && !super::running() {
        // Another game owns the opponent until it stops.
        SWITCHING.store(false, Ordering::Release);
        return;
    }
    let ready = !roster::busy() && roster::cpu_reload_ready();
    let observed = ready.then(roster::cpu_kind).flatten();
    let step = picker::service(selected, target, observed, see(Some(target), observed).is(target));
    SWITCHING.store(step.switching, Ordering::Release);
    if step.request {
        if let Some(observed) = observed {
            let previous = TEMPORARY.swap(target, Ordering::AcqRel);
            if roster::request_practice_cpu_kind(target) {
                if selected.is_some() && original == NONE {
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
