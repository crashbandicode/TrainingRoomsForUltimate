//! Bound the extra native analysis work; successful ownership lasts to scene exit.
//! The adapter supplies identity/native-AI checks before calling this state machine.

pub(crate) const LIMIT: u16 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Pending,
    Ready,
    Failed,
    Cancelled,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Pass,
    Initialize,
    Isolate,
}

#[derive(Clone, Copy)]
pub(crate) struct Session {
    pub status: Status,
    pub frames: u16,
}

impl Session {
    pub(crate) fn start(intent: bool, human_only: bool, phase: i32, d0: u8) -> Option<Self> {
        (intent && human_only && phase == 1 && d0 & 1 == 0).then_some(Self {
            status: Status::Pending,
            frames: 0,
        })
    }

    pub(crate) fn before(&mut self, intent: bool, paused: bool) -> Action {
        match self.status {
            // Phase 2 keeps running natively even when the request/menu ends.
            Status::Ready => Action::Isolate,
            Status::Pending if !intent => {
                self.status = Status::Cancelled;
                Action::Pass
            }
            Status::Pending if !paused => {
                self.frames += 1;
                Action::Initialize
            }
            _ => Action::Pass,
        }
    }

    pub(crate) fn after(&mut self, phase: i32) {
        if self.status == Status::Pending {
            if phase == 2 {
                self.status = Status::Ready;
            } else if self.frames == LIMIT {
                self.status = Status::Failed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn success_on_final_attempt_keeps_isolation_after_owner_and_during_menu() {
        let mut session = Session::start(true, true, 1, 0).unwrap();
        for _ in 0..119 {
            session.before(true, false);
            session.after(1);
        }
        assert_eq!(session.before(true, false), Action::Initialize);
        session.after(2);
        assert_eq!(session.status, Status::Ready);
        assert_eq!(session.before(false, true), Action::Isolate);
        // A later phase regression is not permission for another unbounded raise.
        session.after(1);
        assert_eq!(session.before(false, false), Action::Isolate);
        assert_eq!(session.frames, 120);
    }

    fn cancel_before_completion_never_raises_again() {
        let mut session = Session::start(true, true, 1, 0).unwrap();
        assert_eq!(session.before(false, false), Action::Pass);
        assert_eq!(session.status, Status::Cancelled);
        assert_eq!(session.before(true, false), Action::Pass);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("success_on_final_attempt_keeps_isolation_after_owner_and_during_menu", success_on_final_attempt_keeps_isolation_after_owner_and_during_menu as fn()),
            ("cancel_before_completion_never_raises_again", cancel_before_completion_never_raises_again as fn()),
        ]);
    }
    }
}
