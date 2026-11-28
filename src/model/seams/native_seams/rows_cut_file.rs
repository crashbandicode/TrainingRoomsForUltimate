use super::*;

/// The guarded picture-copy observer.
pub const ROWS: &[Seam] = &include!(concat!(env!("OUT_DIR"), "/tables/rows_cut_file.ROWS.rs"));
