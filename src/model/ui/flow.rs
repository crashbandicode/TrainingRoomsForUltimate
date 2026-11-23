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

/// What the worker does about the Recording page's name draft on one pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameStep { Nothing, Cancel, Discard }

/// `page`: the page's keyboard is up. `asked`: its offer reached the layer and no answer is waiting to be taken.
/// `draft`: the layer's draft for it is open. An open draft holds both fighters and is never drawn for this page,
/// so it may be open only while the page waits for it.
pub const fn name_step(page: bool, asked: bool, draft: bool) -> NameStep {
    match (page, asked, draft) {
        (true, true, false) => NameStep::Cancel,
        (false, _, true) => NameStep::Discard,
        _ => NameStep::Nothing,
    }
}

/// Where the named layer's one system-keyboard job stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamePhase { Active, Closing, Parked, Discarding }

/// The named layer's one system-keyboard job. `owner`: the token of the caller-owned draft it names, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameJob { pub id: u64, pub generation: u64, pub replay: bool, pub waiting: bool, pub phase: NamePhase, pub owner: Option<u64> }

impl NameJob {
    /// Retire the slot's job. `Some(id)`: interrupt that keyboard. A waiting job stays until its keyboard has closed
    /// (`Closing`: parked then; `Discarding`: dropped then); any other job leaves at once.
    pub fn retire(slot: &mut Option<Self>, keep_waiting: bool) -> Option<u64> {
        let job = slot.as_mut()?;
        let id = job.id;
        if job.waiting && keep_waiting && job.phase != NamePhase::Discarding {
            if job.phase == NamePhase::Parked { return None; }
            job.phase = NamePhase::Closing;
        } else if job.waiting && job.phase != NamePhase::Parked {
            job.phase = NamePhase::Discarding;
        } else {
            *slot = None;
        }
        Some(id)
    }

    /// The owner of a caller-owned draft is gone: its job never comes back. Another owner's job is left alone.
    pub fn retire_owner(slot: &mut Option<Self>, owner: u64) -> Option<u64> {
        if slot.is_some_and(|job| job.owner == Some(owner)) { Self::retire(slot, false) } else { None }
    }

    /// Keyboard `id` closed. `Some`: the job whose answer is applied. `None`: a stale id, a discarded job (dropped
    /// here) or a waiting job parked for later.
    pub fn complete(slot: &mut Option<Self>, id: u64, cancelled: bool, interrupted: bool, ready: bool, generation: u64) -> Option<Self> {
        let job = slot.as_mut().filter(|job| job.id == id)?;
        if job.phase == NamePhase::Discarding { *slot = None; return None; }
        if job.waiting && !cancelled && (job.phase == NamePhase::Closing || !ready || job.generation != generation || interrupted) {
            job.phase = NamePhase::Parked;
            return None;
        }
        slot.take()
    }
}

/// Whether the menu side still owns Player 1's input (the `Owners::MENU` bit): the open menu, its closing frames, the
/// pop-up, or the closed-menu fighter question. Releasing on any one of them alone handed the question's presses to the
/// match and let the frame strip draw over it (rc.5 XPS proof).
pub fn menu_owns_input(open: bool, popup: bool, closing: bool, prompt: bool) -> bool {
    open || popup || closing || prompt
}

/// Whether the open menu holds the fighters that are not Player 1, as its panels hold both: the CPU stands frozen
/// behind the menu. Not while a preview plays, a save state loads, the recorder is busy or a macro plays on the CPU:
/// those go on as before.
pub fn menu_holds_cpu(open: bool, previewing: bool, loading: bool, recorder: bool) -> bool {
    open && !previewing && !loading && !recorder
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
        assert!(hold.contains("menu && !player"));
        assert!(hold.contains("clip_previewing") && hold.contains("replay_loading"));
        let state = include_str!("../../runtime/ui/state.rs");
        let fact = state.split("pub fn holds_cpu(").nth(1).unwrap_or("").split("\n}").next().unwrap();
        assert!(fact.contains("OPEN.load("));
        assert!(fact.contains("states::is_loading()"));
        assert!(fact.contains("recorder::recording_or_playback()"));
        assert!(fact.contains("recording_host::cpu_playback_blocks_drill()"));
    }

    fn f9_name_step_truth_table() {
        for (page, asked, draft, expected) in [
            (false, false, false, NameStep::Nothing), (false, false, true, NameStep::Discard),
            (false, true, false, NameStep::Nothing), (false, true, true, NameStep::Discard),
            (true, false, false, NameStep::Nothing), (true, false, true, NameStep::Nothing),
            (true, true, false, NameStep::Cancel), (true, true, true, NameStep::Nothing),
        ] { assert_eq!(name_step(page, asked, draft), expected); }
    }

    fn f9_job(waiting: bool, phase: NamePhase, owner: Option<u64>) -> NameJob {
        NameJob { id: 19, generation: 7, replay: false, waiting, phase, owner }
    }

    fn f9_owner_loss_retires_the_keyboard_job() {
        use crate::engine::macro_ui::macro_panel::{MacroUi, NameContext, Pad};
        let token = 1u64 << 62;
        for (waiting, phase, remaining) in [
            (false, NamePhase::Active, None),
            (true, NamePhase::Active, Some(NamePhase::Discarding)),
            (true, NamePhase::Closing, Some(NamePhase::Discarding)),
            (true, NamePhase::Parked, None),
        ] {
            let mut ui = MacroUi::new();
            ui.offer_name(NameContext { token, title: "Name chain" }, "", "");
            assert!(ui.is_open());
            assert_eq!(ui.external_name_token(), Some(token));
            assert_eq!(name_step(false, true, true), NameStep::Discard);
            let mut slot = Some(f9_job(waiting, phase, Some(token)));
            assert_eq!(NameJob::retire_owner(&mut slot, token), Some(19));
            assert_eq!(slot.map(|job| job.phase), remaining);
            ui.on_pad(Pad::Back);
            assert!(!ui.is_open());
            assert_eq!(ui.external_name_token(), None);
            assert_eq!(NameJob::complete(&mut slot, 19, false, false, true, 7), None);
            assert_eq!(slot, None);
        }
    }

    fn f9_another_owners_job_is_left_alone() {
        let token = 1u64 << 62;
        for owner in [None, Some(token + 1)] {
            for phase in [NamePhase::Active, NamePhase::Closing, NamePhase::Parked, NamePhase::Discarding] {
                let mut slot = Some(f9_job(true, phase, owner));
                let before = slot;
                assert_eq!(NameJob::retire_owner(&mut slot, token), None);
                assert_eq!(slot, before);
            }
        }
    }

    fn f9_name_job_retire_and_complete_rules() {
        use NamePhase::*;
        for (waiting, phase, keep, result, after) in [
            (true, Active, true, Some(19), Some(Closing)),
            (true, Parked, true, None, Some(Parked)),
            (true, Discarding, true, Some(19), Some(Discarding)),
            (true, Active, false, Some(19), Some(Discarding)),
            (true, Closing, false, Some(19), Some(Discarding)),
            (true, Parked, false, Some(19), None),
            (false, Active, false, Some(19), None),
            (false, Active, true, Some(19), None),
        ] {
            let mut slot = Some(f9_job(waiting, phase, None));
            assert_eq!(NameJob::retire(&mut slot, keep), result);
            assert_eq!(slot.map(|job| job.phase), after);
        }
        let original = Some(f9_job(true, Active, None));
        let mut slot = original;
        assert_eq!(NameJob::complete(&mut slot, 20, false, false, true, 7), None);
        assert_eq!(slot, original);
        for (waiting, phase, cancelled, interrupted, ready, generation, consumed, after) in [
            (true, Discarding, false, false, true, 7, false, None),
            (true, Closing, false, false, true, 7, false, Some(Parked)),
            (true, Active, false, false, false, 7, false, Some(Parked)),
            (true, Active, false, false, true, 8, false, Some(Parked)),
            (true, Active, false, true, true, 7, false, Some(Parked)),
            (true, Closing, true, false, true, 7, true, None),
            (true, Active, false, false, true, 7, true, None),
            (false, Active, false, false, false, 8, true, None),
        ] {
            let job = f9_job(waiting, phase, None);
            let mut slot = Some(job);
            assert_eq!(NameJob::complete(&mut slot, 19, cancelled, interrupted, ready, generation),
                if consumed { Some(job) } else { None });
            assert_eq!(slot.map(|job| job.phase), after);
        }
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
            ("f9_name_step_truth_table", f9_name_step_truth_table as fn()),
            ("f9_owner_loss_retires_the_keyboard_job", f9_owner_loss_retires_the_keyboard_job as fn()),
            ("f9_another_owners_job_is_left_alone", f9_another_owners_job_is_left_alone as fn()),
            ("f9_name_job_retire_and_complete_rules", f9_name_job_retire_and_complete_rules as fn()),
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
