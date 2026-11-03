#[path = "../src/model/mii/bust_context.rs"]
mod policy;

#[test]
fn only_exact_native_return_pcs_are_observed() {
    let base = 0x7100000000;
    assert_eq!(policy::CONSTRUCTOR, 0x377c0d0);
    for (i, pc) in [0x1811408, 0x19d1480].into_iter().enumerate() {
        assert_eq!(policy::caller(base, base + pc), Some(i));
        assert_eq!(policy::caller(base, base + pc - 4), None); // BL address is not LR
        assert_eq!(policy::caller(base, base + pc + 4), None);
    }
    assert_eq!(policy::caller(0, 0x1811408), None);
    assert_eq!(policy::caller(base, base - 1), None);
    assert_eq!(policy::caller(base, base + 0x150d7f0), None); // unrelated copy-enabled caller
}

#[test]
fn every_guard_is_required_before_installation() {
    for mask in 0..8 {
        assert_eq!(policy::admitted([mask & 1 != 0, mask & 2 != 0, mask & 4 != 0]), mask == 7);
    }
}

#[test]
fn each_caller_has_a_finite_observation_budget() {
    assert!((0..8).all(policy::within_limit));
    assert!(!policy::within_limit(8));
    assert!(!policy::within_limit(usize::MAX));
    assert_eq!(policy::LIMIT_PER_CALLER, 8);
}
