//! The automation helper's pure half (the fork's `automation_helper`: the native results adapter's state machine, its controls, popup
//! snapshot, input pulses and the results observer's recording). Carried from the fork (stage m5o) by tools/carry_pkg6.py; never edited
//! by hand. The game-facing files (`native`, `observer`, `plugin`) are Switch-only under `runtime::lobby::automation_helper`.
//!
//! Development-only: the product is built with neither results feature (`automation-results-observer` is a diag feature,
//! `automation-results-native` needs the Deluxe SDK, decision L1), so the carried tests are what runs these files by default.

pub mod adapter;
pub mod control;
pub mod flow;
pub mod popup;
pub mod pulse;
pub mod recording;
