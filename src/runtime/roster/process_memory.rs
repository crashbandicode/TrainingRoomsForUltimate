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
//! | helper owner creation for a speculative target (`sources::create_capacity`) | with the load it hosts | (that load) |
//! | helper source load for a visible request (demand) | not refused here | in flight: all |
//! | Kirby copy packages (`swap_native::copy_packages`) | inside the admitted group (first tick of its prefetch) | the group's |
//!
//! Never call with a bank lock or the source lock held: [`outstanding`] takes each in turn.

use crate::engine::preload::preload_metrics::MemorySample;
use crate::engine::swap::body_bank::memory::{self, Decision, GroupCost};

/// What every admitted consumer still holds: both roles' groups and every helper load in flight.
pub(crate) fn outstanding() -> u64 {
    #[cfg(feature = "native-body-bank-alpha")]
    let groups = super::body_bank::group_outstanding();
    #[cfg(not(feature = "native-body-bank-alpha"))]
    let groups = 0u64;
    groups.saturating_add(super::sources::helper_outstanding())
}

/// Diag build only: process memory an Eden row holds back (`diag-memory-reserve.req`, bytes), so the discretionary
/// admissions refuse as on a console with less headroom (F9-SEAT-NOTICE's row). 0, the default, changes nothing.
#[cfg(feature = "diag")]
pub(crate) static DIAG_RESERVE: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

/// One discretionary admission against the current headroom less [`outstanding`], with the sample it used.
pub(crate) fn admit(cost: GroupCost) -> (Decision, MemorySample) {
    let sample = MemorySample::capture();
    #[cfg(feature = "diag")]
    let headroom = sample.headroom().map(|headroom| {
        headroom.saturating_sub(DIAG_RESERVE.load(core::sync::atomic::Ordering::Acquire))
    });
    #[cfg(not(feature = "diag"))]
    let headroom = sample.headroom();
    (memory::admit(headroom, outstanding(), cost), sample)
}
