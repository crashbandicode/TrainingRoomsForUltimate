//! Everything that touches the game. Compiled only for the Switch (`cfg(target_os = "switch")`), so
//! `cargo test` on the host never builds it; the Switch build is `cargo skyline build --release`.
//!
//! NOT YET SWITCH-COMPILED: written against Skyline 0.6.0's API from its sources. The first
//! `cargo skyline build` may need small fixes.

mod alloc;
mod boot;
mod clock;
mod diag_log;
mod features;
mod game;
mod hub;
mod install;
mod published;
mod state_boot;
mod state_natives;

#[skyline::main(name = "training_rooms")]
pub fn main() {
    boot::run();
}
