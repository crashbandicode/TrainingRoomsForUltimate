//! U19 — the request bit `R+0x4c` bit 0 across a Phase B build, and the G68 copy keys the load
//! completion takes under it (`D175/refactor/u18/u19/PROGRESS.md`).
//!
//! FACTs (owned 13.0.5 ELF `0789f0a4…`; listings and scans in `u18/re/u19_*` and `u18/u19/re/`):
//! - **Readers of bit 0.** The load completion `0x17e8440(R)` runs its body only when
//!   `(R+0x4c & 3) == 1` (`0x17e8440–0x17e844c`). The per-kind path builder `0x17e99c0` returns an
//!   empty path while it is clear (`0x17e99d0–0x17e99e8`; callers `0x60b98c`/`0x60bbf4` in the body
//!   construction `0x60b6b0`, and `0xba4624`). Loader configure is skipped while it is set
//!   (`0x6510f4`, W-M2-4); the scene path configures only while it is clear (`0x66ddd8`).
//! - **Writers.** Set: loader configure `0x6513a8–0x6513b4`, the scene path `0x66de44–0x66de50`, the
//!   Pokémon Trainer object owners (`0x14ea984`, `0x14eaa48`, `0x14eab0c`) and the bank's Phase B
//!   (`r_handle::request_bit`). Bits 1|2: the completion (`0x17e85f0–0x17e85f8`). Cleared only by the
//!   reset `0x17e45f0` (`stur xzr,[x19,#0x3c]` with `x19 = R+0x10`, which also zeroes `R+0x50`) and
//!   the constructor `0x17e3ef4`. The reset is `R->vtable[+0x20]` (vtable `0x509fb10`, RELATIVE
//!   record at `0x3ce0440`); it runs from `0x17e8610`'s tail when the map is empty
//!   (`0x17e883c–0x17e8880`), from the pair rule (`0x60dea0–0x60deb4`) and from the scene loop.
//! - **Acquire.** The completion calls `0x17efb40(G68, R+0xb0, m, sel)` for the header members
//!   `m = R+0x58` (always), `R+0x5c`, `R+0x60` (unless `-1`, `0x4b`, `0x4c`), then sets bits 1|2. It
//!   is the only caller of `0x17efb40`, which is the only caller of `0x17effa0`; that one takes the
//!   G68 key `kind<<17 | costume<<25 | 0x1003f` (`0x17f0130–0x17f015c` → `0x17e2df0`).
//! - **Release.** `0x17e8610(R, kind)` calls `0x17f0290(G68, R+0xb0, kind, sel)` while bit 2 is set
//!   (`0x17e8708–0x17e87ac`): the keys of the **released kind**. It is the only caller of
//!   `0x17f0290`. The key release `0x17e2fe0` decrements the node, erases it at zero, and ignores a
//!   key it does not find (`0x17e2fe0–0x17e30dc`).
//! - **Callers of the completion.** The configure functor (vtable `0x509fb58`, slot `+0x30` =
//!   `0x17e8410`: `0x17e5bc0(R)` then `0x17e8440(R)`), created only at `0x6513b8` and posted with
//!   `0x3549720(task, 0x17e8310, 2, 1)` at `0x651418`; the scene path `0x66de64` (function
//!   `0x66dcf0`, 20 callers); the loop over every fighter `R` at `0x14f5734` (function `0x14f10c0`,
//!   12 callers); the Pokémon Trainer owners `0x14ea9b8`, `0x14eaa7c`, `0x14eab50`.
//!
//! Native pairs acquisition and release through the header. Configure writes the header and sets
//! bit 0. The completion takes the keys of the header members. Each member's construction tail
//! `0x17e8610(R, member)` (`0x60dec4`) returns the same keys, and the last tail empties the map, so
//! the reset clears the bits. The bank's Phase B sets bit 0 for its synchronous construction, because
//! the path builder needs it. Its R also holds other batches' keys (prefetched groups), so the map
//! does not empty, the reset does not run, and bit 0 outlives the build. A later completion then
//! takes keys for the header members. Every later per-kind release hands back keys by the kind it
//! releases instead (`today_*` tests). The root fix is at the writer (GROUP-SWAP-IMPL §14.2). Phase B
//! holds the bit as a [`Lease`] for its construction and ends it when the construction is over. That
//! is native's own end of the bit, without native's reset, which would also drop the other batches'
//! keys.

use super::copy_registry::{Record, Roster, ENTRIES, KIRBY, NO_KIND, SUBS};
use std::collections::BTreeMap;

/// `R+0x4c` bit 0: configured for the header (the request bit).
pub const REQUEST: u8 = 1;
/// Bit 1: the completion ran.
pub const COMPLETED: u8 = 2;
/// Bit 2: the completion took the header members' G68 keys; `0x17e8610` releases by kind.
pub const ACQUIRED: u8 = 4;
const POPO: u32 = 0x4b;
const NANA: u32 = 0x4c;
const COPY_KEY_LOW: u32 = 0x1003f;

fn mii(kind: u32) -> bool { (0x48..=0x4a).contains(&kind) }

/// `0x17effa0(G68, costume, kind)`: no key for `-1`, Kirby, `0x4d..=0x50` or a negative kind
/// (`0x17effbc–0x17effdc`); a Mii kind takes costume 0 (`0x17effe0–0x17efff4`); the key is
/// `kind<<17 | costume<<25 | 0x1003f` (`0x17f0130–0x17f0140`).
pub fn copy_key(kind: u32, costume: u32) -> Option<u32> {
    if kind == NO_KIND || kind == KIRBY || (0x4d..=0x50).contains(&kind) || (kind as i32) < 0 {
        return None;
    }
    let costume = if mii(kind) { 0 } else { costume };
    Some((kind << 17) | (costume << 25) | COPY_KEY_LOW)
}

/// `0x17efb40(G68, roster, kind, costume)` as its control flow (`0x17efb50–0x17eff9c`): Kirby takes a
/// key per kind of every record whose first kind is neither `-1` nor Kirby (the second and third
/// unless `0x4b/0x4c/-1`) under `costume`; any other kind takes its own key once per Kirby record,
/// under that record's `+0x51`.
pub fn acquired_keys(roster: &Roster, kind: u32, costume: u32) -> Vec<u32> {
    acquired_keys_iter(roster, kind, costume).collect()
}

/// Native callers can inspect or stream the same key multiset without allocating it.
pub fn acquired_keys_iter(roster: &Roster, kind: u32, costume: u32) -> impl Iterator<Item = u32> + '_ {
    roster.iter().flat_map(move |record| {
        record.kinds.into_iter().enumerate().filter_map(move |(sub, member)| {
            if kind == KIRBY {
                if record.kinds[0] == NO_KIND || record.kinds[0] == KIRBY
                    || (sub != 0 && ((POPO..=NANA).contains(&member) || member == NO_KIND)) {
                    return None;
                }
                copy_key(member, costume)
            } else if sub == 0 && record.kinds[0] == KIRBY {
                copy_key(kind, record.costume)
            } else { None }
        })
    })
}

/// `0x17f0290(G68, roster, kind, costume)`, transcribed a second time from its own listing (not
/// from [`acquired_keys`]): early exit for `-1`, `0x4d..=0x50` or negative (`0x17f02a4–0x17f02bc`);
/// Kirby walks the records (`0x17f03ac–0x17f03c8`), the first kind unless `-1`/Kirby
/// (`0x17f02e4–0x17f030c`), the second and third unless `0x4b/0x4c/-1`/Kirby (`0x17f0310–0x17f03a4`),
/// Mii kinds without costume; any other kind releases its key per Kirby record, a Mii kind without
/// costume (`0x17f03d8–0x17f0518`), otherwise with the record's `+0x51` (`0x17f051c…`).
pub fn released_keys(roster: &Roster, kind: u32, costume: u32) -> Vec<u32> {
    let mut keys = Vec::new();
    let out_of_range = |k: u32| (k as i32) < 0 || (0x4d..=0x50).contains(&k);
    if out_of_range(kind) { return keys; }
    let key = |k: u32, c: u32| {
        let costume = if (0x48..0x4b).contains(&k) { 0 } else { c };
        (k << 17) | (costume << 25) | COPY_KEY_LOW
    };
    if kind == KIRBY {
        for record in roster {
            let [k0, k1, k2] = record.kinds;
            if k0 == NO_KIND || k0 == KIRBY { continue; }
            if !out_of_range(k0) { keys.push(key(k0, costume)); }
            for k in [k1, k2] {
                if (POPO..=NANA).contains(&k) || k == NO_KIND || k == KIRBY || out_of_range(k) { continue; }
                keys.push(key(k, costume));
            }
        }
    } else {
        for record in roster {
            if record.kinds[0] != KIRBY { continue; }
            keys.push(key(kind, record.costume));
        }
    }
    keys
}

/// The header members the completion takes keys for (`0x17e8484`, `0x17e84f0–0x17e8504`,
/// `0x17e8570–0x17e8584`).
pub fn completion_members(header: [u32; 3]) -> Vec<u32> {
    completion_members_iter(header).collect()
}

pub fn completion_members_iter(header: [u32; 3]) -> impl Iterator<Item = u32> + Clone {
    header.into_iter().enumerate().filter_map(|(sub, kind)| {
        (sub == 0 || (!(POPO..=NANA).contains(&kind) && kind != NO_KIND)).then_some(kind)
    })
}

// ---------------------------------------------------------------------------------------------
// The writer: Phase B's lease on bit 0
// ---------------------------------------------------------------------------------------------

/// Phase B's hold on bit 0, from the write before its synchronous construction to the end of that
/// construction. `before` is the low byte of `R+0x4c` as Phase B found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    pub before: u8,
}

/// Begin: the byte to write (`before | 1`) when bit 0 is clear, `None` when it is already set
/// (then it is native's, and so is its end).
pub fn begin(before: u8) -> (Lease, Option<u8>) {
    (Lease { before }, (before & REQUEST == 0).then_some(before | REQUEST))
}

/// How a lease ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Bit 0 was Phase B's and nothing else wrote the byte: restore `before`.
    Restored,
    /// The last construction tail emptied the map and the reset zeroed the byte: native ended it.
    ResetRan,
    /// Bit 0 was already set when Phase B began: not Phase B's to clear.
    NotOurs,
    /// Another writer changed the byte during the construction (a completion or a configure). Not
    /// reachable while Phase B owns the game thread and R's task is idle (INF: every completion
    /// caller is a scene-level function or the configure task); never cleared, always logged.
    Changed,
}

/// End: the byte to write, if any. Phase B clears only the bit it set, and only while the byte is
/// exactly what it wrote.
pub fn end(lease: Lease, now: u8) -> (End, Option<u8>) {
    if lease.before & REQUEST != 0 { return (End::NotOurs, None); }
    if now == lease.before | REQUEST { return (End::Restored, Some(lease.before)); }
    if now == 0 { return (End::ResetRan, None); }
    (End::Changed, None)
}

// ---------------------------------------------------------------------------------------------
// One resource owner and the G68 copy keys
// ---------------------------------------------------------------------------------------------

/// An acquisition and a release that do not pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// A completion of this R took `key` and no release of this R returned it before the reset.
    Kept { key: u32 },
    /// `0x17f0290` handed back `key` that this R never took: another R's reference (`foreign`), or
    /// nothing (a no-op natively, still not a pair).
    Unpaired { key: u32, foreign: bool },
    /// A body construction ran with bit 0 clear: the path builder gives it empty paths.
    EmptyPath { kind: u32 },
}

/// One resource owner `R`: the fields the request bit and the copy keys depend on.
#[derive(Clone, Debug)]
pub struct Owner {
    /// Low byte of `R+0x4c`.
    pub flags: u8,
    /// The key map `R+0x10` as registrations per kind; empty = `R+0x18 == 0`.
    pub registrations: BTreeMap<u32, u32>,
    /// `R+0x58..0x60`.
    pub header: [u32; SUBS],
    /// The costume the selector gives (`R+0x64` through `sel`; one per role, CATALOG R64).
    pub costume: u32,
    /// `R+0xb0`.
    pub roster: Roster,
    /// G68 keys this R's completions took and its releases have not returned.
    pub taken: BTreeMap<u32, u32>,
}

impl Owner {
    pub fn new(costume: u32) -> Self {
        Owner { flags: 0, registrations: BTreeMap::new(), header: [NO_KIND; SUBS], costume,
            roster: [Record::DEFAULT; ENTRIES], taken: BTreeMap::new() }
    }

    /// Loader configure (`0x651334–0x6513b4`) or the scene path (`0x66dde0–0x66de50`): only while
    /// bit 0 is clear. Returns whether it ran.
    pub fn configure(&mut self, header: [u32; SUBS], roster: Roster) -> bool {
        if self.flags & REQUEST != 0 { return false; }
        self.header = header;
        self.roster = roster;
        self.flags |= REQUEST;
        true
    }

    /// `0x17e64d0(R, kind)`: registrations only; it never reads `R+0x4c` (FEASIBILITY §5.1).
    pub fn register(&mut self, kind: u32) { *self.registrations.entry(kind).or_insert(0) += 1; }

    /// `0x17e8440(R)`.
    pub fn completion(&mut self) {
        if self.flags & (REQUEST | COMPLETED) != REQUEST { return; }
        for member in completion_members(self.header) {
            for key in acquired_keys(&self.roster, member, self.costume) {
                *self.taken.entry(key).or_insert(0) += 1;
            }
        }
        self.flags |= COMPLETED | ACQUIRED;
    }

    /// `0x17e8610(R, kind)`: the copy keys of `kind` while bit 2 is set, then R's own keys, then
    /// the reset when the map is empty. `g68` holds the other owners' references.
    pub fn release_kind(&mut self, kind: u32, g68: &mut BTreeMap<u32, u32>, faults: &mut Vec<Fault>) {
        if self.flags & ACQUIRED != 0 {
            for key in released_keys(&self.roster, kind, self.costume) {
                if take_one(&mut self.taken, key) { continue; }
                let foreign = take_one(g68, key);
                faults.push(Fault::Unpaired { key, foreign });
            }
        }
        if let Some(n) = self.registrations.get_mut(&kind) {
            *n -= 1;
            if *n == 0 { self.registrations.remove(&kind); }
        }
        if self.registrations.is_empty() { self.reset(faults); }
    }

    /// `0x17e45f0`: map and bits cleared. Copy keys still taken can no longer be returned.
    pub fn reset(&mut self, faults: &mut Vec<Fault>) {
        for (key, n) in std::mem::take(&mut self.taken) {
            for _ in 0..n { faults.push(Fault::Kept { key }); }
        }
        self.registrations.clear();
        self.flags = 0;
    }

    /// One body construction: the path builder's gate (`0x17e99d8`), then the tail `0x60dec4`.
    pub fn construct(&mut self, kind: u32, g68: &mut BTreeMap<u32, u32>, faults: &mut Vec<Fault>) {
        if self.flags & REQUEST == 0 { faults.push(Fault::EmptyPath { kind }); }
        self.release_kind(kind, g68, faults);
    }
}

fn take_one(map: &mut BTreeMap<u32, u32>, key: u32) -> bool {
    match map.get_mut(&key) {
        Some(n) => {
            *n -= 1;
            if *n == 0 { map.remove(&key); }
            true
        }
        None => false,
    }
}

/// The roster `0x17e4f50` writes for our entry 0 and the other role's entry 1 (`(kinds, costume)`).
pub fn roster(own: ([u32; SUBS], u32), other: Option<([u32; SUBS], u32)>) -> Roster {
    let mut roster = [Record::DEFAULT; ENTRIES];
    roster[0] = Record { entry: 0, kinds: own.0, costume: own.1 };
    if let Some((kinds, costume)) = other { roster[1] = Record { entry: 1, kinds, costume }; }
    roster
}

/// A kind list as descriptor words (`-1` padded).
pub fn words(kinds: &[u32]) -> [u32; SUBS] {
    let mut words = [NO_KIND; SUBS];
    words[..kinds.len()].copy_from_slice(kinds);
    words
}

// ---------------------------------------------------------------------------------------------
// The bank's flows on one role, under a policy for the request bit
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// C116.1 before U19: `R+0x4c |= 1` when clear, never ended (GROUP-SWAP-IMPL §14.2).
    Today,
    /// U19: [`begin`] before the construction, [`end`] after it.
    Lease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    Planned,
    Prefetched,
    /// Bodies built (window or detached); its keys were consumed by the construction tails.
    Built,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `0x17e64d0(R, k)` for each kind of the group (`r_handle::acquire`).
    Prefetch(usize),
    /// Phase B of the group: U18's header and roster, the request bit, the synchronous
    /// construction (each body's tail), the end of the lease.
    Build(usize),
    /// `0x17e8610(R, k)` for each kind of a prefetched group (`r_handle::release`, a drain).
    Drain(usize),
    /// A completion call on this R from the game thread (`0x14f5734` or `0x66de64`).
    Complete,
}

/// One role's R through the bank's flows, with the other role's references in G68.
#[derive(Clone, Debug)]
pub struct Scene {
    pub policy: Policy,
    pub owner: Owner,
    /// The other owners' references (another R's completion taken and not yet returned).
    pub g68: BTreeMap<u32, u32>,
    pub groups: Vec<(Vec<u32>, GroupState)>,
    pub other: Option<([u32; SUBS], u32)>,
    pub faults: Vec<Fault>,
    /// `R+0x4c` after each Phase B, as the next flow finds it.
    pub after_builds: Vec<u8>,
}

impl Scene {
    /// The window's Reload, native from configure to the last tail: our R ends reset.
    pub fn new(policy: Policy, window: &[u32], costume: u32, groups: &[&[u32]], other: Option<([u32; SUBS], u32)>) -> Self {
        let mut scene = Scene { policy, owner: Owner::new(costume), g68: BTreeMap::new(),
            groups: groups.iter().map(|g| (g.to_vec(), GroupState::Planned)).collect(), other, faults: Vec::new(),
            after_builds: Vec::new() };
        let header = words(window);
        scene.owner.configure(header, roster((header, costume), other));
        for kind in window { scene.owner.register(*kind); }
        scene.owner.completion();
        for kind in window { scene.owner.construct(*kind, &mut scene.g68, &mut scene.faults); }
        scene
    }

    /// The other role's R between its completion and its tails holds these references.
    pub fn other_loading(&mut self, own_window: &[u32]) {
        let Some((kinds, costume)) = self.other else { return; };
        let roster = roster((words(own_window), self.owner.costume), self.other);
        for member in completion_members(kinds) {
            for key in acquired_keys(&roster, member, costume) { *self.g68.entry(key).or_insert(0) += 1; }
        }
    }

    pub fn apply(&mut self, op: Op) {
        match op {
            Op::Prefetch(g) if self.groups[g].1 == GroupState::Planned => {
                for kind in self.groups[g].0.clone() { self.owner.register(kind); }
                self.groups[g].1 = GroupState::Prefetched;
            }
            Op::Drain(g) if self.groups[g].1 == GroupState::Prefetched => {
                for kind in self.groups[g].0.clone() { self.owner.release_kind(kind, &mut self.g68, &mut self.faults); }
                self.groups[g].1 = GroupState::Planned;
            }
            Op::Build(g) if self.groups[g].1 == GroupState::Prefetched => {
                let kinds = self.groups[g].0.clone();
                // U18: the header and roster describe the incoming window before native runs.
                self.owner.header = words(&kinds);
                self.owner.roster = roster((words(&kinds), self.owner.costume), self.other);
                let lease = match self.policy {
                    Policy::Today => {
                        self.owner.flags |= REQUEST;
                        None
                    }
                    Policy::Lease => {
                        let (lease, write) = begin(self.owner.flags);
                        if let Some(byte) = write { self.owner.flags = byte; }
                        Some(lease)
                    }
                };
                for kind in &kinds { self.owner.construct(*kind, &mut self.g68, &mut self.faults); }
                if let Some(lease) = lease {
                    if let (_, Some(byte)) = end(lease, self.owner.flags) { self.owner.flags = byte; }
                }
                self.groups[g].1 = GroupState::Built;
                self.after_builds.push(self.owner.flags);
            }
            Op::Complete => self.owner.completion(),
            _ => {}
        }
    }

    /// Scene exit: the bank drains, the scene loop resets every R (`0x134efb4`).
    pub fn finish(&mut self) {
        for g in 0..self.groups.len() { self.apply(Op::Drain(g)); }
        self.owner.reset(&mut self.faults);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARIO: u32 = 0x00;
    const FOX: u32 = 0x07;
    const PIKACHU: u32 = 0x08;
    const SAMUS: u32 = 0x03;
    const BRAWLER: u32 = 0x48;
    const ROB: u32 = 0x2d;

    fn key(kind: u32, costume: u32) -> u32 { (kind << 17) | (costume << 25) | 0x1003f }

    fn all_rosters() -> Vec<Roster> {
        let descriptors: [Option<([u32; SUBS], u32)>; 7] = [
            None,
            Some(([KIRBY, NO_KIND, NO_KIND], 3)),
            Some(([MARIO, FOX, NO_KIND], 1)),
            Some(([BRAWLER, NO_KIND, NO_KIND], 5)),
            Some(([POPO, NANA, NO_KIND], 2)),
            Some(([0x4e, 0x4f, 0x50], 4)),
            Some(([FOX, KIRBY, SAMUS], 7)),
        ];
        let mut rosters = Vec::new();
        for a in &descriptors {
            for b in &descriptors {
                let mut roster = [Record::DEFAULT; ENTRIES];
                if let Some((kinds, costume)) = a { roster[0] = Record { entry: 0, kinds: *kinds, costume: *costume }; }
                if let Some((kinds, costume)) = b { roster[1] = Record { entry: 1, kinds: *kinds, costume: *costume }; }
                rosters.push(roster);
            }
        }
        rosters
    }

    /// The two listings agree: per (roster, kind, costume), `0x17f0290` returns exactly the
    /// multiset `0x17efb40` took. Native's pairing is per kind, so it holds only when every kind
    /// released while bit 2 is set is a kind the completion acquired for.
    #[test]
    fn the_two_listings_pair_per_kind_roster_and_costume() {
        let kinds = [MARIO, KIRBY, FOX, BRAWLER, 0x49, 0x4a, POPO, NANA, 0x4d, 0x4e, 0x50, ROB, 0x52, 0x75, NO_KIND];
        let mut nonempty = 0;
        for roster in all_rosters() {
            for kind in kinds {
                for costume in [0u32, 1, 7, 0xff] {
                    let mut acquired = acquired_keys(&roster, kind, costume);
                    let mut released = released_keys(&roster, kind, costume);
                    acquired.sort_unstable();
                    released.sort_unstable();
                    assert_eq!(acquired, released, "kind {:#x} costume {} roster {:?}", kind, costume, roster);
                    if !acquired.is_empty() { nonempty += 1; }
                }
            }
        }
        assert!(nonempty > 100, "{}", nonempty);
    }

    #[test]
    fn copy_keys_follow_the_listing() {
        assert_eq!(copy_key(FOX, 3), Some(key(FOX, 3)));
        assert_eq!(copy_key(BRAWLER, 3), Some(key(BRAWLER, 0)));
        for kind in [NO_KIND, KIRBY, 0x4d, 0x4e, 0x4f, 0x50, 0x8000_0000] { assert_eq!(copy_key(kind, 1), None); }
        assert_eq!(completion_members([POPO, NANA, NO_KIND]), vec![POPO]);
        assert_eq!(completion_members([MARIO, FOX, NO_KIND]), vec![MARIO, FOX]);
        assert_eq!(completion_members([NO_KIND, NO_KIND, SAMUS]), vec![NO_KIND, SAMUS]);
        // A non-Kirby kind takes one key per Kirby record, under that record's costume.
        let roster = roster(([MARIO, NO_KIND, NO_KIND], 2), Some(([KIRBY, NO_KIND, NO_KIND], 5)));
        assert_eq!(acquired_keys(&roster, MARIO, 2), vec![key(MARIO, 5)]);
        assert_eq!(acquired_keys(&roster, KIRBY, 5), vec![key(MARIO, 5)]);
    }

    #[test]
    fn the_lease_clears_only_the_bit_it_set() {
        for before in 0u8..8 {
            let (lease, write) = begin(before);
            assert_eq!(write, (before & 1 == 0).then_some(before | 1));
            for now in 0u8..8 {
                let (how, write) = end(lease, now);
                match how {
                    End::NotOurs => assert!(before & 1 == 1 && write.is_none()),
                    End::Restored => assert!(before & 1 == 0 && now == before | 1 && write == Some(before)),
                    End::ResetRan => assert!(before & 1 == 0 && now == 0 && write.is_none()),
                    End::Changed => assert!(before & 1 == 0 && now != before | 1 && now != 0 && write.is_none()),
                }
            }
        }
    }

    /// Native's own lifecycle is symmetric for every window shape the model covers (pair and PT
    /// windows excepted: their tails take the pair rule): after the Reload R is reset and nothing is
    /// kept or unpaired.
    #[test]
    fn a_native_reload_pairs_every_acquisition() {
        let windows: [&[u32]; 6] = [&[MARIO], &[KIRBY], &[KIRBY, FOX], &[FOX, SAMUS, PIKACHU], &[BRAWLER], &[ROB, MARIO]];
        let others = [None, Some(([KIRBY, NO_KIND, NO_KIND], 4)), Some(([FOX, KIRBY, NO_KIND], 1)), Some(([BRAWLER, NO_KIND, NO_KIND], 0))];
        for window in windows {
            for other in others {
                let scene = Scene::new(Policy::Today, window, 2, &[], other);
                assert!(scene.faults.is_empty(), "{:x?} {:x?}: {:x?}", window, other, scene.faults);
                assert_eq!(scene.owner.flags, 0);
                assert!(scene.owner.registrations.is_empty() && scene.owner.taken.is_empty());
            }
        }
    }

    /// The reported shape: two prefetched groups, Phase B of one; the other group's keys keep the
    /// map non-empty, so the reset does not run and bit 0 stays. A later completion takes keys for
    /// the header members and the drain hands back keys of the drained kind.
    #[test]
    fn today_a_build_beside_another_prefetch_leaves_bit0_and_unpairs_the_keys() {
        let other = Some(([KIRBY, NO_KIND, NO_KIND], 4));
        let mut scene = Scene::new(Policy::Today, &[MARIO], 2, &[&[FOX], &[SAMUS]], other);
        for op in [Op::Prefetch(0), Op::Prefetch(1), Op::Build(0)] { scene.apply(op); }
        assert_eq!(scene.after_builds, vec![REQUEST], "bit 0 outlives the build");
        scene.apply(Op::Complete);
        assert_eq!(scene.owner.flags, REQUEST | COMPLETED | ACQUIRED);
        assert_eq!(scene.owner.taken.keys().copied().collect::<Vec<_>>(), vec![key(FOX, 4)]);
        scene.apply(Op::Drain(1));
        assert_eq!(scene.faults, vec![
            Fault::Unpaired { key: key(SAMUS, 4), foreign: false },
            Fault::Kept { key: key(FOX, 4) },
        ]);
    }

    /// A key that an unpaired release finds belongs to whoever holds it: with a Kirby CPU between
    /// its completion and its tails, `0x17e2fe0` drops that R's reference (the K1 shape). The
    /// bank's flows below never reach it (`every_interleaving_pairs_under_the_lease` counts 0); the
    /// test pins the release semantics the model charges.
    #[test]
    fn an_unpaired_release_takes_whatever_reference_it_finds() {
        let other = Some(([KIRBY, NO_KIND, NO_KIND], 4));
        let mut scene = Scene::new(Policy::Today, &[MARIO], 2, &[&[FOX], &[SAMUS]], other);
        for op in [Op::Prefetch(0), Op::Prefetch(1), Op::Build(0), Op::Complete] { scene.apply(op); }
        scene.g68.insert(key(SAMUS, 4), 1);
        scene.apply(Op::Drain(1));
        assert_eq!(scene.faults[0], Fault::Unpaired { key: key(SAMUS, 4), foreign: true });
        assert!(scene.g68.is_empty());
    }

    /// Under the lease the same flows leave `R+0x4c` as Phase B found it (or reset), a completion
    /// after the build takes nothing, and every construction still sees bit 0.
    #[test]
    fn the_lease_ends_with_the_construction() {
        let other = Some(([KIRBY, NO_KIND, NO_KIND], 4));
        let mut scene = Scene::new(Policy::Lease, &[MARIO], 2, &[&[FOX], &[SAMUS]], other);
        for op in [Op::Prefetch(0), Op::Prefetch(1), Op::Build(0), Op::Complete, Op::Drain(1)] { scene.apply(op); }
        assert_eq!(scene.after_builds, vec![0]);
        assert!(scene.faults.is_empty(), "{:x?}", scene.faults);
        assert_eq!(scene.owner.flags, 0);
    }

    fn sequences(ops: &[Op], len: usize) -> Vec<Vec<Op>> {
        let mut out = vec![Vec::new()];
        let mut frontier = vec![Vec::new()];
        for _ in 0..len {
            let mut next = Vec::new();
            for seq in &frontier {
                for op in ops {
                    let mut s: Vec<Op> = seq.clone();
                    s.push(*op);
                    next.push(s);
                }
            }
            out.extend(next.iter().cloned());
            frontier = next;
        }
        out
    }

    /// Every shape and every interleaving of prefetch, build, drain and completion up to length 4,
    /// with and without the other role loading: the lease pairs every copy key and leaves no bit
    /// behind; today's policy keeps keys and releases unpaired ones wherever a build leaves other
    /// keys on R and a completion follows (none of its stray releases meets another owner's
    /// reference in these flows).
    #[test]
    fn every_interleaving_pairs_under_the_lease() {
        let windows: [&[u32]; 3] = [&[MARIO], &[KIRBY], &[BRAWLER, FOX]];
        let group_sets: [[&[u32]; 2]; 4] = [
            [&[FOX], &[SAMUS]],
            [&[KIRBY], &[PIKACHU, SAMUS]],
            [&[BRAWLER], &[KIRBY, FOX]],
            [&[ROB, SAMUS], &[PIKACHU]],
        ];
        let others = [None, Some(([KIRBY, NO_KIND, NO_KIND], 4)), Some(([FOX, NO_KIND, NO_KIND], 1))];
        let ops = [Op::Prefetch(0), Op::Prefetch(1), Op::Build(0), Op::Build(1), Op::Drain(0), Op::Drain(1), Op::Complete];
        let seqs = sequences(&ops, 4);
        let (mut runs, mut today_faulted, mut today_foreign) = (0usize, 0usize, 0usize);
        for window in windows {
            for groups in group_sets {
                if groups.iter().any(|g| g.iter().any(|k| window.contains(k))) { continue; }
                for other in others {
                    for loading in [false, true] {
                        for seq in &seqs {
                            for policy in [Policy::Today, Policy::Lease] {
                                let mut scene = Scene::new(policy, window, 2, &groups, other);
                                if loading { scene.other_loading(window); }
                                for op in seq { scene.apply(*op); }
                                scene.finish();
                                let paths = scene.faults.iter().any(|f| matches!(f, Fault::EmptyPath { .. }));
                                assert!(!paths, "{:?} {:x?} {:x?}: {:x?}", policy, window, seq, scene.faults);
                                match policy {
                                    Policy::Lease => {
                                        assert!(scene.faults.is_empty(), "{:x?} {:x?} {:x?} {:x?}: {:x?}", window, groups, other, seq, scene.faults);
                                        assert!(scene.after_builds.iter().all(|f| *f == 0), "{:x?}: {:?}", seq, scene.after_builds);
                                        runs += 1;
                                    }
                                    Policy::Today => {
                                        if !scene.faults.is_empty() { today_faulted += 1; }
                                        if scene.faults.iter().any(|f| matches!(f, Fault::Unpaired { foreign: true, .. })) {
                                            today_foreign += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(runs > 100_000, "{}", runs);
        // Today faults need both groups prefetched, a build of one, then a completion while a Kirby
        // record or a Kirby header is in play: 112 of the 4-step interleavings.
        assert!(today_faulted >= 100, "{}", today_faulted);
        assert_eq!(today_foreign, 0);
    }
}
