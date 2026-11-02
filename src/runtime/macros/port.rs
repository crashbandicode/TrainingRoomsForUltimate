//! What the named-macro layer adds around the carried files: the fork's lock helpers (`sync`), the controller record (`input`), the typed
//! menu names (`consts`) and the recorder as the fork's `input_record` (SPEC-input-01 section 9). The practice session, the lobby, the
//! roster cycle, the Mii bridge, the notifications and the save-state flag are package 6's and package 3's own providers (the carry
//! manifest's module map points the carried paths at them). What is still not provided is named by `install_report`.

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

/// What the install line says about the dependency boundary that is left: the clip preview's temporary setup restore (package 3 does not
/// provide it yet) and the menu's panel screens (package 5; the layer's API is ready for them).
pub(crate) fn install_report() -> &'static str {
    "preview_restore=unavailable_until_pkg3"
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

/// Design C (lead, 2026-11-02): the named-macro layer's model has one owner, the menu worker. This is the worker's tick for it, called
/// on the worker thread at every wake (`ui::worker::run`): package 4 points it at its layer's worker service (draining the layer's
/// `hos_lock::Mailbox` of game-thread events, the deferred effects, publishing `MacroQueries`). A game thread that posts an event calls
/// `crate::runtime::ui::worker::wake()` after posting.
pub(crate) fn worker_tick() {
    crate::runtime::macros::named_macros::run_deferred();
}
