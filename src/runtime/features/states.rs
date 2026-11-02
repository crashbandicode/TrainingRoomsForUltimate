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
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use parking_lot::Mutex;

use crate::model::cpu::hash::hash40;
use crate::model::cpu::rng::Rng;
use crate::model::input_delay::Clear;
use crate::model::lra_reset;
use crate::model::sites_pkg3 as s;
use crate::model::state::buff::{BuffFacts, BuffKind, BuffState};
use crate::model::state::charge;
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
use crate::runtime::features::{reset_hub, state_effects, state_recorder, state_restore};
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives as n;
use crate::runtime::hub;

/// Everything the per-frame step owns, behind one lock.
struct State {
    machine: Machine,
    settings: StateSettings,
    rng: Rng,
    ids: Table,
    /// The two roles' fighter kinds as last seen (an item that needs a fighter's data is granted only
    /// when one of them is that fighter).
    last_kinds: [i32; 2],
    /// One buff bookkeeping per record (SPEC-state-03 OQ 5): player, CPU.
    buff: [BuffState; 2],
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

/// Set by boot once every site of the save states passed its guard and the hooks are installed; the
/// frame step does nothing before that (it calls the natives unguarded).
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_active(on: bool) {
    ACTIVE.store(on, Ordering::Release);
}

static CONSTS_READY: AtomicBool = AtomicBool::new(false);
static CONST_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static NANA_KIND: AtomicI32 = AtomicI32::new(UNRESOLVED);
static QUAKE_NONE: AtomicI32 = AtomicI32::new(UNRESOLVED);
static QUAKE_MAX: AtomicI32 = AtomicI32::new(UNRESOLVED);
static JOSTLE_ID: AtomicI32 = AtomicI32::new(UNRESOLVED);
static SHULK_FLASH_ID: AtomicI32 = AtomicI32::new(UNRESOLVED);
/// The accessor of the fighter each record is buffing right now (0 when none), and whether it is Wii Fit:
/// the effect, sound, heal and wing hooks read these without the lock.
static BUFF_ACC: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
static BUFF_WIIFIT: [AtomicBool; 2] = [const { AtomicBool::new(false) }; 2];
/// A Little Mac is in the match, and KO Punch is selected (the meter hook).
static MAC_PRESENT: AtomicBool = AtomicBool::new(false);
static KO_SELECTED: AtomicBool = AtomicBool::new(false);
static JOKER_SUMMON: AtomicI32 = AtomicI32::new(UNRESOLVED);
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
    *STATE.lock() = Some(State { machine: Machine::new(file), settings, rng: Rng::new(seed), ids: Table::unresolved(), last_kinds: [-1, -1], buff: [BuffState::new(); 2] });
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
    KO_SELECTED.store(settings.buffs.has(crate::model::state::settings::Buffs::KO), Ordering::Release);
}

fn any_buffing() -> bool {
    BUFF_ACC.iter().any(|a| a.load(Ordering::Acquire) != 0)
}

fn fighter_buffing(accessor: usize) -> bool {
    accessor != 0 && BUFF_ACC.iter().any(|a| a.load(Ordering::Acquire) == accessor)
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
    with(|state| {
        state.machine.cancel_all();
        state.buff.iter_mut().for_each(BuffState::clear);
    });
    LOADING.store(false, Ordering::Release);
    KILLING.store(false, Ordering::Release);
    MAC_PRESENT.store(false, Ordering::Release);
    for slot in &BUFF_ACC {
        slot.store(0, Ordering::Release);
    }
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
    JOKER_SUMMON.store(table.get(C::JokerStatusSummon), Ordering::Release);
    state_restore::set_pikmin_statuses(table.get(C::PikminStatusAttackAir), table.get(C::PikminStatusAttackHi4Landing));
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
    fighter: usize,
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
        fighter,
    };

    #[cfg(feature = "diag")]
    if DUMP.load(Ordering::Acquire) {
        diagnostic(format_args!(
            "[TrainingRooms] state pos entry={} kind={} x={} y={} lr={} dmg={} status={} situation={}",
            raw.entry,
            raw.kind,
            natives::pos_x(accessor),
            n::pos_y(accessor),
            n::lr(accessor),
            natives::damage(accessor, 0),
            raw.status,
            raw.situation
        ));
        if raw.entry == 1 && !nana {
            DUMP.store(false, Ordering::Release);
        }
    }
    let save_chord = SAVE_REQUEST.swap(false, Ordering::AcqRel);
    // The Load chord is read only on a non-CPU-operated, non-Nana fighter's callback.
    let load_chord = !nana && !reset_hub::cpu_operated(fighter) && LOAD_REQUEST.swap(false, Ordering::AcqRel);
    let stage_id = n::stage_id();
    let out = with(|state| {
        let ids = &state.ids;
        if raw.entry <= 1 && !nana {
            state.last_kinds[raw.entry as usize] = raw.kind;
        }
        if ids.is(C::KindLittleMac, raw.kind) {
            MAC_PRESENT.store(true, Ordering::Release);
        }
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
        // A new load: no buff carries over.
        with(|state| state.buff.iter_mut().for_each(BuffState::clear));
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
        finish(raw, &done);
    }
    if let Some(marker) = out.wait {
        run_wait(raw, marker);
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
unsafe fn finish(raw: &Raw, done: &Done) {
    let accessor = raw.accessor;
    if let Some(target) = done.damage {
        n::heal(accessor, -natives::damage(accessor, 0));
        n::add_damage(accessor, target);
    }
    if done.reset_sound {
        n::play_se_no3d(accessor, hash40("se_system_position_reset"));
    }
    // Charges (SPEC-state-02): only when the saved kind equals the current kind (the machine checked).
    if let Some(saved) = &done.charge {
        if let Some(ids) = with(|s| s.ids.clone()) {
            let plan = charge::plan_write(&ids, raw.kind, saved);
            state_effects::run_charge(accessor, raw.fighter, &ids, &plan);
        }
    }
    // Steve's materials and tools (SPEC-state-05 §5.4), only for Steve.
    if let Some(steve) = &done.steve {
        if let Some(ids) = with(|s| s.ids.clone()) {
            if ids.is(C::KindSteve, raw.kind) {
                state_restore::steve_write(accessor, &ids, steve);
            }
        }
    }
    // The player's held-item snapshot (SPEC-state-04 §5.1): the player record only, never Nana.
    if let Some(item) = &done.item {
        if let Some(ids) = with(|s| s.ids.clone()) {
            let other = with(|s| s.last_kinds).unwrap_or([-1, -1]);
            state_restore::restore_item(accessor, &ids, item, other);
        }
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

/// `WorkModule` reads for the charge table.
struct Fields<'a>(usize, &'a Table);

impl charge::Work for Fields<'_> {
    fn olimar_colours(&self) -> [Option<i32>; 3] {
        // SAFETY: only used from the frame step with a live fighter's accessor.
        unsafe { state_restore::olimar_line(self.0, self.1) }
    }
    fn get_int(&self, id: i32) -> i32 {
        // SAFETY: only used from the frame step with a live fighter's accessor.
        unsafe { n::work_get_int(self.0, id) }
    }
    fn get_float(&self, id: i32) -> f32 {
        unsafe { natives::get_float(self.0, id) }
    }
    fn get_flag(&self, id: i32) -> bool {
        unsafe { natives::is_flag(self.0, id) }
    }
}

/// A feature's wait (SPEC-state-01 §5.4): `ApplyBuff` runs the buff step; the Pokemon switch and the
/// Kirby copy are stepped by `state_restore` (milestone 3) and end at once until then.
unsafe fn run_wait(raw: &Raw, marker: Marker) {
    let Some(role) = Role::of_entry(raw.entry) else { return };
    match marker {
        Marker::ApplyBuff => apply_buff(raw, role),
        _ => {
            with(|s| s.machine.finish_wait(role));
        }
    }
}

/// One `ApplyBuff` frame for the fighter of `raw` (SPEC-state-03 §5).
unsafe fn apply_buff(raw: &Raw, role: Role) {
    let accessor = raw.accessor;
    // Facts first, with no lock held.
    let ids = with(|s| s.ids.clone());
    let Some(ids) = ids else { return };
    let Some(kind) = state_effects::buff_kind(&ids, raw.kind) else {
        with(|s| s.machine.finish_wait(role));
        return;
    };
    let is = |c: C, status: i32| ids.is(c, status);
    let facts = BuffFacts {
        hero_cast_now: is(C::HeroStatusSpecialLwStart, raw.status),
        hero_cast_before: is(C::HeroStatusSpecialLwStart, raw.prev_status),
        shulk_action_now: is(C::ShulkStatusAction, raw.status),
        shulk_action_before: is(C::ShulkStatusAction, raw.prev_status),
        wii_success_now: is(C::WiiFitStatusSuccess, raw.status),
        wii_success_before: is(C::WiiFitStatusSuccess, raw.prev_status),
        wing_activated: ids.resolved(C::SephirothOneWinged) && natives::is_flag(accessor, ids.get(C::SephirothOneWinged)),
    };
    let out = with(|s| {
        let buffs = s.settings.buffs;
        s.buff[role.index()].step(kind, buffs, &facts, &mut s.rng)
    });
    let Some(out) = out else { return };
    let slot = role.index();
    BUFF_ACC[slot].store(if out.buffing { accessor } else { 0 }, Ordering::Release);
    BUFF_WIIFIT[slot].store(out.buffing && kind == BuffKind::WiiFit, Ordering::Release);
    state_effects::run_buff(accessor, raw.fighter, raw.entry, &ids, &out);
    if out.done {
        with(|s| s.machine.finish_wait(role));
        state_diag!("[TrainingRooms] state buff_done entry={} kind={:?}", raw.entry, kind);
    }
}

/// The Save step (§5.5 step 2): read the values, store them, write the file when both are done.
unsafe fn capture(raw: &Raw, nana: bool, slot: usize) {
    if nana {
        return;
    }
    let Some(role) = Role::of_entry(raw.entry) else { return };
    let accessor = raw.accessor;
    let ids = with(|s| s.ids.clone());
    let state = SavedState {
        x: natives::pos_x(accessor),
        y: n::pos_y(accessor),
        percent: natives::damage(accessor, 0),
        lr: n::lr(accessor),
        situation_kind: raw.situation,
        marker: Marker::NoAction,
        fighter_kind: raw.kind,
        charge: ids.as_ref().map(|ids| charge::read(ids, raw.kind, &Fields(accessor, ids))).unwrap_or_default(),
        steve: ids.as_ref().filter(|ids| ids.is(C::KindSteve, raw.kind)).and_then(|ids| state_restore::steve_read(accessor, ids)),
        // The player's held item (the CPU stores none).
        held_item: ids.as_ref().filter(|_| role == Role::Player).map(|ids| state_restore::item_snapshot(accessor, ids)),
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
    if !ACTIVE.load(Ordering::Acquire) || !ensure_consts() {
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
    // A buffing fighter keeps its visuals (SPEC-state-03 §5.2).
    if hooks::effect_hidden(LOADING.load(Ordering::Acquire), fighter_buffing(accessor)) && game::is_training_mode() {
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

// ---- the buff hooks (SPEC-state-03 §4) --------------------------------------------------------------

/// `FighterSpecializer_Jack::check_doyle_summon_dispatch`: no summon cutscene while buffing.
#[skyline::hook(offset = s::STATE_JOKER_SUMMON_OFFSET)]
pub unsafe fn state_joker_summon_hook(accessor: usize, a: bool, b: bool) -> u32 {
    let _scope = HookScope::enter();
    let result = call_original!(accessor, a, b);
    let summon = JOKER_SUMMON.load(Ordering::Acquire);
    if summon != UNRESOLVED && result == summon as u32 && fighter_buffing(accessor) && game::is_training_mode() {
        return 0xFFFF_FFFF;
    }
    result
}

/// The shared UI-update routine (entry data, value): Little Mac's KO meter reads 100 while a load with
/// KO Punch runs (whichever Little Mac is in the match).
#[skyline::hook(offset = s::STATE_MAC_METER_OFFSET)]
pub unsafe fn state_mac_meter_hook(entry: usize, value: i32) -> u64 {
    let _scope = HookScope::enter();
    if LOADING.load(Ordering::Acquire) && KO_SELECTED.load(Ordering::Acquire) && MAC_PRESENT.load(Ordering::Acquire) && game::is_training_mode() {
        return call_original!(entry, 100);
    }
    call_original!(entry, value)
}

/// The damage module's add-damage routine (module, amount, kind): Wii Fit does not heal while buffing.
#[skyline::hook(offset = s::STATE_WIIFIT_HEAL_OFFSET)]
pub unsafe fn state_wiifit_heal_hook(module: usize, amount: f32, kind: i32) -> u64 {
    let _scope = HookScope::enter();
    if module != 0 && BUFF_WIIFIT.iter().any(|w| w.load(Ordering::Acquire)) {
        let accessor = ((module + 8) as *const usize).read();
        let wii_buffing = BUFF_ACC.iter().zip(BUFF_WIIFIT.iter()).any(|(a, w)| w.load(Ordering::Acquire) && a.load(Ordering::Acquire) == accessor);
        if wii_buffing && game::is_training_mode() {
            return call_original!(module, 0.0, kind);
        }
    }
    call_original!(module, amount, kind)
}

/// `EffectModule::req_screen`: Sephiroth's wing background is replaced by `bg` while he is buffed.
#[skyline::hook(offset = s::STATE_REQ_SCREEN_OFFSET)]
pub unsafe fn state_req_screen_hook(accessor: usize, hash: u64, a: bool, b: bool, c: bool) -> u64 {
    let _scope = HookScope::enter();
    const WING_BACKGROUND: u64 = 0x10DCB5700E;
    if hash == WING_BACKGROUND && fighter_buffing(accessor) && game::is_training_mode() {
        return call_original!(accessor, hash40("bg"), a, b, c);
    }
    call_original!(accessor, hash, a, b, c)
}

/// The fighter sound module's `play_se`: every sound is `se_silent` while a record is buffing. The hash is
/// the first argument after the module (the register order of the `lua_bind` thunk this one sits under).
#[skyline::hook(offset = s::STATE_FIGHTER_PLAY_SE_OFFSET)]
pub unsafe fn state_fighter_play_se_hook(a0: usize, a1: u64, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::SOUND);
    if any_buffing() && game::is_training_mode() {
        return call_original!(a0, HASHES[H_SE_SILENT].load(Ordering::Acquire), a2, a3, a4, a5, a6, a7);
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

/// Diag build: dump both fighters' positions at their next frames.
#[cfg(feature = "diag")]
static DUMP: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "diag")]
fn request_dump() {
    DUMP.store(true, Ordering::Release);
}

/// Diag build: the Eden tests drive the Save and Load chords by creating request files in the profile's
/// `TrainingRooms` folder (`diag-save.req`, `diag-load.req`), which this thread consumes. Not in the
/// shipped NRO.
#[cfg(feature = "diag")]
pub fn start_diag_triggers() {
    let _ = std::thread::Builder::new().name("tr-state-diag".into()).stack_size(64 * 1024).spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // A settings file dropped by the test script replaces the live settings (the menu is not built yet).
        if let Some(path) = FixedPath::<128>::in_root("diag-settings.json") {
            if let Ok(bytes) = std::fs::read(path.as_str()) {
                let _ = std::fs::remove_file(path.as_str());
                let loaded = crate::model::settings::parse_bytes(&bytes);
                crate::runtime::published::publish(&loaded.settings.menu, 0);
                diagnostic(format_args!("[TrainingRooms] state diag_settings applied issues={}", loaded.issues.len()));
            }
        }
        for (name, action) in [
            ("diag-save.req", request_save as fn()),
            ("diag-load.req", request_load as fn()),
            ("diag-pos.req", request_dump as fn()),
        ] {
            if let Some(path) = FixedPath::<128>::in_root(name) {
                if std::fs::remove_file(path.as_str()).is_ok() {
                    action();
                    diagnostic(format_args!("[TrainingRooms] state diag_request {name}"));
                }
            }
        }
    });
}
