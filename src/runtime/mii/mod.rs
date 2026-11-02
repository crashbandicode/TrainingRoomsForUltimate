//! Package 6, Mii game side (worker B): the native Mii access, the identity binding and the software keyboard (carried by
//! `tools/carry_pkg6.py`). The files that are children of the roster cycle's `character_roster` (`mii_backend`, `mii_ordinary_*`,
//! `mii_hud_portrait`, ...) are declared by it. Switch only.

pub(crate) mod mii_identity_binding;
pub(crate) mod native_keyboard;
