//! Numbered-picker checks that call Character Cycles and the picker together.
//! They live on the lib, not in `model.rs`: the menu integration tests compile
//! `model.rs` by path and have no `engine`.

use crate::engine::swap::roster_grid::{deselect_kind, select_kind};
use crate::model::numbered_picker::{
    apply_key, chain_recordings, give_next_number, take_number_off, CatalogTile, NumberKey,
    TileKind,
};

#[derive(Clone, Copy)]
struct Step {
    key: NumberKey,
    index: usize,
    /// R on the only remaining number: the picker removes it, Character Cycles refuses.
    picker_removes_last: bool,
}

struct Case {
    name: &'static str,
    start: &'static [u8],
    steps: &'static [Step],
}

fn cycles(values: &mut [u8], index: usize, key: NumberKey) -> bool {
    match key {
        NumberKey::A => select_kind(values, index),
        NumberKey::R => deselect_kind(values, index),
        NumberKey::B => false,
    }
}

fn run(case: &Case) {
    let mut cycles_values = case.start.to_vec();
    let mut picker = case.start.to_vec();
    for (n, step) in case.steps.iter().enumerate() {
        if step.picker_removes_last {
            assert_eq!(
                step.key,
                NumberKey::R,
                "{} step {n} is the R difference",
                case.name
            );
            let before = cycles_values.clone();
            assert!(
                !cycles(&mut cycles_values, step.index, step.key),
                "{} step {n}: Character Cycles refuses",
                case.name
            );
            assert!(
                apply_key(&mut picker, step.index, step.key),
                "{} step {n}: the picker removes",
                case.name
            );
            assert_eq!(cycles_values, before, "{} step {n}", case.name);
            assert_eq!(picker[step.index + 1], 0, "{} step {n}", case.name);
            assert!(
                picker.iter().skip(1).all(|rank| *rank == 0),
                "{} step {n}: the list is empty ({picker:?})",
                case.name
            );
            assert_ne!(picker, cycles_values, "{} step {n}", case.name);
        } else {
            let cycles_changed = cycles(&mut cycles_values, step.index, step.key);
            let picker_changed = apply_key(&mut picker, step.index, step.key);
            assert_eq!(
                (picker_changed, picker.as_slice()),
                (cycles_changed, cycles_values.as_slice()),
                "{} step {n}",
                case.name
            );
        }
    }
}

fn numbered_picker_matches_character_cycles() {
    let table = [
        Case {
            name: "A gives the next number",
            start: &[0, 0, 0, 0],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: 2,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: 1,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "A on a numbered tile does nothing",
            start: &[0, 1, 2, 3],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: 2,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: 1,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "R closes the later numbers",
            start: &[0, 1, 2, 3, 4],
            steps: &[
                Step {
                    key: NumberKey::R,
                    index: 1,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 0,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "R on an unnumbered tile does nothing",
            start: &[0, 1, 0, 2],
            steps: &[Step {
                key: NumberKey::R,
                index: 1,
                picker_removes_last: false,
            }],
        },
        Case {
            name: "B is done",
            start: &[0, 1, 2, 3],
            steps: &[
                Step {
                    key: NumberKey::B,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::B,
                    index: 99,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: 1,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "an index past the catalog does nothing",
            start: &[0, 1, 2],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 5,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 5,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::A,
                    index: usize::MAX,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: usize::MAX,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "an empty buffer does nothing",
            start: &[],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::B,
                    index: 0,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "a buffer with only the All flag does nothing",
            start: &[0],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 0,
                    picker_removes_last: false,
                },
            ],
        },
        Case {
            name: "R on the only remaining number",
            start: &[0, 1, 0, 0],
            steps: &[Step {
                key: NumberKey::R,
                index: 0,
                picker_removes_last: true,
            }],
        },
        Case {
            name: "A then R clears the number just given",
            start: &[0, 0, 0],
            steps: &[
                Step {
                    key: NumberKey::A,
                    index: 1,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 1,
                    picker_removes_last: true,
                },
            ],
        },
        Case {
            name: "R down to one number, then R clears it",
            start: &[0, 1, 2, 3],
            steps: &[
                Step {
                    key: NumberKey::R,
                    index: 2,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 0,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::B,
                    index: 1,
                    picker_removes_last: false,
                },
                Step {
                    key: NumberKey::R,
                    index: 1,
                    picker_removes_last: true,
                },
            ],
        },
    ];
    for case in &table {
        run(case);
    }
}

fn r_removes_the_last_number() {
    let mut values = [0, 2, 1, 3];
    assert!(take_number_off(&mut values, 0));
    assert_eq!(values, [0, 0, 1, 2]);
    assert!(take_number_off(&mut values, 2));
    assert_eq!(values, [0, 0, 1, 0]);
    assert!(take_number_off(&mut values, 1), "the last number comes off");
    assert_eq!(values, [0, 0, 0, 0]);
    assert!(!take_number_off(&mut values, 1));
    assert!(!apply_key(&mut values, 1, NumberKey::R));
    assert!(give_next_number(&mut values, 2));
    assert_eq!(values, [0, 0, 0, 1]);
    assert!(!apply_key(&mut values, 2, NumberKey::A));
    assert!(apply_key(&mut values, 2, NumberKey::R));
    assert_eq!(values, [0, 0, 0, 0]);
    let mut kept = values;
    assert!(!apply_key(&mut kept, 0, NumberKey::B));
    assert_eq!(kept, values);
}

fn the_chain_picker_lists_recordings_only() {
    let tiles = [
        CatalogTile {
            id: "macro-a",
            kind: TileKind::Recording,
        },
        CatalogTile {
            id: "chain-1",
            kind: TileKind::Chain,
        },
        CatalogTile {
            id: "macro-b",
            kind: TileKind::Recording,
        },
        CatalogTile {
            id: "chain-2",
            kind: TileKind::Chain,
        },
    ];
    let listed: Vec<_> = chain_recordings(&tiles)
        .map(|tile| (tile.id, tile.kind))
        .collect();
    assert_eq!(
        listed,
        [
            ("macro-a", TileKind::Recording),
            ("macro-b", TileKind::Recording)
        ]
    );
    let only_chains = [
        CatalogTile {
            id: "chain-1",
            kind: TileKind::Chain,
        },
        CatalogTile {
            id: "chain-2",
            kind: TileKind::Chain,
        },
    ];
    assert_eq!(chain_recordings(&only_chains).count(), 0);
    let only_recordings = [CatalogTile {
        id: "macro-a",
        kind: TileKind::Recording,
    }];
    assert_eq!(chain_recordings(&only_recordings).count(), 1);
    assert_eq!(chain_recordings::<&str>(&[]).count(), 0);

    let source = include_str!("model/numbered_picker.rs");
    let filter = source
        .lines()
        .find(|line| line.contains("tile.kind == TileKind::Recording"))
        .expect("filter line");
    assert!(
        filter.contains("// restriction: chain-recordings-only"),
        "{filter}"
    );
    let restrictions = include_str!("../docs/RESTRICTIONS.md");
    assert!(
        restrictions.lines().any(|line| {
            let cells: Vec<_> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            cells.len() >= 4
                && cells[0].trim_matches('`') == "chain-recordings-only"
                && cells[1] == "(none)"
                && cells[2] == "a chain inside a chain would need a loop check"
                && cells[3] == "operator 2026-11-06, revision 8 Q13"
        }),
        "RESTRICTIONS.md row"
    );
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("numbered_picker_matches_character_cycles", numbered_picker_matches_character_cycles as fn()),
        ("r_removes_the_last_number", r_removes_the_last_number as fn()),
        ("the_chain_picker_lists_recordings_only", the_chain_picker_lists_recordings_only as fn()),
    ]);
}
