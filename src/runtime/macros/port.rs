//! What the named-macro layer adds around the carried files: the fork's lock helpers (`sync`), the controller record (`input`), the typed
//! menu names (`consts`). The practice session, the lobby, the
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

/// The game's Controller as the carried layer names it.
pub(crate) mod input {
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

/// The preview uses Recording's playback.
pub(crate) fn install_report() -> &'static str {
    "preview=recording_playback"
}

/// Design C (lead, 2026-11-02): the named-macro layer's model has one owner, the menu worker. This is the worker's tick for it, called
/// on the worker thread at every wake (`ui::worker::run`): package 4 points it at its layer's worker service (draining the layer's
/// `hos_lock::Mailbox` of game-thread events, the deferred effects, publishing `MacroQueries`). A game thread that posts an event calls
/// `crate::runtime::ui::worker::wake()` after posting.
pub(crate) fn worker_events() {
    if super::ready() {
        super::named_macros::worker_lifecycle();
        super::replay_capture::worker_vault_events();
        super::live_replay::worker_events();
    }
}

static RELOAD_FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn worker_tick() {
    worker_events();
    if !super::ready() { return; }
    super::identity::worker_tick();
    #[cfg(feature = "diag")]
    super::diag::run_pending();
    super::replay_capture::worker_preview_notices();
    CLIP_EVENTS.drain(|event| {
        if event.revision != super::replay_capture::selection_revision() { return; }
        match event.kind {
            ClipEventKind::Input(controller) => super::replay_capture::on_input(&controller),
            ClipEventKind::Draw => super::replay_capture::after_draw(),
        }
    });
    let dropped = CLIP_EVENTS.take_dropped();
    if dropped != 0 {
        crate::runtime::diag_log::diagnostic(format_args!("[MacroHandoff] clip_events_dropped={dropped}"));
    }
    super::named_macros::service_character();
    let frame = crate::runtime::ui::facts::fact_frame_number();
    if RELOAD_FRAME.swap(frame, std::sync::atomic::Ordering::AcqRel) != frame {
        super::replay_capture::service_reload();
    }
    super::named_macros::run_deferred();
    super::named_macros::publish_queries();
    super::sync_owner();
    super::named_macros::service_keyboard();
    super::named_macros::run_deferred();
    super::named_macros::publish_queries();
    super::sync_owner();
    crate::runtime::features::recording_host::service();
}

#[derive(Clone, Copy)]
enum ClipEventKind { Input(input::Controller), Draw }
#[derive(Clone, Copy)]
struct ClipEvent { revision: u64, kind: ClipEventKind }
static CLIP_EVENTS: hos_lock::Mailbox<ClipEvent, 16> = hos_lock::Mailbox::new();

fn post_clip(kind: ClipEventKind) {
    CLIP_EVENTS.post(ClipEvent { revision: super::replay_capture::selection_revision(), kind });
    crate::runtime::ui::worker::wake();
}

pub(crate) fn post_clip_input(controller: input::Controller) {
    post_clip(ClipEventKind::Input(controller));
}

pub(crate) fn post_clip_draw() {
    if super::named_macros::hook_queries().clip_foreground {
        post_clip(ClipEventKind::Draw);
    }
}
