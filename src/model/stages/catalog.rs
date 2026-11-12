//! The stage catalog as package 5's stage grid reads it: a pure, public facade over the carried lobby-stage model (written for this
//! repository, not carried). `lobby_stages::STAGES` is the one authoritative list (117 selectable stages of the owned 13.0.5
//! `ui_stage_db.prc`, Waiting Room first, native display order; Training is appended by the lobby cycle, not listed); the grid adds
//! an All tile (MENU-DRAFT, decision Q14: 4 x 4 tiles per page). The carried paging and help-line helpers are crate-private in the
//! fork's files, so the grid reaches them here.

use super::lobby_stages;
pub use super::stage_forms::{Form, HelpControls};
#[cfg(test)]
use super::stage_menu;

/// Selectable stages, in grid order: `(stage id, label)`.
pub const STAGES: &[(u32, &str)] = lobby_stages::STAGES;
/// Number of selectable stages (117 at stage m5o).
#[cfg(test)]
pub const STAGE_COUNT: usize = STAGES.len();
/// Tiles of the grid: the stages plus the All tile.
#[cfg(test)]
pub const TILE_COUNT: usize = STAGE_COUNT + 1;

/// The native alternate forms of a stage: `[Normal, Omega, Battlefield]`, `None` where the stage has none.
pub fn native_forms(stage: u32) -> [Option<u32>; 3] {
    lobby_stages::native_forms(stage)
}

/// The next or previous page's cell for a cursor, wrapping at the ends (`None` for an empty grid).
#[cfg(test)]
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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn paging_wraps_and_clamps_on_the_last_page() {
        assert_eq!(page_cursor(0, TILE_COUNT, true), Some(16));
        assert_eq!(page_cursor(0, TILE_COUNT, false), Some(TILE_COUNT - 1).map(|last| (7 * 16).min(last)));
        assert_eq!(page_cursor(117, TILE_COUNT, true), Some(5));
        assert_eq!(page_cursor(0, 0, true), None);
    }

    fn the_help_lines_name_the_form_and_the_legend() {
        let forms = native_forms(0);
        let line = form_help(Form::Normal, forms, HelpControls::TrainingApply);
        assert!(line.starts_with("Form: Normal"));
        assert!(line.ends_with(controls_help(HelpControls::TrainingApply)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("paging_wraps_and_clamps_on_the_last_page", paging_wraps_and_clamps_on_the_last_page as fn()),
            ("the_help_lines_name_the_form_and_the_legend", the_help_lines_name_the_form_and_the_legend as fn()),
        ]);
    }
    }
}