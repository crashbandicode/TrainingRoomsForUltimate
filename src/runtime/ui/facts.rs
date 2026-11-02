//! The game facts the menu's entry points need, published from game-synchronized context (lead ruling 2026-11-02: game-state
//! reads run in the frame wrapper or the UI layout tick, never on the free-running menu worker thread, where they would race the
//! game). `macros::frame_after` calls [`refresh`] once per game frame on the primary CPU body, so a reader on any thread sees an
//! atomic that is at most one frame old; when no frame has refreshed them for [`FRESH_MS`] (the scene ended, the CPU body is gone)
//! every fact reads as "not ready". No allocation.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use smash::app::{utility, BattleObjectModuleAccessor, Fighter};

use crate::runtime::clock;
use crate::runtime::features::recorder;

/// A fact older than this is stale (two UI polls of the menu worker, a few frames of the game).
pub const FRESH_MS: u64 = 250;

const NO_KIND: i32 = i32::MIN;

static STAMP_MS: AtomicU64 = AtomicU64::new(0);
static READY_GO: AtomicBool = AtomicBool::new(false);
static PRESENT: AtomicBool = AtomicBool::new(false);
static CPU_KIND: AtomicI32 = AtomicI32::new(NO_KIND);

/// Publish the facts of this frame.
///
/// # Safety
/// Called from the frame wrapper of the primary CPU body (`cpu` is that body's live accessor), the game thread.
pub unsafe fn refresh(cpu: &mut BattleObjectModuleAccessor) {
    READY_GO.store(Fighter::is_ready_go(), Ordering::Release);
    PRESENT.store(recorder::body_of(0).is_some(), Ordering::Release);
    CPU_KIND.store(utility::get_kind(cpu), Ordering::Release);
    STAMP_MS.store(clock::uptime_ms().max(1), Ordering::Release);
}

/// The scene ended or the bodies were destroyed: nothing is ready until the next frame says so.
pub fn clear() {
    STAMP_MS.store(0, Ordering::Release);
}

fn fresh() -> bool {
    let stamp = STAMP_MS.load(Ordering::Acquire);
    stamp != 0 && clock::uptime_ms().saturating_sub(stamp) <= FRESH_MS
}

/// `Fighter::is_ready_go()` as of the last frame.
pub fn ready_go() -> bool {
    fresh() && READY_GO.load(Ordering::Acquire)
}

/// Player 1's body and the CPU's body both exist (the practice session's two fighters).
pub fn fighters_present() -> bool {
    fresh() && PRESENT.load(Ordering::Acquire)
}

/// The CPU's fighter kind as of the last frame.
pub fn cpu_kind() -> Option<i32> {
    let kind = CPU_KIND.load(Ordering::Acquire);
    (fresh() && kind != NO_KIND).then_some(kind)
}
