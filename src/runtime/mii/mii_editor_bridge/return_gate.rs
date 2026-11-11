//! A native editor interruption first reconstructs practice to deliver the
//! queued opponent. That immediate return must not reopen the interrupted UI.
//! Latch the native practice's match receipt, then wait for ready practice.
//! A receiving practice never clears its match flag; only a fresh one can be
//! ready. The outer Quickplay scene need not change state during this cycle.

#[derive(Clone, Copy)]
struct Armed {
    generation: u64,
    delivered: bool,
    returned: bool,
    attempted: bool,
}

pub(super) struct ReturnGate { armed: Option<Armed> }

impl ReturnGate {
    pub(super) const fn new() -> Self { Self { armed: None } }

    pub(super) fn arm(&mut self, generation: u64) {
        self.armed = Some(Armed { generation, delivered: false, returned: false, attempted: false });
    }

    pub(super) fn observe(&mut self, generation: u64, match_received: bool, practice_ready: bool) {
        let Some(armed) = self.armed.as_mut().filter(|armed| armed.generation == generation) else { return; };
        if match_received {
            // A later match also permits one new attempt on a future
            // return after a failed restore; never retry each ready frame.
            armed.delivered = true;
            armed.returned = false;
            armed.attempted = false;
        } else if practice_ready && armed.delivered { armed.returned = true; }
    }

    pub(super) fn ready(&self, generation: u64) -> bool {
        self.armed.as_ref().is_some_and(|armed| armed.generation == generation
            && armed.returned && !armed.attempted)
    }

    pub(super) fn attempted(&mut self, generation: u64) {
        if let Some(armed) = self.armed.as_mut().filter(|armed| armed.generation == generation) {
            armed.attempted = true;
        }
    }

    /// No editor was requested. A transient UI/lock refusal may try again
    /// when the caller's readiness checks succeed.
    pub(super) fn request_refused(&mut self, generation: u64) {
        if let Some(armed) = self.armed.as_mut().filter(|armed| armed.generation == generation) {
            armed.attempted = false;
        }
    }

    pub(super) fn end(&mut self, generation: u64) {
        if self.armed.as_ref().is_some_and(|armed| armed.generation == generation) { self.armed = None; }
    }
}

