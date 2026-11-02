//! The diagnostics channel on the card (SPEC-core-01 §7): a size-capped ring, a drain that writes
//! whole lines to `practice-diagnostics.log` (two files of at most 128 KiB, rotated) and to the
//! kernel debug output, and the panic breadcrumb. The shipped build admits only the
//! `diag_ring::SHIPPED_TAGS`; a `diag` build writes every line. Formatting happens on the caller's
//! stack and the ring is a fixed array, so a log call allocates nothing.
//!
//! NOT YET SWITCH-COMPILED. Lines 1-265 below the imports come from the earlier fork's
//! `src/logging.rs` lines 20-284 (stage m5o, fork-only, ours); CARRY-OVER.md lists the edits and
//! what was written new here (the end of `append`, `sync`, `report_drops`, the drain thread).

use std::cell::UnsafeCell;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::diag_ring::{self, ByteRing, StackLine};
use crate::model::storage;

const DIAGNOSTIC_PATH: &str = storage::DIAGNOSTICS_PATH;
const PREVIOUS_PATH: &str = storage::DIAGNOSTICS_PREVIOUS_PATH;
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
    file: Option<File>,
    bytes: u64,
    initialized: bool,
    sync_warning: bool,
}

static DIAGNOSTICS: Mutex<DiagnosticLog> = Mutex::new(DiagnosticLog {
    file: None,
    bytes: 0,
    initialized: false,
    sync_warning: false,
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

fn rotate_diagnostics(log: &mut DiagnosticLog) -> io::Result<()> {
    log.file.take();
    if fs::metadata(DIAGNOSTIC_PATH).is_ok() {
        match fs::remove_file(PREVIOUS_PATH) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        fs::rename(DIAGNOSTIC_PATH, PREVIOUS_PATH)?;
    }
    log.file = Some(
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(DIAGNOSTIC_PATH)?,
    );
    log.bytes = 0;
    Ok(())
}

/// Sparse, persistent breadcrumbs for console crash investigation. At most two
/// 128 KiB files; no per-frame writes. Failures never abort game initialization.
pub fn init_diagnostics() {
    if let Ok(mut log) = DIAGNOSTICS.lock() {
        if log.initialized {
            return;
        }
        log.initialized = true;
        if let Err(e) = fs::create_dir_all(storage::ROOT)
            .and_then(|_| rotate_diagnostics(&mut log))
        {
            diagnostic(format_args!("[PracticeDiagnostics] SD log unavailable: {}", e));
        }
    }
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

/// Record one diagnostic line. Never blocks on I/O, never allocates, never panics.
pub fn diagnostic(args: fmt::Arguments<'_>) {
    let mut line = StackLine::new();
    let whole = line.format(args);
    if !cfg!(feature = "diag") && !diag_ring::shipped(line.text()) {
        return;
    }
    let ts = now_ms();
    let text = whole.then_some(line.text());
    if RING.with(|ring| ring.push(ts, text, args, LINE_MAX)).is_none() {
        LOCK_MISSES.fetch_add(1, Ordering::Relaxed);
    }
    if !WRITER.load(Ordering::Acquire) {
        drain();
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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

/// Write everything buffered (version guard and panic hook use it before they return).
pub fn flush() {
    drain();
}

/// Move buffered lines to the card and the kernel debug output. Called every 50 ms by the drain
/// thread, and by callers until it runs. Concurrent calls skip: one drainer at a time.
pub fn drain() {
    let _ = DRAIN.with(|chunk| {
        let mut wrote = false;
        loop {
            let n = RING.with(|ring| ring.drain_into(&mut chunk[..])).unwrap_or(0);
            if n == 0 {
                break;
            }
            let bytes = &chunk[..n];
            console_write(bytes);
            wrote |= append(bytes);
        }
        if wrote {
            sync();
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
    // process but its result register; x0–x7 are declared clobbered as for any supervisor call.
    unsafe {
        core::arch::asm!(
            "svc #0x27",
            inout("x0") bytes.as_ptr() => _,
            inout("x1") bytes.len() => _,
            lateout("x2") _,
            lateout("x3") _,
            lateout("x4") _,
            lateout("x5") _,
            lateout("x6") _,
            lateout("x7") _,
            options(nostack, readonly),
        );
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn console_write(_bytes: &[u8]) {}

/// Append whole lines to the current file, rotating at `DIAGNOSTIC_LIMIT` as before.
fn append(bytes: &[u8]) -> bool {
    let Ok(mut log) = DIAGNOSTICS.try_lock() else {
        return false;
    };
    if log.file.is_none() {
        return false;
    }
    if storage::should_rotate(log.bytes, bytes.len()) {
        if let Err(e) = rotate_diagnostics(&mut log) {
            diagnostic(format_args!("[PracticeDiagnostics] rotation failed: {}", e));
            log.file = None;
            return false;
        }
    }
    let file = log.file.as_mut().unwrap();
    if let Err(e) = file.write_all(bytes) {
        diagnostic(format_args!("[PracticeDiagnostics] write failed: {}", e));
        log.file = None;
        return false;
    }
    log.bytes += bytes.len() as u64;
    true
}

/// Flush the file's data to the card once per drain that wrote something (one `sync` per drain,
/// SPEC-core-01 §7). A failure is reported once.
fn sync() {
    let Ok(mut log) = DIAGNOSTICS.try_lock() else {
        return;
    };
    let failed = match log.file.as_mut() {
        Some(file) => file.sync_data().is_err(),
        None => false,
    };
    if failed && !log.sync_warning {
        log.sync_warning = true;
        drop(log);
        diagnostic(format_args!("[PracticeDiagnostics] sync failed"));
    }
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

/// The drain thread: every 50 ms it moves buffered lines to the card. A 64 KiB stack, named. If it
/// cannot be created, callers keep writing synchronously (SPEC-core-01 §7).
pub fn start_drain_thread() {
    let spawned = std::thread::Builder::new().name("tr-heartbeat".into()).stack_size(64 * 1024).spawn(|| {
        writer_started();
        loop {
            drain();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
    if spawned.is_err() {
        diagnostic(format_args!("[PracticeDiagnostics] drain thread unavailable; writing synchronously"));
    }
}