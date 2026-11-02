//! The frame-counter service (SPEC-cpu-07 §5): a fixed set of counters that advance once per
//! simulation frame and that a Training reset or save-state load clears. Every delay of the CPU specs
//! (aerial, fast fall, throw, pummel, reaction time, ledge, lay-down lockout) counts on one of them.
//! Decision B6 gives each fighter body its own set, so Popo's delay never runs on Nana's clock; the
//! clock itself still ticks exactly once per simulation frame, from the primary CPU body
//! (`Cpu::tick_clock`), whatever the number of bodies.

/// One counter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    pub frames: u32,
    pub counting: bool,
}

impl Counter {
    pub const fn new() -> Self {
        Self { frames: 0, counting: false }
    }

    /// Count 0 and stop.
    pub fn full_reset(&mut self) {
        *self = Self::new();
    }

    /// One simulation frame passed.
    pub fn tick(&mut self) {
        if self.counting {
            self.frames = self.frames.saturating_add(1);
        }
    }

    /// "Still waiting?": false at once when `delay == 0`; otherwise start counting when the count is
    /// 0; when the count has reached `delay` do a full reset and return false; else true.
    pub fn should_delay(&mut self, delay: u16) -> bool {
        if delay == 0 {
            return false;
        }
        if self.frames == 0 {
            self.counting = true;
        }
        if self.frames >= u32::from(delay) {
            self.full_reset();
            return false;
        }
        true
    }
}

/// The consumers of one body's set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterId {
    Aerial,
    FastFall,
    Throw,
    Pummel,
    Reaction,
    Ledge,
    LayDown,
}

impl CounterId {
    pub const COUNT: usize = 7;
}

/// One body's counters. All are in-game counters: the reset bundle clears them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters([Counter; CounterId::COUNT]);

impl Counters {
    pub const fn new() -> Self {
        Self([Counter::new(); CounterId::COUNT])
    }

    pub fn get(&mut self, id: CounterId) -> &mut Counter {
        &mut self.0[id as usize]
    }

    pub fn count(&self, id: CounterId) -> u32 {
        self.0[id as usize].frames
    }

    pub fn tick(&mut self) {
        for counter in &mut self.0 {
            counter.tick();
        }
    }

    /// The reset bundle's "reset all in-game counters".
    pub fn reset_all(&mut self) {
        for counter in &mut self.0 {
            counter.full_reset();
        }
    }
}
