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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_repeated_race_is_noted_once_and_the_final_refusal_fails_with_its_reason", a_repeated_race_is_noted_once_and_the_final_refusal_fails_with_its_reason as fn()),
        ]);
    }
    }
}
