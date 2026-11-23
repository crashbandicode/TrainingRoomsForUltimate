//! Named-macro runtime.
//!
//! The in-game panel, character requests and replay-capture glue.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use smash::app::{lua_bind::*, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
use smash::phx::{Hash40, Vector3f};
use {crate::engine::macros::Library, crate::engine::macros::MacroRecording};
use crate::runtime::macros::port::sync::*;
use crate::engine::macro_ui::macro_panel::{
    Effect, FighterChoice, FighterGate, FighterOrigin, LibraryRow, LoadContext, MacroUi,
    NameContext, Pad, View, MACRO_KEYBOARD_TITLE, SETUP_KEYBOARD_TITLE,
};

use crate::runtime::macros::port::consts::{FighterId, MacroSource, OnOff, MENU};
use crate::runtime::compat::common::menu;
use super::practice;
use crate::runtime::lobby::training::ui::notifications::upsert;

static UI: RwLock<MacroUi> = RwLock::new(MacroUi::new());
static SUSPENDED: AtomicBool = AtomicBool::new(false);
static CONFIRMED_NAME: RwLock<Option<NameConfirmation>> = RwLock::new(None);
static LIST_ACTION: RwLock<Option<String>> = RwLock::new(None);

/// Keyboard result for a draft the caller still owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameConfirmation {
    pub context: NameContext,
    pub name: String,
}

pub fn take_confirmed_name() -> Option<NameConfirmation> {
    lock_write(&CONFIRMED_NAME).take()
}

/// The confirmed name only when this caller offered the keyboard. Another owner's answer stays put.
pub fn take_confirmed_name_for(token: u64) -> Option<String> {
    let mut slot = lock_write(&CONFIRMED_NAME);
    if slot.as_ref().is_some_and(|found| found.context.token == token) {
        slot.take().map(|found| found.name)
    } else {
        None
    }
}

/// Y on a library opened through [`open_external_library`].
pub fn take_list_action() -> Option<String> {
    lock_write(&LIST_ACTION).take()
}

static LIST_CHOICE: RwLock<Option<String>> = RwLock::new(None);
/// The answer to a combo's fighter question (`None` inside: Cancel), for the combos UI (SPEC-input-07 §4a).
static COMBO_FIGHTER: RwLock<Option<Option<FighterChoice>>> = RwLock::new(None);
static RECORDING_FIGHTER: RwLock<Option<Option<FighterChoice>>> = RwLock::new(None);
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

/// The answer to the combo fighter question, once (`Some(None)`: cancelled).
pub fn take_combo_fighter_choice() -> Option<Option<FighterChoice>> {
    lock_write(&COMBO_FIGHTER).take()
}

/// The answer to a recording's fighter question, once (`Some(None)`: cancelled).
pub fn take_recording_fighter_choice() -> Option<Option<FighterChoice>> {
    lock_write(&RECORDING_FIGHTER).take()
}

/// Open the shared Fighter Mismatch screen for a recording, a combo drill, or a chain's macro 1, played by `side`.
pub fn open_recording_fighter_prompt(origin: FighterOrigin, side: crate::model::recording_menu::ViewRole, recorded: String, current: String) -> bool {
    *lock_write(&RECORDING_FIGHTER) = None;
    let opened = lock_write(&UI).open_fighter_prompt_for(origin, side, 0, recorded, current);
    if opened { assign(&menu::MENU_RECEIVED_INPUT, true); }
    opened
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
/// never ask, and a mismatch there keeps the current CPU.
fn commit_selected(id:String, recording:MacroRecording, prompt:bool, current:Option<i32>) -> Result<Effect,crate::engine::macros::MacroError> {
    let context=load_context(&recording,prompt,current);
    // A pending question changes nothing yet: the old selection's revision, and
    // any exact-Mii receipt tied to it, stay valid until the user answers.
    if context.gate(&recording)!=FighterGate::Prompt {begin_selection();}
    lock_write(&UI).offer_load(id,recording,context)
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
        can_switch:target.is_some(),
        revision:selection_revision(),
        recorded_label:if target.is_some_and(|target|target.configuration().is_some()) {format!("{recorded} (recorded Mii)")} else {recorded},
        current_label:current.map_or_else(||"CPU".to_owned(),fighter_label),
    }
}
/// Compare identities, not a previous loader request's acknowledgement. A
/// manually selected exact Mii is already a match even without a replay receipt.
/// #54, reading B (primary 2026-11-08 23:03Z): exactly, against the target as Switch installs it, which is what the
/// CPU holds after a Switch (a replay Mii's filled create ID and name); until that is known, against the target.
pub(crate) fn recorded_target_matches(target:crate::engine::mii::replay::Target,current:Option<i32>) -> bool {
    let target=crate::runtime::macros::identity::installed(target).unwrap_or(target);
    current.is_some_and(|kind| target.ready(kind as u32,
        crate::runtime::macros::identity::current_configuration()))
}
fn exact_target_ready(target:crate::engine::mii::replay::Target,current:Option<i32>) -> bool {
    let Some(kind)=current else{return false;};
    if target.configuration().is_none() {return target.ready(kind as u32,None);}
    crate::runtime::macros::identity::named_ready(target)
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
        FighterOrigin::Combo=>crate::runtime::ui::combos::prompt_revision(),
        FighterOrigin::Recording=>0,
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
/// Open the shared prompt for a chosen combo (Player 1's fighter). The combos UI
/// owns the candidate and receives the answer through [`take_combo_fighter_choice`].
pub(crate) fn open_combo_fighter_prompt(revision:u64, recorded:String, current:String) -> bool {
    *lock_write(&COMBO_FIGHTER)=None;
    let opened=lock_write(&UI).open_fighter_prompt_for(FighterOrigin::Combo,crate::model::recording_menu::ViewRole::Player,revision,recorded,current);
    if opened {assign(&menu::MENU_RECEIVED_INPUT,true);}
    opened
}
static CHARACTER_PENDING: AtomicBool = AtomicBool::new(false);
static CHARACTER_REQUEST: hos_lock::Mailbox<(crate::engine::mii::replay::Target, u64), 1> = hos_lock::Mailbox::new();
static CHARACTER_RESULT: hos_lock::Mailbox<(u64, bool), 1> = hos_lock::Mailbox::new();

fn submit_loaded_character() {
    if CHARACTER_PENDING.load(Ordering::Acquire) { return; }
    let target=lock_read(&UI).loaded_target();
    let Some(target)=target else{return;};
    if lock_write(&UI).claim_character_request()!=Some(target.kind() as i32) {return;}
    CHARACTER_PENDING.store(true, Ordering::Release);
    let posted = CHARACTER_REQUEST.post((target, selection_revision()));
    debug_assert!(posted, "one outstanding named character request");
}

/// Game callback only. The roster remains the sole admission/loader provider.
pub fn service_character_request() {
    let Ok(context) = crate::runtime::roster::character_roster::try_cpu_request_context() else { return; };
    CHARACTER_REQUEST.drain(|(target, revision)| {
        let eligible = revision == selection_revision() && context.menu.macro_source == MacroSource::NAMED
            && crate::runtime::roster::character_roster::cpu_reload_ready_in(&context);
        let accepted = eligible && crate::runtime::roster::character_roster::mii_replay::request_named_in(&context, target);
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
            (name.to_owned(), cursor, title, ui.native_name_allows_empty())
        })
    };
    let Some((name, cursor, title, allow_empty)) = values else { pending.take(); return };
    let id = NAME_SERIAL.fetch_add(1, Ordering::Relaxed);
    let opened = if job.waiting {
        crate::runtime::mii::native_keyboard::request_inline(id, &name, cursor, allow_empty)
    } else {
        crate::runtime::mii::native_keyboard::request(id, title, &name, allow_empty)
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
    observe_loaded();
    let marker = lock_read(&UI).marker();
    let (browser_pending, browser_release) = crate::runtime::macros::replay_capture::browser_queries();
    let clip_previewing = crate::runtime::macros::replay_capture::clip_view().is_some_and(|v| v.hides_page())
        || crate::engine::rr_host::preview_away();
    QUERIES.publish(crate::model::macro_handoff::MacroQueries {
        session: session_revision(),
        panel_visible: panel_visible(),
        clip_foreground: clip_foreground(), naming_pending: naming_pending(),
        clip_editor_open: crate::runtime::macros::replay_capture::editor_open(),
        clip_previewing,
        replay_loading: crate::runtime::macros::replay_capture::loading(), browser_pending, browser_release,
        suspended: suspended(), marker,
    });
}

/// The shared Fighter Mismatch question is up (any origin). With the menu closed the worker draws it alone (`flow::Phase::Prompt`).
pub fn fighter_prompt_up() -> bool {
    !suspended() && lock_read(&UI).fighter_prompt().is_some()
}

pub fn panel_view() -> Option<View> {
    if suspended() { return None; }
    let ui = lock_read(&UI);
    ui.native_name_view().or_else(|| ui.view())
        .or_else(crate::runtime::macros::replay_capture::browser_view)
        .or_else(crate::runtime::macros::replay_capture::loading_view)
}

pub fn clip_foreground() -> bool {
    // UI's guard ends with this statement: the keyboard service holds NAME_JOB while it takes UI, so UI is never held
    // while NAME_JOB is taken (tests/lock_order.rs).
    let open = lock_read(&UI).is_open();
    !suspended() && !open && crate::runtime::macros::replay_capture::panel_visible()
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
            (name.to_owned(), cursor, title, ui.native_name_allows_empty())
        })
    };
    let Some((initial, cursor, title, allow_empty)) = values else { return };
    let id = NAME_SERIAL.fetch_add(1, Ordering::Relaxed);
    let job = NameJob { id, generation: SESSION_GENERATION.load(Ordering::Relaxed),
        replay: crate::runtime::macros::replay_capture::panel_visible(), waiting: crate::runtime::ui::facts::fact_waiting_scene(),
        phase: NamePhase::Active };
    *NAME_JOB.lock().unwrap() = Some(job);
    let result = if job.waiting { crate::runtime::mii::native_keyboard::request_inline(id, &initial, cursor, allow_empty) }
        else { crate::runtime::mii::native_keyboard::request(id, title, &initial, allow_empty) };
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
    // A restored selection is not an explicit choice: it plays on whichever CPU
    // Training starts with instead of forcing the recorded fighter.
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
    if slot.is_none() {
        notify("Choose a Record Slot first");
        return;
    }
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
    lock_write(&UI).open_library_titled(
        rows,
        skipped,
        "Named Library",
        None,
    );
    assign(&menu::MENU_RECEIVED_INPUT, true);
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

/// Edit placement edits the Recording pop-up role's active macro, the file Play plays (primary 2026-11-08 #70 (a)); the anchor
/// is where that macro's start save puts the role (`rr_host::placement_target`).
pub fn open_editor() {
    if !practice_ready() {
        return;
    }
    let root = std::path::Path::new(crate::model::storage::ROOT);
    let target = match crate::engine::rr_host::placement_target(root) {
        Ok(target) => target,
        Err(refusal) => {
            notify(refusal);
            return;
        }
    };
    if !lock_write(&UI).open_placement(target.id, target.recording, target.anchor) {
        notify("Placement is not usable");
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

/// Close the panel while retaining the loaded recording.
pub fn clear_session() {
    // Game-owned input guard must not survive scene cleanup.
    crate::runtime::macros::replay_capture::owns_close_input(false, false);
    SESSION_GENERATION.fetch_add(1, Ordering::Relaxed);
    crate::engine::rr_host::drop_preview_home();
    post_lifecycle(Lifecycle::Clear { keep: suspended() });
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
    let Ok(context) = practice::try_context() else { return; };
    let id = module_accessor.battle_object_id;
    let pointer = module_accessor as *mut BattleObjectModuleAccessor as usize;
    let mut owned = lock_write(&PANEL_STOPS);
    let found = owned.iter().position(|&(object, address, _)| object == id && address == pointer);
    // Keep P1 held while the editor is parked for a CPU reload. The new CPU must
    // remain free to finish its native install and land before preview acquisition.
    let queries = hook_queries();
    let hold_panel = (queries.panel_visible && !queries.clip_previewing) || (queries.replay_loading
        && WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID)
            == FighterId::Player as i32);
    let entry = if hold_panel && !crate::runtime::macros::replay_capture::browser_needs_native_update(context, module_accessor) {
        practice::panel_entry_in(context, module_accessor)
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
        Effect::BindToSlot { .. } => {}
        Effect::Alternate { id } => *lock_write(&LIST_ACTION) = Some(id),
        Effect::Rename { id } => *lock_write(&LIST_RENAME) = Some(id),
        Effect::ForwardChoose { id } => *lock_write(&LIST_CHOICE) = Some(id),
        Effect::NameConfirmed { context, name } => {
            *lock_write(&CONFIRMED_NAME) = Some(NameConfirmation { context, name });
        }
        Effect::NeedCharacter(kind) => request_kind(kind),
        Effect::ClipFighter(choice) => crate::runtime::macros::replay_capture::fighter_choice(choice),
        Effect::ComboFighter(choice) => *lock_write(&COMBO_FIGHTER) = Some(choice),
        Effect::RecordingFighter(choice) => *lock_write(&RECORDING_FIGHTER) = Some(choice),
        Effect::KeepPlacement { id, placement } => {
            crate::engine::rr_host::keep_placement(&id, placement);
            notify("Placement kept for this session");
        }
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
        Ok(()) => {
            crate::engine::rr_host::placement_written(&id);
            notify("Placement written");
        }
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

/// Diag build only: the keyboard's confirmed-name path without the applet (`service_keyboard` does exactly this with the typed text).
#[cfg(feature = "diag")]
pub fn diag_confirm_name(name: &str) {
    let effect = lock_write(&UI).confirm_native_name(name.to_owned());
    apply_effect(effect);
}
