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

use super::copy_registry::{registry_costume, Registry, Roster, ENTRIES, NO_KIND, SUBS};
use super::request_bit::{acquired_keys, completion_members, copy_key, released_keys};
use std::collections::BTreeMap;

/// One native acquisition `0x17efb40(G68, roster, member, costume)`. Its release is
/// `0x17f0290(G68, roster, member, costume)` with the same values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Acquisition {
    pub set: [u32; SUBS],
    pub member: u32,
    pub costume: u32,
    pub roster: Roster,
}

impl Acquisition {
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
pub fn acquisitions(roster: &Roster, owner: u32, set: [u32; SUBS], costume: u32, region: u32) -> Vec<Acquisition> {
    if set[0] == NO_KIND { return Vec::new(); }
    let roster = roster_for(roster, owner, set);
    let costume = registry_costume(set[0], costume, region);
    completion_members(set).into_iter()
        .map(|member| Acquisition { set, member, costume, roster })
        .filter(|a| !a.keys().is_empty())
        .collect()
}

/// The packages the registrations of `set`'s bodies need: `copy_key(kind, costume)` of every tuple
/// they register (`prepare_entry`, U18's transcription), the other entries' bodies registered first.
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sync {
    pub acquire: Vec<Acquisition>,
    /// Indices into `held`, highest first (remove in this order).
    pub release: Vec<usize>,
}

pub fn sync(held: &[Acquisition], wanted: &[Acquisition]) -> Sync {
    let mut unmatched: Vec<Option<&Acquisition>> = held.iter().map(Some).collect();
    let mut acquire = Vec::new();
    for want in wanted {
        match unmatched.iter().position(|h| *h == Some(want)) {
            Some(i) => unmatched[i] = None,
            None => acquire.push(want.clone()),
        }
    }
    let mut release: Vec<usize> = unmatched.iter().enumerate().filter(|(_, h)| h.is_some()).map(|(i, _)| i).collect();
    release.reverse();
    Sync { acquire, release }
}

/// The key multiset `held` keeps referenced in G68.
pub fn referenced(held: &[Acquisition]) -> BTreeMap<u32, u32> {
    let mut refs = BTreeMap::new();
    for acquisition in held {
        for key in acquisition.keys() { *refs.entry(key).or_insert(0) += 1; }
    }
    refs
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

/// Every key a release of `held[i]` returns, as `0x17f0290` computes it.
pub fn released(held: &Acquisition) -> Vec<u32> { released_keys(&held.roster, held.member, held.costume) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::swap::swap_model::copy_registry::{Record, KIRBY};
    use std::convert::TryInto;

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

    /// The s3 crash: a Kirby group `[6,4,5]` built under the home `[1,2,3]` against a Mega Man CPU.
    /// Kirby's registration needs Mega Man's copy package under Kirby's costume. That is the package of
    /// the null hat-model node (`0x62003f`), and nothing in the bank's flow took it. Holding the set's
    /// acquisitions covers it.
    #[test]
    fn the_k15_crash_needs_the_cpu_s_copy_package() {
        let r = roster([DK, LINK, SAMUS], 0, Some(([MEGAMAN, N, N], 3)));
        let needed = needed_by_registration(&r, 0, [KIRBY, SAMUSD, YOSHI], 0, 1);
        assert!(needed.contains(&((MEGAMAN << 17) | 0x1003f)), "{:x?}", needed);
        // Its hat model's part key in the s3 registers is the same kind and costume.
        assert_eq!((0x62003f >> 17) & 0xff, MEGAMAN);
        let held = acquisitions(&r, 0, [KIRBY, SAMUSD, YOSHI], 0, 1);
        let keys = held_keys(&held);
        assert!(needed.iter().all(|k| keys.contains(k)), "needed {:x?} held {:x?}", needed, keys);
    }

    /// K14 (Yoga `m4b3`/`m4bg1`): the non-Kirby group `[1b,16,15]` under the Kirby home `[6,5,7]`
    /// against Falcon. The roster's own record is the group (`R+0xb0` dump: `rec+0x28 = [1b,16,15]`).
    /// The record with Kirby at k0 is manager entry 2, a cycle-preload helper (the `0x17f211c` frame
    /// has x27 = 2). Its copy relation for Chrom loads `fighter/chrom/kirbycopy/c06`: the null node
    /// key `0xc36003f` is (kind 0x1b, costume 6). Without that helper nothing is needed. The attach
    /// back to the Kirby home needs Falcon's package and the home's own non-Kirby members' packages.
    #[test]
    fn k14_needs_the_helper_kirby_s_packages_for_every_group_body() {
        let mut r = roster([KIRBY, YOSHI, FOX], 2, Some(([FALCON, N, N], 1)));
        assert!(needed_by_registration(&r, 0, [CHROM, LUCINA, MARTH], 2, 1).is_empty());
        r[2] = Record { entry: 2, kinds: [KIRBY, N, N], costume: 6 };
        let group = needed_by_registration(&r, 0, [CHROM, LUCINA, MARTH], 2, 1);
        assert_eq!(group, vec![(MARTH << 17) | (6 << 25) | 0x1003f, (LUCINA << 17) | (6 << 25) | 0x1003f,
            (CHROM << 17) | (6 << 25) | 0x1003f]);
        assert_eq!((0xc36003fu32 >> 17) & 0xff, CHROM);
        assert_eq!(0xc36003fu32 >> 25, 6);
        let keys = held_keys(&acquisitions(&r, 0, [CHROM, LUCINA, MARTH], 2, 1));
        assert!(group.iter().all(|k| keys.contains(k)), "{:x?} {:x?}", group, keys);
        let r = roster([KIRBY, YOSHI, FOX], 2, Some(([FALCON, N, N], 1)));
        let home = needed_by_registration(&r, 0, [KIRBY, YOSHI, FOX], 2, 1);
        for key in [(FALCON << 17) | (2 << 25) | 0x1003f, (YOSHI << 17) | (2 << 25) | 0x1003f, (FOX << 17) | (2 << 25) | 0x1003f] {
            assert!(home.contains(&key), "{:x?}", home);
        }
        let keys = held_keys(&acquisitions(&r, 0, [KIRBY, YOSHI, FOX], 2, 1));
        assert!(home.iter().all(|k| keys.contains(k)));
    }

    /// For every roster shape, member set and costume: what the bank holds for a set covers every
    /// package that set's registrations need. Those are the completion's own key sets, so native's
    /// precondition holds.
    #[test]
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

    /// The sync keeps G68's references exact. Over every sequence of wanted sets (a roster change
    /// and any group set appearing or leaving), each acquisition is later released with the same
    /// inputs. No release ever takes a key the holdings did not take, and after the final release
    /// nothing is referenced.
    #[test]
    fn sync_pairs_every_acquisition_with_its_release() {
        let rosters = [
            roster([DK, LINK, SAMUS], 0, Some(([MEGAMAN, N, N], 3))),
            roster([DK, LINK, SAMUS], 0, Some(([KIRBY, N, N], 3))),
            roster([KIRBY, YOSHI, FOX], 0, Some(([FALCON, N, N], 1))),
        ];
        let sets: [[u32; SUBS]; 4] = [[DK, LINK, SAMUS], [KIRBY, SAMUSD, YOSHI], [KIRBY, YOSHI, FOX], [CHROM, LUCINA, MARTH]];
        let mut runs = 0;
        for seed in 0u64..400 {
            let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
            let mut next = |n: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % n };
            let mut held: Vec<Acquisition> = Vec::new();
            let mut g68: BTreeMap<u32, i64> = BTreeMap::new();
            for _ in 0..8 {
                let r = &rosters[next(rosters.len() as u64) as usize];
                let mut wanted = Vec::new();
                for set in sets { if next(2) == 0 { wanted.extend(acquisitions(r, 0, set, 0, 1)); } }
                let plan = sync(&held, &wanted);
                for a in &plan.acquire { for k in a.keys() { *g68.entry(k).or_insert(0) += 1; } }
                for i in &plan.release {
                    for k in released(&held[*i]) {
                        let refs = g68.entry(k).or_insert(0);
                        *refs -= 1;
                        assert!(*refs >= 0, "seed {}: key {:#x} released below zero", seed, k);
                    }
                }
                for i in &plan.release { held.remove(*i); }
                held.extend(plan.acquire);
                let mut want_sorted = wanted.clone();
                want_sorted.sort_by_key(|a| (a.set, a.member));
                let mut held_sorted = held.clone();
                held_sorted.sort_by_key(|a| (a.set, a.member));
                assert_eq!(held_sorted.len(), want_sorted.len());
                assert_eq!(referenced(&held).into_iter().map(|(k, v)| (k, v as i64)).collect::<BTreeMap<_, _>>(),
                    g68.iter().filter(|(_, v)| **v != 0).map(|(k, v)| (*k, *v)).collect::<BTreeMap<_, _>>(), "seed {}", seed);
                runs += 1;
            }
            let plan = sync(&held, &[]);
            for i in &plan.release { for k in released(&held[*i]) { *g68.entry(k).or_insert(0) -= 1; } }
            assert!(g68.values().all(|v| *v == 0), "seed {}: {:x?}", seed, g68);
        }
        assert!(runs >= 3200);
    }

    #[test]
    fn the_buffer_carries_what_the_natives_read() {
        let r = roster([KIRBY, SAMUSD, YOSHI], 3, Some(([MEGAMAN, N, N], 5)));
        let b = roster_bytes(&r);
        assert_eq!(i32::from_le_bytes(b[0x8..0xc].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(b[0x28..0x2c].try_into().unwrap()), KIRBY);
        assert_eq!(u32::from_le_bytes(b[0x30..0x34].try_into().unwrap()), YOSHI);
        assert_eq!(b[0x51], 3);
        assert_eq!(u32::from_le_bytes(b[0xa8..0xac].try_into().unwrap()), MEGAMAN);
        assert_eq!(b[0xd1], 5);
        assert_eq!(i32::from_le_bytes(b[0x108..0x10c].try_into().unwrap()), -1);
    }
}
