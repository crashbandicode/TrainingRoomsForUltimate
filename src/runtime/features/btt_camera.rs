//! Shared practice placement camera (rc.14, `artifacts/rc14-eggsercise/SPEC.md` §V2.5 `camera`), the runtime half of
//! `model::btt_camera`: the game's own Training Fixed camera (mode 4) shows the whole stage while the mode is Placing, and the camera the game was in comes back on every placement exit. The switch is the game's own `change_active_camera`
//! (`game::natives::change_active_camera`), asked from Player 1's pass on the game thread with no lock held (the camera hooks
//! take the CPU lock). The row is the camera unit's: a rejected guard turns off this camera and nothing else (boundaries in
//! `tests/capabilities.rs`). Eden evidence: `tmp-deluxe/btt-camera/FINDINGS.md`, run `d2-btt-cam-1`.

use std::sync::atomic::{AtomicBool, Ordering};

use std::cell::Cell;

use crate::model::btt::{self as model, Camera, CameraRequests};
use crate::model::sites_pkg2;
use crate::runtime::game::{self, natives};
use crate::runtime::{published, scene};

/// The camera unit's rows passed their guards (boot sets it); nothing here asks the game before.
static USABLE: AtomicBool = AtomicBool::new(false);
/// This mode holds the Fixed camera: the Hide Tech camera hooks put the game's own centre in, not Hide Tech's (`holding`).
static HOLD: AtomicBool = AtomicBool::new(false);
/// Owners publish atomically; only the game thread holds save/restore state.
static OWNERS: CameraRequests = CameraRequests::idle();
struct GameCamera(Cell<Camera>);
// SAFETY: only frame(), on the game thread, accesses this cell; state is copied and committed before native calls.
// Switch std thread_local! allocates TLS boxes, so the single game-thread state stays in static storage.
unsafe impl Sync for GameCamera {}
static CAMERA: GameCamera = GameCamera(Cell::new(Camera::IDLE));

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

/// Editors publish their game-thread placement state. Tech may also own the camera during a run.
pub(super) fn request(owner_bit: u8, want: bool) {
    OWNERS.request(owner_bit, want);
}

/// End the borrowed session even if a feature opens a fresh editor in the same P1 pass.
pub(crate) fn interrupt_placement() {
    OWNERS.interrupt();
}

/// Both CPU dispatch paths use the same session as the camera. This is an
/// atomic read, independent of whether the native camera rows were admitted.
pub(super) fn placing() -> bool {
    OWNERS.placement(covered())
}

fn covered() -> bool {
    published::input_owners().covers_strip() || super::states::is_loading()
}

pub(super) fn placement_pad(buttons: u32) -> Option<crate::model::edge_juggle::perfect::Correction> {
    if placing() { crate::model::edge_juggle::Phase::Place.held_pad(buttons) } else { None }
}

/// Step once per game frame, after placing owners publish their requests.
/// # Safety
/// Player 1's non-Nana game-thread callback; no plugin lock is held.
pub unsafe fn frame() {
    OWNERS.request(model::EGG_ZONE_OWNER, super::egg_zone::wizard_active());
    OWNERS.request(model::EDGE_JUGGLE_OWNER, super::edge_juggle::placement_active());
    let want = OWNERS.camera_wanted(covered());
    if !USABLE.load(Ordering::Acquire) { return; }
    let manager = manager();
    let mode = (manager != 0 && scene::current().is_some()).then(|| ((manager + sites_pkg2::CAMERA_MODE_FIELD) as *const i32).read_volatile());
    let mut camera = CAMERA.0.get();
    let act = camera.step(want, mode);
    HOLD.store(camera.holding(), Ordering::Release);
    CAMERA.0.set(camera);
    let Some(to) = act else { return };
    let changed = natives::change_active_camera(manager, to);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::enqueue_diagnostic(format_args!(
        "[Btt] camera want={want} mode_before={mode:?} mode_after={} changed={changed:?}",
        ((manager + sites_pkg2::CAMERA_MODE_FIELD) as *const i32).read_volatile()
    ));
    #[cfg(not(feature = "diag"))]
    let _ = changed;
}
