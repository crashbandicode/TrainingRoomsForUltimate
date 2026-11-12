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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::ReturnGate;

    fn observed_match_return_resumes_without_an_outer_transition() {
        let mut gate = ReturnGate::new();
        gate.arm(7);
        // Actual private diagnostic sequence: waiting rebuild, match receipt,
        // then post-match practice. Outer state stayed 8 throughout.
        let flags = [0xb1_u8, 0xb5, 0xbd, 0xb9, 0xbe, 0xb0, 0xb1, 0xbd];
        for (index, flags) in flags.iter().copied().enumerate() {
            gate.observe(7, flags & 2 != 0, flags & 1 != 0);
            assert_eq!(gate.ready(7), index >= 6);
        }
    }

    fn failed_reconstruction_does_not_retry_on_every_frame() {
        let mut gate = ReturnGate::new();
        gate.arm(7);
        gate.observe(7, true, false);
        gate.observe(7, false, true);
        gate.attempted(7);
        for _ in 0..60 { gate.observe(7, false, true); }
        assert!(!gate.ready(7));
        gate.observe(7, true, false);
        gate.observe(7, false, true);
        assert!(gate.ready(7));
    }

    fn old_lobby_callbacks_cannot_resume_or_retire_a_new_lobby() {
        let mut gate = ReturnGate::new();
        gate.arm(8);
        gate.observe(7, true, false);
        gate.observe(7, false, true);
        gate.end(7);
        assert!(!gate.ready(7));
        assert!(!gate.ready(8));
        gate.observe(8, true, false);
        gate.observe(8, false, true);
        assert!(gate.ready(8));
        gate.end(8);
        assert!(!gate.ready(8));
    }

    fn request_refused_before_editor_start_does_not_burn_return() {
        let mut gate = ReturnGate::new();
        gate.arm(7);
        gate.observe(7, true, false);
        gate.observe(7, false, true);
        gate.attempted(7);
        gate.request_refused(6);
        assert!(!gate.ready(7));
        gate.request_refused(7);
        assert!(gate.ready(7));
        gate.attempted(7);
        assert!(!gate.ready(7)); // an accepted request still gets only one attempt
    }

    fn a_new_checkpoint_requires_its_own_match_receipt() {
        let mut gate = ReturnGate::new();
        gate.arm(7);
        gate.observe(7, true, false);
        gate.observe(7, false, true);
        assert!(gate.ready(7));
        gate.arm(7);
        for _ in 0..60 { gate.observe(7, false, true); }
        assert!(!gate.ready(7));
        gate.observe(7, true, false);
        assert!(!gate.ready(7));
        gate.observe(7, false, true);
        assert!(gate.ready(7));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("observed_match_return_resumes_without_an_outer_transition", observed_match_return_resumes_without_an_outer_transition as fn()),
            ("failed_reconstruction_does_not_retry_on_every_frame", failed_reconstruction_does_not_retry_on_every_frame as fn()),
            ("old_lobby_callbacks_cannot_resume_or_retire_a_new_lobby", old_lobby_callbacks_cannot_resume_or_retire_a_new_lobby as fn()),
            ("request_refused_before_editor_start_does_not_burn_return", request_refused_before_editor_start_does_not_burn_return as fn()),
            ("a_new_checkpoint_requires_its_own_match_receipt", a_new_checkpoint_requires_its_own_match_receipt as fn()),
        ]);
    }
    }
}
