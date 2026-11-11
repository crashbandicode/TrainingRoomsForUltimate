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

