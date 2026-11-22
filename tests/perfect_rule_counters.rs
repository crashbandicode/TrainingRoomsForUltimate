use crate::model::edge_juggle::perfect::rule_counts;

use rule_counts::{Counters, PerfectRule, RuleCounts};

#[test]
fn perfect_counts_isolate_every_rule_and_attribute_the_latest_changed_pad() {
    let counts = Counters::new();
    assert_eq!(counts.snapshot(), RuleCounts::default());
    let rules = [
        PerfectRule::Route,
        PerfectRule::Jump,
        PerfectRule::DistanceDodge,
        PerfectRule::Reach,
        PerfectRule::Detour,
        PerfectRule::Landing,
        PerfectRule::ReactDodge,
        PerfectRule::HitRecovery,
        PerfectRule::Meteor,
    ];
    for (index, rule) in rules.into_iter().enumerate() {
        for _ in 0..=index {
            counts.record(rule);
        }
        assert_eq!(counts.snapshot().last_rule, rule as u32);
    }
    assert_eq!(
        counts.snapshot(),
        RuleCounts {
            edge: [1, 2, 3, 4, 5],
            juggler: [6, 7, 8, 9],
            last_rule: 14,
        }
    );
    counts.record(PerfectRule::Niche);
    assert_eq!(
        counts.snapshot(),
        RuleCounts {
            edge: [1, 2, 3, 4, 5],
            juggler: [6, 7, 8, 9],
            last_rule: 6,
        }
    );
}
