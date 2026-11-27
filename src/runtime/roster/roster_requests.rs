//! What the menu worker asks of the roster, handed to the game frame (the lock-free runtime, operator order 2026-11-10 00:44Z,
//! `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`). The roster's admission takes its role states, the Mii owners' states, the
//! sources' and the planner's locks, which the game's hooks wait on, so a plugin thread never runs it: the worker posts the request
//! here and the game frame (`lobby::update`, on MainThread) runs `character_roster::request_*_kind` as the worker did. A request that
//! needs its answer (the combo panel's switch at menu close) gets it back through [`take_reply`]. Player 1's fighter, which the
//! worker reads, is a copy the game frame keeps current ([`player_kind`]).
//!
//! The queues are [`DropRing`]s: one producer each (the worker for requests, the game frame for replies), bounded and
//! preallocated; a full ring drops the oldest and counts it, never waits.
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::model::drop_ring::DropRing;

const PLAYER: u64 = 0;
const CPU: u64 = 1;
const PLAYER_REPLIED: u64 = 2;
const NONE: u32 = u32::MAX;

/// The worker's requests: `[what, fighter kind]`.
static REQUESTS: DropRing<2, 8> = DropRing::new();
/// The game frame's answers to `PLAYER_REPLIED`: `[fighter kind, queued]`.
static REPLIES: DropRing<2, 8> = DropRing::new();
// One clip switch at a time. Tags 0..=2 belong to the older request paths.
// Upper bits identify the switch; the low two bits of its answer are 0=pending/consumed, 1=yes, 2=no.
const FIRST_CLIP: u64 = 4;
static CLIP_REPLY: AtomicU64 = AtomicU64::new(FIRST_CLIP);
/// Player 1's fighter kind as the roster last observed it (`NONE` before a practice entry), as of the last game frame.
static PLAYER_KIND: AtomicU32 = AtomicU32::new(NONE);

/// The worker: ask the roster to change the CPU to `kind` (`character_roster::request_cpu_kind`, on the next game frame).
pub(crate) fn request_cpu_kind(kind: u32) {
    REQUESTS.push([CPU, u64::from(kind)]);
}

/// The worker: ask the roster to change Player 1 to `kind` (`character_roster::request_player_kind`, on the next game frame).
pub(crate) fn request_player_kind(kind: u32) {
    REQUESTS.push([PLAYER, u64::from(kind)]);
}

/// The worker: [`request_player_kind`], answered through [`take_reply`] with whether the roster queued it.
pub(crate) fn request_player_kind_replied(kind: u32) {
    REQUESTS.push([PLAYER_REPLIED, u64::from(kind)]);
}

/// The worker: the next answer, `(fighter kind, queued)`.
pub(crate) fn take_reply() -> Option<(u32, bool)> {
    REPLIES.pop().map(|[kind, queued]| (kind as u32, queued != 0))
}

/// Worker only. A late answer for a cancelled switch cannot answer its successor, even for the same kind.
pub(crate) fn request_clip_kind(player: bool, kind: u32) {
    let ticket = (CLIP_REPLY.load(Ordering::Relaxed) & !3).wrapping_add(4).max(FIRST_CLIP);
    CLIP_REPLY.store(ticket, Ordering::Release);
    REQUESTS.push([ticket | u64::from(!player), u64::from(kind)]);
}

pub(crate) fn take_clip_reply() -> Option<bool> {
    let word = CLIP_REPLY.load(Ordering::Acquire);
    if word & 3 == 0 { return None; }
    CLIP_REPLY.store(word & !3, Ordering::Relaxed);
    Some(word & 3 == 1)
}

/// What became of the clip switch the worker asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipOutcome { Asked, Admitted, Refused, Dropped, Withdrawn, Committed }

/// Worker. Ask to take the current ticket's request back.
pub(crate) fn withdraw_clip() {}

/// Worker. The current ticket's state; `None` when no ticket is in flight. Reading does not consume it.
pub(crate) fn clip_outcome() -> Option<ClipOutcome> {
    match CLIP_REPLY.load(Ordering::Acquire) & 3 {
        1 => Some(ClipOutcome::Admitted),
        2 => Some(ClipOutcome::Refused),
        _ => None,
    }
}

/// Worker. Forget the ticket after a final state was shown.
pub(crate) fn clear_clip() {}

/// The game side: the roster gave up the direct request of this role's clip ticket.
pub(crate) fn note_drop(player: bool) {
    let _ = player;
}

/// Player 1's fighter kind as the roster last observed it, as of the last game frame (the worker's read; never a lock).
pub(crate) fn player_kind() -> Option<u32> {
    Some(PLAYER_KIND.load(Ordering::Acquire)).filter(|kind| *kind != NONE)
}

/// The game frame (`lobby::update`): the worker's requests, in the order it posted them, and Player 1's fighter for its next read.
pub(crate) fn service() {
    PLAYER_KIND.store(super::character_roster::player_kind().unwrap_or(NONE), Ordering::Release);
    let mut replied = false;
    while let Some([what, kind]) = REQUESTS.pop() {
        let kind = kind as u32;
        match what {
            CPU => _ = super::character_roster::request_cpu_kind(kind),
            PLAYER => _ = super::character_roster::request_player_kind(kind),
            PLAYER_REPLIED => {
                let queued = super::character_roster::request_player_kind(kind);
                REPLIES.push([u64::from(kind), u64::from(queued)]);
                replied = true;
            }
            _ => {
                let queued = if what & 1 == 0 { super::character_roster::request_player_kind(kind) }
                    else { super::character_roster::request_cpu_kind(kind) };
                let ticket = what & !3;
                replied |= CLIP_REPLY.compare_exchange(ticket, ticket | if queued { 1 } else { 2 },
                    Ordering::Release, Ordering::Relaxed).is_ok();
            }
        }
    }
    #[cfg(feature = "diag")]
    crate::runtime::features::tech_spike::service_reference_cpu();
    crate::runtime::features::edge_juggle::practice_cpu::service_practice_cpu_selection();
    let dropped = REQUESTS.take_dropped();
    if dropped != 0 {
        crate::engine::compat::logging::diagnostic(format_args!("[CharacterRoster] worker requests dropped={dropped} (a full ring)"));
    }
    if replied {
        crate::runtime::ui::worker::wake();
    }
}
