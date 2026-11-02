//! The one way the runtime starts a thread (stream/stack): every plugin thread runs on a static, page-aligned stack sized
//! from `model::thread_stacks::THREADS`, with a static thread object, created through the SDK (`nn::os::CreateThread`,
//! `SetThreadNamePointer`, `StartThread`) as the playbook's plugins do (CHECKLIST: "a writer thread on a static stack";
//! gsp-visualizer, ssbu-gnw-judge-log). Nothing is allocated for a stack or a name. The boot loaders (`BOOT_LOADERS`, joined
//! one after another) share one stack; a start while a stack is in use is refused (`ErrorKind::ResourceBusy`).
//!
//! Diag builds also measure each thread. At its start the region holding its stack is read with `svcQueryMemory` at the
//! stack pointer (the SDK maps the static array into the stack region; the array itself is not readable meanwhile), logged
//! (`[ThreadStack] spawn name=… base=… size=…`, so a fault's SP can be matched to a thread), and everything below the stack
//! pointer is painted with `PAINT`. The heartbeat later reads each live stack from the bottom up; the first word that lost
//! the paint is the deepest the thread has reached (`[ThreadStack] high_water name=… used=… free=… low=…`, logged when it
//! grows; `low=true` when less than the SDK reserve is left). A thread that returns reports its own mark
//! (`[ThreadStack] exit …`). The boot thread (Skyline's, which runs `main`) is measured the same way across `boot::run`.

use std::io;

use crate::model::thread_stacks::ThreadId;

/// Start the detached thread `id` (it runs for the session or ends on its own). A closure that captures nothing is passed
/// without an allocation; one with captures is boxed once, here, at its start (never in a hook).
pub fn spawn<F>(id: ThreadId, f: F) -> io::Result<()>
where
    F: FnOnce() + Send + 'static,
{
    imp::spawn(id, f)
}

/// Run `f` on the joined thread `id` and wait for it (the boot loaders). The closure and its result stay on the caller's
/// stack; the thread object is destroyed and the stack free again when this returns.
pub fn run_joined<F, T>(id: ThreadId, f: F) -> io::Result<T>
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    imp::run_joined(id, f)
}

#[cfg(target_os = "switch")]
mod imp {
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicBool, Ordering};
    use std::io;

    use skyline::nn;

    use crate::model::thread_stacks::{self, ThreadId, THREAD_OBJECT_BYTES};

    #[repr(C, align(4096))]
    struct Stack<const N: usize>(UnsafeCell<[u8; N]>);
    #[repr(C, align(16))]
    struct Object(UnsafeCell<[u8; THREAD_OBJECT_BYTES]>);

    /// One thread's static stack and thread object; `busy` while a thread uses them.
    struct Slot<const N: usize> {
        stack: Stack<N>,
        object: Object,
        busy: AtomicBool,
    }

    // SAFETY: the stack and the object are handed to nn::os by the one caller that set `busy`, and touched by nothing else
    // until the thread is destroyed (joined threads) or for the session (detached threads).
    unsafe impl<const N: usize> Sync for Slot<N> {}

    impl<const N: usize> Slot<N> {
        const fn new() -> Self {
            Self {
                stack: Stack(UnsafeCell::new([0; N])),
                object: Object(UnsafeCell::new([0; THREAD_OBJECT_BYTES])),
                busy: AtomicBool::new(false),
            }
        }

        fn parts(&'static self) -> Parts {
            Parts {
                object: self.object.0.get().cast(),
                stack: self.stack.0.get().cast(),
                bytes: N,
                busy: &self.busy,
            }
        }
    }

    struct Parts {
        object: *mut nn::os::ThreadType,
        stack: *mut u8,
        bytes: usize,
        busy: &'static AtomicBool,
    }

    /// One static slot per thread (the boot loaders share one), each the size of its table row; diag-only and
    /// feature-only threads have their stacks only in those builds.
    macro_rules! slots {
        ($( $(#[$attr:meta])* $name:ident = [$($id:ident),+]; )*) => {
            $( $(#[$attr])* static $name: Slot<{ thread_stacks::slot_bytes(&[$(ThreadId::$id),+]) }> = Slot::new(); )*
            fn parts(id: ThreadId) -> Option<Parts> {
                #[allow(unreachable_patterns)]
                match id {
                    $( $(#[$attr])* $(ThreadId::$id)|+ => Some($name.parts()), )*
                    _ => None,
                }
            }
        };
    }

    slots! {
        HEARTBEAT = [Heartbeat];
        BOOT_LOADERS = [Import, SettingsLoad, StateLoad];
        STATE_WRITER = [StateWriter];
        #[cfg(feature = "diag")] STATE_DIAG = [StateDiag];
        INI_WRITER = [IniWriter];
        TRACE_WRITER = [TraceWriter];
        #[cfg(feature = "diag")] DIAG_RELOAD = [DiagReload];
        IMPORT_CAPTURES = [ImportCaptures];
        MENU = [Menu];
        ARCHIVE = [Archive];
        #[cfg(feature = "diag")] PROBE = [Probe];
        #[cfg(feature = "diag")] MACRO_DIAG = [MacroDiag];
        REPLAY_SAVE = [ReplaySave];
        #[cfg(feature = "mii-saved-alpha")] MII_CATALOG = [MiiCatalog];
        #[cfg(feature = "console-practice-trace")] PRACTICE_TRACE = [PracticeTrace];
    }

    /// Create `id`'s thread on its static stack, running `entry(argument)`, and name it. `busy` is held on success.
    fn create(id: ThreadId, entry: extern "C" fn(*mut skyline::libc::c_void), argument: *mut u8) -> io::Result<Parts> {
        let Some(parts) = parts(id) else {
            return Err(io::Error::other("no static stack for this thread in this build"));
        };
        if parts.busy.swap(true, Ordering::AcqRel) {
            return Err(io::Error::new(io::ErrorKind::ResourceBusy, "the thread's static stack is in use"));
        }
        // SAFETY: the object and the stack are this slot's statics (page-aligned stack, whole pages), unused while `busy`
        // was clear; the priority is the creating thread's, as in the playbook's plugins.
        let result = unsafe {
            let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
            nn::os::CreateThread(parts.object, entry, argument, parts.stack, parts.bytes, priority)
        };
        if result != 0 {
            parts.busy.store(false, Ordering::Release);
            return Err(io::Error::other(format!("nn::os::CreateThread failed: {result:#x}")));
        }
        // SAFETY: the object was just created; the name is a static C string.
        unsafe { nn::os::SetThreadNamePointer(parts.object, thread_stacks::of(id).c_name.as_ptr().cast()) };
        Ok(parts)
    }

    /// A detached thread's argument: the closure, boxed (in the shipped build a closure without captures is a zero-sized
    /// box: no allocation); the diag build adds the id for its stack report.
    struct Detached<F> {
        #[cfg(feature = "diag")]
        id: ThreadId,
        f: F,
    }

    extern "C" fn detached<F: FnOnce() + Send + 'static>(argument: *mut skyline::libc::c_void) {
        // SAFETY: `spawn` leaked exactly this box for this thread.
        let job = unsafe { Box::from_raw(argument.cast::<Detached<F>>()) };
        #[cfg(feature = "diag")]
        let _watch = super::diag::Watch::begin(job.id);
        (job.f)();
    }

    pub fn spawn<F: FnOnce() + Send + 'static>(id: ThreadId, f: F) -> io::Result<()> {
        let job = Box::into_raw(Box::new(Detached {
            #[cfg(feature = "diag")]
            id,
            f,
        }));
        match create(id, detached::<F>, job.cast()) {
            Ok(parts) => {
                // SAFETY: created above; the thread owns `job` from here on.
                unsafe { nn::os::StartThread(parts.object) };
                Ok(())
            }
            Err(error) => {
                // SAFETY: no thread was created, so the box is still ours.
                drop(unsafe { Box::from_raw(job) });
                Err(error)
            }
        }
    }

    /// A joined thread's closure and result, on the waiting caller's stack.
    struct Joined<F, T> {
        id: ThreadId,
        f: Option<F>,
        out: Option<T>,
    }

    extern "C" fn joined<F: FnOnce() -> T, T>(argument: *mut skyline::libc::c_void) {
        // SAFETY: `run_joined` passes its packet and waits for this thread before reading or dropping it.
        let packet = unsafe { &mut *argument.cast::<Joined<F, T>>() };
        #[cfg(feature = "diag")]
        let _watch = super::diag::Watch::begin(packet.id);
        #[cfg(not(feature = "diag"))]
        let _ = packet.id;
        if let Some(f) = packet.f.take() {
            packet.out = Some(f());
        }
    }

    pub fn run_joined<F: FnOnce() -> T + Send, T: Send>(id: ThreadId, f: F) -> io::Result<T> {
        if !thread_stacks::joined(id) {
            return Err(io::Error::other("not a joined thread"));
        }
        let mut packet = Joined { id, f: Some(f), out: None };
        let parts = create(id, joined::<F, T>, (&mut packet as *mut Joined<F, T>).cast())?;
        // SAFETY: created above; the packet outlives the thread (waited for and destroyed before `packet` is read).
        unsafe {
            nn::os::StartThread(parts.object);
            nn::os::WaitThread(parts.object);
            nn::os::DestroyThread(parts.object);
        }
        parts.busy.store(false, Ordering::Release);
        packet.out.ok_or_else(|| io::Error::other("the joined thread returned nothing"))
    }
}

/// Host builds (only `tests/runtime_diag_log.rs` compiles this file there): plain std threads with the table's sizes.
#[cfg(not(target_os = "switch"))]
mod imp {
    use std::io;

    use crate::model::thread_stacks::{self, ThreadId};

    pub fn spawn<F: FnOnce() + Send + 'static>(id: ThreadId, f: F) -> io::Result<()> {
        let row = thread_stacks::of(id);
        std::thread::Builder::new().name(row.name.into()).stack_size(row.bytes).spawn(f).map(|_| ())
    }

    pub fn run_joined<F: FnOnce() -> T + Send, T: Send>(id: ThreadId, f: F) -> io::Result<T> {
        let row = thread_stacks::of(id);
        std::thread::scope(|scope| {
            std::thread::Builder::new().name(row.name.into()).stack_size(row.bytes).spawn_scoped(scope, f)?.join()
                .map_err(|_| io::Error::other("the joined thread panicked"))
        })
    }
}

#[cfg(feature = "diag")]
pub use diag::{boot_begin, boot_end, headroom, report};

/// Diag builds: record the stack room left at a hook or callback entry (`[HookStack]` lines). The shipped build: nothing.
#[cfg(not(feature = "diag"))]
#[inline(always)]
pub fn headroom(_site: &'static str) {}

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

    /// The heartbeat's step: one `high_water` line per live thread whose mark grew since its last line, then one
    /// `[HookStack]` line per hook or callback site whose smallest entry room shrank since its last line.
    pub fn report() {
        {
            let mut live = LIVE.lock();
            for slot in live.iter_mut().flatten() {
                let used = thread_stacks::used(slot.size, untouched(slot.base, slot.base + slot.size));
                if used > slot.reported {
                    slot.reported = used;
                    line("high_water", slot, used);
                }
            }
        }
        let mut heads = HEADS.lock();
        for head in heads.iter_mut().flatten() {
            if head.min_free < head.reported {
                head.reported = head.min_free;
                diagnostic(format_args!(
                    "[HookStack] site={} calls={} min_free={} stack={:#x} base={:#x}",
                    head.name, head.calls, head.min_free, head.stack, head.base
                ));
            }
        }
    }

    /// One hook or callback site: how often it ran and the least stack room it found at entry (SP minus the bottom of the
    /// region holding SP), with that region.
    #[derive(Clone, Copy)]
    struct Head {
        name: &'static str,
        calls: u64,
        min_free: usize,
        stack: usize,
        base: usize,
        reported: usize,
    }

    const HEAD_SLOTS: usize = 96;
    static HEADS: Mutex<[Option<Head>; HEAD_SLOTS]> = Mutex::new([None; HEAD_SLOTS]);

    /// Record the room left below the stack pointer at the entry of `site` (a game thread's stack, or a fiber's).
    pub fn headroom(site: &'static str) {
        let here = sp();
        let mut heads = HEADS.lock();
        let index = match heads.iter().position(|h| h.is_some_and(|h| h.name == site)) {
            Some(i) => i,
            None => match heads.iter().position(Option::is_none) {
                Some(i) => {
                    heads[i] = Some(Head { name: site, calls: 0, min_free: usize::MAX, stack: 0, base: 0, reported: usize::MAX });
                    i
                }
                None => return,
            },
        };
        let Some(head) = heads[index].as_mut() else { return };
        head.calls += 1;
        // The region of the last minimum is reused while SP lies in it; another stack (thread, fiber) is queried.
        let (base, size) = if head.base <= here && here < head.base + head.stack {
            (head.base, head.stack)
        } else {
            match region(here) {
                Some((base, size, _)) => (base, size),
                None => return,
            }
        };
        let free = here - base;
        if free < head.min_free {
            head.min_free = free;
            head.base = base;
            head.stack = size;
        }
    }

    static BOOT: Mutex<Option<Slot>> = Mutex::new(None);

    /// `boot::run`'s first step: measure the thread Skyline runs `main` on, and register the hook-entry probe.
    pub fn boot_begin() {
        let _ = crate::model::hook_stats::ENTRY_PROBE.set(headroom);
        *BOOT.lock() = start("boot", 0);
    }

    /// `boot::run`'s last step.
    pub fn boot_end() {
        if let Some(slot) = BOOT.lock().take() {
            line("exit", &slot, thread_stacks::used(slot.size, untouched(slot.base, slot.base + slot.size)));
        }
    }
}
