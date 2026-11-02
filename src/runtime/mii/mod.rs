//! Package 6, Mii game side (worker B): the native Mii access, the identity binding and the software keyboard (carried by
//! `tools/carry_pkg6.py`). The files that are children of the roster cycle's `character_roster` (`mii_backend`, `mii_ordinary_*`,
//! `mii_hud_portrait`, ...) are declared by it. Switch only.

pub(crate) mod mii_identity_binding;
pub(crate) mod native_keyboard;
#[cfg(feature = "mii-editor-alpha")]
pub(crate) mod mii_editor_bridge;
pub(crate) mod mii_native_access;

/// Start the Mii files that are not started by the roster's own `init` (it runs first): the editor bridge. Called once from boot.
pub(crate) fn init() {
    #[cfg(feature = "mii-editor-alpha")]
    mii_editor_bridge::init();
    #[cfg(feature = "mii-pool-observer")]
    crate::runtime::roster::character_roster::mii_pool_observer::init();
}
