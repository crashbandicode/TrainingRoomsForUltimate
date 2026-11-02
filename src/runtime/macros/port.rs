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
    use crate::engine::macros::MacroRecording;
    use crate::runtime::features::recorder;

    pub(crate) fn stop_playback() {
        recorder::stop();
    }

    pub(crate) fn is_playback() -> bool {
        recorder::is_playback()
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


}

/// Design C (lead, 2026-11-02): the named-macro layer's model has one owner, the menu worker. This is the worker's tick for it, called
/// on the worker thread at every wake (`ui::worker::run`): package 4 points it at its layer's worker service (draining the layer's
/// `hos_lock::Mailbox` of game-thread events, the deferred effects, publishing `MacroQueries`). A game thread that posts an event calls
/// `crate::runtime::ui::worker::wake()` after posting.
pub(crate) fn worker_tick() {
    if !super::ready() { return; }
    slots::worker_slot_tick();
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
    super::named_macros::consume_capture();
    super::named_macros::service_character();
    super::named_macros::run_deferred();
    super::named_macros::publish_queries();
    super::sync_owner();
    super::named_macros::service_keyboard();
    super::named_macros::run_deferred();
    super::named_macros::publish_queries();
    super::sync_owner();
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

/// Numbered-slot file/UI work stays on the menu worker. Only the fixed transfer
/// visits the game-owned recorder; strings and completion effects never do.
pub(crate) mod slots {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use crate::model::macro_handoff::{SlotTransfer, SlotResult};
    use crate::model::recording::{Origin, StartSnapshot};
    use crate::model::input_delay::MappedInputs;
    use crate::engine::macros::{InputFrame, MacroRecording, StartState};

    enum Completion { Export, Import { name: String, slot: usize } }
    struct Job { request: Message, completion: Completion }
    struct Worker { queue: VecDeque<Job>, pending: Option<(u64, Completion)> }
    static WORKER: Mutex<Worker> = Mutex::new(Worker { queue: VecDeque::new(), pending: None });
    #[derive(Clone, Copy)]
    struct Message { session: u64, transfer: SlotTransfer, cancelled: bool }
    static REQUEST: hos_lock::Mailbox<Message, 1> = hos_lock::Mailbox::new();
    static REPLY: hos_lock::Mailbox<Message, 1> = hos_lock::Mailbox::new();

    fn enqueue(transfer: SlotTransfer, completion: Completion) {
        let request = Message { session: super::super::named_macros::session_revision(), transfer, cancelled: false };
        WORKER.lock().unwrap().queue.push_back(Job { request, completion });
        crate::runtime::ui::worker::wake();
    }

    pub fn export(slot: usize) { enqueue(SlotTransfer::export(slot), Completion::Export); }

    pub fn import(slot: usize, recording: &MacroRecording) -> Result<(), String> {
        let start = recording.start;
        let origin = Origin {
            facing: if start.facing < 0.0 { -1 } else { 1 }, status: start.status,
            start: StartSnapshot { fighter_kind: start.fighter_kind, costume: start.costume,
                situation: start.situation, speed_x: start.speed_x, speed_y: start.speed_y },
        };
        let frames: Vec<_> = recording.frames.iter().map(|frame| MappedInputs {
            buttons: frame.buttons, lstick_x: frame.lstick_x, lstick_y: frame.lstick_y,
            rstick_x: frame.rstick_x, rstick_y: frame.rstick_y,
        }).collect();
        let transfer = SlotTransfer::import(slot, &frames, origin).map_err(|error| error.message())?;
        enqueue(transfer, Completion::Import { name: recording.name.clone(), slot });
        Ok(())
    }

    // Keep the bounded frame-array copies out of the enclosing fighter callback's stack frame.
    #[inline(never)]
    pub fn service_slot_request() {
        REQUEST.drain(|mut message| {
            message.cancelled = message.session != super::super::named_macros::session_revision();
            if !message.cancelled {
                crate::runtime::features::recorder::transfer_slot(&mut message.transfer);
            }
            let posted = REPLY.post(message);
            debug_assert!(posted, "one outstanding slot transfer");
            crate::runtime::ui::worker::wake();
        });
    }

    pub fn worker_slot_tick() {
        REPLY.drain(|message| {
            let pending = WORKER.lock().unwrap().pending.take();
            let Some((session, completion)) = pending else { return; };
            if message.cancelled || session != super::super::named_macros::session_revision() { return; }
            match completion {
                Completion::Export => {
                    match message.transfer.result {
                        SlotResult::ExportFailed(error) => super::super::named_macros::slot_export_failed(error.message()),
                        SlotResult::Exported => {
                            let (frames, origin) = message.transfer.exported().unwrap();
                            let start = StartState { fighter_kind: origin.start.fighter_kind, costume: origin.start.costume,
                                facing: f32::from(origin.facing), situation: origin.start.situation, status: origin.status,
                                speed_x: origin.start.speed_x, speed_y: origin.start.speed_y };
                            let frames = frames.iter().map(|frame| InputFrame { buttons: frame.buttons,
                                lstick_x: frame.lstick_x, lstick_y: frame.lstick_y,
                                rstick_x: frame.rstick_x, rstick_y: frame.rstick_y }).collect();
                            super::super::named_macros::slot_exported(frames, start);
                        }
                        _ => unreachable!("export reply"),
                    }
                }
                Completion::Import { name, slot } => {
                    let result = match message.transfer.result {
                        SlotResult::Imported => Ok(()),
                        SlotResult::ImportFailed(error) => Err(error),
                        _ => unreachable!("import reply"),
                    };
                    super::super::named_macros::slot_imported(&name, slot, result);
                }
            }
        });
        let mut worker = WORKER.lock().unwrap();
        if worker.pending.is_some() { return; }
        let current = super::super::named_macros::session_revision();
        while let Some(job) = worker.queue.pop_front() {
            if job.request.session != current { continue; }
            let posted = REQUEST.post(job.request);
            debug_assert!(posted, "one outstanding slot transfer");
            worker.pending = Some((current, job.completion));
            break;
        }
    }
}
