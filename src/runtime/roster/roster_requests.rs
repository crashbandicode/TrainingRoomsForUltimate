//! What the menu worker asks of the roster, handed to the game frame (the lock-free runtime, operator order 2026-11-10 00:44Z,
//! `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`). The roster's admission takes its role states, the Mii owners' states, the
//! sources' and the planner's locks, which the game's hooks wait on, so a plugin thread never runs it: the worker posts the request
//! here and the game frame (`lobby::update`, on MainThread) runs `character_roster::request_*_kind` as the worker did. A request that
//! needs its answer (the combo panel's switch at menu close) gets it back through [`take_reply`]. Player 1's fighter, which the
//! worker reads, is a copy the game frame keeps current ([`player_kind`]).
//!
//! The queues are [`DropRing`]s: one producer each (the worker for requests, the game frame for replies), bounded and
//! preallocated; a full ring drops the oldest and counts it, never waits.
use std::sync::atomic::{AtomicU32, Ordering};

use crate::model::drop_ring::DropRing;

const PLAYER: u64 = 0;
const CPU: u64 = 1;
const PLAYER_REPLIED: u64 = 2;
const NONE: u32 = u32::MAX;

/// The worker's requests: `[what, fighter kind]`.
static REQUESTS: DropRing<2, 8> = DropRing::new();
/// The game frame's answers to `PLAYER_REPLIED`: `[fighter kind, queued]`.
static REPLIES: DropRing<2, 8> = DropRing::new();
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
            CPU => {
                let _ = super::character_roster::request_cpu_kind(kind);
            }
            PLAYER => {
                let _ = super::character_roster::request_player_kind(kind);
            }
            _ => {
                let queued = super::character_roster::request_player_kind(kind);
                REPLIES.push([u64::from(kind), u64::from(queued)]);
                replied = true;
            }
        }
    }
    let dropped = REQUESTS.take_dropped();
    if dropped != 0 {
        crate::engine::compat::logging::diagnostic(format_args!("[CharacterRoster] worker requests dropped={dropped} (a full ring)"));
    }
    if replied {
        crate::runtime::ui::worker::wake();
    }
}
