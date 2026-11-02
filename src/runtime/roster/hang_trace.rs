//! Optional hang tracing plus the always-on `practice-heartbeat` worker: layout archive retirement
//! every 5 s and, every 50 ms, the late seam report (`seam_install::report_late`, R-A0b) and the
//! drain of the diagnostics ring (`logging::drain`, WP-D) — the one thread that writes
//! `practice-diagnostics.log`, off the game threads.
//!
//! The default build leaves hook probes and trace line budgets inactive. Enable the
//! `hang-trace` feature (part of `diag`) to record hook counts/times and emit heartbeat summaries.
//! Worker-spawn failures remain reported even when tracing is disabled; without the worker,
//! `logging::diagnostic` keeps writing synchronously.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// Diagnostics drain period: lines reach the card at most this long after they are logged.
const DRAIN_INTERVAL: Duration = Duration::from_millis(50);
const TRACE_ENABLED: bool = cfg!(feature = "hang-trace");

/// One instrumented hook: call count, time of the last call and, between `enter` and the guard's
/// drop, the time the current call started plus one caller-chosen detail word.
pub struct Probe {
    name: &'static str,
    count: AtomicU64,
    last_ms: AtomicU64,
    active_since_ms: AtomicU64,
    detail: AtomicU64,
}

impl Probe {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            count: AtomicU64::new(0),
            last_ms: AtomicU64::new(0),
            active_since_ms: AtomicU64::new(0),
            detail: AtomicU64::new(0),
        }
    }

    /// Record a call only in an opt-in diagnostic build.
    #[inline]
    pub fn hit(&self) {
        if TRACE_ENABLED {
            self.count.fetch_add(1, Ordering::Relaxed);
            self.last_ms.store(now_ms(), Ordering::Relaxed);
        }
    }

    /// Record one call and mark the probe active until the returned guard drops.
    #[inline]
    pub fn enter(&self, detail: u64) -> ProbeGuard<'_> {
        if TRACE_ENABLED {
            self.hit();
            self.detail.store(detail, Ordering::Relaxed);
            self.active_since_ms.store(now_ms().max(1), Ordering::Release);
        }
        ProbeGuard { probe: self }
    }

    fn summary(&self, now: u64) -> String {
        let count = self.count.load(Ordering::Relaxed);
        let last = self.last_ms.load(Ordering::Relaxed);
        let active = self.active_since_ms.load(Ordering::Acquire);
        let age = if last == 0 {
            String::from("never")
        } else {
            format!("{}ms", now.saturating_sub(last))
        };
        if active == 0 {
            format!("{}={}/{}", self.name, count, age)
        } else {
            format!(
                "{}={}/{} ACTIVE {}ms detail={:#x}",
                self.name,
                count,
                age,
                now.saturating_sub(active),
                self.detail.load(Ordering::Relaxed)
            )
        }
    }
}

pub struct ProbeGuard<'a> {
    probe: &'a Probe,
}

impl Drop for ProbeGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        if TRACE_ENABLED {
            self.probe.active_since_ms.store(0, Ordering::Release);
        }
    }
}

/// Per-frame UI pass anchor (UI job thread).
pub static UI_PASS: Probe = Probe::new("ui_pass");
/// Manager-insert anchor (main thread), one call per newly registered layout.
pub static UI_REGISTER: Probe = Probe::new("ui_register");
/// The modpack's `ui::tick` body entered from the per-frame pass.
pub static TICK_PASS: Probe = Probe::new("tick_pass");
/// The modpack's `ui::tick` body entered from a layout registration.
pub static TICK_REGISTER: Probe = Probe::new("tick_register");
/// Fighter entry service callback (game thread, per entry per frame).
pub static ROSTER_SERVICE: Probe = Probe::new("roster_service");
/// Fighter entry allocation, reload and destruction boundaries.
pub static ALLOCATION: Probe = Probe::new("alloc");
pub static RELOAD: Probe = Probe::new("reload");
pub static DESTROY: Probe = Probe::new("destroy");
/// Layout archive decompression hook (the Training and waiting UI injection point).
pub static LAYOUT_ARC: Probe = Probe::new("layout_arc");
/// Skyline NRO load callback (fighter and common modules load here during a scene load).
pub static NRO_LOAD: Probe = Probe::new("nro_load");

static PROBES: [&Probe; 10] = [
    &UI_PASS,
    &UI_REGISTER,
    &TICK_PASS,
    &TICK_REGISTER,
    &ROSTER_SERVICE,
    &ALLOCATION,
    &RELOAD,
    &DESTROY,
    &LAYOUT_ARC,
    &NRO_LOAD,
];

/// Bounded per-session line budgets for the chattier trace lines.
pub static REGISTER_LINES: AtomicU32 = AtomicU32::new(0);
pub static NRO_LINES: AtomicU32 = AtomicU32::new(0);

/// Disabled builds return false without touching the counter; diagnostic builds consume a unit.
#[inline]
pub fn within_budget(counter: &AtomicU32, limit: u32) -> bool {
    TRACE_ENABLED && counter.fetch_add(1, Ordering::Relaxed) < limit
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

