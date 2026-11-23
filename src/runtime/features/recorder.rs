//! The input recorder, the runtime half (SPEC-input-01): the glue between the game and
//! `model::recording`.
//!
//! Entry points:
//! * `on_p1_poll` (from the P1 input poll, after the input delay): stores the frame while recording
//!   and neutralises P1's output while the recorder owns P1.
//! * `on_ai_frame` (the inline hook at the AI control function's epilogue): the CPU's control frame.
//!   Checks the controller is the CPU entry's own (rule R3: never Nana, never an AI-driven P1), starts a
//!   mash-triggered playback, injects the frame, runs the end-of-run check and the chords, applies the
//!   phase tint and posts the phase notice.
//! * `tint_player` (from Player 1's fighter frame, `cpu::fighter_frame`): Player 1's fighter is red while its own take is open.
//! * `request_playback` / `stop`: the entry points the CPU behaviours and hubs call.
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

use skyline::hooks::InlineCtx;

use super::recording_host;
use crate::engine::rr_host;

use crate::model::chords::Request;
use crate::model::cpu::facts::PlaybackStart;
use crate::model::cpu::ids::{C, UNRESOLVED};
use crate::model::cpu::role::FighterKey;
use crate::model::cpu::status;
use crate::model::input_delay::{MappedInputs, Owners};
use crate::model::recording::{record_chord, Inject, MashCtx, Phase, RecordRefusal, RecorderSettings};
use crate::model::sites_pkg4;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::features::{chords, cpu, hide_cpu, input_display, notices, states};
use crate::runtime::game::{self, natives};
use crate::runtime::macros::named_macros;
use crate::runtime::{published, scene};
use crate::runtime::ui::pilot_export;

/// The recorder's hook was installed with every row of its closure (`model::capabilities`, review R2-1): until
/// boot says so, the entry points the other features call (`on_p1_poll`, `request_playback`)
/// do nothing and read no native.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Boot: the recorder's closure passed its guards and `ai_epilogue` is installed.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}

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

/// The diag build counts the hook; the shipped build's body is empty.
fn bump(_which: usize) {
    #[cfg(feature = "diag")]
    counts::bump(_which);
}

fn report_counts() {
    #[cfg(feature = "diag")]
    counts::report_if_due();
}

const COUNT_POLL: usize = 0;
const COUNT_AI_FRAME: usize = 1;
const COUNT_INJECT: usize = 2;
const COUNT_NOTICE: usize = 3;

/// One recorder event line (diag build only: the shipped build writes nothing per event, playbook).
fn event(_name: &str, _key_a: &str, _a: i64, _key_b: &str, _b: i64) {
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] {_name} {_key_a}={_a} {_key_b}={_b}"));
}

/// The recorder rows as the last publish left them (the named layer reads Recording Trigger and Save State Playback).
/// Read wait-free from `published::Live` (stream/sync); the factory rows before the first publish.
pub fn settings() -> RecorderSettings {
    crate::runtime::published::live().map_or(RecorderSettings::factory(), |live| live.recorder)
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

/// A fighter entry's active body, resolved now (no pointer is kept between frames).
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

/// `module` is the practice CPU's own body: a practice scene is live (`scene::current`), and `module` is the active body of package 2's
/// primary CPU entry (`cpu::primary_entry_now`; never Nana, the entry's second body). The named layer's per-fighter work and the clip
/// preview run for this body only (the fork's `practice::primary_cpu`, which package 6 retired in favour of these owners).
///
/// # Safety
/// `module` is a live fighter's accessor; the package 4 natives must have passed their guards.
pub(crate) unsafe fn is_primary_cpu_body(module: &smash::app::BattleObjectModuleAccessor) -> bool {
    scene::current().is_some()
        && cpu_body().is_some_and(|body| body.accessor == module as *const smash::app::BattleObjectModuleAccessor as usize)
}

/// The active body of `entry_id` (0 is P1), using the guarded F11 bank reader.
///
/// # Safety
/// The package 4 natives must have passed their guards.
pub(crate) unsafe fn body_of(entry_id: i32) -> Option<Body> {
    let entry_ptr = natives::fighter_entry(entry_id);
    let fighter = natives::entry_active_body(entry_ptr);
    if fighter == 0 {
        return None;
    }
    let accessor = natives::fighter_accessor(fighter);
    (accessor != 0).then_some(Body { entry_ptr, fighter, accessor, entry_id })
}

pub(super) fn facing_of(lr: f32) -> i8 {
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
    // Nested in the input hook that calls it, so an allocation of the poll stage is charged here (R5 attribution).
    let _scope = HookScope::enter_hook(HookId::RecorderPoll);
    bump(COUNT_POLL);
    if SWALLOW.load(Ordering::Relaxed) > 0 {
        SWALLOW.fetch_sub(1, Ordering::Relaxed);
        *out = MappedInputs::NEUTRAL;
    }
    // CPU-role menu record owns this poll: lockout, standby, then the player's frame.
    if recording_host::cpu_role_poll(out) {
        return;
    }
    if !rr_host::slot_running() {
        recording_host::on_player_poll(out);
    }
}

// ---- the CPU control stage ----------------------------------------------------------------------------

/// The inline hook at the AI control function's epilogue: `x19` is the AI object and `*(x19 + 8)` the
/// controller it just wrote.
#[skyline::hook(offset = sites_pkg4::AI_EPILOGUE_OFFSET, inline)]
pub unsafe fn ai_epilogue(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter_hook(HookId::RecorderAi);
    on_ai_frame(ctx.registers[19].x() as usize);
}

unsafe fn on_ai_frame(ai: usize) {
    // Native Training or an active lobby practice (the hot swap check below stops the recorder).
    if ai == 0 {
        return;
    }
    let native_training = scene::is_native_training();
    if !(native_training || scene::lobby_term()) {
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
    // Rule R3: the recorder owns only the primary CPU entry's controller, never Nana's or P1's.
    let primary = cpu_body();
    let owns_controller = |entry_ptr: usize| {
        entry_ptr != 0 && ((entry_ptr + ENTRY_CONTROLLER_FIELD) as *const usize).read() == controller
    };
    let matched = primary.filter(|body| owns_controller(body.entry_ptr));
    // The waiting room has the one practice CPU: only native Training walks the other entries.
    crate::runtime::bridge_state::note_ai_think(matched.map(|body| body.entry_id), |entry| {
        native_training && owns_controller(natives::fighter_entry(entry))
    });
    let Some(body) = matched else {
        return;
    };
    let buttons = ((controller + CTRL_BUTTONS) as *const u32).read();
    if let Some(hold) = super::btt_camera::placement_pad(buttons) {
        write_controller(controller, Inject { buttons: (buttons & !hold.release) | hold.press, ..Inject::NEUTRAL });
        return;
    }
    if crate::model::edge_juggle::drill_owns_pad(
        super::edge_juggle::controls_cpu(body.accessor),
        busy_for_drill(),
    ) {
        let button_field = (controller + CTRL_BUTTONS) as *mut u32;
        let x_field = (controller + CTRL_STICK_X) as *mut f32;
        let y_field = (controller + CTRL_STICK_Y) as *mut f32;
        let buttons = button_field.read();
        let y = y_field.read();
        let stock = super::edge_juggle::StockPad {
            buttons,
            stick: [x_field.read(), y],
            clamped_x: ((controller + CTRL_CLAMPED_X) as *const f32).read(),
        };
        let correction = super::edge_juggle::ai_correction(ai, body.accessor, stock);
        if let Some([next_x, next_y]) = correction.stick {
            x_field.write(next_x);
            ((controller + CTRL_CLAMPED_X) as *mut f32).write(next_x);
            if next_y != y {
                y_field.write(next_y);
                ((controller + CTRL_CLAMPED_Y) as *mut f32).write(next_y);
            }
        }
        if correction.press != 0 || correction.release != 0 {
            button_field.write((buttons & !correction.release) | correction.press);
        }
        return;
    }
    if !load_consts() {
        return;
    }
    if super::mashout::owns_cpu(body.accessor) {
        write_controller(controller, super::mashout::control());
        return;
    }
    bump(COUNT_AI_FRAME);
    let settings = settings();
    let key = FighterKey::new(body.entry_id, false);
    let facing = facing_of(natives::lr(body.accessor));
    let kind = ((body.fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let dial = FLAG_SHULK_DIAL.load(Ordering::Acquire);
    let shulk = kind == KIND_SHULK.load(Ordering::Acquire) && dial != UNRESOLVED && game::natives::is_flag(body.accessor, dial);

    // A playback the mash queue asked for starts before this frame's injection (§5.2 step 2).
    if let Some(key) = key {
        mash_start(&settings, key, body, facing);
    }

    let control = rr_host::slot_control(shulk);
    if let Some(refusal) = rr_host::take_slot_refusal() {
        notices::refuse(refusal.text());
    }
    if let Some(write) = control {
        bump(COUNT_INJECT);
        write_controller(controller, write);
    }
    let role = recording_host::cpu_role_control();
    if let Some(write) = role.write {
        bump(COUNT_INJECT);
        write_controller(controller, write);
    }
    // Aerial Drift: on the mashed aerial's airborne frames the rolled drift holds the stick x the AI just wrote.
    if let (None, None, Some(key)) = (control, role.write, key) {
        let cpu_x = game::natives::pos_x(body.accessor);
        if let Some(drift) = cpu::aerial_drift(key, cpu_x, f32::from(facing)) {
            let stick = drift.unwrap_or(0.0) as i64;
            event("aerial_drift", "x", cpu_x as i64, "stick", stick);
            if let Some(x) = drift {
                ((controller + CTRL_STICK_X) as *mut u32).write(x.to_bits());
                ((controller + CTRL_CLAMPED_X) as *mut u32).write(x.to_bits());
            }
        }
    }
    if role.hold_cliff_timer {
        let rest = INT_CLIFF_REST.load(Ordering::Acquire);
        if rest != UNRESOLVED
            && game::natives::status_kind(body.accessor) == STATUS_CLIFF_WAIT.load(Ordering::Acquire)
            && natives::get_int(body.accessor, rest) < crate::model::recording::CLIFF_REST_TIMER_LOW
        {
            natives::set_int(body.accessor, crate::model::recording::CLIFF_REST_TIMER, rest);
        }
    }

    let queue_head = key.and_then(cpu::mash_head);
    if let Some(slot) = rr_host::slot_post() {
        event("playback_complete", "slot", i64::from(slot), "", 0);
        if queue_head.is_some() {
            if let Some(key) = key {
                cpu::mash_pop(key);
            }
        }
    }
    chord_step(body);

    // The phase: tint every frame it lasts, the notice once per change.
    let (phase, length) = if let Some(phase) = recording_host::cpu_role_phase() {
        (phase, 0)
    } else {
        rr_host::slot_phase()
    };
    if let Some(rgb) = phase.tint() {
        tint(body.accessor, rgb);
    }
    if LAST_PHASE.swap(phase as u8, Ordering::AcqRel) != phase as u8 {
        notices::post_phase(phase, length);
    }
    let state = recording_host::cpu_playback(queue_head.is_some());
    cpu::set_playback(state);
    bump(COUNT_NOTICE);
    host_cpu(controller);
    report_counts();
}

/// One frame of a recorded fighter's colour-blend tint: written on every frame it lasts, as the phase tint always was.
unsafe fn tint(accessor: usize, [r, g, b]: [f32; 3]) {
    let blend = MODEL_COLOR_BLEND.load(Ordering::Acquire);
    if blend != UNRESOLVED {
        natives::set_color_rgb(accessor, r, g, b, blend);
    }
}

/// Player 1's fighter frame in a practice scene (`cpu::fighter_frame`, entry 0; Training and the waiting room): on every
/// frame of Player 1's take (`rr_host::player_tint`) entry 0's active body is the Player role's red, never Nana, as the
/// CPU's. It does not depend on the CPU's AI frame, which a drill, a placement or Mash Out skips.
///
/// # Safety
/// `accessor` is the live accessor of the entry-0 fighter whose frame just ran.
pub(crate) unsafe fn tint_player(accessor: usize) {
    if !ENABLED.load(Ordering::Acquire) {
        return;
    }
    let Some(rgb) = recording_host::player_tint() else { return };
    if load_consts() && natives::module_accessor(0) == Some(accessor) {
        tint(accessor, rgb);
    }
}

unsafe fn write_controller(controller: usize, write: Inject) {
    let at = |offset: usize| (controller + offset) as *mut u32;
    at(CTRL_BUTTONS).write(write.buttons);
    at(CTRL_STICK_X).write(write.stick_x.to_bits());
    at(CTRL_STICK_Y).write(write.stick_y.to_bits());
    at(CTRL_CLAMPED_X).write(write.clamped_x.to_bits());
    at(CTRL_CLAMPED_Y).write(write.clamped_y.to_bits());
}

unsafe fn host_cpu(controller: usize) {
    // CPU-role recording captures the player poll; this controller receives CPU playback.
    // The macro owns the pad through the start restore, including before its first queued frame.
    if let Some(write) = crate::engine::rr_host::cpu_controller(recording_host::take_cpu_frame) {
        write_controller(controller, write);
    }
}

unsafe fn mash_start(settings: &RecorderSettings, key: FighterKey, body: Body, facing: i8) {
    let idle = rr_host::cpu_idle();
    if !idle {
        return;
    }
    let Some(head) = cpu::mash_head(key) else { return };
    let Some(take) = rr_host::slot_take(head) else { return };
    let Some(table) = cpu::ids() else { return };
    let status = game::natives::status_kind(body.accessor);
    let in_hitstun = status::in_hitstun(&table, status);
    let in_hitstop = natives::is_stop(body.accessor);
    let needs = crate::model::recording::mash_needs_term(idle, settings, Some(head), in_hitstun);
    let term_enabled = needs
        && match crate::model::recording::term_for_status(&table, take.status()) {
            Some(c) => {
                let term = table.get(c);
                term != UNRESOLVED && game::natives::is_enable_transition_term(body.accessor, term)
            }
            None => cpu::actionable(body.accessor),
        };
    let start = crate::model::recording::mash_should_start(
        idle,
        settings,
        &MashCtx { head_slot: Some(head), in_hitstun, in_hitstop, term_enabled },
    );
    let Some(slot) = start else { return };
    let result = rr_host::slot_start_mash(slot, take, facing);
    match result {
        Ok(Some(started)) => {
            event("playback_mash", "slot", i64::from(started.slot), "frames", i64::from(started.frames));
        }
        Ok(None) => {}
        Err(refusal) => {
            // The queue would ask again every frame: drop the head and say why.
            cpu::mash_pop(key);
            notices::refuse(refusal.text());
        }
    }
}

/// Record, Play on Player 1, and Play on CPU. The hook only posts the flags.
unsafe fn chord_step(body: Body) {
    let play_player = chords::consume(Request::PlayPlayer);
    let playback = chords::consume(Request::InputPlayback);
    let record = chords::consume(Request::InputRecord);
    if super::record_place::wizard_active() {
        return;
    }
    let playback = playback && !hide_cpu::refuses_play();
    if states::is_loading() {
        if record || playback || play_player {
            event("chord_ignored_loading", "record", i64::from(record), "playback", i64::from(playback));
        }
        return;
    }
    // rc.9 C14: Play on Player 1 is refused while an AI drives Player 1 (the same check as Record's B4).
    let play_player = match crate::model::recording::play_player_chord(play_player, || {
        crate::model::recording::p1_driven_by_ai(
            natives::is_operation_cpu(0),
            crate::runtime::p1::cpu_pilot::emitting(),
            pilot_export(),
        )
    }) {
        Ok(play) => play,
        Err(text) => {
            notices::refuse(text);
            event("play_player_refused", "reason", 1, "", 0);
            false
        }
    };
    // A Record that would start the worker's capture asks the same refusals as every other record start (B4).
    let record = match record_chord(record, recording_host::capturing(), || record_refusal()) {
        Ok(record) => record,
        Err(refusal) => {
            notices::refuse(refusal.text());
            let reason = refusal as i64;
            event("record_refused", "reason", reason, "", 0);
            false
        }
    };
    let record = match crate::model::record_place::chord(record, recording_host::capturing(), settings().place && super::record_place::installed()) {
        crate::model::record_place::Chord::Pass(record) => record,
        crate::model::record_place::Chord::Wizard => {
            super::record_place::open();
            false
        }
    };
    recording_host::on_chords(body, record, play_player, playback);
    recording_host::cpu_role_after_chord();
}

unsafe fn begin_playback(slot: u8, facing: i8, ledge: bool) {
    let catch = ledge && STATUS_CLIFF_CATCH.load(Ordering::Acquire) != UNRESOLVED && cpu_body().is_some_and(|b| {
        game::natives::status_kind(b.accessor) == STATUS_CLIFF_CATCH.load(Ordering::Acquire)
    });
    let result = rr_host::slot_start_ledge(slot, facing, ledge.then_some(catch));
    match result {
        Ok(Some(started)) => event("playback_shortcut", "slot", i64::from(started.slot), "frames", i64::from(started.frames)),
        Ok(None) => {}
        Err(refusal) => notices::refuse(refusal.text()),
    }
}

/// Why a recording (numbered or named) is not started now: B4 an AI drives P1, a fighter is missing, a save state is loading.
/// The order lives in the model (`record_refusal`, host-tested); these are the live facts, all guarded per-frame reads. B4's AI
/// fact includes the standalone pilot, read through its own export (`p1_driven_by_ai`).
unsafe fn record_refusal() -> Option<RecordRefusal> {
    crate::model::recording::record_refusal(
        crate::model::recording::p1_driven_by_ai(
            natives::is_operation_cpu(0),
            crate::runtime::p1::cpu_pilot::emitting(),
            pilot_export(),
        ),
        natives::fighter_entry(0) != 0,
        states::is_loading(),
    )
}

/// The CPU's fighter kind now (diag lines of the named layer's roster row).
#[cfg(feature = "diag")]
pub fn cpu_kind() -> Option<i32> {
    crate::runtime::ui::facts::fact_cpu_kind()
}

/// The recorder's phase (diag lines: its one caller is the diag build's named-layer harness, `macros::diag`, so the function is not in
/// the shipped NRO).
#[cfg(feature = "diag")]
pub fn phase() -> Phase {
    recording_host::cpu_role_phase().unwrap_or_else(|| rr_host::slot_phase().0)
}

/// P1's next `frames` polls read as neutral (see `SWALLOW`).
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

/// The recorder is in a phase other than Idle (the record lockout, standby, recording or a playback).
pub fn recording_or_playback() -> bool {
    rr_host::slot_running() || recording_host::taking()
}

pub(super) fn busy_for_drill() -> bool {
    recording_or_playback() || recording_host::cpu_playback_blocks_drill()
}

/// The replay editor holds the input (`Owners::REPLAY_EDITOR`; the editor is the named-macro layer, S4-4, so
/// this is false until that layer exists and sets the owner bit).
pub fn replay_editor_occupied() -> bool {
    let owners = published::input_owners();
    owners.contains(Owners::REPLAY_EDITOR) || owners.contains(Owners::REPLAY_LOADING)
}

/// A replay capture is loading (S4-4's clip loader; false until that layer exists).
pub fn replay_capture_loading() -> bool {
    false
}

/// Playback Mash Interrupt (SPEC-cpu-01 §3, decision B5): the CPU's mash asked the running playback to stop. A
/// recording in progress is not touched.
pub fn interrupt_playback() {
    // restriction: parry-mash-ignored
    if super::parry::on() {
        return;
    }
    recording_host::note_mash();
    let stopped = settings().mash_interrupt && rr_host::slot_stop();
    // The CPU sees the playback gone before the next injection, also when only the Recording page's run stopped (REC-027).
    cpu::set_playback(recording_host::cpu_playback(false));
    if stopped {
        // The same bookkeeping as `stop`.
        LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
        notices::post_phase(Phase::Idle, 0);
        event("playback_interrupt", "mash", 1, "", 0);
    }
}
/// A save-state load request, mash interrupt, session end, hot swap or reset (decision B8).
pub fn stop() {
    event("recorder_stop", "running", i64::from(rr_host::slot_running()), "", 0);
    rr_host::slot_stop();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    notices::post_phase(Phase::Idle, 0);
}

/// A save-state load was accepted (package 3).
pub fn on_load_request() {
    stop();
    // SAV-002: the Recording pages macros play in rr_host, which stop does not reach.
    recording_host::note_load();
}

/// The Training reset hub's step: the L+R+A reset stops the recorder (decision B8).
pub fn on_reset(cause: crate::model::input_delay::Clear) {
    crate::engine::rr_host::clear_playback(cause);
    stop();
}

/// Session clear: leaving Training, a hot swap, a lobby change.
pub fn session_clear() {
    super::record_place::session_clear();
    rr_host::slot_stop();
    named_macros::clear_session();
    LAST_PHASE.store(Phase::Idle as u8, Ordering::Release);
    chords::session_clear();
    notices::session_clear();
    input_display::session_clear();
    super::parry::reset();
    // The menu's session ends with the same departure (package 5; the fork cleared its menu and chord state together).
    crate::runtime::ui::clear_session_state();
}
