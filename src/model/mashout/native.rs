//! Native 13.0.5 clatter transition, exact escape bounds, and the approved per-frame clock.
//! Evidence: rc15-mashout/NATIVE.md and main+0x3f89b0. The solver runs outside game hooks.

use super::grade::Bounds;

pub const BUTTONS: u8 = 0x2f;
const STATES: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub remaining: f32,
    pub automatic: f32,
    pub manual: f32,
    pub button_mul: f32,
    /// Actual native threshold getter parameter, captured alongside the inline fields;
    /// this is not a field at an offset inside the native clatter record.
    pub threshold: f32,
    pub sign: [i8; 2],
    pub source: i8,
    pub stick_block: i8,
    pub button_block: i8,
    pub accepted: u8,
    pub other_block: i8,
    pub cross_reload: i8,
    pub other_reload: i8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub axes: [f32; 2],
    /// Native Control input trigger, not physical poll buttons.
    pub trigger: u32,
}

pub fn mapped(buttons: u32) -> u8 {
    ((buttons & 0x2e) | ((buttons >> 9) & 1)) as u8
}

fn dec(value: i8) -> i8 {
    if value > 0 {
        value - 1
    } else {
        value
    }
}

fn reduce(remaining: f32, manual: f32, automatic: f32) -> f32 {
    // Separate expressions preserve the native multiply/subtract/subtract rounding.
    let remaining = remaining - manual;
    let remaining = remaining - automatic;
    if remaining <= 0.0 {
        0.0
    } else {
        remaining
    }
}

impl Record {
    pub fn valid(self) -> bool {
        [self.remaining, self.automatic, self.manual, self.button_mul, self.threshold]
            .into_iter()
            .all(|n| n.is_finite() && n >= 0.0)
            && (self.manual * self.button_mul).is_finite()
            && self.sign.into_iter().all(|n| (-1..=1).contains(&n))
            && self.accepted & !BUTTONS == 0
            && self.stick_block <= 4
            && self.button_block <= 4
            && self.other_block <= 3
            && self.cross_reload == 4
            && self.other_reload == 3
    }

    pub fn step(&mut self, input: Input) {
        if self.remaining <= 0.0 {
            return;
        }
        self.stick_block = dec(self.stick_block);
        self.button_block = dec(self.button_block);
        self.other_block = dec(self.other_block);
        let sign = [0, 1].map(|i| {
            if input.axes[i] > self.threshold {
                1
            } else if input.axes[i] < -self.threshold {
                -1
            } else {
                self.sign[i]
            }
        });
        if sign != self.sign && self.stick_block <= 0 {
            self.sign = sign;
            self.button_block = self.cross_reload;
            self.remaining -= self.manual;
        }
        let mut trigger = mapped(input.trigger);
        if self.other_block > 0 {
            trigger &= self.accepted;
        }
        if trigger != 0 && self.button_block <= 0 {
            self.stick_block = self.cross_reload;
            self.other_block = self.other_reload;
            self.accepted = trigger;
            let amount = self.manual * self.button_mul;
            self.remaining -= amount;
        }
        self.remaining -= self.automatic;
        if self.remaining <= 0.0 {
            self.remaining = 0.0;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub record: Record,
    /// Counted categories held before the first controllable input edge.
    pub held: u8,
    /// Only a verified native deadline, in eligible ticks from this seed.
    pub deadline: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSeed,
    NeverEscapes,
    TickRange,
}

#[derive(Clone, Copy)]
struct Controls {
    stick: usize,
    button: usize,
    other: usize,
    held: bool,
    mask: bool,
}

impl Controls {
    fn from_record(record: Record) -> Self {
        Self {
            stick: record.stick_block.max(0) as usize,
            button: record.button_block.max(0) as usize,
            other: record.other_block.max(0) as usize,
            held: false,
            mask: record.accepted != 0,
        }
    }

    fn index(self) -> usize {
        (((self.stick * 5 + self.button) * 4 + self.other) * 2 + usize::from(self.held)) * 2
            + usize::from(self.mask)
    }

    fn at(mut index: usize) -> Self {
        let mask = index & 1 != 0;
        index /= 2;
        let held = index & 1 != 0;
        index /= 2;
        let other = index % 4;
        index /= 4;
        Self {
            stick: index / 5,
            button: index % 5,
            other,
            held,
            mask,
        }
    }
}

/// Incremental exact search. Work may pause/resume; no attempt deadline is invented.
/// Two control-equivalent states keep the lower remaining value, by monotonicity of f32 subtraction.
pub struct Solver {
    seed: Seed,
    frontier: [f32; STATES],
    next: [f32; STATES],
    ticks: u32,
    fastest: Option<u32>,
    baseline: f32,
    no_input: Option<Option<u32>>,
    result: Option<Result<Bounds, Error>>,
}

impl Solver {
    pub fn new(seed: Seed) -> Result<Self, Error> {
        if !seed.record.valid()
            || seed.record.remaining <= 0.0
            || seed.held & !BUTTONS != 0
            || seed.deadline == Some(0)
        {
            return Err(Error::InvalidSeed);
        }
        let r = seed.record;
        // Effective axes are normalized to [-1, 1]; strict comparison makes a stick
        // sign change unreachable at threshold 1 or above, including from neutral.
        let stick = if r.threshold < 1.0 { r.manual } else { 0.0 };
        let maximum = stick.max(r.manual * r.button_mul);
        let result = if reduce(r.remaining, maximum, r.automatic) == r.remaining {
            Some(
                seed.deadline
                    .map(|end| Bounds {
                        fastest: end,
                        no_input: Some(end),
                    })
                    .ok_or(Error::NeverEscapes),
            )
        } else {
            None
        };
        let no_input =
            (reduce(r.remaining, 0.0, r.automatic) == r.remaining).then_some(seed.deadline);
        Ok(Self {
            seed,
            frontier: [f32::INFINITY; STATES],
            next: [f32::INFINITY; STATES],
            ticks: 0,
            fastest: None,
            baseline: r.remaining,
            no_input,
            result,
        })
    }

    fn put(&mut self, controls: Controls, remaining: f32) {
        if remaining <= 0.0 {
            self.fastest = Some(self.ticks);
        } else {
            let slot = &mut self.next[controls.index()];
            *slot = slot.min(remaining);
        }
    }

    fn expand(&mut self, mut controls: Controls, remaining: f32, first: bool) {
        controls.stick = controls.stick.saturating_sub(1);
        controls.button = controls.button.saturating_sub(1);
        controls.other = controls.other.saturating_sub(1);
        let released = Controls {
            held: false,
            ..controls
        };
        let r = self.seed.record;
        self.put(released, reduce(remaining, 0.0, r.automatic));
        if controls.stick == 0 && r.threshold < 1.0 {
            self.put(
                Controls {
                    button: 4,
                    ..released
                },
                reduce(remaining, r.manual, r.automatic),
            );
        }
        let button = if first {
            let fresh = BUTTONS & !self.seed.held;
            fresh
                & if controls.other == 0 {
                    BUTTONS
                } else {
                    r.accepted
                }
                != 0
        } else {
            controls.other == 0 || controls.mask && !controls.held
        };
        if controls.button == 0 && button {
            self.put(
                Controls {
                    stick: 4,
                    other: 3,
                    held: true,
                    mask: true,
                    ..controls
                },
                reduce(remaining, r.manual * r.button_mul, r.automatic),
            );
        }
    }

    pub fn advance(&mut self, work: u16) -> Option<Result<Bounds, Error>> {
        for _ in 0..work {
            if self.result.is_some() {
                break;
            }
            let Some(ticks) = self.ticks.checked_add(1) else {
                self.result = Some(Err(Error::TickRange));
                break;
            };
            self.ticks = ticks;
            if self.fastest.is_none() {
                self.next.fill(f32::INFINITY);
                if ticks == 1 {
                    self.expand(
                        Controls::from_record(self.seed.record),
                        self.seed.record.remaining,
                        true,
                    );
                } else {
                    for i in 0..STATES {
                        let remaining = self.frontier[i];
                        if remaining.is_finite() {
                            self.expand(Controls::at(i), remaining, false);
                        }
                    }
                }
                std::mem::swap(&mut self.frontier, &mut self.next);
            }
            if self.no_input.is_none() {
                self.baseline = reduce(self.baseline, 0.0, self.seed.record.automatic);
                if self.baseline <= 0.0 {
                    self.no_input = Some(Some(ticks));
                }
            }
            if self.seed.deadline.is_some_and(|end| ticks >= end) {
                self.fastest.get_or_insert(ticks);
                self.no_input.get_or_insert(Some(ticks));
            }
            if let (Some(fastest), Some(no_input)) = (self.fastest, self.no_input) {
                self.result = Some(Ok(Bounds { fastest, no_input }));
            }
        }
        self.result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    InvalidRecord,
    Reinitialized,
    TransitionMismatch,
    Ambiguous,
    TickRange,
}

#[derive(Clone, Copy, Debug)]
pub struct Clock {
    last: Record,
    elapsed: u32,
    exhausted: bool,
}

impl Clock {
    pub fn new(seed: Record) -> Result<Self, ClockError> {
        if !seed.valid() || seed.remaining <= 0.0 {
            return Err(ClockError::InvalidRecord);
        }
        Ok(Self {
            last: seed,
            elapsed: 0,
            exhausted: false,
        })
    }

    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }
    pub fn exhausted(&self) -> bool {
        self.exhausted
    }
    pub fn last(&self) -> Record {
        self.last
    }

    /// Approved M3 reconstruction: a strict decrease counts one tick. Validate it against
    /// the actual native inputs; missed/multiple updates are evidence, never a guessed count.
    pub fn observe(&mut self, live: Record, input: Input, escaped: bool) -> Result<(), ClockError> {
        if self.exhausted {
            return Ok(());
        }
        if !live.valid() {
            return Err(ClockError::InvalidRecord);
        }
        if live.remaining > self.last.remaining {
            return Err(ClockError::Reinitialized);
        }
        if live.remaining < self.last.remaining {
            self.elapsed = self.elapsed.checked_add(1).ok_or(ClockError::TickRange)?;
            let mut expected = self.last;
            expected.step(input);
            if expected != live {
                return Err(ClockError::TransitionMismatch);
            }
            self.exhausted = live.remaining <= 0.0;
        } else if live != self.last {
            return Err(ClockError::Ambiguous);
        }
        self.last = live;
        self.exhausted |= escaped;
        Ok(())
    }

    /// M3 G1: adopt an observed native write between ticks only when its `old` is bit-equal to the model's
    /// remaining; continue from `new`. Anything else is the existing mismatch (no adjustment is inferred).
    pub fn rebase(&mut self, old: u32, new: u32) -> Result<(), ClockError> {
        if self.exhausted || self.last.remaining.to_bits() != old {
            return Err(ClockError::TransitionMismatch);
        }
        let new = f32::from_bits(new);
        if !new.is_finite() || new <= 0.0 {
            return Err(ClockError::InvalidRecord);
        }
        self.last.remaining = new;
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
