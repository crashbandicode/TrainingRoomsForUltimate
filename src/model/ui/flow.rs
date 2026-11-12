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
    /// A fighter question the layer opened with the menu closed (Play on a recording made on another fighter): it is drawn
    /// alone, like the clip editor, and Player 1's presses answer it.
    Prompt,
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
    /// The layer permits physical clip input: no mismatch/name panel or native keyboard owns it.
    pub clip_foreground: bool,
    /// The layer's fighter question is up and a renderer exists to draw it.
    pub prompt_visible: bool,
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
    /// Draw the fighter question alone and give it Player 1's presses.
    ShowPrompt,
    /// The question was answered or retired: stop drawing it and give Player 1 back.
    HidePrompt,
}

/// Whether the menu side still owns Player 1's input (the `Owners::MENU` bit): the open menu, its closing frames, the
/// pop-up, or the closed-menu fighter question. Releasing on any one of them alone handed the question's presses to the
/// match and let the frame strip draw over it (rc.5 XPS proof).
pub fn menu_owns_input(open: bool, popup: bool, closing: bool, prompt: bool) -> bool {
    open || popup || closing || prompt
}

/// Whether the frame of `phase` is built from the menu's model. The clip editor's is not: the hand-over closed the menu,
/// and every close drops the model (`menu::PlaceKeeper`), so the editor draws from the layer's own view.
pub fn frame_needs_menu(phase: Phase) -> bool {
    phase != Phase::Clip
}

/// The phase after the menu's session is reset (every close moves the input generation). The clip editor outlives the
/// menu on purpose (the hand-over closes it), so it keeps drawing until the layer closes it (`next`: `HideClip`).
pub fn after_session_reset(phase: Phase) -> Phase {
    if phase == Phase::Clip { Phase::Clip } else { Phase::Menu }
}

/// The next phase and the step to take, given the phase and what the layer reports.
pub fn next(phase: Phase, seen: Seen) -> (Phase, Step) {
    match phase {
        Phase::Menu if !seen.menu_open && seen.prompt_visible && !seen.clip_visible => (Phase::Prompt, Step::ShowPrompt),
        Phase::Menu => (Phase::Menu, Step::Nothing),
        // The clip editor's own question answered or retired: the editor is drawn again.
        Phase::Prompt if !seen.prompt_visible && !seen.menu_open && seen.clip_visible => (Phase::Clip, Step::HidePrompt),
        Phase::Prompt if !seen.prompt_visible || seen.menu_open => (Phase::Menu, Step::HidePrompt),
        Phase::Prompt => (Phase::Prompt, Step::Nothing),
        Phase::Panel if !seen.menu_open => (Phase::Menu, Step::CloseLayer),
        Phase::Panel if seen.clip_visible && seen.clip_foreground => (Phase::Clip, Step::HandOverToClip),
        Phase::Panel if !seen.panel_visible => (Phase::Menu, Step::PopRow),
        Phase::Panel => (Phase::Panel, Step::Nothing),
        Phase::Clip if !seen.clip_visible => (Phase::Menu, Step::HideClip),
        // The editor asked its Fighter Mismatch question (a track or capture whose recorded fighter is not the CPU).
        Phase::Clip if seen.prompt_visible => (Phase::Prompt, Step::ShowPrompt),
        Phase::Clip => (Phase::Clip, Step::Nothing),
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn seen(menu_open: bool, panel_visible: bool, clip_visible: bool) -> Seen {
        Seen { menu_open, panel_visible, clip_visible, clip_foreground: clip_visible, prompt_visible: false }
    }

    /// The same runs: the hand-over's own close reset the menu session, and the reset put the worker back in Menu, so
    /// the editor was never drawn although it held the game. The clip phase survives the reset; the layer closing the
    /// editor ends it.
    fn the_hand_over_close_keeps_the_clip_editor_drawn() {
        assert_eq!(after_session_reset(Phase::Clip), Phase::Clip);
        assert_eq!(next(after_session_reset(Phase::Clip), seen(false, true, true)), (Phase::Clip, Step::Nothing));
        assert_eq!(next(Phase::Clip, seen(false, false, false)), (Phase::Menu, Step::HideClip));
        for phase in [Phase::Menu, Phase::Panel, Phase::Prompt] {
            assert_eq!(after_session_reset(phase), Phase::Menu);
        }
    }

    fn a_fighter_question_inside_the_open_menu_stays_the_menu_panels() {
        // The library and combo questions open from a menu row: Phase::Panel draws them, not Prompt.
        let in_menu = Seen { prompt_visible: true, ..seen(true, true, false) };
        assert_eq!(next(Phase::Menu, in_menu), (Phase::Menu, Step::Nothing));
        assert_eq!(next(Phase::Panel, in_menu), (Phase::Panel, Step::Nothing));
        // A clip editor's own question is the clip's.
        let clip = Seen { prompt_visible: true, ..seen(false, true, true) };
        assert_eq!(next(Phase::Menu, clip), (Phase::Menu, Step::Nothing));
        // The menu opening over a question hides the closed-menu copy.
        assert_eq!(next(Phase::Prompt, in_menu), (Phase::Menu, Step::HidePrompt));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_hand_over_close_keeps_the_clip_editor_drawn", the_hand_over_close_keeps_the_clip_editor_drawn as fn()),
            ("a_fighter_question_inside_the_open_menu_stays_the_menu_panels", a_fighter_question_inside_the_open_menu_stays_the_menu_panels as fn()),
        ]);
    }
    }
}
