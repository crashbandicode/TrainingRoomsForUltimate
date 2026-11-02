//! The game facts the menu's entry points need, published from game-synchronized context (lead ruling 2026-11-02: game-state
//! reads run in the frame wrapper or the UI layout tick, never on the free-running menu worker thread, where they would race the
//! game). `macros::frame_after` calls [`publish_frame_facts`] once per game frame on the primary CPU body, so a reader on any thread sees an
//! atomic that is at most one frame old; when no frame has refreshed them for [`FRESH_MS`] (the scene ended, the CPU body is gone)
//! every fact reads as "not ready". No allocation.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use smash::app::{lua_bind::StatusModule, utility, BattleObjectModuleAccessor, Fighter};

use crate::runtime::clock;

/// A fact older than this is stale (two UI polls of the menu worker, a few frames of the game).
pub const FRESH_MS: u64 = 250;

const NO_KIND: i32 = i32::MIN;

static STAMP_MS: AtomicU64 = AtomicU64::new(0);
static READY_GO: AtomicBool = AtomicBool::new(false);
static PRESENT: AtomicBool = AtomicBool::new(false);
static CPU_KIND: AtomicI32 = AtomicI32::new(NO_KIND);
static CPU_STATUS: AtomicI32 = AtomicI32::new(NO_KIND);
static CPU_SITUATION: AtomicI32 = AtomicI32::new(NO_KIND);
static SCENE_ENABLED: AtomicBool = AtomicBool::new(false);
static WAITING_SCENE: AtomicBool = AtomicBool::new(false);
static WAITING_READY: AtomicBool = AtomicBool::new(false);

/// UI-layout/game-frame boundary only. These scene facts survive a modal applet
/// pausing fighter frames; explicit session retirement invalidates them.
pub fn publish_scene_facts() {
    SCENE_ENABLED.store(crate::runtime::lobby::training::practice::scene_enabled(), Ordering::Release);
    WAITING_SCENE.store(crate::runtime::lobby::training::lobby::is_waiting_scene(), Ordering::Release);
}


/// Publish the facts of this frame.
///
/// # Safety
/// Called from the frame wrapper of the primary CPU body (`cpu` is that body's live accessor), the game thread.
pub unsafe fn publish_frame_facts(cpu: &mut BattleObjectModuleAccessor) {
    publish_scene_facts();
    READY_GO.store(Fighter::is_ready_go(), Ordering::Release);
    WAITING_READY.store(crate::runtime::lobby::training::lobby::active()
        && crate::runtime::roster::character_roster::waiting_drafts_ready(), Ordering::Release);
    CPU_STATUS.store(StatusModule::status_kind(cpu), Ordering::Release);
    CPU_SITUATION.store(StatusModule::situation_kind(cpu), Ordering::Release);
    // Package 6's provider is the one implementation of practice is acquired (both practice bodies present, native Training or lobby
    // practice): it reads the game here, on the game thread, and the worker reads the answer (lead, 2026-11-02 merge ruling).
    PRESENT.store(crate::runtime::lobby::training::practice::acquire(), Ordering::Release);
    CPU_KIND.store(utility::get_kind(cpu), Ordering::Release);
    STAMP_MS.store(clock::uptime_ms().max(1), Ordering::Release);
}

/// The scene ended or the bodies were destroyed: nothing is ready until the next frame says so.
pub fn forget_frame_facts() {
    STAMP_MS.store(0, Ordering::Release);
    SCENE_ENABLED.store(false, Ordering::Release);
    WAITING_SCENE.store(false, Ordering::Release);
    WAITING_READY.store(false, Ordering::Release);
}

fn fresh() -> bool {
    let stamp = STAMP_MS.load(Ordering::Acquire);
    stamp != 0 && clock::uptime_ms().saturating_sub(stamp) <= FRESH_MS
}

/// `Fighter::is_ready_go()` as of the last frame.
pub fn fact_ready_go() -> bool {
    fresh() && READY_GO.load(Ordering::Acquire)
}

/// Player 1's body and the CPU's body both exist (the practice session's two fighters).
pub fn fact_fighters_present() -> bool {
    fresh() && PRESENT.load(Ordering::Acquire)
}

/// The CPU's fighter kind as of the last frame.
pub fn fact_cpu_kind() -> Option<i32> {
    let kind = CPU_KIND.load(Ordering::Acquire);
    (fresh() && kind != NO_KIND).then_some(kind)
}

/// Scene validity is explicitly retired, rather than expiring while a keyboard pauses the game.
pub fn fact_scene_enabled() -> bool { SCENE_ENABLED.load(Ordering::Acquire) }

pub fn fact_waiting_scene() -> bool { WAITING_SCENE.load(Ordering::Acquire) }

pub fn fact_practice_enabled() -> bool {
    fact_scene_enabled()
        && !crate::runtime::roster::character_roster::busy()
        && !crate::runtime::lobby::training::lobby::native_ui_busy()
}

pub fn fact_waiting_ready() -> bool {
    fact_practice_enabled() && fact_ready_go() && fact_fighters_present()
        && WAITING_READY.load(Ordering::Acquire)
}

pub fn fact_cpu_status() -> Option<i32> {
    fresh().then(|| CPU_STATUS.load(Ordering::Acquire)).filter(|status| *status != NO_KIND)
}

pub fn fact_cpu_situation() -> Option<i32> {
    fresh().then(|| CPU_SITUATION.load(Ordering::Acquire)).filter(|situation| *situation != NO_KIND)
}
