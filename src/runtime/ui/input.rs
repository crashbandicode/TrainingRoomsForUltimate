//! Player 1's input for the menu (SPEC-core-03 §3): once per P1 poll of the game's final input mapping,
//! the hook hands this module the controller holder the call was given. The holder's record carries the
//! raw buttons (`current_buttons` at +0x8) and the controller style (+0x9c, the field the game's own mapping reads at `main+0x1750e0c`: `ldr w2, [x20, #0x9c]` after `ldr x20, [x3, #0x10]`; the same `Controller` package 4's chords read, `chords::field`). The adapter (`model::ui::
//! adapter`) turns them into the open chord and logical presses, which go to the menu worker through a
//! lock-free ring; while the menu is open or closing the `Owners::MENU` bit empties P1's mapped output
//! (the delay stage of `features::input_delay` already neutralises a poll whose owners are set).
//!
//! The hook allocates nothing and takes no blocking lock.

use std::sync::atomic::{AtomicU32, Ordering};

use hos_lock::Mutex;

use crate::model::input_delay::Owners;
use crate::model::adapter::{Adapter, Style};
use crate::runtime::features::chords;
use crate::runtime::published::INPUT_OWNERS;
use crate::model::readiness;
use crate::model::ui_events::Packet;
use crate::runtime::ui::state::{self, OPEN};
use crate::runtime::ui::worker;

static ADAPTER: Mutex<(Adapter, u32)> = Mutex::new((Adapter::new(), 0));
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
/// progress keeps its lock; the ring's generation invalidation resets it on the next poll too.
pub fn reset_session() {
    if let Some(mut adapter) = ADAPTER.try_lock() {
        adapter.0 = Adapter::new();
    }
}

/// One P1 poll in Training.
pub fn on_p1_poll(holder: usize) {
    // Attribution (lead, 2026-11-02): the menu's own allocations inside the P1 hook are charged here.
    let _scope = crate::runtime::alloc::HookScope::enter_hook(crate::runtime::alloc::HookId::MenuInput);
    // SAFETY: `holder` is the controller-holder argument of the game's call; the reads are the record's
    // own fields, guarded against null and misalignment.
    let Some((buttons, style)) = (unsafe { read_record(holder) }) else { return };
    POLLS.fetch_add(1, Ordering::Relaxed);
    LAST_BUTTONS.store(buttons, Ordering::Relaxed);
    // Consume any pending request from the chord engine. The adapter also sees
    // this poll's raw permanent/INI chords before modal Plus suppression below.
    let chord_open = chords::take_open_menu();
    let Some(mut state) = ADAPTER.try_lock() else { return };
    let generation = worker::input_generation();
    if state.1 != generation {
        *state = (Adapter::new(), generation);
        // A held confirmation/opening edge from the old session is not new
        // input. Observe its release before accepting another physical edge.
        state.0.resync(buttons, style);
    }
    let adapter = &mut state.0;
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
    // ZL + Up / Down are jumps only where the screen showing takes them (the worker publishes it, review MN-1).
    adapter.set_jumps(state::JUMPS.load(Ordering::Acquire));
    let step = adapter.poll(buttons, style, open, allowed, shortcuts);
    if step.open || (chord_open && allowed && !open) {
        adapter.resync(buttons, style);
        INPUT_OWNERS.fetch_or(Owners::MENU.bits(), Ordering::AcqRel);
        OPEN.store(true, Ordering::Release);
        if !worker::push_open(generation) {
            state::release_menu();
        }
        worker::wake();
        return;
    }
    if open && (step.changed || step.presses().next().is_some()) {
        if !worker::push_poll(generation, Packet::from_step(&step)) {
            // A missing held/release edge cannot confirm a partial chord.
            state::release_menu();
        }
        worker::wake();
    }
}
