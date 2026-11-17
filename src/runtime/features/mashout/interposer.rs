//! Guarded FighterControl data slots. Only the existing worker installs/restores.
use super::*;
use crate::runtime::features::mashout;

const TABLE: usize = 0x4f85de0;
const START_SLOT: usize = 0x4f85e58;
const END_SLOT: usize = 0x4f85ea8;
static ORIGINAL_START: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_END: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static LOST: AtomicBool = AtomicBool::new(false);

unsafe fn slot(offset: usize) -> &'static AtomicUsize {
    &*((game::text_base() + offset) as *const AtomicUsize)
}
fn targets() -> [usize; 2] {
    [start as *const () as usize, end as *const () as usize]
}
pub(super) unsafe fn reader_table(table: usize) -> bool {
    table == game::text_base() + TABLE
        && INSTALLED.load(Ordering::Acquire)
        && AtomicUsize::load(slot(START_SLOT), Ordering::Acquire) == targets()[0]
        && AtomicUsize::load(slot(END_SLOT), Ordering::Acquire) == targets()[1]
}

/// Called outside every hook, before the worker lock. Foreign values are never overwritten.
pub(super) fn service() {
    // M3 Set observer: same owner site, restore-first when its gate is closed.
    super::clatter_set::service();
    let wanted = WANTED.load(Ordering::Acquire)
        && READY.load(Ordering::Acquire)
        && scene::current().is_some();
    let originals = [game::text_base() + 0x6b9150, game::text_base() + 0x3f8230];
    let wrappers = targets();
    let offsets = [START_SLOT, END_SLOT];
    if !wanted || LOST.load(Ordering::Acquire) {
        SLOTS_READY.store(false, Ordering::Release);
        ARMED.store(0, Ordering::Release);
        if INSTALLED.swap(false, Ordering::AcqRel) {
            for i in 0..2 {
                let found = unsafe { slot(offsets[i]) };
                if found
                    .compare_exchange(
                        wrappers[i],
                        originals[i],
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_err()
                    || found.load(Ordering::Acquire) != originals[i]
                {
                    LOST.store(true, Ordering::Release);
                }
            }
            crate::runtime::diag_log::diagnostic(format_args!(
                "[Mashout] slots restored={} start={:#x} end={:#x}",
                !LOST.load(Ordering::Acquire),
                unsafe { AtomicUsize::load(slot(START_SLOT), Ordering::Acquire) },
                unsafe { AtomicUsize::load(slot(END_SLOT), Ordering::Acquire) }
            ));
        }
        return;
    }
    if INSTALLED.load(Ordering::Acquire) {
        if unsafe { reader_table(game::text_base() + TABLE) } {
            return;
        }
        SLOTS_READY.store(false, Ordering::Release);
        ARMED.store(0, Ordering::Release);
        LOST.store(true, Ordering::Release);
        // Next service restores any still-owned slot, preserving the foreign replacement.
        crate::runtime::diag_log::diagnostic(format_args!("[Mashout] slots ownership_lost"));
        return;
    }
    ORIGINAL_START.store(originals[0], Ordering::Release);
    ORIGINAL_END.store(originals[1], Ordering::Release);
    let mut installed = 0;
    for i in 0..2 {
        let found = unsafe { slot(offsets[i]) };
        if found
            .compare_exchange(
                originals[i],
                wrappers[i],
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
            || found.load(Ordering::Acquire) != wrappers[i]
        {
            // Includes a readback race: rollback only exact pointers we own.
            for j in 0..=i {
                let _ = unsafe { slot(offsets[j]) }.compare_exchange(
                    wrappers[j],
                    originals[j],
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
            }
            LOST.store(true, Ordering::Release);
            break;
        }
        installed += 1;
    }
    let ready = installed == 2
        && offsets
            .iter()
            .enumerate()
            .all(|(i, offset)| unsafe { slot(*offset).load(Ordering::Acquire) == wrappers[i] });
    if !ready {
        for i in 0..2 {
            let _ = unsafe { slot(offsets[i]) }.compare_exchange(
                wrappers[i],
                originals[i],
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        LOST.store(true, Ordering::Release);
    }
    INSTALLED.store(ready, Ordering::Release);
    SLOTS_READY.store(ready, Ordering::Release);
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Mashout] slots installed={ready} start={:#x} end={:#x}",
        unsafe { AtomicUsize::load(slot(START_SLOT), Ordering::Acquire) },
        unsafe { AtomicUsize::load(slot(END_SLOT), Ordering::Acquire) }
    ));
}

// Incoming LR is saved before any compiler prologue/call. All integer and FP
// arguments reach the original unchanged; its return registers survive the observer.
#[unsafe(naked)]
unsafe extern "C" fn start(
    _control: usize,
    _source: i32,
    _index: i32,
    _c: bool,
    _d: bool,
    _remaining: f32,
    _automatic: f32,
    _manual: f32,
) -> u64 {
    core::arch::naked_asm!(
        "stp x29, x30, [sp, #-128]!",
        "stp x19, x20, [sp, #16]",
        "mov x29, sp",
        "mov x19, x0",
        "mov x20, x30",
        "adrp x16, {original}",
        "add x16, x16, :lo12:{original}",
        "ldar x16, [x16]",
        "blr x16",
        "stp x0, x1, [sp, #32]",
        "stp q0, q1, [sp, #48]",
        "stp q2, q3, [sp, #80]",
        "mov x0, x19",
        "mov x1, x20",
        "bl {observe}",
        "ldp q2, q3, [sp, #80]",
        "ldp q0, q1, [sp, #48]",
        "ldp x0, x1, [sp, #32]",
        "ldp x19, x20, [sp, #16]",
        "ldp x29, x30, [sp], #128",
        "ret",
        original = sym ORIGINAL_START,
        observe = sym started,
    )
}
unsafe extern "C" fn started(control: usize, caller: usize) {
    if caller == game::text_base() + 0x639d44 {
        mashout::throw_seed_after(control);
    }
}

unsafe extern "C" fn end(control: usize, index: i32) -> u64 {
    let token = mashout::end_token(control, index);
    let original: unsafe extern "C" fn(usize, i32) -> u64 =
        core::mem::transmute(ORIGINAL_END.load(Ordering::Acquire));
    let result = original(control, index);
    if let Some(token) = token {
        mashout::end_after(control, token);
    }
    result
}
