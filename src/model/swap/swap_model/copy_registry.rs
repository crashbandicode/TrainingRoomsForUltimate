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

