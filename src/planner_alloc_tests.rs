//! Reproduce the old planner allocation and exercise the bounded replacement.
use crate::{engine, model::hook_stats::{HookId, HookScope}};
use crate::engine::swap::mii_cycle::compact_regressions::AllocationCount;

#[test]
fn planning_lists_preserve_order_and_exclusion_without_hook_allocations() {
    const CHILD: &str = "TR_PLANNER_ALLOC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "planner_alloc_tests::planning_lists_preserve_order_and_exclusion_without_hook_allocations", "--test-threads=1"])
            .env(CHILD, "1").output().unwrap();
        assert!(output.status.success(), "{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        return;
    }

    use engine::swap::{ordered_roster::OrderedFighterRoster, trio::TrioFighter};
    let ranked = OrderedFighterRoster::from_fighters(&[TrioFighter::KIRBY, TrioFighter::MARIO]).unwrap();
    // The previous native planner used this public Vec-returning path.
    let measurement = AllocationCount::start();
    {
        let _scope = HookScope::enter_hook(HookId::RosterPreload);
        std::hint::black_box(ranked.fighters());
    }
    assert!(measurement.finish() > 0, "old planner path must reproduce the allocation");
    let measurement = AllocationCount::start();
    let (all, ordered, excluded, hats) = {
        let _scope = HookScope::enter_hook(HookId::RosterPreload);
        (OrderedFighterRoster::all().fighters_excluding_fixed(None),
         ranked.fighters_excluding_fixed(None),
         ranked.fighters_excluding_fixed(Some(TrioFighter::KIRBY.kind())),
         ranked.hat_kinds_fixed())
    };
    assert_eq!(measurement.finish(), 0, "replacement planner lists allocate nothing including first use");
    assert!(!all.is_empty());
    assert_eq!(&*ordered, &[TrioFighter::KIRBY, TrioFighter::MARIO]);
    assert_eq!(&*excluded, &[TrioFighter::MARIO]);
    assert_eq!(&*hats, &[TrioFighter::MARIO.kind()]);
}
