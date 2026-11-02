//! The fork's `logging::{diagnostic, heap_profile}` as the carried code names them (written for this repository, not carried).
//!
//! On the Switch `diagnostic` is package 1's `runtime::diag_log::diagnostic` (the carried, ring-buffered, shipped-tag-filtered
//! writer): the same call the carried lines make in the fork. On the host it records the line into a bounded capture buffer that
//! tests read with [`take`], so a carried test can assert what was logged and no host build needs the runtime.
//!
//! `heap_profile` is the fork's Eden-only heap meter hook (`plugin-heap-meter`, a diag feature whose allocator is not carried):
//! here it does nothing, and the carried `preload_metrics` calls it only to mark a snapshot for that meter.

use std::fmt;

/// One diagnostic line.
pub fn diagnostic(args: fmt::Arguments<'_>) {
    #[cfg(target_os = "switch")]
    crate::runtime::diag_log::diagnostic(args);
    #[cfg(not(target_os = "switch"))]
    capture::push(args);
}

/// Marks a heap-meter snapshot (no-op: the meter is not carried).
pub fn heap_profile(_label: &str) {}

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