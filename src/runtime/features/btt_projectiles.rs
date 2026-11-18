//! Optional BTT projectile boundary: a rejected accessor guard leaves melee and Hitbox installed.
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "diag")]
use crate::model::eggs;
use crate::runtime::state_natives;
use super::{btt, hitbox};
static READY: AtomicBool = AtomicBool::new(false);
pub fn set_ready(on: bool) { READY.store(on, Ordering::Release); }
pub fn ready() -> bool { READY.load(Ordering::Acquire) }
/// # Safety
/// Called on the game thread with a live fighter accessor.
pub unsafe fn hit(owner: usize, spot: [f32; 2]) -> bool {
    if !READY.load(Ordering::Acquire) || !btt::projectiles() { return false; }
    let pool = crate::engine::seams::article_pool::read(owner, crate::runtime::game::text_base());
    crate::model::btt::projectile_breaks(true, spot, pool.into_iter().flatten(),
        |kind, ordinal| state_natives::article_get_from_no(owner, kind, ordinal),
        |article| {
            let id = state_natives::article_object_id(article);
            if !state_natives::object_is_active(id) { return None; }
            let at = state_natives::object_accessor(id);
            if !crate::model::btt::pointer_ok(at) { return None; }
            let (hits, n) = hitbox::object_hits(at);
            #[cfg(feature = "diag")]
            if hits[..n].iter().any(|h| h.3 > 0.0 && eggs::breaks(eggs::ANY, false, h, spot)) {
                crate::runtime::diag_log::diagnostic(format_args!("[Btt] projectile id={id:#x} owner={owner:#x} at={at:#x} hits={n} target={spot:?}"));
            }
            Some((hits, n))
        })
}

