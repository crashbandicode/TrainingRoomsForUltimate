//! K15 root fix: the Kirby copy packages a registration needs, and the bank's holdings of them
//! (`D175/refactor/u18/u19/copypkg/PROGRESS.md`; Eden s3/s4 on the M4b NRO).
//!
//! FACTs (owned 13.0.5 ELF `0789f0a4…`; listings in `u18/re/crash_*`, `u18/u19/re/`):
//! - Registering a body runs `prepare_entry 0x17f1a60` (init `0x607f34`). For every relation that
//!   takes a registration, `prepare_tuple 0x17f0b90` loads Kirby's copy models through
//!   `0x17f06a0`: path `fighter/kirby/model/<hat>/c%02d/model.numdlb` (format `0x4421b30`),
//!   handle `0x353e8e0`, then `0x17dde90(G68+0x48, costume<<25 | kind<<17 | part, &handle)`. A key
//!   that is new builds its object with `0x35c2890(handle)`, which needs the handle's resource loaded
//!   (`0x35c28f4–0x35c2950`). Otherwise the object is null, the node is still inserted, and
//!   `0x17def20` calls `null->vtable[+0x240]` (s3: x0 = 0, key `0x62003f` = Mega Man c00; s4:
//!   `0xb2003f`).
//! - Those models live in the package `fighter/<kind>/kirbycopy/c%02d` (format `0x43f01cb`). Its only
//!   loader is `0x17effa0` (G68 key `kind<<17 | costume<<25 | 0x1003f`). That loader's only caller
//!   is `0x17efb40`, whose only caller is the load completion `0x17e8440`, i.e. native's configure.
//!   The release is `0x17f0290`, reached only from `0x17e8610` while `R+0x4c` bit 2 is set.
//! - The roster a registration reads is the one `0x17e4f50` wrote. Per manager entry `E_i` it copies
//!   `E_i+0x70` while `E_i+0x5935 != 0 && E_i+0x591c <= 4`, otherwise `E_i+0x0`
//!   (`0x17e5594–0x17e57a0`), so `rec+0x28..0x30 = src+0x18..0x20`, `rec+0x51 = src+0x41` and
//!   `rec+0x8 = E_i+0xe0`. Every bank registration happens at loader `0xc`, so the owner's record
//!   carries the words the bank installed (`E+0x18..0x20`).
//! - G68's readiness is its vtable `+0x30`, the predicate the all-holders wait `0x17eb140(G)` calls on
//!   `G+0x0..0xc0`. For the map class it is `0x17e2880`: every node's handle state 3, with the ready
//!   byte `+0x20`, which inserting a key clears (`0x17e2fb0`).
//!
//! Native preloads the packages for a window's bodies before constructing them, and its tails return
//! them by kind. The bank registers bodies outside that lifecycle: a Phase B construction, U18's
//! attach re-registration, and the teardown's window registration. So the bank holds, for every
//! member set it may register, exactly what the completion would take for that set
//! ([`acquisitions`]). It releases each with the same inputs (exact inverse), and admits a
//! registration only once G68 reports ready.

use super::copy_registry::{registry_costume, Roster, ENTRIES, NO_KIND, SUBS};
#[cfg(test)]
use super::copy_registry::Registry;
use super::request_bit::{acquired_keys_iter, completion_members_iter};
#[cfg(test)]
use super::request_bit::{acquired_keys, copy_key};
#[cfg(test)]
use std::collections::BTreeMap;
use crate::engine::compat::fixed::FixedList;
use crate::engine::swap::body_bank::{MAX_EXTRA_GROUPS, NATIVE_MEMBERS};

/// The attached window plus the bank's physical groups; each has at most three
/// completion members. This bounds native holdings, not the generic model API.
pub type PackageSets = FixedList<[u32; NATIVE_MEMBERS], { MAX_EXTRA_GROUPS + 1 }>;
pub const MAX_HOLDINGS: usize = (MAX_EXTRA_GROUPS + 1) * NATIVE_MEMBERS;
pub type Holdings = FixedList<Acquisition, MAX_HOLDINGS>;

pub const fn empty_holdings() -> Holdings {
    Holdings::new(Acquisition { set: [NO_KIND; SUBS], member: NO_KIND, costume: 0,
        roster: [super::copy_registry::Record::DEFAULT; ENTRIES] })
}

pub struct BoundedSync {
    pub acquire: Holdings,
    pub release: FixedList<usize, MAX_HOLDINGS>,
}

pub fn sync_bounded(held: &Holdings, wanted: &Holdings) -> BoundedSync {
    let mut plan = BoundedSync { acquire: empty_holdings(), release: FixedList::new(0) };
    sync_changes(held.iter().copied(), wanted.iter().copied(),
        |value| assert!(plan.acquire.push(value)), |index| assert!(plan.release.push(index)));
    plan
}

/// One native acquisition `0x17efb40(G68, roster, member, costume)`. Its release is
/// `0x17f0290(G68, roster, member, costume)` with the same values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acquisition {
    pub set: [u32; SUBS],
    pub member: u32,
    pub costume: u32,
    pub roster: Roster,
}

impl Acquisition {
    #[cfg(test)]
    pub fn keys(&self) -> Vec<u32> { acquired_keys(&self.roster, self.member, self.costume) }
}

/// The roster at a registration of `set` by entry `owner`: the owner's record carries `set`'s words.
pub fn roster_for(roster: &Roster, owner: u32, set: [u32; SUBS]) -> Roster {
    let mut out = *roster;
    for record in out.iter_mut() {
        if record.entry == owner as i32 { record.kinds = set; }
    }
    out
}

/// What registering `set` takes: one acquisition per completion member that takes any key, with the
/// costume the selector gives over the set's primary (`R+0x64` = `costume`).
#[cfg(test)]
pub fn acquisitions(roster: &Roster, owner: u32, set: [u32; SUBS], costume: u32, region: u32) -> Vec<Acquisition> {
    acquisitions_iter(roster, owner, set, costume, region).collect()
}

pub fn acquisitions_iter(roster: &Roster, owner: u32, set: [u32; SUBS], costume: u32, region: u32)
    -> impl Iterator<Item = Acquisition> + Clone {
    let roster = roster_for(roster, owner, set);
    let costume = registry_costume(set[0], costume, region);
    completion_members_iter(set).filter(move |_| set[0] != NO_KIND)
        .map(move |member| Acquisition { set, member, costume, roster })
        .filter(|a| acquired_keys_iter(&a.roster, a.member, a.costume).next().is_some())
}

/// The packages the registrations of `set`'s bodies need: `copy_key(kind, costume)` of every tuple
/// they register (`prepare_entry`, U18's transcription), the other entries' bodies registered first.
#[cfg(test)]
pub fn needed_by_registration(roster: &Roster, owner: u32, set: [u32; SUBS], costume: u32, region: u32) -> Vec<u32> {
    let roster = roster_for(roster, owner, set);
    let mut registry = Registry::new();
    for record in roster.iter().filter(|r| r.entry >= 0 && r.entry != owner as i32 && (r.entry as usize) < ENTRIES) {
        for (sub, kind) in record.kinds.iter().enumerate() {
            if *kind != NO_KIND { registry.prepare_entry(&roster, *kind, record.costume, record.entry as usize, sub); }
        }
    }
    let before = registry.registrations();
    let selector = registry_costume(set[0], costume, region);
    for (sub, kind) in set.iter().enumerate() {
        if *kind != NO_KIND && (owner as usize) < ENTRIES { registry.prepare_entry(&roster, *kind, selector, owner as usize, sub); }
    }
    let mut needed = Vec::new();
    for (tuple, refs) in registry.registrations() {
        let had = before.iter().find(|(t, _)| *t == tuple).map_or(0, |(_, r)| *r);
        for _ in had..refs {
            needed.extend(copy_key(tuple.kind, tuple.costume));
        }
    }
    needed.sort_unstable();
    needed
}

/// The calls that bring `held` to `wanted`: every new acquisition first, then the releases of the
/// held ones no longer wanted, so a key two sets share never drops to zero in between.
#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sync {
    pub acquire: Vec<Acquisition>,
    /// Indices into `held`, highest first (remove in this order).
    pub release: Vec<usize>,
}

#[cfg(test)]
pub fn sync(held: &[Acquisition], wanted: &[Acquisition]) -> Sync {
    sync_iter(held.iter().copied(), wanted.iter().copied())
}

/// Match occurrences, not just membership: duplicate sets retain their native reference counts.
/// Rescanning borrowed/value iterators avoids a scratch allocation on every unchanged native tick.
#[cfg(test)]
pub fn sync_iter(held: impl Iterator<Item = Acquisition> + Clone,
    wanted: impl Iterator<Item = Acquisition> + Clone) -> Sync {
    let mut acquire = Vec::new();
    let mut release = Vec::new();
    sync_changes(held, wanted, |value| acquire.push(value), |index| release.push(index));
    Sync { acquire, release }
}

fn sync_changes(held: impl Iterator<Item = Acquisition> + Clone,
    wanted: impl Iterator<Item = Acquisition> + Clone,
    mut acquire: impl FnMut(Acquisition), mut release: impl FnMut(usize)) {
    for (i, want) in wanted.clone().enumerate() {
        let occurrence = wanted.clone().take(i).filter(|other| *other == want).count();
        if occurrence >= held.clone().filter(|other| *other == want).count() {
            acquire(want);
        }
    }
    for i in (0..held.clone().count()).rev() {
        let have = held.clone().nth(i).expect("repeatable holdings iterator");
        let occurrence = held.clone().take(i).filter(|other| *other == have).count();
        if occurrence >= wanted.clone().filter(|other| *other == have).count() { release(i); }
    }
}

/// The key multiset `held` keeps referenced in G68.
#[cfg(test)]
pub fn referenced(held: &[Acquisition]) -> BTreeMap<u32, u32> {
    let mut refs = BTreeMap::new();
    for acquisition in held {
        for key in acquisition.keys() { *refs.entry(key).or_insert(0) += 1; }
    }
    refs
}

/// Sorted unique keys for diagnostics, without building the reference-count map.
/// Holdings remain unchanged while the caller formats this borrowed/value view.
pub fn referenced_keys(held: impl Iterator<Item = Acquisition> + Clone) -> impl Iterator<Item = u32> + Clone {
    let mut previous = None;
    std::iter::from_fn(move || {
        let mut next = None;
        for acquisition in held.clone() {
            for key in acquired_keys_iter(&acquisition.roster, acquisition.member, acquisition.costume) {
                if previous.is_none_or(|old| key > old) && next.is_none_or(|old| key < old) { next = Some(key); }
            }
        }
        if let Some(key) = next { previous = Some(key); }
        next
    })
}

/// One record of a native-layout roster buffer (`R+0xb0` layout, 8 × 0x80): `rec+0x8` entry,
/// `rec+0x28/0x2c/0x30` kinds, `rec+0x51` costume. Those are the only fields `0x17efb40` and
/// `0x17f0290` read.
pub const ROSTER_BYTES: usize = ENTRIES * 0x80;

pub fn roster_bytes(roster: &Roster) -> [u8; ROSTER_BYTES] {
    let mut bytes = [0u8; ROSTER_BYTES];
    for (i, record) in roster.iter().enumerate() {
        let at = i * 0x80;
        bytes[at + 0x8..at + 0xc].copy_from_slice(&record.entry.to_le_bytes());
        for (j, kind) in record.kinds.iter().enumerate() {
            bytes[at + 0x28 + 4 * j..at + 0x2c + 4 * j].copy_from_slice(&kind.to_le_bytes());
        }
        bytes[at + 0x51] = record.costume as u8;
    }
    bytes
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::swap_model::copy_registry::{Record, KIRBY};
    

    const MARIO: u32 = 0x00;
    const DK: u32 = 0x01;
    const LINK: u32 = 0x02;
    const SAMUS: u32 = 0x03;
    const SAMUSD: u32 = 0x04;
    const YOSHI: u32 = 0x05;
    const FOX: u32 = 0x07;
    const FALCON: u32 = 0x0b;
    const MARTH: u32 = 0x15;
    const LUCINA: u32 = 0x16;
    const CHROM: u32 = 0x1b;
    const ROB: u32 = 0x2d;
    const MEGAMAN: u32 = 0x31;
    const BRAWLER: u32 = 0x48;
    const POPO: u32 = 0x4b;
    const NANA: u32 = 0x4c;
    const N: u32 = NO_KIND;

    fn roster(own: [u32; SUBS], own_costume: u32, other: Option<([u32; SUBS], u32)>) -> Roster {
        let mut roster = [Record::DEFAULT; ENTRIES];
        roster[0] = Record { entry: 0, kinds: own, costume: own_costume };
        if let Some((kinds, costume)) = other { roster[1] = Record { entry: 1, kinds, costume }; }
        roster
    }

    fn held_keys(acquisitions: &[Acquisition]) -> Vec<u32> {
        let mut keys: Vec<u32> = acquisitions.iter().flat_map(|a| a.keys()).collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    /// For every roster shape, member set and costume: what the bank holds for a set covers every
    /// package that set's registrations need. Those are the completion's own key sets, so native's
    /// precondition holds.
    fn holdings_cover_every_registration() {
        let others: [Option<([u32; SUBS], u32)>; 8] = [
            None,
            Some(([KIRBY, N, N], 4)),
            Some(([MEGAMAN, N, N], 3)),
            Some(([FALCON, N, N], 1)),
            Some(([BRAWLER, N, N], 5)),
            Some(([POPO, NANA, N], 2)),
            Some(([ROB, N, N], 7)),
            Some(([KIRBY, FOX, SAMUS], 6)),
        ];
        let sets: [[u32; SUBS]; 9] = [
            [MARIO, N, N], [KIRBY, N, N], [KIRBY, SAMUSD, YOSHI], [DK, LINK, SAMUS], [CHROM, LUCINA, MARTH],
            [KIRBY, YOSHI, FOX], [BRAWLER, N, N], [ROB, MARIO, N], [POPO, NANA, N],
        ];
        let mut covered = 0;
        for other in others {
            for home in sets {
                for set in sets {
                    for costume in [0u32, 2, 7] {
                        for region in [0u32, 1, 14] {
                            let r = roster(home, costume, other);
                            let needed = needed_by_registration(&r, 0, set, costume, region);
                            let keys = held_keys(&acquisitions(&r, 0, set, costume, region));
                            assert!(needed.iter().all(|k| keys.contains(k)),
                                "home {:x?} set {:x?} other {:x?} c{} r{}: needed {:x?} held {:x?}", home, set, other, costume, region, needed, keys);
                            if !needed.is_empty() { covered += 1; }
                        }
                    }
                }
            }
        }
        assert!(covered > 300, "{}", covered);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("holdings_cover_every_registration", holdings_cover_every_registration as fn()),
        ]);
    }
    }
}
