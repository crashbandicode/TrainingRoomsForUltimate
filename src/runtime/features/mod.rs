//! The features' hooks (the rules are in `model`): package 1's three, and package 2's CPU behaviours.

pub mod btt;
pub(crate) mod lockon;
pub mod btt_projectiles;
pub mod btt_camera;
pub mod chords;
pub mod combo;
pub mod cpu;
pub mod cpu_common;
pub mod cpu_level;
pub mod game_speed;
pub mod hide_cpu;
pub mod hitbox;
pub mod hud_draw;
pub mod input_delay;
pub mod input_display;
pub mod item_bridge;
pub mod lra_reset;
pub mod mashout;
pub mod notices;
pub mod practice;
pub mod recorder;
pub mod recording_host;
pub mod reset_hub;
pub mod state_effects;
pub mod state_restore;
pub mod states;
pub mod stage_hazards;
#[cfg(feature = "diag")]
pub mod tech_spike;
#[cfg(feature = "diag")]
mod tech_spike_guards;
mod tech_info_seed;
mod tech_native;
mod tech_launch_native;
pub mod tech_adapter;
mod tech_decision;
pub mod writer;

pub mod tech;
pub mod tech_surfaces;

pub mod tech_hitlag;
pub(crate) mod tech_hitlag_read;
