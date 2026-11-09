//! Package 6, lobby side (worker B): the waiting-room practice engine's game-facing files (carried by `tools/carry_pkg6.py`), the
//! scene observation they share (`game_mode`) and the stage rotation. Switch only.
//!
//! The functions at the bottom are the call sites the fork's mixed code made (decision D5): `init` once from boot after the roster
//! cycle, `update` once per game frame, `observe` from the fighter init path and `frame` from the per-fighter frame callback.

pub(crate) mod automation_helper;
pub(crate) mod game_mode;
#[cfg(feature = "practice-stage")]
pub(crate) mod lobby_items;
pub(crate) mod lobby_persist;
pub(crate) mod lobby_stage;
#[cfg(feature = "practice-stage")]
pub(crate) mod practice_stage;
pub(crate) mod practice_stage_request;
pub(crate) mod training;
pub(crate) mod training_stage_rotation;

use smash::app::BattleObjectModuleAccessor;

use crate::model::lra_reset::{entry_id_valid, FIGHTER_ENTRY_ID_FIELD};
use crate::runtime::game;

/// Start the practice engine, the feature-gated practice update hook (it initializes stage requests only after its native
/// checks pass), then the stage rotation. Called once from boot, after the roster cycle's own `init` and before the install line.
///
/// The practice observer runs first, as the fork does (`src/lib.rs`: `practice_stage_observer::init` before
/// `lobby_stage_probe::init`, "Verify original resource instructions before the entry probe patches them"): its seam group reads
/// `lobby_stage.resource_request` (`0x25f7874`), which `lobby_stage::init` then hooks. rc.2 had the other order: the row read our own
/// hook (`occupied`) and the group skipped all its sites. The seam installer now reads our own writes as the bytes they replaced
/// (`seam_install::prime` / `seal`, `model::own_patches`), so either order verifies; this one stays the fork's.
pub(crate) fn init() {
    training::lobby::init();
    #[cfg(feature = "practice-stage")]
    practice_stage::init();
    // Lobby Items (N11) compares the observer's item-gate rows too; its own hook site is nobody else's.
    #[cfg(feature = "practice-stage")]
    lobby_items::init();
    // The waiting room's practice panel: saved Training partners and behavior (rc.6). Its sites are nobody else's.
    lobby_persist::init();
    lobby_stage::init();
}

/// Once per game frame, before the input processing (the input poll's note for Player 1).
pub(crate) fn update() {
    training::lobby::update();
    // A practice session starts (or ends) with the lobby practice: a lease left from before is dropped silently, the next frame
    // acquires a fresh one (`frame`). Training's session starts with the scene (the lease is empty then).
    static WAS_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let active = training::lobby::active();
    if active != WAS_ACTIVE.swap(active, std::sync::atomic::Ordering::AcqRel) {
        training::practice::forget();
    }
}

/// A fighter's on-init has run (the reset hub's wrapper): the practice engine records the entry.
///
/// # Safety
/// `fighter` is the fighter object the game just initialised.
pub(crate) unsafe fn observe(fighter: usize) {
    let accessor = game::natives::fighter_accessor(fighter);
    if accessor != 0 {
        training::lobby::observe_fighter(&mut *(accessor as *mut BattleObjectModuleAccessor));
    }
}

/// A fighter's frame has run: Player 1's body refreshes the practice entry lease (once per game frame).
///
/// # Safety
/// `fighter` is the fighter object the game just ran a frame of.
pub(crate) unsafe fn frame(fighter: usize) {
    let entry = ((fighter + FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    if entry_id_valid(entry) && entry == 0 {
        // Entries replaced (a hot swap) retire the session first, then a session with both entries ready acquires the lease: the
        // practice session the save and record slots belong to. `entry_departed` (the roster's destruction path) retires at once.
        let lease = crate::model::hook_stats::HookScope::enter_hook(crate::model::hook_stats::HookId::PracticeLease);
        training::practice::refresh();
        #[cfg(feature = "diag")]
        let held = training::practice::owned();
        if !training::practice::owned() && training::practice::enabled() {
            training::practice::acquire();
        }
        // Diag build: one line per change of the practice session (the entry lease), so an Eden row sees a hot swap retire it
        // (`refresh` found an entry replaced) and the next frame acquire a fresh one.
        #[cfg(feature = "diag")]
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            static LOGGED: AtomicBool = AtomicBool::new(false);
            let now = training::practice::owned();
            // Why a session is not held (once until it changes): the practice context and the two entries the lease needs.
            static REFUSAL: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
            if !now {
                let (enabled, p1, cpu) =
                    (training::practice::enabled(), training::practice::entry(0).is_some(), training::practice::entry(1).is_some());
                let code = 1 | (u8::from(enabled) << 1) | (u8::from(p1) << 2) | (u8::from(cpu) << 3);
                if REFUSAL.swap(code, Ordering::Relaxed) != code {
                    crate::runtime::diag_log::diagnostic(format_args!(
                        "[TrainingRooms] practice_session refused context={:?} enabled={enabled} p1_entry={p1} cpu_entry={cpu}",
                        training::practice::context()
                    ));
                }
            } else {
                REFUSAL.store(0, Ordering::Relaxed);
            }
            if held != LOGGED.load(Ordering::Relaxed) || now != held {
                if held != LOGGED.load(Ordering::Relaxed) {
                    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] practice_session held={held} (retired or acquired before this frame)"));
                }
                if now != held {
                    crate::runtime::diag_log::diagnostic(format_args!("[TrainingRooms] practice_session held={now}"));
                }
                LOGGED.store(now, Ordering::Relaxed);
            }
        }
        drop(lease);
        // The roster cycle's once-per-game-frame steps run on Player 1's body (each returns at once when nothing is pending): the
        // source leases, the item engine (a granted or restored item) and the preload planner. The Kirby hat requests run per
        // Kirby with that Kirby's own body instead (`features::states::state_kirby_opff_hook`, the fork's call site).
        let accessor = game::natives::fighter_accessor(fighter);
        if accessor != 0 {
            let body = accessor as *mut BattleObjectModuleAccessor;
            use crate::model::hook_stats::{HookId, HookScope};
            {
                let _step = HookScope::enter_hook(HookId::RosterSources);
                crate::runtime::roster::character_roster::sources::on_frame(body);
            }
            {
                // HAT-REQUEST-END R1: the pending hat requests' clock, once per game frame.
                let _step = HookScope::enter_hook(HookId::RosterKirby);
                crate::runtime::roster::kirby_hats::tick();
            }
            let _items = HookScope::enter_hook(HookId::RosterItems);
            // Diag build: an item grant Eden asked for in `diag-item.req` (the Give item screen is not built yet).
            #[cfg(feature = "diag")]
            if let Some(choice) = crate::runtime::diag_reload::take_item_request() {
                crate::runtime::roster::character_items::request(choice);
            }
            crate::runtime::roster::character_items::on_frame(body);

            drop(_items);
            // Lobby Items (N11): the one removal an Off edge owes in the waiting room (nothing on the other frames).
            #[cfg(feature = "practice-stage")]
            lobby_items::frame();
            let _preload = HookScope::enter_hook(HookId::RosterPreload);
            crate::runtime::roster::cycle_preload::on_frame(&mut *body);
        }
    }
    // Diag build: the character-item table's constants as the game resolves them (once).
    // (Diagnostic instrumentation, not product code: its one-time text is not a hook allocation of the plugin.)
    #[cfg(feature = "diag")]
    crate::model::hook_stats::uncounted(training::character_specific::items::dump_once);
}
