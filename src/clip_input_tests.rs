//! Recording input ownership exercises the actual engine guard and model route together.
use crate::model::macro_handoff::{MacroQueries, ClipInputRoute, route_clip_input};

fn parked_clip_keeps_cancel_route_and_closing_b_never_reaches_a_later_editor() {
    use crate::engine::macro_ui::clip_panel::CloseInputGuard;
    use ClipInputRoute::{Consume, Forward, Pass};
    let mut queries = MacroQueries::IDLE;
    let mut guard = CloseInputGuard::new();
    assert_eq!(queries.clip_input_route(guard.poll(true, false)), Pass);
    queries.clip_foreground = true;
    assert_eq!(queries.clip_input_route(guard.poll(true, false)), Forward);
    // A pending Mii read parks EDITOR, but its physical B still has to reach
    // the existing worker cancellation handler through the same mailbox.
    queries.clip_foreground = false;
    queries.replay_loading = true;
    assert_eq!(queries.clip_input_route(guard.poll(true, true)), Forward);
    // Cancel closes the opening draft. Its held press cannot leak after the
    // short grace, nor be forwarded to a new editor published meanwhile.
    queries = MacroQueries::IDLE;
    guard.arm(15);
    for poll in 0..90 {
        if poll == 30 { queries.clip_foreground = true; }
        assert_eq!(queries.clip_input_route(guard.poll(true, true)), Consume);
    }
    assert_eq!(queries.clip_input_route(guard.poll(true, false)), Forward);
    queries = MacroQueries::IDLE;
    assert_eq!(queries.clip_input_route(guard.poll(true, true)), Pass);
    guard.arm(15);
    assert_eq!(queries.clip_input_route(guard.poll(false, true)), Pass);
}

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
        ("parked_clip_keeps_cancel_route_and_closing_b_never_reaches_a_later_editor", parked_clip_keeps_cancel_route_and_closing_b_never_reaches_a_later_editor as fn()),
        ("a_close_published_at_the_query_read_cannot_follow_an_old_guard_poll", a_close_published_at_the_query_read_cannot_follow_an_old_guard_poll as fn()),
    ]);
}
