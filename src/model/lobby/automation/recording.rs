//! Bounded change capture. Allocation happens once, before native hooks start.
//! Tick/age are metadata: a ticking clock alone must not fill the change buffer.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    NativeExit,
    SnapshotLimit,
    TickLimit,
    BindingChanged,
    Contention,
    InvalidBinding,
}

pub struct Frame<T> {
    pub tick: u32,
    pub native_age: u32,
    pub last_tick: u32,
    pub last_native_age: u32,
    pub value: T,
}

pub struct Recording<T> {
    frames: Vec<Frame<T>>,
    limit: usize,
    ticks: u32,
    tick_limit: u32,
    stop: Option<Stop>,
}

impl<T: PartialEq> Recording<T> {
    pub fn new(limit: usize, tick_limit: u32) -> Self {
        assert!(limit > 0 && tick_limit > 0);
        Self {
            frames: Vec::with_capacity(limit),
            limit,
            ticks: 0,
            tick_limit,
            stop: None,
        }
    }

    /// No growth, blocking, or allocation. False means the caller must close capture.
    pub fn observe(&mut self, value: T, native_age: u32) -> bool {
        if self.stop.is_some() {
            return false;
        }
        self.ticks += 1;
        if self.frames.last().is_none_or(|last| last.value != value) {
            if self.frames.len() == self.limit {
                self.finish(Stop::SnapshotLimit);
                return false;
            }
            self.frames.push(Frame {
                tick: self.ticks,
                native_age,
                last_tick: self.ticks,
                last_native_age: native_age,
                value,
            });
        } else if let Some(last) = self.frames.last_mut() {
            last.last_tick = self.ticks;
            last.last_native_age = native_age;
        }
        if self.ticks >= self.tick_limit {
            self.finish(Stop::TickLimit);
            return false;
        }
        true
    }

    pub fn finish(&mut self, reason: Stop) {
        if self.stop.is_none() {
            self.stop = Some(reason);
        }
    }

    pub fn frames(&self) -> &[Frame<T>] {
        &self.frames
    }
    pub fn ticks(&self) -> u32 {
        self.ticks
    }
    pub fn stop(&self) -> Option<Stop> {
        self.stop
    }
}

