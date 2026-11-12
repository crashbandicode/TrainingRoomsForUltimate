//! GS-PANIC: the one process-memory contract every fighter load the mod starts goes through.
//!
//! The game aborts when a page commit is refused (`nn::os::AllocateMemoryPages` at `main+0x37ca664`, `abort()` at
//! `0x37ca688`): s19/s21 in a group build, s30 in a helper source's speculative Reload. The rule and the measured
//! costs are `crate::engine::swap::body_bank::memory`; this module joins what each consumer holds:
//!
//! | Consumer | Admission | Held while |
//! | --- | --- | --- |
//! | group prefetch (`body_bank`, `Step::Start`) | discretionary: Admit / Defer / Refuse (group fails, canonical) | prefetching: all; prefetched: its build |
//! | helper source load (`sources::service`), speculative | discretionary: refused = stays planned, retried | in flight: all |
//! | helper owner creation for a speculative target (`sources::create_capacity`) | with the load it hosts, beside what the cycle waits for | (that load) |
//! | speculative load on a created entry (`sources::service`) | discretionary, beside what the cycle waits for ([`admit_created`]) | in flight: all |
//! | helper source load for a visible request (demand) | not refused here | in flight: all |
//! | a switch's new window, its supplemental members (`body_bank::proposed`) | the planner's prefix that fits ([`admit_window`]); the target is never refused | built under the switch's INSTALL |
//! | Kirby copy packages (`swap_native::copy_packages`) | inside the admitted group (first tick of its prefetch) | the group's |
//!
//! Never call with a bank lock or the source lock held: [`outstanding`] takes each in turn.

use crate::engine::preload::preload_metrics::MemorySample;
use crate::engine::swap::body_bank::memory::{self, Decision, GroupCost};

/// What every admitted consumer still holds: both roles' groups and every helper load in flight.
pub(crate) fn outstanding() -> u64 {
    let groups = super::body_bank::group_outstanding();
    groups.saturating_add(super::sources::helper_outstanding())
}

/// Diag build only: process memory an Eden row holds back (`diag-memory-reserve.req`, bytes), so the discretionary
/// admissions refuse as on a console with less headroom (F9-SEAT-NOTICE's row). 0, the default, changes nothing.
#[cfg(feature = "diag")]
pub(crate) static DIAG_RESERVE: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// The headroom an admission reads from `sample` (less [`DIAG_RESERVE`] in the diag build).
fn headroom(sample: &MemorySample) -> Option<u64> {
    #[cfg(feature = "diag")]
    return sample.headroom().map(|headroom| {
        headroom.saturating_sub(DIAG_RESERVE.load(core::sync::atomic::Ordering::Acquire))
    });
    #[cfg(not(feature = "diag"))]
    sample.headroom()
}

/// One discretionary admission against the current headroom less [`outstanding`], with the sample it used.
pub(crate) fn admit(cost: GroupCost) -> (Decision, MemorySample) {
    let sample = MemorySample::capture();
    (memory::admit(headroom(&sample), outstanding(), cost), sample)
}

/// rc.6 window admission (hs08b): how many of a switch's new window's supplemental members, in the planner's order, the
/// process holds now (`memory::window_extras`), and the decision that stopped the count. Called where the switch plans
/// the window (`body_bank::proposed`, the request's preflight), never in a hook.
pub(crate) fn admit_window(supplements: &[u32]) -> (usize, Option<Decision>) {
    let sample = MemorySample::capture();
    let headroom = headroom(&sample);
    memory::window_extras(headroom, outstanding(), supplements)
}

/// What the cycle waits for: the supplements its windows were refused and its groups held for memory.
pub(crate) fn cycle_waiting() -> u64 {
    super::body_bank::cycle_waiting()
}

/// A created entry's creation or speculative load: [`admit`] that also leaves [`cycle_waiting`]
/// (`memory::created_admission`).
pub(crate) fn admit_created(cost: GroupCost) -> (Decision, MemorySample) {
    let sample = MemorySample::capture();
    let headroom = headroom(&sample);
    (memory::created_admission(headroom, outstanding(), cycle_waiting(), cost), sample)
}
