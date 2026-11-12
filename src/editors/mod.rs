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

// P-CYCLE: the grid's numbered list is the CPU cycle's order (engine::swap::cycle_members with the grid's own badges).
#[cfg(test)]
mod cycle_members_tests;
// P1-CYCLE: the Player grid numbers the Player walk, and a missing roster key is one value.
#[cfg(test)]
mod player_cycle_tests;
