//! Package 6 Mii fighters, root-crate pure policy files (the Mii crate itself is `engine::mii`): the HUD portrait policy, the catalog
//! menu model (`mii_menu`) and the resume tickets the editor bridge keeps (`resume`, shared with package 4's named-macro panel).
//!
//! Carried from the fork (stage m5o) by tools/carry_pkg6.py (docs/provenance/pkg6-carry-manifest.json); never edited by hand.

pub mod mii_hud_portrait_policy;
pub mod mii_menu;
pub mod resume;