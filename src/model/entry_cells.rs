//! F11 (fix ledger; lead ruling 2026-11-02): which window cell of a fighter entry is the running fighter. The entry holds up to three
//! bodies in its window (`+0x4160/68/70`, count `+0x4150`); the game's active-cell index is the byte `+0x5918` (the game's own
//! reader, main+0x642c88: `ldrb w8,[entry+0x5918]; ldr x9,[entry+0x4150]; cmp; b.ls; ldr x0,[entry+0x4160+8*index]`). A body-bank
//! group swap and a native Pokemon Trainer switch both change the index, so reading cell 0 (`+0x4160`) misses the running body.
//! Pure and host-tested; `runtime::game::natives::entry_active_body` reads the two fields and calls it.

/// The window's capacity (three cells).
pub const CELLS: usize = 3;

/// The cell (0..3) the game runs: the index when it is a filled cell (below the count and the capacity), else cell 0 (the
/// first body, what every reader used before).
pub fn active_cell(index: u8, count: u64) -> usize {
    let index = usize::from(index);
    if (index as u64) < count && index < CELLS { index } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_cell_is_the_games_index_when_it_is_filled() {
        // One body: cell 0. A swapped three-body group with the second body running: cell 1. PT's Charizard: cell 2.
        assert_eq!(active_cell(0, 1), 0);
        assert_eq!(active_cell(1, 3), 1);
        assert_eq!(active_cell(2, 3), 2);
    }

    #[test]
    fn an_index_past_the_filled_cells_falls_back_to_the_first_body() {
        assert_eq!(active_cell(1, 1), 0, "index past the count");
        assert_eq!(active_cell(3, 4), 0, "index past the capacity");
        assert_eq!(active_cell(0, 0), 0, "an empty window");
    }
}