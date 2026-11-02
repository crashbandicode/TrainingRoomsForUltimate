//! The scene predicate every feature calls (SPEC-core-01 section 5, decision D4): the fork widened "Training" to
//! "Training, or lobby practice active, and no hot swap in progress" and made that one function the seam for all
//! hooks. This is its pure rule; `runtime::scene` gathers the three facts.
//!
//! * `native_training`: the game's own `is_training_mode()` is true (native Training).
//! * `lobby_active`: lobby practice is active (the waiting room has the practice session; owned by the lobby engine).
//! * `busy`: a fighter replacement owns the scene (the hot-swap install gate suspends practice, or
//!   `published::HOT_SWAP_IN_PROGRESS`): the scene reads as "not Training" so no behaviour acts on a half-built fighter.

pub use crate::model::cpu::role::Scene;

/// What the runtime knows right now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub native_training: bool,
    pub lobby_active: bool,
    pub busy: bool,
}

/// The widened predicate: while busy it is false; otherwise native Training or an active lobby practice.
pub const fn is_training(facts: Facts) -> bool {
    !facts.busy && (facts.native_training || facts.lobby_active)
}

/// Where the practice behaviours are live (`None` while busy or outside practice). Native Training wins over a stale lobby flag.
pub const fn scene(facts: Facts) -> Option<Scene> {
    if facts.busy {
        None
    } else if facts.native_training {
        Some(Scene::Training)
    } else if facts.lobby_active {
        Some(Scene::Lobby)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn f(native_training: bool, lobby_active: bool, busy: bool) -> Facts {
        Facts { native_training, lobby_active, busy }
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
        for bits in 0..8u8 {
            let facts = f(bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
            assert_eq!(is_training(facts), scene(facts).is_some(), "{facts:?}");
        }
    }
}