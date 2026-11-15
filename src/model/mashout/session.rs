//! Worker-owned attempt and incremental solver coordination; never called from hooks.

use super::grade::Bounds;
use super::native::{Error, Seed, Solver};
use super::run::{Phase, Run};

#[derive(Clone, Copy)]
struct Cached<K> {
    key: K,
    seed: Seed,
    bounds: Bounds,
}

pub struct Session<K: Copy + Eq> {
    run: Run<K>,
    solver: Option<Solver>,
    generation: u64,
    current: Option<(K, Seed)>,
    cache: [Option<Cached<K>>; 8],
    replace: usize,
    error: Option<Error>,
}

impl<K: Copy + Eq> Default for Session<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + Eq> Session<K> {
    pub const fn new() -> Self {
        Self {
            run: Run::new(),
            solver: None,
            generation: 0,
            current: None,
            cache: [None; 8],
            replace: 0,
            error: None,
        }
    }

    /// Generations start at one and must strictly increase, including after cancel.
    /// Invalid/replayed generations return InvalidSeed without changing any state.
    pub fn begin(&mut self, generation: u64, key: K, seed: Seed) -> Result<(), Error> {
        if generation <= self.generation {
            return Err(Error::InvalidSeed);
        }
        self.generation = generation;
        self.current = Some((key, seed));
        self.solver = None;
        self.error = None;
        self.run.enable(true);
        self.run.restart();
        self.run.begin(key);
        self.run.captured();
        if let Some(cached) = self
            .cache
            .iter()
            .flatten()
            .find(|cached| cached.key == key && cached.seed == seed)
        {
            self.run.set_bounds(cached.bounds);
            return Ok(());
        }
        match Solver::new(seed) {
            Ok(solver) => {
                self.solver = Some(solver);
                Ok(())
            }
            Err(error) => {
                self.error = Some(error);
                Err(error)
            }
        }
    }

    pub fn observe(&mut self, generation: u64, elapsed: u32, escaped: bool) {
        if self.current.is_some() && generation == self.generation {
            self.run.observe(elapsed, escaped);
        }
    }

    /// True once when the result countdown finishes, requesting the caller's retry.
    pub fn pass(&mut self, generation: u64, tick: bool) -> bool {
        if self.current.is_none() || generation != self.generation {
            return false;
        }
        let done = self.run.phase() == Phase::Done;
        self.run.pass(tick);
        if done && self.run.phase() == Phase::Setup {
            self.current = None;
            return true;
        }
        false
    }

    /// Spend at most this many solver ticks on the current attempt, outside hooks.
    pub fn advance(&mut self, mut tick_budget: u32) {
        while tick_budget != 0 {
            let Some(solver) = self.solver.as_mut() else {
                return;
            };
            let work = tick_budget.min(u32::from(u16::MAX)) as u16;
            let result = solver.advance(work);
            tick_budget -= u32::from(work);
            let Some(result) = result else {
                continue;
            };
            self.solver = None;
            match result {
                Ok(bounds) => {
                    // Only this owned solver can publish; begin/cancel discard its predecessor.
                    let Some((key, seed)) = self.current else {
                        return;
                    };
                    self.cache[self.replace] = Some(Cached { key, seed, bounds });
                    self.replace = (self.replace + 1) % self.cache.len();
                    self.run.set_bounds(bounds);
                }
                Err(error) => self.error = Some(error),
            }
            return;
        }
    }

    pub fn run(&self) -> &Run<K> {
        &self.run
    }

    /// Worker notice access. Lifecycle changes must go through Session's methods.
    pub fn run_mut(&mut self) -> &mut Run<K> {
        &mut self.run
    }

    /// Cancel pending work/current observations; retain bests, cache and watermark.
    pub fn cancel(&mut self) {
        self.solver = None;
        self.current = None;
        self.error = None;
        self.run.enable(false);
    }

    pub fn error(&self) -> Option<Error> {
        self.error
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
