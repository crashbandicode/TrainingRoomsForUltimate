//! The automation helper's development-only game-facing bridge. The obsolete
//! results observer was retired for the RC; the pure automation model remains.

#[cfg(feature = "automation-results-native")]
pub(crate) use crate::engine::lobby::automation::{adapter, control, flow, popup, pulse};
