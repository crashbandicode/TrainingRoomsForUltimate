//! F11 (fix ledger; lead ruling 2026-11-02): which window cell of a fighter entry is the running fighter. The entry holds up to three
//! bodies in its window (`+0x4160/68/70`, count `+0x4150`); the game's active-cell index is the byte `+0x5918` (the game's own
//! reader, main+0x642c88: `ldrb w8,[entry+0x5918]; ldr x9,[entry+0x4150]; cmp; b.ls; ldr x0,[entry+0x4160+8*index]`). A body-bank
//! group swap and a native Pokemon Trainer switch both change the index, so reading cell 0 (`+0x4160`) misses the running body.
//! An empty or out-of-count window has no active body: the native `b.ls` skips its load. During initial construction the unused
//! cells can contain heap residue (F6-30 captured count 0 and three non-pointer values); cell 0 is not a fallback for that state.
//! Pure and host-tested; `runtime::game::natives::entry_active_body` reads the two fields and calls it.

/// The window's capacity (three cells).
pub const CELLS: usize = 3;
pub const BODY_FIELD: usize = 0x4160;
const COUNT_FIELD: usize = 0x4150;
const ACTIVE_FIELD: usize = 0x5918;

/// Strict resolver for native copy cleanup. Unlike the legacy accessor reader,
/// this path never falls back to an unguarded cell or retains a game pointer.
/// `resolve_entry` validates the native entry id and manager before reading it.
pub fn read_cleanup_body(
    entry: i32,
    layout_guarded: bool,
    mut resolve_entry: impl FnMut(i32) -> usize,
    word: impl FnMut(usize) -> u64,
    byte: impl FnMut(usize) -> u8,
) -> usize {
    if !layout_guarded {
        return 0;
    }
    read_active_body(resolve_entry(entry), true, word, byte)
}

/// Identity must match both fields before invoking native copy cleanup.
pub fn cleanup_identity_matches(
    callback_accessor: usize,
    callback_id: u32,
    object_accessor: usize,
    object_id: u32,
) -> bool {
    callback_accessor != 0 && callback_accessor == object_accessor && callback_id == object_id
}

/// Read the live window through the caller's native-memory reader. The layout
/// guard admits the count/index fields; without it only the legacy first-body
/// field is read, per F11's recorded fallback contract. No pointer is retained.
pub fn read_active_body(
    entry: usize,
    layout_guarded: bool,
    mut word: impl FnMut(usize) -> u64,
    mut byte: impl FnMut(usize) -> u8,
) -> usize {
    if entry == 0 {
        return 0;
    }
    let cell = if layout_guarded {
        let Some(cell) = active_cell(byte(entry + ACTIVE_FIELD), word(entry + COUNT_FIELD)) else {
            return 0;
        };
        cell
    } else {
        0
    };
    word(entry + BODY_FIELD + 8 * cell) as usize
}

/// The cell (0..3) the game runs, only when the index names an installed body.
pub fn active_cell(index: u8, count: u64) -> Option<usize> {
    let index = usize::from(index);
    if (index as u64) < count && index < CELLS {
        Some(index)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_cell_is_the_games_index_when_it_is_filled() {
        // One body: cell 0. A swapped three-body group with the second body running: cell 1. PT's Charizard: cell 2.
        assert_eq!(active_cell(0, 1), Some(0));
        assert_eq!(active_cell(1, 3), Some(1));
        assert_eq!(active_cell(2, 3), Some(2));
    }

    #[test]
    fn an_index_past_the_filled_cells_has_no_body() {
        assert_eq!(active_cell(1, 1), None, "index past the count");
        assert_eq!(active_cell(3, 4), None, "index past the capacity");
        assert_eq!(active_cell(0, 0), None, "an empty window");
    }

    #[test]
    fn native_reader_tracks_bank_and_trainer_changes_without_caching() {
        // Fake native entry, with sentinel bytes around each native-width field.
        let mut bank = vec![0xa5; 0x5920];
        let count = 3u64;
        bank[0x4150..0x4158].copy_from_slice(&count.to_le_bytes());
        for (i, body) in [0xabc0u64, 0xdef0, 0x1230].into_iter().enumerate() {
            bank[0x4160 + i * 8..0x4168 + i * 8].copy_from_slice(&body.to_le_bytes());
        }
        let entry = 0x10000;
        for (index, expected) in [(0, 0xabc0), (1, 0xdef0), (2, 0x1230), (0, 0xabc0)] {
            bank[0x5918] = index;
            let actual = read_active_body(
                entry,
                true,
                |address| {
                    u64::from_le_bytes(
                        bank[address - entry..address - entry + 8]
                            .try_into()
                            .unwrap(),
                    )
                },
                |address| bank[address - entry],
            );
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn null_entry_does_not_read_memory() {
        for guarded in [false, true] {
            assert_eq!(
                read_active_body(0, guarded, |_| panic!("word read"), |_| panic!("byte read")),
                0
            );
        }
    }

    #[test]
    fn failed_layout_guard_reads_only_the_legacy_first_body() {
        let mut reads = Vec::new();
        let body = read_active_body(
            0x10000,
            false,
            |address| {
                reads.push(address);
                0xabc0
            },
            |_| panic!("unguarded index read"),
        );
        assert_eq!(body, 0xabc0);
        assert_eq!(reads, [0x14160]);
    }

    #[test]
    fn invalid_index_never_reads_outside_the_three_cell_window() {
        for (index, count) in [(1, 1), (2, 2), (3, 4), (255, u64::MAX), (0, 0)] {
            let mut reads = Vec::new();
            let body = read_active_body(
                0x10000,
                true,
                |address| {
                    reads.push(address);
                    match address {
                        0x14150 => count,
                        _ => panic!("unexpected cell read: {address:x}"),
                    }
                },
                |address| {
                    assert_eq!(address, 0x15918);
                    index
                },
            );
            assert_eq!(body, 0);
            assert_eq!(reads, [0x14150]);
        }
    }
}
