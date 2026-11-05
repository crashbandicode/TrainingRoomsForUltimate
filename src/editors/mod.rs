//! The editors of the menu that need the carried engines (package 6's roster grid and Mii lists, package 4's clip editor): pure
//! models that turn engine state into `model::frame::Frame`s, host-tested. `model` cannot see `engine` (the by-path test crates
//! include `model` alone), so these live beside it; the menu worker (`runtime::ui::worker`) is their only caller.

pub mod clip;
#[cfg(any(test, target_os = "switch"))]
pub mod combo_fighter;
#[cfg(any(test, target_os = "switch"))]
pub mod mii;
#[cfg(any(test, target_os = "switch"))]
pub mod roster;
#[cfg(any(test, target_os = "switch"))]
pub mod stages;

#[cfg(any(test, target_os = "switch"))]
pub mod setups;
