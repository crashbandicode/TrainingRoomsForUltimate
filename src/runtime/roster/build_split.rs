//! Diag meter of the group build's game-thread time (stream f6, ledger F6): the runtime half of `model::build_split`.
//!
//! The carried `native_body_bank` calls these at its native calls (recorded manifest edits). Only the `diag` build records and logs;
//! the release build compiles every call to nothing. Everything runs on the game thread inside the entry pre-service: no heap (fixed
//! statics, `trace` lines formatted into package 1's stack line), one short mutex per call, never held across a native call.
#[cfg(feature = "diag")]
mod meter {
    use crate::model::build_split::{Gaps, Split, Step, Work};
    use std::sync::atomic::{AtomicU32, Ordering};
    // Switch std::Mutex initializes a OnceBox on its first lock. These meters
    // first run inside hooks, so use the existing const, allocation-free lock.
    // Only game-thread entry service calls this module; no worker owns a meter.
    use hos_lock::Mutex;

    /// The span being split (one exchange or one settlement at a time: both run inside one game-thread call).
    static SPLIT: Mutex<Split> = Mutex::new(Split::IDLE);
    /// Per role (0 P1, 1 CPU): the gaps of the swap in flight.
    static GAPS: [Mutex<Gaps>; 2] = [Mutex::new(Gaps::IDLE), Mutex::new(Gaps::IDLE)];
    /// Asynchronous builds queued since boot (the first one is the first-use build the ledger row is about).
    static BUILDS: AtomicU32 = AtomicU32::new(0);
    static SETTLES: AtomicU32 = AtomicU32::new(0);

    fn now() -> u64 {
        crate::engine::preload::preload_metrics::micros(
            crate::engine::preload::preload_metrics::ticks(),
            crate::engine::preload::preload_metrics::tick_hz(),
        )
        .unwrap_or(0)
    }

    pub fn start(role: usize, started_us: u64) {
        *SPLIT.lock() = Split::start(started_us);
        note(role, Work::Setup);
    }

    pub fn mark(step: Step) {
        let t = now();
        SPLIT.lock().mark(step, t);
    }

    pub fn report_setup(owner: u32, group: usize, kinds: &[u32], setup_us: u64) {
        mark(Step::Log);
        let n = BUILDS.fetch_add(1, Ordering::Relaxed) + 1;
        let split = {
            let mut s = SPLIT.lock();
            s.stop();
            *s
        };
        crate::engine::compat::logging::diagnostic(format_args!(
            "[BuildSplit] setup owner={} group={} kinds={:x?} build_of_boot={} carried_setup_us={} {}",
            owner, group, kinds, n, setup_us, split
        ));
    }

    pub fn start_settle(role: usize) {
        let t = now();
        *SPLIT.lock() = Split::start(t);
        note(role, Work::Settle);
    }

    pub fn report_settle(owner: u32, group: usize) {
        let n = SETTLES.fetch_add(1, Ordering::Relaxed) + 1;
        let split = {
            let mut s = SPLIT.lock();
            s.stop();
            *s
        };
        crate::engine::compat::logging::diagnostic(format_args!(
            "[BuildSplit] settle owner={} group={} settle_of_boot={} {}",
            owner, group, n, split
        ));
    }

    static CLOCKS: [Mutex<crate::model::build_split::TickClock>; 2] =
        [Mutex::new(crate::model::build_split::TickClock::IDLE), Mutex::new(crate::model::build_split::TickClock::IDLE)];

    /// Every entry pre-service tick, whatever the plugin does: a gap that jumps by more than `STALL_US` past the previous
    /// gap is logged with the entry's loader state. A steady slow frame rate does not log. A stall the game makes itself
    /// (a native Reload building a fighter) is measured the same way as the plugin's builds.
    pub fn tick_any(role: usize, owner: u32, now_us: u64, e: &crate::engine::swap::character_forms::EntryState) {
        let gap = CLOCKS.get(role).and_then(|m| m.lock().tick(now_us, crate::model::build_split::STALL_US));
        if let Some(gap) = gap {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[BuildSplit] stall owner={} gap_us={} loader={} outer={} installed={} changing={} count={} kinds={:x?} flags={:#x}",
                owner, gap, e.loader, e.outer, e.installed, e.changing, e.count, &e.kinds[..(e.count as usize).min(3)], e.flags
            ));
        }
    }

    /// Native state 10's tick (F6): the bodies allocated and queued once the control init settled on the worker.
    pub fn start_queue(role: usize) {
        let t = now();
        *SPLIT.lock() = Split::start(t);
        note(role, Work::Queue);
    }

    pub fn report_queue(owner: u32, group: usize) {
        let split = {
            let mut s = SPLIT.lock();
            s.stop();
            *s
        };
        crate::engine::compat::logging::diagnostic(format_args!("[BuildSplit] queue owner={} group={} {}", owner, group, split));
    }

    pub fn arm(role: usize, now_us: u64) {
        if let Some(mut g) = GAPS.get(role).map(|m| m.lock()) {
            *g = Gaps::arm(now_us);
        }
    }

    pub fn observe(role: usize, now_us: u64) {
        if let Some(mut g) = GAPS.get(role).map(|m| m.lock()) {
            g.observe(now_us);
        }
    }

    fn note(role: usize, work: Work) {
        if let Some(mut g) = GAPS.get(role).map(|m| m.lock()) {
            g.note(work);
        }
    }

    pub fn report_gaps(role: usize, owner: u32, kind: u32) {
        let gaps = GAPS.get(role).map(|m| m.lock()).map(|mut g| {
            let copy = *g;
            g.disarm();
            copy
        });
        if let Some(g) = gaps.filter(|g| g.armed()) {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[BuildSplit] gaps owner={} kind={:#x} {}",
                owner, kind, g
            ));
        }
    }
}

#[cfg(not(feature = "diag"))]
mod meter {
    use crate::model::build_split::Step;

    #[inline(always)]
    pub fn start(_role: usize, _started_us: u64) {}
    #[inline(always)]
    pub fn mark(_step: Step) {}
    #[inline(always)]
    pub fn report_setup(_owner: u32, _group: usize, _kinds: &[u32], _setup_us: u64) {}
    #[inline(always)]
    pub fn start_settle(_role: usize) {}
    #[inline(always)]
    pub fn report_settle(_owner: u32, _group: usize) {}
    #[inline(always)]
    pub fn tick_any(_role: usize, _owner: u32, _now_us: u64, _e: &crate::engine::swap::character_forms::EntryState) {}
    #[inline(always)]
    pub fn start_queue(_role: usize) {}
    #[inline(always)]
    pub fn report_queue(_owner: u32, _group: usize) {}
    #[inline(always)]
    pub fn arm(_role: usize, _now_us: u64) {}
    #[inline(always)]
    pub fn observe(_role: usize, _now_us: u64) {}
    #[inline(always)]
    pub fn report_gaps(_role: usize, _owner: u32, _kind: u32) {}
}

pub(crate) use crate::model::build_split::Step;
pub(crate) use meter::*;
