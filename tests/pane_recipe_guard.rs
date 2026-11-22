//! The committed layout recipes (`assets/info_training.tra1`, `assets/info_waiting.tra1`) must be built from the pane
//! table the runtime resolves (`src/model/ui/panes.rs`). 2026-11-04: the badge port added 354 panes, the recipes were
//! not rebuilt, and on the candidate the menu never opened (`menu_panes=665 missing=354`). `tools/layout-builder` writes
//! the digest of the pane table it built from (`tools/layout-builder/src/pane_table.rs`) to `panes-recipe.sha256`,
//! committed beside the recipes; this test recomputes it.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
use crate::model::panes;
#[path = "../tools/layout-builder/src/pane_table.rs"]
mod pane_table;

const REBUILD: &str = "the pane table (src/model/ui/panes.rs) differs from the one assets/*.tra1 were built from: rebuild \
both recipes as docs/BUILD.md \"Layout archive recipe\" says (cargo build --release --manifest-path \
tools/layout-builder/Cargo.toml; layout-builder build <stock info_training.arc> <out>; layout-builder build-waiting \
<stock info_waiting.arc> <out>) and copy info_training.tra1, info_waiting.tra1 and panes-recipe.sha256 into assets/";

fn committed() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/panes-recipe.sha256");
    std::fs::read_to_string(path).expect("assets/panes-recipe.sha256").trim().to_owned()
}

#[test]
fn the_committed_recipes_were_built_from_this_pane_table() {
    assert_eq!(pane_table::digest(&panes::specs()), committed(), "{REBUILD}");
}

/// The crop window adds no panes. A count change needs a boot that reaches Training.
#[test]
fn the_pane_table_keeps_the_count_the_game_loaded() {
    assert_eq!(panes::specs().len(), 1646, "pane count changed: boot the build to Training");
}

#[test]
fn the_orb_pools_keep_the_pill_cap() {
    assert_eq!((panes::ORBS, panes::CAPS, panes::BANDS), (512, 340, 172));
    assert_eq!(panes::CAPS / 2, 170);
}

/// The guard sees a change to any spec field, a renamed pane and an added pane, as the badge port's were.
#[test]
fn a_changed_pane_table_fails_the_guard() {
    let specs = panes::specs();
    let mut moved = specs.clone();
    moved[panes::IDX_TILE_BADGE].w += 1.0;
    let mut renamed = specs.clone();
    renamed[panes::IDX_TILE_SOLO].name = panes::Name::new("tr_rp_99");
    let shorter = &specs[..specs.len() - 1];
    for changed in [pane_table::digest(&moved), pane_table::digest(&renamed), pane_table::digest(shorter)] {
        assert_ne!(changed, committed());
    }
}
