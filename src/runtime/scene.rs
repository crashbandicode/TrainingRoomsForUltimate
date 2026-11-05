//! The scene wrapper (decision D4): the widened `is_training_mode()` every package calls instead of the raw game predicate.
//! Pure rule in `model::scene`; the three facts are read here. Owned by package 6 (the lobby engine sets `lobby_active`, the
//! hot-swap engine sets `HOT_SWAP_IN_PROGRESS` and holds the install gate).
//!
//! NOT YET SWITCH-COMPILED beyond the build recorded in IMPLEMENTATION-PROGRESS-pkg6.md (removed after v0.2.0-rc.1; see that tag).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::scene::{self, Facts, Scene};
use crate::runtime::{game, published};

static LOBBY_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The lobby engine's activation (lobby practice owns the waiting-room session). Package 2's CPU role scene follows it
/// through `cpu::set_lobby`, which the lobby engine also calls.
pub fn set_lobby_active(on: bool) {
    LOBBY_ACTIVE.store(on, Ordering::Release);
}

pub fn lobby_active() -> bool {
    LOBBY_ACTIVE.load(Ordering::Acquire)
}

/// What the runtime knows now (the game's native predicate is read through its guarded Called site).
pub fn facts() -> Facts {
    Facts {
        native_training: game::is_training_mode(),
        lobby_active: lobby_active(),
        busy: published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire),
    }
}

/// The widened predicate: false while a hot swap runs, otherwise native Training or an active lobby practice.
pub fn is_training() -> bool {
    scene::is_training(facts())
}

/// Where the practice behaviours are live right now.
pub fn current() -> Option<Scene> {
    scene::scene(facts())
}

/// The game's own predicate alone (native Training, not widened): for the few rules the specs bind to native Training only
/// (the reset check, hazards).
pub fn is_native_training() -> bool {
    game::is_training_mode()
}