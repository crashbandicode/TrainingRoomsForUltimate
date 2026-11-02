//! Same-entry ordinary fighter bodies, built and retired by the native loader.
//!
//! A bank is a cache, never a saved selection. Its descriptor/table belongs to
//! one visible entry and owner; no body or controller crosses that boundary.
//! INSTALL serializes construction and switching. Only the native pre-service
//! callback requests a change, and native case12/13 owns activation/restore.
//! Canonical groups and exact Miis continue to use the ordinary loader. Kirby is
//! an ordinary bank member whose *departure* is gated: a live Kirby parks only
//! while it holds no copy ability (`kirby_park_gate`, EXCLUSIONS-AUDIT §2);
//! otherwise its request takes the canonical Reload exactly as before. The
//! role's slot-0 original is an ordinary member too when it is an ordinary
//! single-body kind and `original_bank` is on (`excluded_original`, §9): the
//! adapters plan `Target::Original` through `plan()`, so the return to it is a
//! window reuse or a group swap instead of the canonical Reload that retires
//! every bank and group; Off keeps today's canonical-only original.
//!
//! D-KIRBY1 (D-KIRBY1-ROB1-RCA.md): a Kirby copy registers the copied kind on
//! Kirby's *own* R and the per-kind release `0x17e8610(R, k)` also drops R's G68
//! mirror for `(k, costume)`, which that copy shares. So while a role's live
//! member is Kirby its R holds no bank prefetch key: Planned groups wait,
//! Prefetched groups hand their keys back (`group_lifecycle`), and every key
//! release is refused for the kind a live Kirby copies (the ledger's live-consumer
//! check, `release_group_keys`). Since WP4 every acquisition and release goes through
//! the per-R ledger (`crate::engine::swap::swap_model::ledger`, `crate::runtime::roster::r_handle`):
//! a token per kind with its release recipe, drained before a Kirby becomes live and
//! suspended while it is (`swap_model::occupancy`), the pair-rule `R+0x50` write only under
//! an exclusive reset batch, and a `[Ledger] reconcile` line after every build and teardown.
//!
//! # Group swap (GROUP-SWAP-FEASIBILITY §6)
//!
//! The native window at `entry+0x4160` holds exactly three cells (fixed-capacity
//! vector, FACT §2.1). A role may keep further *groups* of one to three built
//! bodies detached from that window. All groups share the entry's one resource
//! owner `R = *(G+0xe8+8*slot)`; no R is allocated and no Reload runs while a
//! group is detached. The window group is this module's existing `Bank.members`
//! and `Bank.bodies`; extra groups are `Bank.groups`, and a swap exchanges the
//! window record with a detached record, so every existing proof keeps looking
//! at what native sees.
//!
//! Owned native writes, each limited to the fields the report enumerates (§2.2,
//! GROUP-SWAP-IMPL §14):
//! * the window fields `+0x4150`, cells `+0x4160/68/70`, contexts
//!   `+0x4178/80/88/90/98`, descriptor words `+0x14/+0x18..+0x20/+0x28`, flags
//!   `+0x591f`, per-cell ints `+0x5960..+0x5a1b`, indices `+0x5918/+0x5919`;
//! * the entry-info observer dispatcher's two array pointers `info+0x10` (listener
//!   lists) and `info+0x18` (deferred removals), `info = *(entry+0xf8)`. The
//!   registrations made while a group is in the window belong to that group:
//!   `654dc0` destroys and re-creates both arrays at every construction
//!   (`0x37ae5e0(info+8)` at `0x654de0`, §14.1), so a build detaches the window
//!   group's arrays (pointers nulled, exactly the state the destructor leaves),
//!   lets `654dc0` create the new group's, and a warm swap exchanges them;
//! * the acknowledgement `+0x5933` and request bytes `+0x5932/+0x591a` the bank
//!   already writes, and at the exchange the previous-cell index `+0x5910`
//!   (`set_previous_index`: the activation's body event reads that cell's id, so
//!   it must be a cell the attached group has);
//! * `R+0x4c |= 1` before a synchronous build when bit0 is clear. Native writers
//!   of the same bit in the same state: the scene path `0x66de44-0x66de50` (map
//!   non-empty from the synchronous `0x17e5bc0`, `R+0x54..` configured, right
//!   before `0x658af0`) and loader state 6 `0x6513a8-0x6513b4` (before the body
//!   constructions that consume the keys). The reset `0x17e45f0` clears it at
//!   the last per-kind release (`0x17e46e4`); `0x17e99d8` gates every body
//!   construction on it (§14.2). On the bank's R other batches' keys keep the map
//!   non-empty, so the build holds the bit as a lease and clears it itself once
//!   the construction returned, only if the byte is still what it wrote (U19,
//!   `swap_model::request_bit`).
//!
//! Native calls: `0x17e64d0(R,k)` prefetch (the existing `0x17e5bc0` post-hook
//! call), the R ready vcall (`+0x30`, `0x17e4750`: idle task, NRO tree and every
//! map handle loaded; the only readiness predicate native uses), the synchronous
//! constructor `0x658af0(entry)` (guarded by `+0x4150 == 0`, §1.1), the control
//! selector `0x64fb70(entry)` the commit itself just made (§3), finalize's
//! per-body destroyer `0x3aeb50(id)` (§4.1), context vtable `+0x8` as `0x651660`
//! does, the dispatcher-array destructor `0x37ae5e0` on a detached group's
//! arrays (finalize's own order: bodies at `0x652ebc`, then `0x37ae5e0` at
//! `0x6531bc`), the per-kind release `0x17e8610(R,k)` for prefetched-only
//! kinds (§5.1), and the per-key release `0x17e2fe0(R,key)` for the one key
//! `0x17e8610` skips for the Aegis object kind (`0x1338`, D-SG1-RCA; computed as
//! `ledger::orphan_keys` and issued by `r_handle::release`, orphans first).
//!
//! Every detached group is destroyed before any native destroyer: the loader
//! pre-dispatch when `+0x591c == 0` (before state 0), a function hook at
//! finalize `0x6524e0`, and `before_reload` on every mod-initiated Reload.
//!
//! # Phase B′: asynchronous build (operator decision 2026-11-02, W20 FINDINGS §2b′)
//!
//! `0x658af0` constructs its bodies synchronously on the game thread (console C116.2: the whole
//! game froze ~1.0 s at the first switch into a prefetched 3-body group). The game's own Reload
//! constructs them on the task worker instead (loader state 10, `6556b0.c:215-250`): one 0x90-byte
//! task per body, `0x3549720(task+0x10, 0x3cb190, 2, 1)`, functor `0x3cb2a0` = body vtable
//! `+0x10` then `+0x18`; state 0xb polls `task+0x20 == 0`, installs the cells and frees the tasks.
//! An ordinary group's build (`async_build_eligible`) now takes that path at the acknowledgement
//! seam: the entry-level third of `0x658af0` (`0x654dc0`, `0x655320`, `0x64f820`) and the per-body
//! `0x3ae5f0` allocation on the game thread, the init+construct as queued tasks, the entry parked
//! at loader `0xc` with an empty window while frames run (the native shape of states 6-0xb; a
//! request meanwhile is refused as busy), then on the settling tick the cell install, the request
//! bit's return, the ordinary verification and the acknowledgement (`settle_construction`,
//! `complete_exchange`). A destroyer that arrives first waits for the tasks as state 0xb's tail
//! does and installs the bodies for native's own window destroyer (`drain_construction`). Mii,
//! native (PT/Aegis) and pair groups keep the synchronous build: an operator-approved restriction
//! dated 2026-11-02 (`R` identity through the construction; the `+0x5a88` object `0x658af0`'s
//! branches create; the `R+0x50` batch rule). P3-0: `Bank.tick` measures the largest gap between
//! two entry ticks over a swap (`[GroupSwap] swap … max_tick_gap_us=`), the stall the game thread took.
//!
//! The entry's identity follows the window (U18, `swap_model::copy_registry`):
//! native builds one body per (entry, cell), registers it in the Kirby copy
//! registry under the header word at its cell with its own entry's roster
//! record (init `0x607eb4-0x607f30`), and unregisters it under its own kind at
//! unload (`0x6118d0`). The window and every detached group share those cells,
//! so the header `R+0x58..0x60`, the roster `R+0xb0` (`0x17e4f50`) and the
//! registrations (`unprepare 0x17f26c0` / `prepare_entry 0x17f1a60`) always
//! describe the attached window: a build or warm exchange moves them as a
//! native Reload into that group would, a detached group is destroyed under its
//! own header over empty cells, and the window registers again afterwards.
//!
//! A one-member group builds, attaches, detaches and tears down through the same
//! code: every cell walk is bounded by `Members::count`, `write_group_words`
//! publishes `u32::MAX` padding words and `651660` frees only non-null contexts.
//! It is *left* warm by a self-commit (`ExitRule::SelfCell`): `switch` requests
//! its own cell 0, `6581f0` parks the one body and `64fb70` re-selects that body's
//! own context (the commit never compares `+0x591a` with `+0x5918`), and the
//! exchange at the acknowledgement seam replaces the window as for any group swap.
//! A refused exchange (`perform_swap` fallback) re-activates the outgoing body.
//!
//! Solo Popo is a window member at any rotation position, laid out at physical
//! cell 0 like Kirby (`CellRule::PinCell0`, `0x17e8410` registers only the
//! primary's Popo); a lane `0x4b` reaches the window planner only when the role's
//! numbered form is Solo (`window_lane`).
//!
//! # Ice Climbers pair bank (ICE-CLIMBERS-OPTIMIZATION addendum, PN1/PN2 facts)
//!
//! A normal-pair target may own a *pair-lead* window `[Popo, Nana, X]` (count 3,
//! `+0x28 = 2`, `Members::is_pair_lead`). Native constructs, activates, commits
//! and frees it through the same bit-2 paths as every bank; three narrow seams
//! make Nana behave as the vanilla partner:
//! * **N1** inline at `0x6555a0` (the `mov w2,w20` before `bl 0x6b6f10` for
//!   table[1]): `x6 = *(entry+0x4188)` (Popo's context), the `prev` argument
//!   vanilla `655320` passes at `0x655504-0x655510`. `6b6f10` is the only writer
//!   of `ctx+0x60` and binds Nana's control to route `owner+8` (U2 FACT).
//! * **N2** function hook on `656ee0`: while the pair is the active member
//!   (`+0x5918 == 0`) alias `+0x4180 = +0x4190` and run the call with flags `2`
//!   (bit 1, no bit 2) so the pair branch wakes cells 0/1 and registers Nana on
//!   `+0x4180` (`activate_00656ee0.c:65`); flags return to `6` afterwards.
//! * **N3** post-hook on `6581f0`: when this commit parked the pair (old index 0,
//!   new index 2, loader `0xc`), apply the commit's own outgoing sequence to cell
//!   1 (`646fe0` wrapper, save/restore `wrapper[1]+0x1a4`, fighter vtable
//!   `+0x48(body, !(flags&0x40))`; `change_commit_006581f0.c:60-75`), then
//!   `+0x4180 = 0` and flags `4`. The commit already released both pads.
//!
//! Flags: `6` while the pair is active, `4` otherwise; bit 2 is set at every
//! destroyer so `651660` frees `+0x4188/90/98` once (`free_651660.asm`). Gates:
//! the pair is parked only with both Climbers state 4 and outside Dead, Rebirth,
//! Entry and Standby; otherwise the canonical Reload runs. A pair window joins a
//! group swap only with `ic_pair_groups` on (default on since B123-pre row 4,
//! EXCLUSIONS §4c): the
//! commit index is pair ↔ extra (`swap_commit_index_in`, never Nana's cell), the
//! pair is parked by the same N3 sequence as an in-window change before the
//! exchange, the saved window carries flags `4` and `+0x4180 = 0`, and the warm
//! return re-activates the pair through N2. The pair group itself is never built
//! detached. Runtime switch `ic_pair_bank` (menu/INI, default on).
use super::{aligned, read, BASE};
use crate::engine::seams::native_fields as fields;
use skyline::hooks::InlineCtx;
use std::convert::TryInto;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Mutex;
use crate::engine::swap::body_bank::{
    count_extra_groups, dispatcher_array_header_ok, dispatcher_idle_words, native_group_request, original_bankable,
    pair_request, pair_resource_references, plan_extra_groups_with, prefetch_outcome,
    swap_commit_index_in, async_build_eligible, construction_task, TickGap,
    GroupBudget, GroupExchange, GroupOwner, GroupPolicy, GroupState, Members, Phase, Special,
    SpecialPolicy, BANK_FLAG, MAX_EXTRA_GROUPS, NANA, NATIVE_MEMBERS, PAIR_ACTIVE_FLAG, POOL_RESERVE, POPO,
    entry_bytes_of, mii_reuse_bytes_match, MiiEntryBytes, MiiIdentity, MII_ENTRY_START, RESOURCE_WORDS,
};
use crate::engine::swap::body_bank::memory::{self, Admission};
use crate::engine::swap::body_bank::mii_publish::{PublishAttempts, PublishRefusal, PublishStep};
use crate::engine::preload::preload_metrics::MemorySample;
use crate::engine::preload::cycle_preload_config::Settings;
use crate::engine::swap::character_forms::{EntryState, Forms};
use crate::engine::mii::lifetime::Configuration;
use crate::engine::swap::swap_model::{capabilities, form_class, group_serves, previous_cell_index};
use crate::engine::swap::swap_model::lifecycle::{destroy_plan_many, AttachedGroup, BodyRef, Bundle, DestroyStep, DetachedGroup, DispatcherRef, RecordsRef};
use crate::engine::swap::swap_model::copy_registry::{self, RegistryOps};
use crate::engine::swap::swap_model::fallback::{self, Disposition, Fallback};
use crate::engine::swap::swap_model::request_bit::{End as LeaseEnd, Lease};
use crate::engine::swap::swap_model::ledger::{native_window_requests, GroupId, Holder, LedgerRefusal, ReleaseRecipe, Role as LedgerRole};
use crate::engine::swap::swap_model::occupancy::{KindOccupancy, Occupancy, Rules};
use crate::runtime::roster::body_graph::{self as graph, DESTROY_BODY, DISPATCHER_DESTROY, HEAP_FREE, SPECIAL_OBJECT_RELEASE};
use crate::runtime::roster::r_handle;
use crate::runtime::roster::special_articles as articles;
use crate::runtime::roster::build_split as split;

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

static ENABLED: AtomicBool = AtomicBool::new(false);
/// Group-swap fingerprints and the finalize hook passed. The INI/menu switch is
/// still consulted at every decision.
static GROUPS_ENABLED: AtomicBool = AtomicBool::new(false);
/// Pair-bank fingerprints passed and the N1/N2/N3 hooks are installed. The
/// `ic_pair_bank` switch is still consulted at every plan.
static PAIR_ENABLED: AtomicBool = AtomicBool::new(false);
/// Special-group fingerprints (`0x3a6b80`, `0x392eb40`, `0x17e2fe0`) passed.
static SPECIAL_ENABLED: AtomicBool = AtomicBool::new(false);
/// Phase B′ fingerprints (the asynchronous construction path) and the functor vtable slot passed.
static ASYNC_ENABLED: AtomicBool = AtomicBool::new(false);
static BANKS: [Mutex<Option<Bank>>; 2] = [Mutex::new(None), Mutex::new(None)];
static RESERVED: AtomicU8 = AtomicU8::new(0);
static ROLE_BUDGET: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
// Display targets are not resource reservations: canonical-only original
// restoration still occupies a body cell but cannot promise a reuse hit.
static PLANNED_TARGETS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
static EXTRAS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
/// Extra-group bodies per role (planned, prefetched or built; failed excluded).
static GROUP_BODIES: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
/// Fighter pool `**0x52b8548` capacity/live, sampled inside an entry callback.
static POOL_CAPACITY: AtomicU32 = AtomicU32::new(0);
static POOL_LIVE: AtomicU32 = AtomicU32::new(0);

const SYNC_CONSTRUCT: usize = 0x658af0;
const SELECT_CONTROL: usize = 0x64fb70;
/// Phase B′ (operator decision 2026-11-02, W20 FINDINGS §2b′): the game's own asynchronous
/// construction, as loader state 10 runs it (`research/phase2-native/native_entry_dispatch_6556b0.c:215-250`).
/// `0x658af0`'s entry-level third in its order (`0x658b14-0x658b24`): `0x654dc0` (dispatcher arrays,
/// hooked by `prepare_models`), `0x655320` (control contexts from the table hook) and `0x64f820`
/// (AI params, the `+0x4178` select for `+0x5918`, the pad route). Then per kind `0x3ae5f0(0, kind,
/// owner)` pops the pool slot and runs the base constructor on the game thread; a 0x90-byte task
/// from the heap `0x392e290(0x10, 0x90)` carries the body; `0x3cb050` swaps the functor
/// `{0x4f69bb8, task}` into `task+0x50`; `0x3549720(task+0x10, 0x3cb190, 2, 1)` queues it; the callback
/// `0x3cb190` runs the functor `0x3cb2a0` on the task worker: body vtable `+0x10` (init) then
/// `+0x18` (construct). State 0xb polls `task+0x20 == 0`, installs each body into its cell and
/// frees the task with `0x392eb40` (`native_entry_load_650010.c:681-700, 886-924`); its teardown
/// waits with `0x354ccd0(*(*(main+0x5333f58 + 8*id) + 0x10), &task+0x20)`.
const MODELS_SETUP: usize = 0x654dc0;
const COUNT_SETUP: usize = 0x655320;
const CONTROL_INIT: usize = 0x64f820;
const BODY_ALLOC: usize = 0x3ae5f0;
const HEAP_ALLOC: usize = 0x392e290;
const FUNCTOR_COPY: usize = 0x3cb050;
const TASK_SCHEDULE: usize = 0x3549720;
const TASK_CALLBACK: usize = 0x3cb190;
const TASK_WAIT: usize = 0x354ccd0;
const TASK_QUEUES: usize = 0x5333f58;
const CONSTRUCT_FUNCTOR_VTABLE: usize = 0x4f69bb8;
const CONSTRUCT_FUNCTOR: usize = 0x3cb2a0;
/// F6: native loader state 8's control-init task (`native_entry_dispatch_6556b0.c` case 8): the functor vtable `0x4f81510`, whose
/// `+0x30` slot `0x6574e0` runs `0x64f820(*task)` (`ldr x8, [x0, #8]; ldr x0, [x8]; b 0x64f820`), queued with the callback
/// `0x655f30` (the same eight words as `0x3cb190`) and the scheduler arguments `(2, 1)`.
const CONTROL_FUNCTOR_VTABLE: usize = 0x4f81510;
const CONTROL_FUNCTOR: usize = 0x6574e0;
const CONTROL_CALLBACK: usize = 0x655f30;
const RELEASE_KIND: usize = 0x17e8610;
const ACQUIRE_KIND: usize = 0x17e64d0;
/// Per-key release of the resource owner's map, the primitive `0x17e8610` runs
/// for every key it releases: lock `R+0x28`, find the key in the tree at
/// `R+0x10` (a missing key is a no-op), decrement `node+0x28`, and on the last
/// reference remove the node (`R+0x18 -= 1`), release its handle and free it
/// (`0x17e2fe0-0x17e30dc`).
const RELEASE_KEY: usize = 0x17e2fe0;
/// Destructor of the entry-info observer dispatcher (`{n, _, lists, removals, depth, pending}`,
/// 0x20 bytes at `info+8`): releases every listener node of both arrays, frees
/// the arrays, nulls `+8/+0x10`, zeroes `+0x18/+0`. `654dc0` calls it at
/// `0x654de0`; finalize at `0x6531bc` after the bodies (`0x652ebc`). Called only by
/// `swap_native::body_graph::execute` (`DISPATCHER_DESTROY`).
const FINALIZE_ENTRY: usize = 0x6524e0;
/// The Kirby copy registry (U18, `swap_model::copy_registry`): `prepare_entry(G68, R+0xb0, kind,
/// costume, entry, sub)` is what init runs per body at `0x607f30`; `unprepare(G68, kind, entry,
/// sub)` what unload runs at `0x6118d0`; `0x17e4f50(R)` the roster writer configure runs at
/// `0x651380`. Each takes the registry mutex `G68+0x1d238` itself. `G68 = *(G+0x68)`, `G` the
/// owner table; the region word feeds init's R.O.B. costume selector (`0x607ecc`).
const PREPARE_ENTRY: usize = 0x17f1a60;
const UNPREPARE: usize = 0x17f26c0;
const ROSTER_REFRESH: usize = 0x17e4f50;
/// K15: the copy-package acquire/release `swap_native::copy_packages` calls (fingerprinted here).
const COPY_ACQUIRE: usize = 0x17efb40;
const COPY_RELEASE: usize = 0x17f0290;
const REGION_WORD: usize = 0x523d00c;
const G68_SLOT: usize = 0x68;
const REGISTRY_CELLS: usize = 0x1d258;
const REGISTRY_KINDS: usize = 0x1d498;
/// Entry object records a native group's synchronous build creates (`0x658af0`
/// PT branch `0x658d8c`/`0x658e10`, Aegis branch `0x658ed8`): a 0x238-byte object
/// with this vtable at `+0x5a88` (PT also a 0x10-byte object at `+0x5a98`).
/// Finalize destroys it with `0x3a6b80(obj)` then `0x392eb40(obj)` and frees
/// `+0x5a98` (`0x652f5c-0x652f84`); loader state 1 does the same (`0x651288`).
const SPECIAL_OBJECT_VTABLE: usize = 0x4f626e8;
/// Pokémon Trainer and Pyra/Mythra form their own swap groups (DESIGN §2/§3);
/// the `native_groups` menu/INI switch (default on) still gates every plan.
const NATIVE_GROUPS_SHIPPED: bool = true;
/// The normal Ice Climbers pair as a detached group repeats the B119 class on a
/// path whose runtime precedent is the pair-lead window (B123-pre row 4 passed);
/// shipped on for C116.1 (the final Eden run decides; the primary flips the
/// `pair_groups` switch, default on, if its row fails). The B123-pre row could
/// not reach it: this constant was false and `plan()` compared the request to
/// `member_forms`, which never yields the pair (see `group_reachable`).
const PAIR_GROUPS_SHIPPED: bool = true;
/// Mii one-member groups (MII-GROUPS-DESIGN) ship on: the adapter half (record
/// publication before the switch, exact settlement, `PortraitIntent::Exact`) is
/// in, behind the `mii_groups` menu/INI switch (default on). C116.1 keeps the
/// default only while the Eden matrix (M1-M13) is green at the final freeze
/// (refactor WORKPLAN §5.3); the switch still gates every plan.
const MII_GROUPS_SHIPPED: bool = true;
const FIGHTER_VTABLE: usize = 0x4f80210;
const CONTEXT_VTABLE: usize = 0x4f85d30;
const CELL_VECTOR_VTABLE: usize = 0x4f81438;
const RESOURCE_VTABLE: usize = 0x509fb10;
const OWNER_TABLE: usize = 0x5324680;
const FIGHTER_POOL: usize = 0x52b8548;
const BODY_STRIDE: usize = 0xf940;
/// A group whose resource owner does not report ready within this time is failed
/// and its keys released; the INSTALL token it held is returned.
const PREFETCH_TIMEOUT_US: u64 = 15_000_000;
// GS-PANIC: the process-memory admission of a group is `crate::engine::swap::body_bank::memory` (measured per-phase
// costs, the unspent part of every admitted group held until it is built). It replaced a 96 MiB-per-body constant
// checked against the current headroom only, which let a second group spend what the first group's build needed.
/// Pair bank seams: entry activation (first, rebirth, return), the change commit,
/// the `mov w2,w20` before `655320`'s table[1] `bl 0x6b6f10`, the commit's per-body
/// information selector and the fighter deactivate reached through vtable `+0x48`.
const ACTIVATE_ENTRY: usize = 0x656ee0;
const COMMIT_CHANGE: usize = 0x6581f0;
const PAIR_PARTNER_SITE: usize = 0x6555a0;
const OUTGOING_OWNER: usize = 0x646fe0;
const DEACTIVATE_BODY: usize = 0x60ffd0;
/// Pad/control manager singleton slot: `P = *(main+0x52b6fd8)`, `S = *P`. `S+0x98` is a table of 16
/// pointers to 0x20-byte route objects `A_i` (pad slot i): `A->+0` = the routed controller object
/// (`ctx+0x58` of the activated context), `A->+8` = tick-disabled flag, `A->+0x14` = frame counter.
/// The per-frame route tick `main+0x2f7b80` (pad thread, from `0x311db0`) dereferences `**(A->+0)` and
/// `(**(A->+0))->+0x38` whenever `A->+8 == 0`. Only `main+0x313b00(mgr, idx, flag, obj != 0)` (activation
/// `656ee0+0xd0`, construction) writes `A->+0`; the route release `313e90` and the context destructor
/// `6b7920` (which frees the controller) leave it stale. D-PTA1c: a Reload destroys the window contexts in
/// states 1-3 (`651660`) and re-points the slot only at activation; a prequeue refusal (D-PTA1b) never
/// activates, so the tick reads freed memory every frame (`Unmapped Read64 @ 0` / `Read8 @ 0x38`;
/// D-PTA1BC-FIX.md). Nulling the slot first turns the tick into its no-route path (counter only).
/// The teardown protocol clears it before any context is freed, by us or by native
/// (`swap_model::lifecycle`, executed by `swap_native::body_graph`).
const KIRBY: u32 = 0x06;

fn canonical_original(menu: &crate::engine::compat::menu::TrainingModpackMenu, role: usize) -> Option<u32> {
    let role = if role == 0 { super::Role::Player } else { super::Role::Cpu };
    let kind = super::original_kind(role)?;
    (super::requested_forms_at(menu, role, kind, None)
        == super::requested_forms_at(menu, role, kind, Some(0))).then_some(kind)
}

/// The role's original that stays canonical-only, excluded from extra groups and
/// from the reusable count. With `original_bank` on, an ordinary single-body
/// original is an ordinary member instead (EXCLUSIONS §9): the adapters then plan
/// `Target::Original` through `plan()` like a numbered fighter, and nothing is
/// excluded. Ice Climbers, Miis and native-group originals stay canonical.
fn excluded_original(settings: &Settings, menu: &crate::engine::compat::menu::TrainingModpackMenu, role: usize) -> Option<u32> {
    canonical_original(menu, role).filter(|kind| !(settings.original_bank_enabled() && original_bankable(*kind)))
}

fn groups_enabled(settings: &crate::engine::preload::cycle_preload_config::Settings) -> bool {
    settings.groups_enabled() && GROUPS_ENABLED.load(Ordering::Acquire)
}

/// Special groups the planner may form: native groups when group swap, the
/// `native_groups` switch and their fingerprints are on; the pair group
/// additionally needs the pair seams, the `pair_groups` switch and the shipped
/// constant (DESIGN §5). `group_swap` off yields `NONE` whatever the per-kind
/// switches say (`groups_enabled` is the master switch, C116.1).
fn special_policy(settings: &Settings) -> SpecialPolicy {
    if !groups_enabled(settings) { return SpecialPolicy::NONE; }
    let special = SPECIAL_ENABLED.load(Ordering::Acquire);
    SpecialPolicy {
        native: NATIVE_GROUPS_SHIPPED && special && settings.native_groups_enabled(),
        pair: PAIR_GROUPS_SHIPPED && special && settings.pair_groups_enabled() && pair_enabled(settings),
        mii: MII_GROUPS_SHIPPED && special && settings.mii_groups_enabled(),
    }
}

/// The `[GroupSwap] enabled` line, written once by the startup cycle-preload load
/// (`cycle_preload_config::load_cache`) after it caches the settings. `install`
/// runs before that load, so a line written there showed only the fingerprints and
/// shipped constants, not the INI (Eden E8: `mii_groups=true` on a boot that ran
/// fail-closed). Here `group_swap` is the master switch as applied and each kind is
/// [`special_policy`]: fingerprints, shipped constant and the loaded switch.
pub(crate) fn report_group_swap(settings: &Settings) {
    if !GROUPS_ENABLED.load(Ordering::Acquire) { return; }
    let policy = special_policy(settings);
    trace!("[GroupSwap] enabled finalize_hook=+0x{:x} max_extra_groups={} pool_reserve={} prefetch_timeout_us={} memory=measured kinds={} sets={} helpers={} margin=1/{} floor={} consumers=groups+helpers special={} group_swap={} native_groups={} pair_groups={} mii_groups={}",
        FINALIZE_ENTRY, MAX_EXTRA_GROUPS, POOL_RESERVE, PREFETCH_TIMEOUT_US,
        crate::engine::swap::body_bank::memory_table::KIND_COSTS.len(), crate::engine::swap::body_bank::memory_table::SET_COSTS.len(),
        crate::engine::swap::body_bank::memory_table::HELPER_COSTS.len(),
        memory::MARGIN_DIVISOR, memory::FLOOR,
        SPECIAL_ENABLED.load(Ordering::Acquire), groups_enabled(settings), policy.native, policy.pair, policy.mii);
}

// ----- Mii one-member groups: the identity in `R`, the entry and the D mirror -----
//
// The value rules (`MiiIdentity`, its state-6 word derivation, the entry bytes it
// owns, the D mirror) live in `crate::engine::swap::body_bank`; this module only
// performs the writes under INSTALL in the entry callback (MII-GROUPS-DESIGN §3).

unsafe fn save_resource_words(r: usize) -> [u32; RESOURCE_WORDS] {
    std::array::from_fn(|i| fields::read_unchecked::<u32>(r, 0x58 + i * 4))
}

unsafe fn write_resource_words(r: usize, words: &[u32; RESOURCE_WORDS]) {
    for (i, word) in words.iter().enumerate() { fields::write_unchecked(r, 0x58 + i * 4, *word); }
}

/// Header `R+0x54..0x68` implied by saved resource words (owner word unchanged).
fn header_of(owner: u32, words: &[u32; RESOURCE_WORDS]) -> [u32; 5] {
    [owner, words[0], words[1], words[2], words[3]]
}

/// `R` takes the identity's words: the state-6 derivation over the window's
/// current words (`R+0x7c`, `R+0x7e`, `R+0x80` and the unused halves stay).
unsafe fn write_identity_words(r: usize, identity: MiiIdentity) {
    let words = identity.resource_words(save_resource_words(r));
    write_resource_words(r, &words);
}

/// Run a native key request or release for a Mii group with its identity in `R`,
/// then put the window's words back (the bracket of MII-GROUPS-DESIGN §3.2).
unsafe fn with_mii_identity(r: usize, identity: Option<MiiIdentity>, f: impl FnOnce()) {
    match identity {
        Some(identity) => {
            let saved = save_resource_words(r);
            write_identity_words(r, identity);
            f();
            write_resource_words(r, &saved);
        }
        None => f(),
    }
}

/// Everything a Mii identity owns while its group is the window besides `R`: the
/// entry's identity bytes (`FE+0x41/0x43/0x46..0x50/0x52..0x55`) and the
/// information block's descriptor mirror (`D+0x72..0x7a`), both of which loader
/// state 6 derives for a canonical Mii and nothing native writes for a bank
/// build (R-M3). Saved before the write so a rollback or a warm departure puts
/// the previous window's bytes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EntryIdentity { entry: MiiEntryBytes, mirror: [u16; 4] }

unsafe fn save_entry_identity(entry: usize) -> EntryIdentity {
    let descriptor: [u8; 0x70] = fields::read_unchecked(entry, 0);
    let mirror = entry_info(entry).map_or([0; 4], |info| fields::read_unchecked::<[u16; 4]>(info, 0x72));
    EntryIdentity { entry: entry_bytes_of(&descriptor), mirror }
}

unsafe fn write_entry_identity(entry: usize, saved: &EntryIdentity) {
    for (i, byte) in saved.entry.iter().enumerate() {
        fields::write_unchecked(entry, MII_ENTRY_START + i, *byte);
    }
    if let Some(info) = entry_info(entry) {
        for (i, half) in saved.mirror.iter().enumerate() { fields::write_unchecked(info, 0x72 + i * 2, *half); }
    }
}

/// Write a Mii identity into the window (`R` words, entry bytes, D mirror) and
/// return the entry bytes written, the exchange receipt's expectation.
unsafe fn write_identity(entry: usize, r: usize, identity: MiiIdentity) -> MiiEntryBytes {
    write_identity_words(r, identity);
    let descriptor: [u8; 0x70] = fields::read_unchecked(entry, 0);
    let written = EntryIdentity { entry: identity.entry_bytes(&descriptor), mirror: identity.d_mirror() };
    write_entry_identity(entry, &written);
    written.entry
}

/// The Mii identity a planned Mii group carries: the lane target's configuration
/// staged over the window's current descriptor, exactly the bytes that owner's
/// canonical exact-Mii request would INSTALL (`staged_descriptor_for_owner`:
/// Player0 keeps the window's record index, a selector takes its slot). `None`
/// leaves the Mii canonical (no layout, no configuration, an unstageable owner).
fn mii_identity_for(bank: &Bank, lane: &[Option<crate::runtime::roster::cycle_preload::Target>; crate::runtime::roster::cycle_preload::MAX_TARGETS],
    kind: u32) -> Option<(MiiIdentity, Configuration)> {
    let layout = super::mii_backend::current()?;
    let (observed, _) = bank.observed?;
    let configuration = lane.iter().flatten().find(|t| t.kind == kind)?.configuration?;
    let staged = crate::engine::mii::ordinary_cpu::staged_descriptor_for_owner(layout, bank.owner as usize, observed, Some(configuration))?;
    Some((MiiIdentity::from_descriptor(&staged)?, configuration))
}

/// The identity and configuration of the reachable Mii group holding `kind`: the
/// adapter's proof, independent of its own request bytes, that the group was
/// planned for exactly this configuration (same tokens, same CharInfo) and that
/// its prefetch registered this hat (`HatPublication::validate_bank`).
pub(super) fn reachable_mii_group(owner: u32, kind: u32) -> Option<(MiiIdentity, Configuration)> {
    let role = role(owner)?;
    BANKS[role].lock().unwrap().as_ref().and_then(|bank| bank.groups.iter()
        .find(|g| g.state.reachable() && g.members.special() == Special::Mii && g.members.index_of(kind).is_some())
        .and_then(|g| g.identity.zip(g.configuration)))
}

/// Interim canonical-before-mutation rule (WP-M2, WORKPLAN §2.11 step 4): a Mii
/// target may take its one-member group only when the native Mii-setup preflight
/// holds for the identity the bank would install (fingerprints, owner table, record
/// bounds, published CharInfo, `D+0x94 == 0`, no other Mii owner on the record, entry
/// Ready). Otherwise the adapter keeps the canonical Reload; the group stays planned.
/// Phase B re-proves the same clauses with the entry in the bank's parked change
/// (`mii_build_preflight`, `prepare_mii_group`; MII-PHASEB).
pub(super) fn mii_setup_admitted(entry: usize, owner: u32, kind: u32) -> Result<(), &'static str> {
    let role = role(owner).ok_or("Mii setup: owner has no bank role")?;
    let (alias, identity, c) = {
        let state = BANKS[role].lock().unwrap();
        let bank = state.as_ref().filter(|b| b.owner == owner && !b.departed).ok_or("Mii setup: bank absent")?;
        let (identity, c) = bank.groups.iter()
            .find(|g| g.state.reachable() && g.members.special() == Special::Mii && g.members.index_of(kind).is_some())
            .and_then(|g| g.identity.zip(g.configuration)).ok_or("Mii setup: no reachable Mii group")?;
        (bank.alias, identity, c)
    };
    let base = BASE.load(Ordering::Acquire);
    if base == 0 || alias == 0 { return Err("Mii setup: bank not installed"); }
    unsafe { crate::runtime::roster::mii_setup::admitted(base, entry, owner, alias, &identity, &c) }
}

/// A planned Mii group whose configuration's CharInfo is not yet on the owner's
/// selector record. The key request copies that record at `R+0x68` (`0x17e64d0`,
/// `0x17e6c10`) and builds the face from it, so the adapter publishes the bytes
/// (G, its own receipt adopts them) before the bank prefetches the group.
pub(super) fn pending_mii_publication(owner: u32) -> Option<(usize, MiiIdentity, Configuration)> {
    let role = role(owner)?;
    let state = BANKS[role].lock().unwrap();
    let bank = state.as_ref().filter(|b| b.owner == owner && b.phase == Phase::Ready && !b.departed
        && b.swap.is_none() && b.prefetch.is_none())?;
    bank.groups.iter().enumerate().find_map(|(i, g)| {
        if g.state != GroupState::Planned || g.charinfo_published { return None; }
        let (identity, c) = g.identity.zip(g.configuration)?;
        Some((i, identity, c))
    })
}

/// The adapter's record publication outcome for `pending_mii_publication`: `Ok`
/// admits the group's prefetch. A refusal is classified where it was produced
/// (`body_bank::mii_publish`): a read/write race counts an attempt and fails the
/// group after `MII_PUBLISH_ATTEMPTS`, so a persistently racing owner never
/// spins; a refusal that cannot clear while the owner idles fails the group at
/// once. Returns the reason to note when it differs from the group's previous
/// refusal (one line per reason, not one per frame); the fallback line below
/// carries the last reason itself.
pub(super) fn mii_charinfo_published(owner: u32, group: usize, identity: MiiIdentity, outcome: Result<(), PublishRefusal>) -> Option<&'static str> {
    let role = role(owner)?;
    let (failed, note) = {
        let mut state = BANKS[role].lock().unwrap();
        let bank = state.as_mut().filter(|b| b.owner == owner)?;
        let g = bank.groups.get_mut(group).filter(|g| g.state == GroupState::Planned && g.identity == Some(identity))?;
        match outcome {
            Ok(()) => {
                g.charinfo_published = true;
                trace!("[GroupSwap] published owner={} group={} kind={:#x} index={} tokens={:?} charinfo=record",
                    owner, group, identity.kind(), identity.index, identity.tokens);
                (false, None)
            }
            Err(refusal) => match g.publish.refused(refusal) {
                PublishStep::Retry { note } => (false, note),
                PublishStep::Fail { refusals, reason } => {
                    trace!("[GroupSwap] fallback owner={} group={} kind={:#x} reason=Mii record publication refused {} times, last: {}",
                        owner, group, identity.kind(), refusals, reason);
                    (true, None)
                }
            },
        }
    };
    if failed { fail_group(role, group, "Mii record publication refused"); }
    note
}

/// The Ice Climbers pair bank ships. The B119 stall (loader state 11, member 2 never
/// loads; IC119-RUNTIME.md) was the vanilla pair reference count `R+0x50 = 2` under a
/// three-body window: the construction tail `0x60de60` resets R after the second
/// body (see `prepare_resources` and `body_bank::pair_resource_references`).
const IC_PAIR_BANK_SHIPPED: bool = true;

fn pair_enabled(settings: &crate::engine::preload::cycle_preload_config::Settings) -> bool {
    IC_PAIR_BANK_SHIPPED && settings.pair_bank_enabled() && PAIR_ENABLED.load(Ordering::Acquire)
}

/// Window members for a requested form: the normal pair gets a pair-lead bank
/// (first eligible upcoming kind in cell 2) when the pair bank is on; every other
/// single form uses the ordinary planner.
fn members_for(settings: &crate::engine::preload::cycle_preload_config::Settings, forms: Forms,
    upcoming: &[u32], extra_budget: usize) -> Option<Members> {
    if forms == Forms::starting(POPO) {
        return if pair_enabled(settings) { Members::from_pair(upcoming, extra_budget) } else { None };
    }
    Members::from_forms(forms, upcoming, extra_budget)
}

/// Row 4: the lane entries a window may take as members (`body_bank::member_lane`, host-tested),
/// with this role's proof that a lane `0x4b` is Solo Popo: the role's numbered form is Solo
/// (`requested_forms_at`, the form every numbered slot of the role takes; the pair is never a cell
/// 1–2 member, `Members::from_pair` builds its own window), and the entry is not the role's
/// canonical-only original: with equal original and cycle forms the cycle roster excludes Ice
/// Climbers (`roster_grid::cycle_exclusion`), so that entry is the slot-0 original, whose return is
/// the canonical Reload (`original_bankable(0x4b)` is false). The form of every kept `0x4b` is still
/// re-proven per request (`group_serves`). No allocation.
fn window_lane(menu: &crate::engine::compat::menu::TrainingModpackMenu, role: usize, upcoming: &[u32])
    -> ([u32; crate::runtime::roster::cycle_preload::MAX_TARGETS], usize) {
    let logical = if role == 0 { super::Role::Player } else { super::Role::Cpu };
    let solo = canonical_original(menu, role) != Some(POPO)
        && super::requested_forms_at(menu, logical, POPO, Some(0)) == Forms::solo_popo();
    crate::engine::swap::body_bank::member_lane(upcoming, solo)
}

/// One shared warm budget. Reserve upcoming native banks before helper loads,
/// but report/filter only banks actually built. Construction is demand-driven:
/// the first selected fighter loads its upcoming members; later hits reuse them.
/// With group swap on, the shared budget is `ready_fighters` (bodies across both
/// roles, active fighters included): window groups first, then Kirby-hat
/// neighbours of a Kirby role, then extra groups by cycle depth; helper slots
/// get what remains of that budget, capped by the saved slot count.
pub(crate) fn plan_budget(menu: &crate::engine::compat::menu::TrainingModpackMenu,
    lanes: &[[Option<crate::runtime::roster::cycle_preload::Target>; crate::runtime::roster::cycle_preload::MAX_TARGETS]; 4])
    -> [Option<Members>; 2] {
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    let mut wanted = [0; 2];
    let mut wanted_banks = [None; 2];
    let grouped = groups_enabled(&settings);
    if ENABLED.load(Ordering::Acquire) && settings.fighters && settings.bank_members >= 2 {
        for (role, lane) in lanes[2..].iter().enumerate() {
            // CPU-WINDOW-REFORM: with group swap off a cycling role's window demand is what its whole cycle can use
            // (`body_bank::window_share`), not what the next `MAX_TARGETS` entries can: a lookahead of special kinds
            // (Pyra, PT, IC, the Miis) must not give its share to helper sources, which stay resident for the scene
            // (Yoga swoff2 steps 79-97).
            if !grouped && lane.iter().any(Option::is_some) {
                let roster = if role == 0 { menu.player_roster.fighters() } else { menu.cpu_roster.fighters() };
                let cycle: Vec<u32> = roster.iter().map(|fighter| fighter.kind()).collect();
                // The `window_lane` rule: a cycle `0x4b` is a member only as this role's Solo Popo.
                let logical = if role == 0 { super::Role::Player } else { super::Role::Cpu };
                let solo = canonical_original(menu, role) != Some(POPO)
                    && super::requested_forms_at(menu, logical, POPO, Some(0)) == Forms::solo_popo();
                wanted[role] = crate::engine::swap::body_bank::window_share::cycle_window_demand(
                    &cycle, settings.bank_members, solo);
            }
            let kinds = lane.map(|target| target.map_or(u32::MAX, |t| t.kind));
            for (i, &kind) in kinds.iter().enumerate() {
                // Lookahead retains kind, not original/numbered-slot identity.
                // When those two IC choices differ, do not reserve a bank on
                // this ambiguous value. The actual staged request below still
                // uses its exact form and the remaining shared body budget.
                if role == 0 && kind == 0x4b
                    && super::original_kind(super::Role::Player) == Some(0x4b)
                    && menu.solo_popo_original != menu.solo_popo_player_cycle {
                    continue;
                }
                let forms = super::requested_forms_at(menu,
                    if role == 0 { super::Role::Player } else { super::Role::Cpu }, kind, Some(i));
                let (members_lane, count) = window_lane(menu, role, &kinds[i + 1..]);
                if let Some(members) = members_for(&settings, forms, &members_lane[..count],
                    usize::from(settings.bank_members - 1)) {
                    wanted[role] = wanted[role].max(members.count() - 1);
                    wanted_banks[role] = Some((members, kind));
                    break;
                }
            }
        }
    }
    let live = std::array::from_fn(|role| usize::from(EXTRAS[role].load(Ordering::Acquire)));
    // Window groups draw from the ready-fighter budget when grouping. Otherwise they reserve out of the saved slot
    // count before the helpers (`body_bank::window_share::groups_off_shares`; the helpers get the rest through
    // `RESERVED`): the share was `slots − warm helpers` before CPU-WINDOW-REFORM, so the helpers, whose sources stay
    // resident for the scene, kept whatever a window-less step let them take. Whether a window is built at a request
    // is still decided on the physical state (`proposed`: slots less the helpers present).
    let window_pool = if grouped {
        usize::from(settings.ready_fighters).saturating_sub(2)
    } else {
        usize::from(settings.slots)
    };
    let reserved = crate::engine::swap::body_bank::reserve_extras(wanted, live, window_pool);
    let originals = [excluded_original(&settings, menu, 0), excluded_original(&settings, menu, 1)];
    let mut group_targets = [0u8; 2];
    let mut group_bodies = 0usize;
    if grouped {
        group_bodies = plan_groups(&settings, menu, lanes, originals, &mut group_targets);
    }
    for (role, value) in reserved.iter().enumerate() {
        ROLE_BUDGET[role].store(*value as u8, Ordering::Release);
        let targets = wanted_banks[role].map_or(0, |(members, target)| {
            // A pair-lead bank plans one selectable extra (cell 2); Nana is not a target. The
            // target keeps its own cell (`CellRule::PinCell0` puts Kirby or Solo Popo at cell 0),
            // so the reserved members are the target plus its first `value` window neighbours,
            // never "the first cells" with cell 0 taken as the active fighter.
            let (kinds, count) = members.logical_kinds();
            let extras = if members.is_pair_lead() { count } else { *value };
            let shown: Vec<u32> = std::iter::once(target)
                .chain(kinds[..count].iter().copied().filter(|kind| *kind != target).take(extras)).collect();
            crate::engine::preload::cycle_preload_capacity::reusable_extras(&shown, target, originals[role])
        });
        PLANNED_TARGETS[role].store(targets.saturating_add(group_targets[role]), Ordering::Release);
    }
    let window_extras = reserved.iter().sum::<usize>();
    let helper_reserved = if grouped {
        usize::from(settings.slots)
            .saturating_sub(window_pool.saturating_sub(window_extras + group_bodies))
    } else {
        window_extras
    };
    RESERVED.store(helper_reserved.min(usize::from(u8::MAX)) as u8, Ordering::Release);
    // A planned kind alone is not a loaded body. This is a value receipt from
    // the current native callbacks; no pointer is followed on the planner thread.
    std::array::from_fn(|role| BANKS[role].lock().unwrap().as_ref()
        // This filter reserves occupied native resources, not UI readiness.
        // A failed/lapsed receipt still owns its cells until native retirement.
        .filter(|bank| bank.phase == Phase::Ready && !bank.departed)
        .map(|bank| *bank.members))
}

/// Value-only group planning on the sparse planner: appends `Planned` groups to
/// a Ready window bank; never removes or touches a native object. Returns the
/// extra-group bodies charged to the shared budget and the per-role UI targets.
fn plan_groups(settings: &crate::engine::preload::cycle_preload_config::Settings,
    menu: &crate::engine::compat::menu::TrainingModpackMenu,
    lanes: &[[Option<crate::runtime::roster::cycle_preload::Target>; crate::runtime::roster::cycle_preload::MAX_TARGETS]; 4],
    originals: [Option<u32>; 2], targets: &mut [u8; 2]) -> usize {
    let capacity = POOL_CAPACITY.load(Ordering::Acquire) as usize;
    let live = POOL_LIVE.load(Ordering::Acquire) as usize;
    let pool_free = if capacity == 0 { 0 } else { capacity.saturating_sub(live).saturating_sub(POOL_RESERVE) };
    let hats_wanted = lanes[..2].iter().flatten().flatten().count().min(usize::from(settings.slots));
    let policy = GroupPolicy {
        solo_popo_groups: settings.solo_popo_groups_enabled(),
        ic_pair_groups: settings.ic_pair_groups_enabled() && PAIR_ENABLED.load(Ordering::Acquire),
    };
    let special = special_policy(settings);
    let mut windows = [None; 2];
    let mut existing: [Vec<Members>; 2] = [Vec::new(), Vec::new()];
    let mut alias = [0usize; 2];
    for role in 0..2 {
        let state = BANKS[role].lock().unwrap();
        if let Some(bank) = state.as_ref().filter(|bank| bank.phase == Phase::Ready && !bank.departed) {
            windows[role] = Some(*bank.members);
            alias[role] = bank.alias;
            existing[role] = bank.groups.iter().filter(|g| g.state.counts_bodies()).map(|g| *g.members).collect();
        }
    }
    // A lane's Ice Climbers entry is a group candidate only when the role's
    // original is not Ice Climbers (whose slot-0 form may differ from the
    // cycle's, `plan_budget`) and the numbered slot's configured form matches the
    // switch that admits it: the normal pair for a pair group (`pair_groups`),
    // Solo Popo for a Solo-led ordinary group (`solo_popo_groups`); the pair
    // group wins when both are on.
    let upcoming: [Vec<u32>; 2] = std::array::from_fn(|role| {
        let logical_role = if role == 0 { super::Role::Player } else { super::Role::Cpu };
        lanes[2 + role].iter().enumerate().filter_map(|(i, t)| t.map(|t| (i, t)))
            // Configured targets stay canonical, except a lane Mii under the Mii-group policy.
            .filter(|(_, t)| t.configuration.is_none() || (special.mii && (0x48..=0x4a).contains(&t.kind)))
            .filter(|(i, t)| t.kind != POPO
                || (super::original_kind(logical_role) != Some(POPO) && {
                    let form = super::requested_forms_at(menu, logical_role, POPO, Some(*i));
                    if special.pair { form == Forms::starting(POPO) }
                    else { policy.solo_popo_groups && form == Forms::solo_popo() }
                }))
            .map(|(_, t)| t.kind).collect()
    });
    let planned = plan_extra_groups_with(policy, windows, [&existing[0], &existing[1]], [&upcoming[0], &upcoming[1]],
        originals, GroupBudget { ready_fighters: usize::from(settings.ready_fighters), hats_wanted, pool_free, special });
    let mut bodies = 0usize;
    for role in 0..2 {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut().filter(|bank| bank.phase == Phase::Ready && !bank.departed) else { continue; };
        for members in planned[role].iter().flatten() {
            if bank.groups.len() >= MAX_EXTRA_GROUPS { break; }
            if bank.groups.iter().any(|g| g.members.index_of(members.kinds()[0]).is_some()) { continue; }
            let identity = if members.carries_identity() {
                match mii_identity_for(bank, &lanes[2 + role], members.kinds()[0]) {
                    Some(identity) => Some(identity),
                    None => {
                        trace!("[GroupSwap] plan owner={} skipped Mii kind={:#x} reason=no stageable identity", bank.owner, members.kinds()[0]);
                        continue;
                    }
                }
            } else { None };
            bank.groups.push(Group::planned_with(*members, identity));
            trace!("[GroupSwap] plan owner={} group={} kinds={:x?} special={:?} bodies={} ready_fighters={} pool={}/{} reserve={} hats={} alias={:#x} identity={:?}",
                bank.owner, bank.groups.len() - 1, &members.kinds()[..members.count()], members.special(), members.count(),
                settings.ready_fighters, live, capacity, POOL_RESERVE, hats_wanted, alias[role],
                identity.map(|(i, _)| (i.index, i.tokens)));
        }
        let counts = count_extra_groups(&bank.groups.iter().map(|g| (*g.members, g.state)).collect::<Vec<_>>());
        targets[role] = counts.planned;
        let role_bodies: usize = bank.groups.iter().filter(|g| g.state.counts_bodies()).map(|g| g.members.count()).sum();
        GROUP_BODIES[role].store(role_bodies.min(usize::from(u8::MAX)) as u8, Ordering::Release);
        bodies += role_bodies;
    }
    bodies
}

/// Pending targets stay in the denominator. Only a current native Ready receipt
/// contributes to `ready` (built), a prefetched group is counted apart (loaded, not built),
/// and helper assets are counted separately. Extra groups count every member: built
/// groups are ready, prefetched ones prefetched, planned or prefetching ones pending and
/// failed groups stay. A role without a bank has nothing running: its planned fighters are
/// `unstarted` and the first swap builds their window (`role_counts`, host-tested).
pub(crate) fn progress() -> crate::engine::preload::cycle_preload_policy::BuiltFighters {
    use crate::engine::preload::cycle_preload_policy::BuiltFighters;
    use crate::engine::swap::body_bank::{role_counts, WindowStatus};
    use crate::engine::preload::cycle_preload_capacity::reusable_extras;
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    let mut result = BuiltFighters::default();
    if settings.slots == 0 || !settings.fighters || settings.bank_members < 2 { return result; }
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    // Read roster metadata before BANKS; never acquire a roster lock inside it.
    let originals = [excluded_original(&settings, &menu, 0), excluded_original(&settings, &menu, 1)];
    for (role, state) in BANKS.iter().enumerate() {
        let planned = PLANNED_TARGETS[role].load(Ordering::Acquire);
        let counts = match state.lock().unwrap().as_ref()
            .filter(|bank| bank.phase != Phase::Retired && !bank.departed) {
            Some(bank) => {
                let (kinds, count) = bank.members.logical_kinds();
                let active = bank.observed.map_or(bank.members.kinds()[usize::from(bank.target)], |(_, kind)| kind);
                let extra = reusable_extras(&kinds[..count], active, originals[role]);
                let groups = count_extra_groups(&bank.groups.iter().map(|g| (*g.members, g.state)).collect::<Vec<_>>());
                let window = if bank.failed { WindowStatus::Failed }
                    else if bank.phase == Phase::Ready && bank.observed.is_some() { WindowStatus::Built }
                    else { WindowStatus::Busy };
                role_counts(planned, extra, window, groups)
            }
            None => role_counts(planned, 0, WindowStatus::Absent, Default::default()),
        };
        result.planned = result.planned.saturating_add(counts.planned);
        result.ready = result.ready.saturating_add(counts.ready);
        result.prefetched = result.prefetched.saturating_add(counts.prefetched);
        result.failed = result.failed.saturating_add(counts.failed);
        result.pending = result.pending.saturating_add(counts.pending);
        // The Waiting text names the role whose own swap its unstarted fighters wait for.
        result.swap |= counts.swap_owed(role);
    }
    result
}

/// Value-only menu hint for a numbered cycle choice, using the same native
/// receipt as the ready count. It never follows a remembered native pointer.
/// The active body is not a parked alternative, nor is a canonical-original
/// reload; a banked original (`original_bank`) is a member like any other.
/// A built member of a detached group is a warm swap target too.
pub(crate) fn cycle_ready(role: usize, kind: u32, solo: bool) -> bool {
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if role >= BANKS.len() || settings.slots == 0 || !settings.fighters
        || settings.bank_members < 2 { return false; }
    let forms = if kind == 0x4b && solo { Forms::solo_popo() } else { Forms::starting(kind) };
    let logical_role = if role == 0 { super::Role::Player } else { super::Role::Cpu };
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    if super::original_kind(logical_role) == Some(kind)
        && super::requested_forms_at(&menu, logical_role, kind, None) == forms
        && !(settings.original_bank_enabled() && original_bankable(kind)) { return false; }
    let grouped = groups_enabled(&settings);
    // The same rule as `plan()`/`switch()` (WP-I): the holding window or group serves the kind in
    // the requested form (`group_serves`), so a parked pair-lead window reads as ready for the
    // pair and never for Solo. Every window can be left warm (a one-member one by its self-commit).
    BANKS[role].lock().unwrap().as_ref().is_some_and(|bank|
        bank.phase == Phase::Ready && !bank.departed && !bank.failed
            && bank.observed.is_some_and(|(_, active)| active != kind)
            && (group_serves(*bank.members, kind, forms).is_some()
                || (grouped
                    && bank.groups.iter().any(|g| g.state == GroupState::Built && group_serves(*g.members, kind, forms).is_some()))))
}

/// A kind held by any of this role's groups (window or extra): helper look-ahead
/// must not duplicate it.
pub(crate) fn group_member(role: usize, kind: u32) -> bool {
    role < BANKS.len() && BANKS[role].lock().unwrap().as_ref().is_some_and(|bank|
        !bank.departed && bank.groups.iter().any(|g| g.state.counts_bodies() && g.members.index_of(kind).is_some()))
}

/// A group exchange leaves the shared R header unchanged. Every adapter owner
/// walk uses this exact receipt, including walks initiated by the other role.
/// None preserves the ordinary header comparison; Some(false) never bypasses it.
/// The receipt tolerates only the entry's natively owned live behavior bytes
/// (Training CPU Behavior writes FE+0x10..0x14 and FE+0x34..=0x37), each inside
/// its native domain (`GroupExchange::permits`); a refusal here fails both roles'
/// walks, since the Player walk checks every physical owner (C118 D5).
pub(super) unsafe fn group_header(entry: usize, owner: u32, resource: usize) -> Option<bool> {
    let state = BANKS[role(owner)?].lock().unwrap();
    let bank = state.as_ref().filter(|b| b.entry == entry && b.owner == owner)?;
    let exchange = bank.exchange?;
    Some(ready_group_owner(entry as *const u8, bank)
        .is_some_and(|now| now.resource == resource && exchange.permits(now)))
}

/// Rebaseline only the exact before/after pair minted by our verified exchange.
/// G, adapter ticket/epoch, protected rows and INSTALL are fenced by the caller.
pub(super) unsafe fn group_rebaseline(entry: usize, owner: u32, epoch: u64,
    before: &[u8; 0x70], after: &[u8; 0x70]) -> bool {
    let Some(role) = role(owner) else { return false; };
    let state = BANKS[role].lock().unwrap();
    let Some(bank) = state.as_ref().filter(|b| b.entry == entry && b.owner == owner) else { return false; };
    bank.exchange.is_some_and(|exchange| ready_group_owner(entry as *const u8, bank)
        .is_some_and(|now| now.epoch == epoch && now.descriptor == *after && exchange.rebaseline(before, now)))
}

pub(crate) fn helper_budget(slots: u8) -> u8 { slots.saturating_sub(RESERVED.load(Ordering::Acquire)) }

pub(super) fn proposed(owner: u32, target: Forms, upcoming: &[u32]) -> Option<Members> {
    let role = role(owner)?;
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if !ENABLED.load(Ordering::Acquire) || !settings.fighters || settings.bank_members < 2 { return None; }
    let other = usize::from(EXTRAS[1 - role].load(Ordering::Acquire));
    // A new window bank for this role replaces its own extra groups (torn down
    // before the Reload), so only the other role's groups are charged.
    let (pool, other_total) = if groups_enabled(&settings) {
        (usize::from(settings.ready_fighters).saturating_sub(2),
            other + usize::from(GROUP_BODIES[1 - role].load(Ordering::Acquire)))
    } else {
        (usize::from(settings.slots).saturating_sub(super::sources::warm_occupancy()), other)
    };
    let budget = pool.saturating_sub(other_total).min(usize::from(settings.bank_members - 1));
    // Before the first sparse plan, the actual free budget is authoritative.
    // Once planned, preserve the other role's fair share as well.
    let planned_other = usize::from(ROLE_BUDGET[1 - role].load(Ordering::Acquire)).max(other);
    let budget = budget.min(pool.saturating_sub(planned_other + (other_total - other)));
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    let (lane, count) = window_lane(&menu, role, upcoming);
    members_for(&settings, target, &lane[..count], budget)
}

/// Actual native descriptor plus the selected logical fighter. Exact-Mii
/// transactions retain their original descriptor when no ordinary bank applies.
#[derive(Clone, Copy, Debug)]
pub(super) struct Plan {
    pub descriptor: [u8; 0x70],
    pub target: u32,
    /// The requested form of `target` (the pair or Solo for `0x4b`, the native expansion for a
    /// PT/Aegis form, the single body otherwise): the warm path serves it only in this form (WP-I).
    pub forms: Forms,
    /// The window's physical cell layout and the target's cell in it (the cell map: the rotation
    /// order is the lane's, the layout is `Members::laid_out`'s — Kirby at cell 0,
    /// `CellRule::PinCell0`). Receipts carry both; `u32::MAX` pads unused cells.
    pub cells: [u32; NATIVE_MEMBERS],
    pub active: u8,
    pub reuse: bool,
    members: Option<Members>,
}

impl Plan {
    pub(super) fn is_bank(&self) -> bool { self.members.is_some() }
}

/// Short label of a requested form for the request diagnostics (I1 row: `form=pair`).
fn form_label(forms: Forms) -> &'static str {
    if forms == Forms::solo_popo() { "solo" }
    else if forms.actor_count == 2 { "pair" }
    else if forms.count > 1 { "native" }
    else { "single" }
}

/// Called at the request's ready preflight, before publishing a transaction.
/// Native-origin options/configuration bytes are preserved byte-for-byte.
/// A target in a built or prefetched extra group is a reuse (group swap).
pub(super) unsafe fn plan(entry: *const u8, owner: u32, descriptor: [u8; 0x70], upcoming: &[u32]) -> Plan {
    let target = u32::from_le_bytes(descriptor[0x18..0x1c].try_into().unwrap());
    let mut result = Plan { descriptor, target, forms: Forms::starting(target), cells: [u32::MAX; NATIVE_MEMBERS], active: 0,
        reuse: false, members: None };
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if settings.slots == 0 || !settings.fighters || settings.bank_members < 2
        || !ENABLED.load(Ordering::Acquire) { return result; }
    // Both reuse and construction require the complete requested form. Checking
    // just the kind would reuse solo Popo after Solo was switched off, or a Solo
    // bank for the normal pair. The normal pair qualifies only for a pair-lead bank.
    let word = |offset| u32::from_le_bytes(descriptor[offset..offset + 4].try_into().unwrap());
    let request = (word(0x14), [target, word(0x1c), word(0x20)], word(0x28));
    let forms = match crate::engine::swap::body_bank::single_member_request(request.0, request.1, request.2) {
        Some(forms) => forms,
        None if pair_request(request.0, request.1, request.2) && pair_enabled(&settings) => Forms::starting(POPO),
        // A native-group target (PT form, Pyra, Mythra) may be served by its own
        // group; a window bank never adopts it (`proposed` -> `from_forms` -> None).
        None => match native_group_request(request.0, request.1, request.2) {
            Some(forms) if special_policy(&settings).native => forms,
            // A single-Mii request may be served by a Mii group of the same identity.
            _ => match crate::engine::swap::body_bank::mii_group_request(request.0, request.1, request.2) {
                Some(forms) if special_policy(&settings).mii => forms,
                _ => return result,
            },
        },
    };
    result.forms = forms;
    if let Some(members) = ready_members(entry, owner) {
        let in_window = group_serves(members, target, forms).is_some();
        // The holding group's own form of the target decides (C116.1, B123-pre row 4
        // O1): a parked pair-lead window or a pair group answers the normal-pair
        // request, a Solo-led group the Solo request; `member_forms(target)` alone
        // never yields the pair, so the return from a group to the pair window took
        // the canonical Reload (Ally S4 L7002, 3.2 s) instead of the warm swap.
        let reachable = !in_window && groups_enabled(&settings) && group_reachable(owner, target, forms)
            // A Mii group serves only its own identity (record index, tokens,
            // costume, flag bytes): another configuration of the class is canonical.
            && (!(0x48..=0x4a).contains(&target) || group_identity_matches(owner, target, &descriptor));
        // Every window can be left warm: a one-member window commits to its own cell
        // (`ExitRule::SelfCell`, `switch`), every other window to its exit rule's cell.
        let in_group = reachable;
        // Parking the pair needs both Climbers alive and settled (addendum gates),
        // for an in-window change to the extra and for a group swap away from the
        // active pair alike; otherwise this request takes the Reload path below.
        let gated = (in_window || in_group) && members.is_pair()
            && match pair_switch_gate(entry, owner, members, target) {
                Ok(()) => false,
                Err(reason) => {
                    trace!("[IcPair] fallback owner={} target={:#x} reason={}; canonical Reload", owner, target, reason);
                    true
                }
            };
        // A live Kirby parks only copy-free (EXCLUSIONS §2); a refusal is today's
        // Reload, which destroys this window and its groups as it always did.
        let gated = gated || ((in_window || in_group) && target != KIRBY
            && members.ready_index(snapshot(entry)).is_some_and(|from|
                match kirby_park_gate(entry, owner, members, from) {
                    Ok(()) => false,
                    Err(reason) => {
                        trace!("[KirbyPark] fallback owner={} target={:#x} reason={}; canonical Reload", owner, target, reason);
                        true
                    }
                }));
        if (in_window || in_group) && !gated {
            let live = fields::read::<[u8; 0x70]>(entry as usize, 0);
            let mut physical = super::native_fighter_install::Descriptor::from_bytes(&descriptor);
            // A reuse's descriptor is the live entry's: the window's group words and the
            // starting-form byte `+0x43` its Reload requested (the target's cell under
            // `CellRule::PinCell0`; a Fighter request always carries 0). FACT (owned ELF): the Reload
            // start stages the request at `+0x70` and loads `+0x5918`/`+0x5919` from its `+0x43`
            // (`0x6533a8..0x653400`), loader state 4 copies the staged bytes to `+0x0` verbatim
            // (`native_entry_load_650010.c:344-357`), and no entry code writes `+0x43` again
            // (`popo-oneexit/re/entry_640000_660000.asm`: readers `0x6533f4`, `0x6548f4` only), so a
            // warm member change leaves it as built. Every other byte stays exact.
            physical.request_group_starting(super::native_fighter_install::Group {
                count: members.count() as u32, kinds: members.kinds(), actor_count: members.actor_count(),
            }, live.map_or(descriptor[0x43], |current| current[0x43]));
            // A Mii group's build rewrites exactly the identity bytes besides the group
            // words, so a Mii request matches the live entry once its identity is
            // applied to it (`mii_reuse_bytes_match`); the group carries that identity
            // (`group_identity_matches` above). Every other request is byte-exact.
            let matches = live.is_some_and(|current| current == *physical.bytes()
                || (in_group && (0x48..=0x4a).contains(&target) && mii_reuse_bytes_match(&current, &descriptor)));
            if matches {
                result.descriptor = *physical.bytes(); result.members = Some(members); result.reuse = true;
                result.cells = members.kinds(); result.active = members.index_of(target).unwrap_or(0);
                return result;
            }
            // A warm member exists but the request's non-group bytes differ from
            // the live entry (an Original request carries the origin descriptor):
            // the request takes the Reload path below; name the bytes for triage.
            if let Some(current) = fields::read::<[u8; 0x70]>(entry as usize, 0) {
                let differing: Vec<usize> = (0..0x70).filter(|i| current[*i] != physical.bytes()[*i]).collect();
                trace!("[BodyBank] reuse declined owner={} kind={:#x} reason=descriptor bytes differ at {:x?}; Reload path",
                    owner, target, differing);
            }
        }
    }
    if let Some(members) = proposed(owner, forms, upcoming) {
        // Cell map (CellRule::PinCell0): `Members::laid_out` put a pinned member (Kirby, Solo Popo) at cell 0
        // and the others in rotation order; the target activates through its own cell, requested
        // as the descriptor's starting form (`+0x43`), so the Reload reaches it at Ready.
        let active = members.index_of(target).unwrap_or(0);
        let mut physical = super::native_fighter_install::Descriptor::from_bytes(&descriptor);
        physical.request_group_starting(super::native_fighter_install::Group {
            count: members.count() as u32, kinds: members.kinds(), actor_count: members.actor_count(),
        }, active);
        result.descriptor = *physical.bytes(); result.members = Some(members);
        result.cells = members.kinds(); result.active = active;
        trace!("[BodyBank] layout owner={} target={:#x} cells={:x?} active={} form={}",
            owner, target, &members.kinds()[..members.count()], active, form_label(forms));
    }
    result
}

/// Leaving the active pair, for cell 2 or for a detached group, is allowed only
/// while both Climbers are state 4 and outside Dead, Rebirth, Entry and Standby.
/// Returning to the pair, or any change while the extra is active, has no extra
/// gate. Never called under the bank lock.
unsafe fn pair_switch_gate(entry: *const u8, owner: u32, members: Members, target: u32) -> Result<(), &'static str> {
    if members.index_of(target) == Some(0) || members.ready_index(snapshot(entry)) != Some(0) { return Ok(()); }
    pair_parkable(entry, owner)
}

/// Both pair bodies present, this owner's, active, with a status that may be
/// parked. Reads the status through each body's own module accessor (`+0x20`).
unsafe fn pair_parkable(entry: *const u8, owner: u32) -> Result<(), &'static str> {
    use smash::app::{lua_bind::StatusModule, BattleObjectModuleAccessor};
    use smash::lib::lua_const::{FIGHTER_STATUS_KIND_DEAD, FIGHTER_STATUS_KIND_ENTRY,
        FIGHTER_STATUS_KIND_REBIRTH, FIGHTER_STATUS_KIND_STANDBY};
    let base = BASE.load(Ordering::Acquire);
    for (cell, kind) in [(0x4160usize, POPO), (0x4168, NANA)] {
        let Some(body) = fields::child(entry as usize, cell) else { return Err("pair body missing"); };
        if fields::read::<usize>(body, 0) != Some(base + FIGHTER_VTABLE)
            || fields::read::<u32>(body, 0xc) != Some(kind)
            || fields::read::<u64>(body, 0x10) != Some(u64::from(owner)) { return Err("pair body identity"); }
        if fields::read::<u8>(body, 0x3a) != Some(4) { return Err("pair body not active"); }
        let Some(module) = fields::child(body, 0x20) else { return Err("pair accessor missing"); };
        let status = StatusModule::status_kind(module as *mut BattleObjectModuleAccessor);
        if [*FIGHTER_STATUS_KIND_DEAD, *FIGHTER_STATUS_KIND_REBIRTH,
            *FIGHTER_STATUS_KIND_ENTRY, *FIGHTER_STATUS_KIND_STANDBY].contains(&status) {
            return Err("pair status dead/rebirth/entry/standby");
        }
    }
    Ok(())
}

/// Copy-free departure gate for a live Kirby (EXCLUSIONS-AUDIT §2): a parked
/// copy-source lease is unqualified, so Kirby parks only while it holds no copy
/// ability (`FLAG_COPY` clear, no retained copy object groups, no hat lease on
/// this body). Any other live member passes. The cell's body is reached through
/// the current entry and its own module accessor (`+0x20`), never through a
/// remembered pointer; the decision itself is `body_bank::kirby_parkable`.
/// Never called under the bank lock (the source lock is taken inside).
unsafe fn kirby_park_gate(entry: *const u8, owner: u32, members: Members, from: u8) -> Result<(), &'static str> {
    use smash::app::{lua_bind::WorkModule, BattleObjectModuleAccessor};
    use smash::lib::lua_const::FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY;
    if members.kinds()[usize::from(from)] != KIRBY { return Ok(()); }
    let base = BASE.load(Ordering::Acquire);
    let Some(body) = fields::child(entry as usize, 0x4160 + usize::from(from) * 8) else { return Err("Kirby body missing"); };
    if fields::read::<usize>(body, 0) != Some(base + FIGHTER_VTABLE)
        || fields::read::<u32>(body, 0xc) != Some(KIRBY)
        || fields::read::<u64>(body, 0x10) != Some(u64::from(owner)) { return Err("Kirby body identity"); }
    if fields::read::<u8>(body, 0x3a) != Some(4) { return Err("Kirby body not active"); }
    let Some(id) = fields::read::<u32>(body, 8) else { return Err("Kirby body id"); };
    let Some(module) = fields::child(body, 0x20) else { return Err("Kirby accessor missing"); };
    let module = module as *mut BattleObjectModuleAccessor;
    let copy = WorkModule::is_flag(module, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY);
    let retained = super::sources::retained_copy_kinds(module);
    let leased = super::sources::hat_lease_on_body(id);
    crate::engine::swap::body_bank::kirby_parkable(copy, retained, leased)
}

/// D-KIRBY1 (D-KIRBY1-ROB1-RCA.md §3): the kind the window's live Kirby copies
/// right now (`FLAG_COPY` with `INT_COPY_CHARA`), or `None` when the live member
/// is not Kirby, holds no copy, or its cell fails the same identity proof as
/// `kirby_park_gate`. Value-only, through the current entry's cell and its own
/// module accessor; never called under the bank lock.
unsafe fn live_kirby_copy_kind(entry: *const u8, owner: u32, members: Members) -> Option<u32> {
    use smash::app::{lua_bind::WorkModule, BattleObjectModuleAccessor};
    use smash::lib::lua_const::{FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY, FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA};
    let (index, kind) = members.ready_live(snapshot(entry))?;
    if kind != KIRBY { return None; }
    let base = BASE.load(Ordering::Acquire);
    let body = fields::child(entry as usize, 0x4160 + usize::from(index) * 8)?;
    if fields::read::<usize>(body, 0) != Some(base + FIGHTER_VTABLE)
        || fields::read::<u32>(body, 0xc) != Some(KIRBY)
        || fields::read::<u64>(body, 0x10) != Some(u64::from(owner))
        || fields::read::<u8>(body, 0x3a) != Some(4) { return None; }
    let module = fields::child(body, 0x20)? as *mut BattleObjectModuleAccessor;
    WorkModule::is_flag(module, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY)
        .then(|| WorkModule::get_int(module, *FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA) as u32)
}

fn ledger_role(role: usize) -> LedgerRole { LedgerRole::from_index(role).unwrap_or(LedgerRole::Player) }

/// Value snapshot of this role's consumers for the ledger (WP4, `swap_model::occupancy`): the live
/// kind, the parked kinds (window cells and Built groups), the kinds whose prefetch keys rest on R
/// (groups that `holds_keys()`), and the live Kirby's copy (`copy`, read before the bank lock by
/// `live_kirby_copy_kind`). Hat-source and helper kinds stay empty until the sources expose them
/// (the K2 rule is inert, HYP); `never_activated_kirby` is WP3/WP-K's to fill.
fn occupancy_of(bank: &Bank, role: usize, copy: Option<u32>) -> Occupancy {
    let mut kinds = KindOccupancy::default();
    kinds.live[role] = bank.observed.map(|(_, kind)| kind);
    for kind in bank.members.kinds().iter().take(bank.members.count()) { kinds.parked[role].insert(*kind); }
    for group in &bank.groups {
        if group.state == GroupState::Built {
            for kind in group.members.kinds().iter().take(group.members.count()) { kinds.parked[role].insert(*kind); }
        }
        if group.state.holds_keys() {
            let (prefetch, count) = group.members.prefetch_kinds();
            for kind in &prefetch[..count] { kinds.prefetched[role].insert(*kind); }
        }
    }
    kinds.kirby_copy[role] = copy;
    Occupancy { kinds, rules: Rules::C116_1 }
}

/// [`occupancy_of`] for the role's current bank (takes and releases the bank lock).
fn occupancy_snapshot(role: usize, copy: Option<u32>) -> Occupancy {
    BANKS[role].lock().unwrap().as_ref().map_or_else(Occupancy::default, |bank| occupancy_of(bank, role, copy))
}

/// Release the prefetch keys the ledger holds for `group` (`None` = every group), restricted to
/// `kinds` when given, through `r_handle::release` (an Aegis orphan key first, D-SG1), except a key
/// whose kind the live Kirby copies: `0x17e8610(R, k)` also releases R's G68 mirror for
/// `(k, costume)`, which that copy shares (RCA §3), so the ledger keeps such a token as a retained
/// obligation and it is reported here in the line the E4 rows read. Returns `(released, kept)`.
/// Never called under the bank lock.
unsafe fn release_group_keys(base: usize, role: usize, group: Option<usize>, kinds: Option<&[u32]>,
    occupancy: &Occupancy, owner: u32, reason: &str) -> (usize, usize) {
    let receipt = r_handle::release(base, ledger_role(role), group, kinds, occupancy, reason);
    for (kind, why) in &receipt.kept {
        trace!("[GroupSwap] key kept owner={} kind={:#x} reason={} ({}); D-KIRBY1 invariant violated",
            owner, kind, why, reason);
    }
    (receipt.released, receipt.kept.len())
}

/// The Mii identity a detached group carries (`None` for every other group).
fn group_identity(role: usize, group: usize) -> Option<MiiIdentity> {
    BANKS[role].lock().unwrap().as_ref().and_then(|bank| bank.groups.get(group)).and_then(|g| g.identity)
}

/// [`release_group_keys`] for one group, with that group's Mii identity in `r` while its keys go
/// (MII-GROUPS-DESIGN §3.2: the release selector `0x17e8410` keys on `R+0x58` like the request
/// did; `with_mii_identity` puts the window's words back after). Every per-group key return outside
/// a build goes through here: the drain before a Kirby or pair-rule target, the live-Kirby return
/// and a failed prefetch. Never called under the bank lock.
unsafe fn release_group_keys_of(base: usize, role: usize, r: usize, group: usize, occupancy: &Occupancy,
    owner: u32, reason: &str) -> (usize, usize) {
    let mut outcome = (0usize, 0usize);
    with_mii_identity(r, group_identity(role, group), || {
        outcome = release_group_keys(base, role, Some(group), None, occupancy, owner, reason);
    });
    outcome
}

/// U20 (`swap_model::fallback`): a swap fallback that failed a Prefetched group returns every key
/// the group holds, the exact inverse of its prefetch, on the R the ledger bound then (`alias`).
/// The `[Ledger] reconcile … phase=fallback` line follows when that R is still the owner's.
/// Never called under the bank lock.
unsafe fn return_group_keys(base: usize, entry: *mut u8, owner: u32, role: usize, returning: (usize, usize, Members), why: &str) {
    let (group, alias, window) = returning;
    let copy = live_kirby_copy_kind(entry, owner, window);
    let occupancy = occupancy_snapshot(role, copy);
    let (released, kept) = if alias != 0 { release_group_keys_of(base, role, alias, group, &occupancy, owner, why) } else { (0, 0) };
    trace!("[GroupSwap] keys returned owner={} group={} released={} kept={} reason=fallback ({}) map={}",
        owner, group, released, kept, why, fields::read::<u64>(alias, 0x18).unwrap_or(0));
    if alias != 0 && resource(base, owner) == Some(alias) { r_handle::reconcile(ledger_role(role), "fallback"); }
}

/// The ledger recipe a group member's acquisition takes: the pair rule for a `[Popo, Nana, …]`
/// group (its tails decrement `R+0x50`, C122), the construction tail otherwise; object kinds are
/// `OwnedPerKind` (no tail releases them, D-SG1).
fn member_recipe(members: Members, kind: u32) -> ReleaseRecipe {
    if members.object_kind() == Some(kind) {
        ReleaseRecipe::OwnedPerKind
    } else if let Some(references) = pair_resource_references(members) {
        ReleaseRecipe::PairReset { references: references as u8 }
    } else {
        ReleaseRecipe::ConstructionTail
    }
}

/// A built or prefetched extra group holds `kind` in the requested `forms`
/// (`Members::forms_of`: the pair for the Popo of a parked pair-lead window or a
/// pair group, Solo for a Solo-led group's Popo, the native group form for a PT
/// or Aegis member, the single form otherwise), no swap is pending and no
/// prefetch is in flight for this role.
fn group_reachable(owner: u32, kind: u32, forms: Forms) -> bool {
    let Some(role) = role(owner) else { return false; };
    BANKS[role].lock().unwrap().as_ref().is_some_and(|bank|
        bank.swap.is_none() && bank.prefetch.is_none() && !bank.departed
            && bank.groups.iter().any(|g| g.state.reachable() && group_serves(*g.members, kind, forms).is_some()))
}

/// The reachable Mii group holding `kind` carries exactly the identity this
/// request's staged descriptor names.
fn group_identity_matches(owner: u32, kind: u32, descriptor: &[u8; 0x70]) -> bool {
    let Some(role) = role(owner) else { return false; };
    let Some(wanted) = MiiIdentity::from_descriptor(descriptor) else { return false; };
    BANKS[role].lock().unwrap().as_ref().is_some_and(|bank|
        bank.groups.iter().any(|g| g.state.reachable() && g.members.index_of(kind).is_some() && g.identity == Some(wanted)))
}

/// Same current native callback as preflight, INSTALL owned, no intervening
/// scene transition. The caller publishes its exact receipt before this call.
pub(super) unsafe fn submit(entry: *mut u8, owner: u32, plan: Plan,
    pose: super::native_reload_pose::ReloadPose) -> bool {
    if plan.reuse {
        pose.stage();
        return switch(entry, owner, plan.target, plan.forms);
    }
    // A Reload replaces the window; every detached group goes first (§6.5 iii).
    before_reload(entry);
    if let Some(members) = plan.members {
        if !build(entry, owner, members) { return false; }
    }
    let descriptor = super::native_fighter_install::Descriptor::from_bytes(&plan.descriptor);
    super::reload_at_pose(BASE.load(Ordering::Acquire), entry, &descriptor, pose);
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Body { address: usize, id: u32 }

/// Every entry field a detached group needs restored (GROUP-SWAP-FEASIBILITY §2.2).
/// Values only; the pointers inside are restored into the same entry they came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    installed: u64,
    cells: [usize; NATIVE_MEMBERS],
    group_ctx: [usize; NATIVE_MEMBERS],
    ctx: [usize; 2],
    count: u32,
    kinds: [u32; NATIVE_MEMBERS],
    actor_count: u32,
    flags: u8,
    cell_ints: [u32; NATIVE_MEMBERS],
    w5960: u32,
    w5970: u32,
    w5974: u32,
    blob: [u32; 40],
    /// `info+0x10` (listener lists) and `info+0x18` (deferred removals) of the
    /// entry-info dispatcher while this group was in the window (§14.1).
    dispatch: [usize; 2],
    /// `+0x5a88/+0x5a90/+0x5a98`: a native group's trainer/partner object records
    /// (DESIGN §2.5). Zero for ordinary groups.
    records: [usize; 3],
    /// `R+0x58..0x84`, the entry's identity bytes and the D mirror while this
    /// group was the window, saved only when a Mii identity moved in or out at
    /// the exchange (MII-GROUPS-DESIGN §3.3); `None` keeps today's rule that an
    /// exchange leaves `R` and those bytes untouched.
    identity: Option<([u32; RESOURCE_WORDS], EntryIdentity)>,
    index: u8,
    previous: u8,
    old: u8,
}

struct Group {
    // Immutable heap storage lent to the table hooks while this group is in the window.
    members: Box<Members>,
    state: GroupState,
    /// Saved window fields while built and detached.
    saved: Option<Window>,
    bodies: [Body; NATIVE_MEMBERS],
    since_us: u64,
    busy_logged: bool,
    /// One-shot "deferred for a live Kirby" diagnostic (D-KIRBY1).
    kirby_logged: bool,
    /// One-shot "deferred: process memory held by admitted groups" diagnostic (GS-PANIC).
    memory_logged: bool,
    /// Committed process charge when the prefetch was admitted (its measured cost is logged when it settles).
    prefetch_committed: Option<u64>,
    /// The Mii identity a `Special::Mii` group carries (`None` otherwise), with
    /// the lane configuration it was planned for (its CharInfo and tokens).
    identity: Option<MiiIdentity>,
    configuration: Option<Configuration>,
    /// The adapter has written the configuration's CharInfo onto the owner's
    /// selector record (`pending_mii_publication`); the prefetch waits for it.
    charinfo_published: bool,
    /// The refusals of that publication so far, with the last reason (`body_bank::mii_publish`).
    publish: PublishAttempts,
}

impl Group {
    fn planned_with(members: Members, identity: Option<(MiiIdentity, Configuration)>) -> Self {
        let (identity, configuration) = identity.map_or((None, None), |(i, c)| (Some(i), Some(c)));
        Self { members: Box::new(members), state: GroupState::Planned, saved: None,
            bodies: [Body::default(); NATIVE_MEMBERS], since_us: 0, busy_logged: false, kirby_logged: false,
            memory_logged: false, prefetch_committed: None,
            identity, configuration, charinfo_published: false, publish: PublishAttempts::new() }
    }
}

/// A cross-group request committed in-window; executed at the acknowledgement seam.
#[derive(Clone, Copy, Debug)]
struct Swap { group: usize, member: u8, kind: u32, via: u8, requested_us: u64 }

struct Bank {
    entry: usize,
    owner: u32,
    epoch: u64,
    // Immutable heap storage: the two inline hooks lend its table to their
    // enclosing native function. No mutex/reference survives that function.
    // Replacement occurs only in a later pre-service callback with INSTALL
    // owned and the prior entry natively ready (or after its destruction).
    // Retire keeps the allocation; native destructor paths use native tables.
    members: Box<Members>,
    phase: Phase,
    bodies: [Body; NATIVE_MEMBERS],
    // Value-only planning receipt, refreshed only from the current native callback.
    // The entry address is an identity token here, never a pointer to follow.
    observed: Option<([u8; 0x70], u32)>,
    from: u8,
    target: u8,
    acknowledged: bool,
    departed: bool,
    // A settled native change failed its retained-identity proof. The owner
    // consumes this as a refusal, never as successful completion.
    failed: bool,
    /// Extra groups beyond the window (group swap).
    groups: Vec<Group>,
    /// Resource owner `R` seen when this window became Ready; identity only.
    alias: usize,
    /// Pinned at the verified exchange, never refreshed from arbitrary live state.
    exchange: Option<GroupExchange>,
    swap: Option<Swap>,
    /// Group index, INSTALL token and start time of an in-flight prefetch.
    prefetch: Option<(usize, super::InstallToken, u64)>,
    /// Swap whose Ready is awaited, for the request-to-ready diagnostic.
    swap_started: Option<(u32, u64)>,
    /// One-shot pair diagnostics (missing N3, alias mismatch) for this bank.
    pair_logged: bool,
    /// Mii identity of the window group when it is a Mii group (`R` holds it).
    identity: Option<MiiIdentity>,
    /// Ids of every body that was the window's live member at least once (this
    /// bank's receipt; bodies keep their id across detach/attach). Feeds the
    /// teardown protocol's `BodyRef::activated` (WP-K evidence field).
    activated: Vec<u32>,
    /// Phase B′: the construction tasks of the group whose words are in the (empty) window,
    /// settled on a later tick (`settle_construction`) or drained before a destroyer.
    construction: Option<Construction>,
    /// P3-0: the tick-gap meter of the swap in flight (armed at the request, read at Ready).
    tick: TickGap,
}

fn role(owner: u32) -> Option<usize> {
    match owner { 0 => Some(0), 1 | 4 => Some(1), _ => None }
}

fn now_us() -> u64 {
    crate::engine::preload::preload_metrics::micros(crate::engine::preload::preload_metrics::ticks(), crate::engine::preload::preload_metrics::tick_hz())
        .unwrap_or(0)
}

unsafe fn snapshot(entry: *const u8) -> EntryState {
    EntryState { count: read(entry, 0x14),
        kinds: [read(entry, 0x18), read(entry, 0x1c), read(entry, 0x20)],
        actor_count: read(entry, 0x28), installed: read(entry, 0x4150),
        index: read(entry, 0x5918), previous_or_next: read(entry, 0x5919),
        pending: read(entry, 0x591a), flags: read(entry, 0x591f),
        outer: read(entry, 0x5920), loader: read(entry, 0x591c),
        changing: read(entry, 0x5934), guard: read(entry, 0x5935) }
}

/// Entry is borrowed from its current native callback, never from the receipt.
unsafe fn descriptor(entry: *const u8, bank: &Bank) -> bool {
    bank.phase != Phase::Retired && aligned(entry) && entry as usize == bank.entry
        && read::<u32>(entry, 0xe0) == bank.owner
        && read::<u32>(entry, 0x14) as usize == bank.members.count()
        && read::<u32>(entry, 0x28) == bank.members.actor_count()
        && bank.members.kinds().iter().enumerate()
            .all(|(i, kind)| read::<u32>(entry, 0x18 + i * 4) == *kind)
}

/// Validate every child reached through the current native entry. The remembered
/// addresses/IDs are comparison values only. `None` requires all bodies parked.
/// A pair-lead bank with the pair active has cells 0 and 1 active (`body_state`).
unsafe fn bodies(entry: *const u8, owner: u32, members: Members, active: Option<u8>)
    -> Option<[Body; NATIVE_MEMBERS]> {
    let mut observed = [Body::default(); NATIVE_MEMBERS];
    for (i, kind) in members.kinds().iter().enumerate().take(members.count()) {
        let body = fields::child(entry as usize, 0x4160 + i * 8)?;
        if fields::read::<usize>(body, 0)? != BASE.load(Ordering::Acquire) + FIGHTER_VTABLE
            || fields::read::<u32>(body, 0xc)? != *kind
            || fields::read::<u64>(body, 0x10)? != u64::from(owner)
            || fields::read::<u8>(body, 0x3a)? != members.body_state(i, active)
            || fields::child(body, 0x20).is_none()
        { return None; }
        observed[i] = Body { address: body, id: fields::read(body, 8)? };
    }
    Some(observed)
}

/// Pair-lead alias invariant at Ready: pair active means `+0x4180 == +0x4190`
/// (Nana's own group context, set by N2); cell 2 active means `+0x4180 == 0`
/// (cleared by N3). Ordinary banks have no second alias.
unsafe fn pair_alias_ok(entry: *const u8, members: Members, index: u8) -> bool {
    if !members.is_pair_lead() { return true; }
    let alias = read::<usize>(entry, 0x4180);
    if index == 0 { alias != 0 && alias == read::<usize>(entry, 0x4190) } else { alias == 0 }
}

/// The selected control context is the window's own context for `index`, with
/// the context vtable, this owner and that member's kind (`control_select_0064fb70.c`).
unsafe fn selected_context(entry: *const u8, owner: u32, members: Members, index: u8) -> bool {
    let Some(context) = fields::child(entry as usize, 0x4178) else { return false; };
    Some(context) == fields::child(entry as usize, 0x4188 + usize::from(index) * 8)
        && fields::read::<usize>(context, 0) == Some(BASE.load(Ordering::Acquire) + CONTEXT_VTABLE)
        && fields::read::<u32>(context, 8) == Some(owner)
        && fields::read::<u32>(context, 0x6c) == Some(owner)
        && fields::read::<u32>(context, 0x74) == Some(members.kinds()[usize::from(index)])
}

unsafe fn group_owner(entry: *const u8, bank: &Bank) -> Option<GroupOwner> {
    let role = role(bank.owner)?;
    let epoch = super::mii_saved_alpha::current_generation(role as u32);
    let resource = resource(BASE.load(Ordering::Acquire), bank.owner)?;
    if epoch != bank.epoch || resource != bank.alias { return None; }
    Some(GroupOwner { entry: entry as usize, owner: bank.owner, epoch, resource,
        header: fields::read(resource, 0x54)?, descriptor: read(entry, 0),
        bodies: bank.bodies.map(|body| (body.address, body.id)) })
}

unsafe fn ready_group_owner(entry: *const u8, bank: &Bank) -> Option<GroupOwner> {
    if bank.departed || bank.failed || bank.phase != Phase::Ready || !descriptor(entry, bank) { return None; }
    let members = *bank.members;
    let index = members.ready_index(snapshot(entry))?;
    if bodies(entry, bank.owner, members, Some(index))? != bank.bodies
        || !selected_context(entry, bank.owner, members, index)
        || !pair_alias_ok(entry, members, index) { return None; }
    group_owner(entry, bank)
}

/// Does not infer ownership from flags4. Only an owned immutable descriptor plus
/// a current complete body receipt admits a bank to shared readiness readers.
pub(crate) unsafe fn ready_members(entry: *const u8, owner: u32) -> Option<Members> {
    let state = BANKS[role(owner)?].lock().unwrap();
    let bank = state.as_ref()?;
    if bank.owner != owner || bank.phase != Phase::Ready || !descriptor(entry, bank) {
        return None;
    }
    let members = *bank.members;
    let index = members.ready_index(snapshot(entry))?;
    (bodies(entry, owner, members, Some(index))? == bank.bodies && pair_alias_ok(entry, members, index))
        .then_some(members)
}

pub(crate) unsafe fn ready_kind(entry: *const u8, owner: u32) -> Option<u32> {
    ready_members(entry, owner)?.ready_live(snapshot(entry)).map(|(_, kind)| kind)
}

/// Logical form and kind of the ready member: the pair-lead Popo is the normal
/// pair, a Solo bank's Popo is Solo, every other member its single form.
pub(crate) unsafe fn ready_forms(entry: *const u8, owner: u32) -> Option<(Forms, u32)> {
    let members = ready_members(entry, owner)?;
    let (_, kind) = members.ready_live(snapshot(entry))?;
    Some((members.forms_of(kind), kind))
}

/// Pair-lead bank (or pair group) owning this entry (any phase but Retired): the
/// construction (N1) and activation (N2) seams run while the bank is still Building.
unsafe fn pair_lead_owner(entry: *const u8) -> Option<u32> {
    pair_window(entry).map(|(owner, _)| owner)
}

/// Owner and members of a window whose cells 0/1 are Popo/Nana: the pair-lead
/// window or a pair group (N1/N2/N3 serve both).
unsafe fn pair_window(entry: *const u8) -> Option<(u32, Members)> {
    if !aligned(entry) { return None; }
    let owner = read::<u32>(entry, 0xe0);
    let state = BANKS[role(owner)?].lock().unwrap();
    state.as_ref().filter(|bank| bank.members.is_pair() && descriptor(entry, bank)).map(|bank| (owner, *bank.members))
}

/// Value-only logical identity for menu/preload planning. This does not qualify
/// a native write: each request still repeats the current native ready proof.
/// The Ready-time descriptor tolerates the same live behavior bytes as the
/// receipt, so a native CPU Behavior change keeps the active member's identity
/// for the next cycle instead of falling back to the group's first kind.
pub(crate) fn cached_kind(entry: usize, owner: u32, descriptor: &[u8; 0x70]) -> Option<u32> {
    let state = BANKS[role(owner)?].lock().unwrap();
    let bank = state.as_ref()?;
    if bank.entry != entry || bank.owner != owner || bank.departed || bank.phase != Phase::Ready {
        return None;
    }
    bank.observed
        .filter(|(physical, _)| crate::engine::mii::identity::verify_descriptor(physical, descriptor,
            crate::engine::mii::identity::ExpectedMutation::NativeBehavior).is_ok())
        .map(|(_, kind)| kind)
}

/// True only after native settlement disproved the operation's identity.
/// No timeout, address alone, or partly constructed entry makes a refusal.
pub(super) fn failed(entry: *const u8, owner: u32) -> bool {
    let Some(role) = role(owner) else { return false; };
    BANKS[role].lock().unwrap().as_ref().is_some_and(|bank|
        bank.entry == entry as usize && bank.owner == owner && bank.failed)
}

/// Native resource/entry hooks share this token. Publish before Reload, but
/// never hold the bank mutex while calling any native function.
pub(super) unsafe fn build(entry: *const u8, owner: u32, members: Members) -> bool {
    let Some(role) = role(owner) else { return false; };
    if !ENABLED.load(Ordering::Acquire) || !super::INSTALL.active()
        || super::ready_forms(entry, owner).is_none() { return false; }
    // The incoming Reload destroys only the window; detached groups go first.
    before_reload(entry);
    let mut state = BANKS[role].lock().unwrap();
    // Reload is asynchronous. Even if the outgoing canonical descriptor still
    // looks ready, do not replace storage borrowed by an in-flight constructor.
    if state.as_ref().is_some_and(|bank| matches!(bank.phase, Phase::Building | Phase::Changing)) {
        return false;
    }
    *state = Some(Bank {
        entry: entry as usize, owner, epoch: super::mii_saved_alpha::current_generation(role as u32),
        members: Box::new(members), phase: Phase::Building,
        bodies: [Body::default(); NATIVE_MEMBERS], observed: None, from: 0, target: 0,
        acknowledged: false, departed: false, failed: false,
        groups: Vec::new(), alias: 0, exchange: None, swap: None, prefetch: None, swap_started: None,
        pair_logged: false, identity: None, activated: Vec::new(),
        construction: None, tick: TickGap::default(),
    });
    EXTRAS[role].store((members.count() - 1) as u8, Ordering::Release);
    GROUP_BODIES[role].store(0, Ordering::Release);
    trace!("[BodyBank] build owner={} entry={:p} members={:?}", owner, entry, members);
    if members.is_pair_lead() {
        trace!("[IcPair] build owner={} entry={:p} kinds={:x?} actor_count=2 extra={:#x}",
            owner, entry, members.kinds(), members.kinds()[2]);
    }
    true
}

/// INSTALL and outgoing pose are already owned by the ordinary roster request.
/// Returns false without native writes when this target is not currently warm.
/// A target held by a reachable extra group commits an in-window change to park
/// the current body; the group exchange happens at the acknowledgement seam.
/// `forms` is the requested form of `kind` (`Plan::forms`): a group serves it only
/// in that form (`group_serves`), the same rule `plan()` reached it by (WP-I).
pub(super) unsafe fn switch(entry: *mut u8, owner: u32, kind: u32, forms: Forms) -> bool {
    let Some(role) = role(owner) else { return false; };
    let Some(members) = ready_members(entry, owner) else { return false; };
    let Some(from) = members.ready_index(snapshot(entry)) else { return false; };
    if !super::INSTALL.active() { return false; }
    // Same gates as preflight, re-proven now (no lock held across the status reads).
    // The pair parks for its extra and for a group swap alike; never for itself.
    if members.is_pair() && from == 0 && members.index_of(kind) != Some(0) {
        if let Err(reason) = pair_parkable(entry, owner) {
            trace!("[IcPair] fallback owner={} target={:#x} reason={}; switch refused", owner, kind, reason);
            return false;
        }
    }
    if kind != KIRBY {
        if let Err(reason) = kirby_park_gate(entry, owner, members, from) {
            trace!("[KirbyPark] fallback owner={} target={:#x} reason={}; switch refused", owner, kind, reason);
            return false;
        }
    }
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    // WP4 (Astra §4.3 drain, §4.2 exclusive reset batch): before the commit, Prefetched groups other
    // than the target hand their keys back and return to Planned when the target is a member whose
    // row suspends prefetch while live (Kirby: no debt may be on R when it becomes live) or when the
    // target group's build takes the pair rule (its tails reset the whole R). An in-flight prefetch
    // refuses a group request below, as before; the window's own live member decides the copy.
    let copy = live_kirby_copy_kind(entry, owner, members);
    let drain = {
        let state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_ref() else { return false; };
        let target_group = bank.groups.iter().position(|g| g.state.reachable() && group_serves(*g.members, kind, forms).is_some());
        let kirby_target = !capabilities(form_class(kind, forms)).prefetch_while_live;
        let pair_target = target_group.is_some_and(|g| bank.groups[g].state == GroupState::Prefetched
            && pair_resource_references(*bank.groups[g].members).is_some());
        let holders: Vec<usize> = bank.groups.iter().enumerate()
            .filter(|(i, g)| g.state == GroupState::Prefetched && Some(*i) != target_group).map(|(i, _)| i).collect();
        ((kirby_target || pair_target) && !holders.is_empty() && bank.prefetch.is_none() && bank.alias != 0)
            .then(|| (holders, occupancy_of(bank, role, copy), bank.alias,
                if kirby_target { "Kirby target (drain before live)" } else { "pair build needs an exclusive reset batch" }))
    };
    if let Some((holders, occupancy, alias, why)) = drain {
        let base = BASE.load(Ordering::Acquire);
        if resource(base, owner) == Some(alias) && resource_ready(alias) {
            for group in holders {
                let (released, kept) = release_group_keys_of(base, role, alias, group, &occupancy, owner, why);
                let mut state = BANKS[role].lock().unwrap();
                let Some(bank) = state.as_mut() else { return false; };
                let Some(g) = bank.groups.get_mut(group) else { continue; };
                g.state = GroupState::Planned;
                g.busy_logged = false;
                g.kirby_logged = false;
                g.memory_logged = false;
                trace!("[GroupSwap] keys returned owner={} group={} kinds={:x?} released={} kept={} reason={}; replanned map={}",
                    owner, group, &g.members.kinds()[..g.members.count()], released, kept, why,
                    fields::read::<u64>(alias, 0x18).unwrap_or(0));
            }
        }
    }
    let mut state = BANKS[role].lock().unwrap();
    let Some(bank) = state.as_mut() else { return false; };
    if bank.departed || bank.phase != Phase::Ready || !descriptor(entry, bank) { return false; }
    let (target, swap) = match group_serves(members, kind, forms) {
        Some(target) => {
            if from == target { return false; }
            (target, None)
        }
        None => {
            if !groups_enabled(&settings) || bank.swap.is_some() || bank.prefetch.is_some() { return false; }
            let Some((group, member)) = bank.groups.iter().enumerate().find_map(|(index, g)| {
                if !g.state.reachable() { return None; }
                group_serves(*g.members, kind, forms).map(|member| (index, member))
            }) else { return false; };
            let Some(via) = swap_commit_index_in(members, from) else {
                trace!("[GroupSwap] request refused owner={} kind={:#x} reason=no in-window commit index from={} count={} pair_lead={}",
                    owner, kind, from, members.count(), members.is_pair_lead());
                return false;
            };
            (via, Some(Swap { group, member, kind, via, requested_us: now_us() }))
        }
    };
    bank.phase = Phase::Changing; bank.observed = None; bank.from = from; bank.target = target;
    bank.acknowledged = false; bank.failed = false; bank.swap = swap;
    bank.tick = TickGap::arm(now_us());
    split::arm(role, now_us());
    fields::write_unchecked(entry as usize, 0x5932, 0u8);
    fields::write_unchecked(entry as usize, 0x591a, target);
    match swap {
        Some(swap) => trace!("[GroupSwap] request owner={} kind={:#x} group={} member={} via={} from={} state={:?} form={}",
            owner, kind, swap.group, swap.member, swap.via, from, bank.groups[swap.group].state, form_label(forms)),
        None => trace!("[BodyBank] reuse owner={} from={} to={} kind=0x{:x}", owner, from, target, kind),
    }
    if members.is_pair() {
        trace!("[IcPair] request owner={} {} flags={:#x} alias={:#x}", owner,
            match (from, swap.is_some()) {
                (0, false) => "park pair -> extra",
                (0, true) => "park pair -> group swap via extra",
                (_, false) => "park extra -> pair",
                (_, true) => "park extra -> group swap via pair",
            },
            read::<u8>(entry, 0x591f), read::<usize>(entry, 0x4180));
    }
    true
}

enum Action { None, Swap, Lifecycle, Construct }

/// Called before all normal roster/source/HUD work, never instead of that work.
/// INSTALL completion remains owned by the original roster/Mii transaction.
pub(super) unsafe fn service(entry: *mut u8, owner: u32) {
    let Some(role) = role(owner) else { return; };
    let observed = snapshot(entry);
    split::tick_any(role, owner, now_us(), &observed);
    if GROUPS_ENABLED.load(Ordering::Acquire) {
        sample_pool();
        // A Reload begins at loader state 0 right after this pre-dispatch (§4.2):
        // every detached group must already be gone.
        if observed.loader == 0 { retire_groups(entry, owner, "reload"); }
    }
    if PAIR_ENABLED.load(Ordering::Acquire) && observed.loader == 0 { pair_teardown(entry, "reload"); }
    // A native-initiated Reload (`0x653240` wrote loader 0 and outer 3 over an installed
    // window) frees the window contexts in states 1-3 without re-pointing the pad route
    // (D-PTA1c): clear the attached window's routes here too, groups on or off
    // (WORKPLAN §2.4, declared behaviour change); silent once nothing is left to clear.
    if observed.loader == 0 && observed.outer == 3 && observed.installed != 0 { retire_window(entry, owner, "reload", false); }
    match service_locked(entry, owner, role) {
        Action::None => {}
        Action::Swap => perform_swap(entry, owner, role),
        Action::Lifecycle => group_lifecycle(entry, owner, role),
        Action::Construct => settle_construction(entry, owner, role),
    }
}

unsafe fn service_locked(entry: *mut u8, owner: u32, role: usize) -> Action {
    let mut state = BANKS[role].lock().unwrap();
    let Some(bank) = state.as_mut() else { return Action::None; };
    if bank.owner != owner || bank.entry != entry as usize || bank.phase == Phase::Retired {
        return Action::None;
    }
    // P3-0: one tick of this entry's pre-service; the meter is armed by a request and read at Ready.
    bank.tick.observe(now_us());
    split::observe(role, now_us());
    // A former Ready receipt must not survive a callback observing an unfinished
    // native reset/replacement. Publish it again only after today's full proof.
    bank.observed = None;
    if !descriptor(entry, bank) {
        // A native reset, exact-Mii load or canonical fallback may replace this
        // cache using the same entry. Canonical readiness proves replacement is
        // complete. Keep the immutable table until a later qualified build.
        if snapshot(entry).ready().is_some() && bank.phase == Phase::Ready {
            bank.phase = Phase::Retired;
            bank.exchange = None;
            EXTRAS[role].store(0, Ordering::Release);
            GROUP_BODIES[role].store(0, Ordering::Release);
            if !bank.groups.is_empty() {
                trace!("[GroupSwap] canonical replacement owner={} with {} group records; native teardown already ran",
                    owner, bank.groups.len());
                bank.groups.clear();
            }
            // Native's canonical path reset R itself (state 6 picks a free R): the ledger starts over.
            r_handle::reset(ledger_role(role), "canonical replacement");
            trace!("[BodyBank] canonical replacement owner={}; cache retired", owner);
        }
        return Action::None;
    }
    // Phase B′: the window holds the group's words and no cells until the tasks settle.
    if bank.construction.is_some() { return Action::Construct; }
    let members = *bank.members;
    let observed = snapshot(entry);
    if let Some(index) = members.ready_index(observed) {
        let now = bodies(entry, owner, members, Some(index));
        if now.is_some() && !pair_alias_ok(entry, members, index) {
            if !bank.pair_logged {
                bank.pair_logged = true;
                trace!("[IcPair] alias mismatch owner={} index={} +0x4180={:#x} +0x4190={:#x} flags={:#x}; not ready",
                    owner, index, read::<usize>(entry, 0x4180), read::<usize>(entry, 0x4190), observed.flags);
            }
            return Action::None;
        }
        if let Some(now) = now {
            bank.observed = Some((read(entry, 0), members.kinds()[index as usize]));
            if !bank.activated.contains(&now[usize::from(index)].id) { bank.activated.push(now[usize::from(index)].id); }
            if bank.phase == Phase::Changing && (index != bank.target || now != bank.bodies) {
                // Native is already ready and all current children pass their
                // full proof. Retire this operation, not the native resources.
                // A later request may use the new current generation; this
                // request must be refused by its snapshot/INSTALL owner.
                let generation_changed = now != bank.bodies;
                bank.bodies = now;
                bank.phase = Phase::Ready;
                // A wrong member of the same bank refuses only this request.
                // New objects invalidate the owner's snapshot even if native
                // reached the requested kind: adapters must retire authority.
                bank.failed = generation_changed;
                bank.swap = None;
                bank.swap_started = None;
                trace!("[BodyBank] settled reuse refused owner={} generation_changed={} index={} target={}",
                    owner, generation_changed, index, bank.target);
                return Action::None;
            }
            if bank.phase != Phase::Ready || bank.bodies != now {
                trace!("[BodyBank] ready owner={} index={} kind=0x{:x} previous={:?} bodies={:?}",
                    owner, index, members.kinds()[index as usize], bank.phase, now);
                #[cfg(feature = "diag")]
                crate::runtime::roster::identity_view::report(owner, resource(BASE.load(Ordering::Acquire), owner).unwrap_or(0), "ready");
                if members.is_pair_lead() {
                    trace!("[IcPair] ready owner={} {} flags={:#x} states=[{},{},{}] +0x4178={:#x} +0x4180={:#x} ids=[{:#x},{:#x},{:#x}]",
                        owner, if index == 0 { "pair active" } else { "extra active" }, observed.flags,
                        fields::read::<u8>(now[0].address, 0x3a).unwrap_or(0xff),
                        fields::read::<u8>(now[1].address, 0x3a).unwrap_or(0xff),
                        fields::read::<u8>(now[2].address, 0x3a).unwrap_or(0xff),
                        read::<usize>(entry, 0x4178), read::<usize>(entry, 0x4180), now[0].id, now[1].id, now[2].id);
                }
                // The window's bodies constructed: every acquisition recorded or made for the window
                // (`prepare_resources`) was consumed by a construction tail (WP4 ledger).
                if bank.phase == Phase::Building {
                    r_handle::construction_settled(ledger_role(role), r_handle::WINDOW, &members.kinds()[..members.count()]);
                }
                bank.bodies = now; bank.phase = Phase::Ready; bank.pair_logged = false;
            }
            if bank.alias == 0 {
                bank.alias = resource(BASE.load(Ordering::Acquire), owner).unwrap_or(0);
            }
            if let Some((kind, requested)) = bank.swap_started.take() {
                // P3-0: the largest tick gap since the request is the stall the game thread took.
                trace!("[GroupSwap] swap owner={} kind={:#x} request_to_ready_us={} bodies={:?} max_tick_gap_us={} ticks={}",
                    owner, kind, now_us().saturating_sub(requested), now, bank.tick.max_gap_us, bank.tick.ticks);
                bank.tick.disarm();
                split::report_gaps(role, owner, kind);
            }
            let idle = GROUPS_ENABLED.load(Ordering::Acquire) && !bank.departed && bank.swap.is_none()
                && (bank.prefetch.is_some() || bank.groups.iter().any(|g| g.state == GroupState::Planned));
            return if idle { Action::Lifecycle } else { Action::None };
        }
        return Action::None;
    }
    if members.is_pair_lead() && bank.phase == Phase::Changing && !bank.acknowledged && !bank.pair_logged
        && observed.outer == 3 && observed.loader == 12 && observed.flags != BANK_FLAG {
        // The commit parked the pair but N3 left flags unchanged: nothing is
        // acknowledged (fail safe); report once for the runtime investigation.
        bank.pair_logged = true;
        trace!("[IcPair] N3 missing owner={} from={} target={} flags={:#x} nana_state={:?} +0x4180={:#x}", owner, bank.from,
            bank.target, observed.flags, fields::child(entry as usize, 0x4168).and_then(|b| fields::read::<u8>(b, 0x3a)),
            read::<usize>(entry, 0x4180));
    }
    if bank.phase != Phase::Changing || bank.acknowledged
        || !members.awaiting_completion(observed, bank.target)
        || read::<u8>(entry, 0x5910) != bank.from || read::<u8>(entry, 0x5933) != 0
        || bodies(entry, owner, members, None) != Some(bank.bodies) { return Action::None; }
    if !selected_context(entry, owner, members, bank.target) { return Action::None; }
    // The outgoing body is parked and its routes released (§3): exchange the
    // window here, before native activation, never after.
    if bank.swap.is_some() { return Action::Swap; }
    // Ordinary kinds lack the PT/Aegis completion script. All bodies are parked
    // and unchanged; selected context is current and same-owner. Native resource
    // checks, activation and restore ordering remain intact. No timeout qualifies.
    fields::write_unchecked(entry as usize, 0x5933, 1u8);
    bank.acknowledged = true;
    Action::None
}

// ----- native access helpers (values and single-callback borrows only) -----

/// `R = *(G+0xe8+8*slot)` while `G+0x128+slot == 0`, with the R class vtable.
unsafe fn resource(base: usize, owner: u32) -> Option<usize> {
    let table = fields::child(base, OWNER_TABLE)?;
    if fields::read::<u8>(table, 0x128 + owner as usize)? != 0 { return None; }
    let r = fields::child(table, 0xe8 + owner as usize * 8)?;
    (fields::read::<usize>(r, 0) == Some(base + RESOURCE_VTABLE)).then_some(r)
}

/// R vtable `+0x30` (`0x17e4750`): no task, NRO tree loaded, every map handle loaded.
unsafe fn resource_ready(r: usize) -> bool {
    let Some(vtable) = fields::child(r, 0) else { return false; };
    let Some(ready) = fields::read::<usize>(vtable, 0x30) else { return false; };
    if !fields::object(ready) { return false; }
    let ready: unsafe extern "C" fn(*mut u8) -> u64 = std::mem::transmute(ready);
    ready(r as *mut u8) & 1 != 0
}

/// Outstanding work units of the R task object (`task+0x10`). The pointer at
/// Read `R+0xa8` fresh: P-GS2 C4 observed replacement across a canonical Reload.
/// Reset/release may skip a currently idle task; that is not a lifetime guarantee.
unsafe fn task_pending(r: usize) -> u32 {
    fields::child(r, 0xa8).and_then(|task| fields::read::<u32>(task, 0x10)).unwrap_or(0)
}

/// Fighter pool `**0x52b8548`: capacity u32 `+0x18`, live u16 `+0x1e` (`0x3ae0c4-0x3ae110`).
unsafe fn pool_counts(base: usize) -> Option<(u32, u16)> {
    let holder = fields::child(base, FIGHTER_POOL)?;
    let pool = fields::child(holder, 0)?;
    Some((fields::read::<u32>(pool, 0x18)?, fields::read::<u16>(pool, 0x1e)?))
}

unsafe fn sample_pool() {
    if let Some((capacity, live)) = pool_counts(BASE.load(Ordering::Acquire)) {
        POOL_CAPACITY.store(capacity, Ordering::Release);
        POOL_LIVE.store(u32::from(live), Ordering::Release);
    }
}

/// Pool slab address the destroyer `0x3aeb50` derives from an id
/// (`ubfx w8,w0,#16,#12`, stride `0xf940`, bounds `+0x18`).
unsafe fn pool_slot_address(base: usize, id: u32) -> Option<usize> {
    if id >> 28 != 0 { return None; }
    let holder = fields::child(base, FIGHTER_POOL)?;
    let pool = fields::child(holder, 0)?;
    let capacity = fields::read::<u32>(pool, 0x18)?;
    let slot = (id >> 16) & 0xfff;
    if slot >= capacity { return None; }
    let data = fields::child(pool, 8)?;
    data.checked_add(slot as usize * BODY_STRIDE)
}

/// Stale-id hazard (§4.1): the destroyer dereferences a null lookup. Validate the
/// pointer, id, kind, owner, parked state and the id's own slab address.
unsafe fn valid_parked_body(base: usize, body: Body, kind: u32, owner: u32) -> bool {
    fields::object(body.address)
        && fields::read::<usize>(body.address, 0) == Some(base + FIGHTER_VTABLE)
        && fields::read::<u32>(body.address, 8) == Some(body.id)
        && fields::read::<u32>(body.address, 0xc) == Some(kind)
        && fields::read::<u64>(body.address, 0x10) == Some(u64::from(owner))
        && fields::read::<u8>(body.address, 0x3a) == Some(3)
        && pool_slot_address(base, body.id) == Some(body.address)
}

unsafe fn valid_context(base: usize, context: usize, owner: u32) -> bool {
    fields::object(context)
        && fields::read::<usize>(context, 0) == Some(base + CONTEXT_VTABLE)
        && fields::read::<u32>(context, 8) == Some(owner)
}

/// The kind native's init registered a body under: the home header word `R+0x58[cell]`
/// (`0x607eb4` passes `R+0x58[sub]`, not the body's own kind, to `prepare_entry` — WP-R
/// FACT; a Phase-B group keeps the home header). `None` without a valid R.
unsafe fn registered_kind(r: Option<usize>, cell: usize) -> Option<u32> {
    r.filter(|r| *r != 0).and_then(|r| fields::read::<u32>(r, 0x58 + cell * 4))
}


/// K15 (`swap_model::copy_packages`): the member sets this bank may register, as descriptor words:
/// the attached window and every group that holds keys or bodies. While no group does, nothing:
/// the window's registration stays native's own.
fn copy_package_sets(bank: &Bank) -> Vec<[u32; NATIVE_MEMBERS]> {
    let words = |members: Members| {
        let mut words = [u32::MAX; NATIVE_MEMBERS];
        let count = members.count().min(NATIVE_MEMBERS);
        words[..count].copy_from_slice(&members.kinds()[..count]);
        words
    };
    let groups: Vec<[u32; NATIVE_MEMBERS]> = bank.groups.iter()
        .filter(|g| g.state.holds_keys() || g.state == GroupState::Built)
        .map(|g| words(*g.members)).collect();
    if groups.is_empty() { return Vec::new(); }
    std::iter::once(words(*bank.members)).chain(groups).collect()
}

/// The attached-window arm of the teardown protocol (`swap_model::lifecycle`, Astra §5
/// step 4): clear every pad route that names one of this window's controllers (D-PTA1c)
/// and run the Kirby arm for a never-activated Kirby cell (WP-K), then let native destroy
/// the window itself (`NativeRetiring`). Runs before every mod Reload, at a native Reload's
/// loader state 0 and at finalize (match start, Quit Training, room close), groups on or
/// off (WORKPLAN §2.4). With `always` false (the per-frame loader-0 caller) an empty plan
/// is silent. Never called under the bank lock. Keeps the E1/E2 interface line
/// `[GroupSwap] pad route released …` when a route was cleared.
unsafe fn retire_window(entry: *const u8, owner: u32, reason: &str, always: bool) {
    let base = BASE.load(Ordering::Acquire);
    // The attached window is whatever the live descriptor names. A bank describes it only while
    // its descriptor is the live one: `build()` installs the incoming window's bank (Building)
    // before `reload_at_pose` runs this teardown, while the outgoing window's bodies are still in
    // the cells (Eden M4b `m4b2` #85/#86: the bank's `[6, 5, 7]` over the canonical Mario body
    // read as a Kirby with `kirby_branch_mismatch=1`).
    let window = role(owner).and_then(|role| BANKS[role].lock().unwrap().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.owner == owner && descriptor(entry, bank))
        .map(|bank| (bank.members.kinds(), bank.members.count(), bank.activated.clone())));
    let (kinds, count, activated) = match window {
        Some(window) => window,
        None => {
            let count = (read::<u32>(entry, 0x14) as usize).min(NATIVE_MEMBERS);
            ([read::<u32>(entry, 0x18), read::<u32>(entry, 0x1c), read::<u32>(entry, 0x20)], count, Vec::new())
        }
    };
    let mut cells = Vec::with_capacity(count);
    for (i, kind) in kinds.iter().enumerate().take(count) {
        let Some(body) = fields::child(entry as usize, 0x4160 + i * 8) else { continue; };
        let id = fields::read::<u32>(body, 8).unwrap_or(0);
        // The ref carries the body's own kind (`body+0xc`, what its unload's `unprepare`
        // reads), not the planned one: at a mod Reload the cells still hold the outgoing
        // bodies while the bank already lists the incoming kinds (Yoga `m4b3` #140).
        let own = fields::read::<u32>(body, 0xc);
        let fighter = fields::read::<usize>(body, 0) == Some(base + FIGHTER_VTABLE);
        let valid = fighter && own.is_some();
        let live = fields::read::<u8>(body, 0x3a) == Some(4);
        cells.push(BodyRef { id, kind: if valid { own.unwrap_or(*kind) } else { *kind }, cell: i as u8,
            activated: live || activated.contains(&id), valid, registered: registered_kind(resource(base, owner), i) });
    }
    let contexts = graph::window_contexts(entry as usize, |context| valid_context(base, context, owner));
    let probed = contexts.len();
    let bundle = Bundle::Attached(AttachedGroup { owner, cells, contexts });
    let consumers = graph::probe_consumers(base, owner);
    let plan = destroy_plan_many(std::slice::from_ref(&bundle), &consumers);
    if !always && plan.steps.is_empty() && plan.kirby_misplaced == 0 { return; }
    let receipt = graph::execute(base, &plan, owner, reason, &mut |_: &[u32]| (0usize, 0usize));
    if receipt.routes_cleared != 0 {
        trace!("[GroupSwap] pad route released owner={} reason={} contexts={} cleared={}", owner, reason, probed, receipt.routes_cleared);
    }
}

// `0x17e64d0` / `0x17e8610` / `0x17e2fe0` and the `R+0x4c`/`R+0x50` writes are issued only by
// `crate::runtime::roster::r_handle` after a ledger decision (WP4); the constants above stay for the
// install-time fingerprints. Every release of a prefetched kind (post-build object release,
// failed prefetch, build rollback, teardown, live-Kirby drain) goes through `release_group_keys`.
// WP-R's identity-view publication (`identity_view::publish` before a group kind's first native
// request, formerly in `acquire_kind`) runs in `r_handle::acquire` for a group prefetch.

unsafe fn select_control(base: usize, entry: *mut u8) {
    let select: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + SELECT_CONTROL);
    select(entry);
}

unsafe fn sync_construct(base: usize, entry: *mut u8) {
    let construct: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + SYNC_CONSTRUCT);
    construct(entry);
}

/// `G68 = *(G+0x68)`, the registry init and unload read under `G+0xc8` (`0x607e94-0x607eb0`,
/// `0x6118a8-0x6118b8`); scene-wide, constructed at scene setup (`0x66dc54`).
unsafe fn copy_registry_g68(base: usize) -> Option<usize> {
    fields::child(fields::child(base, OWNER_TABLE)?, G68_SLOT)
}

/// U18: the native side of `swap_model::copy_registry` for one role's entry and `R`. Every call is
/// one native itself makes for the window it describes; none runs under the bank lock (the
/// registry functions take `G68+0x1d238` themselves).
struct NativeRegistry { base: usize, r: usize, owner: u32, g68: usize }

impl NativeRegistry {
    /// `R+0x54` is the entry index init and unload pass (`0x607f24`, body `+0x10`).
    unsafe fn new(base: usize, r: usize, owner: u32) -> Option<Self> {
        let g68 = copy_registry_g68(base)?;
        ((owner as usize) < copy_registry::ENTRIES && fields::read::<u32>(r, 0x54) == Some(owner))
            .then_some(Self { base, r, owner, g68 })
    }

    /// Diagnostics only: `(count, registered kind)` of this entry's three cells (`G68+0x1d268+e*0x48+s*0x18`,
    /// `G68+0x1d498+e*0xc+s*4`). Aligned single-word reads outside the registry lock.
    unsafe fn cells(&self) -> [(u64, u32); NATIVE_MEMBERS] {
        let e = self.owner as usize;
        std::array::from_fn(|s| (
            fields::read::<u64>(self.g68, REGISTRY_CELLS + 0x10 + e * 0x48 + s * 0x18).unwrap_or(u64::MAX),
            fields::read::<u32>(self.g68, REGISTRY_KINDS + e * 0xc + s * 4).unwrap_or(0xdead)))
    }

    unsafe fn header(&self) -> [u32; NATIVE_MEMBERS] {
        std::array::from_fn(|i| fields::read_unchecked::<u32>(self.r, 0x58 + i * 4))
    }

    unsafe fn trace(&self, event: &str, from: Members, to: Members) {
        trace!("[Registry] {} owner={} from={:x?} to={:x?} header={:x?} cells={:x?} r4c={:#x} task_pending={}",
            event, self.owner, &from.kinds()[..from.count()], &to.kinds()[..to.count()], self.header(), self.cells(),
            fields::read::<u32>(self.r, 0x4c).unwrap_or(0), task_pending(self.r));
    }
}

impl RegistryOps for NativeRegistry {
    fn unregister(&mut self, members: Members) {
        unsafe {
            let unprepare: unsafe extern "C" fn(*mut u8, u32, u32, u32) = std::mem::transmute(self.base + UNPREPARE);
            split::mark(split::Step::Plugin);
            for (sub, kind) in members.kinds().iter().enumerate().take(members.count()) {
                unprepare(self.g68 as *mut u8, *kind, self.owner, sub as u32);
            }
            split::mark(split::Step::Unprepare);
        }
    }

    fn write_header(&mut self, members: Members) {
        unsafe { for (i, kind) in members.kinds().iter().enumerate() { fields::write_unchecked(self.r, 0x58 + i * 4, *kind); } }
    }

    fn refresh_roster(&mut self) {
        unsafe {
            let refresh: unsafe extern "C" fn(*mut u8) = std::mem::transmute(self.base + ROSTER_REFRESH);
            split::mark(split::Step::Plugin);
            refresh(self.r as *mut u8);
            split::mark(split::Step::Roster);
        }
    }

    fn register(&mut self, members: Members) {
        unsafe {
            let prepare: unsafe extern "C" fn(*mut u8, *mut u8, u32, u32, u32, u32) = std::mem::transmute(self.base + PREPARE_ENTRY);
            let header = self.header();
            let costume = copy_registry::registry_costume(header[0], fields::read_unchecked::<u32>(self.r, 0x64),
                fields::read_unchecked::<u32>(self.base, REGION_WORD));
            split::mark(split::Step::Plugin);
            for (sub, kind) in header.iter().enumerate().take(members.count()) {
                prepare(self.g68 as *mut u8, (self.r + 0xb0) as *mut u8, *kind, costume, self.owner, sub as u32);
            }
            split::mark(split::Step::Prepare);
        }
    }
}

unsafe fn save_window(entry: usize) -> Window {
    let mut blob = [0u32; 40];
    for (i, word) in blob.iter_mut().enumerate() { *word = fields::read_unchecked(entry, 0x597c + i * 4); }
    let rd = |offset| fields::read_unchecked::<usize>(entry, offset);
    Window {
        installed: fields::read_unchecked(entry, 0x4150),
        cells: [rd(0x4160), rd(0x4168), rd(0x4170)],
        group_ctx: [rd(0x4188), rd(0x4190), rd(0x4198)],
        ctx: [rd(0x4178), rd(0x4180)],
        count: fields::read_unchecked(entry, 0x14),
        kinds: [fields::read_unchecked(entry, 0x18), fields::read_unchecked(entry, 0x1c), fields::read_unchecked(entry, 0x20)],
        actor_count: fields::read_unchecked(entry, 0x28),
        flags: fields::read_unchecked(entry, 0x591f),
        cell_ints: [fields::read_unchecked(entry, 0x5964), fields::read_unchecked(entry, 0x5968), fields::read_unchecked(entry, 0x596c)],
        w5960: fields::read_unchecked(entry, 0x5960),
        w5970: fields::read_unchecked(entry, 0x5970),
        w5974: fields::read_unchecked(entry, 0x5974),
        blob,
        dispatch: dispatcher_arrays(entry).unwrap_or([0, 0]),
        records: [rd(0x5a88), rd(0x5a90), rd(0x5a98)],
        identity: None,
        index: fields::read_unchecked(entry, 0x5918),
        previous: fields::read_unchecked(entry, 0x5919),
        old: fields::read_unchecked(entry, 0x5910),
    }
}

// ----- entry-info observer dispatcher (`info+8`, §14.1) -----

/// `info = *(entry+0xf8)`; the dispatcher struct starts at `info+8`:
/// `+0 n_types (1)`, `+8 lists`, `+0x10 removals`, `+0x18 depth`, `+0x1c pending`.
unsafe fn entry_info(entry: usize) -> Option<usize> {
    fields::child(entry, 0xf8)
}

unsafe fn dispatcher_arrays(entry: usize) -> Option<[usize; 2]> {
    let info = entry_info(entry)?;
    Some([fields::read::<usize>(info, 0x10)?, fields::read::<usize>(info, 0x18)?])
}

/// Idle dispatcher as `654dc0` leaves it and as every dispatch site requires
/// between dispatches (`0x653b54-0x653bb4`): one list, both arrays present and
/// one element long, no dispatch in progress, no deferred removal pending.
unsafe fn dispatcher_idle(entry: usize) -> bool {
    let Some(info) = entry_info(entry) else { return false; };
    let (Some(types), Some(depth), Some(pending)) = (fields::read::<u32>(info, 0x8),
        fields::read::<u32>(info, 0x20), fields::read::<u32>(info, 0x24)) else { return false; };
    dispatcher_idle_words(types, depth, pending)
        && dispatcher_arrays(entry).is_some_and(|[lists, removals]| {
            dispatcher_array_ok(lists) && dispatcher_array_ok(removals)
        })
}

/// After a native call: the dispatcher exists and is between dispatches (one list
/// type, depth 0, both arrays valid). `+0x24` is not required to be 0 here: only
/// the flush clears it (`0x37ae858`), never `654dc0`, so a stale count is possible.
unsafe fn dispatcher_present(entry: usize) -> bool {
    let Some(info) = entry_info(entry) else { return false; };
    fields::read::<u32>(info, 0x8) == Some(1)
        && fields::read::<u32>(info, 0x20) == Some(0)
        && dispatcher_arrays(entry).is_some_and(|[lists, removals]| {
            dispatcher_array_ok(lists) && dispatcher_array_ok(removals)
        })
}

/// An array `654dc0` allocated: `[arr-8] == 1` element of `{size, next, prev}`
/// (`0x654e38-0x654e54`), sentinel at `arr+8`, a consistent doubly linked ring of
/// at most `size` nodes whose `+0x10` is null or an object with a vtable.
unsafe fn dispatcher_array_ok(arr: usize) -> bool {
    if arr == 0 || !arr.is_multiple_of(8) || !fields::object(arr) { return false; }
    let Some(header) = arr.checked_sub(8) else { return false; };
    let (Some(elements), Some(size)) = (fields::read::<u64>(header, 0), fields::read::<u64>(arr, 0)) else { return false; };
    if !dispatcher_array_header_ok(elements, size) { return false; }
    let sentinel = arr + 8;
    let mut previous = sentinel;
    let Some(mut node) = fields::read::<usize>(sentinel, 0) else { return false; };
    let mut walked = 0u64;
    while node != sentinel {
        if walked == size || node == 0 || node % 8 != 0 || !fields::object(node) { return false; }
        if fields::read::<usize>(node, 8) != Some(previous) { return false; }
        if let Some(listener) = fields::child(node, 0x10) {
            if !fields::object(listener) || fields::child(listener, 0).is_none() { return false; }
        }
        previous = node;
        let Some(next) = fields::read::<usize>(node, 0) else { return false; };
        node = next;
        walked += 1;
    }
    // The ring closes on the sentinel and holds exactly `size` nodes.
    walked == size && fields::read::<usize>(sentinel, 8) == Some(previous)
}

/// Phase B: leave the window group's arrays to its record and let `654dc0`
/// create the new group's. The nulls are the exact pointer state `0x37ae5e0`
/// leaves (`0x37ae69c`, `0x37ae740`) before `654dc0` stores the new arrays.
unsafe fn detach_dispatcher(entry: usize) {
    if let Some(info) = entry_info(entry) {
        fields::write_unchecked(info, 0x10, 0usize);
        fields::write_unchecked(info, 0x18, 0usize);
    }
}

unsafe fn install_dispatcher(entry: usize, arrays: [usize; 2]) {
    if let Some(info) = entry_info(entry) {
        fields::write_unchecked(info, 0x10, arrays[0]);
        fields::write_unchecked(info, 0x18, arrays[1]);
    }
}

/// Empty the window exactly as `0x658af0` requires (`+0x4150 == 0`) and as
/// `655320`'s bit-2 path would otherwise overwrite without freeing (§2.2).
unsafe fn clear_window(entry: usize) {
    fields::write_unchecked(entry, 0x4150, 0u64);
    for offset in [0x4160usize, 0x4168, 0x4170, 0x4178, 0x4180, 0x4188, 0x4190, 0x4198,
        // A native group's object records travel with its saved window; the
        // PT/Aegis branches of `0x658af0` build their object only into zeros.
        0x5a88, 0x5a90, 0x5a98] {
        fields::write_unchecked(entry, offset, 0usize);
    }
}

unsafe fn write_group_words(entry: usize, members: Members) {
    fields::write_unchecked(entry, 0x14, members.count() as u32);
    for (i, kind) in members.kinds().iter().enumerate() { fields::write_unchecked(entry, 0x18 + i * 4, *kind); }
    // Two actors for a pair group (`654dc0` then sets bit 1 natively), one otherwise.
    fields::write_unchecked(entry, 0x28, members.actor_count());
}

/// Restore every saved field except the active indices, which the caller sets.
unsafe fn restore_window(entry: usize, w: &Window) {
    fields::write_unchecked(entry, 0x14, w.count);
    for (i, kind) in w.kinds.iter().enumerate() { fields::write_unchecked(entry, 0x18 + i * 4, *kind); }
    fields::write_unchecked(entry, 0x28, w.actor_count);
    fields::write_unchecked(entry, 0x4150, w.installed);
    for (i, cell) in w.cells.iter().enumerate() { fields::write_unchecked(entry, 0x4160 + i * 8, *cell); }
    fields::write_unchecked(entry, 0x4178, w.ctx[0]);
    fields::write_unchecked(entry, 0x4180, w.ctx[1]);
    for (i, ctx) in w.group_ctx.iter().enumerate() { fields::write_unchecked(entry, 0x4188 + i * 8, *ctx); }
    fields::write_unchecked(entry, 0x591f, w.flags);
    fields::write_unchecked(entry, 0x5960, w.w5960);
    for (i, value) in w.cell_ints.iter().enumerate() { fields::write_unchecked(entry, 0x5964 + i * 4, *value); }
    fields::write_unchecked(entry, 0x5970, w.w5970);
    fields::write_unchecked(entry, 0x5974, w.w5974);
    for (i, word) in w.blob.iter().enumerate() { fields::write_unchecked(entry, 0x597c + i * 4, *word); }
    install_dispatcher(entry, w.dispatch);
    for (i, record) in w.records.iter().enumerate() { fields::write_unchecked(entry, 0x5a88 + i * 8, *record); }
}

/// The 0x238-byte object a native group's build stored at `+0x5a88`: its vtable
/// and the slot `0x3a6270` recorded at `+0x230` (`0x658d74`/`0x658eb8`, then the
/// init stores `w4 = slot`, `re/trainer_obj_3a6270_3a6b80.asm`).
unsafe fn valid_special_object(base: usize, object: usize, owner: u32) -> bool {
    fields::object(object)
        && fields::read::<usize>(object, 0) == Some(base + SPECIAL_OBJECT_VTABLE)
        && fields::read::<i32>(object, 0x230) == Some(owner as i32)
}

/// The entry's current `+0x5a88`, when it is a valid holder of `owner`.
unsafe fn current_holder_valid(base: usize, entry: usize, owner: u32) -> bool {
    fields::child(entry, 0x5a88).is_some_and(|holder| valid_special_object(base, holder, owner))
}

/// PT-TRAINER-PARK (`swap_model::special_articles`): at the exchange, the window that left (now `group`'s saved
/// record) keeps no live article. Without this, its claimed trainer stays drawn on stage (`sp1` `m-075849`). The
/// saved `+0x5a90` goes with the pool. A warm PT attach that activates gets its pool back before the activation
/// claims a trainer. No bank lock is held across the native calls.
unsafe fn exchange_articles(base: usize, role: usize, entry: usize, owner: u32, prepared: Prepared, activates: bool) {
    let group = match prepared { Prepared::Warm { group, .. } | Prepared::Build { group, .. } => group };
    let outgoing = {
        let state = BANKS[role].lock().unwrap();
        state.as_ref().and_then(|bank| bank.groups.get(group)).and_then(|g| g.saved).map(|w| (w.flags, w.records[0]))
    };
    if let Some((flags, holder)) = outgoing {
        let valid = holder != 0 && valid_special_object(base, holder, owner);
        if articles::retire_saved(base, owner, flags, holder, valid, "detach") {
            let mut state = BANKS[role].lock().unwrap();
            if let Some(saved) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)).and_then(|g| g.saved.as_mut()) {
                saved.records[1] = 0;
            }
        }
    }
    if activates && matches!(prepared, Prepared::Warm { .. }) {
        articles::rebuild_current(base, owner, entry, current_holder_valid(base, entry, owner), "warm attach");
    }
}

unsafe fn set_active_index(entry: usize, index: u8) {
    fields::write_unchecked(entry, 0x5918, index);
    fields::write_unchecked(entry, 0x5919, index);
}

/// `+0x5910`, the cell whose body id the activation's body event `0x1e` reads beside
/// `cell[+0x5918]` (`0x655f60` case 7, no count check). The commit left the window's previous
/// index there; an attached group must have that cell or the console faults on the null (C116.1
/// first console boot, Atmosphère report 01793544752, PC main+0x656668; Eden only logged
/// `Unmapped Read32 @ 0x8`). `swap_model::capabilities::previous_cell_index` picks the cell.
unsafe fn set_previous_index(entry: usize, index: u8) {
    fields::write_unchecked(entry, 0x5910, index);
}

/// Records `+0x5a20..+0x5a48` are null in steady state and unread by `0x658af0`
/// (§1.4); `+0x5a88/+0x5a90/+0x5a98` are null too unless the *current* window is
/// a native group, whose own object record must then be present and valid (a
/// detached group's records live in its saved `Window`); the cell vector is the
/// fixed inline one (§2.1).
unsafe fn window_shape_ok(base: usize, entry: usize, owner: u32, carries_records: bool) -> bool {
    let records_ok = if carries_records {
        fields::child(entry, 0x5a88).is_some_and(|object| valid_special_object(base, object, owner))
    } else {
        [0x5a88usize, 0x5a90, 0x5a98].iter().all(|offset| fields::read::<u64>(entry, *offset) == Some(0))
    };
    records_ok
        && [0x5a20usize, 0x5a28, 0x5a30, 0x5a38, 0x5a40, 0x5a48].iter()
            .all(|offset| fields::read::<u64>(entry, *offset) == Some(0))
        && fields::read::<usize>(entry, 0x4140) == Some(base + CELL_VECTOR_VTABLE)
        && fields::read::<u32>(entry, 0x4148) == Some(NATIVE_MEMBERS as u32)
        && fields::read::<usize>(entry, 0x4158) == Some(entry + 0x4160)
        && dispatcher_idle(entry)
}

// ----- group swap at the acknowledgement seam -----

#[derive(Clone, Copy)]
enum Prepared {
    /// Phase S: the detached group's fields are in the window; select its context.
    /// `dispatch` are the arrays now installed (leaked, logged, if the swap fails).
    /// `identity_move` is the shared `R+0x54..0x68` header and the entry identity
    /// bytes the exchange wrote (a Mii identity moved in or out); `None` leaves
    /// both as found (every other exchange).
    /// `outgoing`/`members` are the window that detached and the one attaching (U18: the copy
    /// registry, header and roster move from one to the other before the native call).
    Warm { group: usize, member: u8, via: u8, dispatch: [usize; 2], identity_move: Option<([u32; 5], MiiEntryBytes)>,
        outgoing: Members, members: Members },
    /// Phase B: the window is empty with the group's words; build synchronously, or (Phase B′,
    /// `asynchronous`) through the game's queued construction tasks with the ordinary verification
    /// at their settlement. A one-member Mii group's identity and configuration travel beside
    /// `Prepared` (`perform_swap`'s `mii_build`), not inside it, so this enum stays small and `Copy`.
    /// `lease` is the build's hold on `R+0x4c` bit 0, ended once the construction is over (U19).
    Build { group: usize, member: u8, via: u8, members: Members, live_before: u16, identity_move: Option<([u32; 5], MiiEntryBytes)>,
        outgoing: Members, lease: Lease, asynchronous: bool },
}

/// The verification and acknowledgement half of an exchange (`complete_exchange`), run right after
/// a synchronous construction or at the settlement of an asynchronous one.
struct ExchangeTail {
    prepared: Prepared,
    before: GroupOwner,
    alias: usize,
    setup: Option<crate::engine::mii::mii_setup::Outcome>,
    setup_ok: bool,
    construct_committed: Option<u64>,
    started: u64,
    asynchronous: Option<AsyncReport>,
}

/// What an asynchronous construction reports in its `built` line: the entry-level setup time, when
/// the tasks were queued and when each settled (all relative to the exchange's start), and the ticks
/// the entry ran meanwhile.
#[derive(Clone, Copy, Debug)]
struct AsyncReport { setup_us: u64, queued_us: u64, settled_us: [u64; NATIVE_MEMBERS], ticks: u32 }

/// Phase B′ in flight: the construction tasks of the group whose words are in the (empty) window.
struct Construction {
    tail: ExchangeTail,
    group: usize,
    count: usize,
    tasks: [usize; NATIVE_MEMBERS],
    /// F6: state 8's control-init task until it settled (state 9); then 0 and the bodies are queued (state 10).
    control: usize,
    members: Members,
    setup_us: u64,
    queued_us: u64,
    settled_us: [u64; NATIVE_MEMBERS],
    ticks: u32,
    waited_logged_us: u64,
}

/// State 0xb's cell install (`650010.c:886-890`): the bodies in order, then `+0x4150`.
unsafe fn install_cells(entry: usize, bodies: &[usize]) {
    for (i, body) in bodies.iter().enumerate() {
        fields::write_unchecked(entry, 0x4160 + i * 8, *body);
    }
    fields::write_unchecked(entry, 0x4150, bodies.len() as u64);
}

/// Free settled task objects as state 0xb's tail does (`650010.c:898-917`): the functor through its
/// own vtable (`+0x20` when it sits in place at `task+0x50`, `+0x28` otherwise), then the heap free.
unsafe fn free_tasks(base: usize, tasks: &[usize]) {
    let heap_free: unsafe extern "C" fn(usize) = std::mem::transmute(base + HEAP_FREE);
    for &task in tasks {
        if task == 0 { continue; }
        let holder = fields::read_unchecked::<usize>(task, construction_task::FUNCTOR_PTR);
        if holder != 0 {
            let slot = if holder == task + construction_task::FUNCTOR { 0x20 } else { 0x28 };
            if let Some(destroy) = fields::child(holder, 0).and_then(|vtable| fields::read::<usize>(vtable, slot)) {
                let destroy: unsafe extern "C" fn(usize) = std::mem::transmute(destroy);
                destroy(holder);
            }
        }
        heap_free(task);
    }
}

/// Phase B′ start, in native loader state 8's order (`native_entry_dispatch_6556b0.c` case 8, `0x6558bc-0x655a18`): `0x654dc0`
/// (dispatcher arrays) and `0x655320` (control contexts) on the game thread, then the control init `0x64f820` queued as a task on the
/// worker (`queue_task` with the functor `0x4f81510`, whose `+0x30` slot runs `0x64f820(*task)`, and the callback `0x655f30`).
/// State 9's wait for it is `settle_construction`'s, state 10's allocation `queue_bodies`. F6: the carried version ran `0x64f820` on
/// the game thread here (the synchronous `0x658af0` order, whose only caller is the respawn `0x65be14`); native runs it on the task
/// worker. Returns the control task and the time the game-thread setup ended, or `Err(0)` when the task object could not be
/// allocated: no body is allocated, so the ordinary verification fails and its rollback restores the window.
unsafe fn start_async_construction(base: usize, entry: *mut u8) -> Result<(usize, u64), usize> {
    let models: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + MODELS_SETUP);
    let contexts: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + COUNT_SETUP);
    split::mark(split::Step::Plugin);
    models(entry);
    split::mark(split::Step::Models);
    contexts(entry);
    split::mark(split::Step::Contexts);
    let control = queue_task(base, entry as usize, CONTROL_FUNCTOR_VTABLE, CONTROL_CALLBACK);
    split::mark(split::Step::Control);
    let setup_done = now_us();
    if control == 0 {
        install_cells(entry as usize, &[]);
        return Err(0);
    }
    Ok((control, setup_done))
}

/// One task object as loader states 8 and 10 build it (`6556b0.c` cases 8 and 10, `0x655ae8-0x655b1c`): the 0x90-byte object
/// `0x392e290(0x10, 0x90)`, zeroed, `object` at `+0` (the entry for state 8's control init, the body for state 10's construction),
/// queue id 1 at `+0x24`; the functor `{vtable, task, _, _, self}` built on the stack and swapped into `task+0x50` by `0x3cb050`, the
/// emptied local destroyed as native does; then `0x3549720(task+0x10, callback, 2, 1)`. Returns the task, or 0 (nothing queued)
/// when the heap returned null.
unsafe fn queue_task(base: usize, object: usize, vtable: usize, callback: usize) -> usize {
    use construction_task as task;
    let heap_alloc: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(base + HEAP_ALLOC);
    let functor_swap: unsafe extern "C" fn(*mut [usize; 5], usize) = std::mem::transmute(base + FUNCTOR_COPY);
    let schedule: unsafe extern "C" fn(usize, usize, u32, u32) = std::mem::transmute(base + TASK_SCHEDULE);
    split::mark(split::Step::Plugin);
    let t = heap_alloc(task::ALIGN, task::SIZE);
    split::mark(split::Step::TaskAlloc);
    if t == 0 {
        return 0;
    }
    std::ptr::write_bytes(t as *mut u8, 0, task::SIZE);
    fields::write_unchecked(t, task::BODY, object);
    fields::write_unchecked(t, task::QUEUE, task::QUEUE_ID);
    let mut functor = [base + vtable, t, 0, 0, 0];
    functor[4] = functor.as_ptr() as usize;
    functor_swap(&mut functor, t + task::FUNCTOR);
    let holder = functor[4];
    if holder != 0 {
        let slot = if holder == functor.as_ptr() as usize { 0x20 } else { 0x28 };
        if let Some(destroy) = fields::child(holder, 0).and_then(|vtable| fields::read::<usize>(vtable, slot)) {
            let destroy: unsafe extern "C" fn(usize) = std::mem::transmute(destroy);
            destroy(holder);
        }
    }
    split::mark(split::Step::Functor);
    schedule(t + task::HANDLE, base + callback, task::SCHEDULE_ARGS.0, task::SCHEDULE_ARGS.1);
    split::mark(split::Step::Schedule);
    t
}

/// Phase B′, loader state 10 (`6556b0.c:215-250`), once the control init settled: per kind `0x3ae5f0(0, kind, owner)` pops the
/// pool slot and runs the base constructor on the game thread, and the body's init and construct are queued on the task worker
/// (`queue_task` with the functor `0x4f69bb8` and the callback `0x3cb190`). Returns the task objects, or the number of bodies
/// already allocated when a native allocation returned null: those are installed into the cells so the ordinary verification
/// fails and its rollback destroys them.
unsafe fn queue_bodies(base: usize, entry: usize, owner: u32, members: Members) -> Result<[usize; NATIVE_MEMBERS], usize> {
    let allocate: unsafe extern "C" fn(u8, u32, u32) -> usize = std::mem::transmute(base + BODY_ALLOC);
    let mut tasks = [0usize; NATIVE_MEMBERS];
    let mut bodies = [0usize; NATIVE_MEMBERS];
    for (i, kind) in members.kinds().iter().copied().enumerate().take(members.count()) {
        split::mark(split::Step::Plugin);
        let body = allocate(0, kind, owner);
        split::mark(split::Step::Allocate);
        if body == 0 {
            install_cells(entry, &bodies[..i]);
            return Err(i);
        }
        bodies[i] = body;
        let t = queue_task(base, body, CONSTRUCT_FUNCTOR_VTABLE, TASK_CALLBACK);
        if t == 0 {
            install_cells(entry, &bodies[..=i]);
            return Err(i + 1);
        }
        tasks[i] = t;
    }
    Ok(tasks)
}

/// Phase B′ settlement on the entry tick (state 0xb's rule): once every task's pending count is 0,
/// install the bodies into the cells in order, free the tasks and run the ordinary verification and
/// acknowledgement. Until then the entry stays parked at loader `0xc` with an empty window while
/// frames run (the native shape of states 6-0xb); a request meanwhile is refused as busy.
unsafe fn settle_construction(entry: *mut u8, owner: u32, role: usize) {
    use construction_task as task;
    let base = BASE.load(Ordering::Acquire);
    let now = now_us();
    // State 9 (F6): the control init queued at the exchange runs on the task worker. Once its pending count is 0, state 10 frees
    // the task and, outside the bank lock, allocates the bodies and queues their construction; they settle on later ticks.
    let control = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut() else { return; };
        let Some(c) = bank.construction.as_mut() else { return; };
        if c.control == 0 {
            None
        } else {
            c.ticks = c.ticks.saturating_add(1);
            let pending = fields::read::<i32>(c.control, task::PENDING).unwrap_or(i32::MAX);
            if pending > 0 {
                if now.saturating_sub(c.waited_logged_us) >= 5_000_000 {
                    c.waited_logged_us = now;
                    trace!("[GroupSwap] control init pending owner={} group={} pending={} us={} ticks={}",
                        owner, c.group, pending, now.saturating_sub(c.queued_us), c.ticks);
                }
                return;
            }
            let control = c.control;
            c.control = 0;
            Some((control, c.members, c.group, now.saturating_sub(c.queued_us)))
        }
    };
    if let Some((control, members, group, control_us)) = control {
        split::start_queue(role);
        free_tasks(base, &[control]);
        split::mark(split::Step::FreeTasks);
        let queued = queue_bodies(base, entry as usize, owner, members);
        let mut state = BANKS[role].lock().unwrap();
        match queued {
            Ok(tasks) => {
                match state.as_mut().and_then(|bank| bank.construction.as_mut()) {
                    Some(c) => {
                        c.tasks = tasks;
                        c.queued_us = now_us();
                        trace!("[GroupSwap] bodies queued owner={} group={} tasks={:x?} control_us={} us={}; frames run until the tasks settle",
                            owner, group, &tasks[..members.count()], control_us, c.queued_us.saturating_sub(c.tail.started));
                    }
                    None => trace!("[GroupSwap] bodies queued owner={} group={} UNEXPECTED: construction gone; tasks {:x?} leak (reported)",
                        owner, group, &tasks[..members.count()]),
                }
                drop(state);
                split::report_queue(owner, group);
            }
            Err(installed) => {
                let construction = state.as_mut().and_then(|bank| bank.construction.take());
                drop(state);
                trace!("[GroupSwap] bodies queued owner={} group={} UNEXPECTED: a native allocation returned null after {} of {} bodies; the verification rolls back",
                    owner, group, installed, members.count());
                if let Some(construction) = construction {
                    let mut tail = construction.tail;
                    tail.asynchronous = Some(AsyncReport { setup_us: construction.setup_us,
                        queued_us: construction.queued_us.saturating_sub(tail.started), settled_us: construction.settled_us, ticks: construction.ticks });
                    complete_exchange(base, entry, owner, role, tail);
                }
            }
        }
        return;
    }
    let construction = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut() else { return; };
        let Some(c) = bank.construction.as_mut() else { return; };
        c.ticks = c.ticks.saturating_add(1);
        let mut pending = [0i32; NATIVE_MEMBERS];
        let started = c.tail.started;
        for (i, task_object) in c.tasks.iter().enumerate().take(c.count) {
            pending[i] = fields::read::<i32>(*task_object, task::PENDING).unwrap_or(i32::MAX);
            if pending[i] <= 0 && c.settled_us[i] == 0 { c.settled_us[i] = now.saturating_sub(started); }
        }
        if !task::settled(&pending[..c.count]) {
            if now.saturating_sub(c.waited_logged_us) >= 5_000_000 {
                c.waited_logged_us = now;
                trace!("[GroupSwap] constructing owner={} group={} pending={:?} us={} ticks={}",
                    owner, c.group, &pending[..c.count], now.saturating_sub(c.queued_us), c.ticks);
            }
            return;
        }
        bank.construction.take().unwrap()
    };
    split::start_settle(role);
    let mut bodies = [0usize; NATIVE_MEMBERS];
    for (i, task_object) in construction.tasks.iter().enumerate().take(construction.count) {
        bodies[i] = fields::read::<usize>(*task_object, task::BODY).unwrap_or(0);
    }
    install_cells(entry as usize, &bodies[..construction.count]);
    split::mark(split::Step::Install);
    free_tasks(base, &construction.tasks[..construction.count]);
    split::mark(split::Step::FreeTasks);
    let mut tail = construction.tail;
    tail.asynchronous = Some(AsyncReport { setup_us: construction.setup_us,
        queued_us: construction.queued_us.saturating_sub(tail.started), settled_us: construction.settled_us, ticks: construction.ticks });
    complete_exchange(base, entry, owner, role, tail);
    split::report_settle(owner, construction.group);
}

/// A destroyer (finalize, reload, scene end) while a Phase B′ construction is in flight: wait for
/// every task as state 0xb's tail waits (`0x354ccd0` on the queue the task names, `650010.c:886-924`),
/// install the bodies into the cells so native's own window destroyer frees them, free the tasks,
/// hand the request bit back and settle the ledger. The cancel byte is not written: a cancelled task
/// would leave a body that never ran its init, which no native destroyer expects; the wait is bounded
/// by the remaining construction (about the build time), as native's own wait is.
unsafe fn drain_construction(entry: *const u8, owner: u32, role: usize, reason: &str) {
    use construction_task as task;
    let base = BASE.load(Ordering::Acquire);
    let construction = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        match bank.construction.take() { Some(construction) => construction, None => return }
    };
    let started = now_us();
    let wait: unsafe extern "C" fn(u32, usize) = std::mem::transmute(base + TASK_WAIT);
    let mut bodies = [0usize; NATIVE_MEMBERS];
    let mut waited = 0usize;
    let mut construction = construction;
    // State 9 at a destroyer (F6): wait for the control init as native waits for its tasks, then run state 10 (allocate and queue
    // the bodies), so the wait below finishes the construction and the destroyer finds the shape it finds for every drained build.
    let mut partial = None;
    if construction.control != 0 {
        let control = std::mem::replace(&mut construction.control, 0);
        if fields::read::<i32>(control, task::PENDING).unwrap_or(0) > 0 {
            let queue = fields::read::<u32>(control, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
            if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
                wait(handle, control + task::PENDING);
                waited += 1;
            }
        }
        free_tasks(base, &[control]);
        match queue_bodies(base, entry as usize, owner, construction.members) {
            Ok(tasks) => construction.tasks = tasks,
            Err(installed) => {
                trace!("[GroupSwap] construction drained owner={} group={} UNEXPECTED: a native allocation returned null after {} of {} bodies (installed as allocated)",
                    owner, construction.group, installed, construction.count);
                partial = Some(installed);
            }
        }
    }
    for (i, &t) in construction.tasks.iter().enumerate().take(if partial.is_some() { 0 } else { construction.count }) {
        if fields::read::<i32>(t, task::PENDING).unwrap_or(0) > 0 {
            let queue = fields::read::<u32>(t, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
            if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
                wait(handle, t + task::PENDING);
                waited += 1;
            }
        }
        bodies[i] = fields::read::<usize>(t, task::BODY).unwrap_or(0);
    }
    if partial.is_none() {
        install_cells(entry as usize, &bodies[..construction.count]);
        free_tasks(base, &construction.tasks[..construction.count]);
    }
    let alias = construction.tail.alias;
    if let Prepared::Build { group, lease, members, .. } = construction.tail.prepared {
        let alias_ok = alias != 0 && resource(base, owner) == Some(alias);
        if alias_ok {
            let (how, now, after) = r_handle::end_request(ledger_role(role), alias, lease);
            trace!("[RequestBit] end owner={} group={} how={:?} before={:#x} built={:#x} after={:#x} reason=drained",
                owner, group, how, lease.before, now, after);
            // Every body ran its construction tail (the waits returned): their tokens settle as a built group's.
            // (F6: not when state 10 failed to allocate at the destroyer: those members never constructed and keep their keys.)
            if partial.is_none() {
                r_handle::construction_settled(ledger_role(role), group, &members.kinds()[..members.count()]);
            }
        }
        trace!("[GroupSwap] construction drained owner={} group={} reason={} bodies={:x?} waited={} us={} alias_ok={}",
            owner, group, reason, &bodies[..construction.count], waited, now_us().saturating_sub(started), alias_ok);
    }
}

/// Executes the committed cross-group request. Lock discipline: decide and
/// write entry fields under the bank lock, release it for every native call
/// (the table hooks take the same lock), then verify and acknowledge.
/// MII-PHASEB: the Mii setup's Phase B preflight for a Prefetched one-member Mii group, run
/// inside the parked change before any window mutation; `None` for every other build. It is
/// the same rule `prepare_mii_group` re-proves after the identity install (the identity is
/// predicted here: `R+0x68`/`FE+0x46` are written by that install).
unsafe fn mii_build_preflight(base: usize, entry: usize, owner: u32, bank: &Bank, group: usize) -> Option<Result<(), &'static str>> {
    let g = bank.groups.get(group)?;
    if g.state != GroupState::Prefetched || g.members.special() != Special::Mii { return None; }
    let (identity, c) = g.identity.zip(g.configuration)?;
    Some(crate::runtime::roster::mii_setup::phase_b_admitted(base, entry, owner, bank.alias, &identity, &c))
}

unsafe fn perform_swap(entry: *mut u8, owner: u32, role: usize) {
    let base = BASE.load(Ordering::Acquire);
    // A one-member Mii group's identity and configuration for the native Mii-setup step
    // (WP-M2): set by the Phase B arm below, read after the bank lock is dropped.
    let mut mii_build: Option<(MiiIdentity, Configuration)> = None;
    // K15: the Phase B construction and U18's attach register bodies through `prepare_entry`, whose
    // copy relations load Kirby's hat models (`0x17f06a0`); their packages must be resident first, as
    // native's completion and its holders wait guarantee (`swap_model::copy_packages`). No bank lock
    // is held across the native calls.
    let copies = BANKS[role].lock().unwrap().as_ref().filter(|bank| bank.swap.is_some())
        .map(|bank| (copy_package_sets(bank), bank.alias));
    let copies_resident = match copies {
        Some((sets, alias)) if alias != 0 && resource(base, owner) == Some(alias) =>
            crate::runtime::roster::copy_packages::admit(base, role, owner, alias, &sets),
        _ => true,
    };
    let (prepared, before, alias) = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut() else { return; };
        let Some(swap) = bank.swap else { return; };
        let before = group_owner(entry, bank).filter(|now| match bank.exchange {
            Some(exchange) => exchange.permits(*now),
            None => now.header[1] == u32::from_le_bytes(now.descriptor[0x18..0x1c].try_into().unwrap()),
        });
        let settings = crate::runtime::roster::cycle_preload_config::snapshot();
        let reason = if before.is_none() { Some(Fallback::OwnerReceiptChanged) }
            else if !groups_enabled(&settings) { Some(Fallback::GroupSwapDisabled) }
            else if swap.group >= bank.groups.len() || !bank.groups[swap.group].state.reachable() { Some(Fallback::NotReachable) }
            else if !window_shape_ok(base, entry as usize, owner, bank.members.carries_records()) { Some(Fallback::WindowShape) }
            else if resource(base, owner) != Some(bank.alias) || bank.alias == 0 { Some(Fallback::AliasChanged) }
            else if !resource_ready(bank.alias) { Some(Fallback::ResourceNotReady) }
            else if NativeRegistry::new(base, bank.alias, owner).is_none() { Some(Fallback::RegistryUnreadable) }
            else if bank.groups[swap.group].state == GroupState::Prefetched
                && pool_counts(base).is_none_or(|(capacity, live)|
                    (capacity as usize).saturating_sub(usize::from(live)) < bank.groups[swap.group].members.count()) {
                Some(Fallback::PoolFull)
            }
            // WP4 (Astra §4.2): a pair-rule build resets the whole R; another batch's keys on it
            // defer the build (`switch` drained them; a key that reappeared keeps the group Prefetched).
            else if bank.groups[swap.group].state == GroupState::Prefetched
                && pair_resource_references(*bank.groups[swap.group].members).is_some()
                && r_handle::reset_exclusive(ledger_role(role), swap.group).is_err() {
                Some(Fallback::PairResetWait)
            }
            // K15: every package the registration's copy relations load is resident.
            else if !copies_resident { Some(Fallback::CopyPackagesLoading) }
            // MII-PHASEB: a one-member Mii group's build replays state 6's Mii setup. Its
            // preflight (Phase B seam: the entry parked in this change; identity predicted)
            // decides here, before the window detaches, so a refused setup never needs the
            // rollback; the re-proof after the identity install checks the same clauses. That
            // refusal touched nothing: the group keeps its keys and stays `Prefetched` (U20 keeps
            // that rule as a transient reason).
            else if let Some(Err(why)) = mii_build_preflight(base, entry as usize, owner, bank, swap.group) {
                Some(Fallback::MiiSetupRefused(why))
            }
            else { None };
        if let Some(reason) = reason {
            // U20 (`swap_model::fallback`): a transient reason keeps the group (and its keys) for the
            // retry; a permanent one fails it, and a Prefetched group returns every key it holds
            // through the ledger's inverse below, after the bank lock is dropped.
            let state_before = bank.groups.get(swap.group).map(|g| g.state);
            let disposition = state_before.map_or(Disposition::Keep, |state| fallback::disposition(reason, state));
            trace!("[GroupSwap] fallback owner={} kind={:#x} reason={}; in-window member {} activates; group={} state={:?} disposition={:?}",
                owner, swap.kind, reason.text(), swap.via, swap.group, state_before, disposition);
            if matches!(disposition, Disposition::Fail | Disposition::FailReturningKeys) {
                bank.groups[swap.group].state = GroupState::Failed;
                GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
                    .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
            }
            let returning = (disposition == Disposition::FailReturningKeys).then_some((swap.group, bank.alias, *bank.members));
            bank.swap = None;
            fields::write_unchecked(entry as usize, 0x5933, 1u8);
            bank.acknowledged = true;
            drop(state);
            if let Some(returning) = returning {
                return_group_keys(base, entry, owner, role, returning, reason.text());
            }
            return;
        }
        let mut saved_x = save_window(entry as usize);
        let x_members = std::mem::replace(&mut bank.members, Box::new(*bank.groups[swap.group].members));
        let target_members = *bank.members;
        // The Mii identity follows the window: `R` holds the window group's identity
        // (MII-GROUPS-DESIGN §3.3). When a Mii moves in or out, save the words the
        // detaching window leaves and write the words the attaching window needs.
        let x_identity = std::mem::replace(&mut bank.identity, bank.groups[swap.group].identity);
        let identity_moves = x_identity.is_some() || bank.identity.is_some();
        if identity_moves {
            saved_x.identity = Some((save_resource_words(bank.alias), save_entry_identity(entry as usize)));
        }
        let alias = bank.alias;
        let target_identity = bank.identity;
        // The window's previous active index the commit stored at `+0x5910` (`service` proved it).
        let from = bank.from;
        let group = &mut bank.groups[swap.group];
        group.identity = x_identity;
        let prepared = match (group.state, group.saved.take()) {
            (GroupState::Built, Some(y)) => {
                let x_bodies = std::mem::replace(&mut bank.bodies, group.bodies);
                group.members = x_members; group.bodies = x_bodies; group.saved = Some(saved_x);
                trace!("[GroupSwap] detach owner={} group={} kinds={:x?} bodies={:?}", owner, swap.group,
                    &group.members.kinds()[..group.members.count()], x_bodies);
                restore_window(entry as usize, &y);
                set_active_index(entry as usize, swap.member);
                set_previous_index(entry as usize, previous_cell_index(from, swap.member, target_members.count()));
                // A warm return into a Mii group rewrites its identity (R, entry bytes,
                // D mirror); a return from one restores what that window saved.
                let identity_move = match (target_identity, y.identity) {
                    (Some(identity), _) => Some((identity.header(owner), write_identity(entry as usize, alias, identity))),
                    (None, Some((words, saved))) => {
                        write_resource_words(alias, &words);
                        write_entry_identity(entry as usize, &saved);
                        Some((header_of(owner, &words), saved.entry))
                    }
                    (None, None) => None,
                };
                trace!("[GroupSwap] attach owner={} group={} member={} kinds={:x?} dispatch={:x?} identity={} header={:x?}", owner, swap.group, swap.member,
                    &target_members.kinds()[..target_members.count()], y.dispatch, target_identity.is_some(), identity_move.map(|m| m.0));
                Prepared::Warm { group: swap.group, member: swap.member, via: swap.via, dispatch: y.dispatch, identity_move,
                    outgoing: *group.members, members: target_members }
            }
            (GroupState::Prefetched, None) => {
                let x_bodies = std::mem::replace(&mut bank.bodies, [Body::default(); NATIVE_MEMBERS]);
                group.members = x_members; group.bodies = x_bodies; group.saved = Some(saved_x); group.state = GroupState::Built;
                trace!("[GroupSwap] detach owner={} group={} kinds={:x?} bodies={:?}", owner, swap.group,
                    &group.members.kinds()[..group.members.count()], x_bodies);
                clear_window(entry as usize);
                // The window group keeps its listener registrations (saved_x.dispatch);
                // `654dc0` creates the new group's arrays over the nulls (§14.1).
                detach_dispatcher(entry as usize);
                write_group_words(entry as usize, target_members);
                set_active_index(entry as usize, swap.member);
                // Before the synchronous build: Eden's null read sat inside the construction window.
                set_previous_index(entry as usize, previous_cell_index(from, swap.member, target_members.count()));
                // `R+0x4c |= 1` and the pair references go through the ledger's writer (WP4).
                // The bit is the construction's (the path builder `0x17e99c0` gates on it) and
                // is handed back when the construction is over (U19, `end_request` below).
                let (lease, _) = r_handle::request_bit(ledger_role(role), bank.alias);
                let flags = lease.before;
                // Pair group: one `R+0x50` reference per body before the synchronous
                // build, or the first construction tail resets R under the second
                // (`0x60de60`, C122 rule; the window left `R+0x50 = 0`). Written only under
                // the reset-batch permit `reset_exclusive` just proved (Astra §4.2).
                let refs_before = fields::read_unchecked::<u32>(bank.alias, 0x50);
                if let Some(references) = pair_resource_references(target_members) {
                    if r_handle::set_pair_refs(ledger_role(role), bank.alias, swap.group, references as u8).is_err() {
                        trace!("[GroupSwap] build owner={} group={} refs NOT written: reset batch no longer exclusive", owner, swap.group);
                    }
                }
                // Mii group: `R` takes the Mii identity for the construction and keeps
                // it while the Mii is the window (the costume selectors at `0x607dfc`,
                // `0x17e9ae0` and the unload `0x611b04` all key on `R+0x58`); the entry
                // and the D mirror take the identity bytes state 6 would have derived,
                // so the entry reads as a canonical exact-Mii entry with the bank flag.
                let identity_move = target_identity.map(|identity|
                    (identity.header(owner), write_identity(entry as usize, alias, identity)));
                let live_before = pool_counts(base).map_or(0, |(_, live)| live);
                // Phase B′ (operator decision 2026-11-02): an ordinary group constructs through the
                // game's queued tasks; Mii, native and pair groups keep the synchronous build (the
                // operator-approved restriction, `async_build_eligible`), as does every group while
                // the path's fingerprints have not passed.
                let eligibility = async_build_eligible(target_members, target_identity.is_some());
                let fingerprinted = ASYNC_ENABLED.load(Ordering::Acquire);
                let asynchronous = fingerprinted && eligibility.is_ok();
                trace!("[GroupSwap] build owner={} group={} kinds={:x?} special={:?} actors={} R={:#x} flags={:#x}->{:#x} refs={}->{} map={} task_pending={} pool_live={} records={:x?} dispatch_detached={:x?} identity={:?} mode={} reason={}",
                    owner, swap.group, &target_members.kinds()[..target_members.count()], target_members.special(),
                    target_members.actor_count(), bank.alias, flags,
                    fields::read_unchecked::<u8>(bank.alias, 0x4c), refs_before, fields::read_unchecked::<u32>(bank.alias, 0x50),
                    fields::read_unchecked::<u64>(bank.alias, 0x18), task_pending(bank.alias), live_before,
                    group.saved.map(|w| w.records), group.saved.map(|w| w.dispatch),
                    target_identity.map(|i| (i.index, i.tokens)),
                    if asynchronous { "async" } else { "sync" },
                    eligibility.err().unwrap_or(if fingerprinted { "-" } else { "fingerprint" }));
                mii_build = target_identity.zip(group.configuration).filter(|_| target_members.special() == Special::Mii);
                Prepared::Build { group: swap.group, member: swap.member, via: swap.via, members: target_members, live_before, identity_move,
                    outgoing: *group.members, lease, asynchronous }
            }
            (found, saved) => {
                // Inconsistent record: put the table back and refuse this swap. A permanent
                // failure: a Prefetched record returns its keys first (U20, `swap_model::fallback`).
                bank.members = x_members;
                group.saved = saved;
                group.identity = target_identity;
                bank.identity = x_identity;
                group.state = GroupState::Failed;
                let returning = (found == GroupState::Prefetched).then_some((swap.group, bank.alias, *bank.members));
                bank.swap = None;
                trace!("[GroupSwap] fallback owner={} reason=group record inconsistent state={:?}", owner, found);
                fields::write_unchecked(entry as usize, 0x5933, 1u8);
                bank.acknowledged = true;
                drop(state);
                if let Some(returning) = returning {
                    return_group_keys(base, entry, owner, role, returning, "group record inconsistent");
                }
                return;
            }
        };
        bank.target = swap.member;
        bank.swap = None;
        bank.swap_started = Some((swap.kind, swap.requested_us));
        (prepared, before.unwrap(), alias)
    };
    let started = now_us();
    split::start(role, started);
    // U18: the outgoing window leaves the copy registry and the entry's header and roster become
    // the incoming group's before native runs for it, as a Reload into that group would leave
    // them; a warm attach then registers the group as its inits did (`swap_model::copy_registry`).
    // Unconditional for a build (before the Mii setup, which may skip the construction): the
    // rollback's `restore` then always finds the window unregistered. The reason chain above
    // proved the registry readable under this R and owner.
    let registry_ok = match (prepared, NativeRegistry::new(base, alias, owner)) {
        (Prepared::Warm { outgoing, members, .. }, Some(mut registry)) => {
            copy_registry::attach(&mut registry, outgoing, members);
            split::mark(split::Step::Plugin);
            registry.trace("attach", outgoing, members);
            split::mark(split::Step::Log);
            true
        }
        (Prepared::Build { outgoing, members, .. }, Some(mut registry)) => {
            copy_registry::build(&mut registry, outgoing, members);
            split::mark(split::Step::Plugin);
            registry.trace("build", outgoing, members);
            split::mark(split::Step::Log);
            true
        }
        (_, None) => {
            trace!("[Registry] unavailable owner={} after the reason chain; native call skipped, rollback follows", owner);
            false
        }
    };
    // A Mii group's Phase B first replays the loader's native Mii setup for the
    // identity now installed in R/FE/E/D (`0x67e7d0` then `0x67ead0`, WP-M2): without
    // it the record keeps a zero resource pair (B124PRE D-MII1). A refused preflight
    // or a failed setup skips the build; the verification below then fails and the
    // ordinary rollback restores the window. No lock is held across the calls.
    let setup = match (&prepared, &mii_build) {
        (Prepared::Build { .. }, Some((identity, c))) if registry_ok =>
            Some(crate::runtime::roster::mii_setup::prepare_mii_group(base, entry as usize, owner, alias, identity, c)),
        _ => None,
    };
    split::mark(split::Step::MiiSetup);
    let setup_ok = registry_ok && !matches!(setup, Some(crate::engine::mii::mii_setup::Outcome::Refused(_)
        | crate::engine::mii::mii_setup::Outcome::Failed(_)));
    // GS-PANIC: the build's own charge (the bodies and their article models, `0x6081f8 → … → 0x392e290`).
    split::mark(split::Step::Plugin);
    let construct_committed = matches!(prepared, Prepared::Build { .. }).then(|| MemorySample::capture().committed.get()).flatten();
    split::mark(split::Step::Memory);
    exchange_articles(base, role, entry as usize, owner, prepared, registry_ok);
    split::mark(split::Step::Articles);
    match prepared {
        Prepared::Warm { .. } if registry_ok => select_control(base, entry),
        Prepared::Warm { .. } => {}
        Prepared::Build { asynchronous: true, members, group, .. } if setup_ok => {
            // Phase B′: queue the construction and return; `settle_construction` verifies and
            // acknowledges on the tick the tasks settle. Frames run meanwhile.
            match start_async_construction(base, entry) {
                Ok((control, setup_done)) => {
                    let queued_us = now_us();
                    let mut state = BANKS[role].lock().unwrap();
                    match state.as_mut() {
                        Some(bank) => {
                            bank.construction = Some(Construction {
                                tail: ExchangeTail { prepared, before, alias, setup, setup_ok, construct_committed, started, asynchronous: None },
                                group, count: members.count(), tasks: [0; NATIVE_MEMBERS], control, members,
                                setup_us: setup_done.saturating_sub(started), queued_us,
                                settled_us: [0; NATIVE_MEMBERS], ticks: 0, waited_logged_us: queued_us });
                            trace!("[GroupSwap] build owner={} group={} mode=async queued={} control={:#x} setup_us={} us={}; the control init runs on the task worker, then the bodies are queued (native states 8-10)",
                                owner, group, members.count(), control, setup_done.saturating_sub(started), queued_us.saturating_sub(started));
                            split::report_setup(owner, group, &members.kinds()[..members.count()], setup_done.saturating_sub(started));
                        }
                        None => trace!("[GroupSwap] build owner={} group={} mode=async UNEXPECTED: bank gone after queuing; control task {:#x} leaks (reported)",
                            owner, group, control),
                    }
                    return;
                }
                Err(installed) => trace!("[GroupSwap] build owner={} group={} mode=async UNEXPECTED: a native allocation returned null after {} of {} bodies; the verification rolls back",
                    owner, group, installed, members.count()),
            }
        }
        Prepared::Build { .. } if setup_ok => sync_construct(base, entry),
        Prepared::Build { .. } => {}
    }
    complete_exchange(base, entry, owner, role, ExchangeTail { prepared, before, alias, setup, setup_ok, construct_committed, started, asynchronous: None });
}

/// The exchange's verification and acknowledgement (or rollback): right after a synchronous
/// construction, or at the settlement of an asynchronous one (`settle_construction`).
unsafe fn complete_exchange(base: usize, entry: *mut u8, owner: u32, role: usize, tail: ExchangeTail) {
    let ExchangeTail { prepared, before, alias, setup, setup_ok, construct_committed, started, asynchronous } = tail;
    if let Prepared::Build { group, members: built, .. } = prepared {
        if setup_ok {
            let kinds = &built.kinds()[..built.count()];
            log_memory(owner, group, kinds, "build", construct_committed, MemorySample::capture(), memory::group_cost(kinds).build);
        }
    }
    split::mark(split::Step::Memory);
    // U19: the construction is over (each body's tail ran inside `0x658af0`), so the request bit
    // the build set goes back, as the reset would hand it back if the last tail had emptied the
    // map. Other batches' keys keep the map non-empty here; a bit left set would let a later load
    // completion `0x17e8440` take G68 copy keys for the header members that per-kind releases
    // never return (`swap_model::request_bit`). Before the verification and any rollback, which
    // construct nothing.
    if let Prepared::Build { group, lease, .. } = prepared {
        let (how, now, after) = r_handle::end_request(ledger_role(role), alias, lease);
        trace!("[RequestBit] end owner={} group={} how={:?} before={:#x} built={:#x} after={:#x} map={} task_pending={} constructed={}",
            owner, group, how, lease.before, now, after, fields::read::<u64>(alias, 0x18).unwrap_or(0), task_pending(alias), setup_ok);
        if how == LeaseEnd::Changed {
            trace!("[RequestBit] UNEXPECTED owner={} group={}: R+0x4c changed during the construction ({:#x} -> {:#x}); left as found", owner, group, lease.before, now);
        }
    }
    split::mark(split::Step::Request);
    let elapsed = now_us().saturating_sub(started);
    // Verify under the lock; roll back on any failure.
    let rollback = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut() else { return; };
        let members = *bank.members;
        let (group, member, via, ok) = match prepared {
            Prepared::Warm { group, member, via, .. } => {
                let ok = bodies(entry, owner, members, None) == Some(bank.bodies)
                    && selected_context(entry, owner, members, member)
                    && dispatcher_present(entry as usize);
                (group, member, via, ok)
            }
            Prepared::Build { group, member, via, members: built, live_before, .. } => {
                let live_after = pool_counts(base).map_or(0, |(_, live)| live);
                let cells = bodies(entry, owner, built, None);
                // A native group's build created its object (the trainer is not a
                // pool body: `0x392e290` at `0x658d10`), and `654dc0` wrote its flags.
                let records = [0usize, 8, 16].map(|o| read::<usize>(entry, 0x5a88 + o));
                let records_ok = if built.carries_records() {
                    valid_special_object(base, records[0], owner)
                } else {
                    records == [0, 0, 0]
                };
                let flags_ok = read::<u8>(entry, 0x591f) == built.expected_flags(member);
                // `654dc0` ran iff fresh, idle dispatcher arrays replaced the nulls.
                let ok = setup_ok
                    && read::<u64>(entry, 0x4150) == built.count() as u64
                    && cells.is_some()
                    && selected_context(entry, owner, built, member)
                    && usize::from(live_after) == usize::from(live_before) + built.count()
                    && dispatcher_present(entry as usize)
                    && records_ok && flags_ok;
                if ok { bank.bodies = cells.unwrap(); }
                trace!("[GroupSwap] built owner={} group={} ok={} us={} installed={} pool_live={}->{} bodies={:?} map={} flags={:#x} refs={} entry_flags={:#x} records={:x?} dispatch={:x?} mii_setup={:?} mode={} setup_us={} queued_us={} settled_us={:?} ticks={} max_tick_gap_us={}",
                    owner, group, ok, elapsed, read::<u64>(entry, 0x4150), live_before, live_after, cells,
                    fields::read::<u64>(bank.alias, 0x18).unwrap_or(0), fields::read::<u8>(bank.alias, 0x4c).unwrap_or(0),
                    fields::read::<u32>(bank.alias, 0x50).unwrap_or(0), read::<u8>(entry, 0x591f), records,
                    dispatcher_arrays(entry as usize), setup,
                    if asynchronous.is_some() { "async" } else { "sync" },
                    asynchronous.map_or(0, |r| r.setup_us), asynchronous.map_or(0, |r| r.queued_us),
                    asynchronous.map_or([0; NATIVE_MEMBERS], |r| r.settled_us), asynchronous.map_or(0, |r| r.ticks),
                    bank.tick.max_gap_us);
                (group, member, via, ok)
            }
        };
        // A built group: its members' construction tails released their keys (settled in the
        // ledger below); the object kind a native group registered for its build has no
        // construction tail and is released through the ledger so the map empties and the native
        // reset runs (`0x17e8848-0x17e8880`), the Aegis `0x1338` orphan key first (D-SG1-RCA).
        let post_release = match (prepared, ok) {
            (Prepared::Build { group, members: built, .. }, true) => Some((group, built, bank.alias)),
            _ => None,
        };
        // The entry identity bytes stay the window's, unless this exchange moved a Mii
        // identity in (its header and bytes) or out (the words and bytes the window had
        // saved). The header kinds `R+0x58..0x60` are the attached window's (U18); the
        // owner and costume words keep that rule.
        let (mut expected_header, expected_entry) = match prepared {
            Prepared::Warm { identity_move: Some((header, bytes)), .. }
            | Prepared::Build { identity_move: Some((header, bytes)), .. } => (header, Some(bytes)),
            _ => (before.header, None),
        };
        expected_header[1..4].copy_from_slice(&members.kinds());
        let exchange = ok.then(|| group_owner(entry, bank)
            .and_then(|after| GroupExchange::new_with_entry(before, after, members, expected_header, expected_entry))).flatten();
        if let Some(exchange) = exchange {
            bank.exchange = Some(exchange);
            fields::write_unchecked(entry as usize, 0x5933, 1u8);
            bank.acknowledged = true;
            bank.target = member;
            trace!("[GroupSwap] acknowledged owner={} group={} member={} previous={} us={}", owner, group, member,
                read::<u8>(entry, 0x5910), elapsed);
            (None, post_release)
        } else {
            (Some((group, via)), None)
        }
    };
    let (rollback, post_release) = rollback;
    split::mark(split::Step::Verify);
    // Diag only: who listens to the body event the activation posts for this exchange.
    #[cfg(feature = "diag")]
    if rollback.is_none() { super::trace_event_listeners("[GroupSwap] body-event listeners at acknowledgement", 0x1e); }
    if let Some((group, built, alias)) = post_release {
        let lrole = ledger_role(role);
        r_handle::construction_settled(lrole, group, &built.kinds()[..built.count()]);
        if let Some(kind) = built.object_kind() {
            if alias != 0 && resource(base, owner) == Some(alias) {
                let object = [kind];
                if fields::read::<u64>(alias, 0x18).unwrap_or(0) != 0 {
                    let receipt = r_handle::release(base, lrole, Some(group), Some(&object[..]), &Occupancy::default(), "post-build object release");
                    trace!("[GroupSwap] released owner={} object_kind={:#x} map={} flags={:#x} orphan_keys={:x?}", owner, kind,
                        fields::read::<u64>(alias, 0x18).unwrap_or(0), fields::read::<u8>(alias, 0x4c).unwrap_or(0), receipt.keys);
                } else {
                    // IC-PAIR-STALL (Yoga `swon0` step 82): the object's own native tail (`0x34cb1ec`
                    // PT `0x72`, `0x33f45e4` Aegis `0x75`) ran inside the build and the last tail's
                    // reset emptied the map, so native already freed every key of the object token:
                    // it is settled with no native call. Left in the ledger it stayed a batch with
                    // keys the next pair-rule window's reset-batch permit refused on.
                    let settled = r_handle::settle(lrole, group, &object, "post-build object freed by native (map=0)");
                    trace!("[GroupSwap] settled owner={} object_kind={:#x} map=0 settled={:x?}", owner, kind, settled);
                }
            }
        }
        // The role's real ledger against the owner's map (WP2's line at the build seam, U13:
        // `r_handle::reconcile` supersedes WP2's mirror-based `observe::reconcile`).
        if alias != 0 && resource(base, owner) == Some(alias) { r_handle::reconcile(lrole, "build"); }
    }
    split::mark(split::Step::Ledger);
    let Some((group, via)) = rollback else { return; };
    // PT-TRAINER-PARK: a failed warm attach rebuilt its pool; it goes before the previous window returns.
    if matches!(prepared, Prepared::Warm { .. }) {
        articles::retire_current(base, owner, entry as usize, current_holder_valid(base, entry as usize, owner), "warm rollback");
    }
    // Failure: destroy whatever the build produced in the window, put the
    // previous window back and let native activate the committed in-window
    // member — through the one teardown protocol (routes, bodies, contexts,
    // dispatcher arrays, records, then the unbuilt members' keys; finalize's order).
    let (bundle, alias, records, current) = {
        let state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_ref() else { return; };
        let members = *bank.members;
        let mut detached = DetachedGroup { owner, ..Default::default() };
        let mut records = [0usize; 3];
        let mut current = [0usize; 2];
        if matches!(prepared, Prepared::Build { .. }) {
            for i in 0..members.count() {
                let kind = members.kinds()[i];
                let built = fields::child(entry as usize, 0x4160 + i * 8).and_then(|address| {
                    let id = fields::read::<u32>(address, 8)?;
                    valid_parked_body(base, Body { address, id }, kind, owner).then_some(id)
                });
                match built {
                    Some(id) => detached.bodies.push(BodyRef { id, kind, cell: i as u8, activated: false, valid: true,
                        registered: registered_kind(Some(bank.alias), i) }),
                    // A constructed body released its own kind's keys at construction
                    // end (§1.2); a member that never constructed still holds them.
                    None => detached.held_kinds.push(kind),
                }
            }
            for i in 0..NATIVE_MEMBERS {
                if let Some(context) = fields::child(entry as usize, 0x4188 + i * 8) {
                    detached.contexts.push(graph::context_ref(context, valid_context(base, context, owner)));
                }
            }
            // Dispatcher arrays: a failed build leaves the arrays `654dc0` created (or
            // the nulls if it never ran); they go after the bodies, as finalize does,
            // before the previous window's arrays come back.
            current = dispatcher_arrays(entry as usize).unwrap_or([0, 0]);
            detached.dispatch = Some(DispatcherRef { arrays: current, valid: current.iter().all(|a| *a == 0 || dispatcher_array_ok(*a)) });
            // A native group's failed build may have created its object records;
            // they go with the bodies (finalize's order), and the object kind's
            // prefetch key is released with the unbuilt members' keys.
            let created = [0usize, 8, 16].map(|o| read::<usize>(entry, 0x5a88 + o));
            if members.carries_records() && valid_special_object(base, created[0], owner) {
                records = created;
                detached.records = Some(RecordsRef { records: created, valid: true });
            }
            if let Some(kind) = members.object_kind() { detached.held_kinds.push(kind); }
        }
        (Bundle::Detached(detached), bank.alias, records, current)
    };
    let consumers = graph::probe_consumers(base, owner);
    let plan = destroy_plan_many(std::slice::from_ref(&bundle), &consumers);
    // The ledger (WP4): a constructed body released its own kind's keys at construction end
    // (§1.2), so its token is settled; a member that never constructed still holds its prefetch
    // keys, and the object kind never has a tail. The plan's key step returns those tokens through
    // the ledger now (`r_handle::release`: the Aegis orphan key first, D-SG1; DEFECT #2's per-key
    // release on every teardown reason), or the map never empties and the next Reload is refused at
    // state 7. A Mii group's identity is still in `R` here: the bodies are unloaded and these keys
    // are released under it; the window's words return below.
    let alias_ok = alias != 0 && resource(base, owner) == Some(alias);
    let built: Vec<u32> = bundle.bodies().iter().map(|body| body.kind).collect();
    if alias_ok && !built.is_empty() { r_handle::construction_settled(ledger_role(role), group, &built); }
    let releasable = alias_ok && fields::read::<u64>(alias, 0x18).unwrap_or(0) != 0;
    let mut release = |kinds: &[u32]| {
        if !releasable { return (0, kinds.len()); }
        release_group_keys(base, role, Some(group), Some(kinds), &Occupancy::default(), owner, "build rollback")
    };
    let receipt = graph::execute(base, &plan, owner, "swap rollback", &mut release);
    let map_after = if releasable { fields::read::<u64>(alias, 0x18).map_or(-1i64, |m| m as i64) } else { -1 };
    // A failed warm swap leaks the unverified group's arrays with its bodies (logged).
    let dispatch_note = match prepared {
        Prepared::Build { .. } => format!("dispatch_destroyed={} arrays={:x?}", receipt.dispatchers == 1, current),
        Prepared::Warm { dispatch, .. } => format!("dispatch_leaked={:x?}", dispatch),
    };
    let (failed, restored) = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut() else { return; };
        let target = std::mem::replace(&mut bank.members, Box::new(Members::new(&[0, 1]).unwrap()));
        let target_identity = bank.identity;
        let record = &mut bank.groups[group];
        let Some(x) = record.saved.take() else { return; };
        bank.members = std::mem::replace(&mut record.members, target);
        let failed = *record.members;
        bank.bodies = std::mem::replace(&mut record.bodies, [Body::default(); NATIVE_MEMBERS]);
        record.state = GroupState::Failed;
        // The identity goes back with the failed group; `R`, the entry bytes and the
        // D mirror get the previous window's values.
        bank.identity = std::mem::replace(&mut record.identity, target_identity);
        restore_window(entry as usize, &x);
        if let Some((words, saved)) = x.identity {
            write_resource_words(alias, &words);
            write_entry_identity(entry as usize, &saved);
        }
        // A pair window never activates Nana alone: fall back to the pair (N2).
        let via = bank.members.fallback_index(via);
        set_active_index(entry as usize, via);
        // The restored window has the cell the commit named (`bank.from` < its count).
        set_previous_index(entry as usize, bank.from);
        bank.target = via;
        bank.swap_started = None;
        GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
            .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
        trace!("[GroupSwap] fallback owner={} group={} reason=verification failed; destroyed={} contexts={} routes={} records={:x?} keys_released={:x?} map_after={} skipped={} {}; window restored, member {} activates",
            owner, group, receipt.bodies, receipt.contexts, receipt.routes_cleared, records, plan.keys(), map_after, receipt.skipped, dispatch_note, via);
        (failed, *bank.members)
    };
    // U18: whatever the failed group still registers leaves (its destroyed bodies' unloads already
    // emptied their cells) and the restored window registers again, under its own header and the
    // roster of its restored descriptor words.
    if let Some(mut registry) = NativeRegistry::new(base, alias, owner) {
        copy_registry::restore(&mut registry, failed, restored);
        registry.trace("restore", failed, restored);
    }
    // PT-TRAINER-PARK: the restored window's pool was retired at the detach; it is rebuilt before activation.
    articles::rebuild_current(base, owner, entry as usize, current_holder_valid(base, entry as usize, owner), "rollback restore");
    select_control(base, entry);
    // The ordinary acknowledgement runs on the next pre-dispatch once the
    // restored window passes its parked-body and context proofs again.
}

// ----- prefetch lifecycle (bank Ready, no change pending) -----

unsafe fn group_lifecycle(entry: *mut u8, owner: u32, role: usize) {
    let base = BASE.load(Ordering::Acquire);
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    let now = now_us();
    // K15: hold the Kirby copy packages every set this bank may register needs, from the moment a
    // group holds keys or bodies, so they are resident before any registration; release them when
    // no group does or the bank retired (`swap_native::copy_packages`). Outside a swap only.
    let copy_sets = BANKS[role].lock().unwrap().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.swap.is_none())
        .map(|bank| (if bank.phase == Phase::Ready && !bank.departed { copy_package_sets(bank) } else { Vec::new() }, bank.alias));
    if let Some((sets, alias)) = copy_sets {
        if sets.is_empty() || (alias != 0 && resource(base, owner) == Some(alias)) {
            crate::runtime::roster::copy_packages::sync(base, role, owner, alias, &sets, "tick");
        }
    }
    // D-KIRBY1: no bank prefetch key rests on this R while its live member is
    // Kirby (a copy of a group kind would share R's G68 mirror with the key, and
    // the key's later `0x17e8610` release corrupts the registry the Reload walks;
    // RCA §3). A live Kirby defers Planned groups and hands Prefetched groups'
    // keys back; Built groups consumed their keys at construction.
    enum Step { Poll(usize, super::InstallToken, u64, Members, usize), Start(usize, Members, usize),
        Release(usize, Members, Members, usize), Idle }
    let step = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.phase == Phase::Ready
            && !bank.departed && bank.swap.is_none()) else { return; };
        let live = bank.members.ready_live(snapshot(entry)).map(|(_, kind)| kind);
        if let Some((group, token, since)) = bank.prefetch {
            Step::Poll(group, token, since, *bank.groups[group].members, bank.alias)
        } else if let Err(reason) = crate::engine::swap::body_bank::prefetch_admitted(live) {
            let (window, alias) = (*bank.members, bank.alias);
            match bank.groups.iter().position(|g| g.state == GroupState::Prefetched) {
                Some(group) => Step::Release(group, *bank.groups[group].members, window, alias),
                None => {
                    for (group, g) in bank.groups.iter_mut().enumerate()
                        .filter(|(_, g)| g.state == GroupState::Planned && !g.kirby_logged) {
                        g.kirby_logged = true;
                        trace!("[GroupSwap] prefetch deferred owner={} group={} kinds={:x?} reason={} (D-KIRBY1)",
                            owner, group, &g.members.kinds()[..g.members.count()], reason);
                    }
                    Step::Idle
                }
            }
        // A Mii group's key request reads its CharInfo from the owner's record:
        // it starts only after the adapter published it (`pending_mii_publication`).
        } else if let Some(group) = bank.groups.iter().position(|g| g.state == GroupState::Planned
            && (g.identity.is_none() || g.charinfo_published)) {
            Step::Start(group, *bank.groups[group].members, bank.alias)
        } else { Step::Idle }
    };
    match step {
        Step::Idle => {}
        Step::Release(group, members, window, alias) => {
            // Same preconditions as a prefetch: this R, idle. A freshly live Kirby
            // is hatless (Reloaded, or parked copy-free by `kirby_park_gate`); a
            // copy of a group kind seen here keeps every key and reports once.
            let Some(r) = resource(base, owner).filter(|r| *r == alias && alias != 0) else { return; };
            if !resource_ready(r) { return; }
            let copy = live_kirby_copy_kind(entry, owner, window);
            let (kinds, count) = members.prefetch_kinds();
            if copy.is_some_and(|kind| kinds[..count].contains(&kind)) {
                let mut state = BANKS[role].lock().unwrap();
                if let Some(g) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)) {
                    if !g.kirby_logged {
                        g.kirby_logged = true;
                        trace!("[GroupSwap] keys kept owner={} group={} kinds={:x?} copy={:#x} reason=live Kirby copies a group kind; D-KIRBY1 invariant violated",
                            owner, group, &kinds[..count], copy.unwrap_or(0));
                    }
                }
                return;
            }
            let occupancy = occupancy_snapshot(role, copy);
            let (released, kept) = release_group_keys_of(base, role, r, group, &occupancy, owner, "live Kirby");
            let mut state = BANKS[role].lock().unwrap();
            let Some(bank) = state.as_mut() else { return; };
            let Some(g) = bank.groups.get_mut(group) else { return; };
            g.state = GroupState::Planned;
            g.busy_logged = false;
            g.kirby_logged = false;
            g.memory_logged = false;
            GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
                .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
            trace!("[GroupSwap] keys returned owner={} group={} kinds={:x?} released={} kept={} reason=live Kirby (D-KIRBY1); replanned map={}",
                owner, group, &kinds[..count], released, kept, fields::read::<u64>(r, 0x18).unwrap_or(0));
        }
        Step::Poll(group, token, since, members, alias) => {
            let r = resource(base, owner).filter(|r| *r == alias);
            let ready = r.is_some_and(|r| resource_ready(r));
            let outcome = prefetch_outcome(ready, now.saturating_sub(since), PREFETCH_TIMEOUT_US);
            if outcome == GroupState::Prefetching { return; }
            if outcome == GroupState::Failed {
                if let Some(r) = r {
                    // D-KIRBY1: a copy acquired during the prefetch keeps its kind's key (the ledger's
                    // live-consumer check decides which tokens go). A Mii group's keys are released
                    // with its identity in `R` (the release selector `0x17e8410` keys on `R+0x58`
                    // like the request did).
                    let copy = ready_members(entry, owner).and_then(|window| live_kirby_copy_kind(entry, owner, window));
                    let occupancy = occupancy_snapshot(role, copy);
                    release_group_keys_of(base, role, r, group, &occupancy, owner, "prefetch failed");
                }
            }
            let mut state = BANKS[role].lock().unwrap();
            let Some(bank) = state.as_mut() else { return; };
            bank.groups[group].state = outcome;
            bank.prefetch = None;
            let released = super::finish_install(token, "group prefetch settled");
            GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
                .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
            let started = bank.groups[group].prefetch_committed.take();
            match outcome {
                GroupState::Prefetched => trace!("[GroupSwap] prefetched owner={} group={} kinds={:x?} us={} INSTALL_released={} map={} tree={}",
                    owner, group, &members.kinds()[..members.count()], now.saturating_sub(since), released,
                    fields::read::<u64>(alias, 0x18).unwrap_or(0), fields::read::<u64>(alias, 0x4c0).unwrap_or(0)),
                _ => trace!("[GroupSwap] prefetch failed owner={} group={} kinds={:x?} us={} alias_valid={} keys_released={} INSTALL_released={}",
                    owner, group, &members.kinds()[..members.count()], now.saturating_sub(since), r.is_some(), r.is_some(), released),
            }
            // The measured charge of this prefetch next to the estimate it was admitted on (the table's input,
            // `tools/group_memory_costs.py`). Prefetches are serialized by INSTALL.
            if outcome == GroupState::Prefetched {
                let kinds = &members.kinds()[..members.count()];
                log_memory(owner, group, kinds, "prefetch", started, MemorySample::capture(), memory::group_cost(kinds).prefetch);
            }
        }
        Step::Start(group, members, alias) => {
            if !groups_enabled(&settings) || super::INSTALL.active() { return; }
            if crate::runtime::roster::cycle_preload::speculative_admission().is_none() { return; }
            let Some(r) = resource(base, owner).filter(|r| *r == alias && alias != 0) else { return; };
            let (capacity, live) = pool_counts(base).unwrap_or((0, 0));
            let pool_free = (capacity as usize).saturating_sub(usize::from(live)).saturating_sub(POOL_RESERVE);
            if pool_free < members.count() {
                fail_group(role, group, "fighter pool reserve");
                trace!("[GroupSwap] fallback owner={} group={} reason=fighter pool {}/{} reserve {} < {} bodies",
                    owner, group, live, capacity, POOL_RESERVE, members.count());
                return;
            }
            // GS-PANIC: the group's measured cost (prefetch + build) against the headroom every other consumer has
            // not spent yet: both roles' admitted groups and every helper load in flight (`process_memory`). The
            // build happens seconds later at the request; until then the prefetched group's build stays held, so a
            // later group or a helper preload cannot be admitted on it (s21 L7804, s19 L9084, s30 L8292).
            let cost = memory::group_cost(&members.kinds()[..members.count()]);
            let (decision, sample) = super::process_memory::admit(cost);
            let headroom = decision.headroom;
            match decision.admission {
                Admission::Admit => {}
                Admission::Defer => {
                    let mut state = BANKS[role].lock().unwrap();
                    if let Some(g) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)) {
                        if !g.memory_logged {
                            g.memory_logged = true;
                            trace!("[GroupSwap] prefetch deferred owner={} group={} kinds={:x?} reason=process memory held by admitted groups headroom={:?} outstanding={} need={} prefetch={} build={}",
                                owner, group, &members.kinds()[..members.count()], headroom, decision.outstanding, decision.need,
                                cost.prefetch, cost.build);
                        }
                    }
                    return;
                }
                Admission::Refuse => {
                    trace!("[GroupSwap] fallback owner={} group={} reason=process headroom {:?} < {} bytes kinds={:x?} outstanding={} prefetch={} build={}",
                        owner, group, headroom, decision.need, &members.kinds()[..members.count()], decision.outstanding,
                        cost.prefetch, cost.build);
                    fail_group(role, group, "process headroom");
                    return;
                }
            }
            // Native's own readiness (`0x17e4750`): the current task at `R+0xa8` is
            // idle at `task+0x10 == 0`; its pointer may change across Reload (C4).
            // Only a running task
            // or an unloaded handle defers the prefetch.
            if !resource_ready(r) {
                let mut state = BANKS[role].lock().unwrap();
                if let Some(g) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)) {
                    if !g.busy_logged {
                        g.busy_logged = true;
                        trace!("[GroupSwap] prefetch deferred owner={} group={} reason=resource owner not ready task_pending={} map={} tree={}",
                            owner, group, task_pending(r), fields::read::<u64>(r, 0x18).unwrap_or(0),
                            fields::read::<u64>(r, 0x4c0).unwrap_or(0));
                    }
                }
                return;
            }
            // WP-E row `native_body_bank::group_prefetch` (U14): a prefetch starts acquiring native map keys;
            // outside Training and waiting rooms it does not start and the group stays Planned, as for
            // every deferral above (nothing acquired yet, no INSTALL taken).
            if !crate::runtime::roster::scope_gate::admitted("native_body_bank::group_prefetch") { return; }
            let Some(token) = super::INSTALL.try_begin(false) else { return; };
            // Every acquisition goes through the ledger (WP4): a token per kind, the pair rule or
            // the construction tail as its recipe, the object kind owned; a refusal (a live consumer,
            // a kind already held on this R) stops the prefetch before any further native request
            // and returns what this group already acquired.
            let lrole = ledger_role(role);
            r_handle::bind(lrole, owner, r, crate::runtime::roster::observe::owner_flags(r));
            let copy = ready_members(entry, owner).and_then(|window| live_kirby_copy_kind(entry, owner, window));
            let occupancy = occupancy_snapshot(role, copy);
            let (kinds, count) = members.prefetch_kinds();
            // A Mii group's keys (`kind*42+n`, hat `+3` from `R+0x70`, body params
            // `R+0x6c/0x74`, costume 0 by the `R+0x58` selector) are requested with
            // its identity in `R`; the window's words come back right after. A refusal
            // returns what this group already acquired under the same identity.
            let identity = group_identity(role, group);
            let mut refused = None;
            let mut returned = (0usize, 0usize);
            with_mii_identity(r, identity, || {
                for kind in &kinds[..count] {
                    if let Err(why) = r_handle::acquire(base, lrole, group, *kind, Holder::Prefetch(GroupId(group as u8)),
                        member_recipe(members, *kind), &occupancy) {
                        refused = Some((*kind, why));
                        break;
                    }
                }
                if refused.is_some() {
                    returned = release_group_keys(base, role, Some(group), None, &occupancy, owner, "prefetch refused");
                }
            });
            if let Some((kind, why)) = refused {
                let (released, kept) = returned;
                trace!("[GroupSwap] prefetch refused owner={} group={} kind={:#x} reason={:?} released={} kept={}; group stays Planned",
                    owner, group, kind, why, released, kept);
                super::finish_install(token, "group prefetch refused by the ledger");
                return;
            }
            let mut state = BANKS[role].lock().unwrap();
            let Some(bank) = state.as_mut() else { super::finish_install(token, "group prefetch bank vanished"); return; };
            bank.groups[group].state = GroupState::Prefetching;
            bank.groups[group].since_us = now;
            bank.groups[group].prefetch_committed = sample.committed.get();
            bank.prefetch = Some((group, token, now));
            trace!("[GroupSwap] prefetch owner={} group={} kinds={:x?} R={:#x} map={} flags={:#x} task_pending={} pool={}/{} headroom={} INSTALL(resource) outstanding={} need={}",
                owner, group, &members.kinds()[..members.count()], r, fields::read::<u64>(r, 0x18).unwrap_or(0),
                fields::read::<u8>(r, 0x4c).unwrap_or(0), task_pending(r), live, capacity, headroom.unwrap_or(0),
                decision.outstanding, decision.need);
        }
    }
}

/// The groups' part of `process_memory::outstanding`: what the admitted groups of both roles have not spent yet
/// (`body_bank::memory::outstanding`): a prefetch in flight its whole cost, a prefetched group its build. A built
/// group's bodies are in the headroom. Takes each role's bank lock in turn; never call it with a bank lock held.
pub(super) fn group_outstanding() -> u64 {
    BANKS.iter().map(|state| state.lock().unwrap().as_ref().map_or(0, |bank| bank.groups.iter()
        .map(|g| memory::outstanding(g.state, memory::group_cost(&g.members.kinds()[..g.members.count()])))
        .fold(0u64, u64::saturating_add)))
        .fold(0u64, u64::saturating_add)
}

/// `[GroupSwap] memory`: the committed process charge one phase of a group took, next to the estimate the
/// admission used (`tools/group_memory_costs.py` reads these lines as exact observations).
fn log_memory(owner: u32, group: usize, kinds: &[u32], phase: &str, before: Option<u64>, after: MemorySample, estimate: u64) {
    // LOG-REFUSED: the per-load measurement is an Eden data line, diag builds only (the `[GroupSwap]` tag ships).
    if !cfg!(feature = "diag") { return; }
    let committed = after.committed.get();
    let delta = before.zip(committed).map(|(b, a)| i128::from(a) - i128::from(b));
    trace!("[GroupSwap] memory owner={} group={} kinds={:x?} phase={} committed={:?}->{:?} delta={} estimate={} headroom={:?}",
        owner, group, kinds, phase, before, committed, delta.map_or_else(|| "na".to_string(), |d| d.to_string()), estimate,
        after.headroom());
}

fn fail_group(role: usize, group: usize, _reason: &str) {
    let mut state = BANKS[role].lock().unwrap();
    if let Some(bank) = state.as_mut() {
        if let Some(g) = bank.groups.get_mut(group) { g.state = GroupState::Failed; }
        GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
            .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
    }
}

// ----- teardown before every native destroyer -----

/// Destroy every detached group of this entry and release prefetched keys through
/// the teardown protocol (`swap_model::lifecycle::destroy_plan_many`, executed by
/// `swap_native::body_graph`), in the native-equivalent order (§4.1): the pad
/// routes naming a context's controller first (D-PTA1c), then per body
/// `0x3aeb50(id)` (unload through vtable `+0x50`, then the pool return), each
/// saved context through vtable `+0x8`, the dispatcher arrays, the object
/// records, then the ledger's release of every token still held (`r_handle::release`:
/// the Aegis `0x1338` key, then `0x17e8610(R,k)`) for kinds only registered, refused
/// for the kind a live Kirby copies (D-KIRBY1). Must run in an entry callback strictly
/// before the destroyer. Objects that fail validation are skipped and reported; they
/// leak rather than corrupt.
unsafe fn retire_groups(entry: *const u8, owner: u32, reason: &str) {
    let Some(role) = role(owner) else { return; };
    let base = BASE.load(Ordering::Acquire);
    // Phase B′: a construction in flight settles into the window first, so native's destroyer finds its bodies.
    drain_construction(entry, owner, role, reason);
    // D-KIRBY1: the kind a live Kirby copies keeps its key (read before the lock).
    let window = BANKS[role].lock().unwrap().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.owner == owner).map(|bank| *bank.members);
    let copy = window.and_then(|members| live_kirby_copy_kind(entry, owner, members));
    // U18: each detached body's group, for the per-group header during the destroys.
    let mut owners: Vec<(u32, Members)> = Vec::new();
    let (mut bundles, identities, token, alias, groups) = {
        let mut state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        if bank.groups.is_empty() && bank.prefetch.is_none() { return; }
        let token = bank.prefetch.take().map(|p| p.1);
        let (alias, groups, activated) = (bank.alias, bank.groups.len(), bank.activated.clone());
        let mut bundles = Vec::with_capacity(groups);
        // A Mii group's keys are released with its identity in `R` (the release selector
        // `0x17e8410` keys on `R+0x58` like the request did): each key-holding Mii group keeps
        // its index (the ledger's group of its tokens) and identity.
        let mut identities: Vec<(usize, MiiIdentity)> = Vec::new();
        for (index, group) in bank.groups.drain(..).enumerate() {
            let mut detached = DetachedGroup { owner, ..Default::default() };
            match (group.state, group.saved) {
                (GroupState::Built, Some(saved)) => {
                    for (i, kind) in group.members.kinds().iter().enumerate().take(group.members.count()) {
                        let body = group.bodies[i];
                        if saved.cell_ints[i] != 0 {
                            trace!("[GroupSwap] teardown note owner={} nonzero per-cell int {:#x} for kind {:#x} (655080 release not applied)", owner, saved.cell_ints[i], kind);
                        }
                        // U18: a group is built under its own header (`copy_registry::build`), so its
                        // init registered each body under the group's own kind; the window's header at
                        // teardown time names another group's bodies.
                        detached.bodies.push(BodyRef { id: body.id, kind: *kind, cell: i as u8, activated: activated.contains(&body.id),
                            valid: valid_parked_body(base, body, *kind, owner), registered: Some(*kind) });
                        owners.push((body.id, *group.members));
                    }
                    for context in saved.group_ctx.iter().copied().filter(|c| *c != 0) {
                        detached.contexts.push(graph::context_ref(context, valid_context(base, context, owner)));
                    }
                    detached.dispatch = Some(DispatcherRef { arrays: saved.dispatch,
                        valid: saved.dispatch.iter().all(|a| *a == 0 || dispatcher_array_ok(*a)) });
                    if saved.records != [0, 0, 0] {
                        detached.records = Some(RecordsRef { records: saved.records,
                            valid: group.members.carries_records() && valid_special_object(base, saved.records[0], owner) });
                    }
                }
                (state, _) if state.holds_keys() => {
                    let (kinds, count) = group.members.prefetch_kinds();
                    detached.held_kinds.extend(kinds[..count].iter().copied());
                    if let Some(identity) = group.identity { identities.push((index, identity)); }
                }
                _ => {}
            }
            bundles.push(Bundle::Detached(detached));
        }
        bank.swap = None;
        bank.swap_started = None;
        GROUP_BODIES[role].store(0, Ordering::Release);
        (bundles, identities, token, alias, groups)
    };
    // Without the alias R (`0x3aeb50` and every key release read it) bodies and keys
    // are not touched: they are reported as skipped, as before.
    let alias_ok = alias != 0 && resource(base, owner) == Some(alias);
    let mut unreachable = 0usize;
    if !alias_ok {
        for bundle in &mut bundles {
            if let Bundle::Detached(group) = bundle {
                unreachable += group.bodies.len() + group.held_kinds.len();
                group.bodies.clear();
                group.held_kinds.clear();
            }
        }
    }
    // U18: detached bodies hold no registration but unload over the cells the window's registrations
    // occupy: the window leaves the registry, each group is destroyed under its own header, then the
    // window registers again (`copy_registry::teardown_begin/group/end`). Without the registry the
    // bodies are left (this function's rule: leak rather than corrupt).
    let has_bodies = bundles.iter().any(|b| matches!(b, Bundle::Detached(group) if !group.bodies.is_empty()));
    let mut registry = if has_bodies { NativeRegistry::new(base, alias, owner).zip(window) } else { None };
    if has_bodies && registry.is_none() {
        for bundle in &mut bundles {
            if let Bundle::Detached(group) = bundle { unreachable += group.bodies.len(); group.bodies.clear(); }
        }
        trace!("[Registry] teardown owner={} reason={} registry unreadable; detached bodies left", owner, reason);
    }
    let cells = registry.as_mut().map(|(registry, window)| {
        let before = registry.cells();
        copy_registry::teardown_begin(registry, *window);
        (before, registry.cells())
    });
    let consumers = graph::probe_consumers(base, owner);
    let plan = destroy_plan_many(&bundles, &consumers);
    // The plan's key step (DEFECT #2's per-key release on every teardown reason, row 15) hands
    // every token the ledger still holds for this role — these groups' unconsumed prefetches and a
    // window whose build never settled — back through `r_handle::release` (WP4: the orphan `0x1338`
    // by its own key first, then `0x17e8610(R, k)`; D-SG1). The live-consumer check decides which
    // go (D-KIRBY1: a refused token is retained and reported, never destroyed). A Mii group's
    // tokens go with its identity in `R`, every other token in one pass.
    let occupancy = occupancy_snapshot(role, copy);
    let mut release = |_: &[u32]| {
        let mut outcome = (0usize, 0usize);
        for (group, identity) in &identities {
            let mut one = (0usize, 0usize);
            with_mii_identity(alias, Some(*identity), || {
                one = release_group_keys(base, role, Some(*group), None, &occupancy, owner, reason);
            });
            outcome.0 += one.0;
            outcome.1 += one.1;
        }
        let rest = release_group_keys(base, role, None, None, &occupancy, owner, reason);
        (outcome.0 + rest.0, outcome.1 + rest.1)
    };
    // U18: each group's own header right before its first body, the window's before any later step.
    let mut current: Option<Members> = None;
    let receipt = {
        let mut before_step = |step: &DestroyStep| {
            let Some((registry, window)) = registry.as_mut() else { return; };
            let wanted = match step {
                DestroyStep::DestroyBody { id } => owners.iter().find(|(body, _)| body == id).map(|(_, group)| *group),
                _ => Some(*window),
            };
            if let Some(members) = wanted.filter(|m| current != Some(*m)) {
                copy_registry::teardown_group(registry, members);
                current = Some(members);
            }
        };
        graph::execute_with(base, &plan, owner, reason, &mut release, &mut before_step)
    };
    if let (Some((registry, window)), Some((before, emptied))) = (registry.as_mut(), cells) {
        copy_registry::teardown_end(registry, *window);
        let mut detached: Vec<Vec<u32>> = Vec::new();
        for (_, group) in &owners {
            let kinds = group.kinds()[..group.count()].to_vec();
            if !detached.contains(&kinds) { detached.push(kinds); }
        }
        trace!("[Registry] teardown owner={} reason={} window={:x?} detached={:x?} cells={:x?}->{:x?}->{:x?} header={:x?} r4c={:#x} task_pending={}",
            owner, reason, &window.kinds()[..window.count()], detached, before, emptied, registry.cells(), registry.header(),
            fields::read::<u32>(alias, 0x4c).unwrap_or(0), task_pending(alias));
    }
    if let Some(token) = token { super::finish_install(token, "group prefetch retired"); }
    let pool = pool_counts(base);
    // `map_after` = R's remaining key count (`R+0x18`) after the release: the last-line
    // evidence that no key stayed registered (0 expected whenever `alias_ok` and `keys_kept=0`).
    let map_after = if alias_ok { fields::read::<u64>(alias, 0x18).map_or(-1i64, |m| m as i64) } else { -1 };
    trace!("[GroupSwap] teardown owner={} reason={} groups={} bodies={} contexts={} routes={} dispatchers={} records={} keys={} keys_kept={} map_after={} skipped={} alias_ok={} pool={:?}",
        owner, reason, groups, receipt.bodies, receipt.contexts, receipt.routes_cleared, receipt.dispatchers, receipt.records,
        receipt.keys_released, receipt.keys_kept, map_after, receipt.skipped + unreachable, alias_ok, pool);
    // Every record is gone: the role's real ledger against the owner's map (WP2's `teardown end`
    // line; U13: `r_handle::reconcile` supersedes the mirror-based `observe::reconcile`).
    if alias_ok { r_handle::reconcile(ledger_role(role), reason); }
}

/// Every mod-initiated Reload of a visible entry calls this first (§6.5 iii):
/// pair flags, then the attached window's routes (D-PTA1c: states 1-3 free the
/// window contexts and only activation re-points the route, which a refused
/// prequeue never reaches), then every detached group — groups on or off.
pub(super) unsafe fn before_reload(entry: *const u8) {
    if !aligned(entry) { return; }
    let owner = read::<u32>(entry, 0xe0);
    if PAIR_ENABLED.load(Ordering::Acquire) { pair_teardown(entry, "mod reload"); }
    retire_window(entry, owner, "mod reload", true);
    if !GROUPS_ENABLED.load(Ordering::Acquire) { return; }
    retire_groups(entry, owner, "mod reload");
    // WP4: state 6 of the Reload needs `R+0x18 == 0` (`0x17e4700`); after the teardown the ledger
    // must hold nothing. A key left here names the quirk the recipes miss (diagnostic, no cleanup;
    // it replaces WP2's `before reload` observation, U13).
    if let Some(role) = role(owner) {
        if let Err(keys) = r_handle::expect_free(ledger_role(role)) {
            trace!("[Ledger] expect_free owner={} phase=mod reload keys_held={:x?}", owner, keys);
        }
    }
}

/// Teardown assertion for a pair-lead window (Ready or Changing): `651660` frees
/// `+0x4188/90/98` only on its bit-2 path, so bit 2 must be set whenever a
/// destroyer can run. N2 clears it solely inside the `656ee0` call, which cannot
/// overlap a destroyer. A clear bit here is an invariant violation: it is logged
/// with its full context and **not repaired** (operator decision #6,
/// WORKAROUND-DECISIONS §C row 18 — the logged check stays, the silent write goes).
unsafe fn pair_teardown(entry: *const u8, reason: &str) {
    let Some(owner) = pair_lead_owner(entry) else { return; };
    {
        let state = BANKS[role(owner).unwrap_or(0)].lock().unwrap();
        if !state.as_ref().is_some_and(|bank| matches!(bank.phase, Phase::Ready | Phase::Changing)) { return; }
    }
    let flags = read::<u8>(entry, 0x591f);
    let (contexts, aliases) = ([read::<usize>(entry, 0x4188), read::<usize>(entry, 0x4190), read::<usize>(entry, 0x4198)],
        [read::<usize>(entry, 0x4178), read::<usize>(entry, 0x4180)]);
    if flags & BANK_FLAG == 0 {
        trace!("[IcPair] teardown owner={} reason={} flags={:#x} INVARIANT VIOLATED: bit 2 clear while a destroyer can run (651660 frees +0x4188/90/98 only on its bit-2 path); not repaired (operator decision #6) index={} loader={:#x} outer={} contexts=[{:#x},{:#x},{:#x}] aliases=[{:#x},{:#x}] states=[{:?},{:?},{:?}]",
            owner, reason, flags, read::<u8>(entry, 0x5918), read::<u8>(entry, 0x591c), read::<u8>(entry, 0x5920),
            contexts[0], contexts[1], contexts[2], aliases[0], aliases[1],
            fields::child(entry as usize, 0x4160).and_then(|b| fields::read::<u8>(b, 0x3a)),
            fields::child(entry as usize, 0x4168).and_then(|b| fields::read::<u8>(b, 0x3a)),
            fields::child(entry as usize, 0x4170).and_then(|b| fields::read::<u8>(b, 0x3a)));
    } else {
        trace!("[IcPair] teardown owner={} reason={} flags={:#x} contexts=[{:#x},{:#x},{:#x}] aliases=[{:#x},{:#x}]",
            owner, reason, flags, contexts[0], contexts[1], contexts[2], aliases[0], aliases[1]);
    }
}

pub(super) fn scene_changed() {
    RESERVED.store(0, Ordering::Release);
    for value in &ROLE_BUDGET { value.store(0, Ordering::Release); }
    for value in &PLANNED_TARGETS { value.store(0, Ordering::Release); }
    for state in &BANKS {
        if let Some(bank) = state.lock().unwrap().as_mut() { bank.departed = true; }
    }
    for role in 0..2 { r_handle::reset(ledger_role(role), "scene changed"); }
    // The scene loop resets every holder, G68 included: the copy holdings end without native calls.
    for role in 0..2 { crate::runtime::roster::copy_packages::forget(role, role as u32, "scene changed"); }
}

pub(super) fn on_destroy(entry: *const u8) {
    for (role, state) in BANKS.iter().enumerate() {
        if let Some(bank) = state.lock().unwrap().as_mut() {
            if bank.entry == entry as usize {
                bank.phase = Phase::Retired; bank.departed = true;
                bank.exchange = None;
                r_handle::reset(ledger_role(role), "entry destroyed");
                crate::runtime::roster::copy_packages::forget(role, bank.owner, "entry destroyed");
                EXTRAS[role].store(0, Ordering::Release);
                GROUP_BODIES[role].store(0, Ordering::Release);
                if !bank.groups.is_empty() || bank.prefetch.is_some() {
                    // Finalize ran before this destructor hook; the groups should be gone.
                    trace!("[GroupSwap] destroy owner={} with {} group records remaining (leak reported)", bank.owner, bank.groups.len());
                    bank.groups.clear();
                    bank.prefetch = None;
                }
                // Do not free the table before native teardown has returned.
                // A subsequent build requires a ready current native entry.
            }
        }
    }
}

#[skyline::hook(offset = 0x654dc0)]
unsafe fn prepare_models(entry: *mut u8) {
    original!()(entry);
    if !aligned(entry) { return; }
    let Some(role) = role(read(entry, 0xe0)) else { return; };
    let state = BANKS[role].lock().unwrap();
    if let Some(bank) = state.as_ref().filter(|bank| descriptor(entry, bank)) {
        // A pair-lead window (or pair group) is always constructed with the pair
        // active (cell 0): bit 1 as vanilla `654dc0` set it for two actors, plus
        // the bank bit. A native group keeps the flags `654dc0` wrote (`0x05`/`0x64`).
        let flags = bank.members.expected_flags(0);
        let native = read::<u8>(entry, 0x591f);
        fields::write_unchecked(entry as usize, 0x591f, flags);
        if bank.members.is_pair() {
            trace!("[IcPair] construct owner={} entry={:p} flags={:#x} native_flags_before={:#x}", bank.owner, entry, flags, native);
        }
    }
}

/// N1: Nana's group context receives Popo's context as `prev` (`x6`), exactly the
/// argument vanilla `655320` passes at `0x655504-0x655510`. The hook sits on the
/// `mov w2,w20` at `0x6555a0`; `x6` was zeroed at `0x655598` and nothing else
/// writes it before the `bl 0x6b6f10` at `0x6555a4`. `x19` is the entry, `w20`
/// the table[1] kind and `+0x4188` was stored at `0x655514`.
#[skyline::hook(offset = 0x6555a0, inline)]
unsafe fn pair_partner(ctx: &mut InlineCtx) {
    let entry = ctx.registers[19].x() as *const u8;
    let Some(owner) = pair_lead_owner(entry) else { return; };
    let popo = read::<usize>(entry, 0x4188);
    let kind = ctx.registers[20].x() as u32;
    if kind != NANA || popo == 0 {
        trace!("[IcPair] N1 skipped owner={} table_kind={:#x} popo_ctx={:#x}", owner, kind, popo);
        return;
    }
    ctx.registers[6].set_x(popo as u64);
    trace!("[IcPair] N1 owner={} entry={:p} nana_prev=popo_ctx {:#x}", owner, entry, popo);
}

/// N2: entry activation (`656ee0`: first activation, rebirth, return from cell 2).
/// Pair active: alias `+0x4180` to Nana's group context and run the vanilla pair
/// branch (flags `2`), which registers Nana on `+0x4180` because bit 2 is clear
/// (`activate_00656ee0.c:65`); flags return to `6`. Cell 2 active: flags `4`.
#[skyline::hook(offset = 0x656ee0)]
unsafe fn activate_entry(entry: *mut u8, status: i32) {
    let Some(owner) = pair_lead_owner(entry) else { original!()(entry, status); return; };
    let index = read::<u8>(entry, 0x5918);
    let before = read::<u8>(entry, 0x591f);
    if index == 0 {
        let nana = read::<usize>(entry, 0x4190);
        fields::write_unchecked(entry as usize, 0x4180, nana);
        fields::write_unchecked(entry as usize, 0x591f, PAIR_ACTIVE_FLAG & !BANK_FLAG);
        original!()(entry, status);
        fields::write_unchecked(entry as usize, 0x591f, PAIR_ACTIVE_FLAG);
        trace!("[IcPair] N2 activate pair owner={} status={:#x} flags {:#x}->2->6 popo_ctx={:#x} nana_ctx={:#x} +0x4178={:#x} +0x4180={:#x} states=[{:?},{:?}]",
            owner, status, before, read::<usize>(entry, 0x4188), nana, read::<usize>(entry, 0x4178),
            read::<usize>(entry, 0x4180),
            fields::child(entry as usize, 0x4160).and_then(|b| fields::read::<u8>(b, 0x3a)),
            fields::child(entry as usize, 0x4168).and_then(|b| fields::read::<u8>(b, 0x3a)));
        return;
    }
    if before != BANK_FLAG {
        // Bit 1 with cell 2 active would wake the parked pair. The flags are an invariant of
        // the protocol (N3 parks with `4`); a different value here is logged with its context
        // and NOT corrected (operator decision #6, WORKAROUND-DECISIONS §C row 18).
        trace!("[IcPair] N2 flags owner={} index={} flags={:#x} expected={:#x} INVARIANT VIOLATED: bit 1 with cell 2 active would wake the parked pair; not repaired (operator decision #6) status={:#x} loader={:#x} outer={} +0x4178={:#x} +0x4180={:#x} contexts=[{:#x},{:#x},{:#x}] states=[{:?},{:?},{:?}]",
            owner, index, before, BANK_FLAG, status, read::<u8>(entry, 0x591c), read::<u8>(entry, 0x5920),
            read::<usize>(entry, 0x4178), read::<usize>(entry, 0x4180),
            read::<usize>(entry, 0x4188), read::<usize>(entry, 0x4190), read::<usize>(entry, 0x4198),
            fields::child(entry as usize, 0x4160).and_then(|b| fields::read::<u8>(b, 0x3a)),
            fields::child(entry as usize, 0x4168).and_then(|b| fields::read::<u8>(b, 0x3a)),
            fields::child(entry as usize, 0x4170).and_then(|b| fields::read::<u8>(b, 0x3a)));
    }
    original!()(entry, status);
    trace!("[IcPair] N2 activate extra owner={} status={:#x} index={} +0x4178={:#x} +0x4180={:#x}",
        owner, status, index, read::<usize>(entry, 0x4178), read::<usize>(entry, 0x4180));
}

/// N3: after a change commit that parked the pair (old index 0, new index 2, loader
/// `0xc`), park Nana with the commit's own outgoing-body sequence, clear the
/// second alias and drop bit 1. Runs inside the same call, before any service.
#[skyline::hook(offset = 0x6581f0)]
unsafe fn commit_change(entry: *mut u8) {
    let parking = aligned(entry) && read::<u8>(entry, 0x5920) == 6 && read::<u8>(entry, 0x591a) != 0xff
        && read::<u8>(entry, 0x5918) == 0 && pair_lead_owner(entry).is_some();
    original!()(entry);
    if !parking { return; }
    let Some((owner, members)) = pair_window(entry) else { return; };
    let (outer, loader, old, index) = (read::<u8>(entry, 0x5920), read::<u8>(entry, 0x591c),
        read::<u8>(entry, 0x5910), read::<u8>(entry, 0x5918));
    // The pair-lead window parks towards its extra (2); a pair group parks through
    // Nana (1) as the transient commit index of a cross-group exit.
    let park_index = members.swap_commit_index_for(0).unwrap_or(2);
    if outer != 3 || loader != 0xc || old != 0 || index != park_index {
        trace!("[IcPair] N3 not a pair park owner={} outer={} loader={:#x} old={} index={} expected={}", owner, outer, loader, old, index, park_index);
        return;
    }
    match park_nana(entry, owner) {
        Ok((id, saved, state)) => {
            fields::write_unchecked(entry as usize, 0x4180, 0usize);
            fields::write_unchecked(entry as usize, 0x591f, BANK_FLAG);
            trace!("[IcPair] N3 park pair owner={} nana_id={:#x} nana_state 4->{} info_saved={} popo_state={:?} +0x4178={:#x} +0x4180=0 flags=4",
                owner, id, state, saved, fields::child(entry as usize, 0x4160).and_then(|b| fields::read::<u8>(b, 0x3a)),
                read::<usize>(entry, 0x4178));
        }
        Err(reason) => trace!("[IcPair] N3 refused owner={} reason={}; flags left {:#x} (no acknowledgement will follow)",
            owner, reason, read::<u8>(entry, 0x591f)),
    }
}

/// The commit's outgoing sequence on cell 1 (`change_commit_006581f0.c:60-75`):
/// `p = 646fe0(body)`; if `p->vtable[2](p)` then save `*(p[1]+0x1a4)`;
/// fighter vtable `+0x48(body, !(flags & 0x40))`; restore. `646fe0` maps a Nana
/// body of a two-kind entry to its own wrapper `entry+0x100` (U1 FACT).
unsafe fn park_nana(entry: *mut u8, owner: u32) -> Result<(u32, bool, u8), &'static str> {
    let base = BASE.load(Ordering::Acquire);
    let Some(nana) = fields::child(entry as usize, 0x4168) else { return Err("cell 1 empty"); };
    if fields::read::<usize>(nana, 0) != Some(base + FIGHTER_VTABLE) { return Err("cell 1 vtable"); }
    if fields::read::<u32>(nana, 0xc) != Some(NANA) { return Err("cell 1 kind"); }
    if fields::read::<u64>(nana, 0x10) != Some(u64::from(owner)) { return Err("cell 1 owner"); }
    if fields::read::<u8>(nana, 0x3a) != Some(4) { return Err("cell 1 not active"); }
    let id = fields::read::<u32>(nana, 8).ok_or("cell 1 id")?;
    let outgoing: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(base + OUTGOING_OWNER);
    let wrapper = outgoing(nana);
    let vtable = fields::read::<usize>(wrapper, 0).ok_or("wrapper vtable")?;
    let has_info: unsafe extern "C" fn(usize) -> u64 =
        std::mem::transmute(fields::read::<usize>(vtable, 0x10).ok_or("wrapper vtable slot")?);
    let info = |wrapper: usize| fields::read::<usize>(wrapper, 8).filter(|p| *p != 0);
    let saved = if has_info(wrapper) & 1 != 0 { info(wrapper).and_then(|p| fields::read::<u32>(p, 0x1a4)) } else { None };
    let deactivate: unsafe extern "C" fn(usize, u32) =
        std::mem::transmute(fields::read::<usize>(base + FIGHTER_VTABLE, 0x48).ok_or("fighter vtable")?);
    deactivate(nana, u32::from(read::<u8>(entry, 0x591f) & 0x40 == 0));
    if let Some(value) = saved {
        if has_info(wrapper) & 1 != 0 {
            if let Some(p) = info(wrapper) { fields::write_unchecked(p, 0x1a4, value); }
        }
    }
    Ok((id, saved.is_some(), fields::read::<u8>(nana, 0x3a).unwrap_or(0xff)))
}

unsafe fn table(ctx: &mut InlineCtx, register: usize) {
    let entry = ctx.registers[19].x() as *const u8;
    if !aligned(entry) { return; }
    let Some(role) = role(read(entry, 0xe0)) else { return; };
    let state = BANKS[role].lock().unwrap();
    if let Some(bank) = state.as_ref().filter(|bank| descriptor(entry, bank)) {
        ctx.registers[register].set_x(bank.members.table().as_ptr() as u64);
    }
}
#[skyline::hook(offset = 0x655484, inline)]
unsafe fn create_contexts(ctx: &mut InlineCtx) { table(ctx, 23); }
#[skyline::hook(offset = 0x64fbcc, inline)]
unsafe fn select_context(ctx: &mut InlineCtx) { table(ctx, 9); }

#[skyline::hook(offset = 0x17e5bc0)]
unsafe fn prepare_resources(resource: *mut u8) {
    // IC-PAIR-STALL: the key map before native registers this descriptor's requests. State 6 runs
    // this on the R `0x17e4700` picked as free (`R+0x18 == 0`), so any token the ledger still holds
    // for it is stale and is settled below, after the bind.
    let empty_before = aligned(resource) && fields::read::<u64>(resource as usize, 0x18) == Some(0);
    original!()(resource);
    if !aligned(resource) || read::<u8>(resource, 0x4c) & 7 != 1 { return; }
    let owner = read::<u32>(resource, 0x54);
    let Some(role) = role(owner) else { return; };
    // Copy values before native calls. The resource worker never borrows a
    // table pointer or holds a Rust lock while native acquisition runs.
    let members = {
        let state = BANKS[role].lock().unwrap();
        let Some(bank) = state.as_ref().filter(|bank| bank.owner == owner && bank.phase != Phase::Retired) else { return; };
        *bank.members
    };
    if !members.kinds().iter().enumerate().all(|(i, kind)| read::<u32>(resource, 0x58 + i * 4) == *kind) {
        return;
    }
    let base = BASE.load(Ordering::Acquire);
    let Some(pool) = fields::child(base, OWNER_TABLE) else { return; };
    if fields::read::<usize>(pool, 0xe8 + owner as usize * 8) != Some(resource as usize)
        || fields::read::<u8>(pool, 0x128 + owner as usize) != Some(0) { return; }
    // WP4: the ledger of this role is bound to this R (a new R starts empty; `before_reload`
    // released every group token), native's own requests for the descriptor are recorded
    // (`0x17e5bc0` model: the primary; the Popo primary's `R+0x5c`, SOLO-STALL-RCA §3; the PT/Aegis
    // object keys), and the hook acquires every remaining member once: a second construction
    // acquisition of a kind native already requested is refused structurally (`AlreadyHeld`), the
    // rule `supplemental_acquisitions` encoded by hand. The per-body `0x17e8610` releases then
    // empty the map (native reset) and the tokens settle at the window's Ready.
    let lrole = ledger_role(role);
    let flags = crate::runtime::roster::observe::owner_flags(resource as usize);
    r_handle::bind(lrole, owner, resource as usize, flags);
    // Native is the authority on R's keys: on a configure that found the map empty, nothing the
    // ledger predicts for this R exists (each settled token prints a `[Ledger] stale …` line).
    if empty_before { r_handle::settle_stale_on_empty(lrole, "configure on an empty R"); }
    r_handle::record_native(lrole, &native_window_requests(members.kinds(), flags));
    let occupancy = Occupancy::default();
    let mut supplemental: Vec<u32> = Vec::new();
    for kind in members.kinds().iter().take(members.count()) {
        match r_handle::acquire(base, lrole, r_handle::WINDOW, *kind, Holder::Build(GroupId(u8::MAX)),
            member_recipe(members, *kind), &occupancy) {
            Ok(()) => supplemental.push(*kind),
            Err(LedgerRefusal::AlreadyHeld { .. }) => {}
            Err(other) => trace!("[Ledger] window acquisition refused owner={} kind={:#x} reason={:?}", owner, kind, other),
        }
    }
    trace!("[BodyBank] resource owner={} alias={:p} supplemental={:x?} native={:x?} flags r48={} r7d={} r7f={}",
        owner, resource, supplemental, native_window_requests(members.kinds(), flags).iter().map(|r| r.kind).collect::<Vec<_>>(),
        flags.r48, flags.r7d, flags.r7f);
    // Pair rule: the construction tail `0x60de60-0x60deb8` decrements `R+0x50` for
    // every body of an entry whose cells 0/1 are `[0x4b, 0x4c]` and resets the whole
    // R when the value before the decrement is at most 1. State 6 left the vanilla
    // pair's 2 (`0x651384-0x6513a4`); a pair-lead window constructs three bodies, so
    // the second to finish would reset R under the third (B119 state-11 stall).
    // One reference per window body: the native reset then follows the last body,
    // as it follows the second of a vanilla pair. This task completes before state
    // 7 admits any construction (`0x17e4750` waits on `R+0xa8`), so no decrement
    // can race this write and nothing else reads `R+0x50`. Written through the
    // ledger's writer under its reset-batch permit (a fresh window is the only batch).
    if let Some(references) = crate::engine::swap::body_bank::pair_resource_references(members) {
        match r_handle::set_pair_refs(lrole, resource as usize, r_handle::WINDOW, references as u8) {
            Ok(before) => trace!("[IcPair] resource owner={} alias={:p} refs {}->{} kinds={:x?} flags={:#x}",
                owner, resource, before, references, members.kinds(), read::<u32>(resource, 0x4c)),
            Err(others) => trace!("[IcPair] resource owner={} alias={:p} refs NOT written: other batches {:?} hold keys on R",
                owner, resource, others.iter().map(|b| b.0).collect::<Vec<_>>()),
        }
    }
}

/// Entry finalize (`0x6524e0`): frees the window contexts, destroys the window
/// cells and resets the alias R. Detached groups are destroyed first (§4.2) and
/// the attached window's pad routes are cleared (D-PTA1c at finalize: B123-pre
/// Yoga l123a showed 64 `Unmapped Read64 @ 0x0` inside the match-start finalize
/// of an attached Phase-B window), groups on or off.
#[skyline::hook(offset = 0x6524e0)]
unsafe fn finalize_entry(entry: *mut u8) -> u64 {
    if aligned(entry) {
        let owner = read::<u32>(entry, 0xe0);
        if PAIR_ENABLED.load(Ordering::Acquire) { pair_teardown(entry, "finalize"); }
        retire_window(entry, owner, "finalize", true);
        if GROUPS_ENABLED.load(Ordering::Acquire) { retire_groups(entry, owner, "finalize"); }
    }
    original!()(entry)
}

/// Owned 13.0.5 words at `offset`, or a diagnostic naming the mismatch.
unsafe fn fingerprint(base: usize, offset: usize, words: &[u32], label: &str) -> bool {
    for (i, word) in words.iter().enumerate() {
        if fields::read_unchecked::<u32>(base, offset + i * 4) != *word {
            trace!("[{}] fingerprint mismatch +0x{:x} word {}", label, offset, i);
            return false;
        }
    }
    true
}

/// Seam-table rows (`crate::engine::seams::native_seams::rows_a1`) this module hooks or calls, one list per
/// all-or-nothing feature. WP-A1 shadow: today's word checks still decide (`legacy`); `seam_install::gate`
/// checks and logs the 64-byte guards until the `native_body_bank` group is flipped to authoritative.
const SEAM_GROUP: &str = "native_body_bank";
const BANK_SEAMS: &[&str] = &["body_bank.prepare_models", "body_bank.prepare_resources",
    "body_bank.create_contexts", "body_bank.select_context"];
const FINALIZE_SEAMS: &[&str] = &["body_bank.finalize_entry"];
const GROUP_SWAP_SEAMS: &[&str] = &["group_swap.dispatcher_destroy", "group_swap.sync_construct",
    "group_swap.destroy_body", "group_swap.select_control", "group_swap.release_kind", "group_swap.acquire_kind",
    "group_swap.prepare_entry", "group_swap.unprepare", "group_swap.roster_refresh",
    "group_swap.copy_acquire", "group_swap.copy_release"];
/// Phase B′ rows (`0x655320` is `capacity.load_655320`, the capacity module's row at the same site; `0x654dc0` is
/// `body_bank.prepare_models` in `BANK_SEAMS`).
const ASYNC_BUILD_SEAMS: &[&str] = &["group_swap.body_alloc", "group_swap.heap_alloc", "group_swap.functor_copy",
    "group_swap.task_schedule", "group_swap.task_callback", "group_swap.task_wait", "group_swap.control_init",
    "group_swap.construct_functor", "capacity.load_655320"];
const PAIR_SEAMS: &[&str] = &["ic_pair.partner_window", "ic_pair.pair_partner", "ic_pair.activate_entry",
    "ic_pair.commit_change", "ic_pair.outgoing_owner", "ic_pair.deactivate_body"];
const SPECIAL_SEAMS: &[&str] = &["special_groups.object_release", "special_groups.heap_free",
    "special_groups.release_key"];

pub(super) unsafe fn install(base: usize) {
    use crate::runtime::seams::seam_install::gate;
    let legacy = [(0x654dc0, 0xd10103ffu32), (0x17e5bc0, 0xd105c3ff),
        (0x655484, 0xb94002f4), (0x64fbcc, 0xb940012a)].iter().all(|&(offset, word)| {
        let stock = fields::read_unchecked::<u32>(base, offset) == word;
        if !stock { trace!("[BodyBank] fingerprint mismatch +0x{:x}; disabled", offset); }
        stock
    });
    if !gate(SEAM_GROUP, BANK_SEAMS, legacy) {
        if legacy { trace!("[BodyBank] seam guard refused; disabled"); }
        return;
    }
    skyline::install_hooks!(prepare_models, prepare_resources, create_contexts, select_context);
    ENABLED.store(true, Ordering::Release);
    // WP-R: per-body identity view (all 18 selector exits or none; logs its own line).
    crate::runtime::roster::identity_view::install(base);
    // Both extensions and the attached window's route clear (D-PTA1c at finalize) tear down through the
    // finalize hook, so it is installed on its own row (WP3). Every gate below is evaluated (gate first, no
    // short circuit) so the install line records each row once.
    let finalize = gate(SEAM_GROUP, FINALIZE_SEAMS,
        fingerprint(base, FINALIZE_ENTRY, &[0xd10143ff, 0xa9015ff8, 0xa90257f6, 0xa9034ff4], "BodyBank"));
    if finalize { skyline::install_hooks!(finalize_entry); }
    // Group swap: every native function it calls or hooks, owned 13.0.5 words.
    let groups: [(usize, &[u32]); 11] = [
        (DISPATCHER_DESTROY, &[0xa9bb67fa, 0xa9015ff8, 0xa90257f6, 0xa9034ff4, 0xa9047bfd, 0x910103fd, 0xf9400416, 0xaa0003f3]),
        (SYNC_CONSTRUCT, &[0xd10183ff, 0xa9025ff8, 0xa90357f6, 0xa9044ff4, 0xa9057bfd, 0x910143fd, 0xf960a808]),
        (DESTROY_BODY, &[0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd, 0x531c7c08, 0x7100111f]),
        (SELECT_CONTROL, &[0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd, 0xf920bc1f, 0x528b23e8]),
        (RELEASE_KIND, &[0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd, 0xf9405415, 0x2a0103f4]),
        (ACQUIRE_KIND, &[0xa9ba6ffc, 0xa90167fa, 0xa9025ff8, 0xa90357f6]),
        // U18: the copy registry's three writers (prologue, then `cmn w2|w1, #1; b.eq` / `mov x20, #0xcf`).
        (PREPARE_ENTRY, &[0xd10243ff, 0xa9036ffc, 0xa90467fa, 0xa9055ff8, 0xa90657f6, 0xa9074ff4, 0xa9087bfd, 0x910203fd, 0x3100045f, 0x540001e0]),
        (UNPREPARE, &[0xd101c3ff, 0xa9016ffc, 0xa90267fa, 0xa9035ff8, 0xa90457f6, 0xa9054ff4, 0xa9067bfd, 0x910183fd, 0x3100043f, 0x54000180]),
        (ROSTER_REFRESH, &[0xd103c3ff, 0xa9096ffc, 0xa90a67fa, 0xa90b5ff8, 0xa90c57f6, 0xa90d4ff4, 0xa90e7bfd, 0x910383fd, 0xd28019f4, 0xf2aff214]),
        // K15: the copy-package acquire and release (`swap_native::copy_packages`).
        (COPY_ACQUIRE, &[0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd, 0x2a0203f6, 0xb9402822, 0xaa0103f4, 0xaa0003f3]),
        (COPY_RELEASE, &[0xa9bc5ff8, 0xa90157f6, 0xa9024ff4, 0xa9037bfd, 0x9100c3fd, 0x3100045f, 0x54001e20, 0x51013448]),
    ];
    let groups_ok = gate(SEAM_GROUP, GROUP_SWAP_SEAMS,
        finalize && groups.iter().all(|(offset, words)| fingerprint(base, *offset, words, "GroupSwap"))) && finalize;
    // Pair bank: the N1 site (`mov x6,xzr; ldrb w3; mov w2,w20; bl 6b6f10; str x21,[x19,#0x4190]`),
    // activation, commit and the outgoing-owner selector prologues, and the
    // fighter vtable's deactivate slot (`+0x48` = `0x60ffd0`).
    let pair: [(usize, &[u32]); 4] = [
        (PAIR_PARTNER_SITE - 8, &[0xaa1f03e6, 0x39424903, 0x2a1403e2, 0x9401865b, 0xf920ca75]),
        (ACTIVATE_ENTRY, &[0xd10243ff, 0xa90467fa, 0xa9055ff8, 0xa90657f6]),
        (COMMIT_CHANGE, &[0xd101c3ff, 0xfd0013e8, 0xf90017f9, 0xa9035ff8]),
        (OUTGOING_OWNER, &[0xa9bf7bfd, 0x910003fd, 0x2941a009, 0x51012d2a]),
    ];
    // The deactivate slot is relocated data (no static bytes to guard): its runtime check stays after the flip.
    let slot = fields::read_unchecked::<usize>(base, FIGHTER_VTABLE + 0x48) == base + DEACTIVATE_BODY;
    let pair_ok = gate(SEAM_GROUP, PAIR_SEAMS,
        finalize && pair.iter().all(|(offset, words)| fingerprint(base, *offset, words, "IcPair"))
            && slot && fingerprint(base, DEACTIVATE_BODY, &[0xd10203ff, 0xa9026ffc, 0xa90367fa, 0xa9045ff8], "IcPair"))
        && finalize && slot;
    // Special groups: the object-record destroyer pair finalize uses (`0x652f5c-0x652f84`)
    // and the per-key release `0x17e8610` is built on (`add x19,x0,#0x28; mov x20,x0`).
    let special_ok = gate(SEAM_GROUP, SPECIAL_SEAMS, groups_ok
        && fingerprint(base, SPECIAL_OBJECT_RELEASE, &[0xf81b0ff9, 0xa9015ff8, 0xa90257f6, 0xa9034ff4], "SpecialGroups")
        && fingerprint(base, HEAP_FREE, &[0xd100c3ff, 0xa9027bfd, 0x910083fd, 0xf81f83a0], "SpecialGroups")
        && fingerprint(base, articles::POOL_BUILD, &articles::POOL_BUILD_WORDS, "SpecialGroups")
        && fingerprint(base, RELEASE_KEY, &[0xa9bd57f6, 0xa9014ff4, 0xa9027bfd, 0x910083fd, 0x9100a013, 0xaa0003f4], "SpecialGroups"))
        && groups_ok;
    SPECIAL_ENABLED.store(special_ok, Ordering::Release);
    // Phase B′ (operator decision 2026-11-02): the game's asynchronous construction path, owned 13.0.5
    // words, all or nothing. The functor vtable is relocated data (no static bytes to guard): its `+0x30`
    // slot must be the functor the callback runs (checked live, as the deactivate slot is). A mismatch
    // keeps every build synchronous.
    let asynchronous: [(usize, &[u32]); 10] = [
        (BODY_ALLOC, &[0xf81b0ff9, 0xa9015ff8, 0xa90257f6, 0xa9034ff4, 0xa9047bfd, 0x910103fd, 0x12001c08, 0x7100111f]),
        (HEAP_ALLOC, &[0xd10203ff, 0xa9077bfd, 0x9101c3fd, 0xf81f83a0, 0xf81f03a1, 0x14000001, 0x14000001, 0x9100c3e0]),
        (FUNCTOR_COPY, &[0xd10103ff, 0xa9024ff4, 0xa9037bfd, 0x9100c3fd, 0xeb00003f, 0x540008a0, 0xaa0003f4, 0xf9401000]),
        (TASK_SCHEDULE, &[0xa9bb67fa, 0xa9015ff8, 0xa90257f6, 0xa9034ff4, 0xa9047bfd, 0x910103fd, 0xb9401008, 0x340000e8]),
        (TASK_CALLBACK, &[0x3941c008, 0x34000048, 0xd65f03c0, 0xf9403000, 0xb4000080, 0xf9400008, 0xf9401901, 0xd61f0020]),
        (TASK_WAIT, &[0xd10183ff, 0xa90167fa, 0xa9025ff8, 0xa90357f6, 0xa9044ff4, 0xa9057bfd, 0x910143fd, 0xaa0103f4]),
        (COUNT_SETUP, &[0xd101c3ff, 0xa9016ffc, 0xa90267fa, 0xa9035ff8, 0xa90457f6, 0xa9054ff4, 0xa9067bfd, 0x910183fd]),
        (CONTROL_INIT, &[0xf81b0ff9, 0xa9015ff8, 0xa90257f6, 0xa9034ff4, 0xa9047bfd, 0x910103fd, 0x528b23e8, 0x38686808]),
        (CONTROL_CALLBACK, &[0x3941c008, 0x34000048, 0xd65f03c0, 0xf9403000, 0xb4000080, 0xf9400008, 0xf9401901, 0xd61f0020]),
        (CONTROL_FUNCTOR, &[0xf9400408, 0xf9400100, 0x17ffe0ce]),
    ];
    let functor_slot = fields::read_unchecked::<usize>(base, CONSTRUCT_FUNCTOR_VTABLE + 0x30);
    // F6: state 8's functor vtable is relocated data too; its `+0x30` slot must be the control-init functor.
    let control_slot = fields::read_unchecked::<usize>(base, CONTROL_FUNCTOR_VTABLE + 0x30);
    let slot_ok = functor_slot == base + CONSTRUCT_FUNCTOR && control_slot == base + CONTROL_FUNCTOR;
    let async_ok = gate(SEAM_GROUP, ASYNC_BUILD_SEAMS, groups_ok && slot_ok
        && fingerprint(base, CONSTRUCT_FUNCTOR, &[0xf81e0ff3, 0xa9017bfd, 0x910043fd, 0xf9400413, 0xf9400260, 0xf9400008, 0x29410801, 0xb9401003], "GroupSwap")
        && asynchronous.iter().all(|(offset, words)| fingerprint(base, *offset, words, "GroupSwap")))
        && groups_ok && slot_ok;
    ASYNC_ENABLED.store(async_ok, Ordering::Release);
    trace!("[GroupSwap] async build {} functor_slot={:#x} expected={:#x} control_slot={:#x} expected={:#x}",
        if async_ok { "enabled" } else { "disabled by fingerprint; synchronous build" }, functor_slot, base + CONSTRUCT_FUNCTOR,
        control_slot, base + CONTROL_FUNCTOR);
    if groups_ok {
        // The `[GroupSwap] enabled` line follows the startup cycle-preload load
        // (`report_group_swap`), so it shows the switches this boot applies.
        GROUPS_ENABLED.store(true, Ordering::Release);
    } else {
        trace!("[GroupSwap] disabled by fingerprint; window bank active");
    }
    if pair_ok {
        skyline::install_hooks!(pair_partner, activate_entry, commit_change);
        PAIR_ENABLED.store(true, Ordering::Release);
        trace!("[IcPair] enabled N1=+0x{:x} N2=+0x{:x} N3=+0x{:x} outgoing_owner=+0x{:x} deactivate=+0x{:x} flags pair/extra={:#x}/{:#x}",
            PAIR_PARTNER_SITE, ACTIVATE_ENTRY, COMMIT_CHANGE, OUTGOING_OWNER, DEACTIVATE_BODY, PAIR_ACTIVE_FLAG, BANK_FLAG);
    } else {
        trace!("[IcPair] disabled by fingerprint; the normal pair keeps the canonical Reload");
    }
}
