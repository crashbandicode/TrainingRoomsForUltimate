//! The scene predicate every feature calls (SPEC-core-01 section 5, decision D4): the fork widened "Training" to
//! "Training, or lobby practice active, and no hot swap in progress" and made that one function the seam for all
//! hooks. This is its pure rule; `runtime::scene` gathers the facts.
//!
//! * `native_training`: the game's own `is_training_mode()` is true (native Training).
//! * `lobby_active`: the waiting-room practice flag, a cache the lobby clock sets. It can outlive the room until the
//!   next tick, so it is not enough on its own.
//! * `waiting_room`: the live mode words are the waiting room right now (`model::lobby_flag::is_waiting_room`).
//!   Unreadable mode words are not a waiting room. The lobby term is `lobby_active && waiting_room`.
//! * `busy`: a fighter replacement owns the scene (the hot-swap install gate suspends practice, or
//!   `published::HOT_SWAP_IN_PROGRESS`): the scene reads as "not Training" so no behaviour acts on a half-built fighter.

pub use crate::model::cpu::role::Scene;

/// What the runtime knows right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub native_training: bool,
    pub lobby_active: bool,
    pub waiting_room: bool,
    pub busy: bool,
}

/// The widened predicate: while busy it is false; otherwise native Training, or lobby practice whose live scene is
/// still the waiting room. A cached flag from an earlier room does not answer for a battle.
pub const fn is_training(facts: Facts) -> bool {
    !facts.busy && (facts.native_training || (facts.lobby_active && facts.waiting_room))
}

/// A combo's name row may open a keyboard. Training uses ShowKeyboard. A local-wireless
/// waiting room uses the inline Calc. An online waiting mode (`mode & 0x7fff0000 == 0x07010000`,
/// the online half of the waiting scene) is not shown safe, so the row opens neither.
pub const fn combo_name_keyboard(waiting: bool, mode: u32) -> bool {
    !(waiting && (mode & 0x7fff_0000) == 0x0701_0000)
}

/// Where the practice behaviours are live (`None` while busy or outside practice). Native Training wins over a stale lobby flag.
pub const fn scene(facts: Facts) -> Option<Scene> {
    if facts.busy {
        None
    } else if facts.native_training {
        Some(Scene::Training)
    } else if facts.lobby_active && facts.waiting_room {
        Some(Scene::Lobby)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn f(native_training: bool, lobby_active: bool, busy: bool) -> Facts {
        // The older cases mean a live lobby: the flag and the room agree.
        Facts { native_training, lobby_active, waiting_room: lobby_active, busy }
    }

    #[test]
    fn busy_always_reads_as_not_training() {
        for native in [false, true] {
            for lobby in [false, true] {
                assert!(!is_training(f(native, lobby, true)));
                assert_eq!(scene(f(native, lobby, true)), None);
            }
        }
    }

    #[test]
    fn native_training_or_active_lobby_is_training_when_not_busy() {
        assert!(is_training(f(true, false, false)));
        assert!(is_training(f(false, true, false)));
        assert!(is_training(f(true, true, false)));
        assert!(!is_training(f(false, false, false)));
    }

    #[test]
    fn the_scene_follows_the_facts_and_native_wins() {
        assert_eq!(scene(f(true, false, false)), Some(Scene::Training));
        assert_eq!(scene(f(false, true, false)), Some(Scene::Lobby));
        assert_eq!(scene(f(true, true, false)), Some(Scene::Training));
        assert_eq!(scene(f(false, false, false)), None);
    }

    #[test]
    fn the_scene_and_the_predicate_agree() {
        for bits in 0..16u8 {
            let facts = Facts {
                native_training: bits & 1 != 0,
                lobby_active: bits & 2 != 0,
                waiting_room: bits & 4 != 0,
                busy: bits & 8 != 0,
            };
            assert_eq!(is_training(facts), scene(facts).is_some(), "{facts:?}");
        }
    }

    #[test]
    fn a_stale_lobby_flag_outside_the_waiting_room_is_not_training() {
        let stale = Facts { native_training: false, lobby_active: true, waiting_room: false, busy: false };
        assert!(!is_training(stale));
        assert_eq!(scene(stale), None);
        let native = Facts { native_training: true, ..stale };
        assert!(is_training(native));
        assert_eq!(scene(native), Some(Scene::Training));
    }
}
