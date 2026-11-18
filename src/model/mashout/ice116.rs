//! Mash out 116 (Sora Ice magic, Blizzard) as a consumer of the M3 Set observer (M3-EVENT-INTERFACE v1.2:
//! `Slot::Set`, FighterControl +0x90 → main+0x3f81a0, tag 116, qualifier LR common+0x111f80). Pure; the runtime
//! wiring waits for the observer's gate and review, so nothing grades on observed writes yet.
//!
//! G2, full-range method (OPUS-DESIGN-REVIEW-sora116-refreeze.md, 23:00Z): each Set stores
//! `fmax_arm(old + F)` with the shard's ice frame F unpublished, so every write is bracketed by `[F_lo, F_hi]` and the
//! bounds are solved twice (all F_lo, all F_hi) in the shared solver; they must agree, and no replayed sum may reach
//! the native cap `d_max`.

use super::clatter::{fmax_zero, g2};
use super::grade::{grade, Bounds, Outcome};
use super::native::{Error, Scheduled, Seed, Solver, SCHEDULE};

pub const TAG: u16 = 116;

/// One published Set (f32 bits) and the tick boundary G1 applied it at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applied {
    pub boundary: u32,
    pub old: u32,
    pub operand: u32,
    pub new: u32,
    pub d_max: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unscored {
    /// S1: the Blizzard provider was still open at the escape or Result.
    ProviderOpen,
    /// AuxInvalid: d_max not finite or <= 0, changing within the attempt, or below an operand.
    Aux,
    /// An operand at d_max (the native `le` cap), or a replayed sum reaching it.
    Cap,
    /// No finite F > 0 explains a write (`fadd_rne(old, F) != operand` for all F).
    Range,
    /// More writes than the ring, unordered boundaries, or a `new` the Set callee cannot have stored.
    Writes,
    /// The all-F_lo and all-F_hi bounds differ, so the grade depends on the unpublished ice frames.
    Sensitive,
    /// The replayed bounds have no answer (a write ended a cadence, an invalid seed, no end).
    Bounds(Error),
}

fn solve(mut solver: Solver) -> Result<Bounds, Error> {
    loop {
        if let Some(result) = solver.advance(64) {
            return result;
        }
    }
}

fn unscored(error: Error) -> Unscored {
    match error {
        Error::ScheduleCap => Unscored::Cap,
        Error::InvalidSchedule => Unscored::Writes,
        other => Unscored::Bounds(other),
    }
}

/// The attempt's bounds. An attempt with no observed Set is the plain solver (C2); otherwise the full-range G2.
pub fn bounds(seed: Seed, writes: &[Applied]) -> Result<Bounds, Unscored> {
    let Some(first) = writes.first() else {
        return Solver::new(seed).and_then(solve).map_err(unscored);
    };
    let d_max = f32::from_bits(first.d_max);
    if writes.len() > SCHEDULE {
        return Err(Unscored::Writes);
    }
    let mut lo = [Scheduled { boundary: 0, operand: 0.0 }; SCHEDULE];
    let mut hi = lo;
    for (k, w) in writes.iter().enumerate() {
        let (old, operand) = (f32::from_bits(w.old), f32::from_bits(w.operand));
        if !d_max.is_finite() || d_max <= 0.0 || w.d_max != first.d_max || operand > d_max {
            return Err(Unscored::Aux);
        }
        if w.operand == w.d_max {
            return Err(Unscored::Cap);
        }
        if !old.is_finite() || !operand.is_finite() || w.new != fmax_zero(operand).to_bits() {
            return Err(Unscored::Writes);
        }
        let (f_lo, f_hi) = g2::admissible(old, operand).ok_or(Unscored::Range)?;
        lo[k] = Scheduled { boundary: w.boundary, operand: f_lo };
        hi[k] = Scheduled { boundary: w.boundary, operand: f_hi };
    }
    let n = writes.len();
    let low = Solver::scheduled_capped(seed, &lo[..n], d_max).and_then(solve).map_err(unscored)?;
    let high = Solver::scheduled_capped(seed, &hi[..n], d_max).and_then(solve).map_err(unscored)?;
    if low != high {
        return Err(Unscored::Sensitive);
    }
    Ok(low)
}

/// The user's grade from the G2 bounds and the actual (G1) elapsed ticks, only with the provider closed (S1).
pub fn judge(bounds: Result<Bounds, Unscored>, elapsed: u32, provider_closed: bool) -> Result<Outcome, Unscored> {
    let bounds = bounds?;
    if !provider_closed {
        return Err(Unscored::ProviderOpen);
    }
    Ok(grade(elapsed, bounds))
}

#[cfg(test)]
#[path = "ice116_tests.rs"]
mod tests;
