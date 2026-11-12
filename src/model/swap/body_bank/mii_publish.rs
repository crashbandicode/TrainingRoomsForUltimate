//! MII-GROUPS record publication: how a refused CharInfo publication is retried (C116.2 `mii-fix`).
//!
//! FACT (first console boot of C116.1, `practice-diagnostics.previous.log` #318-#509): one planned CPU Mii group's
//! publication was refused with the same reason on 180 consecutive frames (`CPU Mii group publication source
//! provenance changed`, one log line each), and the group then fell back. The refusal itself was a wrong baseline in
//! the adapter's `prefetch_publish` (fixed there; `R/c1162/MII-PROVENANCE-REPORT.md`). What this module owns is the
//! policy that was missing around it: a refusal that cannot clear while the owner idles must not spend 180 frames
//! (3 s at 60 fps) finding that out, and a reason that repeats is one log line, not one per frame.
//!
//! The adapter classifies each refusal where it is produced (`PublishRefusal`); `PublishAttempts` lives on the group
//! and decides what the caller does next.

/// Frames a planned Mii group waits for its adapter's record publication (read/write races only) before it is
/// failed instead of prefetched, so a persistently racing owner never spins.
pub const MII_PUBLISH_ATTEMPTS: u8 = 180;

/// What a refused record publication means for the retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishRefusal {
    /// Another transaction owns the owner, INSTALL or a loading source. It clears by itself and costs no attempt.
    Busy,
    /// A read/write race inside the publication (the native loader is not idle yet, a view moved between its proof
    /// and its commit, a native read failed). The next frame reads everything again; counted against the group.
    Race(&'static str),
    /// Cannot clear while the owner idles: the identity, owner/entry/epoch or owned state differ from what the group
    /// was planned for, or a helper has no ordinary authority. The group fails at once.
    Permanent(&'static str),
}

/// What the caller does after a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishStep {
    /// Try again next frame. `note` is the reason to write when it differs from the group's previous refusal (the
    /// first one included): one line per reason, not one per frame.
    Retry { note: Option<&'static str> },
    /// Fail the group now: `refusals` counted refusals ended in `reason`, the last one. The fallback line carries both.
    Fail { refusals: u8, reason: &'static str },
}

/// One group's refusals so far (the last reason is kept for the one-line log and the fallback line).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PublishAttempts {
    refusals: u8,
    last: Option<&'static str>,
}

impl PublishAttempts {
    pub const fn new() -> Self {
        Self { refusals: 0, last: None }
    }

    /// Counted refusals (races and permanent ones); `Busy` is never counted.
    pub const fn refusals(&self) -> u8 {
        self.refusals
    }

    /// The reason of the previous counted refusal.
    pub const fn last(&self) -> Option<&'static str> {
        self.last
    }

    /// Record one refusal and say what follows.
    pub fn refused(&mut self, refusal: PublishRefusal) -> PublishStep {
        match refusal {
            PublishRefusal::Busy => PublishStep::Retry { note: None },
            PublishRefusal::Race(reason) => {
                let changed = self.count(reason);
                if self.refusals >= MII_PUBLISH_ATTEMPTS {
                    PublishStep::Fail { refusals: self.refusals, reason }
                } else {
                    PublishStep::Retry { note: if changed { Some(reason) } else { None } }
                }
            }
            PublishRefusal::Permanent(reason) => {
                self.count(reason);
                PublishStep::Fail { refusals: self.refusals, reason }
            }
        }
    }

    /// Count one refusal; true when `reason` is not the previous refusal's.
    fn count(&mut self, reason: &'static str) -> bool {
        self.refusals = self.refusals.saturating_add(1);
        let changed = self.last != Some(reason);
        self.last = Some(reason);
        changed
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const RACE: &str = "the native loader idle at 6/14";
    const OTHER: &str = "CPU reservation refused";
    const PERMANENT: &str = "CPU Mii group publication identity changed";

    fn noted(reason: &'static str) -> PublishStep {
        PublishStep::Retry { note: Some(reason) }
    }

    const QUIET: PublishStep = PublishStep::Retry { note: None };

    fn a_repeated_race_is_noted_once_and_the_final_refusal_fails_with_its_reason() {
        let mut attempts = PublishAttempts::new();
        assert_eq!(attempts.refused(PublishRefusal::Race(RACE)), noted(RACE));
        let mut lines = 1;
        for _ in 1..MII_PUBLISH_ATTEMPTS - 1 {
            if attempts.refused(PublishRefusal::Race(RACE)) != QUIET {
                lines += 1;
            }
        }
        assert_eq!(lines, 1, "179 identical refusals are one line, not 179");
        assert_eq!(attempts.refusals(), MII_PUBLISH_ATTEMPTS - 1);
        assert_eq!(
            attempts.refused(PublishRefusal::Race(RACE)),
            PublishStep::Fail { refusals: MII_PUBLISH_ATTEMPTS, reason: RACE }
        );
        assert_eq!(attempts.last(), Some(RACE));
    }

    fn the_console_shape_of_180_identical_refusals_is_one_line_then_the_fallback() {
        // C116.1 first boot: 180 x the same reason, 180 lines, then the fallback.
        let mut attempts = PublishAttempts::new();
        let steps: Vec<PublishStep> =
            (0..MII_PUBLISH_ATTEMPTS).map(|_| attempts.refused(PublishRefusal::Race(RACE))).collect();
        assert_eq!(steps.iter().filter(|step| **step == noted(RACE)).count(), 1);
        assert_eq!(steps[0], noted(RACE));
        let last = usize::from(MII_PUBLISH_ATTEMPTS) - 1;
        assert_eq!(steps[last], PublishStep::Fail { refusals: MII_PUBLISH_ATTEMPTS, reason: RACE });
        assert!(steps[1..last].iter().all(|step| *step == QUIET));
    }

    fn a_changed_reason_is_noted_again_and_a_permanent_refusal_fails_at_once() {
        let mut attempts = PublishAttempts::new();
        assert_eq!(attempts.refused(PublishRefusal::Race(RACE)), noted(RACE));
        assert_eq!(attempts.refused(PublishRefusal::Race(RACE)), QUIET);
        assert_eq!(attempts.refused(PublishRefusal::Race(OTHER)), noted(OTHER));
        assert_eq!(attempts.refused(PublishRefusal::Race(OTHER)), QUIET);
        // The permanent refusal on the fifth attempt, nowhere near the limit.
        assert_eq!(
            attempts.refused(PublishRefusal::Permanent(PERMANENT)),
            PublishStep::Fail { refusals: 5, reason: PERMANENT }
        );
    }

    fn a_permanent_first_refusal_fails_on_the_first_frame() {
        let mut attempts = PublishAttempts::new();
        assert_eq!(
            attempts.refused(PublishRefusal::Permanent(PERMANENT)),
            PublishStep::Fail { refusals: 1, reason: PERMANENT }
        );
        assert_eq!(attempts.last(), Some(PERMANENT));
    }

    fn busy_is_never_counted_never_noted_and_never_fails() {
        let mut attempts = PublishAttempts::new();
        for _ in 0..10 * usize::from(MII_PUBLISH_ATTEMPTS) {
            assert_eq!(attempts.refused(PublishRefusal::Busy), QUIET);
        }
        assert_eq!(attempts, PublishAttempts::new());
        // The counted attempts start from zero afterwards.
        assert_eq!(attempts.refused(PublishRefusal::Race(RACE)), noted(RACE));
        assert_eq!(attempts.refusals(), 1);
        // A Busy between two identical races does not make the second one a new line.
        assert_eq!(attempts.refused(PublishRefusal::Busy), QUIET);
        assert_eq!(attempts.refused(PublishRefusal::Race(RACE)), QUIET);
    }

    fn the_counter_saturates_instead_of_wrapping() {
        let mut attempts = PublishAttempts { refusals: u8::MAX, last: None };
        assert_eq!(
            attempts.refused(PublishRefusal::Race(RACE)),
            PublishStep::Fail { refusals: u8::MAX, reason: RACE }
        );
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_repeated_race_is_noted_once_and_the_final_refusal_fails_with_its_reason", a_repeated_race_is_noted_once_and_the_final_refusal_fails_with_its_reason as fn()),
            ("the_console_shape_of_180_identical_refusals_is_one_line_then_the_fallback", the_console_shape_of_180_identical_refusals_is_one_line_then_the_fallback as fn()),
            ("a_changed_reason_is_noted_again_and_a_permanent_refusal_fails_at_once", a_changed_reason_is_noted_again_and_a_permanent_refusal_fails_at_once as fn()),
            ("a_permanent_first_refusal_fails_on_the_first_frame", a_permanent_first_refusal_fails_on_the_first_frame as fn()),
            ("busy_is_never_counted_never_noted_and_never_fails", busy_is_never_counted_never_noted_and_never_fails as fn()),
            ("the_counter_saturates_instead_of_wrapping", the_counter_saturates_instead_of_wrapping as fn()),
        ]);
    }
    }
}
