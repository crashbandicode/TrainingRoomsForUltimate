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
//! * Chords: this step pulls the Save and Load chords from package 4's chord engine and calls `request_save` /
//!   `request_load` (a diag build also reads request files from the Eden profile, `diag_triggers`).
//!
//! NOT YET SWITCH-RUN: compiled for the Switch, not run on Eden or the console.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use hos_lock::Mutex;
use skyline::hooks::InlineCtx;

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
use crate::model::state::machine::{Done, FrameIn, Machine, Out, Placing, Request, Role, StatusNow};
use crate::model::state::persist;
use crate::model::state::pokemon::{self, Descriptor, Kinds, KirbyKinds, PtWait, Sound, SwitchEffect};
use crate::model::state::record::{Marker, SavedState};
use crate::model::state::settings::StateSettings;
use crate::model::storage::{self, FixedPath};
use crate::runtime::alloc::HookScope;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, recorder, reset_hub, state_effects, state_restore};
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives as n;
use crate::runtime::hub;
use crate::runtime::scene;

mod quick_reset;
mod preview;
pub(crate) mod setups;
pub(crate) use quick_reset::on_input as on_quick_reset_input;
pub(crate) use quick_reset::request as request_quick_reset;
pub(crate) use preview::{request as request_preview_restore, finish as finish_preview_restore, end as end_preview_restore};

/// Everything the per-frame step owns, behind one lock.
struct State {
    machine: Machine,
    saved_miis: [[Option<crate::engine::mii::lifetime::Configuration>; 5]; 2],
    settings: StateSettings,
    rng: Rng,
    ids: Table,
    /// The two roles' fighter kinds as last seen (an item that needs a fighter's data is granted only
    /// when one of them is that fighter).
    last_kinds: [i32; 2],
    /// One buff bookkeeping per record (SPEC-state-03 OQ 5): player, CPU.
    buff: [BuffState; 2],
    /// One trainer-switch wait per record (SPEC-state-05 open question 2): player, CPU.
    pt_wait: [PtWait; 2],
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// A load is in progress / killing (the hooks read these without the lock).
static LOADING: AtomicBool = AtomicBool::new(false);
static KILLING: AtomicBool = AtomicBool::new(false);
static TEMPORARY: AtomicBool = AtomicBool::new(false);
static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);
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
static HASHES: [AtomicU64; 15] = [const { AtomicU64::new(0) }; 15];
const H_FLYING_PLATE: usize = 10;
const H_CHANGE_LIGHT: usize = 11;
const H_SE_CHANGE_APPEAR: usize = 12;
const H_SE_BALL_OPEN: usize = 13;
const H_SE_BALL_SWING: usize = 14;

/// The game constants the Pokemon Trainer, Kirby and Sora rules need without the lock (hooks and the
/// per-frame dead test read these; set once by `ensure_consts`).
static KIDS: [AtomicI32; K_COUNT] = [const { AtomicI32::new(UNRESOLVED) }; K_COUNT];
const K_SQUIRTLE: usize = 0;
const K_IVYSAUR: usize = 1;
const K_CHARIZARD: usize = 2;
const K_WEAPON_PTRAINER: usize = 3;
const K_WEAPON_MBALL: usize = 4;
const K_TRAIL_N1: usize = 5;
const K_KIRBY_DRINK: usize = 6;
const K_PT_RUN: usize = 7;
const K_PT_RESTART: usize = 8;
const K_STATUS_WAIT: usize = 9;
const K_LINK_TRAINER: usize = 10;
const K_PT_FLAG: usize = 11;
const K_PT_MBALL: usize = 12;
const K_ALL_TARGET: usize = 13;
const K_SIT_GROUND: usize = 14;
const K_SIT_AIR: usize = 15;
const K_COPY_ON_START: usize = 16;
const K_COPY_FLAG: usize = 17;
const K_COPY_CHARA: usize = 18;
const K_STATUS_NONE: usize = 19;
const K_COUNT: usize = 20;

fn kid(index: usize) -> i32 {
    KIDS[index].load(Ordering::Acquire)
}

fn is_pokemon_kind(kind: i32) -> bool {
    [K_SQUIRTLE, K_IVYSAUR, K_CHARIZARD].iter().any(|k| kid(*k) == kind && kid(*k) != UNRESOLVED)
}

/// The fighter objects of the two records (accessor and entry id), refreshed every frame: Kirby's hat
/// restore reads the opponent's.
static FIGHTER_ACC: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
static FIGHTER_ENTRY: [AtomicI32; 2] = [const { AtomicI32::new(-1) }; 2];
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
    /// Trainer, Kirby and Sora sounds silenced while loading / matching sounds that passed while loading.
    pub const SE_SILENCED: usize = 8;
    pub const SE_PASSED: usize = 9;
    /// Switch effects made size 0, decide hook rewrites, Kirby copy hook writes.
    pub const SWITCH_FX: usize = 10;
    pub const DECIDE: usize = 11;
    pub const COPY_HOOK: usize = 12;
    static COUNTS: [AtomicU32; 13] = [const { AtomicU32::new(0) }; 13];
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
            "[TrainingRooms] state_hook_counts frames={} param={} quake={} sound={} effect={} follow={} rumble={} star_ko={} se_silenced={} se_passed={} switch_fx={} decide={} copy_hook={} heap_allocations={}",
            take(FRAME),
            take(PARAM),
            take(QUAKE),
            take(SOUND),
            take(EFFECT),
            take(FOLLOW),
            take(RUMBLE),
            take(STAR_KO),
            take(SE_SILENCED),
            take(SE_PASSED),
            take(SWITCH_FX),
            take(DECIDE),
            take(COPY_HOOK),
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
    pub const SE_SILENCED: usize = 8;
    pub const SE_PASSED: usize = 9;
    pub const SWITCH_FX: usize = 10;
    pub const DECIDE: usize = 11;
    pub const COPY_HOOK: usize = 12;
}

/// A line for the diag build only: the shipped NRO writes the install line and nothing else.
macro_rules! state_diag {
    ($($arg:tt)*) => {
        #[cfg(feature = "diag")]
        diagnostic(format_args!($($arg)*));
    };
}

/// The last published settings generation the state machine adopted (only read and written under `STATE`).
static ADOPTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Run `f` on the state; a newly published value (`published::Live`) is adopted first (stream/sync: the menu worker publishes
/// through `published`, never through this lock).
fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    let mut guard = STATE.lock();
    let state = guard.as_mut()?;
    if let Some(live) = crate::runtime::published::live_if_newer(&ADOPTED) {
        adopt(state, live.states);
    }
    Some(f(state))
}

/// Create the state at boot (before any hook is installed): the slot table, the settings and the
/// generator seed.
pub fn init(settings: StateSettings, file: SaveFile, seed: u64) {
    // The writer thread keeps its own copy (boot, before any hook): Saves reach it through `hand_to_writer`.
    *WRITER_FILE.lock() = Some(file.clone());
    let saved_miis = setups::saved_identities(&file);
    let file = file.into_game_records();
    *STATE.lock() = Some(State {
        machine: Machine::new(file),
        saved_miis,
        settings,
        rng: Rng::new(seed),
        ids: Table::unresolved(),
        last_kinds: [-1, -1],
        buff: [BuffState::new(); 2],
        pt_wait: [PtWait::new(); 2],
    });
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
        "sys_flying_plate",
        "ptrainer_change_light",
        "se_ptrainer_change_appear",
        "se_ptrainer_ball_open",
        "se_ptrainer_ball_swing",
    ]) {
        slot.store(hash40(name), Ordering::Release);
    }
    SCORE_AURA.store(hash40("sys_score_aura"), Ordering::Release);
}

/// The publishing thread's half of a publish (atomics only): the flags the hooks test before they take `STATE`, and, when the
/// feature is off, the cancel of whatever runs (done by `adopt` at the next hook under `STATE`; the flags clear now).
pub(crate) fn set_flags(settings: &StateSettings) {
    if !settings.enable && ENABLE.load(Ordering::Acquire) {
        CANCEL.store(true, Ordering::Release);
        LOADING.store(false, Ordering::Release);
        KILLING.store(false, Ordering::Release);
    }
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
/// A published value reaches the state machine (at the next hook, under `STATE`): turning the feature off cancels what runs. The
/// flags were set by the publishing thread (`set_flags`).
fn adopt(state: &mut State, settings: StateSettings) {
    if CANCEL.swap(false, Ordering::AcqRel) || (state.settings.enable && !settings.enable) {
        state.machine.cancel_all();
    }
    state.settings = settings;
}

/// A publish turned the feature off: the running save or load is cancelled at the next `with`.
static CANCEL: AtomicBool = AtomicBool::new(false);

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
        state.pt_wait.iter_mut().for_each(PtWait::clear);
    });
    for slot in FIGHTER_ACC.iter() {
        slot.store(0, Ordering::Release);
    }
    LOADING.store(false, Ordering::Release);
    KILLING.store(false, Ordering::Release);
    TEMPORARY.store(false, Ordering::Release);
    MAC_PRESENT.store(false, Ordering::Release);
    for slot in &BUFF_ACC {
        slot.store(0, Ordering::Release);
    }
    // A hat restore the hat engine queued for the load that was cancelled does not apply later (SPEC-state-05 §5.2 step 6).
    if crate::runtime::roster::running() {
        crate::runtime::roster::kirby_hats::cancel_pending();
    }
}

pub(crate) fn clear_session() {
    with(|_| setups::clear_facts());
    cancel_pending();
    quick_reset::clear();
    preview::clear();
    SAVE_REQUEST.store(false, Ordering::Release);
    LOAD_REQUEST.store(false, Ordering::Release);
}

/// The roster's existing waiting-room lifetime decision, rather than a practice
/// active edge, retires Setup drafts. Serialize this epoch with game commits.
pub(crate) fn retire_setup_session() {
    with(|_| setups::retire_session());
}

fn role_of_entry(entry: i32) -> Option<Role> {
    Role::for_scene(scene::current()?, entry)
}

/// A save-state load is in progress: the load machine's own flag, published each fighter frame. The one source
/// of this fact (the CPU runtime's mash and the recorder's chords read it here). The hooks of this module read
/// `LOADING` and `KILLING` directly.
pub fn is_loading() -> bool {
    LOADING.load(Ordering::Acquire)
}

/// What the hooks need of the settings: the three bits, as a `StateSettings`.
fn hook_settings() -> StateSettings {
    StateSettings {
        enable: ENABLE.load(Ordering::Acquire) || TEMPORARY.load(Ordering::Acquire),
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
    for (slot, c) in [
        (K_SQUIRTLE, C::KindPokemonSquirtle),
        (K_IVYSAUR, C::KindPokemonIvysaur),
        (K_CHARIZARD, C::KindPokemonCharizard),
        (K_WEAPON_PTRAINER, C::WeaponPtrainer),
        (K_WEAPON_MBALL, C::WeaponMball),
        (K_TRAIL_N1, C::TrailStatusSpecialN1),
        (K_KIRBY_DRINK, C::KirbyStatusDrink),
        (K_PT_RUN, C::PtStatusRun),
        (K_PT_RESTART, C::PtStatusRestartChange),
        (K_STATUS_WAIT, C::StatusWait),
        (K_LINK_TRAINER, C::LinkNoPtrainer),
        (K_PT_FLAG, C::PtFlagEnableChange),
        (K_PT_MBALL, C::PtArticleMball),
        (K_ALL_TARGET, C::ArticleAllTarget),
        (K_SIT_GROUND, C::SituationGround),
        (K_SIT_AIR, C::SituationAir),
        (K_COPY_ON_START, C::KirbyFlagCopyOnStart),
        (K_COPY_FLAG, C::KirbyFlagCopy),
        (K_COPY_CHARA, C::KirbyIntCopyChara),
        (K_STATUS_NONE, C::StatusNone),
    ] {
        KIDS[slot].store(table.get(c), Ordering::Release);
    }
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
    role: Role,
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
    if accessor == 0 {
        return;
    }
    preview::service_retirement();
    let scene_facts = scene::facts();
    if !crate::model::scene::is_training(scene_facts) {
        // The shared predicate is also false during a hot swap. A suspended
        // callback is not an entry-lifetime boundary: practice retirement owns
        // that boundary, and only actual scene departure clears the cache here.
        if entry == 0 && !scene_facts.native_training && !scene_facts.lobby_active
            && SESSION_ACTIVE.swap(false, Ordering::AcqRel) {
            clear_session();
        }
        return;
    }
    let Some(role) = role_of_entry(entry) else { return };
    SESSION_ACTIVE.store(true, Ordering::Release);
    #[cfg(feature = "diag")]
    if entry == 0 && ((fighter + FIGHTER_KIND_FIELD) as *const i32).read() != NANA_KIND.load(Ordering::Acquire) {
        counts::bump(counts::FRAME);
    }
    let kind = ((fighter + FIGHTER_KIND_FIELD) as *const i32).read();
    let nana = kind == NANA_KIND.load(Ordering::Acquire);
    let raw = Raw {
        entry,
        role,
        kind,
        accessor,
        status: natives::status_kind(accessor),
        prev_status: natives::prev_status_kind(accessor, 0),
        situation: natives::situation_kind(accessor),
        fighter,
    };
    quick_reset::observe(&raw, nana);
    if role == Role::Player && !nana { setups::service(); }
    if !ENABLE.load(Ordering::Acquire) && !TEMPORARY.load(Ordering::Acquire) {
        return;
    }

    #[cfg(feature = "diag")]
    if DUMP.load(Ordering::Acquire) {
        diagnostic(format_args!(
            "[TrainingRooms] state pos entry={} kind={} x={} y={} lr={} dmg={} status={} situation={}",
            raw.entry,
            raw.kind,
            natives::pos_x(accessor),
            natives::pos_y(accessor),
            natives::lr(accessor),
            natives::damage(accessor, 0),
            raw.status,
            raw.situation
        ));
        if let Some(ids) = with(|s| s.ids.clone()) {
            if ids.is(C::KindSteve, raw.kind) {
                if let Some(steve) = state_restore::steve_read(accessor, &ids) {
                    diagnostic(format_args!("[TrainingRooms] state steve entry={} {:?}", raw.entry, steve));
                }
            }
            if ids.is(C::KindOlimar, raw.kind) {
                diagnostic(format_args!("[TrainingRooms] state olimar entry={} line={:?}", raw.entry, state_restore::olimar_line(accessor, &ids)));
            }
            if raw.entry == 0 && !nana {
                diagnostic(format_args!("[TrainingRooms] state item entry=0 {:?}", crate::runtime::features::item_bridge::snapshot(accessor)));
            }
            diagnostic(format_args!(
                "[TrainingRooms] state charge entry={} {:?}",
                raw.entry,
                charge::read(&ids, raw.kind, &Fields(accessor, &ids))
            ));
            if ids.is(C::KindKirby, raw.kind) {
                // The copy flag, the copy-chara int, and the copy module's kind (SPEC-state-05 open question 5).
                let module = n::work_get_int64(accessor, 0x1000_0106) as usize;
                let module_kind = if module != 0 && module % 8 == 0 { ((module + 0x17398) as *const i32).read() } else { -999 };
                diagnostic(format_args!(
                    "[TrainingRooms] state kirby entry={} copy_flag={} on_start={} chara={} module_kind={}",
                    raw.entry,
                    natives::is_flag(accessor, ids.get(C::KirbyFlagCopy)),
                    natives::is_flag(accessor, ids.get(C::KirbyFlagCopyOnStart)),
                    natives::get_int(accessor, ids.get(C::KirbyIntCopyChara)),
                    module_kind
                ));
            }
            if is_pokemon_kind(raw.kind) {
                let trainer = state_restore::trainer_of(accessor, kid(K_LINK_TRAINER));
                diagnostic(format_args!(
                    "[TrainingRooms] state pokemon entry={} kind={} trainer={} trainer_status={} trainer_situation={}",
                    raw.entry,
                    raw.kind,
                    trainer != 0,
                    if trainer != 0 { natives::status_kind(trainer) } else { -999 },
                    if trainer != 0 { natives::situation_kind(trainer) } else { -999 }
                ));
            }
        }
        // Both bodies are dumped, in whichever order the game runs them (a switched Pokemon runs after the CPU).
        if !nana {
            let seen = DUMPED.fetch_or(1 << raw.role.index(), Ordering::AcqRel) | (1 << raw.role.index());
            if seen == 0b11 {
                DUMPED.store(0, Ordering::Release);
                DUMP.store(false, Ordering::Release);
            }
        }
    }
    // A diag copy request: give Kirby the opponent's hat through the game's own routine.
    #[cfg(feature = "diag")]
    if COPY_REQ.load(Ordering::Acquire) && raw.entry == 0 && !nana {
        if let Some(ids) = with(|s| s.ids.clone()) {
            if ids.is(C::KindKirby, raw.kind) {
                COPY_REQ.store(false, Ordering::Release);
                let opponent_kind = with(|s| s.last_kinds[1]).unwrap_or(-1);
                let opponent_entry = FIGHTER_ENTRY[1].load(Ordering::Acquire);
                diagnostic(format_args!("[TrainingRooms] state diag_copy kind={opponent_kind} entry={opponent_entry}"));
                n::kirby_copy_setup(accessor, opponent_entry, opponent_kind);
            }
        }
    }
    if !nana {
        FIGHTER_ACC[role.index()].store(accessor, Ordering::Release);
        FIGHTER_ENTRY[role.index()].store(entry, Ordering::Release);
    }
    // The Pokemon dead test (SPEC-state-01 §3): the one switching frame the plain test misses.
    let switching = is_pokemon_kind(kind)
        && pokemon::switching_frame_is_dead(
            raw.status == kid(K_STATUS_WAIT),
            raw.prev_status == kid(K_STATUS_NONE),
            raw.status == kid(K_STATUS_WAIT) && raw.prev_status == kid(K_STATUS_NONE) && {
                let trainer = state_restore::trainer_of(accessor, kid(K_LINK_TRAINER));
                trainer != 0 && natives::status_kind(trainer) == kid(K_PT_RESTART)
            },
        );
    // The chord engine is pull-based (SPEC-input-04 §2): this per-fighter step consumes the Save and Load chords
    // and latches them for the first eligible fighter (a Load only on a non-CPU-operated, non-Nana body).
    if chords::consume(crate::model::chords::Request::SaveState) {
        request_save();
    }
    if !nana && !reset_hub::cpu_operated(fighter) && chords::consume(crate::model::chords::Request::LoadState) {
        request_load();
    }
    let save_chord = SAVE_REQUEST.swap(false, Ordering::AcqRel);
    // The Load chord is read only on a non-CPU-operated, non-Nana fighter's callback.
    let load_chord = !nana && !reset_hub::cpu_operated(fighter) && LOAD_REQUEST.swap(false, Ordering::AcqRel);
    let stage_id = natives::stage_id();
    let out = with(|state| {
        let ids = &state.ids;
        if !nana {
            state.last_kinds[raw.role.index()] = raw.kind;
        }
        if ids.is(C::KindLittleMac, raw.kind) {
            MAC_PRESENT.store(true, Ordering::Release);
        }
        let dead = ids.is(C::StatusDead, raw.status) || ids.is(C::StatusStandby, raw.status) || switching;
        let input = FrameIn {
            entry: raw.role.index() as i32,
            nana,
            popo: ids.is(C::KindPopo, raw.kind) && !nana_removed(raw.entry),
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
        TEMPORARY.store(state.machine.temporary_active(), Ordering::Release);
        (out, input.situation.classify(raw.situation))
    });
    if let Some((out, _)) = out {
        apply(&raw, nana, out);
    }
    #[cfg(feature = "diag")]
    counts::report_if_due();
}

/// Solo Popo (the roster cycle's feature) has removed Nana from this role: there is no second body to hand the record to, so a load
/// must not wait for her (SPEC-state-01 §5.3 step 9: the fork skips the hand-off then; `character_roster::solo_popo_active` is the
/// fact it published for exactly this). Lock-free reads, safe inside the machine's lock.
fn nana_removed(entry: i32) -> bool {
    crate::runtime::roster::running() && crate::runtime::roster::character_roster::solo_popo_active(entry != 0)
}

/// The per-role "state load started / completed" notification of the Mii HUD portrait (SPEC-state-01 section 9): the roster cycle's
/// presentation check observes the role's HUD card when a load starts and compares it when that role's restore completes (a load must
/// change no card). Only while the cycle runs and its install recorded the main image base.
#[cfg(feature = "mii-hud-portrait-alpha")]
unsafe fn hud_load(role: usize, started: bool) {
    let base = if crate::runtime::roster::running() { crate::runtime::roster::character_roster::main_base() } else { 0 };
    if base == 0 {
        return;
    }
    if started {
        crate::runtime::roster::presentation::state_load_started(base, role);
    } else {
        crate::runtime::roster::presentation::state_load_completed(base, role);
    }
}

#[cfg(not(feature = "mii-hud-portrait-alpha"))]
unsafe fn hud_load(_role: usize, _started: bool) {}

/// Do what the frame asked, with no lock held.
unsafe fn apply(raw: &Raw, nana: bool, out: Out) {
    let accessor = raw.accessor;
    if out.stop_playback {
        // A new load: no buff carries over.
        with(|state| state.buff.iter_mut().for_each(BuffState::clear));
        recorder::on_load_request();
        // The Mii HUD portrait's state-load check (SPEC-state-01 section 9: the per-role "load started" notification): both records load.
        hud_load(0, true);
        hud_load(1, true);
        state_diag!("[TrainingRooms] state load_request entry={}", raw.entry);
    }
    if out.pokemon_predeath {
        state_restore::pt_predeath(accessor, &pt_ids());
    }
    if out.force_dead {
        natives::change_status_force(accessor, with(|s| s.ids.get(C::StatusDead)).unwrap_or(UNRESOLVED), true);
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
        recorder::on_state_restored(ledge);
        // SPEC-core-06: the hitbox visualizer's held spheres go and come back from the restored state (no game call here).
        crate::runtime::features::hitbox::on_state_restored();
        state_diag!("[TrainingRooms] state restored ledge={ledge}");
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
    hud_load(raw.role.index(), false);
    // Diag: which Ice Climbers path this load took (Solo Popo removes Nana: no hand-off).
    if with(|s| s.ids.is(C::KindPopo, raw.kind)).unwrap_or(false) {
        state_diag!("[TrainingRooms] state popo entry={} nana_removed={}", raw.entry, nana_removed(raw.entry));
    }
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
        crate::runtime::features::item_bridge::restore(accessor, item);
    }
    // Pokemon Trainer (SPEC-state-05 §5.1 step 4): the trainer may switch again.
    if done.pokemon_switch {
        state_restore::pt_raise_switch(accessor, &pt_ids());
    }
    // Kirby (§5.2 step 2): the saved hat comes back when the opponent is the fighter it came from.
    if let Some(saved) = &done.charge {
        if let (Some(copied), Some(true)) = (saved.int_z, saved.has_charge) {
            if with(|s| s.ids.is(C::KindKirby, raw.kind)).unwrap_or(false) {
                kirby_copy(accessor, raw.role, copied);
            }
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
        unsafe { natives::get_int(self.0, id) }
    }
    fn get_float(&self, id: i32) -> f32 {
        unsafe { natives::get_float(self.0, id) }
    }
    fn get_flag(&self, id: i32) -> bool {
        unsafe { natives::is_flag(self.0, id) }
    }
}

/// The game constants of the Pokemon Trainer rules, as `state_restore` takes them.
fn pt_ids() -> state_restore::PtIds {
    state_restore::PtIds {
        link: kid(K_LINK_TRAINER),
        enable_flag: kid(K_PT_FLAG),
        mball: kid(K_PT_MBALL),
        all_target: kid(K_ALL_TARGET),
        run: kid(K_PT_RUN),
        ground: kid(K_SIT_GROUND),
        air: kid(K_SIT_AIR),
    }
}

/// A feature's wait (SPEC-state-01 §5.4): `ApplyBuff` runs the buff step, `WaitForPokemonSwitch` the
/// trainer's five frames (SPEC-state-05 §5.1 step 5), `WaitForCopyAbility` ends whenever Kirby's copy-start
/// flag is clear (the copy hook consumes it and writes the charge, §5.2 steps 4 and 5).
unsafe fn run_wait(raw: &Raw, marker: Marker) {
    let role = raw.role;
    match marker {
        Marker::ApplyBuff => apply_buff(raw, role),
        Marker::WaitForPokemonSwitch => pt_wait(raw, role),
        Marker::WaitForCopyAbility => {
            let flag = kid(K_COPY_ON_START);
            if flag == UNRESOLVED || !natives::is_flag(raw.accessor, flag) {
                with(|s| s.machine.finish_wait(role));
            }
        }
        _ => {
            with(|s| s.machine.finish_wait(role));
        }
    }
}

/// One `WaitForPokemonSwitch` frame for the Pokemon of `raw`.
unsafe fn pt_wait(raw: &Raw, role: Role) {
    let ids = pt_ids();
    let trainer = state_restore::trainer_of(raw.accessor, ids.link);
    if trainer == 0 {
        // No trainer to wait for (the trainer left, or the link did not resolve): the switch is over.
        with(|s| {
            s.pt_wait[role.index()].clear();
            s.machine.finish_wait(role);
        });
        return;
    }
    let in_run = natives::status_kind(trainer) == ids.run;
    let airborne = natives::situation_kind(trainer) == ids.air;
    let Some(out) = with(|s| s.pt_wait[role.index()].step(in_run, airborne)) else { return };
    state_restore::pt_apply(trainer, &ids, &out);
    if out.done {
        with(|s| s.machine.finish_wait(role));
        state_diag!("[TrainingRooms] state pt_switch_done entry={} grounded={}", raw.entry, out.ground_and_run);
    }
}

/// Kirby's hat (SPEC-state-05 §5.2 step 2): give the copy routine the opponent's entry id and the saved kind
/// when the opponent is the fighter the hat came from, or in its family. While the roster cycle runs, the hat engine takes the
/// restore instead (step 6: it services the hat asynchronously from its own sources, does not need the opponent in the match and
/// waits for this load to finish, `kirby_hats::on_frame`).
unsafe fn kirby_copy(accessor: usize, role: Role, copied: i32) {
    if let Some(kind) = u32::try_from(copied).ok().filter(|_| crate::runtime::roster::running()) {
        crate::runtime::roster::kirby_hats::restore(accessor as *mut smash::app::BattleObjectModuleAccessor, kind);
        return;
    }
    let opponent = 1 - role.index();
    let opponent_accessor = FIGHTER_ACC[opponent].load(Ordering::Acquire);
    if opponent_accessor == 0 {
        return;
    }
    let opponent_entry = FIGHTER_ENTRY[opponent].load(Ordering::Acquire);
    let Some((opponent_kind, kinds)) = with(|s| (s.last_kinds[opponent], kirby_kinds(&s.ids))) else { return };
    let trainer_valid = opponent_kind == kinds.none && state_restore::trainer_of(opponent_accessor, kid(K_LINK_TRAINER)) != 0;
    let accepted = pokemon::kirby_accepts(copied, opponent_kind, trainer_valid, &kinds);
    state_diag!("[TrainingRooms] state kirby_copy copied={} opponent_kind={} entry={} accepted={}", copied, opponent_kind, opponent_entry, accepted);
    if accepted {
        n::kirby_copy_setup(accessor, opponent_entry, copied);
    }
}

fn kirby_kinds(ids: &Table) -> KirbyKinds {
    KirbyKinds {
        pokemon: pokemon_kinds(ids),
        pyra: ids.get(C::KindPyra),
        mythra: ids.get(C::KindMythra),
        none: -1,
    }
}

fn pokemon_kinds(ids: &Table) -> Kinds {
    Kinds { squirtle: ids.get(C::KindPokemonSquirtle), ivysaur: ids.get(C::KindPokemonIvysaur), charizard: ids.get(C::KindPokemonCharizard) }
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
    state_effects::run_buff(accessor, raw.entry, &ids, &out);
    if out.done {
        with(|s| s.machine.finish_wait(role));
        state_diag!("[TrainingRooms] state buff_done entry={} kind={:?}", raw.entry, kind);
    }
}

/// Read value-only fighter state without changing a numbered slot, requesting
/// persistence or emitting the Save effect. All native calls stay outside locks.
unsafe fn read_record(raw: &Raw) -> SavedState {
    let role = raw.role;
    let accessor = raw.accessor;
    let ids = with(|s| s.ids.clone());
    let (charge, copy_charge) = ids
        .as_ref()
        .map(|ids| {
            let fields = Fields(accessor, ids);
            if ids.is(C::KindKirby, raw.kind) {
                // §5.2 step 1: the copy flag and the copied fighter's kind, then its charge values; the
                // copied fighter's own flag goes to `copy_charge`.
                let has_copy = natives::is_flag(accessor, ids.get(C::KirbyFlagCopy));
                charge::read_kirby(ids, has_copy, natives::get_int(accessor, ids.get(C::KirbyIntCopyChara)), &fields)
            } else {
                (charge::read(ids, raw.kind, &fields), None)
            }
        })
        .unwrap_or_default();
    SavedState {
        x: natives::pos_x(accessor),
        y: natives::pos_y(accessor),
        percent: natives::damage(accessor, 0),
        lr: natives::lr(accessor),
        situation_kind: raw.situation,
        marker: Marker::NoAction,
        fighter_kind: raw.kind,
        charge,
        copy_charge,
        steve: ids.as_ref().filter(|ids| ids.is(C::KindSteve, raw.kind)).and_then(|ids| state_restore::steve_read(accessor, ids)),
        // The player's held item (the CPU stores none).
        held_item: (role == Role::Player).then(|| crate::runtime::features::item_bridge::snapshot(accessor)),
        saved_stage: Some(natives::stage_id()),
        mii: None,
    }
}

/// The Save step (§5.5 step 2): read the values, store them, write the file when both are done.
unsafe fn capture(raw: &Raw, nana: bool, slot: usize) {
    if nana { return; }
    let role = raw.role;
    let accessor = raw.accessor;
    let state = read_record(raw);
    // Capture only the settled, exact value belonging to this loaded role.
    // JSON conversion belongs to the writer, never to this hook. A not-yet-ready
    // Mii remains explicitly absent (Setup Save refuses it until saved again).
    let mii = (crate::runtime::roster::running() && crate::model::state::setup::MII_KINDS.contains(&raw.kind))
        .then(|| crate::runtime::roster::character_roster::current_mii_configuration(role.index()))
        .flatten()
        .filter(|value| value.canonical().class as i32 == raw.kind);
    hand_to_writer(role, slot, state.clone(), mii);
    let write = with(|s| {
        s.saved_miis[role.index()][slot.min(4)] = mii;
        s.machine.capture(role, slot, state)
    }).unwrap_or(false);
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
        if let Some(value) = hooks::param_override(param, &hook_settings(), LOADING.load(Ordering::Acquire), scene::is_training()) {
            if param == DeathParam::DeadRebirthWaitFrame {
                // The jostle and Shulk cleanups that ride on this override (SPEC-state-01 §4).
                let jostle = JOSTLE_ID.load(Ordering::Acquire);
                if jostle != UNRESOLVED && natives::get_int(accessor, jostle) > 1 {
                    natives::set_int(accessor, 1, jostle);
                }
                let flash = SHULK_FLASH_ID.load(Ordering::Acquire);
                if flash != UNRESOLVED {
                    natives::set_int(accessor, 0, flash);
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
    if (ENABLE.load(Ordering::Acquire) || TEMPORARY.load(Ordering::Acquire)) && scene::is_training() {
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
    if none != UNRESOLVED && hooks::quake_suppressed(KILLING.load(Ordering::Acquire)) && scene::is_training() {
        return call_original!(accessor, none);
    }
    call_original!(accessor, kind)
}

/// `SoundModule::play_se`: `se_silent` while killing.
#[skyline::hook(offset = s::STATE_PLAY_SE_OFFSET)]
pub unsafe fn state_play_se_hook(accessor: usize, hash: u64, a: bool, b: bool, c: bool, d: bool, kind: u32) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::SOUND);
    if hooks::sound_silenced(KILLING.load(Ordering::Acquire)) && scene::is_training() {
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
    // The Pokemon Trainer switch effects first (SPEC-state-05 §4.1), then the load rule.
    if switch_effect_hidden(accessor, effect) {
        return call_original!(accessor, effect, pos, rot, 0.0, a, b, c, d);
    }
    // A buffing fighter keeps its visuals (SPEC-state-03 §5.2).
    if hooks::effect_hidden(LOADING.load(Ordering::Acquire), fighter_buffing(accessor)) && scene::is_training() {
        return call_original!(accessor, effect, pos, rot, 0.0, a, b, c, d);
    }
    call_original!(accessor, effect, pos, rot, size, a, b, c, d)
}

/// Whether a request is one of the trainer's switch effects on a trainer-family object, to be made size 0.
/// The effect hash is compared first (two compares), the object's kind only for those effects.
unsafe fn switch_effect_hidden(accessor: usize, effect: u64) -> bool {
    let class = if effect == HASHES[H_FLYING_PLATE].load(Ordering::Acquire) {
        SwitchEffect::FlyingPlate
    } else if effect == HASHES[H_CHANGE_LIGHT].load(Ordering::Acquire) || effect == pokemon::CHANGE_LIGHT_RAW {
        SwitchEffect::ChangeLight
    } else {
        return false;
    };
    if !pokemon::switch_effect_hidden(class, LOADING.load(Ordering::Acquire)) || !scene::is_training() {
        return false;
    }
    let kind = n::object_kind(accessor);
    let hidden = kind == -1 || [K_SQUIRTLE, K_IVYSAUR, K_CHARIZARD, K_WEAPON_PTRAINER, K_WEAPON_MBALL].iter().any(|k| kid(*k) == kind);
    if hidden {
        bump(counts_index::SWITCH_FX);
    }
    state_diag!("[TrainingRooms] state switch_fx effect={:#x} class={:?} object_kind={} hidden={}", effect, class, kind, hidden);
    hidden
}

/// `EffectModule::req_on_joint`: the trainer-switch size rule only (SPEC-state-05 §4.1).
#[skyline::hook(offset = s::STATE_EFFECT_REQ_ON_JOINT_HOOK_OFFSET)]
pub unsafe fn state_effect_req_on_joint_hook(
    accessor: usize,
    effect: u64,
    joint: u64,
    pos: *const [f32; 3],
    rot: *const [f32; 3],
    size: f32,
    pos_rand: *const [f32; 3],
    rot_rand: *const [f32; 3],
    a: bool,
    b: u32,
    c: i32,
    d: i32,
) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::EFFECT);
    if switch_effect_hidden(accessor, effect) {
        return call_original!(accessor, effect, joint, pos, rot, 0.0, pos_rand, rot_rand, a, b, c, d);
    }
    call_original!(accessor, effect, joint, pos, rot, size, pos_rand, rot_rand, a, b, c, d)
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
    if effect == SCORE_AURA.load(Ordering::Acquire) && hooks::score_aura_hidden(LOADING.load(Ordering::Acquire)) && scene::is_training() {
        return call_original!(accessor, effect, joint, pos, rot, 0.0, a, b, c, d, e, f, g, h);
    }
    call_original!(accessor, effect, joint, pos, rot, size, a, b, c, d, e, f, g, h)
}

/// The stage's "can star KO" query: false in Training, so a death is followed by the respawn at once.
#[skyline::hook(offset = s::STATE_NO_STAR_KO_OFFSET)]
pub unsafe fn state_no_star_ko_hook(a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> u64 {
    let _scope = HookScope::enter();
    bump(counts_index::STAR_KO);
    if (ENABLE.load(Ordering::Acquire) || TEMPORARY.load(Ordering::Acquire)) && scene::is_training() {
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
    if summon != UNRESOLVED && result == summon as u32 && fighter_buffing(accessor) && scene::is_training() {
        return 0xFFFF_FFFF;
    }
    result
}

/// The shared UI-update routine (entry data, value): Little Mac's KO meter reads 100 while a load with
/// KO Punch runs (whichever Little Mac is in the match).
#[skyline::hook(offset = s::STATE_MAC_METER_OFFSET)]
pub unsafe fn state_mac_meter_hook(entry: usize, value: i32) -> u64 {
    let _scope = HookScope::enter();
    if LOADING.load(Ordering::Acquire) && KO_SELECTED.load(Ordering::Acquire) && MAC_PRESENT.load(Ordering::Acquire) && scene::is_training() {
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
        if wii_buffing && scene::is_training() {
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
    if hash == WING_BACKGROUND && fighter_buffing(accessor) && scene::is_training() {
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
    if (any_buffing() || fighter_sound_silenced(a0, a1)) && scene::is_training() {
        return call_original!(a0, HASHES[H_SE_SILENT].load(Ordering::Acquire), a2, a3, a4, a5, a6, a7);
    }
    call_original!(a0, a1, a2, a3, a4, a5, a6, a7)
}

/// The Pokemon Trainer, Kirby and Sora sound silences (SPEC-state-05 §4.1). The hash is classified first
/// (a few compares); the object's status is read only for the two status-dependent sounds while loading.
unsafe fn fighter_sound_silenced(module: usize, hash: u64) -> bool {
    let sound = if hash == HASHES[H_SE_CHANGE_APPEAR].load(Ordering::Acquire)
        || hash == HASHES[H_SE_BALL_OPEN].load(Ordering::Acquire)
        || hash == HASHES[H_SE_BALL_SWING].load(Ordering::Acquire)
    {
        Sound::TrainerSwitch
    } else if pokemon::KIRBY_COPY_SE.contains(&hash) {
        Sound::KirbyCopy
    } else if hash == pokemon::SORA_SPELL_SE {
        Sound::SoraSpell
    } else {
        return false;
    };
    let loading = LOADING.load(Ordering::Acquire);
    if !loading {
        return false;
    }
    // The sound module's owner accessor is the pointer at `+8`, as for the damage module.
    let owner = if module == 0 { 0 } else { ((module + 8) as *const usize).read() };
    let status = if owner == 0 { UNRESOLVED } else { natives::status_kind(owner) };
    let (drinking, above) = match sound {
        Sound::KirbyCopy => (status == kid(K_KIRBY_DRINK), false),
        Sound::SoraSpell => {
            let n1 = kid(K_TRAIL_N1);
            (false, n1 != UNRESOLVED && status != UNRESOLVED && status >= n1)
        }
        _ => (false, false),
    };
    let silenced = pokemon::sound_silenced(sound, loading, drinking, above);
    bump(if silenced { counts_index::SE_SILENCED } else { counts_index::SE_PASSED });
    state_diag!("[TrainingRooms] state se hash={:#x} sound={:?} status={} silenced={}", hash, sound, status, silenced);
    silenced
}

// ---- Pokemon Trainer decide and Kirby copy (SPEC-state-05 §5.1 step 3, §5.2 step 4) ----------------------

/// The inline hook at the store of the game's chosen Pokemon index: while a load runs, `w8` becomes the
/// saved Pokemon's index (`x20 + 0x10` is the entry id; `x9` is the descriptor: count at `+0x14`, three
/// kinds at `+0x18`, `+0x1c`, `+0x20`). Anything that does not check out leaves the register alone.
#[skyline::hook(offset = s::STATE_POKEMON_DECIDE_OFFSET, inline)]
pub unsafe fn state_pokemon_decide_hook(ctx: &mut InlineCtx) {
    let _scope = HookScope::enter();
    if !LOADING.load(Ordering::Acquire) || !scene::is_training() {
        return;
    }
    let (entry_object, descriptor) = (ctx.registers[20].x() as usize, ctx.registers[9].x() as usize);
    if entry_object == 0 || descriptor == 0 || descriptor % 8 != 0 {
        return;
    }
    let entry = ((entry_object + 0x10) as *const i32).read();
    let Some(role) = role_of_entry(entry) else { return };
    let Some((saved, kinds)) = with(|s| {
        (s.machine.restoring_record(role, &s.settings).fighter_kind, pokemon_kinds(&s.ids))
    }) else {
        return;
    };
    let read = |offset: usize| ((descriptor + offset) as *const u32).read();
    let found = Descriptor { count: read(0x14), kinds: [read(0x18), read(0x1c), read(0x20)] };
    state_diag!("[TrainingRooms] state pt_decide entry={} saved={} descriptor={:?}", entry, saved, found);
    if let Some(index) = pokemon::decide_index(saved, Some(found), kinds) {
        bump(counts_index::DECIDE);
        ctx.registers[8].set_x(index as u64);
    } else {
        state_diag!("[TrainingRooms] state pt_decide_rejected entry={} saved={}", entry, saved);
    }
}

/// Kirby's per-frame copy routine `(param, kirby fighter)`: while a load runs, whether the copy start flag
/// was set is remembered before the original (which clears it); if it was, the copied charge is written
/// from the record and the wait ends.
#[skyline::hook(offset = s::STATE_KIRBY_OPFF_OFFSET)]
pub unsafe fn state_kirby_opff_hook(param: usize, fighter: usize) -> u64 {
    let _scope = HookScope::enter();
    let started = fighter != 0 && LOADING.load(Ordering::Acquire) && scene::is_training() && {
        let flag = kid(K_COPY_ON_START);
        let accessor = ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read();
        flag != UNRESOLVED && accessor != 0 && natives::is_flag(accessor, flag)
    };
    let result = call_original!(param, fighter);
    if started {
        kirby_copy_started(fighter);
    }
    result
}

unsafe fn kirby_copy_started(fighter: usize) {
    let entry = ((fighter + FIGHTER_ENTRY_FIELD) as *const i32).read();
    let accessor = ((fighter + FIGHTER_ACCESSOR_FIELD) as *const usize).read();
    let Some(role) = role_of_entry(entry) else { return };
    let Some((ids, plan)) = with(|s| {
        let record = s.machine.restoring_record(role, &s.settings);
        (s.ids.clone(), charge::plan_kirby(&s.ids, &record.charge, record.copy_charge.as_ref()))
    }) else {
        return;
    };
    state_effects::run_charge(accessor, fighter, &ids, &plan);
    bump(counts_index::COPY_HOOK);
    with(|s| s.machine.finish_wait(role));
    state_diag!("[TrainingRooms] state kirby_copy_started entry={} steps={}", entry, plan.iter().count());
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

/// The writer thread's own copy of the save file (stream/sync, lead decision (a)): the file boot loaded, plus every record a Save
/// captured, applied in capture order. Taken by boot and the writer thread only; the game's hooks never wait on it.
static WRITER_FILE: Mutex<Option<SaveFile>> = Mutex::new(None);

const HANDOFF_SLOTS: usize = 8;
const SLOT_EMPTY: u8 = 0;
const SLOT_WRITING: u8 = 1;
const SLOT_FULL: u8 = 2;
const SLOT_READING: u8 = 3;
struct CapturedRecord {
    sequence: u64,
    role: Role,
    slot: usize,
    state: SavedState,
    mii: Option<crate::engine::mii::lifetime::Configuration>,
}
type Captured = Option<CapturedRecord>;
struct HandoffSlot(std::cell::UnsafeCell<Captured>);
// SAFETY: a slot's cell is touched only by the one thread that moved its state to SLOT_WRITING or SLOT_READING.
unsafe impl Sync for HandoffSlot {}
/// Captured records on their way to the writer: fixed slots claimed by compare-exchange (any game thread may capture), drained by
/// the writer; a sequence number keeps the capture order. A full mailbox drops the record and counts it.
static HANDOFF: [HandoffSlot; HANDOFF_SLOTS] = [const { HandoffSlot(std::cell::UnsafeCell::new(None)) }; HANDOFF_SLOTS];
static HANDOFF_STATE: [std::sync::atomic::AtomicU8; HANDOFF_SLOTS] = [const { std::sync::atomic::AtomicU8::new(SLOT_EMPTY) }; HANDOFF_SLOTS];
static HANDOFF_SEQ: AtomicU64 = AtomicU64::new(0);
static HANDOFF_DROPPED: AtomicU32 = AtomicU32::new(0);

/// A Save captured `state` for `role` in `slot` (game thread): the writer's copy follows. Never waits, no heap.
fn hand_to_writer(role: Role, slot: usize, state: SavedState, mii: Option<crate::engine::mii::lifetime::Configuration>) {
    let seq = HANDOFF_SEQ.fetch_add(1, Ordering::AcqRel);
    for (i, cell) in HANDOFF_STATE.iter().enumerate() {
        if cell.compare_exchange(SLOT_EMPTY, SLOT_WRITING, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            // SAFETY: the slot is ours between SLOT_WRITING and SLOT_FULL.
            unsafe { *HANDOFF[i].0.get() = Some(CapturedRecord { sequence: seq, role, slot, state, mii }) };
            cell.store(SLOT_FULL, Ordering::Release);
            return;
        }
    }
    HANDOFF_DROPPED.fetch_add(1, Ordering::Relaxed);
}

/// The writer applies the captured records to its copy, oldest first (as `Machine::capture` stores them).
fn take_captured(file: &mut SaveFile) {
    let mut binding = setups::take_writer_bind();
    let mut taken: [Captured; HANDOFF_SLOTS] = [const { None }; HANDOFF_SLOTS];
    for (i, cell) in HANDOFF_STATE.iter().enumerate() {
        if cell.compare_exchange(SLOT_FULL, SLOT_READING, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            // SAFETY: the slot is ours between SLOT_READING and SLOT_EMPTY.
            taken[i] = unsafe { (*HANDOFF[i].0.get()).take() };
            cell.store(SLOT_EMPTY, Ordering::Release);
        }
    }
    taken.sort_by_key(|record| record.as_ref().map_or(u64::MAX, |record| record.sequence));
    for CapturedRecord { sequence, role, slot, mut state, mii } in taken.into_iter().flatten() {
        if binding.as_ref().is_some_and(|job| job.sequence() < sequence) {
            setups::apply_writer_bind(file, binding.take().unwrap());
        }
        state.marker = Marker::NoAction;
        // Configuration serializes only validated integer/byte values. Its
        // allocation-bearing JSON representation stays on this writer thread.
        state.attach_mii(mii).expect("validated Mii configuration serialization");
        let records = match role {
            Role::Player => &mut file.player,
            Role::Cpu => &mut file.cpu,
        };
        records[slot.min(records.len() - 1)] = state;
    }
    if let Some(binding) = binding { setups::apply_writer_bind(file, binding); }
    let dropped = HANDOFF_DROPPED.swap(0, Ordering::Relaxed);
    if dropped != 0 {
        diagnostic(format_args!("[TrainingRooms] save-state records dropped before the writer took them: {dropped}"));
    }
}
/// The writer thread: whenever a Save marked the file dirty, write the whole table through the atomic
/// replace. A failed write keeps the old file and is retried (the file stays dirty).
pub fn start_writer() {
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::StateWriter, || loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if !DIRTY.swap(false, Ordering::AcqRel) {
            continue;
        }
        let Some(path) = save_path() else { continue };
        let mut mirror = WRITER_FILE.lock();
        let Some(file) = mirror.as_mut() else { continue };
        take_captured(file);
        if let Err(error) = persist::save(Path::new(path.as_str()), file) {
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

/// Diag build: which of the two bodies the current dump has printed (bit 0 player, bit 1 CPU).
#[cfg(feature = "diag")]
static DUMPED: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

#[cfg(feature = "diag")]
fn request_dump() {
    DUMPED.store(0, Ordering::Release);
    DUMP.store(true, Ordering::Release);
}

/// Diag build: Kirby copies the opponent through the game's own routine at his next frame.
#[cfg(feature = "diag")]
static COPY_REQ: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "diag")]
fn request_copy() {
    COPY_REQ.store(true, Ordering::Release);
}

/// Diag build: the Eden tests drive the Save and Load chords by creating request files in the profile's
/// `TrainingRooms` folder (`diag-save.req`, `diag-load.req`), which this thread consumes. Not in the
/// shipped NRO.
#[cfg(feature = "diag")]
pub fn start_diag_triggers() {
    let _ = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::StateDiag, || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // A settings file dropped by the test script replaces the live settings (the menu is not built yet).
        if let Some(path) = FixedPath::<128>::in_root("diag-settings.json") {
            if let Ok(bytes) = std::fs::read(path.as_str()) {
                let _ = std::fs::remove_file(path.as_str());
                let loaded = crate::model::settings::parse_bytes(&bytes);
                crate::runtime::published::publish(&loaded.settings.menu, 0);
                crate::runtime::compat::menu::apply_kept(&loaded.settings);
                diagnostic(format_args!("[TrainingRooms] state diag_settings applied issues={}", loaded.issues.len()));
            }
        }
        for (name, action) in [
            ("diag-save.req", request_save as fn()),
            ("diag-load.req", request_load as fn()),
            ("diag-pos.req", request_dump as fn()),
            ("diag-copy.req", request_copy as fn()),
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
