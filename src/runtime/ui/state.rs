//! What the menu's three actors share: the input hook (game thread), the menu worker and the layout tick
//! (UI job thread). Atomics and one mutex-guarded frame; nothing here allocates.

use std::sync::atomic::{AtomicBool, AtomicU32};

use parking_lot::Mutex;

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
/// The standalone CPU pilot plugin is on the card (Q11): found by its exported state symbol at boot.
pub static PILOT_PRESENT: AtomicBool = AtomicBool::new(false);
/// The game's own pause menu is open (`L_staying_help` left its rest position), SPEC-core-03 §2.4.
pub static PAUSE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The frame the worker last built, and its sequence number (the tick draws when it changed).
pub static FRAME: Mutex<Frame> = Mutex::new(Frame::new());
pub static FRAME_SEQ: AtomicU32 = AtomicU32::new(0);

/// Frames of discarded input and fade after a close.
pub const CLOSE_FRAMES: u32 = 15;