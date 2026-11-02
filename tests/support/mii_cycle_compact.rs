//! Independent heap-backed ring oracle for the actual carried planner.
//! The oracle describes one ordered ring and rotates it; it does not use the
//! compact storage or the production repeated-qualification walk.
use super::*;
use crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId, CHARINFO_LEN};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct AllocationProbe;
thread_local! { static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
fn allocation() {
    let _ = ALLOCS.try_with(|count| {
        if let Some(n) = count.get() { count.set(Some(n + 1)); }
    });
}
unsafe impl GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { allocation(); System.alloc(layout) }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { allocation(); System.alloc_zeroed(layout) }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
        allocation(); System.realloc(p, layout, n)
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) { System.dealloc(p, layout) }
}
#[global_allocator]
static ALLOCATOR: AllocationProbe = AllocationProbe;

fn catalog() -> Snapshot {
    let rows = [MiiClass::Brawler, MiiClass::Swordfighter, MiiClass::Gunner]
        .into_iter().flat_map(|class| [0u8, 1].map(move |variant| (class, variant)))
        .enumerate().map(|(index, (class, variant))| {
            let mut charinfo = [0u8; CHARINFO_LEN];
            charinfo[0] = index as u8 + 1;
            charinfo[0x10] = b'A' + index as u8;
            RawSlot {
                index, slot_id: 100 + index as u32, name_len: 1,
                name_units: [u16::from(b'A' + index as u8), 0, 0, 0, 0, 0, 0, 0, 0, 0],
                colour: variant, voice: 37, specials: [variant; 4],
                tag_outfit: TaggedId(0x1300011bfef7a5f7),
                tag_hat: TaggedId(0x5a00000fa9efa5ea),
                tag_chara: TaggedId((0xc1u64 << 56) | class.chara_hash40()),
                charinfo,
            }
        }).collect();
    build_snapshot(rows)
}

fn same(target: Target, identity: Identity) -> bool {
    match (target, identity) {
        (Target::Fighter(a), Identity::Fighter(b)) => Forms::starting(a).same_group(b),
        (Target::Saved(a), Identity::Saved(b)) => a.canonical == b,
        _ => false,
    }
}

/// A specification ring: all eligible unique numbered positions, then slot 0.
fn ring(ordered: &[TrioFighter], catalog: &Snapshot, preferences: &Preferences,
    role: Role, original: Identity, separate: bool) -> Vec<Step>
{
    if matches!(original, Identity::Fighter(k) if MiiClass::from_kind(k).is_some()) {
        return vec![];
    }
    let origin = match original {
        Identity::Saved(c) => PlayerOrigin::Mii(Origin(c)),
        _ => PlayerOrigin::NotMii,
    };
    let mut targets = Vec::new();
    for fighter in ordered {
        let kind = fighter.kind();
        if let Some(class) = MiiClass::from_kind(kind) {
            if targets.iter().any(|t| matches!(t, Target::Saved(c) if c.canonical.class == class)) {
                continue;
            }
            let choice = selection::resolve(catalog, preferences, role, class, origin)
                .and_then(|choice| {
                    if original != Identity::Saved(choice.canonical) { return Some(choice); }
                    selection::eligible(catalog, role, class, origin)
                        .find(|entry| original != Identity::Saved(entry.canonical))
                        .map(|entry| Choice { canonical: entry.canonical, slot_id: entry.slot_id,
                            source: Source::FirstEligible(Fallback::PreferenceIsOrigin) })
                });
            if let Some(choice) = choice { targets.push(Target::Saved(choice)); }
        } else {
            let group = Forms::starting(kind);
            if matches!(original, Identity::Fighter(k) if group.same_group(k))
                && !(separate && kind == 0x4b) { continue; }
            if targets.iter().any(|t| matches!(t, Target::Fighter(k) if group.same_group(*k))) {
                continue;
            }
            targets.push(Target::Fighter(kind));
        }
    }
    let mut result: Vec<_> = targets.into_iter().enumerate()
        .map(|(i, target)| Step { target, cursor: Some(i) }).collect();
    if matches!(original, Identity::Saved(_))
        || matches!(original, Identity::Fighter(k) if TrioFighter::from_kind(k).is_some())
    { result.push(Step { target: Target::Original, cursor: None }); }
    result
}

fn start(ring: &[Step], live: Live) -> Option<usize> {
    if ring.is_empty()
        || matches!(live, Live::Numbered { identity: Identity::Fighter(k), .. }
            if MiiClass::from_kind(k).is_some())
    { return None; }
    let position = match live {
        Live::Original => None,
        Live::Numbered { identity, cursor } => cursor
            .filter(|i| ring.get(*i).is_some_and(|s| same(s.target, identity)))
            .or_else(|| ring.iter().position(|s| same(s.target, identity))),
    };
    Some(position.map_or(0, |i| (i + 1) % ring.len()))
}

fn qualifies(step: Step, mode: u8) -> bool {
    match mode {
        0 => true,
        1 => false,
        2 => matches!(step.target, Target::Saved(_)),
        3 => step.cursor.is_some_and(|i| i % 2 == 1),
        _ => step.target == Target::Original,
    }
}

#[test]
fn compact_ring_matches_reference_for_full_mixed_duplicate_and_rejected_rosters() {
    let full = catalog();
    assert_eq!(full.configs.len(), 6, "all fixture identities must be valid");
    let empty = build_snapshot(Vec::new());
    let all: Vec<_> = crate::engine::swap::trio::CHOICES.iter()
        .map(|(kind, _)| TrioFighter::from_kind(*kind).unwrap()).collect();
    let mut reversed = all.clone(); reversed.reverse(); reversed.extend_from_slice(&all);
    let mixed: Vec<_> = [6, 0x48, 0x24, 0x25, 0x49, 6, 0x4a, 0x48, 0x4b, 0x4b]
        .into_iter().map(|k| TrioFighter::from_kind(k).unwrap()).collect();
    let preferred = full.configs[1].canonical;
    let preferences = Preferences::default()
        .with(Role::Cpu, MiiClass::Brawler, Some(preferred)).unwrap()
        .with(Role::Player, MiiClass::Brawler, Some(preferred)).unwrap();
    let mut compared = 0usize;
    for catalog in [&full, &empty] {
        for ordered in [&[][..], &mixed[..], &all[..], &reversed[..]] {
            for role in [Role::Cpu, Role::Player] {
                for original in [Identity::Fighter(0), Identity::Fighter(0x4b),
                    Identity::Saved(preferred), Identity::Fighter(u32::MAX), Identity::Fighter(0x48)] {
                    for separate in [false, true] {
                        let reference = ring(ordered, catalog, &preferences, role, original, separate);
                        let mut lives = vec![Live::Original,
                            Live::Numbered { identity: Identity::Fighter(0x25), cursor: Some(999) },
                            Live::Numbered { identity: Identity::Saved(preferred), cursor: Some(0) },
                            Live::Numbered { identity: Identity::Fighter(0x48), cursor: None }];
                        for step in &reference {
                            if let Some(cursor) = step.cursor {
                                let identity = match step.target {
                                    Target::Fighter(k) => Identity::Fighter(k),
                                    Target::Saved(c) => Identity::Saved(c.canonical),
                                    Target::Original => unreachable!(),
                                };
                                lives.push(Live::Numbered { identity, cursor: Some(cursor) });
                                lives.push(Live::Numbered { identity, cursor: Some(999) });
                            }
                        }
                        for (live_index, live) in lives.into_iter().enumerate() {
                            let first = start(&reference, live);
                            let expected = first.map(|i| reference[i]);
                            assert_eq!(next_cycle_with_ice_climbers(ordered, catalog, &preferences,
                                role, original, live, separate), expected);
                            // All positions check the one-step oracle; bounded representative
                            // positions additionally exercise every qualification policy.
                            if live_index >= 4 && live_index % 17 != 0 { continue; }
                            for mode in 0..5 {
                                let expected = first.and_then(|i| {
                                    for step in reference.iter().cycle().skip(i).take(reference.len()) {
                                        if qualifies(*step, mode) { return Some((*step, mode)); }
                                        if step.target == Target::Original { break; }
                                    }
                                    None
                                });
                                let actual = next_qualified(Walk { ordered, catalog, preferences: &preferences,
                                    role, original, separate_ice_climbers: separate }, live,
                                    |step| qualifies(step, mode).then_some(mode));
                                assert_eq!(actual, expected, "role={role:?} original={original:?} live={live:?} separate={separate} mode={mode}");
                                compared += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(compared > 1_000, "reference cases={compared}");
}

#[test]
fn first_and_repeated_full_roster_qualified_walks_allocate_nothing() {
    let catalog = catalog();
    let preferences = Preferences::default();
    let ordered: Vec<_> = crate::engine::swap::trio::CHOICES.iter()
        .map(|(kind, _)| TrioFighter::from_kind(*kind).unwrap()).collect();
    for pass in 0..2 {
        ALLOCS.with(|c| c.set(Some(0)));
        let mut live = Live::Original;
        for _ in 0..ordered.len() + 2 {
            let result = next_qualified_player(&ordered, &catalog, &preferences,
                Identity::Fighter(0), live, |step| Some(step.target));
            if let Some((step, _)) = std::hint::black_box(result) {
                live = match step.target {
                    Target::Original => Live::Original,
                    Target::Fighter(k) => Live::Numbered { identity: Identity::Fighter(k), cursor: step.cursor },
                    Target::Saved(c) => Live::Numbered { identity: Identity::Saved(c.canonical), cursor: step.cursor },
                };
            }
        }
        let count = ALLOCS.with(|c| c.replace(None).unwrap());
        assert_eq!(count, 0, "pass={pass}: first and repeated actual planner calls");
    }
}