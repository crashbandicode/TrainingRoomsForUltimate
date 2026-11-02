//! The Recording page's panels on the menu (package 4's named-macro / replay layer): Named library, Save slot to library, Edit
//! placement and the Replay window. The layer keeps its own panel state (`named_macros`, `replay_capture`); this module opens a panel
//! when the menu's row is pressed, hands the player's buttons to it as the layer's pads, and says what is on screen so the worker can
//! draw it (`model::screens::panel_frame`, the clip editor's `editors::clip_frame`).
//!
//! Threads (lead rulings 2026-11-02, design C): the layer's panel model has one owner, the menu worker. Everything here runs on the worker:
//! presses arrive over the input hook's lock-free ring, the worker runs the layer's entry points (they may allocate and do file I/O here,
//! never in a hook) and builds the menu frame from the layer's `View`; the UI tick draws that frame from a wait-free triple buffer
//! (`ui::state::FRAME`). Game-thread events reach the layer through its own `hos_lock::Mailbox`, drained in `macros::port::worker_tick`;
//! hooks read the layer's published queries, never the model (package 4's side of design C). What the entry points need from the game is
//! the fact the frame wrapper published (`ui::facts`).

use crate::engine::macro_ui::macro_panel::{Pad, View};
use crate::model::menu::input::Button;
use crate::model::menu::table::Editor;
use crate::runtime::macros::{named_macros, replay_capture};

/// The editors this module drives.
pub fn is_layer_panel(editor: Editor) -> bool {
    matches!(editor, Editor::NamedLibrary | Editor::SaveSlotToLibrary | Editor::EditPlacement | Editor::ReplayWindow)
}

/// Open the panel of `editor`. `slot` is the Record Slot of the page (`recording_slot`), which the library and the export freeze.
/// True when a panel is on screen afterwards; false when the layer refused (its notice says why).
pub fn open_panel(editor: Editor, slot: Option<usize>) -> bool {
    match editor {
        Editor::NamedLibrary => named_macros::open_library(slot),
        Editor::SaveSlotToLibrary => named_macros::export_record_slot(slot),
        Editor::EditPlacement => named_macros::open_editor(),
        Editor::ReplayWindow => replay_capture::open_editor(),
        _ => {}
    }
    panel_up()
}

/// Any panel of the layer is up (the name page, the library, the placement page, the capture browser or the clip editor).
pub fn panel_up() -> bool {
    named_macros::panel_visible()
}

/// The clip editor is up: it owns Player 1's controls, so the menu closes and the editor draws over the Training HUD.
pub fn clip_up() -> bool {
    !named_macros::suspended() && replay_capture::panel_visible()
}

/// The layer's pad for a menu button (D-pad moves, A confirms, B backs out, Y is its extra key).
pub fn pad_of(button: Button) -> Option<Pad> {
    Some(match button {
        Button::Left => Pad::Left,
        Button::Right => Pad::Right,
        Button::Up => Pad::Up,
        Button::Down => Pad::Down,
        Button::A => Pad::Confirm,
        Button::B => Pad::Back,
        Button::Y => Pad::Extra,
        _ => return None,
    })
}

/// A button on a panel.
pub fn press_panel(button: Button) {
    if let Some(pad) = pad_of(button) {
        named_macros::on_pad(pad);
    }
}

/// What the panel shows now (the layer's own view: title, footer, the labels of its 4 x 8 page).
pub fn panel_view_now() -> Option<View> {
    named_macros::panel_view()
}

/// Step back out of a panel the menu can no longer show (the menu lost its renderer, the session ended): B until it is gone.
pub fn close_panel() {
    for _ in 0..4 {
        if !panel_up() || clip_up() {
            break;
        }
        named_macros::on_pad(Pad::Back);
    }
}

/// Why `editor`'s panel did not open, in the words of the layer's own refusals (its notice is hidden while the menu is open, so the
/// menu says it on the row's help line). The predicates are the layer's: the published facts, the recorder and the save states.
pub fn refusal_of(editor: Editor, slot: Option<usize>) -> &'static str {
    use crate::runtime::macros::port;
    if matches!(editor, Editor::NamedLibrary | Editor::SaveSlotToLibrary) && slot.is_none() {
        return "Choose a Record Slot first";
    }
    if !crate::runtime::ui::facts::fact_fighters_present() {
        return "Wait for both practice fighters";
    }
    if port::recording_or_playback() || port::save_states::is_loading() {
        return "Finish the current practice action first";
    }
    match editor {
        Editor::EditPlacement => "Load a macro and restore a Training state first",
        Editor::ReplayWindow => "Open the replay window in idle practice, with a saved replay capture",
        Editor::SaveSlotToLibrary => "Record into the slot first",
        _ => "Not available right now",
    }
}
