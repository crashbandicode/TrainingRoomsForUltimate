//! Adapters for the names the carried fork code expects from the fork's mixed files (package 6, docs/PACKAGE-6-PLAN.md (removed after v0.2.0-rc.1; see that tag) section 2).
//! Our code, small, and the only place the carried files' expectations are met: the carried files themselves are never edited.
//!
//! `grid` and `logging` are unconditional; the typed-menu adapters serve the engines that run on the Switch (the model/runtime
//! split: `cfg(any(test, target_os = "switch"))`, the host build then has no dead code).

#[cfg(any(test, target_os = "switch"))]
pub mod choice;
pub mod grid;
pub mod logging;
#[cfg(any(test, target_os = "switch"))]
pub mod fixed;
#[cfg(any(test, target_os = "switch"))]
pub mod menu;
#[cfg(any(test, target_os = "switch"))]
pub mod menu_json;
#[cfg(any(test, target_os = "switch"))]
pub mod panel_view;
