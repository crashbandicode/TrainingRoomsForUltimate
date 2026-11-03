//! Persisted identity and native value ownership survive representation changes.
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

use mii::inventory::{MiiClass, MiiConfig, TaggedId};
use mii::lifetime::{Configuration, Tokens};
use mii::native_values::RowIds;

#[global_allocator]
static ALLOCATOR: hook_stats::CountingAllocator = hook_stats::CountingAllocator;

const TOKENS: Tokens = Tokens { outfit: 7, hat: 9, colour: 255, voice: 254, specials: [0, 1, 2, 1] };
const ROWS: RowIds = RowIds { outfit: 0x1300070102030405, hat: 0x5a0009060708090a };

fn observed() -> Configuration {
    Configuration::from_observed(MiiClass::Brawler, TOKENS, [42; 88], ROWS).unwrap()
}

fn saved(value: Configuration) -> MiiConfig {
    MiiConfig {
        slot_id: 0, slot_index: 0, name: String::new(), charinfo: *value.charinfo(),
        raw_tags: [TaggedId(ROWS.outfit), TaggedId(ROWS.hat), TaggedId(0xc1000013fdfdfb2b)],
        canonical: value.canonical(), copies: Vec::new(),
    }
}

#[test]
fn persisted_bytes_follow_the_existing_wire_contract() {
    let value = observed();
    // Independent wire construction: 38 masked identity bytes, 50 appearance
    // bytes, four specials, colour/voice, then three little-endian hash40s.
    let canonical = format!("48{}{}00010201fffe05040302010a090807062bfbfdfd13", "00".repeat(38), "2a".repeat(50));
    let charinfo = std::iter::repeat_n("42", 88).collect::<Vec<_>>().join(",");
    let expected = format!(
        "{{\"canonical\":\"{canonical}\",\"charinfo\":[{charinfo}],\"tags\":[{},{},{}]}}",
        0x1300070102030405u64, 0x5a0009060708090au64, 0xc1000013fdfdfb2bu64,
    );
    assert_eq!(serde_json::to_vec(&value).unwrap(), expected.as_bytes());
    assert_eq!(serde_json::from_str::<Configuration>(&expected).unwrap(), value);
    assert_eq!(Configuration::from_saved(&saved(value)).unwrap(), value);
}

#[test]
fn equality_keeps_exact_bytes_and_hints_beyond_canonical_identity() {
    let original = observed();
    let mut identity_only = [42; 88];
    identity_only[0] = 43;
    let renamed = Configuration::from_observed(MiiClass::Brawler, TOKENS, identity_only, ROWS).unwrap();
    assert_eq!(original.canonical(), renamed.canonical());
    assert_ne!(original, renamed);
    let changed_hint = Configuration::from_observed(
        MiiClass::Brawler, Tokens { outfit: 8, ..TOKENS }, [42; 88],
        RowIds { outfit: 0x1300080102030405, ..ROWS },
    ).unwrap();
    assert_eq!(original.canonical(), changed_hint.canonical());
    assert_ne!(original, changed_hint);
    let copy = original;
    assert_eq!(original, copy);
}

#[test]
fn constructors_reject_values_that_cannot_be_reconstructed_losslessly() {
    for tokens in [
        Tokens { colour: 256, ..TOKENS }, Tokens { voice: 256, ..TOKENS },
        Tokens { specials: [0, 1, 3, 1], ..TOKENS }, Tokens { outfit: u16::MAX, ..TOKENS },
    ] {
        assert!(Configuration::from_observed(MiiClass::Brawler, tokens, [42; 88], ROWS).is_err());
    }
    let original = observed();
    let mut mismatched = saved(original);
    mismatched.charinfo[38] ^= 1;
    assert!(Configuration::from_saved(&mismatched).is_err());
    let mut mismatched = saved(original);
    mismatched.raw_tags[0].0 ^= 1;
    assert!(Configuration::from_saved(&mismatched).is_err());
    let mut mismatched = saved(original);
    mismatched.canonical.appearance.outfit |= 1 << 48;
    assert!(Configuration::from_saved(&mismatched).is_err());
    let mut mismatched = saved(original);
    mismatched.canonical.class = MiiClass::Gunner;
    assert!(Configuration::from_saved(&mismatched).is_err());
}

#[test]
fn actual_configuration_first_and_repeated_value_paths_allocate_nothing() {
    const CHILD: &str = "TR_CONFIGURATION_ALLOC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "actual_configuration_first_and_repeated_value_paths_allocate_nothing", "--test-threads=1", "--nocapture"])
            .env(CHILD, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    fn measure(count: usize) -> u64 {
        let before = hook_stats::hook_allocations();
        {
            let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
            for _ in 0..count {
                let value = observed();
                let restored = Configuration::from_saved(&saved(value)).unwrap();
                assert_eq!(ROWS.validate(restored), Ok(()));
                assert_eq!(value, restored);
                std::hint::black_box((restored.canonical(), restored.tokens(), *restored.charinfo()));
            }
        }
        hook_stats::hook_allocations() - before
    }
    let first = measure(1);
    let repeated = measure(64);
    let before = hook_stats::hook_allocations();
    {
        let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
        std::hint::black_box(vec![7u8; 1024]);
    }
    let control = hook_stats::hook_allocations() - before;
    println!("Configuration allocations first={first} repeated={repeated} positive_control={control}");
    assert!(control > 0);
    assert_eq!((first, repeated), (0, 0));
}
