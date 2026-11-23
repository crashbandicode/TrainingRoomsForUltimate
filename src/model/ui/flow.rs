//! Which screen owns the buttons while a panel of the named-macro / replay layer is involved (the menu's own page, a layer panel
//! opened from a Recording row, or the clip editor that took Player 1 from the menu). Pure, so the transitions are host-tested; the
//! menu worker (`runtime::ui::worker`) asks it after every press and every poll and does what it answers.

use super::frame::CLOSE_FRAMES;

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

/// Whether the open menu holds the fighters that are not Player 1, as its panels hold both: the CPU stands frozen
/// behind the menu. Not while a preview plays, a save state loads or the recorder is busy: those go on as before.
pub fn menu_holds_cpu(_open: bool, _previewing: bool, _loading: bool, _recorder: bool) -> bool {
    false
}

/// Whether the menu's own screens are what Player 1's presses reach now, as the worker's `press` routes them: the
/// open menu (not while it waits to close), or the Recording pop-up over the closed menu. Not the layer's panel, the
/// clip editor or the closed-menu fighter question. An external screen of the open menu (the Mii list) is the
/// worker's own list: the model reports no focused list for it, so the bridge publishes none there either.
pub fn model_takes_presses(open: bool, popup: bool, phase: Phase, pending_close: bool) -> bool {
    match phase {
        Phase::Clip => false,
        Phase::Prompt if !open => false,
        Phase::Panel if open => false,
        _ => if open { !pending_close } else { popup },
    }
}

/// What a close window starts waiting for (SPEC-core-03 §2.3): every action button (A, B, X, Y, L, R, ZL, ZR, the adapter's
/// logical bits 0–7). Each P1 poll then keeps only the ones still held (`held & poll`), so the window waits for the buttons
/// held at its first poll, the press that closed it among them. Directions are movement and are not waited for.
pub const CLOSE_HELD_START: u32 = 0x00ff;

/// The close window of the menu or the pop-up ends after its `CLOSE_FRAMES` real frames once nothing it waits for is still
/// held, so a B held through the close never reaches the fighter as a fresh press (CODE-FINDINGS #23).
pub fn close_done(count: u32, held: u32) -> bool {
    count >= CLOSE_FRAMES && held == 0
}

/// The pop-up just closed over the closed menu (`was` is the state the worker last published): the worker starts the close
/// window before `POPUP` falls, so the owner bit never drops on the press that closed it (`menu_owns_input` keeps it while
/// closing). Releasing on that press handed the B to the fighter (CODE-FINDINGS #23, Mario's fireball).
pub fn popup_closed(menu_open: bool, was: bool, now: bool) -> bool {
    was && !now && !menu_open
}

/// Whether the frame of `phase` is built from the menu's model. The clip editor's is not: the hand-over closed the menu,
/// and every close drops the model (`menu::PlaceKeeper`), so the editor draws from the layer's own view.
pub fn frame_needs_menu(phase: Phase) -> bool {
    phase != Phase::Clip
}

/// Clip state outlives the disposable menu, including the last sync that hides it.
pub fn panels_can_sync(menu_present: bool, phase: Phase, prompt_visible: bool, open: bool) -> bool {
    menu_present || phase == Phase::Clip || (phase == Phase::Menu && prompt_visible && !open)
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
        Phase::Menu if seen.menu_open && seen.clip_visible && seen.clip_foreground => (Phase::Clip, Step::HandOverToClip),
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

    fn the_open_menu_holds_the_cpu() {
        for open in [false, true] { for previewing in [false, true] {
            for loading in [false, true] { for recorder in [false, true] {
                assert_eq!(menu_holds_cpu(open, previewing, loading, recorder),
                    open && !previewing && !loading && !recorder,
                    "open={open} previewing={previewing} loading={loading} recorder={recorder}");
            } }
        } }
    }

    fn the_cpu_hold_is_wired() {
        let named = include_str!("../../runtime/macros/named_macros.rs");
        let hold = named.split("pub unsafe fn hold_panel_fighter(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(hold.contains("state::holds_cpu(queries.clip_previewing)"));
        assert!(hold.contains("!player()"));
        assert!(hold.contains("clip_previewing") && hold.contains("replay_loading"));
        let state = include_str!("../../runtime/ui/state.rs");
        let fact = state.split("pub fn holds_cpu(").nth(1).unwrap_or("").split("\n}").next().unwrap();
        assert!(fact.contains("OPEN.load("));
        assert!(fact.contains("states::is_loading()"));
        assert!(fact.contains("recorder::recording_or_playback()"));
    }

    fn seen(menu_open: bool, panel_visible: bool, clip_visible: bool) -> Seen {
        Seen { menu_open, panel_visible, clip_visible, clip_foreground: clip_visible, prompt_visible: false }
    }

    /// The same runs: the hand-over's own close reset the menu session, and the reset put the worker back in Menu, so
    /// the editor was never drawn although it held the game. The clip phase survives the reset; the layer closing the
    /// editor ends it.
    fn the_hand_over_close_keeps_the_clip_editor_drawn() {
        // A file's Crop opens directly from the menu; a replay opens from its panel.
        for phase in [Phase::Menu, Phase::Panel] {
            assert_eq!(next(phase, seen(true, true, true)), (Phase::Clip, Step::HandOverToClip));
            assert_eq!(next(phase, Seen { clip_foreground: false, ..seen(true, true, true) }), (phase, Step::Nothing));
        }
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

    fn clip_sync_outlives_menu_on_both_entry_routes() {
        for entry in [Phase::Menu, Phase::Panel] {
            let (phase, step) = next(entry, seen(true, true, true));
            assert_eq!(step, Step::HandOverToClip);
            let phase = after_session_reset(phase);
            assert!(panels_can_sync(false, phase, false, false), "handle and preview redraw after menu drops");
            assert_eq!(next(phase, seen(false, true, true)), (Phase::Clip, Step::Nothing));
            assert!(panels_can_sync(false, phase, false, false), "B must reach HideClip");
            assert_eq!(next(phase, seen(false, false, false)), (Phase::Menu, Step::HideClip));
        }
        for phase in [Phase::Menu, Phase::Panel, Phase::Prompt] {
            for menu in [false, true] { for prompt in [false, true] { for open in [false, true] {
                let previous = menu || (phase == Phase::Menu && prompt && !open);
                assert_eq!(panels_can_sync(menu, phase, prompt, open), previous);
            } } }
        }
    }

    /// CODE-FINDINGS #23: the pop-up's close starts the close window; over the open menu the menu's own close does.
    fn the_pop_ups_close_starts_the_close_window() {
        assert!(popup_closed(false, true, false));
        assert!(!popup_closed(true, true, false));
        assert!(!popup_closed(false, false, true));
        assert!(!popup_closed(false, true, true));
        assert!(!popup_closed(false, false, false));
        // The window keeps the owner bit the pop-up took.
        assert!(menu_owns_input(false, false, true, false));
    }

    /// The window waits for the action buttons held at its first poll, and 15 frames stay the minimum.
    fn the_close_window_waits_for_the_buttons_held_when_it_began() {
        const A: u32 = 1 << 0;
        const B: u32 = 1 << 1;
        const DPAD_UP: u32 = 1 << 8;
        let first = CLOSE_HELD_START & (A | B | DPAD_UP);
        assert_eq!(first, A | B, "a held direction is movement");
        assert!(!close_done(CLOSE_FRAMES * 10, first & B));
        assert!(!close_done(CLOSE_FRAMES - 1, 0));
        assert!(close_done(CLOSE_FRAMES, first & !(A | B)));
        assert!(!close_done(CLOSE_FRAMES, CLOSE_HELD_START), "a window waits for its first poll");
    }

    fn the_model_list_follows_the_press_owner() {
        use Phase::*;
        // The Mii list is an external screen with no model list.
        for (phase, open, popup, pending, expected) in [
            (Menu, false, false, false, false), (Menu, false, false, true, false),
            (Menu, false, true, false, true), (Menu, false, true, true, true),
            (Menu, true, false, false, true), (Menu, true, false, true, false),
            (Menu, true, true, false, true), (Menu, true, true, true, false),
            (Panel, false, false, false, false), (Panel, false, false, true, false),
            (Panel, false, true, false, true), (Panel, false, true, true, true),
            (Panel, true, false, false, false), (Panel, true, false, true, false),
            (Panel, true, true, false, false), (Panel, true, true, true, false),
            (Clip, false, false, false, false), (Clip, false, false, true, false),
            (Clip, false, true, false, false), (Clip, false, true, true, false),
            (Clip, true, false, false, false), (Clip, true, false, true, false),
            (Clip, true, true, false, false), (Clip, true, true, true, false),
            (Prompt, false, false, false, false), (Prompt, false, false, true, false),
            (Prompt, false, true, false, false), (Prompt, false, true, true, false),
            (Prompt, true, false, false, true), (Prompt, true, false, true, false),
            (Prompt, true, true, false, true), (Prompt, true, true, true, false),
        ] {
            assert_eq!(model_takes_presses(open, popup, phase, pending), expected, "{phase:?} {open} {popup} {pending}");
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_open_menu_holds_the_cpu", the_open_menu_holds_the_cpu as fn()),
            ("the_cpu_hold_is_wired", the_cpu_hold_is_wired as fn()),
            ("the_hand_over_close_keeps_the_clip_editor_drawn", the_hand_over_close_keeps_the_clip_editor_drawn as fn()),
            ("clip_sync_outlives_menu_on_both_entry_routes", clip_sync_outlives_menu_on_both_entry_routes as fn()),
            ("a_fighter_question_inside_the_open_menu_stays_the_menu_panels", a_fighter_question_inside_the_open_menu_stays_the_menu_panels as fn()),
            ("the_pop_ups_close_starts_the_close_window", the_pop_ups_close_starts_the_close_window as fn()),
            ("the_close_window_waits_for_the_buttons_held_when_it_began", the_close_window_waits_for_the_buttons_held_when_it_began as fn()),
            ("the_model_list_follows_the_press_owner", the_model_list_follows_the_press_owner as fn()),
        ]);
    }
    }
}
