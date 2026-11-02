//! Save states, the runtime half (package 3, SPEC-state-01): the glue between the game and
//! `model::state`.
//!
//! Contract (the same as the CPU behaviours'): every hook first reads what it needs from the game with no
//! lock held (a game call can re-enter one of these hooks), then takes the one `STATE` lock for the pure
//! decision, releases it, and applies the answer. Nothing here allocates in a hook (the counting allocator's
//! `HookScope` wraps every entry point); the slot file is read at boot and written by its own thread.
//!
//! * `on_fighter_frame` runs once per fighter per game frame, called from the shared per-frame wrapper
//!   (`cpu::on_frame`) after the game's own frame work.
//! * The hooks (`get_param_int`, `set_dead_rumble`, `req_quake`, `play_se`, `effect_req`,
//!   `effect_req_follow`, the star-KO query) make a load silent and instant (SPEC-state-01 §4.1); each
//!   reads two flags the frame step publishes (`LOADING`, `KILLING`) and the three settings bits.
//! * Chords: pkg4's chord engine calls `request_save` / `request_load`; a diag build also reads request
//!   files from the Eden profile (`diag_triggers`).
//!
//! NOT YET SWITCH-RUN: compiled for the Switch, not run on Eden or the console.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::model::cpu::hash::hash40;
use crate::model::cpu::rng::Rng;
use crate::model::input_delay::Clear;
use crate::model::lra_reset;
use crate::model::sites_pkg3 as s;
use crate::model::state::file::SaveFile;
use crate::model::state::hooks::{self, DeathParam};
use crate::model::state::ids::{self, Table, C, UNRESOLVED};
use crate::model::state::machine::{Done, FrameIn, Machine, Out, Placing, Request, Role, StatusNow, Trigger};
use crate::model::state::persist;
use crate::model::state::record::{Marker, SavedState};
use crate::model::state::settings::StateSettings;
use crate::model::storage::{self, FixedPath};
use crate::runtime::alloc::HookScope;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{reset_hub, state_recorder};
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives as n;
use crate::runtime::hub;

/// Everything the per-frame step owns, behind one lock.
struct State {
    machine: Machine,
    settings: StateSettings,
    rng: Rng,
    ids: Table,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// A load is in progress / killing (the hooks read these without the lock).
static LOADING: AtomicBool = AtomicBool::new(false);
static KILLING: AtomicBool = AtomicBool::new(false);
/// The settings bits the hooks need (`StateSettings::enable`, `autoload`, `respawn_invincibility`).
static ENABLE: AtomicBool = AtomicBool::new(true);
static AUTOLOAD: AtomicBool = AtomicBool::new(false);
static RESPAWN_INVINCIBILITY: AtomicBool = AtomicBool::new(false);
/// The chord latches: set by the chord engine, consumed once by the first eligible fighter callback.
static SAVE_REQUEST: AtomicBool = AtomicBool::new(false);
static LOAD_REQUEST: AtomicBool = AtomicBool::new(false);
/// The slot file needs writing (the writer thread takes it).
static DIRTY: AtomicBool = AtomicBool::new(false);

static CONSTS_READY: AtomicBool = AtomicBool::new(false);
static CONST_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static NANA_KIND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static QUAKE_NONE: AtomicI32 = AtomicI32::new(UNRESOLVED);
static QUAKE_MAX: AtomicI32 = AtomicI32::new(UNRESOLVED);
static JOSTLE_ID: AtomicI32 = AtomicI32::new(UNRESOLVED);
static SHULK_FLASH_ID: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// `phx::Hash40` of the names the hooks compare (set once at `init`).
static HASHES: [AtomicU64; 10] = [const { AtomicU64::new(0) }; 10];
const H_COMMON: usize = 0;
const H_DEAD_REBIRTH_WAIT: usize = 1;
const H_REBIRTH_MOVE: usize = 2;
const H_REBIRTH_WAIT: usize = 3;
const H_REBIRTH_INVINCIBLE: usize = 4;
const H_REBIRTH_INVINCIBLE_ADD: usize = 5;
const H_REBIRTH_MOVE_TRAINER: usize = 6;
const H_PARAM_MBALL: usize = 7;
const H_CHANGE_FLY: usize = 8;
const H_SE_SILENT: usize = 9;
/// `sys_score_aura`, kept apart (a hook compares it on every follow request).
static SCORE_AURA: AtomicU64 = AtomicU64::new(0);

const CONST_ATTEMPT_LIMIT: u32 = 900;

/// The fighter object's layout (as in `cpu.rs`): kind +0xc, entry id +0x10, module accessor +0x20.
const FIGHTER_KIND_FIELD: usize = 0xc;
const FIGHTER_ENTRY_FIELD: usize = lra_reset::FIGHTER_ENTRY_ID_FIELD;
const FIGHTER_ACCESSOR_FIELD: usize = 0x20;

/// How often each hook ran (the diag build writes the counts: review R5 wants the hook activity seen).
#[cfg(feature = "diag")]
mod counts {
    use std::sync::atomic::{AtomicU32, Ordering};

    pub const FRAME: usize = 0;
    pub const PARAM: usize = 1;
    pub const QUAKE: usize = 2;
    pub const SOUND: usize = 3;
    pub const EFFECT: usize = 4;
    pub const FOLLOW: usize = 5;
    pub const RUMBLE: usize = 6;
    pub const STAR_KO: usize = 7;
    static COUNTS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
    const REPORT_EVERY: u32 = 600;

    pub fn bump(which: usize) {
        COUNTS[which].fetch_add(1, Ordering::Relaxed);
    }

    pub fn report_if_due() {
        if COUNTS[FRAME].load(Ordering::Relaxed) < REPORT_EVERY {
            return;
        }
        let take = |i: usize| COUNTS[i].swap(0, Ordering::Relaxed);
        crate::runtime::diag_log::diagnostic(format_args!(
            "[TrainingRooms] state_hook_counts frames={} param={} quake={} sound={} effect={} follow={} rumble={} star_ko={} heap_allocations={}",
            take(FRAME),
            take(PARAM),
            take(QUAKE),
            take(SOUND),
            take(EFFECT),
            take(FOLLOW),
            take(RUMBLE),
            take(STAR_KO),
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

mod counts_index {
    pub const PARAM: usize = 1;
    pub const QUAKE: usize = 2;
    pub const SOUND: usize = 3;
    pub const EFFECT: usize = 4;
    pub const FOLLOW: usize = 5;
    pub const RUMBLE: usize = 6;
    pub const STAR_KO: usize = 7;
}

/// A line for the diag build only: the shipped NRO writes the install line and nothing else.
macro_rules! state_diag {
    ($($arg:tt)*) => {
        #[cfg(feature = "diag")]
        diagnostic(format_args!($($arg)*));
    };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.lock().as_mut().map(f)
}

/// Create the state at boot (before any hook is installed): the slot table, the settings and the
/// generator seed.
pub fn init(settings: StateSettings, file: SaveFile, seed: u64) {
    *STATE.lock() = Some(State { machine: Machine::new(file), settings, rng: Rng::new(seed), ids: Table::unresolved() });
    set_flags(&settings);
    for (slot, name) in HASHES.iter().zip([
        "common",
        "dead_rebirth_wait_frame",
        "rebirth_move_frame",
        "rebirth_wait_frame",
        "rebirth_invincible_frame",
        "rebirth_invincible_add_frame",
        "rebirth_move_frame_trainer",
        "param_mball",
        "change_fly_frame",
        "se_silent",
    ]) {
        slot.store(hash40(name), Ordering::Release);
    }
    SCORE_AURA.store(hash40("sys_score_aura"), Ordering::Release);
}

fn set_flags(settings: &StateSettings) {
    ENABLE.store(settings.enable, Ordering::Release);
    AUTOLOAD.store(settings.autoload, Ordering::Release);
    RESPAWN_INVINCIBILITY.store(settings.respawn_invincibility, Ordering::Release);
}

/// A menu commit or the first publish. Turning Enable Save States Off cancels every load in progress
/// (SPEC-state-01 §5.6).
pub fn publish(settings: StateSettings) {
    set_flags(&settings);
    with(|state| {
        if state.settings.enable && !settings.enable {
            state.machine.cancel_all();
            LOADING.store(false, Ordering::Release);
            KILLING.store(false, Ordering::Release);
        }
        state.settings = settings;
    });
}

/// The chord engine's calls (package 4): the Save and Load chords passed.
pub fn request_save() {
    SAVE_REQUEST.store(true, Ordering::Release);
}

pub fn request_load() {
    LOAD_REQUEST.store(true, Ordering::Release);
}

/// Cancel every pending load (scene departure, hot swap: SPEC-state-01 §6.11).
pub fn cancel_pending() {
    with(|state| state.machine.cancel_all());
    LOADING.store(false, Ordering::Release);
    KILLING.store(false, Ordering::Release);
}

pub fn is_loading() -> bool {
    LOADING.load(Ordering::Acquire)
}

pub fn is_killing() -> bool {
    KILLING.load(Ordering::Acquire)
}

/// What the hooks need of the settings: the three bits, as a `StateSettings`.
fn hook_settings() -> StateSettings {
    StateSettings {
        enable: ENABLE.load(Ordering::Acquire),
        autoload: AUTOLOAD.load(Ordering::Acquire),
        respawn_invincibility: RESPAWN_INVINCIBILITY.load(Ordering::Acquire),
        ..StateSettings::default()
    }
}

/// Ask the game's table for every constant, once. Called from the frame step outside its `HookScope`.
fn ensure_consts() -> bool {
    if CONSTS_READY.load(Ordering::Acquire) {
        return true;
    }
    if CONST_ATTEMPTS.fetch_add(1, Ordering::Relaxed) >= CONST_ATTEMPT_LIMIT {
        return false;
    }
    let mut table = Table::unresolved();
    // SAFETY: only reached from the frame step, installed after `lua_const_lookup` passed its guard.
    let missing = table.resolve(|key| unsafe { game::lua_const(key) });
    if missing == ids::COUNT {
        return false; // the game's table is not built yet; ask again next frame
    }
    NANA_KIND.store(table.get(C::KindNana), Ordering::Release);
    QUAKE_NONE.store(table.get(C::QuakeKindNone), Ordering::Release);
    QUAKE_MAX.store(table.get(C::QuakeKindMax), Ordering::Release);
    JOSTLE_ID.store(table.get(C::WorkIntJostle), Ordering::Release);
    SHULK_FLASH_ID.store(table.get(C::WorkIntShulkFlash), Ordering::Release);
    if missing > 0 {
        let mut names = FixedPath::<400>::new();
        for name in table.missing().take(8) {
            let _ = std::fmt::Write::write_fmt(&mut names, format_args!("{name} "));
        }
        diagnostic(format_args!("[TrainingRooms] state constants missing={missing}: {}", names.as_str()));
    }
    with(|state| state.ids = table);
    CONSTS_READY.store(true, Ordering::Release);
    true
}

// ---- the per-fighter frame --------------------------------------------------------------------------

/// The raw facts of one fighter body, read from the game with no lock held.
struct Raw {
    entry: i32,
    kind: i32,
    accessor: usize,
    status: i32,
    prev_status: i32,
    situation: i32,
}

/// Run the save-state step for one fighter (the object the game just ran a frame of). Called by the
/// shared frame wrapper inside its hook scope.
///
/// # Safety
/// `fighter` is the fighter object pointer the game passed to its on-frame function.
pub unsafe fn on_fighter_frame(fighter: usize) {
    if fighter == 0 || !CONSTS_READY.load(Ordering::Acquire) {
        return;
    }
    let entry = ((fighter + FIGHTER_ENTRY_FIELD) as *const i32).read();
    let accessor = ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read();
    if accessor == 0 || Role::of_entry(entry).is_none() {
        return;
    }
    if !game::is_training_mode() {
        if LOADING.load(Ordering::Acquire) {
            cancel_pending();
        }
        return;
    }
    if !ENABLE.load(Ordering::Acquire) {
        return;
    }
    #[cfg(feature = "diag")]
    if entry == 0 && ((fighter + FIGHTER_KIND_FIELD) as *const i32).read() != NANA_KIND.load(Ordering::Acquire) {
        counts::bump(counts::FRAME);
    }
    let kind = ((fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let nana = kind == NANA_KIND.load(Ordering::Acquire);
    let raw = Raw {
        entry,
        kind,
        accessor,
        status: natives::status_kind(accessor),
        prev_status: natives::prev_status_kind(accessor, 0),
        situation: natives::situation_kind(accessor),
    };
    let save_chord = SAVE_REQUEST.swap(false, Ordering::AcqRel);
    // The Load chord is read only on a non-CPU-operated, non-Nana fighter's callback.
    let load_chord = !nana && !reset_hub::cpu_operated(fighter) && LOAD_REQUEST.swap(false, Ordering::AcqRel);
    let stage_id = n::stage_id();
    let out = with(|state| {
        let ids = &state.ids;
        let dead = ids.is(C::StatusDead, raw.status) || ids.is(C::StatusStandby, raw.status);
        let input = FrameIn {
            entry: raw.entry,
            nana,
            popo: ids.is(C::KindPopo, raw.kind),
            kind: raw.kind,
            dead,
            status: if ids.is(C::StatusWait, raw.status) {
                StatusNow::Wait
            } else if ids.is(C::StatusFall, raw.status) {
                StatusNow::Fall
            } else if ids.is(C::StatusCliffCatch, raw.status) || ids.is(C::StatusCliffCatchMove, raw.status) {
                StatusNow::CliffCatch
            } else {
                StatusNow::Other
            },
            prev_rebirth: ids.is(C::StatusRebirth, raw.prev_status),
            standby: ids.is(C::StatusStandby, raw.status),
            stage_id,
            situation: ids.situations(),
            save_chord,
            load_chord,
            kirby: ids.is(C::KindKirby, raw.kind),
            pokemon: ids.is(C::KindPokemonSquirtle, raw.kind) || ids.is(C::KindPokemonIvysaur, raw.kind) || ids.is(C::KindPokemonCharizard, raw.kind),
            buffable: ids.is_buffable(raw.kind),
        };
        let settings = state.settings;
        let out = state.machine.frame(&input, &settings, &mut state.rng);
        LOADING.store(state.machine.is_loading(), Ordering::Release);
        KILLING.store(state.machine.is_killing(), Ordering::Release);
        (out, input.situation.classify(raw.situation))
    });
    if let Some((out, _)) = out {
        apply(&raw, nana, out);
    }
    #[cfg(feature = "diag")]
    counts::report_if_due();
}

/// Do what the frame asked, with no lock held.
unsafe fn apply(raw: &Raw, nana: bool, out: Out) {
    let accessor = raw.accessor;
    if out.stop_playback {
        state_recorder::stop();
        state_diag!("[TrainingRooms] state load_request entry={}", raw.entry);
    }
    if out.force_dead {
        natives::change_status_force(accessor, with(|s| s.ids.get(C::StatusDead)).unwrap_or(UNRESOLVED));
        state_diag!("[TrainingRooms] state kill entry={}", raw.entry);
    }
    if out.cleanup {
        respawn_cleanup(accessor, raw.kind);
        state_diag!("[TrainingRooms] state cleanup entry={}", raw.entry);
    }
    if let Some(placing) = out.placing {
        place(accessor, &placing);
        if out.run_hub {
            hub::run(Clear::SaveStateLoad);
        }
        if placing.request.is_none() {
            state_diag!("[TrainingRooms] state placed entry={} x={} y={} lr={}", raw.entry, placing.x, placing.y, placing.lr);
        }
    }
    if let Some(done) = out.done {
        finish(accessor, &done);
    }
    if let Some(slot) = out.capture {
        capture(raw, nana, slot);
    }
    if let Some(slot) = out.saved_notice {
        state_diag!("[TrainingRooms] state saved_notice slot={}", slot + 1);
        #[cfg(not(feature = "diag"))]
        let _ = slot;
    }
    if let Some(ledge) = out.restored {
        state_recorder::on_state_restored(ledge);
        state_diag!("[TrainingRooms] state restored ledge={ledge}");
    }
    match out.trigger {
        Some(Trigger::Record) => state_recorder::start_record(),
        Some(Trigger::Playback(slot)) => state_recorder::start_playback(slot),
        None => {}
    }
}

/// The respawn cleanup (SPEC-state-01 §5.2 and §4): stop sounds, remove every article not on the
/// keep-list and every item in the match, put animcmd to sleep, pause SE, stop rumble, clear slow and
/// zoom, kill the critical-hit screen effects and stop every quake.
unsafe fn respawn_cleanup(accessor: usize, kind: i32) {
    n::stop_all_sound(accessor);
    for article in 0..ids::ARTICLE_COUNT {
        if !n::article_is_exist(accessor, article) {
            continue;
        }
        if with(|state| state.ids.keeps_article(kind, article)).unwrap_or(false) {
            continue;
        }
        let object = n::article_get(accessor, article);
        if object != 0 {
            n::article_remove_object(accessor, n::article_object_id(object));
        }
    }
    let manager = n::item_manager();
    if manager != 0 {
        // Collect the ids first: removing shifts the active list.
        let mut found = [0u32; 64];
        let count = (n::item_active_count(manager) as usize).min(found.len());
        for (index, slot) in found.iter_mut().enumerate().take(count) {
            let item = n::item_active(manager, index as u64);
            if item != 0 {
                *slot = n::item_object_id(item);
            }
        }
        for id in found.iter().take(count).filter(|id| **id != 0) {
            n::item_remove(manager, *id);
        }
    }
    n::set_sleep(accessor, true);
    n::pause_se_all(accessor, true);
    n::stop_rumble(accessor, true);
    n::slow_clear_whole(accessor);
    n::camera_zoom_out(accessor, 0);
    n::effect_kill_kind(accessor, hash40("sys_bg_criticalhit"));
    n::effect_remove_screen(accessor, hash40("bg_criticalhit"));
    let (first, last) = (QUAKE_NONE.load(Ordering::Acquire), QUAKE_MAX.load(Ordering::Acquire));
    if first != UNRESOLVED && last != UNRESOLVED {
        for kind in first..=last {
            n::camera_stop_quake(accessor, kind);
        }
    }
}

/// One placement frame (SPEC-state-01 §5.3 steps 2 to 7).
unsafe fn place(accessor: usize, placing: &Placing) {
    n::stop_all_sound(accessor);
    n::set_sleep(accessor, false);
    n::pause_se_all(accessor, false);
    n::stop_rumble(accessor, false);
    n::clear_speed_all(accessor);
    let pos = n::Vector3f { x: placing.x, y: placing.y, z: 0.0 };
    if placing.grounded {
        n::init_pos(accessor, &pos);
    } else {
        n::set_pos(accessor, &pos);
    }
    n::set_lr(accessor, placing.lr);
    if let Some(request) = placing.request {
        let wanted = with(|s| match request {
            Request::Wait => s.ids.get(C::StatusWait),
            Request::Fall => s.ids.get(C::StatusFall),
        })
        .unwrap_or(UNRESOLVED);
        if wanted != UNRESOLVED {
            n::status_request(accessor, wanted);
        }
    }
}

/// A fighter has just been placed (§5.3 step 8): the damage, then what the milestones add (charges,
/// buffs, the item snapshot, the fighter-specific restores).
unsafe fn finish(accessor: usize, done: &Done) {
    if let Some(target) = done.damage {
        n::heal(accessor, -natives::damage(accessor, 0));
        n::add_damage(accessor, target);
    }
    if done.reset_sound {
        n::play_se_no3d(accessor, hash40("se_system_position_reset"));
    }
    state_diag!(
        "[TrainingRooms] state done damage={:?} item={} charge={} steve={} next={}",
        done.damage,
        done.item.is_some(),
        done.charge.is_some(),
        done.steve.is_some(),
        done.next.name()
    );
}

/// The Save step (§5.5 step 2): read the values, store them, write the file when both are done.
unsafe fn capture(raw: &Raw, nana: bool, slot: usize) {
    if nana {
        return;
    }
    let Some(role) = Role::of_entry(raw.entry) else { return };
    let accessor = raw.accessor;
    let state = SavedState {
        x: natives::pos_x(accessor),
        y: n::pos_y(accessor),
        percent: natives::damage(accessor, 0),
        lr: n::lr(accessor),
        situation_kind: raw.situation,
        marker: Marker::NoAction,
        fighter_kind: raw.kind,
        charge: Default::default(),
        steve: None,
        held_item: None,
        saved_stage: Some(n::stage_id()),
        mii: None,
    };
    let write = with(|s| s.machine.capture(role, slot, state)).unwrap_or(false);
    // The save flash.
    let attributes = with(|s| {
        [C::EffectNoJointScale, C::EffectFollow, C::EffectConcludeStatus]
            .iter()
            .fold(0u32, |mask, c| mask | s.ids.get(*c) as u32)
    })
    .unwrap_or(0);
    natives::req_on_joint(accessor, hash40("sys_deku_flash"), hash40("top"), 0.25, attributes);
    if write {
        DIRTY.store(true, Ordering::Release);
        state_diag!("[TrainingRooms] state saved slot={} entry={} (file queued)", slot + 1, raw.entry);
    }
}

/// The per-frame wrapper's one call (`cpu::on_frame`): constants first, then the fighter.
pub fn frame(fighter: usize) {
    if !ensure_consts() {
        return;
    }
    // SAFETY: `fighter` is the object the game passed to its on-frame function.
    unsafe { on_fighter_frame(fighter) };
}

// ---- the hooks of SPEC-state-01 §4.1 ----------------------------------------------------------------

fn param_of(kind: u64, name: u64) -> Option<DeathParam> {
    let h = |i: usize| HASHES[i].load(Ordering::Acquire);
    if kind == h(H_COMMON) {
        return Some(match name {
            n if n == h(H_DEAD_REBIRTH_WAIT) => DeathParam::DeadRebirthWaitFrame,
            n if n == h(H_REBIRTH_MOVE) => DeathParam::RebirthMoveFrame,
            n if n == h(H_REBIRTH_WAIT) => DeathParam::RebirthWaitFrame,
            n if n == h(H_REBIRTH_INVINCIBLE) => DeathParam::RebirthInvincibleFrame,
            n if n == h(H_REBIRTH_INVINCIBLE_ADD) => DeathParam::RebirthInvincibleAddFrame,
            n if n == h(H_REBIRTH_MOVE_TRAINER) => DeathParam::RebirthMoveFrameTrainer,
            _ => return None,
        });
    }
    if kind == h(H_PARAM_MBALL) && name == h(H_CHANGE_FLY) {
        return Some(DeathParam::ChangeFlyFrame);
    }
    None
}

/// `WorkModule::get_param_int`: the respawn parameters of a death.
#[skyline::hook(offset = s::STATE_GET_PARAM_INT_OFFSET)]
pub unsafe fn state_get_param_int_hook(accessor: usize, kind: u64, name: u64) -> i32 {
    let _scope = HookScope::enter();
    bump(counts_index::PARAM);
    if let Some(param) = param_of(kind, name) {
        if let Some(value) = hooks::param_override(param, &hook_settings(), LOADING.load(Ordering::Acquire), game::is_training_mode()) {
            if param == DeathParam::DeadRebirthWaitFrame {
                // The jostle and Shulk cleanups that ride on this override (SPEC-state-01 §4).
                let jostle = JOSTLE_ID.load(Ordering::Acquire);
                if jostle != UNRESOLVED && n::work_get_int(accessor, jostle) > 1 {
                    n::work_set_int(accessor, 1, jostle);
                }
                let flash = SHULK_FLASH_ID.load(Ordering::Acquire);
                if flash != UNRESOLVED {
                    n::work_set_int(accessor, 0, flash);
                    n::effect_remove_common(accessor, hash40("monad_arts_damage_buster"));
                    n::effect_remove_common(accessor, hash40("monad_arts_damage_smash"));
                }
            }
            return value;
        }
    }
    call_original!(accessor, kind, name)
}

/// `sv_fighter_util::set_dead_rumble`: no death rumble in Training.
#[skyline::hook(offset = s::STATE_SET_DEAD_RUMBLE_OFFSET)]
pub unsafe fn state_set_dead_rumble_hook(lua_state: usize) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::RUMBLE);
    if ENABLE.load(Ordering::Acquire) && game::is_training_mode() {
        return 0;
    }
    call_original!(lua_state)
}

/// `CameraModule::req_quake`: no quake while killing.
#[skyline::hook(offset = s::STATE_REQ_QUAKE_OFFSET)]
pub unsafe fn state_req_quake_hook(accessor: usize, kind: i32) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::QUAKE);
    let none = QUAKE_NONE.load(Ordering::Acquire);
    if none != UNRESOLVED && hooks::quake_suppressed(KILLING.load(Ordering::Acquire)) && game::is_training_mode() {
        return call_original!(accessor, none);
    }
    call_original!(accessor, kind)
}

/// `SoundModule::play_se`: `se_silent` while killing.
#[skyline::hook(offset = s::STATE_PLAY_SE_OFFSET)]
pub unsafe fn state_play_se_hook(accessor: usize, hash: u64, a: bool, b: bool, c: bool, d: bool, kind: u32) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::SOUND);
    if hooks::sound_silenced(KILLING.load(Ordering::Acquire)) && game::is_training_mode() {
        return call_original!(accessor, HASHES[H_SE_SILENT].load(Ordering::Acquire), a, b, c, d, kind);
    }
    call_original!(accessor, hash, a, b, c, d, kind)
}

/// `EffectModule::req`: size 0 while a load is in progress (throw explosions, the Ice Climbers' squall).
#[skyline::hook(offset = s::STATE_EFFECT_REQ_OFFSET)]
pub unsafe fn state_effect_req_hook(
    accessor: usize,
    effect: u64,
    pos: *const n::Vector3f,
    rot: *const n::Vector3f,
    size: f32,
    a: u32,
    b: i32,
    c: bool,
    d: i32,
) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::EFFECT);
    // Buffing fighters (package 3 milestone 2) keep their visuals; none is recorded yet.
    if hooks::effect_hidden(LOADING.load(Ordering::Acquire), false) && game::is_training_mode() {
        return call_original!(accessor, effect, pos, rot, 0.0, a, b, c, d);
    }
    call_original!(accessor, effect, pos, rot, size, a, b, c, d)
}

/// `EffectModule::req_follow`: `sys_score_aura` at size 0 while a load is in progress.
#[skyline::hook(offset = s::STATE_EFFECT_REQ_FOLLOW_OFFSET)]
pub unsafe fn state_effect_req_follow_hook(
    accessor: usize,
    effect: u64,
    joint: u64,
    pos: *const n::Vector3f,
    rot: *const n::Vector3f,
    size: f32,
    a: bool,
    b: u32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: bool,
    h: bool,
) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::FOLLOW);
    if effect == SCORE_AURA.load(Ordering::Acquire) && hooks::score_aura_hidden(LOADING.load(Ordering::Acquire)) && game::is_training_mode() {
        return call_original!(accessor, effect, joint, pos, rot, 0.0, a, b, c, d, e, f, g, h);
    }
    call_original!(accessor, effect, joint, pos, rot, size, a, b, c, d, e, f, g, h)
}

/// The stage's "can star KO" query: false in Training, so a death is followed by the respawn at once.
#[skyline::hook(offset = s::STATE_NO_STAR_KO_OFFSET)]
pub unsafe fn state_no_star_ko_hook(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::STAR_KO);
    if ENABLE.load(Ordering::Acquire) && game::is_training_mode() {
        return 0;
    }
    call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
}

// ---- the slot file ----------------------------------------------------------------------------------

fn save_path() -> Option<FixedPath<128>> {
    FixedPath::<128>::in_root(storage::file::SAVE_STATES)
}

/// Read the slot file (the boot calls this on a thread with an explicit stack). A missing or unusable
/// file gives the defaults; an unusable one is copied aside, never moved (review R1).
pub fn load_file() -> SaveFile {
    let Some(path) = save_path() else { return SaveFile::default() };
    let (file, outcome) = persist::load(Path::new(path.as_str()));
    match outcome {
        persist::Outcome::Read | persist::Outcome::Missing => {}
        persist::Outcome::Defaulted { why, kept } => {
            diagnostic(format_args!("[TrainingRooms] save_states.json unusable ({why:?}); defaults, copy kept={kept:?}"));
        }
        persist::Outcome::Unreadable => {
            diagnostic(format_args!("[TrainingRooms] save_states.json unreadable; defaults, file untouched"));
        }
    }
    file
}

/// The writer thread: whenever a Save marked the file dirty, write the whole table through the atomic
/// replace. A failed write keeps the old file and is retried (the file stays dirty).
pub fn start_writer() {
    let spawned = std::thread::Builder::new().name("tr-state-writer".into()).stack_size(128 * 1024).spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if !DIRTY.swap(false, Ordering::AcqRel) {
            continue;
        }
        let Some(path) = save_path() else { continue };
        let Some(file) = STATE.lock().as_ref().map(|state| state.machine.file.clone()) else { continue };
        if let Err(error) = persist::save(Path::new(path.as_str()), &file) {
            DIRTY.store(true, Ordering::Release);
            diagnostic(format_args!("[TrainingRooms] save_states.json write failed: {error}; the old file stays"));
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            state_diag!("[TrainingRooms] state file_written");
        }
    });
    if spawned.is_err() {
        diagnostic(format_args!("[TrainingRooms] save-state writer thread did not start"));
    }
}

/// Diag build: the Eden tests drive the Save and Load chords by creating request files in the profile's
/// `TrainingRooms` folder (`diag-save.req`, `diag-load.req`), which this thread consumes. Not in the
/// shipped NRO.
#[cfg(feature = "diag")]
pub fn start_diag_triggers() {
    let _ = std::thread::Builder::new().name("tr-state-diag".into()).stack_size(64 * 1024).spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        for (name, action) in [("diag-save.req", request_save as fn()), ("diag-load.req", request_load as fn())] {
            if let Some(path) = FixedPath::<128>::in_root(name) {
                if std::fs::remove_file(path.as_str()).is_ok() {
                    action();
                    diagnostic(format_args!("[TrainingRooms] state diag_request {name}"));
                }
            }
        }
    });
}
