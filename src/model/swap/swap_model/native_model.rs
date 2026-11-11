//! Host simulator of exactly the native resource/loader semantics the RCAs established, and the
//! replay harness that runs the recorded traces through a swap *policy* (WORKPLAN §1.4/§2.3;
//! SYNTHESIS agree-7; Astra §10 independence rule).
//!
//! Independence: nothing in this module calls the ledger's rule tables. The registration set
//! (`keys_registered_by_17e64d0`), the release set (`keys_released_by_17e8610`), the state-6
//! requests (`requests_of_17e5bc0`), the construction tail, the free predicate and the reset are
//! transcribed here a second time, as control flow following the listings, and the `0x65dd70`
//! table comes from the committed ELF dump (`tests/fixtures/swap/keys-13.0.5.json`), not from
//! `ledger::TABLE_BIT1_KINDS`. The test `the_two_transcriptions_agree_for_every_kind_and_flag_set`
//! is the check that the ledger's data matches this model; a disagreement is a transcription
//! error in one of them, never a reason to make one call the other.
//!
//! Hazards are modelled at the grade the evidence has (FACT / INF) and named in `Fault`:
//! - `LoaderRefused7`: state 6 picks a free R with `R+0x4c0 == 0 && R+0x18 == 0` (`0x17e4700`,
//!   FACT SOLO §1, D-SG1 §2.6); state 3 cleared the NRO tree, never the key map
//!   (`650010.c:332–340`); a refusal sets `E+0x591c = 7` and publishes nothing.
//! - `State11Stall`: a pair-rule tail resets the whole R when the pre-decrement `R+0x50 <= 1`
//!   (`0x60dea0–0x60dea8`, FACT C122 §1.2); a body whose construction starts after that reset
//!   never completes (C122 §1.4, HYP strong).
//! - `Spin { MirrorReleaseUnderLiveCopy }`: `0x17e8610(R, k)` with `R+0x4c` bit 2 releases R's G68
//!   mirror for `(k, costume)`; a live Kirby copy of `k` on the same R shares that registry state,
//!   and the next keyed list walk meets a null `next` (K1 §3.2–3.4, INF strong; FACT for the
//!   release path and the spin signature).
//! - `Spin { KirbyCopyListAtNonZeroCell }`: unloading a Kirby body built at cell 1/2 of a window or
//!   group walks a copy-tuple list indexed as if Kirby were cell 0 and removes a node twice
//!   (`unprepare 0x17f26c0` kind-6 branch `0x17f2784–0x17f2884`; `wp-k/RE-KIRBY-UNLOAD.md`, FACT at
//!   code level, Eden K-R1 pending); D-HANG1-SEPARATION §1: S-B (Kirby cell 1) 3/3 hung, S-K (cell 0)
//!   completed; activation plays no part.
//! - `DanglingRoute`: the context destructor frees the controller `ctx+0x58` but never writes the
//!   pad route `A->+0` (`0x6b7920`, FACT D-PTA1BC-FIX §2.5); the per-frame tick `0x2f7b80`
//!   dereferences it (`Read64 @ 0x0`, `Read8 @ 0x38`) until activation re-points the slot.
//! - `StrandedInstall`: a prequeue refusal after the Reload began branches to the epilogue
//!   `0x651464`, skips the state-6 tail and suppresses every later tick, so the loader never
//!   leaves 6 and INSTALL is owned until destruction (D-PTA1-RCA §3, FACT).
//!
//! Fixture format (`tests/fixtures/swap/README.md`): one JSON object per line, `"ev"` tagged,
//! `"note"` free text (the log line it transcribes). The parser `tools/swap_trace.py` (WP0) emits
//! the same vocabulary from Eden/diag logs.

use super::ledger::{
    AcqToken, BatchId, GroupId, Holder, Ledger, LedgerRefusal, NativeCall, NoConsumers, RFlags, ReleaseRecipe,
    Role, AEGIS_OBJECT_KIND, BYLETH, KIRBY, NANA, NO_KIND, POPO, PT_OBJECT_KIND,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ROLES: usize = 2;
pub const CELLS: usize = 3;
/// `(key value, refs)` pairs as the ledger predicts and the owner's map holds them.
pub type KeyRefs = Vec<(u32, u32)>;
/// `pool=…/24` in every B123-pre line.
pub const POOL_CAPACITY: u32 = 24;

// ---------------------------------------------------------------------------------------------
// Native tables, transcribed independently of `ledger.rs`
// ---------------------------------------------------------------------------------------------

/// The `0x65dd70` per-kind table as dumped from the ELF (`keys-13.0.5.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindTable {
    bit1: BTreeSet<u32>,
}
impl KindTable {
    /// Parse `wpB-keys-dump.py` output: `entries[].kind`, `entries[].bit1` (1, 0 or "other").
    pub fn from_elf_dump_json(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let entries = value.get("entries").and_then(|e| e.as_array()).ok_or("no entries")?;
        let mut bit1 = BTreeSet::new();
        for entry in entries {
            let kind = entry.get("kind").and_then(|k| k.as_u64()).ok_or("entry without kind")? as u32;
            match entry.get("bit1") {
                Some(serde_json::Value::Number(n)) if n.as_u64() == Some(1) => { bit1.insert(kind); }
                Some(serde_json::Value::Number(n)) if n.as_u64() == Some(0) => {}
                other => return Err(format!("kind {kind:#x}: unclassified table target {other:?}")),
            }
        }
        Ok(KindTable { bit1 })
    }
    /// `0x65dd70(kind) & 1`: `sub w8,w0,#5; cmp w8,#0x70; b.hi → 0` bounds the table to `5..=0x75`.
    pub fn bit1(&self, kind: u32) -> bool { (5..=0x75).contains(&kind) && self.bit1.contains(&kind) }
    pub fn kinds_with_bit1(&self) -> impl Iterator<Item = u32> + '_ { self.bit1.iter().copied() }
}

/// `0x17e64d0(R, kind)` as control flow (17e64d0.c:84, :586, :886–888, :972, :1016, :1078, :1283,
/// :1371): the key values it registers, in program order.
pub fn keys_registered_by_17e64d0(kind: u32, r48: u8, r7d: u8, r7f: u8, table: &KindTable) -> Vec<u32> {
    let mut keys = Vec::new();
    // `-1` only: the listing has no Nana branch, so Nana registers like any ordinary kind (Yoga
    // `icp4d`: `0xc78`/`0xc7e` on R after a `[4b, 4c]` prefetch).
    if kind == NO_KIND { return keys; }
    let base = kind * 42;
    if (0x48..=0x4a).contains(&kind) {
        keys.push(base);
        keys.extend([base + 2, base + 3, base + 4]);
    } else if (0x4e..=0x50).contains(&kind) && r48 == 2 {
        keys.push(base);
        keys.push(base + 5);
    } else {
        keys.push(base);
    }
    if r48 != 1 {
        if table.bit1(kind) { keys.push(base | 1); }
        if r7f == 0 { keys.push(base + 6); }
    }
    if kind == BYLETH && r7d == 0 { keys.push(0xd7b); }
    keys
}

/// `0x17e8610(R, kind)` key releases as control flow (17e8610.asm `17e86e4–17e8838`), in program
/// order; duplicates are what the code does (`0x72` releases `0x12b4` twice).
pub fn keys_released_by_17e8610(kind: u32, r48: u8) -> Vec<u32> {
    if kind == NO_KIND { return Vec::new(); }
    if kind == AEGIS_OBJECT_KIND { return vec![0x1332, 0x1333]; }
    let mut keys = Vec::new();
    if kind == PT_OBJECT_KIND { keys.push(0x12b4); }
    let base = kind * 42;
    keys.push(base);
    if (0x48..=0x4a).contains(&kind) {
        keys.extend([base + 2, base + 3, base + 4]);
    } else if (0x4e..=0x50).contains(&kind) && r48 == 2 {
        keys.push(base + 5);
    }
    keys.extend([base | 1, base + 6, base + 7]);
    keys
}

/// A registration native makes in state 6 (`0x17e5bc0`): a full `0x17e64d0` request for a kind,
/// or the object-kind keys it registers directly (17e5bc0.c:184–213 PT, :329–360 Aegis).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeRegistration {
    Kind(u32),
    Keys { kind: u32, keys: [u32; 2], count: usize },
}

/// `0x17e5bc0` for the entry cells (17e5bc0.c:226–239 and the PT/Aegis arms).
pub fn requests_of_17e5bc0(cells: [u32; 3]) -> Vec<NativeRegistration> {
    let mut out = Vec::new();
    let primary = cells[0];
    if primary == NO_KIND { return out; }
    out.push(NativeRegistration::Kind(primary));
    match primary {
        0x24..=0x26 => {
            out.push(NativeRegistration::Keys { kind: PT_OBJECT_KIND, keys: [0x12b4, 0], count: 1 });
            if cells[1] != NO_KIND { out.push(NativeRegistration::Kind(cells[1])); }
            if cells[2] != NO_KIND { out.push(NativeRegistration::Kind(cells[2])); }
        }
        POPO => { if cells[1] != NO_KIND { out.push(NativeRegistration::Kind(cells[1])); } }
        NANA => out.push(NativeRegistration::Kind(POPO)),
        0x5a | 0x5b => {
            out.push(NativeRegistration::Keys { kind: AEGIS_OBJECT_KIND, keys: [0x1332, 0x1333], count: 2 });
            if cells[1] != NO_KIND { out.push(NativeRegistration::Kind(cells[1])); }
        }
        _ => {}
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The model
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum G68Hazard {
    /// K1 §3.2–3.4 (INF strong): `0x17e8610(R, k)` released R's G68 mirror while a live Kirby copy of `k` shared it.
    MirrorReleaseUnderLiveCopy,
    /// `wp-k/RE-KIRBY-UNLOAD.md` (FACT at code level, K-R1 pending): a Kirby body at cell ≠ 0 is unloaded
    /// through a copy-tuple list indexed as if it were cell 0.
    KirbyCopyListAtNonZeroCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    LoaderRefused7 { role: usize, residual: Vec<u32> },
    State11Stall { role: usize },
    Spin { role: usize, hazard: G68Hazard },
    DanglingRoute { role: usize, slot: usize },
    NullStore { role: usize },
    PoolExhausted { role: usize, live: u32 },
    StrandedInstall { role: usize, loader: u8 },
    /// A fixture `expect` line disagreed with the model (calibration against the logged numbers).
    Expectation { at: usize, field: &'static str, expected: u64, actual: u64 },
    /// A fixture event the model could not apply (wrong state); the replay is not meaningful.
    Malformed { at: usize, reason: String },
}

/// One resource owner `R` (`G+0xe8+8*role`).
#[derive(Clone, Debug, Default)]
pub struct RModel {
    /// `R+0x10` tree: key value → refs (`node+0x28`); `R+0x18` = `map.len()`.
    pub map: BTreeMap<u32, u32>,
    /// `R+0x4b8` NRO tree: kinds; `R+0x4c0` = `tree.len()`.
    pub tree: BTreeSet<u32>,
    /// `R+0x4c`: bit 0 request bit (state 6), bit 2 loads completed (the worker).
    pub flags4c: u8,
    /// `R+0x50`.
    pub pair_refs: u32,
    /// `R+0xa8` task with `task+0x10 != 0`.
    pub task_pending: bool,
    /// `R+0x54..0x68`.
    pub header: [u32; 5],
    pub r48: u8,
    pub r7d: u8,
    pub r7f: u8,
    /// G68 mirror records of this R (`R+0xb0..`): `(kind, costume)`.
    pub mirror: BTreeSet<(u32, u32)>,
    /// Key values whose file is absent: `0x17e2df0` inserts no node for the `0xffffff` handle.
    pub missing_keys: BTreeSet<u32>,
}
impl RModel {
    fn register_key(&mut self, key: u32) {
        if self.missing_keys.contains(&key) { return; }
        *self.map.entry(key).or_insert(0) += 1;
    }
    /// `0x17e64d0(R, kind)`: keys (`0x17e2df0` insert-or-bump) then the NRO (`0x17e4900`).
    pub fn acquire(&mut self, kind: u32, table: &KindTable) {
        for key in keys_registered_by_17e64d0(kind, self.r48, self.r7d, self.r7f, table) { self.register_key(key); }
        // `0x17e4900(R, kind)` ends every request (`0x17e7440`), Nana's included: the B119 decode
        // (`native-stall1.json`, C122 §1.3) shows `nro_count 3` for `[4b, 4c, 2]`.
        if kind != NO_KIND { self.tree.insert(kind); }
    }
    /// A direct object-kind registration of state 6 (`0x17e2df0` + `0x17e4900`).
    pub fn register(&mut self, kind: u32, keys: &[u32]) {
        for key in keys { self.register_key(*key); }
        self.tree.insert(kind);
    }
    /// `0x17e2fe0(R, key)`: a missing key is a no-op; `node+0x28 -= 1`, remove at zero.
    pub fn release_key(&mut self, key: u32) {
        if let Some(refs) = self.map.get_mut(&key) {
            *refs -= 1;
            if *refs == 0 { self.map.remove(&key); }
        }
    }
    /// `0x17e8610(R, kind)`: task cancel, G68 mirror release (bit 2), keys, reset when empty.
    /// `live_copy` is the kind a live Kirby copies on this R, if any (the K1 hazard input).
    pub fn release_kind(&mut self, kind: u32, live_copy: Option<u32>, g68: &mut G68) {
        if kind != NO_KIND {
            if self.task_pending { self.task_pending = false; }
            if self.flags4c & 4 != 0 {
                self.mirror.remove(&(kind, self.header[4]));
                if live_copy == Some(kind) { g68.poisoned = Some(G68Hazard::MirrorReleaseUnderLiveCopy); }
            }
            for key in keys_released_by_17e8610(kind, self.r48) { self.release_key(key); }
        }
        if self.map.is_empty() { self.reset(); }
    }
    /// `0x17e45f0`: task destroyed, map cleared, `R+0x20` ready byte, `R+0x4c..0x53` zeroed.
    pub fn reset(&mut self) {
        self.task_pending = false;
        self.map.clear();
        self.flags4c = 0;
        self.pair_refs = 0;
    }
    /// Loader state 3 (`650010.c:336–339`, `0x17e48a0`): the NRO tree, never the key map.
    pub fn clear_tree(&mut self) { self.tree.clear(); }
    /// `0x17e4700`: free when `R+0x4c0 == 0 && R+0x18 == 0`.
    pub fn free(&self) -> bool { self.tree.is_empty() && self.map.is_empty() }
    pub fn map_nodes(&self) -> usize { self.map.len() }
    pub fn snapshot(&self) -> Vec<(u32, u32)> { self.map.iter().map(|(k, r)| (*k, *r)).collect() }
}

/// The game's per-kind/costume resource registry `*(G+0x68)`: only the hazards the catalogue
/// needs (Astra §10: "hazard assertions, not an invented full G68 emulator").
#[derive(Clone, Debug, Default)]
pub struct G68 {
    /// `(kind, costume)` → contexts registered in the record's ring.
    pub records: BTreeMap<(u32, u32), BTreeSet<u32>>,
    pub poisoned: Option<G68Hazard>,
}
impl G68 {
    fn prepare_tuple(&mut self, kind: u32, costume: u32, ctx: u32) {
        self.records.entry((kind, costume)).or_default().insert(ctx);
    }
    fn release_tuple(&mut self, kind: u32, costume: u32, ctx: u32) {
        if let Some(ring) = self.records.get_mut(&(kind, costume)) {
            ring.remove(&ctx);
            if ring.is_empty() { self.records.remove(&(kind, costume)); }
        }
    }
    /// Any keyed list walk (prepare/release/unprepare) after a poisoning spins (K1 §3.1).
    fn walk(&self) -> Option<G68Hazard> { self.poisoned }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub kind: u32,
    pub id: u32,
    pub ctx: u32,
    /// The pad-manager controller object `ctx+0x58`.
    pub controller: u32,
    pub activated: bool,
    /// The window/group cell the body was constructed into (`sub`).
    pub cell: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    Planned,
    Prefetched,
    Built,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub state: GroupState,
    /// Kinds this record currently holds: the group's own while Planned/Prefetched; after a Phase B
    /// exchange the previous window's (the detached bundle) while Built.
    pub kinds: Vec<u32>,
    pub object_kind: Option<u32>,
    /// Detached bodies (Built).
    pub bodies: Vec<Body>,
    pub batch: BatchId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// `br 0x651464`: loader stuck at 6, INSTALL owned, no publication (D-PTA1b).
    Epilogue,
    /// D-PTA1BC-FIX §4: CONTINUE; native installs the staged descriptor and settles; authority dropped.
    NativeContinue,
}

#[derive(Debug)]
pub struct RoleState {
    pub r: RModel,
    pub window: [Option<Body>; CELLS],
    pub cells: [u32; CELLS],
    pub count: usize,
    pub actors: u32,
    pub active: usize,
    /// Recorded cell → planned cell (the fixture's `active`/`activate` indices are the recorded
    /// layout's; the planner may lay the window out differently, `CellRule::PinCell0`).
    pub cell_map: [usize; CELLS],
    pub groups: BTreeMap<u8, Group>,
    /// `E+0x591c`: 14 ready; 6 published/refused; 7 refused unpublished.
    pub loader: u8,
    pub published: bool,
    pub refused: Option<Refusal>,
    pub install_held: bool,
    /// A prequeue refusal armed for the next Reload (`PublishRejected`).
    pub pending_refusal: bool,
    /// `FLAG_COPY`/`INT_COPY_CHARA` of the live Kirby.
    pub kirby_copy: Option<u32>,
    /// The policy's view (shadow ledger), kept in step with the mod's own calls.
    pub ledger: Ledger,
    pub tokens: Vec<AcqToken>,
    /// The construction batch of the current window load / Phase B.
    pub batch: BatchId,
    pub stalled: bool,
}
impl RoleState {
    fn new() -> Self {
        RoleState { r: RModel::default(), window: [None; CELLS], cells: [NO_KIND; CELLS], count: 0, actors: 1, active: 0,
            cell_map: [0, 1, 2], groups: BTreeMap::new(), loader: 14, published: true, refused: None, install_held: false, pending_refusal: false,
            kirby_copy: None, ledger: Ledger::new(RFlags::TRAINING), tokens: Vec::new(), batch: BatchId(0), stalled: false }
    }
    /// Drop the shadow copy of a token the ledger has already discharged.
    fn forget_token(&mut self, kind: u32, native_only: bool) {
        if let Some(i) = self.tokens.iter().position(|t| t.kind() == kind && t.recipe() != ReleaseRecipe::OwnedPerKind
            && (!native_only || t.holder() == Holder::Native)) {
            let _ = self.tokens.remove(i);
        } else if let Some(i) = self.tokens.iter().position(|t| t.kind() == kind && t.holder() == Holder::Native) {
            let _ = self.tokens.remove(i);
        }
    }
    fn live_kind(&self) -> Option<u32> { self.window[self.active].map(|b| b.kind) }
    fn flags(&self) -> RFlags { RFlags { r48: self.r.r48, r7d: self.r.r7d, r7f: self.r.r7f } }
}

/// The mod's decisions the catalogue bugs turned on. `OldPolicy` = B123-pre; `RecipePolicy` =
/// the C116.1 baseline point fixes plus the 7a Kirby rule.
pub trait Policy {
    fn name(&self) -> &'static str;
    /// Window cells the planner builds for a requested window (7a: no never-activated Kirby parked).
    fn plan_window(&self, requested: [u32; CELLS]) -> [u32; CELLS];
    /// The resource hook's supplemental acquisitions for a bank descriptor (SOLO §3).
    fn supplemental(&self, cells: [u32; CELLS], count: usize) -> Vec<u32>;
    /// `R+0x50` the hook writes for a pair-lead window; `None` keeps native's value (C122).
    fn pair_refs(&self, cells: [u32; CELLS], count: usize) -> Option<u32>;
    /// Calls that release a prefetched-only kind (D-SG1: orphans first).
    fn release_prefetched(&self, kind: u32, flags: RFlags) -> Vec<NativeCall>;
    /// Prefetch admitted while `live` is the live member (D-KIRBY1).
    fn prefetch_admitted(&self, live: Option<u32>) -> bool;
    /// Prefetched keys are returned when Kirby becomes live (D-KIRBY1).
    fn returns_keys_on_live_kirby(&self) -> bool;
    /// Key release admitted given the live copy kind (D-KIRBY1 guard).
    fn key_release_admitted(&self, kind: u32, copy: Option<u32>) -> bool;
    /// Pad routes cleared before contexts are freed / before a Reload (D-PTA1c).
    fn clears_routes(&self) -> bool;
    /// What a prequeue refusal after the Reload began does (D-PTA1b).
    fn on_prequeue_refusal(&self, plain_kind: bool) -> Refusal;
}

/// B123-pre as recorded in the fixtures.
pub struct OldPolicy;
impl Policy for OldPolicy {
    fn name(&self) -> &'static str { "old" }
    fn plan_window(&self, requested: [u32; CELLS]) -> [u32; CELLS] { requested }
    fn supplemental(&self, cells: [u32; CELLS], count: usize) -> Vec<u32> {
        cells.iter().take(count).skip(1).copied().filter(|k| *k != NO_KIND).collect()
    }
    fn pair_refs(&self, _cells: [u32; CELLS], _count: usize) -> Option<u32> { None }
    fn release_prefetched(&self, kind: u32, _flags: RFlags) -> Vec<NativeCall> { vec![NativeCall::ReleaseKind { kind }] }
    fn prefetch_admitted(&self, _live: Option<u32>) -> bool { true }
    fn returns_keys_on_live_kirby(&self) -> bool { false }
    fn key_release_admitted(&self, _kind: u32, _copy: Option<u32>) -> bool { true }
    fn clears_routes(&self) -> bool { false }
    fn on_prequeue_refusal(&self, _plain_kind: bool) -> Refusal { Refusal::Epilogue }
}

/// The baseline point fixes (FINAL-INT items 1–3, C122, SOLO) without 7a: Kirby may still be parked.
pub struct RecipePolicy;
impl Policy for RecipePolicy {
    fn name(&self) -> &'static str { "recipe" }
    fn plan_window(&self, requested: [u32; CELLS]) -> [u32; CELLS] { requested }
    fn supplemental(&self, cells: [u32; CELLS], count: usize) -> Vec<u32> { RecipeWith7aPolicy.supplemental(cells, count) }
    fn pair_refs(&self, cells: [u32; CELLS], count: usize) -> Option<u32> { RecipeWith7aPolicy.pair_refs(cells, count) }
    fn release_prefetched(&self, kind: u32, flags: RFlags) -> Vec<NativeCall> { RecipeWith7aPolicy.release_prefetched(kind, flags) }
    fn prefetch_admitted(&self, live: Option<u32>) -> bool { RecipeWith7aPolicy.prefetch_admitted(live) }
    fn returns_keys_on_live_kirby(&self) -> bool { true }
    fn key_release_admitted(&self, kind: u32, copy: Option<u32>) -> bool { RecipeWith7aPolicy.key_release_admitted(kind, copy) }
    fn clears_routes(&self) -> bool { true }
    fn on_prequeue_refusal(&self, plain_kind: bool) -> Refusal { RecipeWith7aPolicy.on_prequeue_refusal(plain_kind) }
}

/// The recipe plus 7a (WP1's first-only Kirby candidate; the cell-0-only rule is gone since row 4, WP-K): a Kirby is never built as
/// an upcoming member of a window (cells 1–2); it is only ever the target at cell 0.
pub struct RecipeWith7aPolicy;
impl Policy for RecipeWith7aPolicy {
    fn name(&self) -> &'static str { "recipe+7a" }
    fn plan_window(&self, requested: [u32; CELLS]) -> [u32; CELLS] {
        let mut out = requested;
        for cell in out.iter_mut().skip(1) { if *cell == KIRBY { *cell = NO_KIND; } }
        out
    }
    fn supplemental(&self, cells: [u32; CELLS], count: usize) -> Vec<u32> {
        let first_extra = if cells[0] == POPO { 2 } else { 1 };
        cells.iter().take(count).skip(first_extra).copied().filter(|k| *k != NO_KIND).collect()
    }
    fn pair_refs(&self, cells: [u32; CELLS], count: usize) -> Option<u32> {
        (cells[0] == POPO && cells[1] == NANA).then_some(count as u32)
    }
    fn release_prefetched(&self, kind: u32, flags: RFlags) -> Vec<NativeCall> {
        let mut calls: Vec<NativeCall> = super::ledger::orphan_keys(kind, flags).iter(kind).map(NativeCall::ReleaseKey).collect();
        calls.push(NativeCall::ReleaseKind { kind });
        calls
    }
    fn prefetch_admitted(&self, live: Option<u32>) -> bool { live != Some(KIRBY) }
    fn returns_keys_on_live_kirby(&self) -> bool { true }
    fn key_release_admitted(&self, kind: u32, copy: Option<u32>) -> bool { copy != Some(kind) }
    fn clears_routes(&self) -> bool { true }
    fn on_prequeue_refusal(&self, plain_kind: bool) -> Refusal {
        if plain_kind { Refusal::NativeContinue } else { Refusal::Epilogue }
    }
}

fn one() -> u32 { 1 }

/// One recorded event (`tests/fixtures/swap/README.md`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "ev", rename_all = "snake_case")]
pub enum Event {
    /// `R+0x48`, `R+0x7d`, `R+0x7f` of the role's owner.
    Flags { role: usize, #[serde(default)] r48: u8, #[serde(default)] r7d: u8, #[serde(default)] r7f: u8, #[serde(default)] note: String },
    /// Key values whose file is absent on this fixture's card (no node).
    MissingKeys { role: usize, keys: Vec<u32>, #[serde(default)] note: String },
    /// A completed window: bodies constructed (tails ran), `active` activated, the others parked.
    Window { role: usize, cells: Vec<i64>, #[serde(default = "one")] actors: u32, #[serde(default)] active: usize, #[serde(default)] note: String },
    /// `[GroupSwap] plan` + `prefetch`: the bank registers every prefetch kind (incl. the object kind).
    Prefetch { role: usize, group: u8, kinds: Vec<u32>, #[serde(default)] note: String },
    /// `[GroupSwap] build`: Phase B exchange of the group into the window.
    Build { role: usize, group: u8, #[serde(default)] note: String },
    /// `[GroupSwap] request … state=Built` warm attach of a Built group.
    Attach { role: usize, group: u8, #[serde(default)] note: String },
    /// In-window member switch (`[BodyBank] reuse`).
    Activate { role: usize, cell: usize, #[serde(default)] note: String },
    /// `[KirbyHats] confirmed`: the live Kirby copies `kind` (registers on its own R).
    CopyAcquired { role: usize, kind: u32, #[serde(default)] note: String },
    /// Arms a prequeue refusal for the role's next Reload (D-PTA1 / an induced protected-row change).
    PublishRejected { role: usize, #[serde(default)] note: String },
    /// `[MiiOrdinary*] submit` → teardown → states 0–6 of a mod Reload (stops before construction).
    Reload { role: usize, cells: Vec<i64>, #[serde(default = "one")] actors: u32, #[serde(default)] bank: bool, #[serde(default)] note: String },
    /// States 7–14: construction tails, activation, `ready`.
    Ready { role: usize, #[serde(default)] note: String },
    /// The pad thread's per-frame route tick.
    Tick { #[serde(default = "one")] frames: u32, #[serde(default)] note: String },
    /// Calibration against the logged numbers; checked when `policy` is absent or matches.
    Expect { role: usize, #[serde(default)] policy: Option<String>, #[serde(default)] map: Option<u64>, #[serde(default)] tree: Option<u64>,
        #[serde(default)] loader: Option<u8>, #[serde(default)] install: Option<bool>, #[serde(default)] pair_refs: Option<u64>, #[serde(default)] note: String },
    /// Scene exit / entry destruction: everything destroyed, INSTALL released.
    Finalize { role: usize, #[serde(default)] note: String },
}

fn cells_of(list: &[i64]) -> Result<([u32; CELLS], usize), String> {
    if list.is_empty() || list.len() > CELLS { return Err(format!("cells must hold 1..=3 entries, got {}", list.len())); }
    let mut cells = [NO_KIND; CELLS];
    let mut count = 0;
    for (i, k) in list.iter().enumerate() {
        cells[i] = if *k < 0 { NO_KIND } else { *k as u32 };
        if cells[i] != NO_KIND { count = i + 1; }
    }
    Ok((cells, count))
}

/// Parse one JSONL fixture.
pub fn parse_fixture(text: &str) -> Result<Vec<Event>, String> {
    text.lines().enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|(i, line)| serde_json::from_str::<Event>(line).map_err(|e| format!("line {}: {e}: {line}", i + 1)))
        .collect()
}

/// The whole scene: two roles, the shared registry, the pad manager's route table and the pool.
pub struct World<'p> {
    pub roles: Vec<RoleState>,
    pub g68: G68,
    /// `S->+0x98[slot]->+0`: controller the pad tick dereferences each frame; slot = role.
    pub routes: [Option<u32>; 16],
    pub freed_controllers: BTreeSet<u32>,
    pub pool_live: u32,
    pub faults: Vec<Fault>,
    pub table: KindTable,
    policy: &'p dyn Policy,
    next_id: u32,
    step: usize,
}

impl<'p> World<'p> {
    pub fn new(policy: &'p dyn Policy, table: KindTable) -> Self {
        World { roles: vec![RoleState::new(), RoleState::new()], g68: G68::default(), routes: [None; 16],
            freed_controllers: BTreeSet::new(), pool_live: 0, faults: Vec::new(), table, policy, next_id: 0x1000, step: 0 }
    }
    fn fresh(&mut self) -> u32 { self.next_id += 1; self.next_id }
    fn fault(&mut self, fault: Fault) { self.faults.push(fault); }
    fn malformed(&mut self, reason: String) { let at = self.step; self.fault(Fault::Malformed { at, reason }); }

    fn ledger_role(role: usize) -> Role { Role::from_index(role).unwrap_or(Role::Player) }

    /// Apply the policy's native calls for a prefetched-only kind to R (the K1 hazard input is the
    /// live Kirby copy of this role).
    fn apply_calls(&mut self, role: usize, calls: &[NativeCall]) {
        let copy = self.roles[role].kirby_copy;
        for call in calls {
            match *call {
                NativeCall::ReleaseKey(key) => self.roles[role].r.release_key(key.value()),
                NativeCall::ReleaseKind { kind } => { let g68 = &mut self.g68; self.roles[role].r.release_kind(kind, copy, g68); }
                NativeCall::Acquire { kind } => { let table = &self.table; self.roles[role].r.acquire(kind, table); }
                NativeCall::WritePairRefs(refs) => self.roles[role].r.pair_refs = u32::from(refs),
                NativeCall::SetRequestBit => self.roles[role].r.flags4c |= 1,
            }
        }
    }

    /// Construct `kind` into `cell` of the window (body + context + controller + G68 tuple).
    fn construct(&mut self, role: usize, cell: usize, kind: u32) -> Body {
        let body = Body { kind, id: self.fresh(), ctx: self.fresh(), controller: self.fresh(), activated: false, cell };
        let costume = self.roles[role].r.header[4];
        self.g68.prepare_tuple(kind, costume, body.ctx);
        self.pool_live += 1;
        if self.pool_live > POOL_CAPACITY { let live = self.pool_live; self.fault(Fault::PoolExhausted { role, live }); }
        self.roles[role].window[cell] = Some(body);
        body
    }

    /// Destroy one body the way `0x3aeb50 → 0x610f20` / the Reload's state 1 do: unprepare its G68
    /// tuple (the kind-6 branch of `unprepare 0x17f26c0` walks the per-(entry, sub) copy-tuple list
    /// `G68+0x1d258+entry*0x48+sub*0x18`, filled at construction assuming Kirby is sub 0: a Kirby at
    /// cell 1/2 removes a node twice and the walk spins — `wp-k/RE-KIRBY-UNLOAD.md`, FACT at code level;
    /// S-B/D-KIRBY1 had Kirby at cell 1, S-K at cell 0 completed; Eden K-R1 pending), then the context
    /// destructor frees the controller without touching the pad route.
    fn destroy_body(&mut self, role: usize, body: Body, clear_route: bool) {
        if self.roles[role].stalled { return; }
        if body.kind == KIRBY && body.cell != 0 {
            self.g68.poisoned = Some(G68Hazard::KirbyCopyListAtNonZeroCell);
        }
        if let Some(hazard) = self.g68.walk() {
            self.fault(Fault::Spin { role, hazard });
            self.roles[role].stalled = true;
            return;
        }
        let costume = self.roles[role].r.header[4];
        self.g68.release_tuple(body.kind, costume, body.ctx);
        if clear_route {
            for slot in self.routes.iter_mut() { if *slot == Some(body.controller) { *slot = None; } }
        }
        self.freed_controllers.insert(body.controller);
        self.pool_live = self.pool_live.saturating_sub(1);
    }

    /// `retire_groups`: Built records are destroyed (bodies, contexts, records), key-holding records
    /// release their keys through the policy's recipe under the Kirby guard.
    fn retire_groups(&mut self, role: usize) {
        let groups: Vec<(u8, Group)> = self.roles[role].groups.iter().map(|(g, r)| (*g, r.clone())).collect();
        let clear = self.policy.clears_routes();
        for (_, group) in groups {
            match group.state {
                GroupState::Built => {
                    for body in group.bodies.clone() { self.destroy_body(role, body, clear); }
                }
                GroupState::Prefetched => {
                    let kinds: Vec<u32> = group.kinds.iter().copied().chain(group.object_kind).collect();
                    self.release_kinds(role, &kinds);
                }
                GroupState::Planned => {}
            }
        }
        self.roles[role].groups.clear();
    }

    fn release_kinds(&mut self, role: usize, kinds: &[u32]) {
        let flags = self.roles[role].flags();
        let copy = self.roles[role].kirby_copy;
        for kind in kinds {
            if !self.policy.key_release_admitted(*kind, copy) { continue; }
            let calls = self.policy.release_prefetched(*kind, flags);
            self.apply_calls(role, &calls);
            // The shadow ledger follows the mod's release.
            let position = self.roles[role].tokens.iter().position(|t| t.kind() == *kind && t.recipe() == ReleaseRecipe::OwnedPerKind);
            if let Some(i) = position {
                let token = self.roles[role].tokens.remove(i);
                let state = &mut self.roles[role];
                match state.ledger.release(token, &NoConsumers, Self::ledger_role(role)) {
                    Ok(_) => {}
                    Err((token, _)) => state.ledger.retain_dropped(token, "simulator release refused"),
                }
            }
        }
    }

    /// D-KIRBY1 new policy: a live Kirby returns every Prefetched group's keys (group → Planned).
    fn kirby_became_live(&mut self, role: usize) {
        if !self.policy.returns_keys_on_live_kirby() { return; }
        let prefetched: Vec<u8> = self.roles[role].groups.iter().filter(|(_, g)| g.state == GroupState::Prefetched).map(|(g, _)| *g).collect();
        for g in prefetched {
            let group = self.roles[role].groups.get(&g).cloned().unwrap();
            let kinds: Vec<u32> = group.kinds.iter().copied().chain(group.object_kind).collect();
            self.release_kinds(role, &kinds);
            if let Some(record) = self.roles[role].groups.get_mut(&g) { record.state = GroupState::Planned; }
        }
    }

    fn check_expect(&mut self, field: &'static str, expected: Option<u64>, actual: u64) {
        if let Some(expected) = expected {
            if expected != actual { let at = self.step; self.fault(Fault::Expectation { at, field, expected, actual }); }
        }
    }

    pub fn apply(&mut self, event: &Event) {
        self.step += 1;
        match event {
            Event::Flags { role, r48, r7d, r7f, .. } => {
                let Some(state) = self.roles.get_mut(*role) else { return self.malformed(format!("role {role}")); };
                state.r.r48 = *r48; state.r.r7d = *r7d; state.r.r7f = *r7f;
                state.ledger.set_flags(RFlags { r48: *r48, r7d: *r7d, r7f: *r7f });
            }
            Event::MissingKeys { role, keys, .. } => {
                let Some(state) = self.roles.get_mut(*role) else { return self.malformed(format!("role {role}")); };
                state.r.missing_keys.extend(keys.iter().copied());
            }
            Event::Window { role, cells, actors, active, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                let (requested, _) = match cells_of(cells) { Ok(c) => c, Err(e) => return self.malformed(e) };
                let planned = self.policy.plan_window(requested);
                let count = planned.iter().filter(|k| **k != NO_KIND).count();
                let mut compact = [NO_KIND; CELLS];
                for (i, k) in planned.iter().filter(|k| **k != NO_KIND).enumerate() { compact[i] = *k; }
                // The fixture's cell indices (`active`, later `activate`) are the recorded layout's; the
                // planner may lay the window out differently (WP4 `LedgerPolicy`, `CellRule::PinCell0`:
                // Kirby to cell 0), so keep a recorded→planned cell map and follow it.
                let requested_compact: Vec<u32> = requested.iter().copied().filter(|k| *k != NO_KIND).collect();
                let mut cell_map = [0usize, 1, 2];
                for (i, kind) in requested_compact.iter().enumerate() {
                    if let Some(p) = compact.iter().position(|k| k == kind) { cell_map[i] = p; }
                }
                let active = cell_map.get(*active).copied().unwrap_or(0).min(count.saturating_sub(1));
                {
                    let state = &mut self.roles[role];
                    state.cells = compact; state.count = count; state.actors = *actors; state.active = active;
                    state.cell_map = cell_map;
                    state.r.header = [1, compact[0], compact[1], compact[2], 0];
                    state.r.tree = compact.iter().copied().filter(|k| *k != NO_KIND).collect();
                    if (0x24..=0x26).contains(&compact[0]) { state.r.tree.insert(PT_OBJECT_KIND); }
                    if matches!(compact[0], 0x5a | 0x5b) { state.r.tree.insert(AEGIS_OBJECT_KIND); }
                    state.r.flags4c = 4;
                    state.r.pair_refs = 0;
                    state.loader = 14; state.published = true; state.refused = None; state.install_held = false;
                    state.window = [None; CELLS];
                }
                for (i, kind) in compact.iter().copied().take(count).enumerate() {
                    let mut body = self.construct(role, i, kind);
                    if i == active { body.activated = true; self.roles[role].window[i] = Some(body); self.routes[role] = Some(body.controller); }
                }
            }
            Event::Prefetch { role, group, kinds, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                if self.roles[role].groups.get(group).is_some_and(|g| g.state != GroupState::Planned) {
                    return self.malformed(format!("group {group} prefetched twice without a teardown"));
                }
                let live = self.roles[role].live_kind();
                let (members, object): (Vec<u32>, Option<u32>) = {
                    let object = kinds.iter().copied().find(|k| *k == PT_OBJECT_KIND || *k == AEGIS_OBJECT_KIND);
                    (kinds.iter().copied().filter(|k| *k != PT_OBJECT_KIND && *k != AEGIS_OBJECT_KIND).collect(), object)
                };
                let batch = self.roles[role].ledger.new_batch();
                if !self.policy.prefetch_admitted(live) {
                    self.roles[role].groups.insert(*group, Group { state: GroupState::Planned, kinds: members, object_kind: object, bodies: Vec::new(), batch });
                    return;
                }
                for kind in members.iter().copied().chain(object) {
                    let table = &self.table;
                    self.roles[role].r.acquire(kind, table);
                    let state = &mut self.roles[role];
                    if let Ok((token, _)) = state.ledger.acquire(kind, Holder::Prefetch(GroupId(*group)), ReleaseRecipe::OwnedPerKind, batch, &NoConsumers, Self::ledger_role(role)) {
                        state.tokens.push(token);
                    }
                }
                self.roles[role].groups.insert(*group, Group { state: GroupState::Prefetched, kinds: members, object_kind: object, bodies: Vec::new(), batch });
            }
            Event::Build { role, group, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                let Some(record) = self.roles[role].groups.get(group).cloned() else { return self.malformed(format!("build of unknown group {group}")); };
                if record.state != GroupState::Prefetched { return; } // Planned: the mod routes canonically instead.
                if record.kinds.is_empty() { return self.malformed(format!("build of an empty group {group}")); }
                // Phase B: the previous window detaches (saved), the group's kinds construct into the window.
                let previous: Vec<Body> = self.roles[role].window.iter().flatten().copied().collect();
                let prev_cells = self.roles[role].cells;
                let kinds = record.kinds.clone();
                let mut cells = [NO_KIND; CELLS];
                for (i, k) in kinds.iter().take(CELLS).enumerate() { cells[i] = *k; }
                {
                    let state = &mut self.roles[role];
                    state.window = [None; CELLS];
                    state.cells = cells; state.count = kinds.len().min(CELLS); state.active = 0;
                    state.batch = record.batch;
                    // A pair group's build writes one pair-rule reference per body itself (E6 `refs=0->2`).
                    if cells[0] == POPO && cells[1] == NANA { state.r.pair_refs = kinds.len() as u32; }
                }
                for (i, kind) in kinds.iter().take(CELLS).enumerate() {
                    if !self.construct_with_tail(role, i, *kind, cells) { return; }
                }
                // The native object consumer releases the object kind during the synchronous build, then
                // the bank's post-release (old: `0x17e8610` alone; recipe: the orphan key first).
                if let Some(object) = record.object_kind {
                    self.native_object_release(role, object);
                    self.release_kinds(role, &[object]);
                }
                let Some(mut active) = self.roles[role].window[0] else { return; };
                active.activated = true;
                self.roles[role].window[0] = Some(active);
                self.roles[role].r.flags4c |= 4;
                self.routes[role] = Some(active.controller);
                if let Some(record) = self.roles[role].groups.get_mut(group) {
                    record.state = GroupState::Built;
                    record.kinds = prev_cells.iter().copied().filter(|k| *k != NO_KIND).collect();
                    record.bodies = previous;
                }
            }
            Event::Attach { role, group, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                let Some(record) = self.roles[role].groups.get(group).cloned() else { return self.malformed(format!("attach of unknown group {group}")); };
                if record.state != GroupState::Built || record.bodies.is_empty() { return self.malformed(format!("attach of a group without bodies {group}")); }
                let previous: Vec<Body> = self.roles[role].window.iter().flatten().copied().collect();
                let prev_cells = self.roles[role].cells;
                let state = &mut self.roles[role];
                state.window = [None; CELLS];
                let mut cells = [NO_KIND; CELLS];
                for (i, body) in record.bodies.iter().take(CELLS).enumerate() { state.window[i] = Some(*body); cells[i] = body.kind; }
                state.cells = cells; state.count = record.bodies.len().min(CELLS); state.active = 0;
                let mut active = state.window[0].unwrap();
                active.activated = true;
                state.window[0] = Some(active);
                self.routes[role] = Some(active.controller);
                if let Some(record) = self.roles[role].groups.get_mut(group) {
                    record.kinds = prev_cells.iter().copied().filter(|k| *k != NO_KIND).collect();
                    record.bodies = previous;
                }
            }
            Event::Activate { role, cell, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                // The recorded cell → the planned layout's cell (see `Window`).
                let cell = self.roles[role].cell_map.get(*cell).copied().unwrap_or(*cell);
                let Some(mut body) = self.roles[role].window.get(cell).copied().flatten() else { return self.fault(Fault::NullStore { role }); };
                body.activated = true;
                self.roles[role].window[cell] = Some(body);
                self.roles[role].active = cell;
                self.roles[role].kirby_copy = None;
                self.routes[role] = Some(body.controller);
                if body.kind == KIRBY { self.kirby_became_live(role); }
            }
            Event::CopyAcquired { role, kind, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                if self.roles[role].live_kind() != Some(KIRBY) { return self.malformed("copy without a live Kirby".into()); }
                self.roles[role].kirby_copy = Some(*kind);
                let costume = self.roles[role].r.header[4];
                let ctx = self.roles[role].window[self.roles[role].active].unwrap().ctx;
                self.g68.prepare_tuple(*kind, costume, ctx);
                self.roles[role].r.mirror.insert((*kind, costume));
            }
            Event::PublishRejected { role, .. } => {
                let Some(state) = self.roles.get_mut(*role) else { return self.malformed(format!("role {role}")); };
                state.pending_refusal = true;
            }
            Event::Reload { role, cells, actors, bank, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                let (requested, _) = match cells_of(cells) { Ok(c) => c, Err(e) => return self.malformed(e) };
                self.reload(role, requested, *actors, *bank);
            }
            Event::Ready { role, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                self.ready(role);
            }
            Event::Tick { frames, .. } => {
                for _ in 0..*frames {
                    for slot in 0..16 {
                        if let Some(controller) = self.routes[slot] {
                            if self.freed_controllers.contains(&controller) {
                                self.fault(Fault::DanglingRoute { role: slot, slot });
                            }
                        }
                    }
                }
            }
            Event::Expect { role, policy, map, tree, loader, install, pair_refs, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                if policy.as_deref().is_some_and(|p| p != self.policy.name()) { return; }
                let (m, t, l, i, p) = {
                    let s = &self.roles[role];
                    (s.r.map_nodes() as u64, s.r.tree.len() as u64, u64::from(s.loader), u64::from(s.install_held), u64::from(s.r.pair_refs))
                };
                self.check_expect("map", *map, m);
                self.check_expect("tree", *tree, t);
                self.check_expect("loader", loader.map(u64::from), l);
                self.check_expect("install", install.map(u64::from), i);
                self.check_expect("pair_refs", *pair_refs, p);
            }
            Event::Finalize { role, .. } => {
                let role = *role;
                if role >= ROLES { return self.malformed(format!("role {role}")); }
                self.finalize(role);
            }
        }
    }

    /// Construct one window cell and run its tail (`0x60de60`): the pair rule or the per-kind release.
    /// Returns false when the role stalled.
    fn construct_with_tail(&mut self, role: usize, cell: usize, kind: u32, cells: [u32; CELLS]) -> bool {
        if self.roles[role].stalled { return false; }
        let pair = cells[0] == POPO && cells[1] == NANA;
        if pair && self.roles[role].r.pair_refs == 0 && cell > 0 {
            // The reset already ran under this batch: this body's construction never completes.
            self.fault(Fault::State11Stall { role });
            self.roles[role].stalled = true;
            return false;
        }
        if let Some(hazard) = self.g68.walk() {
            self.fault(Fault::Spin { role, hazard });
            self.roles[role].stalled = true;
            return false;
        }
        self.construct(role, cell, kind);
        if pair {
            let before = self.roles[role].r.pair_refs;
            self.roles[role].r.pair_refs = before.saturating_sub(1);
            if before <= 1 { self.roles[role].r.reset(); }
        } else {
            let copy = self.roles[role].kirby_copy;
            let g68 = &mut self.g68;
            self.roles[role].r.release_kind(kind, copy, g68);
        }
        let batch = self.roles[role].batch;
        if self.roles[role].ledger.construction_settled(kind, batch) { self.roles[role].forget_token(kind, false); }
        true
    }

    /// The native object consumer's own `0x17e8610(R, object)` (the partner weapon `0x33f45e4`,
    /// the trainer object; D-SG1 §2.5 INF: it runs during the synchronous build, and `built …
    /// map=9` / `map=1` already excluded the object keys).
    fn native_object_release(&mut self, role: usize, object: u32) {
        let copy = self.roles[role].kirby_copy;
        let g68 = &mut self.g68;
        self.roles[role].r.release_kind(object, copy, g68);
        if self.roles[role].ledger.native_consumed(object) { self.roles[role].forget_token(object, true); }
    }

    /// A mod Reload up to the end of state 6: teardown, destruction of the window, tree clear, the
    /// free check, publication (or the prequeue refusal), native and supplemental requests.
    fn reload(&mut self, role: usize, requested: [u32; CELLS], actors: u32, bank: bool) {
        self.roles[role].install_held = true;
        // before_reload: D-PTA1c route clearing (new), then retire_groups.
        if self.policy.clears_routes() {
            let window: Vec<u32> = self.roles[role].window.iter().flatten().map(|b| b.controller).collect();
            for slot in self.routes.iter_mut() { if slot.is_some_and(|c| window.contains(&c)) { *slot = None; } }
        }
        self.retire_groups(role);
        if self.roles[role].stalled { return; }
        // States 1–3: the window's bodies and contexts are destroyed natively (`651660`), the NRO tree cleared.
        self.roles[role].loader = 1;
        self.roles[role].published = false;
        let bodies: Vec<Body> = self.roles[role].window.iter().flatten().copied().collect();
        for body in bodies { self.destroy_body(role, body, false); }
        if self.roles[role].stalled { return; }
        self.roles[role].window = [None; CELLS];
        self.roles[role].r.clear_tree();
        self.roles[role].loader = 3;
        self.roles[role].kirby_copy = None;
        // State 6: the free predicate on the role's only R.
        if !self.roles[role].r.free() {
            let residual: Vec<u32> = self.roles[role].r.map.keys().copied().collect();
            self.fault(Fault::LoaderRefused7 { role, residual });
            let state = &mut self.roles[role];
            state.loader = 7; state.published = false; state.refused = Some(Refusal::Epilogue); state.stalled = true;
            return;
        }
        let planned = if bank { self.policy.plan_window(requested) } else { requested };
        let count = planned.iter().filter(|k| **k != NO_KIND).count();
        let mut cells = [NO_KIND; CELLS];
        for (i, k) in planned.iter().filter(|k| **k != NO_KIND).enumerate() { cells[i] = *k; }
        // The requested target is the starting form (descriptor `+0x43`): its cell in the planned
        // layout, which is 0 unless the planner moved a pinned member there (`CellRule::PinCell0`).
        // The recorded→planned cell map serves the fixture's later `activate` indices.
        let requested_compact: Vec<u32> = requested.iter().copied().filter(|k| *k != NO_KIND).collect();
        let mut cell_map = [0usize, 1, 2];
        for (i, kind) in requested_compact.iter().enumerate() {
            if let Some(p) = cells.iter().position(|k| k == kind) { cell_map[i] = p; }
        }
        let active = cell_map[0].min(count.saturating_sub(1));
        {
            let state = &mut self.roles[role];
            state.cells = cells; state.count = count; state.actors = actors; state.active = active;
            state.cell_map = cell_map;
            state.r.header = [1, cells[0], cells[1], cells[2], 0];
            state.r.pair_refs = if cells[0] == POPO && cells[1] != NO_KIND { 2 } else { 1 };
            state.r.flags4c |= 1;
            state.loader = 6; state.published = false; state.refused = None;
        }
        let batch = self.roles[role].ledger.new_batch();
        self.roles[role].batch = batch;
        for request in requests_of_17e5bc0(cells) {
            match request {
                NativeRegistration::Kind(kind) => {
                    let table = &self.table;
                    self.roles[role].r.acquire(kind, table);
                    let state = &mut self.roles[role];
                    let recorded = state.ledger.record_native(super::ledger::NativeRequest { kind, keys: super::ledger::registration_set(kind, state.ledger.flags()), nro: true }, batch);
                    state.tokens.push(recorded);
                }
                NativeRegistration::Keys { kind, keys, count } => {
                    self.roles[role].r.register(kind, &keys[..count]);
                    let state = &mut self.roles[role];
                    let mut set = super::ledger::KeySet::EMPTY;
                    for key in &keys[..count] { set = set.with((*key - kind * 42) as u8); }
                    let recorded = state.ledger.record_native(super::ledger::NativeRequest { kind, keys: set, nro: true }, batch);
                    state.tokens.push(recorded);
                }
            }
        }
        // The bank's resource hook (`prepare_resources`): supplemental acquisitions and `R+0x50`.
        if bank {
            for kind in self.policy.supplemental(cells, count) {
                let table = &self.table;
                self.roles[role].r.acquire(kind, table);
                let state = &mut self.roles[role];
                match state.ledger.acquire(kind, Holder::Build(GroupId(0)), ReleaseRecipe::ConstructionTail, batch, &NoConsumers, Self::ledger_role(role)) {
                    Ok((token, _)) => state.tokens.push(token),
                    Err(LedgerRefusal::AlreadyHeld { .. }) => {} // the ledger names the duplicate; the old hook issued it anyway
                    Err(_) => {}
                }
            }
            if let Some(refs) = self.policy.pair_refs(cells, count) { self.roles[role].r.pair_refs = refs; }
        }
        // The prequeue (`0x6510f4`): publication or the refusal path.
        if std::mem::take(&mut self.roles[role].pending_refusal) {
            let plain_kind = !(0x48..=0x4a).contains(&cells[0]);
            match self.policy.on_prequeue_refusal(plain_kind) {
                Refusal::Epilogue => {
                    let state = &mut self.roles[role];
                    state.refused = Some(Refusal::Epilogue); state.stalled = true; state.published = false;
                }
                Refusal::NativeContinue => {
                    let state = &mut self.roles[role];
                    state.refused = Some(Refusal::NativeContinue); state.published = true;
                }
            }
        } else {
            self.roles[role].published = true;
        }
    }

    /// States 7–14: the construction tails per cell (pair rule), activation (routes re-pointed), ready.
    fn ready(&mut self, role: usize) {
        if self.roles[role].stalled || self.roles[role].loader != 6 || !self.roles[role].published { return; }
        let cells = self.roles[role].cells;
        let count = self.roles[role].count;
        for i in 0..count {
            if !self.construct_with_tail(role, i, cells[i], cells) { return; }
        }
        // A canonical PT/Aegis load: the native object consumer releases the object kind it registered
        // (`0x12b4`; `0x1332/0x1333` — native never registers `0x1338`, D-SG1 §4).
        match cells[0] {
            0x24..=0x26 => self.native_object_release(role, PT_OBJECT_KIND),
            0x5a | 0x5b => self.native_object_release(role, AEGIS_OBJECT_KIND),
            _ => {}
        }
        if self.roles[role].stalled { return; }
        let active = self.roles[role].active;
        if let Some(mut body) = self.roles[role].window[active] {
            body.activated = true;
            self.roles[role].window[active] = Some(body);
            self.routes[role] = Some(body.controller);
        }
        let state = &mut self.roles[role];
        state.r.flags4c |= 4;
        state.loader = 14;
        state.install_held = false;
    }

    /// Entry destruction / scene exit: a stranded INSTALL is reported, then everything is torn down.
    fn finalize(&mut self, role: usize) {
        if self.roles[role].install_held && self.roles[role].loader != 14 {
            let loader = self.roles[role].loader;
            self.fault(Fault::StrandedInstall { role, loader });
        }
        self.g68.poisoned = None;
        self.roles[role].stalled = false;
        self.retire_groups(role);
        let bodies: Vec<Body> = self.roles[role].window.iter().flatten().copied().collect();
        for body in bodies { self.destroy_body(role, body, true); }
        let state = &mut self.roles[role];
        state.window = [None; CELLS];
        state.r = RModel { r48: state.r.r48, r7d: state.r.r7d, r7f: state.r.r7f, missing_keys: state.r.missing_keys.clone(), ..RModel::default() };
        state.loader = 14; state.published = true; state.refused = None; state.install_held = false; state.kirby_copy = None;
        state.tokens.clear();
        state.ledger = Ledger::new(state.flags());
        self.routes[role] = None;
    }

    /// The ledger's prediction against the model's map for `role` (shadow-mode check): `(residual, missing)`.
    pub fn drift(&self, role: usize) -> Option<(KeyRefs, KeyRefs)> {
        let state = &self.roles[role];
        let actual = state.r.snapshot();
        let r = state.ledger.reconcile(&actual);
        (!r.balanced()).then_some((r.residual, r.missing))
    }
}

/// Replay a fixture under a policy; the faults in order, with an implicit finalize of both roles
/// at the end (a stranded INSTALL is a fault even when the recording stopped in the stall).
pub fn replay(fixture: &[Event], policy: &dyn Policy, table: &KindTable) -> Vec<Fault> {
    let mut world = World::new(policy, table.clone());
    for event in fixture { world.apply(event); }
    for role in 0..ROLES {
        if world.roles[role].install_held && world.roles[role].loader != 14 {
            let loader = world.roles[role].loader;
            world.fault(Fault::StrandedInstall { role, loader });
        }
    }
    world.faults
}

/// The ELF dump committed with the fixtures.
pub const KEYS_13_0_5_JSON: &str = include_str!("../../../../tests/fixtures/swap/keys-13.0.5.json");

pub fn kind_table() -> KindTable {
    KindTable::from_elf_dump_json(KEYS_13_0_5_JSON).expect("keys-13.0.5.json parses")
}

