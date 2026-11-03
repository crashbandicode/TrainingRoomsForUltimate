//! Bounded correlation for one synchronous, diagnostic Hitbox request.
use core::sync::atomic::{
    AtomicBool, AtomicUsize, Ordering::AcqRel, Ordering::Acquire, Ordering::Relaxed,
    Ordering::Release,
};

pub const CONSTRUCTOR_RETURN: usize = 0x3563764;
pub const HELPER_RETURN: usize = 0x356d2e0;
const MISSING: usize = usize::MAX;

pub struct Capture {
    owner: AtomicUsize,
    frame: AtomicUsize,
    object: AtomicUsize,
    constructors: AtomicUsize,
    helper: AtomicUsize,
    returned: AtomicUsize,
    final_phase: AtomicBool,
    ambiguous: AtomicBool,
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
        true
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
        let snapshot = Snapshot {
            object: self.object.load(Relaxed),
            frame: self.frame.load(Relaxed),
            helper: (helper != MISSING).then_some(helper as u32),
            returned: (returned != MISSING).then_some(returned as u32),
            outcome,
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
