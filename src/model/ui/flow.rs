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
    use super::*;

    fn seen(menu_open: bool, panel_visible: bool, clip_visible: bool) -> Seen {
        Seen { menu_open, panel_visible, clip_visible, clip_foreground: clip_visible, prompt_visible: false }
    }

    #[test]
    fn a_fighter_question_with_the_menu_closed_is_drawn_until_it_is_answered() {
        // rc.5: Play on a recording made on another fighter opened the question with the menu closed; nothing drew it or
        // gave it Player 1, and the held fighters stayed stopped.
        let asked = Seen { prompt_visible: true, ..seen(false, true, false) };
        assert_eq!(next(Phase::Menu, asked), (Phase::Prompt, Step::ShowPrompt));
        assert_eq!(next(Phase::Prompt, asked), (Phase::Prompt, Step::Nothing));
        let answered = seen(false, false, false);
        assert_eq!(next(Phase::Prompt, answered), (Phase::Menu, Step::HidePrompt));
    }

    /// Eden rc6-covrec-diag1 and diag2 (2026-11-08): the Replay window's A opened the clip editor (status editor=true; the
    /// game held on the clip), but no editor frame was ever drawn. The hand-over closes the menu, the close drops the
    /// menu's model, and the worker built every frame only when it still held a model.
    #[test]
    fn the_clip_editor_draws_without_a_menu_model() {
        let (phase, step) = next(Phase::Panel, seen(true, true, true));
        assert_eq!((phase, step), (Phase::Clip, Step::HandOverToClip));
        assert!(!frame_needs_menu(phase), "the editor's frame comes from the layer's view");
        for phase in [Phase::Menu, Phase::Panel, Phase::Prompt] {
            assert!(frame_needs_menu(phase), "{phase:?} is drawn from the menu's model");
        }
    }

    /// The same runs: the hand-over's own close reset the menu session, and the reset put the worker back in Menu, so
    /// the editor was never drawn although it held the game. The clip phase survives the reset; the layer closing the
    /// editor ends it.
    #[test]
    fn the_hand_over_close_keeps_the_clip_editor_drawn() {
        assert_eq!(after_session_reset(Phase::Clip), Phase::Clip);
        assert_eq!(next(after_session_reset(Phase::Clip), seen(false, true, true)), (Phase::Clip, Step::Nothing));
        assert_eq!(next(Phase::Clip, seen(false, false, false)), (Phase::Menu, Step::HideClip));
        for phase in [Phase::Menu, Phase::Panel, Phase::Prompt] {
            assert_eq!(after_session_reset(phase), Phase::Menu);
        }
    }

    #[test]
    fn the_question_keeps_player_1_until_nothing_holds_it() {
        // A press during the question used to release the bit (the pop-up's sync saw neither menu nor pop-up).
        assert!(menu_owns_input(false, false, false, true));
        assert!(!menu_owns_input(false, false, false, false));
        assert!(menu_owns_input(true, false, false, false) && menu_owns_input(false, true, false, false));
        assert!(menu_owns_input(false, false, true, false));
    }

    #[test]
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
    fn mismatch_keeps_menu_input_until_the_layer_releases_it() {
        // ui-16: the replay is loaded, but Fighter Mismatch still needs physical A/B.
        let mismatch = Seen { clip_foreground: false, ..seen(true, true, true) };
        assert_eq!(next(Phase::Panel, mismatch), (Phase::Panel, Step::Nothing));
        let accepted = Seen { clip_foreground: true, ..mismatch };
        assert_eq!(next(Phase::Panel, accepted), (Phase::Clip, Step::HandOverToClip));
        // Canceling the pending replay instead returns to the Recording row.
        assert_eq!(next(Phase::Panel, seen(true, false, false)), (Phase::Menu, Step::PopRow));
    }

    #[test]
    fn keyboard_over_an_existing_clip_does_not_discard_the_clip_phase() {
        let naming = Seen { clip_foreground: false, ..seen(false, true, true) };
        assert_eq!(next(Phase::Clip, naming), (Phase::Clip, Step::Nothing));
        assert_eq!(next(Phase::Clip, seen(false, true, true)), (Phase::Clip, Step::Nothing));
    }

    /// Eden covrest-yoga-01 (2026-11-08, `p23/rc6-cov-rest` with the #68 fix): Down to the Mii track resolved the
    /// recorded Mii and opened the clip editor's own Fighter Mismatch question (`[NamedMacroDiag] view title="Fighter
    /// Mismatch"`), but the frames kept showing the editor: the Clip phase had no way to the question, so nothing drew
    /// it and the editor kept Player 1's presses (Down and A went to the editor). The question is drawn alone and owns
    /// the presses; once answered, the editor is drawn again.
    #[test]
    fn the_clip_editors_own_question_is_drawn_and_then_the_editor_again() {
        let editor = seen(false, true, true);
        let asked = Seen { prompt_visible: true, ..editor };
        assert_eq!(next(Phase::Clip, asked), (Phase::Prompt, Step::ShowPrompt));
        assert_eq!(next(Phase::Prompt, asked), (Phase::Prompt, Step::Nothing));
        assert_eq!(next(Phase::Prompt, editor), (Phase::Clip, Step::HidePrompt), "answered: back to the editor");
        assert_eq!(next(Phase::Prompt, seen(false, false, false)), (Phase::Menu, Step::HidePrompt));
        assert_eq!(next(Phase::Clip, Seen { prompt_visible: true, ..seen(false, false, false) }), (Phase::Menu, Step::HideClip));
        let clip = include_str!("../../runtime/macros/replay_capture.rs");
        assert!(clip.contains("if crate::runtime::macros::named_macros::fighter_prompt_up() { return; }"),
            "the question owns Player 1's presses, not the editor");
    }

    #[test]
    fn the_menu_phase_does_nothing_on_its_own() {
        assert_eq!(next(Phase::Menu, seen(true, true, true)), (Phase::Menu, Step::Nothing));
    }
}
