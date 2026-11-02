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

use std::sync::atomic::{AtomicI32, AtomicU8, Ordering};

use parking_lot::Mutex;
use skyline::hooks::InlineCtx;

use crate::model::chords::Request;
use crate::model::cpu::facts::PlaybackStart;
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::cpu::rng::Rng;
use crate::model::cpu::role::FighterKey;
use crate::model::cpu::status;
use crate::model::input_delay::{MappedInputs, Owners};
use crate::model::lra_reset;
use crate::model::recording::{
    ChordAction, ControlCtx, Ended, MashCtx, Mode, Phase, PlayRefusal, PollFacts, PostCtx, RecordRefusal, Recorder,
    RecorderSettings, StartSnapshot,
};
#[cfg(feature = "diag")]
use crate::model::recording::LoadAction;
use crate::model::settings::Snapshot;
use crate::model::sites;
use crate::model::sites_pkg4;
use crate::runtime::alloc::HookScope;
use crate::runtime::features::{chords, cpu, input_display, notices};
use crate::runtime::{game, published};

static REC: Mutex<Recorder> = Mutex::new(Recorder::new());
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

/// Fighter object and entry layout (read from the owned 13.0.5 `main`; `tests/input_sites.rs` and the
/// on-init notes): the fighter kind at +0xc, the module accessor at +0x20; the entry's body pointers
/// from +0x4160 and its controller at +0x4178.
pub(crate) const FIGHTER_KIND_FIELD: usize = 0xc;
const FIGHTER_ACCESSOR_FIELD: usize = 0x20;
const ENTRY_BODY_FIELD: usize = 0x4160;
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

/// Boot: the generator's seed (the plugin's own, never the game's, SPEC-input-01 question 12).
pub fn init(seed: u64) {
    *RNG.lock() = Rng::new(seed);
}

/// A menu commit or the first publish.
pub fn publish(snapshot: &Snapshot) {
    *SETTINGS.lock() = RecorderSettings::from_snapshot(snapshot);
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

/// The lua_bind natives of package 4 (guarded `Called` rows of `sites_pkg4`).
pub(crate) mod natives {
    use super::{game, sites, sites_pkg4};

    unsafe fn at<F: Copy>(offset: usize) -> F {
        debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<usize>());
        std::mem::transmute_copy(&(game::text_base() + offset))
    }

    fn offset(name: &str) -> usize {
        sites_pkg4::SITES.iter().find(|s| s.name == name).map_or(0, |s| s.offset)
    }

    pub unsafe fn lr(accessor: usize) -> f32 {
        at::<extern "C" fn(usize) -> f32>(offset("posture_lr"))(accessor)
    }

    pub unsafe fn is_stop(accessor: usize) -> bool {
        at::<extern "C" fn(usize) -> bool>(offset("stop_is_stop"))(accessor)
    }

    pub unsafe fn get_int(accessor: usize, id: i32) -> i32 {
        at::<extern "C" fn(usize, i32) -> i32>(offset("work_get_int"))(accessor, id)
    }

    pub unsafe fn set_int(accessor: usize, id: i32, value: i32) {
        at::<extern "C" fn(usize, i32, i32)>(offset("work_set_int"))(accessor, value, id)
    }

    pub unsafe fn set_color_rgb(accessor: usize, r: f32, g: f32, b: f32, kind: i32) {
        at::<extern "C" fn(usize, f32, f32, f32, i32)>(offset("model_set_color_rgb"))(accessor, r, g, b, kind)
    }

    /// `FighterManager::get_fighter_entry(manager, entry id)`.
    pub unsafe fn fighter_entry(entry: i32) -> usize {
        let manager = ((game::text_base() + super::lra_reset::FIGHTER_MANAGER_SLOT_OFFSET) as *const usize).read_volatile();
        if manager == 0 {
            return 0;
        }
        at::<extern "C" fn(usize, i32) -> usize>(offset("fighter_manager_get_entry"))(manager, entry)
    }

    /// `FighterInformation::is_operation_cpu` of an entry (the game's own operator flag).
    pub unsafe fn operated_by_cpu(entry: i32) -> bool {
        let manager = ((game::text_base() + super::lra_reset::FIGHTER_MANAGER_SLOT_OFFSET) as *const usize).read_volatile();
        if manager == 0 {
            return false;
        }
        let information: extern "C" fn(usize, i32) -> usize = std::mem::transmute(game::text_base() + sites::FIGHTER_INFORMATION_OFFSET);
        let information = information(manager, entry);
        if information == 0 {
            return false;
        }
        let is_cpu: extern "C" fn(usize) -> bool = std::mem::transmute(game::text_base() + sites::IS_OPERATION_CPU_OFFSET);
        is_cpu(information)
    }
}

/// A fighter entry's first body, resolved now (no pointer is kept between frames).
#[derive(Clone, Copy)]
pub(crate) struct Body {
    entry_ptr: usize,
    pub(crate) fighter: usize,
    pub(crate) accessor: usize,
    entry_id: i32,
}

unsafe fn cpu_body() -> Option<Body> {
    body_of(i32::from(cpu::primary_entry_now()))
}

/// The first body of ntry_id (0 is P1).
///
/// # Safety
/// The package 4 natives must have passed their guards.
pub(crate) unsafe fn body_of(entry_id: i32) -> Option<Body> {
    let entry_ptr = natives::fighter_entry(entry_id);
    if entry_ptr == 0 {
        return None;
    }
    let fighter = ((entry_ptr + ENTRY_BODY_FIELD) as *const usize).read();
    if fighter == 0 {
        return None;
    }
    let accessor = ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read();
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
    bump(COUNT_POLL);
    if REC.lock().mode() == Mode::None {
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

// ---- the CPU control stage ----------------------------------------------------------------------------

/// The inline hook at the AI control function's epilogue: `x19` is the AI object and `*(x19 + 8)` the
/// controller it just wrote.
#[skyline::hook(offset = sites_pkg4::AI_EPILOGUE_OFFSET, inline)]
pub unsafe fn ai_epilogue(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    on_ai_frame(ctx.registers[19].x() as usize);
}

unsafe fn on_ai_frame(ai: usize) {
    if ai == 0 || !game::is_training_mode() {
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

    let control = REC.lock().control(&ControlCtx { facing, shulk_dial: shulk });
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
    if let Some(Ended::Recorded { frames, .. }) = post.ended {
        crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] recording_complete frames={frames}"));
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
    // Interim HUD tick (package 5 moves these two calls to the layout's tick).
    input_display::tick_frame();
    let owners = published::input_owners();
    input_display::hud_tick(owners.contains(Owners::MENU) || owners.contains(Owners::VANILLA_PAUSE));
    notices::hud_tick();
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
            crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] playback_mash slot={} frames={}", started.slot, started.frames));
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
    // Eden stand-ins (diag build only, removed when packages 3 and 5 own these): the save-state chords stand
    // for "a save state was restored" and for the menu's shortcut edit.
    #[cfg(feature = "diag")]
    {
        if chords::consume(Request::SaveState) {
            on_state_restored(false);
        }
        if chords::consume(Request::LoadState) {
            let result = chords::commit_shortcut(1, crate::model::menu::chord::Chord((1 << 2) | (1 << 3)));
            crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] standin shortcut_edit slot=2 chord=X+Y result={result:?}"));
        }
    }
    let playback = chords::consume(Request::InputPlayback);
    let record = chords::consume(Request::InputRecord);
    if cpu::save_loading() {
        // Both chords are consumed and ignored while a save state loads (§6.4).
        return;
    }
    let record = record && settings.trigger_chord;
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
            crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] playback_shortcut slot={} frames={}", started.slot, started.frames));
        }
        Err(refusal) => notices::refuse(refusal.text()),
    }
}

unsafe fn begin_recording(settings: &RecorderSettings, body: Body, facing: i8) {
    // B4: while an AI drives P1 the recorder is refused with a notice, nothing is recorded.
    let refusal = if natives::operated_by_cpu(0) {
        Some(RecordRefusal::P1DrivenByAi)
    } else if natives::fighter_entry(0) == 0 {
        Some(RecordRefusal::FightersMissing)
    } else if cpu::save_loading() {
        Some(RecordRefusal::SaveStateLoading)
    } else {
        None
    };
    if let Some(refusal) = refusal {
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
    crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] record_lockout cpu_kind={kind} frames={}", settings.duration));
}

// ---- entry points for the other features ---------------------------------------------------------------

/// A playback the CPU behaviours ask for (the ledge options, SPEC-cpu-03): `FrameOut::playback`.
pub fn request_playback(start: PlaybackStart) {
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
/// settings. `saved_on_ledge` is the saved situation. Package 3 will call it once per restore; until that
/// caller exists the Eden stand-in below does (diag build only), so the release build has no dead code.
#[cfg(feature = "diag")]
pub fn on_state_restored(saved_on_ledge: bool) {
    let settings = *SETTINGS.lock();
    let action = Recorder::on_state_restored(&settings, saved_on_ledge, &mut RNG.lock());
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

/// `stop_playback`: a save-state load request, a mash interrupt, the session end, a hot swap, a reset
/// (decision B8), the named layer.
pub fn stop() {
    REC.lock().stop();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    notices::post_phase(Phase::Idle, 0);
}

/// The Training reset hub's step: the L+R+A reset stops the recorder (decision B8).
pub fn on_reset() {
    stop();
}

/// Session clear: leaving Training, a hot swap, a lobby change.
pub fn session_clear() {
    REC.lock().stop();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    chords::session_clear();
    notices::session_clear();
    input_display::session_clear();
}
