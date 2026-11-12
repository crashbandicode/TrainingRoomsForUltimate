//! What a delay row means to its consumers (SPEC-core-02 §2.8): `Empty`, `Set(weighted values)` or
//! `Range(min, max, step)`. A range is picked uniformly among the step values from min to max
//! inclusive; min = max is a fixed delay.

/// A set range of a delay row, in the row's own units (frames, or hits for OoS Offset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DelayRange {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

impl DelayRange {
    /// How many values the range holds (`min..=max` by `step`).
    #[cfg(test)]
    pub const fn count(&self) -> u32 {
        (self.max - self.min) as u32 / self.step as u32 + 1
    }

    /// The `i`-th value, `i < count()`.
    #[cfg(test)]
    pub const fn value_at(&self, i: u32) -> u16 {
        self.min + (i as u16) * self.step
    }

    /// A uniform pick, given a uniform random `u32` from the caller's own generator.
    #[cfg(test)]
    pub const fn pick(&self, random: u32) -> u16 {
        self.value_at(random % self.count())
    }
}

/// A delay row as a consumer reads it.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelaySpec<'a> {
    /// Nothing ticked and no range: the row's own empty meaning (SPEC-cpu-*).
    Empty,
    /// The ticked values: element `i` of `weights` is the weight of the value `first + i * step`
    /// (`0` = never).
    Set { weights: &'a [u8], first: u16, step: u16 },
    Range(DelayRange),
}
