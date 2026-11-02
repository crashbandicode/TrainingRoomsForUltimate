//! The settings every hook reads, as atomics: replaced as a whole at load and at every commit
//! (SPEC-core-04 §5, SPEC-core-01 question 10), read without a lock.
//!
//! NOT YET SWITCH-COMPILED.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

use crate::model::input_delay::Owners;

use crate::model::settings::Snapshot;

static INPUT_DELAY: AtomicU8 = AtomicU8::new(0);
static LRA_RESET: AtomicBool = AtomicBool::new(true);
static STAGE_HAZARDS: AtomicBool = AtomicBool::new(false);
static DEVELOPER_PAGE: AtomicBool = AtomicBool::new(false);

/// Make `snapshot` the live settings. `rerolls` is the menu outbox's re-roll bits (decisions Q18 A,
/// B11); the first publish at boot has none.
pub fn publish(snapshot: &Snapshot, rerolls: u8) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_reload::remember(snapshot);
    crate::runtime::features::cpu::publish(snapshot, rerolls);
    INPUT_DELAY.store(snapshot.input_delay(), Ordering::Release);
    LRA_RESET.store(snapshot.lra_reset(), Ordering::Release);
    STAGE_HAZARDS.store(snapshot.stage_hazards(), Ordering::Release);
}

pub fn input_delay() -> u8 {
    INPUT_DELAY.load(Ordering::Acquire)
}

pub fn lra_reset() -> bool {
    LRA_RESET.load(Ordering::Acquire)
}

pub fn stage_hazards() -> bool {
    STAGE_HAZARDS.load(Ordering::Acquire)
}

/// Decision N1: whether `developer.ini` said `developer_page=on` at boot (the menu reads it in a
/// later package).
pub fn set_developer_page(on: bool) {
    DEVELOPER_PAGE.store(on, Ordering::Release);
}

/// What other than gameplay owns P1's input right now (`Owners` bits). The packages that own those
/// states set and clear their bit: the plugin menu and modal panels (package 5), the named stop chord
/// and the replay editor (package 4), the native waiting-room UI (package 6), the game's pause menu.
/// The input-delay hook reads it on every poll.
pub static INPUT_OWNERS: AtomicU8 = AtomicU8::new(0);

pub fn input_owners() -> Owners {
    Owners::from_bits(INPUT_OWNERS.load(Ordering::Acquire))
}

/// Set while the hot-swap engine replaces a fighter (package 6); the reset hub does not run then.
pub static HOT_SWAP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
/// The input-recording package's inbox for what the CPU behaviours ask of it. A playback request is
/// `1 + slot` in the low byte (0 = none) and bit 8 set for a ledge playback (`PlaybackStart::request_word`).
/// The consumer takes it with `PlaybackStart::from_request_word(CPU_PLAYBACK_REQUEST.swap(0, Ordering::AcqRel))`.
pub static CPU_PLAYBACK_REQUEST: AtomicU32 = AtomicU32::new(0);
/// Playback Mash Interrupt: stop the running playback. The consumer swaps it to false.
pub static CPU_STOP_PLAYBACK: AtomicBool = AtomicBool::new(false);

/// The frame-advantage notification's inbox for the UI package: the last measured value and a sequence
/// number that changes with every new measurement (the consumer shows `+N`, `-N` or `+0` for 60 frames).
pub static FRAME_ADVANTAGE_VALUE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
pub static FRAME_ADVANTAGE_SEQ: AtomicU32 = AtomicU32::new(0);
