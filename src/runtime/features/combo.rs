//! The combo helper at run time (SPEC-input-07; operator decision COMBO-HELPER A, 2026-11-05).
//!
//! - On P1's entry callback, right after the frame strip's tracker stepped (`input_display::sample_frame`), the helper
//!   (`model::combo`) takes one step on the same tracker. Fixed arrays; nothing here allocates.
//! - After an attempt, *Load save state* asks for the Load chord's own load (`states::request_load`).
//! - The menu worker sets the target and reads the last combo (`runtime::ui::combos`).
//! - The HUD tick draws the bar under the strip (`model::combo_plan`), writing only what changed.
//! - Every graded link, resolved attempt and saved combo calls an exported trace function (§8): an Eden guest trace
//!   point on it records the facts, with no log line.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hos_lock::Mutex;

use crate::model::combo::{Combo, Helper, Label, Mark, Out, Outcome, Settings};
use crate::model::combo_plan::Renderer;
use crate::model::frame_strip::{Tracker, Verdict};
use crate::model::frame_strip_plan::{Changed, Look, Text};
use crate::runtime::published;

static HELPER: Mutex<Helper> = Mutex::new(Helper::new());
/// The worker's copy of the last combo, with the helper's count of links captured (`Helper::seen`). The game thread
/// publishes it after it has let go of HELPER.
static SAMPLE: Mutex<(u32, Option<Combo>)> = Mutex::new((0, None));
static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());
/// The target's name for the bar, and the names the menu shows (target, draft).
static NAME: Mutex<Text> = Mutex::new(Text::EMPTY);
static LABELS: Mutex<(Label, Label)> = Mutex::new((Label::EMPTY, Label::EMPTY));
/// P1's fighter kind at the strip's last sample (-1 none): *Save last combo* writes its name as `recorded_on`.
static P1_KIND: AtomicI32 = AtomicI32::new(-1);

fn settings() -> Settings {
    published::live().map_or(Settings::factory(), |live| live.combo)
}

/// One frame, right after the strip's tracker stepped (game thread, inside the fighter's frame). `p1_kind` is the
/// fighter P1 is now: kept for the next save's `recorded_on` only; the helper grades whatever fighter P1 is.
pub fn after_strip(tracker: &Tracker, p1_kind: Option<i32>) {
    P1_KIND.store(p1_kind.unwrap_or(-1), Ordering::Relaxed);
    let (out, sample, names) = {
        let mut helper = HELPER.lock();
        let names = pull_staged().map(|staged| {
            let (combo, text, label) = match staged {
                Some((combo, text, label)) => (Some(combo), text, label),
                None => (None, Text::EMPTY, Label::EMPTY),
            };
            helper.set_target(combo);
            (text, label)
        });
        let out = helper.step(tracker, settings());
        let sample = (helper.seen(), helper.last_combo());
        (out, sample, names)
    };
    if let Some((text, label)) = names {
        *NAME.lock() = text;
        LABELS.lock().0 = label;
    }
    report(out);
    *SAMPLE.lock() = sample;
}

/// The strip's session clear (leaving Training, a hot swap, a lobby change).
pub fn session_clear() {
    let (out, seen) = {
        let mut helper = HELPER.lock();
        (helper.clear(), helper.seen())
    };
    report(out);
    *SAMPLE.lock() = (seen, None);
}

fn report(out: Out) {
    if let Some((index, grade)) = out.link {
        let tolerance = settings().tolerance;
        let flags = u32::from(grade.is_true())
            | u32::from(grade.clean(tolerance)) << 1
            | u32::from(grade.wrong_move) << 2
            | u32::from(grade.missing().is_some()) << 3;
        let offset = match grade.marks[0] {
            Some(Mark::At(d)) => d,
            _ => i32::MIN,
        };
        tr_combo_link_v1(index as u32, verdict_value(grade.link.verdict), flags, offset);
    }
    if let Some(s) = out.resolved {
        let outcome = match s.outcome {
            Outcome::Completed => 1,
            Outcome::Stopped => 2,
            Outcome::Cleared => 3,
        };
        tr_combo_attempt_v1(outcome, u32::from(s.true_links), u32::from(s.total), s.fault.map_or(0, |(k, _)| u32::from(k) + 1));
    }
    if out.load {
        crate::runtime::features::states::request_load();
    }
}

fn verdict_value(verdict: Verdict) -> i32 {
    match verdict {
        Verdict::True(Some(link)) | Verdict::Drop(link) => link,
        Verdict::True(None) => i32::MIN,
    }
}

/// A link of an attempt was graded (SPEC-input-07 §8): its index, the verdict's link value (`i32::MIN` unknown), flags
/// (bit 0 true, 1 clean, 2 wrong move, 3 a missing event) and the first event's offset (`i32::MIN` none).
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_combo_link_v1(index: u32, link: i32, flags: u32, offset: i32) {
    // Each trace function keeps a tag of its own: identical bodies are folded into one address by the release link
    // (rc.4's ELF had link and attempt at the same offset), and a trace point could not tell them apart.
    core::hint::black_box((0x4c49_4e4b_u32, index, link, flags, offset));
}

/// An attempt resolved: outcome (1 completed, 2 stopped, 3 cleared), true links, reference links, first link not true + 1.
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_combo_attempt_v1(outcome: u32, true_links: u32, total: u32, first_bad: u32) {
    core::hint::black_box((0x4154_5450_u32, outcome, true_links, total, first_bad));
}

/// A combo was saved: its links and the first link's verdict value.
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_combo_saved_v1(links: u32, first: i32, _reserved0: u32, _reserved1: u32) {
    core::hint::black_box((0x5341_5645_u32, links, first));
}

/// The saved-combo list was read (SPEC-input-07 §8): files listed, files skipped (a notice names the first), and listed
/// files whose names the plugin did not choose (copied in).
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_combo_listed_v1(entries: u32, skipped: u32, shared: u32, _reserved: u32) {
    core::hint::black_box((0x4c49_5354_u32, entries, skipped, shared));
}

/// A chosen combo's fighter question (SPEC-input-07 §4a, §8): the event (`editors::combo_fighter::event`), the recorded
/// fighter's kind and P1's (`u32::MAX` none), and its detail (why it was kept; whether the switch was queued).
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_combo_fighter_v1(event: u32, recorded: u32, current: u32, detail: u32) {
    core::hint::black_box((0x4647_4854_u32, event, recorded, current, detail));
}

/// P1's fighter kind at the strip's last sample, for a save's `recorded_on` (worker).
pub fn p1_kind() -> Option<i32> {
    let kind = P1_KIND.load(Ordering::Relaxed);
    (kind >= 0).then_some(kind)
}

/// *Save last combo* (worker): the open exchange when it has a link, else the one before.
pub fn last_combo() -> Option<Combo> {
    HELPER.lock().last_combo()
}

/// The last combo published by the strip and the count of links captured, without taking the helper lock the frame
/// hook holds.
pub fn strip_sample() -> (u32, Option<Combo>) {
    *SAMPLE.lock()
}

/// The worker read the saved-combo list (`tr_combo_listed_v1`).
pub fn listed(entries: usize, skipped: usize, shared: usize) {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    tr_combo_listed_v1(count(entries), count(skipped), count(shared), 0);
}

/// The worker decided or answered a combo's fighter question (`tr_combo_fighter_v1`).
pub fn fighter_event(event: u32, recorded: Option<u32>, current: Option<u32>, detail: u32) {
    tr_combo_fighter_v1(event, recorded.unwrap_or(u32::MAX), current.unwrap_or(u32::MAX), detail);
}

/// The worker saved `combo`.
pub fn saved(combo: &Combo) {
    tr_combo_saved_v1(combo.len() as u32, combo.get(0).map_or(i32::MIN, |l| verdict_value(l.verdict)), 0, 0);
}

/// A new target (worker): the helper starts over on it, and the bar shows its name.
pub fn set_target(target: Option<(Combo, &str)>) {
    HELPER.lock().set_target(target.map(|(combo, _)| combo));
    *NAME.lock() = Text::new(target.map_or("", |(_, name)| name));
    LABELS.lock().0 = Label::new(target.map_or("", |(_, name)| name));
}

/// A drill the menu worker staged for the frame hook. The hook holds `HELPER`; the worker must not.
struct StagedTarget {
    seq: AtomicU32,
    value: core::cell::UnsafeCell<Option<(Combo, Text, Label)>>,
}

// The seqlock below is the synchronization. One menu worker writes; the frame hook reads.
unsafe impl Sync for StagedTarget {}

static STAGED: StagedTarget = StagedTarget {
    seq: AtomicU32::new(0),
    value: core::cell::UnsafeCell::new(None),
};
static STAGED_SEEN: AtomicU32 = AtomicU32::new(0);

/// Publish a grade target without taking `HELPER`. The next `after_strip` installs it.
pub fn stage_target(target: Option<(Combo, &str)>) {
    let packed = target.map(|(combo, name)| (combo, Text::new(name), Label::new(name)));
    let prev = STAGED.seq.fetch_add(1, Ordering::Relaxed);
    unsafe { *STAGED.value.get() = packed; }
    STAGED.seq.store(prev.wrapping_add(2), Ordering::Release);
}

/// `Some` when a staged target is ready: `Some(None)` clears, `Some(Some(_))` installs.
fn pull_staged() -> Option<Option<(Combo, Text, Label)>> {
    let seq = STAGED.seq.load(Ordering::Acquire);
    if seq & 1 == 1 || seq == STAGED_SEEN.load(Ordering::Relaxed) {
        return None;
    }
    let value = unsafe { *STAGED.value.get() };
    if STAGED.seq.load(Ordering::Acquire) != seq {
        return None;
    }
    STAGED_SEEN.store(seq, Ordering::Relaxed);
    Some(value)
}

/// The draft name the next save uses (worker).
pub fn set_draft(draft: &str) {
    LABELS.lock().1 = Label::new(draft);
}

/// The menu's labels: the target's name and the draft (`model::menu::Env`).
pub fn labels() -> (Label, Label) {
    *LABELS.lock()
}

/// The plugin's layout was registered again: every pane is written again.
pub fn on_layout_registered() {
    RENDERER.lock().invalidate();
}

/// One HUD tick: the bar's writes, shown with the strip while the helper is On and has a target.
pub fn hud_tick(strip_shown: bool, apply: impl FnMut(usize, &Look, Changed)) -> usize {
    let helper = *HELPER.lock();
    let name = *NAME.lock();
    RENDERER.lock().draw(&helper, &name, strip_shown, apply)
}
