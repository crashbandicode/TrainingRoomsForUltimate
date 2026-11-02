//! Training Rooms for Ultimate: a Skyline plugin for Super Smash Bros. Ultimate 13.0.5.
//!
//! Layout (playbook): `model` is pure and host-tested; `runtime` holds everything that touches the
//! game and exists only for the Switch. The crate is a `cdylib` only, so tests include `model` by
//! path.

pub mod model;

// The carried fork engines (package 6): each area is test-only until its consumer lands, except those engine/mod.rs leaves open.
pub mod engine;

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
}
