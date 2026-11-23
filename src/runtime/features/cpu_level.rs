//! The mod's CPU level (rc.6), the Switch half. The rules are `model::cpu::level`; every address used here has a seam
//! row (`native_seams::rows_cpu_level`, group `cpu_level`), compared once at boot.
//!
//! No hook of its own:
//! - The Training session clock runs on the waiting-room clock (`lobby::update`, every controller poll and every fighter
//!   frame, in every scene). A session lasts while native Training has Player 1's entry; the character select has none.
//! - The step runs once per game frame on Player 1's body (`lobby::frame`). In native Training it judges the session's
//!   notice once, and in Training and the waiting room's practice it keeps the targeted CPUs at the setting's rank the
//!   way every native writer does: `main+0x6b8400(controller, rank)` and the entry's `FE+0x37`. Anywhere else (every
//!   match) it reads and writes nothing.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::engine::seams::native_fields as field;
use crate::model::cpu::level::{self as rules, CpuLevel, EntryRank, NoticeGate, SessionClock, Where};
use crate::model::edge_juggle::{BehaviorLease, BehaviorMenu, BehaviorPause, CpuBehavior, PartnerLease};
use crate::model::input_delay::Owners;
use crate::model::hook_stats::{HookId, HookScope};
use crate::runtime::game;

/// Every row the step relies on: the setter it calls, the rank table it reads, and the native readers and writers it follows.
const ROWS: &[&str] = &[
    "cpu_level.ai_set_rank",
    "cpu_level.rank_table",
    "cpu_level.training_rank_choice",
    "cpu_level.training_rank_write",
    "cpu_level.training_level_init",
    "cpu_level.training_level_label",
    "cpu_level.lobby_partner_spawn",
    "cpu_level.lobby_partner_start",
    "cpu_level.lobby_rank_up",
    "cpu_level.lobby_rank_event",
    "cpu_level.control_select",
];

/// `cpu_level.ai_set_rank`.
const AI_SET_RANK: usize = 0x6b8400;
/// `edge_juggle.ai_set_behavior`.
const AI_SET_BEHAVIOR: usize = 0x6b82b0;
/// `cpu_level.rank_table`: the ranks of Lv 2..=9.
const RANK_TABLE: usize = 0x4470dc0;
/// The Training state root and the pause menu's chain (`cpu_level.training_rank_choice`, as `roster_cpu_quantity_root`).
const TRAINING_ROOT: usize = 0x52c3420;
const PANEL_CHAIN: [usize; 7] = [8, 0, 0xb38, 0, 0x210, 8, 0x80];
/// HUD, immediately before the panel's final two links.
const HUD_DEPTH: usize = 5;
const HUD_PAUSE_STATE: usize = 0x200;
/// The pause menu's CPU Behavior and Training's own level index (`cpu_level.training_level_label`).
const PANEL_BEHAVIOR: usize = 0xb4c;
const PANEL_LEVEL: usize = 0xb50;
/// FighterEntry: the controller and the rank byte the native writers store (`cpu_level.training_rank_write`).
const ENTRY_CONTROLLER: usize = 0x4178;
const ENTRY_RANK: usize = 0x37;
/// The controller's AI and the AI's rank (`cpu_level.ai_set_rank`'s own loads).
const CONTROLLER_AI: usize = 0x58;
const AI_RANK: usize = 0xc0c;
const ENTRY_BEHAVIOR: usize = 0x35;
const AI_BEHAVIOR: usize = 0xc08;
const DRILL_BEHAVIOR_ROWS: &[&str] = &[
    "edge_juggle.ai_set_behavior",
    "edge_juggle.training_behavior_choice",
    "edge_juggle.training_behavior_write",
    "edge_juggle.training_operation_apply",
    "edge_juggle.training_operation_mirrors",
    "edge_juggle.live_behavior_choice",
    "edge_juggle.live_behavior_write",
    "edge_juggle.live_operation_apply",
    "edge_juggle.live_operation_mirrors",
    "edge_juggle.pause_hud_state",
    "edge_juggle.pause_hud_open",
];
/// About four seconds on the plate.
const NOTICE_FRAMES: u16 = 240;

/// `main`'s base once every row passed the authoritative gate at boot, else 0: the only base this module forms an
/// address with (the seam-gated natives' rule, as `cpu_pilot` and the Mii bridge keep theirs).
static BASE: AtomicUsize = AtomicUsize::new(0);
static DRILL_BEHAVIOR: AtomicBool = AtomicBool::new(false);
static DRILL_APPLIED: AtomicBool = AtomicBool::new(false);
static DRILL_PARTNER_HELD: AtomicBool = AtomicBool::new(false);

pub(super) fn install_drill_behavior() -> bool {
    use crate::runtime::seams::seam_install::gate;
    let ready = gate("cpu_level", ROWS, false) && gate("edge_juggle", DRILL_BEHAVIOR_ROWS, false);
    DRILL_BEHAVIOR.store(ready, Ordering::Release);
    ready
}

/// The base, or `None` before (or without) the gate.
fn base() -> Option<usize> {
    Some(BASE.load(Ordering::Acquire)).filter(|base| *base != 0)
}

struct State {
    clock: SessionClock,
    session: Option<u64>,
    gate: NoticeGate,
    entries: [EntryRank; 8],
    drill_behavior: BehaviorLease,
    drill_partner: PartnerLease,
    drill_pause: BehaviorPause,
    drill_panel: usize,
    /// The controller each entry's override belongs to: a new fighter starts with nothing to give back.
    owners: [usize; 8],
    #[cfg(feature = "diag")]
    seen: [i32; 8],
    #[cfg(feature = "diag")]
    judged: Option<u64>,
    #[cfg(feature = "diag")]
    menu_seen: Option<(u32, u32)>,
    #[cfg(feature = "diag")]
    place_seen: &'static str,
}

impl State {
    const fn new() -> Self {
        Self {
            clock: SessionClock::new(),
            session: None,
            gate: NoticeGate::new(),
            entries: [EntryRank::new(); 8],
            drill_behavior: BehaviorLease::new(),
            drill_partner: PartnerLease::new(),
            drill_pause: BehaviorPause::Idle,
            drill_panel: 0,
            owners: [0; 8],
            #[cfg(feature = "diag")]
            seen: [-1; 8],
            #[cfg(feature = "diag")]
            judged: None,
            #[cfg(feature = "diag")]
            menu_seen: None,
            #[cfg(feature = "diag")]
            place_seen: "elsewhere",
        }
    }

    fn forget_entries(&mut self) {
        self.entries = [EntryRank::new(); 8];
        self.owners = [0; 8];
        #[cfg(feature = "diag")]
        {
            self.seen = [-1; 8];
        }
    }
}

/// Game threads only (the poll and Player 1's frame); `try_lock`, so a hook never waits.
static STATE: hos_lock::Mutex<State> = hos_lock::Mutex::new(State::new());

/// Once from `lobby::init`: compare every row; nothing runs unless all passed.
pub(crate) fn init() {
    // `cpu_level` is an authoritative group (`seam_install::AUTHORITATIVE`): the rows decide, not `legacy`.
    let rows = crate::runtime::seams::seam_install::gate("cpu_level", ROWS, false);
    if rows {
        BASE.store(game::text_base(), Ordering::Release);
    }
    crate::engine::compat::logging::diagnostic(format_args!("[TrainingRooms] cpu_level installed={rows}"));
}

/// The Training session clock (`lobby::update`): a session ends when native Training has no Player 1 entry.
pub(crate) fn poll() {
    if base().is_none() {
        return;
    }
    let _scope = HookScope::enter_hook(HookId::CpuLevelClock);
    let training = crate::runtime::scene::is_native_training();
    // SAFETY: the guarded `FighterManager::get_fighter_entry` (0 when absent); nothing is dereferenced here.
    let player = training && unsafe { game::natives::fighter_entry(0) } != 0;
    let place = if training { Where::Training { behavior: 0 } } else { Where::Elsewhere };
    let Some(mut state) = STATE.try_lock() else { return };
    state.session = state.clock.poll(place, player);
    // Diag build: Training's menu values on each change, also while paused (no fighter frame runs then), and only while
    // Player 1's entry exists (the chain is the live session's).
    #[cfg(feature = "diag")]
    {
        // SAFETY: native Training with Player 1's entry present, on the game thread.
        let menu = match (player, base()) {
            (true, Some(base)) => unsafe { training_menu(base) },
            _ => None,
        };
        if menu != state.menu_seen {
            state.menu_seen = menu;
            if let Some((behavior, index)) = menu {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[CpuLevel] menu behavior={behavior} level={}", index.saturating_add(1)
                ));
            }
        }
    }
}

/// Diag build: a parse-friendly name of the place and its CPU Behavior.
#[cfg(feature = "diag")]
fn place_tag(place: Where) -> (&'static str, u32) {
    match place {
        Where::Training { behavior } => ("training", behavior),
        Where::Lobby => ("lobby", 0),
        Where::Elsewhere => ("elsewhere", 0),
    }
}

/// Diag build: the setting as a number (0 = Off).
#[cfg(feature = "diag")]
fn level_number(level: CpuLevel) -> u8 {
    match level {
        CpuLevel::Lv(level) => level,
        CpuLevel::Off => 0,
    }
}

/// Training's pause menu (behavior, level index), or `None` while its chain is not readable.
///
/// # Safety
/// Native Training on the game thread: the chain is the one the game itself reads every frame there.
unsafe fn training_panel(base: usize) -> Option<usize> {
    let root = field::read::<usize>(base, TRAINING_ROOT)?;
    field::chain(root, &PANEL_CHAIN)
}

unsafe fn training_menu(base: usize) -> Option<(u32, u32)> {
    let panel = training_panel(base)?;
    Some((field::read_unchecked::<u32>(panel, PANEL_BEHAVIOR), field::read_unchecked::<u32>(panel, PANEL_LEVEL)))
}

/// Where the game is, and Training's own level when it is Training.
///
/// # Safety
/// The game thread.
unsafe fn place(base: usize) -> (Where, Option<u8>) {
    if crate::runtime::scene::is_native_training() {
        let menu = training_menu(base);
        let behavior = menu.map_or(0, |(behavior, _)| behavior);
        (Where::Training { behavior }, menu.and_then(|(_, index)| rules::level_of_menu_index(index)))
    } else if crate::runtime::lobby::training::lobby::is_waiting_scene() {
        (Where::Lobby, None)
    } else {
        (Where::Elsewhere, None)
    }
}

/// The plate: the body is formatted on the stack and copied into the notice queue's fixed buffer (no heap).
fn post(notice: rules::Notice) {
    let mut text = crate::model::storage::FixedPath::<96>::new();
    let _ = rules::notice_body(notice, &mut text);
    crate::runtime::features::notices::post_body(
        rules::NOTICE_HEADER,
        crate::model::notify::Body::copied(text.as_str()),
        NOTICE_FRAMES,
        crate::model::notify::DEFAULT_COLOR,
    );
}

/// Player 1's frame (`lobby::frame`, once per game frame).
///
/// # Safety
/// Called on the game thread from the per-fighter frame callback.
pub(crate) unsafe fn frame() {
    let Some(base) = base() else { return };
    let _scope = HookScope::enter_hook(HookId::CpuLevel);
    let level = crate::runtime::published::cpu_level();
    let (place, menu_level) = place(base);
    let drill = super::edge_juggle::running();
    let Some(mut state) = STATE.try_lock() else { return };
    // Diag build: each change of place (the waiting room to a match is `lobby -> elsewhere`: the step stops there).
    #[cfg(feature = "diag")]
    {
        let (name, _) = place_tag(place);
        if name != state.place_seen {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[CpuLevel] place {} -> {name} mod={}", state.place_seen, level_number(level)
            ));
            state.place_seen = name;
        }
    }
    if place == Where::Elsewhere {
        // The native Training panel is gone. Never carry its saved selection to another scene.
        state.drill_behavior = BehaviorLease::new();
        state.drill_partner = PartnerLease::new();
        state.drill_pause = BehaviorPause::Idle;
        state.drill_panel = 0;
        DRILL_APPLIED.store(false, Ordering::Release);
        state.forget_entries();
        return;
    }
    let session = state.session;
    if let Some(notice) = state.gate.poll(session, level, menu_level) {
        post(notice);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!(
            "[CpuLevel] notice session={} mod={} menu={}", session.unwrap_or(0), notice.level, notice.menu
        ));
    }
    #[cfg(feature = "diag")]
    if state.gate.judged() != state.judged {
        state.judged = state.gate.judged();
        crate::runtime::diag_log::diagnostic(format_args!(
            "[CpuLevel] judged session={} mod={} menu={}",
            state.judged.unwrap_or(0),
            level_number(level),
            menu_level.unwrap_or(0)
        ));
    }
    if !drill && !state.drill_behavior.held() && !state.drill_partner.held() && level == CpuLevel::Off && !state.entries.iter().any(EntryRank::holds) && !cfg!(feature = "diag") {
        return;
    }
    // Busy: a hot swap or an install owns the entries this frame.
    if crate::runtime::roster::character_roster::busy() {
        return;
    }
    let drill_rank = drill_applied().then(super::edge_juggle::stock_level).flatten();
    let pause_rank = (drill && drill_rank.is_none()) || (!drill && (state.drill_behavior.held() || state.drill_partner.held()));
    let table = field::read_unchecked::<[i32; 8]>(base, RANK_TABLE);
    let entries = if place == Where::Lobby { rules::LOBBY_ENTRIES } else { 1..=7 };
    let opponent = if place == Where::Lobby { *rules::LOBBY_ENTRIES.start() } else { 1 };
    for entry in entries {
        if entry == opponent && pause_rank {
            continue;
        }
        let slot = entry as usize;
        let fighter_entry = game::natives::fighter_entry(entry);
        let controller = if fighter_entry == 0 { None } else { field::child(fighter_entry, ENTRY_CONTROLLER) };
        let Some(controller) = controller else {
            state.entries[slot].forget();
            state.owners[slot] = 0;
            continue;
        };
        if state.owners[slot] != controller {
            state.entries[slot].forget();
            state.owners[slot] = controller;
        }
        let Some(ai) = field::child(controller, CONTROLLER_AI) else { continue };
        let current = field::read_unchecked::<i32>(ai, AI_RANK);
        let target = rules::drill_target(drill_rank, level, place, entry, table);
        #[cfg(feature = "diag")]
        if state.seen[slot] != current {
            state.seen[slot] = current;
            let (name, behavior) = place_tag(place);
            crate::runtime::diag_log::diagnostic(format_args!(
                "[CpuLevel] rank place={name} behavior={behavior} entry={entry} ai_rank={current} target={} mod={}",
                target.map_or(-1, i32::from),
                level_number(level)
            ));
        }
        let Some(rank) = state.entries[slot].step(target, current.clamp(0, 100) as u8) else { continue };
        // The native writers' pair: the setter, then the entry's copy (fighter control select reads it back).
        let set_rank: extern "C" fn(usize, i32) = std::mem::transmute(base + AI_SET_RANK);
        set_rank(controller, i32::from(rank));
        field::write_unchecked::<u8>(fighter_entry, ENTRY_RANK, rank);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!(
            "[CpuLevel] apply place={} entry={entry} from={current} rank={rank} ai_rank={}",
            place_tag(place).0,
            field::read_unchecked::<i32>(ai, AI_RANK)
        ));
    }
}

pub(super) fn drill_applied() -> bool {
    DRILL_APPLIED.load(Ordering::Acquire)
}

pub(super) fn partner_held() -> bool {
    DRILL_PARTNER_HELD.load(Ordering::Acquire)
}

pub(super) fn drill_started() {
    DRILL_APPLIED.store(false, Ordering::Release);
}

/// Change between AI types on P1's poll. A change involving Control or a non-CPU operator uses the
/// normal pause/resume transaction to apply the controller and operation flags.
pub(super) fn poll_drill(quiet: bool) -> bool {
    if !DRILL_BEHAVIOR.load(Ordering::Acquire) {
        return false;
    }
    let Some(base) = base() else { return false };
    let Some(mut state) = STATE.try_lock() else { return false };
    let result = poll_drill_locked(base, &mut state, quiet);
    DRILL_PARTNER_HELD.store(state.drill_partner.held(), Ordering::Release);
    result
}

fn poll_drill_locked(base: usize, state: &mut State, quiet: bool) -> bool {
    let scene = crate::runtime::scene::practice_scene();
    if scene.is_none() {
        state.drill_behavior = BehaviorLease::new();
        state.drill_partner = PartnerLease::new();
        state.drill_pause = BehaviorPause::Idle;
        state.drill_panel = 0;
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    }
    let level = super::edge_juggle::stock_level();
    if level.is_none() && !state.drill_behavior.held() && !state.drill_partner.held() && state.drill_pause == BehaviorPause::Idle {
        return false;
    }
    let owners = crate::runtime::published::input_owners();
    if owners.bits() & !Owners::VANILLA_PAUSE.bits() != 0
        || super::states::is_loading()
        || crate::runtime::roster::character_roster::busy()
    {
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    }
    if scene == Some(crate::model::scene::Scene::Lobby) {
        // SAFETY: the P1 controller poll in waiting-room practice owns the live partner entry.
        unsafe { step_drill_partner(base, state, level) };
        return false;
    }
    // SAFETY: the P1 controller poll in native Training owns the live panel and entry.
    unsafe { step_drill_panel(base, state, level, owners.contains(Owners::VANILLA_PAUSE), quiet) }
}

/// The waiting room has no pause panel: save the first partner's Training Partner Behavior type, ask for stock CPU,
/// and restore the saved type at Stop. Use the game's own setter and the entry's copy, as between AI kinds in Training.
unsafe fn step_drill_partner(base: usize, state: &mut State, level: Option<u8>) {
    let entry = game::natives::fighter_entry(crate::model::cpu::role::LOBBY_CPU_ENTRY);
    let Some(controller) = field::child(entry, ENTRY_CONTROLLER) else {
        state.drill_partner.absent(level, super::edge_juggle::practice_cpu::restoring());
        DRILL_APPLIED.store(false, Ordering::Release);
        return;
    };
    let Some(ai) = field::child(controller, CONTROLLER_AI) else {
        state.drill_partner.absent(level, super::edge_juggle::practice_cpu::restoring());
        DRILL_APPLIED.store(false, Ordering::Release);
        return;
    };
    let entry_type = field::read_unchecked::<u8>(entry, ENTRY_BEHAVIOR);
    let ai_type = field::read_unchecked::<u8>(ai, AI_BEHAVIOR);
    let cpu = game::natives::is_operation_cpu(crate::model::cpu::role::LOBBY_CPU_ENTRY);
    if let Some(control) = state.drill_partner.step(level, controller, entry_type, ai_type, cpu) {
        if ai_type != control {
            let set_behavior: extern "C" fn(usize, i32) = std::mem::transmute(base + AI_SET_BEHAVIOR);
            set_behavior(controller, i32::from(control));
        }
        if field::read_unchecked::<u8>(ai, AI_BEHAVIOR) == control {
            field::write_unchecked(entry, ENTRY_BEHAVIOR, control);
        }
    }
    DRILL_APPLIED.store(level.is_some() && cpu
        && field::read_unchecked::<u8>(entry, ENTRY_BEHAVIOR) == 0
        && field::read_unchecked::<u8>(ai, AI_BEHAVIOR) == 0, Ordering::Release);
}

unsafe fn step_drill_panel(base: usize, state: &mut State, level: Option<u8>, paused: bool, quiet: bool) -> bool {
    let resolved = field::read::<usize>(base, TRAINING_ROOT)
        .and_then(|root| field::chain(root, &PANEL_CHAIN[..HUD_DEPTH]))
        .and_then(|hud| {
            let panel = field::chain(hud, &PANEL_CHAIN[HUD_DEPTH..])?;
            let menu = BehaviorMenu::from_stock(field::read_unchecked(hud, HUD_PAUSE_STATE), paused);
            Some((panel, menu))
        });
    let Some((panel, menu)) = resolved else {
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    };
    if state.drill_panel != panel {
        state.drill_pause = BehaviorPause::Idle;
        state.drill_panel = panel;
    }
    let entry = game::natives::fighter_entry(1);
    let Some(controller) = field::child(entry, ENTRY_CONTROLLER) else {
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    };
    let Some(ai) = field::child(controller, CONTROLLER_AI) else {
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    };
    let current = CpuBehavior {
        menu: field::read_unchecked(panel, PANEL_BEHAVIOR),
        level: field::read_unchecked(panel, PANEL_LEVEL),
    };
    if current.control_type().is_none() || current.level > 8 {
        DRILL_APPLIED.store(false, Ordering::Release);
        return false;
    }
    let entry_type = field::read_unchecked(entry, ENTRY_BEHAVIOR);
    let ai_type = field::read_unchecked(ai, AI_BEHAVIOR);
    let cpu = game::natives::is_operation_cpu(1);
    let applied = current.applied(entry_type, ai_type, cpu);
    let desired = state.drill_behavior.step(level, panel, current, applied);
    let needs_stock = desired.is_some_and(|desired| !applied || desired.menu != current.menu);
    if needs_stock {
        if let Some((desired, control)) = desired
            .and_then(|d| Some((d, d.direct(entry_type, ai_type, cpu, menu, state.drill_pause)?)))
        {
            // Between AI kinds the game's own apply is its type setter and the entry's copy: no pause is needed.
            if ai_type != control {
                let set_behavior: extern "C" fn(usize, i32) = std::mem::transmute(base + AI_SET_BEHAVIOR);
                set_behavior(controller, i32::from(control));
            }
            if field::read_unchecked::<u8>(ai, AI_BEHAVIOR) == control {
                field::write_unchecked(entry, ENTRY_BEHAVIOR, control);
                field::write_unchecked(panel, PANEL_LEVEL, desired.level);
                field::write_unchecked(panel, PANEL_BEHAVIOR, desired.menu);
                DRILL_APPLIED.store(false, Ordering::Release);
                return false;
            }
        }
    }
    if let Some(desired) = desired.filter(|_| menu == BehaviorMenu::Open || menu == BehaviorMenu::Closed && !needs_stock) {
        if desired != current {
            field::write_unchecked(panel, PANEL_LEVEL, desired.level);
            field::write_unchecked(panel, PANEL_BEHAVIOR, desired.menu);
        }
    }
    DRILL_APPLIED.store(level.is_some() && menu == BehaviorMenu::Closed && !needs_stock, Ordering::Release);
    state.drill_pause.pulse(needs_stock, menu, quiet)
}
