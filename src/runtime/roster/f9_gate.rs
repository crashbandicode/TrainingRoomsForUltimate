//! F9's hold between a CPU-count change and the Reload that may follow it.
//!
//! The model (`parked_entries`) decides. This module remembers that decision
//! until the activation frame has passed: a raise seen while the pause menu is
//! open waits for the close, and the close's own frame still admits no new
//! load on entries 2 and 3. The Reload is armed on a later frame.

use crate::engine::mii::lifetime::Configuration;
use crate::model::parked_entries::{After, Decision};
use crate::runtime::roster::cycle_preload::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    /// The count changed while the pause menu was open. The close is still ahead.
    Paused,
    /// The activation frame. Publishes may already have been applied. No Reload yet.
    Activated,
}

struct Gate {
    previous: Option<u32>,
    phase: Phase,
    seen: u32,
    decision: Decision,
    /// The CPU rotation's next member and the one after, with an exact Mii's configuration.
    members: [Option<Target>; 2],
    /// A Reload is armed with the configuration it loads, taken from the member when it is armed.
    armed2: Option<(After, Option<Configuration>)>,
    armed3: Option<(After, Option<Configuration>)>,
    /// Entries 2/3 the game activated with a published seat: their Mii proof is taken again once parked.
    reproof: [bool; 2],
}

impl Gate {
    const fn empty() -> Self {
        Self {
            previous: None,
            phase: Phase::Idle,
            seen: 0,
            decision: Decision {
                publish: [None, None],
                release_now: None,
                hold_installs: false,
                after_entry2: None,
                after_entry3: None,
            },
            members: [None, None],
            armed2: None,
            armed3: None,
            reproof: [false, false],
        }
    }
}

static GATE: hos_lock::Mutex<Gate> = hos_lock::Mutex::new(Gate::empty());

/// F9-SEAT-NOTICE (primary 2026-11-08 23:03Z): per seat (entry 2, entry 3), the kind plus one whose last speculative
/// load on that entry process memory refused; 0 when none is refused now. Written by the loader callback, read by the
/// readiness notice (`sources::seat_states`).
static SEAT_HELD: [core::sync::atomic::AtomicU32; 2] =
    [core::sync::atomic::AtomicU32::new(0), core::sync::atomic::AtomicU32::new(0)];

/// A speculative helper load of `kind` on entry `slot` was refused for memory (`held`), or admitted. Other entries are
/// not seats and are ignored.
pub(crate) fn note_seat_memory(slot: u32, kind: u32, held: bool) {
    if let Some(cell) = SEAT_HELD.get((slot as usize).wrapping_sub(2)) {
        let now = if held { kind.saturating_add(1) } else { 0 };
        let was = cell.swap(now, core::sync::atomic::Ordering::AcqRel);
        if was != now {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[F9Seat] memory slot={slot} held={} kind=0x{:x}",
                held,
                if held { kind } else { was.saturating_sub(1) }
            ));
        }
    }
}

/// The kind a seat's last load was refused for memory, if that refusal still stands.
pub(crate) fn seat_held(slot: u32) -> Option<u32> {
    let held = SEAT_HELD.get((slot as usize).wrapping_sub(2))?.load(core::sync::atomic::Ordering::Acquire);
    held.checked_sub(1)
}

/// What the player frame should do with a stored raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Wait,
    /// Apply the HUD publication and a Promoted release. Do not start a Reload.
    Activate,
    /// The walk has had a later frame. These Reloads may start.
    Reload,
}

pub(crate) struct Seen {
    pub previous: Option<u32>,
    pub changed: bool,
    pub fixed: [Option<u32>; 2],
}

impl Gate {
    fn fixed(&self) -> [Option<u32>; 2] {
        self.members.map(|member| member.map(|member| member.kind))
    }

    fn member(&self, slot: u32) -> Option<(u32, Option<Configuration>)> {
        let member = match slot {
            2 => self.members[0],
            3 => self.members[1],
            _ => None,
        };
        member.map(|member| (member.kind, member.configuration))
    }

    /// The Reload as armed: its kind's member configuration (`seat_load`). None when the seat cannot be loaded.
    fn arm(&self, after: Option<After>) -> Option<(After, Option<Configuration>)> {
        let after = after?;
        let (_, configuration) =
            crate::model::parked_entries::seat_load(after.kind, self.member(after.slot))?;
        Some((after, configuration))
    }
}

pub(crate) fn set_fixed_members(first: Option<Target>, second: Option<Target>) {
    let mut gate = GATE.lock();
    gate.members = [first, second];
}

/// The seat member of entry `slot` (2 or 3): the kind and the exact-Mii configuration.
pub(crate) fn seat_member(slot: u32) -> Option<(u32, Option<Configuration>)> {
    GATE.lock().member(slot)
}

/// The armed Reload of `slot`: the kind and the exact-Mii configuration it loads.
pub(crate) fn override_seat(slot: u32) -> Option<(u32, Option<Configuration>)> {
    let gate = GATE.lock();
    [gate.armed2, gate.armed3]
        .into_iter()
        .flatten()
        .find(|(after, _)| after.slot == slot)
        .map(|(after, configuration)| (after.kind, configuration))
}

/// The game activated a published seat on `slot` (2 or 3).
pub(crate) fn mark_promoted(slot: u32) {
    if let Some(flag) = GATE.lock().reproof.get_mut((slot as usize).wrapping_sub(2)) {
        *flag = true;
    }
}

pub(crate) fn reproof_pending(slot: u32) -> bool {
    GATE.lock().reproof.get((slot as usize).wrapping_sub(2)).copied().unwrap_or(false)
}

pub(crate) fn clear_reproof(slot: u32) {
    if let Some(flag) = GATE.lock().reproof.get_mut((slot as usize).wrapping_sub(2)) {
        *flag = false;
    }
}

pub(crate) fn handover_pending() -> bool {
    let gate = GATE.lock();
    gate.phase != Phase::Idle || gate.armed2.is_some() || gate.armed3.is_some()
}

/// Entries 2 and 3 take no new load from the observation through the activation frame.
pub(crate) fn block_new_load(slot: u32) -> bool {
    matches!(slot, 2 | 3) && GATE.lock().phase != Phase::Idle
}

pub(crate) fn observe_quantity(quantity: Option<u32>) -> Seen {
    let mut gate = GATE.lock();
    let previous = gate.previous;
    let changed = previous != quantity;
    if changed {
        gate.previous = quantity;
    }
    Seen {
        previous,
        changed,
        fixed: gate.fixed(),
    }
}

/// Remember a raise. When `pause_open` is false the caller applies Activate itself.
pub(crate) fn begin_raise(decision: Decision, frame: u32, pause_open: bool) {
    if !decision.hold_installs {
        return;
    }
    let mut gate = GATE.lock();
    gate.seen = frame;
    // One raise seen in steps before the same close (1→2, then 2→3): keep what the earlier step decided.
    gate.decision = if gate.phase == Phase::Paused {
        crate::model::parked_entries::merge_paused(gate.decision, decision)
    } else {
        decision
    };
    gate.phase = if pause_open {
        Phase::Paused
    } else {
        Phase::Activated
    };
    gate.armed2 = None;
    gate.armed3 = None;
}

pub(crate) fn decision() -> Decision {
    GATE.lock().decision
}

/// Advance the stored raise. `Activate` is returned once, on the close; `Reload` once, on a later frame.
pub(crate) fn poll(frame: u32, pause_open: bool) -> Step {
    let mut gate = GATE.lock();
    if gate.phase == Phase::Idle || pause_open {
        return Step::Wait;
    }
    if gate.phase == Phase::Paused {
        gate.phase = Phase::Activated;
        gate.seen = frame;
        return Step::Activate;
    }
    if frame <= gate.seen {
        return Step::Wait;
    }
    gate.phase = Phase::Idle;
    gate.armed2 = gate.arm(gate.decision.after_entry2);
    gate.armed3 = gate.arm(gate.decision.after_entry3);
    Step::Reload
}

pub(crate) fn armed() -> [Option<After>; 2] {
    let gate = GATE.lock();
    [gate.armed2, gate.armed3].map(|armed| armed.map(|(after, _)| after))
}

pub(crate) fn clear_armed_if(slot: u32, kind: u32) {
    let mut gate = GATE.lock();
    if gate
        .armed2
        .is_some_and(|(after, _)| after.slot == slot && after.kind == kind)
    {
        gate.armed2 = None;
    }
    if gate
        .armed3
        .is_some_and(|(after, _)| after.slot == slot && after.kind == kind)
    {
        gate.armed3 = None;
    }
}

pub(crate) fn reset() {
    let mut gate = GATE.lock();
    let members = gate.members;
    *gate = Gate::empty();
    gate.members = members;
    drop(gate);
    for slot in [2, 3] {
        note_seat_memory(slot, 0, false);
    }
}
