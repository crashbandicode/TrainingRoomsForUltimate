//! Package 6, Mii game side (worker B): the native Mii access, the identity binding and the software keyboard (carried by
//! `tools/carry_pkg6.py`). The files that are children of the roster cycle's `character_roster` (`mii_backend`, `mii_ordinary_*`,
//! `mii_hud_portrait`, ...) are declared by it. Switch only.

pub(crate) mod mii_identity_binding;
pub(crate) mod native_keyboard;
#[cfg(feature = "mii-editor-alpha")]
pub(crate) mod mii_editor_bridge;
pub(crate) mod mii_native_access;
/// The Mii identity / saved-file / full-configuration diag probes (Eden evidence for the Mii rows), each under its own feature.
#[cfg(feature = "mii-identity-probe")]
pub(crate) mod mii_identity_probe;
#[cfg(feature = "mii-saved-file-probe")]
pub(crate) mod mii_saved_file_probe;
#[cfg(feature = "mii-fullconfig-probe")]
pub(crate) mod mii_fullconfig_probe;

/// Start the Mii files that are not started by the roster's own `init` (it runs first): the editor bridge. Called once from boot.
pub(crate) fn init() {
    #[cfg(feature = "mii-editor-alpha")]
    mii_editor_bridge::init();
    #[cfg(feature = "mii-pool-observer")]
    crate::runtime::roster::character_roster::mii_pool_observer::init();
    #[cfg(feature = "mii-identity-probe")]
    mii_identity_probe::init();
    #[cfg(feature = "mii-saved-file-probe")]
    mii_saved_file_probe::init();
    #[cfg(feature = "mii-fullconfig-probe")]
    mii_fullconfig_probe::init();
}

#[cfg(feature = "mii-saved-alpha")]
pub(crate) mod menu_handoff;
