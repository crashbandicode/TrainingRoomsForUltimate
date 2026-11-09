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

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::engine::seams::native_fields as field;
use crate::model::cpu::level::{self as rules, CpuLevel, EntryRank, NoticeGate, SessionClock, Where};
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
/// `cpu_level.rank_table`: the ranks of Lv 2..=9.
const RANK_TABLE: usize = 0x4470dc0;
/// The Training state root and the pause menu's chain (`cpu_level.training_rank_choice`, as `roster_cpu_quantity_root`).
const TRAINING_ROOT: usize = 0x52c3420;
const PANEL_CHAIN: [usize; 7] = [8, 0, 0xb38, 0, 0x210, 8, 0x80];
/// The pause menu's CPU Behavior and Training's own level index (`cpu_level.training_level_label`).
const PANEL_BEHAVIOR: usize = 0xb4c;
const PANEL_LEVEL: usize = 0xb50;
/// FighterEntry: the controller and the rank byte the native writers store (`cpu_level.training_rank_write`).
const ENTRY_CONTROLLER: usize = 0x4178;
const ENTRY_RANK: usize = 0x37;
/// The controller's AI and the AI's rank (`cpu_level.ai_set_rank`'s own loads).
const CONTROLLER_AI: usize = 0x58;
const AI_RANK: usize = 0xc0c;
/// About four seconds on the plate.
const NOTICE_FRAMES: u16 = 240;

/// `main`'s base once every row passed the authoritative gate at boot, else 0: the only base this module forms an
/// address with (the seam-gated natives' rule, as `cpu_pilot` and the Mii bridge keep theirs).
static BASE: AtomicUsize = AtomicUsize::new(0);

/// The base, or `None` before (or without) the gate.
fn base() -> Option<usize> {
    Some(BASE.load(Ordering::Acquire)).filter(|base| *base != 0)
}

struct State {
    clock: SessionClock,
    session: Option<u64>,
    gate: NoticeGate,
    entries: [EntryRank; 8],
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
unsafe fn training_menu(base: usize) -> Option<(u32, u32)> {
    let root = field::read::<usize>(base, TRAINING_ROOT)?;
    let panel = field::chain(root, &PANEL_CHAIN)?;
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
        // A match or any other scene: nothing is read or written, and its fighters are not the ones the overrides were for.
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
    if level == CpuLevel::Off && !state.entries.iter().any(EntryRank::holds) && !cfg!(feature = "diag") {
        return;
    }
    // Busy: a hot swap or an install owns the entries this frame.
    if crate::runtime::roster::character_roster::busy() {
        return;
    }
    let table = field::read_unchecked::<[i32; 8]>(base, RANK_TABLE);
    let entries = if place == Where::Lobby { rules::LOBBY_ENTRIES } else { 1..=7 };
    for entry in entries {
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
        let target = rules::target(level, place, entry, table);
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
