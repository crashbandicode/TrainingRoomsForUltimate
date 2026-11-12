//! Goldens for WP1 (WORKPLAN §2.2 (a)): every derived predicate equals the body it replaced, over
//! every kind, form, policy and window shape, except the intended deviations asserted where they
//! occur (`CellRule::PinCell0`: Kirby is no pair-lead extra, `0x4b` is a Solo Popo member — row 4;
//! `ExitRule::SelfCell`: a one-member window has a commit index — row 2). The old bodies live in
//! `oracle` below, verbatim from `body_bank.rs` as of stage c10ce3df (B123-rc); they are deleted
//! when WP1 is accepted.

lib_unit_tests! {
use super::*;
use crate::engine::swap::body_bank::{GroupPolicy, Members, Special, SpecialPolicy, KIRBY, NANA, POPO};
use crate::engine::swap::character_forms::Forms;
use crate::engine::mii::inventory::MiiClass;

mod oracle {
    use crate::engine::swap::body_bank::{GroupPolicy, Members, Special, SpecialPolicy, KIRBY, NANA, NATIVE_MEMBERS, POPO};
    use crate::engine::swap::character_forms::Forms;
    use crate::engine::swap::trio::TrioFighter;

    pub fn bankable_kind(kind: u32) -> bool {
        TrioFighter::from_kind(kind).is_some()
            && !(0x48..=0x4a).contains(&kind)
            && Forms::starting(kind).count == 1
    }

    pub fn kirby_parkable(copy_flag: bool, retained_copy_kinds: Option<u128>, hat_lease_on_body: bool)
        -> Result<(), &'static str> {
        if copy_flag {
            return Err("Kirby holds a copy ability");
        }
        match retained_copy_kinds {
            None => return Err("Kirby copy resources are not readable"),
            Some(0) => {}
            Some(_) => return Err("Kirby still retains copy resources"),
        }
        if hat_lease_on_body {
            return Err("a hat lease names this Kirby");
        }
        Ok(())
    }

    pub fn prefetch_admitted(live_kind: Option<u32>) -> Result<(), &'static str> {
        if live_kind == Some(KIRBY) { Err("live Kirby may copy a group kind") } else { Ok(()) }
    }

    pub fn key_release_admitted(kind: u32, live_copy_kind: Option<u32>) -> Result<(), &'static str> {
        if live_copy_kind == Some(kind) { Err("live Kirby copies this kind") } else { Ok(()) }
    }

    pub fn pair_request(count: u32, kinds: [u32; 3], actors: u32) -> bool {
        let pair = Forms::starting(POPO);
        count == pair.count && kinds == pair.kinds && actors == pair.actor_count
    }

    pub fn pair_lead_extra(kind: u32) -> bool {
        bankable_kind(kind) && kind != POPO && kind != NANA
    }

    pub fn member_forms(kind: u32) -> Forms {
        if kind == 0x4b { Forms::solo_popo() } else { Forms::starting(kind) }
    }

    pub fn member_forms_in(count: u32, kinds: [u32; 3], actor_count: u32, kind: u32) -> Forms {
        let pair_lead = count == NATIVE_MEMBERS as u32 && kinds[0] == POPO && kinds[1] == NANA && actor_count == 2;
        if pair_lead && kind == POPO { Forms::starting(POPO) } else { member_forms(kind) }
    }

    pub fn single_member_request(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
        let forms = member_forms(kinds[0]);
        ((bankable_kind(kinds[0]) || kinds[0] == 0x4b)
            && count == forms.count && kinds == forms.kinds && actors == forms.actor_count)
            .then_some(forms)
    }

    pub fn native_group_request(count: u32, kinds: [u32; 3], actors: u32) -> Option<Forms> {
        if !matches!(kinds[0], 0x24..=0x26 | 0x5a | 0x5b) { return None; }
        let forms = Forms::starting(kinds[0]);
        (count == forms.count && kinds == forms.kinds && actors == forms.actor_count).then_some(forms)
    }

    pub fn special_group_kind(kind: u32, policy: SpecialPolicy) -> bool {
        Members::special_group(kind, policy).is_some()
    }

    pub fn swappable_kind(kind: u32) -> bool {
        bankable_kind(kind)
            && !(0x4b..=0x4c).contains(&kind)
            && !(0x4e..=0x50).contains(&kind)
            && !(0x6e..=0x75).contains(&kind)
    }

    pub fn original_bankable(kind: u32) -> bool {
        swappable_kind(kind)
    }

    pub fn group_member_kind(kind: u32, policy: GroupPolicy) -> bool {
        swappable_kind(kind) || (policy.solo_popo_groups && kind == POPO)
    }

    pub fn window_swappable(window: Members) -> bool {
        window_swappable_with(window, GroupPolicy::DEFAULT)
    }

    pub fn window_swappable_with(window: Members, policy: GroupPolicy) -> bool {
        match window.special() {
            Special::Native(_) | Special::Pair => true,
            Special::Mii => false,
            Special::None if window.is_pair_lead() => policy.ic_pair_groups && swappable_kind(window.kinds()[2]),
            Special::None => window.kinds()[..window.count()].iter().enumerate()
                .all(|(cell, kind)| swappable_kind(*kind) || (cell == 0 && *kind == POPO)),
        }
    }

    pub fn swap_commit_index(from: u8, count: usize) -> Option<u8> {
        (count >= 2 && usize::from(from) < count).then(|| ((usize::from(from) + 1) % count) as u8)
    }

    /// `Members::swap_commit_index_for` as of c10ce3df, frozen here (the method itself now gives a
    /// one-member window its self-commit, row 2).
    pub fn swap_commit_index_in(window: Members, from: u8) -> Option<u8> {
        match window.special() {
            Special::Pair => (from == 0).then_some(1),
            _ if window.is_pair_lead() => match from { 0 => Some(2), 2 => Some(0), _ => None },
            _ => swap_commit_index(from, window.count()),
        }
    }
}

/// Every roster kind, the gaps between them, the Ice Climbers pair, Nana, both object kinds and a
/// few out-of-range values.
fn kinds() -> impl Iterator<Item = u32> {
    (0u32..=0x90).chain([0x100, 0xffff, u32::MAX])
}

fn group_policies() -> [GroupPolicy; 4] {
    [
        GroupPolicy::DEFAULT,
        GroupPolicy { solo_popo_groups: true, ic_pair_groups: false },
        GroupPolicy { solo_popo_groups: false, ic_pair_groups: true },
        GroupPolicy { solo_popo_groups: true, ic_pair_groups: true },
    ]
}

fn special_policies() -> [SpecialPolicy; 6] {
    [SpecialPolicy::NONE, SpecialPolicy::NATIVE, SpecialPolicy::ALL, SpecialPolicy::MII,
        SpecialPolicy { native: false, pair: true, mii: false }, SpecialPolicy { native: false, pair: false, mii: true }]
}

/// Descriptor words a request may carry for `kind`: its native expansion, Solo, and perturbations
/// (count, actors, rotated kinds, truncated kinds, a Nana partner, the single-body shapes).
fn request_words(kind: u32) -> Vec<(u32, [u32; 3], u32)> {
    let s = Forms::starting(kind);
    vec![
        (s.count, s.kinds, s.actor_count),
        (1, [kind, u32::MAX, u32::MAX], 1),
        (1, [kind, u32::MAX, u32::MAX], 2),
        (s.count + 1, s.kinds, s.actor_count),
        (s.count, s.kinds, 3 - s.actor_count),
        (s.count, [s.kinds[1], s.kinds[0], s.kinds[2]], s.actor_count),
        (s.count, [s.kinds[0], u32::MAX, u32::MAX], s.actor_count),
        (2, [kind, NANA, u32::MAX], 2),
        (2, [kind, NANA, u32::MAX], 1),
        (3, [kind, NANA, 0x00], 2),
    ]
}

/// Every window shape the bank builds: one to three ordinary members (R.O.B. and Kirby included),
/// Solo-led windows and the one-member Solo group, pair-lead windows with every extra, the two
/// native groups and the pair group.
fn windows() -> Vec<Members> {
    let singles = [0x00u32, 0x02, 0x03, 0x06, 0x07, 0x08, 0x1c, 0x2d, 0x59];
    let mut out = Vec::new();
    // Rotations are laid out (`Members::laid_out`): Kirby takes cell 0 whatever its rotation
    // position, so rotations that differ only in Kirby's position yield one layout (deduplicated).
    fn push(out: &mut Vec<Members>, members: Members) { if !out.contains(&members) { out.push(members); } }
    for &a in &singles {
        push(&mut out, Members::laid_out(&[a]).unwrap());
        for &b in &singles {
            if b == a { continue; }
            push(&mut out, Members::laid_out(&[a, b]).unwrap());
            for &c in &singles {
                if c == a || c == b { continue; }
                push(&mut out, Members::laid_out(&[a, b, c]).unwrap());
            }
        }
        // The pair-lead window never takes the cell-0-pinned Kirby as its extra.
        if let Some(lead) = Members::pair_lead(a) { push(&mut out, lead); }
    }
    out.push(Members::new(&[POPO]).unwrap());
    out.push(Members::from_forms(Forms::solo_popo(), &[0x02], 1).unwrap());
    out.push(Members::from_forms(Forms::solo_popo(), &[0x02, 0x07], 2).unwrap());
    // Row 4: an upcoming Solo Popo pinned to cell 0 under another target ([4b, 5, 7], Yoshi's).
    push(&mut out, Members::from_cycle(0x05, &[POPO, 0x07], 2).unwrap());
    out.push(Members::native_group(0x24).unwrap());
    out.push(Members::native_group(0x5b).unwrap());
    out.push(Members::pair_group());
    out
}

fn derived_predicates_match_todays_scattered_gates() {
    for kind in kinds() {
        if kind == POPO {
            // Intended deviation (row 4, `CellRule::PinCell0`): `0x4b` has a single-body member
            // form, Solo Popo, laid out at cell 0; the old body admitted Popo only as the target.
            assert!(bankable_kind(kind) && !oracle::bankable_kind(kind));
        } else {
            assert_eq!(bankable_kind(kind), oracle::bankable_kind(kind), "bankable_kind {kind:#x}");
        }
        assert_eq!(swappable_kind(kind), oracle::swappable_kind(kind), "swappable_kind {kind:#x}");
        assert_eq!(original_bankable(kind), oracle::original_bankable(kind), "original_bankable {kind:#x}");
        if kind == KIRBY {
            // Intended deviation from the old body (CellRule::PinCell0): the pair holds cell 0.
            assert!(!pair_lead_extra(kind) && oracle::pair_lead_extra(kind));
        } else {
            assert_eq!(pair_lead_extra(kind), oracle::pair_lead_extra(kind), "pair_lead_extra {kind:#x}");
        }
        assert_eq!(member_forms(kind), oracle::member_forms(kind), "member_forms {kind:#x}");
        assert_eq!(prefetch_admitted(Some(kind)), oracle::prefetch_admitted(Some(kind)), "prefetch_admitted {kind:#x}");
        for copy in [None, Some(kind), Some(kind ^ 1), Some(0x00)] {
            assert_eq!(key_release_admitted(kind, copy), oracle::key_release_admitted(kind, copy), "key_release {kind:#x} {copy:?}");
        }
        for policy in group_policies() {
            assert_eq!(group_member_kind(kind, policy), oracle::group_member_kind(kind, policy), "group_member_kind {kind:#x} {policy:?}");
        }
        for policy in special_policies() {
            assert_eq!(special_group_kind(kind, policy), oracle::special_group_kind(kind, policy), "special_group_kind {kind:#x} {policy:?}");
        }
        for (count, words, actors) in request_words(kind) {
            assert_eq!(pair_request(count, words, actors), oracle::pair_request(count, words, actors), "pair_request {kind:#x} {count} {words:x?} {actors}");
            assert_eq!(native_group_request(count, words, actors), oracle::native_group_request(count, words, actors), "native_group_request {kind:#x} {count} {words:x?} {actors}");
            assert_eq!(single_member_request(count, words, actors), oracle::single_member_request(count, words, actors), "single_member_request {kind:#x} {count} {words:x?} {actors}");
            for member in words {
                assert_eq!(member_forms_in(count, words, actors, member), oracle::member_forms_in(count, words, actors, member), "member_forms_in {count} {words:x?} {actors} {member:#x}");
            }
        }
    }
    assert_eq!(prefetch_admitted(None), oracle::prefetch_admitted(None));
    for copy in [false, true] {
        for retained in [None, Some(0u128), Some(1), Some(1 << 0x07), Some(1 << 0x2d)] {
            for lease in [false, true] {
                assert_eq!(kirby_parkable(copy, retained, lease), oracle::kirby_parkable(copy, retained, lease), "{copy} {retained:?} {lease}");
            }
        }
    }
    for count in 0..=4usize {
        for from in 0..5u8 {
            assert_eq!(swap_commit_index(from, count), oracle::swap_commit_index(from, count), "swap_commit_index {from} {count}");
        }
    }
    let shapes = windows();
    // 9 singles, 64 pairs and 392 triples after the Kirby layouts collapse (rotations that differ only
    // in Kirby's position are one physical layout), 8 pair-lead windows, Solo and the special shapes.
    assert!(shapes.len() >= 470, "the window set covers singles, pairs, triples and every special shape: {}", shapes.len());
    for window in shapes {
        assert_eq!(window_swappable(window), oracle::window_swappable(window), "window_swappable {window:?}");
        for policy in group_policies() {
            assert_eq!(window_swappable_with(window, policy), oracle::window_swappable_with(window, policy), "window_swappable_with {window:?} {policy:?}");
        }
        for from in 0..4u8 {
            if window.count() == 1 {
                // Intended deviation (row 2, `ExitRule::SelfCell`): a one-member window commits to
                // its own cell; the old body had no commit index for it (canonical Reload).
                assert_eq!(swap_commit_index_in(window, from), (from == 0).then_some(0), "{window:?} {from}");
                assert_eq!(oracle::swap_commit_index_in(window, from), None, "{window:?} {from}");
            } else {
                assert_eq!(swap_commit_index_in(window, from), oracle::swap_commit_index_in(window, from), "swap_commit_index_in {window:?} {from}");
            }
            assert_eq!(swap_commit_index_in(window, from), window.swap_commit_index_for(from), "the window's own view agrees {window:?} {from}");
        }
        // Every window shape can be left warm (the old `window_exit_warm` is gone with the Reload).
        assert!(swap_commit_index_in(window, 0).is_some(), "{:?}", window);
        let words = (window.count() as u32, window.kinds(), window.actor_count());
        for member in window.kinds().iter().take(window.count()) {
            assert_eq!(member_forms_in(words.0, words.1, words.2, *member), oracle::member_forms_in(words.0, words.1, words.2, *member), "{window:?} {member:#x}");
            // The descriptor-word rule agrees with the window's own `forms_of` for every window the
            // adapters ask about; the detached pair group's words are the canonical pair's, which
            // `member_forms_in` reads as a non-bank window (its existing test), and no adapter asks.
            if window.special() != Special::Pair {
                assert_eq!(member_forms_in(words.0, words.1, words.2, *member), window.forms_of(*member), "forms_of agrees with the descriptor-word rule for {window:?} {member:#x}");
            }
        }
    }
}

fn the_pair_and_solo_popo_are_distinct_classes_and_targets() {
    assert_eq!(form_class(POPO, Forms::starting(POPO)), FormClass::IcPair);
    assert_eq!(form_class(POPO, Forms::solo_popo()), FormClass::SoloPopo);
    let original_pair = Target::new(Selection::Original, Forms::starting(POPO), None);
    let numbered_solo = Target::new(Selection::Numbered(2), Forms::solo_popo(), None);
    assert_eq!((original_pair.kind, numbered_solo.kind), (POPO, POPO));
    assert!(!original_pair.same_fighter(&numbered_solo), "Astra §6.1: the original pair and a numbered Solo Popo never collapse");
    assert!(original_pair.is_pair() && !original_pair.is_solo_popo());
    assert!(numbered_solo.is_solo_popo() && !numbered_solo.is_pair());
    assert_eq!(original_pair.request_words(), (2, [POPO, NANA, u32::MAX], 2));
    assert_eq!(numbered_solo.request_words(), (1, [POPO, u32::MAX, u32::MAX], 1));
    assert!(original_pair.capabilities().pair_rule && !numbered_solo.capabilities().pair_rule);
    // From descriptor words: only words that spell a form make a target.
    assert_eq!(Target::from_request(Selection::Direct, 2, [POPO, NANA, u32::MAX], 2, None).map(|t| t.class()), Some(FormClass::IcPair));
    assert_eq!(Target::from_request(Selection::Direct, 1, [POPO, u32::MAX, u32::MAX], 1, None).map(|t| t.class()), Some(FormClass::SoloPopo));
    assert!(Target::from_request(Selection::Direct, 2, [POPO, NANA, u32::MAX], 1, None).is_none());
    assert!(Target::from_request(Selection::Replay, 3, [0x25, 0x24, 0x26], 1, None).is_none(), "rotated order is not the expansion of 0x25");
    assert_eq!(Target::from_request(Selection::Replay, 3, [0x25, 0x26, 0x24], 1, None).map(|t| (t.kind, t.class())), Some((0x25, FormClass::PokemonTrainer)));
    // The same fighter under another selection is the same fighter.
    let direct_pair = Target::new(Selection::Direct, Forms::starting(POPO), None);
    assert!(original_pair.same_fighter(&direct_pair) && original_pair != direct_pair);
    // Mii classes and foreign kinds.
    assert_eq!(form_class(0x4a, Forms::starting(0x4a)), FormClass::Mii(MiiClass::Gunner));
    assert_eq!(form_class(0x48, Forms::starting(0x48)), FormClass::Mii(MiiClass::Brawler));
    for kind in [NANA, 0x4d, 0x4e, 0x50, 0x6e, 0x75, 0x5e, u32::MAX] {
        assert_eq!(form_class(kind, Forms::starting(kind)), FormClass::Foreign, "{kind:#x}");
    }
    assert_eq!(form_class(KIRBY, Forms::starting(KIRBY)), FormClass::Kirby);
    assert_eq!(form_class(ROB, Forms::starting(ROB)), FormClass::Rob);
}

fn the_kirby_fallback_row_makes_kirby_canonical_only_without_touching_the_shipped_row() {
    // Shipped (B123-pre rows, E4a–E4d): Kirby is an ordinary bank, group and original member, laid
    // out at cell 0 (`CellRule::PinCell0`); the pair-lead window, whose cell 0 is the pair, never
    // takes it as the extra.
    assert!(bankable_kind(KIRBY) && swappable_kind(KIRBY) && original_bankable(KIRBY));
    assert!(!pair_lead_extra(KIRBY) && group_member_kind(KIRBY, GroupPolicy::DEFAULT));
    assert_eq!(capabilities(FormClass::Kirby).qualification, Qualification::Shipped);
    // WORKPLAN §2.14 step 6 / §5.3: one row flip makes Kirby canonical-only; hats and the park
    // gate are untouched by it (they are not derived from these predicates).
    let gated = Capabilities {
        qualification: Qualification::Gated("D-HANG1/D-KIRBY1 class unproven"),
        ..capabilities(FormClass::Kirby)
    };
    assert!(!single_body_member_of(&gated));
    for cell in 0..3 { assert!(!can_window_member_of(&gated, cell)); }
    for policy in group_policies() { assert!(!can_group_member_of(&gated, policy)); }
    assert_eq!(departure_admitted(FormClass::Kirby, false, Some(0), false), Ok(()));
    // Every row other than `Foreign` is `Shipped` (the Mii row since M0 merged the Mii groups).
    for class in [FormClass::Ordinary, FormClass::Kirby, FormClass::Rob, FormClass::Mii(MiiClass::Gunner), FormClass::PokemonTrainer, FormClass::Aegis, FormClass::IcPair, FormClass::SoloPopo] {
        assert_eq!(capabilities(class).qualification, Qualification::Shipped, "{class:?}");
    }
    assert!(matches!(capabilities(FormClass::Foreign).qualification, Qualification::Gated(_)));
}

fn kirby_is_laid_out_at_cell_0_whatever_its_rotation_position() {
    // RE-KIRBY-UNLOAD §0–§3 (K-R1 on Eden): native's copy registry assumes Kirby is sub 0 of its
    // entry. The planner keeps the player's rotation and changes only the physical layout; the
    // target activates through its own cell.
    const MARIO: u32 = 0x00;
    const YOSHI: u32 = 0x05;
    const FOX: u32 = 0x07;
    assert_eq!(capabilities(FormClass::Kirby).window_cell, CellRule::PinCell0);
    // Solo Popo (`0x4b` as a member) pins too since row 4; Nana and the pair never are members.
    assert!(pins_cell0(KIRBY) && pins_cell0(POPO) && !pins_cell0(FOX) && !pins_cell0(NANA) && !pins_cell0(u32::MAX));
    for (rotation, cells, target_cell) in [
        ([YOSHI, KIRBY, FOX], [KIRBY, YOSHI, FOX], 1u8),
        ([YOSHI, FOX, KIRBY], [KIRBY, YOSHI, FOX], 1),
        ([KIRBY, YOSHI, FOX], [KIRBY, YOSHI, FOX], 0),
        ([MARIO, KIRBY, FOX], [KIRBY, MARIO, FOX], 1),
    ] {
        let members = Members::from_cycle(rotation[0], &rotation[1..], 2).unwrap();
        assert_eq!(members.kinds(), cells, "{:x?}", rotation);
        assert_eq!(members.index_of(rotation[0]), Some(target_cell), "{:x?}", rotation);
        assert!(!self_mirror_hazard(&members.kinds()), "{:x?}", rotation);
        // The other members keep their rotation order.
        let others: Vec<u32> = rotation.iter().copied().filter(|k| *k != KIRBY).collect();
        let laid: Vec<u32> = members.kinds().iter().copied().filter(|k| *k != KIRBY && *k != u32::MAX).collect();
        assert_eq!(laid, others, "{:x?}", rotation);
    }
    // Two-member windows too, and Kirby as the target stays cell 0.
    assert_eq!(Members::from_cycle(YOSHI, &[KIRBY], 1).unwrap().kinds(), [KIRBY, YOSHI, u32::MAX]);
    assert_eq!(Members::from_cycle(KIRBY, &[YOSHI], 1).unwrap().kinds(), [KIRBY, YOSHI, u32::MAX]);
    // Every window the planners build satisfies the RE §5 predicate; hand-built invalid layouts are
    // refused by `Members::new` and named by the predicate.
    for window in windows() { assert!(!self_mirror_hazard(&window.kinds()), "{:x?}", window.kinds()); }
    assert!(self_mirror_hazard(&[YOSHI, KIRBY, FOX]) && self_mirror_hazard(&[YOSHI, FOX, KIRBY]) && !self_mirror_hazard(&[KIRBY, YOSHI, FOX]));
    assert!(Members::new(&[YOSHI, KIRBY]).is_none() && Members::new(&[YOSHI, FOX, KIRBY]).is_none());
    // One pinned member per window; a Solo Popo lead cannot share a layout with it (both need cell 0).
    assert!(Members::laid_out(&[POPO, KIRBY]).is_none() && Members::laid_out(&[KIRBY, POPO]).is_none());
    // The pair-lead window never takes Kirby as its extra: it waits for the next window.
    assert!(Members::pair_lead(KIRBY).is_none());
    assert_eq!(Members::from_pair(&[KIRBY, FOX], 2).unwrap().kinds(), [POPO, NANA, FOX]);
    // The derived predicates still admit Kirby everywhere a rotation may place it.
    assert!(bankable_kind(KIRBY) && swappable_kind(KIRBY) && (0..3).all(|cell| can_window_member(FormClass::Kirby, cell)));
}

/// rc.6 item 3 (operator decisions "All groups in rc.6" and "a session that starts on IC/PT still gets its groups
/// built"; primary 2026-11-09 00:23Z): the return to an Ice Climbers original is warm. Eden rc6-bg-ic2/3: P1 started
/// on the pair, and the return Yoshi → Ice Climbers was the canonical Reload (6–7 frames). It retired P1's bank, and the
/// next switch out rebuilt the window and its groups. The pair original, in its pair form and while the pair bank is
/// on, is now a bank member: the pair-lead window or a pair group serves it (`group_serves`). Solo Popo, the Miis and
/// the native groups keep the canonical Reload, and the kind-only `original_bankable` is unchanged for every caller.
fn an_ice_climbers_pair_original_is_a_bank_member_while_the_pair_bank_is_on() {
    use capabilities::{original_bankable, original_form_bankable, FormClass};
    assert!(capabilities::capabilities(FormClass::IcPair).original_bankable);
    assert!(original_form_bankable(POPO, Forms::starting(POPO), true, false));
    assert!(!original_form_bankable(POPO, Forms::starting(POPO), false, false), "pair bank off");
    assert!(!original_form_bankable(POPO, Forms::solo_popo(), true, false), "Solo Popo");
    assert!(!original_bankable(POPO) && !original_bankable(NANA), "the kind-only predicate is unchanged");
    for kind in kinds().filter(|kind| *kind != POPO) {
        for pair_bank in [false, true] {
            assert_eq!(original_form_bankable(kind, Forms::starting(kind), pair_bank, false), original_bankable(kind), "{kind:#x}");
        }
    }
    assert!(original_form_bankable(0x00, Forms::starting(0x00), false, false));
    assert!(original_form_bankable(KIRBY, Forms::starting(KIRBY), false, false));
    for kind in [0x24, 0x5a, 0x48] {
        assert!(!original_form_bankable(kind, Forms::starting(kind), true, false), "{kind:#x}: native group or Mii");
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("derived_predicates_match_todays_scattered_gates", derived_predicates_match_todays_scattered_gates as fn()),
        ("the_pair_and_solo_popo_are_distinct_classes_and_targets", the_pair_and_solo_popo_are_distinct_classes_and_targets as fn()),
        ("the_kirby_fallback_row_makes_kirby_canonical_only_without_touching_the_shipped_row", the_kirby_fallback_row_makes_kirby_canonical_only_without_touching_the_shipped_row as fn()),
        ("kirby_is_laid_out_at_cell_0_whatever_its_rotation_position", kirby_is_laid_out_at_cell_0_whatever_its_rotation_position as fn()),
        ("an_ice_climbers_pair_original_is_a_bank_member_while_the_pair_bank_is_on", an_ice_climbers_pair_original_is_a_bank_member_while_the_pair_bank_is_on as fn()),
    ]);
}
}
