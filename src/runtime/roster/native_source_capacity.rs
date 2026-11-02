//! Shared read-only native entry7 contract and native creation boundary.
//! Saved addresses are identities, never authority to dereference old objects.
//! Created entries are 7, then 6 and 5 for the extended Training candidate only.
use super::{aligned, native_source, read, setting, BASE};
use crate::runtime::compat::common;
use skyline::hooks::InlineCtx;
use super::native_service::ServiceEntryContext;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

const SLOT: usize = 7;
/// Scene-specific creation plans share a stable physical claim index.
use crate::engine::preload::cycle_preload_capacity::{extra_slots, ALL_EXTRA_SLOTS, EXTRA_SLOTS};
use crate::engine::mii::backend::Layout as Backend;

fn claim_index(slot: usize) -> Option<usize> {
    ALL_EXTRA_SLOTS.iter().position(|extra| *extra == slot)
}

/// A compiled slot is not admission: preflight also checks the scene plan,
/// vacant native roots, selection rows, input ownership and prior registrations.
fn extra_compiled(slot: usize) -> bool {
    slot == SLOT
        || (cfg!(feature = "cycle-preload-extended") && EXTRA_SLOTS.contains(&slot))
        || (cfg!(feature = "lobby-preload-extended") && extra_slots(Backend::Ldn).contains(&slot))
}

#[cfg(feature = "managed-grouped-source-alpha")]
#[path = "native_source_actors.rs"]
mod canonical;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) identity: usize,
    pub(super) count: u32,
    pub(super) kinds: [u32; 3],
    pub(super) installed: u64,
    pub(super) phase: [u8; 3],
    pub(super) bodies: [(usize, u32); 3],
}

/// Two witnessed layouts. The legacy snapshot wrapper always selects Training.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layout {
    Training,
    #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
    LdnWaiting,
    #[cfg(feature = "lobby-preload-extended")]
    QuickplayWaiting,
}

impl Layout {
    pub(super) fn backend(self) -> Backend {
        match self {
            Self::Training => Backend::Training,
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Self::LdnWaiting => Backend::Ldn,
            #[cfg(feature = "lobby-preload-extended")]
            Self::QuickplayWaiting => Backend::Quickplay,
        }
    }
    pub(super) fn waiting(self) -> bool { self.backend().waiting() }
    pub(super) fn owners(self) -> [u32; 4] { self.backend().owners().map(|slot| slot as u32) }
    pub(super) fn extra_slots(self) -> &'static [usize; EXTRA_SLOTS.len()] { extra_slots(self.backend()) }
    #[cfg(feature = "source-capacity-probe")]
    pub(super) fn callback(self, owner: u32) -> u8 {
        match self {
            Self::Training => u8::from(owner == 0),
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Self::LdnWaiting => match owner { 5 => 1, 6 => 2, _ => 0 },
            #[cfg(feature = "lobby-preload-extended")]
            Self::QuickplayWaiting => match owner { 5 => 1, 6 => 2, _ => 0 },
        }
    }
    #[cfg(feature = "source-capacity-probe")]
    pub(super) fn callbacks_complete(self, seen: u8) -> bool {
        match self {
            Self::Training => seen == 1,
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Self::LdnWaiting => seen == 3,
            #[cfg(feature = "lobby-preload-extended")]
            Self::QuickplayWaiting => seen == 3,
        }
    }
}

/// Waiting-room additions to the scene a created-entry load must leave unchanged (scene bits 17-20), with what the
/// load's validity takes from each (FAIL 2 scene-proof follow-up):
/// - `selection`: every row's seat bytes (`+0xa`, `+0x79`, `+0xa7`). The seats the reservation and the load were
///   admitted under; `owned_checked` requires the same tuples on every callback. Relevant: a member taking a seat
///   changes the layout the created entries live beside.
/// - `resources`: the four layout owners' resource kind/costume. Relevant: the load must not touch another owner's
///   resource owner.
/// - `input`: the legacy entry-7 view (`inputs()`), whatever entry is loading; the entry-7 probes read it as that.
///   Its rows and shared fields are relevant and proven by the load's input contract; entry 7's own
///   mapping/controller/flag/CSS role are relevant to loads on entry 7 only (`canonical::comparison_view`).
#[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LdnFields {
    pub(super) selection: [(u8, u8, u8); 8],
    pub(super) resources: Option<[(u32, u32); 4]>,
    pub(super) input: Option<Inputs>,
}

/// The scene a created-entry load must leave unchanged between its commit and completion, apart from
/// `listeners` (the load registers its own actors). Every field is relevant to the load's validity: the native
/// roots it registers with (`manager`, `internal`, `context`, `service`, `service_inner`), the entry count,
/// Training CPU quantity, rule bytes and ratio bits it is built under, the eight entries' roots
/// (`primary`, `backup`, `resources`, `aliases`) and seat bytes (`selection`) that a load on one entry must
/// not move for another, and the four layout owners' entries (`original`), the side-effect guard on the
/// visible and helper fighters. Waiting rooms add [`LdnFields`]. Diff bits: `managed::snapshot_difference`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) layout: Layout,
    pub(super) manager: usize,
    pub(super) internal: usize,
    pub(super) context: usize,
    pub(super) service: usize,
    pub(super) service_inner: usize,
    pub(super) listeners: u64,
    pub(super) count: u32,
    pub(super) quantity: Option<u32>,
    #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
    pub(super) ldn: Option<LdnFields>,
    pub(super) rules: [u8; 2],
    pub(super) ratio_bits: u32,
    pub(super) primary: [usize; 8],
    pub(super) backup: [usize; 8],
    pub(super) resources: [usize; 8],
    pub(super) aliases: [usize; 8],
    pub(super) selection: [u8; 8],
    pub(super) original: [Entry; 4],
}

/// The dispatcher owns this list node and callback object right now. Verify its
/// preserved registers and exact non-virtual callback tuple before using globals.
pub(super) unsafe fn dispatch_identity(
    ctx: &ServiceEntryContext,
    entry: *const u8,
) -> Option<(usize, usize)> {
    let [dispatcher_inner, dispatcher_sentinel, dispatcher_node, caller_lr] = ctx.dispatcher_tuple();
    let base = BASE.load(Ordering::Acquire);
    if caller_lr != base + 0x1755e80 {
        return None;
    }
    let service = read::<*const u8>(base as *const u8, 0x52c50f8);
    if !aligned(service) {
        return None;
    }
    let inner = read::<*const u8>(service, 0);
    if !aligned(inner)
        || dispatcher_inner != inner as usize
        || dispatcher_sentinel != inner as usize + 0x938
    {
        return None;
    }
    let node = dispatcher_node as *const u8;
    if !aligned(node) || node == inner.add(0x938) {
        return None;
    }
    let callback = read::<*const u8>(node, 0x10);
    if !aligned(callback)
        || read::<usize>(callback, 0) != base + 0x4f81468
        || read::<*const u8>(callback, 0x10) != entry
        || read::<usize>(callback, 0x18) != base + 0x650010
        || read::<usize>(callback, 0x20) != 0
    {
        return None;
    }
    Some((service as usize, inner as usize))
}

pub(super) unsafe fn entry_observation(entry: *const u8, owner: u32) -> Option<Entry> {
    if !aligned(entry) || read::<u32>(entry, 0xe0) != owner {
        return None;
    }
    let installed = read::<u64>(entry, 0x4150);
    if installed > 3 {
        return None;
    }
    let mut bodies = [(0, 0); 3];
    for (index, result) in bodies.iter_mut().enumerate().take(installed as usize) {
        let body = read::<*const u8>(entry, 0x4160 + index * 8);
        if !aligned(body)
            || read::<usize>(body, 0) != BASE.load(Ordering::Acquire) + 0x4f80210
            || read::<u64>(body, 0x10) != u64::from(owner)
        {
            return None;
        }
        *result = (body as usize, read(body, 8));
    }
    Some(Entry {
        identity: entry as usize,
        count: read(entry, 0x14),
        kinds: std::array::from_fn(|i| read(entry, 0x18 + i * 4)),
        installed,
        phase: [
            read(entry, 0x5920),
            read(entry, 0x591c),
            read(entry, 0x5935),
        ],
        bodies,
    })
}

/// Reacquire every pointer from current native roots, only at a verified service
/// callback. No pointer in a previously saved Snapshot is dereferenced here.
#[cfg(any(feature = "source-capacity-link-probe", feature = "managed-third-source-alpha"))]
pub(super) unsafe fn snapshot(ctx: &ServiceEntryContext, entry: *const u8, owner: u32) -> Option<Snapshot> {
    snapshot_for(Layout::Training, ctx, entry, owner)
}

pub(super) unsafe fn snapshot_for(
    layout: Layout, ctx: &ServiceEntryContext, entry: *const u8, owner: u32,
) -> Option<Snapshot> {
    let (service, service_inner) = dispatch_identity(ctx, entry)?;
    let base = BASE.load(Ordering::Acquire);
    let symbol = setting(&common::FIGHTER_MANAGER_ADDR);
    // The creation helper uses this exact singleton; reject an alternate binding.
    if symbol != base + 0x52b94f8 {
        return None;
    }
    let manager = read::<*const u8>(symbol as *const u8, 0);
    if !aligned(manager) {
        return None;
    }
    let internal = read::<*const u8>(manager, 0);
    let context = read::<*const u8>(base as *const u8, 0x5324680);
    if !aligned(internal) || !aligned(context) {
        return None;
    }
    let primary: [usize; 8] = std::array::from_fn(|i| read(internal, 0x20 + i * 8));
    if owner >= 8 || primary[owner as usize] != entry as usize {
        return None;
    }
    let owners = layout.owners();
    let original = [
        entry_observation(primary[owners[0] as usize] as *const u8, owners[0])?,
        entry_observation(primary[owners[1] as usize] as *const u8, owners[1])?,
        entry_observation(primary[owners[2] as usize] as *const u8, owners[2])?,
        entry_observation(primary[owners[3] as usize] as *const u8, owners[3])?,
    ];
    let current = Snapshot {
        layout,
        manager: manager as usize,
        internal: internal as usize,
        context: context as usize,
        service,
        service_inner,
        listeners: read(service_inner as *const u8, 0x930),
        count: read(internal, 0xa0),
        quantity: match layout {
            Layout::Training => Some(native_source::training_cpu_quantity()?),
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Layout::LdnWaiting => None,
            #[cfg(feature = "lobby-preload-extended")]
            Layout::QuickplayWaiting => None,
        },
        #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
        ldn: None,
        // Preserve observed rule bytes and ratio bits without reinterpreting them.
        rules: [read(internal, 0xea), read(internal, 0xeb)],
        ratio_bits: read(internal, 0xf0),
        primary,
        backup: std::array::from_fn(|i| read(internal, 0x60 + i * 8)),
        resources: std::array::from_fn(|i| read(context, i * 8)),
        aliases: std::array::from_fn(|i| read(context, 0xe8 + i * 8)),
        selection: std::array::from_fn(|i| read((base + 0x52c5730 + i * 0x260) as *const u8, 0xa)),
        original,
    };
    #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
    if layout.waiting() {
        // A failed supplemental read remains visible. It must not discard the
        // current native owner needed to bind its eventual retirement.
        return Some(Snapshot {
            ldn: Some(LdnFields {
                selection: std::array::from_fn(|i| {
                    let row = (base + 0x52c5730 + i * 0x260) as *const u8;
                    (read(row, 0xa), read(row, 0x79), read(row, 0xa7))
                }),
                resources: original_resources(current),
                input: inputs().ok(),
            }),
            ..current
        });
    }
    Some(current)
}

/// Legacy entry7 wrapper; its remaining callers are the capacity probes.
#[cfg(feature = "source-capacity-probe")]
pub(super) fn creation_delta(before: Snapshot, after: Snapshot) -> bool {
    creation_delta_at(before, after, SLOT)
}

/// Exactly one new entry at `slot`: its three roots, count+1 and listeners+1.
pub(super) fn creation_delta_at(before: Snapshot, after: Snapshot, slot: usize) -> bool {
    if slot >= 8 {
        return false;
    }
    let mut expected = before;
    expected.primary[slot] = after.primary[slot];
    expected.resources[slot] = after.resources[slot];
    expected.aliases[slot] = after.aliases[slot];
    expected.count += 1;
    let Some(listeners) = expected.listeners.checked_add(1) else {
        return false;
    };
    expected.listeners = listeners;
    expected == after
        && after.primary[slot] != 0
        && !before.primary.contains(&after.primary[slot])
        && !before.resources.contains(&after.resources[slot])
        && !before.aliases.contains(&after.resources[slot])
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct InputSlot {
    pub(super) pointer: usize,
    pub(super) state: u8,
    pub(super) key: i32,
    pub(super) owner: i32,
    pub(super) alias: i32,
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Inputs {
    /// Created entry these records/mapping belong to: records `slot` and `slot + 8`.
    pub(super) slot: usize,
    pub(super) service: usize,
    pub(super) data: usize,
    pub(super) records: usize,
    pub(super) mapping: usize,
    /// Mapping of `slot` (name kept for existing callers; 7 unless extended).
    pub(super) map7: i32,
    pub(super) controller: usize,
    pub(super) controller_property: usize,
    pub(super) controller_flag: u8,
    pub(super) css_role: u8,
    pub(super) cpu_flag: u8,
    pub(super) slots: [InputSlot; 16],
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
impl Inputs {
    // Native313e90 supplies the occupancy/alias predicates. Also reject an
    // active control bound to either record through its owner at+160, and
    // require null pointers/unassigned keys for the two records it clears.
    // No meaning is invented for the other state-byte values.
    pub(super) fn unclaimed(self) -> bool {
        if self.slot >= 8 {
            return false;
        }
        let routes = [self.slot, self.slot + 8];
        let route = |index: i32| routes.iter().any(|route| *route as i32 == index);
        routes
            .iter()
            .all(|i| self.slots[*i].pointer == 0 && self.slots[*i].key == -1)
            && !self.slots.iter().enumerate().any(|(i, slot)| {
                slot.state == 1
                    && slot.key != -1
                    && slot.pointer != 0
                    && (routes.contains(&i) || route(slot.owner) || route(slot.alias))
            })
    }
}

/// Legacy entry7 view for the probes and the LDN snapshot.
#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[cfg(any(
    feature = "source-lifecycle-probe",
    feature = "source-ldn-capacity-empty-probe",
    feature = "managed-ldn-third-source-alpha"
))]
pub(super) unsafe fn inputs() -> Result<Inputs, (&'static str, usize, usize)> {
    inputs_for(SLOT)
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
pub(super) unsafe fn inputs_for(slot: usize) -> Result<Inputs, (&'static str, usize, usize)> {
    if slot >= 8 {
        return Err(("input slot outside the eight native entries", slot, 0));
    }
    let base = BASE.load(Ordering::Acquire);
    let service = read::<*const u8>(base as *const u8, 0x52b6fd8);
    if !aligned(service) {
        return Err((
            "current native input service unavailable",
            service as usize,
            0,
        ));
    }
    let active = read::<u8>(service, 8);
    if active != 1 {
        return Err((
            "current native input service is not active",
            service as usize,
            usize::from(active),
        ));
    }
    let data = read::<*const u8>(service, 0);
    if !aligned(data) {
        return Err((
            "current native input data identity unavailable",
            service as usize,
            data as usize,
        ));
    }
    let records = read::<*const u8>(data, 0x90);
    let mapping = read::<*const u8>(base as *const u8, 0x52c50f0);
    if !aligned(records) || !aligned(mapping) {
        return Err((
            "current native input records or owner-map identity unavailable",
            records as usize,
            mapping as usize,
        ));
    }
    let mut slots = [InputSlot {
        pointer: 0,
        state: 0,
        key: -1,
        owner: -1,
        alias: -1,
    }; 16];
    for (index, slot) in slots.iter_mut().enumerate() {
        slot.pointer = read(data, index * 8);
        slot.state = read(data, 0x80 + index);
        slot.key = read(records, index * 0x280 + 0x28);
        // Native cleanup dereferences only these current active records.
        if slot.state == 1 && slot.key != -1 && slot.pointer != 0 {
            let control = slot.pointer as *const u8;
            if !aligned(control) {
                return Err((
                    "active input control pointer is unaligned",
                    index,
                    slot.pointer,
                ));
            }
            slot.owner = read(control, 0x160);
            slot.alias = read(control, 0xc50);
        }
    }
    let map7 = read::<i32>(mapping, 0x298 + slot * 4);
    let controller = if (map7 as u32) <= 9 {
        read::<*const u8>((base + 0x5339860) as *const u8, map7 as usize * 8)
    } else {
        (base + 0x5339830) as *const u8
    };
    // The native constructor permits a null table member and skips its
    // property read; out-of-range mappings use the native fallback object.
    let (controller_property, controller_flag) = if controller.is_null() {
        (0, 0)
    } else {
        if !aligned(controller) {
            return Err((
                "current mapped controller is unaligned",
                map7 as u32 as usize,
                controller as usize,
            ));
        }
        let property = read::<*const u8>(controller, 0x18);
        if !aligned(property) {
            return Err((
                "current mapped controller property is unavailable",
                controller as usize,
                property as usize,
            ));
        }
        (property as usize, read::<u8>(property, 8))
    };
    Ok(Inputs {
        slot,
        service: service as usize,
        data: data as usize,
        records: records as usize,
        mapping: mapping as usize,
        map7,
        controller: controller as usize,
        controller_property,
        controller_flag,
        css_role: read((base + 0x52c5730 + slot * 0x260) as *const u8, 0x1ba),
        cpu_flag: read(data, 0xd0),
        slots,
    })
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
pub(super) unsafe fn original_resources(current: Snapshot) -> Option<[(u32, u32); 4]> {
    let mut fields = [(0, 0); 4];
    for (index, field) in fields.iter_mut().enumerate() {
        let owner = current.resources[current.layout.owners()[index] as usize] as *const u8;
        if !aligned(owner) || read::<usize>(owner, 0) != BASE.load(Ordering::Acquire) + 0x509fb10 {
            return None;
        }
        *field = (read(owner, 0x58), read(owner, 0x64));
    }
    Some(fields)
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[cfg(all(not(feature = "source-capacity-group-probe"), any(
    feature = "source-capacity-link-probe", not(feature = "managed-grouped-source-alpha")
)))]
#[cfg(feature = "source-capacity-link-probe")]
pub(super) unsafe fn ordinary_actor(entry: *const u8, kind: u32) -> Option<(usize, usize)> {
    ordinary_actor_at(entry, SLOT, kind)
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[cfg(all(not(feature = "source-capacity-group-probe"), any(
    feature = "source-capacity-link-probe", not(feature = "managed-grouped-source-alpha")
)))]
pub(super) unsafe fn ordinary_actor_at(entry: *const u8, slot: usize, kind: u32) -> Option<(usize, usize)> {
    let actor = read::<*const u8>(entry, 0x4178);
    if !aligned(actor)
        || read::<usize>(actor, 0) != BASE.load(Ordering::Acquire) + 0x4f85d30
        || read::<u32>(actor, 8) != slot as u32
        || read::<u32>(actor, 0x6c) != slot as u32
        || read::<u8>(actor, 0x68) != 0
        || read::<u8>(actor, 0x69) != 1
        || read::<u8>(actor, 0x70) != 0
        || read::<u32>(actor, 0x74) != kind
    {
        return None;
    }
    let control = read::<*const u8>(actor, 0x58);
    let information = read::<*const u8>(entry, 0xf8);
    if !aligned(control)
        || !aligned(information)
        || read::<i32>(control, 0x160) != slot as i32
        || read::<u8>(information, 0x92) != 0
    {
        return None;
    }
    Some((actor as usize, read(control, 0x168)))
}

unsafe fn verify_creation(base: usize) -> Result<(), usize> {
    // FNV-1a fingerprints of exact game13.0.5 instruction ranges. Full helper
    // identity is checked without embedding a native function body in source.
    for (offset, len, expected) in [
        (0x1652f40, 20, 0xc885cab694e69bc8u64),
        (0x4eb6a0, 224, 0xf0083382faa7d480),
        (0x1755e58, 52, 0x6b1cbd1a14a8a68c),
        (0x651640, 28, 0xf1df3a8ba145274c),
        (0x652214, 16, 0xd5869a938fb63fa8),
        (0x17e44b0, 48, 0x513f7e02e0de5031),
        (0x17eb000, 32, 0xb91b0e94d893f84f),
    ] {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in std::slice::from_raw_parts((base + offset) as *const u8, len) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
        if hash != expected {
            return Err(offset);
        }
    }
    let mut symbol = 0usize;
    skyline::nn::ro::LookupSymbol(
        &mut symbol,
        b"_ZN3app16kiiladarzmanager20entry_summon_fighterEi\0".as_ptr(),
    );
    if symbol != base + 0x1652f40 {
        return Err(0x1652f40);
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum Claimant {
    #[cfg(feature = "source-capacity-probe")]
    Probe = 1,
    #[cfg(feature = "managed-third-source-alpha")]
    Managed = 2,
}
// One claim per ALL_EXTRA_SLOTS index. Each is held from its creation call until the
// native cleanup of the owner bound to it; index 0 (entry 7) is the legacy claim.
static CLAIM: [AtomicU8; ALL_EXTRA_SLOTS.len()] = [const { AtomicU8::new(0) }; ALL_EXTRA_SLOTS.len()];
static CLAIM_OWNER: [AtomicUsize; ALL_EXTRA_SLOTS.len()] = [const { AtomicUsize::new(0) }; ALL_EXTRA_SLOTS.len()];
static CREATION_VERIFIED: AtomicBool = AtomicBool::new(false);
#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
static LOAD_VERIFIED: AtomicBool = AtomicBool::new(false);

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
pub(super) fn load_supported() -> bool {
    LOAD_VERIFIED.load(Ordering::Acquire)
}

/// All clients verify the pristine ranges before either shared hook is installed.
#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
unsafe fn verify_load(base: usize) -> bool {
    for (offset, len, expected) in [
        (0x653240, 592, 0xee56df013a817f73u64),
        (0x654dc0, 704, 0x549eb59c76d87db2),
        (0x655320, 912, 0xe0a543092d07ab2a),
        (0x6b6f10, 2572, 0xdfd785a8a55d9fcf),
        (0x6b7920, 132, 0xcb03f13e76de2a80),
        (0x313e90, 796, 0x8b2b8d1c33b31bc6),
        (0x17e4700, 80, 0x5d4213a39662f638),
        #[cfg(feature = "managed-third-source-alpha")]
        (0x150fc20, 1316, 0xb240697c885cdbeb),
        #[cfg(feature = "managed-third-source-alpha")]
        (0x15095cc, 36, 0x40be917c207acaf7),
        #[cfg(feature = "managed-third-source-alpha")]
        (0x65bd80, 132, 0x505c394d7cdcaa02),
        #[cfg(any(feature = "source-capacity-aegis-probe", feature = "source-capacity-trainer-probe", feature = "managed-grouped-source-alpha"))]
        (0x453c158, 24, 0x74162f9ac96d0337),
        #[cfg(any(feature = "source-capacity-group-probe", feature = "managed-grouped-source-alpha"))]
        (0x651660, 260, 0x871c1dd21365e5e3),
        #[cfg(any(feature = "source-capacity-group-probe", feature = "managed-grouped-source-alpha"))]
        (0x6b7aa0, 52, 0x7009ea488edbd02a),
    ] {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in std::slice::from_raw_parts((base + offset) as *const u8, len) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
        if hash != expected {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[NativeSourceCapacity] load contract mismatch +0x{:x} len={} expected=0x{:016x} actual=0x{:016x}; load clients disabled",
                offset, len, expected, hash));
            return false;
        }
    }
    #[cfg(any(feature = "source-capacity-group-probe", feature = "managed-grouped-source-alpha"))]
    if read::<usize>((base + 0x4f85d30) as *const u8, 0) != base + 0x6b7920
        || read::<usize>((base + 0x4f85d30) as *const u8, 8) != base + 0x6b7aa0
    {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[NativeSourceCapacity] grouped actor vtable mismatch; load disabled"));
        return false;
    }
    true
}

#[skyline::hook(offset = 0x652214, inline)]
unsafe fn listener_removed(ctx: &InlineCtx) {
    #[cfg(feature = "source-capacity-probe")]
    super::source_capacity_probe::listener_removed(ctx);
    #[cfg(feature = "managed-third-source-alpha")]
    super::sources::capacity_listener_removed(ctx);
}

#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
#[skyline::hook(offset = 0x6b79a0, inline)]
unsafe fn actor_cleanup(ctx: &InlineCtx) {
    #[cfg(feature = "source-capacity-link-probe")]
    super::source_capacity_probe::actor_cleanup(ctx);
    #[cfg(feature = "managed-third-source-alpha")]
    super::sources::capacity_actor_cleanup(ctx);
}

#[cfg(feature = "source-capacity-probe")]
pub(super) fn creation_supported() -> bool {
    CREATION_VERIFIED.load(Ordering::Acquire)
}

/// The caller also owns INSTALL. Retain this claim until actual owner cleanup;
/// failed post-call qualification deliberately cannot rearm another creator.
/// Legacy entry7 wrapper; its remaining caller is the capacity probe.
#[cfg(feature = "source-capacity-probe")]
pub(super) unsafe fn create(claimant: Claimant) -> bool {
    create_at(claimant, SLOT)
}

/// The same single native call with `slot` as its index argument, under that
/// slot's own claim. Slots 6/5 also require the extended feature; the managed
/// preflight additionally requires Training and the earlier slots' registrations.
pub(super) unsafe fn create_at(claimant: Claimant, slot: usize) -> bool {
    let Some(index) = claim_index(slot) else {
        return false;
    };
    if !extra_compiled(slot) || !CREATION_VERIFIED.load(Ordering::Acquire) {
        return false;
    }
    if CLAIM[index]
        .compare_exchange(0, claimant as u8, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    let create: unsafe extern "C" fn(i32) =
        std::mem::transmute(BASE.load(Ordering::Acquire) + 0x1652f40);
    create(slot as i32);
    true
}

/// An unknown slot reads as claimed, so it can never be created.
#[cfg(feature = "managed-third-source-alpha")]
pub(super) fn claimed_at(slot: usize) -> bool {
    claim_index(slot).is_none_or(|index| CLAIM[index].load(Ordering::Acquire) != 0)
}

#[cfg(feature = "source-capacity-probe")]
pub(super) fn bind_owner(claimant: Claimant, owner: usize) {
    bind_owner_at(claimant, SLOT, owner);
}

pub(super) fn bind_owner_at(claimant: Claimant, slot: usize, owner: usize) {
    let Some(index) = claim_index(slot) else {
        return;
    };
    if owner != 0 && CLAIM[index].load(Ordering::Acquire) == claimant as u8 {
        CLAIM_OWNER[index].store(owner, Ordering::Release);
    }
}

/// Current claim holder and bound owner for one slot; values only.
#[cfg(feature = "managed-third-source-alpha")]
fn claim_state(index: usize) -> (u8, usize) {
    (CLAIM[index].load(Ordering::Acquire), CLAIM_OWNER[index].load(Ordering::Acquire))
}

#[skyline::hook(offset = 0x17e44c4, inline)]
unsafe fn owner_cleanup_complete(ctx: &InlineCtx) {
    #[cfg(feature = "source-capacity-probe")]
    super::source_capacity_probe::owner_cleanup_complete(ctx);
    let owner = ctx.registers[19].x() as usize;
    #[cfg(feature = "managed-third-source-alpha")]
    let managed_owner = super::sources::capacity_owner_retired(owner);
    // Route by the actual retiring owner. Owners are distinct native objects, so
    // at most one slot is bound to it. Its registration ends before its claim.
    let released_index = if owner == 0 {
        None
    } else {
        (0..ALL_EXTRA_SLOTS.len()).find(|index| {
            CLAIM_OWNER[*index]
                .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        })
    };
    if let Some(index) = released_index {
        #[cfg(feature = "managed-third-source-alpha")]
        managed::unregister(index);
        CLAIM[index].store(0, Ordering::Release);
    }
    #[cfg(feature = "source-ldn-capacity-empty-probe")]
    super::source_capacity_probe::claim_retired(
        owner, released_index.is_some(),
        CLAIM[0].load(Ordering::Acquire), CLAIM_OWNER[0].load(Ordering::Acquire),
    );
    #[cfg(feature = "managed-third-source-alpha")]
    if managed_owner {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[ManagedSourceCleanup] claim owner=0x{:x} released={} claim={} bound_owner=0x{:x}",
            owner,
            released_index.is_some(),
            CLAIM[0].load(Ordering::Acquire),
            CLAIM_OWNER[0].load(Ordering::Acquire)
        ));
    }
    #[cfg(feature = "managed-third-source-alpha")]
    if let Some(index) = released_index.filter(|index| *index != 0) {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[ManagedSourceCleanup] extended claim slot={} owner=0x{:x} released=true",
            ALL_EXTRA_SLOTS[index], owner));
    }
}

/// Seam-table rows (`native_seams::rows_a1`) at the start of each creation contract range and the two hooks
/// installed with it. The FNV contracts above stay: they cover whole helper bodies (up to 2572 B) without
/// embedding them, which a 64-byte row cannot. WP-A1 shadow: the contracts alone decide until
/// `native_source_capacity` is flipped to authoritative; after the flip a contract AND its rows must pass.
const CREATION_SEAMS: &[&str] = &["capacity.entry_summon", "capacity.creation_4eb6a0", "capacity.caller_1755e58",
    "capacity.functor_651640", "capacity.listener_removed", "capacity.owner_cleanup_window",
    "capacity.owner_cleanup_complete", "capacity.creation_17eb000"];
/// The load contract's rows (same cfg as `verify_load`'s ranges) and the actor-cleanup hook. The rodata
/// descriptor tables (0x453c158) are outside the code-seam table and stay an FNV contract only.
#[cfg(any(
    feature = "source-capacity-link-probe",
    feature = "managed-third-source-alpha"
))]
const LOAD_SEAMS: &[&str] = &[
    "lifecycle.reload_entry", "body_bank.prepare_models", "capacity.load_655320", "capacity.load_6b6f10",
    "capacity.load_6b7920", "capacity.load_313e90", "capacity.owner_load", "capacity.actor_cleanup",
    #[cfg(feature = "managed-third-source-alpha")]
    "capacity.load_150fc20",
    #[cfg(feature = "managed-third-source-alpha")]
    "capacity.load_15095cc",
    #[cfg(feature = "managed-third-source-alpha")]
    "capacity.load_65bd80",
    #[cfg(any(feature = "source-capacity-group-probe", feature = "managed-grouped-source-alpha"))]
    "capacity.group_651660",
    #[cfg(any(feature = "source-capacity-group-probe", feature = "managed-grouped-source-alpha"))]
    "capacity.group_6b7aa0",
    // Rows 27/36: the 24-B `.rodata` contract at 0x453c158 (main row, read once WP-A core r6 adds the region).
    #[cfg(any(feature = "source-capacity-aegis-probe", feature = "source-capacity-trainer-probe", feature = "managed-grouped-source-alpha"))]
    "capacity.actor_tables",
];

pub(super) unsafe fn init(base: usize) {
    use crate::runtime::seams::seam_install::gate;
    let creation = verify_creation(base);
    if let Err(offset) = creation {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[NativeSourceCapacity] creation contract mismatch +0x{:x}; disabled",
            offset
        ));
    }
    // `gate(..) && contract`: in shadow the gate returns the contract; once authoritative it returns the
    // rows' outcome, and the long-range contract still has to hold.
    if !gate("native_source_capacity", CREATION_SEAMS, creation.is_ok()) {
        if creation.is_ok() {
            crate::engine::compat::logging::diagnostic(format_args!("[NativeSourceCapacity] creation seam guard refused; disabled"));
        }
        return;
    }
    if creation.is_err() {
        return;
    }
    #[cfg(any(
        feature = "source-capacity-link-probe",
        feature = "managed-third-source-alpha"
    ))]
    let load_verified = {
        let contract = verify_load(base);
        gate("native_source_capacity", LOAD_SEAMS, contract) && contract
    };
    // Both original ranges have now been checked, before any overlapping patch.
    skyline::install_hooks!(owner_cleanup_complete, listener_removed);
    #[cfg(any(
        feature = "source-capacity-link-probe",
        feature = "managed-third-source-alpha"
    ))]
    if load_verified {
        skyline::install_hooks!(actor_cleanup);
        LOAD_VERIFIED.store(true, Ordering::Release);
    }
    CREATION_VERIFIED.store(true, Ordering::Release);
    #[cfg(feature = "managed-third-source-alpha")]
    managed::init();
}

#[cfg(feature = "managed-third-source-alpha")]
use crate::engine::swap::source_descriptor as source_descriptor;

#[cfg(feature = "managed-third-source-alpha")]
pub(super) mod managed {
    use super::*;
    use std::sync::Mutex;
    use crate::engine::swap::capacity_creation::{self as creation, ExactMiiProof, OwnerAdmission, OwnerFacts, Refusal};
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    use super::super::mii_ordinary_player::native::VisibleMii;
    /// Without the ordinary Mii adapters there is no visible exact-Mii witness.
    #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
    type VisibleMii = ();
    static ENABLED: AtomicBool = AtomicBool::new(false);

    /// A qualified creation's own native roots, numeric identities only. Present
    /// from a successful `create_reserved` until native cleanup of its bound owner.
    /// It is the only authority for tolerating another created slot.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Created {
        layout: Layout,
        entry: usize,
        resource: usize,
        alias: usize,
        selection: u8,
        /// Creation descriptor word 1 (entry+4). Every load preflight requires the
        /// current descriptor to equal the creation descriptor apart from kind/count/
        /// actor words, byte 0x43 and the reset reference pair, so this word stays
        /// fixed for the entry's life. Native creation does not mirror the row selector here.
        descriptor_selector: u32,
    }
    type CreatedSlots = [Option<Created>; ALL_EXTRA_SLOTS.len()];
    static CREATED: Mutex<CreatedSlots> = Mutex::new([None; ALL_EXTRA_SLOTS.len()]);

    /// A poisoned table reads as empty, so every ownership check fails closed.
    fn created() -> CreatedSlots {
        CREATED.lock().map(|created| *created).unwrap_or([None; ALL_EXTRA_SLOTS.len()])
    }

    fn register(index: usize, record: Created) -> bool {
        let Ok(mut created) = CREATED.lock() else {
            return false;
        };
        created[index] = Some(record);
        true
    }

    /// Native owner cleanup only; never scene departure.
    pub(super) fn unregister(index: usize) {
        if let Ok(mut created) = CREATED.lock() {
            created[index] = None;
        }
    }

    /// Registered creations as the one value Mii owner checks may accept. An
    /// inconsistent registry (order or claim/owner) yields no extras, so any live
    /// created entry stays unknown and every Mii shape check refuses it.
    #[cfg_attr(not(all(feature = "mii-saved-alpha",
        any(feature = "mii-ordinary-player-alpha", feature = "mii-ordinary-cpu-alpha"))), allow(dead_code))]
    pub(crate) fn owned_extras() -> crate::engine::preload::cycle_preload_capacity::Extras {
        use crate::engine::mii::capacity::{Extras, Roots};
        let created = created();
        let Some(layout) = created.iter().flatten().next().map(|record| record.layout) else {
            return Extras::NONE;
        };
        if registered_prefix(created, layout).is_none() { return Extras::NONE; }
        Extras::registered_for(layout.backend(), layout.extra_slots().map(|slot|
            registered_at(created, slot).map(|record| Roots {
                entry: record.entry, resource: record.resource, alias: record.alias,
                selection: record.selection, descriptor_selector: record.descriptor_selector,
            })))
            .unwrap_or(Extras::NONE)
    }

    /// A physical identity survives its scene until native retirement. Do not
    /// reinterpret an old claim using the next scene's creation order.
    fn registered_prefix(created: CreatedSlots, layout: Layout) -> Option<usize> {
        if created.iter().flatten().any(|record| record.layout != layout) { return None; }
        let order = layout.extra_slots();
        if created.iter().enumerate().any(|(index, record)|
            record.is_some() && !order.contains(&ALL_EXTRA_SLOTS[index])) { return None; }
        let count = order.iter().take_while(|slot| registered_at(created, **slot).is_some()).count();
        if order[count..].iter().any(|slot| registered_at(created, *slot).is_some()) { return None; }
        created.iter().enumerate().all(|(index, record)| record.is_none_or(|record|
            claim_state(index) == (Claimant::Managed as u8, record.resource))).then_some(count)
    }

    fn registered_at(created: CreatedSlots, owner: usize) -> Option<Created> {
        claim_index(owner).and_then(|index| created[index])
    }

    fn roots(scene: &Snapshot, owner: usize) -> (usize, usize, usize, u8) {
        (scene.primary[owner], scene.resources[owner], scene.aliases[owner], scene.selection[owner])
    }

    fn created_roots(record: Created) -> (usize, usize, usize, u8) {
        (record.entry, record.resource, record.alias, record.selection)
    }

    unsafe fn bound_resource(slot: usize) -> usize {
        // Same root as Snapshot::resources; used only to bind retirement when the
        // post-call snapshot itself is unavailable.
        let context = read::<*const u8>(BASE.load(Ordering::Acquire) as *const u8, 0x5324680);
        if aligned(context) { read(context, slot * 8) } else { 0 }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct Preflight {
        scene: Snapshot,
        input: Inputs,
        resources: [(u32, u32); 4],
        #[cfg(feature = "managed-grouped-source-alpha")]
        previous: Option<u32>,
        #[cfg(feature = "managed-grouped-source-alpha")]
        visible: [Option<canonical::VisibleBank>; 2],
        #[cfg(feature = "managed-visible-grouped-source-alpha")]
        initial_visible: [Option<canonical::CreationVisible>; 2],
        #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
        parked_groups: [Option<ParkedGroup>; 2],
        /// lobby-preload A: each visible owner's proven exact Mii (index 0 Player, 1 CPU), or None.
        /// Part of the stability equality and of `create_reserved`'s pre-call recheck.
        #[cfg(feature = "mii-ordinary-cpu-alpha")]
        visible_mii: [Option<VisibleMii>; 2],
    }

    // Temporary experiment evidence, captured only by pending creation/load work.
    // Actor pointers are compared as numbers, never decoded as ordinary actors.
    #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct ParkedGroup {
        descriptor: [u32; 28],
        form: [u8; 5],
        bodies: [(usize, u32, u32, u64, u8, usize); 3],
        actors: [usize; 5],
    }

    #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
    unsafe fn parked_groups(scene: Snapshot) -> Option<[Option<ParkedGroup>; 2]> {
        let mut groups = [None; 2];
        if !cfg!(feature = "managed-grouped-source-alpha") && matches!(scene.layout, Layout::Training) {
            return Some(groups);
        }
        for (index, output) in groups.iter_mut().enumerate() {
            let owner = scene.layout.owners()[index + 2];
            let observed = scene.original[index + 2];
            if observed.count == 1 {
                continue;
            }
            let entry = native_source::entry(owner)?;
            let kind = observed.kinds[0];
            let forms = super::super::Forms::starting(kind);
            if scene.primary[owner as usize] != entry as usize
                || !(2..=3).contains(&forms.count)
                || observed.count != forms.count
                || observed.kinds != forms.kinds
                || observed.installed != u64::from(forms.count)
                || read::<u8>(entry, 0x591f) != forms.flags
                || native_source::ready(entry, owner, kind).is_none()
            {
                return None;
            }
            let base = BASE.load(Ordering::Acquire);
            let mut bodies = [(0, 0, 0, 0, 0, 0); 3];
            for (index, body_fields) in bodies.iter_mut().enumerate().take(forms.count as usize) {
                let body = read::<*const u8>(entry, 0x4160 + index * 8);
                if !aligned(body) || read::<usize>(body, 0) != base + 0x4f80210 {
                    return None;
                }
                *body_fields = (
                    body as usize,
                    read(body, 8),
                    read(body, 0xc),
                    read(body, 0x10),
                    read(body, 0x3a),
                    read(body, 0x20),
                );
            }
            let group = ParkedGroup {
                descriptor: descriptor(entry),
                form: [0x5918, 0x5919, 0x591a, 0x591f, 0x5934].map(|offset| read(entry, offset)),
                bodies,
                actors: [0x4178, 0x4180, 0x4188, 0x4190, 0x4198].map(|offset| read(entry, offset)),
            };
            if native_source::entry(owner) != Some(entry)
                || entry_observation(entry, owner) != Some(observed)
            {
                return None;
            }
            *output = Some(group);
        }
        Some(groups)
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(crate) struct Reservation {
        /// Created native entry: 7, or 6/5 for the extended Training candidate.
        pub(crate) slot: u32,
        pub(crate) entry: usize,
        pub(crate) resource: usize,
        pub(crate) entry_destroyed: bool,
        pub(crate) owner_retired: bool,
        scene: Snapshot,
        input: Inputs,
        descriptor: [u32; 28],
    }

    #[derive(Clone, Copy, Debug)]
    pub(crate) struct ActorCleanup {
        pub(crate) actor: usize,
        #[cfg(feature = "managed-grouped-source-alpha")]
        pub(crate) index: usize,
        #[cfg(feature = "managed-grouped-source-alpha")]
        pub(crate) record: Option<canonical::Actor>,
        pub(crate) kind: u32,
        pub(crate) identity: bool,
        pub(crate) resource_associated: bool,
        pub(crate) returned: bool,
        pub(in crate::runtime::roster::character_roster) input: Result<Inputs, (&'static str, usize, usize)>,
        pub(crate) valid: bool,
    }

    /// Called only by the native ordinary actor destructor, before entry removal.
    /// The current entry still owns +4178. No saved pointer is dereferenced.
    #[cfg(not(feature = "managed-grouped-source-alpha"))]
    pub(crate) unsafe fn actor_cleanup_observation(
        ctx: &InlineCtx,
        reservation: Reservation,
    ) -> Option<ActorCleanup> {
        if reservation.entry_destroyed || reservation.owner_retired {
            return None;
        }
        let slot = reservation.slot;
        let current = native_source::entry(slot)?;
        let actor = ctx.registers[20].x() as *const u8;
        if current as usize != reservation.entry
            || !aligned(actor)
            || read::<usize>(current, 0x4178) != actor as usize
        {
            return None;
        }
        // The current entry owns this actor. Report the independent resource
        // association even if native teardown has already changed its alias.
        let resource_associated = native_source::resource_owner(slot).map(|p| p as usize)
            == Some(reservation.resource);
        let base = BASE.load(Ordering::Acquire);
        let kind = read::<u32>(actor, 0x74);
        let identity = read::<usize>(actor, 0) == base + 0x4f85d30
            && read::<u32>(actor, 8) == slot
            && read::<u32>(actor, 0x6c) == slot;
        let returned = ctx.registers[30].x() as usize == base + 0x6b79a0;
        let input = inputs_for(slot as usize);
        let valid = identity
            && resource_associated
            && returned
            && input.is_ok_and(|input| {
                input.unclaimed()
                    && input.service == reservation.input.service
                    && input.data == reservation.input.data
                    && input.records == reservation.input.records
            });
        Some(ActorCleanup {
            actor: actor as usize,
            kind,
            identity,
            resource_associated,
            returned,
            input,
            valid,
        })
    }

    #[cfg(feature = "managed-grouped-source-alpha")]
    pub(crate) unsafe fn actor_cleanup_observation(
        ctx: &InlineCtx, reservation: Reservation, kind: u32, expected: Option<Actors>, first: Option<Actor>,
    ) -> Option<ActorCleanup> {
        if reservation.entry_destroyed || reservation.owner_retired { return None; }
        let slot = reservation.slot;
        let current = native_source::entry(slot)?;
        if current as usize != reservation.entry { return None; }
        let (index, record, identity) =
            canonical::cleanup(ctx, current, slot as usize, kind, expected, first)?;
        let resource_associated = native_source::resource_owner(slot).map(|p| p as usize)
            == Some(reservation.resource);
        let returned = ctx.registers[30].x() as usize == BASE.load(Ordering::Acquire) + 0x6b79a0;
        let input = inputs_for(slot as usize);
        let route = slot as usize;
        let valid = identity && resource_associated && returned && input.is_ok_and(|input|
            input.unclaimed() && input.service == reservation.input.service
                && input.data == reservation.input.data && input.records == reservation.input.records
                && input.slots[route].state == 2 && input.slots[route + 8].state == 2);
        Some(ActorCleanup { actor: ctx.registers[20].x() as usize, index, record,
            kind: record.map_or(u32::MAX, |actor| actor.kind), identity,
            resource_associated, returned, input, valid })
    }

    /// Current service root at the native entry destructor/removal boundary.
    pub(crate) unsafe fn listener_count(reservation: Reservation) -> Option<u64> {
        let service = read::<*const u8>(BASE.load(Ordering::Acquire) as *const u8, 0x52c50f8);
        if !aligned(service) || service as usize != reservation.scene.service {
            return None;
        }
        let inner = read::<*const u8>(service, 0);
        (aligned(inner) && inner as usize == reservation.scene.service_inner)
            .then(|| read::<u64>(inner, 0x930))
    }

    pub(crate) unsafe fn listener_after(ctx: &InlineCtx, reservation: Reservation) -> Option<u64> {
        if ctx.registers[19].x() as usize != reservation.entry
            || ctx.registers[21].x() as usize != reservation.scene.service
        {
            return None;
        }
        listener_count(reservation)
    }

    /// The opt-in path accepts catalog sources with a proven native actor plan.
    /// It does not change the selectable catalog or coalesce form resource keys.
    pub(crate) fn source_kind(kind: u32) -> bool {
        #[cfg(feature = "managed-grouped-source-alpha")]
        { canonical::supported(kind) }
        #[cfg(not(feature = "managed-grouped-source-alpha"))]
        { native_source::ordinary_kind(kind) }
    }

    #[cfg(feature = "managed-grouped-source-alpha")]
    pub(crate) use super::canonical::{Actor, Actors};

    #[cfg(feature = "managed-grouped-source-alpha")]
    pub(crate) unsafe fn actors_at(entry: *const u8, slot: u32, kind: u32) -> Option<Actors> {
        canonical::ready(entry, slot as usize, kind)
    }

    pub(crate) fn enabled() -> bool {
        ENABLED.load(Ordering::Acquire)
    }

    unsafe fn training() -> bool {
        enabled()
            && crate::runtime::roster::character_lifecycle::native_hooks_ready()
            && common::is_native_training_mode()
            && matches!(common::game_mode::current(), Some(mode)
                if mode.initialized && mode.melee == 0x20 && mode.mode == 0x04040000)
    }

    unsafe fn layout_permitted(layout: Layout) -> bool {
        match layout {
            Layout::Training => training(),
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Layout::LdnWaiting => cfg!(feature = "managed-ldn-third-source-alpha")
                && enabled() && crate::runtime::roster::character_lifecycle::native_hooks_ready()
                && matches!(common::game_mode::current(), Some(mode)
                    if mode.initialized && mode.melee == 0x0d && mode.mode == 0x02010000)
                && super::super::lobby::active(),
            #[cfg(feature = "lobby-preload-extended")]
            Layout::QuickplayWaiting => enabled() && crate::runtime::roster::character_lifecycle::native_hooks_ready()
                && matches!(common::game_mode::current(), Some(mode)
                    if mode.initialized && mode.melee == 0x0d && Backend::quickplay_waiting_mode(mode.mode))
                && super::super::lobby::active(),
        }
    }

    unsafe fn current_layout() -> Option<Layout> {
        if training() { return Some(Layout::Training); }
        #[cfg(feature = "managed-ldn-third-source-alpha")]
        if layout_permitted(Layout::LdnWaiting) { return Some(Layout::LdnWaiting); }
        #[cfg(feature = "lobby-preload-extended")]
        if layout_permitted(Layout::QuickplayWaiting) { return Some(Layout::QuickplayWaiting); }
        None
    }

    unsafe fn current_snapshot(
        layout: Layout, ctx: &ServiceEntryContext, entry: *const u8, slot: u32,
    ) -> Option<Snapshot> {
        // Preserve the legacy Training wrapper, including its mandatory quantity.
        if layout == Layout::Training { snapshot(ctx, entry, slot) }
        else { snapshot_for(layout, ctx, entry, slot) }
    }

    /// The witnessed solo waiting rows, extended only by exact registered owners.
    /// Active consumers are allowed; their source leases prevent replacement.
    #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
    unsafe fn waiting_creation_shape(scene: Snapshot, created: CreatedSlots) -> bool {
        if scene.quantity.is_some() { return false; }
        let Some(fields) = scene.ldn else { return false; };
        if fields.resources.is_none()
            || fields.selection != [(0,0,0), (2,0x64,1), (2,0x64,2), (2,0x64,3),
                (1,1,4), (1,1,5), (1,1,6), (2,0x64,7)] { return false; }
        // Never take an occupied local seat; admitted extras are the only
        // difference from the exact solo waiting shape captured at creation.
        (0..8).filter(|slot| !scene.layout.owners().contains(&(*slot as u32))).all(|slot|
            match registered_at(created,slot) {
                Some(record) => roots(&scene,slot)==created_roots(record),
                None => scene.primary[slot]==0 && scene.resources[slot]==0 && scene.aliases[slot]==0,
            })
    }

    fn waiting_input(input: Inputs) -> bool {
        input.unclaimed() && input.map7 == -1
            && input.controller == BASE.load(Ordering::Acquire) + 0x5339830
            && input.controller_flag == 0 && input.css_role == 0 && input.cpu_flag == 1
            && [input.slot,input.slot+8].iter().all(|i| input.slots[*i].state == 2)
    }

    unsafe fn descriptor(entry: *const u8) -> [u32; 28] {
        std::array::from_fn(|i| read(entry, i * 4))
    }

    /// Reset's native all-entry loop writes borrowed configuration references
    /// before its visible-quantity cutoff. Derive them from current original
    /// entries, never from retained pointers or by dereferencing either value.
    unsafe fn training_reset_references(
        ctx: &ServiceEntryContext,
        current: *const u8,
        reservation: Reservation,
    ) -> Option<[[u64; 2]; 4]> {
        if reservation.scene.layout != Layout::Training {
            return None;
        }
        let scene = current_snapshot(Layout::Training, ctx, current, reservation.slot)?;
        let mut references = [[0; 2]; 4];
        for (owner, references) in references.iter_mut().enumerate() {
            let original = native_source::entry(owner as u32)?;
            if original as usize != scene.primary[owner]
                || original as usize != reservation.scene.primary[owner]
            {
                return None;
            }
            *references = [read(original, 0x58), read(original, 0x60)];
        }
        // Revalidate roots and the callback after reads. Only numeric identities
        // are retained; neither borrowed configuration pointer is followed.
        (owned(reservation, current)
            && current_snapshot(Layout::Training, ctx, current, reservation.slot) == Some(scene))
            .then_some(references)
    }

    unsafe fn empty(entry: *const u8, slot: usize, resource: *const u8) -> bool {
        entry_observation(entry, slot as u32).is_some_and(|entry| {
            entry.count == 1
                && entry.kinds == [u32::MAX; 3]
                && entry.installed == 0
                && entry.phase == [0, 14, 0]
        }) && read::<u32>(entry, 0x28) == 1
            && aligned(resource)
            && read::<usize>(resource, 0) == BASE.load(Ordering::Acquire) + 0x509fb10
            && read::<u32>(resource, 0x58) == u32::MAX
    }

    /// The admission shapes compiled into this build, for the value rule in consts.
    const COMPILED: creation::Compiled = creation::Compiled {
        grouped_helper: cfg!(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha")),
        managed_grouped: cfg!(feature = "managed-grouped-source-alpha"),
        visible_grouped: cfg!(feature = "managed-visible-grouped-source-alpha"),
    };

    /// One layout owner as the preflight read it, for `creation::owner_admission`.
    fn owner_facts(index: usize, count: u32, installed: u64, kind: u32, exact_mii: ExactMiiProof) -> OwnerFacts {
        OwnerFacts {
            index, count, installed, kind,
            ordinary: native_source::ordinary_kind(kind),
            starting_forms: super::super::Forms::starting(kind).count,
            exact_mii,
        }
    }

    /// One already-present layout owner while an extra is created (lobby-preload A: the value
    /// rule is `crate::engine::swap::capacity_creation::owner_admission`, tested on the host).
    /// Count-1 exact Miis, helpers (owners-array index 2/3: Training 2/3, waiting 5/6) and visible
    /// owners (index 0/1) alike, are admitted only with a proven exact payload. Grouped and
    /// native payload guards are unchanged.
    fn owner_admission(layout: Layout, facts: OwnerFacts) -> OwnerAdmission {
        creation::owner_admission(matches!(layout, Layout::Training), facts, COMPILED)
    }

    /// lobby-preload A: a visible count-1 Mii (owners index 0 Player, 1 CPU) as the creation
    /// preflight proved it. The owner, rederived now under the native pool lock, is exactly the
    /// authority the ordinary Mii adapter committed for it (`role_owner::Committed::permits`);
    /// the witness adds its descriptor and selected record so `visible_mii_kept` can show that
    /// native creation left them unchanged. Reads only.
    #[cfg(feature = "mii-ordinary-cpu-alpha")]
    unsafe fn exact_visible_mii(index: usize, current: *const u8, kind: u32) -> (ExactMiiProof, Option<VisibleMii>) {
        use super::super::mii_ordinary_player::native::helpers;
        let proof = if index == 0 {
            super::super::mii_ordinary_player::visible_exact(current, kind)
        } else {
            super::super::mii_ordinary_cpu::visible_exact(current, kind)
        };
        match proof {
            Ok(witness) => (ExactMiiProof::Proven, Some(witness)),
            Err(reason) if reason == helpers::BUSY || reason == helpers::CAPTURE_BUSY => (ExactMiiProof::Busy, None),
            Err(_) => (ExactMiiProof::Refused, None),
        }
    }

    /// The per-owner exact-Mii proof: visible owners through the committed authority, helpers
    /// through `ready_checked`. Not a count-1 installed Mii: not applicable, no native lock.
    unsafe fn exact_mii_owner(
        index: usize, owner: u32, current: *const u8, count: u32, installed: u64, kind: u32,
    ) -> (ExactMiiProof, Option<VisibleMii>) {
        if index >= 2 { return (exact_mii_for_expansion(index, owner, count, installed, kind), None); }
        if count != 1 || installed != 1
            || crate::engine::mii::inventory::MiiClass::from_kind(kind).is_none()
        {
            return (ExactMiiProof::NotApplicable, None);
        }
        #[cfg(feature = "mii-ordinary-cpu-alpha")]
        { exact_visible_mii(index, current, kind) }
        #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
        { let _ = current; (ExactMiiProof::NotApplicable, None) }
    }

    /// lobby-preload A, after a qualified creation: each visible exact Mii the preflight proved,
    /// proved again. `Some(true)`: the same committed authority, descriptor and record; `Some(false)`:
    /// native creation changed or removed it (the R-LP-1 safety failure); `None`: no visible Mii.
    pub(crate) unsafe fn visible_mii_kept(before: &Preflight) -> [Option<bool>; 2] {
        let mut kept = [None; 2];
        #[cfg(feature = "mii-ordinary-cpu-alpha")]
        for (index, witness) in before.visible_mii.iter().enumerate() {
            let Some(witness) = witness else { continue; };
            let owner = before.scene.layout.owners()[index];
            // An absent entry (null) refuses the proof: the witness is then not kept.
            let current = native_source::entry(owner).unwrap_or_default();
            let (_, after) = exact_mii_owner(index, owner, current, 1, 1, before.scene.original[index].kinds[0]);
            kept[index] = Some(after == Some(*witness));
        }
        #[cfg(not(feature = "mii-ordinary-cpu-alpha"))]
        let _ = before;
        kept
    }

    /// lobby-preload B, after a qualified creation: each visible body-bank window the preflight
    /// admitted by the committed body-bank rule, captured again from a fresh scene with the same
    /// rule (the bank's committed window re-read under its lock). `Some(true)`: the same window,
    /// members, bodies, actors, routes, resource words and descriptor; `Some(false)`: native creation
    /// changed it or the bank no longer commits it (the R-LP-B1 safety failure); `None`: no such window.
    pub(crate) unsafe fn visible_bank_kept(
        ctx: &ServiceEntryContext, entry: *const u8, slot: u32, before: &Preflight,
    ) -> [Option<bool>; 2] {
        let mut kept = [None; 2];
        #[cfg(feature = "managed-visible-grouped-source-alpha")]
        {
            let after = current_snapshot(before.scene.layout, ctx, entry, slot)
                .and_then(|after| canonical::creation_visible_checked(after, before.input).ok());
            for (index, captured) in before.initial_visible.iter().enumerate() {
                let Some(captured) = captured.filter(|captured| captured.body_bank()) else { continue; };
                kept[index] = Some(after.is_some_and(|after| after[index] == Some(captured)));
            }
        }
        #[cfg(not(feature = "managed-visible-grouped-source-alpha"))]
        let _ = (ctx, entry, slot, before);
        kept
    }

    /// Ask the shared helper proof only for a count-1 Mii base helper.
    /// Visible slots, ordinary kinds and other shapes do not take the native lock.
    #[cfg(feature = "mii-managed-helper-alpha")]
    unsafe fn exact_owned_mii_helper(slot: u32, kind: u32) -> ExactMiiProof {
        use super::super::mii_ordinary_player::native::{helper_snapshot, helpers};
        let view = match helper_snapshot() {
            Ok(view) => view,
            Err(reason) if reason == helpers::BUSY || reason == helpers::CAPTURE_BUSY => {
                return ExactMiiProof::Busy;
            }
            Err(_) => return ExactMiiProof::Refused,
        };
        let Some(ordinal) = view.ordinal(slot as usize) else {
            return ExactMiiProof::Refused;
        };
        let helpers::Helper::Ready { ready, key, .. } = view.slots[ordinal] else {
            return ExactMiiProof::Refused;
        };
        let Some(mii) = view.miis[ordinal] else {
            return ExactMiiProof::Refused;
        };
        if key.kind != kind || mii.configuration.canonical().class as u32 != kind {
            return ExactMiiProof::Refused;
        }
        match super::super::mii_managed_helper::ready_checked(ready, mii.configuration) {
            Ok(()) => ExactMiiProof::Proven,
            Err(reason) if reason == helpers::BUSY || reason == helpers::CAPTURE_BUSY => {
                ExactMiiProof::Busy
            }
            Err(_) => ExactMiiProof::Refused,
        }
    }

    unsafe fn exact_mii_for_expansion(
        index: usize, slot: u32, count: u32, installed: u64, kind: u32,
    ) -> ExactMiiProof {
        if index < 2 || count != 1 || installed != 1
            || crate::engine::mii::inventory::MiiClass::from_kind(kind).is_none()
        {
            return ExactMiiProof::NotApplicable;
        }
        #[cfg(feature = "mii-managed-helper-alpha")]
        {
            exact_owned_mii_helper(slot, kind)
        }
        #[cfg(not(feature = "mii-managed-helper-alpha"))]
        {
            let _ = slot;
            ExactMiiProof::NotApplicable
        }
    }

    /// Creation preflight for `extra` from helper callback `slot`. Entry 7 keeps
    /// the exact count4 shape. Training-only 6 then 5 additionally require exactly
    /// the earlier creations, registered and unchanged; count is 4 plus those.
    /// `Preflight.input.slot` carries `extra` to `create_reserved`.
    pub(crate) unsafe fn preflight_for(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        slot: u32,
        extra: u32,
    ) -> Option<Preflight> {
        preflight_checked(ctx, entry, slot, extra).ok()
    }

    /// `preflight_for` naming the first check that refused (lobby-preload D: the creation-waiting
    /// line). The checks, their order, their native reads and the admitted value are unchanged.
    pub(crate) unsafe fn preflight_checked(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        slot: u32,
        extra: u32,
    ) -> Result<Preflight, Refusal> {
        let layout = current_layout().ok_or(Refusal::Layout)?;
        let owners = layout.owners();
        let target = extra as usize;
        let earlier = layout.extra_slots().iter().position(|slot| *slot == target).ok_or(Refusal::Extra)?;
        if !extra_compiled(target) || (layout.waiting() && target != SLOT && !cfg!(feature = "lobby-preload-extended")) {
            return Err(Refusal::Extra);
        }
        if claimed_at(target) { return Err(Refusal::Claimed); }
        if !owners[2..].contains(&slot) { return Err(Refusal::Callback); }
        let created = created();
        if registered_prefix(created,layout).ok_or(Refusal::Order)? != earlier { return Err(Refusal::Order); }
        let scene = current_snapshot(layout, ctx, entry, slot).ok_or(Refusal::Snapshot)?;
        let shape = match layout {
            // Every slot 4..8 is null except an earlier registered creation, which
            // must still have exactly its registered roots and selection byte.
            Layout::Training => scene.quantity == Some(1)
                && (4..8).all(|owner| match registered_at(created, owner) {
                    Some(record) => roots(&scene, owner) == created_roots(record),
                    None => scene.primary[owner] == 0,
                }),
            #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
            Layout::LdnWaiting => waiting_creation_shape(scene,created),
            #[cfg(feature = "lobby-preload-extended")]
            Layout::QuickplayWaiting => waiting_creation_shape(scene,created),
        };
        if scene.count != 4 + earlier as u32 || !shape { return Err(Refusal::Shape); }
        if scene.backup.iter().any(|p| *p != 0) { return Err(Refusal::Backup); }
        if scene.resources[target] != 0
            || scene.aliases[target] != 0
            || scene.selection[target] != 2
        {
            return Err(Refusal::Occupied);
        }
        #[cfg(feature = "mii-ordinary-cpu-alpha")]
        let mut visible_mii = [None; 2];
        for (index, owner) in owners.iter().copied().enumerate() {
            let at = index as u8;
            let current = scene.primary[owner as usize] as *const u8;
            let observed = scene.original[index];
            // Busy clears this callback's preflight. The existing creation
            // counter repeats it; this is not a new retry loop.
            let (exact_mii, _witness) = exact_mii_owner(
                index, owner, current, observed.count, observed.installed, observed.kinds[0]);
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            if index < 2 { visible_mii[index] = _witness; }
            let admission = owner_admission(layout, owner_facts(
                index, observed.count, observed.installed, observed.kinds[0], exact_mii));
            #[cfg(feature = "native-body-bank-alpha")]
            let admission = if index < 2 && super::super::body_bank::ready_members(current, owner).is_some() {
                // A private bank is admitted only by its live owned token and
                // complete bodies. All actor/input/resource checks below still
                // apply; value-only canonical OwnerFacts remains unchanged.
                OwnerAdmission { admits: true, grouped_visible: true }
            } else { admission };
            if !admission.admits {
                return Err(Refusal::Owner {
                    index: at, count: observed.count, kind: observed.kinds[0], proof: exact_mii.name(),
                });
            }
            if scene.aliases[owner as usize] != scene.resources[owner as usize] {
                return Err(Refusal::Alias { index: at });
            }
            if index < 2 {
                if !super::super::ready_forms(current, owner)
                    .is_some_and(|(forms, _)| admission.grouped_visible
                        || (forms.count == 1 && forms.actor_count == 1))
                {
                    return Err(Refusal::Forms { index: at });
                }
            } else if !native_source::parked_ready(current, owner, observed.kinds[0]) {
                return Err(Refusal::Parked { index: at });
            }
            #[cfg(feature = "managed-ldn-third-source-alpha")]
            if layout.waiting() && index >= 2
                && native_source::ready(current, owner, observed.kinds[0]).is_none()
            { return Err(Refusal::HelperReady { index: at }); }
        }
        let resources = original_resources(scene).ok_or(Refusal::Resources)?;
        if let Some(index) = resources
            .iter()
            .enumerate()
            .position(|(i, resource)| resource.0 != scene.original[i].kinds[0])
        {
            return Err(Refusal::ResourceKind { index: index as u8 });
        }
        // Records `target`/`target + 8` are the two the native constructor clears.
        let input = inputs_for(target).map_err(|_| Refusal::Input)?;
        if layout.waiting() && !waiting_input(input) { return Err(Refusal::Input); }
        #[cfg(feature = "managed-grouped-source-alpha")]
        if !canonical::preflight_input(input, None) { return Err(Refusal::Input); }
        // The captures run before the unclaimed check, as they did inside `then_some`.
        let preflight = Preflight {
            scene,
            input,
            resources,
            #[cfg(feature = "managed-grouped-source-alpha")]
            previous: None,
            #[cfg(feature = "managed-grouped-source-alpha")]
            visible: [None; 2], // Load-time switching authority is captured at each install.
            #[cfg(feature = "managed-visible-grouped-source-alpha")]
            initial_visible: canonical::creation_visible_checked(scene, input)
                .map_err(|(index, check)| Refusal::Visible { index, check })?,
            #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
            parked_groups: parked_groups(scene).ok_or(Refusal::ParkedGroups)?,
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            visible_mii,
        };
        if input.unclaimed() { Ok(preflight) } else { Err(Refusal::Unclaimed) }
    }

    /// Synchronous creation only. Caller consumes its epoch intent and releases
    /// its mutex before this function, while retaining the shared INSTALL gate.
    pub(crate) unsafe fn create_reserved(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        slot: u32,
        before: Preflight,
    ) -> Option<Reservation> {
        let target = before.input.slot;
        if preflight_for(ctx, entry, slot, target as u32) != Some(before)
            || !create_at(Claimant::Managed, target)
        {
            return None;
        }
        let after = current_snapshot(before.scene.layout, ctx, entry, slot);
        // Bind native retirement even when a subsequent observation declines,
        // including an unavailable snapshot (same root, read once).
        bind_owner_at(Claimant::Managed, target,
            after.map_or_else(|| bound_resource(target), |after| after.resources[target]));
        let after = after?;
        if !creation_delta_at(before.scene, after, target)
            || after.aliases[target] != after.resources[target]
            || original_resources(after) != Some(before.resources)
            || inputs_for(target).ok() != Some(before.input)
            || !empty(
                after.primary[target] as *const u8,
                target,
                after.resources[target] as *const u8,
            )
        {
            return None;
        }
        #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
        {
            let current_groups = parked_groups(after);
            let preserved = current_groups == Some(before.parked_groups);
            crate::engine::compat::logging::diagnostic(format_args!(
                "[GroupedHelperCapacity] creation layout={:?} entry=0x{:x} preserved={} before={:x?} after={:x?}",
                after.layout, after.primary[target], preserved, before.parked_groups, current_groups));
            if !preserved {
                return None;
            }
        }
        #[cfg(feature = "managed-visible-grouped-source-alpha")]
        {
            // Native creation is synchronous: permit no original body, actor,
            // control, role, form or descriptor change across this call.
            let current = canonical::creation_visible(after, before.input);
            let preserved = current == Some(before.initial_visible);
            if before.initial_visible.iter().any(Option::is_some) {
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[GroupedVisibleCapacity] creation layout={:?} entry=0x{:x} preserved={} before={:x?} after={:x?}",
                    after.layout, after.primary[target], preserved, before.initial_visible, current));
            }
            if !preserved { return None; }
        }
        let reservation = Reservation {
            slot: target as u32,
            entry: after.primary[target],
            resource: after.resources[target],
            entry_destroyed: false,
            owner_retired: false,
            scene: after,
            input: before.input,
            descriptor: descriptor(after.primary[target] as *const u8),
        };
        // Only a fully qualified creation becomes tolerable by other reservations.
        let index = claim_index(target)?;
        register(index, Created {
            layout: after.layout,
            entry: reservation.entry,
            resource: reservation.resource,
            alias: after.aliases[target],
            selection: after.selection[target],
            descriptor_selector: reservation.descriptor[1],
        }).then_some(reservation)
    }

    fn same_input_role(current: Inputs, before: Inputs) -> bool {
        // Other actors may legitimately change their own key after a hotswap.
        // Each individual install additionally compares the complete input view.
        let mut expected = before;
        expected.slots = current.slots;
        current == expected && current.unclaimed()
    }

    /// Reacquire all roots; retained addresses are equality values only.
    pub(crate) unsafe fn owned(reservation: Reservation, current: *const u8) -> bool {
        owned_checked(reservation, current).is_ok()
    }

    /// Every root equals this reservation's creation snapshot, except another
    /// slot holding a registered creation, which must equal its own registered
    /// roots. Count is 4 plus the registered prefix; nothing else may differ.
    unsafe fn owned_checked(reservation: Reservation, current: *const u8) -> Result<(), (&'static str, usize)> {
        let target = reservation.slot as usize;
        if !layout_permitted(reservation.scene.layout)
            || reservation.entry_destroyed
            || reservation.owner_retired
            || current as usize != reservation.entry
            || native_source::entry(reservation.slot) != Some(current)
        {
            return Err(("scene/lifetime/current-entry", target));
        }
        let created = created();
        let Some(registered) = registered_prefix(created,reservation.scene.layout) else {
            return Err(("created registry order or claims", target));
        };
        if registered_at(created, target).is_none_or(|record|
            record.entry != reservation.entry || record.resource != reservation.resource)
        {
            return Err(("reservation not registered for this layout", target));
        }
        let base = BASE.load(Ordering::Acquire);
        let manager = read::<*const u8>(base as *const u8, 0x52b94f8);
        let context = read::<*const u8>(base as *const u8, 0x5324680);
        let service = read::<*const u8>(base as *const u8, 0x52c50f8);
        if !aligned(manager)
            || manager as usize != reservation.scene.manager
            || !aligned(context)
            || context as usize != reservation.scene.context
            || !aligned(service)
            || service as usize != reservation.scene.service
        {
            return Err(("manager/context/service identity", target));
        }
        let internal = read::<*const u8>(manager, 0);
        if !aligned(internal)
            || internal as usize != reservation.scene.internal
            || read::<usize>(service, 0) != reservation.scene.service_inner
            || read::<u32>(internal, 0xa0) != 4 + registered as u32
            || (reservation.scene.layout == Layout::Training
                && native_source::training_cpu_quantity() != Some(1))
            || [read::<u8>(internal, 0xea), read(internal, 0xeb)] != reservation.scene.rules
            || read::<u32>(internal, 0xf0) != reservation.scene.ratio_bits
        {
            return Err(("internal/count/quantity/rules/ratio", target));
        }
        for owner in 0..8 {
            let expected = match registered_at(created, owner).filter(|_| owner != target) {
                Some(record) => created_roots(record),
                None => roots(&reservation.scene, owner),
            };
            let observed = (
                read::<usize>(internal, 0x20 + owner * 8),
                read::<usize>(context, owner * 8),
                read::<usize>(context, 0xe8 + owner * 8),
                read::<u8>((base + 0x52c5730 + owner * 0x260) as *const u8, 0xa),
            );
            if observed != expected || read::<usize>(internal, 0x60 + owner * 8) != 0 {
                return Err(("original/owned roots, aliases or selection", owner));
            }
        }
        #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
        if reservation.scene.layout.waiting() {
            let Some(fields) = reservation.scene.ldn else { return Err(("LDN snapshot unavailable", target)); };
            for owner in 0..8 {
                let row = (base + 0x52c5730 + owner * 0x260) as *const u8;
                if (read::<u8>(row, 0xa), read::<u8>(row, 0x79), read::<u8>(row, 0xa7))
                    != fields.selection[owner]
                { return Err(("LDN selection tuple", owner)); }
            }
        }
        let input = inputs_for(target).map_err(|_| ("input view unavailable", target))?;
        if !same_input_role(input, reservation.input) {
            return Err(("input service/role identity", target));
        }
        Ok(())
    }

    /// Only this current managed LDN reservation may admit helper5/6 at count5.
    /// The strict count4 predicate remains unchanged for every unowned scene.
    #[cfg(feature = "managed-ldn-third-source-alpha")]
    pub(crate) unsafe fn owned_ldn_helper(
        reservation: Reservation, current: *const u8, slot: u32,
    ) -> bool {
        if !reservation.scene.layout.waiting() || !matches!(slot, 5 | 6)
            || reservation.scene.primary[slot as usize] != current as usize
            || native_source::entry(slot) != Some(current)
        { return false; }
        let Some(candidate) = native_source::entry(reservation.slot) else { return false; };
        owned(reservation, candidate)
    }

    /// The same ordered admission checks, with a value-only decline reason.
    /// The ordinary wrapper still returns Option; reasons never authorize work.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum LoadDecline {
        Owned,
        Descriptor,
        PreviousKind,
        Parked,
        Resource,
        Empty,
        Available,
        #[cfg(feature = "managed-grouped-source-alpha")]
        Canonical,
        Snapshot,
        Inputs,
        #[cfg(feature = "managed-grouped-source-alpha")]
        InputRole,
        OriginalResources,
        #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
        ParkedGroups,
    }

    pub(crate) unsafe fn load_preflight(
        ctx: &ServiceEntryContext, entry: *const u8, reservation: Reservation, previous: Option<u32>,
    ) -> Option<Preflight> {
        load_preflight_checked(ctx, entry, reservation, previous).ok()
    }

    /// Event-only supplementary reads from this live callback, never saved pointers.
    #[cfg(feature = "source-capacity-probe")]
    pub(in crate::runtime::roster::character_roster) type AdmissionDetail = (
        bool, bool, Option<(&'static str, usize)>, Option<[u32; 28]>, Option<bool>, Option<bool>,
        Result<Inputs, (&'static str, usize, usize)>,
    );

    #[cfg(feature = "source-capacity-probe")]
    pub(in crate::runtime::roster::character_roster) unsafe fn admission_detail(
        ctx: &ServiceEntryContext, entry: *const u8, previous: u32, reservation: Option<Reservation>,
    ) -> AdmissionDetail {
        let slot = reservation.map_or(SLOT as u32, |owner| owner.slot);
        let current_entry = native_source::entry(slot) == Some(entry);
        let dispatch = current_entry && dispatch_identity(ctx, entry).is_some();
        let parked = dispatch.then(|| native_source::parked_ready(entry, slot, previous));
        #[cfg(feature = "managed-grouped-source-alpha")]
        let canonical = (parked == Some(true))
            .then(|| canonical::ready(entry, slot as usize, previous).is_some());
        #[cfg(not(feature = "managed-grouped-source-alpha"))]
        let canonical = None;
        let owned_error = reservation.and_then(|owner| owned_checked(owner, entry).err());
        (current_entry, dispatch, owned_error, dispatch.then(|| descriptor(entry)),
            parked, canonical, inputs_for(slot as usize))
    }

    pub(crate) unsafe fn load_preflight_checked(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        reservation: Reservation,
        previous: Option<u32>,
    ) -> Result<Preflight, LoadDecline> {
        if !owned(reservation, entry) {
            return Err(LoadDecline::Owned);
        }
        let expected_kind = previous.unwrap_or(u32::MAX);
        let mut expected = reservation.descriptor;
        expected[0x18 / 4] = expected_kind;
        #[cfg(feature = "managed-grouped-source-alpha")]
        if let Some(kind) = previous {
            let forms = super::super::Forms::starting(kind);
            expected[0x14 / 4] = forms.count;
            expected[0x18 / 4..0x24 / 4].copy_from_slice(&forms.kinds);
            expected[0x28 / 4] = forms.actor_count;
        }
        expected[0x40 / 4] &= 0x00ff_ffff; // Native install's descriptor byte43=0.
        let current_descriptor = descriptor(entry);
        if current_descriptor != expected {
            // Creation/first load and LDN keep the exact original comparison.
            // A Training replacement may retain only the pair written by native
            // reset to every current entry; every other descriptor word is exact.
            let references =
                previous.and_then(|_| training_reset_references(ctx, entry, reservation));
            if descriptor(entry) != current_descriptor
                || !source_descriptor::matches(
                    current_descriptor,
                    expected,
                    references,
                    (BASE.load(Ordering::Acquire) + 0x52c5730) as u64,
                )
            {
                return Err(LoadDecline::Descriptor);
            }
        }
        let slot = reservation.slot;
        if let Some(kind) = previous {
            if !source_kind(kind) { return Err(LoadDecline::PreviousKind); }
            if !native_source::parked_ready(entry, slot, kind) {
                return Err(LoadDecline::Parked);
            }
        } else {
            let resource = native_source::resource_owner(slot).ok_or(LoadDecline::Resource)?;
            let available: unsafe extern "C" fn(*const u8) -> bool =
                std::mem::transmute(BASE.load(Ordering::Acquire) + 0x17e4700);
            if !empty(entry, slot as usize, resource) { return Err(LoadDecline::Empty); }
            if !available(resource) { return Err(LoadDecline::Available); }
        }
        #[cfg(feature = "managed-grouped-source-alpha")]
        if previous.is_some_and(|kind| canonical::ready(entry, slot as usize, kind).is_none()) {
            return Err(LoadDecline::Canonical);
        }
        let scene = current_snapshot(reservation.scene.layout, ctx, entry, slot)
            .ok_or(LoadDecline::Snapshot)?;
        let input = inputs_for(slot as usize).map_err(|_| LoadDecline::Inputs)?;
        #[cfg(feature = "managed-grouped-source-alpha")]
        if !canonical::preflight_input(input, previous) { return Err(LoadDecline::InputRole); }
        #[cfg(feature = "managed-grouped-source-alpha")]
        let visible = canonical::visible_preflight(scene, input);
        Ok(Preflight {
            scene,
            input,
            resources: original_resources(scene).ok_or(LoadDecline::OriginalResources)?,
            #[cfg(feature = "managed-grouped-source-alpha")]
            previous,
            #[cfg(feature = "managed-grouped-source-alpha")]
            visible,
            #[cfg(feature = "managed-visible-grouped-source-alpha")]
            initial_visible: [None; 2], // Creation-only authority; never freezes later native switches.
            #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
            parked_groups: parked_groups(scene).ok_or(LoadDecline::ParkedGroups)?,
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            visible_mii: [None; 2], // Creation-only, like `initial_visible`.
        })
    }

    pub(crate) unsafe fn loaded(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        reservation: Reservation,
        before: Preflight,
        kind: u32,
    ) -> bool {
        if !owned(reservation, entry) {
            return false;
        }
        let Some(current) = current_snapshot(reservation.scene.layout, ctx, entry, reservation.slot) else {
            return false;
        };
        #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
        if parked_groups(current) != Some(before.parked_groups) {
            return false;
        }
        let expected_input = inputs_for(reservation.slot as usize).ok()
            .and_then(|input| load_expected_input(before, kind, input));
        #[cfg(feature = "managed-grouped-source-alpha")]
        let current = canonical::comparison_view(current, before.scene, before.input, expected_input);
        let mut expected = before.scene;
        expected.listeners = current.listeners; // Native actor/body registration.
        expected == current
            && original_resources(current) == Some(before.resources)
            && expected_input.is_some()
            && ready(reservation, entry, kind)
    }

    unsafe fn load_expected_input(before: Preflight, kind: u32, input: Inputs) -> Option<Inputs> {
        #[cfg(feature = "managed-grouped-source-alpha")]
        { canonical::loaded_visible_input(before.scene, before.input, before.previous, kind, before.visible, input) }
        #[cfg(not(feature = "managed-grouped-source-alpha"))]
        { let _ = kind; (input == before.input).then_some(before.input) }
    }

    // Diagnostic-only copies of the existing load contract. Neither result
    // participates in admission, completion or native ownership release.
    #[cfg(feature = "source-capacity-probe")]
    pub(crate) unsafe fn observe_install_inputs(
        source: crate::engine::swap::source_lifetime::SourceToken,
        previous: Option<(crate::engine::swap::source_lifetime::SourceToken, u32)>,
        kind: u32,
        before: Option<Preflight>,
    ) {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[SourceOverlap] install_commit source={:?} previous(source,kind)={:?} kind=0x{:x} captured_preflight_input={:x?}",
            source, previous, kind, before.map(|before| before.input)));
        #[cfg(feature = "managed-grouped-source-alpha")]
        if let Some(before) = before {
            canonical::observe_visible(source, before.scene, inputs_for(before.input.slot).ok(),
                before.visible, "commit");
        }
    }

    #[cfg(feature = "source-capacity-probe")]
    pub(crate) unsafe fn observe_load_inputs(
        source: crate::engine::swap::source_lifetime::SourceToken,
        kind: u32,
        event: &str,
        before: Option<Preflight>,
    ) {
        let Some(before) = before else {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[SourceOverlap] load_event source={:?} kind=0x{:x} event={} preflight_unavailable=true",
                source, kind, event));
            return;
        };
        #[cfg(feature = "managed-grouped-source-alpha")]
        let strict_expected = canonical::loaded_input(before.input, before.previous, kind);
        #[cfg(not(feature = "managed-grouped-source-alpha"))]
        let strict_expected = Some(before.input);
        // One bounded read at this terminal event, after the caller released
        // STATE. This is not the earlier read that decided completion.
        let observed = inputs_for(before.input.slot);
        let current = observed.as_ref().ok().copied();
        let qualified = current.and_then(|input| load_expected_input(before, kind, input));
        let expected = qualified.or(strict_expected);
        let mask = |left: Inputs, right: Inputs| {
            left.slots.iter().zip(right.slots).enumerate().fold(0u16, |mask, (index, (a, b))| {
                mask | if *a != b { 1u16 << index } else { 0 }
            })
        };
        let changed_from_preflight = current.map(|current| mask(before.input, current));
        let authorized_visible_mask = strict_expected.zip(qualified).map(|(strict, qualified)| mask(strict, qualified));
        let unexpected_slots = expected.zip(current).map(|(expected, current)| mask(expected, current));
        let non_slot_fields_equal = expected.zip(current).map(|(expected, mut current)| {
            current.slots = expected.slots;
            current == expected
        });
        crate::engine::compat::logging::diagnostic(format_args!(
            "[SourceOverlap] load_event source={:?} kind=0x{:x} event={} event_only_resample=true changed_from_preflight_mask={:x?} unexpected_slots_mask={:x?} non_slot_fields_equal={:?} expected_available={} authorized_visible_mask={:x?} current={:x?}",
            source, kind, event, changed_from_preflight, unexpected_slots, non_slot_fields_equal,
            expected.is_some(), authorized_visible_mask, observed));
        #[cfg(feature = "managed-grouped-source-alpha")]
        if event == "stalled; native ownership retained" {
            canonical::observe_visible(source, before.scene, current, before.visible, "stalled");
        }
    }

    // Fixed field bits documented by the diagnostic contract. This observes the
    // same normalized view as the decision; it never changes a comparison.
    // Bits: 0 layout, 1 manager, 2 internal, 3 context, 4 service, 5 service_inner,
    // 6 listeners, 7 count, 8 quantity, 9 rules, 10 ratio_bits, 11 primary, 12 backup,
    // 13 resources, 14 aliases, 15 selection, 16 original (the four owners' entries).
    fn snapshot_difference(expected: Snapshot, current: Snapshot) -> u32 {
        let fields = [
            expected.layout != current.layout,
            expected.manager != current.manager,
            expected.internal != current.internal,
            expected.context != current.context,
            expected.service != current.service,
            expected.service_inner != current.service_inner,
            expected.listeners != current.listeners,
            expected.count != current.count,
            expected.quantity != current.quantity,
            expected.rules != current.rules,
            expected.ratio_bits != current.ratio_bits,
            expected.primary != current.primary,
            expected.backup != current.backup,
            expected.resources != current.resources,
            expected.aliases != current.aliases,
            expected.selection != current.selection,
            expected.original != current.original,
        ];
        let mask = fields.iter().enumerate().fold(0u32, |mask, (index, differs)| {
            mask | if *differs { 1 << index } else { 0 }
        });
        #[cfg(any(feature = "source-ldn-capacity-empty-probe", feature = "managed-ldn-third-source-alpha"))]
        let mask = mask
            | (u32::from(expected.ldn.is_some() != current.ldn.is_some()) << 17)
            | (u32::from(expected.ldn.map(|f| f.selection) != current.ldn.map(|f| f.selection)) << 18)
            | (u32::from(expected.ldn.map(|f| f.resources) != current.ldn.map(|f| f.resources)) << 19)
            | (u32::from(expected.ldn.map(|f| f.input) != current.ldn.map(|f| f.input)) << 20);
        mask
    }

    pub(crate) unsafe fn ready(reservation: Reservation, entry: *const u8, kind: u32) -> bool {
        #[cfg(feature = "managed-grouped-source-alpha")]
        { owned(reservation, entry) && canonical::ready(entry, reservation.slot as usize, kind).is_some()
            && native_source::ready(entry, reservation.slot, kind).is_some() }
        #[cfg(not(feature = "managed-grouped-source-alpha"))]
        { ready_ordinary(reservation, entry, kind) }
    }

    #[cfg(not(feature = "managed-grouped-source-alpha"))]
    unsafe fn ready_ordinary(reservation: Reservation, entry: *const u8, kind: u32) -> bool {
        let slot = reservation.slot as usize;
        owned(reservation, entry)
            && native_source::ordinary_kind(kind)
            && ordinary_actor_at(entry, slot, kind)
                .is_some_and(|(_, binding)| binding == reservation.input.records + slot * 0x280)
            && native_source::ready(entry, reservation.slot, kind).is_some()
    }

    /// One diagnostic at the stalled-load boundary. None means a prerequisite
    /// prevented a safe observation; it is not reported as a successful check.
    #[derive(Default)]
    pub(crate) struct LoadChecks {
        pub(crate) reservation: bool,
        pub(crate) preflight: bool,
        pub(crate) owned: bool,
        pub(crate) snapshot: Option<bool>,
        pub(crate) scene: Option<bool>,
        pub(crate) original_resources: Option<bool>,
        pub(crate) inputs: Option<bool>,
        pub(crate) ready: Option<bool>,
        /// FAIL 2: commit-time scene fields that differ now (`snapshot_difference` bits).
        pub(crate) scene_diff: Option<u32>,
        /// FAIL 2: input rows differing from the load's strict expectation (bit per row),
        /// whether every non-row input field is equal, and the first two changed rows as
        /// (row, expected, now), each (pointer, state, key, owner, alias).
        pub(crate) input_rows: Option<u16>,
        pub(crate) input_fields_equal: Option<bool>,
        pub(crate) changed_rows: [Option<RowChange>; 2],
    }

    /// One input row as (pointer, state, key, owner, alias).
    pub(crate) type InputRow = (usize, u8, i32, i32, i32);
    /// One changed input row as (row, expected, now).
    pub(crate) type RowChange = (u8, InputRow, InputRow);

    /// Row-by-row difference of two input views of one entry (diagnostic only).
    fn input_difference(expected: Inputs, current: Inputs) -> (u16, bool, [Option<RowChange>; 2]) {
        let row = |slot: &InputSlot| (slot.pointer, slot.state, slot.key, slot.owner, slot.alias);
        let mut rows = 0u16;
        let mut changed = [None; 2];
        let mut noted = 0;
        for (index, (want, now)) in expected.slots.iter().zip(current.slots.iter()).enumerate() {
            if want != now {
                rows |= 1 << index;
                if noted < changed.len() {
                    changed[noted] = Some((index as u8, row(want), row(now)));
                    noted += 1;
                }
            }
        }
        let mut fields = current;
        fields.slots = expected.slots;
        (rows, fields == expected, changed)
    }

    pub(crate) unsafe fn stalled_checks(
        ctx: &ServiceEntryContext,
        entry: *const u8,
        reservation: Option<Reservation>,
        before: Option<Preflight>,
        kind: u32,
    ) -> LoadChecks {
        let mut checks = LoadChecks {
            reservation: reservation.is_some(),
            preflight: before.is_some(),
            ..LoadChecks::default()
        };
        let Some((reservation, before)) = reservation.zip(before) else {
            return checks;
        };
        checks.owned = owned(reservation, entry);
        if !checks.owned {
            return checks;
        }
        let current = current_snapshot(reservation.scene.layout, ctx, entry, reservation.slot);
        checks.snapshot = Some(current.is_some());
        let observed_input = inputs_for(reservation.slot as usize).ok();
        let expected_input = observed_input.and_then(|input| load_expected_input(before, kind, input));
        if let Some(observed) = observed_input {
            #[cfg(feature = "managed-grouped-source-alpha")]
            let strict = canonical::loaded_input(before.input, before.previous, kind).unwrap_or(before.input);
            #[cfg(not(feature = "managed-grouped-source-alpha"))]
            let strict = before.input;
            let (rows, fields_equal, changed) = input_difference(strict, observed);
            checks.input_rows = Some(rows);
            checks.input_fields_equal = Some(fields_equal);
            checks.changed_rows = changed;
        }
        if let Some(current) = current {
            #[cfg(feature = "managed-grouped-source-alpha")]
            let current = canonical::comparison_view(current, before.scene, before.input, expected_input);
            let mut expected = before.scene;
            expected.listeners = current.listeners;
            checks.scene = Some(expected == current);
            checks.scene_diff = Some(snapshot_difference(expected, current));
            #[cfg(feature = "source-capacity-probe")]
            crate::engine::compat::logging::diagnostic(format_args!(
                "[SourceOverlap] stalled_scene entry={:p} kind=0x{:x} normalized_scene_diff_mask=0x{:x} input_qualified={} listeners_normalized=true",
                entry, kind, snapshot_difference(expected, current), expected_input.is_some()));
            checks.original_resources = Some(original_resources(current) == Some(before.resources));
            #[cfg(any(feature = "source-ldn-grouped-helper-capacity-probe", feature = "managed-grouped-source-alpha"))]
            if before.parked_groups.iter().any(Option::is_some) {
                let groups = parked_groups(current);
                crate::engine::compat::logging::diagnostic(format_args!(
                    "[GroupedHelperCapacity] stalled entry={:p} preserved={} before={:x?} current={:x?}",
                    entry, groups == Some(before.parked_groups), before.parked_groups, groups));
            }
        }
        checks.inputs = Some(expected_input.is_some());
        checks.ready = Some(ready(reservation, entry, kind));
        checks
    }

    pub(super) fn init() {
        if !load_supported() {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[NativeSourceCapacity] managed load contract unavailable; disabled"
            ));
            return;
        }
        ENABLED.store(true, Ordering::Release);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[NativeSourceCapacity] managed third-source alpha enabled; native Training; separate_ldn_opt_in={}",
            cfg!(feature = "managed-ldn-third-source-alpha")
        ));
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::native_source;
        use super::super::Layout;
        use super::{exact_mii_for_expansion, exact_mii_owner, owner_admission, owner_facts, ExactMiiProof, OwnerFacts};
        use crate::engine::mii::inventory::MiiClass;

        fn facts(
            index: usize, count: u32, installed: u64, kind: u32, exact_mii: ExactMiiProof,
        ) -> OwnerFacts {
            owner_facts(index, count, installed, kind, exact_mii)
        }

        #[test]
        fn exact_gunner_base_helper_admits_and_a_bare_mii_kind_does_not() {
            assert_eq!(MiiClass::from_kind(0x4a), Some(MiiClass::Gunner));
            assert!(!native_source::ordinary_kind(0x4a));
            assert!(native_source::ordinary_kind(0xb));
            // Extra creation still uses source_kind. Without grouped admission that
            // remains ordinary_kind, so a Mii is not itself an extra target.
            #[cfg(not(feature = "managed-grouped-source-alpha"))]
            {
                assert!(!super::source_kind(0x4a));
                assert!(super::source_kind(0xb));
            }
            assert!(owner_admission(
                Layout::Training, facts(2, 1, 1, 0x4a, ExactMiiProof::Proven)
            ).admits);
            for proof in [ExactMiiProof::NotApplicable, ExactMiiProof::Busy, ExactMiiProof::Refused] {
                assert!(!owner_admission(
                    Layout::Training, facts(2, 1, 1, 0x4a, proof)
                ).admits, "{:?}", proof);
            }
        }

        #[test]
        fn sword_and_brawler_helpers_use_the_same_exact_proof() {
            for (index, kind, class) in [
                (2usize, 0x48u32, MiiClass::Brawler),
                (3, 0x49, MiiClass::Swordfighter),
            ] {
                assert_eq!(MiiClass::from_kind(kind), Some(class));
                assert!(!native_source::ordinary_kind(kind));
                assert!(owner_admission(
                    Layout::Training, facts(index, 1, 1, kind, ExactMiiProof::Proven)
                ).admits);
                assert!(!owner_admission(
                    Layout::Training, facts(index, 1, 1, kind, ExactMiiProof::Refused)
                ).admits);
            }
        }

        /// lobby-preload A (was `visible_mii_and_non_mii_proof_flags_stay_refused`): a visible
        /// exact Mii is admitted only with the visible committed-authority proof.
        #[test]
        fn visible_mii_admits_only_with_the_visible_proof() {
            for index in [0usize, 1] {
                assert!(owner_admission(
                    Layout::Training, facts(index, 1, 1, 0x4a, ExactMiiProof::Proven)
                ).admits);
                for proof in [ExactMiiProof::NotApplicable, ExactMiiProof::Busy, ExactMiiProof::Refused] {
                    assert!(!owner_admission(
                        Layout::Training, facts(index, 1, 1, 0x4a, proof)
                    ).admits);
                }
            }
            // Pyra (0x5a) is not a Mii class. A Proven flag cannot admit it.
            assert!(MiiClass::from_kind(0x5a).is_none());
            assert!(!native_source::ordinary_kind(0x5a));
            assert!(!owner_admission(
                Layout::Training, facts(2, 1, 1, 0x5a, ExactMiiProof::Proven)
            ).admits);
            assert!(!owner_admission(
                Layout::Training, facts(2, 1, 0, 0x4a, ExactMiiProof::Proven)
            ).admits);
            // Count 4 is outside both the exact count-1 proof and the grouped 2..=3 range.
            assert!(!owner_admission(
                Layout::Training, facts(2, 4, 1, 0x4a, ExactMiiProof::Proven)
            ).admits);
        }

        #[test]
        fn ordinary_owners_beside_one_exact_gunner_helper_still_admit() {
            let owners = [
                (0usize, 0x1cu32, ExactMiiProof::NotApplicable),
                (1, 0x00, ExactMiiProof::NotApplicable),
                (2, 0x4a, ExactMiiProof::Proven),
                (3, 0x0b, ExactMiiProof::NotApplicable),
            ];
            for (index, kind, proof) in owners {
                assert!(native_source::ordinary_kind(kind) || kind == 0x4a);
                assert!(owner_admission(Layout::Training, facts(index, 1, 1, kind, proof)).admits);
            }
            assert!(!owner_admission(
                Layout::Training, facts(2, 1, 1, 0x4a, ExactMiiProof::Refused)
            ).admits);
        }

        #[test]
        fn helper_proof_does_not_lock_a_visible_or_ordinary_owner() {
            unsafe {
                // The visible path takes the native lock only for a count-1 installed Mii.
                for (count, installed, kind) in [(1u32, 1u64, 0x0bu32), (2, 1, 0x4a), (1, 0, 0x4a)] {
                    assert_eq!(
                        exact_mii_owner(1, 1, core::ptr::null(), count, installed, kind).0,
                        ExactMiiProof::NotApplicable
                    );
                }
                assert_eq!(
                    exact_mii_for_expansion(0, 0, 1, 1, 0x4a),
                    ExactMiiProof::NotApplicable
                );
                assert_eq!(
                    exact_mii_for_expansion(1, 1, 1, 1, 0x4a),
                    ExactMiiProof::NotApplicable
                );
                assert_eq!(
                    exact_mii_for_expansion(2, 2, 1, 1, 0x0b),
                    ExactMiiProof::NotApplicable
                );
                assert_eq!(
                    exact_mii_for_expansion(2, 2, 2, 1, 0x4a),
                    ExactMiiProof::NotApplicable
                );
                assert_eq!(
                    exact_mii_for_expansion(2, 2, 1, 0, 0x4a),
                    ExactMiiProof::NotApplicable
                );
            }
        }
    }
}
