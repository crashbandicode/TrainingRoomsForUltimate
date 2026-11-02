//! The fork's `logging::{diagnostic, heap_profile}` as the carried code names them (written for this repository, not carried).
//!
//! On the Switch `diagnostic` is package 1's `runtime::diag_log::diagnostic` (the carried, ring-buffered, shipped-tag-filtered
//! writer): the same call the carried lines make in the fork. On the host it records the line into a bounded capture buffer that
//! tests read with [`take`], so a carried test can assert what was logged and no host build needs the runtime.
//!
//! `heap_profile` is the fork's Eden-only heap meter hook (`plugin-heap-meter`, a diag feature): the carried `preload_metrics` calls
//! it to mark a snapshot; the meter is the byte meter of package 1's counting allocator (the fork's own allocator is not carried).

use std::fmt;

/// One diagnostic line.
pub fn diagnostic(args: fmt::Arguments<'_>) {
    #[cfg(target_os = "switch")]
    crate::runtime::diag_log::diagnostic(args);
    #[cfg(not(target_os = "switch"))]
    capture::push(args);
}

/// The fork's `log::info!`, `log::warn!` and `log::error!` (a level in the carried INI loader's messages, a Switch-only file): one
/// diagnostic line each.
#[cfg(target_os = "switch")]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}
#[cfg(target_os = "switch")]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}
#[cfg(target_os = "switch")]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}
#[cfg(target_os = "switch")]
pub(crate) use {log_error, log_info, log_warn};

/// Marks a heap-meter snapshot: with `plugin-heap-meter` (a diag feature) on the Switch, one `[HeapMeter]` line with the byte meter of
/// the plugin's one counting allocator (`model::hook_stats`); nothing otherwise.
pub fn heap_profile(label: &str) {
    #[cfg(all(target_os = "switch", feature = "plugin-heap-meter"))]
    {
        let heap = crate::model::hook_stats::heap_bytes();
        diagnostic(format_args!(
            "[HeapMeter] label={label} live={} peak={} total={} calls={} largest={}",
            heap.live, heap.peak, heap.total, heap.calls, heap.largest
        ));
    }
    #[cfg(not(all(target_os = "switch", feature = "plugin-heap-meter")))]
    let _ = label;
}

/// The host's capture buffer (bounded: the oldest lines are dropped).
#[cfg(not(target_os = "switch"))]
mod capture {
    use std::fmt;
    use std::sync::Mutex;

    const KEEP: usize = 256;
    static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn push(args: fmt::Arguments<'_>) {
        if let Ok(mut lines) = LINES.lock() {
            if lines.len() == KEEP {
                lines.remove(0);
            }
            lines.push(args.to_string());
        }
    }

    pub fn take() -> Vec<String> {
        LINES.lock().map(|mut lines| std::mem::take(&mut *lines)).unwrap_or_default()
    }
}

/// The lines captured since the last call (host builds only).
#[cfg(not(target_os = "switch"))]
pub fn take() -> Vec<String> {
    capture::take()
}

#[cfg(all(test, not(target_os = "switch")))]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_captured_once_and_the_buffer_is_bounded() {
        let _ = take();
        diagnostic(format_args!("[Probe] one {}", 1));
        assert_eq!(take(), ["[Probe] one 1"]);
        assert!(take().is_empty());
        for n in 0..300 {
            diagnostic(format_args!("line {n}"));
        }
        let lines = take();
        assert_eq!(lines.len(), 256);
        assert_eq!(lines[0], "line 44");
        heap_profile("label");
    }
}