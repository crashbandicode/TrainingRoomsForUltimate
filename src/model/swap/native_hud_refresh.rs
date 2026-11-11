//! The ordinary Training roster refresh needs a new portrait, not the native
//! Pokemon transformation flash. Event 0x1d writes both: 13.0.5 `1363c6c` sets
//! the animation byte, and `139df68..139df90` consumes it to play `pokemon`.
//! Runtime142's exact-store experiment preserved the portrait update without
//! the white/purple flash in both directions. No texture or animation is owned here.

use crate::engine::seams::native_fields;

unsafe fn animation_flag(hud: usize, owner: u32) -> Option<usize> {
    if owner > 3 || !native_fields::object(hud) {
        return None;
    }
    // The native owner-to-presentation mapping is independent of fighter kind.
    let row = (0..8).find(|&row| {
        native_fields::read::<u32>(hud, row * 0x1d8 + 0xd68) == Some(owner)
    })?;
    let slot = native_fields::read::<u32>(hud, row * 0x1d8 + 0xd60)?;
    if slot >= 8 {
        return None;
    }
    native_fields::child(hud, 0xb0)?.checked_add(slot as usize * 0x2d0 + 0x23b)
}

/// Deliver our synthetic event, preserving a transformation already requested
/// by the game. Clear only a newly set boolean on the same owner's same record.
/// No address survives this synchronous call; no per-frame work or allocation.
///
/// # Safety
/// Called only in ordinary Training on the native entry callback, with the HUD
/// listener held by the dispatch-depth protocol. The HUD and its child table
/// must stay live through `refresh`; that closure invokes only this listener's
/// native event functor, not a scene transition or arbitrary user callback.
/// Null structural checks do not themselves establish that lifetime.
pub(crate) unsafe fn without_roster_animation(hud: usize, owner: u32, refresh: impl FnOnce()) {
    let idle = animation_flag(hud, owner)
        .filter(|&flag| native_fields::read::<u8>(flag, 0) == Some(0));
    refresh();
    if let Some(flag) = idle {
        if animation_flag(hud, owner) == Some(flag)
            && native_fields::read::<u8>(flag, 0) == Some(1)
        {
            native_fields::write_unchecked(flag, 0, 0u8);
        }
    }
}

