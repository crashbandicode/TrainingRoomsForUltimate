//! Adapters for the names the carried fork code expects from the fork's mixed files (package 6, docs/PACKAGE-6-PLAN.md section 2).
//! Our code, small, and the only place the carried files' expectations are met: the carried files themselves are never edited.
//!
//! `grid` is unconditional (the carried stage grid names it). The typed-menu adapters and the list view are on the Switch as well as in
//! tests: the lobby and Mii game-facing files read the typed menu and fill the list.

#[cfg(any(test, target_os = "switch"))]
pub mod choice;
pub mod grid;
pub mod logging;
#[cfg(any(test, target_os = "switch"))]
pub mod fixed;
#[cfg(any(test, target_os = "switch"))]
pub mod menu;
#[cfg(test)]
pub mod menu_json;
#[cfg(any(test, target_os = "switch"))]
pub mod panel_view;
