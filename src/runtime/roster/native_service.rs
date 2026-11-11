//! The one native650010 function entry. Capture dispatcher registers before Rust,
//! run plugin pre-service work, then preserve the existing native call/refusal path.
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Numeric entry identities only. Borrowed for this callback, never retained as
/// native pointer authority. Private words are actual x19/x20/x21/LR, in that
/// order, from native_service.S; no Rust constructor or InlineCtx conversion.
#[repr(C)]
pub(crate) struct ServiceEntryContext {
    words: [usize; 4],
}
impl ServiceEntryContext {
    pub(super) fn caller_lr(&self) -> usize {
        self.words[3]
    }
    #[cfg(feature = "console-swap-trace")]
    fn native_sp(&self) -> usize {
        // native_service.S places these four words at original SP - 0x20.
        // Capture the actual callback stack; TLS alone cannot identify a fiber stack.
        self as *const Self as usize + std::mem::size_of::<Self>()
    }
    #[cfg(feature = "managed-third-source-alpha")]
    pub(super) fn dispatcher_tuple(&self) -> [usize; 4] {
        self.words
    }
}

const _: () = assert!(std::mem::size_of::<ServiceEntryContext>() == 0x20);
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
core::arch::global_asm!(include_str!("native_service.S"));
extern "C" {
    fn training_mod_native_service_entry();
    fn training_mod_native_service_on_stack(entry: *mut u8, arg: usize, function: usize, top: usize) -> u64;
}

/// rc.7 FLAG 6: the plugin's part of every dispatch runs on one of these (`model::thread_stacks::SERVICE_STACK`), never
/// on the caller's stack (at `acc-rename`, 64 KiB in a heap block). Static, so nothing is allocated on the hot path.
const STACK: crate::model::thread_stacks::CallbackStack = crate::model::thread_stacks::SERVICE_STACK;
#[repr(C, align(16))]
struct ServiceStack(core::cell::UnsafeCell<[u8; STACK.bytes]>);
// SAFETY: a stack is used only by the dispatch that claimed it (`CLAIMED`), until it releases it.
unsafe impl Sync for ServiceStack {}
static STACKS: [ServiceStack; STACK.count] = [const { ServiceStack(core::cell::UnsafeCell::new([0; STACK.bytes])) }; STACK.count];
const _: () = assert!(STACK.count <= 32 && STACK.bytes % 16 == 0);
static CLAIMED: AtomicU32 = AtomicU32::new(0);
/// Pre-service polls skipped because every stack was in use, and the most stacks in use at once (diag reports both).
static EXHAUSTED: AtomicU32 = AtomicU32::new(0);
static PEAK: AtomicU32 = AtomicU32::new(0);
#[cfg(feature = "diag")]
static REPORTED_PEAK: AtomicU32 = AtomicU32::new(0);
/// rc.8 R5 (diag; the docs lane's review, finding 3): the most bytes any poll used of a service stack, measured from the
/// paint, against the static path the stack is sized for (`model::thread_stacks::SERVICE_STACK`, a lower bound: the
/// analysis does not follow indirect calls).
#[cfg(feature = "diag")]
static REPORTED_USED: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "diag")]
static POLLS: AtomicU32 = AtomicU32::new(0);

/// Diag, startup only (before any dispatch): every service stack painted word by word.
#[cfg(feature = "diag")]
fn paint_stacks() {
    for stack in &STACKS {
        let base = stack.0.get() as usize;
        for at in (base..base + STACK.bytes).step_by(8) {
            // SAFETY: no dispatch has claimed a stack yet; the words are this static's own.
            unsafe { core::ptr::write_volatile(at as *mut u64, crate::model::thread_stacks::PAINT) };
        }
    }
}

/// Diag: the most bytes used of any service stack so far (the paint's lowest overwritten word), read, never written.
#[cfg(feature = "diag")]
fn stacks_used() -> usize {
    STACKS.iter().map(|stack| {
        let base = stack.0.get() as usize;
        let mut at = base;
        // SAFETY: a static's words, read only; a poll on the other stack only ever lowers what is read here.
        while at < base + STACK.bytes && unsafe { core::ptr::read_volatile(at as *const u64) } == crate::model::thread_stacks::PAINT {
            at += 8;
        }
        crate::model::thread_stacks::used(STACK.bytes, at - base)
    }).max().unwrap_or(0)
}

/// Lock-free: a compare-and-swap on `CLAIMED` that retries only when another claim or release changed it meanwhile.
fn claim() -> Option<usize> {
    let mut held = CLAIMED.load(Ordering::Acquire);
    loop {
        let free = crate::model::thread_stacks::free_stack(held, STACK.count)?;
        match CLAIMED.compare_exchange_weak(held, held | 1 << free, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {
                PEAK.fetch_max((held | 1 << free).count_ones(), Ordering::Relaxed);
                return Some(free);
            }
            Err(now) => held = now,
        }
    }
}

fn release(stack: usize) {
    CLAIMED.fetch_and(!(1 << stack), Ordering::Release);
}

fn top(stack: usize) -> usize {
    STACKS[stack].0.get() as usize + STACK.bytes
}

/// Startup only, after character_roster's existing pristine instruction checks
/// and before game callbacks. No second650010 hook is installed.
pub(super) unsafe fn install(base: usize) {
    #[cfg(feature = "diag")]
    paint_stacks();
    let mut original = std::ptr::null_mut();
    skyline::hooks::A64HookFunction(
        (base + 0x650010) as *const _,
        training_mod_native_service_entry as *const _,
        &mut original,
    );
    assert!(!original.is_null(), "native service trampoline unavailable");
    ORIGINAL.store(original as usize, Ordering::Release);
}

/// Native functor651640 passes only FE in x0 and tail-branches to650010. The
/// assembly entry supplies the actual pre-Rust context in x1. This is a C ABI
/// function boundary, unlike the mid-function ordinary publication gate.
#[no_mangle]
unsafe extern "C" fn training_mod_native_service_dispatch(
    entry: *mut u8,
    context: &ServiceEntryContext,
) -> u64 {
    crate::runtime::threads::headroom("native_service");
    // rc.7 FLAG 6: the plugin's pre-service poll runs on a service stack, held only for the poll. With every stack in use
    // it skips this one poll, which a plugin consumer returning early already does; nothing waits for a stack, and the
    // gates, the game's call and the leaves below always run, on the game's stack (their paths fit it: SERVICE_STACK).
    match claim() {
        Some(stack) => {
            training_mod_native_service_on_stack(entry, context as *const _ as usize,
                training_mod_native_service_pre as *const () as usize, top(stack));
            release(stack);
        }
        None => {
            EXHAUSTED.fetch_add(1, Ordering::Relaxed);
        }
    }
    if !training_mod_native_service_gates(entry) {
        return 0;
    }
    let original: unsafe extern "C" fn(*mut u8) -> u64 =
        std::mem::transmute(ORIGINAL.load(Ordering::Acquire));
    let result = original(entry);
    training_mod_native_service_post(entry, result)
}

/// The plugin's pre-service poll, on a service stack (its path is SERVICE_STACK's `static_bound`).
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn training_mod_native_service_pre(entry: *mut u8, context: &ServiceEntryContext) -> u64 {
    #[cfg(feature = "console-swap-trace")]
    let native_sp = context.native_sp() as u64;
    #[cfg(not(feature = "console-swap-trace"))]
    let native_sp = 0;
    crate::runtime::roster::practice_trace::event(220, [entry as u64, context.caller_lr() as u64, native_sp, 0, 0]);
    #[cfg(feature = "diag")]
    {
        let (exhausted, peak) = (EXHAUSTED.swap(0, Ordering::Relaxed), PEAK.load(Ordering::Relaxed));
        if exhausted != 0 || peak > REPORTED_PEAK.swap(peak, Ordering::Relaxed) {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[NativeService] service stacks peak={} of {} skipped_polls={}", peak, STACK.count, exhausted));
        }
        // Every 512th poll: the stacks' measured use, one line when it grew.
        if POLLS.fetch_add(1, Ordering::Relaxed) % 512 == 0 {
            let used = stacks_used();
            if used > REPORTED_USED.fetch_max(used, Ordering::Relaxed) {
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[NativeService] service stack used={} of {} static_bound={}", used, STACK.bytes, STACK.static_bound));
            }
        }
    }
    // A plugin consumer returning early ends only pre-dispatch, never native work.
    super::service(context, entry);
    crate::runtime::roster::practice_trace::event(221, [entry as u64, 0, 0, 0, 0]);
    1
}

/// The owners' gates on the game's stack: false refuses the game's call this time (a nested callback inside INSTALL, a
/// stale or refused track), as before.
#[inline(never)]
unsafe fn training_mod_native_service_gates(entry: *mut u8) -> bool {
    #[cfg(feature = "mii-ordinary-player-alpha")]
    if !super::mii_ordinary_player::enter_native(entry) {
        return false;
    }
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    if !super::mii_ordinary_cpu::enter_native(entry) {return false;}
    #[cfg(feature="mii-managed-helper-alpha")]
    if !super::mii_managed_helper::enter_native(entry) {return false;}
    let _ = entry;
    true
}

/// The owners leave after the game's call, on the game's stack; it never waits for a service stack.
#[no_mangle]
#[inline(never)]
unsafe extern "C" fn training_mod_native_service_post(entry: *mut u8, result: u64) -> u64 {
    crate::runtime::roster::practice_trace::event(222, [entry as u64, result, 0, 0, 0]);
    #[cfg(feature="mii-managed-helper-alpha")]
    super::mii_managed_helper::leave_native(entry);
    #[cfg(feature = "mii-ordinary-player-alpha")]
    super::mii_ordinary_player::leave_native(entry);
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    super::mii_ordinary_cpu::leave_native(entry);
    crate::runtime::roster::practice_trace::event(223, [entry as u64, result, 0, 0, 0]);
    result
}
