//! Diag builds: the stall watchdog (primary inbox 2026-11-08 20:56Z). Eden covrest-yoga-05 saw the game thread stop at a
//! plugin menu close: no hook ran and no diag line came for two minutes. This thread samples the hooks' total entry
//! count every 250 ms (`model::stall_watch`); when it stays still for more than 2 s it writes, once per stall, the last
//! hook entered and the hook scopes still open, the menu worker's step, how long ago the diag drain last ran, and every
//! plugin lock held with its holder and the return address of the call that took it (`hos_lock::owners`). The lines go
//! to the diag ring and, because a stalled drain thread or file system would keep them there, straight to
//! `svcOutputDebugString` (Eden's console log). The shipped build has none of it.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::model::stall_watch::{Event, Watch, WORKER_STEPS};

/// The menu worker's step (`WORKER_STEPS` index), set by the worker at each point of a press and a close.
static WORKER_STEP: AtomicUsize = AtomicUsize::new(0);
/// The diag drain thread's last tick (uptime ms).
static DRAIN_TICK: AtomicU64 = AtomicU64::new(0);

/// The worker's place in a press and a close.
pub fn worker_step(step: usize) {
    WORKER_STEP.store(step, Ordering::Relaxed);
}

/// The diag drain thread ran now.
pub fn drain_tick() {
    DRAIN_TICK.store(crate::runtime::clock::uptime_ms(), Ordering::Relaxed);
}

/// Start the watchdog thread (diag boot).
pub fn start() {
    let _ = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::StallWatch, || {
        let mut watch = Watch::new();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let now = crate::runtime::clock::uptime_ms();
            match watch.observe(crate::model::hook_stats::total_entries(), now) {
                Some(Event::Stalled { entries, quiet_ms }) => report(entries, quiet_ms, now),
                Some(Event::Resumed { entries, quiet_ms }) => {
                    line(format_args!("[StallWatch] resumed entries={entries} quiet_ms={quiet_ms}"));
                }
                None => {}
            }
        }
    });
}

fn report(entries: u64, quiet_ms: u64, now: u64) {
    let (hook, open) = crate::model::hook_stats::last_entered();
    let step = WORKER_STEP.load(Ordering::Relaxed);
    let drain = now.saturating_sub(DRAIN_TICK.load(Ordering::Relaxed));
    line(format_args!(
        "[StallWatch] stalled entries={entries} quiet_ms={quiet_ms} last_hook={hook} open_scopes={open} worker_step={}:{} \
         drain_ms_ago={drain} lock_overflow={}",
        step,
        WORKER_STEPS.get(step).copied().unwrap_or("?"),
        hos_lock::owners::overflow()
    ));
    let mut held = [hos_lock::owners::Held::default(); 32];
    let n = hos_lock::owners::snapshot(&mut held);
    for (i, h) in held[..n].iter().enumerate() {
        line(format_args!("[StallWatch] held {i} lock={:#x} thread={:#x} pc={:#x}", h.lock, h.thread, h.pc));
    }
    line(format_args!("[StallWatch] held count={n}"));
}

/// A line of at most `LINE` bytes, formatted on the stack (longer text is cut).
const LINE: usize = 480;

struct Small {
    bytes: [u8; LINE],
    len: usize,
}

impl core::fmt::Write for Small {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = s.len().min(LINE - self.len);
        self.bytes[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

/// One line to the kernel debug output and to the diag ring.
fn line(args: core::fmt::Arguments<'_>) {
    let mut text = Small { bytes: [0; LINE], len: 0 };
    let _ = core::fmt::Write::write_fmt(&mut text, args);
    let bytes = &text.bytes[..text.len];
    crate::runtime::diag_log::console_line(bytes);
    crate::runtime::diag_log::diagnostic(format_args!("{}", core::str::from_utf8(bytes).unwrap_or("[StallWatch] ?")));
}
