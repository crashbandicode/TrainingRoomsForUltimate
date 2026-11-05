//! Named-macro runtime.
//!
//! The portable recording lives in `training_mod_macros`. This module owns the
//! Training-mode file library, the in-game panel, character requests, and the
//! restore offset. Capture and playback still go through the existing recorder
//! hooks: live playback advances only `INPUT_RECORD_FRAME` inside
//! `set_cpu_controls`. `MacroUi::feed_playback` is the host-tested clock and is
//! not called from these hooks, so a frame is not advanced twice.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};

use smash::app::{lua_bind::*, utility, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
use smash::phx::{Hash40, Vector2f, Vector3f};
use {crate::engine::macros::InputFrame, crate::engine::macros::Library, crate::engine::macros::MacroRecording, crate::engine::macros::StartState};
use crate::runtime::macros::port::sync::*;
use crate::engine::macro_ui::macro_panel::{
    Effect, FighterGate, FighterOrigin, LibraryRow, LoadContext, MacroUi,
    NameContext, Pad, View, MACRO_KEYBOARD_TITLE, SETUP_KEYBOARD_TITLE,
};

use crate::runtime::macros::port::consts::{FighterId, MacroSource, OnOff, MENU};
use crate::runtime::macros::port::input::MappedInputs;
use crate::runtime::compat::common::menu;
use super::practice;
use crate::runtime::lobby::training::ui::notifications::upsert;

static UI: RwLock<MacroUi> = RwLock::new(MacroUi::new());
static SUSPENDED: AtomicBool = AtomicBool::new(false);
static CONFIRMED_NAME: RwLock<Option<NameConfirmation>> = RwLock::new(None);
static LIST_ACTION: RwLock<Option<String>> = RwLock::new(None);
/// Record slot captured when the named library or slot export was opened.
/// Import uses this frozen index. It is not reread from the committed menu.
static RECORD_SLOT: RwLock<Option<usize>> = RwLock::new(None);

/// Keyboard result for a draft the caller still owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameConfirmation {
    pub context: NameContext,
    pub name: String,
}

pub fn take_confirmed_name() -> Option<NameConfirmation> {
    lock_write(&CONFIRMED_NAME).take()
}

/// Y on a library opened through [`open_external_library`].
pub fn take_list_action() -> Option<String> {
    lock_write(&LIST_ACTION).take()
}

static LIST_CHOICE: RwLock<Option<String>> = RwLock::new(None);
static LIST_RENAME: RwLock<Option<String>> = RwLock::new(None);

/// X on a library opened through [`open_external_library`]: the opener renames the row.
pub fn take_list_rename() -> Option<String> {
    lock_write(&LIST_RENAME).take()
}

/// A on a library opened through [`open_external_library`]. The named-macro
/// loader never sees that row; the opener owns it.
pub fn take_list_choice() -> Option<String> {
    lock_write(&LIST_CHOICE).take()
}

/// True while the caller-owned name draft for `token` is on the panel, open or
/// parked across a same-lobby interruption. False once it was confirmed,
/// cancelled, or discarded by a new lobby.
pub fn external_name_open(token: u64) -> bool {
    lock_read(&UI).external_name_token() == Some(token)
}

pub fn suspended() -> bool { SUSPENDED.load(Ordering::Acquire) }
#[derive(Clone, Copy)]
enum Lifecycle { Suspend, SceneChanged { discard: bool }, Clear { keep: bool } }
static LIFECYCLE: hos_lock::Mailbox<Lifecycle, 32> = hos_lock::Mailbox::new();

fn post_lifecycle(event: Lifecycle) {
    LIFECYCLE.post(event);
    crate::runtime::ui::worker::wake();
}

pub fn suspend_waiting() {
    if SUSPENDED.swap(true, Ordering::AcqRel) { return; }
    SESSION_GENERATION.fetch_add(1, Ordering::AcqRel);
    post_lifecycle(Lifecycle::Suspend);
}
pub fn waiting_scene_changed(same_lobby: bool) {
    if !same_lobby {
        crate::runtime::features::states::retire_setup_session();
        let discard = SUSPENDED.swap(false, Ordering::AcqRel);
        post_lifecycle(Lifecycle::SceneChanged { discard });
    }
}

/// Drain before UI input so an old retirement cannot close a new scene's panel.
pub fn worker_lifecycle() {
    LIFECYCLE.drain(|event| match event {
        Lifecycle::Suspend => {
            lock_write(&UI).invalidate_practice_context();
            crate::runtime::macros::replay_capture::suspend_waiting();
            cancel_character_selection();
            retire_name(true);
        }
        Lifecycle::SceneChanged { discard } => {
            retire_name(false);
            if discard {
                lock_write(&UI).close_transient();
                crate::runtime::macros::replay_capture::discard_waiting();
            }
        }
        Lifecycle::Clear { keep } => {
            retire_name(keep);
            *lock_write(&CONFIRMED_NAME) = None;
            *lock_write(&LIST_ACTION) = None;
            *lock_write(&LIST_CHOICE) = None;
            *lock_write(&LIST_RENAME) = None;
            if !keep {
                *lock_write(&RECORD_SLOT) = None;
                lock_write(&UI).close_transient();
                crate::runtime::macros::replay_capture::close_editor();
            }
        }
    });
    let dropped = LIFECYCLE.take_dropped();
    if dropped != 0 { println!("[MacroHandoff] lifecycle_events_dropped={dropped}"); }
}

pub fn resume_waiting() {
    if waiting_name_ready() { SUSPENDED.store(false, Ordering::Release); }
}
fn waiting_name_ready() -> bool { crate::runtime::ui::facts::fact_waiting_ready() }

static GENERATION: AtomicU64 = AtomicU64::new(0);
// A preview's normal stop/restore advances the motion generation while its
// export keyboard is open. Only session cleanup invalidates that naming draft.
static SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);
pub(crate) fn session_revision() -> u64 { SESSION_GENERATION.load(Ordering::Acquire) }
static NAME_SERIAL: AtomicU64 = AtomicU64::new(1);
// Kept through the requested native install's ordinary session cleanup. True
// scene/destruction retirement remains the existing CPU generation boundary.
static NAMED_SELECTION_REVISION: AtomicU64 = AtomicU64::new(1);
static NAMED_REFUSAL_SHOWN: AtomicU64 = AtomicU64::new(0);
static NAMED_SOURCE_ACTIVE: AtomicBool = AtomicBool::new(false);
pub(crate) fn selection_revision() -> u64 { NAMED_SELECTION_REVISION.load(Ordering::Acquire) }
pub(crate) fn cancel_character_selection() {
    NAMED_SELECTION_REVISION.fetch_add(1, Ordering::AcqRel);
    NAMED_REFUSAL_SHOWN.store(0, Ordering::Release);
    lock_write(&UI).invalidate_character_request();
}
/// Select a recording as the named macro. `prompt` is true only for an explicit
/// library load: boot restore and a clip saved right after its own fighter choice
/// never ask. During the RC freeze only matching-current playback is enabled;
/// a boot-restored selection waits for its recorded CPU identity.
fn commit_selected(id:String, recording:MacroRecording, prompt:bool, current:Option<i32>) -> Result<Effect,String> {
    let context=load_context(&recording,prompt,current);
    if !crate::runtime::ADVANCED_RECORDING_WORKFLOWS {
        let target = recording.playback_target().map_err(|e| e.to_string())?;
        if target.configuration().is_some() {
            return Err("Exact Mii named loading is unavailable in this release".into());
        }
        // Boot may restore the selection before a CPU exists. Published playback
        // remains gated on its recorded identity once the fighter settles.
        if current.is_some() && !context.matches {
            return Err("Choose the recorded CPU fighter before loading this macro".into());
        }
    }
    // A pending question changes nothing yet: the old selection's revision, and
    // any exact-Mii receipt tied to it, stay valid until the user answers.
    if context.gate(&recording)!=FighterGate::Prompt {begin_selection();}
    lock_write(&UI).offer_load(id,recording,context).map_err(|e| e.to_string())
}
/// The named selection changes now: retire every request tied to the old one.
fn begin_selection() {
    NAMED_SELECTION_REVISION.fetch_add(1, Ordering::AcqRel);
    NAMED_REFUSAL_SHOWN.store(0, Ordering::Release);
}
/// Everything the panel needs to decide, from the runtime's exact predicate. Class
/// equality alone is never a match: a same-class Mii with another exact
/// configuration is a mismatch. `current` is the live CPU kind, or None before
/// any fighter exists (plugin init).
fn load_context(recording:&MacroRecording, prompt:bool, current:Option<i32>) -> LoadContext {
    let target=recording.playback_target().ok();
    let matches=target.is_some_and(|target|recorded_target_matches(target,current));
    let recorded=fighter_label(recording.start.fighter_kind);
    LoadContext {
        cpu_kind:current,
        matches,
        prompt_enabled:prompt && read(&MENU).fighter_prompt==OnOff::ON,
        // An exact Mii can only be requested by the Mii alpha; other builds keep the CPU.
        can_switch:crate::runtime::ADVANCED_RECORDING_WORKFLOWS && target.is_some_and(|target|cfg!(feature="mii-ordinary-cpu-alpha") || target.configuration().is_none()),
        revision:selection_revision(),
        recorded_label:if target.is_some_and(|target|target.configuration().is_some()) {format!("{recorded} (recorded Mii)")} else {recorded},
        current_label:current.map_or_else(||"CPU".to_owned(),fighter_label),
    }
}
/// Compare identities, not a previous loader request's acknowledgement. A
/// manually selected exact Mii is already a match even without a replay receipt.
pub(crate) fn recorded_target_matches(target:crate::engine::mii::replay::Target,current:Option<i32>) -> bool {
    current.is_some_and(|kind| target.ready(kind as u32,
        crate::runtime::macros::identity::current_configuration()))
}
fn exact_target_ready(target:crate::engine::mii::replay::Target,current:Option<i32>) -> bool {
    let Some(kind)=current else{return false;};
    if target.configuration().is_none() {return target.ready(kind as u32,None);}
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    {crate::runtime::macros::identity::named_ready(target)}
    #[cfg(not(feature="mii-ordinary-cpu-alpha"))]
    {false}
}
fn observe_loaded() {
    let (plays_current,target)={let ui=lock_read(&UI);(ui.plays_current(),ui.loaded_target())};
    let current=cpu_kind();
    let busy=crate::runtime::roster::character_roster::busy();
    // A macro playing on the current CPU has no recorded identity to acknowledge.
    let configuration=if plays_current {None} else {
        target.filter(|target|exact_target_ready(*target,current)).and_then(|target|target.configuration())
    };
    lock_write(&UI).observe_exact_cpu(current,configuration,busy);
}
/// The prompt belongs to one explicit selection. A moved-on selection revision, a
/// suspended waiting scene or a departed practice scene retires it unanswered.
fn fighter_prompt_stale(origin:FighterOrigin, revision:u64) -> bool {
    let current=match origin {
        FighterOrigin::Named=>selection_revision(),
        FighterOrigin::Clip=>crate::runtime::macros::replay_capture::selection_revision(),
    };
    revision!=current || suspended() || !crate::runtime::ui::facts::fact_scene_enabled()
}
fn retire_stale_fighter_prompt() -> bool {
    let Some((origin,revision))=lock_read(&UI).fighter_prompt() else {return false;};
    if !fighter_prompt_stale(origin,revision) {return false;}
    lock_write(&UI).drop_fighter_prompt();
    assign(&menu::MENU_RECEIVED_INPUT,true);
    true
}
/// Open the shared prompt for a replay clip or track. The clip editor owns the
/// candidate and receives the answer through [`apply_effect`].
pub(crate) fn open_clip_fighter_prompt(revision:u64, recorded:String, current:String) -> bool {
    let opened=lock_write(&UI).open_fighter_prompt(revision,recorded,current);
    if opened {assign(&menu::MENU_RECEIVED_INPUT,true);}
    opened
}
static CHARACTER_PENDING: AtomicBool = AtomicBool::new(false);
static CHARACTER_REQUEST: hos_lock::Mailbox<(crate::engine::mii::replay::Target, u64), 1> = hos_lock::Mailbox::new();
static CHARACTER_RESULT: hos_lock::Mailbox<(u64, bool), 1> = hos_lock::Mailbox::new();

fn submit_loaded_character() {
    if !crate::runtime::ADVANCED_RECORDING_WORKFLOWS { return; }
    if CHARACTER_PENDING.load(Ordering::Acquire) { return; }
    let target=lock_read(&UI).loaded_target();
    let Some(target)=target else{return;};
    #[cfg(not(feature="mii-ordinary-cpu-alpha"))]
    if target.configuration().is_some() {
        let revision=selection_revision();
        if NAMED_REFUSAL_SHOWN.swap(revision,Ordering::AcqRel)!=revision {
            notify("Exact Mii macro loading is unavailable in this build");
        }
        return;
    }
    if lock_write(&UI).claim_character_request()!=Some(target.kind() as i32) {return;}
    CHARACTER_PENDING.store(true, Ordering::Release);
    let posted = CHARACTER_REQUEST.post((target, selection_revision()));
    debug_assert!(posted, "one outstanding named character request");
}

/// Game callback only. The roster remains the sole admission/loader provider.
pub fn service_character_request() {
    CHARACTER_REQUEST.drain(|(target, revision)| {
        let eligible = crate::runtime::ADVANCED_RECORDING_WORKFLOWS
            && revision == selection_revision() && source_is_named()
            && crate::runtime::roster::character_roster::cpu_reload_ready();
        #[cfg(feature="mii-ordinary-cpu-alpha")]
        let accepted = eligible && crate::runtime::roster::character_roster::mii_replay::request_named(target);
        #[cfg(not(feature="mii-ordinary-cpu-alpha"))]
        let accepted = eligible && target.configuration().is_none()
            && crate::runtime::roster::character_roster::request_cpu_kind(target.kind());
        let posted = CHARACTER_RESULT.post((revision, accepted));
        debug_assert!(posted, "one outstanding named character result");
        crate::runtime::ui::worker::wake();
    });
}

fn consume_character_result() {
    CHARACTER_RESULT.drain(|(revision, accepted)| {
        CHARACTER_PENDING.store(false, Ordering::Release);
        if revision == selection_revision() && !accepted {
            lock_write(&UI).invalidate_character_request();
        }
    });
}

static NAME_JOB: std::sync::Mutex<Option<NameJob>> = std::sync::Mutex::new(None);

#[derive(Clone, Copy, PartialEq, Eq)]
enum NamePhase { Active, Closing, Parked, Discarding }

#[derive(Clone, Copy)]
struct NameJob { id: u64, generation: u64, replay: bool, waiting: bool, phase: NamePhase }

/// Retirement callbacks may be on another thread. Retain the native job until
/// the owning UI boundary has captured values and acknowledged Finalize.
fn retire_name(keep_waiting: bool) {
    let mut pending = NAME_JOB.lock().unwrap();
    let Some(job) = pending.as_mut() else { return };
    if job.waiting && keep_waiting && job.phase != NamePhase::Discarding {
        if job.phase == NamePhase::Parked { return; }
        job.phase = NamePhase::Closing;
    } else if job.waiting && job.phase != NamePhase::Parked {
        job.phase = NamePhase::Discarding;
    } else {
        crate::runtime::mii::native_keyboard::interrupt(job.id);
        pending.take();
        return;
    }
    crate::runtime::mii::native_keyboard::interrupt(job.id);
}

/// Evaluated before opening and again after Calc, outside all keyboard locks.
fn waiting_name_allowed() -> bool {
    if suspended() || !waiting_name_ready() { return false; }
    NAME_JOB.lock().unwrap().as_ref().is_some_and(|job| job.waiting
        && job.phase == NamePhase::Active
        && job.generation == SESSION_GENERATION.load(Ordering::Relaxed))
}

fn resume_name() {
    if !NAME_JOB.lock().unwrap().as_ref().is_some_and(|job| job.phase == NamePhase::Parked) { return; }
    if suspended() || !waiting_name_ready() || crate::runtime::mii::native_keyboard::busy() { return; }
    let mut pending = NAME_JOB.lock().unwrap();
    let Some(job) = pending.as_mut().filter(|job| job.phase == NamePhase::Parked) else { return };
    let values = {
        let ui = lock_read(&UI);
        let title = ui.native_name_title().unwrap_or(MACRO_KEYBOARD_TITLE);
        ui.name_prompt().zip(ui.native_name_cursor()).map(|(name, cursor)| {
            (name.to_owned(), cursor, title)
        })
    };
    let Some((name, cursor, title)) = values else { pending.take(); return };
    let id = NAME_SERIAL.fetch_add(1, Ordering::Relaxed);
    let opened = if job.waiting {
        crate::runtime::mii::native_keyboard::request_inline(id, &name, cursor)
    } else {
        crate::runtime::mii::native_keyboard::request(id, title, &name)
    };
    match opened {
        Ok(()) => {
            job.id = id;
            job.generation = SESSION_GENERATION.load(Ordering::Relaxed);
            job.phase = NamePhase::Active;
        }
        Err(error) => { pending.take(); drop(pending); notify(&error); }
    }
}
// Only flags set by this panel are released, and only on the same live object.
static PANEL_STOPS: RwLock<Vec<(u32, usize, usize)>> = RwLock::new(Vec::new());
/// Boot reservation of `PANEL_STOPS`: one entry per held fighter body, more than a match has (8 entries and their partners).
const PANEL_STOPS_CAPACITY: usize = 16;

pub fn forget_stopped_entry(entry: usize) {
    lock_write(&PANEL_STOPS).retain(|&(_, _, owner)| owner != entry);
}

pub fn forget_stopped_scene() { lock_write(&PANEL_STOPS).clear(); }
static RESTORING: AtomicBool = AtomicBool::new(false);
// 0 = no automatic action, 1 = record, 2 = playback after both fighters restore.
static RESTORE_ACTION: AtomicU8 = AtomicU8::new(0);
static PLAYBACK_START_PENDING: AtomicBool = AtomicBool::new(false);
static PLAYBACK_START_WAITS: AtomicU8 = AtomicU8::new(0);
static RESTORE_POSITION_WAITS: AtomicU8 = AtomicU8::new(0);
static PLAYBACK_OBSERVATIONS: AtomicU8 = AtomicU8::new(0);
const PLAYBACK_OBSERVATION_LIMIT: u8 = 8;
#[cfg(feature = "named-input-trace")]
static TRACE_RUN: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "named-input-trace")]
static TRACE_WRITES: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "named-input-trace")]
static TRACE_OBSERVATIONS: AtomicU64 = AtomicU64::new(0);
static RESTORE_POSE: RwLock<Option<RestorePose>> = RwLock::new(None);

#[derive(Clone, Copy)]
struct RestorePose {
    object: u32,
    address: usize,
    x: f32,
    y: f32,
    facing: f32,
    air: Option<bool>,
}

// Capture origin is game-owned. Only copied values leave it at completion.
static CAPTURE_ORIGIN: hos_lock::Mutex<StartState> = hos_lock::Mutex::new(StartState {
    fighter_kind: -1, costume: 0, facing: 1.0, situation: 0, status: 0, speed_x: 0.0, speed_y: 0.0,
});
static PLAY_FRAMES: crate::model::macro_handoff::PlayBuffers<InputFrame> = crate::model::macro_handoff::PlayBuffers::new();
static PREVIEW_FRAMES: crate::model::macro_handoff::PlayBuffers<InputFrame> = crate::model::macro_handoff::PlayBuffers::new();
// 0 idle, 1 named, 2 transient preview. Only game callbacks change this.
static PLAYING: AtomicU8 = AtomicU8::new(0);
static PLAY_GENERATION: AtomicU32 = AtomicU32::new(0);
static PREPARED_REVISION: AtomicU64 = AtomicU64::new(u64::MAX);
static RESTORE_ANCHOR: RwLock<Option<(u64, f32, f32)>> = RwLock::new(None);
static RESTORE_ANCHORS: hos_lock::Mailbox<(u64, f32, f32), 8> = hos_lock::Mailbox::new();

static CAPTURE: crate::model::macro_handoff::CaptureBuffer<InputFrame> =
    crate::model::macro_handoff::CaptureBuffer::new();

#[derive(Clone, Copy)]
struct CaptureCompleted { start: StartState, crop: bool, session: u64 }
// Only one completed capture can exist: CAPTURE stays worker-owned until this
// event is consumed. A keyboard may take arbitrarily long without blocking hooks.
static CAPTURE_COMPLETED: hos_lock::Mailbox<CaptureCompleted, 1> = hos_lock::Mailbox::new();

#[derive(Clone, Copy)]
pub struct PlaybackStart {
    pub facing: f32,
    pub status: i32,
    pub len: usize,
    pub generation: u32,
}

fn notify(message: &str) {
    // R5: the queue copies the text into its fixed slot (no heap); the fork's Cow::Owned(to_owned) allocated once per notice in a hook.
    upsert("Named Macro", message, 180);
}

fn macros_dir() -> String {
    format!("{}/macros", crate::model::storage::ROOT)
}

fn library() -> Library {
    Library::new(macros_dir())
}

fn remember(id: &str) {
    let dir = macros_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(format!("{dir}/selected.txt"), id);
}

fn game_version() -> String {
    crate::runtime::compat::common::SMASH_VERSION.clone()
}

pub fn source_is_named() -> bool {
    read(&MENU).macro_source == MacroSource::NAMED
}

pub fn panel_visible() -> bool {
    !suspended() && (lock_read(&UI).is_open() || crate::runtime::macros::replay_capture::panel_visible()
        || crate::runtime::macros::replay_capture::browser_visible())
}

/// Worker-published answers: game callbacks never inspect the panel model.
static QUERIES: hos_lock::Published<crate::model::macro_handoff::MacroQueries> =
    hos_lock::Published::new(crate::model::macro_handoff::MacroQueries::IDLE);

pub fn hook_queries() -> crate::model::macro_handoff::MacroQueries {
    let q = QUERIES.read();
    if q.session == session_revision() { q } else { crate::model::macro_handoff::MacroQueries::IDLE }
}

/// The menu worker is the sole publisher. Publish after draining layer events and
/// after panel mutations, before servicing a potentially blocking native applet.
pub fn publish_queries() {
    RESTORE_ANCHORS.drain(|(generation, x, y)| {
        if generation == GENERATION.load(Ordering::Acquire) {
            lock_write(&UI).restored_target(generation, (x, y));
        }
    });
    let dropped = RESTORE_ANCHORS.take_dropped();
    if dropped != 0 { println!("[MacroHandoff] restore_events_dropped={dropped}"); }
    observe_loaded();
    let revision = selection_revision();
    let (playback_allowed, plays_current, start, restore_in_air, marker, placement, velocity, target) = {
        let ui = lock_read(&UI);
        if PLAYING.load(Ordering::Acquire) != 1 && PREPARED_REVISION.load(Ordering::Acquire) != revision {
            // SAFETY: the menu worker alone fills/retracts these buffers.
            unsafe {
                match ui.loaded_frames() {
                    Some(frames) => { PLAY_FRAMES.fill(frames); }
                    None => PLAY_FRAMES.retract(),
                }
            }
            PREPARED_REVISION.store(revision, Ordering::Release);
        }
        (ui.playback_allowed(), ui.plays_current(), ui.loaded_start(), ui.restore_in_air(),
            ui.marker(), ui.loaded_placement().map(|p| (p.delta_x, p.delta_y)),
            ui.ready_velocity(true), ui.loaded_target())
    };
    let loaded_matches = (crate::runtime::ADVANCED_RECORDING_WORKFLOWS && plays_current)
        || target.is_some_and(|target| exact_target_ready(target, cpu_kind()));
    let (browser_pending, browser_release) = crate::runtime::macros::replay_capture::browser_queries();
    QUERIES.publish(crate::model::macro_handoff::MacroQueries {
        session: session_revision(),
        source_is_named: source_is_named(),
        playback_allowed: playback_allowed && loaded_matches && PREPARED_REVISION.load(Ordering::Acquire) == revision,
        playback_len: PLAY_FRAMES.len() as u32,
        playback_facing: start.map_or(1.0, |start| start.facing),
        playback_status: start.map_or(0, |start| start.status),
        playback_generation: PLAY_FRAMES.generation(),
        plays_current, loaded_matches, placement, velocity,
        loaded_kind: start.map_or(-1, |start| start.fighter_kind),
        restore_in_air, panel_visible: panel_visible(),
        clip_foreground: clip_foreground(), naming_pending: naming_pending(),
        clip_editor_open: crate::runtime::macros::replay_capture::editor_open(),
        replay_loading: crate::runtime::macros::replay_capture::loading(), browser_pending, browser_release,
        suspended: suspended(), marker,
    });
}

pub fn panel_view() -> Option<View> {
    if suspended() { return None; }
    let ui = lock_read(&UI);
    ui.native_name_view().or_else(|| ui.view())
        .or_else(crate::runtime::macros::replay_capture::browser_view)
        .or_else(crate::runtime::macros::replay_capture::loading_view)
}

pub fn clip_foreground() -> bool {
    !suspended() && !lock_read(&UI).is_open() && crate::runtime::macros::replay_capture::panel_visible()
        && !naming_pending() && !crate::runtime::mii::native_keyboard::busy()
}

pub fn naming_pending() -> bool { NAME_JOB.lock().unwrap().is_some() }

fn request_native_name() {
    // Diag build: the software keyboard is a modal library applet; a headless Eden run opens it only when `diag-keyboard.on` exists in the data
    // folder (the keyboard row answers it through Eden's control API); otherwise the rows confirm the name with `diag_confirm_name`.
    #[cfg(feature = "diag")]
    if !std::path::Path::new("sd:/ultimate/TrainingRooms/diag-keyboard.on").exists() {
        return;
    }
    if suspended() || crate::runtime::mii::native_keyboard::busy() || naming_pending() { return; }
    let values = {
        let ui = lock_read(&UI);
        let title = ui.native_name_title().unwrap_or(MACRO_KEYBOARD_TITLE);
        ui.name_prompt().zip(ui.native_name_cursor()).map(|(name, cursor)| {
            (name.to_owned(), cursor, title)
        })
    };
    let Some((initial, cursor, title)) = values else { return };
    let id = NAME_SERIAL.fetch_add(1, Ordering::Relaxed);
    let job = NameJob { id, generation: SESSION_GENERATION.load(Ordering::Relaxed),
        replay: crate::runtime::macros::replay_capture::panel_visible(), waiting: crate::runtime::ui::facts::fact_waiting_scene(),
        phase: NamePhase::Active };
    *NAME_JOB.lock().unwrap() = Some(job);
    let result = if job.waiting { crate::runtime::mii::native_keyboard::request_inline(id, &initial, cursor) }
        else { crate::runtime::mii::native_keyboard::request(id, title, &initial) };
    if let Err(error) = result {
        NAME_JOB.lock().unwrap().take();
        notify(&error);
    }
}

/// Poll outside input mapping and without holding UI/game locks across the applet.
pub fn service_keyboard() {
    resume_name();
    crate::runtime::mii::native_keyboard::service(waiting_name_allowed);
    // Every Calc checkpoints text/caret without invoking confirm_native_name.
    // A new-lobby discard cannot publish a late callback into another draft.
    {
        let mut pending = NAME_JOB.lock().unwrap();
        if let Some(job) = pending.as_mut().filter(|job| job.waiting && job.phase != NamePhase::Discarding) {
            if let Some((text, cursor)) = crate::runtime::mii::native_keyboard::snapshot(job.id) {
                if !lock_write(&UI).checkpoint_native_name(&text, cursor) {
                    job.phase = NamePhase::Discarding;
                    crate::runtime::mii::native_keyboard::interrupt(job.id);
                }
            }
        }
    }
    let Some((id, result)) = crate::runtime::mii::native_keyboard::poll() else { return };
    // A stale completion may race a new scene's request. Never retire the newer
    // job just because the old applet finally returned.
    let ready = !suspended() && waiting_name_ready();
    let job = {
        let mut pending = NAME_JOB.lock().unwrap();
        let Some(job) = pending.as_mut().filter(|job| job.id == id) else { return };
        if job.phase == NamePhase::Discarding { pending.take(); return; }
        // Only an acknowledged, closed keyboard may become a value-only parked
        // request. An interruption racing the completion still wins over Enter.
        if job.waiting && !matches!(result, crate::runtime::mii::native_keyboard::KeyboardResult::Cancelled)
            && (job.phase == NamePhase::Closing || !ready
                || job.generation != SESSION_GENERATION.load(Ordering::Relaxed)
                || matches!(result, crate::runtime::mii::native_keyboard::KeyboardResult::Interrupted)) {
            job.phase = NamePhase::Parked;
            return;
        }
        pending.take()
    };
    let Some(job) = job else { return };
    crate::runtime::diag_log::diagnostic(format_args!("[NativeKeyboard] consume id={} replay={}", id, job.replay));
    if !job.waiting && (job.generation != SESSION_GENERATION.load(Ordering::Relaxed)
        || !crate::runtime::ui::facts::fact_practice_enabled() || suspended())
    { return; }
    // The closing applet's confirm/cancel must not become a gameplay input.
    crate::runtime::features::recorder::swallow_p1(15);
    match result {
        crate::runtime::mii::native_keyboard::KeyboardResult::Confirmed(name) => {
            let effect = lock_write(&UI).confirm_native_name(name);
            // Only a valid confirmation closes the clip. A failed write restores
            // the naming draft; the selected clip remains available until success.
            defer_effect(effect);
        }
        crate::runtime::mii::native_keyboard::KeyboardResult::Cancelled => {
            if job.waiting || job.replay { lock_write(&UI).on_pad(Pad::Back); }
        }
        crate::runtime::mii::native_keyboard::KeyboardResult::Failed(code) => {
            notify(&format!("System keyboard failed ({code:#X}); draft kept"));
        }
        crate::runtime::mii::native_keyboard::KeyboardResult::Interrupted => {}
    }
    assign(&menu::MENU_RECEIVED_INPUT, true);
    crate::runtime::diag_log::diagnostic(format_args!("[NativeKeyboard] applied id={}", id));
}

#[cfg(feature = "named-input-trace")]
pub fn playback_active() -> bool { PLAYING.load(Ordering::Acquire) != 0 }

pub fn init() {
    // R5: the panel-hold list is reserved at boot so hold_panel_fighter (a hook) never grows it (sync-01: +1 at the first library open).
    lock_write(&PANEL_STOPS).reserve(PANEL_STOPS_CAPACITY);
    let path = format!("{}/selected.txt", macros_dir());
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let id = text.trim();
    if id.is_empty() {
        return;
    }
    let Ok(recording) = library().load(id) else {
        return;
    };
    // A restored selection is not an explicit choice. It never forces a fighter;
    // the RC published gate waits for its recorded CPU before allowing playback.
    // No fighter exists at plugin init, so no native accessor is read here.
    let _ = commit_selected(id.to_owned(), recording, false, None);
}

pub fn on_pad(pad: Pad) {
    if naming_pending() || crate::runtime::mii::native_keyboard::busy() { return; }
    // The fighter question owns the pad until answered. An answer to a selection
    // or scene that has moved on is dropped, never applied.
    let prompt = lock_read(&UI).fighter_prompt();
    if let Some((origin, revision)) = prompt {
        if fighter_prompt_stale(origin, revision) {
            lock_write(&UI).drop_fighter_prompt();
            notify("Selection changed; choose again");
        } else {
            let effect = lock_write(&UI).on_pad(pad);
            // Only an answered library prompt commits its candidate. Cancel and
            // navigation leave the previous selection and selected.txt alone.
            let committed = lock_write(&UI).take_prompt_commit();
            if let Some((id, name)) = committed {
                begin_selection();
                remember(&id);
                notify(&format!("Loaded {name}"));
            }
            apply_effect(effect);
        }
        assign(&menu::MENU_RECEIVED_INPUT, true);
        return;
    }
    if lock_read(&UI).name_prompt().is_some() {
        match pad {
            Pad::Confirm => request_native_name(),
            Pad::Back => { lock_write(&UI).on_pad(Pad::Back); }
            Pad::Extra => {
                let effect = lock_write(&UI).native_name_placement();
                apply_effect(effect);
            }
            _ => {}
        }
        assign(&menu::MENU_RECEIVED_INPUT, true);
        return;
    }
    if crate::runtime::macros::replay_capture::loading() {
        if pad == Pad::Back { crate::runtime::macros::replay_capture::cancel_reload(); }
        return;
    }
    if crate::runtime::macros::replay_capture::browser_visible() {
        crate::runtime::macros::replay_capture::browser_input(pad);
        return;
    }
    if crate::runtime::macros::replay_capture::panel_visible() {
        crate::runtime::macros::replay_capture::on_pad(pad);
        return;
    }
    let effect = lock_write(&UI).on_pad(pad);
    apply_effect(effect);
    assign(&menu::MENU_RECEIVED_INPUT, true);
}

pub fn open_library(slot: Option<usize>) {
    if !practice_ready() {
        return;
    }
    let Some(slot) = slot else {
        notify("Choose a Record Slot first");
        return;
    };
    let listing = match library().list() {
        Ok(listing) => listing,
        Err(err) => {
            notify(&err.to_string());
            return;
        }
    };
    let skipped = listing.skipped.len();
    let rows = listing
        .entries
        .into_iter()
        .map(|entry| LibraryRow {
            id: entry.id,
            label: format!("{} · {}", entry.name, fighter_label(entry.fighter_kind)),
        })
        .collect();
    *lock_write(&RECORD_SLOT) = Some(slot);
    lock_write(&UI).open_library_titled(
        rows,
        skipped,
        "Named Library",
        Some(format!("Record Slot {}", slot + 1)),
    );
    assign(&menu::MENU_RECEIVED_INPUT, true);
}

pub fn export_record_slot(slot: Option<usize>) {
    if !practice_ready() {
        return;
    }
    let Some(slot) = slot else {
        notify("Choose a Record Slot first");
        return;
    };
    crate::runtime::macros::port::slots::export(slot);
}

pub fn slot_export_failed(message: &str) { notify(message); }

/// The requested slot copy has completed on the game thread.
pub fn slot_exported(frames: Vec<InputFrame>, start: StartState) {
    let version = game_version();
    let effect = lock_write(&UI).offer_save_with_mii(frames, start, false, &version, None);
    apply_effect(effect);
    request_native_name();
    if panel_visible() {
        assign(&menu::MENU_RECEIVED_INPUT, true);
    }
}

pub fn offer_external_name(token: u64, default_name: &str, summary: &str) {
    offer_external_name_titled(token, SETUP_KEYBOARD_TITLE, default_name, summary);
}

/// A caller-owned name under its own keyboard header (the combo library's "Name combo", SPEC-input-07 §5).
pub fn offer_external_name_titled(token: u64, title: &'static str, default_name: &str, summary: &str) {
    let effect = lock_write(&UI).offer_name(NameContext { token, title }, default_name, summary);
    apply_effect(effect);
    request_native_name();
    if panel_visible() {
        assign(&menu::MENU_RECEIVED_INPUT, true);
    }
}

pub fn open_external_library(
    rows: Vec<LibraryRow>,
    skipped: usize,
    title: impl Into<String>,
    alternate: Option<String>,
) {
    if !practice_ready() {
        return;
    }
    lock_write(&UI).open_library_forward(rows, skipped, title, alternate);
    assign(&menu::MENU_RECEIVED_INPUT, true);
}

pub fn open_editor() {
    if !crate::runtime::ADVANCED_RECORDING_WORKFLOWS {
        notify("Placement editing is unavailable in this release");
        return;
    }
    if !practice_ready() {
        return;
    }
    if !lock_write(&UI).open_editor(None) {
        notify("Load a macro and restore a Training state first");
        return;
    }
    assign(&menu::MENU_RECEIVED_INPUT, true);
}

fn practice_ready() -> bool {
    if !crate::runtime::ui::facts::fact_practice_enabled() || !crate::runtime::ui::facts::fact_fighters_present() {
        notify("Wait for both practice fighters");
        return false;
    }
    if crate::runtime::lobby::training::recording_or_playback()
        || crate::runtime::macros::save_states::is_loading()
    {
        notify("Finish the current practice action first");
        return false;
    }
    true
}

pub unsafe fn snapshot_start(module_accessor: &mut BattleObjectModuleAccessor) -> StartState {
    StartState {
        fighter_kind: utility::get_kind(module_accessor),
        costume: WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) as u32,
        facing: PostureModule::lr(module_accessor),
        situation: StatusModule::situation_kind(module_accessor),
        status: StatusModule::status_kind(module_accessor),
        speed_x: KineticModule::get_sum_speed_x(
            module_accessor,
            *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN,
        ),
        speed_y: KineticModule::get_sum_speed_y(
            module_accessor,
            *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN,
        ),
    }
}

pub fn begin_capture(start: StartState) -> bool {
    if !CAPTURE.begin() { return false; }
    PLAYING.store(0, Ordering::Release);
    *CAPTURE_ORIGIN.lock() = start;
    true
}

pub unsafe fn capture_start(module_accessor: &mut BattleObjectModuleAccessor) -> StartState {
    let start = snapshot_start(module_accessor);
    *CAPTURE_ORIGIN.lock() = start;
    start
}

pub fn store_frame(index: usize, input: &MappedInputs) {
    if index >= crate::engine::macros::MAX_FRAMES {
        return;
    }
    let frame = InputFrame {
        buttons: input.buttons,
        lstick_x: input.lstick_x,
        lstick_y: input.lstick_y,
        rstick_x: input.rstick_x,
        rstick_y: input.rstick_y,
    };
    let _ = CAPTURE.store(index, frame);
}

pub fn finish_capture(crop: bool) {
    let start = *CAPTURE_ORIGIN.lock();
    if CAPTURE.finish().is_none() { return; }
    // No second completion can be posted until the worker takes CAPTURE.
    let accepted = CAPTURE_COMPLETED.post(CaptureCompleted {
        start, crop, session: SESSION_GENERATION.load(Ordering::Acquire),
    });
    debug_assert!(accepted);
    crate::runtime::ui::worker::wake();
}

/// Worker only: copy the completed recording before releasing its capture buffer,
/// then let the panel own the draft and request the native keyboard.
pub fn consume_capture() {
    CAPTURE_COMPLETED.drain(|done| {
        let current = done.session == SESSION_GENERATION.load(Ordering::Acquire);
        let frames = CAPTURE.take(|frames| current.then(|| frames.to_vec())).flatten();
        let Some(frames) = frames else { return; };
        let version = game_version();
        crate::runtime::diag_log::diagnostic(format_args!(
            "[PracticeRecording] named_recording_complete captured_frames={} crop={}", frames.len(), done.crop
        ));
        let effect = lock_write(&UI).offer_save(frames, done.start, done.crop, &version);
        apply_effect(effect);
        request_native_name();
        if panel_visible() { assign(&menu::MENU_RECEIVED_INPUT, true); }
    });
}

pub fn begin_playback() -> Option<PlaybackStart> {
    let q = hook_queries();
    if !q.playback_allowed || q.playback_len == 0 || PLAY_FRAMES.generation() != q.playback_generation { return None; }
    CAPTURE.cancel();
    PLAY_GENERATION.store(q.playback_generation, Ordering::Release);
    PLAYING.store(1, Ordering::Release);
    let info = PlaybackStart { facing: q.playback_facing, status: q.playback_status,
        len: q.playback_len as usize, generation: q.playback_generation };
    trace_begin(info.len, false);
    Some(info)
}

/// Worker only: prepare a transient preview without changing the named selection.
pub fn prepare_preview(recording: &MacroRecording) -> PlaybackStart {
    // SAFETY: request_play runs on the menu worker, the sole preview-buffer writer.
    let generation = unsafe { PREVIEW_FRAMES.fill(&recording.frames) };
    PlaybackStart { facing: recording.start.facing, status: recording.start.status,
        len: recording.frames.len(), generation }
}

/// Game callback only: the requested preview is still the prepared one.
pub fn begin_preview(info: PlaybackStart) -> bool {
    if PREVIEW_FRAMES.generation() != info.generation { return false; }
    CAPTURE.cancel();
    PLAY_GENERATION.store(info.generation, Ordering::Release);
    PLAYING.store(2, Ordering::Release);
    PLAYBACK_START_WAITS.store(0, Ordering::Relaxed);
    PLAYBACK_START_PENDING.store(true, Ordering::Relaxed);
    trace_begin(info.len, true);
    true
}

fn trace_begin(_len: usize, _preview: bool) {
    #[cfg(feature = "named-input-trace")]
    {
        TRACE_WRITES.store(0, Ordering::Relaxed);
        TRACE_OBSERVATIONS.store(0, Ordering::Relaxed);
        let run = TRACE_RUN.fetch_add(1, Ordering::Relaxed) + 1;
        println!("[NamedTrace] begin run={} len={} preview={}", run, _len, _preview);
    }
}

/// Qualification only: the native getter observation is independent of this write.
/// Never use the persistent diagnostic logger for per-frame samples.
#[cfg(feature = "named-input-trace")]
pub fn trace_write(index: usize, input: &MappedInputs, buffered: bool, mirror: f32) {
    if TRACE_WRITES.load(Ordering::Relaxed) >= crate::engine::macros::MAX_FRAMES as u64 { return; }
    let seq = TRACE_WRITES.fetch_add(1, Ordering::Relaxed) + 1;
    println!(
        "[NamedTrace] w run={} seq={} idx={} buttons={:#x} lx={} ly={} rx={} ry={} buffered={} mirror={}",
        TRACE_RUN.load(Ordering::Relaxed), seq, index, input.buttons.bits(),
        input.lstick_x, input.lstick_y, input.rstick_x, input.rstick_y,
        u8::from(buffered), mirror
    );
}

pub fn end_playback() {
    PLAYBACK_OBSERVATIONS.store(0, Ordering::Relaxed);
    PLAYING.store(0, Ordering::Release);
    crate::runtime::ui::worker::wake();
}

pub fn cancel_motion() {
    CAPTURE.cancel();
    PLAYBACK_START_PENDING.store(false, Ordering::Relaxed);
    PLAYBACK_START_WAITS.store(0, Ordering::Relaxed);
    RESTORE_POSITION_WAITS.store(0, Ordering::Relaxed);
    PLAYBACK_OBSERVATIONS.store(0, Ordering::Relaxed);
    PLAYING.store(0, Ordering::Release);
    crate::runtime::ui::worker::wake();
}

/// Close the panel and stop playback. The loaded recording stays, because a
/// character install calls session cleanup as part of the swap this macro asked for.
pub fn clear_session() {
    // Game-owned input guard must not survive scene cleanup.
    crate::runtime::macros::replay_capture::owns_close_input(false, false);
    GENERATION.fetch_add(1, Ordering::Relaxed);
    SESSION_GENERATION.fetch_add(1, Ordering::Relaxed);
    crate::runtime::macros::replay_preview::clear_session();
    post_lifecycle(Lifecycle::Clear { keep: suspended() });
    cancel_motion();
    RESTORING.store(false, Ordering::Relaxed);
    RESTORE_ACTION.store(0, Ordering::Relaxed);
    *lock_write(&RESTORE_POSE) = None;
}

pub fn begin_restore() {
    *lock_write(&RESTORE_POSE) = None;
    PLAYBACK_START_PENDING.store(false, Ordering::Relaxed);
    RESTORE_POSITION_WAITS.store(0, Ordering::Relaxed);
    GENERATION.fetch_add(1, Ordering::Relaxed);
    RESTORING.store(true, Ordering::Relaxed);
    if crate::runtime::macros::replay_preview::occupied() {
        RESTORE_ACTION.store(0, Ordering::Relaxed);
    } else if source_is_named() && RESTORE_ACTION.load(Ordering::Relaxed) == 0 {
        let settings = crate::runtime::features::recorder::settings();
        let action = if settings.trigger_save_state {
            1
        } else if !settings.save_state_slots.is_empty() {
            2
        } else { 0 };
        RESTORE_ACTION.store(action, Ordering::Relaxed);
    }
}

pub unsafe fn request_playback() {
    if !hook_queries().playback_allowed {
        notify("Load a named macro and wait for its fighter");
        return;
    }
    super::input_record::stop_playback();
    match super::save_states::request_macro_restore() {
        Ok(()) => RESTORE_ACTION.store(2, Ordering::Relaxed),
        Err(message) => notify(message),
    }
}

/// Run after the CPU's restore callback. No inputs start while either fighter
/// still has charge/buff/character restoration work pending.
pub unsafe fn service_restore() {
    if !RESTORING.load(Ordering::Relaxed) || super::save_states::is_loading() {
        return;
    }
    RESTORING.store(false, Ordering::Relaxed);
    if crate::runtime::macros::replay_preview::occupied() {
        RESTORE_ACTION.store(0, Ordering::Relaxed);
        // Native preview starts after this callback and still needs this target
        // at input zero, after BOTH fighters have completed restoration.
        return;
    }
    match RESTORE_ACTION.swap(0, Ordering::Relaxed) {
        1 => super::input_record::start_named_lockout(),
        2 => {
            if super::input_record::playback_named() {
                // The CPU may have finished restoring before the player. Reapply
                // its saved target and speed at the actual input-0 boundary.
                PLAYBACK_START_WAITS.store(0, Ordering::Relaxed);
                PLAYBACK_START_PENDING.store(true, Ordering::Relaxed);
                return;
            }
        }
        _ => {}
    }
    *lock_write(&RESTORE_POSE) = None;
}

/// Remember values, not a live-position anchor. The accessor identity prevents a
/// pending restore from being applied to a replacement fighter.
pub fn remember_restore_pose(
    module_accessor: &mut BattleObjectModuleAccessor,
    x: f32,
    y: f32,
    facing: f32,
    air: Option<bool>,
) {
    if crate::runtime::macros::replay_preview::start_context().is_some()
        || (source_is_named() && loaded_fighter_matches() && !crate::runtime::macros::replay_preview::occupied()) {
        *lock_write(&RESTORE_POSE) = Some(RestorePose {
            object: module_accessor.battle_object_id,
            address: module_accessor as *mut BattleObjectModuleAccessor as usize,
            x,
            y,
            facing,
            air,
        });
    }
}

/// Called immediately before named input 0. A status transition may need another
/// callback; until then no input is emitted and the recorder clock stays at zero.
pub unsafe fn prepare_playback_start(module_accessor: &mut BattleObjectModuleAccessor) -> bool {
    if !PLAYBACK_START_PENDING.load(Ordering::Relaxed) {
        return true;
    }
    if !crate::runtime::macros::replay_preview::occupied() && source_is_named() && !loaded_fighter_matches() {
        super::input_record::stop_playback();
        notify("Wait for the exact macro fighter before playback");
        return false;
    }
    let pose = *lock_read(&RESTORE_POSE);
    let Some(pose) = pose.filter(|pose| {
        pose.object == module_accessor.battle_object_id
            && pose.address == module_accessor as *mut BattleObjectModuleAccessor as usize
    }) else {
        PLAYBACK_START_PENDING.store(false, Ordering::Relaxed);
        super::input_record::stop_playback();
        notify("Restore the Training setup again before playback");
        return false;
    };
    let Some(air) = pose.air else {
        super::input_record::stop_playback();
        notify("Use an idle/fall saved setup for named playback");
        return false;
    };
    KineticModule::clear_speed_all(module_accessor);
    PostureModule::set_pos(module_accessor, &Vector3f { x: pose.x, y: pose.y, z: 0.0 });
    PostureModule::set_lr(module_accessor, pose.facing);
    let target_status = if air { *FIGHTER_STATUS_KIND_FALL } else { *FIGHTER_STATUS_KIND_WAIT };
    if StatusModule::status_kind(module_accessor) != target_status {
        let waits = PLAYBACK_START_WAITS.fetch_add(1, Ordering::Relaxed);
        if waits == 0 {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[PracticeRecording] named_playback_wait status={} target={}",
                StatusModule::status_kind(module_accessor), target_status
            ));
        }
        if waits >= 60 {
            super::input_record::stop_playback();
            notify("Start position could not settle; adjust placement and retry");
            return false;
        }
        StatusModule::change_status_request(module_accessor, target_status, false);
        return false;
    }
    crate::runtime::features::recorder::set_current_facing(pose.facing);
    apply_ready_speed(module_accessor, air);
    PLAYBACK_START_PENDING.store(false, Ordering::Relaxed);
    PLAYBACK_OBSERVATIONS.store(PLAYBACK_OBSERVATION_LIMIT, Ordering::Relaxed);
    *lock_write(&RESTORE_POSE) = None;
    let start = snapshot_start(module_accessor);
    crate::runtime::diag_log::diagnostic(format_args!(
        "[PracticeRecording] named_playback_start x={} y={} facing={} situation={} status={} speed_x={} speed_y={}",
        PostureModule::pos_x(module_accessor), PostureModule::pos_y(module_accessor),
        start.facing, start.situation, start.status, start.speed_x, start.speed_y
    ));
    true
}

unsafe fn observation_fighter(module_accessor: &mut BattleObjectModuleAccessor) -> bool {
    crate::runtime::features::recorder::is_primary_cpu_body(module_accessor)
}

/// Pair the pre-original boundary with the post-original observations below.
/// This measures input/state ordering rather than inferring it from clock names.
pub unsafe fn observe_playback_before(module_accessor: &mut BattleObjectModuleAccessor) {
    if PLAYBACK_OBSERVATIONS.load(Ordering::Relaxed) == 0
        && !(PLAYBACK_START_PENDING.load(Ordering::Relaxed)
            && PLAYBACK_START_WAITS.load(Ordering::Relaxed) == 0)
    { return; }
    if !observation_fighter(module_accessor) { return; }
    let start = snapshot_start(module_accessor);
    crate::runtime::diag_log::diagnostic(format_args!(
        "[PracticeRecording] named_playback_before next_input={} x={} y={} status={} speed_x={} speed_y={}",
        crate::runtime::features::recorder::frame(), PostureModule::pos_x(module_accessor),
        PostureModule::pos_y(module_accessor), start.status, start.speed_x, start.speed_y
    ));
}

/// Eight primary-CPU post-fighter observations per restored start, bounded by the
/// diagnostics logger. These distinguish applied energy from actual later motion.
pub unsafe fn observe_playback_start(module_accessor: &mut BattleObjectModuleAccessor) {
    #[cfg(feature = "named-input-trace")]
    if playback_active() && observation_fighter(module_accessor)
        && TRACE_WRITES.load(Ordering::Relaxed) > 0
        && TRACE_OBSERVATIONS.load(Ordering::Relaxed) < crate::engine::macros::MAX_FRAMES as u64
    {
        let sample = TRACE_OBSERVATIONS.fetch_add(1, Ordering::Relaxed) + 1;
        println!(
            "[NamedTrace] o run={} seq={} sample={} next={} button={:#x} sx={} sy={} ncx={} ncy={} subx={} suby={}",
            TRACE_RUN.load(Ordering::Relaxed), TRACE_WRITES.load(Ordering::Relaxed), sample,
            crate::runtime::features::recorder::frame(), ControlModule::get_button(module_accessor),
            ControlModule::get_stick_x(module_accessor), ControlModule::get_stick_y(module_accessor),
            ControlModule::get_stick_x_no_clamp(module_accessor), ControlModule::get_stick_y_no_clamp(module_accessor),
            ControlModule::get_sub_stick_x(module_accessor), ControlModule::get_sub_stick_y(module_accessor)
        );
    }
    let remaining = PLAYBACK_OBSERVATIONS.load(Ordering::Relaxed);
    if remaining == 0 || !observation_fighter(module_accessor) { return; }
    PLAYBACK_OBSERVATIONS.store(remaining - 1, Ordering::Relaxed);
    let start = snapshot_start(module_accessor);
    crate::runtime::diag_log::diagnostic(format_args!(
        "[PracticeRecording] named_playback_observed sample={} next_input={} x={} y={} situation={} status={} speed_x={} speed_y={} buttons={} raw_x={} raw_y={}",
        PLAYBACK_OBSERVATION_LIMIT + 1 - remaining, crate::runtime::features::recorder::frame(),
        PostureModule::pos_x(module_accessor), PostureModule::pos_y(module_accessor),
        start.situation, start.status, start.speed_x, start.speed_y,
        ControlModule::get_button(module_accessor), ControlModule::get_stick_x_no_clamp(module_accessor),
        ControlModule::get_stick_y_no_clamp(module_accessor)
    ));
}

pub fn restored_target(x: f32, y: f32) -> Option<(f32, f32)> {
    if crate::runtime::macros::replay_preview::occupied() { return None; }
    let q = hook_queries();
    if q.panel_visible || !q.source_is_named || !q.loaded_matches { return None; }
    let generation = GENERATION.load(Ordering::Relaxed);
    if generation == 0 { return None; }
    let (dx, dy) = q.placement?;
    let mut anchor = lock_write(&RESTORE_ANCHOR);
    if anchor.is_none_or(|(saved, _, _)| saved != generation) {
        *anchor = Some((generation, x, y));
        RESTORE_ANCHORS.post((generation, x, y));
        crate::runtime::ui::worker::wake();
    }
    let (_, x, y) = (*anchor)?;
    Some((x + dx, y + dy))
}

/// The recorded-identity gate only. A macro the user chose to play on the current
/// CPU has no recorded identity to match; ownership, scope, readiness and restore
/// guards at every caller still apply.
fn loaded_fighter_matches() -> bool {
    hook_queries().loaded_matches
}

/// Recorded aerial motion must survive even at zero vertical offset.
pub fn restore_in_air() -> bool {
    if crate::runtime::macros::replay_preview::occupied() {
        return crate::runtime::macros::replay_preview::start_context().is_some_and(|start| start.situation == *SITUATION_KIND_AIR);
    }
    source_is_named() && loaded_fighter_matches()
        && hook_queries().restore_in_air
}

pub fn offer_replay_recording(recording: MacroRecording) {
    if let Err(error) = recording.validate() {
        notify(&error.to_string());
        return;
    }
    lock_write(&UI).restore_draft(recording);
    assign(&menu::MENU_RECEIVED_INPUT, true);
    request_native_name();
}

unsafe fn apply_ready_speed(
    module_accessor: &mut BattleObjectModuleAccessor,
    air: bool,
) {
    let (x, y, facing) = if let Some(start) = crate::runtime::macros::replay_preview::start_context() {
        (start.speed_x, start.speed_y, start.facing)
    } else {
        if crate::runtime::macros::replay_preview::occupied() || !source_is_named() || hook_queries().panel_visible || !loaded_fighter_matches() {
            return;
        }
        let q = hook_queries();
        let Some((x, y)) = q.velocity else { return; };
        (x, y, q.playback_facing)
    };
    // Match the recorder's facing-relative input transform; placement remains a
    // world-unit delta from the already mirrored save-state anchor.
    let x = x * facing * PostureModule::lr(module_accessor);
    if air {
        set_normal_speed(module_accessor, *FIGHTER_KINETIC_ENERGY_ID_CONTROL, x, 0.0);
        set_gravity_speed(module_accessor, y);
    } else {
        set_normal_speed(module_accessor, *FIGHTER_KINETIC_ENERGY_ID_MOTION, x, 0.0);
    }
}

unsafe fn set_normal_speed(
    module_accessor: &mut BattleObjectModuleAccessor,
    energy_id: i32,
    x: f32,
    y: f32,
) {
    let energy = KineticModule::get_energy(module_accessor, energy_id)
        as *mut smash::app::KineticEnergyNormal;
    if energy.is_null() {
        return;
    }
    let speed = Vector2f { x, y };
    KineticEnergyNormal::set_speed(energy, &speed);
}

unsafe fn set_gravity_speed(module_accessor: &mut BattleObjectModuleAccessor, y: f32) {
    let energy = KineticModule::get_energy(module_accessor, *FIGHTER_KINETIC_ENERGY_ID_GRAVITY)
        as *mut smash::app::FighterKineticEnergyGravity;
    if energy.is_null() {
        return;
    }
    FighterKineticEnergyGravity::set_speed(energy, y);
}

pub fn service_character() {
    consume_character_result();
    retire_stale_fighter_prompt();
    let named=source_is_named();
    if !named && NAMED_SOURCE_ACTIVE.swap(false,Ordering::AcqRel) {
        cancel_character_selection();
    } else if named { NAMED_SOURCE_ACTIVE.store(true,Ordering::Release); }
    // Persisted selection exists before the stage is playable. Native Training
    // mode and an observed entry alone also hold during the initial load screen.
    if !crate::runtime::ui::facts::fact_practice_enabled() || suspended() || !crate::runtime::ui::facts::fact_ready_go()
        || panel_visible() || crate::runtime::macros::replay_capture::loading() || !source_is_named()
    {
        return;
    }
    if crate::runtime::lobby::training::recording_or_playback()
        || crate::runtime::macros::save_states::is_loading()
    {
        return;
    }
    if lock_read(&UI).plays_current() {
        // The recorded identity is bypassed: no refusal, re-acknowledgement or
        // character request. Only the settled current CPU gates playback.
        observe_loaded();
        return;
    }
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    {
        let target=lock_read(&UI).loaded_target();
        if let Some(reason)=target.and_then(crate::runtime::macros::identity::named_refusal) {
            let revision=selection_revision();
            if NAMED_REFUSAL_SHOWN.swap(revision,Ordering::AcqRel)!=revision {notify(reason);}
            return;
        }
    }
    observe_loaded();
    if crate::runtime::roster::character_roster::busy() {return;}
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    {
        let target=lock_read(&UI).loaded_target();
        if let Some(target)=target {
            crate::runtime::macros::identity::reacknowledge_named(target);
        }
    }
    submit_loaded_character();
}

pub unsafe fn preview(module_accessor: &mut BattleObjectModuleAccessor) {
    let Some((x, y)) = hook_queries().marker else {
        return;
    };
    draw_cross(module_accessor, x, y);
}

pub unsafe fn hold_panel_fighter(module_accessor: &mut BattleObjectModuleAccessor) {
    let id = module_accessor.battle_object_id;
    let pointer = module_accessor as *mut BattleObjectModuleAccessor as usize;
    let mut owned = lock_write(&PANEL_STOPS);
    let found = owned.iter().position(|&(object, address, _)| object == id && address == pointer);
    // Keep P1 held while the editor is parked for a CPU reload. The new CPU must
    // remain free to finish its native install and land before preview acquisition.
    let hold_panel = hook_queries().panel_visible || (hook_queries().replay_loading
        && WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)
            == FighterId::Player as i32);
    let entry = if hold_panel && !crate::runtime::macros::replay_preview::needs_native_update(module_accessor)
        && !crate::runtime::macros::replay_capture::browser_needs_native_update(module_accessor) {
        practice::panel_entry(module_accessor)
    } else { None };
    if let Some(entry) = entry {
        if found.is_none() && !StopModule::is_special_stop(module_accessor) {
            StopModule::set_special_stop(module_accessor, true);
            owned.retain(|&(object, _, _)| object != id);
            owned.push((id, pointer, entry));
        }
    } else if let Some(index) = found {
        StopModule::set_special_stop(module_accessor, false);
        owned.remove(index);
    }
}

unsafe fn draw_cross(module_accessor: &mut BattleObjectModuleAccessor, x: f32, y: f32) {
    // Same spark the fast-fall cue uses. Killing the kind also clears other
    // flashes of this effect while the placement editor is open.
    EffectModule::kill_kind(
        module_accessor,
        Hash40::new("sys_smash_flash_s"),
        false,
        true,
    );
    let rotation = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    for (dx, dy) in [(0.0, 0.0), (2.5, 0.0), (-2.5, 0.0), (0.0, 2.5), (0.0, -2.5)] {
        let pos = Vector3f {
            x: x + dx,
            y: y + dy,
            z: 0.0,
        };
        EffectModule::req(
            module_accessor,
            Hash40::new("sys_smash_flash_s"),
            &pos,
            &rotation,
            0.45,
            0,
            0,
            true,
            *EFFECT_SUB_ATTRIBUTE_CONCLUDE_STATUS,
        );
    }
}

/// The effect of a confirmed keyboard name, waiting for the menu worker (one at a time: a second keyboard cannot open before the first
/// was applied). Handing it over allocates nothing.
static DEFERRED: std::sync::Mutex<Option<Effect>> = std::sync::Mutex::new(None);

fn defer_effect(effect: Effect) {
    *DEFERRED.lock().unwrap() = Some(effect);
    crate::runtime::ui::worker::wake();
}

/// Apply the deferred effect, on the menu worker (the library write and the loads are file I/O).
pub fn run_deferred() {
    let effect = DEFERRED.lock().unwrap().take();
    if let Some(effect) = effect {
        apply_effect(effect);
    }
}

fn apply_effect(effect: Effect) {
    match effect {
        Effect::None => {}
        Effect::Notify(text) => notify(&text),
        Effect::Save(recording) => save_new(*recording),
        Effect::Overwrite { id, recording } => overwrite(id, *recording),
        Effect::Choose(id) => load_id(&id),
        Effect::BindToSlot { id } => bind_to_slot(&id),
        Effect::Alternate { id } => *lock_write(&LIST_ACTION) = Some(id),
        Effect::Rename { id } => *lock_write(&LIST_RENAME) = Some(id),
        Effect::ForwardChoose { id } => *lock_write(&LIST_CHOICE) = Some(id),
        Effect::NameConfirmed { context, name } => {
            *lock_write(&CONFIRMED_NAME) = Some(NameConfirmation { context, name });
        }
        Effect::NeedCharacter(kind) => request_kind(kind),
        Effect::ClipFighter(choice) => crate::runtime::macros::replay_capture::fighter_choice(choice),
    }
}

fn bind_to_slot(id: &str) {
    if !practice_ready() {
        return;
    }
    let Some(slot) = *lock_read(&RECORD_SLOT) else {
        notify("Choose a Record Slot first");
        return;
    };
    let recording = match library().load(id) {
        Ok(recording) => recording,
        Err(err) => {
            notify(&err.to_string());
            return;
        }
    };
    if let Err(error) = crate::runtime::macros::port::slots::import(slot, &recording) {
        notify(&error);
    }
}

pub fn slot_imported(name: &str, slot: usize, result: Result<(), crate::model::recording::ImportRefusal>) {
    match result {
        Ok(()) => notify(&format!("Copied {name} to Record Slot {}", slot + 1)),
        Err(error) => notify(&error.message()),
    }
}

fn save_new(recording: MacroRecording) {
    match library().save_new(&recording) {
        Ok(id) => {
            crate::runtime::macros::replay_capture::close_editor();
            remember(&id);
            let name = recording.name.clone();
            // The clip's own fighter choice already stands; do not ask again.
            let result = commit_selected(id, recording, false, cpu_kind());
            match result {
                Ok(effect) => {
                    notify(&format!("Saved {name}"));
                    apply_effect(effect);
                }
                Err(err) => notify(&err.to_string()),
            }
        }
        Err(err) => {
            let text = err.to_string();
            lock_write(&UI).restore_draft(recording);
            assign(&menu::MENU_RECEIVED_INPUT, true);
            notify(&text);
        }
    }
}

fn overwrite(id: String, recording: MacroRecording) {
    match library().overwrite(&id, &recording) {
        Ok(()) => notify("Placement written"),
        Err(err) => notify(&err.to_string()),
    }
}

fn load_id(id: &str) {
    match library().load(id) {
        Ok(recording) => {
            let name = recording.name.clone();
            match commit_selected(id.to_owned(), recording, true, cpu_kind()) {
                Ok(effect) => {
                    // A fighter question leaves the previous selection, and
                    // selected.txt, in place until it is answered.
                    if lock_read(&UI).fighter_prompt().is_none() {
                        remember(id);
                        notify(&format!("Loaded {name}"));
                    }
                    apply_effect(effect);
                }
                Err(err) => notify(&err.to_string()),
            }
        }
        Err(err) => notify(&err.to_string()),
    }
}

fn request_kind(kind: i32) {
    if kind >= 0 {submit_loaded_character();}
}

fn cpu_kind() -> Option<i32> {
    crate::runtime::ui::facts::fact_cpu_kind()
}

pub(crate) fn fighter_label(kind: i32) -> String {
    if kind < 0 {
        return "CPU".to_owned();
    }
    crate::engine::swap::trio::TrioFighter::from_kind(kind as u32)
        .map(|fighter| fighter.name().to_owned())
        .unwrap_or_else(|| format!("0x{kind:X}"))
}

/// SPEC-input-01 section 9, the core's per-frame read (`has_captured_frame` and `frame_at` in one short lock, no allocation): the frame the run
/// reads at `index` from the buffer in use, `None` while a capture has not stored it yet.
pub fn frame_for(index: usize) -> Option<MappedInputs> {
    let source = PLAYING.load(Ordering::Acquire);
    let frame = if source == 0 {
        CAPTURE.frame_at(index)
    } else {
        let buffers = if source == 1 { &PLAY_FRAMES } else { &PREVIEW_FRAMES };
        let generation = PLAY_GENERATION.load(Ordering::Acquire);
        if buffers.generation() != generation { return None; }
        let frame = buffers.frame_at(index);
        if buffers.generation() != generation { return None; }
        frame
    }?;
    Some(MappedInputs { buttons: frame.buttons, lstick_x: frame.lstick_x, lstick_y: frame.lstick_y,
        rstick_x: frame.rstick_x, rstick_y: frame.rstick_y })
}

/// Publishing/boot reservation only; the capture owner atomically excludes an
/// active capture. Playback buffers are filled exclusively by the menu worker.
pub fn prepare_buffers() {
    CAPTURE.prepare(crate::engine::macros::MAX_FRAMES);
}

/// Diag build only: the keyboard's confirmed-name path without the applet (`service_keyboard` does exactly this with the typed text).
#[cfg(feature = "diag")]
pub fn diag_confirm_name(name: &str) {
    let effect = lock_write(&UI).confirm_native_name(name.to_owned());
    apply_effect(effect);
}

/// Diag build only: `export_record_slot` without the keyboard request (see `request_native_name`).
#[cfg(feature = "diag")]
pub fn diag_export_slot(slot: usize) {
    if practice_ready() {
        crate::runtime::macros::port::slots::export_diag(slot);
    } else {
        crate::runtime::macros::diag::slot_export_complete(false);
    }
}
