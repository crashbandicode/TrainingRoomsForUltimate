//! Counted off-hook reload requests. Primary decision 2026-11-03: preserve
//! one request per held-chord P1 poll; file reads run on the existing worker.
use std::sync::atomic::{AtomicU64, Ordering};

const ENABLED: u64 = 1 << 63;
const COUNT: u64 = u32::MAX as u64;
const EPOCH: u64 = !(ENABLED | COUNT);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket(u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub requested: u64,
    pub completed: u64,
    pub pending: u64,
    pub dropped: u64,
}

/// The production limit is the counter's representable bound, not a silently
/// coalesced dirty bit. A small test limit exercises the same overflow path.
pub struct Queue<const LIMIT: u32> {
    state: AtomicU64,
    requested: AtomicU64,
    completed: AtomicU64,
    dropped: AtomicU64,
}

impl<const LIMIT: u32> Queue<LIMIT> {
    pub const fn new() -> Self {
        Self { state: AtomicU64::new(0), requested: AtomicU64::new(0),
            completed: AtomicU64::new(0), dropped: AtomicU64::new(0) }
    }

    pub fn enabled(&self) -> bool { self.state.load(Ordering::Acquire) & ENABLED != 0 }

    pub fn set_enabled(&self, enabled: bool) {
        let result = self.state.fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
            if (old & ENABLED != 0) == enabled { return None; }
            let epoch = (old & EPOCH).wrapping_add(1 << 32) & EPOCH;
            Some(epoch | if enabled { ENABLED } else { 0 })
        });
        if let Ok(old) = result { self.dropped.fetch_add(old & COUNT, Ordering::Relaxed); }
    }

    /// No heap, lock, file or wake operation. Returns whether a request was counted.
    pub fn request(&self) -> bool {
        match self.state.fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
            (old & ENABLED != 0 && (old & COUNT) < u64::from(LIMIT)).then(|| old + 1)
        }) {
            Ok(_) => { self.requested.fetch_add(1, Ordering::Relaxed); true }
            Err(old) if old & ENABLED != 0 => {
                self.requested.fetch_add(1, Ordering::Relaxed);
                self.dropped.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(_) => false,
        }
    }

    pub fn take(&self) -> Option<Ticket> {
        self.state.fetch_update(Ordering::AcqRel, Ordering::Acquire, |old| {
            (old & ENABLED != 0 && old & COUNT != 0).then(|| old - 1)
        }).ok().map(|old| Ticket(old & (ENABLED | EPOCH)))
    }

    pub fn current(&self, ticket: Ticket) -> bool {
        self.state.load(Ordering::Acquire) & (ENABLED | EPOCH) == ticket.0
    }

    /// Finish an attempted read (including a reported I/O error); a cancelled
    /// in-flight ticket contributes to dropped, never a later enabled epoch.
    pub fn finish(&self, ticket: Ticket) {
        if self.current(ticket) { self.completed.fetch_add(1, Ordering::Relaxed); }
        else { self.dropped.fetch_add(1, Ordering::Relaxed); }
    }

    /// Counts are sampled while producers may run; exact accounting is asserted
    /// at quiescence. The worker adds its one in-flight read to pending.
    pub fn counts(&self, in_flight: bool) -> Counts {
        Counts { requested: self.requested.load(Ordering::Acquire), completed: self.completed.load(Ordering::Acquire),
            pending: (self.state.load(Ordering::Acquire) & COUNT) + u64::from(in_flight),
            dropped: self.dropped.load(Ordering::Acquire) }
    }
}

impl<const LIMIT: u32> Default for Queue<LIMIT> {
    fn default() -> Self { Self::new() }
}

/// Held raw L+R+A, not an edge-triggered menu chord. Extra buttons are harmless.
pub fn qualifies(slot: i64, training: bool, buttons: u32) -> bool {
    use super::chords::bit;
    const CHORD: u32 = bit::L | bit::R | bit::A;
    slot == 0 && training && buttons & CHORD == CHORD
}

// File work is invoked only by the runtime menu worker; input hooks use Queue.
const CHUNK_BYTES: usize = 4096;
pub const OPERATIONS_PER_TURN: usize = 8;

#[derive(Default)]
pub struct Reader {
    current: Option<(Ticket, std::fs::File)>,
    pub errors: u64,
    pub bytes: u64,
}

impl Reader {
    pub fn drain<const LIMIT: u32>(&mut self, queue: &Queue<LIMIT>, path: &str) -> Counts {
        use std::io::Read;
        let mut bytes = [0u8; CHUNK_BYTES];
        for _ in 0..OPERATIONS_PER_TURN {
            if self.current.as_ref().is_some_and(|(ticket, _)| !queue.current(*ticket)) {
                let (ticket, _) = self.current.take().unwrap();
                queue.finish(ticket);
            }
            if let Some((ticket, file)) = self.current.as_mut() {
                let result = file.read(&mut bytes);
                match result {
                    Ok(0) => {
                        let ticket = *ticket;
                        self.current = None;
                        queue.finish(ticket);
                    }
                    Ok(n) => self.bytes = self.bytes.saturating_add(n as u64),
                    Err(_) => {
                        let ticket = *ticket;
                        self.current = None;
                        self.errors += 1;
                        queue.finish(ticket);
                    }
                }
            } else if let Some(ticket) = queue.take() {
                if !queue.current(ticket) { queue.finish(ticket); continue; }
                match std::fs::File::open(path) {
                    Ok(file) => self.current = Some((ticket, file)),
                    Err(_) => { self.errors += 1; queue.finish(ticket); }
                }
            } else { break; }
        }
        // The config struct has no fields: consuming the file has no gameplay effect.
        // No file is created, rewritten or truncated, and no file-size cap is imposed.
        queue.counts(self.current.is_some())

    }
}
