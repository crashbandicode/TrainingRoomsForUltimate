//! Diagnostics of the group build's game-thread time (stream f6, 2026-11-02; ledger F6, the first-boot group-build hitch).
//!
//! The carried group build (`runtime::roster::native_body_bank`, Phase B′) reports `setup_us`: the game-thread span from the exchange's
//! start to the end of the entry-level native setup. The first build of a boot paid 75-107 ms of it in Eden, later builds 10-16 ms
//! (W20 / M5q rows). This model splits that span, and the queueing after it, into named steps (each native call, the plugin's own code
//! between them, the log lines) and keeps, per role, the largest gaps between consecutive entry ticks of a swap together with the work
//! the plugin did in the tick before each gap. Pure: the runtime passes the clock (`now_us`); nothing here reads the game.
//!
//! Only the `diag` build records anything (the runtime's release shim is empty); the line formats are fixed-size and never allocate.
use std::fmt;

/// One named part of the build's game-thread time. `Plugin` is the plugin's own code between native calls (accumulated).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Step {
    /// The plugin's own code between the native calls below (lock-free reads, model logic, field writes).
    Plugin = 0,
    /// `0x17f26c0` unprepare, the copy-registry cells of the outgoing window (U18), every call.
    Unprepare,
    /// `0x17e4f50` roster refresh of `R` (U18).
    Roster,
    /// `0x17f1a60` prepare_entry, registration of the attaching window (U18; warm attach only).
    Prepare,
    /// Diagnostic lines formatted on the game thread (`[Registry]`, `[GroupSwap] build ... queued`).
    Log,
    /// The Mii group's Phase B setup (`0x67e7d0`/`0x67ead0`; Mii groups only, on the game thread before either build mode).
    MiiSetup,
    /// `MemorySample::capture`: four `svcGetInfo` reads for the build's memory charge.
    Memory,
    /// `exchange_articles` (PT/Aegis saved article retire, warm activation rebuild).
    Articles,
    /// `0x654dc0`: the entry's dispatcher arrays.
    Models,
    /// `0x655320`: the control contexts from the kinds table.
    Contexts,
    /// `0x64f820`: AI params, the `+0x4178` select, the pad route.
    Control,
    /// `0x3ae5f0` per kind: pool pop and base constructor.
    Allocate,
    /// `0x392e290` per kind: the 0x90-byte task object.
    TaskAlloc,
    /// `0x3cb050` per kind plus the emptied local functor's destructor.
    Functor,
    /// `0x3549720` per kind: the queued construction task.
    Schedule,
    /// Settlement: the bodies into the cells (`install_cells`).
    Install,
    /// Settlement: the settled task objects freed (`free_tasks`).
    FreeTasks,
    /// Settlement: `r_handle::end_request` and its line.
    Request,
    /// Settlement: verification, the `built` line and the acknowledgement under the bank lock.
    Verify,
    /// Settlement: post-build release, ledger reconcile, rollback (when any).
    Ledger,
}

pub const STEPS: usize = 20;

impl Step {
    pub const ALL: [Step; STEPS] = [
        Step::Plugin, Step::Unprepare, Step::Roster, Step::Prepare, Step::Log, Step::MiiSetup, Step::Memory, Step::Articles,
        Step::Models, Step::Contexts, Step::Control, Step::Allocate, Step::TaskAlloc, Step::Functor, Step::Schedule,
        Step::Install, Step::FreeTasks, Step::Request, Step::Verify, Step::Ledger,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Step::Plugin => "plugin",
            Step::Unprepare => "unprepare_17f26c0",
            Step::Roster => "roster_17e4f50",
            Step::Prepare => "prepare_17f1a60",
            Step::Log => "log",
            Step::MiiSetup => "mii_setup",
            Step::Memory => "memory_svc",
            Step::Articles => "articles",
            Step::Models => "models_654dc0",
            Step::Contexts => "contexts_655320",
            Step::Control => "control_64f820",
            Step::Allocate => "allocate_3ae5f0",
            Step::TaskAlloc => "task_392e290",
            Step::Functor => "functor_3cb050",
            Step::Schedule => "schedule_3549720",
            Step::Install => "install_cells",
            Step::FreeTasks => "free_tasks",
            Step::Request => "end_request",
            Step::Verify => "verify_ack",
            Step::Ledger => "ledger",
        }
    }
}

/// The time of one span split by step: `mark(step, now)` charges the time since the previous mark (or the start) to `step`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Split {
    start_us: u64,
    last_us: u64,
    us: [u64; STEPS],
    calls: [u16; STEPS],
    running: bool,
}

impl Split {
    /// Not running, nothing recorded (`Default`, usable in a `static`).
    pub const IDLE: Split = Split { start_us: 0, last_us: 0, us: [0; STEPS], calls: [0; STEPS], running: false };

    pub fn start(now_us: u64) -> Self {
        Self { start_us: now_us, last_us: now_us, running: true, ..Self::default() }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn mark(&mut self, step: Step, now_us: u64) {
        if !self.running {
            return;
        }
        let i = step as usize;
        self.us[i] = self.us[i].saturating_add(now_us.saturating_sub(self.last_us));
        self.calls[i] = self.calls[i].saturating_add(1);
        self.last_us = now_us.max(self.last_us);
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn us(&self, step: Step) -> u64 {
        self.us[step as usize]
    }

    pub fn calls(&self, step: Step) -> u16 {
        self.calls[step as usize]
    }

    /// Start to the last mark.
    pub fn total_us(&self) -> u64 {
        self.last_us.saturating_sub(self.start_us)
    }

    /// The steps the carried `setup_us` covers (start to the end of `0x64f820`).
    pub fn setup_us(&self) -> u64 {
        Step::ALL.iter().take_while(|s| **s != Step::Allocate).map(|s| self.us(*s)).sum()
    }

    /// The largest step: where the span went.
    pub fn largest(&self) -> Step {
        let mut best = Step::Plugin;
        for s in Step::ALL {
            if self.us(s) > self.us(best) {
                best = s;
            }
        }
        best
    }
}

/// `name=us/calls` for every step that ran, then `total_us=` and `largest=`.
impl fmt::Display for Split {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for s in Step::ALL {
            if self.calls(s) != 0 {
                write!(f, "{}={}/{} ", s.name(), self.us(s), self.calls(s))?;
            }
        }
        write!(f, "setup_us={} total_us={} largest={}", self.setup_us(), self.total_us(), self.largest().name())
    }
}

/// What the plugin did in the tick before a gap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Work {
    /// Nothing of the build's (a poll of pending tasks, or no construction at all).
    #[default]
    Poll,
    /// The exchange: setup and, for Phase B′, the queueing (or the whole synchronous build).
    Setup,
    /// The settlement: cells, verification, acknowledgement.
    Settle,
    /// Native state 10 after the control init settled on the worker: the bodies allocated and their construction queued.
    Queue,
}

impl Work {
    pub fn name(self) -> &'static str {
        match self {
            Work::Poll => "poll",
            Work::Setup => "setup",
            Work::Settle => "settle",
            Work::Queue => "queue",
        }
    }
}

/// One gap between two consecutive entry ticks: the index of the tick that ended it, its length, the work of the tick before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gap {
    pub tick: u32,
    pub us: u64,
    pub after: Work,
}

pub const TOP_GAPS: usize = 4;

/// The largest gaps of one swap, attributed to the work of the tick before each (armed at the request, read at Ready).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gaps {
    last_us: u64,
    ticks: u32,
    work: Work,
    top: [Gap; TOP_GAPS],
    armed: bool,
}

impl Gaps {
    /// Not armed (`Default`, usable in a `static`).
    pub const IDLE: Gaps = Gaps { last_us: 0, ticks: 0, work: Work::Poll, top: [Gap { tick: 0, us: 0, after: Work::Poll }; TOP_GAPS], armed: false };

    pub fn arm(now_us: u64) -> Self {
        Self { last_us: now_us, armed: true, ..Self::default() }
    }

    pub fn armed(&self) -> bool {
        self.armed
    }

    /// The work this tick does (the last call in a tick wins: settle after poll).
    pub fn note(&mut self, work: Work) {
        if self.armed {
            self.work = work;
        }
    }

    /// One entry tick: the gap since the previous one is charged to the previous tick's work.
    pub fn observe(&mut self, now_us: u64) {
        if !self.armed {
            return;
        }
        self.ticks = self.ticks.saturating_add(1);
        let gap = Gap { tick: self.ticks, us: now_us.saturating_sub(self.last_us), after: self.work };
        self.last_us = now_us;
        self.work = Work::Poll;
        // Insert keeping `top` sorted by length, largest first; equal lengths keep the earlier tick first.
        if let Some(at) = self.top.iter().position(|g| gap.us > g.us) {
            self.top.copy_within(at..TOP_GAPS - 1, at + 1);
            self.top[at] = gap;
        }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn top(&self) -> &[Gap] {
        let n = self.top.iter().take_while(|g| g.tick != 0).count();
        &self.top[..n]
    }

    /// The largest gap after a tick that did `work`, if any is among the top gaps.
    pub fn largest_after(&self, work: Work) -> Option<Gap> {
        self.top().iter().copied().find(|g| g.after == work)
    }
}

/// `ticks=<n> top=[t<tick>:<us>:<work>, ...]`.
impl fmt::Display for Gaps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ticks={} top=[", self.ticks)?;
        for (i, g) in self.top().iter().enumerate() {
            if i != 0 {
                f.write_str(", ")?;
            }
            write!(f, "t{}:{}:{}", g.tick, g.us, g.after.name())?;
        }
        f.write_str("]")
    }
}

/// A gap this far above the previous one is a stall (diag): about four displayed frames at 60 Hz.
/// A steady run of slow frames, each about as long as the last, is not a stall.
pub const STALL_US: u64 = 60_000;

/// Every entry tick of one role, whatever the plugin does (diag): the gap since the role's previous tick.
/// Reported when it exceeds the threshold and jumps by more than that threshold past the previous gap.
/// The baseline for a stall the game makes itself (a native Reload building a fighter) next to the plugin's swaps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickClock {
    last_us: u64,
    last_gap: u64,
}

impl TickClock {
    pub const IDLE: TickClock = TickClock { last_us: 0, last_gap: 0 };

    pub fn tick(&mut self, now_us: u64, threshold_us: u64) -> Option<u64> {
        let last = std::mem::replace(&mut self.last_us, now_us);
        let gap = now_us.saturating_sub(last);
        let previous = std::mem::replace(&mut self.last_gap, if last == 0 { 0 } else { gap });
        let jumped = gap > previous.saturating_add(threshold_us);
        (last != 0 && gap > threshold_us && jumped).then_some(gap)
    }
}

