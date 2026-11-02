//! Adapters for the names the carried fork code expects from the fork's mixed files (package 6, docs/PACKAGE-6-PLAN.md section 2).
//! Our code, small, and the only place the carried files' expectations are met: the carried files themselves are never edited.
//!
//! `grid` is unconditional (the carried stage grid names it); the typed-menu adapters stay test-only with the preload engine they
//! serve until its runtime consumer lands.

#[cfg(test)]
pub mod choice;
pub mod grid;
pub mod logging;
#[cfg(test)]
pub mod menu;
#[cfg(test)]
pub mod menu_json;