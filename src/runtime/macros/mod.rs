//! Package 4, the named-macro / replay layer (S4-4), game side. The files are the fork's `src/training/named_macros.rs`, `src/replay_*.rs`,
//! `src/live_replay.rs` and `src/capture_plugin.rs`, carried by `tools/carry_pkg4.py` (stage m5q; recorded edits in
//! `docs/provenance/pkg4-carry-ledger.md`). They name the fork's mixed modules; `runtime::compat` and the lobby `training` facade meet
//! those names (package 6's shims, extended in place), and `port` holds what depends on a package that is not merged yet.
//!
//! This module is also the carried files' `super::`: the fork's `training` module names they reach through their parent
//! (`practice`, `lobby`, `input_record`, `save_states`, `frame_counter`) are re-exported here.

// The carried files print their event lines with `println!`: inside this module that is the plugin's diagnostics ring (no Skyline logger,
// no allocation), by textual macro scope, so the carried text stays as it is.
macro_rules! println {
    ($($arg:tt)*) => {
        $crate::runtime::diag_log::diagnostic(format_args!($($arg)*))
    };
}

#[cfg(feature = "diag")]
pub(crate) mod diag;
pub(crate) mod live_replay;
pub(crate) mod named_macros;
pub(crate) mod port;
pub(crate) mod replay_preview;
pub(crate) mod replay_move_start;
pub(crate) mod replay_capture;
pub(crate) mod replay_store;

// The fork's `training` names the carried files reach through `super::`: package 6's practice session and lobby facade, the recorder
// (`port::input_record`) and the save-state flag of package 3.
pub(crate) use crate::runtime::lobby::training::{lobby, practice};
pub(crate) use port::input_record;

/// The fork's `save_states` as the named layer and the clip preview call it: package 3's own functions, and the preview restore, which
/// package 3 does not provide yet (`install_report` names it).
pub(crate) mod save_states {
    pub(crate) use crate::runtime::features::states::cancel_pending;
    pub(crate) use crate::runtime::lobby::training::save_states::is_loading;

    /// Restore the saved state; the restore's completion calls `recorder::on_state_restored`, which starts the named macro.
    pub(crate) fn request_macro_restore() -> Result<(), &'static str> {
        crate::runtime::features::states::request_load();
        Ok(())
    }

    /// The clip preview's temporary setup restore (a restore of the clip's own start state, then the saved state again).
    pub(crate) fn request_preview_restore(_start: bool) -> Result<(), &'static str> {
        Err("Native preview is unavailable until the preview restore is provided")
    }

    pub(crate) fn finish_preview_restore() {}

    pub(crate) fn end_preview_restore() {}
}

use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::published;

use std::sync::atomic::{AtomicBool, Ordering};

/// Every row the layer calls passed its guard and install ran.
static READY: AtomicBool = AtomicBool::new(false);

pub(crate) fn ready() -> bool {
    READY.load(Ordering::Acquire)
}

/// Macro Source is the Named Library and the layer is installed.
pub(crate) fn named_active() -> bool {
    ready() && named_macros::source_is_named()
}

/// Boot: the replay-capture preference and writer, the named library's selected macro, the capture adapter's guards and hooks (the seam
/// installer decides; hooks only when the INI says on). Returns `(named layer ready, live capture adapter ready)`.
pub(crate) fn install(named_source: bool, rows_ok: bool) -> (bool, bool) {
    // R2-1: every game call the layer makes is a guarded row (NamedMacros, plus the recorder's own); a rejected row leaves the whole layer off.
    if !rows_ok {
        return (false, false);
    }
    READY.store(true, Ordering::Release);
    replay_store::init();
    named_macros::init();
    replay_capture::init();
    if named_source {
        named_macros::prepare_buffers();
    }
    live_replay::init();
    #[cfg(feature = "diag")]
    diag::start();
    (true, replay_store::toggle().is_ready())
}

/// One fighter's frame, before the game's own on-frame function (the shared per-fighter wrapper): the capture observers sample the state
/// the game is about to advance, and the CPU body's preview and named-start observers run.
///
/// # Safety
/// `fighter` is the live fighter object the game is running a frame of.
pub(crate) unsafe fn frame_before(fighter: usize) {
    if fighter == 0 || !ready() {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::LiveReplay);
    let fighter = &mut *(fighter as *mut smash::app::Fighter);
    live_replay::before_frame(fighter);
    replay_capture::before_frame(fighter);
    if let Some(module) = fighter.battle_object.module_accessor.as_mut() {
        if practice::primary_cpu(module) {
            named_macros::observe_playback_before(module);
            replay_preview::before_frame(module);
        }
    }
}

/// The same fighter's frame, after the game's on-frame function.
///
/// # Safety
/// As `frame_before`.
pub(crate) unsafe fn frame_after(fighter: usize) {
    if fighter == 0 || !ready() {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::LiveReplay);
    let fighter = &mut *(fighter as *mut smash::app::Fighter);
    live_replay::after_frame(fighter);
    replay_capture::after_frame(fighter);
    if let Some(module) = fighter.battle_object.module_accessor.as_mut() {
        named_macros::hold_panel_fighter(module);
        if practice::primary_cpu(module) {
            named_macros::observe_playback_start(module);
            replay_preview::after_frame(module);
            named_macros::preview(module);
            // Once per game frame, on the CPU body: the keyboard, the character request and the reload of the clip editor.
            named_macros::service_keyboard();
            named_macros::service_character();
            replay_capture::service_reload();
            sync_owner();
            #[cfg(feature = "diag")]
            diag::run_pending();
        }
    }
}

/// The replay editor holds the input while its panel (or the capture browser) is up: the owner bit the input stages read.
fn sync_owner() {
    use crate::model::input_delay::Owners;
    let held = replay_capture::panel_visible() || replay_capture::browser_visible() || replay_preview::occupied();
    if held {
        published::INPUT_OWNERS.fetch_or(Owners::REPLAY_EDITOR.bits(), std::sync::atomic::Ordering::AcqRel);
    } else {
        published::INPUT_OWNERS.fetch_and(!Owners::REPLAY_EDITOR.bits(), std::sync::atomic::Ordering::AcqRel);
    }
}

/// P1's physical controls while the clip editor is in the foreground (the Replay Window's own controls): `out` is made neutral.
///
/// # Safety
/// `controller` is the live `Controller` of the mapping call (or 0); `out` the record the game just mapped.
pub(crate) unsafe fn on_p1_input(controller: usize, out: *mut crate::model::input_delay::MappedInputs) {
    if controller == 0 || !ready() || !named_macros::clip_foreground() {
        return;
    }
    replay_capture::on_input(&port::input::Controller::read(controller));
    *out = crate::model::input_delay::MappedInputs::NEUTRAL;
}