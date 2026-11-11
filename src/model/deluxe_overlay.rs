//! Online Deluxe's window state (rc.9 C4, `artifacts/phase23/TASK-rc9-chord-fixes.md`): Deluxe exports
//! `ssbu_online_deluxe_overlay_state_v1`, and the chords stand their D-pad rows down while its window is drawn
//! (`chords::passes_row`). Pure: the ABI, the answer's reading and the lookup's retry rule; `runtime::deluxe_overlay`
//! looks the export up and calls it on Player 1's poll.

/// The export's name, NUL-terminated for `nn::ro::LookupSymbol`.
pub const SYMBOL: &[u8] = b"ssbu_online_deluxe_overlay_state_v1\0";

/// The export's out-struct (`#[repr(C)]`, 8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeluxeOverlayStateV1 {
    pub size: u32,
    pub flags: u32,
}

/// Deluxe's window is drawn (any page).
pub const DELUXE_WINDOW_VISIBLE: u32 = 1 << 0;
/// The shown page has selectable or editable rows (D-pad and ZR + Left/Right act).
pub const DELUXE_ROWS_PAGE: u32 = 1 << 1;
/// The Debug view is open.
pub const DELUXE_DEBUG_VIEW: u32 = 1 << 2;

/// `ssbu_online_deluxe_overlay_state_v1(out) -> bool`: fills `*out` and returns true; false if `out` is null. Lock-free.
pub type OverlayStateExport = unsafe extern "C" fn(*mut DeluxeOverlayStateV1) -> bool;

/// Whether Deluxe's window is drawn: the export answered with at least the v1 struct and the visible flag. No export
/// (an older Deluxe, or none on the card) is "no window".
///
/// # Safety
/// `export` is Deluxe's `ssbu_online_deluxe_overlay_state_v1`, or another function with that ABI.
pub unsafe fn window_visible(export: Option<OverlayStateExport>) -> bool {
    let mut state = DeluxeOverlayStateV1 { size: core::mem::size_of::<DeluxeOverlayStateV1>() as u32, flags: 0 };
    let answered = export.is_some_and(|f| unsafe { f(&mut state) });
    answered && state.size >= 8 && state.flags & DELUXE_WINDOW_VISIBLE != 0
}

/// The lookup's last try, before any: the first ask always looks.
pub const NEVER_TRIED: u64 = u64::MAX;

/// A missed lookup is tried again at most once a second (the contract: never block on it).
pub const fn retry_due(last_try_ms: u64, now_ms: u64) -> bool {
    last_try_ms == NEVER_TRIED || now_ms.saturating_sub(last_try_ms) >= 1000
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    unsafe extern "C" fn shown(out: *mut DeluxeOverlayStateV1) -> bool {
        unsafe { out.write(DeluxeOverlayStateV1 { size: 8, flags: DELUXE_WINDOW_VISIBLE | DELUXE_ROWS_PAGE }) };
        true
    }
    unsafe extern "C" fn hidden(out: *mut DeluxeOverlayStateV1) -> bool {
        unsafe { out.write(DeluxeOverlayStateV1 { size: 8, flags: 0 }) };
        true
    }
    unsafe extern "C" fn refused(_: *mut DeluxeOverlayStateV1) -> bool {
        false
    }
    unsafe extern "C" fn short(out: *mut DeluxeOverlayStateV1) -> bool {
        unsafe { out.write(DeluxeOverlayStateV1 { size: 4, flags: DELUXE_WINDOW_VISIBLE }) };
        true
    }

    /// The contract (`TASK-rc9-chord-fixes.md`): visible only on a true answer with the v1 size and the visible flag; a
    /// missing export is no window; a miss is looked up again at most once a second.
    #[test]
    fn deluxe_window_state_reads_the_contract() {
        assert_eq!(core::mem::size_of::<DeluxeOverlayStateV1>(), 8);
        assert!(unsafe { window_visible(Some(shown)) });
        assert!(!unsafe { window_visible(Some(hidden)) });
        assert!(!unsafe { window_visible(Some(refused)) });
        assert!(!unsafe { window_visible(Some(short)) });
        assert!(!unsafe { window_visible(None) }, "an older Deluxe, or none: no window");
        assert_eq!(SYMBOL.last(), Some(&0));
        assert!(retry_due(NEVER_TRIED, 0));
        assert!(!retry_due(5_000, 5_999));
        assert!(retry_due(5_000, 6_000));
    }
    }
}
