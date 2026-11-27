//! What the menu worker asks of the roster, handed to the game frame (the lock-free runtime, operator order 2026-11-10 00:44Z,
//! `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`). The roster's admission takes its role states, the Mii owners' states, the
//! sources' and the planner's locks, which the game's hooks wait on, so a plugin thread never runs it: the worker posts the request
//! here and the game frame (`lobby::update`, on MainThread) runs `character_roster::request_*_kind` as the worker did. A request that
//! needs its answer (the combo panel's switch at menu close) gets it back through [`take_reply`]. Player 1's fighter, which the
//! worker reads, is a copy the game frame keeps current ([`player_kind`]). The clip editor's fighter switch has a ticket: one
//! word that says what became of it ([`clip_outcome`]).
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
// The clip editor's fighter switch, one at a time, in one word. A request's tags 0..=2 belong to the older paths, so a
// clip request's tag is its ticket (the sequence and the role), never below `SEQ`.
//   bits 0-2    what became of it (`ClipOutcome`; 0: no switch in flight)
//   bit 3       the worker asked to take it back
//   bit 4       the roster is working, as of the last game frame
//   bit 5       the role: set for the CPU
//   bits 6-31   the sequence, never 0
//   bits 32-63  the fighter kind asked for
const STATE: u64 = 7;
const ASKED: u64 = 1;
const ADMITTED: u64 = 2;
const REFUSED: u64 = 3;
const DROPPED: u64 = 4;
const WITHDRAWN: u64 = 5;
const COMMITTED: u64 = 6;
const WITHDRAW: u64 = 8;
const BUSY: u64 = 16;
const ROLE_CPU: u64 = 32;
const SEQ: u64 = 64;
const SEQUENCE: u64 = 0xffff_ffc0;
const TICKET: u64 = SEQUENCE | ROLE_CPU;
static CLIP_REPLY: AtomicU64 = AtomicU64::new(0);
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

/// What became of the clip switch the worker asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipOutcome {
    /// Posted; the roster has not heard of it yet.
    Asked,
    /// The roster took it. It is applied when the roster is no longer busy and the game has the fighter.
    Admitted,
    /// The roster refused it at admission.
    Refused,
    /// The roster gave it up after admission and said so ([`note_drop`]).
    Dropped,
    /// Taken back before the roster began.
    Withdrawn,
    /// The worker asked to take it back after the roster had begun: it runs to its end.
    Committed,
}

/// Worker only. A new ticket: a late answer for an earlier switch cannot answer this one, even for the same kind.
pub(crate) fn request_clip_kind(player: bool, kind: u32) {
    let sequence = ((CLIP_REPLY.load(Ordering::Relaxed) & SEQUENCE).wrapping_add(SEQ) & SEQUENCE).max(SEQ);
    let ticket = sequence | if player { 0 } else { ROLE_CPU };
    CLIP_REPLY.store((u64::from(kind) << 32) | ticket | BUSY | ASKED, Ordering::Release);
    REQUESTS.push([ticket, u64::from(kind)]);
}

/// Worker. Ask to take the current ticket's request back. The game frame answers: withdrawn, or committed.
pub(crate) fn withdraw_clip() {
    let _ = CLIP_REPLY.fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| {
        matches!(word & STATE, ASKED | ADMITTED).then_some(word | WITHDRAW)
    });
}

/// Worker. The current ticket's state; `None` when no switch is in flight. Reading does not consume it.
pub(crate) fn clip_outcome() -> Option<ClipOutcome> {
    Some(match CLIP_REPLY.load(Ordering::Acquire) & STATE {
        ASKED => ClipOutcome::Asked,
        ADMITTED => ClipOutcome::Admitted,
        REFUSED => ClipOutcome::Refused,
        DROPPED => ClipOutcome::Dropped,
        WITHDRAWN => ClipOutcome::Withdrawn,
        COMMITTED => ClipOutcome::Committed,
        _ => return None,
    })
}

/// Worker. A switch is in flight and the roster was working on the last game frame (or has not heard of it yet).
pub(crate) fn clip_busy() -> bool {
    let word = CLIP_REPLY.load(Ordering::Acquire);
    matches!(word & STATE, ASKED | ADMITTED | COMMITTED) && word & BUSY != 0
}

/// Worker. Forget the ticket: its last state was shown, or the load ended on the fighter the game has.
pub(crate) fn clear_clip() {
    let _ = CLIP_REPLY.fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| Some(word & !(STATE | WITHDRAW | BUSY)));
}

/// The game side, wherever the roster gives up a user's direct request after admission: the role's ticket is marked
/// dropped. `kind` is the request's own fighter; `None` when the role's whole load state goes (a scene change, the
/// entry's destruction, a Mii owner's track). Lock-free: the callers hold the roster's locks. Never inlined: the
/// callers sit on the deepest native service path, whose frames must not grow (`model::thread_stacks`).
#[inline(never)]
pub(crate) fn note_drop(player: bool, kind: Option<u32>) {
    let _ = CLIP_REPLY.fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| {
        (matches!(word & STATE, ADMITTED | COMMITTED) && (word & ROLE_CPU == 0) == player
            && kind.is_none_or(|kind| (word >> 32) as u32 == kind)).then_some((word & !STATE) | DROPPED)
    });
}

/// Move `ticket` from one of the states `from` to `to`. Another ticket, or another state, is left alone.
fn step(ticket: u64, from: &[u64], to: u64) -> bool {
    CLIP_REPLY.fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| {
        (word & TICKET == ticket && from.contains(&(word & STATE))).then_some((word & !STATE) | to)
    }).is_ok()
}

/// Player 1's fighter kind as the roster last observed it, as of the last game frame (the worker's read; never a lock).
pub(crate) fn player_kind() -> Option<u32> {
    Some(PLAYER_KIND.load(Ordering::Acquire)).filter(|kind| *kind != NONE)
}

/// The game frame (`lobby::update`): the worker's requests, in the order it posted them, Player 1's fighter for its next read
/// and the clip switch's ticket.
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
            ticket => {
                // Only the current ticket, and only while it is still asked. A withdraw that came first: never sent.
                let withdrawn = CLIP_REPLY.load(Ordering::Acquire) & WITHDRAW != 0 && step(ticket, &[ASKED], WITHDRAWN);
                // Admitted before the roster hears it, so a drop the roster marks at once is kept.
                if !withdrawn && step(ticket, &[ASKED], ADMITTED) {
                    let queued = if ticket & ROLE_CPU == 0 { super::character_roster::request_player_kind(kind) }
                        else { super::character_roster::request_cpu_kind(kind) };
                    if !queued { step(ticket, &[ADMITTED], REFUSED); }
                }
                replied = true;
            }
        }
    }
    let word = CLIP_REPLY.load(Ordering::Acquire);
    if matches!(word & STATE, ADMITTED | COMMITTED) {
        let (ticket, player) = (word & TICKET, word & ROLE_CPU == 0);
        if word & STATE == ADMITTED && word & WITHDRAW != 0 {
            // B: a request that still waits in the roster is removed; one the roster has begun runs to its end.
            let took = super::character_roster::withdraw_direct(player, (word >> 32) as u32);
            replied |= step(ticket, &[ADMITTED], if took { WITHDRAWN } else { COMMITTED });
        }
        // The roster's own state for the worker's read; the worker never takes the roster's locks.
        let busy = super::character_roster::notice_blocked();
        let _ = CLIP_REPLY.fetch_update(Ordering::AcqRel, Ordering::Acquire, |now| {
            (now & TICKET == ticket && matches!(now & STATE, ADMITTED | COMMITTED))
                .then_some(if busy { now | BUSY } else { now & !BUSY })
        });
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
