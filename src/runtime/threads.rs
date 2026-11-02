//! The one way the runtime starts a thread (stream/stack): its name and stack size come from `model::thread_stacks::THREADS`.
//!
//! Diag builds also measure it. At the start of the thread the region holding its stack is read with `svcQueryMemory` at
//! the stack pointer, logged (`[ThreadStack] spawn name=… base=… size=…`, so a fault's SP can be matched to a thread), and
//! everything below the stack pointer is painted with `PAINT`. The heartbeat later reads each live stack from the bottom
//! up; the first word that lost the paint is the deepest the thread has reached (`[ThreadStack] high_water name=… used=…
//! free=… low=…`, logged when it grows; `low=true` when less than the SDK reserve is left). A thread that returns reports
//! its own mark (`[ThreadStack] exit …`). The boot thread (Skyline's, which runs `main`) is measured the same way across
//! `boot::run`. The shipped NRO only sets the name and size.

use std::io;
use std::thread::JoinHandle;

use crate::model::thread_stacks::{self, ThreadId};

/// Start the thread `id` with its table name and stack size.
pub fn spawn<F, T>(id: ThreadId, f: F) -> io::Result<JoinHandle<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let row = thread_stacks::of(id);
    std::thread::Builder::new().name(row.name.into()).stack_size(row.bytes).spawn(move || {
        #[cfg(feature = "diag")]
        let _watch = diag::Watch::begin(id);
        f()
    })
}

#[cfg(feature = "diag")]
pub use diag::{boot_begin, boot_end, report};

#[cfg(feature = "diag")]
mod diag {
    use parking_lot::Mutex;

    use crate::model::thread_stacks::{self, ThreadId, PAINT};
    use crate::runtime::diag_log::diagnostic;

    /// Bytes left unpainted just below the stack pointer of the painting frame (its own callees' room).
    const PAINT_GAP: usize = 1024;
    const SLOTS: usize = 32;

    #[repr(C)]
    #[derive(Default)]
    struct MemoryInfo {
        address: u64,
        size: u64,
        kind: u32,
        attributes: u32,
        permission: u32,
        ipc: u32,
        device: u32,
        padding: u32,
    }

    /// The mapped region holding `address` (`svcQueryMemory`, the same pinned ABI as `mii_ordinary_native::span`).
    fn region(address: usize) -> Option<(usize, usize, u32)> {
        let mut info = MemoryInfo::default();
        let result: u64;
        // SAFETY: SVC 0x6 writes one MemoryInfo to the pointer in x0 and the page info to x1; nothing else changes.
        unsafe {
            core::arch::asm!("svc 0x6",
                inout("x0") &mut info as *mut MemoryInfo as u64 => result,
                lateout("x1") _, in("x2") address as u64,
                clobber_abi("C"), options(nostack));
        }
        (result as u32 == 0).then_some((info.address as usize, info.size as usize, info.kind))
    }

    #[inline(always)]
    fn sp() -> usize {
        let sp: usize;
        // SAFETY: reads the stack pointer only.
        unsafe { core::arch::asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags)) };
        sp
    }

    /// Paint `[base, end)` word by word: a plain loop with no calls, so nothing below this frame is in use meanwhile.
    #[inline(never)]
    fn paint(base: usize, end: usize) {
        let mut at = base;
        while at + 8 <= end {
            // SAFETY: `[base, end)` is this thread's own stack below its stack pointer, mapped and unused.
            unsafe { core::ptr::write_volatile(at as *mut u64, PAINT) };
            at += 8;
        }
    }

    /// Untouched bytes from `base` up (stops at `limit`).
    fn untouched(base: usize, limit: usize) -> usize {
        let mut at = base;
        // SAFETY: the stack is mapped while its thread is registered (the scan holds the registry lock, a thread
        // unregisters before it returns); the words are read, never written.
        while at + 8 <= limit && unsafe { core::ptr::read_volatile(at as *const u64) } == PAINT {
            at += 8;
        }
        at - base
    }

    #[derive(Clone, Copy)]
    struct Slot {
        name: &'static str,
        base: usize,
        size: usize,
        reported: usize,
    }

    static LIVE: Mutex<[Option<Slot>; SLOTS]> = Mutex::new([None; SLOTS]);

    /// Find, log and paint the current stack; `None` when the region cannot be read.
    fn start(name: &'static str, table: usize) -> Option<Slot> {
        let here = sp();
        let Some((base, size, kind)) = region(here) else {
            diagnostic(format_args!("[ThreadStack] spawn name={name} sp={here:#x} region=unreadable table={table:#x}"));
            return None;
        };
        diagnostic(format_args!(
            "[ThreadStack] spawn name={name} base={base:#x} size={size:#x} top={:#x} sp={here:#x} kind={kind:#x} table={table:#x}",
            base + size
        ));
        if here < base + PAINT_GAP || here > base + size {
            return None;
        }
        paint(base, here - PAINT_GAP);
        Some(Slot { name, base, size, reported: 0 })
    }

    pub struct Watch {
        slot: Option<usize>,
    }

    impl Watch {
        pub fn begin(id: ThreadId) -> Self {
            let row = thread_stacks::of(id);
            let Some(slot) = start(row.name, row.bytes) else { return Self { slot: None } };
            let mut live = LIVE.lock();
            let index = live.iter().position(Option::is_none);
            if let Some(i) = index {
                live[i] = Some(slot);
            }
            Self { slot: index }
        }
    }

    impl Drop for Watch {
        fn drop(&mut self) {
            let Some(i) = self.slot else { return };
            let mut live = LIVE.lock();
            if let Some(slot) = live[i].take() {
                line("exit", &slot, thread_stacks::used(slot.size, untouched(slot.base, slot.base + slot.size)));
            }
        }
    }

    fn line(what: &str, slot: &Slot, used: usize) {
        diagnostic(format_args!(
            "[ThreadStack] {what} name={} used={used} size={} free={} low={}",
            slot.name,
            slot.size,
            slot.size.saturating_sub(used),
            thread_stacks::low(slot.size, used)
        ));
    }

    /// The heartbeat's step: one `high_water` line per live thread whose mark grew since its last line.
    pub fn report() {
        let mut live = LIVE.lock();
        for slot in live.iter_mut().flatten() {
            let used = thread_stacks::used(slot.size, untouched(slot.base, slot.base + slot.size));
            if used > slot.reported {
                slot.reported = used;
                line("high_water", slot, used);
            }
        }
    }

    static BOOT: Mutex<Option<Slot>> = Mutex::new(None);

    /// `boot::run`'s first step: measure the thread Skyline runs `main` on.
    pub fn boot_begin() {
        *BOOT.lock() = start("boot", 0);
    }

    /// `boot::run`'s last step.
    pub fn boot_end() {
        if let Some(slot) = BOOT.lock().take() {
            line("exit", &slot, thread_stacks::used(slot.size, untouched(slot.base, slot.base + slot.size)));
        }
    }
}
