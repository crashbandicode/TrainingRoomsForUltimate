//! The diagnostics channel on the card (SPEC-core-01 §7): a size-capped ring, a drain that writes
//! whole lines to `practice-diagnostics.log` (two files of at most 128 KiB, rotated) and to the
//! kernel debug output, and the panic breadcrumb. The shipped build admits only the
//! `diag_ring::SHIPPED_TAGS`; a `diag` build writes every line. Formatting happens on the caller's
//! stack and the ring is a fixed array, so enqueueing allocates nothing. The normal diagnostic
//! API retains a synchronous filesystem drain before the writer starts; that drain may allocate.
//!
//! Lines 1-265 below the imports come from the earlier fork's `src/logging.rs` lines 20-284 (stage
//! m5o, fork-only, ours); CARRY-OVER.md lists the edits. The file is no longer held open: each drain
//! opens it in append mode, writes the lines and closes it, the way `events.log` is written (a failure
//! of either is written to `events.log` with its error text, so it cannot stay silent).

use std::cell::UnsafeCell;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use crate::model::diag_ring::{self, ByteRing, StackLine};
use crate::model::storage;
use crate::runtime::clock;

const DIAGNOSTIC_LIMIT: u64 = storage::DIAGNOSTIC_LIMIT;
/// Ring and per-line limits. The diag build keeps every oracle line whole (the longest seen in
/// Eden logs is about 8 KiB); the shipped build keeps sparse breadcrumbs, whole up to 9 KiB
/// (a Preload Samples snapshot reaches 8 KiB).
#[cfg(feature = "diag")]
const RING_BYTES: usize = 128 * 1024;
#[cfg(feature = "diag")]
const LINE_MAX: usize = 12 * 1024;
#[cfg(not(feature = "diag"))]
const RING_BYTES: usize = 32 * 1024;
#[cfg(not(feature = "diag"))]
const LINE_MAX: usize = 9 * 1024;
/// Spins before a busy ring drops the line (the holder copies at most `LINE_MAX` bytes).
const LOCK_SPINS: u32 = 4096;

struct DiagnosticLog {
    /// The current file, the previous one, and the file failures are reported to.
    current: String,
    previous: String,
    failures: String,
    bytes: u64,
    /// The file exists and can be appended to; false after a failure.
    ready: bool,
    initialized: bool,
}

static DIAGNOSTICS: Mutex<DiagnosticLog> = Mutex::new(DiagnosticLog {
    current: String::new(),
    previous: String::new(),
    failures: String::new(),
    bytes: 0,
    ready: false,
    initialized: false,
});
/// One spin-locked cell. The lock is only ever held for a copy into or out of the ring.
struct Locked<T> {
    busy: AtomicBool,
    cell: UnsafeCell<T>,
}

// SAFETY: every access to `cell` goes through `with`, which holds `busy`.
unsafe impl<T> Sync for Locked<T> {}

impl<T> Locked<T> {
    const fn new(value: T) -> Self {
        Self { busy: AtomicBool::new(false), cell: UnsafeCell::new(value) }
    }

    /// Run `f` under the lock, or return None after `LOCK_SPINS` failed attempts.
    fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        for spin in 0..LOCK_SPINS {
            if self
                .busy
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // SAFETY: `busy` is held until the store below.
                let result = f(unsafe { &mut *self.cell.get() });
                self.busy.store(false, Ordering::Release);
                return Some(result);
            }
            if spin % 256 == 255 {
                std::thread::yield_now();
            } else {
                std::hint::spin_loop();
            }
        }
        None
    }
}

static RING: Locked<ByteRing<RING_BYTES>> = Locked::new(ByteRing::new());
/// The drain's copy buffer; only the holder of `DRAIN` touches it.
static DRAIN: Locked<[u8; LINE_MAX]> = Locked::new([0; LINE_MAX]);
/// Lines lost because the ring lock stayed busy.
static LOCK_MISSES: AtomicU64 = AtomicU64::new(0);
/// Set once `start_drain_thread` runs the drain loop; before that callers write synchronously.
static WRITER: AtomicBool = AtomicBool::new(false);
/// Drops already reported in the log, and the report in progress.
static REPORTED_DROPS: AtomicU64 = AtomicU64::new(0);
static REPORTING: AtomicBool = AtomicBool::new(false);

/// Write one failure line to the failures file (open, append, close). Used where the diagnostics file
/// itself cannot be trusted; never panics.
fn report_failure(failures: &str, what: &str, error: &io::Error) {
    if failures.is_empty() {
        return;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(failures) {
        let _ = writeln!(file, "{} diag_log {what}: {error}", clock::uptime_ms());
    }
}

/// Start a new current file: the old one becomes the previous one, and an empty current file is
/// created.
fn rotate_diagnostics(log: &mut DiagnosticLog) -> io::Result<()> {
    if fs::metadata(&log.current).is_ok() {
        match fs::remove_file(&log.previous) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        fs::rename(&log.current, &log.previous)?;
    }
    OpenOptions::new().create(true).append(true).open(&log.current)?;
    log.bytes = 0;
    Ok(())
}

/// Use these files from now on (rotating the current one) and report a failure to `failures`.
/// `init_diagnostics` passes the files on the card; the host test passes temporary ones.
pub fn init_at(current: &str, previous: &str, failures: &str) {
    let Ok(mut log) = DIAGNOSTICS.lock() else {
        return;
    };
    log.current = current.to_owned();
    log.previous = previous.to_owned();
    log.failures = failures.to_owned();
    log.initialized = true;
    log.ready = match rotate_diagnostics(&mut log) {
        Ok(()) => true,
        Err(e) => {
            report_failure(failures, "start", &e);
            false
        }
    };
}

/// Sparse, persistent breadcrumbs for console crash investigation. At most two
/// 128 KiB files; no per-frame writes. Failures never abort game initialization.
pub fn init_diagnostics() {
    if DIAGNOSTICS.lock().is_ok_and(|log| log.initialized) {
        return;
    }
    if let Err(e) = fs::create_dir_all(storage::ROOT) {
        report_failure(storage::EVENTS_LOG_PATH, "data folder", &e);
    }
    init_at(storage::DIAGNOSTICS_PATH, storage::DIAGNOSTICS_PREVIOUS_PATH, storage::EVENTS_LOG_PATH);
    // The game identity is not known yet here: the version guard logs it next, and the install
    // line carries it.
    diagnostic(format_args!(
        "[PracticeDiagnostics] adapter=ring{} session_start; max_files=2 max_bytes_each={} ring_bytes={} line_max={}",
        if cfg!(feature = "diag") { "-diag" } else { "" },
        DIAGNOSTIC_LIMIT,
        RING_BYTES,
        LINE_MAX
    ));
}
/// Record one diagnostic line; before the writer starts, drain synchronously.
pub fn diagnostic(args: fmt::Arguments<'_>) {
    if enqueue(args) && !WRITER.load(Ordering::Acquire) {
        drain();
    }
}

/// Queue a diagnostic without invoking the caller-side synchronous drain.
/// The existing bounded ring owns the formatted bytes and reports drops.
#[cfg(any(feature = "diag", test))]
pub(crate) fn enqueue_diagnostic(args: fmt::Arguments<'_>) {
    let _ = enqueue(args);
}

/// True means the line passed the shipped-tag filter, not guaranteed delivery.
fn enqueue(args: fmt::Arguments<'_>) -> bool {
    let mut line = StackLine::new();
    let whole = line.format(args);
    if !cfg!(feature = "diag") && !diag_ring::shipped(line.text()) {
        return false;
    }
    let ts = clock::uptime_ms();
    let text = whole.then_some(line.text());
    if RING.with(|ring| ring.push(ts, text, args, LINE_MAX)).is_none() {
        LOCK_MISSES.fetch_add(1, Ordering::Relaxed);
    }
    true
}


/// Ring counters plus lock misses, for the install line and drop reports (`ring_dropped=…`).
#[derive(Clone, Copy, Debug)]
pub struct RingStats(pub diag_ring::Stats);

impl fmt::Display for RingStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ring_dropped={} ring_truncated={} ring_lines={}",
            self.0.dropped, self.0.truncated, self.0.committed
        )
    }
}

pub fn ring_stats() -> RingStats {
    let mut stats = RING.with(|ring| ring.stats()).unwrap_or_default();
    stats.dropped += LOCK_MISSES.load(Ordering::Relaxed);
    RingStats(stats)
}

/// Called once by the drain thread when its loop starts: callers stop writing synchronously.
pub fn writer_started() {
    WRITER.store(true, Ordering::Release);
}

/// Diagnostic observers must never take the pre-writer synchronous filesystem path.
/// WRITER is monotonic, so a true observation remains true through `diagnostic`.
#[cfg(feature = "diag")]
pub(crate) fn async_writer_ready() -> bool { WRITER.load(Ordering::Acquire) }

/// Write everything buffered (version guard and panic hook use it before they return).
pub fn flush() {
    drain();
}

/// Move buffered lines to the card and the kernel debug output. Called every 50 ms by the drain
/// thread, and by callers until it runs. Concurrent calls skip: one drainer at a time.
pub fn drain() {
    let _ = DRAIN.with(|chunk| {
        loop {
            let n = RING.with(|ring| ring.drain_into(&mut chunk[..])).unwrap_or(0);
            if n == 0 {
                break;
            }
            let bytes = &chunk[..n];
            console_write(bytes);
            append(bytes);
        }
    });
    report_drops();
}

/// Hand one chunk of whole lines to the kernel debug output (`svcOutputDebugString`; Eden logs it as
/// `OutputDebugString`, the console ignores it without a debugger). Only the holder of `DRAIN` calls
/// this, so chunks never interleave and no shared queue is involved.
#[cfg(target_arch = "aarch64")]
fn console_write(bytes: &[u8]) {
    // SAFETY: SVC 0x27 reads `len` bytes at `ptr`, both live for the call, and writes nothing in this
    // process but its result register. The console kernel returns from an SVC with x8-x17 zeroed as
    // well as x0-x7 (the v0.1.0 console boot crash: a static pointer the compiler kept in x8 was zeroed
    // by the SVC return and the load after it faulted at 0), so the block clobbers the whole
    // `extern "C"` integer register set with `clobber_abi("C")`, exactly as the plugin's other SVC
    // sites do (`preload_metrics.rs`, `practice_trace.rs`, `mii_ordinary_native.rs`, `threads.rs`,
    // `practice_stage_request.rs`).
    unsafe {
        core::arch::asm!(
            "svc #0x27",
            inout("x0") bytes.as_ptr() => _,
            inout("x1") bytes.len() => _,
            clobber_abi("C"),
            options(nostack, readonly),
        );
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn console_write(_bytes: &[u8]) {}

/// Append whole lines to the current file, rotating at `DIAGNOSTIC_LIMIT`: open in append mode, write,
/// close. The first failure turns the file off and is reported to the failures file.
fn append(bytes: &[u8]) -> bool {
    let Ok(mut log) = DIAGNOSTICS.try_lock() else {
        return false;
    };
    if !log.ready {
        return false;
    }
    if storage::should_rotate(log.bytes, bytes.len()) {
        if let Err(e) = rotate_diagnostics(&mut log) {
            report_failure(&log.failures, "rotation", &e);
            log.ready = false;
            return false;
        }
    }
    let written = OpenOptions::new().append(true).open(&log.current).and_then(|mut file| file.write_all(bytes));
    if let Err(e) = written {
        report_failure(&log.failures, "write", &e);
        log.ready = false;
        return false;
    }
    log.bytes += bytes.len() as u64;
    true
}
/// Report lines lost to a full ring or a busy lock, once per change (`ring_dropped=<n>` in the file).
fn report_drops() {
    let dropped = ring_stats().0.dropped;
    if dropped > REPORTED_DROPS.load(Ordering::Relaxed)
        && REPORTING.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    {
        REPORTED_DROPS.store(dropped, Ordering::Relaxed);
        diagnostic(format_args!("[PracticeDiagnostics] {}", ring_stats()));
        REPORTING.store(false, Ordering::Release);
    }
}

/// Drain ticks (50 ms each) between two `hook_stats` samples in a diag build: 10 seconds.
#[cfg(feature = "diag")]
const HOOK_STATS_TICKS: u32 = 200;

/// A step the drain thread runs on every tick (registered once at boot).
static TICK: std::sync::OnceLock<fn()> = std::sync::OnceLock::new();

/// Register the step the drain thread runs on every 50 ms tick, after the drain (the first registration wins).
pub fn set_tick(step: fn()) {
    let _ = TICK.set(step);
}

/// The drain thread: every 50 ms it moves buffered lines to the card. Its stack is the table's
/// (`model::thread_stacks`). If it cannot be created, callers keep writing synchronously (SPEC-core-01 §7).
pub fn start_drain_thread() {
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::Heartbeat, || {
        writer_started();
        #[cfg(feature = "diag")]
        let (mut observer, mut ticks) = (crate::model::hook_stats::Observer::new(), 0u32);
        #[cfg(feature = "plugin-heap-meter")]
        let mut metered_calls = 0usize;
        loop {
            drain();
            // A step the owner of a late report registered (`set_tick`): package 6's seam installer writes the seam outcomes that
            // settled after the install line (R-A0b) here, on this thread and never in a hook.
            if let Some(tick) = TICK.get() {
                tick();
            }
            // Diag builds only (review R5): every `HOOK_STATS_TICKS` drains, one `hook_stats` line if
            // any hook ran or allocated since the previous line. This thread is not a hook, so its own
            // work is never counted. The shipped build has no such line.
            #[cfg(feature = "diag")]
            {
                ticks += 1;
                if ticks % HOOK_STATS_TICKS == 0 {
                    if let Some(observation) = observer.observe(crate::model::hook_stats::snapshot()) {
                        diagnostic(format_args!("{observation}"));
                    }
                    // The threads' stack high-water marks (stream/stack), on the same 10 s beat.
                    crate::runtime::threads::report();
                    // `plugin-heap-meter`: the byte meter of the same allocator, one line when this NRO allocated since the last one.
                    #[cfg(feature = "plugin-heap-meter")]
                    {
                        let heap = crate::runtime::roster::heap_metrics::snapshot();
                        if heap.calls != metered_calls {
                            metered_calls = heap.calls;
                            diagnostic(format_args!(
                                "[HeapMeter] live={} peak={} total={} calls={} largest={}",
                                heap.live, heap.peak, heap.total, heap.calls, heap.largest
                            ));
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
    if spawned.is_err() {
        diagnostic(format_args!("[PracticeDiagnostics] drain thread unavailable; writing synchronously"));
    }
}
