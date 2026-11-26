//! Diag builds only: what a game run reads beside the shared hold's seam (`placement::hold_at`) to compare one
//! float of a held body with another. The body's animation, its status and the animation's frame have no use in
//! the product, so the shipped hold does not read them (rv-edge-juggle-sol, 2026-11-26 15:45:47Z).
use crate::model::sites_pkg2 as s;
use crate::runtime::game::{self, natives};

/// Read by the tracer: the seam's own first word (the body's id, its flags, the act and the hold's clock), the
/// body's animation, its status with the animation's frame (f32 bits) above it, and which of the three reads are
/// granted (bit 0 the animation, 1 the status, 2 the frame): a read that is not granted is 0, not a 0 that was read.
#[inline(never)]
#[no_mangle]
pub extern "C" fn placement_hold_state(meta: u64, motion: u64, status: u64, granted: u64) {
    std::hint::black_box((meta, motion, status, granted));
}

/// The body as the game left it, before the hold's write of this frame.
pub(super) unsafe fn hold_state(meta: u64, body: usize) {
    let granted = [s::MOTION_KIND_OFFSET, s::STATUS_KIND_OFFSET, s::MOTION_FRAME_OFFSET]
        .into_iter()
        .enumerate()
        .fold(0, |bits, (bit, site)| bits | (u64::from(game::granted(site)) << bit));
    placement_hold_state(
        meta,
        natives::motion_kind(body),
        u64::from(natives::status_kind(body) as u32) | (u64::from(natives::motion_frame(body).to_bits()) << 32),
        granted,
    );
}
