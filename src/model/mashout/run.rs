//! Attempt lifecycle; native observations own the elapsed clock and escape fact.

use super::grade::{grade, Bounds, Outcome};
use super::native::{Error, Seed, Solver};
use crate::model::practice_games::{record_time, result_tick};
use crate::model::{eggs::END_FRAMES, storage::FixedPath};
use core::fmt::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Off,
    Setup,
    Entering,
    Mashing,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Result {
    pub elapsed: u32,
    pub bounds: Bounds,
    pub outcome: Outcome,
    pub best: Option<u32>,
}

pub struct Run<K: Copy + Eq> {
    phase: Phase,
    key: Option<K>,
    bounds: Option<Bounds>,
    elapsed: u32,
    escaped: bool,
    result: Option<Result>,
    native_ended: bool,
    left: u16,
    bests: [Option<(K, u32)>; 16],
    generation: u64,
    solver: Option<Solver>,
    error: Option<Error>,
}

impl<K: Copy + Eq> Default for Run<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + Eq> Run<K> {
    pub const fn new() -> Self {
        Self {
            phase: Phase::Off,
            key: None,
            bounds: None,
            elapsed: 0,
            escaped: false,
            result: None,
            native_ended: false,
            left: 0,
            bests: [None; 16],
            generation: 0,
            solver: None,
            error: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    #[cfg(test)]
    pub fn key(&self) -> Option<K> {
        self.key
    }
    #[cfg(test)]
    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }
    #[cfg(test)]
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    #[cfg(test)]
    pub fn escaped(&self) -> bool {
        self.escaped
    }
    #[cfg(test)]
    pub fn result(&self) -> Option<Result> {
        self.result
    }
    #[cfg(test)]
    pub fn remaining(&self) -> u16 {
        self.left
    }

    #[cfg(test)]
    pub fn best(&self, key: K) -> Option<u32> {
        crate::model::practice_games::best_time(&self.bests, key)
    }

    fn clear(&mut self, phase: Phase) {
        self.phase = phase;
        self.key = None;
        self.bounds = None;
        self.elapsed = 0;
        self.escaped = false;
        self.result = None;
        self.native_ended = false;
        self.left = 0;
        self.solver = None;
        self.error = None;
    }

    pub fn enable(&mut self, on: bool) {
        if !on {
            self.clear(Phase::Off);
        } else if self.phase == Phase::Off {
            self.clear(Phase::Setup);
        }
    }

    /// Explicit menu/retry cancellation never grades a partial attempt.
    pub fn restart(&mut self) {
        self.clear(if self.phase == Phase::Off {
            Phase::Off
        } else {
            Phase::Setup
        });
    }

    pub fn begin(&mut self, key: K) -> bool {
        if self.phase != Phase::Setup {
            return false;
        }
        self.clear(Phase::Entering);
        self.key = Some(key);
        true
    }

    pub fn captured(&mut self) -> bool {
        if self.phase != Phase::Entering {
            return false;
        }
        self.phase = Phase::Mashing;
        self.elapsed = 0;
        true
    }

    /// Begin the observed native attempt. Generation is retained across cancellation.
    pub fn capture(
        &mut self,
        generation: u64,
        key: K,
        seed: Seed,
    ) -> core::result::Result<(), Error> {
        if generation <= self.generation {
            return Err(Error::InvalidSeed);
        }
        self.generation = generation;
        self.enable(true);
        self.restart();
        self.begin(key);
        self.captured();
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

    pub fn observe_generation(&mut self, generation: u64, elapsed: u32, escaped: bool) {
        if generation == self.generation {
            self.observe(elapsed, escaped);
        }
    }

    pub fn pass_generation(&mut self, generation: u64, tick: bool) -> bool {
        if generation != self.generation {
            return false;
        }
        let done = self.phase == Phase::Done;
        self.pass(tick);
        done && self.phase == Phase::Setup
    }

    /// Incremental solver work, called only by the existing worker. No separate cache or attempt owner.
    pub fn advance(&mut self, mut budget: u32) {
        while budget != 0 {
            let Some(solver) = self.solver.as_mut() else {
                return;
            };
            let work = budget.min(u32::from(u16::MAX)) as u16;
            budget -= u32::from(work);
            let Some(result) = solver.advance(work) else {
                continue;
            };
            self.solver = None;
            match result {
                Ok(bounds) => self.set_bounds(bounds),
                Err(error) => self.error = Some(error),
            }
            return;
        }
    }

    pub fn cancel(&mut self) {
        self.enable(false);
    }
    pub fn error(&self) -> Option<Error> {
        self.error
    }

    /// The caller binds this result to the current attempt before delivering it.
    pub fn set_bounds(&mut self, bounds: Bounds) {
        if matches!(self.phase, Phase::Entering | Phase::Mashing) {
            self.bounds = Some(bounds);
        }
    }

    /// G2 (116): bounds solved with the attempt's observed native writes replace the plain solver's answer.
    pub fn rebound(&mut self, generation: u64, bounds: Bounds) {
        if generation == self.generation && self.phase == Phase::Mashing && !self.escaped {
            self.solver = None;
            self.error = None;
            self.bounds = Some(bounds);
        }
    }

    /// Replace the preliminary bounds with the exact opening-write replay
    /// before admitting the terminal observation.
    pub fn replay_opening(
        &mut self,
        generation: u64,
        solver: Solver,
        update_key: impl FnOnce(K) -> K,
    ) {
        if generation == self.generation && self.phase == Phase::Mashing && !self.escaped {
            self.key = self.key.map(update_key);
            self.bounds = None;
            self.error = None;
            self.solver = Some(solver);
        }
    }

    pub fn observe(&mut self, elapsed: u32, escaped: bool) {
        if self.phase != Phase::Mashing || self.escaped {
            return;
        }
        self.elapsed = elapsed;
        self.escaped = escaped;
    }

    /// Proven native end of a positive hold, never an inferred countdown mismatch.
    /// Uses the ordinary result delay without a score, best or synthetic escape.
    pub fn native_end(&mut self, generation: u64) {
        if generation != self.generation || self.phase != Phase::Mashing || self.escaped {
            return;
        }
        self.solver = None;
        self.error = None;
        self.native_ended = true;
        self.phase = Phase::Done;
        self.left = END_FRAMES;
    }

    /// Commit only after the worker's final observation-admission check.
    pub fn finish(&mut self) {
        if self.phase != Phase::Mashing || !self.escaped {
            return;
        }
        let (Some(bounds), Some(key)) = (self.bounds, self.key) else {
            return;
        };
        let outcome = grade(self.elapsed, bounds);
        let best = if matches!(outcome, Outcome::Graded(_)) {
            Some(record_time(&mut self.bests, key, self.elapsed))
        } else {
            None
        };
        self.result = Some(Result {
            elapsed: self.elapsed,
            bounds,
            outcome,
            best,
        });
        self.phase = Phase::Done;
        self.left = END_FRAMES;
    }

    /// Only the result delay counts here; native mash ticks arrive via observe.
    pub fn pass(&mut self, tick: bool) {
        if self.phase == Phase::Done && result_tick(&mut self.left, tick) {
            self.clear(Phase::Setup);
        }
    }

    /// The notice worker owns delivery deduplication, including setup messages.
    pub fn notice(&self) -> FixedPath<256> {
        let mut text = FixedPath::new();
        let _ = self.write_notice(&mut text);
        text
    }

    pub fn practice_result(&self) -> Option<crate::model::practice_state::Fields> {
        use crate::model::practice_state::Fields;
        use super::grade::Grade;

        if self.phase != Phase::Done {
            return None;
        }
        if self.native_ended {
            return Some(Fields { last_outcome: "ended_before_escape", ..Fields::default() });
        }
        let result = self.result?;
        let last_outcome = match result.outcome {
            Outcome::Graded(Grade::Poor) => "poor",
            Outcome::Graded(Grade::Ok) => "ok",
            Outcome::Graded(Grade::Good) => "good",
            Outcome::Graded(Grade::Great) => "great",
            Outcome::Graded(Grade::Optimal) => "optimal",
            Outcome::BelowOptimum => "below_optimum",
            Outcome::NeverEnds => "never_ends",
            Outcome::NoSaving => "no_saving",
            Outcome::InvalidBounds => "invalid_bounds",
        };
        Some(Fields {
            last_outcome,
            last_frames: Some(result.elapsed),
            best: result.best,
            fastest_frames: Some(result.bounds.fastest),
            baseline_frames: result.bounds.no_input,
            ..Fields::default()
        })
    }

    pub fn write_notice(&self, out: &mut impl Write) -> fmt::Result {
        match self.phase {
            Phase::Off => Ok(()),
            Phase::Setup => out.write_str("Mash out · Setting up"),
            Phase::Entering => out.write_str("Mash out · Waiting for capture"),
            Phase::Mashing if self.escaped => {
                write!(out, "Escaped · {}f · Calculating fastest", self.elapsed)
            }
            Phase::Mashing => write!(out, "Mash out · Mash! · {}f", self.elapsed),
            Phase::Done if self.native_ended => {
                // restriction: MASH-NATIVE-END
                out.write_str("Move ended before escape · no grade")
            }
            Phase::Done => {
                let Some(r) = self.result else {
                    return Ok(());
                };
                let (elapsed, fastest) = (r.elapsed, r.bounds.fastest);
                match r.outcome {
                    Outcome::Graded(g) => {
                        write!(out, "{} · {elapsed}f · fastest {fastest}f", g.label())?;
                        if let Some(end) = r.bounds.no_input {
                            write!(out, " · no mash {end}f")?;
                        }
                        if let Some(best) = r.best {
                            write!(out, " · best {best}f")?;
                        }
                        Ok(())
                    }
                    Outcome::NeverEnds => {
                        // restriction: MASH-R2-NO-END
                        write!(out, "Escaped · {elapsed}f · fastest {fastest}f (no grade: this hold never ends on its own)")
                    }
                    Outcome::NoSaving => {
                        // restriction: MASH-R2-NO-SAVING
                        write!(
                            out,
                            "Escaped · {elapsed}f · Mashing can't shorten this hold"
                        )
                    }
                    Outcome::BelowOptimum => {
                        // restriction: MASH-M1-MISMATCH
                        write!(
                            out,
                            "Model mismatch · {elapsed}f · fastest {fastest}f (no grade)"
                        )
                    }
                    Outcome::InvalidBounds => {
                        // restriction: MASH-M1-MISMATCH
                        write!(
                            out,
                            "Model mismatch · {elapsed}f (no grade: invalid bounds)"
                        )
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::{Bounds, Outcome, Phase, Run};
    use super::super::grade::Grade;
    use crate::model::storage::FixedPath;

    const FINITE: Bounds = Bounds { fastest: 10, no_input: Some(90) };

    fn attempt(run: &mut Run<u32>, key: u32, elapsed: u32, bounds: Bounds) {
        run.restart();
        assert!(run.begin(key));
        assert!(run.captured());
        run.set_bounds(bounds);
        run.observe(elapsed, true);
        run.finish();
    }

    #[test]
    fn mashout_practice_result_publishes_the_scored_native_clock() {
        let mut game = Run::new();
        assert!(game.practice_result().is_none());
        attempt(&mut game, 4, 11, FINITE);
        let fields = game.practice_result().unwrap();
        assert_eq!((fields.last_outcome, fields.last_frames, fields.best, fields.fastest_frames, fields.baseline_frames),
            ("great", Some(11), Some(11), Some(10), Some(90)));
    }

    #[test]
    fn mashout_run_entry_without_score() {
        let mut run = Run::<u32>::new();
        assert_eq!(run.phase(), Phase::Off);
        assert!(!run.begin(1));
        run.observe(30, true);
        run.set_bounds(FINITE);
        run.enable(true);
        run.set_bounds(FINITE);
        assert_eq!(run.bounds(), None);
        assert!(run.begin(1));
        assert!(!run.begin(2));
        run.observe(30, true);
        run.set_bounds(FINITE);
        for _ in 0..200 { run.pass(true); }
        assert_eq!(run.phase(), Phase::Entering);
        assert_eq!(run.result(), None);
        assert_eq!(run.best(1), None);
        assert!(run.captured());
        assert!(!run.captured());
        assert_eq!(run.elapsed(), 0);
        assert_eq!(run.phase(), Phase::Mashing);
    }

    #[test]
    fn mashout_run_bounds_order_and_clock() {
        let mut early = Run::<u32>::new();
        early.enable(true);
        early.begin(9);
        early.set_bounds(FINITE);
        early.captured();
        early.observe(7, false);
        for _ in 0..20 { early.pass(true); }
        assert_eq!(early.elapsed(), 7);
        assert_eq!(early.result(), None);
        early.observe(30, true);
        early.finish();
        let mut late = Run::<u32>::new();
        late.enable(true);
        late.begin(9);
        late.captured();
        late.observe(30, true);
        assert_eq!(late.phase(), Phase::Mashing);
        assert!(late.escaped());
        late.observe(99, false);
        late.observe(100, true);
        assert_eq!(late.elapsed(), 30);
        assert_eq!(late.result(), None);
        late.set_bounds(FINITE);
        late.finish();
        assert_eq!(early.result(), late.result());
        assert_eq!(late.result().unwrap().outcome, Outcome::Graded(Grade::Good));
        late.set_bounds(Bounds { fastest: 100, no_input: None });
        late.observe(999, true);
        assert_eq!(early.result(), late.result());
    }

    #[test]
    fn mashout_run_grader_edges() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        for (elapsed, expected) in [(10, Grade::Optimal), (18, Grade::Great),
            (19, Grade::Good), (30, Grade::Good), (31, Grade::Ok),
            (50, Grade::Ok), (51, Grade::Poor), (91, Grade::Poor)] {
            attempt(&mut run, elapsed, elapsed, FINITE);
            assert_eq!(run.result().unwrap().outcome, Outcome::Graded(expected));
            assert_eq!(run.best(elapsed), Some(elapsed));
        }
    }

    #[test]
    fn mashout_run_best_keys() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        attempt(&mut run, 0, 51, FINITE);
        attempt(&mut run, 1, 24, FINITE);
        attempt(&mut run, 0, 30, FINITE);
        attempt(&mut run, 0, 50, FINITE);
        assert_eq!(run.best(0), Some(30));
        assert_eq!(run.best(1), Some(24));
        assert_eq!(run.best(17), None);
        assert_eq!(run.result().unwrap().best, Some(30));
        run.enable(false);
        run.enable(true);
        assert_eq!(run.best(1), Some(24));
    }

    #[test]
    fn mashout_run_unscored() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        attempt(&mut run, 7, 30, FINITE);
        for (elapsed, bounds, expected) in [
            (34, Bounds { fastest: 12, no_input: None }, Outcome::NeverEnds),
            (80, Bounds { fastest: 80, no_input: Some(80) }, Outcome::NoSaving),
            (9, FINITE, Outcome::BelowOptimum),
            (30, Bounds { fastest: 31, no_input: Some(30) }, Outcome::InvalidBounds),
        ] {
            attempt(&mut run, 7, elapsed, bounds);
            let result = run.result().unwrap();
            assert_eq!(result.outcome, expected);
            assert_eq!(result.best, None);
            assert_eq!(run.best(7), Some(30));
            attempt(&mut run, 8, elapsed, bounds);
            assert_eq!(run.best(8), None);
        }
    }

    #[test]
    fn mashout_run_cancel_restart() {
        let mut run = Run::<u32>::new();
        run.restart();
        assert_eq!(run.phase(), Phase::Off);
        run.enable(true);
        attempt(&mut run, 1, 24, FINITE);
        run.restart();
        assert_eq!(run.result(), None);
        assert_eq!(run.key(), None);
        assert_eq!(run.bounds(), None);
        run.begin(2);
        run.captured();
        run.observe(30, true);
        run.restart();
        run.set_bounds(FINITE);
        assert_eq!(run.result(), None);
        run.begin(3);
        run.set_bounds(FINITE);
        run.captured();
        run.observe(15, false);
        run.enable(true);
        assert_eq!(run.elapsed(), 15);
        run.enable(false);
        run.observe(30, true);
        run.set_bounds(FINITE);
        assert_eq!(run.phase(), Phase::Off);
        assert_eq!(run.result(), None);
        assert_eq!(run.best(2), None);
        assert_eq!(run.best(3), None);
        assert_eq!(run.best(1), Some(24));
        run.enable(true);
        run.begin(4);
        run.captured();
        run.observe(30, true);
        assert_eq!(run.result(), None);
    }

    #[test]
    fn mashout_run_done_ticks() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        attempt(&mut run, 1, 30, FINITE);
        assert_eq!(run.remaining(), 180);
        for _ in 0..179 { run.pass(false); run.pass(true); }
        assert_eq!(run.phase(), Phase::Done);
        assert_eq!(run.remaining(), 1);
        for _ in 0..200 { run.pass(false); }
        assert_eq!(run.phase(), Phase::Done);
        run.pass(true);
        assert_eq!(run.phase(), Phase::Setup);
        assert_eq!(run.result(), None);
        assert_eq!(run.elapsed(), 0);
        assert_eq!(run.best(1), Some(30));
    }

    #[test]
    fn mashout_run_cache_replacement() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        for key in 0..16 { attempt(&mut run, key, 30 + key, FINITE); }
        attempt(&mut run, 1, 24, FINITE);
        attempt(&mut run, 16, 50, FINITE);
        assert_eq!(run.best(0), None);
        assert_eq!(run.best(1), Some(24));
        for key in 2..16 { assert_eq!(run.best(key), Some(30 + key)); }
        assert_eq!(run.best(16), Some(50));
        attempt(&mut run, 17, 20, FINITE);
        assert_eq!(run.best(1), None);
        assert_eq!(run.best(17), Some(20));
    }

    #[test]
    fn mashout_run_notices() {
        let mut run = Run::<u32>::new();
        assert_eq!(run.notice().as_str(), "");
        run.enable(true);
        assert_eq!(run.notice().as_str(), "Mash out · Setting up");
        assert_eq!(run.notice().as_str(), "Mash out · Setting up");
        attempt(&mut run, 1, 24, FINITE);
        attempt(&mut run, 1, 30, FINITE);
        assert_eq!(run.notice().as_str(), "Good · 30f · fastest 10f · no mash 90f · best 24f");
        assert_eq!(run.notice().as_str(), "Good · 30f · fastest 10f · no mash 90f · best 24f");
        attempt(&mut run, 2, 34, Bounds { fastest: 12, no_input: None });
        assert_eq!(run.notice().as_str(), "Escaped · 34f · fastest 12f (no grade: this hold never ends on its own)");
        attempt(&mut run, 3, 80, Bounds { fastest: 80, no_input: Some(80) });
        let text = run.notice();
        assert!(text.as_str().contains("Mashing can't shorten this hold"));
        assert!(!text.as_str().contains("best"));
        attempt(&mut run, 4, 9, FINITE);
        assert!(run.notice().as_str().contains("Model mismatch"));
        attempt(&mut run, 5, u32::MAX, Bounds { fastest: 1, no_input: Some(u32::MAX) });
        let mut out = FixedPath::<256>::new();
        assert!(run.write_notice(&mut out).is_ok());
        run.enable(false);
        assert_eq!(run.notice().as_str(), "");
        assert_eq!(run.notice().as_str(), "");
    }

    #[test]
    fn mashout_run_identical_result_after_setup_is_rendered_again() {
        let mut run = Run::<u32>::new();
        run.enable(true);
        attempt(&mut run, 1, 30, FINITE);
        let first = run.notice();
        run.restart();
        assert_eq!(run.notice().as_str(), "Mash out · Setting up");
        attempt(&mut run, 1, 30, FINITE);
        assert_eq!(run.notice().as_str(), first.as_str());
    }
    }
}
