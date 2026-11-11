//! The stage catalog as package 5's stage grid reads it: a pure, public facade over the carried lobby-stage model (written for this
//! repository, not carried). `lobby_stages::STAGES` is the one authoritative list (117 selectable stages of the owned 13.0.5
//! `ui_stage_db.prc`, Waiting Room first, native display order; Training is appended by the lobby cycle, not listed); the grid adds
//! an All tile (MENU-DRAFT, decision Q14: 4 x 4 tiles per page). The carried paging and help-line helpers are crate-private in the
//! fork's files, so the grid reaches them here.

use super::lobby_stages;
pub use super::stage_forms::{Form, HelpControls};
use super::stage_menu;

/// Selectable stages, in grid order: `(stage id, label)`.
pub const STAGES: &[(u32, &str)] = lobby_stages::STAGES;
/// Number of selectable stages (117 at stage m5o).
pub const STAGE_COUNT: usize = STAGES.len();
/// Tiles of the grid: the stages plus the All tile.
pub const TILE_COUNT: usize = STAGE_COUNT + 1;
/// Tiles per page (4 rows x 4 columns).
pub const PAGE_SIZE: usize = stage_menu::PAGE_SIZE;
/// Pages of the full grid.
pub const PAGE_COUNT: usize = TILE_COUNT.div_ceil(PAGE_SIZE);

/// The native alternate forms of a stage: `[Normal, Omega, Battlefield]`, `None` where the stage has none.
pub fn native_forms(stage: u32) -> [Option<u32>; 3] {
    lobby_stages::native_forms(stage)
}

/// The next or previous page's cell for a cursor, wrapping at the ends (`None` for an empty grid).
pub fn page_cursor(selected: usize, count: usize, next: bool) -> Option<usize> {
    stage_menu::page_cursor(selected, count, next)
}

/// The help pane's line for the highlighted form: the form label, the Y hint when another form exists, and the button legend.
pub fn form_help(form: Form, native_forms: [Option<u32>; 3], controls: HelpControls) -> &'static str {
    form.help_line(native_forms, controls)
}

/// The button legend alone (a tile without a form row).
pub fn controls_help(controls: HelpControls) -> &'static str {
    controls.bare()
}

