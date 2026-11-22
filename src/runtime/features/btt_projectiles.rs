//! Optional P1-article boundary of both practice games (rc.15: Break the Targets and Eggs-ercise; docs/DECISIONS.md): a
//! rejected accessor guard leaves melee and Hitbox installed.
use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::btt::{self, Search};
#[cfg(feature = "diag")]
use crate::model::eggs;
use crate::model::hit_streak::{Motion, Streak};
use crate::runtime::{game, state_natives};
use super::hitbox;

static READY: AtomicBool = AtomicBool::new(false);
pub fn set_ready(on: bool) { READY.store(on, Ordering::Release); }
pub fn ready() -> bool { READY.load(Ordering::Acquire) }

/// # Safety
/// A live guarded fighter accessor and its article ID.
unsafe fn active_article(article: usize) -> Option<(u32, usize)> {
    btt::pointer_ok(article).then_some(())?;
    let id = state_natives::article_object_id(article);
    state_natives::object_is_active(id).then_some(())?;
    let at = state_natives::object_accessor(id);
    btt::pointer_ok(at).then_some((id, at))
}

/// Observe attack boundaries and owned live weapons before the target-scoring pass.
/// # Safety
/// Called on the game thread with a live fighter accessor.
pub unsafe fn sample(streak: &mut Streak, owner: usize, damaging: bool) -> Option<(u32, usize)> {
    if !streak.enabled() {
        return None;
    }
    let id = (*(owner as *const smash::app::BattleObjectModuleAccessor)).battle_object_id;
    let mut motion = Motion {
        status: game::natives::status_kind(owner),
        kind: game::natives::motion_kind(owner),
        frame: game::natives::motion_frame(owner),
    };
    // The looping rapid jab and its finishing animation are one attack.
    let rapid = crate::model::hash40::hash40("attack_100");
    if [rapid, crate::model::hash40::hash40("attack_100_start"), crate::model::hash40::hash40("attack_100_end")].contains(&motion.kind) {
        motion.kind = rapid;
        motion.frame = 0.0;
    }
    let body = streak.begin(id, motion, damaging);
    observe_articles(streak, owner, body);
    Some((id, body))
}

/// This part of the observation runs only when the existing article-accessor guards passed.
unsafe fn observe_articles(streak: &mut Streak, owner: usize, body: usize) {
    if !READY.load(Ordering::Acquire) {
        return;
    }
    let pool = crate::engine::seams::article_pool::read(owner, game::text_base());
    let current = crate::model::hitbox::article_instances(true, |kind, ordinal| state_natives::article_get_from_no(owner, kind, ordinal));
    for (id, at) in pool.into_iter().flatten().chain(current).filter_map(|article| active_article(article)) {
        streak.projectile(body, id, || {
            let (hits, n) = hitbox::object_hits(at);
            hits[..n].iter().any(|h| h.3 > 0.0)
        });
    }
}

/// The damaging object and depth of a qualifying hit among the owner's live articles.
/// # Safety
/// Called on the game thread with a live fighter accessor.
pub unsafe fn hit(owner: usize, spot: [f32; 2], rule: u8, aerial: bool, enabled: bool, search: Search) -> Option<(u32, f32)> {
    if !READY.load(Ordering::Acquire) || !enabled { return None; }
    let pool = crate::engine::seams::article_pool::read(owner, game::text_base());
    btt::projectile_contact(search, rule, aerial, spot, pool.into_iter().flatten(),
        |kind, ordinal| state_natives::article_get_from_no(owner, kind, ordinal),
        |article| {
            let (_id, at) = active_article(article)?;
            let (hits, n) = hitbox::object_hits(at);
            #[cfg(feature = "diag")]
            if hits[..n].iter().any(|h| h.3 > 0.0 && eggs::breaks(eggs::ANY, false, h, spot)) {
                crate::runtime::diag_log::diagnostic(format_args!("[Btt] projectile id={_id:#x} owner={owner:#x} at={at:#x} hits={n} target={spot:?} rule={rule}"));
            }
            Some((hits, n))
        }).map(|(article, depth)| (state_natives::article_object_id(article), depth))
}
