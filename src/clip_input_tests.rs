//! Recording input ownership exercises the actual engine guard and model route together.
use crate::model::macro_handoff::{MacroQueries, ClipInputRoute, route_clip_input};

fn a_close_published_at_the_query_read_cannot_follow_an_old_guard_poll() {
    let close_arm = std::cell::Cell::new(false);
    let route = route_clip_input(
        || {
            // Worker close lands just before the mapper observes the new
            // closed snapshot, with its release-store of ARM before publish.
            close_arm.set(true);
            MacroQueries::IDLE
        },
        || close_arm.replace(false),
    );
    assert_eq!(route, ClipInputRoute::Consume);
    assert!(!close_arm.get(), "this mapping consumed the published arm");
}


/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("a_close_published_at_the_query_read_cannot_follow_an_old_guard_poll", a_close_published_at_the_query_read_cannot_follow_an_old_guard_poll as fn()),
    ]);
}
