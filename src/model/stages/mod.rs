//! Package 6 practice stages, lobby stages and Training stage rotation, pure half.
//!
//! Carried from the fork (stage m5o) by tools/carry_pkg6.py (docs/provenance/pkg6-carry-manifest.json); never edited by hand.
//! Unconditional since gate 1 (the lead's ruling): package 5's stage grid sizes itself from `lobby_stages::STAGES`.
//! The policies the lobby's game-facing files call are on the Switch as well (`waiting_hazards_policy` needs the Mii crate, which is).

pub mod catalog;
#[cfg(any(test, target_os = "switch"))]
pub mod lobby_stage_cycle;
pub mod lobby_stages;
#[cfg(any(test, target_os = "switch"))]
pub mod practice_stage_request_policy;
pub mod stage_forms;
pub mod stage_menu;
#[cfg(any(test, target_os = "switch"))]
pub mod training_stage_rotation_policy;
#[cfg(any(test, target_os = "switch"))]
pub mod waiting_hazards_policy;