//! The input recorder, the runtime half (SPEC-input-01): the glue between the game and
//! `model::recording`.
//!
//! Three entry points, one per stage of the model:
//! * `on_p1_poll` (from the P1 input poll, after the input delay): stores the frame while recording
//!   and neutralises P1's output while the recorder owns P1.
//! * `on_ai_frame` (the inline hook at the AI control function's epilogue): the CPU's control frame.
//!   Checks the controller is the CPU entry's own (rule R3: never Nana, never an AI-driven P1), starts a
//!   mash-triggered playback, injects the frame, runs the end-of-run check and the chords, applies the
//!   phase tint and posts the phase notice.
//! * `request_playback` / `on_state_restored` / `stop`: the entry points the CPU behaviours, the save
//!   states and the hubs call.
//!
//! Contract (as the CPU runtime's): game reads happen with no lock held (a game call can re-enter one
//! of this plugin's hooks), the model is called under one short lock, the answer is applied after the
//! lock is released. Nothing allocates in a hook.
//!
//! The per-frame work is in the AI hook instead of the fighter on-frame wrapper because the controller
//! write and the end check belong to the same CPU control frame; the end check follows the injection
//! immediately (SPEC-input-01 §5.2 step 3 runs just after step 2 in the same frame).
//!
//! NOT YET SWITCH-RUN.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};

use parking_lot::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::chords::Request;
use crate::model::cpu::facts::PlaybackStart;
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::cpu::rng::Rng;
use crate::model::cpu::role::FighterKey;
use crate::model::cpu::status;
use crate::model::input_delay::{MappedInputs, Owners};
use crate::model::recording::{
    ChordAction, ControlCtx, ControlOut, Ended, LoadAction, MashCtx, Mode, Phase, PlayRefusal, PollFacts, PostCtx, RecordRefusal, Recorder,
    RecorderSettings, StartSnapshot,
};
use crate::model::settings::Snapshot;
use crate::model::sites_pkg4;
use crate::runtime::alloc::HookScope;
use crate::runtime::features::{chords, cpu, input_display, notices, states};
use crate::runtime::game::{self, natives};
use crate::runtime::macros::{named_macros, replay_preview};
use crate::runtime::published;
use crate::runtime::scene;

static REC: Mutex<Recorder> = Mutex::new(Recorder::new());
/// The recorder's hook was installed with every row of its closure (`model::capabilities`, review R2-1): until
/// boot says so, the entry points the other features call (`on_p1_poll`, `request_playback`,
/// `on_state_restored`) do nothing and read no native.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Boot: the recorder's closure passed its guards and `ai_epilogue` is installed.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}
static SETTINGS: Mutex<RecorderSettings> = Mutex::new(RecorderSettings::factory());
static RNG: Mutex<Rng> = Mutex::new(Rng::new(0x5EED));

// The constants the recorder needs, taken from the CPU runtime's table once it exists.
static CONSTS_READY: AtomicU8 = AtomicU8::new(0);
static KIND_SHULK: AtomicI32 = AtomicI32::new(UNRESOLVED);
static FLAG_SHULK_DIAL: AtomicI32 = AtomicI32::new(UNRESOLVED);
static INT_CLIFF_REST: AtomicI32 = AtomicI32::new(UNRESOLVED);
static STATUS_CLIFF_WAIT: AtomicI32 = AtomicI32::new(UNRESOLVED);
static STATUS_CLIFF_CATCH: AtomicI32 = AtomicI32::new(UNRESOLVED);
static MODEL_COLOR_BLEND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static INT_COLOR: AtomicI32 = AtomicI32::new(UNRESOLVED);

/// The phase last announced (a `Phase` discriminant), so a notice is posted once per change.
static LAST_PHASE: AtomicU8 = AtomicU8::new(0);

/// P1 polls still to be made neutral (the native keyboard's closing press must not become a gameplay input).
static SWALLOW: AtomicU8 = AtomicU8::new(0);

/// Fighter object and entry layout (read from the owned 13.0.5 `main`; `tests/input_sites.rs` and the
/// on-init notes): the fighter kind at +0xc; the entry's controller at +0x4178 (the body and accessor
/// fields are in `game::natives`).
pub(crate) const FIGHTER_KIND_FIELD: usize = 0xc;
const ENTRY_CONTROLLER_FIELD: usize = 0x4178;
/// The CPU's controller struct (`*(ai + 8)`): the written fields (SPEC-input-01 §4). Never +0x50/+0x54.
const CTRL_BUTTONS: usize = 0xc;
const CTRL_STICK_X: usize = 0x10;
const CTRL_STICK_Y: usize = 0x14;
const CTRL_CLAMPED_X: usize = 0x40;
const CTRL_CLAMPED_Y: usize = 0x44;
const AI_CONTROLLER_FIELD: usize = 0x8;

#[cfg(feature = "diag")]
mod counts {
    use std::sync::atomic::{AtomicU32, Ordering};

    pub const POLL: usize = 0;
    pub const AI_FRAME: usize = 1;
    pub const INJECT: usize = 2;
    pub const NOTE: usize = 3;
    static COUNTS: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
    const REPORT_EVERY: u32 = 600;

    pub fn bump(which: usize) {
        COUNTS[which].fetch_add(1, Ordering::Relaxed);
    }

    /// One line per `REPORT_EVERY` AI frames: the hook invocation counts and the allocation count since
    /// boot (review R5: the count after the hooks have run, not only at install).
    pub fn report_if_due() {
        if COUNTS[AI_FRAME].load(Ordering::Relaxed) < REPORT_EVERY {
            return;
        }
        let take = |i: usize| COUNTS[i].swap(0, Ordering::Relaxed);
        crate::runtime::diag_log::diagnostic(format_args!(
            "[TrainingRooms] input_hook_counts p1_poll={} ai_frame={} inject={} notice_tick={} heap_allocations={}",
            take(POLL),
            take(AI_FRAME),
            take(INJECT),
            take(NOTE),
            crate::runtime::alloc::hook_allocations()
        ));
    }
}

#[cfg(feature = "diag")]
fn bump(which: usize) {
    counts::bump(which);
}

#[cfg(not(feature = "diag"))]
#[inline(always)]
fn bump(_which: usize) {}

#[cfg(feature = "diag")]
fn report_counts() {
    counts::report_if_due();
}

#[cfg(not(feature = "diag"))]
#[inline(always)]
fn report_counts() {}

const COUNT_POLL: usize = 0;
const COUNT_AI_FRAME: usize = 1;
const COUNT_INJECT: usize = 2;
const COUNT_NOTICE: usize = 3;

/// One recorder event line (diag build only: the shipped build writes nothing per event, playbook).
#[cfg(feature = "diag")]
fn event(name: &str, key_a: &str, a: i64, key_b: &str, b: i64) {
    crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] {name} {key_a}={a} {key_b}={b}"));
}

#[cfg(not(feature = "diag"))]
#[inline(always)]
fn event(_name: &str, _key_a: &str, _a: i64, _key_b: &str, _b: i64) {}

/// Boot: the generator's seed (the plugin's own, never the game's, SPEC-input-01 question 12).
pub fn init(seed: u64) {
    *RNG.lock() = Rng::new(seed);
}

/// The recorder rows as the last publish left them (the named layer reads Recording Trigger and Save State Playback).
pub fn settings() -> RecorderSettings {
    *SETTINGS.lock()
}

/// A menu commit or the first publish.
pub fn publish(snapshot: &Snapshot) {
    *SETTINGS.lock() = RecorderSettings::from_snapshot(snapshot);
    // Macro Source = Named Library: the layer's frame buffers are reserved here (the committing thread), never in a hook.
    if snapshot.get("macro_source").is_some_and(|row| row.get(1).is_some_and(|v| *v != 0)) {
        named_macros::prepare_buffers();
    }
}

/// The CPU runtime's constants (ready once the game's table answers).
fn load_consts() -> bool {
    if CONSTS_READY.load(Ordering::Acquire) != 0 {
        return true;
    }
    let Some(table) = cpu::ids() else { return false };
    KIND_SHULK.store(table.get(C::KindShulk), Ordering::Release);
    FLAG_SHULK_DIAL.store(table.get(C::WorkFlagShulkDial), Ordering::Release);
    INT_CLIFF_REST.store(table.get(C::WorkIntCliffRestTime), Ordering::Release);
    STATUS_CLIFF_WAIT.store(table.get(C::StatusCliffWait), Ordering::Release);
    STATUS_CLIFF_CATCH.store(table.get(C::StatusCliffCatch), Ordering::Release);
    MODEL_COLOR_BLEND.store(table.get(C::ModelColorBlend), Ordering::Release);
    INT_COLOR.store(table.get(C::WorkIntColor), Ordering::Release);
    CONSTS_READY.store(1, Ordering::Release);
    true
}

/// A fighter entry's first body, resolved now (no pointer is kept between frames).
#[derive(Clone, Copy)]
pub(crate) struct Body {
    entry_ptr: usize,
    pub(crate) fighter: usize,
    pub(crate) accessor: usize,
    entry_id: i32,
}

pub(crate) unsafe fn cpu_body() -> Option<Body> {
    body_of(i32::from(cpu::primary_entry_now()))
}

/// The first body of `entry_id (0 is P1).
///
/// # Safety
/// The package 4 natives must have passed their guards.
pub(crate) unsafe fn body_of(entry_id: i32) -> Option<Body> {
    let entry_ptr = natives::fighter_entry(entry_id);
    let fighter = natives::entry_body(entry_ptr);
    if fighter == 0 {
        return None;
    }
    let accessor = natives::fighter_accessor(fighter);
    (accessor != 0).then_some(Body { entry_ptr, fighter, accessor, entry_id })
}

fn facing_of(lr: f32) -> i8 {
    if lr >= 0.0 {
        1
    } else {
        -1
    }
}

// ---- the P1 poll stage --------------------------------------------------------------------------------

/// P1's final mapped record after the input delay: store while recording, neutralise while the recorder
/// owns P1. `out` is the game's 8-byte record.
///
/// # Safety
/// `out` is the record the game just mapped.
pub unsafe fn on_p1_poll(out: *mut MappedInputs) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    bump(COUNT_POLL);
    if SWALLOW.load(Ordering::Relaxed) > 0 {
        SWALLOW.fetch_sub(1, Ordering::Relaxed);
        *out = MappedInputs::NEUTRAL;
    }
    let (mode, named) = {
        let rec = REC.lock();
        (rec.mode(), rec.is_named())
    };
    if mode == Mode::None {
        return;
    }
    if named {
        on_named_poll(out);
        return;
    }
    let wants = REC.lock().wants_cpu_status();
    let cpu_status = if wants && load_consts() {
        cpu_body().map_or(0, |b| game::natives::status_kind(b.accessor))
    } else {
        0
    };
    REC.lock().poll(&mut *out, PollFacts { cpu_status });
}

/// The poll stage of a named run (SPEC-input-01 section 9). The record chord while a named capture records is the stop chord: it is
/// consumed here, before this poll's frame is stored, so the stop press is not part of the recording.
unsafe fn on_named_poll(out: *mut MappedInputs) {
    if REC.lock().is_recording() && chords::consume(Request::InputRecord) {
        *out = MappedInputs::NEUTRAL;
        finish_named_capture();
        return;
    }
    let stored = REC.lock().poll_named(&mut *out);
    if let Some((index, frame)) = stored {
        if index == 0 {
            if let Some(body) = cpu_body() {
                named_macros::capture_start(&mut *(body.accessor as *mut smash::app::BattleObjectModuleAccessor));
            }
        }
        named_macros::store_frame(usize::from(index), &frame);
    }
}

// ---- the CPU control stage ----------------------------------------------------------------------------

/// The inline hook at the AI control function's epilogue: `x19` is the AI object and `*(x19 + 8)` the
/// controller it just wrote.
#[skyline::hook(offset = sites_pkg4::AI_EPILOGUE_OFFSET, inline)]
pub unsafe fn ai_epilogue(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    on_ai_frame(ctx.registers[19].x() as usize);
}

unsafe fn on_ai_frame(ai: usize) {
    // Native Training or an active lobby practice (the hot swap check below stops the recorder).
    if ai == 0 || !(scene::is_native_training() || scene::lobby_active()) {
        return;
    }
    let controller = ((ai + AI_CONTROLLER_FIELD) as *const usize).read();
    if controller == 0 {
        return;
    }
    if published::HOT_SWAP_IN_PROGRESS.load(Ordering::Acquire) {
        stop();
        return;
    }
    let Some(body) = cpu_body() else {
        return;
    };
    // Rule R3: only the CPU entry's own controller (never Nana's, never an AI-driven P1's).
    if ((body.entry_ptr + ENTRY_CONTROLLER_FIELD) as *const usize).read() != controller || !load_consts() {
        return;
    }
    bump(COUNT_AI_FRAME);
    let settings = *SETTINGS.lock();
    let key = FighterKey::new(body.entry_id, false);
    let facing = facing_of(natives::lr(body.accessor));
    let kind = ((body.fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let dial = FLAG_SHULK_DIAL.load(Ordering::Acquire);
    let shulk = kind == KIND_SHULK.load(Ordering::Acquire) && dial != UNRESOLVED && game::natives::is_flag(body.accessor, dial);

    // A playback the mash queue asked for starts before this frame's injection (§5.2 step 2).
    if let Some(key) = key {
        mash_start(&settings, key, body, facing);
    }

    let ctx = ControlCtx { facing, shulk_dial: shulk };
    let control = if REC.lock().is_named() {
        // Before input 0 of a named playback the layer repositions the CPU (it may stop the playback itself): neutral controls until then.
        let (playing, at_zero) = {
            let rec = REC.lock();
            (rec.mode() == Mode::Playback, rec.frame() == 0)
        };
        if playing && at_zero && !named_macros::prepare_playback_start(&mut *(body.accessor as *mut smash::app::BattleObjectModuleAccessor)) {
            ControlOut { write: Some(crate::model::recording::Inject::NEUTRAL), hold_cliff_timer: false }
        } else {
            REC.lock().control_named(&ctx, &|index| named_macros::frame_for(usize::from(index)))
        }
    } else {
        REC.lock().control(&ctx)
    };
    if let Some(write) = control.write {
        bump(COUNT_INJECT);
        let at = |offset: usize| (controller + offset) as *mut u32;
        at(CTRL_BUTTONS).write(write.buttons);
        at(CTRL_STICK_X).write(write.stick_x.to_bits());
        at(CTRL_STICK_Y).write(write.stick_y.to_bits());
        at(CTRL_CLAMPED_X).write(write.clamped_x.to_bits());
        at(CTRL_CLAMPED_Y).write(write.clamped_y.to_bits());
    }
    if control.hold_cliff_timer {
        let rest = INT_CLIFF_REST.load(Ordering::Acquire);
        if rest != UNRESOLVED
            && game::natives::status_kind(body.accessor) == STATUS_CLIFF_WAIT.load(Ordering::Acquire)
            && natives::get_int(body.accessor, rest) < crate::model::recording::CLIFF_REST_TIMER_LOW
        {
            natives::set_int(body.accessor, rest, crate::model::recording::CLIFF_REST_TIMER);
        }
    }

    let queue_head = key.and_then(cpu::mash_head);
    let post = REC.lock().post_frame(&settings, &PostCtx { facing, queue_head_is_playback: queue_head.is_some() });
    if post.pop_mash {
        if let Some(key) = key {
            cpu::mash_pop(key);
        }
    }
    match post.ended {
        Some(Ended::Recorded { slot, frames }) => event("recording_complete", "slot", i64::from(slot), "frames", i64::from(frames)),
        Some(Ended::Played { slot }) => event("playback_complete", "slot", i64::from(slot), "restarted", i64::from(post.restarted.is_some())),
        Some(Ended::NamedCaptured { frames }) => {
            event("named_capture_complete", "frames", i64::from(frames), "", 0);
            named_macros::finish_capture(settings.crop);
        }
        Some(Ended::NamedPlayed) => {
            event("named_playback_complete", "", 0, "", 0);
            named_macros::end_playback();
            if replay_preview::occupied() {
                replay_preview::complete();
            }
        }
        None => {}
    }
    chord_step(&settings, body, facing);

    // The phase: tint every frame it lasts, the notice once per change.
    let (phase, length) = {
        let rec = REC.lock();
        (rec.phase(), rec.length())
    };
    if let (Some([r, g, b]), blend) = (phase.tint(), MODEL_COLOR_BLEND.load(Ordering::Acquire)) {
        if blend != UNRESOLVED {
            natives::set_color_rgb(body.accessor, r, g, b, blend);
        }
    }
    if post.restarted.is_some() || LAST_PHASE.swap(phase as u8, Ordering::AcqRel) != phase as u8 {
        notices::post_phase(phase, length);
    }
    let state = REC.lock().playback_state(queue_head.is_some());
    cpu::set_playback(state);
    bump(COUNT_NOTICE);
    // The input log's in-game frame counters tick here, with the CPU body's frame; the HUD tick is the layout's
    // (`features::hud_draw`).
    input_display::tick_frame();
    report_counts();
}

unsafe fn mash_start(settings: &RecorderSettings, key: FighterKey, body: Body, facing: i8) {
    if REC.lock().mode() != Mode::None {
        return;
    }
    let Some(head) = cpu::mash_head(key) else { return };
    let Some(table) = cpu::ids() else { return };
    let status = game::natives::status_kind(body.accessor);
    let in_hitstun = status::in_hitstun(&table, status);
    let in_hitstop = natives::is_stop(body.accessor);
    let needs = REC.lock().mash_needs_term(settings, Some(head), in_hitstun);
    let term_enabled = if needs {
        let from = REC.lock().starting_status_of(head);
        let term = table.get(Recorder::term_for_status(&table, from));
        term != UNRESOLVED && game::natives::is_enable_transition_term(body.accessor, term)
    } else {
        false
    };
    let start = REC.lock().mash_should_start(
        settings,
        &MashCtx { head_slot: Some(head), in_hitstun, in_hitstop, term_enabled },
    );
    let Some(slot) = start else { return };
    let result = REC.lock().start_playback(slot, facing);
    match result {
        Ok(started) => {
            REC.lock().note_mash_started();
            event("playback_mash", "slot", i64::from(started.slot), "frames", i64::from(started.frames));
        }
        Err(refusal) => {
            // The queue would ask again every frame: drop the head and say why.
            cpu::mash_pop(key);
            notices::refuse(refusal.text());
        }
    }
}

/// The record and playback chords (SPEC-input-01 §2.2): consumed on the CPU body's frame.
unsafe fn chord_step(settings: &RecorderSettings, body: Body, facing: i8) {
    let playback = chords::consume(Request::InputPlayback);
    let record = chords::consume(Request::InputRecord);
    if states::is_loading() {
        // Both chords are consumed and ignored while a save state loads (§6.4).
        if record || playback {
            event("chord_ignored_loading", "record", i64::from(record), "playback", i64::from(playback));
        }
        return;
    }
    let record = record && settings.trigger_chord;
    if crate::runtime::macros::named_active() {
        // Macro Source = Named Library: the chords drive the named layer (SPEC-input-01 section 9). The record chord while a capture
        // records was consumed by the poll (the stop chord); here it starts one.
        if playback {
            named_macros::request_playback();
        } else if record && REC.lock().mode() == Mode::None {
            start_named_lockout();
        }
        return;
    }
    match Recorder::resolve_chords(record, playback) {
        ChordAction::None => {}
        ChordAction::Playback => {
            let slot = Recorder::pick_chord_slot(settings, &mut RNG.lock());
            match slot {
                Some(slot) => begin_playback(slot, facing, false),
                None => notices::refuse(PlayRefusal::SlotEmpty.text()),
            }
        }
        ChordAction::Record => begin_recording(settings, body, facing),
    }
}

unsafe fn begin_playback(slot: u8, facing: i8, ledge: bool) {
    let catch = ledge && STATUS_CLIFF_CATCH.load(Ordering::Acquire) != UNRESOLVED && cpu_body().is_some_and(|b| {
        game::natives::status_kind(b.accessor) == STATUS_CLIFF_CATCH.load(Ordering::Acquire)
    });
    let result = if ledge { REC.lock().playback_ledge(slot, facing, catch) } else { REC.lock().start_playback(slot, facing) };
    match result {
        Ok(started) => {
            event("playback_shortcut", "slot", i64::from(started.slot), "frames", i64::from(started.frames));
        }
        Err(refusal) => notices::refuse(refusal.text()),
    }
}

/// Why a recording (numbered or named) is not started now: B4 an AI drives P1, a fighter is missing, a save state is loading.
unsafe fn record_refusal() -> Option<RecordRefusal> {
    if natives::is_operation_cpu(0) {
        Some(RecordRefusal::P1DrivenByAi)
    } else if natives::fighter_entry(0) == 0 {
        Some(RecordRefusal::FightersMissing)
    } else if states::is_loading() {
        Some(RecordRefusal::SaveStateLoading)
    } else {
        None
    }
}

unsafe fn begin_recording(settings: &RecorderSettings, body: Body, facing: i8) {
    if let Some(refusal) = record_refusal() {
        notices::refuse(refusal.text());
        return;
    }
    let kind = ((body.fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let color = INT_COLOR.load(Ordering::Acquire);
    let costume = if color == UNRESOLVED { 0 } else { natives::get_int(body.accessor, color) as u32 };
    let start = StartSnapshot {
        fighter_kind: kind,
        costume,
        situation: game::natives::situation_kind(body.accessor),
        speed_x: 0.0,
        speed_y: 0.0,
    };
    REC.lock().start_recording(settings, facing, start);
    event("record_lockout", "cpu_kind", i64::from(kind), "frames", i64::from(settings.duration));
}

// ---- the named layer's core entry points (SPEC-input-01 section 9) -----------------------------------------

/// One short lock on the recorder for the named layer's slot export and import.
pub(crate) fn with_recorder<R>(f: impl FnOnce(&mut Recorder) -> R) -> R {
    f(&mut REC.lock())
}

/// The record chord of the named layer: the lockout, then the capture (the layer's buffer takes the frames).
pub(crate) fn start_named_lockout() {
    // SAFETY: called from the CPU control frame on a live CPU body.
    unsafe {
        if !load_consts() {
            return;
        }
        if let Some(refusal) = record_refusal() {
            notices::refuse(refusal.text());
            return;
        }
        let Some(body) = cpu_body() else { return };
        let facing = facing_of(natives::lr(body.accessor));
        named_macros::begin_capture(crate::engine::macros::StartState::default());
        REC.lock().start_named_lockout(facing);
        event("named_record_lockout", "facing", i64::from(facing), "", 0);
    }
}

/// The named stop chord: the capture ends and the layer offers the recording for saving.
fn finish_named_capture() {
    let frames = REC.lock().finish_named_capture();
    if let Some(frames) = frames {
        LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
        notices::post_phase(Phase::Idle, 0);
        event("named_capture_stop", "frames", i64::from(frames), "", 0);
        named_macros::finish_capture(SETTINGS.lock().crop);
    }
}

/// The layer's playback (`playback_named`, `playback_preview`): `frames` frames of its buffer, started on the CPU now.
pub fn start_named_playback(frames: usize, recorded_facing: f32, status: i32) -> bool {
    // SAFETY: called on the game thread with live fighters.
    unsafe {
        let Some(body) = cpu_body() else { return false };
        if !load_consts() {
            return false;
        }
        let facing = facing_of(natives::lr(body.accessor));
        let frames = u16::try_from(frames).unwrap_or(u16::MAX);
        let result = REC.lock().start_named_playback(frames, facing_of(recorded_facing), status, facing);
        match result {
            Ok(started) => {
                event("named_playback", "frames", i64::from(started.frames), "", 0);
                true
            }
            Err(refusal) => {
                notices::refuse(refusal.text());
                false
            }
        }
    }
}

/// P1's next `frames` polls read as neutral (see `SWALLOW`).
/// The CPU's fighter kind now (diag lines of the named layer's roster row).
pub fn cpu_kind() -> Option<i32> {
    // SAFETY: the natives answer only once their guards passed; the read is of the live body.
    unsafe { cpu_body() }.map(|body| unsafe { ((body.fighter + FIGHTER_KIND_FIELD) as *const i32).read() })
}

/// The recorder's phase (diag lines).
pub fn phase() -> Phase {
    REC.lock().phase()
}

/// The recorder's frame index (the named layer's diag lines).
pub fn frame() -> usize {
    usize::from(REC.lock().frame())
}

/// The facing the layer restored a pose with (the next playback mirrors against it).
pub fn set_current_facing(facing: f32) {
    REC.lock().set_current_facing(facing_of(facing));
}

pub fn swallow_p1(frames: u8) {
    SWALLOW.store(frames, Ordering::Release);
}

// ---- entry points for the other features ---------------------------------------------------------------

/// A playback the CPU behaviours ask for (the ledge options, SPEC-cpu-03): `FrameOut::playback`.
pub fn request_playback(start: PlaybackStart) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: called from the CPU runtime's frame wrapper on a live CPU body.
    unsafe {
        let Some(body) = cpu_body() else { return };
        if !load_consts() {
            return;
        }
        let facing = facing_of(natives::lr(body.accessor));
        begin_playback(start.slot, facing, start.ledge);
    }
}

/// Both fighters finished restoring after a save-state load (SPEC-input-01 §6.4): record or play per the
/// settings. `saved_on_ledge` is the saved situation. Package 3 calls it once per restore.
pub fn on_state_restored(saved_on_ledge: bool) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    if crate::runtime::macros::named_active() || (crate::runtime::macros::ready() && replay_preview::occupied()) {
        // The named layer owns the restore's follow-up (a named record or playback, or a clip preview's start).
        // SAFETY: both fighters were just restored.
        unsafe { named_macros::service_restore() };
        return;
    }
    let settings = *SETTINGS.lock();
    let action = Recorder::on_state_restored(&settings, saved_on_ledge, &mut RNG.lock());
    event(
        "state_restored",
        "ledge",
        i64::from(saved_on_ledge),
        "action",
        match action {
            LoadAction::None => 0,
            LoadAction::Record => 1,
            LoadAction::Play(slot) => 2 + i64::from(slot),
        },
    );
    // SAFETY: the fighters were just restored.
    unsafe {
        let Some(body) = cpu_body() else { return };
        if !load_consts() {
            return;
        }
        let facing = facing_of(natives::lr(body.accessor));
        match action {
            LoadAction::None => {}
            LoadAction::Record => begin_recording(&settings, body, facing),
            LoadAction::Play(slot) => begin_playback(slot, facing, false),
        }
    }
}

/// The recorder is in a phase other than Idle (the record lockout, standby, recording or a playback).
pub fn recording_or_playback() -> bool {
    REC.lock().phase() != Phase::Idle
}

/// The replay editor holds the input (`Owners::REPLAY_EDITOR`; the editor is the named-macro layer, S4-4, so
/// this is false until that layer exists and sets the owner bit).
pub fn replay_editor_occupied() -> bool {
    published::input_owners().contains(Owners::REPLAY_EDITOR)
}

/// A replay capture is loading (S4-4's clip loader; false until that layer exists).
pub fn replay_capture_loading() -> bool {
    false
}

/// Playback Mash Interrupt (SPEC-cpu-01 §3, decision B5): the CPU's mash asked the running playback to stop. A
/// recording in progress is not touched.
pub fn interrupt_playback() {
    let settings = *SETTINGS.lock();
    let (stopped, state) = {
        let mut rec = REC.lock();
        let stopped = rec.interrupt_on_mash(&settings);
        (stopped, rec.playback_state(false))
    };
    if stopped {
        // The same bookkeeping as `stop`, and the CPU sees the playback gone before the next injection.
        LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
        notices::post_phase(Phase::Idle, 0);
        cpu::set_playback(state);
        event("playback_interrupt", "mash", 1, "", 0);
    }
}
/// `stop_playback`: a save-state load request, a mash interrupt, the session end, a hot swap, a reset
/// (decision B8), the named layer.
pub fn stop() {
    let mode = REC.lock().mode();
    event("recorder_stop", "mode", mode as i64, "", 0);
    REC.lock().stop();
    named_macros::cancel_motion();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    notices::post_phase(Phase::Idle, 0);
}

/// A save-state load was accepted (package 3): everything stops, and the named layer plans what the restore will start (a record or a
/// playback after both fighters are back).
pub fn on_load_request() {
    stop();
    named_macros::begin_restore();
}

/// The Training reset hub's step: the L+R+A reset stops the recorder (decision B8).
pub fn on_reset() {
    stop();
}

/// Session clear: leaving Training, a hot swap, a lobby change.
pub fn session_clear() {
    REC.lock().stop();
    named_macros::clear_session();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    chords::session_clear();
    notices::session_clear();
    input_display::session_clear();
    // The menu's session ends with the same departure (package 5; the fork cleared its menu and chord state together).
    crate::runtime::ui::clear_session_state();
}
