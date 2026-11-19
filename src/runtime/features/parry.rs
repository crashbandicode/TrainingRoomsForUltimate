//! Parry practice, the runtime glue (`artifacts/rc15-parry/DESIGN.md` rev 2, §3–4): the session row, the per-frame step of
//! `model::parry::Detector` from the strip's P1 sample, Tech's `Summary`/`Bests` for the set and its Done line, and the
//! strip's view. Game thread only, except `publish` (the settings publish) and `view` (the HUD tick).

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use core::fmt::Write as _;

use hos_lock::{Mutex, Published};

use crate::model::parry::{Attempt, Detector, Frame, View};
use crate::model::practice_games::{self, Game as PracticeGame};
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::model::tech::session::{Bests, Grade, Summary};

/// The session row: 0 Off, 1 sets of ten, 2 free practice.
static SESSION: AtomicU8 = AtomicU8::new(0);
static FRESH_SET: AtomicBool = AtomicBool::new(false);
const DONE_NOTICE: u8 = 0x33;
static VIEW: Published<Option<View>> = Published::new(None);

struct State {
    detector: Detector,
    view: View,
    summary: Summary,
    bests: Bests,
    total: u32,
    perfect: u32,
    /// Frames since the parry pass ended (`None`: one plays, or none played yet).
    since_end: Option<u32>,
}

const SUMMARY: Summary = Summary { attempts: 0, perfect: 0, held: 0, early: 0, late: 0, locked: 0, missed: 0, streak: 0, best: 0, offset_sum: 0, timed_perfect: 0 };
static STATE: Mutex<State> = Mutex::new(State { detector: Detector::new(), view: View::EMPTY, summary: SUMMARY, bests: Bests::EMPTY, total: 0, perfect: 0, since_end: None });

/// The session row as the last settings publish left it (a stored Off reads as sets of ten, rc.15 practice menu).
static PUBLISHED: AtomicU8 = AtomicU8::new(0);

/// Every settings publish: the row only. The running practice is the menu's (Start and Stop), and its session is the
/// one its Start latched.
pub fn publish(snapshot: &Snapshot) {
    let session = crate::model::tech_settings::selected(snapshot, "parry_practice").min(2) as u8;
    PUBLISHED.store(session, Ordering::Release);
}

/// Start latches the mode and arms replay. A saved practice resumes its current set.
pub fn start(resume: bool) {
    if !resume {
        FRESH_SET.store(true, Ordering::Release);
    }
    SESSION.store(practice_games::mode(PUBLISHED.load(Ordering::Acquire)), Ordering::Release);
    crate::engine::rr_host::set_parry(true);
}

/// Another game's Start or Stop (the menu worker): the replay host disarms.
pub fn stopped() {
    crate::engine::rr_host::set_parry(false);
}

/// Parry practice is on: the strip shows even with Input Display Off (operator Q2, 2026-11-17).
pub fn on() -> bool {
    super::practice::is(PracticeGame::Parry) && SESSION.load(Ordering::Acquire) != 0
}

/// Free practice: a running total, no sets.
pub fn free() -> bool {
    on() && SESSION.load(Ordering::Acquire) == 2
}

/// The session the row shows, for saving a practice: free practice, else sets of ten.
pub fn chosen_free() -> bool {
    practice_games::mode(PUBLISHED.load(Ordering::Acquire)) == 2
}

/// The strip's view, for the HUD tick.
pub fn view() -> Option<View> {
    VIEW.read().filter(|_| on())
}

/// A load, an L+R+A reset, leaving the scene: an open attempt is dropped uncounted; the set and the totals stay.
pub fn reset() {
    if let Some(mut s) = STATE.try_lock() {
        (s.detector, s.since_end) = (Detector::new(), None);
    }
    crate::engine::rr_host::set_parry_open(false);
}

fn record(s: &mut State, a: Attempt, fighter: Option<i32>) {
    s.view.last.copy_within(0..3, 1);
    s.view.last[0] = Some(a);
    if !a.grade.counted() {
        return;
    }
    s.total += 1;
    s.perfect += u32::from(matches!(a.grade, Grade::Perfect(_)));
    s.summary.record(a.grade);
    let best = s.bests.record(fighter.unwrap_or(-1), s.summary.best);
    if !free() && s.summary.attempts >= 10 {
        // Tech's own Done line, in the compact top-left box once rc15/notice-compact merges.
        let mut body = FixedPath::<192>::new();
        let _ = s.summary.write(&mut body);
        let _ = write!(body, " · total {} · fighter best {best}", s.total);
        super::notices::upsert(DONE_NOTICE, "Parry Practice", body.as_str(), 300, crate::model::notify::DEFAULT_COLOR);
        s.summary = SUMMARY;
    }
}

/// One P1 game frame, after the strip stepped (`now`: the strip's frame number): `armed` is computed here from the CPU's
/// parry pass.
pub fn step(mut frame: Frame, fighter: Option<i32>, now: u32) {
    let Some(mut s) = STATE.try_lock() else { return };
    let fresh = FRESH_SET.swap(false, Ordering::AcqRel);
    if fresh {
        (s.summary, s.detector, s.since_end) = (SUMMARY, Detector::new(), None);
        super::notices::clear(DONE_NOTICE);
    }
    let playing = crate::engine::rr_host::cpu_run_active();
    s.since_end = if playing { Some(0) } else { s.since_end.map(|n| n.saturating_add(1)) };
    frame.armed = !fresh && s.since_end.is_some_and(|n| n <= 60);
    for a in s.detector.step(&frame, now).into_iter().flatten() {
        record(&mut s, a, fighter);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!(
            "[Parry] attempt grade={:?} hit={} window={:?} total={} set={}",
            a.grade, a.hit, a.window, s.total, s.summary.attempts
        ));
    }
    crate::engine::rr_host::set_parry_open(s.detector.busy());
    let (mut set, mut line) = (crate::model::frame_strip_plan::Text::EMPTY, crate::model::frame_strip_plan::Text::EMPTY);
    let _ = if free() { write!(set, "#{}", s.total) } else { write!(set, "{}/10", s.summary.attempts) };
    let _ = write!(line, "streak {}, perfect {}/{}", s.summary.streak, s.perfect, s.total);
    (s.view.set, s.view.set_line) = (set, line);
    VIEW.publish(Some(s.view));
}
