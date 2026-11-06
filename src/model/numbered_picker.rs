//! Numbered picker for a chain or a sequence.
//!
//! The buffer is Character Cycles' rank buffer: index 0 is the All flag and the
//! rest are 1-based ranks (`0` is unnumbered). A gives the next number and does
//! nothing on a tile that already has one. R takes a number off and the later
//! numbers close up. B is done and leaves the ranks. R on the only remaining
//! number removes it, so the list may be empty. Inside this picker R is remove.
//!
//! A follows `roster_grid::select_kind`. R follows `roster_grid::deselect_kind`
//! except for that last number. This file does not change `roster_grid.rs`.

/// A, R and B while the numbered picker is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKey {
    /// Give this tile the next number. A numbered tile stays as it is.
    A,
    /// Take this tile's number off. Later numbers close up.
    R,
    /// Leave the picker.
    B,
}

/// Apply one picker key. `index` is the catalog index into the rank slice.
pub fn apply_key(values: &mut [u8], index: usize, key: NumberKey) -> bool {
    match key {
        NumberKey::A => give_next_number(values, index),
        NumberKey::R => take_number_off(values, index),
        NumberKey::B => false,
    }
}

/// Append a catalog entry. A selected entry keeps its rank.
pub fn give_next_number(values: &mut [u8], index: usize) -> bool {
    let Some((all, ranks)) = values.split_first_mut() else {
        return false;
    };
    let Some(&current) = ranks.get(index) else {
        return false;
    };
    let all_mode = *all != 0;
    if !all_mode && current != 0 {
        return false;
    }
    let count = if all_mode {
        0
    } else {
        ranks.iter().filter(|rank| **rank != 0).count()
    };
    let Ok(next) = u8::try_from(count + 1) else {
        return false;
    };
    if all_mode {
        *all = 0;
        ranks.fill(0);
    }
    ranks[index] = next;
    true
}

/// Remove a catalog entry and compact later ranks. The final number comes off too.
pub fn take_number_off(values: &mut [u8], index: usize) -> bool {
    let Some((all, ranks)) = values.split_first_mut() else {
        return false;
    };
    let Some(&removed) = ranks.get(index) else {
        return false;
    };
    if *all != 0 || removed == 0 {
        return false;
    }
    ranks[index] = 0;
    for rank in ranks.iter_mut().filter(|rank| **rank > removed) {
        *rank -= 1;
    }
    true
}

/// What a library tile is. The chain picker lists recordings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    Recording,
    Chain,
}

/// One tile a picker catalog can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogTile<T> {
    pub id: T,
    pub kind: TileKind,
}

/// Recordings a chain picker lists. A chain tile is left out.
pub fn chain_recordings<T>(tiles: &[CatalogTile<T>]) -> impl Iterator<Item = &CatalogTile<T>> {
    tiles.iter().filter(|tile| {
        tile.kind == TileKind::Recording // restriction: chain-recordings-only
    })
}
