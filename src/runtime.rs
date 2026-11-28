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
mod cut_file;
pub(crate) mod diag_log;
#[cfg(feature = "diag")]
mod diag_reload;
mod deluxe_overlay;
mod deluxe_row;
pub(crate) mod export;
pub(crate) mod bridge_state;
mod features;
pub use features::hitbox::{fighter_screen_anchors, FighterScreenAnchor};
mod game;
mod hub;
mod import;
mod install;
mod log_file;
mod lobby;
mod macros;
mod mii;
mod p1;
mod published;
mod roster;
mod ui;
pub(crate) use ui::worker::wake as wake_recording_menu;

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
