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
use mii::{editor_row as value, inventory};
#[path = "../src/runtime/mii/mii_editor_bridge/draft.rs"]
mod draft;

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

fn editor_row() -> value::Row {
    let mut row = [0; 432];
    row[0x114] = 1;
    row[0x118] = b'A';
    row[0x130..0x138].copy_from_slice(&ROWS.outfit.to_le_bytes());
    row[0x138..0x140].copy_from_slice(&ROWS.hat.to_le_bytes());
    row[0x140] = 255;
    row[0x141] = 254;
    row[0x142..0x146].copy_from_slice(&[0, 1, 2, 1]);
    row[0x148..0x1a0].fill(42);
    row[0x1a0..0x1a8].copy_from_slice(&0xc1000013fdfdfb2bu64.to_le_bytes());
    row
}

#[test]
fn draft_refinement_preserves_error_order_and_independent_copies() {
    use draft::Draft;
    let pending = Draft::native_default(MiiClass::Brawler);
    assert_eq!(pending.row(), None);
    assert_eq!(pending.hash(), 0);
    assert_eq!(pending.validate(), Ok(()));
    let row = editor_row();
    let mut refined = pending;
    let mut identity = row;
    identity[0] = 1;
    assert_eq!(refined.materialize(MiiClass::Gunner, identity), Err("default_intent_changed"));
    assert_eq!(refined, pending);
    assert_eq!(refined.materialize(MiiClass::Brawler, identity), Err("default_identity_not_zero"));
    assert_eq!(refined, pending);
    let mut invalid = row;
    invalid[0x142] = 3;
    assert_eq!(refined.materialize(MiiClass::Brawler, invalid), Err("Invalid edited Mii"));
    assert_eq!(refined, pending);
    let mut other_class = row;
    other_class[0x1a0..0x1a8].copy_from_slice(&0xc100001257ab1d87u64.to_le_bytes());
    assert_eq!(refined.materialize(MiiClass::Brawler, other_class), Err("default_class_mismatch"));
    assert_eq!(refined, pending);
    assert_eq!(refined.materialize(MiiClass::Brawler, row), Ok(()));
    assert_eq!(pending.row(), None);
    assert_eq!(refined.row(), Some(&row));
    assert_eq!(refined.pending_class(), None);
    assert_eq!(refined.validate(), Ok(()));
    assert_eq!(refined, Draft::ready(row));
    let copy = refined;
    assert_eq!(refined.materialize(MiiClass::Brawler, identity), Err("default_intent_changed"));
    assert_eq!(refined, copy);
    let mut changed = row;
    changed[20] = 1; // A byte outside parsed appearance still belongs to the exact row.
    assert_ne!(refined, Draft::ready(changed));
    assert_ne!(pending, Draft::native_default(MiiClass::Gunner));
    assert!(Draft::ready([0; 432]).validate().is_err());
}

#[test]
fn editor_value_validation_preserves_errors_and_lossy_name_behavior() {
    let row = editor_row();
    assert_eq!(value::parse(&row).unwrap(), ("A".into(), observed()));
    for (offset, byte, expected) in [
        (0, 1, "Invalid owned editor row"),
        (0x114, 0, "Invalid edited Mii"), (0x114, 11, "Invalid edited Mii"),
        (0x137, 0, "Invalid edited Mii"), (0x13f, 0, "Invalid edited Mii"),
        (0x1a7, 0, "Invalid edited Mii"), (0x142, 3, "Invalid edited Mii"),
    ] {
        let mut invalid = row;
        invalid[offset] = byte;
        assert_eq!(value::configuration(&invalid), Err(expected));
        assert_eq!(value::parse(&invalid), Err(expected));
    }
    let mut bad_hint = row;
    bad_hint[0x135..0x137].fill(255);
    assert_eq!(value::configuration(&bad_hint), Err("Edited Mii resources differ"));
    let mut lossy = row;
    lossy[0x118..0x11a].copy_from_slice(&0xd800u16.to_le_bytes());
    assert_eq!(value::parse(&lossy).unwrap(), ("\u{fffd}".into(), observed()));
    assert_eq!(value::configuration(&lossy), Ok(observed()));
    let mut id = row;
    id[0] = 9;
    assert_eq!(mii::saved::parse_header(0, 0, &id).unwrap_err(), "filename id 0 differs from header id 9");
}

#[test]
fn native_default_seed_and_saved_name_paths_preserve_bytes() {
    let mut seed = saved(observed());
    let mut expected = editor_row();
    expected[0x114] = 7;
    expected[0x118..0x12c].fill(0);
    for (i, byte) in b"New Mii".iter().enumerate() { expected[0x118 + i * 2] = *byte; }
    assert_eq!(value::from_seed(&seed), Ok(expected));
    for (name, expected_name) in [
        ("123456789\u{1f600}", "123456789"), ("12345678\u{1f600}", "12345678\u{1f600}"),
        ("\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}x", "\u{1f600}\u{1f600}\u{1f600}\u{1f600}\u{1f600}"),
        ("", "New Mii"),
    ] {
        seed.name = name.into();
        let row = value::from_seed(&seed).unwrap();
        assert_eq!(value::parse(&row).unwrap().0, expected_name);
        assert_eq!(value::configuration(&row), Ok(observed()));
    }
}

#[test]
fn actual_seed_first_and_repeated_valid_and_invalid_paths_allocate_nothing() {
    const CHILD: &str = "TR_SEED_ALLOC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "actual_seed_first_and_repeated_valid_and_invalid_paths_allocate_nothing", "--test-threads=1", "--nocapture"])
            .env(CHILD, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    let default = saved(observed());
    let mut named = saved(observed());
    named.name = "12345678\u{1f600}x".into();
    let mut invalid = saved(observed());
    invalid.canonical.appearance.specials[0] = 3;
    let measure = |count| {
        let before = hook_stats::hook_allocations();
        {
            let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
            for _ in 0..count {
                std::hint::black_box(value::from_seed(&default).unwrap());
                std::hint::black_box(value::from_seed(&named).unwrap());
                assert_eq!(value::from_seed(&invalid), Err("Invalid editor appearance"));
            }
        }
        hook_stats::hook_allocations() - before
    };
    let first = measure(1);
    let repeated = measure(64);
    let before = hook_stats::hook_allocations();
    {
        let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
        std::hint::black_box(vec![7u8; 1024]);
    }
    let control = hook_stats::hook_allocations() - before;
    println!("Seed allocations first={first} repeated={repeated} positive_control={control}");
    assert!(control > 0);
    assert_eq!((first, repeated), (0, 0));
}

#[test]
fn actual_draft_first_and_repeated_refinement_allocate_nothing() {
    const CHILD: &str = "TR_DRAFT_ALLOC_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "actual_draft_first_and_repeated_refinement_allocate_nothing", "--test-threads=1", "--nocapture"])
            .env(CHILD, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    let row = editor_row();
    let measure = |count| {
        let before = hook_stats::hook_allocations();
        {
            let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
            for _ in 0..count {
                let mut pending = draft::Draft::native_default(MiiClass::Brawler);
                pending.materialize(MiiClass::Brawler, row).unwrap();
                assert_eq!(pending.validate(), Ok(()));
                std::hint::black_box((pending, pending.hash()));
                for (offset, byte) in [(0, 1), (0x114, 0), (0x137, 0), (0x142, 3)] {
                    let mut invalid = row;
                    invalid[offset] = byte;
                    assert!(draft::Draft::ready(invalid).validate().is_err());
                    let mut refused = draft::Draft::native_default(MiiClass::Brawler);
                    assert!(refused.materialize(MiiClass::Brawler, invalid).is_err());
                    assert_eq!(refused.row(), None);
                }
            }
        }
        hook_stats::hook_allocations() - before
    };
    let first = measure(1);
    let repeated = measure(64);
    let before = hook_stats::hook_allocations();
    {
        let _scope = hook_stats::HookScope::enter_hook(hook_stats::HookId::Other);
        std::hint::black_box(vec![7u8; 1024]);
    }
    let control = hook_stats::hook_allocations() - before;
    println!("Draft allocations first={first} repeated={repeated} positive_control={control}");
    assert!(control > 0);
    assert_eq!((first, repeated), (0, 0));
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
