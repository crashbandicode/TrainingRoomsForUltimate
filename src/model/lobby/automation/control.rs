//! Lock-free handoff from the overlay to the native game thread. The complete
//! request and its status share one word, so a late callback cannot complete or
//! cancel a newer request. No callback performs file I/O or calls native code.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const SAVE: u64 = 1;
const REMATCH: u64 = 6;
const ARMED: u64 = 8;
const STATUS: u64 = 0xf0;
const GENERATION: u64 = !0xff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Decision {
    Manual = 0,
    Yes = 1,
    No = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Progress {
    Idle = 0,
    Armed,
    Saving,
    Rematch,
    Done,
    Cancelled,
    Failed,
    Unavailable,
}

impl Progress {
    fn decode(word: u64) -> Self {
        match (word & STATUS) >> 4 {
            0 => Self::Idle,
            1 => Self::Armed,
            2 => Self::Saving,
            3 => Self::Rematch,
            4 => Self::Done,
            5 => Self::Cancelled,
            6 => Self::Failed,
            _ => Self::Unavailable,
        }
    }
    fn terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled | Self::Failed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub generation: u64,
    pub save_replay: bool,
    pub rematch: Decision,
}

pub struct Controls {
    supported: AtomicBool,
    word: AtomicU64,
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}

impl Controls {
    pub const fn new() -> Self {
        Self {
            supported: AtomicBool::new(false),
            word: AtomicU64::new(0),
        }
    }

    /// Called only after the game build and native hook sites were checked.
    pub fn enable(&self) {
        self.supported.store(true, Ordering::Release);
    }

    pub fn available(&self) -> bool {
        self.supported.load(Ordering::Acquire)
    }

    pub fn current(&self, option: usize) -> Option<usize> {
        let word = self.word.load(Ordering::Acquire);
        match option {
            0 => Some((word & SAVE) as usize),
            1 => Some(((word & REMATCH) >> 1) as usize),
            2 => Some(usize::from(word & ARMED != 0)),
            3 => Some(if self.available() {
                Progress::decode(word)
            } else {
                Progress::Unavailable
            } as usize),
            _ => None,
        }
    }

    /// Snapshot settings at Arm. Editing them while armed is rejected rather
    /// than changing the choice for a match already being handled.
    pub fn edit(&self, option: usize, choice: usize) -> bool {
        if !self.available() {
            return false;
        }
        self.word
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| {
                let next_generation = (word & GENERATION).wrapping_add(0x100) & GENERATION;
                match (option, choice) {
                    (0, 0..=1) if word & ARMED == 0 => Some((word & !SAVE) | choice as u64),
                    (1, 0..=2) if word & ARMED == 0 => {
                        Some((word & !REMATCH) | ((choice as u64) << 1))
                    }
                    (2, 1) if word & ARMED == 0 => Some(
                        next_generation
                            | (word & (SAVE | REMATCH))
                            | ARMED
                            | ((Progress::Armed as u64) << 4),
                    ),
                    (2, 0) if word & ARMED != 0 => Some(
                        next_generation
                            | (word & (SAVE | REMATCH))
                            | ((Progress::Cancelled as u64) << 4),
                    ),
                    (2, 0) => Some(word),
                    _ => None,
                }
            })
            .is_ok()
    }

    pub fn pending(&self) -> Option<Request> {
        let word = self.word.load(Ordering::Acquire);
        if word & ARMED == 0 {
            return None;
        }
        Some(Request {
            generation: word & GENERATION,
            save_replay: word & SAVE != 0,
            rematch: match (word & REMATCH) >> 1 {
                1 => Decision::Yes,
                2 => Decision::No,
                _ => Decision::Manual,
            },
        })
    }

    pub fn owns(&self, generation: u64) -> bool {
        let word = self.word.load(Ordering::Acquire);
        word & ARMED != 0 && word & GENERATION == generation
    }

    /// Publishing completion consumes this arm. A stale request can never
    /// overwrite the next arm or turn a user cancellation into success.
    pub fn publish(&self, generation: u64, progress: Progress) -> bool {
        self.word
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |word| {
                if word & ARMED == 0 || word & GENERATION != generation {
                    return None;
                }
                let armed = if progress.terminal() { 0 } else { ARMED };
                Some((word & !(STATUS | ARMED)) | armed | ((progress as u64) << 4))
            })
            .is_ok()
    }
}

