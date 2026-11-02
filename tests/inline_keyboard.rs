//! Native callbacks and host probes share engine::mii::inline_keyboard.
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

use mii::inline_keyboard::{text_signal, Decision, Limit, Signals, Text};

#[global_allocator]
static ALLOCATOR: hook_stats::CountingAllocator = hook_stats::CountingAllocator;

#[test]
fn independent_utf16_vectors_preserve_macro_boundaries() {
    // Explicit UTF-16 facts: A, U+1F600, B. Cursor positions 0,1,3,4 are boundaries.
    let units = [0x41, 0xd83d, 0xde00, 0x42];
    for cursor in [0, 1, 3, 4] {
        let text = text_signal(&units, 4, cursor, Limit::Macro).unwrap();
        assert_eq!(text.units(), units);
        assert_eq!(text.cursor(), cursor as usize);
    }
    for cursor in [-1, 2, 5, 256, i32::MAX] {
        assert!(text_signal(&units, 4, cursor, Limit::Macro).is_none());
    }
    for units in [&[0xd800][..], &[0xdc00], &[0xdc00, 0xd800], &[0xd800, 0x41], &[0xd800, 0xd800]] {
        assert!(text_signal(units, units.len() as i32, 0, Limit::Macro).is_none());
    }
    assert!(text_signal(&[0xdbff, 0xdfff], 2, 2, Limit::Macro).is_some()); // U+10FFFF.
    for unit in [0, 9, 10, 0x7f, 0x85] {
        assert!(text_signal(&[unit], 1, 1, Limit::Macro).is_none());
    }
    assert!(text_signal(&[0x200d], 1, 1, Limit::Macro).is_some()); // ZWJ is not Cc.
    assert!(text_signal(&units, -1, 0, Limit::Macro).is_none());
    assert!(text_signal(&units, 5, 0, Limit::Macro).is_none());
    assert!(text_signal(&units, i32::MAX, 0, Limit::Macro).is_none());
    assert_eq!(text_signal(&[0x41, 0, 0xd800], 1, 1, Limit::Macro).unwrap().units(), &[0x41]);
}

#[test]
fn mii_raw_units_and_terminated_native_buffer_keep_their_contract() {
    let units = [0xd800, 10, 0xdc00, 65, 66, 67, 68, 69, 70, 71];
    let text = Text::new(&units, 1).unwrap();
    assert_eq!(text.units(), units);
    assert_eq!(text.cursor(), 1);
    assert_eq!(text.terminated(), &[0xd800, 10, 0xdc00, 65, 66, 67, 68, 69, 70, 71, 0]);
    assert_eq!(std::mem::size_of_val(text.terminated()), 22);
    assert!(Text::new(&[65; 11], 0).is_none());
    assert!(Text::new(&[65, 0], 0).is_none());
    assert!(Text::new(&units, 256).is_none());
}

#[test]
fn scalar_limit_is_distinct_from_utf16_capacity() {
    let max = "\u{1f600}".repeat(48);
    let text = Text::macro_name(&max, 96).unwrap();
    assert_eq!(text.units().len(), 96);
    assert_eq!(text.terminated().len(), 97);
    assert_eq!(text.cursor(), 96);
    assert!(Text::macro_name(&(max + "a"), 0).is_none());
    assert!(Text::macro_name(&"a".repeat(49), 0).is_none());
    assert!(Text::macro_name(&"a".repeat(48), 48).is_some());
    assert!(Text::macro_name("A\u{1f600}B", 2).is_none());
}

#[test]
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
fn empty_enter_and_new_owner_preserve_distinct_transport_semantics() {
    let mut mii = Signals::default();
    mii.enter(&[], 0);
    assert!(matches!(mii.decision, Some(Decision::Cancel)));
    let mut macros = Signals { limit: Limit::Macro, ..Signals::default() };
    macros.enter(&[], 0);
    assert!(matches!(macros.decision, Some(Decision::Enter(text)) if text.units().is_empty()));
    macros.change(&[0], 1, 0);
    let next = std::mem::take(&mut macros);
    assert!(next.invalid && !macros.invalid && macros.text.is_none() && macros.decision.is_none());
}

#[test]
fn callback_value_fits_without_narrowing_a_valid_caret() {
    assert!(std::mem::size_of::<Text>() <= 200);
    let text = text_signal(&[65; 48], 48, 48, Limit::Macro).unwrap();
    assert_eq!(text.cursor(), 48);
    assert!(text_signal(&[65; 48], 48, 256, Limit::Macro).is_none());
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
