//! The dependency boundary of the named-macro layer: what the carried files call that another stream owns and that has not merged.
//! Each item says what it waits for; `install_report` is what the install line prints about it. A boundary, not a workaround: it
//! closes when the owner merges and the provider replaces the body (docs/implementation-packages.md package 6A and 5).

/// The fork's `training_mod_sync` helpers the carried layer uses (the lock crate is not kept, decision B9).
pub(crate) mod sync {
    use std::sync::atomic::{AtomicBool, Ordering};

    pub(crate) use crate::runtime::compat::sync::read;
    pub(crate) use hos_lock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    pub(crate) fn lock_read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
        lock.read()
    }

    pub(crate) fn lock_write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
        lock.write()
    }

    pub(crate) fn assign(flag: &AtomicBool, value: bool) {
        flag.store(value, Ordering::Release);
    }
}

/// The fork's `common::consts` names the carried layer reads.
pub(crate) mod consts {
    pub(crate) use crate::engine::compat::menu::{MacroSource, OnOff};
    pub(crate) use crate::runtime::compat::consts::FighterId;
    pub(crate) use crate::runtime::compat::menu::MENU;
}

/// The mapped input record and the game's Controller as the carried layer names them.
pub(crate) mod input {
    pub(crate) use crate::model::input_delay::MappedInputs;
    use crate::model::chords::{field, STYLE_GAMECUBE};

    /// The raw button set of the game's `Controller`, with the d-pad and both sticks' digital direction bits (SPEC-core-03 section 4).
    pub(crate) use crate::model::chords::Buttons;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum ControllerStyle {
        GCController,
        Other,
    }

    /// The fields the clip editor reads, copied out of the game's Controller (nothing is retained).
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct Controller {
        pub(crate) current_buttons: Buttons,
        pub(crate) just_down: Buttons,
        pub(crate) just_release: Buttons,
        pub(crate) style: ControllerStyle,
    }

    impl Controller {
        /// # Safety
        /// controller is the live Controller pointer of the P1 mapping call.
        pub(crate) unsafe fn read(controller: usize) -> Controller {
            let at = |offset: usize| ((controller + offset) as *const u32).read_volatile();
            Controller {
                current_buttons: Buttons(at(field::CURRENT_BUTTONS)),
                just_down: Buttons(at(field::JUST_DOWN)),
                just_release: Buttons(at(field::JUST_RELEASE)),
                style: if at(field::STYLE) == STYLE_GAMECUBE { ControllerStyle::GCController } else { ControllerStyle::Other },
            }
        }
    }
}

/// Worker A's roster cycle (`character_roster` and its Mii children): not merged, so every call answers "unavailable". The named
/// layer then plays on the settled CPU only (no fighter or exact-Mii request) and says so in the install line.
pub(crate) mod roster {
    pub(crate) fn busy() -> bool {
        false
    }

    pub(crate) fn waiting_drafts_ready() -> bool {
        true
    }

    pub(crate) fn cpu_reload_ready() -> bool {
        false
    }

    pub(crate) fn request_cpu_kind(_kind: u32) -> bool {
        false
    }

    pub(crate) fn current_mii_configuration(_role: usize) -> Option<crate::engine::mii::lifetime::Configuration> {
        None
    }

    pub(crate) mod mii_replay {
        use crate::engine::mii::replay::ReplayMiis;

        /// A committed replay capture's exact Mii metadata (the cycle remembers it for exact reloads).
        pub(crate) fn committed(_miis: ReplayMiis) {}

        use crate::engine::mii::replay::Target;

        /// An exact-Mii macro or capture track cannot be requested until the cycle exists; fighter targets compare as before.
        pub(crate) fn named_ready(target: Target, kind: u32) -> bool {
            target.configuration().is_none() && target.ready(kind, None)
        }

        pub(crate) fn ready(target: Target, kind: u32) -> bool {
            named_ready(target, kind)
        }

        pub(crate) fn request_named(_target: Target) -> bool {
            false
        }

        pub(crate) fn named_refusal(target: Target) -> Option<&'static str> {
            target.configuration().is_some().then_some("Exact Mii loading is unavailable until the roster cycle lands")
        }

        pub(crate) fn reacknowledge_named(_target: Target) {}

        /// A replay track's Mii refusal (an exact Mii cannot be loaded until the cycle exists).
        pub(crate) fn refusal(target: Target, _revision: u64) -> Option<&'static str> {
            named_refusal(target)
        }

        pub(crate) fn request(_target: Target, _revision: u64) -> bool {
            false
        }

        pub(crate) fn resolve(_raw: crate::engine::mii::replay::RawMii, _revision: u64) -> Result<std::sync::mpsc::Receiver<Result<crate::engine::mii::lifetime::Configuration, String>>, String> {
            Err("Recorded Mii metadata cannot be resolved until the roster cycle lands".to_owned())
        }
    }
}

/// The Mii editor bridge (package 6B, carried and not declared until the roster cycle lands).
pub(crate) mod editor_bridge {
    /// A saved-Mii catalog was read (the bridge keeps its last view for the editor).
    pub(crate) fn catalog_read(_catalog: &crate::engine::mii::inventory::Snapshot) {}
}

/// `Macro Source`: where the CPU's recordings come from (the settings row `macro_source`, option order Numbered Slots, Named Library).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MacroSource(pub(crate) u8);

impl MacroSource {
    pub(crate) const NUMBERED: MacroSource = MacroSource(0);
    pub(crate) const NAMED: MacroSource = MacroSource(1);
}

/// What the install line says about this layer's dependency boundary.
pub(crate) fn install_report() -> &'static str {
    "roster=unavailable_until_cycle_lands menu_app=unavailable_until_pkg5_screens"
}

/// The fork's `training::practice` as the named layer reads it: the clean room's scene and body facts. Lobby practice (the waiting room)
/// joins when package 6's lobby facade is declared; until then the facts are those of native Training.
pub(crate) mod practice {
    use smash::app::BattleObjectModuleAccessor;
    use smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID;

    use crate::runtime::features::recorder;
    use crate::runtime::game::natives;
    use crate::runtime::scene;

    /// The practice session is live (native Training or an active lobby practice) and the native UI is not busy.
    pub(crate) fn enabled() -> bool {
        scene_enabled() && !crate::runtime::published::input_owners().contains(crate::model::input_delay::Owners::NATIVE_UI_BUSY)
    }

    pub(crate) fn scene_enabled() -> bool {
        scene::is_training()
    }

    /// Both practice fighters are present now (nothing is retained).
    ///
    /// # Safety
    /// The natives' guards decide; every read is of the live game.
    pub(crate) unsafe fn acquire() -> bool {
        recorder::body_of(0).is_some() && recorder::cpu_accessor().is_some()
    }

    /// The waiting room's practice session (lobby practice, package 6).
    pub(crate) fn waiting() -> bool {
        crate::runtime::scene::lobby_active()
    }

    /// The practice role of a fighter body: 0 for Player 1, 1 for the CPU.
    ///
    /// # Safety
    /// `module` is a live fighter's accessor.
    pub(crate) unsafe fn role(module: &BattleObjectModuleAccessor) -> Option<usize> {
        let address = module as *const BattleObjectModuleAccessor as usize;
        if recorder::body_of(0).is_some_and(|body| body.accessor == address) {
            Some(0)
        } else if recorder::cpu_accessor() == Some(address) {
            Some(1)
        } else {
            None
        }
    }

    /// The fighter entry of `module` when it is a practice body the panel may hold still (P1 or the CPU).
    ///
    /// # Safety
    /// `module` is a live fighter's accessor.
    pub(crate) unsafe fn panel_entry(module: &BattleObjectModuleAccessor) -> Option<usize> {
        if !enabled() || !natives::is_fighter(module as *const BattleObjectModuleAccessor as usize) {
            return None;
        }
        let entry = natives::get_int(module as *const BattleObjectModuleAccessor as usize, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
        let pointer = natives::fighter_entry(entry);
        (pointer != 0).then_some(pointer)
    }

    /// `module` is the primary CPU body (the recorder's rule: the CPU entry, never Nana).
    ///
    /// # Safety
    /// `module` is a live fighter's accessor.
    pub(crate) unsafe fn primary_cpu(module: &mut BattleObjectModuleAccessor) -> bool {
        recorder::cpu_accessor() == Some(module as *mut BattleObjectModuleAccessor as usize)
    }
}

/// The lobby practice facts the named layer reads (the waiting room): inactive until package 6's lobby facade is declared.
pub(crate) mod lobby {
    pub(crate) fn active() -> bool {
        crate::runtime::scene::lobby_active()
    }

    pub(crate) fn is_waiting_scene() -> bool {
        crate::runtime::scene::lobby_active()
    }

    /// The native waiting-room UI (a dialog or the pause menu) holds P1's input.
    pub(crate) fn native_ui_busy() -> bool {
        crate::runtime::published::input_owners().contains(crate::model::input_delay::Owners::NATIVE_UI_BUSY)
    }
}

/// The fork's `save_states` as the named layer and the clip preview call it, over package 3's machine.
pub(crate) mod save_states {
    use crate::runtime::features::states;

    pub(crate) fn is_loading() -> bool {
        states::is_loading()
    }

    pub(crate) fn cancel_pending() {
        states::cancel_pending();
    }

    /// Restore the saved state, then the named layer plays (the restore's completion calls `recorder::on_state_restored`).
    pub(crate) fn request_macro_restore() -> Result<(), &'static str> {
        states::request_load();
        Ok(())
    }

    /// The clip preview's temporary setup restore is package 3's to provide (a restore of the clip's own start state, then the
    /// saved state again); until it exists a preview refuses with this text.
    pub(crate) fn request_preview_restore(_start: bool) -> Result<(), &'static str> {
        Err("Native preview is unavailable until the preview restore is provided")
    }

    pub(crate) fn finish_preview_restore() {}

    pub(crate) fn end_preview_restore() {}
}

/// The fork's name-keyed notifications over package 4's standing notices.
pub(crate) mod notifications {
    use crate::engine::lobby::notice_ids::{id_of, HEADERS};
    use crate::model::notify::DEFAULT_COLOR;
    use crate::runtime::features::notices;

    /// Show `body` under `header` for `frames` (replacing the notice of the same header).
    pub(crate) fn notification(header: String, body: String, frames: u16) {
        if let Some(slot) = id_of(&header) {
            notices::upsert(HEADERS[slot].0, HEADERS[slot].1, &body, frames, DEFAULT_COLOR);
        }
    }
}

/// A recording or a playback is running (the recorder).
pub(crate) fn recording_or_playback() -> bool {
    crate::runtime::features::recorder::recording_or_playback()
}

/// The fork's `input_record` as the named layer calls the core (SPEC-input-01 section 9), over `features::recorder`.
pub(crate) mod input_record {
    use crate::engine::macros::{InputFrame, MacroRecording, StartState};
    use crate::engine::mii::lifetime::Configuration;
    use crate::model::input_delay::MappedInputs;
    use crate::model::recording::{ExportRefusal, ImportRefusal, Origin, StartSnapshot};
    use crate::runtime::features::recorder;

    pub(crate) fn stop_playback() {
        recorder::stop();
    }

    pub(crate) fn is_playback() -> bool {
        recorder::with_recorder(|rec| rec.is_playback())
    }

    /// A clip preview: the recording plays from the layer's buffer, on whichever fighter is on stage.
    pub(crate) fn playback_preview(recording: MacroRecording) -> bool {
        let started = crate::runtime::macros::named_macros::begin_preview(recording);
        recorder::start_named_playback(started.len, started.facing, started.status)
    }

    /// The loaded named recording plays (playback_named): false when nothing is loaded or the fighter is not ready.
    pub(crate) fn playback_named() -> bool {
        match crate::runtime::macros::named_macros::begin_playback() {
            Some(started) => recorder::start_named_playback(started.len, started.facing, started.status),
            None => false,
        }
    }

    /// The named record chord's start (after a restore, the record-on-load action).
    pub(crate) fn start_named_lockout() {
        recorder::start_named_lockout();
    }

    fn frame_of(m: MappedInputs) -> InputFrame {
        InputFrame { buttons: m.buttons, lstick_x: m.lstick_x, lstick_y: m.lstick_y, rstick_x: m.rstick_x, rstick_y: m.rstick_y }
    }

    fn mapped_of(f: &InputFrame) -> MappedInputs {
        MappedInputs { buttons: f.buttons, lstick_x: f.lstick_x, lstick_y: f.lstick_y, rstick_x: f.rstick_x, rstick_y: f.rstick_y }
    }

    /// `export_named`: a recorded slot as the layer's frames and start state (SPEC-input-01 section 9). The exact Mii of the CPU
    /// at the lockout is not captured until the roster cycle provides it, so none is attached.
    pub(crate) fn export_named(slot: usize) -> Result<(Vec<InputFrame>, StartState, Option<Configuration>), &'static str> {
        let slot = u8::try_from(slot).unwrap_or(u8::MAX);
        recorder::with_recorder(|rec| match rec.export_slot(slot) {
            Ok((frames, origin)) => {
                let start = StartState {
                    fighter_kind: origin.start.fighter_kind,
                    costume: origin.start.costume,
                    facing: f32::from(origin.facing),
                    situation: origin.start.situation,
                    status: origin.status,
                    speed_x: origin.start.speed_x,
                    speed_y: origin.start.speed_y,
                };
                Ok((frames.iter().copied().map(frame_of).collect(), start, None))
            }
            Err(refusal) => Err(refusal.message()),
        })
    }

    /// `import_named_slot`: replace one slot with `frames`, or change nothing.
    pub(crate) fn import_named_slot(slot: usize, frames: &[InputFrame], start: StartState, _mii: Option<Configuration>) -> Result<(), String> {
        let slot = u8::try_from(slot).unwrap_or(u8::MAX);
        let origin = Origin {
            facing: if start.facing < 0.0 { -1 } else { 1 },
            status: start.status,
            start: StartSnapshot {
                fighter_kind: start.fighter_kind,
                costume: start.costume,
                situation: start.situation,
                speed_x: start.speed_x,
                speed_y: start.speed_y,
            },
        };
        let mapped: Vec<MappedInputs> = frames.iter().map(mapped_of).collect();
        recorder::with_recorder(|rec| rec.import_slot(slot, &mapped, origin)).map_err(ImportRefusal::message)
    }

    /// Why a slot cannot be exported, as the text the layer shows.
    pub(crate) fn refusal_text(refusal: ExportRefusal) -> &'static str {
        refusal.message()
    }
}
