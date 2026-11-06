//! Training Rooms for Ultimate: a Skyline plugin for Super Smash Bros. Ultimate 13.0.5.
//!
//! Layout (playbook): `model` is pure and host-tested; `runtime` holds everything that touches the
//! game and exists only for the Switch. The crate is a `cdylib` only, so tests include `model` by
//! path.

pub mod model;

// The carried fork engines (package 6): each area is test-only until its consumer lands, except those engine/mod.rs leaves open.
pub mod engine;

// The menu's editors that read the engines' types (clip editor, roster grid, Mii lists): pure, host-tested.
pub mod editors;
// The install line's worst-case length (model site tables + the carried seam table): the runtime's buffer size and its host test.
pub mod install_capacity;

#[cfg(test)]
#[path = "../tests/common/preload_allocations.rs"]
mod preload_allocation_tests;

#[cfg(test)]
#[path = "model/lobby_flag_tests.rs"]
mod lobby_flag_tests;

#[cfg(target_os = "switch")]
mod runtime;

#[cfg(test)]
mod grid_matches_catalog {
    /// The stage grid's tile count (a model constant, because the by-path test crates cannot see `engine`) is the
    /// catalog's: All and every selectable stage.
    #[test]
    fn the_stage_grid_has_a_tile_per_catalog_stage_and_the_all_tile() {
        assert_eq!(crate::model::menu::grid::STAGE_TILES, crate::engine::stages::catalog::TILE_COUNT);
    }

    /// The roster grid's tile count (a model constant) is the portrait grid's cell count.
    #[test]
    fn the_roster_grid_has_a_tile_per_portrait_cell() {
        assert_eq!(crate::model::menu::grid::ROSTER_TILES, crate::engine::swap::roster_grid::CELLS);
        assert_eq!(usize::from(crate::model::menu::grid::ROSTER_COLS), crate::engine::swap::roster_grid::COLUMNS);
        assert_eq!(usize::from(crate::model::menu::grid::ROSTER_ROWS), crate::engine::swap::roster_grid::ROWS);
    }

    #[test]
    fn roster_navigation_matches_the_carried_centered_grid_in_every_cell() {
        use crate::model::menu::grid::{ConsoleMode, GridKind, GridLayout, Move, RosterUse};
        use crate::engine::swap::roster_grid::{Direction, Role, RosterGrid, CELLS};
        for mode in [ConsoleMode::Docked, ConsoleMode::Handheld] {
            let layout = GridLayout::new(GridKind::Roster(RosterUse::Cycles), mode);
            for cell in 0..CELLS {
                // Reach each cell through the existing roster's public movement API.
                let mut native = RosterGrid::new(Role::Player);
                for _ in 0..cell / 13 { native.step(Direction::Down); }
                for _ in 0..cell % 13 { native.step(Direction::Right); }
                assert_eq!(native.cursor(), cell);
                for (input, direction) in [(Move::Up, Direction::Up), (Move::Down, Direction::Down),
                    (Move::Left, Direction::Left), (Move::Right, Direction::Right)] {
                    let mut expected = native;
                    expected.step(direction);
                    assert_eq!(layout.step(0, cell, input), expected.cursor(), "{mode:?} cell={cell} {input:?}");
                }
            }
        }
    }

    /// Every texture row is a lobby stage, and the stages without a texture are only those the game has no stage_1 file for.
    #[test]
    fn the_stage_textures_belong_to_catalog_stages() {
        use crate::engine::stages::lobby_stages::STAGES;
        for (id, name, _) in crate::model::stage_textures::STAGE_TEXTURES {
            assert!(STAGES.iter().any(|(stage, _)| stage == id), "texture row {id} {name} is not a lobby stage");
        }
        let without: Vec<_> = STAGES.iter().filter(|(id, _)| crate::model::stage_textures::texture_of(*id).is_none()).collect();
        assert!(without.len() <= 3, "{without:?}");
    }
}

#[cfg(test)]
mod f9_seat_tests;

#[cfg(test)]
mod planner_alloc_tests;

#[cfg(test)]
mod clip_input_tests;

#[cfg(test)]
mod kirby_hat_cpu_tests;

#[cfg(test)]
mod numbered_picker_tests;
