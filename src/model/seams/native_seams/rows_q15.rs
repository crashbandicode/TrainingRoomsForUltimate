//! Q15 diag-only observation guards, generated from owned 13.0.5 ELF0789f0a4.
use super::*;

#[cfg(not(any(test, feature = "diag")))]
pub const ROWS: &[Seam] = &[];
// The host census checks every hook attribute, including diagnostic-only hooks.
#[cfg(any(test, feature = "diag"))]
pub const ROWS: &[Seam] = &include!(concat!(env!("OUT_DIR"), "/tables/rows_q15.ROWS.rs"));
