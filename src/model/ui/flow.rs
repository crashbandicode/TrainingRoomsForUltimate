//! Which screen owns the buttons while a panel of the named-macro / replay layer is involved (the menu's own page, a layer panel
//! opened from a Recording row, or the clip editor that took Player 1 from the menu). Pure, so the transitions are host-tested; the
//! menu worker (`runtime::ui::worker`) asks it after every press and every poll and does what it answers.

/// Who owns the buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// The menu's own page or sub-screen.
    #[default]
    Menu,
    /// A layer panel (Named library, Save slot, Edit placement, the capture browser) under its menu row.
    Panel,
    /// The clip editor: the menu is closed, the editor reads Player 1 and its frame is drawn alone.
    Clip,
}

/// What the layer reports now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// The menu is open (it owns Player 1).
    pub menu_open: bool,
    /// Any panel of the layer is up (the clip editor included).
    pub panel_visible: bool,
    /// The clip editor is up.
    pub clip_visible: bool,
}

/// What the worker does about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Nothing,
    /// Step back out of the layer's panel (the menu can no longer show it).
    CloseLayer,
    /// B on the menu: its row's screen is gone.
    PopRow,
    /// Commit and close the menu; the clip editor draws alone from now on.
    HandOverToClip,
    /// The clip editor closed: stop drawing it.
    HideClip,
}

/// The next phase and the step to take, given the phase and what the layer reports.
pub fn next(phase: Phase, seen: Seen) -> (Phase, Step) {
    match phase {
        Phase::Menu => (Phase::Menu, Step::Nothing),
        Phase::Panel if !seen.menu_open => (Phase::Menu, Step::CloseLayer),
        Phase::Panel if seen.clip_visible => (Phase::Clip, Step::HandOverToClip),
        Phase::Panel if !seen.panel_visible => (Phase::Menu, Step::PopRow),
        Phase::Panel => (Phase::Panel, Step::Nothing),
        Phase::Clip if !seen.clip_visible => (Phase::Menu, Step::HideClip),
        Phase::Clip => (Phase::Clip, Step::Nothing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(menu_open: bool, panel_visible: bool, clip_visible: bool) -> Seen {
        Seen { menu_open, panel_visible, clip_visible }
    }

    #[test]
    fn a_panel_stays_while_the_layer_shows_it() {
        assert_eq!(next(Phase::Panel, seen(true, true, false)), (Phase::Panel, Step::Nothing));
    }

    #[test]
    fn a_panel_that_closed_itself_pops_the_menu_row() {
        assert_eq!(next(Phase::Panel, seen(true, false, false)), (Phase::Menu, Step::PopRow));
    }

    #[test]
    fn a_clip_editor_takes_the_screen_and_the_menu_closes() {
        // The clip editor counts as a visible panel too: the clip wins.
        assert_eq!(next(Phase::Panel, seen(true, true, true)), (Phase::Clip, Step::HandOverToClip));
        assert_eq!(next(Phase::Clip, seen(false, true, true)), (Phase::Clip, Step::Nothing));
        assert_eq!(next(Phase::Clip, seen(false, false, false)), (Phase::Menu, Step::HideClip));
    }

    #[test]
    fn losing_the_menu_closes_the_layers_panel() {
        assert_eq!(next(Phase::Panel, seen(false, true, false)), (Phase::Menu, Step::CloseLayer));
        // A clip editor survives the menu closing: it is its own owner.
        assert_eq!(next(Phase::Panel, seen(false, true, true)), (Phase::Menu, Step::CloseLayer));
    }

    #[test]
    fn the_menu_phase_does_nothing_on_its_own() {
        assert_eq!(next(Phase::Menu, seen(true, true, true)), (Phase::Menu, Step::Nothing));
    }
}