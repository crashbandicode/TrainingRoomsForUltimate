//! Menu/request ownership arbitration only; no native calls or pilot timers.
//! A changed setting is the only way to lift a completed request's inhibition.

pub(crate) const fn level(enabled: bool, selected: Option<u8>) -> u8 {
    match (enabled, selected) {
        (true, Some(level @ 1..=9)) => level,
        _ => 0,
    }
}

/// Low four bits hold Off (0) or level 1..9; the rest is a change revision.
/// Publishing the same effective setting is deliberately idempotent.
pub(crate) fn update(previous: u64, level: u8) -> u64 {
    if previous & 15 == u64::from(level) {
        previous
    } else {
        (previous & !15).wrapping_add(16) | u64::from(level)
    }
}

pub(crate) struct Arbitration {
    observed: u64,
    inhibited: bool,
}

impl Arbitration {
    pub(crate) const fn new() -> Self {
        Self {
            observed: 0,
            inhibited: false,
        }
    }

    pub(crate) fn request_started(&mut self) {
        self.inhibited = true;
    }

    /// Call only after menu capture ends. A request owns the whole session,
    /// including menu pauses: commits during it cannot queue menu AI afterward.
    pub(crate) fn observe(&mut self, published: u64, request_active: bool) -> Option<u8> {
        if self.observed != published {
            self.observed = published;
            if !request_active {
                self.inhibited = false;
            }
        }
        if request_active {
            self.request_started();
        }
        let level = (published & 15) as u8;
        (!self.inhibited && (1..=9).contains(&level)).then_some(level)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn menu_changes_replace_or_stop_and_repeated_commits_are_idempotent() {
        let mut arbitration = Arbitration::new();
        assert_eq!(arbitration.observe(0, false), None);
        let on = update(0, level(true, Some(3)));
        assert_eq!(arbitration.observe(on, false), Some(3));
        assert_eq!(update(on, 3), on);
        let changed = update(on, 7);
        assert_eq!(arbitration.observe(changed, false), Some(7));
        assert_eq!(arbitration.observe(update(changed, 0), false), None);
        assert_eq!(level(true, None), 0);
        assert_eq!(level(true, Some(10)), 0);
        assert_eq!(level(false, Some(3)), 0);
    }

    fn request_preempts_until_a_later_explicit_setting_change() {
        let mut arbitration = Arbitration::new();
        let on = update(0, 3);
        assert_eq!(arbitration.observe(on, false), Some(3));
        arbitration.request_started();
        assert_eq!(arbitration.observe(on, true), None);
        let during_request = update(on, 4);
        assert_eq!(arbitration.observe(during_request, true), None);
        assert_eq!(arbitration.observe(during_request, false), None);
        assert_eq!(
            arbitration.observe(update(during_request, 5), false),
            Some(5)
        );
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("menu_changes_replace_or_stop_and_repeated_commits_are_idempotent", menu_changes_replace_or_stop_and_repeated_commits_are_idempotent as fn()),
            ("request_preempts_until_a_later_explicit_setting_change", request_preempts_until_a_later_explicit_setting_change as fn()),
        ]);
    }
    }
}
