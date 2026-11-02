//! The CPU-as-P1 session arbitration (menu and stage), carried; loaded by runtime::p1::cpu_pilot as crate::engine::p1_session::*.
//! Their pub(super) items are widened to pub(crate) by the tool (manifest flag widen): in the fork they were the #[path] children of cpu_pilot.

pub mod menu_session;
pub mod stage_session;
