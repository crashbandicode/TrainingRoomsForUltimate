//! The combo helper at run time (SPEC-input-07; operator decision COMBO-HELPER A, 2026-11-05).
//!
//! - On P1's entry callback, right after the frame strip's tracker stepped (`input_display::sample_frame`), the helper
//!   (`model::combo`) takes one step on the same tracker. Fixed arrays; nothing here allocates.
//! - After an attempt, *Load save state* asks for the Load chord's own load (`states::request_load`).
//! - The menu worker sets the target and reads the last combo (`runtime::ui::combos`).
//! - The HUD tick draws the bar under the strip (`model::combo_plan`), writing only what changed.
//! - Every graded link, resolved attempt and saved combo calls an exported trace function (§8): an Eden guest trace
//!   point on it records the facts, with no log line.
//! - The lock-free runtime (operator order 2026-11-10 00:44Z, `artifacts/phase23/TASK-OPUS-rc6-lockfree.md`): the game's locks
//!   here (`HELPER`, `RUN`, `PLAN`, `NAME`, `RENDERER`) are taken by game threads only. The menu worker posts what it changes
//!   (`STAGED`, `NAME_POSTED`, `PLAN_POST`) and the game installs it where it next reads it, so the game sees each change in
//!   the order and at the point it did when the worker wrote it under the lock; the worker reads copies (`SAMPLE`,
//!   `PROGRESS`) and keeps the menu's labels itself (`LABELS`).

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hos_lock::Mutex;

use crate::model::combo::{Combo, Helper, Label, Mark, Out, Outcome, Settings};
use crate::model::combo_plan::{Renderer, SeqView};
use crate::model::drill::{Advance, SequenceRun, SequenceWalk, Stepped};
use crate::model::frame_strip::{Tracker, Verdict};
use crate::model::frame_strip_plan::{Changed, Look, Text};
use crate::runtime::published;

/// The fighter frame's (`after_strip`, `session_clear`) and the HUD tick's; the menu worker never takes it: it stages a target
/// (`stage_target`) and reads `SAMPLE`.
static HELPER: Mutex<Helper> = Mutex::new(Helper::new());
/// What the frame hands the menu worker, a triple buffer: neither side waits (docs lane review R1 addendum, 2026-11-10 00:25Z).
/// Written under `HELPER` (one writer at a time); read by the menu worker only (`strip_sample`, `last_combo`).
static SAMPLE: hos_lock::TripleBuffer<Sample> = hos_lock::TripleBuffer::new(Sample { seen: 0, strip: None, last: None });

#[derive(Clone, Copy)]
struct Sample {
    /// The helper's count of links captured (`Helper::seen`).
    seen: u32,
    /// The strip's last combo for the recording host: none after a session clear.
    strip: Option<Combo>,
    /// *Save last combo*'s: the helper's, which it keeps across a clear.
    last: Option<Combo>,
}
static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());
/// The target's name for the bar (game threads only: the fighter frame installs a staged target's, the HUD tick reads it).
static NAME: Mutex<Text> = Mutex::new(Text::EMPTY);
/// The names the menu worker gives the bar at once (`set_target`, `stage_sequence`): published, and taken into `NAME` by the
/// game thread that next writes or reads it ([`take_posted_name`]), in the order the two kinds of write came.
static NAME_POSTED: hos_lock::Published<Text> = hos_lock::Published::new(Text::EMPTY);
/// The `NAME_POSTED` generation taken into `NAME` last (under `NAME`).
static NAME_SEEN: AtomicU32 = AtomicU32::new(0);

/// The names the menu shows, the target's and the draft's: the menu worker's own (no game thread takes this lock).
static LABELS: Mutex<Labels> = Mutex::new(Labels { target: Label::EMPTY, draft: Label::EMPTY, pending: None });

struct Labels {
    target: Label,
    draft: Label,
    /// The label of the target the worker staged last (`stage_target`) and its `STAGED` generation: the menu shows it once the
    /// fighter frame installed that generation (`STAGED_SEEN`), as when the frame wrote the label itself.
    pending: Option<(u32, Label)>,
}

impl Labels {
    /// The staged target's label, once the frame installed it.
    fn settle(&mut self) {
        if let Some((generation, label)) = self.pending {
            if STAGED_SEEN.load(Ordering::Acquire) >= generation {
                self.target = label;
                self.pending = None;
            }
        }
    }
}

/// Take a name the menu worker posted into `name` (the guard of `NAME`: one game thread at a time).
fn take_posted_name(name: &mut Text) {
    if let Some(posted) = NAME_POSTED.read_if_newer(&NAME_SEEN) {
        *name = posted;
    }
}
/// P1's fighter kind at the strip's last sample (-1 none): *Save last combo* writes its name as `recorded_on`.
static P1_KIND: AtomicI32 = AtomicI32::new(-1);

fn settings() -> Settings {
    published::live().map_or(Settings::factory(), |live| live.combo)
}

/// One frame, right after the strip's tracker stepped (game thread, inside the fighter's frame). `p1_kind` is the
/// fighter P1 is now: kept for the next save's `recorded_on` only; the helper grades whatever fighter P1 is.
pub fn after_strip(tracker: &Tracker, p1_kind: Option<i32>) {
    P1_KIND.store(p1_kind.unwrap_or(-1), Ordering::Relaxed);
    let (out, name, stepped) = {
        let mut helper = HELPER.lock();
        let name = pull_staged().map(|staged| {
            let (combo, text) = match staged {
                Some((combo, text)) => (Some(combo), text),
                None => (None, Text::EMPTY),
            };
            helper.set_target(combo);
            text
        });
        let mut plan = PLAN.lock();
        PLAN_POST.install(&mut plan);
        let (out, stepped) = match plan.as_deref() {
            Some(plan) => {
                let mut run = RUN.lock();
                let run = sequence_run(&mut run, plan, &mut helper);
                run.step(&mut helper, &plan.steps, tracker, settings())
            }
            None => {
                *RUN.lock() = None;
                (helper.step(tracker, settings()), None)
            }
        };
        let stepped = stepped.map(|s| (s, plan.as_deref().map_or(0, |p| p.generation)));
        drop(plan);
        let stepped = stepped.map(|(s, generation)| note_step(s, generation));
        let last = helper.last_combo();
        SAMPLE.write(Sample { seen: helper.seen(), strip: last, last });
        (out, name, stepped)
    };
    if let Some(text) = name {
        let mut name = NAME.lock();
        // A name the worker posted before this install came first, as when it wrote `NAME` itself.
        take_posted_name(&mut name);
        *name = text;
    }
    report(out);
    if let Some((stepped, streak, best)) = stepped {
        report_step(stepped, streak, best);
    }
}

/// The run of the staged sequence: a new plan (another sequence, or its combos changed) starts over at combo 1 from
/// the streak and best its file had.
fn sequence_run<'a>(run: &'a mut Option<(u32, SequenceRun)>, plan: &SequenceStage, helper: &mut Helper) -> &'a mut SequenceRun {
    if run.as_ref().is_none_or(|(generation, _)| *generation != plan.generation) {
        let fresh = SequenceRun::new(SequenceWalk::resume(plan.steps.len(), plan.streak, plan.best));
        fresh.start(helper, &plan.steps);
        *run = Some((plan.generation, fresh));
    }
    match run {
        Some((_, run)) => run,
        None => unreachable!("set above"),
    }
}

/// The strip's session clear (leaving Training, a hot swap, a lobby change).
pub fn session_clear() {
    let (out, stepped) = {
        let mut helper = HELPER.lock();
        let mut plan = PLAN.lock();
        PLAN_POST.install(&mut plan);
        let (out, stepped) = match plan.as_deref() {
            Some(plan) => {
                let mut run = RUN.lock();
                let run = sequence_run(&mut run, plan, &mut helper);
                let (out, stepped) = run.clear(&mut helper, &plan.steps);
                (out, stepped.map(|s| (s, plan.generation)))
            }
            None => (helper.clear(), None),
        };
        let stepped = stepped.map(|(s, generation)| note_step(s, generation));
        SAMPLE.write(Sample { seen: helper.seen(), strip: None, last: helper.last_combo() });
        (out, stepped)
    };
    report(out);
    if let Some((stepped, streak, best)) = stepped {
        report_step(stepped, streak, best);
    }
}

/// Under `HELPER` (so `PROGRESS` has one writer at a time): the streak and best after a graded step of the drilled sequence, and
/// for a finished run or a drop the progress the worker saves.
fn note_step(stepped: Stepped, generation: u32) -> (Stepped, u32, u32) {
    let (streak, best) = RUN.lock().map_or((0, 0), |(_, run)| (run.walk().streak(), run.walk().best()));
    if matches!(stepped.advance, Advance::Finished | Advance::Dropped) {
        PROGRESS.write(Progress { generation, streak, best });
    }
    (stepped, streak, best)
}

/// A step of the drilled sequence was graded: its trace point (the streak and best after it, from `note_step`).
fn report_step(stepped: Stepped, walk_streak: u32, walk_best: u32) {
    let advance = match stepped.advance {
        Advance::Next(_) => 1,
        Advance::Finished => 2,
        Advance::Dropped => 3,
        Advance::Idle => 0,
    };
    tr_seq_step_v1(stepped.step as u32, advance, walk_streak, walk_best);
}

/// A finished run or a drop changed the streak: the generation of the plan it belongs to, the streak and best.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub generation: u32,
    pub streak: u32,
    pub best: u32,
}

/// The game hands the menu worker the latest progress (written under `HELPER`, read by the worker only): neither side waits.
static PROGRESS: hos_lock::TripleBuffer<Progress> = hos_lock::TripleBuffer::new(Progress { generation: 0, streak: 0, best: 0 });

/// The worker saves the drilled sequence's streak and best to its file (`rr_host::save_sequence_progress`): the latest progress
/// since its last take, as the `Option` this was (a newer one replaced an untaken one there too).
pub fn take_progress() -> Option<Progress> {
    let (progress, fresh) = PROGRESS.read();
    fresh.then_some(progress)
}

/// A drilled sequence staged for the frame hook: the combos, their names, and the streak and best its file had. The
/// worker builds and drops it (`PLAN_POST`); game threads only read it under `PLAN`.
pub struct SequenceStage {
    pub generation: u32,
    pub name: Text,
    pub steps: Vec<Combo>,
    pub names: Vec<Text>,
    pub streak: u32,
    pub best: u32,
}

/// The staged plan as game threads see it (game threads only). Each one that takes it installs a plan the worker posted first
/// (`PLAN_POST.install`), so it sees the plan the worker staged last, as when the worker replaced it under this lock.
static PLAN: Mutex<Option<Box<SequenceStage>>> = Mutex::new(None);
/// The worker's staged plans on their way to `PLAN`, and the replaced ones on their way back: the worker allocates and drops
/// them, a game thread never does.
static PLAN_POST: crate::model::box_handoff::BoxHandoff<SequenceStage> = crate::model::box_handoff::BoxHandoff::new();
/// The frame hook's run of the staged plan, by the plan's generation.
static RUN: Mutex<Option<(u32, SequenceRun)>> = Mutex::new(None);
static PLAN_GENERATION: AtomicU32 = AtomicU32::new(0);

/// The worker, each pass: drop the plans the game replaced (a post does it too).
pub fn reap_plans() {
    PLAN_POST.reap();
    let stalls = PLAN_POST.take_stalls();
    if stalls != 0 {
        crate::runtime::diag_log::diagnostic(format_args!("[Combo] plan installs deferred={stalls} (both retire slots full)"));
    }
}

/// The worker stages a drilled sequence (or none). `same_steps` keeps the run going (a rename, a saved streak);
/// otherwise the next frame starts the new plan at combo 1. Returns the plan's generation. The old plan comes back to the
/// worker and is dropped here (`PLAN_POST`).
pub fn stage_sequence(plan: Option<(&crate::engine::rr_host::SequencePlan, bool)>) -> u32 {
    let next = plan.map(|(plan, same_steps)| {
        let generation = if same_steps {
            PLAN_GENERATION.load(Ordering::Relaxed)
        } else {
            PLAN_GENERATION.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
        };
        Box::new(SequenceStage {
            generation,
            name: Text::new(&plan.name),
            steps: plan.steps.iter().map(|step| step.combo).collect(),
            names: plan.steps.iter().map(|step| Text::new(&step.name)).collect(),
            streak: plan.streak,
            best: plan.best,
        })
    });
    let generation = next.as_ref().map_or(0, |stage| stage.generation);
    if let Some(stage) = next.as_deref() {
        tr_seq_plan_v1(stage.generation, stage.steps.len() as u32, stage.streak, stage.best);
        NAME_POSTED.publish(stage.names.first().copied().unwrap_or(Text::EMPTY));
        let mut labels = LABELS.lock();
        labels.settle();
        labels.target = Label::new(&plan.map_or(String::new(), |(plan, _)| plan.name.clone()));
    }
    PLAN_POST.post(next);
    generation
}

/// The bar's view of the drilled sequence, copied under the locks (the HUD tick; nothing allocates). `None` when the fighter
/// frame holds `PLAN`: the HUD tick runs on the game's task workers and never waits for it.
fn seq_view() -> Option<Option<SeqView>> {
    let mut plan = PLAN.try_lock()?;
    PLAN_POST.install(&mut plan);
    let Some(plan) = plan.as_deref() else { return Some(None) };
    let run = RUN.lock().filter(|(generation, _)| *generation == plan.generation).map(|(_, run)| run);
    let walk = run.map_or(SequenceWalk::resume(plan.steps.len(), plan.streak, plan.best), |run| run.walk());
    let steps = plan.steps.len();
    let at = walk.cursor();
    let first = SeqView::first_chip(at, steps);
    let mut chips = [Text::EMPTY; crate::model::panes::cb::CHIPS];
    for (c, chip) in chips.iter_mut().enumerate() {
        *chip = plan.names.get(first + c).copied().unwrap_or(Text::EMPTY);
    }
    let last = run.and_then(|run| run.last());
    Some(Some(SeqView {
        name: plan.name,
        at,
        steps,
        streak: walk.streak(),
        best: walk.best(),
        first,
        chips,
        last,
        last_name: last.and_then(|s| plan.names.get(s.step).copied()).unwrap_or(Text::EMPTY),
    }))
}

/// A drilled sequence's step was graded (`report_step`): the step (0-based), what it did (1 next, 2 finished, 3
/// dropped), and the streak and best after it.
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_seq_step_v1(step: u32, advance: u32, streak: u32, best: u32) {
    core::hint::black_box((0x5345_5153_u32, step, advance, streak, best));
}

/// The worker staged a drilled sequence: the plan's generation, its combos, and the streak and best from its file.
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_seq_plan_v1(generation: u32, steps: u32, streak: u32, best: u32) {
    core::hint::black_box((0x504c_414e_u32, generation, steps, streak, best));
}

/// The worker saved the drilled sequence's streak and best to its file (`saved` 1) or could not (0).
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_seq_saved_v1(saved: u32, streak: u32, best: u32, _reserved: u32) {
    core::hint::black_box((0x5356_5351_u32, saved, streak, best));
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

/// *Save last combo* (worker): the open exchange when it has a link, else the one before, as of the last frame.
pub fn last_combo() -> Option<Combo> {
    SAMPLE.read().0.last
}

/// The last combo published by the strip and the count of links captured (worker).
pub fn strip_sample() -> (u32, Option<Combo>) {
    let (sample, _) = SAMPLE.read();
    (sample.seen, sample.strip)
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

/// A new target (worker): staged, so the helper starts over on it at the next frame, and the menu and the bar show its name at
/// once (the bar's through `NAME_POSTED`, taken by the next game thread that reads `NAME`).
pub fn set_target(target: Option<(Combo, &str)>) {
    stage_target(target);
    let name = target.map_or("", |(_, name)| name);
    NAME_POSTED.publish(Text::new(name));
    LABELS.lock().target = Label::new(name);
}

/// A drill the menu worker staged for the frame hook. The hook holds `HELPER`; the worker must not. `hos_lock::Published`: the
/// worker publishes (it may wait for a reader to leave the other slot), the hook copies out what is newer than it last saw and
/// never waits (docs lane review R1, finding 5: the hand-rolled seqlock here lacked its fences).
static STAGED: hos_lock::Published<Option<(Combo, Text)>> = hos_lock::Published::new(None);
/// The `STAGED` generation the frame hook installed last (0: nothing staged yet). The worker reads it to show a staged target's
/// label once it is installed (`Labels::settle`).
static STAGED_SEEN: AtomicU32 = AtomicU32::new(0);

/// Publish a grade target without taking `HELPER` (the menu worker: the drill, `ui::worker`, and the combo panel's target,
/// `set_target`). The next `after_strip` installs it, and the menu's target label becomes its name then.
pub fn stage_target(target: Option<(Combo, &str)>) {
    let name = target.map_or("", |(_, name)| name);
    let mut labels = LABELS.lock();
    labels.settle();
    STAGED.publish(target.map(|(combo, name)| (combo, Text::new(name))));
    labels.pending = Some((STAGED.generation(), Label::new(name)));
}

/// `Some` when a staged target is ready: `Some(None)` clears, `Some(Some(_))` installs.
fn pull_staged() -> Option<Option<(Combo, Text)>> {
    STAGED.read_if_newer(&STAGED_SEEN)
}

/// The draft name the next save uses (worker).
pub fn set_draft(draft: &str) {
    LABELS.lock().draft = Label::new(draft);
}

/// The menu's labels: the target's name and the draft (`model::menu::Env`; the worker).
pub fn labels() -> (Label, Label) {
    let mut labels = LABELS.lock();
    labels.settle();
    (labels.target, labels.draft)
}

/// The plugin's layout was registered again: every pane is written again.
pub fn on_layout_registered() {
    RENDERER.lock().invalidate();
}

/// One HUD tick: the bar's writes, shown with the strip while the helper is On and has a target. It runs on the UI pass, on the
/// game's task workers (`anchors::ui_pass_calculate`), while the fighter frame may hold `HELPER`, `NAME` or `PLAN`: it takes
/// them with `try_lock`, and on contention writes nothing this pass (the bar keeps its panes; the next pass draws). No plugin
/// thread takes them (the menu worker posts: `NAME_POSTED`, `PLAN_POST`); this tick takes what it posted first.
pub fn hud_tick(strip_shown: bool, apply: impl FnMut(usize, &Look, Changed)) -> usize {
    let Some(helper) = HELPER.try_lock().map(|helper| *helper) else { return 0 };
    let Some(name) = NAME.try_lock().map(|mut name| {
        take_posted_name(&mut name);
        *name
    }) else {
        return 0;
    };
    let Some(seq) = seq_view() else { return 0 };
    RENDERER.lock().draw(&helper, &name, seq.as_ref(), strip_shown, apply)
}
