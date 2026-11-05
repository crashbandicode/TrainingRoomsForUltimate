//! Everything that touches the game. Compiled only for the Switch (`cfg(target_os = "switch")`), so
//! `cargo test` on the host never builds it; the Switch build is `cargo skyline build --release`.
//!
//! NOT YET SWITCH-COMPILED: written against Skyline 0.6.0's API from its sources. The first
//! `cargo skyline build` may need small fixes.

// The fork's recording workflows this release ships (`engine::macro_ui::workflows`): the replay window and exact-Mii
// named macros (operator 2026-11-05); the Setup library, Edit placement and ordinary-fighter switching stay closed.
// Their shared capture/save-state consumers and cleanup paths stay resident.
pub(crate) const WORKFLOWS: crate::engine::macro_ui::workflows::Workflows = crate::engine::macro_ui::workflows::RELEASE;

mod alloc;
mod boot;
mod clock;
mod developer;
mod developer_reload;
mod compat;
pub(crate) mod diag_log;
#[cfg(feature = "diag")]
mod diag_reload;
mod deluxe_row;
mod features;
mod game;
mod hub;
mod import;
mod install;
mod lobby;
mod macros;
mod mii;
mod p1;
mod published;
mod roster;
mod ui;

mod scene;
mod seams;
mod state_boot;
mod state_natives;
#[cfg(feature = "diag")]
mod sync_probe;
pub(crate) mod threads;

#[skyline::main(name = "training_rooms")]
pub fn main() {
    boot::run();
}


mod native_tls;
pub(crate) mod hook_tls;
