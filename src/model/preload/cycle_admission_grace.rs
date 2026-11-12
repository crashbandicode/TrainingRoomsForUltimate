//! Value-only foreground burst policy. Times are caller-supplied monotonic
//! microseconds; generations retire all prior scene/settings state. No metrics,
//! native ownership, reservations or callback counts participate in this policy.
pub const QUIET_US: u64 = 2_000_000;
pub const BURST_CAP_US: u64 = 5_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settlement {
    Started,
    Extended,
    Unchanged,
}

#[derive(Clone, Copy, Debug)]
struct Burst {
    start: u64,
    deadline: u64,
    expired: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct AdmissionGrace {
    generation: u64,
    burst: Option<Burst>,
}

impl AdmissionGrace {
    pub const fn new(generation: u64) -> Self {
        Self {
            generation,
            burst: None,
        }
    }

    pub fn reset(&mut self, generation: u64) {
        *self = Self::new(generation);
    }

    fn qualify(&mut self, generation: u64) {
        if self.generation != generation {
            self.reset(generation);
        }
    }

    /// True exactly once when the active deadline expires. Expiry is sticky,
    /// including expiry of the quiet window before the absolute cap.
    pub fn advance(&mut self, now: u64, generation: u64) -> bool {
        self.qualify(generation);
        if let Some(burst) = self.burst.as_mut() {
            if !burst.expired && now >= burst.deadline {
                burst.expired = true;
                return true;
            }
        }
        false
    }

    /// Call only for actual successful visible transactions, never input intent.
    pub fn settled(&mut self, now: u64, generation: u64) -> Settlement {
        self.advance(now, generation);
        match self.burst.as_mut() {
            None => {
                self.burst = Some(Burst {
                    start: now,
                    deadline: now.saturating_add(QUIET_US),
                    expired: false,
                });
                Settlement::Started
            }
            Some(burst) if !burst.expired => {
                let deadline = now
                    .saturating_add(QUIET_US)
                    .min(burst.start.saturating_add(BURST_CAP_US));
                if deadline == burst.deadline {
                    return Settlement::Unchanged;
                }
                burst.deadline = deadline;
                Settlement::Extended
            }
            Some(_) => Settlement::Unchanged,
        }
    }

    pub fn allows(&mut self, now: u64, generation: u64, speculative: bool) -> bool {
        self.advance(now, generation);
        !speculative || self.deadline().is_none()
    }

    pub fn deadline(&self) -> Option<u64> {
        self.burst.filter(|b| !b.expired).map(|b| b.deadline)
    }

    /// SourceBook begin_install success is the commitment boundary. Neither
    /// preflight nor extra-owner creation calls this. Stale commits cannot
    /// consume a newer generation's burst.
    pub fn speculative_committed(&mut self, generation: u64) -> bool {
        if self.generation != generation {
            return false;
        }
        self.burst.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    fn non_renewable_cap() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        for now in [1_500_000, 3_000_000, 4_500_000] {
            p.settled(now, 1);
        }
        assert_eq!(p.deadline(), Some(5_000_000));
        assert!(p.advance(5_000_000, 1));
        for now in [5_000_001, 6_000_000, 12_000_000] {
            assert_eq!(p.settled(now, 1), Settlement::Unchanged);
            assert!(p.allows(now, 1, true));
        }
    }
    fn quiet_window_extension() {
        let mut p = AdmissionGrace::new(1);
        assert_eq!(p.settled(100, 1), Settlement::Started);
        assert_eq!(p.settled(1_000_100, 1), Settlement::Extended);
        assert!(!p.allows(3_000_099, 1, true));
        assert!(p.allows(3_000_100, 1, true));
        assert_eq!(p.settled(3_000_101, 1), Settlement::Unchanged);
    }
    fn opportunity_consumed_only_by_committed_speculative_install() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        assert!(p.allows(QUIET_US, 1, true));
        assert!(p.allows(QUIET_US + 1, 1, true)); // selection/preflight does not consume
        assert_eq!(p.settled(QUIET_US + 2, 1), Settlement::Unchanged);
        assert!(p.speculative_committed(1));
        assert_eq!(p.settled(QUIET_US + 3, 1), Settlement::Started);
    }
    fn refusal_before_submit_does_not_consume_or_restart() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        assert!(!p.allows(500_000, 1, true));
        assert_eq!(p.deadline(), Some(2_000_000));
        for now in [2_000_000, 2_500_000, 10_000_000] {
            assert!(p.allows(now, 1, true)); // repeated refused preflight/reservation
            assert_eq!(p.settled(now, 1), Settlement::Unchanged);
        }
    }
    fn real_demand_bypasses_grace_without_consuming_it() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        assert!(p.allows(1, 1, false));
        assert!(!p.allows(1, 1, true));
        assert_eq!(p.deadline(), Some(2_000_000));
    }
    fn stale_generation_resets_and_old_commit_cannot_consume() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        assert!(p.allows(1, 2, true));
        assert_eq!(p.settled(2, 2), Settlement::Started);
        assert!(!p.speculative_committed(1));
        assert!(!p.allows(3, 2, true));
        p.reset(3);
        assert!(p.allows(4, 3, true));
    }
    fn measurements_off_independence_and_rejected_input() {
        // There is deliberately no measurements flag or request-input method.
        let mut p = AdmissionGrace::new(1);
        assert!(p.allows(100, 1, true)); // no-op/rejected input publishes nothing
        p.settled(200, 1);
        assert!(!p.allows(201, 1, true));
        assert!(p.allows(2_000_200, 1, true));
    }
    fn expiry_allows_one_commit_then_normal_speculation() {
        let mut p = AdmissionGrace::new(1);
        p.settled(0, 1);
        assert!(p.allows(QUIET_US, 1, true));
        assert!(p.speculative_committed(1));
        assert!(p.allows(QUIET_US + 1, 1, true));
        assert!(!p.speculative_committed(1)); // normal speculation, no burst to consume
        assert_eq!(p.settled(QUIET_US + 2, 1), Settlement::Started);
        assert!(!p.allows(QUIET_US + 3, 1, true));
    }
    fn settlement_between_selection_and_final_admission_defers() {
        let mut p = AdmissionGrace::new(1);
        assert!(p.allows(0, 1, true));
        p.settled(1, 1);
        assert!(!p.allows(2, 1, true));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("non_renewable_cap", non_renewable_cap as fn()),
            ("quiet_window_extension", quiet_window_extension as fn()),
            ("opportunity_consumed_only_by_committed_speculative_install", opportunity_consumed_only_by_committed_speculative_install as fn()),
            ("refusal_before_submit_does_not_consume_or_restart", refusal_before_submit_does_not_consume_or_restart as fn()),
            ("real_demand_bypasses_grace_without_consuming_it", real_demand_bypasses_grace_without_consuming_it as fn()),
            ("stale_generation_resets_and_old_commit_cannot_consume", stale_generation_resets_and_old_commit_cannot_consume as fn()),
            ("measurements_off_independence_and_rejected_input", measurements_off_independence_and_rejected_input as fn()),
            ("expiry_allows_one_commit_then_normal_speculation", expiry_allows_one_commit_then_normal_speculation as fn()),
            ("settlement_between_selection_and_final_admission_defers", settlement_between_selection_and_final_admission_defers as fn()),
        ]);
    }
    }
}
