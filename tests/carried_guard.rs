//! Host tests of the 64-byte site guard `src/model/guard.rs`, carried over from the earlier fork's
//! `training_mod_consts/src/native_seams/tests.rs` (stage m5o, ours-new): lines 7-96, 343-373 and
//! 401-445, with `use super::*` replaced by the by-path include below. The fork's table-wide tests
//! (rows well formed, card-scan export, ELF check over its rows) are not carried; this plugin's own
//! site table has its tests in `tests/core01_sites.rs`. See CARRY-OVER.md.

#[path = "../src/model/guard.rs"]
#[allow(dead_code)]
mod guard;

use guard::*;
const HOOK: SeamKind = SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry };
const INLINE: SeamKind = SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid };

/// 16 distinct non-hook words (`add x<i>, x<i>, #<i>` style, never `b`/`ldr x17`/`br`/`nop`).
fn stock() -> [u8; GUARD_BYTES] {
    let mut bytes = [0u8; GUARD_BYTES];
    for i in 0..WORDS {
        let word = 0x9100_0000u32 | ((i as u32) << 10) | (i as u32);
        bytes[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

fn with_words(mut bytes: [u8; GUARD_BYTES], at_word: usize, new: &[u32]) -> [u8; GUARD_BYTES] {
    for (k, word) in new.iter().enumerate() {
        let i = at_word + k;
        if i < WORDS {
            bytes[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}

const B: u32 = 0x1400_1234;
const FAR: [u32; 4] = [LDR_X17_LITERAL_8, BR_X17, 0x8100_0000, 0x0000_0000];
const NOP_FAR: [u32; 5] = [NOP, LDR_X17_LITERAL_8, BR_X17, 0x8100_0000, 0x0000_0000];
/// The `.quad` both far jumps above carry (little endian words 0x8100_0000, 0).
const FAR_TARGET: u64 = 0x8100_0000;

#[test]
fn identical_bytes_are_stock_for_every_kind() {
    for kind in [HOOK, INLINE, SeamKind::Called, SeamKind::Relied] {
        assert_eq!(guard_outcome(&stock(), &stock(), kind), GuardOutcome::Stock);
    }
    assert!(GuardOutcome::Stock.usable());
    assert_eq!(GuardOutcome::Stock.code(), "stock");
}

#[test]
fn another_plugins_hook_in_the_words_we_replace_occupies_a_hook_site() {
    for (shape, len) in [(&[B][..], 1), (&FAR[..], 4), (&NOP_FAR[..], 5)] {
        let actual = with_words(stock(), 0, shape);
        let outcome = guard_outcome(&stock(), &actual, HOOK);
        assert_eq!(outcome, GuardOutcome::HookedByOther { at: 0, usable: false }, "shape len {}", len);
        assert_eq!(outcome.code(), "occupied");
        // The same change on a function we only call is fine.
        assert_eq!(guard_outcome(&stock(), &actual, SeamKind::Called), GuardOutcome::HookedByOther { at: 0, usable: true });
    }
    // A far jump that starts inside our span but runs past it still takes our words.
    let actual = with_words(stock(), 3, &FAR);
    assert_eq!(guard_outcome(&stock(), &actual, INLINE), GuardOutcome::HookedByOther { at: 12, usable: false });
}

#[test]
fn hooks_past_our_span_are_allowed_and_reported() {
    let actual = with_words(stock(), HOOK_SPAN_WORDS, &FAR);
    let outcome = guard_outcome(&stock(), &actual, HOOK);
    assert_eq!(outcome, GuardOutcome::HookedByOther { at: 4 * HOOK_SPAN_WORDS, usable: true });
    assert_eq!(outcome.code(), "other");
    // A far jump cut by the end of the window is still recognised.
    let actual = with_words(stock(), WORDS - 2, &FAR);
    assert_eq!(guard_outcome(&stock(), &actual, HOOK), GuardOutcome::HookedByOther { at: 4 * (WORDS - 2), usable: true });
    // Two hooks: the first decides `at`, any blocking one decides `usable`.
    let actual = with_words(with_words(stock(), 9, &[B]), 2, &[B]);
    assert_eq!(guard_outcome(&stock(), &actual, HOOK), GuardOutcome::HookedByOther { at: 8, usable: false });
}

#[test]
fn a_relied_on_word_must_stay_but_hooks_after_it_are_fine() {
    let at0 = with_words(stock(), 0, &[B]);
    assert_eq!(guard_outcome(&stock(), &at0, SeamKind::Relied), GuardOutcome::HookedByOther { at: 0, usable: false });
    let at1 = with_words(stock(), 1, &FAR);
    assert_eq!(guard_outcome(&stock(), &at1, SeamKind::Relied), GuardOutcome::HookedByOther { at: 4, usable: true });
}

#[test]
fn anything_that_is_not_a_hook_shape_is_foreign() {
    for kind in [HOOK, INLINE, SeamKind::Called, SeamKind::Relied] {
        let actual = with_words(stock(), 7, &[0xd280_0000]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 28 });
        // `ldr x17` without its `br x17` is not a far jump.
        let actual = with_words(stock(), 2, &[LDR_X17_LITERAL_8, 0xd503_201f]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 8 });
        // A change after a recognised hook is still foreign.
        let actual = with_words(with_words(stock(), 0, &[B]), 12, &[0]);
        assert_eq!(guard_outcome(&stock(), &actual, kind), GuardOutcome::Foreign { first_diff: 48 });
    }
    assert!(!GuardOutcome::Foreign { first_diff: 0 }.usable());
    assert_eq!(GuardOutcome::Foreign { first_diff: 0 }.code(), "foreign");
}

#[test]
fn an_own_hook_row_reads_stock_only_for_our_registered_far_jump() {
    let own = Sharing::OwnHook { hook: "needle.x" };
    let rule = |actual: &[u8; GUARD_BYTES], target: Option<u64>| outcome_for_own(&stock(), actual, SeamKind::Called, own, target);
    for far in [&FAR[..], &NOP_FAR[..]] {
        let ours = with_words(stock(), 0, far);
        assert_eq!(entry_far_jump_target(&ours), Some(FAR_TARGET));
        assert_eq!(rule(&ours, Some(FAR_TARGET)), GuardOutcome::Stock);
        // Another plugin's far jump (another target), or no registration: `other` as before.
        assert_eq!(rule(&ours, Some(FAR_TARGET ^ 0x10)), GuardOutcome::HookedByOther { at: 0, usable: true });
        assert_eq!(rule(&ours, None), GuardOutcome::HookedByOther { at: 0, usable: true });
        // Our far jump plus another hook further in: there is another plugin there, so not stock.
        let crowded = with_words(ours, 8, &[B]);
        assert_eq!(rule(&crowded, Some(FAR_TARGET)), outcome_for(&stock(), &crowded, SeamKind::Called, own));
        assert_ne!(rule(&crowded, Some(FAR_TARGET)), GuardOutcome::Stock);
        // Our far jump plus a non-hook change stays foreign.
        let changed = with_words(ours, 9, &[0xd280_0000]);
        assert_eq!(rule(&changed, Some(FAR_TARGET)), GuardOutcome::Foreign { first_diff: 36 });
    }
    // A one-word branch at the entry has no target to compare.
    let branch = with_words(stock(), 0, &[B]);
    assert_eq!(entry_far_jump_target(&branch), None);
    assert_eq!(rule(&branch, Some(FAR_TARGET)), outcome_for(&stock(), &branch, SeamKind::Called, own));
    assert_eq!(rule(&stock(), Some(FAR_TARGET)), GuardOutcome::Stock);
    // Other sharing rules ignore the target.
    let far = with_words(stock(), 0, &FAR);
    assert_eq!(
        outcome_for_own(&stock(), &far, SeamKind::Called, Sharing::None, Some(FAR_TARGET)),
        GuardOutcome::HookedByOther { at: 0, usable: true }
    );
}

#[test]
fn a_chain_row_accepts_a_far_jump_at_its_entry_and_nothing_else() {
    let chain = |actual: &[u8; GUARD_BYTES]| outcome_for(&stock(), actual, HOOK, Sharing::Chain);
    for far in [&FAR[..], &NOP_FAR[..]] {
        let actual = with_words(stock(), 0, far);
        assert_eq!(chain(&actual), GuardOutcome::HookedByOther { at: 0, usable: true });
        assert!(far_jump_at(&actual, 0));
        // Without the chain rule the same bytes occupy the entry.
        assert_eq!(outcome_for(&stock(), &actual, HOOK, Sharing::None), GuardOutcome::HookedByOther { at: 0, usable: false });
    }
    // A one-word branch at the entry is not chainable.
    assert_eq!(chain(&with_words(stock(), 0, &[B])), GuardOutcome::HookedByOther { at: 0, usable: false });
    // A far jump plus another hook inside our span after it is not chainable either.
    let crowded = with_words(with_words(stock(), 0, &FAR), 4, &[B]);
    assert!(!chain(&crowded).usable());
    // A non-hook change stays foreign.
    let foreign = with_words(with_words(stock(), 0, &FAR), 9, &[0xd280_0000]);
    assert_eq!(chain(&foreign), GuardOutcome::Foreign { first_diff: 36 });
    assert_eq!(chain(&stock()), GuardOutcome::Stock);
}

/// `Sharing::EntryOrBody` (SEAMS-ALL): the entry is ours when stock; another plugin's far jump over the
/// entry that ends before the body, with every other word stock, is `Shared` (we take the body); a
/// one-word branch, a far jump reaching the body, or any further change stays unusable.
#[test]
fn an_entry_or_body_row_is_stock_or_shared_and_nothing_else() {
    let rule = |actual: &[u8; GUARD_BYTES]| entry_or_body_outcome(&stock(), actual, HOOK, 4);
    assert_eq!(rule(&stock()), GuardOutcome::Stock);
    let far = with_words(stock(), 0, &FAR);
    assert_eq!(rule(&far), GuardOutcome::Shared);
    assert!(GuardOutcome::Shared.usable());
    assert_eq!(GuardOutcome::Shared.code(), "shared");
    // The same far jump with its `nop` covers the body's first word: not shared.
    assert_eq!(rule(&with_words(stock(), 0, &NOP_FAR)), GuardOutcome::HookedByOther { at: 0, usable: false });
    // A body further on leaves room for the `nop` form.
    assert_eq!(entry_or_body_outcome(&stock(), &with_words(stock(), 0, &NOP_FAR), HOOK, 5), GuardOutcome::Shared);
    // A one-word branch at the entry.
    assert_eq!(rule(&with_words(stock(), 0, &[B])), GuardOutcome::HookedByOther { at: 0, usable: false });
    // The far jump plus a hook in the body, or plus a non-hook change.
    assert!(!rule(&with_words(far, 4, &[B])).usable());
    assert!(!rule(&with_words(far, 12, &[B])).usable());
    assert_eq!(rule(&with_words(far, 9, &[0xd280_0000])), GuardOutcome::Foreign { first_diff: 36 });
    // Without the rule the far jump occupies the entry hook (the M5h projection, `Sharing::None`).
    assert_eq!(outcome_for(&stock(), &far, HOOK, Sharing::None), GuardOutcome::HookedByOther { at: 0, usable: false });
}
