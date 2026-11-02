//! The one way the runtime starts a thread (stream/stack): every plugin thread runs on a static, page-aligned stack sized
//! from `model::thread_stacks::THREADS`, with a static thread object, created through the SDK (`nn::os::CreateThread`,
//! `SetThreadNamePointer`, `StartThread`) as the playbook's plugins do (CHECKLIST: "a writer thread on a static stack";
//! gsp-visualizer, ssbu-gnw-judge-log). Nothing is allocated for a stack or a name. The boot loaders (`BOOT_LOADERS`, joined
//! one after another) share one stack, and the import's captures (`BOOT_STACK_LAST`) take it at the end of boot; a start
//! while a stack is in use is refused (`ErrorKind::ResourceBusy`).
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
        BOOT_STACK = [Import, SettingsLoad, StateLoad, ImportCaptures];
        STATE_WRITER = [StateWriter];
        #[cfg(feature = "diag")] STATE_DIAG = [StateDiag];
        INI_WRITER = [IniWriter];
        TRACE_WRITER = [TraceWriter];
        #[cfg(feature = "diag")] DIAG_RELOAD = [DiagReload];
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


/// The diag measurements, lock-free (stream/stack): hooks run on game threads with fixed priorities, where a spinning lock can
/// livelock (ui-screens' menu-close finding, parking_lot's busy-spin parker), so neither the hook-entry probe nor the thread
/// registry takes a lock; every field is an atomic, claimed with a compare-and-swap.
#[cfg(feature = "diag")]
mod diag {
    use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

    use crate::model::thread_stacks::{self, ThreadId, PAINT, THREADS};
    use crate::runtime::diag_log::diagnostic;

    /// Bytes left unpainted just below the stack pointer of the painting frame (its own callees' room).
    const PAINT_GAP: usize = 1024;

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
        // SAFETY: the caller holds a reader count on a live slot (or is the stack's own thread); a thread waits for the
        // readers to leave before its slot is freed and it returns; the words are read, never written.
        while at + 8 <= limit && unsafe { core::ptr::read_volatile(at as *const u64) } == PAINT {
            at += 8;
        }
        at - base
    }

    const FREE: u8 = 0;
    const CLAIMED: u8 = 1;
    const LIVE: u8 = 2;
    /// A thread's row index, or `BOOT` for the boot thread.
    const BOOT: u32 = u32::MAX;

    /// One measured stack. `state` FREE → CLAIMED (fields being written) → LIVE → FREE; `readers` counts heartbeat scans.
    struct Live {
        state: AtomicU8,
        readers: AtomicU32,
        who: AtomicU32,
        base: AtomicUsize,
        size: AtomicUsize,
        reported: AtomicUsize,
    }

    const SLOTS: usize = 32;
    static STACKS: [Live; SLOTS] = [const {
        Live {
            state: AtomicU8::new(FREE),
            readers: AtomicU32::new(0),
            who: AtomicU32::new(0),
            base: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            reported: AtomicUsize::new(0),
        }
    }; SLOTS];

    fn name(who: u32) -> &'static str {
        if who == BOOT { "boot" } else { THREADS.get(who as usize).map_or("?", |row| row.name) }
    }

    fn line(what: &str, who: u32, size: usize, used: usize) {
        diagnostic(format_args!(
            "[ThreadStack] {what} name={} used={used} size={size} free={} low={}",
            name(who),
            size.saturating_sub(used),
            thread_stacks::low(size, used)
        ));
    }

    /// Find, log and paint the current stack, and register it; the slot index, or `None`.
    fn start(who: u32, table: usize) -> Option<usize> {
        let here = sp();
        let Some((base, size, kind)) = region(here) else {
            diagnostic(format_args!("[ThreadStack] spawn name={} sp={here:#x} region=unreadable table={table:#x}", name(who)));
            return None;
        };
        diagnostic(format_args!(
            "[ThreadStack] spawn name={} base={base:#x} size={size:#x} top={:#x} sp={here:#x} kind={kind:#x} table={table:#x}",
            name(who),
            base + size
        ));
        if here < base + PAINT_GAP || here > base + size {
            return None;
        }
        paint(base, here - PAINT_GAP);
        let index = STACKS.iter().position(|s| s.state.compare_exchange(FREE, CLAIMED, Ordering::AcqRel, Ordering::Relaxed).is_ok())?;
        let slot = &STACKS[index];
        slot.who.store(who, Ordering::Relaxed);
        slot.base.store(base, Ordering::Relaxed);
        slot.size.store(size, Ordering::Relaxed);
        slot.reported.store(0, Ordering::Relaxed);
        slot.state.store(LIVE, Ordering::Release);
        Some(index)
    }

    /// Unregister (the stack's own thread, before it returns): wait for the heartbeat's scan to leave, report, free.
    fn finish(index: usize) {
        let slot = &STACKS[index];
        slot.state.store(CLAIMED, Ordering::Release);
        while slot.readers.load(Ordering::Acquire) != 0 {
            // A sleep, not a spin: the reader may be a lower-priority thread on this core.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let (base, size) = (slot.base.load(Ordering::Relaxed), slot.size.load(Ordering::Relaxed));
        line("exit", slot.who.load(Ordering::Relaxed), size, thread_stacks::used(size, untouched(base, base + size)));
        slot.state.store(FREE, Ordering::Release);
    }

    pub struct Watch {
        slot: Option<usize>,
    }

    impl Watch {
        pub fn begin(id: ThreadId) -> Self {
            Self { slot: start(id as u32, thread_stacks::of(id).bytes) }
        }
    }

    impl Drop for Watch {
        fn drop(&mut self) {
            if let Some(index) = self.slot {
                finish(index);
            }
        }
    }

    /// One hook or callback site: how often it ran and the least stack room it found at entry (SP minus the bottom of the
    /// region holding SP), with that region. Claimed by the site name's address.
    struct Head {
        name: AtomicPtr<u8>,
        len: AtomicUsize,
        calls: AtomicU64,
        min_free: AtomicUsize,
        base: AtomicUsize,
        stack: AtomicUsize,
        /// The region's memory state (0xb: a thread stack; another value, e.g. heap, means a fiber stack inside a larger
        /// block, whose own bottom the region does not show).
        kind: AtomicU32,
        reported: AtomicUsize,
    }

    const HEAD_SLOTS: usize = 96;
    static HEADS: [Head; HEAD_SLOTS] = [const {
        Head {
            name: AtomicPtr::new(core::ptr::null_mut()),
            len: AtomicUsize::new(0),
            calls: AtomicU64::new(0),
            min_free: AtomicUsize::new(usize::MAX),
            base: AtomicUsize::new(0),
            stack: AtomicUsize::new(0),
            kind: AtomicU32::new(0),
            reported: AtomicUsize::new(usize::MAX),
        }
    }; HEAD_SLOTS];

    /// Record the room left below the stack pointer at the entry of `site` (a game thread's stack, or a fiber's). No lock,
    /// no allocation; one `svcQueryMemory` when SP is outside the region of the site's last minimum.
    pub fn headroom(site: &'static str) {
        let here = sp();
        let key = site.as_ptr() as *mut u8;
        let mut found = None;
        for head in &HEADS {
            let current = head.name.load(Ordering::Acquire);
            if current == key {
                found = Some(head);
                break;
            }
            if current.is_null() {
                match head.name.compare_exchange(core::ptr::null_mut(), key, Ordering::AcqRel, Ordering::Acquire) {
                    Ok(_) => {
                        head.len.store(site.len(), Ordering::Release);
                        found = Some(head);
                        break;
                    }
                    Err(other) if other == key => {
                        found = Some(head);
                        break;
                    }
                    Err(_) => {}
                }
            }
        }
        let Some(head) = found else { return };
        head.calls.fetch_add(1, Ordering::Relaxed);
        let (base, stack) = (head.base.load(Ordering::Relaxed), head.stack.load(Ordering::Relaxed));
        let (base, stack, kind) = if base <= here && here < base + stack {
            (base, stack, head.kind.load(Ordering::Relaxed))
        } else {
            match region(here) {
                Some(found) => found,
                None => return,
            }
        };
        let free = here - base;
        if free < head.min_free.fetch_min(free, Ordering::AcqRel) {
            head.base.store(base, Ordering::Relaxed);
            head.stack.store(stack, Ordering::Relaxed);
            head.kind.store(kind, Ordering::Relaxed);
        }
    }

    /// The heartbeat's step: one `high_water` line per live thread whose mark grew since its last line, then one
    /// `[HookStack]` line per hook or callback site whose smallest entry room shrank since its last line.
    pub fn report() {
        for slot in &STACKS {
            slot.readers.fetch_add(1, Ordering::AcqRel);
            if slot.state.load(Ordering::Acquire) == LIVE {
                let (base, size) = (slot.base.load(Ordering::Relaxed), slot.size.load(Ordering::Relaxed));
                let used = thread_stacks::used(size, untouched(base, base + size));
                if used > slot.reported.load(Ordering::Relaxed) {
                    slot.reported.store(used, Ordering::Relaxed);
                    line("high_water", slot.who.load(Ordering::Relaxed), size, used);
                }
            }
            slot.readers.fetch_sub(1, Ordering::AcqRel);
        }
        for head in &HEADS {
            let (key, len) = (head.name.load(Ordering::Acquire), head.len.load(Ordering::Acquire));
            if key.is_null() || len == 0 {
                continue;
            }
            let min = head.min_free.load(Ordering::Acquire);
            if min < head.reported.load(Ordering::Relaxed) {
                head.reported.store(min, Ordering::Relaxed);
                // SAFETY: `key`/`len` are a `&'static str` a probe registered.
                let site = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(key, len)) };
                diagnostic(format_args!(
                    "[HookStack] site={site} calls={} min_free={min} stack={:#x} base={:#x} kind={:#x}",
                    head.calls.load(Ordering::Relaxed),
                    head.stack.load(Ordering::Relaxed),
                    head.base.load(Ordering::Relaxed),
                    head.kind.load(Ordering::Relaxed)
                ));
            }
        }
    }

    static BOOT_SLOT: AtomicUsize = AtomicUsize::new(usize::MAX);

    /// `boot::run`'s first step: measure the thread Skyline runs `main` on, and register the hook-entry probe.
    pub fn boot_begin() {
        let _ = crate::model::hook_stats::ENTRY_PROBE.set(headroom);
        if let Some(index) = start(BOOT, 0) {
            BOOT_SLOT.store(index, Ordering::Release);
        }
    }

    /// `boot::run`'s last step.
    pub fn boot_end() {
        let index = BOOT_SLOT.swap(usize::MAX, Ordering::AcqRel);
        if index != usize::MAX {
            finish(index);
        }
    }
}
