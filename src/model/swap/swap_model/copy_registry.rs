//! U18 — the Kirby copy registry as a pure model, and the writer-side protocol that keeps it
//! native-valid while Phase B groups share an entry (`D175/refactor/u18/REPORT.md`).
//!
//! FACTs (owned 13.0.5 ELF `0789f0a4…`, listings in `u18/re/` and `wp-k/re/`):
//! - **Registration** runs once per body, at construction: init `0x6079d0` derives the body's
//!   `sub` as the first `i < E+0x14` with `E+0x18[i] == kind` (`0x607a5c`, stored at
//!   `body+0xf816`), then calls `prepare_entry 0x17f1a60(G68, R+0xb0, R+0x58[sub], sel(R+0x58,
//!   R+0x64), R+0x54, sub)` (`0x607eb4–0x607f30`): the kind is the **header** word at the body's
//!   cell, the costume is the selector over the header's **primary** `R+0x58` (R.O.B. regional,
//!   Mii 0), the roster is `R+0xb0`.
//! - **Unregistration** runs once per body, at unload: `0x610f20` calls `unprepare 0x17f26c0(G68,
//!   body+0xc, body+0x10, body+0xf816)` (`0x6118c0–0x6118d0`): the body's **own** kind.
//! - The registry is one list per **(entry, sub)**, `G68+0x1d258+e*0x48+s*0x18`, with the kind
//!   registered there at `G68+0x1d498+e*0xc+s*4`. `prepare_entry` first frees whatever the cell holds
//!   without releasing it (`0x17f1b18`), so every body ever built at the same (entry, sub) shares it —
//!   the window's body and each detached group's body at that index.
//! - The roster `R+0xb0` (8 records of 0x80) is written only by `0x17e4f50(R)`: defaults, then for
//!   each fighter-manager entry `E_i` its 0x70-byte descriptor (`E_i+0x70` while `E_i+0x5935 != 0 &&
//!   E_i+0x591c <= 4`) at `rec+0x10` and `E_i+0xe0` at `rec+0x8`. State-6 configure calls it at
//!   `0x651380`, right after it wrote `R+0x54..0x80` from the staged descriptor (`0x651334–0x65137c`).
//!   A Phase B group build writes only `E+0x14/0x18..0x20/0x28` and runs neither.
//!
//! Native's invariant is one body per (entry, sub), registered under its own kind with its own
//! entry's record. The bank breaks it twice: a Phase B body registers under the home window's
//! header and record (WP-R), and the window and its detached groups share the cells. Fixing only
//! the header and the record (the 17:3x correction) is not enough: a body still unloads over the
//! nodes another body registered in the same cell (`k15_header_only_still_self_mirrors`). The
//! protocol below gives the registry exactly what native would hold for the window that is
//! attached — the bodies native "has" — and nothing for detached groups.

use crate::engine::swap::body_bank::Members;
use std::collections::BTreeMap;

/// Fighter-manager entries, and the per-entry cells of the registry.
pub const ENTRIES: usize = 8;
pub const SUBS: usize = 3;
pub const KIRBY: u32 = 0x06;
const POPO: u32 = 0x4b;
const NANA: u32 = 0x4c;
const ROB: u32 = 0x2d;
/// `-1` as the native int tables and descriptors store it.
pub const NO_KIND: u32 = u32::MAX;

/// Early returns shared by `prepare_entry` (`0x17f1a80–0x17f1abc`) and `unprepare`
/// (`0x17f26e0–0x17f2710`): kind −1, `0x4d..=0x50`, negative, and Popo/Nana at `sub > 0`.
pub fn skipped(kind: u32, sub: usize) -> bool {
    (kind as i32) < 0 || (0x4d..=0x50).contains(&kind) || ((kind == POPO || kind == NANA) && sub > 0)
}

/// The costume init hands `prepare_entry` (`0x607ebc–0x607f20`): the selector over the header's
/// primary `R+0x58`. R.O.B. swaps costumes 0 and 1 when the region word `*(main+0x523d00c)` is
/// above 13 or one of 1..=10; a Mii primary gives 0; anything else is `R+0x64`. Only Kirby-branch
/// tuples carry it (the other branch takes the Kirby record's `+0x51`), and Kirby is the primary of
/// any window that holds it (`CellRule::PinCell0`), so it equals the costume selected on the body's
/// own kind (WP-R `identity_view`) in every shape the bank builds.
pub fn registry_costume(primary: u32, costume: u32, region: u32) -> u32 {
    if primary == ROB {
        let regional = region > 0xd || (0x7fe_u32 >> region) & 1 != 0;
        if !regional { return costume; }
        match costume { 0 => 1, 1 => 0, other => other }
    } else if (0x48..=0x4a).contains(&primary) {
        0
    } else {
        costume
    }
}

/// `{kind @+0x10, costume @+0x14, ctx @+0x18}` of a registry node; `ctx = kirby_entry<<7 |
/// source_entry<<3 | source_sub`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tuple { pub kind: u32, pub costume: u32, pub ctx: u32 }

/// One roster record: `rec+0x8` entry (`E+0xe0`), `rec+0x28/0x2c/0x30` = the entry's descriptor
/// kinds `E+0x18..0x20`, `rec+0x51` = its costume `E+0x41`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record { pub entry: i32, pub kinds: [u32; SUBS], pub costume: u32 }

impl Record {
    /// The defaults `0x17e4f50` writes before copying (entry −1, kinds −1).
    pub const DEFAULT: Self = Self { entry: -1, kinds: [NO_KIND; SUBS], costume: 0 };
}

pub type Roster = [Record; ENTRIES];

/// `0x17e4f50(R)` over the entries' current descriptors (`(kinds, costume)` per entry, `None` for
/// no entry): record `i` describes entry `i`.
pub fn roster_of(descriptors: &[Option<([u32; SUBS], u32)>; ENTRIES]) -> Roster {
    let mut roster = [Record::DEFAULT; ENTRIES];
    for (i, descriptor) in descriptors.iter().enumerate() {
        if let Some((kinds, costume)) = descriptor {
            roster[i] = Record { entry: i as i32, kinds: *kinds, costume: *costume };
        }
    }
    roster
}

/// WP-K's hazard predicate (`RE-KIRBY-UNLOAD.md` §5): a node in `cell[entry][sub]` whose mirror cell,
/// as the unloading body's branch computes it, is that same cell. The Kirby walk mirrors into
/// `cell[(ctx>>3)&15][ctx&7]` (`0x17f2800–0x17f2814`); every other kind into `cell[ctx>>7][0]`
/// (`0x17f28f0–0x17f2900`). Such a node is removed twice: the count reaches −1, the sentinel is
/// freed and the loop spins (K-R1).
pub fn self_mirror(walker: u32, entry: usize, sub: usize, ctx: u32) -> bool {
    if walker == KIRBY {
        ((ctx >> 3) & 15) as usize == entry && (ctx & 7) as usize == sub
    } else {
        sub == 0 && ((ctx as i32) >> 7) as usize == entry
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The mirror search of an unload found its node in the walked cell itself (removed twice).
    SelfMirror { entry: usize, sub: usize, walker: u32, tuple: Tuple },
    /// `0x17f2858`/`0x17f2944` popped the sentinel: count −1, `free(sentinel)`, the walk never ends.
    SentinelPop { entry: usize, sub: usize, walker: u32 },
}

#[derive(Clone, Copy, Debug)]
struct Node { tuple: Tuple }

/// The registry: `cells`, the kind ints, the per-tuple registration count `prepare_tuple`
/// takes and `release_tuple 0x17f1360` returns, the nodes a clear freed unreleased, and faults.
#[derive(Clone, Debug)]
pub struct Registry {
    cells: [[Vec<Node>; SUBS]; ENTRIES],
    kinds: [[u32; SUBS]; ENTRIES],
    refs: BTreeMap<Tuple, i64>,
    pub dropped: usize,
    pub faults: Vec<Fault>,
}

impl Default for Registry {
    fn default() -> Self { Self::new() }
}

impl Registry {
    /// As the G68 constructor leaves it (`0x17edd34–0x17ee1ac`): 24 empty cells, kinds −1.
    pub fn new() -> Self {
        Self { cells: Default::default(), kinds: [[NO_KIND; SUBS]; ENTRIES], refs: BTreeMap::new(), dropped: 0,
            faults: Vec::new() }
    }

    pub fn len(&self, entry: usize, sub: usize) -> usize { self.cells[entry][sub].len() }
    pub fn kind(&self, entry: usize, sub: usize) -> u32 { self.kinds[entry][sub] }
    pub fn is_empty(&self) -> bool { self.cells.iter().flatten().all(Vec::is_empty) }
    /// Every registration taken was returned.
    pub fn balanced(&self) -> bool { self.refs.values().all(|r| *r == 0) }
    /// The registrations `prepare_tuple` holds, per tuple (each loads the tuple's copy models).
    pub fn registrations(&self) -> Vec<(Tuple, i64)> { self.refs.iter().map(|(t, r)| (*t, *r)).collect() }
    /// The predicate over one cell for a body of kind `walker` about to unload there.
    pub fn hazard(&self, walker: u32, entry: usize, sub: usize) -> bool {
        self.cells[entry][sub].iter().any(|n| self_mirror(walker, entry, sub, n.tuple.ctx))
    }

    /// One relation: `prepare_tuple 0x17f0b90` pushes the own node and takes the registration
    /// unless the kind is −1 or 6 (`0x17f0bb0–0x17f0bbc`); the mirror push is unconditional.
    fn relate(&mut self, tuple: Tuple, own: (usize, usize), mirror: (usize, usize)) {
        if tuple.kind != NO_KIND && tuple.kind != KIRBY {
            self.cells[own.0][own.1].push(Node { tuple });
            *self.refs.entry(tuple).or_insert(0) += 1;
        }
        self.cells[mirror.0][mirror.1].push(Node { tuple });
    }

    /// `prepare_entry 0x17f1a60(G68, roster, kind, costume, entry, sub)`.
    pub fn prepare_entry(&mut self, roster: &Roster, kind: u32, costume: u32, entry: usize, sub: usize) {
        if skipped(kind, sub) { return; }
        let cleared = std::mem::take(&mut self.cells[entry][sub]);
        self.dropped += cleared.len();
        self.kinds[entry][sub] = kind;
        if kind == KIRBY {
            // `0x17f1ba8–0x17f1ed4`: every record but the ones whose k0 is −1 or Kirby.
            for record in roster {
                let k0 = record.kinds[0];
                if k0 == NO_KIND || k0 == KIRBY || (0x4d..=0x50).contains(&k0) || (k0 as i32) < 0 { continue; }
                let source = record.entry as usize;
                let base = ((entry as u32) << 7) | ((source as u32) << 3);
                let int0 = self.kinds[source][0];
                if int0 == NO_KIND || int0 == k0 {
                    self.relate(Tuple { kind: k0, costume, ctx: base }, (entry, sub), (source, 0));
                }
                for j in 1..SUBS {
                    let kj = record.kinds[j];
                    if kj == POPO || kj == NANA || kj == NO_KIND { continue; }
                    let intj = self.kinds[source][j];
                    if intj == NO_KIND || intj == kj {
                        self.relate(Tuple { kind: kj, costume, ctx: base | j as u32 }, (entry, sub), (source, j));
                    }
                }
            }
        } else {
            // `0x17f1edc–0x17f2698`: one relation per record whose k0 is Kirby, mirrored into its sub 0.
            for record in roster {
                if record.kinds[0] != KIRBY { continue; }
                let kirby = record.entry as usize;
                let int0 = self.kinds[kirby][0];
                if int0 != KIRBY && int0 != NO_KIND { continue; }
                let ctx = ((kirby as u32) << 7) | ((entry as u32) << 3) | sub as u32;
                self.relate(Tuple { kind, costume: record.costume, ctx }, (entry, sub), (kirby, 0));
            }
        }
    }

    /// `unprepare 0x17f26c0(G68, kind, entry, sub)` with the native walk exactly: pop the back node,
    /// release it, remove the first node with the same tuple from the mirror cell, pop the back
    /// again; stop at a sentinel pop (the native loop never ends there).
    pub fn unprepare(&mut self, kind: u32, entry: usize, sub: usize) {
        if skipped(kind, sub) { return; }
        while let Some(back) = self.cells[entry][sub].last().copied() {
            *self.refs.entry(back.tuple).or_insert(0) -= 1;
            let ctx = back.tuple.ctx;
            let (me, ms) = if kind == KIRBY {
                (((ctx >> 3) & 15) as usize, (ctx & 7) as usize)
            } else {
                ((((ctx as i32) >> 7) as usize), 0)
            };
            if me < ENTRIES && ms < SUBS {
                if let Some(at) = self.cells[me][ms].iter().position(|n| n.tuple == back.tuple) {
                    if (me, ms) == (entry, sub) {
                        self.faults.push(Fault::SelfMirror { entry, sub, walker: kind, tuple: back.tuple });
                    }
                    self.cells[me][ms].remove(at);
                }
            }
            if self.cells[entry][sub].pop().is_none() {
                self.faults.push(Fault::SentinelPop { entry, sub, walker: kind });
                break;
            }
        }
        self.kinds[entry][sub] = NO_KIND;
    }
}

/// The adapter's native side of the protocol, one role (one entry, one `R`) at a time.
pub trait RegistryOps {
    /// `unprepare 0x17f26c0(G68, kinds[i], R+0x54, i)` for every cell of `members`, in cell order:
    /// exactly what those bodies' unloads would do, without unloading them.
    fn unregister(&mut self, members: Members);
    /// `R+0x58/0x5c/0x60 = members.kinds()`: the header native configure writes for that window.
    fn write_header(&mut self, members: Members);
    /// `0x17e4f50(R)`: the roster native configure writes right after the header.
    fn refresh_roster(&mut self);
    /// `prepare_entry 0x17f1a60(G68, R+0xb0, R+0x58[i], sel(R+0x58, R+0x64), R+0x54, i)` in cell
    /// order: exactly what those bodies' inits did at construction.
    fn register(&mut self, members: Members);
}

/// Phase B build: before the synchronous construction of `incoming`, the outgoing window leaves
/// the registry and the entry's identity becomes the incoming group's, as a native Reload into it
/// would leave them (its bodies' unloads, then configure). The construction registers `incoming`.
pub fn build<O: RegistryOps>(ops: &mut O, outgoing: Members, incoming: Members) {
    ops.unregister(outgoing);
    ops.write_header(incoming);
    ops.refresh_roster();
}

/// Warm exchange: `incoming` (built, detached, unregistered) becomes the window. Its descriptor
/// words are already restored when this runs (the roster copies them).
pub fn attach<O: RegistryOps>(ops: &mut O, outgoing: Members, incoming: Members) {
    build(ops, outgoing, incoming);
    ops.register(incoming);
}

/// A failed build or warm exchange: whatever `failed` registered leaves (a destroyed body's
/// unload already did it; the call is then a no-op on empty cells) and the restored window
/// registers again.
pub fn restore<O: RegistryOps>(ops: &mut O, failed: Members, restored: Members) {
    attach(ops, failed, restored);
}

/// Detached teardown (`retire_groups`): detached bodies hold no registration, but they unload
/// over the cells the window's registrations occupy. The window leaves, each detached group is
/// destroyed under its own header (its unload costumes are the ones its construction used), then
/// the window registers again under its own header. No roster refresh: the entry may already be
/// staging a Reload (`E+0x5935`, `0x17e4f50` would copy `E+0x70`), and the roster still describes
/// the window from its own configure or attach. `destroy` destroys one group's bodies (`0x3aeb50`
/// per body: unload, `unprepare`).
pub fn teardown<O: RegistryOps, B>(ops: &mut O, window: Members, detached: &[(Members, B)],
    mut destroy: impl FnMut(&mut O, Members, &B)) {
    if detached.is_empty() { return; }
    teardown_begin(ops, window);
    for (members, bodies) in detached {
        teardown_group(ops, *members);
        destroy(ops, *members, bodies);
    }
    teardown_end(ops, window);
}

/// [`teardown`] in steps, for an adapter whose destroyer runs the bodies itself (WP3's
/// `body_graph::execute`): before the first detached body, the window leaves the registry.
pub fn teardown_begin<O: RegistryOps>(ops: &mut O, window: Members) { ops.unregister(window); }

/// Right before the first body of `group` is destroyed: its own header.
pub fn teardown_group<O: RegistryOps>(ops: &mut O, group: Members) { ops.write_header(group); }

/// After the last detached body: the window's header, and its registrations again.
pub fn teardown_end<O: RegistryOps>(ops: &mut O, window: Members) {
    ops.write_header(window);
    ops.register(window);
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const YOSHI: u32 = 0x05;
    const FOX: u32 = 0x07;
    const FALCO: u32 = 0x14;
    const FALCON: u32 = 0x0b;
    const MARIO: u32 = 0x00;
    const CHROM: u32 = 0x1b;
    const LUCINA: u32 = 0x16;
    const MARTH: u32 = 0x15;
    const COSTUME: [u32; ENTRIES] = [2, 5, 0, 0, 0, 0, 0, 0];

    fn m(kinds: &[u32]) -> Members { Members::new(kinds).unwrap() }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Policy {
        /// M1 today: no writer; the header and record stay the last native configure's.
        Today,
        /// The 17:3x correction alone: header and roster follow the window, no registry protocol.
        HeaderOnly,
        /// U18: header, roster and the registry follow the window.
        U18,
    }

    /// One scene: the registry, every entry's descriptor (`E+0x18..`), header (`R+0x58..`) and roster
    /// (`R+0xb0`), and for each banked entry its window and detached groups. A body is (kind, cell).
    struct World {
        reg: Registry,
        descriptor: [Option<[u32; SUBS]>; ENTRIES],
        header: [[u32; SUBS]; ENTRIES],
        roster: [Roster; ENTRIES],
        window: [Option<Members>; ENTRIES],
        detached: [Vec<Members>; ENTRIES],
        policy: Policy,
        /// The entry the `RegistryOps` calls act on.
        at: usize,
        /// Invariant violations seen after U18 operations.
        violations: Vec<String>,
    }

    impl RegistryOps for World {
        fn unregister(&mut self, members: Members) {
            for (i, kind) in members.kinds().iter().enumerate().take(members.count()) {
                self.reg.unprepare(*kind, self.at, i);
            }
        }
        fn write_header(&mut self, members: Members) { self.header[self.at] = members.kinds(); }
        fn refresh_roster(&mut self) {
            let descriptors = std::array::from_fn(|e| self.descriptor[e].map(|kinds| (kinds, COSTUME[e])));
            self.roster[self.at] = roster_of(&descriptors);
        }
        fn register(&mut self, members: Members) {
            let e = self.at;
            for i in 0..members.count() {
                let costume = registry_costume(self.header[e][0], COSTUME[e], 0);
                let roster = self.roster[e];
                self.reg.prepare_entry(&roster, self.header[e][i], costume, e, i);
            }
        }
    }

    impl World {
        fn new(policy: Policy) -> Self {
            Self { reg: Registry::new(), descriptor: [None; ENTRIES], header: [[NO_KIND; SUBS]; ENTRIES],
                roster: [[Record::DEFAULT; ENTRIES]; ENTRIES], window: [None; ENTRIES], detached: Default::default(),
                policy, at: 0, violations: Vec::new() }
        }

        /// Native construction of `members` at entry `e`: one init per body in cell order.
        fn construct(&mut self, e: usize, members: Members) {
            for i in 0..members.count() {
                let costume = registry_costume(self.header[e][0], COSTUME[e], 0);
                let roster = self.roster[e];
                self.reg.prepare_entry(&roster, self.header[e][i], costume, e, i);
            }
        }

        /// Body unloads in cell order: `unprepare` with each body's own kind.
        fn unload(&mut self, e: usize, members: Members) {
            for (i, kind) in members.kinds().iter().enumerate().take(members.count()) {
                self.reg.unprepare(*kind, e, i);
            }
        }

        /// U18's invariant after every operation: each attached body is registered under its own kind
        /// (or skipped natively) and its cell holds no node that would self-mirror at its unload.
        fn check(&mut self, what: &str) {
            if self.policy != Policy::U18 { return; }
            for e in 0..ENTRIES {
                let Some(window) = self.window[e] else { continue };
                for (i, kind) in window.kinds().iter().enumerate().take(window.count()) {
                    let expected = if skipped(*kind, i) { None } else { Some(*kind) };
                    if expected.is_some_and(|k| self.reg.kind(e, i) != k) || self.reg.hazard(*kind, e, i) {
                        self.violations.push(format!("{what}: entry {e} cell {i} kind {kind:#x} registered {:#x} hazard {}",
                            self.reg.kind(e, i), self.reg.hazard(*kind, e, i)));
                    }
                }
                // Cells past the window hold nothing: no detached body's registration survives.
                for i in window.count()..SUBS {
                    if self.reg.kind(e, i) != NO_KIND || self.reg.len(e, i) != 0 {
                        self.violations.push(format!("{what}: entry {e} cell {i} past the window holds {:#x}/{}",
                            self.reg.kind(e, i), self.reg.len(e, i)));
                    }
                }
            }
        }

        /// Detached groups go before every native destroyer (`retire_groups`).
        fn retire(&mut self, e: usize) {
            let detached = std::mem::take(&mut self.detached[e]);
            if detached.is_empty() { return; }
            match self.policy {
                Policy::U18 => {
                    self.at = e;
                    let window = self.window[e].unwrap();
                    let groups: Vec<(Members, ())> = detached.iter().map(|g| (*g, ())).collect();
                    teardown(self, window, &groups, |world, members, _| world.unload(e, members));
                }
                Policy::Today | Policy::HeaderOnly => {
                    for g in detached { self.unload(e, g); }
                }
            }
            self.check("teardown");
        }

        /// A native Reload of entry `e` into `window` (canonical Reload or a window bank build):
        /// detached groups, then the window's unloads, configure, construction.
        fn reload(&mut self, e: usize, window: Members) {
            self.retire(e);
            if let Some(old) = self.window[e].take() { self.unload(e, old); }
            self.descriptor[e] = Some(window.kinds());
            self.header[e] = window.kinds();
            let descriptors = std::array::from_fn(|i| self.descriptor[i].map(|kinds| (kinds, COSTUME[i])));
            self.roster[e] = roster_of(&descriptors);
            self.construct(e, window);
            self.window[e] = Some(window);
            self.check("reload");
        }

        /// Phase B build of `group` at entry `e`: the window detaches, the group is built and attached.
        fn build_group(&mut self, e: usize, group: Members) {
            let outgoing = self.window[e].unwrap();
            self.descriptor[e] = Some(group.kinds());
            match self.policy {
                Policy::Today => {}
                Policy::HeaderOnly => {
                    self.at = e;
                    self.write_header(group);
                    self.refresh_roster();
                }
                Policy::U18 => {
                    self.at = e;
                    build(self, outgoing, group);
                }
            }
            self.construct(e, group);
            self.detached[e].push(outgoing);
            self.window[e] = Some(group);
            self.check("build");
        }

        /// A build that fails after `built` bodies: those are destroyed and the window comes back.
        fn failed_build(&mut self, e: usize, group: Members, built: usize) {
            let outgoing = self.window[e].unwrap();
            self.descriptor[e] = Some(group.kinds());
            if self.policy == Policy::U18 { self.at = e; build(self, outgoing, group); }
            for i in 0..built {
                let costume = registry_costume(self.header[e][0], COSTUME[e], 0);
                let roster = self.roster[e];
                self.reg.prepare_entry(&roster, self.header[e][i], costume, e, i);
            }
            for i in 0..built { self.reg.unprepare(group.kinds()[i], e, i); }
            self.descriptor[e] = Some(outgoing.kinds());
            if self.policy == Policy::U18 { self.at = e; restore(self, group, outgoing); }
            self.check("rollback");
        }

        /// Warm exchange to the detached group holding `kind`.
        fn swap_to(&mut self, e: usize, kind: u32) {
            let at = self.detached[e].iter().position(|g| g.index_of(kind).is_some()).unwrap();
            let incoming = self.detached[e].remove(at);
            let outgoing = self.window[e].unwrap();
            self.descriptor[e] = Some(incoming.kinds());
            match self.policy {
                Policy::Today => {}
                Policy::HeaderOnly => { self.at = e; self.write_header(incoming); self.refresh_roster(); }
                Policy::U18 => { self.at = e; attach(self, outgoing, incoming); }
            }
            self.detached[e].push(outgoing);
            self.window[e] = Some(incoming);
            self.check("attach");
        }

        /// End of the scene: every entry's groups, then its window (finalize).
        fn finish(&mut self) {
            for e in 0..ENTRIES {
                self.retire(e);
                if let Some(window) = self.window[e].take() { self.unload(e, window); }
                self.descriptor[e] = None;
            }
        }

        fn clean(&self) -> bool {
            self.reg.faults.is_empty() && self.violations.is_empty() && self.reg.dropped == 0
                && self.reg.balanced() && self.reg.is_empty()
        }
    }

    /// The deciding native shapes: S-B (`[Yoshi, Kirby, Fox]`, Kirby at cell 1) leaves a self-mirror in
    /// Yoshi's cell 0 and spins on its sentinel while Yoshi unloads (K-R1: TaskWorker2 in the
    /// non-Kirby loop, the `[0][0]` sentinel popped, count −1); S-K and S-B's PinCell0 layout complete.
    fn the_transcription_reproduces_k_r1() {
        let mut reg = Registry::new();
        let mut descriptors = [None; ENTRIES];
        descriptors[0] = Some(([YOSHI, KIRBY, FOX], COSTUME[0]));
        descriptors[1] = Some(([FALCON, NO_KIND, NO_KIND], COSTUME[1]));
        let roster = roster_of(&descriptors);
        reg.prepare_entry(&roster, FALCON, COSTUME[1], 1, 0);
        // `Members::new` refuses Kirby at cell 1 (PinCell0): the old bank's entry, cell for cell.
        for (i, kind) in [YOSHI, KIRBY, FOX].iter().enumerate() { reg.prepare_entry(&roster, *kind, COSTUME[0], 0, i); }
        assert!(reg.hazard(YOSHI, 0, 0) && reg.hazard(KIRBY, 0, 1) && !reg.hazard(FOX, 0, 2));
        for (i, kind) in [YOSHI, KIRBY, FOX].iter().enumerate() { reg.unprepare(*kind, 0, i); }
        assert!(matches!(reg.faults.first(), Some(Fault::SelfMirror { entry: 0, sub: 0, walker: YOSHI, .. })), "{:?}", reg.faults);
        assert!(reg.faults.iter().any(|f| matches!(f, Fault::SentinelPop { entry: 0, sub: 0, walker: YOSHI })));
        for window in [m(&[KIRBY, FOX, 0x08]), Members::laid_out(&[YOSHI, KIRBY, FOX]).unwrap()] {
            let mut world = World::new(Policy::Today);
            world.reload(1, m(&[FALCON]));
            world.reload(0, window);
            world.finish();
            assert!(world.clean(), "{:x?} {:?}", window.kinds(), world.reg.faults);
        }
    }

    fn the_costume_is_init_s_selector_on_the_primary() {
        assert_eq!(registry_costume(KIRBY, 3, 0), 3);
        assert_eq!(registry_costume(0x48, 3, 0), 0);
        assert_eq!(registry_costume(ROB, 0, 0), 0);
        assert_eq!(registry_costume(ROB, 0, 1), 1);
        assert_eq!(registry_costume(ROB, 1, 10), 0);
        assert_eq!(registry_costume(ROB, 5, 10), 5);
        assert_eq!(registry_costume(ROB, 0, 11), 0);
        assert_eq!(registry_costume(ROB, 0, 14), 1);
    }

    fn the_hazard_predicate_is_wp_k_s() {
        // Kirby walk: the source cell; any other walk: sub 0 of the Kirby entry.
        // ctx = kirby_entry<<7 | source_entry<<3 | source_sub
        let ctx = |kirby: u32, source: u32, sub: u32| (kirby << 7) | (source << 3) | sub;
        assert!(self_mirror(KIRBY, 0, 1, ctx(0, 0, 1)));
        assert!(!self_mirror(KIRBY, 0, 0, ctx(0, 1, 0)));
        assert!(self_mirror(YOSHI, 0, 0, ctx(0, 1, 0)));
        assert!(!self_mirror(YOSHI, 0, 1, ctx(0, 1, 1)));
        assert!(!self_mirror(YOSHI, 0, 0, ctx(1, 0, 0)));
    }

    /// The opponents a Training scene can hold: a non-Kirby CPU, a Kirby CPU (its own PinCell0
    /// window), a CPU window with Kirby, or none.
    fn cpus() -> Vec<Option<Members>> {
        vec![Some(m(&[FALCON])), Some(m(&[KIRBY])), Some(m(&[KIRBY, MARIO])), Some(m(&[MARIO, FALCO])), None]
    }

    /// Every sequence a group lives through: built and torn down attached (the Reload that ends it),
    /// built then swapped back to the window and torn down detached, swapped back and forth, two
    /// groups, a failed build, and the CPU reloading while the group is resident.
    #[derive(Clone, Copy, Debug)]
    enum Sequence { BuildReload, BuildReturnReload, BuildReturnAgainReload, TwoGroups, FailedBuild, CpuReloads, Finalize }

    const SEQUENCES: [Sequence; 7] = [Sequence::BuildReload, Sequence::BuildReturnReload, Sequence::BuildReturnAgainReload,
        Sequence::TwoGroups, Sequence::FailedBuild, Sequence::CpuReloads, Sequence::Finalize];

    fn run(policy: Policy, home: Members, group: Members, second: Members, cpu: Option<Members>, sequence: Sequence) -> World {
        let mut world = World::new(policy);
        if let Some(cpu) = cpu { world.reload(1, cpu); }
        world.reload(0, home);
        let first = home.kinds()[0];
        match sequence {
            Sequence::BuildReload => { world.build_group(0, group); world.reload(0, m(&[CHROM, MARTH])); }
            Sequence::BuildReturnReload => {
                world.build_group(0, group); world.swap_to(0, first); world.reload(0, m(&[CHROM, MARTH]));
            }
            Sequence::BuildReturnAgainReload => {
                world.build_group(0, group); world.swap_to(0, first); world.swap_to(0, group.kinds()[0]);
                world.reload(0, m(&[CHROM, MARTH]));
            }
            Sequence::TwoGroups => {
                world.build_group(0, group); world.build_group(0, second); world.swap_to(0, first);
                world.swap_to(0, group.kinds()[0]); world.reload(0, m(&[CHROM, MARTH]));
            }
            Sequence::FailedBuild => {
                world.failed_build(0, group, 1); world.build_group(0, group); world.reload(0, m(&[CHROM, MARTH]));
            }
            Sequence::CpuReloads => {
                world.build_group(0, group);
                if cpu.is_some() { world.reload(1, m(&[KIRBY])); }
                world.swap_to(0, first);
                if cpu.is_some() { world.reload(1, m(&[FALCON])); }
                world.reload(0, m(&[CHROM, MARTH]));
            }
            Sequence::Finalize => { world.build_group(0, group); }
        }
        world.finish();
        world
    }

    fn homes() -> Vec<Members> {
        vec![m(&[KIRBY, YOSHI, FOX]), m(&[KIRBY, YOSHI]), m(&[YOSHI, FOX, FALCO]), m(&[YOSHI, FOX])]
    }

    /// Groups the planner forms: ordinary (one to three bodies), a PinCell0 Kirby group, and the
    /// special groups whose cells native skips (the pair's Nana) or which carry records (PT).
    fn groups() -> Vec<Members> {
        vec![m(&[CHROM, LUCINA, MARTH]), m(&[CHROM]), m(&[KIRBY, CHROM, LUCINA]), m(&[KIRBY]), m(&[KIRBY, LUCINA]),
            Members::pair_group(), Members::native_group(0x24).unwrap()]
    }

    /// K14/K15 and every neighbour: under U18 each shape registers and unregisters symmetrically —
    /// no fault, no unreleased clear, every registration returned, the registry empty at the end,
    /// and after every operation each attached body registered under its own kind with no
    /// self-mirror in its cell (WP-K's predicate).
    fn every_k14_k15_shape_registers_and_unregisters_symmetrically() {
        let mut shapes = 0;
        for home in homes() {
            for group in groups() {
                if (0..group.count()).any(|i| home.index_of(group.kinds()[i]).is_some()) { continue; }
                for cpu in cpus() {
                    for sequence in SEQUENCES {
                        let second = if group.index_of(KIRBY).is_some() || home.index_of(KIRBY).is_some() {
                            m(&[MARIO, 0x08])
                        } else { m(&[KIRBY, MARIO]) };
                        if (0..second.count()).any(|i| home.index_of(second.kinds()[i]).is_some()
                            || group.index_of(second.kinds()[i]).is_some()) { continue; }
                        let world = run(Policy::U18, home, group, second, cpu, sequence);
                        assert!(world.clean(), "home {:x?} group {:x?} cpu {:x?} {:?}: faults {:?} violations {:?} dropped {} balanced {} empty {}",
                            home.kinds(), group.kinds(), cpu.map(|c| c.kinds()), sequence, world.reg.faults, world.violations,
                            world.reg.dropped, world.reg.balanced(), world.reg.is_empty());
                        shapes += 1;
                    }
                }
            }
        }
        assert!(shapes > 400, "{}", shapes);
    }

    /// K14 as M1 builds it today (the group body at cell 0 registers through the Kirby branch under
    /// the PinCell0 home's header and unloads through the other) hangs once the group is torn down
    /// before the home Kirby; U18 completes the same sequence.
    fn k14_hangs_today_and_completes_under_u18() {
        let (home, group) = (m(&[KIRBY, YOSHI, FOX]), m(&[CHROM, LUCINA, MARTH]));
        let today = run(Policy::Today, home, group, m(&[MARIO]), Some(m(&[FALCON])), Sequence::BuildReturnReload);
        assert!(today.reg.faults.iter().any(|f| matches!(f, Fault::SelfMirror { entry: 0, sub: 0, walker: CHROM, .. })),
            "{:?}", today.reg.faults);
        let u18 = run(Policy::U18, home, group, m(&[MARIO]), Some(m(&[FALCON])), Sequence::BuildReturnReload);
        assert!(u18.clean(), "{:?}", u18.reg.faults);
    }

    /// K15 as M1 builds it today: the Kirby group body registers through the other branch under the
    /// non-Kirby home's header (`int[0][0]` reads Yoshi while a Kirby body sits there) — silent with
    /// a non-Kirby CPU (it registers no copy relation at all), a hang with a Kirby CPU (its relation
    /// node self-mirrors at the Kirby walk).
    fn k15_today_misregisters_and_hangs_against_a_kirby_cpu() {
        let (home, group) = (m(&[YOSHI, FOX, FALCO]), m(&[KIRBY, CHROM, LUCINA]));
        let mut world = World::new(Policy::Today);
        world.reload(1, m(&[FALCON]));
        world.reload(0, home);
        world.build_group(0, group);
        assert_eq!((world.reg.kind(0, 0), world.reg.len(0, 0)), (YOSHI, 0));
        world.finish();
        assert!(world.reg.faults.is_empty());
        let today = run(Policy::Today, home, group, m(&[MARIO]), Some(m(&[KIRBY])), Sequence::BuildReturnReload);
        assert!(today.reg.faults.iter().any(|f| matches!(f, Fault::SelfMirror { entry: 0, sub: 0, walker: KIRBY, .. })),
            "{:?}", today.reg.faults);
        let mut u18 = World::new(Policy::U18);
        u18.reload(1, m(&[FALCON]));
        u18.reload(0, home);
        u18.build_group(0, group);
        assert_eq!(u18.reg.kind(0, 0), KIRBY);
        assert!(u18.reg.len(0, 0) > 0, "the Kirby group registers its copy relations");
        u18.finish();
        assert!(u18.clean());
    }

    /// Both roles banked, each with a Kirby somewhere, interleaving builds, warm swaps and Reloads:
    /// the cross-entry relations stay symmetric.
    fn both_roles_interleaved_stay_symmetric() {
        for cpu_home in [m(&[KIRBY, MARIO]), m(&[MARIO, 0x08]), m(&[KIRBY])] {
            let mut world = World::new(Policy::U18);
            world.reload(1, cpu_home);
            world.reload(0, m(&[KIRBY, YOSHI, FOX]));
            world.build_group(0, m(&[CHROM, LUCINA]));
            let cpu_group = if cpu_home.index_of(KIRBY).is_some() { m(&[FALCON, FALCO]) } else { m(&[KIRBY, FALCON]) };
            world.build_group(1, cpu_group);
            world.swap_to(0, KIRBY);
            world.swap_to(1, cpu_home.kinds()[0]);
            world.build_group(0, m(&[MARTH]));
            world.swap_to(1, cpu_group.kinds()[0]);
            world.swap_to(0, CHROM);
            world.reload(1, m(&[KIRBY, FALCON]));
            world.swap_to(0, KIRBY);
            world.reload(0, m(&[MARIO]));
            world.finish();
            assert!(world.clean(), "{:x?}: {:?} {:?}", cpu_home.kinds(), world.reg.faults, world.violations);
        }
    }

    /// The 17:3x correction alone (header and record follow the window) is not the root: a Kirby
    /// group under a non-Kirby home (K15), attached when the Reload comes, leaves its registrations in
    /// cell 0, and the detached home's cell-0 body unloads over them first. A CPU Kirby does the same
    /// to K14 through the other branch. U18 completes both.
    fn k15_header_only_still_self_mirrors() {
        let k15 = run(Policy::HeaderOnly, m(&[YOSHI, FOX, FALCO]), m(&[KIRBY, CHROM, LUCINA]), m(&[MARIO]),
            Some(m(&[FALCON])), Sequence::BuildReload);
        assert!(k15.reg.faults.iter().any(|f| matches!(f, Fault::SelfMirror { entry: 0, sub: 0, walker: YOSHI, .. })),
            "{:?}", k15.reg.faults);
        let k14 = run(Policy::HeaderOnly, m(&[KIRBY, YOSHI, FOX]), m(&[CHROM, LUCINA, MARTH]), m(&[MARIO]),
            Some(m(&[KIRBY])), Sequence::BuildReload);
        assert!(k14.reg.faults.iter().any(|f| matches!(f, Fault::SelfMirror { entry: 0, sub: 0, walker: KIRBY, .. })),
            "{:?}", k14.reg.faults);
        for (home, group, cpu) in [(m(&[YOSHI, FOX, FALCO]), m(&[KIRBY, CHROM, LUCINA]), m(&[FALCON])),
            (m(&[KIRBY, YOSHI, FOX]), m(&[CHROM, LUCINA, MARTH]), m(&[KIRBY]))] {
            assert!(run(Policy::U18, home, group, m(&[MARIO]), Some(cpu), Sequence::BuildReload).clean());
        }
    }

    /// The protocol's call order is the adapter's: a build unregisters the window before the header
    /// and roster change, an attach registers after them, a teardown re-registers the window last.
    fn the_protocol_orders_its_native_calls_like_a_reload() {
        #[derive(Default)]
        struct Calls(Vec<String>);
        impl RegistryOps for Calls {
            fn unregister(&mut self, m: Members) { self.0.push(format!("unregister {:x?}", &m.kinds()[..m.count()])); }
            fn write_header(&mut self, m: Members) { self.0.push(format!("header {:x?}", &m.kinds()[..m.count()])); }
            fn refresh_roster(&mut self) { self.0.push("roster".into()); }
            fn register(&mut self, m: Members) { self.0.push(format!("register {:x?}", &m.kinds()[..m.count()])); }
        }
        let destroy = |calls: &mut Calls, m: Members, b: &u8| calls.0.push(format!("destroy {:x?} {}", &m.kinds()[..m.count()], b));
        let (w, g) = (m(&[KIRBY, YOSHI]), m(&[CHROM]));
        let mut calls = Calls::default();
        build(&mut calls, w, g);
        attach(&mut calls, g, w);
        teardown(&mut calls, w, &[(g, 7u8)], destroy);
        teardown(&mut calls, w, &[], destroy);
        assert_eq!(calls.0, ["unregister [6, 5]", "header [1b]", "roster",
            "unregister [1b]", "header [6, 5]", "roster", "register [6, 5]",
            "unregister [6, 5]", "header [1b]", "destroy [1b] 7", "header [6, 5]", "register [6, 5]"]);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_transcription_reproduces_k_r1", the_transcription_reproduces_k_r1 as fn()),
            ("the_costume_is_init_s_selector_on_the_primary", the_costume_is_init_s_selector_on_the_primary as fn()),
            ("the_hazard_predicate_is_wp_k_s", the_hazard_predicate_is_wp_k_s as fn()),
            ("every_k14_k15_shape_registers_and_unregisters_symmetrically", every_k14_k15_shape_registers_and_unregisters_symmetrically as fn()),
            ("k14_hangs_today_and_completes_under_u18", k14_hangs_today_and_completes_under_u18 as fn()),
            ("k15_today_misregisters_and_hangs_against_a_kirby_cpu", k15_today_misregisters_and_hangs_against_a_kirby_cpu as fn()),
            ("both_roles_interleaved_stay_symmetric", both_roles_interleaved_stay_symmetric as fn()),
            ("k15_header_only_still_self_mirrors", k15_header_only_still_self_mirrors as fn()),
            ("the_protocol_orders_its_native_calls_like_a_reload", the_protocol_orders_its_native_calls_like_a_reload as fn()),
        ]);
    }
    }
}
