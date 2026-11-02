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
