//! Track a qualified native popup within one results lifetime. The native
//! adapter identifies the opener; a retained result at an idle popup is not a
//! new answer. Addresses here are identity values, never dereferenced pointers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    SaveConfirmation,
    Rematch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Ready(Kind),
    Decided { kind: Kind, result: u32, source: Source },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Normal,
    Forced,
    Late,
}

/// Read together by the native adapter on the game thread. `pending` is the
/// checked top of the native phase stack, not a guessed next phase.
#[derive(Clone, Copy)]
pub struct Snapshot {
    pub phase: u32,
    pub pending: u32,
    pub previous: u32,
    pub age: u32,
    pub result: u32,
    pub decided: bool,
    pub opening: bool,
}

#[derive(Clone, Copy)]
struct Open {
    owner: usize,
    kind: Kind,
    saw_reset: bool,
    ready: bool,
}

pub struct Popup {
    epoch: Option<u64>,
    open: Option<Open>,
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

impl Popup {
    pub const fn new() -> Self {
        Self {
            epoch: None,
            open: None,
        }
    }

    /// Called at the results lifetime boundary, including an address reused by
    /// the next match. The epoch comes from native construction, not a pointer.
    pub fn begin(&mut self, epoch: u64) {
        self.epoch = Some(epoch);
        self.open = None;
    }

    pub fn end(&mut self) {
        self.epoch = None;
        self.open = None;
    }

    /// Every open on the primary native popup manager must be reported,
    /// including unrecognized messages. The adapter filters other managers.
    /// `None` invalidates the previous popup rather than inheriting its kind.
    /// Qualification includes the native caller, message and observed zero
    /// default result, not just timing. A nonzero default is unsupported.
    pub fn opened(&mut self, epoch: u64, owner: usize, kind: Option<Kind>) {
        self.open = if self.epoch == Some(epoch) && owner != 0 {
            kind.map(|kind| Open {
                owner,
                kind,
                saw_reset: false,
                ready: false,
            })
        } else {
            None
        };
    }

    /// Only settled phase 3 accepts normal input. Phase 4 runs a timer.
    /// Latch the decision before closing animation: the results lifetime can
    /// end before a later idle observation. A forced or late result retains
    /// its provenance so the caller cannot claim its input caused the answer.
    /// Native result interpretation belongs to the version-checked adapter.
    pub fn observe(
        &mut self,
        epoch: u64,
        owner: usize,
        state: Snapshot,
    ) -> Option<Event> {
        let open = self.open.as_mut()?;
        if self.epoch != Some(epoch) {
            self.open = None;
            return None;
        }
        // The native update also runs for a second, unrelated manager.
        if open.owner != owner {
            return None;
        }
        if state.phase > 7 || state.pending > 7 || state.previous > 7 {
            self.open = None;
            return None;
        }
        // Admission requires an observed zero default. Seeing that active
        // reset prevents the old idle result from acknowledging a new request.
        if state.phase != 0 && state.result == 0 {
            open.saw_reset = true;
        }
        let source = match (state.phase, state.pending) {
            (3, 6) if state.decided => Some(Source::Normal),
            (5, 6) => Some(Source::Forced),
            (6 | 7, _) => Some(Source::Late),
            (0, 0) if state.previous == 7 => Some(Source::Late),
            _ => None,
        };
        if let Some(source) = source.filter(|_| open.saw_reset) {
            let event = Event::Decided { kind: open.kind, result: state.result, source };
            self.open = None;
            return Some(event);
        }
        if open.saw_reset && state.phase == 3 && state.pending == 3
            && state.age >= 1 && state.result == 0 && !state.decided && !state.opening
            && !open.ready {
            open.ready = true;
            return Some(Event::Ready(open.kind));
        }
        if state.phase == 0 && state.pending == 0 && !state.opening {
            self.open = None;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn state(phase: u32, result: u32) -> Snapshot {
        Snapshot { phase, pending: phase, previous: 7, age: 1, result,
            decided: false, opening: false }
    }

    fn teardown_reused_address_or_wrong_epoch_discards_pending_answer() {
        // Each path represents a different invalidation boundary, not an
        // additional gameplay matrix.
        for boundary in 0..4 {
            let mut p = Popup::new();
            p.begin(1);
            p.opened(1, 10, Some(Kind::Rematch));
            p.observe(1, 10, state(3, 0));
            match boundary {
                0 => p.end(),
                1 => p.begin(2),
                2 => { p.observe(2, 10, state(3, 0)); }
                _ => { p.observe(1, 10, state(99, 0)); }
            }
            assert_eq!(p.observe(1, 10, state(0, 2)), None);
        }
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("teardown_reused_address_or_wrong_epoch_discards_pending_answer", teardown_reused_address_or_wrong_epoch_discards_pending_answer as fn()),
        ]);
    }
    }
}
