#[path = "../src/model/macros/completion_trace.rs"]
mod completion_trace;
use completion_trace::{CompletionTrace, Event};

#[test]
fn idle_and_invalid_entries_never_consume_a_live_observation() {
    let mut trace = CompletionTrace::new();
    assert!(!trace.take(Event::State, Some(0)));
    trace.begin(17, 2);
    assert!(!trace.take(Event::State, Some(8)));
    assert!(!trace.take(Event::State, Some(255)));
    assert!(trace.take(Event::State, Some(0)));
    assert!(!trace.take(Event::State, Some(0)));
    assert_eq!(trace.record(), Some((17, 2)));
}
#[test]
fn repeated_end_callbacks_preserve_distinct_causes_and_fighters() {
    let mut trace = CompletionTrace::new();
    trace.begin(1, 0);
    for event in [Event::Sample, Event::Input, Event::State] {
        assert!(trace.take(event, Some(0)));
        assert!(trace.take(event, Some(1)));
        for _ in 0..10000 { assert!(!trace.take(event, Some(0))); }
    }
    for event in [Event::Publish, Event::Stop, Event::Serializer, Event::ActiveCopy, Event::Segment] {
        assert!(trace.take(event, None));
        assert!(!trace.take(event, None));
    }
}
#[test]
fn next_record_and_unarmed_clear_do_not_reuse_old_attribution() {
    let mut trace = CompletionTrace::new();
    trace.begin(1, 0);
    assert!(trace.take(Event::Stop, None));
    trace.clear();
    assert_eq!(trace.record(), None);
    assert!(!trace.take(Event::ActiveCopy, None));
    trace.begin(2, 0);
    assert_eq!(trace.record(), Some((2, 0)));
    assert!(trace.take(Event::Stop, None));
    trace.begin(3, 1);
    assert_eq!(trace.record(), Some((3, 1)));
    assert!(trace.take(Event::Stop, None));
}
#[test]
fn all_event_labels_have_independent_bits_and_a_fixed_total_bound() {
    let mut trace = CompletionTrace::new();
    trace.begin(1, 0);
    let events = [Event::Stop, Event::Publish, Event::Serializer, Event::ActiveCopy,
        Event::Segment, Event::State, Event::Sample, Event::Input];
    let mut count = 0;
    for _ in 0..3 {
        for event in events {
            assert!(!event.name().is_empty());
            for entry in (0..8).map(Some).chain([None]) {
                count += usize::from(trace.take(event, entry));
            }
        }
    }
    assert_eq!(count, 72);
}
