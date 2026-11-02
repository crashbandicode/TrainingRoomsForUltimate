//! Native input preview, using the existing Training save-state/playback paths.
//! UI requests carry values only. Game-object work runs on the CPU callback.
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use smash::app::{self, BattleObjectModuleAccessor, Fighter};
use {crate::engine::macros::MacroRecording, crate::engine::macros::StartState};

use crate::runtime::macros::{input_record, save_states, practice};

#[derive(Clone, Copy)]
enum Request { Play { start: StartState, playback: super::named_macros::PlaybackStart, revision: u64 }, Stop }
#[derive(Clone, Copy, PartialEq)]
enum Phase { Starting, Playing, Complete, Stopping, Failed }
struct Session {
    start: StartState,
    playback: super::named_macros::PlaybackStart,
    revision: u64,
    phase: Phase,
    waits: u32,
    cpu: Option<(u32, usize)>,
    /// The actual CPU's fighter kind when the preview started. A preview plays the
    /// recording's inputs on whichever fighter is on stage, so the recorded kind is
    /// not compared; only this fighter changing mid-preview invalidates it.
    kind: i32,
}
static REQUEST: hos_lock::Mailbox<(u64, u64, Request), 32> = hos_lock::Mailbox::new();
static SERIAL: AtomicU64 = AtomicU64::new(1);
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static CONSUMED: AtomicU64 = AtomicU64::new(0);
static STOP_QUEUED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn request_pending() -> bool { REQUESTED.load(Ordering::Acquire) > CONSUMED.load(Ordering::Acquire) }
fn post_request(request: Request) {
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    if REQUEST.post((serial, super::named_macros::session_revision(), request)) {
        REQUESTED.fetch_max(serial, Ordering::Release);
    } else {
        println!("[MacroHandoff] preview_request_dropped=1");
    }
}
fn take_request() -> Option<Request> {
    let mut newest = None;
    let previous = CONSUMED.load(Ordering::Acquire);
    REQUEST.drain(|(serial, session, request)| {
        CONSUMED.fetch_max(serial, Ordering::Release);
        if serial > previous && session == super::named_macros::session_revision()
            && newest.as_ref().is_none_or(|(seen, _)| serial > *seen) {
            newest = Some((serial, request));
        }
    });
    if newest.is_some() { STOP_QUEUED.store(false, Ordering::Release); }
    newest.map(|(_, request)| request)
}
fn discard_requests() { let _ = take_request(); }

static SESSION: hos_lock::Mutex<Option<Session>> = hos_lock::Mutex::new(None);
static STATUS: AtomicU8 = AtomicU8::new(0);

pub fn request_play(recording: MacroRecording, revision: u64) {
    if let Err(error) = recording.validate() {
        crate::runtime::macros::replay_capture::preview_failed(&error.to_string());
        return;
    }
    let playback = super::named_macros::prepare_preview(&recording);
    STATUS.store(1, Ordering::Relaxed);
    STOP_QUEUED.store(false, Ordering::Release);
    post_request(Request::Play { start: recording.start, playback, revision });
}

pub fn request_stop() {
    if occupied() {
        if !STOP_QUEUED.swap(true, Ordering::AcqRel) { post_request(Request::Stop); }
    } else {
        save_states::finish_preview_restore();
    }
}

pub fn occupied() -> bool {
    ACTIVE.load(Ordering::Acquire) || request_pending()
}
/// Failed native cleanup keeps its ownership, but closing the panel must leave
/// a controller route to the native pause/exit menu. No autoplay is re-enabled.
pub fn recovery_available() -> bool {
    ACTIVE.load(Ordering::Acquire) && STATUS.load(Ordering::Acquire) == 5
}
pub fn start_context() -> Option<StartState> {
    SESSION.lock().as_ref().filter(|s| matches!(s.phase, Phase::Starting | Phase::Playing))
        .map(|s| s.start)
}
pub fn can_emit(module: &mut BattleObjectModuleAccessor) -> bool {
    !request_pending()
        && SESSION.lock().as_ref().is_some_and(|s| s.phase == Phase::Playing
            && s.cpu == Some((module.battle_object_id, module as *mut _ as usize)))
        && crate::runtime::macros::named_macros::hook_queries().clip_foreground
}
pub fn status() -> &'static str {
    match STATUS.load(Ordering::Relaxed) {
        1 => "Preparing the temporary preview setup...",
        2 => "Native input preview: attacks and effects run normally",
        3 => "Inputs complete; native effects finish normally. Y: restart",
        4 => "Restoring the temporary preview setup...",
        // The editor keeps the actual failure in its full-width help strip.
        5 => "Preview stopped. Fix the issue above, then press X.",
        _ => "",
    }
}

/// Both fighters and partners remain native: hits, effects and cleanup need
/// normal simulation. P1's controls are consumed separately by the visible UI.
pub fn needs_native_update(_module: &BattleObjectModuleAccessor) -> bool { occupied() }

pub fn clear_session() {
    discard_requests();
    SESSION.lock().take();
    ACTIVE.store(false, Ordering::Release);
    save_states::end_preview_restore();
    STATUS.store(0, Ordering::Relaxed);
}

unsafe fn fail(message: &'static str) {
    input_record::stop_playback();
    discard_requests();
    let was_running = SESSION.lock().as_ref().is_some_and(|s|
        matches!(s.phase, Phase::Playing | Phase::Complete));
    if was_running {
        request_stop();
    } else if save_states::is_loading() {
        // A deadline is not a successful restore. Let native sleep/buff/partner
        // cleanup continue, without injection or autoplay, until reset/departure.
        if let Some(s) = SESSION.lock().as_mut() { s.phase = Phase::Failed; }
    } else { clear_session(); }
    STATUS.store(5, Ordering::Relaxed);
    crate::runtime::macros::replay_capture::post_preview_failed(message);
}

pub fn progress(consumed: usize) {
    let revision = SESSION.lock().as_ref()
        .filter(|s| s.phase == Phase::Playing).map(|s| s.revision);
    if let Some(revision) = revision {
        crate::runtime::macros::replay_capture::preview_progress(revision, consumed as u32);
    }
}

pub fn complete() {
    if let Some(s) = SESSION.lock().as_mut() {
        s.phase = Phase::Complete;
        STATUS.store(3, Ordering::Relaxed);
        println!("[ClipPreview] inputs_complete revision={} frames={}", s.revision, s.playback.len);
    }
}

unsafe fn primary_cpu(module: &mut BattleObjectModuleAccessor) -> bool {
    crate::runtime::features::recorder::is_primary_cpu_body(module) && crate::runtime::compat::common::is_operation_cpu(module)
}

pub unsafe fn before_frame(module: &mut BattleObjectModuleAccessor) {
    if !practice::scene_enabled() {
        if occupied() { input_record::stop_playback(); save_states::cancel_pending(); }
        clear_session();
        return;
    }
    if !practice::enabled() { return; }
    if !primary_cpu(module) {
        return;
    }
    if crate::runtime::roster::character_roster::busy() || !Fighter::is_ready_go() {
        if occupied() { input_record::stop_playback(); save_states::cancel_pending(); }
        clear_session();
        return;
    }
    let replaced = SESSION.lock().as_ref().is_some_and(|s|
        matches!(s.phase, Phase::Playing | Phase::Complete)
            && (s.cpu != Some((module.battle_object_id, module as *mut _ as usize))
                || s.kind != app::utility::get_kind(module)));
    if replaced {
        input_record::stop_playback();
        clear_session();
        STATUS.store(5, Ordering::Relaxed);
        crate::runtime::macros::replay_capture::post_preview_failed("Preview fighter changed; reset Training before retrying");
        return;
    }
    if SESSION.lock().as_ref().is_some_and(|s| s.phase == Phase::Playing)
        && !crate::runtime::macros::named_macros::hook_queries().clip_foreground { request_stop(); }
    // A queued restore owns the native loader until completion. Keep only the
    // newest UI request, then handle it once the current restore is finished.
    if save_states::is_loading() { return; }
    let request = take_request();
    match request {
        Some(Request::Play { start, playback, revision }) => {
            // Identity is the editor's decision (Keep or an answered Switch), not
            // this request's: ownership, scope and restore guards below still apply.
            if !practice::acquire() {
                fail("Wait for the selected CPU fighter before previewing");
                return;
            }
            input_record::stop_playback();
            ACTIVE.store(true, Ordering::Release);
            *SESSION.lock() = Some(Session {
                start, playback, revision, phase: Phase::Starting, waits: 0, cpu: None,
                kind: app::utility::get_kind(module),
            });
            if let Err(error) = save_states::request_preview_restore(true) { fail(error); }
            else { println!("[ClipPreview] restore_start revision={}", revision); }
        }
        Some(Request::Stop) if occupied() => {
            input_record::stop_playback();
            if let Some(s) = SESSION.lock().as_mut() {
                s.phase = Phase::Stopping;
                s.waits = 0;
            }
            STATUS.store(4, Ordering::Relaxed);
            if let Err(error) = save_states::request_preview_restore(false) { fail(error); }
            else { println!("[ClipPreview] restore_stop"); }
        }
        Some(Request::Stop) => STATUS.store(0, Ordering::Relaxed),
        None => {}
    }
}

pub unsafe fn after_frame(module: &mut BattleObjectModuleAccessor) {
    if !practice::enabled() || !primary_cpu(module) { return; }
    let loading = save_states::is_loading();
    let action = {
        let mut session = SESSION.lock();
        let Some(s) = session.as_mut() else { return };
        if s.phase == Phase::Failed {
            if loading { return; }
            drop(session);
            clear_session();
            STATUS.store(5, Ordering::Relaxed);
            return;
        }
        if s.phase == Phase::Playing && !input_record::is_playback() {
            drop(session);
            crate::runtime::macros::replay_capture::post_preview_failed("Preview start did not settle; restoring the preview setup");
            request_stop();
            return;
        }
        if !matches!(s.phase, Phase::Starting | Phase::Stopping) { return; }
        s.waits += 1;
        if s.waits > 600 { Some(Err("Preview restore stalled: B closes, then + exits Training")) }
        else if loading { None }
        else if s.phase == Phase::Stopping { Some(Ok(None)) }
        else {
            s.phase = Phase::Playing;
            s.cpu = Some((module.battle_object_id, module as *mut _ as usize));
            Some(Ok(Some(s.playback)))
        }
    };
    match action {
        Some(Ok(Some(recording))) => {
            if input_record::playback_preview(recording) {
                STATUS.store(2, Ordering::Relaxed);
                println!("[ClipPreview] native_play");
            } else { fail("Preview could not start; close and reopen the editor"); }
        }
        Some(Ok(None)) => {
            discard_requests();
            SESSION.lock().take();
    ACTIVE.store(false, Ordering::Release);
            save_states::finish_preview_restore();
            STATUS.store(0, Ordering::Relaxed);
            println!("[ClipPreview] restored_idle");
        }
        Some(Err(error)) => {
            fail(error);
        }
        None => {}
    }
}
