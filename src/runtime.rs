//! Everything that touches the game. Compiled only for the Switch (`cfg(target_os = "switch")`), so
//! `cargo test` on the host never builds it; the Switch build is `cargo skyline build --release`.
//!
//! Written against Skyline 0.6.0's API from its sources (`Cargo.toml` pins `skyline = "=0.6.0"`);
//! `cargo skyline build --release` builds it.

mod alloc;
mod boot;
mod clock;
mod developer;
mod developer_reload;
mod compat;
pub(crate) mod diag_log;
#[cfg(feature = "diag")]
mod diag_reload;
mod deluxe_overlay;
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
mod stall_watch;
#[cfg(feature = "diag")]
mod sync_probe;
pub(crate) mod threads;

#[skyline::main(name = "training_rooms")]
pub fn main() {
    boot::run();
}


mod native_tls;
pub(crate) mod hook_tls;
