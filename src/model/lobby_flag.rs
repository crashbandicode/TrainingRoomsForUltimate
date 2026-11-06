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

#[cfg(test)]
mod tests {
    use super::{step, FlagClock, Scene, Step};
    use crate::engine::macro_ui::clip_panel::CloseInputGuard;
    use crate::engine::stages::training_stage_rotation_policy::{TRAINING_MELEE, TRAINING_MODE};
    use crate::model::developer_reload::qualifies;
    use crate::model::input_delay::applies;
    use crate::model::lra_reset::x8_write;
    use crate::model::scene::{self, Facts};
    use crate::model::stage_hazards::rule_byte;
    use crate::model::state::hooks::{self, DeathParam};
    use crate::model::state::pokemon::{self, SwitchEffect};
    use crate::model::state::settings::StateSettings;

    const WAITING: Scene = Scene {
        initialized: true,
        melee: 0x0d,
        mode: 0x0701_0100,
    };
    const LOCAL_WAITING: Scene = Scene {
        initialized: true,
        melee: 0x0d,
        mode: 0x0201_0000,
    };
    const BATTLE: Scene = Scene {
        initialized: true,
        melee: 0x0e,
        mode: 0x0701_0101,
    };
    /// The mapped slot is not an input of `step`. Each label calls the same function.
    const STATIONS: &[&str] = &[
        "slot 0",
        "slot 1",
        "slot 2",
        "slot 3",
        "no input-mapping call",
    ];
    const DEATH: [DeathParam; 5] = [
        DeathParam::DeadRebirthWaitFrame,
        DeathParam::RebirthMoveFrame,
        DeathParam::RebirthWaitFrame,
        DeathParam::RebirthInvincibleFrame,
        DeathParam::RebirthInvincibleAddFrame,
    ];

    fn live(native: bool, active: bool) -> bool {
        // After a tick the flag is on only in a live waiting room, so the room fact matches it.
        scene::is_training(Facts {
            native_training: native,
            lobby_active: active,
            waiting_room: true,
            busy: false,
        })
    }

    /// The predicate as a query sees it: the cached flag, the live scene, no clock tick.
    fn asked(flag: bool, scene: Option<Scene>, native: bool) -> bool {
        scene::is_training(Facts {
            native_training: native,
            lobby_active: flag,
            waiting_room: scene.is_some_and(super::is_waiting_room),
            busy: false,
        })
    }

    fn native_of(scene: Scene) -> bool {
        scene.initialized && scene.melee == TRAINING_MELEE && scene.mode == TRAINING_MODE
    }

    /// Menu On, then the UI becomes ready, then one more tick of the same scene. That is a live
    /// waiting room. The first tick is not live: the layout is not ready yet.
    fn ready(scene: Scene) -> FlagClock {
        let (mut clock, first) = step(FlagClock::new(), Some(scene), true);
        assert!(
            !first.active,
            "a waiting room is not practice until its UI is ready"
        );
        clock.ui_ready = true;
        let (clock, decision) = step(clock, Some(scene), true);
        assert_eq!(
            decision,
            Step {
                scene_changed: false,
                suspend_waiting: false,
                active: true,
                active_changed: true
            }
        );
        assert!(live(false, true));
        clock
    }

    fn battle_from(station: &str, clock: FlagClock) -> Step {
        let (_, decision) = step(clock, Some(BATTLE), true);
        assert_eq!(
            decision,
            Step {
                scene_changed: true,
                suspend_waiting: true,
                active: false,
                active_changed: true
            },
            "{station}: leaving a live waiting room"
        );
        assert!(
            !live(false, decision.active),
            "{station}: the battle's first frame is not Training"
        );
        decision
    }

    #[test]
    fn training_behaviour_follows_the_live_scene_at_every_station() {
        let modes = [
            0x0201_0000,
            0x0701_0100,
            0x0701_0101,
            0x0404_0000,
            0,
            0x0601_0000,
            0x8701_0100,
        ];
        for station in STATIONS {
            let clock = ready(WAITING);
            assert!(clock.was_active, "{station}: the waiting room's flag is on");
            assert!(live(false, true), "{station}: the live waiting room stays practice");
            battle_from(station, clock);
            let training = Scene {
                initialized: true,
                melee: TRAINING_MELEE,
                mode: TRAINING_MODE,
            };
            let (clock, entered) = step(clock, Some(training), true);
            assert!(!entered.active, "{station}");
            assert!(
                live(true, entered.active),
                "{station}: native Training stays Training"
            );
            let (_, left) = step(clock, Some(BATTLE), true);
            assert!(left.scene_changed, "{station}");
            assert!(
                !live(false, left.active),
                "{station}: Training then a battle is not Training"
            );
            let clock = ready(WAITING);
            let (_, off) = step(clock, Some(WAITING), false);
            assert!(
                !off.active && !live(false, off.active),
                "{station}: the menu off leaves the room"
            );
            assert!(!scene::is_training(Facts {
                native_training: true,
                lobby_active: true,
                waiting_room: true,
                busy: true
            }));
            for melee in 0u8..=0x21 {
                for mode in modes {
                    for initialized in [false, true] {
                        let dest = Scene {
                            initialized,
                            melee,
                            mode,
                        };
                        let (next, decision) = step(clock, Some(dest), true);
                        let native = native_of(dest);
                        let on = live(native, decision.active);
                        if super::scene_key(dest) == super::scene_key(WAITING) {
                            assert!(
                                decision.active && on && !decision.scene_changed,
                                "{station}: {dest:?}"
                            );
                        } else if native {
                            assert!(
                                on && !decision.active && decision.suspend_waiting,
                                "{station}: {dest:?}"
                            );
                        } else {
                            assert!(
                                !on && !decision.active && decision.suspend_waiting,
                                "{station}: {dest:?}"
                            );
                            if super::is_waiting_room(dest) {
                                let mut again = next;
                                again.ui_ready = true;
                                let (_, resumed) = step(again, Some(dest), true);
                                assert!(
                                    resumed.active && live(false, true),
                                    "{station}: {dest:?} resumes"
                                );
                            }
                        }
                    }
                }
            }
            let local = ready(LOCAL_WAITING);
            battle_from(station, local);
        }
    }

    #[test]
    fn slot_one_still_tears_the_waiting_room_down() {
        let clock = ready(WAITING);
        let decision = battle_from("slot 1", clock);
        assert!(decision.scene_changed && decision.suspend_waiting);
        let again = step(clock, Some(BATTLE), true);
        let (_, second) = step(again.0, Some(BATTLE), true);
        assert!(
            !second.scene_changed && !second.suspend_waiting,
            "the teardown runs once per change"
        );
    }

    #[test]
    fn a_battle_at_station_1_keeps_the_games_death_parameters() {
        let settings = StateSettings::default();
        assert!(settings.enable && !settings.autoload && !settings.respawn_invincibility);
        let clock = ready(WAITING);
        for param in DEATH {
            let stuck =
                hooks::param_override(param, &settings, false, live(false, clock.was_active));
            assert!(
                stuck.is_some(),
                "the live waiting room substitutes {param:?} = {stuck:?}"
            );
        }
        assert_eq!(
            hooks::param_override(DeathParam::DeadRebirthWaitFrame, &settings, false, true),
            Some(1)
        );
        for param in DEATH.into_iter().skip(1) {
            assert_eq!(
                hooks::param_override(param, &settings, false, true),
                Some(0)
            );
        }
        let decision = battle_from("slot 1", clock);
        let training = live(false, decision.active);
        assert!(!training);
        for param in DEATH {
            assert_eq!(
                hooks::param_override(param, &settings, false, training),
                None,
                "{param:?} stays the game's own value"
            );
        }
    }

    /// The scene is already the battle and the clock has not run. The cached flag is still true.
    /// Queries, the five death parameters and every widened gate answer like vanilla. Unreadable
    /// mode words are not a waiting room, so the lobby term is false then too.
    #[test]
    fn a_battle_with_no_clock_tick_is_not_training() {
        let settings = StateSettings::default();
        assert!(asked(true, Some(WAITING), false), "the same flag in the waiting room is practice");
        assert!(!asked(true, Some(BATTLE), false), "no tick, and the battle is not Training");
        assert!(!asked(true, None, false), "unreadable mode words are not a waiting room");
        assert!(asked(true, Some(BATTLE), true), "native Training does not depend on the lobby flag");
        let training = asked(true, Some(BATTLE), false);
        for param in DEATH {
            assert_eq!(
                hooks::param_override(param, &settings, false, training),
                None,
                "{param:?} stays the game's own value before the first tick"
            );
        }
        let found = runtime_gates();
        assert!(found.len() > 20);
        for (file, line, text, site) in &found {
            assert!(
                !replaces(site, training) && !replaces(site, asked(true, None, false)),
                "{file}:{line} {site} answers Training before the first tick: {text}"
            );
        }
    }

    #[test]
    fn a_waiting_room_after_a_station_1_battle_practices_again() {
        let clock = ready(WAITING);
        let (clock, _) = step(clock, Some(BATTLE), true);
        assert!(!clock.was_active);
        let (mut clock, arrived) = step(clock, Some(WAITING), true);
        assert!(
            arrived.scene_changed && !arrived.active,
            "the room is back, the layout is not ready"
        );
        assert!(!live(false, arrived.active));
        clock.ui_ready = true;
        let (_, practicing) = step(clock, Some(WAITING), true);
        assert!(practicing.active && !practicing.scene_changed);
        assert!(live(false, practicing.active));
    }

    #[test]
    fn every_is_training_answer_is_vanilla_outside_training() {
        let found = runtime_gates();
        assert!(found.len() > 20, "the scan missed the runtime gates");
        let mut hooks = 0;
        for (file, line, text, site) in &found {
            match *site {
                "comment" | "definition" | "forwarder" | "native_predicate" => {
                    assert!(!replaces(site, false), "{file}:{line} {text}");
                    assert!(
                        !replaces(site, true),
                        "{file}:{line} is not a lobby-flag answer"
                    );
                }
                _ => {
                    hooks += 1;
                    assert!(
                        !replaces(site, false),
                        "{file}:{line} {site} replaces the game outside Training: {text}"
                    );
                    assert!(
                        replaces(site, true),
                        "{file}:{line} {site} is not actually gated: {text}"
                    );
                }
            }
        }
        assert!(hooks > 15, "the widened gates were classified as comments");
    }

    fn replaces(site: &str, training: bool) -> bool {
        let settings = StateSettings::default();
        match site {
            "comment" | "definition" | "forwarder" | "native_predicate" => false,
            "death_params" => DEATH
                .iter()
                .any(|param| hooks::param_override(*param, &settings, false, training).is_some()),
            "quake" => training && hooks::quake_suppressed(true),
            "sound" => training && hooks::sound_silenced(true),
            "effect" => training && hooks::effect_hidden(true, false),
            "pokemon_switch" => {
                training && pokemon::switch_effect_hidden(SwitchEffect::FlyingPlate, true)
            }
            "score_aura" => training && hooks::score_aura_hidden(true),
            "hazards" => rule_byte(true, training).is_some(),
            "input_delay" => applies(0, training),
            "lra_reset" => x8_write(false, training).is_some(),
            "developer_reload" => qualifies(0, training, u32::MAX),
            "close_input" => {
                let mut guard = CloseInputGuard::new();
                guard.arm(4);
                guard.poll(training, true)
            }
            "forced_off" | "joker" | "mac_meter" | "wiifit" | "wing" | "fighter_se"
            | "pokemon_decide" | "kirby_copy" | "save_state_frame" | "p1_chords"
            | "operator_input" | "developer_float" | "hitbox_draw" | "hud_draw"
            | "character_items" | "kirby_hats" => training,
            other => panic!("no vanilla answer for {other}"),
        }
    }

    fn runtime_gates() -> Vec<(String, usize, String, &'static str)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        files.sort();
        let mut found = Vec::new();
        for path in files {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            for (index, line) in text.lines().enumerate() {
                if !line.contains("is_training(") && !line.contains("is_training_mode(") {
                    continue;
                }
                let site = class(path.to_str().unwrap_or(""), line).unwrap_or_else(|| {
                    panic!("unclassified {}:{}: {line}", path.display(), index + 1)
                });
                found.push((
                    path.display().to_string(),
                    index + 1,
                    line.trim().to_string(),
                    site,
                ));
            }
        }
        found
    }

    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()))
        {
            let path = entry.unwrap_or_else(|err| panic!("{err}")).path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    fn class(file: &str, line: &str) -> Option<&'static str> {
        let text = line.trim();
        if text.starts_with("//") || text.starts_with('*') {
            return Some("comment");
        }
        if line.contains("game::is_training_mode(")
            || (file.ends_with("game.rs") && text.contains("fn is_training_mode("))
        {
            return Some("native_predicate");
        }
        if text.contains("fn is_training(") || text.contains("is_training(facts())") {
            return Some("definition");
        }
        if text.contains("fn is_training_mode(") || text == "scene::is_training()" {
            return Some("forwarder");
        }
        if file.ends_with("character_items.rs") {
            return Some("character_items");
        }
        if file.ends_with("kirby_hats.rs") {
            return Some("kirby_hats");
        }
        if text.contains("param_override") {
            return Some("death_params");
        }
        if text.contains("switch_effect_hidden") {
            return Some("pokemon_switch");
        }
        if text.contains("effect_hidden") {
            return Some("effect");
        }
        if text.contains("quake_suppressed") {
            return Some("quake");
        }
        if text.contains("fighter_sound_silenced") {
            return Some("fighter_se");
        }
        if text.contains("sound_silenced") {
            return Some("sound");
        }
        if text.contains("score_aura_hidden") {
            return Some("score_aura");
        }
        if text.contains("MAC_PRESENT") {
            return Some("mac_meter");
        }
        if text.contains("wii_buffing") {
            return Some("wiifit");
        }
        if text.contains("WING_BACKGROUND") {
            return Some("wing");
        }
        if text.contains("summon") {
            return Some("joker");
        }
        if text.contains("!LOADING.load") && text.contains("!scene::is_training()") {
            return Some("pokemon_decide");
        }
        if text.contains("scene::is_training() &&") {
            return Some("kirby_copy");
        }
        if text.contains("ENABLE.load") && text.contains("TEMPORARY.load") {
            return Some("forced_off");
        }
        if text.contains("model::scene::is_training") {
            return Some("save_state_frame");
        }
        if text.contains("owns_close_input") {
            return Some("close_input");
        }
        if text.contains("rule_byte") {
            return Some("hazards");
        }
        if text.contains("controller != 0") {
            return Some("p1_chords");
        }
        if text.contains("slot == 1") {
            return Some("operator_input");
        }
        if text.contains("input_delay::applies") {
            return Some("input_delay");
        }
        if text.contains("x8_write") {
            return Some("lra_reset");
        }
        if text.contains("is_fighter") {
            return Some("developer_float");
        }
        if text.contains("developer_reload::qualifies") {
            return Some("developer_reload");
        }
        if text.contains("info_training") || text.contains("INFO_TRAINING") {
            return Some(if file.contains("hitbox") {
                "hitbox_draw"
            } else {
                "hud_draw"
            });
        }
        None
    }
}
