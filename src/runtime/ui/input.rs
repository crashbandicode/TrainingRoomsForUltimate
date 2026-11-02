//! Player 1's input for the menu (SPEC-core-03 §3): once per P1 poll of the game's final input mapping,
//! the hook hands this module the controller holder the call was given. The holder's record carries the
//! raw buttons (`current_buttons` at +0x8) and the controller style (+0x9c, the field the game's own mapping reads at `main+0x1750e0c`: `ldr w2, [x20, #0x9c]` after `ldr x20, [x3, #0x10]`; the same `Controller` package 4's chords read, `chords::field`). The adapter (`model::ui::
//! adapter`) turns them into the open chord and logical presses, which go to the menu worker through a
//! lock-free ring; while the menu is open or closing the `Owners::MENU` bit empties P1's mapped output
//! (the delay stage of `features::input_delay` already neutralises a poll whose owners are set).
//!
//! The hook allocates nothing and takes no blocking lock.

use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use crate::model::input_delay::Owners;
use crate::model::adapter::{Adapter, Style};
use crate::runtime::features::chords;
use crate::runtime::published::INPUT_OWNERS;
use crate::model::readiness;
use crate::runtime::ui::state::{self, OPEN};
use crate::runtime::ui::worker;

static ADAPTER: Mutex<Adapter> = Mutex::new(Adapter::new());
/// The last record read, for the diagnostics.
pub static LAST_BUTTONS: AtomicU32 = AtomicU32::new(0);
pub static POLLS: AtomicU32 = AtomicU32::new(0);

/// The holder is `x3` of the call: a pointer whose `+0x10` is the controller record.
unsafe fn read_record(holder: usize) -> Option<(u32, Style)> {
    if holder == 0 || holder % 8 != 0 {
        return None;
    }
    let record = *((holder + 0x10) as *const usize);
    if record == 0 || record % 4 != 0 {
        return None;
    }
    let buttons = *((record + crate::model::chords::field::CURRENT_BUTTONS) as *const u32);
    let style = *((record + crate::model::chords::field::STYLE) as *const u32);
    Some((buttons, Style::from_raw(style)))
}

/// The session ended: the adapter forgets the held buttons and the open chord's progress. A poll in
/// progress keeps its lock; the adapter is then reset by the next session end (it holds no other state).
pub fn reset_session() {
    if let Some(mut adapter) = ADAPTER.try_lock() {
        *adapter = Adapter::new();
    }
}

/// One P1 poll in Training.
pub fn on_p1_poll(holder: usize) {
    // SAFETY: `holder` is the controller-holder argument of the game's call; the reads are the record's
    // own fields, guarded against null and misalignment.
    let Some((buttons, style)) = (unsafe { read_record(holder) }) else { return };
    POLLS.fetch_add(1, Ordering::Relaxed);
    LAST_BUTTONS.store(buttons, Ordering::Relaxed);
    // The INI shortcuts are detected by the chord engine from the raw controller (before it clears the Plus bits
    // of an accepted Plus shortcut, which the adapter would no longer see); one request per poll, always taken.
    let chord_open = chords::take_open_menu();
    let Some(mut adapter) = ADAPTER.try_lock() else { return };
    let facts = state::readiness_facts(POLLS.load(Ordering::Relaxed));
    if readiness::must_release(facts) {
        // The renderer is gone (guard failed, archive refused, layout lost): never trap Player 1's input.
        state::release_menu();
        adapter.resync(buttons, style);
        return;
    }
    let open = OPEN.load(Ordering::Acquire);
    // A chord (the permanent one or an INI shortcut) opens the menu only while a renderer exists (R2-2, SPEC-core-05 question 1).
    let allowed = readiness::may_open(facts);
    let shortcuts = chords::shortcuts();
    let step = adapter.poll(buttons, style, open, allowed, shortcuts);
    if step.open || (chord_open && allowed && !open) {
        adapter.resync(buttons, style);
        INPUT_OWNERS.fetch_or(Owners::MENU.bits(), Ordering::AcqRel);
        OPEN.store(true, Ordering::Release);
        worker::push_open();
        worker::wake();
        return;
    }
    let mut any = false;
    for button in step.presses() {
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic_now(format_args!("[Ui] probe input push {button:?}"));
        worker::push_press(button);
        any = true;
    }
    if any {
        worker::wake();
    }
}