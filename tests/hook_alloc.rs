//! Review R5 of package 1: the allocation detector (`model::hook_stats`) is proven with stubs. The
//! test crate installs the plugin's own `CountingAllocator` as its global allocator, so the stubs'
//! allocations are counted exactly as the Switch runtime's are.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::hook_stats::{hook_allocations, snapshot, CountingAllocator, HookId, HookScope, Observer, HOOK_COUNT};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// A stub hook that follows the rule: it works on the stack only.
fn well_behaved_hook(id: HookId, n: u32) -> u32 {
    let _scope = HookScope::enter(id);
    let mut ring = [0u32; 8];
    for (i, slot) in ring.iter_mut().enumerate() {
        *slot = n.wrapping_mul(i as u32 + 1);
    }
    ring.iter().fold(0, |a, b| a ^ b)
}

/// A stub hook that breaks the rule on purpose: it allocates inside its scope.
fn allocating_hook(id: HookId, bytes: usize) -> usize {
    let _scope = HookScope::enter(id);
    let v = vec![1u8; bytes];
    std::hint::black_box(&v);
    v.len()
}

/// The counters are process-wide and the harness runs tests on several threads: the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn the_detector_reports_a_stub_that_allocates_and_passes_one_that_does_not() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut observer = Observer::new();
    // Whatever the other test already did is the baseline; only what follows is measured.
    let _ = observer.observe(snapshot());

    // Hook coverage without allocation: nonzero counts, delta 0.
    let mut sink = 0;
    for n in 0..50 {
        sink ^= well_behaved_hook(HookId::InputCallerAApply, n);
        sink ^= well_behaved_hook(HookId::ResetCheck, n);
    }
    std::hint::black_box(sink);
    let clean = observer.observe(snapshot()).expect("hooks ran, so a line is due");
    assert_eq!(clean.delta, 0, "a stack-only hook must not move the allocation counter: {clean}");
    assert!(clean.covered(HookId::InputCallerAApply) && clean.covered(HookId::ResetCheck));
    assert!(!clean.covered(HookId::HazardRule), "a hook that never ran is not covered");
    assert_eq!(clean.hook_calls, 100);
    assert!(clean.to_string().contains("delta=0") && clean.to_string().contains("input_caller_a_apply:50"));

    // The intentionally allocating stub: the same detector reports a nonzero delta.
    let before = hook_allocations();
    assert_eq!(allocating_hook(HookId::HazardRule, 4096), 4096);
    assert!(hook_allocations() > before, "the stub's allocation is counted");
    let dirty = observer.observe(snapshot()).expect("an allocation is reported even with one hook call");
    assert!(dirty.delta >= 1, "{dirty}");
    assert!(dirty.to_string().contains(&format!("delta={}", dirty.delta)));
    assert!(dirty.covered(HookId::HazardRule));

    // Allocation outside any hook is not counted (the boot thread allocates freely).
    let outside = hook_allocations();
    let v = vec![0u8; 1 << 16];
    std::hint::black_box(&v);
    assert_eq!(hook_allocations(), outside);
    assert_eq!(snapshot().entries.len(), HOOK_COUNT);

    // A second quiet sample reports nothing.
    assert!(observer.observe(snapshot()).is_none());
}

/// Nesting: a hook that calls another hook's code stays inside a scope until the outer one drops.
#[test]
fn nested_scopes_count_until_the_outermost_ends() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = hook_allocations();
    {
        let _outer = HookScope::enter(HookId::CpuFrame);
        {
            let _inner = HookScope::enter(HookId::CpuSdi);
        }
        let v = vec![0u8; 64];
        std::hint::black_box(&v);
    }
    assert!(hook_allocations() > before, "the allocation after the inner scope ended is still inside the outer");
    let after = hook_allocations();
    let v = vec![0u8; 64];
    std::hint::black_box(&v);
    assert_eq!(hook_allocations(), after);
}
