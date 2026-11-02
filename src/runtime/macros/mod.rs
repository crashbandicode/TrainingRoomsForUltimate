//! Package 4, the named-macro / replay layer (S4-4), game side. The files are the fork's `src/training/named_macros.rs`, `src/replay_*.rs`,
//! `src/live_replay.rs` and `src/capture_plugin.rs`, carried by `tools/carry_pkg4.py` (stage m5q; recorded edits in
//! `docs/provenance/pkg4-carry-ledger.md`). They name the fork's mixed modules; `runtime::compat` and the lobby `training` facade meet
//! those names (package 6's shims, extended in place), and `port` holds what depends on a package that is not merged yet.
//!
//! This module is also the carried files' `super::`: the fork's `training` module names they reach through their parent
//! (`practice`, `lobby`, `input_record`, `save_states`, `frame_counter`) are re-exported here.

pub(crate) mod port;
pub(crate) mod replay_move_start;
pub(crate) mod replay_store;
