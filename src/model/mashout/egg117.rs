//! Mash out 117 (Egg Lay) as a consumer of the M3 Add observer (M3-EVENT-INTERFACE v1.2: `Slot::Add`, FighterControl
//! +0x98 → main+0x3f81c0, tag 117, qualifier LR common+0xc5b38, x0 the armed P1 Control, w1 0). Pure; the runtime
//! wiring waits for the observer's gate and review, so nothing grades on observed writes yet.
//!
//! G1 (the actual clock) applies a write only at the tick boundary where its `old` is bit-equal to the model; this
//! module takes the boundary G1 applied each write at. G2 replays every observed Add with its own operand, the native
//! delta, in the fastest and no-input cadences while they still hold (OPUS-DESIGN-REVIEW-sora116-refreeze.md, 23:00Z).
//! Set's `d_max` does not apply to Add.

use super::grade::{grade, Bounds, Outcome};
use super::native::{add, Error, Scheduled, Seed, Solver, SCHEDULE};

pub const TAG: u16 = 117;

/// One published Add (f32 bits) and the tick boundary G1 applied it at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub boundary: u32,
    pub old: u32,
    pub operand: u32,
    pub new: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unscored {
    /// 117's provider closure (the Egg's damage providers, any Add cap path) is not proven for this attempt.
    ProviderOpen,
    /// S3: more writes than the ring, unordered boundaries, or a `new` the Add callee cannot have stored.
    Writes,
    /// The replayed bounds have no answer (a write ended a cadence, an invalid seed, no end).
    Bounds(Error),
}

/// The G2 solver for one attempt, after S3 checks every write against the Add callee bit-exactly.
pub fn replay(seed: Seed, writes: &[Applied]) -> Result<Solver, Unscored> {
    let mut schedule = [Scheduled { boundary: 0, operand: 0.0 }; SCHEDULE];
    if writes.len() > SCHEDULE {
        return Err(Unscored::Writes);
    }
    for (slot, w) in schedule.iter_mut().zip(writes) {
        let (old, operand) = (f32::from_bits(w.old), f32::from_bits(w.operand));
        let new = add(old, operand);
        if !old.is_finite() || !new.is_finite() || new.to_bits() != w.new {
            return Err(Unscored::Writes);
        }
        *slot = Scheduled { boundary: w.boundary, operand };
    }
    Solver::scheduled(seed, &schedule[..writes.len()]).map_err(|e| match e {
        Error::InvalidSchedule => Unscored::Writes,
        other => Unscored::Bounds(other),
    })
}

/// The user's grade from the replayed bounds and the actual (G1) elapsed ticks, only with the provider closed.
pub fn judge(bounds: Result<Bounds, Error>, elapsed: u32, provider_closed: bool) -> Result<Outcome, Unscored> {
    let bounds = bounds.map_err(Unscored::Bounds)?;
    if !provider_closed {
        return Err(Unscored::ProviderOpen);
    }
    Ok(grade(elapsed, bounds))
}

#[cfg(test)]
#[path = "egg117_tests.rs"]
mod tests;
