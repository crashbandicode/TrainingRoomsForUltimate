//! The fork's mixed modules as the carried game-facing files name them (package 6, docs/PACKAGE-6-PLAN.md section 2): `common`, the
//! typed `MENU` and the `training_mod_sync` helpers. Our code, small, and the only place the carried files' expectations of those
//! modules are met (the carried files are never edited). Each item forwards to the clean room's own provider; nothing here holds
//! game state of its own except the typed menu view.
//!
//! The manifest's module map (`docs/provenance/pkg6-carry-manifest.json`, `modules.root.common` / `modules.sync` /
//! `modules.consts.MENU`) points the carried paths here.

pub(crate) mod common;
pub(crate) mod consts;
pub(crate) mod menu;
pub(crate) mod replay_preview;
pub(crate) mod sync;
#[cfg(any(all(feature = "diag", feature = "mii-editor-alpha"), feature = "automation-results-observer"))]
pub(crate) mod filesystem;
pub(crate) mod hazard_manager;
pub(crate) mod input;
pub(crate) mod menu_app;
