//! The scene wrapper (decision D4): the widened `is_training_mode()` every package calls instead of the raw game predicate.
//! Pure rule in `model::scene`; the facts are read here. Owned by package 6 (the lobby engine sets `lobby_active`, the
//! hot-swap engine sets `HOT_SWAP_IN_PROGRESS` and holds the install gate).
//!
//! Part of the Switch build.

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

/// The live mode words are the waiting room. Unreadable words are not: a stale practice flag then cannot answer.
pub fn waiting_room_now() -> bool {
    crate::runtime::lobby::game_mode::current().is_some_and(|mode| {
        crate::model::lobby_flag::is_waiting_room(crate::model::lobby_flag::Scene {
            initialized: mode.initialized,
            melee: mode.melee,
            mode: mode.mode,
        })
    })
}

/// The lobby term of the widened predicate: the cached flag and the live waiting room, both.
pub fn lobby_term() -> bool {
    lobby_active() && waiting_room_now()
}

/// What the runtime knows now (the game's native predicate is read through its guarded Called site).
pub fn facts() -> Facts {
    Facts {
        native_training: game::is_training_mode(),
        lobby_active: lobby_active(),
        waiting_room: waiting_room_now(),
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