//! Original F6 regression: the owned native reader branches around the body
//! load when count <= active index (13.0.5 main+0x642c94..0x642ca8).
#[path = "../src/model/entry_cells.rs"]
mod entry_cells;

#[test]
fn unpublished_cells_are_not_body_pointers() {
    for poison in [0xffff7e80ffff7e80, 0xffff807dffff807d, 0x50494c466b] {
        for (index, count) in [(0, 0), (1, 1), (2, 2), (255, 3)] {
            let mut body_reads = 0;
            let body = entry_cells::read_active_body(0x10000, true, |address| {
                if address == 0x14150 { count } else {
                    body_reads += 1;
                    poison
                }
            }, |address| {
                assert_eq!(address, 0x15918);
                index
            });
            assert_eq!((body, body_reads), (0, 0), "index={index} count={count}");
        }
    }
}

#[test]
fn fresh_publication_switch_and_teardown_follow_the_current_window() {
    let bodies = [0x20000, 0x30000, 0x40000];
    // Empty allocation, ordinary fighter, bank/Trainer switches, teardown.
    for (count, index, expected) in [(0, 0, 0), (1, 0, bodies[0]),
        (3, 0, bodies[0]), (3, 1, bodies[1]), (3, 2, bodies[2]), (0, 2, 0)] {
        let body = entry_cells::read_active_body(0x10000, true, |address| {
            match address {
                0x14150 => count,
                0x14160 => bodies[0],
                0x14168 => bodies[1],
                0x14170 => bodies[2],
                _ => panic!("unexpected native word {address:x}"),
            }
        }, |_| index);
        assert_eq!(body as u64, expected);
    }
}

// Same strict resolver and identity policy as the game-thread adapter. Synthetic
// addresses are never dereferenced: the spies make every forbidden read fail.
fn cleanup_spy(entry_id: i32, guarded: bool, manager: bool, entry: usize,
    window: (u64, u8, usize), identity: (usize, u32)) -> usize {
    let (count, index, body) = window;
    let (accessor, object_id) = identity;
    let resolved = entry_cells::read_cleanup_body(entry_id, guarded,
        |id| if (0..8).contains(&id) && manager { entry } else { 0 },
        |address| match address - entry {
            0x4150 => count,
            offset if offset == 0x4160 + 8 * usize::from(index) => body as u64,
            _ => panic!("unpublished cell read"),
        },
        |address| { assert_eq!(address, entry + 0x5918); index });
    let mut calls = 0;
    if resolved != 0 {
        assert_eq!(resolved, body);
        if entry_cells::cleanup_identity_matches(0x50000, 0x123, accessor, object_id) {
            calls += 1;
        }
    }
    calls
}

#[test]
fn cleanup_resolves_native_entries_and_each_published_cell_once() {
    // Generic non-zero-cell qualification, not an unsupported Kirby layout.
    // Kirby remains pinned to cell0 even in a non-first roster position.
    for entry in [0, 1, 4] {
        for index in 0..3 {
            assert_eq!(cleanup_spy(entry, true, true, 0x10000,
                (3, index, 0x20000 + 0x1000 * usize::from(index)), (0x50000, 0x123)), 1);
        }
    }
}

#[test]
fn rejected_layout_never_resolves_or_reads_poisoned_cells() {
    for poison in [1u64, 0xffff7e80ffff7e80, 0x50494c466b] {
        assert_eq!(entry_cells::read_cleanup_body(0, false,
            |_| panic!("entry lookup under rejected guard"),
            |_| panic!("poisoned word {poison:x} was read"),
            |_| panic!("unguarded byte read")), 0);
    }
}

#[test]
fn invalid_entry_manager_entry_body_count_or_index_never_cleans_up() {
    for id in [-1, 8, i32::MAX] {
        assert_eq!(cleanup_spy(id, true, true, 0x10000, (3, 0, 1), (0x50000, 0x123)), 0);
    }
    assert_eq!(cleanup_spy(0, true, false, 0x10000, (3, 0, 1), (0x50000, 0x123)), 0);
    assert_eq!(cleanup_spy(0, true, true, 0, (3, 0, 1), (0x50000, 0x123)), 0);
    assert_eq!(cleanup_spy(0, true, true, 0x10000, (3, 0, 0), (0x50000, 0x123)), 0);
    for (count, index) in [(0, 0), (1, 1), (2, 2), (4, 3), (3, 255)] {
        assert_eq!(cleanup_spy(0, true, true, 0x10000, (count, index, 1), (0x50000, 0x123)), 0);
    }
}

#[test]
fn either_identity_mismatch_refuses_cleanup() {
    for (accessor, id) in [(0, 0x123), (0x50001, 0x123), (0x50000, 0x124)] {
        assert_eq!(cleanup_spy(4, true, true, 0x10000, (3, 2, 0x20000), (accessor, id)), 0);
    }
}
