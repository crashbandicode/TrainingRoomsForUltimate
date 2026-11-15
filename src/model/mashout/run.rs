//! Attempt lifecycle; native observations own the elapsed clock and escape fact.

use super::grade::{grade, Bounds, Outcome};
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
    left: u16,
    bests: [Option<(K, u32)>; 16],
    shown: FixedPath<256>,
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
            left: 0,
            bests: [None; 16],
            shown: FixedPath::new(),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn key(&self) -> Option<K> {
        self.key
    }
    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    pub fn escaped(&self) -> bool {
        self.escaped
    }
    pub fn result(&self) -> Option<Result> {
        self.result
    }
    pub fn remaining(&self) -> u16 {
        self.left
    }

    pub fn best(&self, key: K) -> Option<u32> {
        self.bests
            .iter()
            .flatten()
            .find(|b| b.0 == key)
            .map(|b| b.1)
    }

    fn clear(&mut self, phase: Phase) {
        self.phase = phase;
        self.key = None;
        self.bounds = None;
        self.elapsed = 0;
        self.escaped = false;
        self.result = None;
        self.left = 0;
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

    /// The caller binds this result to the current attempt before delivering it.
    pub fn set_bounds(&mut self, bounds: Bounds) {
        if matches!(self.phase, Phase::Entering | Phase::Mashing) {
            self.bounds = Some(bounds);
            self.finish();
        }
    }

    pub fn observe(&mut self, elapsed: u32, escaped: bool) {
        if self.phase != Phase::Mashing || self.escaped {
            return;
        }
        self.elapsed = elapsed;
        self.escaped = escaped;
        self.finish();
    }

    fn finish(&mut self) {
        if self.phase != Phase::Mashing || !self.escaped {
            return;
        }
        let (Some(bounds), Some(key)) = (self.bounds, self.key) else {
            return;
        };
        let outcome = grade(self.elapsed, bounds);
        let best = if matches!(outcome, Outcome::Graded(_)) {
            let i = self
                .bests
                .iter()
                .position(|b| b.is_some_and(|b| b.0 == key))
                .unwrap_or_else(|| {
                    self.bests.rotate_left(1);
                    self.bests[15] = Some((key, u32::MAX));
                    15
                });
            let best = self.bests[i].map_or(self.elapsed, |b| b.1.min(self.elapsed));
            self.bests[i] = Some((key, best));
            Some(best)
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
        if self.phase == Phase::Done && tick {
            self.left -= 1;
            if self.left == 0 {
                self.clear(Phase::Setup);
            }
        }
    }

    pub fn notice(&mut self) -> Option<FixedPath<256>> {
        let mut text = FixedPath::new();
        let _ = self.write_notice(&mut text);
        let changed = text.as_str() != self.shown.as_str();
        self.shown = text;
        changed.then_some(text)
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
        assert!(run.notice().is_none());
        run.enable(true);
        assert_eq!(run.notice().unwrap().as_str(), "Mash out · Setting up");
        assert!(run.notice().is_none());
        attempt(&mut run, 1, 24, FINITE);
        attempt(&mut run, 1, 30, FINITE);
        assert_eq!(run.notice().unwrap().as_str(), "Good · 30f · fastest 10f · no mash 90f · best 24f");
        assert!(run.notice().is_none());
        attempt(&mut run, 2, 34, Bounds { fastest: 12, no_input: None });
        assert_eq!(run.notice().unwrap().as_str(), "Escaped · 34f · fastest 12f (no grade: this hold never ends on its own)");
        attempt(&mut run, 3, 80, Bounds { fastest: 80, no_input: Some(80) });
        let text = run.notice().unwrap();
        assert!(text.as_str().contains("Mashing can't shorten this hold"));
        assert!(!text.as_str().contains("best"));
        attempt(&mut run, 4, 9, FINITE);
        assert!(run.notice().unwrap().as_str().contains("Model mismatch"));
        attempt(&mut run, 5, u32::MAX, Bounds { fastest: 1, no_input: Some(u32::MAX) });
        let mut out = FixedPath::<256>::new();
        assert!(run.write_notice(&mut out).is_ok());
        run.enable(false);
        assert_eq!(run.notice().unwrap().as_str(), "");
        assert!(run.notice().is_none());
    }
    }
}
