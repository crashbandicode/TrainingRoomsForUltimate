//! Consumer side of the M3 clatter-write observer (event interface v1.2 = v1.1 + `d_max` + `AuxInvalid`,
//! `frozen-cluster-devin/M3-EVENT-INTERFACE.md`). The observer (owner: the frozen lane) publishes one
//! `ClatterWrite` per qualified native write; consumers apply a write only when it is bit-exact against the
//! model (G1) and never infer one from a mismatch.

use super::native::{Clock, ClockError, Input, Record};

/// v1.1: the FighterControl vtable offset of the interposed slot, so the `repr(C)` record is stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Slot {
    Set = 0x90,
    Add = 0x98,
}

/// ARM `FMAX(x, +0.0)` as the Control callees compute it: any NaN stays NaN, and -0.0 or a negative gives +0.0.
pub fn fmax_zero(x: f32) -> f32 {
    if x.is_nan() || x > 0.0 {
        x
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ClatterWrite {
    pub generation: u32,
    pub seq: u32,
    pub slot: Slot,
    pub tag: u16,
    pub old: u32,
    pub operand: u32,
    pub new: u32,
    pub frame: u32,
    /// v1.2, Set only: f32 bits of the native ice-frame cap (`[x29-0xc0]` at Set entry, qualified and tag-checked).
    pub d_max: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poison {
    Overflow,
    NewMismatch,
    Nested,
    WrongSlot,
    Stale,
    Lost,
    /// v1.2: the `d_max` read was not an f32 number, not finite or <= 0, changed within the attempt, or below operand.
    AuxInvalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll {
    Event(ClatterWrite),
    Empty,
    Poisoned(Poison),
}

/// Scenario tags and their slot (C4: exactly one slot per tag).
pub const TAG_FROZEN_113: u16 = 113;
pub const TAG_ICE_MAGIC_116: u16 = 116;
pub const TAG_EGG_117: u16 = 117;

pub const fn slot_for(tag: u16) -> Option<Slot> {
    match tag {
        TAG_FROZEN_113 | TAG_ICE_MAGIC_116 => Some(Slot::Set),
        TAG_EGG_117 => Some(Slot::Add),
        _ => None,
    }
}

/// Per-attempt consumer state: the armed generation/tag and the next expected seq (gap-free from 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Writes {
    generation: u32,
    tag: u16,
    slot: Slot,
    next: u32,
    d_max: Option<u32>,
}

impl Writes {
    pub const fn new(generation: u32, tag: u16) -> Option<Self> {
        match slot_for(tag) {
            Some(slot) => Some(Self { generation, tag, slot, next: 1, d_max: None }),
            None => None,
        }
    }

    /// The seq the next accepted event must carry (gap-free from 1); the runtime poller asks for it.
    pub const fn next_seq(&self) -> u32 {
        self.next
    }

    /// Validate one polled item for this attempt. A poison, another tag/slot, a seq gap or a value the callee
    /// cannot have stored is an error; an event of another generation is stale and only refused (v1.1).
    pub fn accept(&mut self, poll: Poll) -> Result<Option<ClatterWrite>, ClockError> {
        let write = match poll {
            Poll::Empty => return Ok(None),
            Poll::Poisoned(_) => return Err(ClockError::WriteRefused),
            Poll::Event(write) => write,
        };
        if write.generation != self.generation {
            return Ok(None);
        }
        if write.tag != self.tag || write.slot != self.slot || write.seq != self.next {
            return Err(ClockError::WriteRefused);
        }
        let expected = fmax_zero(match write.slot {
            Slot::Set => f32::from_bits(write.operand),
            Slot::Add => f32::from_bits(write.old) + f32::from_bits(write.operand),
        });
        if write.new != expected.to_bits() || !expected.is_finite() {
            return Err(ClockError::WriteRefused);
        }
        if write.slot == Slot::Set {
            // AuxInvalid (v1.2), checked by the consumer too: finite positive, constant per attempt, >= operand.
            let d_max = f32::from_bits(write.d_max);
            if !d_max.is_finite() || d_max <= 0.0 || self.d_max.is_some_and(|d| d != write.d_max)
                || f32::from_bits(write.operand) > d_max
            {
                return Err(ClockError::WriteRefused);
            }
            self.d_max = Some(write.d_max);
        }
        self.next += 1;
        Ok(Some(write))
    }
}

impl Clock {
    /// G1: one frame's observation with the native writes published since the previous one (seq order, already
    /// validated by `Writes::accept`). A write applies only when its `old` is bit-equal to the model's remaining,
    /// either before this frame's tick or after it; the tick itself is still the strict-decrease rule of
    /// `observe`. Anything else is a mismatch and is never turned into an inferred write.
    pub fn observe_writes(&mut self, live: Record, input: Input, escaped: bool, writes: &[ClatterWrite]) -> Result<(), ClockError> {
        if writes.is_empty() {
            return self.observe(live, input, escaped);
        }
        if self.exhausted() {
            return Ok(());
        }
        if !live.valid() {
            return Err(ClockError::InvalidRecord);
        }
        let mut model = self.last();
        let mut ticked = false;
        for write in writes {
            if model.remaining.to_bits() != write.old && !ticked {
                model.step(input);
                ticked = true;
            }
            if model.remaining.to_bits() != write.old {
                return Err(ClockError::WriteRefused);
            }
            model.remaining = f32::from_bits(write.new);
        }
        if !ticked && live != model {
            model.step(input);
            ticked = true;
        }
        if live != model {
            return Err(ClockError::TransitionMismatch);
        }
        self.rebase(live, ticked, escaped)
    }
}

/// G2 (Set, 113/116; OPUS-DESIGN-REVIEW-sora116-refreeze 23:00Z full-range method): the per-shard ice frame F is
/// not published, only bracketed (the bounds replay is `ice116::bounds` on the shared solver).
pub mod g2 {

    /// Every finite F > 0 with `fadd_rne(old, F) == operand`, as `[F_lo, F_hi]` by ulp stepping; empty -> None.
    pub fn admissible(old: f32, operand: f32) -> Option<(f32, f32)> {
        let hits = |f: f32| f.is_finite() && f > 0.0 && (old + f).to_bits() == operand.to_bits();
        let up = |f: f32| f32::from_bits(f.to_bits() + 1);
        let down = |f: f32| f32::from_bits(f.to_bits() - 1);
        let guess = operand - old;
        if !guess.is_finite() || guess <= 0.0 {
            return None;
        }
        let mut seed = None;
        let (mut lo, mut hi) = (guess, guess);
        for _ in 0..64 {
            if hits(lo) {
                seed = Some(lo);
                break;
            }
            if hits(hi) {
                seed = Some(hi);
                break;
            }
            lo = down(lo);
            hi = up(hi);
            if lo <= 0.0 {
                lo = hi;
            }
        }
        let seed = seed?;
        let (mut lo, mut hi) = (seed, seed);
        for _ in 0..(1 << 16) {
            let (next_lo, next_hi) = (down(lo), up(hi));
            let (more_lo, more_hi) = (next_lo > 0.0 && hits(next_lo), hits(next_hi));
            if !more_lo && !more_hi {
                return Some((lo, hi));
            }
            if more_lo {
                lo = next_lo;
            }
            if more_hi {
                hi = next_hi;
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "clatter_tests.rs"]
mod tests;
