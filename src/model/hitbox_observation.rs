//! Bounded correlation for one synchronous, diagnostic Hitbox request.
use core::sync::atomic::{
    AtomicBool, AtomicUsize, Ordering::AcqRel, Ordering::Acquire, Ordering::Relaxed,
    Ordering::Release,
};

pub const CONSTRUCTOR_RETURN: usize = 0x3563764;
pub const HELPER_RETURN: usize = 0x356d2e0;
const MISSING: usize = usize::MAX;

pub const FAILED_LIMIT: usize = 8;

pub const POOL_LIMIT: usize = 80;
pub struct PoolBudget(AtomicUsize);
impl PoolBudget {
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
    pub fn available(&self) -> bool {
        self.0.load(Relaxed) < POOL_LIMIT
    }
    pub fn reserve(&self) -> Option<usize> {
        self.0
            .fetch_update(Relaxed, Relaxed, |n| {
                (n < POOL_LIMIT).then_some(n.saturating_add(1))
            })
            .ok()
            .map(|n| n + 1)
    }
}
impl Default for PoolBudget {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PoolStatus {
    MissingHelper = 0,
    Denied = 1,
    AfterHelper = 2,
    Inconsistent = 3,
    Ambiguous = 4,
}
#[derive(Debug)]
pub struct PoolSnapshot {
    pub ordinal: usize,
    pub status: PoolStatus,
    // Identity token only: never dereferenced after the inline callback.
    pub manager: usize,
    pub total: i32,
    pub available: i32,
}

pub struct FailureBudget(AtomicUsize);
impl FailureBudget {
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
    pub fn available(&self) -> bool {
        self.0.load(Relaxed) < FAILED_LIMIT
    }
    pub fn record(&self, handle: u32) -> Option<usize> {
        if handle != 0 {
            return None;
        }
        self.0
            .fetch_update(Relaxed, Relaxed, |n| {
                (n < FAILED_LIMIT).then_some(n.saturating_add(1))
            })
            .ok()
            .map(|n| n + 1)
    }
}
impl Default for FailureBudget {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HelperReason {
    Unattributed,
    ResourceNull,
    ResourceDisabled,
    EmitterDemandRejected,
    Ambiguous,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Demand {
    pub requested: i32,
    pub available: i32,
    pub output: usize,
    pub manager: usize,
    pub sp: usize,
    pub caller: usize,
}

pub struct Capture {
    owner: AtomicUsize,
    frame: AtomicUsize,
    object: AtomicUsize,
    constructors: AtomicUsize,
    helper: AtomicUsize,
    returned: AtomicUsize,
    final_phase: AtomicBool,
    ambiguous: AtomicBool,
    demand_seen: AtomicBool,
    demand: [AtomicUsize; 6],
    registers: [AtomicUsize; 3],
    registers_seen: AtomicBool,
    reason_ambiguous: AtomicBool,
    pool_ordinal: AtomicUsize,
    pool_seen: AtomicBool,
    pool: [AtomicUsize; 4],
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Unobserved,
    Ambiguous,
    NotReturned,
    ConstructorSucceeded,
    HelperRejected,
    HandleOrEpochRejected,
    FinalPayloadRejected,
    BeforeHelper,
}

#[derive(Debug)]
pub struct Snapshot {
    pub object: usize,
    pub frame: usize,
    pub helper: Option<u32>,
    pub returned: Option<u32>,
    pub outcome: Outcome,
    pub helper_reason: HelperReason,
    pub demand: Option<Demand>,
    pub helper_registers: Option<[usize; 3]>,
    pub pool: Option<PoolSnapshot>,
}

impl Capture {
    pub const fn new() -> Self {
        Self {
            owner: AtomicUsize::new(0),
            frame: AtomicUsize::new(0),
            object: AtomicUsize::new(0),
            constructors: AtomicUsize::new(0),
            helper: AtomicUsize::new(MISSING),
            returned: AtomicUsize::new(MISSING),
            final_phase: AtomicBool::new(false),
            ambiguous: AtomicBool::new(false),
            demand_seen: AtomicBool::new(false),
            demand: [const { AtomicUsize::new(0) }; 6],
            registers: [const { AtomicUsize::new(0) }; 3],
            registers_seen: AtomicBool::new(false),
            reason_ambiguous: AtomicBool::new(false),
            pool_ordinal: AtomicUsize::new(0),
            pool_seen: AtomicBool::new(false),
            pool: [const { AtomicUsize::new(0) }; 4],
        }
    }

    pub fn begin(&self, thread: usize) -> bool {
        if thread == 0 {
            return false;
        }
        if let Err(owner) = self.owner.compare_exchange(0, thread, AcqRel, Acquire) {
            if owner == thread {
                self.ambiguous.store(true, Relaxed);
            }
            return false;
        }
        self.frame.store(0, Relaxed);
        self.object.store(0, Relaxed);
        self.constructors.store(0, Relaxed);
        self.helper.store(MISSING, Relaxed);
        self.returned.store(MISSING, Relaxed);
        self.final_phase.store(false, Relaxed);
        self.ambiguous.store(false, Relaxed);
        self.demand_seen.store(false, Relaxed);
        for value in self.demand.iter().chain(self.registers.iter()) {
            value.store(0, Relaxed);
        }
        self.registers_seen.store(false, Relaxed);
        self.reason_ambiguous.store(false, Relaxed);
        self.pool_ordinal.store(0, Relaxed);
        self.pool_seen.store(false, Relaxed);
        for word in &self.pool {
            word.store(0, Relaxed);
        }
        true
    }

    pub fn observe_pool(&self, thread: usize, ordinal: usize) {
        if self.owns(thread) && (1..=POOL_LIMIT).contains(&ordinal) {
            self.pool_ordinal.store(ordinal, Relaxed);
        }
    }

    /// Two reads at the existing helper return only. Exact native caller forces
    /// w7=0; early resource failures retain the outer object in x19 and are denied.
    /// The reader is injected so tests prove denied paths perform no reads.
    pub fn pool_after(
        &self,
        thread: usize,
        sp: usize,
        caller: usize,
        result: u32,
        manager: usize,
        mut read: impl FnMut(usize) -> u32,
    ) {
        if !self.owns(thread) || self.pool_ordinal.load(Relaxed) == 0 {
            return;
        }
        if self.pool_seen.swap(true, Relaxed) {
            self.pool[0].store(PoolStatus::Ambiguous as usize, Relaxed);
            return;
        }
        self.pool[0].store(PoolStatus::Denied as usize, Relaxed);
        let object = self.object.load(Relaxed);
        if self.constructors.load(Relaxed) != 1
            || object == 0
            || self.ambiguous.load(Relaxed)
            || self.reason_ambiguous.load(Relaxed)
            || caller != HELPER_RETURN
            || self.frame.load(Relaxed) == 0
            || sp.checked_add(0x40) != Some(self.frame.load(Relaxed))
            || self.helper.load(Relaxed) != MISSING
            || self.returned.load(Relaxed) != MISSING
            || result > 1
            || manager == 0
            || manager == object
            || !manager.is_multiple_of(4)
            || manager.checked_add(0x84).is_none()
        {
            return;
        }
        let total = read(manager + 0x70);
        let available = read(manager + 0x80);
        let status = if (total as i32) < 0 || (available as i32) < 0 || available > total {
            PoolStatus::Inconsistent
        } else {
            PoolStatus::AfterHelper
        };
        self.pool[1].store(manager, Relaxed);
        self.pool[2].store(total as usize, Relaxed);
        self.pool[3].store(available as usize, Relaxed);
        self.pool[0].store(status as usize, Relaxed);
    }

    pub fn owns(&self, thread: usize) -> bool {
        thread != 0 && self.owner.load(Acquire) == thread
    }

    pub fn constructor(&self, thread: usize, sp: usize, object: usize, caller: usize) {
        if !self.owns(thread) {
            return;
        }
        let count = self
            .constructors
            .fetch_update(Relaxed, Relaxed, |n| Some(n.saturating_add(1).min(2)))
            .unwrap_or(2);
        let Some(frame) = sp
            .checked_sub(0xf0)
            .filter(|n| *n != 0 && n.is_multiple_of(16))
        else {
            self.ambiguous.store(true, Relaxed);
            return;
        };
        if count != 0 || caller != CONSTRUCTOR_RETURN || object == 0 {
            self.ambiguous.store(true, Relaxed);
            return;
        }
        self.frame.store(frame, Relaxed);
        self.object.store(object, Relaxed);
    }

    pub fn helper_return(&self, thread: usize, sp: usize, caller: usize, result: u32) {
        if !self.owns(thread)
            || caller != HELPER_RETURN
            || sp.checked_add(0x40) != Some(self.frame.load(Relaxed))
        {
            return;
        }
        if self.helper.swap(result as usize, Relaxed) != MISSING {
            self.ambiguous.store(true, Relaxed);
        }
    }

    // Called at a50d8 BEFORE the displaced ldp restores the caller's x19.
    pub fn helper_registers(&self, thread: usize, sp: usize, caller: usize, regs: [usize; 3]) {
        if !self.owns(thread)
            || caller != HELPER_RETURN
            || sp.checked_add(0x40) != Some(self.frame.load(Relaxed))
        {
            return;
        }
        if self.registers_seen.swap(true, Relaxed) {
            self.reason_ambiguous.store(true, Relaxed);
        }
        for (slot, value) in self.registers.iter().zip(regs) {
            slot.store(value, Relaxed);
        }
    }

    /// Positive signed demand rejection at a5020, before any displaced store.
    pub fn demand_rejected(&self, thread: usize, sample: Demand) {
        if !self.owns(thread) {
            return;
        }
        if self.constructors.load(Relaxed) != 1
            || self.object.load(Relaxed) == 0
            || self.object.load(Relaxed).checked_add(0x1d0) != Some(sample.output)
            || sample.manager == 0
            || sample.manager == self.object.load(Relaxed)
            || sample.sp == 0
            || sample.sp.checked_add(0x40) != Some(self.frame.load(Relaxed))
            || sample.caller != HELPER_RETURN
            || sample.requested <= sample.available
            || self.helper.load(Relaxed) != MISSING
            || self.returned.load(Relaxed) != MISSING
        {
            self.reason_ambiguous.store(true, Relaxed);
            return;
        }
        if self.demand_seen.swap(true, Relaxed) {
            self.reason_ambiguous.store(true, Relaxed);
        }
        for (slot, value) in self.demand.iter().zip([
            sample.requested as u32 as usize,
            sample.available as u32 as usize,
            sample.output,
            sample.manager,
            sample.sp,
            sample.caller,
        ]) {
            slot.store(value, Relaxed);
        }
    }

    pub fn final_phase(&self, thread: usize, sp: usize, object: usize) {
        if self.owns(thread)
            && sp != 0
            && sp == self.frame.load(Relaxed)
            && object == self.object.load(Relaxed)
        {
            self.final_phase.store(true, Relaxed);
        }
    }

    pub fn constructor_return(&self, thread: usize, sp: usize, result: u32) {
        if self.owns(thread)
            && sp != 0
            && sp == self.frame.load(Relaxed)
            && self.returned.swap(result as usize, Relaxed) != MISSING
        {
            self.ambiguous.store(true, Relaxed);
        }
    }

    pub fn finish(&self, thread: usize) -> Option<Snapshot> {
        if !self.owns(thread) {
            return None;
        }
        let count = self.constructors.load(Relaxed);
        let helper = self.helper.load(Relaxed);
        let returned = self.returned.load(Relaxed);
        let contradictory = self.final_phase.load(Relaxed) && helper != MISSING && helper & 1 == 0;
        let outcome = if self.ambiguous.load(Relaxed) || count > 1 || contradictory {
            Outcome::Ambiguous
        } else if count == 0 {
            Outcome::Unobserved
        } else if returned == MISSING {
            Outcome::NotReturned
        } else if returned & 1 != 0 {
            Outcome::ConstructorSucceeded
        } else if self.final_phase.load(Relaxed) {
            Outcome::FinalPayloadRejected
        } else if helper == MISSING {
            Outcome::BeforeHelper
        } else if helper & 1 == 0 {
            Outcome::HelperRejected
        } else {
            Outcome::HandleOrEpochRejected
        };
        let regs = self
            .registers_seen
            .load(Relaxed)
            .then(|| core::array::from_fn(|i| self.registers[i].load(Relaxed)));
        let demand = self.demand_seen.load(Relaxed).then(|| Demand {
            requested: self.demand[0].load(Relaxed) as u32 as i32,
            available: self.demand[1].load(Relaxed) as u32 as i32,
            output: self.demand[2].load(Relaxed),
            manager: self.demand[3].load(Relaxed),
            sp: self.demand[4].load(Relaxed),
            caller: self.demand[5].load(Relaxed),
        });
        let helper_reason = if self.reason_ambiguous.load(Relaxed) || outcome == Outcome::Ambiguous
        {
            HelperReason::Ambiguous
        } else if outcome != Outcome::HelperRejected {
            if demand.is_some() {
                HelperReason::Ambiguous
            } else {
                HelperReason::Unattributed
            }
        } else if let Some([resource, ready, x19]) = regs {
            if x19 == self.object.load(Relaxed) {
                if demand.is_some() {
                    HelperReason::Ambiguous
                } else if resource == 0 {
                    HelperReason::ResourceNull
                } else if ready as u32 == 0 {
                    HelperReason::ResourceDisabled
                } else {
                    HelperReason::Ambiguous
                }
            } else if x19 == 0 {
                HelperReason::Ambiguous
            } else {
                match demand {
                    Some(sample) if sample.manager == x19 => HelperReason::EmitterDemandRejected,
                    Some(_) => HelperReason::Ambiguous,
                    None => HelperReason::Unattributed,
                }
            }
        } else {
            HelperReason::Unattributed
        };

        let pool = (self.pool_ordinal.load(Relaxed) != 0).then(|| {
            let status = if outcome == Outcome::Ambiguous {
                PoolStatus::Ambiguous
            } else {
                match self.pool[0].load(Relaxed) {
                    0 => PoolStatus::MissingHelper,
                    1 => PoolStatus::Denied,
                    2 => PoolStatus::AfterHelper,
                    3 => PoolStatus::Inconsistent,
                    _ => PoolStatus::Ambiguous,
                }
            };
            PoolSnapshot {
                ordinal: self.pool_ordinal.load(Relaxed),
                status,
                manager: self.pool[1].load(Relaxed),
                total: self.pool[2].load(Relaxed) as u32 as i32,
                available: self.pool[3].load(Relaxed) as u32 as i32,
            }
        });
        let snapshot = Snapshot {
            object: self.object.load(Relaxed),
            frame: self.frame.load(Relaxed),
            helper: (helper != MISSING).then_some(helper as u32),
            returned: (returned != MISSING).then_some(returned as u32),
            outcome,
            helper_reason,
            demand,
            helper_registers: regs,
            pool,
        };
        self.owner.store(0, Release);
        Some(snapshot)
    }
}

impl Default for Capture {
    fn default() -> Self {
        Self::new()
    }
}
