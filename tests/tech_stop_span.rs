#[path = "../src/runtime/features/tech_stop_span.rs"]
mod stop_span;
#[test]
fn retained_reference_ages_count_callbacks_not_changed_records() {
    let mut span = stop_span::Span::new();
    for age in 71..=79 {
        assert_eq!(span.observe(age), Some(age - 70));
    }
    assert_eq!(span.first(), Some(71));
    assert_eq!(span.count(), 9);
}
#[test]
fn missing_duplicate_and_backwards_callbacks_fail_without_mutation() {
    for bad in [70, 71, 73, u32::MAX] {
        let mut span = stop_span::Span::new();
        assert_eq!(span.observe(71), Some(1));
        assert_eq!(span.observe(bad), None);
        assert_eq!(span.count(), 1);
        assert_eq!(span.observe(72), Some(2));
    }
}
#[test]
fn age_wrap_does_not_create_a_continuous_interval() {
    let mut span = stop_span::Span::new();
    assert_eq!(span.observe(u32::MAX), Some(1));
    assert_eq!(span.observe(0), None);
}
