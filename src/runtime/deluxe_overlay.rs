//! Online Deluxe's window state for the chords (rc.9 C4): its export `ssbu_online_deluxe_overlay_state_v1`
//! (`model::deluxe_overlay`), looked up by name on the first ask and again after a miss at most once a second, then
//! called on each of Player 1's polls. Lock-free and allocation-free: atomics only.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::model::deluxe_overlay::{self, OverlayStateExport, NEVER_TRIED, SYMBOL};

/// The export's address once found (0 until then).
static ADDRESS: AtomicUsize = AtomicUsize::new(0);
/// When the lookup last ran and missed (`NEVER_TRIED` before the first).
static LAST_TRY: AtomicU64 = AtomicU64::new(NEVER_TRIED);
#[cfg(feature = "diag")]
static LAST_VISIBLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);

fn export() -> Option<OverlayStateExport> {
    let mut address = ADDRESS.load(Ordering::Acquire);
    if address == 0 {
        let now = crate::runtime::clock::uptime_ms();
        if !deluxe_overlay::retry_due(LAST_TRY.load(Ordering::Acquire), now) {
            return None;
        }
        LAST_TRY.store(now, Ordering::Release);
        // SAFETY: a plain symbol lookup with a NUL-terminated name; it writes one usize.
        unsafe { skyline::nn::ro::LookupSymbol(&mut address, SYMBOL.as_ptr()) };
        if address == 0 {
            return None;
        }
        ADDRESS.store(address, Ordering::Release);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!("[Chords] deluxe_overlay export={address:#x}"));
    }
    // SAFETY: a found address is Deluxe's export, whose ABI `OverlayStateExport` declares (the contract in
    // `TASK-rc9-chord-fixes.md`).
    Some(unsafe { core::mem::transmute::<usize, OverlayStateExport>(address) })
}

/// Whether Online Deluxe's window is drawn now. No export (an older Deluxe, or none on the card) is no window.
pub fn window_visible() -> bool {
    // SAFETY: `export` is Deluxe's own export or none.
    let visible = unsafe { deluxe_overlay::window_visible(export()) };
    #[cfg(feature = "diag")]
    if LAST_VISIBLE.swap(u8::from(visible), Ordering::AcqRel) != u8::from(visible) {
        crate::runtime::diag_log::diagnostic(format_args!("[Chords] deluxe_overlay visible={visible}"));
    }
    visible
}
