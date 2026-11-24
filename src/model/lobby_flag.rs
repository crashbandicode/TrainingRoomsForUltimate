//! The waiting-room practice clock. One step reads the scene the game is in now and says whether
//! waiting-room practice is live. It does not read which controller slot this console maps: an
//! online battle at station 1 never delivers slot 0, and the flag used to stay on for that reason.
//!
//! `runtime::lobby::training::lobby::update` is this step plus the plugin bookkeeping it names
//! (suspend, scene departure, the CPU role scene). The host tests and the Switch path call the
//! same function.

/// One observation of the game's mode words (`runtime::lobby::game_mode::Mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    pub initialized: bool,
    pub melee: u8,
    pub mode: u32,
}

/// What the last step left behind. `last_key` starts at `u64::MAX`, matching `LAST_MODE`, so the
/// first real scene is a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlagClock {
    pub last_key: u64,
    pub ui_ready: bool,
    pub was_active: bool,
}

impl FlagClock {
    pub const fn new() -> Self {
        Self {
            last_key: u64::MAX,
            ui_ready: false,
            was_active: false,
        }
    }
}

impl Default for FlagClock {
    fn default() -> Self {
        Self::new()
    }
}

/// The step's answer. `scene_changed` is the cleanup (roster, practice lease, UI). `suspend_waiting`
/// is that cleanup when the room being left was actually practicing. `active` is the practice flag
/// for this scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub scene_changed: bool,
    pub suspend_waiting: bool,
    pub active: bool,
    pub active_changed: bool,
}

/// The scene key `lobby::update` already stored: mode, then melee, then the initialized bit.
pub const fn scene_key(scene: Scene) -> u64 {
    (scene.mode as u64) | ((scene.melee as u64) << 32) | ((scene.initialized as u64) << 40)
}

/// The waiting room, as `lobby::is_waiting_scene` has always decided it. Melee `0x0d`, the local
/// family `0x02010000` exact, or the online family `mode & 0x7fff0000 == 0x07010000`.
pub const fn is_waiting_room(scene: Scene) -> bool {
    scene.initialized
        && scene.melee == 0x0d
        && (scene.mode == 0x0201_0000 || scene.mode & 0x7fff_0000 == 0x0701_0000)
}

/// Lobby Practice was turned Off in a live room while a drill still holds the first partner (its behaviour, its
/// fighter, or its sleep): practice stays requested until the drill has given them back. There is no deadline: the
/// obligation ends with the give-back, with a departure (`step`'s own rule, whatever this says) or with the partner's
/// replacement (the lease drops then, so `holds` is false). Returns the `lobby_practice_on` that `step` takes,
/// whether a release is under way, and whether the drill is to be stopped now (once, when the release begins).
pub const fn release(setting_on: bool, was_active: bool, holds: bool, releasing: bool) -> (bool, bool, bool) {
    if setting_on || !was_active || !holds {
        return (setting_on, false, false);
    }
    (true, true, !releasing)
}

/// The clock `step` takes: while a release is under way the room counts as ready. The draw pass drops `ui_ready` as
/// soon as it reads the setting Off, which can be before `lobby::update` has armed the release, and the give-back
/// needs no layout. A changed scene still clears the readiness inside `step`, so a departure ends practice as before.
pub const fn released_clock(clock: FlagClock, releasing: bool) -> FlagClock {
    FlagClock { ui_ready: clock.ui_ready || releasing, ..clock }
}

/// Advance the clock. `scene == None` is "the mode words are not readable": the key stays, practice
/// is not requested, and a room that was active turns off. The same scene twice does not tear down
/// again. The mapped slot is not an argument.
pub fn step(prev: FlagClock, scene: Option<Scene>, lobby_practice_on: bool) -> (FlagClock, Step) {
    let mut next = prev;
    let mut scene_changed = false;
    if let Some(scene) = scene {
        let key = scene_key(scene);
        if key != prev.last_key {
            scene_changed = true;
            next.last_key = key;
            next.ui_ready = false;
        }
    }
    let requested = lobby_practice_on && scene.is_some_and(is_waiting_room);
    if !requested {
        next.ui_ready = false;
    }
    let active = requested && next.ui_ready;
    let decision = Step {
        scene_changed,
        suspend_waiting: scene_changed && prev.was_active,
        active,
        active_changed: active != prev.was_active,
    };
    next.was_active = active;
    (next, decision)
}
