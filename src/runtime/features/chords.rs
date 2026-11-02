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
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use crate::model::chords::{self, field, ChordSet, Detector, Request, Sample};
use crate::model::ini_files;
use crate::model::input_delay::Owners;
use crate::model::menu::chord::Chord;
use crate::model::menu_shortcuts::{self, MenuShortcuts};
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::recorder;
use crate::runtime::published;

struct State {
    chords: ChordSet,
    detector: Detector,
    /// The sample of the last poll (the cycle chord of an operator controller reads its own).
    style: u32,
}

static STATE: Mutex<State> = Mutex::new(State { chords: ChordSet::factory(), detector: Detector::new(), style: 5 });

/// The two menu shortcuts as the INI and the menu's edits have them (chord masks, slot 0 low, slot 1 high), for the
/// menu's input adapter and the menu's display: read without a lock on the game's input thread.
static SHORTCUT_MASKS: AtomicU32 = AtomicU32::new(menu_shortcuts::DEFAULT_SHORTCUTS.slots()[0].0 as u32);

fn publish_shortcuts(shortcuts: MenuShortcuts) {
    let [a, b] = shortcuts.slots();
    SHORTCUT_MASKS.store(u32::from(a.0) | u32::from(b.0) << 16, Ordering::Release);
}

/// The two menu shortcuts now.
pub fn shortcuts() -> [Chord; 2] {
    let masks = SHORTCUT_MASKS.load(Ordering::Acquire);
    [Chord(masks as u16), Chord((masks >> 16) as u16)]
}

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
    let modal = published::input_owners().contains(Owners::MENU) || recorder::replay_editor_occupied();
    let accepted = {
        let mut state = STATE.lock();
        state.style = style;
        let block = blocked();
        let State { chords: set, detector, .. } = &mut *state;
        let passed = detector.poll(set, sample, block);
        #[cfg(feature = "diag")]
        {
            if passed != 0 {
                diagnostic(format_args!("[TrainingRooms] chords_passed bits={passed:#x}"));
            }
        }
        #[cfg(not(feature = "diag"))]
        let _ = passed;
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

/// The CPU cycle chord passes on the controller that operates the CPU (package 6: native Behavior = Control): its own raw sample,
/// no pending flag.
///
/// # Safety
/// `controller` is the live `Controller` pointer of the operator's poll, or 0.
pub unsafe fn operator_cycle(controller: usize) -> bool {
    if controller == 0 {
        return false;
    }
    let read = |offset: usize| ((controller + offset) as *const u32).read_volatile();
    let sample = Sample::of(read(field::STYLE), read(field::CURRENT_BUTTONS), read(field::JUST_DOWN));
    let state = STATE.lock();
    !blocked() && Detector::operator_cycle(&state.chords, sample)
}

/// Some shortcut request is pending (package 6's roster cycle waits for it to be handled).
pub fn any_pending() -> bool {
    STATE.lock().detector.has_pending_request()
}

/// The menu shortcut chord passed since the last call (the INI shortcuts, detected from the raw controller before
/// the Plus bits are cleared for an accepted Plus shortcut): package 5's input adapter opens the menu from it.
pub fn take_open_menu() -> bool {
    STATE.lock().detector.consume(Request::OpenMenu)
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
    publish_shortcuts(shortcuts);
}

/// The menu's confirmed shortcut change (draft 9, N7), called by package 5's menu worker when it closes: the new
/// chords are live at once and the file is rewritten by the writer thread as a transaction (this call only stores
/// the request: no file I/O here). A refused edit changes nothing; a failed write keeps the change in memory for
/// the session and says so (SPEC-input-04 question 6 (b), (c)).
pub fn commit_shortcut(slot: usize, chord: Chord) -> Result<(), ini_files::EditRefusal> {
    let current = STATE.lock().chords.shortcuts;
    let next = ini_files::check_edit(current, slot, chord)?;
    STATE.lock().chords.shortcuts = next;
    publish_shortcuts(next);
    crate::runtime::features::writer::request_shortcuts(next);
    Ok(())
}