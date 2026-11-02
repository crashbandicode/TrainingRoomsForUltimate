//! The features' hooks (the rules are in `model`): package 1's three, and package 2's CPU behaviours.

pub mod chords;
pub mod cpu;
pub mod input_delay;
pub mod input_display;
pub mod lra_reset;
pub mod notices;
pub mod recorder;
pub mod reset_hub;
pub mod stage_hazards;
#[cfg(feature = "diag")]
pub mod writer;
