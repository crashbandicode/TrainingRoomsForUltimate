//! Version-checked scene observations for the optional waiting-room adapter.

use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::hooks::{getRegionAddress, Region};
// Globals are pinned by the verified main image; popup objects are borrowed on
// the UI callback. Preserve the existing nonzero-only child checks below.
use crate::engine::seams::native_fields::read_unchecked as read;

static BASE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub initialized: bool,
    pub melee: u8,
    pub mode: u32,
}

pub fn init() -> bool {
    // The game identity is checked once, before anything installs, by the version guard in
    // `main` (13.0.5 main, `crate::model::MAIN_BUILD_ID`), so the globals read by `current()` are
    // fixed offsets of that build. The former `is_training_mode` prologue comparison here was a
    // stand-in for that check and moved into the guard (WORKPLAN §2.6b).
    unsafe {
        let base = getRegionAddress(Region::Text) as usize;
        // AppPopupManager's is_busy Lua binding: singleton, state fields and
        // presentation-idle query. Keep this independent of pane animation guesses.
        const POPUP_BINDING: [u32; 10] = [
            0x9000fe48, 0xf9441108, 0xf9400514, 0xf9408a88, 0xaa0003f3, 0xf9404100, 0x97ae214f,
            0xb940fa88, 0x7100011f, 0x39450a89,
        ];
        // Seam row `game_mode.popup_binding` (`native_seams::rows_a5`, WP-A5). The `game_mode` group is
        // authoritative (SEAMS-ALL): the row decides; the words are the shadow compare.
        if !crate::runtime::seams::seam_install::site("game_mode.popup_binding",
            std::slice::from_raw_parts((base + 0x33678dc) as *const u32, 10) == POPUP_BINDING)
        {
            return false;
        }
        BASE.store(base, Ordering::Release);
    }
    true
}

pub fn current() -> Option<Mode> {
    let base = BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    unsafe {
        Some(Mode {
            initialized: read::<u8>(base, 0x53144d8) & 1 != 0,
            melee: read(base, 0x52c51b0),
            mode: read(base, 0x53050f0),
        })
    }
}

/// Read the game's popup-manager busy condition on the UI draw thread.
/// Missing manager state is unknown; callers must not capture input then.
pub unsafe fn popup_busy() -> Option<bool> {
    let base = BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    unsafe fn pointer(at: usize) -> Option<usize> {
        let value: usize = read(at, 0);
        (value != 0).then_some(value)
    }
    let manager = pointer(base + 0x532f820)?;
    let state = pointer(manager + 8)?;
    if read::<u32>(state, 0xf8) != 0
        || read::<u8>(state, 0x142) != 0
    {
        return Some(true);
    }
    let presentation = pointer(pointer(state + 0x110)? + 0x80)?;
    // The native binding calls this read-only presentation/animation query. Seam row `game_mode.popup_idle`
    // (Called; checked once, on the first query): unguarded before WP-A5, so the shadow decision is `true`.
    static IDLE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0); // 0 unchecked, 1 usable, 2 refused
    let usable = match IDLE.load(Ordering::Acquire) {
        0 => {
            let usable = crate::runtime::seams::seam_install::site("game_mode.popup_idle", true);
            IDLE.store(if usable { 1 } else { 2 }, Ordering::Release);
            usable
        }
        state => state == 1,
    };
    if !usable {
        return None;
    }
    let idle: unsafe extern "C" fn(usize) -> u32 = std::mem::transmute(base + 0x1eefe30);
    Some(idle(presentation) & 1 == 0)
}
