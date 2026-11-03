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
