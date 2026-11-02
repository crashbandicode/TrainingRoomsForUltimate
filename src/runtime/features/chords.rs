//! Button chords, the runtime half (SPEC-input-04): the P1 poll's detection stage, the shortcuts file,
//! the request flags the consumers pull. The rules are `model::chords`; the file transaction is
//! `model::ini_files`.
//!
//! Detection runs once per P1 poll, before the input delay (so a chord is never delayed), on the
//! controller the game maps, read through the pointer the poll hook passes. The Plus bits are cleared
//! only for a modal owner or an accepted Plus shortcut (SPEC-input-04 §4).
//!
//! NOT YET SWITCH-RUN.

use std::path::Path;

use parking_lot::Mutex;

use crate::model::chords::{self, field, ChordSet, Detector, Request, Sample};
use crate::model::ini_files;
use crate::model::input_delay::Owners;
#[cfg(feature = "diag")]
use crate::model::menu::chord::Chord;
use crate::model::menu_shortcuts;
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::published;

struct State {
    chords: ChordSet,
    detector: Detector,
    /// The sample of the last poll (the cycle chord of an operator controller reads its own).
    style: u32,
}

static STATE: Mutex<State> = Mutex::new(State { chords: ChordSet::factory(), detector: Detector::new(), style: 5 });

/// Owners that block the chords: everything but this stage's own stop-chord bit.
fn blocked() -> bool {
    Owners::from_bits(published::input_owners().bits() & !Owners::STOP_CHORD.bits()).any()
}

/// Read the controller's raw button sets and run the detector (one P1 poll). `controller` is the
/// game's `Controller` for player 0 (SPEC-input-04 §3.1).
///
/// # Safety
/// `controller` is the live `Controller` pointer the mapping call passed, or 0.
pub unsafe fn on_poll(controller: usize) {
    if controller == 0 {
        return;
    }
    let read = |offset: usize| ((controller + offset) as *const u32).read_volatile();
    let style = read(field::STYLE);
    let sample = Sample::of(style, read(field::CURRENT_BUTTONS), read(field::JUST_DOWN));
    let modal = published::input_owners().contains(Owners::MENU) || published::input_owners().contains(Owners::REPLAY_EDITOR);
    let accepted = {
        let mut state = STATE.lock();
        state.style = style;
        let block = blocked();
        let State { chords: set, detector, .. } = &mut *state;
        detector.poll(set, sample, block);
        !block && chords::plus_shortcut_accepted(set, sample)
    };
    if chords::clear_plus_bits(modal, accepted) {
        for offset in [field::CURRENT_BUTTONS, field::PREVIOUS_BUTTONS, field::JUST_DOWN, field::JUST_RELEASE] {
            let at = (controller + offset) as *mut u32;
            at.write_volatile(at.read_volatile() & !chords::bit::PLUS_MASK);
        }
    }
}

/// `combo_passes`: the request is pending; clears it.
pub fn consume(request: Request) -> bool {
    STATE.lock().detector.consume(request)
}

/// Session clear: all pending requests go.
pub fn session_clear() {
    STATE.lock().detector.clear_session();
}

/// A menu commit or the first publish: the six chord rows replace the old ones as a whole; the
/// shortcuts (they live in the INI) are kept.
pub fn publish(snapshot: &Snapshot) {
    let mut state = STATE.lock();
    state.chords = state.chords.with_rows(snapshot);
}

fn ini_path() -> Option<FixedPath<128>> {
    FixedPath::<128>::in_root(menu_shortcuts::FILE_NAME)
}

/// Boot: read `button_config.ini` (fail-closed), creating it with the default text when absent.
pub fn load_ini() {
    let Some(path) = ini_path() else { return };
    let path = Path::new(path.as_str());
    ini_files::restore_orphaned_backup(path);
    let (shortcuts, error) = ini_files::load_shortcuts(path);
    if let Some(error) = error {
        diagnostic(format_args!("[TrainingRooms] button_config.ini could not be created: {error}"));
    }
    STATE.lock().chords.shortcuts = shortcuts;
}

/// The menu's confirmed shortcut change (draft 9, N7); package 5's menu calls it, the diag build's stand-in
/// (`recorder::chord_step`) until then: the new chords are live at once, and the file
/// is rewritten as a transaction. A refused edit changes nothing; a failed write keeps the change in
/// memory for the session and says so (SPEC-input-04 question 6 (b), (c)).
#[cfg(feature = "diag")]
pub fn commit_shortcut(slot: usize, chord: Chord) -> Result<(), ini_files::EditRefusal> {
    let current = STATE.lock().chords.shortcuts;
    let next = ini_files::check_edit(current, slot, chord)?;
    STATE.lock().chords.shortcuts = next;
    if let Some(path) = ini_path() {
        if let Err(error) = ini_files::write_shortcuts(Path::new(path.as_str()), next) {
            diagnostic(format_args!("[TrainingRooms] button_config.ini not written: {error}; kept for this session"));
            crate::runtime::features::notices::post("Menu shortcuts", "Could not save button_config.ini", 120, [0, 0, 0, 255]);
        }
    }
    Ok(())
}
