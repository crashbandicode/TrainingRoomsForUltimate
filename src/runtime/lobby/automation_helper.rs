//! The automation helper's game-facing half (the fork's `automation_helper`, development-only: the product enables neither results
//! feature). The pure half is `engine::lobby::automation`; the carried `observer` (diag feature `automation-results-observer`) and the
//! native adapter `native` (feature `automation-results-native`, which needs the Deluxe SDK: decision L1) are declared here, under the
//! names the fork's `mod.rs` gave them, so their `super::` paths resolve.

#[cfg(all(feature = "automation-results-observer", feature = "automation-results-native"))]
compile_error!("results observer and native adapter both hook 0x150e5f0 and 0x150e9d0; enable only one");

#[cfg(feature = "automation-results-native")]
pub(crate) use crate::engine::lobby::automation::{adapter, control, flow, popup, pulse};
#[cfg(feature = "automation-results-observer")]
pub(crate) use crate::engine::lobby::automation::recording;

#[cfg(feature = "automation-results-observer")]
#[path = "automation_helper/observer.rs"]
pub(crate) mod observer;
