//! Native unlink-site installation. The qualified missing-parent guard
//! (feature native-unlink-guard) is installed independently of the optional
//! diagnostic observer (feature native-unlink-probe). Observer preparation can
//! only disable observation; it never prevents the guard. Guard semantics are
//! unchanged and live in native_unlink_guard::store. The writer is unknown.
use skyline::hooks::InlineCtx;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "native-unlink-guard")]
use std::sync::atomic::AtomicU64;

static INSTALLED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "native-unlink-guard")]
static GUARD_EVENTS: AtomicU64 = AtomicU64::new(0);

/// Walker and destructor windows: hooked instruction plus its neighbours
/// (words start at offset - 4). The guard was qualified only with all of these
/// matching, so they remain its refusal precondition.
const OBSERVED_WINDOWS: [(usize, [u32; 3]); 9] = [
    (0x38958a4, [0xaa0003f3, 0xf8408e80, 0xb4000040]),
    (0x38958e4, [0x91002129, 0xf9000128, 0xa941a668]),
    (0x3895148, [0x92800014, 0xaa1403e9, 0xaa1503ea]),
    (0x38951c4, [0x940001b4, 0xa941a2a9, 0xb4000069]),
    (0x3896bbc, [0xf9001668, 0x39410268, 0xb900327c]),
    (0x38952cc, [0xf90002bf, 0xa9017edf, 0xa9007edf]),
    (0x3895904, [0xf900027f, 0xa9017e9f, 0xa9007e9f]),
    (0x3895874, [0xf900027f, 0xa9017e9f, 0xa9007e9f]),
    (0x38951fc, [0xf90002bf, 0xa9017edf, 0xa9007edf]),
];

#[cfg(feature = "native-unlink-guard")]
const GUARDED_STORES: [(usize, [u32; 3]); 8] = [
    (0x38958e4, [0x91002129, 0xf9000128, 0xa941a668]),
    (0x38958fc, [0xf9400269, 0xf9000928, 0xf900027f]),
    (0x3895854, [0x91002129, 0xf9000128, 0xa941a668]),
    (0x389586c, [0xf9400269, 0xf9000928, 0xf900027f]),
    (0x38951dc, [0x91002129, 0xf9000128, 0xa941a6a8]),
    (0x38951f4, [0xf94002a9, 0xf9000928, 0xf90002bf]),
    (0x38952a8, [0x91002129, 0xf9000128, 0xa941a6a8]),
    (0x38952c4, [0x91004129, 0xf9000128, 0xf90002bf]),
];

/// Seam rows for the windows above and the hooked sites (`native_seams::rows_a5`).
const UNLINK_SEAMS: &[&str] = &[
    "unlink.observed_38958a4", "unlink.leave_window", "unlink.observed_3895148", "unlink.observed_38951c4",
    "unlink.observed_3896bbc", "unlink.observed_38952cc", "unlink.observed_3895904", "unlink.observed_3895874",
    "unlink.observed_38951fc", "unlink.leave",
    #[cfg(feature = "native-unlink-guard")] "unlink.recursive_last_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.recursive_last",
    #[cfg(feature = "native-unlink-guard")] "unlink.sister_first_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.sister_first",
    #[cfg(feature = "native-unlink-guard")] "unlink.sister_last_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.sister_last",
    #[cfg(feature = "native-unlink-guard")] "unlink.header_first_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.header_first",
    #[cfg(feature = "native-unlink-guard")] "unlink.header_last_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.header_last",
    #[cfg(feature = "native-unlink-guard")] "unlink.vector_first_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.vector_first",
    #[cfg(feature = "native-unlink-guard")] "unlink.vector_last_window",
    #[cfg(feature = "native-unlink-guard")] "unlink.vector_last",
];

fn mismatch(main: usize, windows: &[(usize, [u32; 3])]) -> Option<usize> {
    windows.iter().find_map(|(offset, words)| {
        words.iter().enumerate().any(|(index, word)| {
            (unsafe { std::ptr::read_volatile((main + offset - 4 + index * 4) as *const u32) }) != *word
        }).then_some(*offset)
    })
}

/// Shared walker-leave site: observe first (inert unless the probe is ready),
/// then execute the displaced store through the guard, as before.
unsafe extern "C" fn leave(ctx: &InlineCtx) {
    #[cfg(feature = "native-unlink-probe")]
    crate::runtime::roster::native_unlink_probe::observe_leave(ctx);
    #[cfg(feature = "native-unlink-guard")]
    guarded_store(ctx, 0x38958e4, false, false);
}

#[cfg(feature = "native-unlink-guard")]
unsafe fn guarded_store(ctx: &InlineCtx, site: u64, add_sixteen: bool, header_first: bool) {
    let node = ctx.registers[if matches!(site, 0x38951dc | 0x38951f4 | 0x38952a8 | 0x38952c4) { 21 } else { 19 }].x();
    let address = ctx.registers[9].x().wrapping_add(if add_sixteen { 16 } else { 0 });
    let value = ctx.registers[8].x();
    let action = crate::engine::seams::native_unlink_guard::store(address, value, node,
        if header_first { ctx.registers[19].x() } else { 0 }, header_first);
    if action != 0 {
        // Diagnostic accounting only; no formatting or file I/O in the guard.
        // A no-op unless console-practice-trace is compiled in.
        let count = GUARD_EVENTS.fetch_add(1, Ordering::Relaxed) + 1;
        if count <= 64 {
            crate::runtime::roster::practice_trace::checkpoint(1209, [site, node, address, action, count]);
        }
    }
}

#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn recursive_last_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x38958fc, true, false); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn sister_first_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x3895854, false, false); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn sister_last_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x389586c, true, false); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn header_first_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x38951dc, false, true); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn header_last_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x38951f4, true, false); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn vector_first_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x38952a8, false, false); }
#[cfg(feature = "native-unlink-guard")]
unsafe extern "C" fn vector_last_guard(ctx: &InlineCtx) { guarded_store(ctx, 0x38952c4, false, false); }

unsafe fn hook(main: usize, offset: usize, callback: unsafe extern "C" fn(&InlineCtx)) {
    skyline::hooks::A64InlineHook((main + offset) as *const _, callback as *const _);
    if std::ptr::read_volatile((main + offset) as *const u32) & 0xfc000000 != 0x14000000 {
        std::process::abort(); // Long patch can overwrite internal branch targets.
    }
}

pub fn init() {
    if INSTALLED.swap(true, Ordering::AcqRel) { return; }
    let main = unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize };
    // x16/x17 and NZCV are dead at every accepted site. Skyline does not
    // preserve them. Require the surrounding store/load windows as well as
    // each displaced word, because the post-store meaning depends on them.
    // Same refusal set as before; a mismatch installs nothing at all.
    // Seam rows (`native_seams::rows_a5`): each 12-byte window starts at site-4; all rows are checked before
    // the first NOP or hook. The group is authoritative (SEAMS-ALL): the rows decide, and the exact windows
    // stay the guard's qualified refusal precondition (OBSERVED_WINDOWS doc): the `observed_*` and `*_window`
    // rows are `Relied` (word 0 only), so the rows alone would accept a hook in a window's later words.
    let legacy = match mismatch(main, &OBSERVED_WINDOWS) {
        Some(offset) => {
            crate::engine::compat::logging::diagnostic(format_args!("[NativeUnlink] refused window at {offset:#x}"));
            false
        }
        None => true,
    };
    #[cfg(feature = "native-unlink-guard")]
    let legacy = legacy && match mismatch(main, &GUARDED_STORES) {
        Some(offset) => {
            crate::engine::compat::logging::diagnostic(format_args!("[NativeUnlink] guard refused window at {offset:#x}"));
            false
        }
        None => true,
    };
    if !(crate::runtime::seams::seam_install::gate("native_unlink_install", UNLINK_SEAMS, legacy) && legacy) {
        if legacy {
            crate::engine::compat::logging::diagnostic(format_args!("[NativeUnlink] seam guard refused"));
        }
        return;
    }
    // Optional diagnostics only. File, SD or build-identity failure disables
    // the observer and never prevents the qualified guard below.
    #[cfg(feature = "native-unlink-probe")]
    let observer = crate::runtime::seams::seam_install::gate("native_unlink_probe", &[
        "unlink_probe.walk_38958a4", "unlink_probe.walk_3895148", "unlink_probe.walk_38951c4",
        "unlink_probe.destroy_3896bbc", "unlink_probe.walk_38952cc", "unlink_probe.walk_3895904",
        "unlink_probe.walk_3895874", "unlink_probe.walk_38951fc"], crate::runtime::roster::native_unlink_probe::prepare(main));
    #[cfg(not(feature = "native-unlink-probe"))]
    let observer = false;
    #[cfg(feature = "native-unlink-guard")]
    let shared = true;
    #[cfg(not(feature = "native-unlink-guard"))]
    let shared = observer;
    unsafe {
        #[cfg(feature = "native-unlink-guard")]
        for (offset, _) in GUARDED_STORES {
            // The callback executes the displaced STR conditionally. NOP is
            // copied into Skyline's trampoline so the STR never executes twice.
            // Init is quiescent; failure must not leave partially patched code running.
            if skyline::patching::Patch::in_text(offset).nop().is_err() {
                std::process::abort();
            }
        }
        if shared {
            hook(main, 0x38958e4, leave);
        }
        #[cfg(feature = "native-unlink-probe")]
        if observer {
            for (offset, callback) in crate::runtime::roster::native_unlink_probe::observer_hooks() {
                hook(main, offset, callback);
            }
        }
        #[cfg(feature = "native-unlink-guard")]
        for (offset, callback) in [
            (0x38958fc, recursive_last_guard as unsafe extern "C" fn(&InlineCtx)),
            (0x3895854, sister_first_guard), (0x389586c, sister_last_guard),
            (0x38951dc, header_first_guard), (0x38951f4, header_last_guard),
            (0x38952a8, vector_first_guard), (0x38952c4, vector_last_guard),
        ] {
            hook(main, offset, callback);
        }
    }
    #[cfg(feature = "native-unlink-probe")]
    if observer {
        crate::runtime::roster::native_unlink_probe::publish_ready();
    }
    #[cfg(feature = "native-unlink-guard")]
    crate::engine::compat::logging::diagnostic(format_args!(
        "[NativeUnlink] experimental missing-parent guard enabled; observer={observer}; writer unknown"));
}
