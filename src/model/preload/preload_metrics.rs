//! Sparse measurements for the configurable fighter/hat preload candidate.
//!
//! Memory is kernel process charge from `svcGetInfo` on the current process, the ABI
//! reviewed in console-memory82/API-REVIEW.md and used by practice_trace. `committed`
//! (id 7) is legacy heap high-water + MapPhysicalMemory pages + code + main stack:
//! committed process pages, NOT allocator live bytes or bytes retained by a load. The
//! game's allocators reuse charged pages, so a zero delta proves nothing and a positive
//! delta is only a lower bound on new page charge. No game heap metric is proven.
//!
//! Time is the Horizon system tick (`cntpct_el0`/`cntfrq_el0`). Frames are opaque
//! callback counts supplied by the caller. Capture never allocates, locks, or logs;
//! only `snapshot` and `OperationStamp::finish` write, one bounded diagnostic line each.
use std::convert::TryFrom;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use hos_lock::Mutex;

/// Result stored when this build cannot issue the SVC (host value tests).
pub const UNSUPPORTED: u32 = u32::MAX;
/// Frame-interval bucket upper bounds; the last bucket holds everything above 50 ms.
const GAP_BOUNDS_US: [u64; 5] = [17_500, 20_000, 25_000, 34_000, 50_000];

#[cfg(target_arch = "aarch64")]
mod native {
    /// Horizon system tick, the libnx `armGetSystemTick` source.
    #[inline]
    pub fn ticks() -> u64 {
        let tick: u64;
        unsafe {
            core::arch::asm!("mrs {tick}, cntpct_el0", tick = out(reg) tick, options(nomem, nostack));
        }
        tick
    }

    #[inline]
    pub fn tick_hz() -> u64 {
        let hz: u64;
        unsafe {
            core::arch::asm!("mrs {hz}, cntfrq_el0", hz = out(reg) hz, options(nomem, nostack));
        }
        hz
    }

    pub fn get_info(id: u64) -> (u32, u64) {
        let result: u64;
        let value: u64;
        // practice_trace::process_memory shape: x1=id, x2=current-process pseudo-handle,
        // x3=subtype 0; x0=Result, x1=value. Register-only: no user memory or stack.
        unsafe {
            core::arch::asm!("svc 0x29", inout("x0") 0u64 => result,
                inout("x1") id => value, in("x2") 0xffff_8001u64, in("x3") 0u64,
                clobber_abi("C"), options(nostack));
        }
        (result as u32, value)
    }
}

/// Host builds cannot read Horizon counters; every value is explicitly unsupported.
#[cfg(not(target_arch = "aarch64"))]
mod native {
    pub fn ticks() -> u64 {
        0
    }
    pub fn tick_hz() -> u64 {
        0
    }
    pub fn get_info(_id: u64) -> (u32, u64) {
        (super::UNSUPPORTED, 0)
    }
}

pub(crate) use native::{tick_hz, ticks};

/// Whole microseconds, or `None` without a known tick frequency.
pub fn micros(ticks: u64, hz: u64) -> Option<u64> {
    (hz != 0)
        .then(|| u64::try_from(u128::from(ticks) * 1_000_000 / u128::from(hz)).unwrap_or(u64::MAX))
}

/// One GetInfo result/value pair as returned; the value is 0 unless `result` is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counter {
    pub result: u32,
    pub value: u64,
}

impl Counter {
    #[cfg(test)]
    const MISSING: Self = Self {
        result: UNSUPPORTED,
        value: 0,
    };

    #[cfg(test)]
    const fn ok(value: u64) -> Self {
        Self { result: 0, value }
    }

    pub fn get(self) -> Option<u64> {
        (self.result == 0).then_some(self.value)
    }

    fn read(id: u64) -> Self {
        let (result, value) = native::get_info(id);
        Self {
            result,
            value: if result == 0 { value } else { 0 },
        }
    }
}

fn delta(before: Counter, after: Counter) -> Option<i64> {
    i64::try_from(i128::from(after.get()?) - i128::from(before.get()?)).ok()
}

/// Process charge at one instant. Sequential reads, not atomic: a concurrent mapping can
/// make one sample inconsistent, so values are kept exactly as returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemorySample {
    /// System tick read immediately before the counters.
    pub tick: u64,
    /// Id 6: this process's physical budget, not console DRAM.
    pub budget: Counter,
    /// Id 7: committed process charge. Not allocator usage; see the module docs.
    pub committed: Counter,
    /// Ids 16/17: secure page-table slab budget/charge, or 0/0 on the default resource.
    /// Ids 21/22 are omitted: they differ from 6/7 only by that constant slab budget.
    pub slab_budget: Counter,
    pub slab_committed: Counter,
}

impl MemorySample {
    /// Four register-only SVCs and one counter read. Never allocates, locks, or logs.
    pub fn capture() -> Self {
        let tick = ticks();
        Self {
            tick,
            budget: Counter::read(6),
            committed: Counter::read(7),
            slab_budget: Counter::read(16),
            slab_committed: Counter::read(17),
        }
    }

    /// Uncharged process budget, only when both reads succeeded and are consistent.
    pub fn headroom(&self) -> Option<u64> {
        self.budget.get()?.checked_sub(self.committed.get()?)
    }

    /// Change in committed process charge since `earlier`; not bytes retained by a load.
    pub fn committed_since(&self, earlier: &Self) -> Option<i64> {
        delta(earlier.committed, self.committed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Fighter,
    Hat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Speculative install into a helper: submitted to parked ready.
    Background,
    /// Accepted user request: input accepted to visible/usable completion.
    Request,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ready,
    Failed,
    /// A new request or setting superseded an unfinished request; not a load failure.
    Abandoned,
    /// The scene ended first.
    Departed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    /// Scene settled, before any speculative install. Sets the baseline; clears totals.
    Baseline,
    /// Every wanted speculative group is resident and nothing is in flight.
    Settled,
    /// Preload settings changed. Pass the new configuration; the line reports totals for
    /// the previous condition, which are then cleared.
    Settings,
    /// Settled after preload was disabled or its speculative groups retired.
    Retired,
    /// Scene departure. Reports, then clears the baseline and totals.
    SceneEnd,
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::Fighter => "fighter",
            Self::Hat => "hat",
        }
    }
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Request => "request",
        }
    }
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
            Self::Departed => "departed",
        }
    }
}

impl Label {
    fn name(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Settled => "settled",
            Self::Settings => "settings",
            Self::Retired => "retired",
            Self::SceneEnd => "scene_end",
        }
    }
}

/// The caller's preload configuration and residency when a value is recorded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preload {
    /// Requested speculative groups; 0 is Off. Mode capacity is reported separately.
    pub slots: u8,
    /// Requested count bounded by this backend's compiled helper capacity.
    pub effective: u8,
    pub fighters: bool,
    pub hats: bool,
    /// Speculative groups resident now.
    pub resident: u8,
    /// Bit n: native entry n holds a speculative group.
    pub entries: u8,
    /// A speculative install is in flight.
    pub busy: bool,
    /// Resident speculative kinds, oldest first.
    pub kinds: [Option<u32>; crate::engine::preload::cycle_preload_capacity::MAX_TARGETS],
}

/// Optional per-frame observations while an operation is open. Fixed size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Frames {
    /// `OperationStamp::frame` calls.
    pub observed: u32,
    /// Observed frames overlapping a busy speculative install. An upper bound
    /// on contention, not proof that the speculative load caused a stall.
    pub blocked: u32,
    /// Largest tick interval between consecutive observed frames.
    pub max_gap: u64,
    /// Interval counts: <=17.5, <=20, <=25, <=34, <=50, >50 ms.
    pub gaps: [u32; 6],
    last: u64,
}

impl Frames {
    fn note(&mut self, tick: u64, hz: u64, blocked: bool) {
        self.observed = self.observed.saturating_add(1);
        self.blocked = self.blocked.saturating_add(u32::from(blocked));
        if self.observed > 1 {
            let gap = tick.saturating_sub(self.last);
            self.max_gap = self.max_gap.max(gap);
            if let Some(us) = micros(gap, hz) {
                let bucket = GAP_BOUNDS_US
                    .iter()
                    .position(|bound| us <= *bound)
                    .unwrap_or(GAP_BOUNDS_US.len());
                self.gaps[bucket] = self.gaps[bucket].saturating_add(1);
            }
        }
        self.last = tick;
    }
}

/// An open operation, held by the caller. `Copy`, fixed size, no allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationStamp {
    pub operation: Operation,
    pub target: Target,
    pub kind: u32,
    /// Request only: the target source was already resident when accepted (a hit).
    pub resident: bool,
    /// Caller frame count at begin.
    pub frame: u32,
    pub hz: u64,
    pub before: MemorySample,
    pub frames: Frames,
}

impl OperationStamp {
    /// `None` when disabled, with no SVC or counter read. Background: when the install is
    /// submitted. Request: when the input is accepted, before any wait for the install gate.
    pub fn begin(
        enabled: bool,
        operation: Operation,
        target: Target,
        kind: u32,
        resident: bool,
        frame: u32,
    ) -> Option<Self> {
        enabled.then(|| Self {
            operation,
            target,
            kind,
            resident,
            frame,
            hz: tick_hz(),
            before: MemorySample::capture(),
            frames: Frames::default(),
        })
    }

    /// Optional, at most once per game frame while open: one counter read, no I/O.
    /// `blocked`: a speculative install overlaps the request. Intervals include
    /// pauses in the caller's callback; overlap does not establish causation.
    pub fn frame(&mut self, blocked: bool) {
        self.frames.note(ticks(), self.hz, blocked);
    }

    /// At the completion event only: captures the after sample, folds the record into the
    /// session totals and writes one diagnostic line.
    pub fn finish(self, frame: u32, outcome: Outcome, preload: Preload) -> Record {
        let record = self.record(MemorySample::capture(), frame, outcome, preload);
        let (scene, baseline) = {
            let mut session = SESSION.lock();
            session.absorb(&record);
            (session.scene, session.baseline)
        };
        legend();
        crate::engine::compat::logging::diagnostic(format_args!(
            "{}",
            RecordLine {
                record: &record,
                scene,
                baseline
            }
        ));
        record
    }

    fn record(self, after: MemorySample, frame: u32, outcome: Outcome, preload: Preload) -> Record {
        Record {
            operation: self.operation,
            target: self.target,
            kind: self.kind,
            resident: self.resident,
            outcome,
            frames: frame.wrapping_sub(self.frame),
            ticks: after.tick.saturating_sub(self.before.tick),
            hz: self.hz,
            before: self.before,
            after,
            observed: self.frames,
            preload,
        }
    }
}

/// One finished operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub operation: Operation,
    pub target: Target,
    pub kind: u32,
    pub resident: bool,
    pub outcome: Outcome,
    /// Caller frame-count difference, wrapping.
    pub frames: u32,
    /// System ticks from begin to finish.
    pub ticks: u64,
    pub hz: u64,
    pub before: MemorySample,
    pub after: MemorySample,
    pub observed: Frames,
    pub preload: Preload,
}

impl Record {
    pub fn micros(&self) -> Option<u64> {
        micros(self.ticks, self.hz)
    }

    pub fn committed_delta(&self) -> Option<i64> {
        self.after.committed_since(&self.before)
    }
}

/// Completion latency in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Latency {
    count: u32,
    sum: u64,
    max: u64,
}

impl Latency {
    const NONE: Self = Self {
        count: 0,
        sum: 0,
        max: 0,
    };

    fn add(&mut self, ticks: u64) {
        self.count = self.count.saturating_add(1);
        self.sum = self.sum.saturating_add(ticks);
        self.max = self.max.max(ticks);
    }
}

/// Totals since the last `Baseline` or `Settings` snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Totals {
    background: Latency,
    /// Failed or departed speculative installs.
    background_failed: u32,
    background_max_gap: u64,
    background_gaps: [u32; 6],
    /// Ready requests whose source was resident when accepted.
    hit: Latency,
    /// Ready requests that needed an install.
    miss: Latency,
    request_failed: u32,
    request_max_gap: u64,
    blocked_frames: u32,
    blocked_requests: u32,
    peak_committed: Option<u64>,
    min_headroom: Option<u64>,
}

impl Totals {
    const EMPTY: Self = Self {
        background: Latency::NONE,
        background_failed: 0,
        background_max_gap: 0,
        background_gaps: [0; 6],
        hit: Latency::NONE,
        miss: Latency::NONE,
        request_failed: 0,
        request_max_gap: 0,
        blocked_frames: 0,
        blocked_requests: 0,
        peak_committed: None,
        min_headroom: None,
    };

    fn observe(&mut self, sample: &MemorySample) {
        self.peak_committed = self.peak_committed.max(sample.committed.get());
        if let Some(headroom) = sample.headroom() {
            self.min_headroom = Some(self.min_headroom.map_or(headroom, |min| min.min(headroom)));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Session {
    scene: u32,
    baseline: Option<MemorySample>,
    condition: Option<Preload>,
    totals: Totals,
}

static SESSION: Mutex<Session> = Mutex::new(Session::NEW);
static LEGEND: AtomicBool = AtomicBool::new(false);

/// What one snapshot line reports, after its label's transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Report {
    label: Label,
    scene: u32,
    sample: MemorySample,
    baseline: Option<MemorySample>,
    /// The configuration the totals belong to.
    condition: Option<Preload>,
    totals: Totals,
    preload: Preload,
}

impl Session {
    const NEW: Self = Self {
        scene: 0,
        baseline: None,
        condition: None,
        totals: Totals::EMPTY,
    };

    fn absorb(&mut self, record: &Record) {
        let totals = &mut self.totals;
        totals.observe(&record.before);
        totals.observe(&record.after);
        let frames = &record.observed;
        match record.operation {
            Operation::Background => {
                match record.outcome {
                    Outcome::Ready => totals.background.add(record.ticks),
                    _ => totals.background_failed = totals.background_failed.saturating_add(1),
                }
                totals.background_max_gap = totals.background_max_gap.max(frames.max_gap);
                for (total, count) in totals.background_gaps.iter_mut().zip(frames.gaps) {
                    *total = total.saturating_add(count);
                }
            }
            Operation::Request => {
                match (record.outcome, record.resident) {
                    (Outcome::Ready, true) => totals.hit.add(record.ticks),
                    (Outcome::Ready, false) => totals.miss.add(record.ticks),
                    _ => totals.request_failed = totals.request_failed.saturating_add(1),
                }
                totals.request_max_gap = totals.request_max_gap.max(frames.max_gap);
                if frames.blocked > 0 {
                    totals.blocked_requests = totals.blocked_requests.saturating_add(1);
                    totals.blocked_frames = totals.blocked_frames.saturating_add(frames.blocked);
                }
            }
        }
    }

    fn snapshot(&mut self, label: Label, sample: MemorySample, preload: Preload) -> Report {
        if label == Label::Baseline {
            self.scene = self.scene.wrapping_add(1);
            self.baseline = Some(sample);
            self.condition = Some(preload);
            self.totals = Totals::EMPTY;
        }
        self.totals.observe(&sample);
        let report = Report {
            label,
            scene: self.scene,
            sample,
            baseline: self.baseline,
            condition: self.condition,
            totals: self.totals,
            preload,
        };
        match label {
            Label::Settings => {
                self.condition = Some(preload);
                self.totals = Totals::EMPTY;
                self.totals.observe(&sample);
            }
            Label::SceneEnd => {
                *self = Self {
                    scene: self.scene,
                    ..Self::NEW
                }
            }
            Label::Baseline | Label::Settled | Label::Retired => {}
        }
        report
    }
}

/// One memory sample and one bounded diagnostic line, at the sparse events named by
/// `Label`; never per frame. No-op when disabled.
pub fn snapshot(enabled: bool, label: Label, frame: u32, preload: Preload) {
    if !enabled {
        return;
    }
    let sample = MemorySample::capture();
    let mut session = SESSION.lock();
    let report = session.snapshot(label, sample, preload);
    drop(session);
    crate::engine::compat::logging::heap_profile(label.name());
    legend();
    crate::engine::compat::logging::diagnostic(format_args!(
        "{}",
        SnapshotLine {
            report: &report,
            frame,
            hz: tick_hz()
        }
    ));
}

const LEGEND_TEXT: &str = "[PreloadMetrics] v1 legend committed=svcGetInfo id7 process charge \
(legacy heap high-water + MapPhysicalMemory + code + main stack), NOT allocator live or retained-asset \
bytes; delta>0 is a lower bound on new page charge and 0 proves nothing; headroom=id6-id7 of this \
process's budget, not console DRAM; slab=id17/id16; us=cntpct_el0/cntfrq_el0; frames=caller callback \
counts; gaps=frame intervals <=17.5,20,25,34,50,>50 ms";

fn legend() {
    if !LEGEND.swap(true, Ordering::Relaxed) {
        crate::engine::compat::logging::diagnostic(format_args!("{LEGEND_TEXT}"));
    }
}

struct Value(Counter);
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.result {
            0 => write!(f, "{}", self.0.value),
            UNSUPPORTED => f.write_str("na"),
            result => write!(f, "e{result:#x}"),
        }
    }
}

struct Maybe<T>(Option<T>);
impl<T: fmt::Display> fmt::Display for Maybe<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(value) => write!(f, "{value}"),
            None => f.write_str("na"),
        }
    }
}

struct Signed(Option<i64>);
impl fmt::Display for Signed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(value) => write!(f, "{value:+}"),
            None => f.write_str("na"),
        }
    }
}

struct Gaps([u32; 6]);
impl fmt::Display for Gaps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d, e, g] = self.0;
        write!(f, "{a},{b},{c},{d},{e},{g}")
    }
}

struct Kind(Option<u32>);
impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(kind) => write!(f, "{kind:#x}"),
            None => f.write_str("-"),
        }
    }
}

/// `slots:apply`, the condition a set of totals belongs to.
struct Condition(Option<Preload>);
impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(preload) => write!(f, "{}:{}", preload.slots, preload.apply()),
            None => f.write_str("na"),
        }
    }
}

impl Preload {
    fn apply(&self) -> &'static str {
        match (self.fighters, self.hats) {
            (true, true) => "fighter+hat",
            (true, false) => "fighter",
            (false, true) => "hat",
            (false, false) => "none",
        }
    }
}

impl fmt::Display for Preload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "slots={} apply={} resident={} entries={:#04x} busy={} effective={} kinds=",
            self.slots,
            self.apply(),
            self.resident,
            self.entries,
            u8::from(self.busy),
            self.effective
        )?;
        for (index, kind) in self.kinds.iter().enumerate() {
            if index != 0 { f.write_str(",")?; }
            write!(f, "{}", Kind(*kind))?;
        }
        Ok(())
    }
}

fn mean(latency: Latency, hz: u64) -> Option<u64> {
    latency
        .sum
        .checked_div(u64::from(latency.count))
        .and_then(|ticks| micros(ticks, hz))
}

fn gap(max: u64, hz: u64) -> Option<u64> {
    if max == 0 {
        None
    } else {
        micros(max, hz)
    }
}

struct RecordLine<'a> {
    record: &'a Record,
    scene: u32,
    baseline: Option<MemorySample>,
}

impl fmt::Display for RecordLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = self.record;
        let hit = match (r.operation, r.resident) {
            (Operation::Background, _) => "-",
            (Operation::Request, true) => "1",
            (Operation::Request, false) => "0",
        };
        write!(
            f,
            "[PreloadMetrics] v1 record scene={} op={} target={} kind={:#x} hit={hit} \
            outcome={} frames={} us={} start_tick={} observed={} blocked={} max_gap_us={} gaps={} \
            committed={}->{} delta={} base_delta={} headroom={} slab={}->{}/{} {}",
            self.scene,
            r.operation.name(),
            r.target.name(),
            r.kind,
            r.outcome.name(),
            r.frames,
            Maybe(r.micros()),
            r.before.tick,
            r.observed.observed,
            r.observed.blocked,
            Maybe(gap(r.observed.max_gap, r.hz)),
            Gaps(r.observed.gaps),
            Value(r.before.committed),
            Value(r.after.committed),
            Signed(r.committed_delta()),
            Signed(
                self.baseline
                    .and_then(|base| r.after.committed_since(&base))
            ),
            Maybe(r.after.headroom()),
            Value(r.before.slab_committed),
            Value(r.after.slab_committed),
            Value(r.after.slab_budget),
            r.preload
        )
    }
}

struct SnapshotLine<'a> {
    report: &'a Report,
    frame: u32,
    hz: u64,
}

impl fmt::Display for SnapshotLine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, hz) = (self.report, self.hz);
        let t = &r.totals;
        write!(f, "[PreloadMetrics] v1 snapshot label={} scene={} frame={} tick={} committed={} \
            base_delta={} headroom={} budget={} slab={}/{} {} condition={} bg_ready={} \
            bg_us_mean={} bg_us_max={} bg_failed={} bg_max_gap_us={} bg_gaps={} hit={} \
            hit_us_mean={} hit_us_max={} miss={} miss_us_mean={} miss_us_max={} req_unfinished={} \
            blocked_requests={} blocked_frames={} req_max_gap_us={} peak_committed={} min_headroom={}",
            r.label.name(), r.scene, self.frame, r.sample.tick, Value(r.sample.committed),
            Signed(r.baseline.and_then(|base| r.sample.committed_since(&base))),
            Maybe(r.sample.headroom()), Value(r.sample.budget), Value(r.sample.slab_committed),
            Value(r.sample.slab_budget), r.preload, Condition(r.condition), t.background.count,
            Maybe(mean(t.background, hz)), Maybe(micros(t.background.max, hz).filter(|_| t.background.count > 0)),
            t.background_failed, Maybe(gap(t.background_max_gap, hz)), Gaps(t.background_gaps),
            t.hit.count, Maybe(mean(t.hit, hz)), Maybe(micros(t.hit.max, hz).filter(|_| t.hit.count > 0)),
            t.miss.count, Maybe(mean(t.miss, hz)), Maybe(micros(t.miss.max, hz).filter(|_| t.miss.count > 0)),
            t.request_failed, t.blocked_requests, t.blocked_frames, Maybe(gap(t.request_max_gap, hz)),
            Maybe(t.peak_committed), Maybe(t.min_headroom))
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const HZ: u64 = 1_000_000;

    fn sample(tick: u64, committed: u64) -> MemorySample {
        MemorySample {
            tick,
            budget: Counter::ok(4_000),
            committed: Counter::ok(committed),
            slab_budget: Counter::ok(100),
            slab_committed: Counter::ok(10),
        }
    }

    fn stamp(operation: Operation, resident: bool, tick: u64, committed: u64) -> OperationStamp {
        OperationStamp {
            operation,
            target: Target::Fighter,
            kind: 0x5a,
            resident,
            frame: 10,
            hz: HZ,
            before: sample(tick, committed),
            frames: Frames::default(),
        }
    }

    fn micros_use_the_tick_frequency_and_refuse_an_unknown_frequency() {
        assert_eq!(micros(19_200_000, 19_200_000), Some(1_000_000));
        assert_eq!(micros(320_000, 19_200_000), Some(16_666));
        assert_eq!(micros(5, 0), None);
        assert_eq!(micros(u64::MAX, 1), Some(u64::MAX));
    }

    fn failed_or_unsupported_counters_never_produce_deltas_or_headroom() {
        let good = sample(0, 1_000);
        let mut failed = sample(1, 1_100);
        failed.committed = Counter {
            result: 0xe401,
            value: 0,
        };
        assert_eq!(failed.committed_since(&good), None);
        assert_eq!(good.committed_since(&failed), None);
        assert_eq!(failed.headroom(), None);
        let mut missing = good;
        missing.budget = Counter::MISSING;
        assert_eq!(missing.headroom(), None);
        assert_eq!(sample(1, 900).committed_since(&good), Some(-100));
        assert_eq!(good.headroom(), Some(3_000));
    }

    fn headroom_refuses_an_internally_inconsistent_sample() {
        assert_eq!(sample(0, 4_001).headroom(), None);
        assert_eq!(sample(0, 4_000).headroom(), Some(0));
    }

    #[cfg(not(target_arch = "aarch64"))]
    #[test]
    fn host_capture_is_explicitly_unsupported() {
        let host = MemorySample::capture();
        assert_eq!(host.committed, Counter::MISSING);
        assert_eq!(host.headroom(), None);
        assert_eq!(tick_hz(), 0);
        assert_eq!(
            OperationStamp::begin(false, Operation::Request, Target::Hat, 0, true, 0),
            None
        );
    }

    fn frame_intervals_bucket_by_wall_time_and_count_blocked_frames() {
        let mut frames = Frames::default();
        for (tick, blocked) in [
            (1_000, true),
            (17_667, true),
            (35_667, false),
            (35_667 + 17_500, false),
            (35_667 + 17_500 + 60_000, false),
        ] {
            frames.note(tick, HZ, blocked);
        }
        assert_eq!(frames.observed, 5);
        assert_eq!(frames.blocked, 2);
        assert_eq!(frames.max_gap, 60_000);
        assert_eq!(frames.gaps, [2, 1, 0, 0, 0, 1]);
        let mut unknown = Frames::default();
        unknown.note(0, 0, false);
        unknown.note(9, 0, false);
        assert_eq!((unknown.max_gap, unknown.gaps), (9, [0; 6]));
    }

    fn record_measures_wall_ticks_and_wrapping_caller_frames() {
        let mut open = stamp(Operation::Request, false, 5_000, 1_000);
        open.frame = u32::MAX - 1;
        let record = open.record(
            sample(455_000, 1_300),
            3,
            Outcome::Ready,
            Preload::default(),
        );
        assert_eq!(record.frames, 5);
        assert_eq!(record.micros(), Some(450_000));
        assert_eq!(record.committed_delta(), Some(300));
    }

    fn session_separates_hits_misses_background_and_blocked_requests() {
        let mut session = Session::NEW;
        session.snapshot(Label::Baseline, sample(0, 1_000), Preload::default());
        let mut background = stamp(Operation::Background, false, 0, 1_000);
        background.frames.note(0, HZ, false);
        background.frames.note(40_000, HZ, false);
        session.absorb(&background.record(
            sample(600_000, 1_500),
            50,
            Outcome::Ready,
            Preload::default(),
        ));
        let hit = stamp(Operation::Request, true, 0, 1_500);
        session.absorb(&hit.record(
            sample(80_000, 1_500),
            15,
            Outcome::Ready,
            Preload::default(),
        ));
        let mut miss = stamp(Operation::Request, false, 0, 1_500);
        miss.frames.note(0, HZ, true);
        miss.frames.note(16_000, HZ, true);
        session.absorb(&miss.record(
            sample(700_000, 1_600),
            52,
            Outcome::Ready,
            Preload::default(),
        ));
        let failed = stamp(Operation::Background, false, 0, 1_600);
        session.absorb(&failed.record(
            sample(10, 1_600),
            11,
            Outcome::Departed,
            Preload::default(),
        ));
        let t = session.totals;
        assert_eq!(
            (t.background.count, t.background.max, t.background_failed),
            (1, 600_000, 1)
        );
        assert_eq!(
            (t.background_max_gap, t.background_gaps),
            (40_000, [0, 0, 0, 0, 1, 0])
        );
        assert_eq!((t.hit.count, t.hit.max), (1, 80_000));
        assert_eq!((t.miss.count, t.miss.max), (1, 700_000));
        assert_eq!((t.blocked_requests, t.blocked_frames), (1, 2));
        assert_eq!(
            (t.peak_committed, t.min_headroom),
            (Some(1_600), Some(2_400))
        );
    }

    fn settings_report_the_previous_condition_then_clear_its_totals() {
        let one = Preload {
            slots: 1,
            fighters: true,
            ..Preload::default()
        };
        let two = Preload {
            slots: 2,
            fighters: true,
            hats: true,
            ..Preload::default()
        };
        let mut session = Session::NEW;
        session.snapshot(Label::Baseline, sample(0, 1_000), one);
        let hit = stamp(Operation::Request, true, 0, 1_000);
        session.absorb(&hit.record(sample(9, 1_000), 11, Outcome::Ready, one));
        let report = session.snapshot(Label::Settings, sample(10, 1_200), two);
        assert_eq!(
            (report.condition, report.preload, report.totals.hit.count),
            (Some(one), two, 1)
        );
        assert_eq!(
            report.sample.committed_since(&report.baseline.unwrap()),
            Some(200)
        );
        assert_eq!(
            (session.condition, session.totals.hit.count),
            (Some(two), 0)
        );
        assert_eq!(session.totals.peak_committed, Some(1_200));
        let end = session.snapshot(Label::SceneEnd, sample(20, 1_100), Preload::default());
        assert_eq!(
            (end.scene, end.baseline.map(|base| base.tick)),
            (1, Some(0))
        );
        assert_eq!(
            (session.scene, session.baseline, session.condition),
            (1, None, None)
        );
        assert_eq!(
            session
                .snapshot(Label::Baseline, sample(30, 1_050), one)
                .scene,
            2
        );
    }

    fn lines_label_committed_charge_and_mark_missing_values() {
        let preload = Preload {
            slots: 1,
            fighters: true,
            hats: true,
            resident: 1,
            entries: 1 << 3,
            kinds: [Some(0x5a), None, None, None, None],
            ..Preload::default()
        };
        let mut open = stamp(Operation::Request, true, 0, 1_000);
        open.before.slab_committed = Counter {
            result: 0xe401,
            value: 0,
        };
        let record = open.record(sample(1_000_000, 1_100), 70, Outcome::Ready, preload);
        let line = RecordLine {
            record: &record,
            scene: 3,
            baseline: Some(sample(0, 900)),
        }
        .to_string();
        for expected in [
            "v1 record scene=3 op=request target=fighter kind=0x5a hit=1 outcome=ready",
            "frames=60 us=1000000",
            "max_gap_us=na",
            "committed=1000->1100 delta=+100",
            "base_delta=+200 headroom=2900 slab=e0xe401->10/100",
            "slots=1 apply=fighter+hat resident=1 entries=0x08 busy=0 effective=0 kinds=0x5a,-",
        ] {
            assert!(
                line.contains(expected),
                "{} missing from {}",
                expected,
                line
            );
        }
        let mut session = Session::NEW;
        let mut unsupported = sample(5, 0);
        unsupported.committed = Counter::MISSING;
        let report = session.snapshot(Label::Retired, unsupported, Preload::default());
        let line = SnapshotLine {
            report: &report,
            frame: 9,
            hz: 0,
        }
        .to_string();
        for expected in [
            "label=retired scene=0 frame=9 tick=5 committed=na base_delta=na headroom=na",
            "condition=na bg_ready=0 bg_us_mean=na bg_us_max=na",
            "peak_committed=na",
        ] {
            assert!(
                line.contains(expected),
                "{} missing from {}",
                expected,
                line
            );
        }
        assert!(!line.contains('\n'));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("micros_use_the_tick_frequency_and_refuse_an_unknown_frequency", micros_use_the_tick_frequency_and_refuse_an_unknown_frequency as fn()),
            ("failed_or_unsupported_counters_never_produce_deltas_or_headroom", failed_or_unsupported_counters_never_produce_deltas_or_headroom as fn()),
            ("headroom_refuses_an_internally_inconsistent_sample", headroom_refuses_an_internally_inconsistent_sample as fn()),
            ("frame_intervals_bucket_by_wall_time_and_count_blocked_frames", frame_intervals_bucket_by_wall_time_and_count_blocked_frames as fn()),
            ("record_measures_wall_ticks_and_wrapping_caller_frames", record_measures_wall_ticks_and_wrapping_caller_frames as fn()),
            ("session_separates_hits_misses_background_and_blocked_requests", session_separates_hits_misses_background_and_blocked_requests as fn()),
            ("settings_report_the_previous_condition_then_clear_its_totals", settings_report_the_previous_condition_then_clear_its_totals as fn()),
            ("lines_label_committed_charge_and_mark_missing_values", lines_label_committed_charge_and_mark_missing_values as fn()),
        ]);
    }
    }
}
