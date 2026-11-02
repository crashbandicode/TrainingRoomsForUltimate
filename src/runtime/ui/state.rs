//! What the menu's three actors share: the input hook (game thread), the menu worker and the layout tick
//! (UI job thread). Atomics and one mutex-guarded frame; nothing here allocates.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};


use crate::model::frame::Frame;

/// The menu owns Player 1's input and is showing (set by the input hook when a chord passes, cleared by
/// the worker when the menu closes).
pub static OPEN: AtomicBool = AtomicBool::new(false);
/// The menu closed and the 15 real frames of discarded input and fade are running (SPEC-core-03 §2.3).
pub static CLOSING: AtomicBool = AtomicBool::new(false);
/// Real frames since the close, counted by the layout tick.
pub static CLOSE_COUNT: AtomicU32 = AtomicU32::new(0);
/// The menu is on screen this frame (the HUD layouts hide while it is, SPEC-core-05 §2).
pub static MENU_SHOWN: AtomicBool = AtomicBool::new(false);
/// A panel the menu does not own is on screen with the menu closed (the replay window's clip editor): the tick draws the frame.
pub static EXT_SHOWN: AtomicBool = AtomicBool::new(false);
/// The standalone CPU pilot plugin is on the card (Q11): found by its exported state symbol at boot.
pub static PILOT_PRESENT: AtomicBool = AtomicBool::new(false);
/// The game's own pause menu is open (`L_staying_help` left its rest position), SPEC-core-03 §2.4.
pub static PAUSE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The frame the worker last built, and its sequence number (the tick draws when it changed). A wait-free triple buffer (stream/sync):
/// the worker is its one writer and the UI tick its one reader, so neither waits on the other.
pub static FRAME: hos_lock::TripleBuffer<Frame> = hos_lock::TripleBuffer::new(Frame::new());
pub static FRAME_SEQ: AtomicU32 = AtomicU32::new(0);

/// The UI anchors and the menu worker are installed (the guards passed): set once at boot (review R2-2).
pub static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The tick resolved the menu's panes in the Training HUD layout now registered (the injected archive was accepted).
pub static LAYOUT_READY: AtomicBool = AtomicBool::new(false);
/// `input::POLLS` at the layout tick's last run: the input hook compares it to tell that the layout is gone.
pub static POLLS_AT_TICK: AtomicU32 = AtomicU32::new(0);
/// How many times the menu's ownership was released because its renderer was lost.
pub static RELEASES: AtomicU32 = AtomicU32::new(0);

/// The facts `model::readiness` decides on.
pub fn readiness_facts(polls: u32) -> crate::model::readiness::Facts {
    use crate::model::input_delay::Owners;
    let closing = CLOSING.load(Ordering::Acquire);
    let owner = crate::runtime::published::INPUT_OWNERS.load(Ordering::Acquire) & Owners::MENU.bits() != 0;
    crate::model::readiness::Facts {
        installed: INSTALLED.load(Ordering::Acquire),
        layout_ready: LAYOUT_READY.load(Ordering::Acquire),
        pause_active: PAUSE_ACTIVE.load(Ordering::Acquire),
        closing,
        owns_input: owner || closing || OPEN.load(Ordering::Acquire),
        polls_since_tick: polls.wrapping_sub(POLLS_AT_TICK.load(Ordering::Acquire)),
    }
}

/// The menu lost its renderer: give Player 1 back at once. Nothing is committed (as when every controller is
/// unplugged, SPEC-core-03 §2.3); the worker's model is rebuilt from the committed settings at the next opening.
pub fn release_menu() {
    use crate::model::input_delay::Owners;
    if OPEN.swap(false, Ordering::AcqRel) {
        crate::runtime::ui::menu_app::note_open();
    }
    CLOSING.store(false, Ordering::Release);
    CLOSE_COUNT.store(0, Ordering::Release);
    MENU_SHOWN.store(false, Ordering::Release);
    crate::runtime::published::INPUT_OWNERS.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
    RELEASES.fetch_add(1, Ordering::Relaxed);
}