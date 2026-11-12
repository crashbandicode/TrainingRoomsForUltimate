//! Native callbacks and host probes share engine::mii::inline_keyboard.
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
// rc.9 coverage: this file's cases run as one table (`case_table`, docs/COVERAGE-rc9.md).
#[path = "../src/test_table.rs"]
mod test_table;
#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
#[allow(unused_imports)]
mod engine {
    pub use super::mii;
}
#[allow(dead_code)]
#[path = "../src/model/hook_stats.rs"]
mod hook_stats;

use mii::inline_keyboard::{Decision, Limit, Signals, Text};

#[global_allocator]
static ALLOCATOR: hook_stats::CountingAllocator = hook_stats::CountingAllocator;

fn enter_freezes_its_value_across_later_change_and_invalid_callbacks() {
    let mut signals = Signals { limit: Limit::Macro, ..Signals::default() };
    signals.enter(&[65, 66], 2);
    signals.change(&[67, 68, 69], 3, 1);
    assert_eq!(signals.text.unwrap().units(), &[67, 68, 69]);
    match signals.decision {
        Some(Decision::Enter(text)) => assert_eq!(text.units(), &[65, 66]),
        _ => panic!("Enter was lost"),
    }
    signals.change(&[0xd800], 1, 1);
    signals.enter(&[0xdc00], 1);
    assert!(signals.invalid);
    assert_eq!(signals.text.unwrap().units(), &[67, 68, 69]);
    match signals.decision {
        Some(Decision::Enter(text)) => assert_eq!(text.units(), &[65, 66]),
        _ => panic!("invalid callback replaced Enter"),
    }
    signals.change(&[70], 1, 0);
    assert!(signals.invalid, "a later valid update cannot clear a protocol error");
    signals.cancel();
    assert!(matches!(signals.decision, Some(Decision::Cancel)));
}

#[test]
fn keyboard_validation_does_not_allocate_on_first_or_repeated_use() {
    const CHILD: &str = "TRAININGROOMS_INLINE_KEYBOARD_ALLOC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "keyboard_validation_does_not_allocate_on_first_or_repeated_use",
                   "--test-threads=1", "--nocapture"])
            .env(CHILD, "1")
            .output().unwrap();
        assert!(output.status.success(), "{}{}", String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr));
        print!("{}", String::from_utf8_lossy(&output.stdout));
        return;
    }
    fn measure(repeats: usize) -> u64 {
        let before = hook_stats::hook_allocations();
        {
            let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
            for _ in 0..repeats {
                let mut signals = Signals { limit: Limit::Macro, ..Signals::default() };
                std::hint::black_box(Text::macro_name("A\u{1f600}B", 3));
                signals.change(&[0x41, 0xd83d, 0xde00, 0x42], 4, 3);
                signals.enter(&[0x41, 0xd83d, 0xde00, 0x42], 4);
                signals.change(&[0xd800], 1, 1);
                signals.cancel();
                std::hint::black_box(signals);
                std::hint::black_box(Text::new(&[0xd800, 65], 1));
            }
        }
        hook_stats::hook_allocations() - before
    }
    let first = measure(1);
    let repeated = measure(64);
    let before_control = hook_stats::hook_allocations();
    {
        let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
        std::hint::black_box(vec![7u8; 1024]);
    }
    let control = hook_stats::hook_allocations() - before_control;
    println!("keyboard allocations first={first} repeated={repeated} positive_control={control}");
    assert!(control > 0, "the detector must observe the positive control");
    assert_eq!((first, repeated), (0, 0));
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("enter_freezes_its_value_across_later_change_and_invalid_callbacks", enter_freezes_its_value_across_later_change_and_invalid_callbacks as fn()),
    ]);
}
