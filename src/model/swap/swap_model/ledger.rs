//! Per-R resource ledger: the native resource owner's registration and release key sets as
//! data, acquisition tokens with release recipes, resident dependencies and release footprints
//! (WORKPLAN §2.3; SYNTHESIS agree-3/-7; Astra §4.1–4.3; Fable §2.1).
//!
//! Nothing here touches native memory. `R` is the per-role resource owner (`G+0xe8+8*owner`),
//! its key map is the libc++ tree at `R+0x10` (node count `R+0x18`), its NRO tree at `R+0x4b8`
//! (count `R+0x4c0`). Every rule below cites the fact it encodes; addresses are `main+offset` in
//! the owned 13.0.5 ELF (`0789f0a4…`). "D-SG1" = D-SG1-RCA.md, "SOLO" = SOLO-STALL-RCA.md,
//! "C122" = C122-IC-FIX.md, "K1" = D-KIRBY1-ROB1-RCA.md, "17e64d0.c" =
//! `research/phase2-native/native_kind_primary_resource_request_17e64d0.c`, "17e5bc0.c" =
//! `…/native_group_resource_request_checked_17e5bc0.c`, "17e8610.asm" =
//! `D175/d-kirby1-rob1/re/release_kind_17e8610.asm`.
//!
//! Key space (FACT 17e64d0.c:146/650/666, :972, :1078, :1371; D-SG1 §2.1): `0x17e64d0(R, kind)`
//! registers map keys `kind*42 + n`:
//! - `n = 0` always (the primary file; a missing file, handle `0xffffff`, creates **no node**,
//!   `0x17e2df0` FACT D-SG1 §2.1 item 1);
//! - `n = 1` when `R+0x48 != 1` and the `0x65dd70` table bit is set for the kind (17e64d0.c:886–888,
//!   :972; the table: `sub w8,w0,#5; cmp w8,#0x70; b.hi → 0`, index `kind-5` at `0x44be510`,
//!   targets `0x65dd94` → 1 / `0x65dd98` → 0; dumped into `tests/fixtures/swap/keys-13.0.5.json`);
//! - `n = 6` when `R+0x48 != 1` and `R+0x7f == 0` (17e64d0.c:886, :1016 `cVar5 = R+0x7f`, :1078);
//! - `n = 7` only for Byleth `0x52` when `R+0x7d == 0` (17e64d0.c:1283, :1371 key `0xd7b`);
//! - `n = 2, 3, 4` for the Mii kinds `0x48..=0x4a` (D-SG1 §2.3, 17e8610.asm `17e87c4–17e87f0`);
//! - `n = 5` for `0x4e..=0x50` only when `R+0x48 == 2` (17e64d0.c:586; 17e8610.asm `17e87f8–17e8814`).
//!
//! `R+0x7d`/`R+0x7f` are written by loader state 6 from the entry descriptor
//! (`native_entry_load_650010.c:542–548, :611–613`: `R+0x7f = FE+0x54`, `R+0x7d = FE+0x52 ? 1 :
//! FE+0x53 ^ 1`); in Training every ordinary kind registers `+0`, `+6` and `+1` per the table
//! (Fox 3 keys, Daisy 2, Byleth 4 — D-SG1 §1 calibration), so `R+0x48 != 1`, `R+0x7f == 0`,
//! `R+0x7d == 0` there. Their QP/LDN values are an open fact (WORKPLAN §8.5): the fixtures carry
//! them as inputs and the runtime reconciliation (`src/swap_native/observe.rs`) reports any drift.
//!
//! Release (FACT 17e8610.asm): `0x17e8610(R, kind)` (1) cancels and frees a *pending* task
//! (`task+0x10 != 0`; an idle task is left alone, `17e8630–17e8634`), (2) when `R+0x4c` bit 2 is
//! set releases R's G68 mirror for `(kind, costume)` through `0x17f0290` (`17e8708–17e87ac`),
//! (3) releases keys: `kind == -1` → none; `0x75` → `0x1332, 0x1333` only (`17e8770–17e8780`);
//! `0x72` → `0x12b4` then the generic loop; generic → `+0`, Mii `+2,+3,+4`, `0x4e..=0x50` `+5`
//! when `R+0x48 == 2`, then `+1`, `+6`, `+7` (`17e87b0–17e8838`); (4) under the `R+0x28` lock, an
//! empty map tail-calls the reset `R->vtable+0x20 = 0x17e45f0` (`17e883c–17e8880`), which clears
//! the map and `R+0x4c..0x53` (`17e46c4–17e46e4`, C122 §1). The Aegis orphan is therefore
//! `registration − release = {+6} = 0x1338` (D-SG1 §0), computed here, never hard-coded.
//!
//! Native window requests (FACT 17e5bc0.c): state 6's `0x17e5bc0` registers the primary `R+0x58`
//! through `0x17e64d0`; by primary kind (`kind-0x24` jump table): PT `0x24..=0x26` registers the
//! trainer object `0x12b4` (+0 of `0x72`) with its NRO (`:184–185, :201, :213`), then `R+0x5c` and
//! `R+0x60` (`:226–227`); Popo `0x4b` requests `R+0x5c` unless `-1` (`:231–236`; SOLO §2); Nana
//! `0x4c` requests `0x4b` (`:237–239`); Aegis `0x5a/0x5b` registers `0x1332`, the NRO `0x75` and
//! `0x1333` (`:329, :358, :360…`) then the partner `R+0x5c`. The bank's hook supplements the
//! remaining members once each (`body_bank::supplemental_acquisitions`, SOLO §3).
//!
//! Construction tail (FACT C122 §1.2, `0x60de60–0x60dec4`): cells 0/1 `[0x4b, 0x4c]` → decrement
//! `R+0x50` under the lock and reset the whole R when the value before was ≤ 1; every other layout
//! → `0x17e8610(R, body.kind)` once per body. State 6 writes `R+0x50 = 1`, then `2` for a Popo
//! primary with a secondary (`650010.c:615–618`).

use std::collections::BTreeMap;

pub const KEY_STRIDE: u32 = 42;
pub const KIRBY: u32 = 0x06;
pub const POPO: u32 = 0x4b;
pub const NANA: u32 = 0x4c;
pub const BYLETH: u32 = 0x52;
pub const PT_OBJECT_KIND: u32 = 0x72;
pub const AEGIS_OBJECT_KIND: u32 = 0x75;
/// `kind == -1` in a descriptor cell.
pub const NO_KIND: u32 = u32::MAX;

/// One map key: `kind*42 + n`, `n` in `0..8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId {
    pub kind: u32,
    pub n: u8,
}
impl KeyId {
    pub const fn value(self) -> u32 { self.kind * KEY_STRIDE + self.n as u32 }
    pub const fn from_value(value: u32) -> Self {
        KeyId { kind: value / KEY_STRIDE, n: (value % KEY_STRIDE) as u8 }
    }
}

/// Bit `n` ⇒ key `kind*42 + n` (`n` in `0..8`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeySet(pub u8);
impl KeySet {
    pub const EMPTY: KeySet = KeySet(0);
    pub const fn of(bits: &[u8]) -> Self {
        let mut mask = 0u8;
        let mut i = 0;
        while i < bits.len() {
            mask |= 1 << bits[i];
            i += 1;
        }
        KeySet(mask)
    }
    pub const fn contains(self, n: u8) -> bool { self.0 & (1 << n) != 0 }
    pub const fn with(self, n: u8) -> Self { KeySet(self.0 | (1 << n)) }
    pub const fn minus(self, other: KeySet) -> KeySet { KeySet(self.0 & !other.0) }
    pub const fn union(self, other: KeySet) -> KeySet { KeySet(self.0 | other.0) }
    pub const fn is_empty(self) -> bool { self.0 == 0 }
    pub const fn len(self) -> usize { self.0.count_ones() as usize }
    pub fn iter(self, kind: u32) -> impl Iterator<Item = KeyId> + Clone {
        (0u8..8).filter(move |n| self.contains(*n)).map(move |n| KeyId { kind, n })
    }
}

/// Resource-owner bytes the key rules depend on: `R+0x48`, `R+0x7d`, `R+0x7f` (module doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RFlags {
    pub r48: u8,
    pub r7d: u8,
    pub r7f: u8,
}
impl RFlags {
    /// Training (FACT by the D-SG1 §1 key-count calibration: `+0`, `+6`, table `+1`, Byleth `+7`).
    pub const TRAINING: RFlags = RFlags { r48: 0, r7d: 0, r7f: 0 };
}
impl Default for RFlags {
    fn default() -> Self { RFlags::TRAINING }
}

/// Kinds whose `0x65dd70` table entry targets `0x65dd94` (returns 1): the `+1` key is registered
/// for them when `R+0x48 != 1`. Transcribed from the ELF dump
/// `tests/fixtures/swap/keys-13.0.5.json` (index `kind-5` at `0x44be510`, 0x71 entries; kinds
/// below 5 or above `0x75` return 0 by the bounds check). The dump is checked against this table
/// by `table_bit1_matches_the_elf_dump`.
pub const TABLE_BIT1_KINDS: [u32; 30] = [
    0x05, 0x07, 0x0b, 0x14, 0x21, 0x2a, 0x2f, 0x31, 0x39, 0x3b, 0x3f, 0x40, 0x42, 0x43, 0x44, 0x45,
    0x47, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x75,
];
pub fn table_bit1(kind: u32) -> bool { TABLE_BIT1_KINDS.contains(&kind) }

/// A condition on the owner's flag bytes (and the kind) under which a key is registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// `0x65dd70(kind) & 1` (17e64d0.c:887–888).
    TableBit1,
    /// `R+0x48 != 1` (17e64d0.c:886).
    R48NotOne,
    /// `R+0x48 == 2` (17e64d0.c:586; 17e8610.asm `17e8804–17e880c`).
    R48IsTwo,
    /// `R+0x7f == 0` (17e64d0.c:83, :1016).
    R7fZero,
    /// `R+0x7d == 0` (17e64d0.c:1283).
    R7dZero,
}
impl Cond {
    pub fn holds(self, kind: u32, flags: RFlags) -> bool {
        match self {
            Cond::TableBit1 => table_bit1(kind),
            Cond::R48NotOne => flags.r48 != 1,
            Cond::R48IsTwo => flags.r48 == 2,
            Cond::R7fZero => flags.r7f == 0,
            Cond::R7dZero => flags.r7d == 0,
        }
    }
}

/// Key `kind*42 + n` is registered when every condition in `when` holds (and its file exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRule {
    pub n: u8,
    pub when: &'static [Cond],
}

const RULE_0: KeyRule = KeyRule { n: 0, when: &[] };
const RULE_1: KeyRule = KeyRule { n: 1, when: &[Cond::R48NotOne, Cond::TableBit1] };
const RULE_6: KeyRule = KeyRule { n: 6, when: &[Cond::R48NotOne, Cond::R7fZero] };
const RULES_ORDINARY: &[KeyRule] = &[RULE_0, RULE_1, RULE_6];
const RULES_MII: &[KeyRule] = &[
    RULE_0,
    KeyRule { n: 2, when: &[] },
    KeyRule { n: 3, when: &[] },
    KeyRule { n: 4, when: &[] },
    RULE_1,
    RULE_6,
];
const RULES_4E_50: &[KeyRule] = &[RULE_0, KeyRule { n: 5, when: &[Cond::R48IsTwo] }, RULE_1, RULE_6];
const RULES_BYLETH: &[KeyRule] = &[RULE_0, RULE_1, RULE_6, KeyRule { n: 7, when: &[Cond::R7dZero] }];

/// What `0x17e64d0(R, kind)` registers for `kind` (module doc). `-1` registers nothing. Nana is an
/// ordinary kind: `17e64d0.c` has no Nana branch (its kind conditions are `:84` Mii, `:586`
/// `0x4e..=0x50`, `:894`/`:896` bounds/`0x72`, `:1124`/`:1283` Byleth). Native requests it for every
/// pair window through `R+0x5c` (`17e5bc0.c:230–235`), and the bank's pair-group prefetch requests it
/// directly. Runtime FACT (Yoga `icp4d` #149–#153, #183): after prefetching `[4b, 4c]`, R held Nana's
/// `0xc78`/`0xc7e` (`+0`, `+6`; `0x4c` has no table bit). Earlier this arm returned nothing, so
/// reconcile reported those keys as residual.
pub fn registration_rules(kind: u32) -> &'static [KeyRule] {
    match kind {
        NO_KIND => &[],
        0x48..=0x4a => RULES_MII,
        0x4e..=0x50 => RULES_4E_50,
        BYLETH => RULES_BYLETH,
        _ => RULES_ORDINARY,
    }
}

/// Keys `0x17e64d0(R, kind)` registers under `flags` (every file present).
pub fn registration_set(kind: u32, flags: RFlags) -> KeySet {
    registration_rules(kind).iter()
        .filter(|rule| rule.when.iter().all(|cond| cond.holds(kind, flags)))
        .fold(KeySet::EMPTY, |set, rule| set.with(rule.n))
}

/// Keys `0x17e8610(R, kind)` releases (module doc; the `0x72` branch releases `+0` twice).
pub fn native_release_set(kind: u32, flags: RFlags) -> KeySet {
    match kind {
        NO_KIND => KeySet::EMPTY,
        AEGIS_OBJECT_KIND => KeySet::of(&[0, 1]),
        0x48..=0x4a => KeySet::of(&[0, 2, 3, 4, 1, 6, 7]),
        0x4e..=0x50 if flags.r48 == 2 => KeySet::of(&[0, 5, 1, 6, 7]),
        _ => KeySet::of(&[0, 1, 6, 7]),
    }
}

/// Keys the owned per-kind release must take itself (`0x17e2fe0(R, key)`, before `0x17e8610`)
/// so that `acquire ∘ release` is the identity: `registration − native release`. Aegis `0x75` in
/// Training → `{+6}` = `0x1338` (D-SG1 §0); every other kind → empty.
pub fn orphan_keys(kind: u32, flags: RFlags) -> KeySet {
    registration_set(kind, flags).minus(native_release_set(kind, flags))
}

/// One registration native's own state-6 request makes for a window descriptor (module doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeRequest {
    pub kind: u32,
    pub keys: KeySet,
    pub nro: bool,
}

/// Model of `0x17e5bc0` for the entry cells `[R+0x58, R+0x5c, R+0x60]` (module doc). Object
/// kinds get exactly the keys native registers for them (`0x72` → `+0`; `0x75` → `+0, +1`),
/// not the generic set the bank's prefetch would.
pub fn native_window_requests(cells: [u32; 3], flags: RFlags) -> Vec<NativeRequest> {
    native_window_requests_fixed(cells, flags).iter().copied().collect()
}

/// Three descriptor cells and at most one special-object request.
pub fn native_window_requests_fixed(cells: [u32; 3], flags: RFlags)
    -> crate::engine::compat::fixed::FixedList<NativeRequest, 4> {
    let mut out = crate::engine::compat::fixed::FixedList::new(
        NativeRequest { kind: NO_KIND, keys: KeySet::EMPTY, nro: false });
    let primary = cells[0];
    if primary == NO_KIND { return out; }
    let member = |kind: u32| NativeRequest { kind, keys: registration_set(kind, flags), nro: true };
    out.push(member(primary));
    match primary {
        0x24..=0x26 => {
            out.push(NativeRequest { kind: PT_OBJECT_KIND, keys: KeySet::of(&[0]), nro: true });
            for kind in [cells[1], cells[2]] {
                if kind != NO_KIND { out.push(member(kind)); }
            }
        }
        POPO => {
            // INF (WORKPLAN §8.6): the Popo table entry tail-requests `R+0x5c` through `0x17e64d0`,
            // so a pair's Nana is modelled as an ordinary registration of `0x4c`.
            if cells[1] != NO_KIND { out.push(member(cells[1])); }
        }
        NANA => { out.push(member(POPO)); }
        0x5a | 0x5b => {
            out.push(NativeRequest { kind: AEGIS_OBJECT_KIND, keys: KeySet::of(&[0, 1]), nro: true });
            if cells[1] != NO_KIND { out.push(member(cells[1])); }
        }
        _ => {}
    }
    out
}

/// Pair-rule reference count state 6 leaves in `R+0x50` (`650010.c:615–618`).
pub fn native_pair_refs(cells: [u32; 3]) -> u32 {
    if cells[0] == POPO && cells[1] != NO_KIND { 2 } else { 1 }
}

/// Whether a body's construction tail takes the pair rule (C122 §1.2: cells 0/1 exactly
/// `[Popo, Nana]`).
pub fn pair_rule(cells: [u32; 3]) -> bool { cells[0] == POPO && cells[1] == NANA }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u8);
/// One acquisition batch: a prefetch or build of one group, or one native window request set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatchId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    /// Native requested it (state 6 / `0x17e5bc0`); recorded, never issued by us.
    Native,
    Prefetch(GroupId),
    Build(GroupId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseRecipe {
    /// A body of this kind constructs in this R; its tail releases the kind (`0x60dec4`).
    ConstructionTail,
    /// Nobody constructs it: owned `0x17e2fe0` per orphan key, then `0x17e8610(R, kind)` (D-SG1).
    OwnedPerKind,
    /// Cells 0/1 are `[Popo, Nana]`: the tails decrement `R+0x50`; the ledger owns the count (C122).
    PairReset { references: u8 },
}

/// Native calls a recipe resolves to, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCall {
    Acquire { kind: u32 },
    ReleaseKey(KeyId),
    ReleaseKind { kind: u32 },
    WritePairRefs(u8),
    SetRequestBit,
}

/// At most eight bits in KeySet, followed by the one per-kind native release.
pub type ReleaseCalls = crate::engine::compat::fixed::FixedList<NativeCall, 9>;

/// An acquisition the ledger tracks. Not `Clone`/`Copy`; dropping it is a leak the owner must
/// report through [`Ledger::retain_dropped`] (no native work in `Drop`, WORKPLAN §5.4).
#[derive(Debug, PartialEq, Eq)]
#[must_use = "an acquisition dropped without release leaks a map node; return it to the ledger"]
pub struct AcqToken {
    kind: u32,
    keys: KeySet,
    holder: Holder,
    recipe: ReleaseRecipe,
    batch: BatchId,
    id: u32,
}
impl AcqToken {
    pub fn kind(&self) -> u32 { self.kind }
    pub fn keys(&self) -> KeySet { self.keys }
    pub fn holder(&self) -> Holder { self.holder }
    pub fn recipe(&self) -> ReleaseRecipe { self.recipe }
    pub fn batch(&self) -> BatchId { self.batch }
    pub fn id(&self) -> u32 { self.id }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}
impl Role {
    pub const fn index(self) -> usize {
        match self { Role::Player => 0, Role::Cpu => 1 }
    }
    pub const fn other(self) -> Role {
        match self { Role::Player => Role::Cpu, Role::Cpu => Role::Player }
    }
    pub const fn from_index(index: usize) -> Option<Role> {
        match index { 0 => Some(Role::Player), 1 => Some(Role::Cpu), _ => None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Acquire,
    Release,
    Build,
    TargetKind,
}

/// Live-consumer conflicts (WP4 `occupancy.rs` supplies the rules; the grades are the RCAs').
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conflict {
    /// FACT K1 §3: a live Kirby copy of this kind shares R's G68 mirror with the key's release.
    LiveCopySharesRegistry,
    /// FACT K1 §0/§5.1 (Astra §4.3): bank prefetch debt may not coexist with a live Kirby on R.
    PrefetchDebtUnderLiveKirby,
    /// HYP K1 §6 K2: a hat-source kind built into the incoming window while a copy exists.
    HatSourceLive,
    /// Disproved for the mirror shape (D-HANG1-SEPARATION §1: S-A 3/3 completed); kept for WP4's table.
    OtherRoleLive,
}

/// What the ledger asks before issuing a native acquisition or release. WP4's `KindOccupancy`
/// implements it; [`NoConsumers`] is the observation-only answer.
pub trait LiveConsumers {
    fn conflict(&self, role: Role, kind: u32, usage: Use) -> Option<Conflict>;
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoConsumers;
impl LiveConsumers for NoConsumers {
    fn conflict(&self, _role: Role, _kind: u32, _usage: Use) -> Option<Conflict> { None }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerRefusal {
    /// SOLO §2: a construction acquisition of this kind already exists on this R.
    AlreadyHeld { kind: u32, by: Holder },
    Conflict(Conflict),
    /// Astra §4.2: another batch still holds temporary obligations on this R.
    ResetNotExclusive(Vec<BatchId>),
    UnknownToken,
}

/// Kinds as a 128-bit set (kinds run to `0x7f`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KindSet64(pub [u64; 2]);
impl KindSet64 {
    pub fn insert(&mut self, kind: u32) {
        if kind < 128 { self.0[(kind / 64) as usize] |= 1 << (kind % 64); }
    }
    pub fn remove(&mut self, kind: u32) {
        if kind < 128 { self.0[(kind / 64) as usize] &= !(1 << (kind % 64)); }
    }
    pub fn contains(self, kind: u32) -> bool {
        kind < 128 && self.0[(kind / 64) as usize] & (1 << (kind % 64)) != 0
    }
    pub fn is_empty(self) -> bool { self.0 == [0, 0] }
    pub fn iter(self) -> impl Iterator<Item = u32> {
        (0u32..128).filter(move |kind| self.contains(*kind))
    }
}

/// Built or parked bodies depend on R's NRO tree after their keys are gone (Astra §4.1;
/// GROUP-SWAP-IMPL §14.3: parked bodies hold no NRO reference of their own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResidentDependencies {
    pub kinds: KindSet64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindCostume {
    pub kind: u32,
    pub costume: u32,
}

/// Everything `0x17e8610(R, kind)` touches besides the key owner's own keys (K1 §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseFootprint {
    pub keys: KeySet,
    pub kind_registry: Vec<KindCostume>,
    pub may_cancel_task: bool,
    pub may_reset_r: bool,
}

/// Owner state the footprint depends on, read at the call seam.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnerState {
    /// `task+0x10 != 0` for the task at `R+0xa8`.
    pub task_pending: bool,
    /// `R+0x4c` bit 2 (the worker set it when the loads completed).
    pub loads_completed: bool,
    /// `R+0x64` after the R.O.B./Mii selector (K1 §4.1).
    pub costume: u32,
    /// Nodes in `R+0x18` right now.
    pub map_nodes: usize,
}

/// Footprint of one `0x17e8610(R, kind)` given the keys this release will actually remove.
pub fn release_kind_footprint(kind: u32, flags: RFlags, state: OwnerState, removed_nodes: usize) -> ReleaseFootprint {
    ReleaseFootprint {
        keys: native_release_set(kind, flags),
        kind_registry: if state.loads_completed && kind != NO_KIND {
            vec![KindCostume { kind, costume: state.costume }]
        } else {
            Vec::new()
        },
        may_cancel_task: state.task_pending,
        may_reset_r: state.map_nodes <= removed_nodes,
    }
}

/// Proof that every outstanding temporary obligation on R belongs to one batch (Astra §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetBatchPermit {
    batch: BatchId,
}
impl ResetBatchPermit {
    pub fn batch(self) -> BatchId { self.batch }
}

/// Result of comparing the ledger's prediction with the owner's actual `(key, refs)` pairs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reconciliation {
    /// Present on R, not predicted (or more refs than predicted): `(key, extra refs)`.
    pub residual: Vec<(u32, u32)>,
    /// Predicted, absent on R (or fewer refs): `(key, missing refs)`.
    pub missing: Vec<(u32, u32)>,
    pub matched: usize,
}

/// BTreeMap collection semantics without storage: sorted keys, final duplicate wins.
fn ordered_actual(input: impl Iterator<Item = (u32, u32)> + Clone)
    -> impl Iterator<Item = (u32, u32)> + Clone {
    let mut previous = None;
    std::iter::from_fn(move || {
        let mut next = None;
        for (key, refs) in input.clone() {
            if previous.is_none_or(|old| key > old) && next.is_none_or(|(best, _)| key <= best) {
                next = Some((key, refs));
            }
        }
        if let Some((key, _)) = next { previous = Some(key); }
        next
    })
}
impl Reconciliation {
    pub fn balanced(&self) -> bool { self.residual.is_empty() && self.missing.is_empty() }
}

/// A retained obligation: a token its owner could not release (conflict, lost identity, drop).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retained {
    pub kind: u32,
    pub keys: KeySet,
    pub holder: Holder,
    pub reason: &'static str,
}

/// The per-R ledger (WORKPLAN §2.3 interface).
#[derive(Debug)]
pub struct Ledger {
    flags: RFlags,
    acquisitions: Vec<AcqToken>,
    residents: Vec<(BatchId, ResidentDependencies)>,
    retained: Vec<Retained>,
    next_id: u32,
    next_batch: u32,
    /// Mirror of `R+0x50` while the pair rule applies.
    pub pair_refs: Option<u8>,
    /// Mirror of `R+0x4c` bit 0.
    pub request_bit: bool,
}
impl Ledger {
    pub fn new(flags: RFlags) -> Self {
        Ledger { flags, acquisitions: Vec::new(), residents: Vec::new(), retained: Vec::new(), next_id: 0, next_batch: 0, pair_refs: None, request_bit: false }
    }
    pub fn flags(&self) -> RFlags { self.flags }
    pub fn set_flags(&mut self, flags: RFlags) { self.flags = flags; }
    pub fn new_batch(&mut self) -> BatchId {
        self.next_batch += 1;
        BatchId(self.next_batch)
    }
    pub fn tokens(&self) -> &[AcqToken] { &self.acquisitions }
    pub fn retained(&self) -> &[Retained] { &self.retained }

    fn mint(&mut self, kind: u32, keys: KeySet, holder: Holder, recipe: ReleaseRecipe, batch: BatchId) -> AcqToken {
        self.next_id += 1;
        let token = AcqToken { kind, keys, holder, recipe, batch, id: self.next_id };
        self.acquisitions.push(AcqToken { kind, keys, holder, recipe, batch, id: self.next_id });
        token
    }

    /// Observation of a request native made itself (`0x17e5bc0`): recorded with the keys native
    /// registers for it; never refused, never issued.
    pub fn record_native(&mut self, request: NativeRequest, batch: BatchId) -> AcqToken {
        let recipe = if request.kind == PT_OBJECT_KIND || request.kind == AEGIS_OBJECT_KIND {
            ReleaseRecipe::OwnedPerKind
        } else {
            ReleaseRecipe::ConstructionTail
        };
        self.mint(request.kind, request.keys, Holder::Native, recipe, batch)
    }

    /// A mod acquisition. Refuses before any native call when the live consumers object, or when a
    /// construction acquisition of the kind already exists on this R (SOLO: the Popo primary's
    /// secondary is native's, not ours). Returns the token and the call to issue (`None` for a
    /// `Holder::Native` record).
    pub fn acquire(
        &mut self,
        kind: u32,
        holder: Holder,
        recipe: ReleaseRecipe,
        batch: BatchId,
        occupancy: &dyn LiveConsumers,
        role: Role,
    ) -> Result<(AcqToken, Option<NativeCall>), LedgerRefusal> {
        if let Some(conflict) = occupancy.conflict(role, kind, Use::Acquire) {
            return Err(LedgerRefusal::Conflict(conflict));
        }
        if recipe != ReleaseRecipe::OwnedPerKind {
            if let Some(held) = self.acquisitions.iter()
                .find(|a| a.kind == kind && a.recipe != ReleaseRecipe::OwnedPerKind)
            {
                return Err(LedgerRefusal::AlreadyHeld { kind, by: held.holder });
            }
        }
        let keys = registration_set(kind, self.flags);
        let token = self.mint(kind, keys, holder, recipe, batch);
        let call = (holder != Holder::Native).then_some(NativeCall::Acquire { kind });
        Ok((token, call))
    }

    /// A body of `kind` finished construction in `batch`: its tail released the kind (or took the
    /// pair rule). Removes the matching construction token; returns whether one existed.
    pub fn construction_settled(&mut self, kind: u32, batch: BatchId) -> bool {
        let position = self.acquisitions.iter().position(|a| a.kind == kind && a.batch == batch
            && a.recipe != ReleaseRecipe::OwnedPerKind)
            .or_else(|| self.acquisitions.iter().position(|a| a.kind == kind && a.recipe != ReleaseRecipe::OwnedPerKind));
        let Some(i) = position else { return false; };
        let token = self.acquisitions.remove(i);
        if let ReleaseRecipe::PairReset { .. } = token.recipe {
            if let Some(refs) = self.pair_refs.as_mut() { *refs = refs.saturating_sub(1); }
        }
        true
    }

    /// Native already freed every key of `token`, as the owner's state proves (`R+0x18 == 0`, the
    /// map empty after a native tail and the reset it triggered). The token is discharged with no
    /// native call, so the ledger keeps mirroring native (IC-PAIR-STALL: a Phase B group's PT
    /// object token outlived the build because its release was skipped on the empty map, and the
    /// pair rule's reset-batch permit then refused on that stale batch). Returns whether it existed.
    pub fn settle(&mut self, token: AcqToken) -> bool {
        let Some(i) = self.acquisitions.iter().position(|a| a.id == token.id) else { return false; };
        let settled = self.acquisitions.remove(i);
        if let ReleaseRecipe::PairReset { .. } = settled.recipe {
            if let Some(refs) = self.pair_refs.as_mut() { *refs = refs.saturating_sub(1); }
        }
        true
    }

    /// Native released a registration it made itself (the partner weapon's / trainer's own
    /// `0x17e8610(R, object)`, D-SG1 §2.5; a window body's tail): the recorded `Holder::Native`
    /// token of `kind` is discharged. Returns whether one existed.
    pub fn native_consumed(&mut self, kind: u32) -> bool {
        let Some(i) = self.acquisitions.iter().position(|a| a.kind == kind && a.holder == Holder::Native) else { return false; };
        let _ = self.acquisitions.remove(i);
        true
    }

    /// The native calls that balance `token`, in order: per-key `0x17e2fe0` for every orphan,
    /// then `0x17e8610(R, kind)` (D-SG1 order: the orphans first so the reset tail still fires).
    /// Refused — token handed back — when the live consumers object (K1).
    pub fn release(
        &mut self,
        token: AcqToken,
        occupancy: &dyn LiveConsumers,
        role: Role,
    ) -> Result<Vec<NativeCall>, (AcqToken, LedgerRefusal)> {
        self.release_fixed(token, occupancy, role).map(|calls| calls.to_vec())
    }

    pub fn release_fixed(
        &mut self,
        token: AcqToken,
        occupancy: &dyn LiveConsumers,
        role: Role,
    ) -> Result<ReleaseCalls, (AcqToken, LedgerRefusal)> {
        if let Some(conflict) = occupancy.conflict(role, token.kind, Use::Release) {
            return Err((token, LedgerRefusal::Conflict(conflict)));
        }
        let Some(i) = self.acquisitions.iter().position(|a| a.id == token.id) else {
            return Err((token, LedgerRefusal::UnknownToken));
        };
        let _ = self.acquisitions.remove(i);
        let mut calls = ReleaseCalls::new(NativeCall::ReleaseKind { kind: token.kind });
        for key in orphan_keys(token.kind, self.flags).iter(token.kind) {
            assert!(calls.push(NativeCall::ReleaseKey(key)));
        }
        assert!(calls.push(NativeCall::ReleaseKind { kind: token.kind }));
        Ok(calls)
    }

    /// A token its owner could not release becomes a logged obligation, never a destructor.
    pub fn retain_dropped(&mut self, token: AcqToken, reason: &'static str) {
        self.acquisitions.retain(|a| a.id != token.id);
        self.retained.push(Retained { kind: token.kind, keys: token.keys, holder: token.holder, reason });
    }

    /// Astra §4.2: a pair-rule build may reset the whole R only when every outstanding temporary
    /// obligation on R belongs to its own batch.
    pub fn reset_batch_permit(&self, batch: BatchId) -> Result<ResetBatchPermit, Vec<BatchId>> {
        let mut others: Vec<BatchId> = self.acquisitions.iter()
            .filter(|a| a.batch != batch && a.holder != Holder::Native)
            .map(|a| a.batch).collect();
        others.sort();
        others.dedup();
        if others.is_empty() { Ok(ResetBatchPermit { batch }) } else { Err(others) }
    }

    /// Pair-rule references a build must leave in `R+0x50`: one per construction under the rule.
    pub fn pair_references(&self, batch: BatchId) -> u8 {
        self.acquisitions.iter()
            .filter(|a| a.batch == batch && matches!(a.recipe, ReleaseRecipe::PairReset { .. }))
            .count() as u8
    }

    pub fn add_resident(&mut self, batch: BatchId, dependencies: ResidentDependencies) {
        self.residents.push((batch, dependencies));
    }
    pub fn remove_residents(&mut self, batch: BatchId) {
        self.residents.retain(|(b, _)| *b != batch);
    }
    /// Kinds some resident body still depends on (a release or reset must not take their NRO/registry).
    pub fn resident_kinds(&self) -> KindSet64 {
        let mut set = KindSet64::default();
        for (_, deps) in &self.residents {
            for kind in deps.kinds.iter() { set.insert(kind); }
        }
        set
    }

    /// `(key value, refs)` the model predicts in `R+0x10` right now, sorted by key.
    pub fn predicted(&self) -> Vec<(u32, u32)> {
        self.predicted_iter().collect()
    }
    pub fn predicted_iter(&self) -> impl Iterator<Item = (u32, u32)> + Clone + '_ {
        let mut previous = None;
        std::iter::from_fn(move || {
            let keys = || self.acquisitions.iter().flat_map(|a| a.keys.iter(a.kind).map(KeyId::value));
            let next = keys().filter(|key| previous.is_none_or(|old| *key > old)).min()?;
            let refs = keys().filter(|key| *key == next).fold(0u32, |n, _| n + 1);
            previous = Some(next);
            Some((next, refs))
        })
    }
    pub fn predicted_map_nodes(&self) -> usize { self.predicted_iter().count() }

    /// Nothing held by us or recorded for native: a canonical Reload may be issued (state 6 needs
    /// `R+0x18 == 0`, `0x17e4700`).
    pub fn expect_free(&self) -> Result<(), Vec<KeyId>> {
        let keys: Vec<KeyId> = self.predicted().into_iter().map(|(value, _)| KeyId::from_value(value)).collect();
        if keys.is_empty() { Ok(()) } else { Err(keys) }
    }

    /// Compare the prediction with the owner's actual `(key value, refs)` pairs.
    pub fn reconcile(&self, actual: &[(u32, u32)]) -> Reconciliation {
        Reconciliation { residual: self.residual_iter(actual.iter().copied()).collect(),
            missing: self.missing_iter(actual.iter().copied()).collect(),
            matched: ordered_actual(actual.iter().copied()).filter(|pair| self.predicted_iter().any(|p| p == *pair)).count() }
    }

    pub fn residual_iter<'a>(&'a self, actual: impl Iterator<Item = (u32, u32)> + Clone + 'a)
        -> impl Iterator<Item = (u32, u32)> + Clone + 'a {
        ordered_actual(actual).filter_map(move |(key, refs)| {
            match self.predicted_iter().find(|(k, _)| *k == key) {
                Some((_, p)) if p < refs => Some((key, refs - p)),
                None => Some((key, refs)),
                _ => None,
            }
        })
    }

    pub fn missing_iter<'a>(&'a self, actual: impl Iterator<Item = (u32, u32)> + Clone + 'a)
        -> impl Iterator<Item = (u32, u32)> + Clone + 'a {
        // Preserve the original two-pass order: short actual keys, then absent keys.
        ordered_actual(actual.clone()).filter_map(move |(key, refs)| {
            self.predicted_iter().find(|(k, _)| *k == key)
                .and_then(|(_, p)| (p > refs).then(|| (key, p - refs)))
        }).chain(self.predicted_iter().filter(move |(key, _)| !actual.clone().any(|(k, _)| k == *key)))
    }

    /// WP2 shadow mode: a ledger mirroring what the current bank code holds — one `OwnedPerKind`
    /// prefetch token per `(group, kind)` of every group that `holds_keys()`. Built groups hold
    /// nothing (their tails ran); the window's native requests settled at construction.
    pub fn mirror(flags: RFlags, prefetched: &[(GroupId, u32)]) -> Ledger {
        let mut ledger = Ledger::new(flags);
        let mut batches: BTreeMap<GroupId, BatchId> = BTreeMap::new();
        for (group, kind) in prefetched {
            let batch = match batches.get(group) {
                Some(b) => *b,
                None => { let b = ledger.new_batch(); batches.insert(*group, b); b }
            };
            let keys = registration_set(*kind, flags);
            let _ = ledger.mint(*kind, keys, Holder::Prefetch(*group), ReleaseRecipe::OwnedPerKind, batch);
        }
        // `mint` returned a second handle for the caller; the mirror keeps only the ledger's copy.
        ledger
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: RFlags = RFlags::TRAINING;

    #[test]
    fn the_training_registration_sets_match_the_d_sg1_calibration() {
        // D-SG1 §1: Fox 3 keys, Daisy 2, Byleth 4 with +7; Aegis 0x75 {0,1,6}; PT 0x72 {0,6}.
        assert_eq!(registration_set(0x07, T), KeySet::of(&[0, 1, 6]));
        assert_eq!(registration_set(0x0e, T), KeySet::of(&[0, 6]));
        assert_eq!(registration_set(BYLETH, T), KeySet::of(&[0, 1, 6, 7]));
        assert_eq!(registration_set(AEGIS_OBJECT_KIND, T), KeySet::of(&[0, 1, 6]));
        assert_eq!(registration_set(PT_OBJECT_KIND, T), KeySet::of(&[0, 6]));
        assert_eq!(registration_set(0x00, T), KeySet::of(&[0, 6]), "Mario: kind < 5 has no table entry");
        assert_eq!(registration_set(0x49, T), KeySet::of(&[0, 2, 3, 4, 6]));
        assert_eq!(registration_set(NANA, T), KeySet::of(&[0, 6]), "Nana is ordinary (icp4d: 0xc78, 0xc7e)");
        assert_eq!(registration_set(NO_KIND, T), KeySet::EMPTY);
        // Flags: R+0x48 == 1 drops +1 and +6; R+0x7f != 0 drops +6; R+0x7d != 0 drops Byleth's +7.
        assert_eq!(registration_set(0x07, RFlags { r48: 1, ..T }), KeySet::of(&[0]));
        assert_eq!(registration_set(0x07, RFlags { r7f: 1, ..T }), KeySet::of(&[0, 1]));
        assert_eq!(registration_set(BYLETH, RFlags { r7d: 1, ..T }), KeySet::of(&[0, 1, 6]));
        assert_eq!(registration_set(0x4f, RFlags { r48: 2, ..T }), KeySet::of(&[0, 5, 6]), "0x4f: +5 under R+0x48 == 2, table bit 0");
        assert_eq!(registration_set(0x4f, T), KeySet::of(&[0, 6]), "0x4f table bit is 0");
    }

    #[test]
    fn the_aegis_object_kind_has_exactly_one_orphan_key_and_nothing_else_does() {
        let orphans: Vec<u32> = orphan_keys(AEGIS_OBJECT_KIND, T).iter(AEGIS_OBJECT_KIND).map(KeyId::value).collect();
        assert_eq!(orphans, vec![0x1338]);
        assert_eq!(crate::engine::swap::body_bank::orphan_object_keys(AEGIS_OBJECT_KIND).0[0], 0x1338, "the shipped point fix and the computed set agree");
        for kind in (0u32..0x80).filter(|k| *k != AEGIS_OBJECT_KIND) {
            for r48 in 0..3u8 { for r7d in 0..2u8 { for r7f in 0..2u8 {
                let flags = RFlags { r48, r7d, r7f };
                assert!(orphan_keys(kind, flags).is_empty(), "kind {kind:#x} flags {flags:?} orphan {:?}", orphan_keys(kind, flags));
            } } }
        }
        // Outside Training the +6 key is not registered, so even Aegis has no orphan.
        assert!(orphan_keys(AEGIS_OBJECT_KIND, RFlags { r7f: 1, ..T }).is_empty());
    }

    #[test]
    fn native_window_requests_follow_the_17e5bc0_table() {
        let ordinary = native_window_requests([0x07, 0x08, 0x09], T);
        assert_eq!(ordinary.len(), 1, "an ordinary primary requests only R+0x58; the hook owns the rest");
        let solo = native_window_requests([POPO, 0x03, 0x07], T);
        assert_eq!(solo.iter().map(|r| r.kind).collect::<Vec<_>>(), vec![POPO, 0x03], "SOLO §2: Link is native's");
        let pt = native_window_requests([0x24, 0x25, 0x26], T);
        assert_eq!(pt.iter().map(|r| r.kind).collect::<Vec<_>>(), vec![0x24, PT_OBJECT_KIND, 0x25, 0x26]);
        assert_eq!(pt[1].keys, KeySet::of(&[0]), "native registers only 0x12b4 for the trainer");
        let aegis = native_window_requests([0x5a, 0x5b, NO_KIND], T);
        assert_eq!(aegis.iter().map(|r| r.kind).collect::<Vec<_>>(), vec![0x5a, AEGIS_OBJECT_KIND, 0x5b]);
        assert_eq!(aegis[1].keys, KeySet::of(&[0, 1]), "native never registers 0x1338");
        assert_eq!(native_pair_refs([POPO, NANA, 0x02]), 2);
        assert_eq!(native_pair_refs([POPO, NO_KIND, NO_KIND]), 1);
        assert!(pair_rule([POPO, NANA, 0x02]) && !pair_rule([POPO, 0x03, 0x07]));
    }

    #[test]
    fn a_kind_native_already_requested_for_a_construction_is_not_acquired_twice() {
        let mut ledger = Ledger::new(T);
        let batch = ledger.new_batch();
        let mut tokens = Vec::new();
        for request in native_window_requests([POPO, 0x03, 0x07], T) {
            tokens.push(ledger.record_native(request, batch));
        }
        let refused = ledger.acquire(0x03, Holder::Build(GroupId(0)), ReleaseRecipe::ConstructionTail, batch, &NoConsumers, Role::Player);
        assert_eq!(refused.err(), Some(LedgerRefusal::AlreadyHeld { kind: 0x03, by: Holder::Native }));
        let (fox, call) = ledger.acquire(0x07, Holder::Build(GroupId(0)), ReleaseRecipe::ConstructionTail, batch, &NoConsumers, Role::Player).unwrap();
        assert_eq!(call, Some(NativeCall::Acquire { kind: 0x07 }));
        tokens.push(fox);
        assert_eq!(ledger.predicted_map_nodes(), 7, "Popo {{0,6}}, Link {{0,6}}, Fox {{0,1,6}}: one node each, Link once");
        for kind in [POPO, 0x03, 0x07] { assert!(ledger.construction_settled(kind, batch)); }
        assert_eq!(ledger.expect_free(), Ok(()));
        drop(tokens);
    }

    #[test]
    fn release_after_acquire_is_the_identity_and_names_the_orphan_first() {
        for kind in 0u32..0x80 {
            for r48 in 0..3u8 { for r7d in 0..2u8 { for r7f in 0..2u8 {
                let flags = RFlags { r48, r7d, r7f };
                let mut ledger = Ledger::new(flags);
                let batch = ledger.new_batch();
                let (token, call) = ledger.acquire(kind, Holder::Prefetch(GroupId(0)), ReleaseRecipe::OwnedPerKind, batch, &NoConsumers, Role::Player).unwrap();
                assert_eq!(call, Some(NativeCall::Acquire { kind }));
                let calls = ledger.release(token, &NoConsumers, Role::Player).unwrap();
                let releases: Vec<u32> = calls.iter().filter_map(|c| match c { NativeCall::ReleaseKey(k) => Some(k.value()), _ => None }).collect();
                let expected: Vec<u32> = orphan_keys(kind, flags).iter(kind).map(KeyId::value).collect();
                assert_eq!(releases, expected, "kind {:#x} flags {:?}", kind, flags);
                assert_eq!(calls.last(), Some(&NativeCall::ReleaseKind { kind }));
                assert_eq!(ledger.expect_free(), Ok(()));
            } } }
        }
    }

    struct Copying(u32);
    impl LiveConsumers for Copying {
        fn conflict(&self, _role: Role, kind: u32, usage: Use) -> Option<Conflict> {
            (kind == self.0 && matches!(usage, Use::Acquire | Use::Release)).then_some(Conflict::LiveCopySharesRegistry)
        }
    }

    #[test]
    fn a_release_the_live_consumers_refuse_hands_the_token_back_and_can_be_retained() {
        let mut ledger = Ledger::new(T);
        let batch = ledger.new_batch();
        let (token, _) = ledger.acquire(0x00, Holder::Prefetch(GroupId(0)), ReleaseRecipe::OwnedPerKind, batch, &NoConsumers, Role::Player).unwrap();
        let (kept, why) = ledger.release(token, &Copying(0x00), Role::Player).unwrap_err();
        assert_eq!(why, LedgerRefusal::Conflict(Conflict::LiveCopySharesRegistry));
        assert_eq!(ledger.predicted_map_nodes(), 2, "kept keys stay predicted");
        assert_eq!(ledger.acquire(0x00, Holder::Prefetch(GroupId(1)), ReleaseRecipe::OwnedPerKind, batch, &Copying(0x00), Role::Player).err(),
            Some(LedgerRefusal::Conflict(Conflict::LiveCopySharesRegistry)));
        ledger.retain_dropped(kept, "live Kirby copies this kind");
        assert_eq!(ledger.retained().len(), 1);
        assert_eq!(ledger.expect_free(), Ok(()), "a retained obligation is reported, not predicted");
    }

    #[test]
    fn a_pair_reset_is_permitted_only_when_no_other_batch_holds_keys() {
        let mut ledger = Ledger::new(T);
        let pair = ledger.new_batch();
        let other = ledger.new_batch();
        let mut held = Vec::new();
        for kind in [POPO, NANA, 0x02] {
            let (t, _) = ledger.acquire(kind, Holder::Build(GroupId(0)), ReleaseRecipe::PairReset { references: 3 }, pair, &NoConsumers, Role::Player).unwrap();
            held.push(t);
        }
        assert_eq!(ledger.pair_references(pair), 3, "C122: one reference per body under the rule");
        assert!(ledger.reset_batch_permit(pair).is_ok());
        let (prefetch, _) = ledger.acquire(0x07, Holder::Prefetch(GroupId(1)), ReleaseRecipe::OwnedPerKind, other, &NoConsumers, Role::Player).unwrap();
        assert_eq!(ledger.reset_batch_permit(pair), Err(vec![other]));
        let _ = ledger.release(prefetch, &NoConsumers, Role::Player).unwrap();
        assert!(ledger.reset_batch_permit(pair).is_ok());
        drop(held);
    }

    #[test]
    fn reconciliation_names_residual_and_missing_keys_by_refs() {
        let ledger = Ledger::mirror(T, &[(GroupId(1), 0x24), (GroupId(1), 0x25), (GroupId(1), 0x26), (GroupId(1), PT_OBJECT_KIND)]);
        assert_eq!(ledger.predicted_map_nodes(), 8, "D-SG1 §1: the PT set is eight keys");
        // The B123-pre lap after the Aegis build: the PT keys plus the orphan 0x1338 (map=9).
        let mut actual: Vec<(u32, u32)> = ledger.predicted();
        actual.push((0x1338, 1));
        let r = ledger.reconcile(&actual);
        assert_eq!(r.residual, vec![(0x1338, 1)]);
        assert!(r.missing.is_empty());
        assert_eq!(r.matched, 8);
        // A missing file: one predicted node absent.
        let fewer: Vec<(u32, u32)> = ledger.predicted().into_iter().filter(|(k, _)| *k != 0x24 * 42 + 6).collect();
        let r = ledger.reconcile(&fewer);
        assert_eq!(r.missing, vec![(0x24 * 42 + 6, 1)]);
        assert!(ledger.reconcile(&ledger.predicted()).balanced());
    }

    #[test]
    fn kind_sets_cover_the_whole_kind_space() {
        let mut set = KindSet64::default();
        set.insert(0x00); set.insert(0x3f); set.insert(0x40); set.insert(0x7f); set.insert(200);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![0x00, 0x3f, 0x40, 0x7f]);
        set.remove(0x40);
        assert!(!set.contains(0x40) && set.contains(0x7f));
        let footprint = release_kind_footprint(0x00, T, OwnerState { task_pending: true, loads_completed: true, costume: 0, map_nodes: 2 }, 2);
        assert_eq!(footprint.kind_registry, vec![KindCostume { kind: 0x00, costume: 0 }]);
        assert!(footprint.may_cancel_task && footprint.may_reset_r);
    }
}
