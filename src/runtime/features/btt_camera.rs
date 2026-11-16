//! Break the Targets' camera (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2.5 `camera`), the runtime half of
//! `model::btt_camera`: the game's own Training Fixed camera (mode 4) shows the whole stage while the mode is Placing, and the camera the game was in comes back on every placement exit. The switch is the game's own `change_active_camera`
//! (`game::natives::change_active_camera`), asked from Player 1's pass on the game thread with no lock held (the camera hooks
//! take the CPU lock). The row is the camera unit's: a rejected guard turns off this camera and nothing else (boundaries in
//! `tests/capabilities.rs`). Eden evidence: `tmp-deluxe/btt-camera/FINDINGS.md`, run `d2-btt-cam-1`.

use std::sync::atomic::{AtomicBool, Ordering};

use std::cell::Cell;

use crate::model::btt::{self as model, Camera, CameraRequests};
use crate::model::sites_pkg2;
use crate::runtime::game::{self, natives};
use crate::runtime::scene;

/// The camera unit's rows passed their guards (boot sets it); nothing here asks the game before.
static USABLE: AtomicBool = AtomicBool::new(false);
/// This mode holds the Fixed camera: the Hide Tech camera hooks put the game's own centre in, not Hide Tech's (`holding`).
static HOLD: AtomicBool = AtomicBool::new(false);
/// Owners publish atomically; only the game thread holds save/restore state.
static OWNERS: CameraRequests = CameraRequests::IDLE;
thread_local! { static CAMERA: Cell<Camera> = const { Cell::new(Camera::IDLE) }; }

pub fn set_usable(on: bool) {
    USABLE.store(on, Ordering::Release);
}

/// Whether the mode holds the Fixed camera now: read by `cpu_camera_mode_hook` and `cpu_camera_setup_hook` before they take the CPU
/// lock (an atomic, no lock).
pub fn holding() -> bool {
    HOLD.load(Ordering::Acquire)
}

/// The camera manager, `**(main + 0x52b8f00)`, or 0 when either pointer does not read as an object.
unsafe fn manager() -> usize {
    let first = ((game::text_base() + sites_pkg2::CAMERA_MANAGER_SLOT_OFFSET) as *const usize).read_volatile();
    let manager = if model::pointer_ok(first) { (first as *const usize).read_volatile() } else { 0 };
    if model::pointer_ok(manager) {
        manager
    } else {
        0
    }
}

/// One pass of Player 1 (the game thread, no lock held): ask for the Fixed camera while `want`, give the game's own back when not,
/// by state (`model::btt_camera::Camera::step`), so a reset, a state load, a stage change or the game's Training Camera option that
/// took the camera is answered within `RETRY` frames. Nothing happens outside a practice scene or without a camera manager.
///
/// # Safety
/// The game thread, in a battle scene.
pub unsafe fn apply(want: bool) {
    if !USABLE.load(Ordering::Acquire) { return; }
    request(1, want);
}

/// Shared placing camera: BTT bit 1, Tech bit 2. The last owner restores the first native mode.
/// # Safety
/// Callers release their bit on every placing exit. The game thread applies requests in frame().
pub unsafe fn request(owner_bit: u8, want: bool) {
    if !USABLE.load(Ordering::Acquire) {
        return;
    }
    OWNERS.request(owner_bit, want);
}

/// Step once per game frame, after placing owners publish their requests.
/// # Safety
/// Player 1's non-Nana game-thread callback; no plugin lock is held.
pub unsafe fn frame() {
    if !USABLE.load(Ordering::Acquire) { return; }
    let manager = manager();
    let mode = (manager != 0 && scene::current().is_some()).then(|| ((manager + sites_pkg2::CAMERA_MODE_FIELD) as *const i32).read_volatile());
    let want = OWNERS.wanted();
    let act = CAMERA.with(|state| {
        let mut camera = state.get();
        let act = camera.step(want, mode);
        HOLD.store(camera.holding(), Ordering::Release);
        state.set(camera);
        act
    });
    let Some(to) = act else { return };
    let changed = natives::change_active_camera(manager, to);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[Btt] camera want={want} mode_before={mode:?} mode_after={} changed={changed:?}",
        ((manager + sites_pkg2::CAMERA_MODE_FIELD) as *const i32).read_volatile()
    ));
    #[cfg(not(feature = "diag"))]
    let _ = changed;
}
