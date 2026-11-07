//! Ordered ids shared by chains and sequences.
//!
//! This file is mounted from `model.rs` and does not import `engine`. A by-path test of `model.rs`
//! has no engine module. JSON field names stay on the documents that use these helpers.

/// The id list inside a chain (`recordings`) or a sequence (`combos`).
pub trait IdList {
    fn ids(&self) -> &[String];
    fn ids_mut(&mut self) -> &mut Vec<String>;
}

/// Drop ids whose file is gone. The caller decides when this runs.
pub fn retain_existing(ids: &mut Vec<String>, mut exists: impl FnMut(&str) -> bool) {
    ids.retain(|id| exists(id));
}

/// Picker ranks become ids. Rank 0 is unnumbered and is left out. The leading values cell is the All flag.
pub fn write_back(values: &[u8], ids: &[String]) -> Vec<String> {
    let mut pairs: Vec<(u8, &str)> = values
        .iter()
        .skip(1)
        .zip(ids)
        .filter(|(rank, _)| **rank != 0)
        .map(|(rank, id)| (*rank, id.as_str()))
        .collect();
    pairs.sort_by_key(|(rank, _)| *rank);
    pairs.into_iter().map(|(_, id)| id.to_string()).collect()
}

/// "1 Name". `index` is zero-based.
pub fn step_line(index: usize, name: &str) -> String {
    format!("{} {name}", index + 1)
}

/// The 1-based place of `id` in the list, used in both badge directions.
pub fn order_of(ids: &[String], id: &str) -> Option<u8> {
    ids.iter()
        .position(|step| step == id)
        .map(|index| (index + 1) as u8)
}
