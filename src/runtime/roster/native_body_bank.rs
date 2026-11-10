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
//! An ordinary or Mii group's build (`async_build_eligible`) now takes that path at the acknowledgement
//! seam: the entry-level third of `0x658af0` (`0x654dc0`, `0x655320`, `0x64f820`) and the per-body
//! `0x3ae5f0` allocation on the game thread, the init+construct as queued tasks, the entry parked
//! at loader `0xc` with an empty window while frames run (the native shape of states 6-0xb; a
//! request meanwhile is refused as busy), then on the settling tick the cell install, the request
//! bit's return, the ordinary verification and the acknowledgement (`settle_construction`,
//! `complete_exchange`). A destroyer that arrives first waits for the tasks as state 0xb's tail
//! does and installs the bodies for native's own window destroyer (`drain_construction`). Native
//! (PT/Aegis) and pair groups keep the synchronous build: an operator-approved restriction dated
//! 2026-11-02 (the `+0x5a88` object `0x658af0`'s branches create; the `R+0x50` batch rule). Mii
//! groups left it by the operator's decision of 2026-11-08 (rc.6 item 2): their identity is in
//! `R` before the construction is queued and stays while the group is the window. P3-0: `Bank.tick` measures the largest gap between
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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use hos_lock::Mutex;
use crate::engine::swap::body_bank::{
    count_extra_groups_iter, dispatcher_array_header_ok, dispatcher_idle_words, native_group_request, original_bankable,
    original_form_bankable, pair_request, pair_resource_references, plan_extra_groups_with, prefetch_outcome, info_wrapper,
    swap_commit_index_in, async_build_eligible, construction_task, TickGap,
    background_build_eligible, next_background, BACKGROUND_INFO_SPANS, trainer_task, HolderStep, object_rearm_due, object_ready,
    PT_OBJECT_KIND,
    GroupBudget, GroupExchange, GroupOwner, GroupPolicy, GroupState, Hold, Members, Phase, Special,
    SpecialPolicy, BANK_FLAG, MAX_EXTRA_GROUPS, NANA, NATIVE_MEMBERS, PAIR_ACTIVE_FLAG, POOL_RESERVE, POPO,
    entry_bytes_of, mii_exit_bytes_match, mii_reuse_bytes_match, MiiEntryBytes, MiiIdentity, MII_ENTRY_START, RESOURCE_WORDS,
    baseline_words, identity_move, IdentityMove, leaves_at_pause, count_group_members, kept_members,
    release_for_new_members, capacity_release, leaves_now, ReleaseWatch, arriving_room, room_coming, still_short,
    next_window_plan, own_words, shadow_values, LiveIdentity, speculative_step_admitted, serving_step, Serving,
    target_first, reload_extras, notice_extras, canonical_replaces_window, neighbour_group, pack_neighbours,
};
use crate::engine::swap::body_bank::memory::{self, Admission};
use crate::engine::swap::body_bank::mii_publish::{PublishAttempts, PublishRefusal, PublishStep};
use crate::engine::preload::preload_metrics::MemorySample;
use crate::engine::preload::cycle_preload_config::Settings;
use crate::engine::swap::character_forms::{EntryState, Forms};
use crate::engine::mii::lifetime::Configuration;
use crate::engine::swap::swap_model::{capabilities, form_class, group_serves, previous_cell_index};
use crate::engine::swap::swap_model::lifecycle::{attached_plan, BodyRef, ContextRef, DetachedPlan, DetachedView, DispatcherRef, RecordsRef};
use crate::engine::compat::fixed::{DebugIter, FixedList, FixedVec};
use crate::engine::swap::swap_model::copy_registry::{self, RegistryOps};
use crate::engine::swap::swap_model::fallback::{self, Disposition, Fallback};
use crate::engine::swap::swap_model::request_bit::{End as LeaseEnd, Lease};
use crate::engine::swap::swap_model::ledger::{native_window_requests_fixed as native_window_requests, GroupId, Holder, LedgerRefusal, ReleaseRecipe, Role as LedgerRole};
use crate::engine::swap::swap_model::occupancy::{KindOccupancy, Occupancy, Rules};
use crate::runtime::roster::body_graph::{self as graph, DESTROY_BODY, DISPATCHER_DESTROY, HEAP_FREE, SPECIAL_OBJECT_RELEASE};
use crate::runtime::roster::r_handle;
use crate::runtime::roster::res_unload;
use crate::runtime::roster::special_articles as articles;
use crate::engine::swap::swap_model::special_articles::{attachable, prepare_due, RetireStage};
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
/// Inbox 02:00Z: pass 2's own calls (`0x3ac560`, `0x3aeb50`, `0x3a9ab0`, the guard pair) passed, so a PT exit retires
/// its pool in stages (`swap_model::special_articles::RetireStage`); otherwise the exit's tick keeps native's whole
/// `0x3a6b80`.
static EXIT_SPLIT: AtomicBool = AtomicBool::new(false);

/// Inbox 02:00Z: a detached PT pool whose exit ran pass 1 only; the group lifecycle runs one more stage per tick
/// (`retire_step`) under INSTALL (`token`, taken by the first later stage).
#[derive(Clone, Copy, Debug)]
struct Retiring {
    group: usize,
    holder: usize,
    stage: RetireStage,
    token: Option<super::InstallToken>,
}

/// Inbox 02:42Z: the prepared pool build's functor checks passed (`install`), so a detached PT group's retired pool is
/// built on the task worker ahead of its attach (`special_articles::prepare_due`); otherwise the attach rebuilds it.
static PREPARE_ENABLED: AtomicBool = AtomicBool::new(false);
/// Native state 11's trainer functor vtable (`0x4f81498`, seven relocated slots) with its call (`+0x30`) and its two
/// clones (`+0x10` heap, `+0x18` in place: they write the native vtable, `0x6519c8`, `0x6519f0`) replaced, filled once
/// at install. The scheduler keeps a clone of the functor, so the clones must keep this vtable.
static PREPARE_VTABLE: [AtomicUsize; 7] = [const { AtomicUsize::new(0) }; 7];
/// The trainer functor's two clones (`0x651970`, `0x6519f0`).
const TRAINER_CLONE_HEAP: usize = 0x651970;
const TRAINER_CLONE_INTO: usize = 0x6519f0;

/// Inbox 02:42Z: a detached PT group's pool being built on the task worker (`queue_prepare`), under INSTALL.
#[derive(Clone, Copy, Debug)]
struct Preparing {
    group: usize,
    holder: usize,
    task: usize,
    since: u64,
    token: super::InstallToken,
}

/// The prepared build's call (`PREPARE_VTABLE + 0x30`), on the task worker: `0x3a6270` on the task's holder with its
/// owner as the slot, exactly what `special_articles::rebuild_current` runs at an attach (`build_pool`). Nothing else of
/// the trainer functor runs (no `0x17e8890` lookup, no G68 registration `0x17eeaa0`).
unsafe extern "C" fn prepare_call(functor: usize) {
    let base = BASE.load(Ordering::Acquire);
    let Some(task) = fields::child(functor, 8) else { return; };
    let Some(holder) = fields::child(task, trainer_task::HOLDER) else { return; };
    let slot = fields::read::<u32>(task, trainer_task::OWNER).unwrap_or(u32::MAX);
    articles::build_pool(base, holder, slot);
}

/// `PREPARE_VTABLE + 0x18`: the in-place clone, as `0x6519f0` with this vtable.
unsafe extern "C" fn prepare_clone_into(src: usize, dst: usize) {
    fields::write_unchecked(dst, 0, PREPARE_VTABLE.as_ptr() as usize);
    fields::write_unchecked(dst, 8, fields::read_unchecked::<usize>(src, 8));
}

/// `PREPARE_VTABLE + 0x10`: the heap clone, as `0x651970` (`0x392e290(0x10, 0x10)`) with this vtable.
unsafe extern "C" fn prepare_clone_heap(src: usize) -> usize {
    let heap_alloc: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(BASE.load(Ordering::Acquire) + HEAP_ALLOC);
    let dst = heap_alloc(0x10, 0x10);
    if dst != 0 { prepare_clone_into(src, dst); }
    dst
}
/// Phase B′ fingerprints (the asynchronous construction path) and the functor vtable slot passed.
static ASYNC_ENABLED: AtomicBool = AtomicBool::new(false);
/// rc.6 item 3: the Phase B′ rows passed and the identity view's body-init site is hooked (its hook skips the
/// worker's registration for a background build, `background_skip`).
static BACKGROUND_ENABLED: AtomicBool = AtomicBool::new(false);
/// rc.6 item 3, Ice Climbers (SPECIAL-GROUPS §2): the construction tail's pair test is redirected for background bodies
/// (`background_tail`), so a group may build beside a live `[Popo, Nana]` window.
static BACKGROUND_PAIR_TAIL: AtomicBool = AtomicBool::new(false);
/// One bit per owner whose background build is in flight.
static BACKGROUND_OWNERS: AtomicU32 = AtomicU32::new(0);
/// Registrations `background_skip` has skipped since boot (each settlement logs its own difference).
static BACKGROUND_SKIPS: AtomicU32 = AtomicU32::new(0);
/// rc.6 item 3, SPECIAL-GROUPS §3: Pokémon Trainer and Pyra/Mythra groups build in the background too (the X5 site is
/// hooked and native state 11's holder step passed its rows).
static NATIVE_BACKGROUND: AtomicBool = AtomicBool::new(false);
/// The cell 1 a Pyra/Mythra group's background bodies read at the X5 site (`background_partner`); `u32::MAX` otherwise.
static BACKGROUND_PARTNER: AtomicU32 = AtomicU32::new(u32::MAX);
/// rc.6 item 3, SPECIAL-GROUPS §0 X4: a background Popo or Nana body takes the info wrapper its building group's words
/// choose (`background_info`), so a pair group may build beside a live one-actor window as its synchronous build does.
static BACKGROUND_X4: AtomicBool = AtomicBool::new(false);
/// Per owner, the building group's words for X4 (`pack_cells`), stored before the owner's background bit is set.
static BACKGROUND_CELLS: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
/// Background Nana bodies left on native's wrapper because the entry held no `+0x108` info, and those given their own
/// `+0x100` (both reported at settlement).
static BACKGROUND_INFO_KEPT: AtomicU32 = AtomicU32::new(0);
static BACKGROUND_INFO_OWN: AtomicU32 = AtomicU32::new(0);
static BANKS: [MirroredMutex<Option<Bank>>; 2] = [MirroredMutex::new(0, None), MirroredMutex::new(1, None)];

/// A role's bank behind its lock, plus the ready observation [`cached_kind`] needs, mirrored in [`READY`] whenever a guard drops: the
/// menu worker's roster re-plan reads that without the lock. The bank's own hooks run on the game's task workers (`allocation_trace`,
/// hot-slots hook-thread table 2026-11-09), so a plugin thread holding `BANKS` could stall them while the game starves it (rc.6 freeze
/// class, `tests/lock_order.rs`).
struct MirroredMutex<T> {
    role: usize,
    lock: Mutex<T>,
}

impl<T> MirroredMutex<T> {
    const fn new(role: usize, value: T) -> Self {
        Self { role, lock: Mutex::new(value) }
    }
}

impl MirroredMutex<Option<Bank>> {
    fn lock(&self) -> BankGuard<'_> {
        BankGuard { role: self.role, guard: self.lock.lock() }
    }
}

struct BankGuard<'a> {
    role: usize,
    guard: hos_lock::MutexGuard<'a, Option<Bank>>,
}

impl std::ops::Deref for BankGuard<'_> {
    type Target = Option<Bank>;
    fn deref(&self) -> &Option<Bank> {
        &self.guard
    }
}

impl std::ops::DerefMut for BankGuard<'_> {
    fn deref_mut(&mut self) -> &mut Option<Bank> {
        &mut self.guard
    }
}

impl Drop for BankGuard<'_> {
    fn drop(&mut self) {
        // Still under the bank's lock: the one writer of this role's mirror.
        READY[self.role].write(&ready_words(&self.guard));
    }
}

/// `[ready, entry, owner, kind, physical descriptor (0x70 bytes)]`, all zero unless the bank is Ready, not departed and observed.
const READY_WORDS: usize = 4 + 0x70 / 8;
/// Rounds of [`crate::model::seq_cell::SeqCell::read_yielding`] for `cached_kind` on a plugin thread (each 64 tries, then a
/// yield).
const READY_ROUNDS: usize = 1024;
static READY: [crate::model::seq_cell::SeqCell<READY_WORDS>; 2] =
    [crate::model::seq_cell::SeqCell::new(), crate::model::seq_cell::SeqCell::new()];

fn ready_words(bank: &Option<Bank>) -> [u64; READY_WORDS] {
    let mut words = [0; READY_WORDS];
    if let Some(bank) = bank.as_ref().filter(|bank| !bank.departed && bank.phase == Phase::Ready) {
        if let Some((physical, kind)) = bank.observed {
            words[..4].copy_from_slice(&[1, bank.entry as u64, u64::from(bank.owner), u64::from(kind)]);
            for (word, bytes) in words[4..].iter_mut().zip(physical.chunks_exact(8)) {
                *word = u64::from_le_bytes(bytes.try_into().expect("8 bytes"));
            }
        }
    }
    words
}
// Filled before the hooks are enabled; each role moves its storage into its
// first bank and retains it through every subsequent native bank lifetime.
static BANK_STORAGE: [Mutex<Option<BankStorage>>; 2] = [Mutex::new(None), Mutex::new(None)];
static RESERVED: AtomicU8 = AtomicU8::new(0);
static ROLE_BUDGET: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
// Display targets are not resource reservations: canonical-only original
// restoration still occupies a body cell but cannot promise a reuse hit.
static PLANNED_TARGETS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
/// New members per role that load once the parked groups marked to go now are freed (operator rule 2026-11-09 13:21Z,
/// `body_bank::release_for_new_members`).
static RELEASE_MEMBERS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
/// Operator rule 2026-11-09 16:06Z: whether a free's room is still arriving (`body_bank::ReleaseWatch`). Taken alone,
/// never under another lock.
static RELEASE_WATCH: Mutex<ReleaseWatch> = Mutex::new(ReleaseWatch::Settled);
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
/// excluded. So are the normal Ice Climbers pair while the pair bank is on, and a
/// native-group original while native groups are on (rc.6, `original_form_bankable`;
/// its window or group serves it). Solo Popo and Miis stay canonical.
fn excluded_original(settings: &Settings, menu: &crate::engine::compat::menu::TrainingModpackMenu, role: usize) -> Option<u32> {
    let logical = if role == 0 { super::Role::Player } else { super::Role::Cpu };
    canonical_original(menu, role).filter(|kind| !(settings.original_bank_enabled()
        && original_form_bankable(*kind, super::requested_forms_at(menu, logical, *kind, None), pair_enabled(settings),
            special_policy(settings).native)))
}

/// The Mii adapters' gate for a return to the role's slot-0 original (EXCLUSIONS §9 with rc.6's pair and native-group
/// originals): may the bank serve it, so the return is a warm switch and never the Reload that retires the bank?
/// `original_bank` on and, for Ice Climbers, Pokémon Trainer and Pyra/Mythra, `original_form_bankable` for the form the
/// staged request's words spell (the pair needs the pair bank, a native group native groups). Every other kind keeps
/// `original_bankable(kind)`.
pub(crate) fn original_member(kind: u32, descriptor: &[u8; 0x70]) -> bool {
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if !settings.original_bank_enabled() { return false; }
    if kind != POPO && Members::native_group(kind).is_none() { return original_bankable(kind); }
    let word = |at: usize| u32::from_le_bytes([descriptor[at], descriptor[at + 1], descriptor[at + 2], descriptor[at + 3]]);
    crate::engine::swap::swap_model::capabilities::request_forms(word(0x14), [word(0x18), word(0x1c), word(0x20)], word(0x28))
        .is_some_and(|forms| original_form_bankable(kind, forms, pair_enabled(&settings), special_policy(&settings).native))
}

fn groups_enabled(settings: &crate::engine::preload::cycle_preload_config::Settings) -> bool {
    settings.groups_enabled() && GROUPS_ENABLED.load(Ordering::Acquire)
}

/// Bodies the shared budget has besides the two active fighters (`ready_fighters` − 2): the room any group could take.
fn group_pool(settings: &crate::engine::preload::cycle_preload_config::Settings) -> usize {
    usize::from(settings.ready_fighters).saturating_sub(2)
}

/// Which ordinary kinds may join a group (the planner's and the target-first seed's one rule).
fn group_policy(settings: &crate::engine::preload::cycle_preload_config::Settings) -> GroupPolicy {
    GroupPolicy {
        solo_popo_groups: settings.solo_popo_groups_enabled(),
        ic_pair_groups: settings.ic_pair_groups_enabled() && PAIR_ENABLED.load(Ordering::Acquire),
    }
}

/// rc.7 target-first: per role, the neighbours of the last target-first Reload's full window (`body_bank::pack_neighbours`),
/// which `plan_groups` plans as that window's first group once it is Ready. `u64::MAX`: none.
static NEIGHBOURS: [AtomicU64; 2] = [AtomicU64::new(u64::MAX), AtomicU64::new(u64::MAX)];

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

/// Run a native key request or release for a group with its own words in `R` (`group_words`; `None`: `R` holds them),
/// then put the window's words back (the bracket of MII-GROUPS-DESIGN §3.2).
unsafe fn with_words(r: usize, words: Option<[u32; RESOURCE_WORDS]>, f: impl FnOnce()) {
    match words {
        Some(words) => {
            let saved = save_resource_words(r);
            write_resource_words(r, &words);
            f();
            write_resource_words(r, &saved);
        }
        None => f(),
    }
}

/// The words `group`'s native work takes in `R` over its `current` words (`body_bank::own_words`): a Mii group its
/// identity's; X6, beside a live Mii window any other group the owner's ordinary baseline with its own header kinds
/// (the release selector `0x17e8410` and the request key on `R+0x58`/`+0x64` as the worker does, SPECIAL-GROUPS §0 X3).
fn group_words(bank: &Bank, group: usize, current: [u32; RESOURCE_WORDS]) -> Option<[u32; RESOURCE_WORDS]> {
    let g = bank.groups.get(group)?;
    own_words(*g.members, g.identity, bank.identity.is_some(), bank.ordinary_identity.map(|(words, _)| words), current)
}

/// [`group_words`] for the role's bank over `r`'s current words. Never called under the bank lock.
unsafe fn group_words_of(role: usize, group: usize, r: usize) -> Option<[u32; RESOURCE_WORDS]> {
    let current = save_resource_words(r);
    BANKS[role].lock().as_ref().and_then(|bank| group_words(bank, group, current))
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
    BANKS[role].lock().as_ref().and_then(|bank| bank.groups.iter()
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
        let state = BANKS[role].lock();
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
    let state = BANKS[role].lock();
    let bank = state.as_ref().filter(|b| b.owner == owner && b.phase == Phase::Ready && !b.departed
        && b.swap.is_none() && b.prefetch.is_none())?;
    // One record per owner: a group's CharInfo stays on it until that group's prefetch copied it, even while a waiting
    // press holds the prefetch back (primary 2026-11-09 07:34Z, `body_bank::serving_step`).
    if bank.groups.iter().any(|g| g.charinfo_published && g.state.startable()) { return None; }
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
        let mut state = BANKS[role].lock();
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
/// Climbers (`roster_grid::cycle_exclusion`), so that entry is the slot-0 original, never a single-body
/// member: its return is served by the pair-lead window or a pair group (`original_member`, rc.6) or
/// is the canonical Reload. The form of every kept `0x4b` is still re-proven per request
/// (`group_serves`). No allocation.
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
/// roles, active fighters included): window groups first, then extra groups by
/// cycle depth; fighter helper slots get what remains of that budget, capped by
/// the saved slot count. Kirby hats have their own budget ("Ready Hats", P-HOT).
pub(crate) fn plan_budget(menu: &crate::engine::compat::menu::TrainingModpackMenu,
    lanes: &[[Option<crate::runtime::roster::cycle_preload::Target>; crate::runtime::roster::cycle_preload::MAX_TARGETS]; 4])
    -> [Option<Members>; 2] {
    let started = now_us();
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    let mut wanted = [0; 2];
    // What the notice counts for each role's next window (`body_bank::notice_extras`), beside the physical want.
    let mut notice_wanted = [0; 2];
    let mut wanted_banks = [None; 2];
    let grouped = groups_enabled(&settings);
    if ENABLED.load(Ordering::Acquire) && settings.fighters && settings.bank_members >= 2 {
        for (role, lane) in lanes[2..].iter().enumerate() {
            // CPU-WINDOW-REFORM: with group swap off a cycling role's window demand is what its whole cycle can use
            // (`body_bank::window_share`), not what the next `MAX_TARGETS` entries can: a lookahead of special kinds
            // (Pyra, PT, IC, the Miis) must not give its share to helper sources, which stay resident for the scene
            // (Yoga swoff2 steps 79-97).
            if !grouped && lane.iter().any(Option::is_some) {
                // P-CYCLE / P1-CYCLE: the one cycle list, with that role's published Mii admission.
                let roster = if role == 0 {
                    crate::engine::swap::cycle_members::cycle_members(menu.player_roster, super::player_cycle_admission())
                } else {
                    crate::engine::swap::cycle_members::cycle_members(menu.cpu_roster, super::cpu_cycle_admission())
                };
                let mut cycle = crate::engine::swap::ordered_roster::KindList::new(0);
                for fighter in roster.iter() { cycle.push(fighter.kind()); }
                // The `window_lane` rule: a cycle `0x4b` is a member only as this role's Solo Popo.
                let logical = if role == 0 { super::Role::Player } else { super::Role::Cpu };
                let solo = canonical_original(menu, role) != Some(POPO)
                    && super::requested_forms_at(menu, logical, POPO, Some(0)) == Forms::solo_popo();
                wanted[role] = crate::engine::swap::body_bank::window_share::cycle_window_demand(
                    &cycle, settings.bank_members, solo);
                notice_wanted[role] = wanted[role];
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
                    // rc.7 target-first: the bodies the next Reload builds besides the target (none when its neighbours
                    // build as groups, which `plan_groups` charges); the notice still counts the window it prepares.
                    wanted[role] = wanted[role].max(reload_extras(members, forms, grouped, group_pool(&settings)));
                    notice_wanted[role] = notice_wanted[role].max(notice_extras(members));
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
    // The notice's share of the same pool: equal to `reserved` except where target-first builds the target alone.
    let notice_reserved = crate::engine::swap::body_bank::reserve_extras(notice_wanted, live, window_pool);
    let originals = [excluded_original(&settings, menu, 0), excluded_original(&settings, menu, 1)];
    let mut group_targets = [0u8; 2];
    let mut group_bodies = 0usize;
    if grouped {
        group_bodies = plan_groups(&settings, menu, lanes, originals, &mut group_targets);
    }
    // rc.6 item 3: with groups on, the live attached window plans what it serves (`body_bank::window_targets`; the values
    // `progress` reads).
    let live: [Option<(Members, u32)>; 2] = std::array::from_fn(|role| BANKS[role].lock().as_ref()
        .filter(|bank| bank.phase == Phase::Ready && !bank.departed)
        .map(|bank| (*bank.members, bank.observed.map_or(bank.members.kinds()[usize::from(bank.target)], |(_, kind)| kind))));
    for (role, value) in reserved.iter().enumerate() {
        ROLE_BUDGET[role].store(*value as u8, Ordering::Release);
        let next = wanted_banks[role].map_or(0, |(members, target)| {
            // A pair-lead bank plans one selectable extra (cell 2); Nana is not a target. The
            // target keeps its own cell (`CellRule::PinCell0` puts Kirby or Solo Popo at cell 0),
            // so the counted members are the target plus its first `notice_reserved` window neighbours,
            // never "the first cells" with cell 0 taken as the active fighter (`body_bank::next_window_plan`).
            // rc6-hot-xps19: a detached record that holds them (the window a Mii group displaced) counts with the groups.
            let grouped = |kind: u32| BANKS[role].lock().as_ref().is_some_and(|bank| !bank.departed
                && bank.groups.iter().any(|g| g.state != GroupState::Released && g.members.index_of(kind).is_some()));
            next_window_plan(members, target, notice_reserved[role], originals[role], grouped)
        });
        let targets = crate::engine::swap::body_bank::window_targets(grouped, live[role], originals[role], next);
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
    slow_step(u32::MAX, "plan", started);
    // A planned kind alone is not a loaded body. This is a value receipt from
    // the current native callbacks; no pointer is followed on the planner thread.
    std::array::from_fn(|role| BANKS[role].lock().as_ref()
        // This filter reserves occupied native resources, not UI readiness.
        // A failed/lapsed receipt still owns its cells until native retirement.
        .filter(|bank| bank.phase == Phase::Ready && !bank.departed)
        .map(|bank| *bank.members))
}

/// The kinds a role's groups may hold (rc.6 mid-session cycle change, `body_bank::group_left_cycle`): its cycle's
/// members, as the cycle list reads them now, and its original.
fn cycle_wanted(menu: &crate::engine::compat::menu::TrainingModpackMenu, role: usize) -> crate::engine::swap::ordered_roster::KindList {
    let roster = if role == 0 {
        crate::engine::swap::cycle_members::cycle_members(menu.player_roster, super::player_cycle_admission())
    } else {
        crate::engine::swap::cycle_members::cycle_members(menu.cpu_roster, super::cpu_cycle_admission())
    };
    let mut wanted = crate::engine::swap::ordered_roster::KindList::new(0);
    for fighter in roster.iter() { wanted.push(fighter.kind()); }
    if let Some(kind) = super::original_kind(if role == 0 { super::Role::Player } else { super::Role::Cpu }) { wanted.push(kind); }
    wanted
}

/// Value-only group planning on the sparse planner: appends `Planned` groups to
/// a Ready window bank; never removes or touches a native object. Returns the
/// extra-group bodies charged to the shared budget and the per-role UI targets.
fn plan_groups(settings: &crate::engine::preload::cycle_preload_config::Settings,
    menu: &crate::engine::compat::menu::TrainingModpackMenu,
    lanes: &[[Option<crate::runtime::roster::cycle_preload::Target>; crate::runtime::roster::cycle_preload::MAX_TARGETS]; 4],
    originals: [Option<u32>; 2], targets: &mut [u8; 2]) -> usize {
    // Operator rule 2026-11-09 16:06Z and option C (17:57Z, primary 19:47Z): a free's room is arriving until the game has
    // unloaded what the frees queued (`res_unload`, read each pass) and then a purge requested after that has run on
    // MainThread (`purge_on_main_thread`); the pass that sees it settles.
    let unloading = res_unload::waiting();
    let (before, after) = {
        let mut watch = RELEASE_WATCH.lock();
        let before = *watch;
        *watch = watch.pass(unloading.map(|(u, _)| u.waiting()), res_unload::purges(), res_unload::base().is_some());
        (before, *watch)
    };
    if let Some(ticket) = after.requested() { res_unload::request_purge(ticket); }
    if before != after {
        if let Some((_, generation)) = unloading.filter(|_| after.settled()) { res_unload::settled(generation); }
        trace!("[GroupSwap] released room {:?} -> {:?} unloading={:?} committed={:?}", before, after,
            unloading.map(|(u, _)| u), MemorySample::capture().committed.get());
    }
    let capacity = POOL_CAPACITY.load(Ordering::Acquire) as usize;
    let live = POOL_LIVE.load(Ordering::Acquire) as usize;
    let pool_free = if capacity == 0 { 0 } else { capacity.saturating_sub(live).saturating_sub(POOL_RESERVE) };
    let hats_wanted = lanes[..2].iter().flatten().flatten().count().min(usize::from(settings.slots));
    let policy = group_policy(settings);
    let special = special_policy(settings);
    let mut windows = [None; 2];
    let mut existing = [crate::engine::compat::fixed::FixedList::<Members, MAX_EXTRA_GROUPS>::new(Members::pair_group()); 2];
    let mut alias = [0usize; 2];
    // rc.6 mid-session cycle change (operator requirement 2026-11-08, inbox 19:21Z): a detached group with a member the
    // role's cycle no longer has leaves. Nothing native exists for a planned, held or failed group: it is Released here.
    // A Prefetched one returns its keys on the entry tick (`leave_group`). A Built one stays parked until Training's pause
    // opens (operator decision 23:42Z, `body_bank::leaves_at_pause`, `pause_opening`): it keeps its bodies in the budget
    // and serves its still-wanted members meanwhile, and new members are planned beside it into free slots, or it goes
    // now when they need its room (operator rule 2026-11-09 13:21Z, below). The window itself (on screen) stays until it
    // is switched away.
    let wanted = [cycle_wanted(menu, 0), cycle_wanted(menu, 1)];
    for role in 0..2 {
        let mut state = BANKS[role].lock();
        if let Some(bank) = state.as_mut().filter(|bank| bank.phase == Phase::Ready && !bank.departed) {
            windows[role] = Some(*bank.members);
            alias[role] = bank.alias;
            let owner = bank.owner;
            for (index, group) in bank.groups.iter_mut().enumerate() {
                if group.state == GroupState::Released || group.leaving
                    || !crate::engine::swap::body_bank::group_left_cycle(*group.members, &wanted[role]) { continue; }
                let before = group.state;
                if matches!(before, GroupState::Planned | GroupState::Held(_) | GroupState::Failed) {
                    group.state = GroupState::Released;
                } else {
                    group.leaving = true;
                }
                trace!("[GroupSwap] left cycle owner={} group={} kinds={:x?} state={:?} {}", owner, index,
                    &group.members.kinds()[..group.members.count()], before,
                    if !group.leaving { "released (nothing native)" }
                    else if crate::engine::swap::body_bank::leaves_at_pause(before) { "parked; the next pause frees it" }
                    else { "leaving; the entry tick releases it" });
            }
            // rc.7 target-first: the full window's neighbours are this one-member window's first group, within the
            // budget the window would have charged them (the other role's bodies as `proposed` counts them).
            let word = NEIGHBOURS[role].swap(u64::MAX, Ordering::AcqRel);
            if let Some(seed) = crate::engine::swap::body_bank::unpack_neighbours(word, *bank.members) {
                let other = usize::from(EXTRAS[1 - role].load(Ordering::Acquire))
                    + usize::from(GROUP_BODIES[1 - role].load(Ordering::Acquire));
                let held = bank.groups.iter().any(|g| g.state != GroupState::Released
                    && seed.kinds()[..seed.count()].iter().any(|kind| g.members.index_of(*kind).is_some()));
                if !held && bank.groups.next_slot().is_some() && seed.count() <= pool_free
                    && seed.count() + other <= group_pool(settings) {
                    let slot = bank.groups.plan(seed, None);
                    trace!("[GroupSwap] plan owner={} group={} kinds={:x?} special={:?} bodies={}: the target-first window's neighbours, first",
                        owner, slot, &seed.kinds()[..seed.count()], seed.special(), seed.count());
                }
            }
            for group in bank.groups.iter().filter(|g| g.state.counts_bodies()) {
                existing[role].push(*group.members);
            }
        }
    }
    // A lane's Ice Climbers entry is a group candidate only when the role's
    // original is not Ice Climbers (whose slot-0 form may differ from the
    // cycle's, `plan_budget`) and the numbered slot's configured form matches the
    // switch that admits it: the normal pair for a pair group (`pair_groups`),
    // Solo Popo for a Solo-led ordinary group (`solo_popo_groups`); the pair
    // group wins when both are on.
    let upcoming: [_; 2] = std::array::from_fn(|role| {
        let mut kinds = crate::engine::compat::fixed::FixedList::<u32, { crate::runtime::roster::cycle_preload::MAX_TARGETS }>::new(0);
        let logical_role = if role == 0 { super::Role::Player } else { super::Role::Cpu };
        for kind in lanes[2 + role].iter().enumerate().filter_map(|(i, t)| t.map(|t| (i, t)))
            // Configured targets stay canonical, except a lane Mii under the Mii-group policy.
            .filter(|(_, t)| t.configuration.is_none() || (special.mii && (0x48..=0x4a).contains(&t.kind)))
            .filter(|(i, t)| t.kind != POPO
                || (super::original_kind(logical_role) != Some(POPO) && {
                    let form = super::requested_forms_at(menu, logical_role, POPO, Some(*i));
                    if special.pair { form == Forms::starting(POPO) }
                    else { policy.solo_popo_groups && form == Forms::solo_popo() }
                }))
            .map(|(_, t)| t.kind) { kinds.push(kind); }
        kinds
    });
    let budget = GroupBudget { ready_fighters: usize::from(settings.ready_fighters), pool_free, special };
    let planned = plan_extra_groups_with(policy, windows, [&existing[0], &existing[1]], [&upcoming[0], &upcoming[1]],
        originals, budget);
    let mut bodies = 0usize;
    for role in 0..2 {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.phase == Phase::Ready && !bank.departed) else { continue; };
        for members in planned[role].iter().flatten() {
            if bank.groups.next_slot().is_none() { break; }
            if bank.groups.iter().any(|g| g.state != GroupState::Released && g.members.index_of(members.kinds()[0]).is_some()) { continue; }
            let identity = if members.carries_identity() {
                match mii_identity_for(bank, &lanes[2 + role], members.kinds()[0]) {
                    Some(identity) => Some(identity),
                    None => {
                        trace!("[GroupSwap] plan owner={} skipped Mii kind={:#x} reason=no stageable identity", bank.owner, members.kinds()[0]);
                        continue;
                    }
                }
            } else { None };
            let slot = bank.groups.plan(*members, identity);
            trace!("[GroupSwap] plan owner={} group={} kinds={:x?} special={:?} bodies={} ready_fighters={} pool={}/{} reserve={} hats={} alias={:#x} identity={:?}",
                bank.owner, slot, &members.kinds()[..members.count()], members.special(), members.count(),
                settings.ready_fighters, live, capacity, POOL_RESERVE, hats_wanted, alias[role],
                identity.map(|(i, _)| (i.index, i.tokens)));
        }
        let counts = count_group_members(bank.groups.iter().map(|g| (shown_members(g, &wanted[role]), g.state)));
        targets[role] = counts.planned;
        let role_bodies: usize = bank.groups.iter().filter(|g| g.state.counts_bodies()).map(|g| g.members.count()).sum();
        GROUP_BODIES[role].store(role_bodies.min(usize::from(u8::MAX)) as u8, Ordering::Release);
        bodies += role_bodies;
    }
    // Operator rule 2026-11-09 (inbox 13:21Z; it replaces the pause wait of the 23:42Z decision): the new members load
    // at once. The parked groups keep their bodies and slots until the pause unless the new members need their room:
    // then the fewest of them are marked to go now (`release_for_new_members`), the entry tick frees them (`leaves_now`)
    // and the members they make room for show as loading; the pass after the release plans them. A leaving group that
    // is not parked goes on the entry tick anyway and is neither kept nor parked here.
    let mut kept = [crate::engine::compat::fixed::FixedList::<Members, MAX_EXTRA_GROUPS>::new(Members::pair_group()); 2];
    let mut parked = [crate::engine::compat::fixed::FixedList::<(Members, u8), MAX_EXTRA_GROUPS>::new((Members::pair_group(), 0)); 2];
    let mut parked_slots = [crate::engine::compat::fixed::FixedList::<usize, MAX_EXTRA_GROUPS>::new(0); 2];
    let mut held = [crate::engine::compat::fixed::FixedList::<u32, { MAX_EXTRA_GROUPS * NATIVE_MEMBERS }>::new(0); 2];
    for role in 0..2 {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.phase == Phase::Ready && !bank.departed) else { continue; };
        for (index, group) in bank.groups.iter_mut().enumerate().filter(|(_, g)| g.state != GroupState::Released) {
            if group.leaving && leaves_at_pause(group.state) {
                group.parked_kept = kept_members(*group.members, &wanted[role]);
                parked[role].push((*group.members, group.parked_kept));
                parked_slots[role].push(index);
                continue;
            }
            for kind in &group.members.kinds()[..group.members.count()] { held[role].push(*kind); }
            if !group.leaving && group.state.counts_bodies() { kept[role].push(*group.members); }
        }
    }
    let release = release_for_new_members([&kept[0], &kept[1]], [&parked[0], &parked[1]], |role, kind| held[role].contains(&kind),
        |existing, freed| plan_extra_groups_with(policy, windows, existing, [&upcoming[0], &upcoming[1]], originals,
            GroupBudget { pool_free: pool_free.saturating_add(freed), ..budget }));
    for role in 0..2 {
        if let Some(bank) = BANKS[role].lock().as_mut().filter(|bank| bank.phase == Phase::Ready && !bank.departed) {
            let owner = bank.owner;
            for (bit, (slot, (members, kept))) in parked_slots[role].iter().zip(parked[role].iter()).enumerate() {
                if release.groups[role] & (1 << bit) == 0 { continue; }
                let Some(group) = bank.groups.get_mut(*slot)
                    .filter(|g| g.leaving && leaves_at_pause(g.state) && !g.release_now && *g.members == *members) else { continue; };
                group.release_now = true;
                trace!("[GroupSwap] release now owner={} group={} kinds={:x?} kept={} reason=the new members need its room; the entry tick frees it",
                    owner, slot, &members.kinds()[..members.count()], kept);
            }
        }
        if RELEASE_MEMBERS[role].swap(release.members[role], Ordering::AcqRel) != release.members[role] {
            trace!("[GroupSwap] loading after a release role={} members={} parked={:#b}", role, release.members[role], release.groups[role]);
        }
        targets[role] = targets[role].saturating_add(release.members[role]);
    }
    // Operator rule 2026-11-09 17:13Z: once the watch settles, the planner decides a held group's capacity release in
    // this pass, before `cycle_preload::on_frame` publishes the notice, so a release in progress shows "Loading".
    if after.settled() {
        for role in 0..2 {
            let held = BANKS[role].lock().as_ref().filter(|bank| bank.phase == Phase::Ready && !bank.departed)
                .and_then(|bank| bank.groups.iter().enumerate().find(|(_, g)| matches!(g.state, GroupState::Held(_)))
                    .map(|(group, g)| (bank.owner, group, *g.members, g.state)));
            let Some((owner, group, members, state)) = held else { continue; };
            let refused = state == GroupState::Held(Hold::Memory) && matches!(
                super::process_memory::admit(memory::group_cost(&members.kinds()[..members.count()])).0.admission,
                Admission::Refuse);
            if let Some(reason) = still_short(state, refused, pool_free, members.count()) {
                release_for_capacity(owner, group, reason);
            }
        }
    }
    bodies
}

/// Whether a parked group of `role` is marked to go now (a release in progress, `room_coming`).
fn releasing(role: usize) -> bool {
    BANKS[role].lock().as_ref().is_some_and(|bank| !bank.departed && bank.groups.iter().any(|g| g.leaving && g.release_now))
}

/// The members of `group` the notice and the planner count: a group that left the cycle (parked until the pause, or a
/// prefetch returning its keys) only those its role's cycle still has (`body_bank::kept_members`).
fn shown_members(group: &Group, wanted: &[u32]) -> u8 {
    if group.leaving { kept_members(*group.members, wanted) } else { group.members.count() as u8 }
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
    let wanted = [cycle_wanted(&menu, 0), cycle_wanted(&menu, 1)];
    // Operator rule 2026-11-09 16:06Z and 17:13Z: a prefetch held for memory is loading while a free's room is arriving
    // or a release decided for it is in progress; "waiting for memory" means nothing more can be freed.
    let settled = RELEASE_WATCH.lock().settled();
    let coming = room_coming(settled, (0..2).any(releasing));
    for (role, state) in BANKS.iter().enumerate() {
        let planned = PLANNED_TARGETS[role].load(Ordering::Acquire);
        let releasing = RELEASE_MEMBERS[role].load(Ordering::Acquire);
        let counts = match state.lock().as_ref()
            .filter(|bank| bank.phase != Phase::Retired && !bank.departed) {
            Some(bank) => {
                let (kinds, count) = bank.members.logical_kinds();
                let active = bank.observed.map_or(bank.members.kinds()[usize::from(bank.target)], |(_, kind)| kind);
                let extra = reusable_extras(&kinds[..count], active, originals[role]);
                // rc.6 item 3: a prefetched group a background build still owes is loading, not yet instant.
                // A parked group (left the cycle, freed at the next pause) counts the members its cycle keeps, which
                // it still serves warm (rc6-hot-xps18), and the new members a parked group freed now makes room for
                // show as loading (operator rule 2026-11-09 13:21Z).
                let groups = count_group_members(bank.groups.iter().map(|g| (shown_members(g, &wanted[role]),
                    crate::engine::swap::body_bank::shown_state(arriving_room(g.state, coming), background_owed(bank, g, Some(active))))))
                    .with_release(releasing);
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
        result.held = result.held.saturating_add(counts.held);
        result.hold = crate::engine::swap::body_bank::merge_hold(result.hold, counts.hold);
        // The Waiting text names the role whose own swap its unstarted fighters wait for.
        result.swap |= counts.swap_owed(role);
    }
    result
}

/// A kind held by any of this role's groups (window or extra): helper look-ahead
/// must not duplicate it.
pub(crate) fn group_member(role: usize, kind: u32) -> bool {
    role < BANKS.len() && BANKS[role].lock().as_ref().is_some_and(|bank|
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
    let state = BANKS[role(owner)?].lock();
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
    let state = BANKS[role].lock();
    let Some(bank) = state.as_ref().filter(|b| b.entry == entry && b.owner == owner) else { return false; };
    bank.exchange.is_some_and(|exchange| ready_group_owner(entry as *const u8, bank)
        .is_some_and(|now| now.epoch == epoch && now.descriptor == *after && exchange.rebaseline(before, now)))
}

pub(crate) fn helper_budget(slots: u8) -> u8 { slots.saturating_sub(RESERVED.load(Ordering::Acquire)) }

/// rc.6 window admission: per role, what the supplements its last window plan refused for memory need
/// (`memory::window_waiting`). Created entries leave it ([`cycle_waiting`]).
static WINDOW_WAITING: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];

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
    let full = members_for(&settings, target, &lane[..count], budget);
    // rc.7 target-first (TASK-rc7.md item 2; the console play test waited 7.6-7.9 s for a three-body window): the Reload
    // builds the picked fighter alone (`body_bank::target_first`). The members the full window would have held are planned
    // as the role's first background group once it is Ready (`NEIGHBOURS`, `plan_groups`), admitted on the cost the window
    // charged them, then the rest of the lane as usual. No supplement is charged here, so the window waits for no memory.
    if let Some(members) = target_first(target, groups_enabled(&settings), group_pool(&settings)) {
        WINDOW_WAITING[role].store(0, Ordering::Release);
        let seed = full.and_then(|full| neighbour_group(full, target.kinds[0], group_policy(&settings),
            excluded_original(&settings, &menu, role)));
        NEIGHBOURS[role].store(pack_neighbours(target.kinds[0], seed), Ordering::Release);
        trace!("[BodyBank] target first owner={} target={:#x} lane={:x?} neighbours={:x?}; they build as groups",
            owner, target.kinds[0], &lane[..count], seed.map(|m| m.kinds()));
        return Some(members);
    }
    NEIGHBOURS[role].store(u64::MAX, Ordering::Release);
    let Some(members) = full else {
        WINDOW_WAITING[role].store(0, Ordering::Release);
        return None;
    };
    // rc.6 window admission (primary 2026-11-09 13:28Z; Eden final-e39-xps-hot hs08b: the window `[42, 44, 1c]` was
    // planned at 27.6 MB of headroom, `prepare_resources` acquired both supplements and the game's page commit aborted in
    // the build). The members beyond the target's own form are admitted here, in the planner's order, against the
    // process headroom like a group prefetch (`memory::window_extras`); the planner runs again with the count admitted,
    // so `prepare_resources` acquires only those. The target is the visible request and is never refused.
    let mut supplements = FixedList::<u32, NATIVE_MEMBERS>::new(0);
    for kind in lane[..count].iter().copied() {
        if members.kinds()[..members.count()].contains(&kind)
            && !target.kinds[..target.count as usize].contains(&kind) && !supplements.contains(&kind) {
            supplements.push(kind);
        }
    }
    let (admitted, stopped) = super::process_memory::admit_window(&supplements);
    WINDOW_WAITING[role].store(memory::window_waiting(&supplements, admitted), Ordering::Release);
    let Some(decision) = stopped else { return Some(members); };
    // The pair-lead window's Nana is the target's form, not a supplement: the body budget counts it.
    let admitted_budget = admitted + (members.count() - 1).saturating_sub(supplements.len());
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ProcessMemory] window owner={} target={:#x} members={:x?} admitted={} of supplements={:x?} admission={:?} headroom={:?} outstanding={} need={}; the rest waits for a later window",
        owner, target.kinds[0], &members.kinds()[..members.count()], admitted, &*supplements, decision.admission,
        decision.headroom, decision.outstanding, decision.need));
    members_for(&settings, target, &lane[..count], admitted_budget)
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
    plan_in(entry, owner, descriptor, upcoming, false)
}

/// [`plan`] for an adapter that kept its live Mii group's switch-time publication (rc.6 warm exit, operator decision
/// 2026-11-08): an ordinary target in the window that Mii group displaced is a warm reuse. The exchange writes that
/// window's saved `R` words, entry bytes and D mirror back (`perform_swap`); the adapter puts its publication back in
/// the settle tick.
pub(super) unsafe fn plan_mii_exit(entry: *const u8, owner: u32, descriptor: [u8; 0x70], upcoming: &[u32]) -> Plan {
    plan_in(entry, owner, descriptor, upcoming, true)
}

unsafe fn plan_in(entry: *const u8, owner: u32, descriptor: [u8; 0x70], upcoming: &[u32], mii_exit: bool) -> Plan {
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
            // (`group_identity_matches` above). Every other request is byte-exact, except the warm exit
            // (`plan_mii_exit`): the displaced window's saved entry bytes come back with it, or with any other Built
            // ordinary group the owner's ordinary baseline (`mii_exit_saved`).
            let exit = (mii_exit && in_group && !(0x48..=0x4a).contains(&target)).then(|| mii_exit_saved(owner, target, forms)).flatten();
            let matches = live.is_some_and(|current| current == *physical.bytes()
                || (in_group && (0x48..=0x4a).contains(&target) && mii_reuse_bytes_match(&current, &descriptor))
                || exit.is_some_and(|saved| mii_exit_bytes_match(&current, physical.bytes(), &saved)));
            if matches {
                if exit.is_some() {
                    trace!("[GroupSwap] warm exit owner={} kind={:#x}: a Built ordinary group, out of the live Mii group", owner, target);
                }
                result.descriptor = *physical.bytes(); result.members = Some(members); result.reuse = true;
                result.cells = members.kinds(); result.active = members.index_of(target).unwrap_or(0);
                return result;
            }
            // A warm member exists but the request's non-group bytes differ from
            // the live entry (an Original request carries the origin descriptor):
            // the request takes the Reload path below; name the bytes for triage.
            if let Some(current) = fields::read::<[u8; 0x70]>(entry as usize, 0) {
                let differing = DebugIter((0..0x70).filter(|i| current[*i] != physical.bytes()[*i]));
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
    for group in bank.groups.iter() {
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
    BANKS[role].lock().as_ref().map_or_else(Occupancy::default, |bank| occupancy_of(bank, role, copy))
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

/// [`release_group_keys`] for one group, with that group's own words in `r` while its keys go
/// (MII-GROUPS-DESIGN §3.2: the release selector `0x17e8410` keys on `R+0x58` like the request
/// did; `with_words` puts the window's words back after): a Mii group's identity, and beside a live
/// Mii window any other group's ordinary baseline (X6, `group_words`). Every per-group key return outside
/// a build goes through here: the drain before a Kirby or pair-rule target, the live-Kirby return
/// and a failed prefetch. Never called under the bank lock.
unsafe fn release_group_keys_of(base: usize, role: usize, r: usize, group: usize, occupancy: &Occupancy,
    owner: u32, reason: &str) -> (usize, usize) {
    let mut outcome = (0usize, 0usize);
    with_words(r, group_words_of(role, group, r), || {
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
    let base = BASE.load(Ordering::Acquire);
    BANKS[role].lock().as_ref().is_some_and(|bank|
        bank.swap.is_none() && bank.prefetch.is_none() && bank.object_fetch.is_none() && bank.preparing.is_none() && !bank.departed
            && bank.groups.iter().enumerate().any(|(i, g)| g.state.reachable() && group_serves(*g.members, kind, forms).is_some()
                // Inbox 02:00Z: a pool still retiring is neither live nor empty.
                && attachable(bank.retiring.filter(|r| r.group == i).map(|r| r.stage))
                // Inbox 00:18Z: a PT group whose attach rebuilds its articles holds its object kind first; without it
                // (a drain, a refused re-acquire) the request takes the Reload path rather than read unloaded files.
                // SAFETY: `detached_pool` reads the group's own saved holder, proven by `valid_special_object`.
                && object_ready(*g.members, unsafe { detached_pool(base, owner, g) }, g.object_held)))
}

/// The article count of a detached group's saved holder (`holder+0x10`); `u64::MAX` when it keeps none to rebuild.
unsafe fn detached_pool(base: usize, owner: u32, g: &Group) -> u64 {
    g.saved.map(|w| w.records[0]).filter(|holder| *holder != 0 && valid_special_object(base, *holder, owner))
        .and_then(|holder| fields::read::<u64>(holder, 0x10)).unwrap_or(u64::MAX)
}

/// The detached Pokémon Trainer group that must take its object kind again before its next attach
/// (`body_bank::object_rearm_due`): none while a prefetch, a construction, a background build or another
/// re-acquisition is in flight.
unsafe fn rearm_candidate(base: usize, bank: &Bank) -> Option<usize> {
    if bank.prefetch.is_some() || bank.construction.is_some() || bank.background.is_some() || bank.object_fetch.is_some() {
        return None;
    }
    let owner = bank.owner;
    bank.groups.iter().position(|g| !g.object_refused && !g.leaving
        && object_rearm_due(g.state, *g.members, detached_pool(base, owner, g), g.object_held))
}

/// [`rearm_candidate`] under the D-KIRBY1 gate: the group whose object kind the live member admits
/// (`body_bank::object_rearm_admitted`: the trainer's object kind under a live Kirby; primary 2026-11-09 05:38Z).
unsafe fn kirby_rearm(base: usize, bank: &Bank, live: Option<u32>) -> Option<usize> {
    let group = rearm_candidate(base, bank)?;
    let kind = bank.groups[group].members.object_kind()?;
    crate::engine::swap::body_bank::object_rearm_admitted(live, kind).ok().map(|()| group)
}

/// Inbox 02:00Z: the next stage of the PT pool the last exit left retiring (`RetireStage`), on this tick. The group
/// lifecycle takes INSTALL for the first later stage (a press meanwhile waits for it, as for a prefetch) and gives it
/// back after the last; a teardown, a leave or a rollback passes `blocking` and runs the stage with whatever INSTALL
/// state it has. The pool must still be the group's valid saved holder. No bank lock is held across the native calls.
unsafe fn retire_step(base: usize, role: usize, owner: u32, r: Retiring, why: &str) {
    retire_stage(base, role, owner, r, why, false);
}

unsafe fn retire_stage(base: usize, role: usize, owner: u32, r: Retiring, why: &str, blocking: bool) {
    let Some(stage) = r.stage.next() else {
        if let Some(bank) = BANKS[role].lock().as_mut() { bank.retiring = None; }
        return;
    };
    let current = BANKS[role].lock().as_ref().and_then(|bank| bank.groups.get(r.group)).and_then(|g| g.saved).map(|w| w.records[0]);
    if current != Some(r.holder) || !valid_special_object(base, r.holder, owner) {
        if let Some(bank) = BANKS[role].lock().as_mut() { bank.retiring = None; }
        if let Some(token) = r.token { super::finish_install(token, "articles retire: holder gone"); }
        trace!("[GroupSwap] articles retire UNEXPECTED owner={} group={} holder={:#x} stage={:?} saved={:x?}; dropped",
            owner, r.group, r.holder, stage, current);
        return;
    }
    let token = match r.token {
        Some(token) => Some(token),
        None if blocking => None,
        None => {
            if super::INSTALL.active() { return; }
            let Some(token) = super::INSTALL.try_begin(false) else { return; };
            Some(token)
        }
    };
    let started = now_us();
    let (pools, articles, gone) = match stage {
        RetireStage::SubPools => {
            let (pools, articles) = articles::release_sub_pools(base, r.holder);
            (pools, articles, 0)
        }
        RetireStage::Destroy => (0, 0, articles::destroy_saved(base, owner, r.holder, why)),
        RetireStage::Deactivate => (0, 0, 0),
    };
    let us = now_us().saturating_sub(started);
    let done = stage.next().is_none();
    if let Some(bank) = BANKS[role].lock().as_mut() {
        bank.retiring = (!done).then_some(Retiring { stage, token, ..r });
    }
    let released = match token {
        Some(token) if done => super::finish_install(token, "articles retired"),
        _ => false,
    };
    trace!("[GroupSwap] articles retire stage owner={} group={} holder={:#x} stage={:?} us={} sub_pools={} sub_articles={} gone={} blocking={} INSTALL_released={}",
        owner, r.group, r.holder, stage, us, pools, articles, gone, blocking, released);
}

/// Inbox 02:42Z: the detached PT group whose retired pool is built next on the task worker
/// (`special_articles::prepare_due`): none while anything else is in flight on the bank.
unsafe fn prepare_candidate(base: usize, bank: &Bank) -> Option<usize> {
    if !PREPARE_ENABLED.load(Ordering::Acquire) || bank.prefetch.is_some() || bank.construction.is_some()
        || bank.background.is_some() || bank.object_fetch.is_some() || bank.retiring.is_some() || bank.preparing.is_some() {
        return None;
    }
    let owner = bank.owner;
    bank.groups.iter().position(|g| !g.leaving && !g.prepare_refused
        && prepare_due(g.members.holder_step() == Some(HolderStep::Trainer), g.state == GroupState::Built,
            detached_pool(base, owner, g), g.object_held, false))
}

/// Inbox 02:42Z: a prepared pool build finished on the worker (or a teardown waited for it): the task freed, INSTALL
/// back; with the pool filled the object kind goes back, as after an attach's rebuild. An empty pool keeps the kind for
/// the attach's own rebuild and is not prepared again. No bank lock is held across the native calls.
unsafe fn prepare_settled(base: usize, role: usize, owner: u32, p: Preparing, why: &str) {
    let us = now_us().saturating_sub(p.since);
    free_tasks(base, &[p.task]);
    let pool = if valid_special_object(base, p.holder, owner) { fields::read::<u64>(p.holder, 0x10).unwrap_or(0) } else { 0 };
    if let Some(bank) = BANKS[role].lock().as_mut() {
        bank.preparing = None;
        if pool == 0 {
            if let Some(g) = bank.groups.get_mut(p.group) { g.prepare_refused = true; }
        }
    }
    let released = super::finish_install(p.token, "articles prepared");
    trace!("[GroupSwap] articles prepared owner={} group={} holder={:#x} pool=0->{} us={} why={} INSTALL_released={}{}",
        owner, p.group, p.holder, pool, us, why, released, if pool == 0 { "; NOT filled, the attach rebuilds it" } else { "" });
    if pool != 0 {
        return_object_key(base, role, owner, p.group, "articles prepared on the worker");
    }
}

/// A prepared pool build in flight finishes first (`TASK_WAIT`, as `drain_background` waits): before a teardown or a
/// leave destroys its holder.
unsafe fn finish_preparation(base: usize, role: usize, owner: u32, why: &str) {
    use construction_task as task;
    let Some(p) = BANKS[role].lock().as_ref().and_then(|bank| bank.preparing) else { return; };
    if fields::read::<i32>(p.task, task::PENDING).unwrap_or(0) > 0 {
        let wait: unsafe extern "C" fn(u32, usize) = std::mem::transmute(base + TASK_WAIT);
        let queue = fields::read::<u32>(p.task, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
        if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
            wait(handle, p.task + task::PENDING);
        }
    }
    prepare_settled(base, role, owner, p, why);
}

/// Every stage still due (`special_articles::remaining`), now, in order: before a teardown, a leave or a rollback
/// reads the pool, and before another exit starts its own.
unsafe fn finish_retirement(base: usize, role: usize, owner: u32, why: &str) {
    for _ in 0..3 {
        let Some(r) = BANKS[role].lock().as_ref().and_then(|bank| bank.retiring) else { return; };
        retire_stage(base, role, owner, r, why, true);
    }
}

/// Inbox 00:18Z: the object kind a detached Pokémon Trainer group took again (`group_lifecycle`) goes back once its
/// attach rebuilt the article pool, or at a drain, so no object token outlives the detached time (IC-PAIR-STALL). The
/// slot keeps the flag across the exchange (the group that attached left it). No bank lock is held across the release.
unsafe fn return_object_key(base: usize, role: usize, owner: u32, group: usize, why: &str) {
    let kind = PT_OBJECT_KIND;
    {
        let mut state = BANKS[role].lock();
        let Some(g) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)) else { return; };
        let held = std::mem::replace(&mut g.object_held, false);
        if !held { return; }
    }
    let alias = BANKS[role].lock().as_ref().map_or(0, |bank| bank.alias);
    let receipt = r_handle::release(base, ledger_role(role), Some(group), Some(&[kind][..]), &Occupancy::default(), why);
    trace!("[GroupSwap] keys returned owner={} group={} object_kind={:#x} released={} kept={} map={} reason={}",
        owner, group, kind, receipt.released, receipt.kept.len(), fields::read::<u64>(alias, 0x18).map_or(-1, |m| m as i64), why);
}

/// The entry bytes a warm exit from this role's live Mii group writes back for `kind`: the reachable group `switch`
/// would take for it is Built and ordinary. The window the Mii displaced gives back what it saved; any other Built
/// ordinary group the owner's ordinary baseline (inbox 21:27Z, `Bank::ordinary_identity`). `None` for a Prefetched
/// group (it would build under the Mii's `R`) and when no ordinary window was ever displaced.
fn mii_exit_saved(owner: u32, kind: u32, forms: Forms) -> Option<MiiEntryBytes> {
    let state = BANKS[role(owner)?].lock();
    let bank = state.as_ref()?;
    if bank.identity.is_none() { return None; }
    let group = bank.groups.iter().find(|g| g.state.reachable() && group_serves(*g.members, kind, forms).is_some())?;
    if group.state != GroupState::Built || group.identity.is_some() { return None; }
    group.saved.as_ref()?.identity.or(bank.ordinary_identity).map(|(_, saved)| saved.entry)
}

/// The reachable Mii group holding `kind` carries exactly the identity this
/// request's staged descriptor names.
fn group_identity_matches(owner: u32, kind: u32, descriptor: &[u8; 0x70]) -> bool {
    let Some(role) = role(owner) else { return false; };
    let Some(wanted) = MiiIdentity::from_descriptor(descriptor) else { return false; };
    BANKS[role].lock().as_ref().is_some_and(|bank|
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
    /// rc.6 item 3 (SPECIAL-GROUPS §1 (c)): the Mii set `R` held while this group was the window (`MiiR`), taken at
    /// every detach and put back at its attach, so each window keeps its own faces and resource object.
    mii_r: Option<MiiR>,
    /// rc.6 item 3: `D+0x3a0..+0x3ac`, which body init writes per kind (`0x60abc0`; a Mii writes `0x69` at `+0x3a8`,
    /// `rc6-hot-xps14`), taken at every detach and put back at its attach like `mii_r`. `None`: never taken.
    info_3a0: Option<[u32; 3]>,
    index: u8,
    previous: u8,
    old: u8,
}

unsafe fn read_info_3a0(entry: usize) -> Option<[u32; 3]> {
    let info = entry_info(entry)?;
    Some(std::array::from_fn(|i| fields::read_unchecked::<u32>(info, 0x3a0 + i * 4)))
}

unsafe fn write_info_3a0(entry: usize, words: [u32; 3]) {
    if let Some(info) = entry_info(entry) {
        for (i, word) in words.iter().enumerate() { fields::write_unchecked(info, 0x3a0 + i * 4, *word); }
    }
}

/// What a Mii window owns in `R` besides its words: the face handles `R+0x90/+0x94` (`0x67ead0` builds them; the HUD
/// portrait reads them, `0x341c6f8`) and the Mii resource object `R+0x98/+0xa0` (a pointer and its shared control
/// block, built by body init at `0x607c94-0x607d20`). Zero for a window that is not a Mii.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MiiR { faces: [u32; 2], object: [usize; 2] }

/// `R`'s Mii set, zeroed in `R` (zero is native's own state after a Reload, `0x6502c0-0x650310`).
unsafe fn take_mii_r(r: usize) -> MiiR {
    let set = MiiR { faces: [fields::read_unchecked(r, 0x90), fields::read_unchecked(r, 0x94)],
        object: [fields::read_unchecked(r, 0x98), fields::read_unchecked(r, 0xa0)] };
    put_mii_r(r, MiiR::default());
    set
}

unsafe fn put_mii_r(r: usize, set: MiiR) {
    fields::write_unchecked(r, 0x90, set.faces[0]);
    fields::write_unchecked(r, 0x94, set.faces[1]);
    fields::write_unchecked(r, 0x98, set.object[0]);
    fields::write_unchecked(r, 0xa0, set.object[1]);
}

/// The face holder `0x6502d0` releases them through (`*(main+0x5338f68)`).
const FACE_HOLDER: usize = 0x5338f68;
const FACE_RELEASE: usize = 0x371e010;
/// libc++ `__shared_weak_count::__release_weak` (`0x607d54`).
const RELEASE_WEAK: usize = 0x39c2670;

/// Release a Mii set no `R` holds any more: each face handle as the Reload releases it (`0x6502c8-0x65030c`: a handle
/// outside {-1, 0, 1} through `0x371e010(holder, handle)`), the object's control block as body init releases the
/// previous one (`0x607d24-0x607d54`: decrement `+8`; at the last owner vtable `+0x10`, then `__release_weak`).
unsafe fn release_mii_r(base: usize, set: MiiR) {
    let holder = fields::read::<usize>(base, FACE_HOLDER).unwrap_or(0);
    let release_face: unsafe extern "C" fn(usize, u32) = std::mem::transmute(base + FACE_RELEASE);
    for handle in set.faces {
        if handle != 0 && holder != 0 && handle.wrapping_add(1) >= 3 { release_face(holder, handle); }
    }
    let block = set.object[1];
    if block != 0 && fields::object(block) {
        let owners = &*((block + 8) as *const std::sync::atomic::AtomicI64);
        if owners.fetch_sub(1, Ordering::AcqRel) == 0 {
            if let Some(dispose) = fields::child(block, 0).and_then(|vtable| fields::read::<usize>(vtable, 0x10)) {
                let dispose: unsafe extern "C" fn(usize) = std::mem::transmute(dispose);
                dispose(block);
            }
            let release_weak: unsafe extern "C" fn(usize) = std::mem::transmute(base + RELEASE_WEAK);
            release_weak(block);
        }
    }
}

struct Group {
    // Immutable heap storage lent to the table hooks while this group is in the window.
    members: Box<Members>,
    data: GroupData,
}

struct GroupData {
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
    /// rc.6 item 3: a background build of this group was refused (logged once); it stays Prefetched and its first
    /// switch builds it, as before.
    background_refused: bool,
    /// rc.6 mid-session cycle change: a member left the role's cycle (`body_bank::group_left_cycle`). A Built group is
    /// parked and released when the pause opens (`pause_opening`), any other on the entry tick (`leave_group`); it then
    /// becomes Released.
    leaving: bool,
    /// Operator rule 2026-11-09 (inbox 13:21Z): a parked group the new members need the room of (the planner's
    /// `release_for_new_members`, or a refused prefetch's `release_for_capacity`); the entry tick frees it now.
    release_now: bool,
    /// The members of a parked group its role's cycle still has (`body_bank::kept_members`), as of the planner's last pass.
    parked_kept: u8,
    /// Inbox 00:18Z: this slot's Pokémon Trainer group holds its object kind `0x72` again (re-acquired while detached,
    /// returned right after its attach's rebuild, `return_object_key`).
    object_held: bool,
    /// Its re-acquisition was refused by the ledger (logged once; not retried; the group then takes the Reload path).
    object_refused: bool,
    /// Inbox 02:42Z: its prepared pool build left the pool empty or was not queued (logged; not retried; its attach
    /// rebuilds the pool itself).
    prepare_refused: bool,
}

impl GroupData {
    fn planned(identity: Option<(MiiIdentity, Configuration)>) -> Self {
        let (identity, configuration) = identity.map_or((None, None), |(i, c)| (Some(i), Some(c)));
        Self { state: GroupState::Planned, saved: None,
            bodies: [Body::default(); NATIVE_MEMBERS], since_us: 0, busy_logged: false, kirby_logged: false,
            memory_logged: false, prefetch_committed: None,
            identity, configuration, charinfo_published: false, publish: PublishAttempts::new(), background_refused: false,
            leaving: false, release_now: false, parked_kept: 0, object_held: false, object_refused: false, prepare_refused: false }
    }
}

impl std::ops::Deref for Group {
    type Target = GroupData;
    fn deref(&self) -> &GroupData { &self.data }
}
impl std::ops::DerefMut for Group {
    fn deref_mut(&mut self) -> &mut GroupData { &mut self.data }
}

/// Exactly the existing native extra-group slots, allocated at boot. Clearing
/// a bank retires its records without freeing tables borrowed by native hooks.
struct Groups {
    slots: Box<[Group; MAX_EXTRA_GROUPS]>,
    len: usize,
}
impl Groups {
    fn new() -> Self {
        Self { slots: Box::new(std::array::from_fn(|_| Group {
            members: Box::new(Members::pair_group()), data: GroupData::planned(None),
        })), len: 0 }
    }
    fn clear(&mut self) { self.len = 0; }
    /// The slot the next planned group takes: a Released one (rc.6 mid-session cycle change), else the next free one.
    fn next_slot(&self) -> Option<usize> {
        crate::engine::swap::body_bank::reuse_slot(self.iter().map(|g| g.state))
            .or((self.len < MAX_EXTRA_GROUPS).then_some(self.len))
    }
    /// Returns the slot the group took. The caller holds BANKS and has proved `next_slot` is `Some`.
    fn plan(&mut self, members: Members, identity: Option<(MiiIdentity, Configuration)>) -> usize {
        let slot = self.next_slot().unwrap_or(self.len);
        let group = &mut self.slots[slot];
        *group.members = members;
        group.data = GroupData::planned(identity);
        if slot == self.len { self.len += 1; }
        slot
    }
}
impl std::ops::Deref for Groups {
    type Target = [Group];
    fn deref(&self) -> &[Group] { &self.slots[..self.len] }
}
impl std::ops::DerefMut for Groups {
    fn deref_mut(&mut self) -> &mut [Group] { &mut self.slots[..self.len] }
}

struct BankStorage { members: Box<Members>, groups: Groups }
impl BankStorage {
    fn new() -> Self { Self { members: Box::new(Members::pair_group()), groups: Groups::new() } }
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
    groups: Groups,
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
    /// The lane configuration of that identity, moving with it at every exchange (Eden `rc6-hot-xps16`: an exit through
    /// the baseline landed the Mii in another record, whose configuration was not its own, so `reachable_mii_group`
    /// no longer matched and the next switch to the Mii took the canonical Reload).
    configuration: Option<Configuration>,
    /// rc.6 warm exit to any Built group (inbox 21:27Z): the `R` words and entry identity the first ordinary window a Mii
    /// displaced left (owner-level besides the header kinds, `body_bank::baseline_words`).
    ordinary_identity: Option<([u32; RESOURCE_WORDS], EntryIdentity)>,
    /// Activation follows exact current body identities across detach/attach.
    /// Retired identities cannot grow the history or mark a later reused ID.
    activated: crate::model::body_activation::History<(usize, u32), { (MAX_EXTRA_GROUPS + 1) * NATIVE_MEMBERS }>,
    /// Phase B′: the construction tasks of the group whose words are in the (empty) window,
    /// settled on a later tick (`settle_construction`) or drained before a destroyer.
    construction: Option<Construction>,
    /// rc.6 item 3: a Prefetched group's bodies under construction beside the live window
    /// (`start_background`), settled on a later tick or drained before a destroyer.
    background: Option<Background>,
    /// P3-0: the tick-gap meter of the swap in flight (armed at the request, read at Ready).
    tick: TickGap,
    /// Advances at every held group's refused retry, so the retries rotate over the held groups.
    held_turn: usize,
    /// P-HOT B (§12): a window published by the scene's own configure (H1) carries the Training mode it was made in
    /// (`(mode, melee)`); that scene's own `scene_changed` leaves it, its ledger and its copy holdings alone.
    scene: Option<(u32, u32)>,
    /// Inbox 00:18Z: the object kind a detached Pokémon Trainer group takes again before its rebuild
    /// (`body_bank::object_rearm_due`), in flight: group, INSTALL token, start time.
    object_fetch: Option<(usize, super::InstallToken, u64)>,
    /// Inbox 02:00Z: the PT pool the last exit left retiring (`Retiring`).
    retiring: Option<Retiring>,
    /// Inbox 02:42Z: a detached PT group's pool being built on the task worker (`Preparing`).
    preparing: Option<Preparing>,
}

fn role(owner: u32) -> Option<usize> {
    match owner { 0 => Some(0), 1 | 4 => Some(1), _ => None }
}

fn now_us() -> u64 {
    crate::engine::preload::preload_metrics::micros(crate::engine::preload::preload_metrics::ticks(), crate::engine::preload::preload_metrics::tick_hz())
        .unwrap_or(0)
}

/// Diagnostics (Eden `rc6-hot-xps16`: native's pad code read a null `S[slot]` at `0x631984`): the pad manager's 16
/// per-slot controller words `S+0x00..+0x80` (`S = *(*(main+0x52b6fd8))`; `0x313b00` writes them). Values only.
unsafe fn pad_controllers(base: usize) -> [usize; 16] {
    let Some(state) = fields::chain(base, &[graph::PAD_MANAGER, 0]) else { return [usize::MAX; 16]; };
    std::array::from_fn(|i| fields::read::<usize>(state, i * 8).unwrap_or(usize::MAX))
}

/// A slot whose controller word was the live window's selected controller (`+0x4178` context `+0x58`) before a release
/// or background build and is 0 after it: written back, since native reads it without a null test (`0x631984`) until
/// the next select re-points it. Returns the slots written.
unsafe fn restore_live_pads(base: usize, entry: usize, before: &[usize; 16]) -> FixedList<usize, 16> {
    let mut written = FixedList::new(0);
    let Some(state) = fields::chain(base, &[graph::PAD_MANAGER, 0]) else { return written; };
    let live = fields::child(entry, 0x4178).and_then(|ctx| fields::read::<usize>(ctx, graph::CONTEXT_CONTROLLER)).unwrap_or(0);
    if live == 0 { return written; }
    for (slot, was) in before.iter().enumerate() {
        if *was == live && fields::read::<usize>(state, slot * 8) == Some(0) {
            fields::write_unchecked(state, slot * 8, live);
            written.push(slot);
        }
    }
    written
}

/// The pad slots whose controller word changed between two `pad_controllers` reads: `(slot, before, after)`.
fn pad_changes(before: &[usize; 16], after: &[usize; 16]) -> FixedList<(usize, usize, usize), 16> {
    let mut out = FixedList::new((0, 0, 0));
    for i in 0..16 { if before[i] != after[i] { out.push((i, before[i], after[i])); } }
    out
}

/// Inbox 22:17Z (no hitch while groups load in the background): a bank step that held the game thread this long or
/// longer is logged by name, so a frame gap in a background load names its step.
const SLOW_STEP_US: u64 = 4_000;

fn slow_step(owner: u32, step: &'static str, started: u64) -> u64 {
    let us = now_us().saturating_sub(started);
    if us >= SLOW_STEP_US { trace!("[GroupSwap] slow step owner={} step={} us={}", owner, step, us); }
    us
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
    let state = BANKS[role(owner)?].lock();
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
    let state = BANKS[role(owner)?].lock();
    state.as_ref().filter(|bank| bank.members.is_pair() && descriptor(entry, bank)).map(|bank| (owner, *bank.members))
}

/// Value-only logical identity for menu/preload planning. This does not qualify
/// a native write: each request still repeats the current native ready proof.
/// The Ready-time descriptor tolerates the same live behavior bytes as the
/// receipt, so a native CPU Behavior change keeps the active member's identity
/// for the next cycle instead of falling back to the group's first kind.
pub(crate) fn cached_kind(entry: usize, owner: u32, descriptor: &[u8; 0x70]) -> Option<u32> {
    // The mirror of the Ready bank (`READY`), never the bank's lock: the menu worker's re-plan reads this. It yields between rounds
    // so a game thread preempted mid-write can finish; a mirror still busy after every round is reported, not used silently.
    // A game hook keeps its frame short (a few rounds); the menu worker can afford to wait for the writer.
    let rounds = if crate::runtime::threads::on_plugin_thread() { READY_ROUNDS } else { 16 };
    let Some(words) = READY[role(owner)?].read_yielding(rounds) else {
        trace!("[BodyBank] ready mirror busy owner={} after {} rounds; no cached kind this plan", owner, rounds);
        return None;
    };
    if words[0] != 1 || words[1] != entry as u64 || words[2] != u64::from(owner) {
        return None;
    }
    let mut physical = [0u8; 0x70];
    for (bytes, word) in physical.chunks_exact_mut(8).zip(&words[4..]) {
        bytes.copy_from_slice(&word.to_le_bytes());
    }
    crate::engine::mii::identity::verify_descriptor(&physical, descriptor,
        crate::engine::mii::identity::ExpectedMutation::NativeBehavior).ok().map(|_| words[3] as u32)
}

/// True only after native settlement disproved the operation's identity.
/// No timeout, address alone, or partly constructed entry makes a refusal.
pub(super) fn failed(entry: *const u8, owner: u32) -> bool {
    let Some(role) = role(owner) else { return false; };
    BANKS[role].lock().as_ref().is_some_and(|bank|
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
    let mut state = BANKS[role].lock();
    // Reload is asynchronous. Even if the outgoing canonical descriptor still
    // looks ready, do not replace storage borrowed by an in-flight constructor.
    if state.as_ref().is_some_and(|bank| matches!(bank.phase, Phase::Building | Phase::Changing)) {
        return false;
    }
    let mut storage = match state.take() {
        Some(bank) => BankStorage { members: bank.members, groups: bank.groups },
        None => BANK_STORAGE[role].lock().take().expect("boot-reserved bank storage"),
    };
    *storage.members = members;
    storage.groups.clear();
    *state = Some(Bank {
        entry: entry as usize, owner, epoch: super::mii_saved_alpha::current_generation(role as u32),
        members: storage.members, phase: Phase::Building,
        bodies: [Body::default(); NATIVE_MEMBERS], observed: None, from: 0, target: 0,
        acknowledged: false, departed: false, failed: false,
        groups: storage.groups, alias: 0, exchange: None, swap: None, prefetch: None, swap_started: None,
        pair_logged: false, identity: None, configuration: None, ordinary_identity: None, activated: Default::default(),
        construction: None, background: None, tick: TickGap::default(), held_turn: 0, scene: None,
        object_fetch: None, retiring: None, preparing: None,
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

/// P-HOT B H1 (LANE-HOT-SLOTS-REPORT.md §12; operator "Yes pursue B"): Training's scene configure has stored a
/// visible role's resource header (`main+0x66de14`, `str w13,[x20,#0x80]`: x20 = R, w13 = the object kind, bit 0
/// still clear, nothing has read the header yet). With room in Ready Fighters and a bankable next member in the
/// role's cycle, the header gets the role's start-up window (`+0x5c`/`+0x60`, the original staying `+0x58`, native's
/// primary) and a pending bank (no entry yet) is published: the served `0x17e5bc0` requests the other members for it
/// (H2), the creation descriptor is rewritten to it (H3) and `0x654dc0` binds it to the entry (H4). Any guard that
/// fails leaves the scene as native built it (count 1, today's path).
#[skyline::hook(offset = 0x66de14, inline)]
unsafe fn scene_header(ctx: &mut InlineCtx) {
    use crate::engine::swap::entry_window::{entry_window, extendable, extras_allowed, HeaderFacts};
    let r = ctx.registers[20].x() as *mut u8;
    if !ENABLED.load(Ordering::Acquire) || !aligned(r) { return; }
    if !crate::runtime::roster::scope_gate::admitted("native_body_bank::scene_header") { return; }
    let facts = HeaderFacts {
        training: crate::runtime::compat::common::is_native_training_mode(),
        owner: read(r, 0x54), flags: read(r, 0x4c), object: ctx.registers[13].w(),
        kinds: [read(r, 0x58), read(r, 0x5c), read(r, 0x60)],
    };
    let Some(original) = extendable(facts) else { return; };
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if settings.slots == 0 || !settings.fighters { return; }
    let menu = crate::runtime::compat::sync::read(&crate::runtime::compat::menu::MENU);
    // rc.6 item 3, Ice Climbers (a session that starts on them gets its groups built): the pair start becomes the
    // pair-lead window only with the pair bank on and the pair as the slot's requested form; Solo Popo stays native's.
    let role = if facts.owner == 0 { super::Role::Player } else { super::Role::Cpu };
    if original == POPO && !(pair_enabled(&settings)
        && super::requested_forms_at(&menu, role, POPO, Some(0)) == Forms::starting(POPO)) { return; }
    // rc.6 item 3, Pokémon Trainer and Pyra/Mythra (inbox 01:09Z): the native group becomes its own window lead only with
    // native groups on, the group as the slot's requested form, and the original's return served by the bank
    // (`original_form_bankable`): otherwise the role's first switch back to it would Reload and retire the bank.
    if Members::native_group(original).is_some() && !(special_policy(&settings).native && settings.original_bank_enabled()
        && super::requested_forms_at(&menu, role, original, Some(0)) == Forms::starting(original)) { return; }
    let roster = if facts.owner == 0 { menu.player_roster } else { menu.cpu_roster };
    let cycle = crate::engine::swap::cycle_members::cycle_members(roster, crate::engine::swap::cycle_members::NO_MII);
    let numbered = crate::engine::swap::cycle_members::numbered(&cycle, Some(original));
    let Some(members) = entry_window(original, &numbered, extras_allowed(settings.ready_fighters, settings.bank_members), false)
        else { return; };
    let scene = crate::runtime::lobby::game_mode::current().map(|m| (m.mode, u32::from(m.melee)));
    if !publish_pending(facts.owner, members, scene) { return; }
    let kinds = members.kinds();
    fields::write_unchecked(r as usize, 0x5c, if members.count() > 1 { kinds[1] } else { u32::MAX });
    fields::write_unchecked(r as usize, 0x60, if members.count() > 2 { kinds[2] } else { u32::MAX });
    trace!("[BodyBank] start-up window owner={} R={:p} kinds={:x?} (scene header)", facts.owner, r, &kinds[..members.count()]);
}

/// H1's bank record: entry unknown (0) until H4 binds it, Building, stamped with the scene it was made in.
unsafe fn publish_pending(owner: u32, members: Members, scene: Option<(u32, u32)>) -> bool {
    let Some(role) = role(owner) else { return false; };
    let mut state = BANKS[role].lock();
    if state.as_ref().is_some_and(|bank| matches!(bank.phase, Phase::Building | Phase::Changing)) { return false; }
    let mut storage = match state.take() {
        Some(bank) => BankStorage { members: bank.members, groups: bank.groups },
        None => match BANK_STORAGE[role].lock().take() { Some(storage) => storage, None => return false },
    };
    *storage.members = members;
    storage.groups.clear();
    *state = Some(Bank {
        entry: 0, owner, epoch: super::mii_saved_alpha::current_generation(role as u32),
        members: storage.members, phase: Phase::Building,
        bodies: [Body::default(); NATIVE_MEMBERS], observed: None, from: 0, target: 0,
        acknowledged: false, departed: false, failed: false,
        groups: storage.groups, alias: 0, exchange: None, swap: None, prefetch: None, swap_started: None,
        pair_logged: false, identity: None, configuration: None, ordinary_identity: None, activated: Default::default(),
        construction: None, background: None, tick: TickGap::default(), held_turn: 0, scene,
        object_fetch: None, retiring: None, preparing: None,
    });
    EXTRAS[role].store((members.count() - 1) as u8, Ordering::Release);
    GROUP_BODIES[role].store(0, Ordering::Release);
    true
}

/// P-HOT B H3: the creation `main+0x678174` (x1 = owner, x2 = the descriptor) of a role with a pending start-up
/// window. Native's single form of the window's original becomes the window (count, kinds, actor count, active 0,
/// as Pokémon Trainer's creation reads); any other shape retires the pending window and leaves the creation as it is.
pub(crate) unsafe fn creation(owner: u32, descriptor: *mut u8) {
    use crate::engine::swap::entry_window::{creation_matches, keeps_native_creation, Words};
    let Some(role) = role(owner) else { return; };
    if !aligned(descriptor) { return; }
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut().filter(|bank| bank.entry == 0 && bank.owner == owner && bank.phase == Phase::Building)
        else { return; };
    let window = Words::of(&bank.members);
    let native = Words { count: read(descriptor, 0x14), kinds: [read(descriptor, 0x18), read(descriptor, 0x1c), read(descriptor, 0x20)],
        actor_count: read(descriptor, 0x28), active: read(descriptor, 0x43) };
    if !creation_matches(native, window) {
        bank.phase = Phase::Retired;
        bank.departed = true;
        EXTRAS[role].store(0, Ordering::Release);
        trace!("[BodyBank] start-up window owner={} dropped: creation {:x?} is not {:x?}", owner, native, window.kinds);
        return;
    }
    // rc.6 item 3: a native-group lead is native's own creation, its starting form included; nothing is written.
    if keeps_native_creation(window) {
        trace!("[BodyBank] start-up window owner={} creation {:x?} kept (native group lead)", owner, native);
        return;
    }
    fields::write_unchecked(descriptor as usize, 0x14, window.count);
    for (i, kind) in window.kinds.iter().enumerate() { fields::write_unchecked(descriptor as usize, 0x18 + i * 4, *kind); }
    fields::write_unchecked(descriptor as usize, 0x28, window.actor_count);
    fields::write_unchecked(descriptor as usize, 0x43, window.active);
    trace!("[BodyBank] start-up window owner={} creation {:x?}", owner, window);
}

/// P-HOT B H4: the first constructed entry of a pending window's owner whose descriptor shows the window binds it.
unsafe fn bind_pending(entry: *mut u8, role: usize) {
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut().filter(|bank| bank.entry == 0 && bank.phase == Phase::Building) else { return; };
    if read::<u32>(entry, 0xe0) != bank.owner { return; }
    let count = bank.members.count();
    let words_match = read::<u32>(entry, 0x14) == count as u32 && read::<u32>(entry, 0x28) == bank.members.actor_count()
        && (0..count).all(|i| read::<u32>(entry, 0x18 + i * 4) == bank.members.kinds()[i]);
    if !words_match { return; }
    bank.entry = entry as usize;
    trace!("[BodyBank] start-up window owner={} bound entry={:p}", bank.owner, entry);
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
        let state = BANKS[role].lock();
        let Some(bank) = state.as_ref() else { return false; };
        let target_group = bank.groups.iter().position(|g| g.state.reachable() && group_serves(*g.members, kind, forms).is_some());
        let kirby_target = !capabilities(form_class(kind, forms)).prefetch_while_live;
        let pair_target = target_group.is_some_and(|g| bank.groups[g].state == GroupState::Prefetched
            && pair_resource_references(*bank.groups[g].members).is_some());
        let mut holders = crate::engine::compat::fixed::FixedList::<usize, MAX_EXTRA_GROUPS>::new(0);
        for (i, g) in bank.groups.iter().enumerate() {
            if g.state == GroupState::Prefetched && Some(i) != target_group { holders.push(i); }
        }
        ((kirby_target || pair_target) && !holders.is_empty() && bank.prefetch.is_none() && bank.alias != 0)
            .then(|| (holders, occupancy_of(bank, role, copy), bank.alias,
                if kirby_target { "Kirby target (drain before live)" } else { "pair build needs an exclusive reset batch" }))
    };
    if let Some((holders, occupancy, alias, why)) = drain {
        let base = BASE.load(Ordering::Acquire);
        if resource(base, owner) == Some(alias) && resource_ready(alias) {
            for group in holders.iter().copied() {
                let (released, kept) = release_group_keys_of(base, role, alias, group, &occupancy, owner, why);
                let mut state = BANKS[role].lock();
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
    // Inbox 00:18Z: a detached PT group's re-acquired object kind goes back at the same drain (IC-PAIR-STALL: no object
    // token in a pair build's reset batch; D-KIRBY1: none under a live Kirby). Its next attach then takes the Reload path
    // unless it takes the kind again first (`group_reachable`).
    let held = {
        let state = BANKS[role].lock();
        state.as_ref().map(|bank| {
            let target = bank.groups.iter().position(|g| g.state.reachable() && group_serves(*g.members, kind, forms).is_some());
            let kirby = !capabilities(form_class(kind, forms)).prefetch_while_live;
            let pair = target.is_some_and(|g| bank.groups[g].state == GroupState::Prefetched
                && pair_resource_references(*bank.groups[g].members).is_some());
            let why = if kirby { "Kirby target (drain before live)" } else { "pair build needs an exclusive reset batch" };
            let mut held = FixedList::<usize, MAX_EXTRA_GROUPS>::new(0);
            for (i, g) in bank.groups.iter().enumerate() {
                if (kirby || pair) && g.object_held && Some(i) != target { held.push(i); }
            }
            (held, why)
        })
    };
    if let Some((held, why)) = held {
        let base = BASE.load(Ordering::Acquire);
        for group in held.iter().copied() {
            return_object_key(base, role, owner, group, why);
        }
    }
    let mut state = BANKS[role].lock();
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

enum Action { None, Swap, Lifecycle, Construct, SyncCopies, Background }

/// Called before all normal roster/source/HUD work, never instead of that work.
/// INSTALL completion remains owned by the original roster/Mii transaction.
pub(super) unsafe fn service(entry: *mut u8, owner: u32) {
    // Operator option C (primary 19:47Z): the release watch's purge runs here, at the top of this hook, under no lock.
    if let Some(ticket) = res_unload::owed_purge() { purge_on_main_thread(ticket); }
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
    let started = now_us();
    let step = match service_locked(entry, owner, role) {
        Action::None => return,
        Action::Swap => { perform_swap(entry, owner, role); "swap" }
        Action::Lifecycle => { group_lifecycle(entry, owner, role); "lifecycle" }
        Action::SyncCopies => { sync_copy_packages(entry, owner, role); "copies" }
        Action::Construct => { settle_construction(entry, owner, role); "construct" }
        Action::Background => { settle_background(entry, owner, role); "background settle" }
    };
    slow_step(owner, step, started);
}

/// Logged once: the service hook ran on a thread other than MainThread while a purge was owed (it waits for one that is).
static PURGE_OFF_MAIN: AtomicBool = AtomicBool::new(false);

/// Operator option C, the primary's 19:47Z conditions: the release watch's purge, `mallctl("arena.4096.purge", 0, 0, 0,
/// 0)` (`main+0x392ec40`; F6), runs only here, on the game's MainThread, and never on a plugin thread: a purge holds
/// jemalloc's arena and decay locks, which the game's workers take to allocate, and a plugin thread pinned to core 0
/// holding them while MainThread busy-joins is the deadlock class (`rc6-qpf3`, `lock_order.rs`). The caller holds no
/// lock. The line records the thread, the time the call took (the frame it lands in pays it) and the charge it returned.
unsafe fn purge_on_main_thread(ticket: u32) {
    let Some(base) = res_unload::base() else { return; };
    // SAFETY: the calling thread's own object and its name (a static C string the SDK keeps); nothing is written.
    let name = skyline::nn::os::GetThreadNamePointer(skyline::nn::os::GetCurrentThread());
    let main = !name.is_null() && core::ffi::CStr::from_ptr(name as *const core::ffi::c_char).to_bytes() == b"MainThread";
    if !main || crate::runtime::threads::on_plugin_thread() {
        if !PURGE_OFF_MAIN.swap(true, Ordering::AcqRel) {
            trace!("[GroupSwap] purge waits: the service hook ran off MainThread (plugin={})", crate::runtime::threads::on_plugin_thread());
        }
        return;
    }
    debug_assert!(!crate::runtime::threads::on_plugin_thread(), "the purge never runs on a plugin thread");
    type Mallctl = unsafe extern "C" fn(*const u8, *mut u8, *mut usize, *mut u8, usize) -> i32;
    // SAFETY: rows `res_unload.mallctl*`, `purge_ctl`, `arena_decay` guard jemalloc's `mallctl` and its
    // `arena.<i>.purge` handler; the name is a static C string and nothing is read or written back.
    let mallctl: Mallctl = core::mem::transmute(base + crate::engine::swap::res_unload::MALLCTL);
    let before = MemorySample::capture();
    let started = now_us();
    let rc = mallctl(crate::engine::swap::res_unload::PURGE_ALL.as_ptr(), core::ptr::null_mut(), core::ptr::null_mut(),
        core::ptr::null_mut(), 0);
    let us = now_us().saturating_sub(started);
    let after = MemorySample::capture();
    res_unload::purged(ticket);
    trace!("[GroupSwap] purge ticket={} rc={} thread=MainThread us={} committed={:?}->{:?} headroom={:?}->{:?}",
        ticket, rc, us, before.committed.get(), after.committed.get(), before.headroom(), after.headroom());
}

unsafe fn service_locked(entry: *mut u8, owner: u32, role: usize) -> Action {
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut() else { return Action::None; };
    if bank.owner != owner || bank.entry != entry as usize || bank.phase == Phase::Retired {
        return Action::None;
    }
    // P3-0: one tick of this entry's pre-service; the meter is armed by a request and read at Ready.
    bank.tick.observe(now_us());
    split::observe(role, now_us());
    // Diag: each pre-service tick of a switch in flight, the entry's native progress (P1's attach into the pair group
    // took a frame more than the CPU's between its acknowledgement and N2; primary 2026-11-09 02:00Z).
    #[cfg(feature = "diag")]
    if bank.phase == Phase::Changing {
        let s = snapshot(entry);
        // Only the entry's installed cells (`+0x4150`): a two-body entry never wrote `+0x4170`, and reading through it
        // logged Unmapped Read8 (Eden rc6-bg-ptx4, 17 times).
        let installed = fields::read::<u64>(entry as usize, 0x4150).unwrap_or(0);
        let state = |cell: usize| ((cell - 0x4160) as u64 / 8 < installed).then(|| fields::child(entry as usize, cell)
            .and_then(|b| fields::read::<u8>(b, 0x3a))).flatten();
        trace!("[GroupSwap] attach tick owner={} target={} acknowledged={} index={} pending={} previous={} flags={:#x} outer={} loader={} changing={} guard={} states=[{:?},{:?},{:?}]",
            owner, bank.target, bank.acknowledged, s.index, s.pending, s.previous_or_next, s.flags, s.outer, s.loader, s.changing,
            s.guard, state(0x4160), state(0x4168), state(0x4170));
    }
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
            let mut live = [(0usize, 0u32); (MAX_EXTRA_GROUPS + 1) * NATIVE_MEMBERS];
            for (slot, body) in live[..NATIVE_MEMBERS].iter_mut().zip(now) {
                *slot = (body.address, body.id);
            }
            for (group, records) in bank.groups.iter().zip(live[NATIVE_MEMBERS..].chunks_mut(NATIVE_MEMBERS)) {
                for (slot, body) in records.iter_mut().zip(group.bodies) {
                    *slot = (body.address, body.id);
                }
            }
            let active = now[usize::from(index)];
            bank.activated.observe(live, (active.address, active.id));
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
            let idle = GROUPS_ENABLED.load(Ordering::Acquire) && !bank.departed && bank.swap.is_none();
            // Other native entries can change after all our groups prefetch.
            // Keep their copy-package dependencies current before the next
            // requested registration, without restarting group lifecycle work.
            return if !idle { Action::None }
                // rc.6 item 3: a background build in flight settles on the window's ticks; one owed starts.
                else if bank.background.is_some() { Action::Background }
                else if bank.retiring.is_some() || bank.preparing.is_some()
                    || prepare_candidate(BASE.load(Ordering::Acquire), bank).is_some() { Action::Lifecycle }
                else if bank.object_fetch.is_some() || rearm_candidate(BASE.load(Ordering::Acquire), bank).is_some() { Action::Lifecycle }
                else if bank.prefetch.is_some()
                    || bank.groups.iter().any(|g| g.state.startable() || (g.leaving && (!leaves_at_pause(g.state) || g.release_now)))
                    || background_candidate(bank, Some(members.kinds()[usize::from(index)])).is_some() {
                    Action::Lifecycle
                } else { Action::SyncCopies };
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
type CopyPackageSets = crate::engine::swap::swap_model::copy_packages::PackageSets;

/// One native group's value-only teardown receipt. Bounds are its three cells,
/// three context words, and those members plus the special-object prefetch key.
#[derive(Clone, Copy)]
struct DetachedSnapshot {
    owner: u32,
    bodies: FixedList<BodyRef, NATIVE_MEMBERS>,
    contexts: FixedList<ContextRef, NATIVE_MEMBERS>,
    dispatch: Option<DispatcherRef>,
    records: Option<RecordsRef>,
    held_kinds: FixedList<u32, { NATIVE_MEMBERS + 1 }>,
}
impl DetachedSnapshot {
    fn new(owner: u32) -> Self {
        Self { owner,
            bodies: FixedList::new(BodyRef { id: 0, kind: 0, cell: 0, activated: false, valid: false, registered: None }),
            contexts: FixedList::new(ContextRef { ctx: 0, controller: 0, valid: false }),
            dispatch: None, records: None, held_kinds: FixedList::new(0) }
    }
    fn view(&self) -> DetachedView<'_> {
        DetachedView { owner: self.owner, bodies: &self.bodies, contexts: &self.contexts,
            dispatch: self.dispatch, records: self.records, held_kinds: &self.held_kinds }
    }
}

fn detached_keys(plan: &DetachedPlan<'_>) -> FixedList<u32, { MAX_EXTRA_GROUPS * (NATIVE_MEMBERS + 1) }> {
    let mut keys = FixedList::new(0);
    for key in plan.keys() { keys.push(key); }
    keys
}

enum DispatchNote { Destroyed(bool, [usize; 2]), Leaked([usize; 2]) }
impl std::fmt::Display for DispatchNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Destroyed(destroyed, arrays) => write!(f, "dispatch_destroyed={} arrays={:x?}", destroyed, arrays),
            Self::Leaked(dispatch) => write!(f, "dispatch_leaked={:x?}", dispatch),
        }
    }
}

struct MemberSets<'a>(&'a [Members]);
impl std::fmt::Debug for MemberSets<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for members in self.0 { list.entry(&&members.kinds()[..members.count()]); }
        list.finish()
    }
}

fn copy_package_sets(bank: &Bank) -> CopyPackageSets {
    let words = |members: Members| {
        let mut words = [u32::MAX; NATIVE_MEMBERS];
        let count = members.count().min(NATIVE_MEMBERS);
        words[..count].copy_from_slice(&members.kinds()[..count]);
        words
    };
    let mut sets = CopyPackageSets::new([u32::MAX; NATIVE_MEMBERS]);
    for group in bank.groups.iter().filter(|g| g.state.holds_keys() || g.state == GroupState::Built) {
        if sets.is_empty() { sets.push(words(*bank.members)); }
        sets.push(words(*group.members));
    }
    sets
}

/// The window's size (`+0x4150`); its data pointer (`+0x4158`) names its three cells (`+0x4160/68/70`).
const WINDOW: usize = 0x4150;

/// The window's cells that hold a body: below its size, with its data pointer on the cells (`fields::inline_elements`).
unsafe fn window_cells(entry: usize) -> [Option<usize>; NATIVE_MEMBERS] {
    fields::inline_elements::<NATIVE_MEMBERS>(entry, WINDOW)
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
    let window = role(owner).and_then(|role| BANKS[role].lock().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.owner == owner && descriptor(entry, bank))
        .map(|bank| (bank.members.kinds(), bank.members.count(), bank.activated)));
    let (kinds, count, activated) = match window {
        Some(window) => window,
        None => {
            let count = (read::<u32>(entry, 0x14) as usize).min(NATIVE_MEMBERS);
            ([read::<u32>(entry, 0x18), read::<u32>(entry, 0x1c), read::<u32>(entry, 0x20)], count, Default::default())
        }
    };
    let mut cells = crate::engine::compat::fixed::FixedList::<BodyRef, NATIVE_MEMBERS>::new(
        BodyRef { id: 0, kind: 0, cell: 0, activated: false, valid: false, registered: None });
    // Only the cells the window holds (`+0x4150`), not every planned member: a slot past the window's size is uninitialised
    // or left over, and final-ally-72344fd9's finalize followed one (float bytes, `0xb31caf33bf800000`: 8 faults).
    let installed = window_cells(entry as usize);
    let held = installed.iter().flatten().count();
    if always && held < count {
        trace!("[BodyBank] retire owner={} reason={} planned={} held={}: slots past the window's size are not read", owner, reason,
            count, held);
    }
    for (i, kind) in kinds.iter().enumerate().take(count) {
        let Some(body) = installed.get(i).copied().flatten() else { continue; };
        let registered = registered_kind(resource(base, owner), i);
        // The game's bytes first: a cell that is not a fighter body is not read further.
        if fields::read::<usize>(body, 0) != Some(base + FIGHTER_VTABLE) {
            cells.push(BodyRef { id: 0, kind: *kind, cell: i as u8, activated: false, valid: false, registered });
            continue;
        }
        let id = fields::read::<u32>(body, 8).unwrap_or(0);
        // The ref carries the body's own kind (`body+0xc`, what its unload's `unprepare`
        // reads), not the planned one: at a mod Reload the cells still hold the outgoing
        // bodies while the bank already lists the incoming kinds (Yoga `m4b3` #140).
        let own = fields::read::<u32>(body, 0xc);
        let live = fields::read::<u8>(body, 0x3a) == Some(4);
        cells.push(BodyRef { id, kind: own.unwrap_or(*kind), cell: i as u8,
            activated: live || activated.contains(&(body, id)), valid: own.is_some(), registered });
    }
    let contexts = graph::window_contexts(entry as usize, |context| valid_context(base, context, owner));
    let probed = contexts.len();
    let consumers = graph::probe_consumers(base, owner);
    let plan = attached_plan(owner, &cells, &contexts, &consumers);
    if !always && plan.routes.is_empty() && plan.kirby_misplaced == 0 { return; }
    let receipt = graph::execute_attached(base, &plan, owner, reason);
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
        mii_r: None,
        info_3a0: None,
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
        let state = BANKS[role].lock();
        state.as_ref().and_then(|bank| bank.groups.get(group)).and_then(|g| g.saved).map(|w| (w.flags, w.records[0]))
    };
    if let Some((flags, holder)) = outgoing {
        let valid = holder != 0 && valid_special_object(base, holder, owner);
        // Inbox 02:00Z: the exit's tick runs pass 1 only (the trainer leaves the world); the group lifecycle releases
        // its own pools and then destroys it on later ticks (`retire_step`), so no tick carries the whole release.
        finish_retirement(base, role, owner, "another exit");
        let split = EXIT_SPLIT.load(Ordering::Acquire);
        let retired = if split {
            articles::deactivate_saved(owner, flags, holder, valid, "detach")
        } else {
            articles::retire_saved(base, owner, flags, holder, valid, "detach")
        };
        if retired {
            let mut state = BANKS[role].lock();
            if let Some(bank) = state.as_mut() {
                if let Some(saved) = bank.groups.get_mut(group).and_then(|g| g.saved.as_mut()) {
                    saved.records[1] = 0;
                }
                if split {
                    bank.retiring = Some(Retiring { group, holder, stage: RetireStage::Deactivate, token: None });
                }
            }
        }
    }
    if activates && matches!(prepared, Prepared::Warm { .. }) {
        articles::rebuild_current(base, owner, entry, current_holder_valid(base, entry, owner), "warm attach");
        // Inbox 00:18Z: the rebuild read the trainer article's files through the re-acquired object kind; it goes back.
        return_object_key(base, role, owner, group, "warm attach rebuilt its articles");
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
    split::mark(split::Step::Plugin);
    let t = heap_alloc(task::ALIGN, task::SIZE);
    split::mark(split::Step::TaskAlloc);
    if t == 0 {
        return 0;
    }
    std::ptr::write_bytes(t as *mut u8, 0, task::SIZE);
    fields::write_unchecked(t, task::BODY, object);
    fields::write_unchecked(t, task::QUEUE, task::QUEUE_ID);
    schedule_task(base, t, base + vtable, callback);
    t
}

/// The functor `{vtable, task, _, _, self}` built on the stack and swapped into `task+0x50` by `0x3cb050`, the emptied
/// local destroyed as native does, then `0x3549720(task+0x10, callback, 2, 1)` (`queue_task`, and state 11's Pokémon
/// Trainer task, `queue_trainer`).
unsafe fn schedule_task(base: usize, t: usize, vtable: usize, callback: usize) {
    // `vtable` is an address: a native one (`base + offset`) or the prepared build's `PREPARE_VTABLE`.
    use construction_task as task;
    let functor_swap: unsafe extern "C" fn(*mut [usize; 5], usize) = std::mem::transmute(base + FUNCTOR_COPY);
    let schedule: unsafe extern "C" fn(usize, usize, u32, u32) = std::mem::transmute(base + TASK_SCHEDULE);
    let mut functor = [vtable, t, 0, 0, 0];
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
        let mut state = BANKS[role].lock();
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
        let mut state = BANKS[role].lock();
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
        let mut state = BANKS[role].lock();
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
        let mut state = BANKS[role].lock();
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

// ----- rc.6 item 3: background builds (operator decision 2026-11-08, inbox 19:12Z) -----
//
// A Prefetched group's bodies are built while the window stays live, so its first switch is a warm exchange. Static RE
// and the traced probe (LANE-OPUS-RC6-HOTSLOTS-REPORT 19:46Z, `artifacts/phase23/rc6-item3-re/`): today's build binds
// to the entry while it runs, which is safe only because the window is empty then. Here:
//  1. the entry-level third (`0x654dc0`, `0x655320`, and the control init `0x64f820` called directly, not queued as
//     state 8's task) runs in one game-thread tick over the group's words, with the live window's fields, dispatcher
//     arrays and entry-info bytes saved before and put back after; what it made becomes the group's saved window;
//  2. body init's registration (`prepare_entry` at `0x607f30`, on the task worker, into the live cells' registry
//     slots) is skipped for this owner (`background_skip`); a detached group holds none (U18) and its warm attach
//     registers it;
//  3. the bodies go into the group record, never into the entry's cells, and the loader is never parked;
//  4. only `body_bank::background_build_eligible` groups; every other group keeps today's build at its first switch.
// It holds INSTALL like a prefetch, so a cycle press meanwhile is held (`character_roster`'s pending request).

fn owner_bit(owner: u32) -> u32 { if owner < 32 { 1 << owner } else { 0 } }

/// `body+0xf816`: the body's cell in its window, written by body init (`0x607a84`, again at `0x60b5bc`) from the entry's
/// descriptor (SPECIAL-GROUPS §0 X2).
const BODY_CELL_BYTE: usize = 0xf816;

// ----- rc.6 item 3, Mii groups (operator 20:26Z "All groups in rc.6"; SPECIAL-GROUPS §1) -----
//
// A Mii body's init and construct read the Mii identity from `R` on the worker, one to three frames after the tick,
// while `R` holds the live window's words again (the game thread reads them for the live fighter, so `R` cannot keep the
// group's across frames). The tick runs the native Mii setup with the identity in `R` for that one tick and copies
// `R+0x40..+0xa8`, the group's words in it, into `SHADOW_R`. On the worker, hooks keyed on the build's real `R` and owner
// (SPECIAL-GROUPS §1 (b), the 21:10Z intervals):
// - body init reads the shadow from `0x607aac` (after its owner-table load) to `0x607e74` (prepare_entry's `R+0xb0` and
//   `R+0x54` must be the real `R`), with the group's `FE+0x52` (`0x607d80`);
// - construct reads it from `0x60b98c` (`x20`, spilled and reloaded inside the interval) to `0x60de68`, and at
//   `0x60bf30` (`R` reloaded by owner), with the group's `D+0x72` (`0x60b7a8`, `0x60b834`) and `FE+0x41` (`0x60b98c`);
// - the owner-indexed readers `0x33a05a8` (costume) and `0x344fcd0` (hat) read it on the worker thread building that
//   body only (`MII_THREAD`; the game thread calls them for the live fighter), marked from each interval's start to its
//   function's single epilogue; so does `0x6e6bc0` (paths and costume), which ran there in `rc6-hot-xps14`. `0x6e655c`
//   and `0x33c3468` are counted there (M-bg2), not redirected.
// The construction tail's per-kind release (`0x60dec4`) gets kind -1: it would key on the live words; the settlement
// releases the group's keys with its identity in `R` (`release_group_keys_of`). The tail's pair test (`0x60de68`, X1)
// is redirected for every background body to the per-kind release.

#[repr(C, align(16))]
struct ShadowR(std::cell::UnsafeCell<[u8; SHADOW_R_SIZE]>);
// SAFETY: written on the game thread before the bodies are queued, read by the worker while they construct, read back
// after they settled (INSTALL: one background build at a time).
unsafe impl Sync for ShadowR {}
const SHADOW_R_SIZE: usize = 0xb0;
/// `R+0x50`: the pair rule's references (read for the X5 diag line only; a background build writes none).
const R_PAIR_REFERENCES: usize = 0x50;
static SHADOW_R: ShadowR = ShadowR(std::cell::UnsafeCell::new([0; SHADOW_R_SIZE]));
/// The background Mii build's real `R` (0: none in flight), its owner, and its values: the record index (bits 0-15),
/// the costume byte `FE+0x41` (16-23) and `FE+0x52` (24-31).
static MII_REAL_R: AtomicUsize = AtomicUsize::new(0);
static MII_OWNER: AtomicU32 = AtomicU32::new(u32::MAX);
static MII_VALUES: AtomicU32 = AtomicU32::new(0);
/// The worker thread inside the background Mii body's init or construct (`GetCurrentThread`, 0: none).
static MII_THREAD: AtomicUsize = AtomicUsize::new(0);
/// The Mii hooks' seam rows passed and they are installed.
static BACKGROUND_MII_ENABLED: AtomicBool = AtomicBool::new(false);
/// Hits per Mii hook for the build line: init enter, flags, leave, index a, index b, construct enter, path, tail, the
/// costume and hat readers, the three counted readers, Pokémon Trainer's pool task and the reader at `0x33c3370` (X6).
const MII_HIT_NAMES: [&str; 15] = ["init", "flags", "leave", "index_a", "index_b", "construct", "path", "tail", "costume",
    "hat", "r6e655c", "paths", "r33c3468", "trainer", "r33c3370"];
static MII_HITS: [AtomicU32; 15] = [const { AtomicU32::new(0) }; 15];

fn shadow_r_address() -> usize { SHADOW_R.0.get() as usize }

#[inline(always)]
fn mii_hit(i: usize) { MII_HITS[i].fetch_add(1, Ordering::Relaxed); }

#[inline(always)]
fn current_thread() -> usize { unsafe { skyline::nn::os::GetCurrentThread() as usize } }

/// A body of the armed build (`MII_REAL_R`), by its owner: the background Mii build's body, or (X6) any body of a group
/// built beside a live Mii window. The live window is built, so a body that constructs for that owner meanwhile is the
/// build's (`background_body`'s rule).
#[inline(always)]
unsafe fn shadowed_body(body: usize) -> bool {
    MII_REAL_R.load(Ordering::Acquire) != 0 && aligned(body as *const u8)
        && fields::read::<u64>(body, 0x10) == Some(u64::from(MII_OWNER.load(Ordering::Acquire)))
}

/// A body of the background Mii build: its owner, a Mii class.
#[inline(always)]
unsafe fn background_mii_body(body: usize) -> bool {
    shadowed_body(body) && fields::read::<u32>(body, 0xc).is_some_and(|kind| (0x48..=0x4a).contains(&kind))
}

/// The worker thread building the background Mii body, reading `R` (the register `r`) by owner.
#[inline(always)]
fn mii_worker_reads(r: usize) -> bool {
    let real = MII_REAL_R.load(Ordering::Acquire);
    real != 0 && r == real && MII_THREAD.load(Ordering::Acquire) == current_thread()
}

/// Body init after its owner-table load (`x19` body, `x24` R): the shadow until `0x607e74`, and the worker marked. Every
/// body of the armed build (X6: an ordinary, native or pair body beside a live Mii window too: its costume and paths,
/// `0x607dc0`/`0x607ebc` and the `0x17e8890` builds, key on `R+0x58`/`+0x64`).
#[skyline::hook(offset = 0x607aac, inline)]
unsafe fn bg_mii_init_enter(ctx: &mut InlineCtx) {
    let real = MII_REAL_R.load(Ordering::Acquire);
    if real != 0 && ctx.registers[24].x() as usize == real && shadowed_body(ctx.registers[19].x() as usize) {
        ctx.registers[24].set_x(shadow_r_address() as u64);
        MII_THREAD.store(current_thread(), Ordering::Release);
        mii_hit(0);
    }
}

/// `ldrb w23, [x21, #0x52]` read the live `FE+0x52`; the group's byte inside the shadow interval.
#[skyline::hook(offset = 0x607d80, inline)]
unsafe fn bg_mii_init_flags(ctx: &mut InlineCtx) {
    if ctx.registers[24].x() as usize == shadow_r_address() {
        ctx.registers[23].set_x(u64::from(MII_VALUES.load(Ordering::Acquire) >> 24));
        mii_hit(1);
    }
}

/// `mov x20, x24` after the path builder at `0x607e70`: the real `R` back; the shadow keeps the resource object
/// `0x607d18` stored.
#[skyline::hook(offset = 0x607e74, inline)]
unsafe fn bg_mii_init_leave(ctx: &mut InlineCtx) {
    if ctx.registers[24].x() as usize == shadow_r_address() {
        ctx.registers[24].set_x(MII_REAL_R.load(Ordering::Acquire) as u64);
        mii_hit(2);
    }
}

/// Body init's and construct's single epilogues: the worker marker ends with the function.
unsafe fn bg_mii_exit() {
    let thread = current_thread();
    let _ = MII_THREAD.compare_exchange(thread, 0, Ordering::AcqRel, Ordering::Acquire);
}
#[skyline::hook(offset = 0x60b5c8, inline)]
unsafe fn bg_mii_init_exit(_ctx: &mut InlineCtx) { bg_mii_exit(); }
#[skyline::hook(offset = 0x60df04, inline)]
unsafe fn bg_mii_construct_exit(_ctx: &mut InlineCtx) { bg_mii_exit(); }

/// Construct's colour reads: `w8` = `D+0x72` of the live window; the group's record index.
unsafe fn bg_mii_index(ctx: &mut InlineCtx, hit: usize) {
    if background_mii_body(ctx.registers[19].x() as usize) {
        ctx.registers[8].set_x(u64::from(MII_VALUES.load(Ordering::Acquire) & 0xffff));
        mii_hit(hit);
    }
}
#[skyline::hook(offset = 0x60b7a8, inline)]
unsafe fn bg_mii_construct_index_a(ctx: &mut InlineCtx) { bg_mii_index(ctx, 3); }
#[skyline::hook(offset = 0x60b834, inline)]
unsafe fn bg_mii_construct_index_b(ctx: &mut InlineCtx) { bg_mii_index(ctx, 4); }

/// `bl 0x17e99c0(out, R, kind, costume)`, straight-line from the owner-table load at `0x60b960` (whose only use so far is
/// `mov x1, x20`): `x20` and `x1` the shadow until `0x60de68`, the group's costume byte, and the worker marked; for every
/// body of the armed build (X6).
#[skyline::hook(offset = 0x60b98c, inline)]
unsafe fn bg_mii_construct_enter(ctx: &mut InlineCtx) {
    let real = MII_REAL_R.load(Ordering::Acquire);
    if real != 0 && ctx.registers[20].x() as usize == real && shadowed_body(ctx.registers[19].x() as usize) {
        ctx.registers[20].set_x(shadow_r_address() as u64);
        ctx.registers[1].set_x(shadow_r_address() as u64);
        ctx.registers[3].set_x(u64::from((MII_VALUES.load(Ordering::Acquire) >> 16) & 0xff));
        MII_THREAD.store(current_thread(), Ordering::Release);
        mii_hit(5);
    }
}

/// `R` reloaded by owner for the path builder `0x17e8890(…, kind, 0x14)` at `0x60bf3c`: the shadow.
#[skyline::hook(offset = 0x60bf30, inline)]
unsafe fn bg_mii_construct_path(ctx: &mut InlineCtx) {
    let real = MII_REAL_R.load(Ordering::Acquire);
    if real != 0 && ctx.registers[1].x() as usize == real && shadowed_body(ctx.registers[19].x() as usize) {
        ctx.registers[1].set_x(shadow_r_address() as u64);
        mii_hit(6);
    }
}

/// The construction tail's `bl 0x17e8610(R, kind)`: kind -1 (`0x17e86e4` then only re-checks the map) for every body of
/// the armed build: the release would key on the live window's words; the settlement releases the group's keys with its
/// own words in `R` (`release_group_keys_of`).
#[skyline::hook(offset = 0x60dec4, inline)]
unsafe fn bg_mii_tail_release(ctx: &mut InlineCtx) {
    if shadowed_body(ctx.registers[19].x() as usize) {
        ctx.registers[1].set_x(u64::from(u32::MAX));
    }
}

/// The owner costume reader (`x8` = `R` by owner) on the background Mii worker: the shadow (B's class and costume).
#[skyline::hook(offset = 0x33a05a8, inline)]
unsafe fn bg_mii_reader_costume(ctx: &mut InlineCtx) {
    if mii_worker_reads(ctx.registers[8].x() as usize) {
        ctx.registers[8].set_x(shadow_r_address() as u64);
        mii_hit(8);
    }
}

/// The hat token reader (`w3` = `R+0x70`, `x21` = `R` by owner, which its path build `0x17e8890` at `0x344fd74` takes
/// too; `x21` is reassigned at `0x344fdc0`) on the background Mii worker: the shadow and its token.
#[skyline::hook(offset = 0x344fcd4, inline)]
unsafe fn bg_mii_reader_hat(ctx: &mut InlineCtx) {
    if mii_worker_reads(ctx.registers[21].x() as usize) {
        ctx.registers[21].set_x(shadow_r_address() as u64);
        ctx.registers[3].set_x(u64::from(fields::read_unchecked::<u32>(shadow_r_address(), 0x70)));
        mii_hit(9);
    }
}

/// The owner reader at `0x6e6bc0` (`x20` = `R` by owner; every later use is an identity read at `0x6e6ccc`-`0x6e6d1c` or
/// the path builder `0x17e8890`): it ran on the worker during a background Mii construction (`rc6-hot-xps14`), so the
/// shadow there.
#[skyline::hook(offset = 0x6e6bc4, inline)]
unsafe fn bg_mii_reader_paths(ctx: &mut InlineCtx) {
    if mii_worker_reads(ctx.registers[20].x() as usize) {
        ctx.registers[20].set_x(shadow_r_address() as u64);
        mii_hit(11);
    }
}

/// Readers of the same shape not seen on the worker yet: counted there (M-bg2), not redirected.
#[skyline::hook(offset = 0x6e655c, inline)]
unsafe fn bg_mii_reader_6e655c(ctx: &mut InlineCtx) { if mii_worker_reads(ctx.registers[21].x() as usize) { mii_hit(10); } }
#[skyline::hook(offset = 0x33c3468, inline)]
unsafe fn bg_mii_reader_33c3468(ctx: &mut InlineCtx) { if mii_worker_reads(ctx.registers[21].x() as usize) { mii_hit(12); } }

/// X6 (Eden `rc6-hot-xps28`: `0x33c3468` counted 4 hits on the worker while Pokémon Trainer's bodies built beside a live
/// Mii window): its function `0x33c3370` loads `R` by owner into `x21` at `0x33c33a4`, and every use of `x21` until it is
/// reassigned at `0x33c37b4` is a `0x17e8890` path build (`0x33c33b8`, `0x33c33f8`, `0x33c3430`, `0x33c3694`,
/// `0x33c36d0`, `0x33c370c`) or the `+0x58`/`+0x64` costume read (`0x33c3468`-`0x33c34b8`) it stores at `x19+0x7ec`. On
/// the worker building the armed build's body, the shadow from its load (the counter at `0x33c3468` then stays 0).
#[skyline::hook(offset = 0x33c33a8, inline)]
unsafe fn bg_mii_reader_33c3370(ctx: &mut InlineCtx) {
    if mii_worker_reads(ctx.registers[21].x() as usize) {
        ctx.registers[21].set_x(shadow_r_address() as u64);
        mii_hit(14);
    }
}

/// X6: Pokémon Trainer's pool task (native state 11's functor `0x651a48-0x651b84`, `x21` the task) for a PT group built
/// beside a live Mii window, after `ldr x1, [x8, #0xe8]` loaded the owner's `R` for the trainer parameters' path
/// (`0x17e8890(…, R, 0x72, 0xc)`; SPECIAL-GROUPS §3: the functor reads only its task, `R` by owner and globals): the
/// shadow, and the worker marked until the pool build `0x3a6270` returns (`0x651b50`), so the owner readers its articles
/// call take the shadow too. Only while a build is armed, for its owner's `R`.
#[skyline::hook(offset = 0x651a70, inline)]
unsafe fn bg_trainer_enter(ctx: &mut InlineCtx) {
    let real = MII_REAL_R.load(Ordering::Acquire);
    if real != 0 && ctx.registers[1].x() as usize == real
        && fields::read::<u32>(ctx.registers[21].x() as usize, trainer_task::OWNER) == Some(MII_OWNER.load(Ordering::Acquire)) {
        ctx.registers[1].set_x(shadow_r_address() as u64);
        MII_THREAD.store(current_thread(), Ordering::Release);
        mii_hit(13);
    }
}

/// `ldr w1, [sp, #0x8]` right after the functor's `bl 0x3a6270`: the worker marker ends.
#[skyline::hook(offset = 0x651b50, inline)]
unsafe fn bg_trainer_exit(_ctx: &mut InlineCtx) { bg_mii_exit(); }

/// The control init `0x64f820` (with its tail, the select `0x64fb70`) costs the game thread 22-151 ms (Eden
/// `rc6-hot-xps9`: `models_contexts_control_us=[2299, 5053, 150913]`), so a background build queues it on the task worker
/// as loader state 8 does, with this copy of the entry as its object: the entry's first 0x100 bytes with the group's words,
/// the group's context words, index 0 and the group's flags (every field `0x64f820`/`0x64fb70` reads), so the live
/// entry is never touched from the worker. The select's table hook (`table`) serves the group's table for it. One build
/// at a time (INSTALL).
#[repr(C, align(16))]
struct Scratch(std::cell::UnsafeCell<[u8; SCRATCH_SIZE]>);
// SAFETY: written on the game thread before the control task is queued and read after it settled; the worker reads it
// only while the task runs.
unsafe impl Sync for Scratch {}
const SCRATCH_SIZE: usize = 0x5a00;
static SCRATCH: Scratch = Scratch(std::cell::UnsafeCell::new([0; SCRATCH_SIZE]));
/// The group's table (`Members::table`, stable in its boxed record) while the scratch's control init is queued.
static BACKGROUND_TABLE: AtomicUsize = AtomicUsize::new(0);

fn scratch_address() -> usize { SCRATCH.0.get() as usize }

/// Body init's `prepare_entry` site (`identity_view::init_prepare_entry` at `0x607f24`, the call at `0x607f30`): true
/// while `owner` has a background build in flight; the caller then passes kind −1, on which `0x17f1a60` returns at once
/// (its first test). The call's result is unused (`x0` is reloaded at `0x607f48`).
pub(crate) fn background_skip(owner: u32) -> bool {
    let skip = BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(owner) != 0;
    if skip { BACKGROUND_SKIPS.fetch_add(1, Ordering::Relaxed); }
    skip
}

/// SPECIAL-GROUPS §0 X5 (`cmn w8, #-1` after `ldr w8, [x21, #0x1c]`): body init reads the live window's cell 1 for a
/// Pyra/Mythra body (x24 = R, `R+0x54` its owner). Beside a one-member live window that is −1 and the body takes model
/// kind `0x73`/`0x74` (`0x607c20`), files the group never prefetched. A body of the owner's background build of a
/// Pyra/Mythra group reads the group's own cell 1, as its synchronous build does (Eden re6: `0x5b` for both bodies).
/// Atomics only; it runs on the task worker.
#[skyline::hook(offset = 0x607bac, inline)]
unsafe fn background_partner(ctx: &mut InlineCtx) {
    let cell = BACKGROUND_PARTNER.load(Ordering::Acquire);
    if cell == u32::MAX { return; }
    let r = ctx.registers[24].x() as usize;
    if fields::object(r) && BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(fields::read_unchecked::<u32>(r, 0x54)) != 0 {
        ctx.registers[8].set_x(u64::from(cell));
    }
}

/// rc.6 item 3, SPECIAL-GROUPS §3: native loader state 11's holder step (`0x650010` case 0xb) for a native group built in
/// the background. The X5 site; the 0x238-byte holder `+0x5a88` (16-aligned, `0x6509f0`, `0x650f38`).
const PARTNER_SITE: usize = 0x607bac;
const HOLDER_SIZE: usize = 0x238;
const HOLDER_ALIGN: usize = 0x10;
/// Pokémon Trainer's pool task: its functor's vtable (relocated data, the `+0x30` slot checked live), the functor the
/// worker runs and the callback state 11 schedules it with (`0x6512b0`).
const TRAINER_FUNCTOR_VTABLE: usize = 0x4f81498;
const TRAINER_FUNCTOR: usize = 0x651a30;
const TRAINER_CALLBACK: usize = 0x651770;
/// Pyra/Mythra's pool table, its entry and the two lazy-init guards state 11 takes before the pool (`0x650a88-0x650a9c`,
/// the inits at `0x651504-0x651580`), through `__cxa_guard_acquire`/`__cxa_guard_release`.
const PARTNER_TABLE: usize = 0x52b94c8;
const PARTNER_ENTRY: usize = 0x52b94a0;
const PARTNER_ENTRY_CALLBACK: usize = 0x6517a0;
const PARTNER_GUARDS: [usize; 2] = [0x52b9498, 0x52b94c0];
const GUARD_ACQUIRE: usize = 0x39c06b0;
const GUARD_RELEASE: usize = 0x39c06c0;

/// State 11's holder words (`0x650a4c-0x650a80`, `0x650f38-0x650f6c`) over a zeroed holder: the vtable, capacity 0x40 at
/// `+0x08`, an empty list at `+0x10`/`+0x18` (`+0x18 = holder+0x20`), `+0x220 = 0`, the table `0x3a6270` records at
/// `+0x228` (0 until then), the slot it records at `+0x230` (−1 until then), `+0x234 = 6`.
unsafe fn init_holder(base: usize, holder: usize) {
    std::ptr::write_bytes(holder as *mut u8, 0, HOLDER_SIZE);
    fields::write_unchecked(holder, 0x00, base + SPECIAL_OBJECT_VTABLE);
    fields::write_unchecked(holder, 0x08, 0x40u64);
    fields::write_unchecked(holder, 0x18, holder + 0x20);
    fields::write_unchecked(holder, 0x230, u32::MAX);
    fields::write_unchecked(holder, 0x234, 6u8);
}

/// State 11's lazy init of the Pyra/Mythra pool table before `0x3a6270` (`0x651504-0x651580`): while a guard's bit 0 is
/// clear it is taken, its words written and released, exactly as native does.
unsafe fn partner_guards(base: usize) {
    let acquire: unsafe extern "C" fn(usize) -> i32 = std::mem::transmute(base + GUARD_ACQUIRE);
    let release: unsafe extern "C" fn(usize) = std::mem::transmute(base + GUARD_RELEASE);
    let [first, second] = PARTNER_GUARDS;
    if fields::read_unchecked::<u8>(base, first) & 1 == 0 && acquire(base + first) != 0 {
        fields::write_unchecked(base, PARTNER_ENTRY, 0x1_0000_01cfu64);
        fields::write_unchecked(base, PARTNER_ENTRY + 0x08, base + PARTNER_ENTRY_CALLBACK);
        fields::write_unchecked(base, PARTNER_ENTRY + 0x10, 0u64);
        fields::write_unchecked(base, PARTNER_ENTRY + 0x18, 0u32);
        release(base + first);
    }
    if fields::read_unchecked::<u8>(base, second) & 1 == 0 && acquire(base + second) != 0 {
        fields::write_unchecked(base, PARTNER_TABLE, base + PARTNER_ENTRY);
        fields::write_unchecked(base, PARTNER_TABLE + 0x08, 1u32);
        release(base + second);
    }
}

/// State 11's Pokémon Trainer task (`0x650fc4-0x6512c8`, `body_bank::trainer_task`): zeroed, with the holder, the
/// owner, queue id 1, the role's costume byte `FE+0x41` and the `FE+0x52`/`+0x53` flag (the live entry's: the role's
/// bytes, which the tick leaves as they are) and `+0x92 = 1`; its functor `{0x4f81498, task}` and the callback
/// `0x651770` scheduled as a body's are (`schedule_task`). Returns the task, or 0 (nothing queued).
unsafe fn queue_trainer(base: usize, entry: usize, owner: u32, holder: usize) -> usize {
    use construction_task as task;
    let heap_alloc: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(base + HEAP_ALLOC);
    let t = heap_alloc(task::ALIGN, trainer_task::SIZE);
    if t == 0 { return 0; }
    std::ptr::write_bytes(t as *mut u8, 0, trainer_task::SIZE);
    fields::write_unchecked(t, trainer_task::HOLDER, holder);
    fields::write_unchecked(t, trainer_task::OWNER, owner);
    fields::write_unchecked(t, task::QUEUE, task::QUEUE_ID);
    fields::write_unchecked(t, trainer_task::COSTUME, fields::read_unchecked::<u8>(entry, 0x41));
    fields::write_unchecked(t, trainer_task::FLAG,
        trainer_task::flag(fields::read_unchecked::<u8>(entry, 0x52), fields::read_unchecked::<u8>(entry, 0x53)));
    fields::write_unchecked(t, trainer_task::BUILD, 1u8);
    schedule_task(base, t, base + TRAINER_FUNCTOR_VTABLE, TRAINER_CALLBACK);
    t
}

/// Inbox 02:42Z: state 11's Pokémon Trainer task for a detached group's retired holder, run with the prepared build's
/// functor (`PREPARE_VTABLE`: `0x3a6270` only) and state 11's callback; the owner is the slot `0x3a6270` records, as the
/// trainer functor passes it (`ldr w4, [x21, #0x8]`). Returns the task, or 0 (nothing queued).
unsafe fn queue_prepare(base: usize, owner: u32, holder: usize) -> usize {
    use construction_task as task;
    let heap_alloc: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(base + HEAP_ALLOC);
    let t = heap_alloc(task::ALIGN, trainer_task::SIZE);
    if t == 0 { return 0; }
    std::ptr::write_bytes(t as *mut u8, 0, trainer_task::SIZE);
    fields::write_unchecked(t, trainer_task::HOLDER, holder);
    fields::write_unchecked(t, trainer_task::OWNER, owner);
    fields::write_unchecked(t, task::QUEUE, task::QUEUE_ID);
    schedule_task(base, t, PREPARE_VTABLE.as_ptr() as usize, TRAINER_CALLBACK);
    t
}

/// SPECIAL-GROUPS §3, native loader state 11 for a native group built in the background, once its bodies settled: the
/// holder with state 11's words, then for Pyra/Mythra the table guards and `0x3a6270(holder, main+0x52b94c8, 0, -1,
/// owner, 0)` here on the game thread (`0x6509e8-0x650abc`; about 2 ms on Eden), for Pokémon Trainer the 1-byte object
/// and the pool task on the worker (`queue_trainer`; its 196 ms ran on the game thread inside `0x658af0`, which is never
/// called here). The records go into the build and from there into the group's saved window, never onto the live entry,
/// whose `+0x5a88..+0x5a98` stay its own window's. A null allocation leaves its record 0 and the settlement fails the
/// group (reported).
unsafe fn start_holder(base: usize, entry: usize, owner: u32, b: &mut Background, step: HolderStep) {
    let heap_alloc: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(base + HEAP_ALLOC);
    let started = now_us();
    b.holder_due = true;
    let holder = heap_alloc(HOLDER_ALIGN, HOLDER_SIZE);
    if holder == 0 {
        trace!("[GroupSwap] background holder owner={} group={} UNEXPECTED: the holder allocation returned null", owner, b.group);
        return;
    }
    init_holder(base, holder);
    // X6: beside a live Mii window the step takes the group's ordinary words for its one call: Pyra/Mythra's pool build
    // reads `R` by owner on the game thread (the shadow's words, as the bodies had them), and Pokémon Trainer's task copies
    // the entry's costume and flag bytes (the baseline's); its worker part reads the shadow (`bg_trainer_enter`).
    let words = b.baseline.map(|_| save_resource_words(shadow_r_address()));
    match step {
        HolderStep::Partner => {
            partner_guards(base);
            let build: unsafe extern "C" fn(usize, usize, u32, u64, u32, u32) = std::mem::transmute(base + articles::POOL_BUILD);
            // `mov w3, #-1` (x3 = 0xffff_ffff) and the entry id in w4 (`0x650aa0-0x650abc`).
            with_words(b.alias, words, || {
                build(holder, base + PARTNER_TABLE, 0, u64::from(u32::MAX), owner, 0);
            });
            b.records = [holder, 0, 0];
        }
        HolderStep::Trainer => {
            let object = heap_alloc(HOLDER_ALIGN, trainer_task::OBJECT_SIZE);
            b.records = [holder, 0, object];
            let live = b.baseline.map(|ordinary| {
                let live = save_entry_identity(entry);
                write_entry_identity(entry, &ordinary);
                live
            });
            b.trainer = queue_trainer(base, entry, owner, holder);
            if let Some(live) = live { write_entry_identity(entry, &live); }
        }
    }
    trace!("[GroupSwap] background holder owner={} group={} step={:?} records={:x?} articles={} trainer_task={:#x} us={}",
        owner, b.group, step, b.records, fields::read::<u64>(holder, 0x10).unwrap_or(0), b.trainer, now_us().saturating_sub(started));
}

/// The construction tail's pair test (`cmp w8, #0x4b` after `ldr w8, [x9, #0x18]`, x9 the entry, x19 the body).
const BACKGROUND_TAIL: usize = 0x60de68;

/// Any body of an owner whose background build is in flight (`background_skip`'s key): the live window is built, so a
/// body that constructs for that owner meanwhile is the background build's.
#[inline(always)]
unsafe fn background_body(body: usize) -> bool {
    BACKGROUND_OWNERS.load(Ordering::Acquire) != 0 && aligned(body as *const u8)
        && fields::read::<u64>(body, 0x10).is_some_and(|owner| owner < 32
            && BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(owner as u32) != 0)
}

/// SPECIAL-GROUPS §0 X1: the construction tail reads the **live** cells 0/1 (`0x60de64`, `0x60de70`) on the worker. Beside
/// a live `[Popo, Nana]` window (a session that starts on Ice Climbers, or the pair group attached) a background body
/// would take the pair rule: `R+0x50` decremented under the lock and, at 0 after the window's own build, the whole-R reset
/// that drops every Prefetched group's keys. A background body sees no Popo in cell 0 instead, so its tail takes the
/// per-kind release `0x17e8610(R, kind)` at `0x60debc`, which returns exactly the keys its group's prefetch registered.
///
/// The one hook at `0x60de68` for every background body (primary 2026-11-08 22:52Z, 2026-11-09 02:24Z: `p23/rc6-bg-ic`
/// and `p23/rc6-hot-slots-mii` both hooked it): a background Mii body's `x20` is the shadow `R` from `0x60b98c`
/// (`bg_mii_construct_enter`), so it gets the real `R` back first, for the tail's lock, `R+0x50` and release
/// (SPECIAL-GROUPS §1 (b)); then the X1 rule.
#[skyline::hook(offset = 0x60de68, inline)]
unsafe fn background_tail(ctx: &mut InlineCtx) {
    if ctx.registers[20].x() as usize == shadow_r_address() {
        ctx.registers[20].set_x(MII_REAL_R.load(Ordering::Acquire) as u64);
        mii_hit(7);
    }
    if ctx.registers[8].w() == POPO && background_body(ctx.registers[19].x() as usize) {
        ctx.registers[8].set_x(u64::from(u32::MAX));
    }
}

/// The construction's info wrapper load (`ldr x9, [x9, #8]`, SPECIAL-GROUPS §0 X4).
const BACKGROUND_INFO: usize = 0x60d270;

/// A group's X4 words: count, cell 0 and cell 1 (each kind as 16 bits, `0xffff` for none), with a presence bit.
fn pack_cells(members: Members) -> u64 {
    let kinds = members.kinds();
    let word = |kind: u32| u64::from(kind.min(0xffff));
    1 << 48 | (members.count() as u64) << 32 | word(kinds[0]) << 16 | word(kinds[1])
}

/// The words `pack_cells` stored for `owner`'s building group.
fn background_cells(owner: u32) -> Option<(u32, [u32; 2])> {
    let packed = BACKGROUND_CELLS.get(owner as usize)?.load(Ordering::Acquire);
    if packed >> 48 == 0 { return None; }
    let kind = |word: u64| if word == 0xffff { u32::MAX } else { word as u32 };
    Some(((packed >> 32 & 0xffff) as u32, [kind(packed >> 16 & 0xffff), kind(packed & 0xffff)]))
}

/// SPECIAL-GROUPS §0 X4: construct (`0x60d1ec-0x60d26c`) picks a Popo or Nana body's info wrapper from the **live**
/// entry's count and cells, and `0x60d270` loads the info whose `+0x94` gate it reads. Beside a live one-actor window (an
/// ordinary or native window) a background pair group's Nana would get `entry+0xf0`. The building group's words choose
/// instead (`body_bank::info_wrapper`), as in her synchronous build: Nana gets her own `entry+0x100`, but only while
/// the entry holds that wrapper's info (`+0x108`); otherwise native's choice stays and is counted. x9 is the chosen
/// wrapper, x8 the manager, x23 the owner, w21 the kind, x19 the body. Atomics only: no allocation on the worker.
#[skyline::hook(offset = 0x60d270, inline)]
unsafe fn background_info(ctx: &mut InlineCtx) {
    let kind = ctx.registers[21].w();
    if !matches!(kind, POPO | NANA) || !background_body(ctx.registers[19].x() as usize) { return; }
    let owner = ctx.registers[23].x() as u32;
    let Some((count, cells)) = background_cells(owner) else { return; };
    let offset = info_wrapper(kind, count, cells);
    let Some(entry) = fields::read::<usize>(ctx.registers[8].x() as usize, 0x20 + 8 * owner as usize) else { return; };
    if offset == 0x100 && !fields::read::<usize>(entry, 0x108).is_some_and(|info| info != 0 && info % 8 == 0) {
        BACKGROUND_INFO_KEPT.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if offset == 0x100 { BACKGROUND_INFO_OWN.fetch_add(1, Ordering::Relaxed); }
    ctx.registers[9].set_x((entry + offset) as u64);
}

/// The group a background build starts next (`body_bank::next_background`) for the live kind `live`: none while a
/// prefetch, a construction or a background build is in flight. A refused group is no candidate.
fn background_candidate(bank: &Bank, live: Option<u32>) -> Option<usize> {
    if !BACKGROUND_ENABLED.load(Ordering::Acquire) || bank.prefetch.is_some() || bank.construction.is_some()
        || bank.background.is_some() { return None; }
    next_background(bank.groups.iter().map(|g| (if g.background_refused || g.leaving || !background_kind_ready(bank, g) { GroupState::Failed } else { g.state },
        *g.members, g.identity.is_some())), *bank.members, live_identity(bank),
        live == Some(KIRBY), BACKGROUND_PAIR_TAIL.load(Ordering::Acquire), BACKGROUND_X4.load(Ordering::Acquire))
}

/// X6: the live window's Mii identity and whether the owner's ordinary baseline is known.
fn live_identity(bank: &Bank) -> LiveIdentity {
    LiveIdentity::of(bank.identity.is_some(), bank.ordinary_identity.is_some())
}

/// A Mii group builds in the background only with its hooks installed (`BACKGROUND_MII_ENABLED`), and so does every
/// group beside a live Mii window (X6: its worker reads the shadow `R` through the same hooks).
fn background_kind_ready(bank: &Bank, group: &Group) -> bool {
    (group.members.special() != Special::Mii && bank.identity.is_none()) || BACKGROUND_MII_ENABLED.load(Ordering::Acquire)
}

/// A Prefetched group a background build still owes: the notice counts it as loading (`progress`).
fn background_owed(bank: &Bank, group: &Group, live: Option<u32>) -> bool {
    BACKGROUND_ENABLED.load(Ordering::Acquire) && group.state == GroupState::Prefetched && !group.background_refused
        && !group.leaving && background_kind_ready(bank, group) && background_build_eligible(*group.members, group.identity.is_some(),
            *bank.members, live_identity(bank), live == Some(KIRBY),
            BACKGROUND_PAIR_TAIL.load(Ordering::Acquire), BACKGROUND_X4.load(Ordering::Acquire))
            .is_ok()
}

const INFO_BYTES: usize = BACKGROUND_INFO_SPANS[0].1 + BACKGROUND_INFO_SPANS[1].1;

unsafe fn save_info(info: usize) -> [u8; INFO_BYTES] {
    let mut out = [0u8; INFO_BYTES];
    let mut at = 0;
    for (offset, len) in BACKGROUND_INFO_SPANS {
        for i in 0..len { out[at + i] = fields::read_unchecked::<u8>(info, offset + i); }
        at += len;
    }
    out
}

unsafe fn restore_info(info: usize, saved: &[u8; INFO_BYTES]) {
    let mut at = 0;
    for (offset, len) in BACKGROUND_INFO_SPANS {
        for i in 0..len { fields::write_unchecked(info, offset + i, saved[at + i]); }
        at += len;
    }
}

/// `info+0x3a0..0x3b0`: body init writes `+0x3a0..0x3ab` from the worker (`0x607eb4`, from `info+0x72`), so the live
/// window's words are compared (and put back) when the build settles.
unsafe fn info_words(info: usize) -> [u32; 4] {
    std::array::from_fn(|i| fields::read_unchecked::<u32>(info, 0x3a0 + i * 4))
}

/// The listener count of each dispatcher array (`[arr]`, the ring size), for the build line.
unsafe fn listener_counts(arrays: [usize; 2]) -> [i64; 2] {
    arrays.map(|a| if a == 0 { -1 } else { fields::read::<u64>(a, 0).map_or(-1, |n| n as i64) })
}

/// A background build in flight: the group's bodies under construction on the task worker.
struct Background {
    group: usize,
    members: Members,
    token: super::InstallToken,
    alias: usize,
    lease: Lease,
    /// The group's window fields as the entry-level third left them; its selected context comes from the scratch's
    /// control init, its cells at the settlement.
    window: Window,
    /// The control init's task on the scratch entry (state 8) until it settled (state 9); then 0 and the bodies are
    /// queued (state 10).
    control: usize,
    control_us: u64,
    tasks: [usize; NATIVE_MEMBERS],
    /// Members whose task was queued (fewer than `members.count()` only when a native allocation returned null).
    queued: usize,
    live_before: u16,
    committed: Option<u64>,
    /// The live window's `info+0x3a0..0x3b0` when the tick ended.
    info_words: [u32; 4],
    skips_before: u32,
    started_us: u64,
    setup_us: u64,
    settled_us: [u64; NATIVE_MEMBERS],
    ticks: u32,
    waited_logged_us: u64,
    /// SPECIAL-GROUPS §3: a native group's holder step ran once its bodies settled (`start_holder`); the records it made
    /// go into the group's saved window, and Pokémon Trainer's pool task runs on the worker until it settled (0 otherwise).
    holder_due: bool,
    records: [usize; 3],
    trainer: usize,
    /// A Mii group's build (SPECIAL-GROUPS §1): the face handles its tick's setup made, and its values for the hooks.
    mii: Option<MiiBuild>,
    /// X6: a group built beside a live Mii window: the owner's ordinary baseline its tick wrote (entry bytes, mirror; its
    /// `R` words are the shadow's), which its worker, holder step and key release take instead of the live Mii's.
    baseline: Option<EntryIdentity>,
    /// The pad manager's per-slot controllers when the build started (diagnostics, `pad_controllers`).
    pads: [usize; 16],
}

#[derive(Clone, Copy, Debug)]
struct MiiBuild { faces: [u32; 2], values: u32 }

fn refuse_background(role: usize, group: usize, owner: u32, why: &str) {
    let mut state = BANKS[role].lock();
    let Some(g) = state.as_mut().and_then(|bank| bank.groups.get_mut(group)) else { return; };
    if !g.background_refused {
        g.background_refused = true;
        trace!("[GroupSwap] background build declined owner={} group={} kinds={:x?} reason={}; its first switch builds it",
            owner, group, &g.members.kinds()[..g.members.count()], why);
    }
}

/// `queue_bodies` for a background build: the same allocations and tasks, never an entry cell. Returns the tasks and
/// how many members were queued (fewer when a native allocation returned null; a body allocated without its task is
/// reported and left).
unsafe fn queue_detached(base: usize, owner: u32, members: Members) -> ([usize; NATIVE_MEMBERS], usize) {
    let allocate: unsafe extern "C" fn(u8, u32, u32) -> usize = std::mem::transmute(base + BODY_ALLOC);
    let mut tasks = [0usize; NATIVE_MEMBERS];
    for (i, kind) in members.kinds().iter().copied().enumerate().take(members.count()) {
        let body = allocate(0, kind, owner);
        if body == 0 { return (tasks, i); }
        let t = queue_task(base, body, CONSTRUCT_FUNCTOR_VTABLE, TASK_CALLBACK);
        if t == 0 {
            trace!("[GroupSwap] background build owner={} UNEXPECTED: task allocation returned null; body {:#x} left", owner, body);
            return (tasks, i);
        }
        tasks[i] = t;
    }
    (tasks, members.count())
}

/// Start a background build of `group` (`background_candidate`) in this tick of the live window.
unsafe fn start_background(entry: *mut u8, owner: u32, role: usize, group: usize) {
    let base = BASE.load(Ordering::Acquire);
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    if !groups_enabled(&settings) || super::INSTALL.active() { return; }
    if !crate::runtime::roster::scope_gate::admitted("native_body_bank::group_prefetch") { return; }
    let (members, alias, carries_records, mii, baseline) = {
        let state = BANKS[role].lock();
        let Some(bank) = state.as_ref().filter(|bank| bank.entry == entry as usize && bank.owner == owner
            && bank.phase == Phase::Ready && !bank.departed && bank.swap.is_none()) else { return; };
        if background_candidate(bank, bank.observed.map(|(_, kind)| kind)) != Some(group) { return; }
        let g = &bank.groups[group];
        (*g.members, bank.alias, bank.members.carries_records(),
            g.identity.zip(g.configuration).filter(|_| g.members.special() == Special::Mii),
            // X6: beside a live Mii window (the candidate rule admits only a non-Mii group there, with a baseline).
            bank.ordinary_identity.filter(|_| bank.identity.is_some() && g.identity.is_none()))
    };
    // SPECIAL-GROUPS §3: a native group needs the X5 hook and state 11's holder step (`install`); without them its first
    // switch builds it, as before.
    if members.carries_records() && !NATIVE_BACKGROUND.load(Ordering::Acquire) {
        refuse_background(role, group, owner, "native rows off");
        return;
    }
    // Transient: retried on a later tick.
    if alias == 0 || resource(base, owner) != Some(alias) || !resource_ready(alias)
        || !window_shape_ok(base, entry as usize, owner, carries_records) { return; }
    let Some(info) = entry_info(entry as usize) else { return; };
    if pool_counts(base).is_none_or(|(capacity, live)| (capacity as usize).saturating_sub(usize::from(live)) < members.count()) {
        refuse_background(role, group, owner, "fighter pool");
        return;
    }
    // A Mii group's setup changes a selector record both owner adapters snapshot: only while neither holds a
    // transaction (transient: retried on a later tick).
    if mii.is_some() && !(super::mii_ordinary_player::background_idle() && super::mii_ordinary_cpu::background_idle()) {
        return;
    }
    let Some(token) = super::INSTALL.try_begin(false) else { return; };
    let started = now_us();
    let pad_before = pad_controllers(base);
    let live_before = pool_counts(base).map_or(0, |(_, live)| live);
    let committed = MemorySample::capture().committed.get();
    let info_saved = save_info(info);
    // The group's table into the window for the table hooks (`create_contexts`, `select_context`), as an exchange lends
    // it; the live window's fields first.
    let live = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut() else { super::finish_install(token, "background build: bank gone"); return; };
        let live = save_window(entry as usize);
        std::mem::swap(&mut bank.members, &mut bank.groups[group].members);
        live
    };
    let lrole = ledger_role(role);
    // `R+0x4c` bit 0 is the construction's (the path builder `0x17e99c0` gates on it), until the tasks settle (U19).
    let (lease, _) = r_handle::request_bit(lrole, alias);
    detach_dispatcher(entry as usize);
    // `0x655320` writes only the group's context words: the others must not keep the live window's.
    for offset in [0x4178usize, 0x4180, 0x4188, 0x4190, 0x4198] { fields::write_unchecked(entry as usize, offset, 0usize); }
    write_group_words(entry as usize, members);
    fields::write_unchecked(entry as usize, 0x5918, 0u8);
    // SPECIAL-GROUPS §1 (a): a Mii group's identity in `R`, the entry and the D mirror for this tick only, the live
    // window's face handles held aside (the setup's bind releases `R+0x90/+0x94`, `0x67e9b0`), then the native setup
    // (`0x67e7d0`, `0x67ead0`) as Phase B runs it before the construction.
    let mii_tick = mii.map(|(identity, c)| {
        let words = save_resource_words(alias);
        let identity_bytes = save_entry_identity(entry as usize);
        let live_faces: [u32; 2] = [fields::read_unchecked(alias, 0x90), fields::read_unchecked(alias, 0x94)];
        fields::write_unchecked(alias, 0x90, 0u32);
        fields::write_unchecked(alias, 0x94, 0u32);
        write_identity(entry as usize, alias, identity);
        let (outcome, change) = crate::runtime::roster::mii_setup::prepare_mii_background(base, entry as usize, owner, alias, &identity, &c);
        // The setup published the group's record pair (global, `0x3759500`); both owner adapters snapshot that record
        // (their own destination, the other's protected record) and adopt the change, as a switch-time build's settle
        // does. They are idle (checked before INSTALL), so nothing else changed meanwhile.
        if let Some(change) = change.filter(|c| c.before != c.after) {
            let player = super::mii_ordinary_player::adopt_background_record(change.index, &change.before, &change.after);
            let cpu = super::mii_ordinary_cpu::adopt_background_record(change.index, &change.before, &change.after);
            trace!("[GroupSwap] background Mii owner={} record={} pair adopted by the adapters player={} cpu={}",
                owner, change.index, player, cpu);
        }
        let faces: [u32; 2] = [fields::read_unchecked(alias, 0x90), fields::read_unchecked(alias, 0x94)];
        let values = u32::from(identity.index) | u32::from(read::<u8>(entry, 0x41)) << 16 | u32::from(read::<u8>(entry, 0x52)) << 24;
        (words, identity_bytes, live_faces, outcome, faces, values)
    });
    // X6 (SPECIAL-GROUPS §0 X3 for every body): beside a live Mii window any other group's tick takes the owner's ordinary
    // baseline in `R` (its own header kinds, `baseline_words`), the entry and the D mirror, the words its warm exit writes,
    // so `0x654dc0` copies the ordinary bytes and the shadow below carries the ordinary costume and flags.
    let baseline_tick = baseline.map(|(words, ordinary)| {
        let live = (save_resource_words(alias), save_entry_identity(entry as usize));
        write_resource_words(alias, &baseline_words(words, members));
        write_entry_identity(entry as usize, &ordinary);
        live
    });
    let mii_prepared = mii_tick.as_ref().is_none_or(|t| matches!(t.3, crate::engine::mii::mii_setup::Outcome::Prepared { .. }));
    let models: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + MODELS_SETUP);
    let contexts: unsafe extern "C" fn(*mut u8) = std::mem::transmute(base + COUNT_SETUP);
    let t0 = now_us();
    if mii_prepared { models(entry); }
    let t1 = now_us();
    if mii_prepared { contexts(entry); }
    let t2 = now_us();
    let steps_us = [t1.saturating_sub(t0), t2.saturating_sub(t1)];
    // A live native window's records stay in the entry through the tick: the group's window takes none from it (a native
    // group's own come from its holder step, `start_holder`).
    let mut made = save_window(entry as usize);
    made.records = [0; 3];
    // The control init's object: the entry as the group's third left it (`Scratch`).
    let scratch = scratch_address();
    std::ptr::copy_nonoverlapping(entry as *const u8, scratch as *mut u8, 0x100);
    std::ptr::copy_nonoverlapping((entry as usize + 0x4178) as *const u8, (scratch + 0x4178) as *mut u8, 0x28);
    fields::write_unchecked(scratch, 0x5918, 0u8);
    fields::write_unchecked(scratch, 0x591f, made.flags);
    // The worker's `R` for the Mii body (SPECIAL-GROUPS §1 (b)): `R+0x40..+0xa8` with the group's words and the build's
    // request bit, no resource object yet; then the live window's words, entry bytes, mirror and faces come back.
    if let Some((words, identity_bytes, live_faces, ..)) = mii_tick.as_ref() {
        let shadow = shadow_r_address();
        std::ptr::copy_nonoverlapping((alias + 0x40) as *const u8, (shadow + 0x40) as *mut u8, 0x68);
        fields::write_unchecked(shadow, 0x98, 0usize);
        fields::write_unchecked(shadow, 0xa0, 0usize);
        // X5 (primary 2026-11-09 02:42Z): the live window beside which the Mii body builds, and what its `R` lends the
        // shadow (the pair references, `R_PAIR_REFERENCES`; the object kind `R+0x80`).
        trace!("[GroupSwap] background Mii owner={} live_window={:x?} r50={:#x} r80={:#x} shadow_r80={:#x}", owner, &words[..3],
            fields::read_unchecked::<u32>(alias, R_PAIR_REFERENCES), words[10], fields::read_unchecked::<u32>(shadow, 0x80));
        write_resource_words(alias, words);
        write_entry_identity(entry as usize, identity_bytes);
        fields::write_unchecked(alias, 0x90, live_faces[0]);
        fields::write_unchecked(alias, 0x94, live_faces[1]);
    }
    // X6: the worker's `R` for every body of a group built beside a live Mii window: `R+0x40..+0xa8` with the baseline
    // words and the build's request bit, none of the live Mii's faces or resource object; then the live Mii's words, entry
    // bytes and mirror come back.
    if let Some((live_words, live_bytes)) = baseline_tick.as_ref() {
        let shadow = shadow_r_address();
        std::ptr::copy_nonoverlapping((alias + 0x40) as *const u8, (shadow + 0x40) as *mut u8, 0x68);
        for offset in [0x90usize, 0x98, 0xa0] { fields::write_unchecked(shadow, offset, 0usize); }
        trace!("[GroupSwap] background baseline tick owner={} group={} kinds={:x?} live_window={:x?} shadow={:x?}", owner, group,
            &members.kinds()[..members.count()], &live_words[..4], save_resource_words(shadow));
        write_resource_words(alias, live_words);
        write_entry_identity(entry as usize, live_bytes);
    }
    let table = {
        let mut state = BANKS[role].lock();
        state.as_mut().map(|bank| {
            std::mem::swap(&mut bank.members, &mut bank.groups[group].members);
            bank.groups[group].members.table().as_ptr() as usize
        })
    };
    restore_window(entry as usize, &live);
    fields::write_unchecked(entry as usize, 0x5918, live.index);
    fields::write_unchecked(entry as usize, 0x5919, live.previous);
    fields::write_unchecked(entry as usize, 0x5910, live.old);
    restore_info(info, &info_saved);
    let setup_us = now_us().saturating_sub(started);
    let count = members.count();
    let contexts_ok = (0..NATIVE_MEMBERS).all(|i|
        if i < count { valid_context(base, made.group_ctx[i], owner) } else { made.group_ctx[i] == 0 });
    // `0x655320` leaves `+0x4178/+0x4180` zero; the select (`0x64fb70`) picks the context on the scratch, later.
    let made_ok = contexts_ok && made.ctx == [0, 0] && made.flags == members.expected_flags(0)
        && made.dispatch != live.dispatch && made.dispatch.iter().all(|a| dispatcher_array_ok(*a));
    let live_ok = dispatcher_present(entry as usize) && dispatcher_arrays(entry as usize) == Some(live.dispatch)
        && read::<usize>(entry, 0x4178) == live.ctx[0] && read::<u64>(entry, 0x4150) == live.installed;
    let refuse = |why: &str, made: &Window| {
        let (how, now, after) = r_handle::end_request(lrole, alias, lease);
        super::finish_install(token, "background build declined");
        trace!("[GroupSwap] background build owner={} group={} UNEXPECTED: {} made_ok={} live_ok={} contexts={:x?} ctx={:x?} flags={:#x} dispatch={:x?} live_dispatch={:x?} request_bit={:?} r4c={:#x}->{:#x}; contexts and arrays left (reported)",
            owner, group, why, made_ok, live_ok, made.group_ctx, made.ctx, made.flags, made.dispatch, dispatcher_arrays(entry as usize), how, now, after);
        refuse_background(role, group, owner, why);
    };
    let mii_build = mii_tick.map(|(_, _, _, _, faces, values)| MiiBuild { faces, values });
    if !mii_prepared {
        // `[MiiSetup]` logged its reason; whatever faces the setup made go back.
        if let Some(m) = mii_build { release_mii_r(base, MiiR { faces: m.faces, object: [0, 0] }); }
        refuse("Mii setup not prepared", &made);
        return;
    }
    let Some(table) = table.filter(|_| made_ok && live_ok) else {
        if let Some(m) = mii_build { release_mii_r(base, MiiR { faces: m.faces, object: [0, 0] }); }
        refuse("setup check failed", &made);
        return;
    };
    BACKGROUND_TABLE.store(table, Ordering::Release);
    let control = queue_task(base, scratch, CONTROL_FUNCTOR_VTABLE, CONTROL_CALLBACK);
    if control == 0 {
        BACKGROUND_TABLE.store(0, Ordering::Release);
        if let Some(m) = mii_build { release_mii_r(base, MiiR { faces: m.faces, object: [0, 0] }); }
        refuse("control task allocation returned null", &made);
        return;
    }
    let queued_us = now_us().saturating_sub(started);
    let skips_before = BACKGROUND_SKIPS.load(Ordering::Acquire);
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut() else {
        // Not reachable on the game thread; the task still runs on the scratch (reported).
        super::finish_install(token, "background build: bank gone");
        trace!("[GroupSwap] background build owner={} group={} UNEXPECTED: bank gone after queuing; control task {:#x} left", owner, group, control);
        return;
    };
    bank.background = Some(Background { group, members, token, alias, lease, window: made, control, control_us: 0,
        tasks: [0; NATIVE_MEMBERS], queued: 0, live_before, committed, info_words: info_words(info), skips_before,
        started_us: started, setup_us, settled_us: [0; NATIVE_MEMBERS], ticks: 0, waited_logged_us: started, mii: mii_build,
        holder_due: false, records: [0; 3], trainer: 0, pads: pad_before, baseline: baseline.map(|(_, ordinary)| ordinary) });
    trace!("[GroupSwap] background build owner={} group={} kinds={:x?} special={:?} mii={:x?} setup_us={} models_contexts_us={:?} us={} control={:#x} contexts={:x?} dispatch={:x?} listeners={:?} live_dispatch={:x?} live_listeners={:?} flags={:#x} R={:#x} map={} r4c={:#x} pool_live={}; the window stays live, the control init runs on the task worker",
        owner, group, &members.kinds()[..count], members.special(), mii_build, setup_us, steps_us, queued_us, control, &made.group_ctx[..count], made.dispatch,
        listener_counts(made.dispatch), live.dispatch, listener_counts(live.dispatch), made.flags, alias,
        fields::read::<u64>(alias, 0x18).unwrap_or(0), fields::read::<u8>(alias, 0x4c).unwrap_or(0), live_before);
}

/// State 9 and 10 of a background build: once the scratch's control init settled, its selected context becomes the
/// group window's and the bodies are allocated and queued (`queue_detached`), the owner's registrations skipped from
/// then on. `false` when the selection is not the group's first context: the build is refused (nothing allocated).
unsafe fn queue_background_bodies(base: usize, owner: u32, b: &mut Background) -> bool {
    let selected = fields::read::<usize>(scratch_address(), 0x4178).unwrap_or(0);
    BACKGROUND_TABLE.store(0, Ordering::Release);
    if selected == 0 || selected != b.window.group_ctx[0] { return false; }
    b.window.ctx[0] = selected;
    b.skips_before = BACKGROUND_SKIPS.load(Ordering::Acquire);
    // SPECIAL-GROUPS §0 X5: a Pyra/Mythra group's bodies read its cell 1 (`background_partner`).
    BACKGROUND_PARTNER.store(b.members.partner_cell().unwrap_or(u32::MAX), Ordering::Release);
    if let Some(cells) = BACKGROUND_CELLS.get(owner as usize) { cells.store(pack_cells(b.members), Ordering::Release); }
    BACKGROUND_OWNERS.fetch_or(owner_bit(owner), Ordering::AcqRel);
    // X6: a group built beside a live Mii window arms the same hooks with the baseline's values (`shadow_values`).
    if let Some(m) = b.mii.or(b.baseline.map(|ordinary| MiiBuild { faces: [0; 2], values: shadow_values(&ordinary.entry) })) {
        // The Mii hooks key on these; the real R last, as the arming store.
        for hit in &MII_HITS { hit.store(0, Ordering::Relaxed); }
        MII_VALUES.store(m.values, Ordering::Release);
        MII_OWNER.store(owner, Ordering::Release);
        MII_REAL_R.store(b.alias, Ordering::Release);
    }
    let (tasks, queued) = queue_detached(base, owner, b.members);
    b.tasks = tasks;
    b.queued = queued;
    true
}

/// A background build's tasks, polled on the window's ticks (state 0xb's rule); settled once every pending count is 0.
unsafe fn settle_background(entry: *mut u8, owner: u32, role: usize) {
    use construction_task as task;
    let base = BASE.load(Ordering::Acquire);
    let now = now_us();
    // State 9: the scratch's control init on the worker; once it settled, state 10 here (outside the bank lock, as the
    // allocation runs native code that may reach the table hooks).
    let control = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        let Some(b) = bank.background.as_mut() else { return; };
        if b.control == 0 {
            None
        } else {
            b.ticks = b.ticks.saturating_add(1);
            let pending = fields::read::<i32>(b.control, task::PENDING).unwrap_or(i32::MAX);
            if pending > 0 {
                if now.saturating_sub(b.waited_logged_us) >= 5_000_000 {
                    b.waited_logged_us = now;
                    trace!("[GroupSwap] background control init pending owner={} group={} pending={} us={} ticks={}",
                        owner, b.group, pending, now.saturating_sub(b.started_us), b.ticks);
                }
                return;
            }
            let control = b.control;
            b.control = 0;
            b.control_us = now.saturating_sub(b.started_us);
            Some((control, bank.background.take().unwrap()))
        }
    };
    if let Some((control, mut b)) = control {
        free_tasks(base, &[control]);
        let alloc_started = now_us();
        let queued = queue_background_bodies(base, owner, &mut b);
        let alloc_us = now_us().saturating_sub(alloc_started);
        if !queued {
            finish_background(entry, owner, role, b, "control init selected another context", 0);
            return;
        }
        let mut state = BANKS[role].lock();
        let (group, queued, tasks, control_us) = (b.group, b.queued, b.tasks, b.control_us);
        match state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) {
            Some(bank) => bank.background = Some(b),
            None => {
                drop(state);
                trace!("[GroupSwap] background build owner={} group={} UNEXPECTED: bank gone after the control init; tasks {:x?} left", owner, group, &tasks[..queued]);
                return;
            }
        }
        trace!("[GroupSwap] background bodies queued owner={} group={} tasks={:x?} control_us={} alloc_us={}", owner, group, &tasks[..queued], control_us, alloc_us);
        return;
    }
    let background = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        let Some(b) = bank.background.as_mut() else { return; };
        b.ticks = b.ticks.saturating_add(1);
        // SPECIAL-GROUPS §3: Pokémon Trainer's pool task, polled as the bodies are (state 11's `task+0x20`).
        if b.trainer != 0 {
            let pending = fields::read::<i32>(b.trainer, task::PENDING).unwrap_or(i32::MAX);
            if pending > 0 {
                if now.saturating_sub(b.waited_logged_us) >= 5_000_000 {
                    b.waited_logged_us = now;
                    trace!("[GroupSwap] background trainer pool pending owner={} group={} pending={} us={} ticks={}",
                        owner, b.group, pending, now.saturating_sub(b.started_us), b.ticks);
                }
                return;
            }
        } else {
            let mut pending = [0i32; NATIVE_MEMBERS];
            for (i, t) in b.tasks.iter().enumerate().take(b.queued) {
                pending[i] = fields::read::<i32>(*t, task::PENDING).unwrap_or(i32::MAX);
                if pending[i] <= 0 && b.settled_us[i] == 0 { b.settled_us[i] = now.saturating_sub(b.started_us); }
            }
            if !task::settled(&pending[..b.queued]) {
                if now.saturating_sub(b.waited_logged_us) >= 5_000_000 {
                    b.waited_logged_us = now;
                    trace!("[GroupSwap] background constructing owner={} group={} pending={:?} us={} ticks={}",
                        owner, b.group, &pending[..b.queued], now.saturating_sub(b.started_us), b.ticks);
                }
                return;
            }
        }
        bank.background.take().unwrap()
    };
    let mut background = background;
    // SPECIAL-GROUPS §3: every body settled; a native group takes native state 11's holder step now (outside the bank
    // lock). Pokémon Trainer's pool task then runs on the worker and settles on a later tick.
    if let Some(step) = background.members.holder_step()
        .filter(|_| !background.holder_due && background.queued == background.members.count()) {
        start_holder(base, entry as usize, owner, &mut background, step);
        if background.trainer != 0 {
            let mut state = BANKS[role].lock();
            if let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) {
                bank.background = Some(background);
                return;
            }
            drop(state);
            // Not reachable on the game thread (as after the control init); the task still runs on its holder.
            trace!("[GroupSwap] background holder owner={} UNEXPECTED: bank gone with the trainer task {:#x} queued; left (reported)",
                owner, background.trainer);
            return;
        }
    }
    finish_background(entry, owner, role, background, "settled", 0);
}

/// A destroyer (finalize, reload, scene end) while a background build is in flight: wait for its tasks as
/// `drain_construction` waits (`0x354ccd0`), then finish it into its group record, which the caller destroys.
unsafe fn drain_background(entry: *const u8, owner: u32, role: usize, reason: &str) {
    use construction_task as task;
    let base = BASE.load(Ordering::Acquire);
    let background = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        match bank.background.take() { Some(background) => background, None => return }
    };
    let wait: unsafe extern "C" fn(u32, usize) = std::mem::transmute(base + TASK_WAIT);
    let mut waited = 0usize;
    let mut background = background;
    // State 9 at a destroyer: wait for the scratch's control init, then state 10, so the wait below finishes the
    // construction and the group record holds every body (`drain_construction`'s rule).
    if background.control != 0 {
        let control = std::mem::replace(&mut background.control, 0);
        if fields::read::<i32>(control, task::PENDING).unwrap_or(0) > 0 {
            let queue = fields::read::<u32>(control, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
            if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
                wait(handle, control + task::PENDING);
                waited += 1;
            }
        }
        free_tasks(base, &[control]);
        if !queue_background_bodies(base, owner, &mut background) {
            finish_background(entry, owner, role, background, "drained: control init selected another context", waited);
            return;
        }
    }
    for &t in background.tasks.iter().take(background.queued) {
        if fields::read::<i32>(t, task::PENDING).unwrap_or(0) > 0 {
            let queue = fields::read::<u32>(t, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
            if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
                wait(handle, t + task::PENDING);
                waited += 1;
            }
        }
    }
    // SPECIAL-GROUPS §3: Pokémon Trainer's pool task, once queued, finishes too; its holder goes with the group record the
    // caller destroys. A native group whose bodies settle only here takes no holder step: the teardown that follows
    // returns its object keys with the rest.
    if background.trainer != 0 && fields::read::<i32>(background.trainer, task::PENDING).unwrap_or(0) > 0 {
        let queue = fields::read::<u32>(background.trainer, task::QUEUE).unwrap_or(task::QUEUE_ID) as usize;
        if let Some(handle) = fields::child(base, TASK_QUEUES + queue * 8).and_then(|q| fields::read::<u32>(q, 0x10)) {
            wait(handle, background.trainer + task::PENDING);
            waited += 1;
        }
    }
    finish_background(entry, owner, role, background, reason, waited);
}

/// The settlement: the bodies into the group record (Built, with the window the tick made), the tasks freed, the
/// request bit handed back, the ledger settled and INSTALL released. A body that fails its proof fails the group: its
/// bodies, contexts and arrays are left and reported (leak rather than corrupt).
unsafe fn finish_background(entry: *const u8, owner: u32, role: usize, b: Background, reason: &str, waited: usize) {
    use construction_task as task;
    let base = BASE.load(Ordering::Acquire);
    BACKGROUND_OWNERS.fetch_and(!owner_bit(owner), Ordering::AcqRel);
    BACKGROUND_PARTNER.store(u32::MAX, Ordering::Release);
    let (own, kept) = (BACKGROUND_INFO_OWN.swap(0, Ordering::AcqRel), BACKGROUND_INFO_KEPT.swap(0, Ordering::AcqRel));
    if own != 0 || kept != 0 {
        trace!("[GroupSwap] background info owner={} kinds={:x?} nana_own_wrapper={} kept_no_info={}", owner,
            &b.members.kinds()[..b.members.count()], own, kept);
    }
    // SPECIAL-GROUPS §1: the Mii hooks off (every task settled); the resource object body init stored in the shadow
    // (`0x607d18`, with its reference) and the tick's faces are the group's Mii set.
    let mii_hits: [u32; 15] = std::array::from_fn(|i| MII_HITS[i].load(Ordering::Relaxed));
    // X6: a group built beside a live Mii window disarms them too; its bodies stored no Mii object in the shadow.
    if b.baseline.is_some() {
        MII_REAL_R.store(0, Ordering::Release);
        MII_OWNER.store(u32::MAX, Ordering::Release);
        MII_THREAD.store(0, Ordering::Release);
    }
    let mii_set = b.mii.map(|m| {
        MII_REAL_R.store(0, Ordering::Release);
        MII_OWNER.store(u32::MAX, Ordering::Release);
        MII_THREAD.store(0, Ordering::Release);
        let shadow = shadow_r_address();
        let object = [fields::read_unchecked::<usize>(shadow, 0x98), fields::read_unchecked::<usize>(shadow, 0xa0)];
        fields::write_unchecked(shadow, 0x98, 0usize);
        fields::write_unchecked(shadow, 0xa0, 0usize);
        MiiR { faces: m.faces, object }
    });
    let skips = BACKGROUND_SKIPS.load(Ordering::Acquire).wrapping_sub(b.skips_before);
    let count = b.members.count();
    let kinds = &b.members.kinds()[..count];
    let mut bodies = [Body::default(); NATIVE_MEMBERS];
    for i in 0..b.queued {
        let address = fields::read::<usize>(b.tasks[i], task::BODY).unwrap_or(0);
        bodies[i] = Body { address, id: fields::read::<u32>(address, 8).unwrap_or(0) };
    }
    free_tasks(base, &b.tasks[..b.queued]);
    free_tasks(base, &[b.trainer]);
    let valid = b.queued == count && (0..count).all(|i| valid_parked_body(base, bodies[i], kinds[i], owner)
        && fields::child(bodies[i].address, 0x20).is_some());
    // SPECIAL-GROUPS §3: the holder step's records, proven as `window_shape_ok` proves them at the warm attach (PT's pool
    // task recorded the owner at `+0x230`, `0x3a6270`); a native group drained before its step keeps none, and the
    // teardown that drained it follows. A holder that fails fails the group (left and reported).
    let holder_ok = !b.holder_due || (valid_special_object(base, b.records[0], owner)
        && (b.members.holder_step() != Some(HolderStep::Trainer) || b.records[2] != 0));
    // SPECIAL-GROUPS §0 X2: body init wrote each body's cell byte (`body+0xf816`, `0x607a84` and `0x60b5bc`) as the kind's
    // index in the live window, or 0xff; native writes its index in its own window, which is what it gets here.
    let mut cells_before = [0xffu8; NATIVE_MEMBERS];
    if valid {
        for (i, body) in bodies.iter().enumerate().take(count) {
            cells_before[i] = fields::read_unchecked::<u8>(body.address, BODY_CELL_BYTE);
            fields::write_unchecked(body.address, BODY_CELL_BYTE, i as u8);
        }
    }
    // Body init wrote these from the worker (the same values for every ordinary kind); the window's go back.
    let info = entry_info(entry as usize);
    let info_now = info.map(|info| info_words(info));
    let info_same = info_now == Some(b.info_words);
    if let (false, Some(info)) = (info_same, info) {
        // Only the three words body init writes; `+0x3ac` stays the game's.
        for (i, word) in b.info_words.iter().enumerate().take(3) { fields::write_unchecked(info, 0x3a0 + i * 4, *word); }
    }
    let alias_ok = b.alias != 0 && resource(base, owner) == Some(b.alias);
    let lrole = ledger_role(role);
    let request = alias_ok.then(|| r_handle::end_request(lrole, b.alias, b.lease));
    // Each queued body ran its construction tail (its kind's keys); a member never queued still holds its keys. A Mii
    // body's tail released nothing (`bg_mii_tail_release`): every key of the group goes here, its identity in R. So does
    // every key of a group built beside a live Mii window (X6), with the baseline in R; its object kind among them, after
    // the holder step that read through it.
    let mut mii_keys = None;
    if alias_ok && b.mii.is_some() {
        let occupancy = occupancy_snapshot(role, None);
        mii_keys = Some(release_group_keys_of(base, role, b.alias, b.group, &occupancy, owner, "background Mii build"));
    } else if alias_ok && b.baseline.is_some() {
        let occupancy = occupancy_snapshot(role, None);
        mii_keys = Some(release_group_keys_of(base, role, b.alias, b.group, &occupancy, owner, "background build beside a Mii window"));
    } else {
        if alias_ok && b.queued > 0 { r_handle::construction_settled(lrole, b.group, &kinds[..b.queued]); }
        if alias_ok && b.queued < count {
            let occupancy = occupancy_snapshot(role, None);
            release_group_keys(base, role, Some(b.group), Some(&kinds[b.queued..]), &occupancy, owner, "background build failed");
        }
    }
    let live_after = pool_counts(base).map_or(0, |(_, live)| live);
    let stored = {
        let mut state = BANKS[role].lock();
        match state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) {
            Some(bank) => {
                let stored = match bank.groups.get_mut(b.group) {
                    Some(g) if g.state == GroupState::Prefetched && valid && holder_ok => {
                        let mut window = b.window;
                        window.installed = count as u64;
                        for (cell, body) in window.cells.iter_mut().zip(bodies.iter()).take(count) { *cell = body.address; }
                        window.records = b.records;
                        window.mii_r = mii_set;
                        // What its body init wrote into `D+0x3a0` is the group's; its attach puts it back.
                        window.info_3a0 = info_now.map(|w| [w[0], w[1], w[2]]);
                        g.saved = Some(window);
                        g.bodies = bodies;
                        g.state = GroupState::Built;
                        true
                    }
                    Some(g) => { g.state = GroupState::Failed; g.background_refused = true; false }
                    None => false,
                };
                GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
                    .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
                stored
            }
            None => false,
        }
    };
    // N-bg: the object kind the prefetch registered (PT `0x72`; Aegis `0x75`, its orphan `0x1338` first) goes back only
    // now, after the holder step: the PT task and the Aegis pool read their files through it. As `complete_exchange`
    // returns it after a switch-time build; with the map already empty native freed it (settled, no native call).
    // X6: beside a live Mii window it went above with the group's other keys, the baseline in R.
    if let Some(kind) = b.members.object_kind().filter(|_| stored && b.holder_due && alias_ok) {
        let object = [kind];
        if b.baseline.is_some() {
            trace!("[GroupSwap] keys returned owner={} group={} object_kind={:#x} with the group's keys={:?} reason=background holder made",
                owner, b.group, kind, mii_keys);
        } else if fields::read::<u64>(b.alias, 0x18).unwrap_or(0) != 0 {
            let receipt = r_handle::release(base, lrole, Some(b.group), Some(&object[..]), &Occupancy::default(), "background object release");
            trace!("[GroupSwap] keys returned owner={} group={} object_kind={:#x} orphan_keys={:x?} map={} reason=background holder made",
                owner, b.group, kind, receipt.keys, fields::read::<u64>(b.alias, 0x18).unwrap_or(0));
        } else {
            let settled = r_handle::settle(lrole, b.group, &object, "background object freed by native (map=0)");
            trace!("[GroupSwap] settled owner={} group={} object_kind={:#x} map=0 settled={:x?}", owner, b.group, kind, settled);
        }
    }
    if b.holder_due {
        let holder = b.records[0];
        trace!("[GroupSwap] background holder settled owner={} group={} ok={} records={:x?} table=+{:#x} slot={} articles={} live_records={:x?}",
            owner, b.group, holder_ok, b.records, fields::read::<usize>(holder, 0x228).unwrap_or(0).wrapping_sub(base),
            fields::read::<i32>(holder, 0x230).unwrap_or(-2), fields::read::<u64>(holder, 0x10).unwrap_or(0),
            [0x5a88usize, 0x5a90, 0x5a98].map(|o| fields::read::<usize>(entry as usize, o).unwrap_or(0)));
    }
    let released = super::finish_install(b.token, "background build settled");
    if stored { log_memory(owner, b.group, kinds, "build", b.committed, MemorySample::capture(), memory::group_cost(kinds).build); }
    if let (false, Some(set)) = (stored, mii_set) { release_mii_r(base, set); }
    if b.mii.is_some() {
        trace!("[GroupSwap] background Mii owner={} group={} ok={} mii_r={:x?} hooks {:?}={:?} keys_released_kept={:?}",
            owner, b.group, stored, mii_set, MII_HIT_NAMES, mii_hits, mii_keys);
    }
    if let Some(ordinary) = b.baseline {
        trace!("[GroupSwap] background baseline owner={} group={} ok={} kinds={:x?} values={:#x} hooks {:?}={:?} keys_released_kept={:?}",
            owner, b.group, stored, kinds, shadow_values(&ordinary.entry), MII_HIT_NAMES, mii_hits, mii_keys);
    }
    let pads = pad_changes(&b.pads, &pad_controllers(base));
    if !pads.is_empty() {
        let restored = restore_live_pads(base, entry as usize, &b.pads);
        trace!("[GroupSwap] background build owner={} group={} pad controllers changed (slot, before, after)={:x?} live restored={:?}",
            owner, b.group, &*pads, &*restored);
    }
    let now = now_us();
    trace!("[GroupSwap] background built owner={} group={} ok={} reason={} kinds={:x?} bodies={:?} cell_bytes_before={:x?} us={} setup_us={} control_us={} settled_us={:?} ticks={} waited={} registrations_skipped={} info_3a0_same={} info_3a0={:x?} pool_live={}->{} dispatch={:x?} listeners={:?} live_listeners={:?} request_bit={:?} map={} INSTALL_released={}",
        owner, b.group, stored, reason, kinds, &bodies[..count], &cells_before[..count], now.saturating_sub(b.started_us), b.setup_us, b.control_us,
        &b.settled_us[..b.queued], b.ticks, waited, skips, info_same, info_now, b.live_before, live_after, b.window.dispatch,
        listener_counts(b.window.dispatch), listener_counts(dispatcher_arrays(entry as usize).unwrap_or([0, 0])), request,
        if alias_ok { fields::read::<u64>(b.alias, 0x18).map_or(-1, |m| m as i64) } else { -1 }, released);
    if !stored {
        trace!("[GroupSwap] background built owner={} group={} UNEXPECTED: the group failed (queued {}/{}); its bodies, contexts {:x?} and arrays {:x?} are left (reported)",
            owner, b.group, b.queued, count, &b.window.group_ctx[..count], b.window.dispatch);
    }
    if alias_ok { r_handle::reconcile(lrole, "background build"); }
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
    let copies = BANKS[role].lock().as_ref().filter(|bank| bank.swap.is_some())
        .map(|bank| (copy_package_sets(bank), bank.alias));
    let copies_resident = match copies {
        Some((sets, alias)) if alias != 0 && resource(base, owner) == Some(alias) =>
            crate::runtime::roster::copy_packages::admit(base, role, owner, alias, &sets),
        _ => true,
    };
    let (prepared, before, alias) = {
        let mut state = BANKS[role].lock();
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
        // Both tables already own stable storage. Exchange their owners instead
        // of allocating a duplicate target table inside this native callback.
        std::mem::swap(&mut bank.members, &mut bank.groups[swap.group].members);
        let target_members = *bank.members;
        // The Mii identity follows the window: `R` holds the window group's identity
        // (MII-GROUPS-DESIGN §3.3). When a Mii moves in or out, save the words the
        // detaching window leaves and write the words the attaching window needs.
        let x_identity = std::mem::replace(&mut bank.identity, bank.groups[swap.group].identity);
        // The configuration follows its identity (a warm exit may land the Mii in any group record, inbox 21:27Z).
        let x_configuration = std::mem::replace(&mut bank.configuration, bank.groups[swap.group].configuration);
        let identity_moves = x_identity.is_some() || bank.identity.is_some();
        if identity_moves {
            saved_x.identity = Some((save_resource_words(bank.alias), save_entry_identity(entry as usize)));
            if x_identity.is_none() { bank.ordinary_identity = saved_x.identity; }
        }
        // Inbox 21:27Z: out of a Mii window, an ordinary group that saved no identity move takes the owner's baseline.
        let baseline = bank.ordinary_identity.filter(|_| x_identity.is_some());
        let alias = bank.alias;
        let target_identity = bank.identity;
        // The window's previous active index the commit stored at `+0x5910` (`service` proved it).
        let from = bank.from;
        let group = &mut bank.groups[swap.group];
        group.identity = x_identity;
        group.configuration = x_configuration;
        // The record now holds the outgoing window; a parked group switched to is the window (never released). The
        // planner marks the record again on its next pass if what it holds left the cycle.
        group.leaving = false;
        group.release_now = false;
        let prepared = match (group.state, group.saved.take()) {
            (GroupState::Built, Some(y)) => {
                let x_bodies = std::mem::replace(&mut bank.bodies, group.bodies);
                // SPECIAL-GROUPS §1 (c): each window keeps its own Mii set in R, and its body init's `D+0x3a0` words.
                saved_x.mii_r = Some(take_mii_r(alias));
                saved_x.info_3a0 = read_info_3a0(entry as usize);
                group.bodies = x_bodies; group.saved = Some(saved_x);
                trace!("[GroupSwap] detach owner={} group={} kinds={:x?} bodies={:?} mii_r={:x?} info_3a0={:x?}", owner, swap.group,
                    &group.members.kinds()[..group.members.count()], x_bodies, saved_x.mii_r, saved_x.info_3a0);
                restore_window(entry as usize, &y);
                put_mii_r(alias, y.mii_r.unwrap_or_default());
                if let Some(words) = y.info_3a0 { write_info_3a0(entry as usize, words); }
                set_active_index(entry as usize, swap.member);
                set_previous_index(entry as usize, previous_cell_index(from, swap.member, target_members.count()));
                // A warm return into a Mii group rewrites its identity (R, entry bytes,
                // D mirror); a return from one restores what that window saved.
                let identity_move = match (target_identity, identity_move(target_identity.is_some(), x_identity.is_some(), y.identity, baseline)) {
                    (Some(identity), _) => Some((identity.header(owner), write_identity(entry as usize, alias, identity))),
                    (None, IdentityMove::Ordinary((words, saved))) => {
                        let words = baseline_words(words, target_members);
                        write_resource_words(alias, &words);
                        write_entry_identity(entry as usize, &saved);
                        Some((header_of(owner, &words), saved.entry))
                    }
                    (None, _) => None,
                };
                trace!("[GroupSwap] attach owner={} group={} member={} kinds={:x?} dispatch={:x?} identity={} header={:x?}", owner, swap.group, swap.member,
                    &target_members.kinds()[..target_members.count()], y.dispatch, target_identity.is_some(), identity_move.map(|m| m.0));
                Prepared::Warm { group: swap.group, member: swap.member, via: swap.via, dispatch: y.dispatch, identity_move,
                    outgoing: *group.members, members: target_members }
            }
            (GroupState::Prefetched, None) => {
                let x_bodies = std::mem::replace(&mut bank.bodies, [Body::default(); NATIVE_MEMBERS]);
                // The detaching window keeps its Mii set; the build starts from an empty one (the Mii setup builds the
                // faces only into an empty `R+0x90`, `0x67ead0`, and its bind no longer releases another window's).
                saved_x.mii_r = Some(take_mii_r(alias));
                saved_x.info_3a0 = read_info_3a0(entry as usize);
                group.bodies = x_bodies; group.saved = Some(saved_x); group.state = GroupState::Built;
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
                // game's queued tasks, and so does a Mii group (operator decision 2026-11-08); native and
                // pair groups keep the synchronous build (the
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
                mii_build = target_identity.zip(bank.configuration).filter(|_| target_members.special() == Special::Mii);
                Prepared::Build { group: swap.group, member: swap.member, via: swap.via, members: target_members, live_before, identity_move,
                    outgoing: *group.members, lease, asynchronous }
            }
            (found, saved) => {
                // Inconsistent record: put the table back and refuse this swap. A permanent
                // failure: a Prefetched record returns its keys first (U20, `swap_model::fallback`).
                std::mem::swap(&mut bank.members, &mut group.members);
                group.saved = saved;
                group.identity = target_identity;
                bank.identity = x_identity;
                group.configuration = bank.configuration;
                bank.configuration = x_configuration;
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
                    let mut state = BANKS[role].lock();
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
        let mut state = BANKS[role].lock();
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
    // Inbox 02:00Z: the outgoing window's pool, deactivated at this exchange, is emptied before it returns (its
    // restore rebuilds it below).
    finish_retirement(base, role, owner, "swap rollback");
    // PT-TRAINER-PARK: a failed warm attach rebuilt its pool; it goes before the previous window returns.
    if matches!(prepared, Prepared::Warm { .. }) {
        articles::retire_current(base, owner, entry as usize, current_holder_valid(base, entry as usize, owner), "warm rollback");
    }
    // Failure: destroy whatever the build produced in the window, put the
    // previous window back and let native activate the committed in-window
    // member — through the one teardown protocol (routes, bodies, contexts,
    // dispatcher arrays, records, then the unbuilt members' keys; finalize's order).
    let (bundle, alias, records, current) = {
        let state = BANKS[role].lock();
        let Some(bank) = state.as_ref() else { return; };
        let members = *bank.members;
        let mut detached = DetachedSnapshot::new(owner);
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
                };
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
        (detached, bank.alias, records, current)
    };
    let consumers = graph::probe_consumers(base, owner);
    let views = [bundle.view()];
    let plan = DetachedPlan { groups: &views, consumers: &consumers };
    let keys = detached_keys(&plan);
    // The ledger (WP4): a constructed body released its own kind's keys at construction end
    // (§1.2), so its token is settled; a member that never constructed still holds its prefetch
    // keys, and the object kind never has a tail. The plan's key step returns those tokens through
    // the ledger now (`r_handle::release`: the Aegis orphan key first, D-SG1; DEFECT #2's per-key
    // release on every teardown reason), or the map never empties and the next Reload is refused at
    // state 7. A Mii group's identity is still in `R` here: the bodies are unloaded and these keys
    // are released under it; the window's words return below.
    let alias_ok = alias != 0 && resource(base, owner) == Some(alias);
    let mut built = FixedList::<u32, NATIVE_MEMBERS>::new(0);
    for body in bundle.bodies.iter() { built.push(body.kind); }
    if alias_ok && !built.is_empty() { r_handle::construction_settled(ledger_role(role), group, &built); }
    let releasable = alias_ok && fields::read::<u64>(alias, 0x18).unwrap_or(0) != 0;
    let mut release = |kinds: &[u32]| {
        if !releasable { return (0, kinds.len()); }
        release_group_keys(base, role, Some(group), Some(kinds), &Occupancy::default(), owner, "build rollback")
    };
    let receipt = graph::execute(base, &plan, &keys, owner, "swap rollback", &mut release, &mut |_| {});
    let map_after = if releasable { fields::read::<u64>(alias, 0x18).map_or(-1i64, |m| m as i64) } else { -1 };
    // A failed warm swap leaks the unverified group's arrays with its bodies (logged).
    let dispatch_note = match prepared {
        Prepared::Build { .. } => DispatchNote::Destroyed(receipt.dispatchers == 1, current),
        Prepared::Warm { dispatch, .. } => DispatchNote::Leaked(dispatch),
    };
    let (failed, restored, failed_set) = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut() else { return; };
        let target_identity = bank.identity;
        let record = &mut bank.groups[group];
        let Some(x) = record.saved.take() else { return; };
        std::mem::swap(&mut bank.members, &mut record.members);
        let failed = *record.members;
        bank.bodies = std::mem::replace(&mut record.bodies, [Body::default(); NATIVE_MEMBERS]);
        record.state = GroupState::Failed;
        // The identity goes back with the failed group; `R`, the entry bytes and the
        // D mirror get the previous window's values.
        bank.identity = std::mem::replace(&mut record.identity, target_identity);
        let target_configuration = bank.configuration;
        bank.configuration = std::mem::replace(&mut record.configuration, target_configuration);
        restore_window(entry as usize, &x);
        // The failed group's Mii set (what its build or attach left in R) is released below; the window's comes back.
        let failed_set = take_mii_r(alias);
        put_mii_r(alias, x.mii_r.unwrap_or_default());
        if let Some(words) = x.info_3a0 { write_info_3a0(entry as usize, words); }
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
            owner, group, receipt.bodies, receipt.contexts, receipt.routes_cleared, records, &*keys, map_after, receipt.skipped, dispatch_note, via);
        (failed, *bank.members, failed_set)
    };
    release_mii_r(base, failed_set);
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

unsafe fn sync_copy_packages(entry: *mut u8, owner: u32, role: usize) {
    let base = BASE.load(Ordering::Acquire);
    // K15: hold the Kirby copy packages every set this bank may register needs, from the moment a
    // group holds keys or bodies, so they are resident before any registration; release them when
    // no group does or the bank retired (`swap_native::copy_packages`). Outside a swap only.
    let copy_sets = BANKS[role].lock().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.swap.is_none())
        .map(|bank| (if bank.phase == Phase::Ready && !bank.departed { copy_package_sets(bank) }
            else { CopyPackageSets::new([u32::MAX; NATIVE_MEMBERS]) }, bank.alias));
    if let Some((sets, alias)) = copy_sets {
        if sets.is_empty() || (alias != 0 && resource(base, owner) == Some(alias)) {
            crate::runtime::roster::copy_packages::sync(base, role, owner, alias, &sets, "tick");
        }
    }
}

/// Whether the last lifecycle tick saw a foreground request waiting (`character_roster::foreground`): one diag line as
/// an episode starts and one as it ends.
static FOREGROUND_HELD: AtomicBool = AtomicBool::new(false);

fn note_foreground(owner: u32, foreground: super::Foreground) {
    if FOREGROUND_HELD.swap(foreground.waiting, Ordering::AcqRel) == foreground.waiting { return; }
    if foreground.waiting {
        trace!("[GroupSwap] speculative steps wait owner={}: a foreground request waits for INSTALL; target={:x?}, only the step serving it goes first",
            owner, foreground.target.map(|t| (t.kind, t.configuration.is_some())));
    } else {
        trace!("[GroupSwap] speculative steps resume owner={}: no foreground request waits", owner);
    }
}

/// Primary 2026-11-09 07:34Z: the group of this bank holding a waiting press's `target` and the step that serves it
/// (`body_bank::serving_step`), under the gates the lifecycle puts on that step everywhere: a prefetch while the live
/// member admits one (D-KIRBY1), a re-arm as `rearm_candidate` and `kirby_rearm` admit it. A target in the live window
/// (a member change) or in no group of this bank needs none.
unsafe fn serving_group(base: usize, bank: &Bank, target: crate::runtime::roster::cycle_preload::Target, live: Option<u32>)
    -> Option<(usize, Serving)> {
    if bank.members.index_of(target.kind).is_some() && bank.configuration == target.configuration { return None; }
    let (group, g) = bank.groups.iter().enumerate().find(|(_, g)| !g.leaving && g.members.index_of(target.kind).is_some()
        && match target.configuration { Some(c) => g.configuration == Some(c), None => g.identity.is_none() })?;
    let rearm_due = !g.object_refused && bank.construction.is_none() && bank.background.is_none()
        && object_rearm_due(g.state, *g.members, detached_pool(base, bank.owner, g), g.object_held)
        && g.members.object_kind().is_some_and(|kind| crate::engine::swap::body_bank::object_rearm_admitted(live, kind).is_ok());
    let serving = serving_step(g.state, g.identity.is_none() || g.charinfo_published, rearm_due)?;
    (serving == Serving::ObjectRearm || crate::engine::swap::body_bank::prefetch_admitted(live).is_ok()).then_some((group, serving))
}

unsafe fn group_lifecycle(entry: *mut u8, owner: u32, role: usize) {
    let base = BASE.load(Ordering::Acquire);
    let settings = crate::runtime::roster::cycle_preload_config::snapshot();
    let now = now_us();
    sync_copy_packages(entry, owner, role);
    // Read before the bank lock (the roster's and the owners' state): a foreground request waiting for INSTALL alone.
    let foreground = super::foreground(role, entry, owner);
    note_foreground(owner, foreground);
    // D-KIRBY1: no bank prefetch key rests on this R while its live member is
    // Kirby (a copy of a group kind would share R's G68 mirror with the key, and
    // the key's later `0x17e8610` release corrupts the registry the Reload walks;
    // RCA §3). A live Kirby defers Planned groups and hands Prefetched groups'
    // keys back; Built groups consumed their keys at construction.
    enum Step { Poll(usize, super::InstallToken, u64, Members, usize), Start(usize, Members, usize, bool),
        Release(usize, Members, Members, usize), Background(usize), Leave(usize), Idle,
        Rearm(usize, Members, usize), RearmPoll(usize, super::InstallToken, u64, u32, usize), Retire(Retiring),
        Prepare(usize, usize), PreparePoll(Preparing) }
    let step = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.phase == Phase::Ready
            && !bank.departed && bank.swap.is_none()) else { return; };
        let live = bank.members.ready_live(snapshot(entry)).map(|(_, kind)| kind);
        let step =
        // Inbox 00:18Z: a PT group's object re-acquisition in flight.
        if let Some((group, token, since)) = bank.object_fetch {
            let kind = bank.groups[group].members.object_kind().unwrap_or(PT_OBJECT_KIND);
            Step::RearmPoll(group, token, since, kind, bank.alias)
        } else if let Some((group, token, since)) = bank.prefetch {
            Step::Poll(group, token, since, *bank.groups[group].members, bank.alias)
        // Inbox 02:00Z: the PT pool the last exit left retiring, one stage per tick, before anything new starts.
        } else if let Some(r) = bank.retiring {
            Step::Retire(r)
        // Inbox 02:42Z: a detached PT group's retired pool built on the task worker, ahead of its attach.
        } else if let Some(p) = bank.preparing {
            Step::PreparePoll(p)
        } else if let Some(group) = prepare_candidate(base, bank) {
            Step::Prepare(group, bank.groups[group].saved.map_or(0, |w| w.records[0]))
        } else if let Err(reason) = crate::engine::swap::body_bank::prefetch_admitted(live) {
            let (window, alias) = (*bank.members, bank.alias);
            // Primary 2026-11-09 05:38Z: after the key returns, a detached PT group's object re-arm runs here too (its
            // object kind is no fighter a Kirby copy takes); fighter keys keep waiting.
            match bank.groups.iter().position(|g| g.state == GroupState::Prefetched) {
                Some(group) => Step::Release(group, *bank.groups[group].members, window, alias),
                None => match kirby_rearm(base, bank, live) {
                    Some(group) => Step::Rearm(group, *bank.groups[group].members, alias),
                    None => {
                        for (group, g) in bank.groups.iter_mut().enumerate()
                            .filter(|(_, g)| g.state == GroupState::Planned && !g.kirby_logged) {
                            g.kirby_logged = true;
                            trace!("[GroupSwap] prefetch deferred owner={} group={} kinds={:x?} reason={} (D-KIRBY1)",
                                owner, group, &g.members.kinds()[..g.members.count()], reason);
                        }
                        Step::Idle
                    }
                },
            }
        // A Mii group's key request reads its CharInfo from the owner's record:
        // it starts only after the adapter published it (`pending_mii_publication`).
        // A group held for memory or pool space is started again, after every group not yet tried
        // (a held group never blocks one the admission has not seen): the admission below decides on
        // the capacity of this tick.
        // Inbox 00:18Z: a detached PT group with a retired pool takes its object kind again before anything new starts
        // (right after its detach, so it is resident by its next attach).
        } else if let Some(group) = rearm_candidate(base, bank) {
            Step::Rearm(group, *bank.groups[group].members, bank.alias)
        // rc.6 mid-session cycle change: a group whose member left the cycle is released before anything new starts.
        // A Built one stays parked until the pause opens (`pause_opening`), unless it is marked to go now because the new
        // members need its room (operator rule 2026-11-09 13:21Z); never one a committed switch targets.
        } else if let Some(group) = bank.groups.iter().enumerate().position(|(i, g)| g.leaving
            && leaves_now(g.state, g.release_now && bank.swap.is_none_or(|s| s.group != i))) {
            Step::Leave(group)
        } else {
            let start = crate::engine::swap::body_bank::next_start(bank.groups.iter()
                .map(|g| (g.state, g.identity.is_none() || g.charinfo_published)), bank.held_turn);
            // rc.6 item 3: every planned group's prefetch first, then the background builds of the prefetched ones;
            // a group held for capacity is retried only when no background build is owed.
            match (start, background_candidate(bank, live)) {
                (Some(group), _) if bank.groups[group].state == GroupState::Planned =>
                    Step::Start(group, *bank.groups[group].members, bank.alias, false),
                (_, Some(group)) => Step::Background(group),
                (Some(group), None) => Step::Start(group, *bank.groups[group].members, bank.alias, false),
                (None, None) => Step::Idle,
            }
        };
        // Primary 2026-11-09 07:17Z and 07:34Z: a foreground request waiting for INSTALL alone is served next
        // (`speculative_step_admitted`): no prefetch, background build, object re-arm or pool preparation takes INSTALL
        // before it, except the one that serves the press's own target (`serving_group`); the polls run.
        match step {
            Step::Start(..) | Step::Background(_) | Step::Rearm(..) | Step::Prepare(..)
                if speculative_step_admitted(foreground.waiting).is_err() =>
                match foreground.target.and_then(|target| serving_group(base, bank, target, live)) {
                    Some((group, Serving::Prefetch)) => Step::Start(group, *bank.groups[group].members, bank.alias, true),
                    Some((group, Serving::ObjectRearm)) => Step::Rearm(group, *bank.groups[group].members, bank.alias),
                    None => Step::Idle,
                },
            step => step,
        }
    };
    let started = now_us();
    let name = match step {
        Step::Idle => return,
        Step::Background(_) => "background start", Step::Leave(_) => "leave", Step::Release(..) => "key release",
        Step::Poll(..) => "prefetch poll", Step::Start(..) => "prefetch start",
        Step::Rearm(..) => "object re-arm", Step::RearmPoll(..) => "object re-arm poll",
        Step::Retire(..) => "articles retire stage", Step::Prepare(..) => "articles prepare",
        Step::PreparePoll(..) => "articles prepare poll",
    };
    // `slow_step` names the step for a frame gap (inbox 22:17Z); every early return below is a cheap refusal.
    struct Timed(u32, &'static str, u64);
    impl Drop for Timed { fn drop(&mut self) { slow_step(self.0, self.1, self.2); } }
    let _timed = Timed(owner, name, started);
    match step {
        Step::Idle => {}
        Step::Retire(r) => retire_step(base, role, owner, r, "detach"),
        Step::Prepare(group, holder) => {
            // As a prefetch starts: INSTALL held until the build settles, so a press meanwhile waits for it.
            if holder == 0 || !valid_special_object(base, holder, owner) || super::INSTALL.active() { return; }
            let Some(token) = super::INSTALL.try_begin(false) else { return; };
            let t = queue_prepare(base, owner, holder);
            let queued = match BANKS[role].lock().as_mut() {
                Some(bank) if t != 0 => {
                    bank.preparing = Some(Preparing { group, holder, task: t, since: now, token });
                    true
                }
                Some(bank) => {
                    if let Some(g) = bank.groups.get_mut(group) { g.prepare_refused = true; }
                    false
                }
                None => false,
            };
            if !queued {
                super::finish_install(token, "articles prepare: not queued");
                trace!("[GroupSwap] articles prepare UNEXPECTED owner={} group={} holder={:#x} task={:#x}; the attach rebuilds the pool",
                    owner, group, holder, t);
                return;
            }
            trace!("[GroupSwap] articles prepare owner={} group={} holder={:#x} task={:#x}; the pool is built on the task worker",
                owner, group, holder, t);
        }
        Step::PreparePoll(p) => {
            use construction_task as task;
            if fields::read::<i32>(p.task, task::PENDING).unwrap_or(0) > 0 { return; }
            prepare_settled(base, role, owner, p, "settled");
        }
        Step::Rearm(group, members, alias) => {
            // As a prefetch starts (`Step::Start`): this R, idle, Training; INSTALL held until it settles, so a press
            // meanwhile waits for it (`character_roster`'s pending request) instead of rebuilding without the files.
            let Some(kind) = members.object_kind() else { return; };
            if !groups_enabled(&settings) || super::INSTALL.active() { return; }
            let Some(r) = resource(base, owner).filter(|r| *r == alias && alias != 0) else { return; };
            if !resource_ready(r) { return; }
            if !crate::runtime::roster::scope_gate::admitted("native_body_bank::group_prefetch") { return; }
            let Some(token) = super::INSTALL.try_begin(false) else { return; };
            let lrole = ledger_role(role);
            r_handle::bind(lrole, owner, r, crate::runtime::roster::observe::owner_flags(r));
            let copy = ready_members(entry, owner).and_then(|window| live_kirby_copy_kind(entry, owner, window));
            let occupancy = occupancy_snapshot(role, copy);
            // X6: beside a live Mii window with the group's own (ordinary) words, as its prefetch requested them.
            let mut acquired = Ok(());
            with_words(r, group_words_of(role, group, r), || {
                acquired = r_handle::acquire(base, lrole, group, kind, Holder::Prefetch(GroupId(group as u8)), member_recipe(members, kind), &occupancy);
            });
            if let Err(why) = acquired {
                super::finish_install(token, "object kind re-acquire declined by the ledger");
                if let Some(g) = BANKS[role].lock().as_mut().and_then(|bank| bank.groups.get_mut(group)) { g.object_refused = true; }
                trace!("[GroupSwap] object kind NOT re-acquired owner={} group={} kind={:#x} reason={:?}; its next attach takes the Reload path",
                    owner, group, kind, why);
                return;
            }
            let mut state = BANKS[role].lock();
            let Some(bank) = state.as_mut() else { super::finish_install(token, "object kind re-acquire: bank gone"); return; };
            bank.object_fetch = Some((group, token, now));
            trace!("[GroupSwap] object kind re-acquire owner={} group={} kind={:#x} R={:#x} map={} task_pending={}; the group's next attach rebuilds its trainer articles",
                owner, group, kind, r, fields::read::<u64>(r, 0x18).unwrap_or(0), task_pending(r));
        }
        Step::RearmPoll(group, token, since, kind, alias) => {
            let r = resource(base, owner).filter(|r| *r == alias);
            let ready = r.is_some_and(|r| resource_ready(r));
            let outcome = prefetch_outcome(ready, now.saturating_sub(since), PREFETCH_TIMEOUT_US);
            if outcome == GroupState::Prefetching { return; }
            if let (GroupState::Failed, Some(r)) = (outcome, r) {
                let occupancy = occupancy_snapshot(role, None);
                with_words(r, group_words_of(role, group, r), || {
                    release_group_keys(base, role, Some(group), Some(&[kind]), &occupancy, owner, "object kind re-acquire timed out");
                });
            }
            let mut state = BANKS[role].lock();
            let Some(bank) = state.as_mut() else { return; };
            bank.object_fetch = None;
            if let Some(g) = bank.groups.get_mut(group) {
                g.object_held = outcome == GroupState::Prefetched;
                g.object_refused |= outcome == GroupState::Failed;
            }
            let released = super::finish_install(token, "object kind re-acquired");
            trace!("[GroupSwap] object kind re-acquired owner={} group={} kind={:#x} ok={} us={} map={} INSTALL_released={}",
                owner, group, kind, outcome == GroupState::Prefetched, now.saturating_sub(since),
                fields::read::<u64>(alias, 0x18).unwrap_or(0), released);
        }
        Step::Background(group) => start_background(entry, owner, role, group),
        Step::Leave(group) => if !super::INSTALL.active() {
            // Operator rule 2026-11-09 13:21Z: a parked group freed now logs its game-thread hold and the headroom it gave
            // back, as the pause's free does. If it could not go, the pause frees it (no retry on every tick).
            let parked = BANKS[role].lock().as_ref().and_then(|bank| bank.groups.get(group))
                .filter(|g| leaves_at_pause(g.state)).map(|g| *g.members);
            let before = parked.map(|_| MemorySample::capture());
            leave_group(entry, owner, role, group);
            if let (Some(members), Some(before)) = (parked, before) {
                let after = MemorySample::capture();
                let still = match BANKS[role].lock().as_mut().and_then(|bank| bank.groups.get_mut(group))
                    .filter(|g| g.leaving && leaves_at_pause(g.state)) {
                    Some(g) => { g.release_now = false; true }
                    None => false,
                };
                trace!("[GroupSwap] frees now owner={} group={} kinds={:x?} us={} headroom={:?}->{:?} committed={:?}->{:?} freed={}{}",
                    owner, group, &members.kinds()[..members.count()], now_us().saturating_sub(started), before.headroom(),
                    after.headroom(), before.committed.get(), after.committed.get(), !still,
                    if still { "; the pause frees it" } else { "" });
                if !still { let mut watch = RELEASE_WATCH.lock(); *watch = watch.freed(); }
            }
        },
        Step::Release(group, members, window, alias) => {
            // Same preconditions as a prefetch: this R, idle. A freshly live Kirby
            // is hatless (Reloaded, or parked copy-free by `kirby_park_gate`); a
            // copy of a group kind seen here keeps every key and reports once.
            let Some(r) = resource(base, owner).filter(|r| *r == alias && alias != 0) else { return; };
            if !resource_ready(r) { return; }
            let copy = live_kirby_copy_kind(entry, owner, window);
            let (kinds, count) = members.prefetch_kinds();
            if copy.is_some_and(|kind| kinds[..count].contains(&kind)) {
                let mut state = BANKS[role].lock();
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
            let mut state = BANKS[role].lock();
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
            let mut state = BANKS[role].lock();
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
        Step::Start(group, members, alias, serving) => {
            if !groups_enabled(&settings) || super::INSTALL.active() { return; }
            // A prefetch serving a waiting press is that press's own load, not speculation: no burst grace holds it.
            if !serving && crate::runtime::roster::cycle_preload::speculative_admission().is_none() { return; }
            let Some(r) = resource(base, owner).filter(|r| *r == alias && alias != 0) else { return; };
            let (capacity, live) = pool_counts(base).unwrap_or((0, 0));
            let pool_free = (capacity as usize).saturating_sub(usize::from(live)).saturating_sub(POOL_RESERVE);
            if pool_free < members.count() {
                if hold_group(role, group, Hold::Pool) {
                    trace!("[GroupSwap] held owner={} group={} reason=fighter pool {}/{} reserve {} < {} bodies; retried",
                        owner, group, live, capacity, POOL_RESERVE, members.count());
                }
                release_for_capacity(owner, group, "fighter pool");
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
                    let mut state = BANKS[role].lock();
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
                // A refusal holds the group (nothing native was touched): the headroom moves as helper
                // sources retarget and other groups settle, so its start is retried; it never fails here.
                Admission::Refuse => {
                    if hold_group(role, group, Hold::Memory) {
                        trace!("[GroupSwap] held owner={} group={} reason=process headroom {:?} < {} bytes kinds={:x?} outstanding={} prefetch={} build={}; retried",
                            owner, group, headroom, decision.need, &members.kinds()[..members.count()], decision.outstanding,
                            cost.prefetch, cost.build);
                    }
                    release_for_capacity(owner, group, "process headroom");
                    return;
                }
            }
            // Native's own readiness (`0x17e4750`): the current task at `R+0xa8` is
            // idle at `task+0x10 == 0`; its pointer may change across Reload (C4).
            // Only a running task
            // or an unloaded handle defers the prefetch.
            if !resource_ready(r) {
                let mut state = BANKS[role].lock();
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
            // its identity in `R`, and beside a live Mii window every other group's with the
            // owner's ordinary baseline (X6: the costume its background build and its warm exit
            // take); the window's words come back right after. A refusal returns what this
            // group already acquired under the same words.
            let words = group_words_of(role, group, r);
            let mut refused = None;
            let mut returned = (0usize, 0usize);
            with_words(r, words, || {
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
            let mut state = BANKS[role].lock();
            let Some(bank) = state.as_mut() else { super::finish_install(token, "group prefetch bank vanished"); return; };
            bank.groups[group].state = GroupState::Prefetching;
            bank.groups[group].since_us = now;
            bank.groups[group].prefetch_committed = sample.committed.get();
            bank.prefetch = Some((group, token, now));
            trace!("[GroupSwap] prefetch owner={} group={} kinds={:x?} R={:#x} map={} flags={:#x} task_pending={} pool={}/{} headroom={} INSTALL(resource) outstanding={} need={} serves_press={}",
                owner, group, &members.kinds()[..members.count()], r, fields::read::<u64>(r, 0x18).unwrap_or(0),
                fields::read::<u8>(r, 0x4c).unwrap_or(0), task_pending(r), live, capacity, headroom.unwrap_or(0),
                decision.outstanding, decision.need, serving);
        }
    }
}

/// The groups' part of `process_memory::outstanding`: what the admitted groups of both roles have not spent yet
/// (`body_bank::memory::outstanding`): a prefetch in flight its whole cost, a prefetched group its build. A built
/// group's bodies are in the headroom. Takes each role's bank lock in turn; never call it with a bank lock held.
pub(super) fn group_outstanding() -> u64 {
    BANKS.iter().map(|state| state.lock().as_ref().map_or(0, |bank| bank.groups.iter()
        .map(|g| memory::outstanding(g.state, memory::group_cost(&g.members.kinds()[..g.members.count()])))
        .fold(0u64, u64::saturating_add)))
        .fold(0u64, u64::saturating_add)
}

/// rc.6 window admission: what the cycle waits for, which a created entry leaves (`process_memory::admit_created`): the
/// supplements each role's last window plan refused for memory (`WINDOW_WAITING`) and every group held for memory, its
/// cost with the margin. Takes each role's bank lock in turn; never call it with a bank lock held.
pub(super) fn cycle_waiting() -> u64 {
    let windows = WINDOW_WAITING.iter().map(|waiting| waiting.load(Ordering::Acquire)).fold(0u64, u64::saturating_add);
    BANKS.iter().map(|state| state.lock().as_ref().map_or(0, |bank| bank.groups.iter()
        .filter(|g| g.state == GroupState::Held(Hold::Memory))
        .map(|g| memory::with_margin(memory::group_cost(&g.members.kinds()[..g.members.count()]).total()))
        .fold(0u64, u64::saturating_add)))
        .fold(windows, u64::saturating_add)
}

/// `[GroupSwap] memory`: the committed process charge one phase of a group took, next to the estimate the
/// admission used (`tools/group_memory_costs.py` reads these lines as exact observations).
fn log_memory(owner: u32, group: usize, kinds: &[u32], phase: &str, before: Option<u64>, after: MemorySample, estimate: u64) {
    // LOG-REFUSED: the per-load measurement is an Eden data line, diag builds only (the `[GroupSwap]` tag ships).
    if !cfg!(feature = "diag") { return; }
    let committed = after.committed.get();
    let delta = before.zip(committed).map(|(b, a)| i128::from(a) - i128::from(b));
    trace!("[GroupSwap] memory owner={} group={} kinds={:x?} phase={} committed={:?}->{:?} delta={} estimate={} headroom={:?}",
        owner, group, kinds, phase, before, committed, MemoryDelta(delta), estimate, after.headroom());
}

/// `delta=` of the memory line, formatted in place: it runs inside the entry hooks, where nothing may allocate.
struct MemoryDelta(Option<i128>);
impl std::fmt::Display for MemoryDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 { Some(delta) => write!(f, "{delta}"), None => f.write_str("na") }
    }
}

/// A refused start holds the group for `hold` (`GroupState::Held`); a held group refused again
/// passes the retry turn to the next held group. True the first time it is held for this reason
/// (one log line per hold, not one per tick).
fn hold_group(role: usize, group: usize, hold: Hold) -> bool {
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut() else { return false; };
    let Some(g) = bank.groups.get_mut(group) else { return false; };
    let first = g.state != GroupState::Held(hold);
    if !first { bank.held_turn = group + 1; }
    g.state = GroupState::Held(hold);
    first
}

fn fail_group(role: usize, group: usize, _reason: &str) {
    let mut state = BANKS[role].lock();
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
    // rc.6 item 3: a background build in flight finishes into its group record, which the destroys below then free.
    drain_background(entry, owner, role, reason);
    // Inbox 02:00Z: a PT pool still retiring is emptied before its holder is destroyed below.
    finish_retirement(base, role, owner, reason);
    // Inbox 02:42Z: a prepared pool build in flight finishes before its holder is destroyed below.
    finish_preparation(base, role, owner, reason);
    // D-KIRBY1: the kind a live Kirby copies keeps its key (read before the lock).
    let window = BANKS[role].lock().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.owner == owner).map(|bank| *bank.members);
    let copy = window.and_then(|members| live_kirby_copy_kind(entry, owner, members));
    // U18: each detached body's group, for the per-group header during the destroys.
    let mut owners = FixedVec::<(u32, Members), { MAX_EXTRA_GROUPS * NATIVE_MEMBERS }>::new();
    let (mut bundles, identities, token, alias, groups, mii_sets) = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        if bank.groups.is_empty() && bank.prefetch.is_none() { return; }
        let token = bank.prefetch.take().map(|p| p.1);
        // Inbox 00:18Z: an object re-acquisition in flight ends here; its token goes with every other below.
        let rearm = bank.object_fetch.take().map(|f| f.1);
        if let Some(token) = rearm { super::finish_install(token, "object re-acquire retired"); }
        let (alias, groups, activated) = (bank.alias, bank.groups.len(), bank.activated);
        let mut bundles = FixedList::<DetachedSnapshot, MAX_EXTRA_GROUPS>::new(DetachedSnapshot::new(owner));
        // SPECIAL-GROUPS §1 (c): each detached window's own Mii set, released after its bodies.
        let mut mii_sets = FixedList::<MiiR, MAX_EXTRA_GROUPS>::new(MiiR::default());
        for set in bank.groups.iter().filter(|g| g.state == GroupState::Built).filter_map(|g| g.saved.and_then(|w| w.mii_r)) {
            if set != MiiR::default() { mii_sets.push(set); }
        }
        // A group's keys are released with its own words in `R` (the release selector
        // `0x17e8410` keys on `R+0x58` like the request did; `group_words`): each key-holding Mii
        // group, and beside a live Mii window every other key-holding group (X6), keeps its index
        // (the ledger's group of its tokens) and its words.
        let mut identities = FixedVec::<(usize, Members, Option<MiiIdentity>), MAX_EXTRA_GROUPS>::new();
        let live_mii = (bank.identity.is_some(), bank.ordinary_identity.map(|(words, _)| words));
        for (index, group) in bank.groups.iter().enumerate() {
            if (group.object_held || group.state.holds_keys()) && (group.identity.is_some() || live_mii.0) {
                identities.push((index, *group.members, group.identity));
            }
            let mut detached = DetachedSnapshot::new(owner);
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
                        detached.bodies.push(BodyRef { id: body.id, kind: *kind, cell: i as u8, activated: activated.contains(&(body.address, body.id)),
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
                    // Inbox 00:18Z: a re-acquired object kind is a held key: the plan's key step returns it (rc6-bg-pt4:
                    // left on R, the next Reload's state 7 refused on `R+0x18 == 2`).
                    if group.object_held { detached.held_kinds.push(PT_OBJECT_KIND); }
                }
                (state, _) if state.holds_keys() => {
                    let (kinds, count) = group.members.prefetch_kinds();
                    for kind in kinds[..count].iter().copied() { detached.held_kinds.push(kind); }
                }
                _ => {}
            }
            bundles.push(detached);
        }
        bank.groups.clear();
        bank.swap = None;
        bank.swap_started = None;
        GROUP_BODIES[role].store(0, Ordering::Release);
        (bundles, (identities, live_mii), token, alias, groups, mii_sets)
    };
    // Without the alias R (`0x3aeb50` and every key release read it) bodies and keys
    // are not touched: they are reported as skipped, as before.
    let alias_ok = alias != 0 && resource(base, owner) == Some(alias);
    let mut unreachable = 0usize;
    if !alias_ok {
        for group in bundles.iter_mut() {
            unreachable += group.bodies.len() + group.held_kinds.len();
            group.bodies.clear();
            group.held_kinds.clear();
        }
    }
    // U18: detached bodies hold no registration but unload over the cells the window's registrations
    // occupy: the window leaves the registry, each group is destroyed under its own header, then the
    // window registers again (`copy_registry::teardown_begin/group/end`). Without the registry the
    // bodies are left (this function's rule: leak rather than corrupt).
    let has_bodies = bundles.iter().any(|group| !group.bodies.is_empty());
    let mut registry = if has_bodies { NativeRegistry::new(base, alias, owner).zip(window) } else { None };
    if has_bodies && registry.is_none() {
        for group in bundles.iter_mut() {
            unreachable += group.bodies.len(); group.bodies.clear();
        }
        trace!("[Registry] teardown owner={} reason={} registry unreadable; detached bodies left", owner, reason);
    }
    let cells = registry.as_mut().map(|(registry, window)| {
        let before = registry.cells();
        copy_registry::teardown_begin(registry, *window);
        (before, registry.cells())
    });
    let consumers = graph::probe_consumers(base, owner);
    let mut views = FixedList::<DetachedView<'_>, MAX_EXTRA_GROUPS>::new(DetachedView {
        owner, bodies: &[], contexts: &[], dispatch: None, records: None, held_kinds: &[] });
    for bundle in bundles.iter() { views.push(bundle.view()); }
    let plan = DetachedPlan { groups: &views, consumers: &consumers };
    let keys = detached_keys(&plan);
    // The plan's key step (DEFECT #2's per-key release on every teardown reason, row 15) hands
    // every token the ledger still holds for this role — these groups' unconsumed prefetches and a
    // window whose build never settled — back through `r_handle::release` (WP4: the orphan `0x1338`
    // by its own key first, then `0x17e8610(R, k)`; D-SG1). The live-consumer check decides which
    // go (D-KIRBY1: a refused token is retained and reported, never destroyed). A Mii group's
    // tokens go with its identity in `R`, beside a live Mii window every other group's with the
    // owner's ordinary baseline (X6), the rest (the window's) in one pass.
    let occupancy = occupancy_snapshot(role, copy);
    let mut release = |_: &[u32]| {
        let mut outcome = (0usize, 0usize);
        let (identities, (window_identity, baseline)) = &identities;
        for (group, members, identity) in identities.iter() {
            let mut one = (0usize, 0usize);
            let words = alias_ok.then(|| own_words(*members, *identity, *window_identity, *baseline, save_resource_words(alias)))
                .flatten();
            with_words(alias, words, || {
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
        let mut before_step = |body: Option<u32>| {
            let Some((registry, window)) = registry.as_mut() else { return; };
            let wanted = match body {
                Some(id) => owners.iter().find(|(body, _)| *body == id).map(|(_, group)| *group),
                None => Some(*window),
            };
            if let Some(members) = wanted.filter(|m| current != Some(*m)) {
                copy_registry::teardown_group(registry, members);
                current = Some(members);
            }
        };
        graph::execute(base, &plan, &keys, owner, reason, &mut release, &mut before_step)
    };
    if let (Some((registry, window)), Some((before, emptied))) = (registry.as_mut(), cells) {
        copy_registry::teardown_end(registry, *window);
        let mut detached = FixedList::<Members, MAX_EXTRA_GROUPS>::new(Members::pair_group());
        for (_, group) in owners.iter() {
            if !detached.iter().any(|m| m.kinds()[..m.count()] == group.kinds()[..group.count()]) { detached.push(*group); }
        }
        trace!("[Registry] teardown owner={} reason={} window={:x?} detached={:x?} cells={:x?}->{:x?}->{:x?} header={:x?} r4c={:#x} task_pending={}",
            owner, reason, &window.kinds()[..window.count()], MemberSets(&detached), before, emptied, registry.cells(), registry.header(),
            fields::read::<u32>(alias, 0x4c).unwrap_or(0), task_pending(alias));
    }
    for set in mii_sets.iter() { release_mii_r(base, *set); }
    if !mii_sets.is_empty() { trace!("[GroupSwap] teardown owner={} reason={} mii_sets_released={:x?}", owner, reason, &*mii_sets); }
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

/// Operator rule 2026-11-09 (inbox 13:21Z): a prefetch refused for process memory or fighter-pool bodies while groups are
/// parked frees one of them now (`body_bank::capacity_release`: the fewest members the cycle keeps, then the fewest
/// bodies; never one a committed switch targets), unless one is going already. The refused group is retried after it, so
/// parked groups go one at a time and only while refusals last; a refusal with nothing parked stays "waiting for memory".
/// Value-only (the entry tick frees the group, `leaves_now`); never under the bank lock.
fn release_for_capacity(owner: u32, group: usize, reason: &str) {
    // Operator rule 2026-11-09 16:06Z: an earlier free's room still arriving is room on its way; nothing more goes.
    let settled = RELEASE_WATCH.lock().settled();
    let mut parked = FixedList::<((usize, usize), Members, u8), { 2 * MAX_EXTRA_GROUPS }>::new(((0, 0), Members::pair_group(), 0));
    for role in 0..2 {
        let state = BANKS[role].lock();
        let Some(bank) = state.as_ref().filter(|bank| bank.phase == Phase::Ready && !bank.departed) else { continue; };
        for (index, g) in bank.groups.iter().enumerate() {
            if !g.leaving || !leaves_at_pause(g.state) { continue; }
            if g.release_now { return; }
            if bank.swap.is_none_or(|s| s.group != index) { parked.push(((role, index), *g.members, g.parked_kept)); }
        }
    }
    let Some((role, index)) = capacity_release(settled, parked.iter().copied()) else { return; };
    let mut state = BANKS[role].lock();
    let Some(bank) = state.as_mut() else { return; };
    let parked_owner = bank.owner;
    if let Some(g) = bank.groups.get_mut(index).filter(|g| g.leaving && leaves_at_pause(g.state)) {
        g.release_now = true;
        trace!("[GroupSwap] release now owner={} group={} kinds={:x?} kept={} reason={} held prefetch owner={} group={}; the entry tick frees it",
            parked_owner, index, &g.members.kinds()[..g.members.count()], g.parked_kept, reason, owner, group);
    }
}

/// rc.6 mid-session cycle change (operator requirement 2026-11-08, inbox 19:21Z): one detached group a member of which
/// left the role's cycle, released while the window stays live, through the teardown protocol `retire_groups` runs for
/// every group: a Prefetched group's keys go back through the ledger; a Built group's bodies, contexts, dispatcher arrays
/// and records are destroyed under its own header, the window leaving the copy registry around them and registering
/// again (U18). The group becomes Released and its slot takes the next planned group. Not while the live member is Kirby
/// (`group_lifecycle` returns its keys first, D-KIRBY1) or a native install runs. Never under the bank lock.
/// Operator decision 2026-11-08 23:42Z ("free under the pause menu"): Training's pause is opening (the running state's
/// exit block, `native_source_capacity::pause_state_exit`), so every parked group (Built, left the cycle,
/// `body_bank::leaves_at_pause`) is destroyed now and native's body destroy (75-134 ms a body) falls on the opening
/// pause screen. Not while a native install runs (a background build or a switch: the next pause frees it), and never
/// a group a committed switch targets. The headroom is logged before and after each group.
pub(super) unsafe fn pause_opening() {
    for role in 0..2 {
        loop {
            let next = BANKS[role].lock().as_ref()
                .filter(|bank| bank.phase == Phase::Ready && !bank.departed && bank.entry != 0)
                .and_then(|bank| bank.groups.iter().enumerate()
                    .find(|(i, g)| g.leaving && leaves_at_pause(g.state) && bank.swap.is_none_or(|s| s.group != *i))
                    .map(|(i, g)| (bank.entry, bank.owner, i, *g.members)));
            let Some((entry, owner, group, members)) = next else { break; };
            if super::INSTALL.active() {
                trace!("[GroupSwap] pause frees owner={} group={} kinds={:x?} deferred: a native install runs; the next pause frees it",
                    owner, group, &members.kinds()[..members.count()]);
                break;
            }
            let before = MemorySample::capture();
            let started = now_us();
            leave_group(entry as *const u8, owner, role, group);
            let us = now_us().saturating_sub(started);
            let after = MemorySample::capture();
            let still = BANKS[role].lock().as_ref().and_then(|bank| bank.groups.get(group))
                .is_some_and(|g| g.leaving && leaves_at_pause(g.state));
            trace!("[GroupSwap] pause frees owner={} group={} kinds={:x?} us={} headroom={:?}->{:?} committed={:?}->{:?} freed={}",
                owner, group, &members.kinds()[..members.count()], us, before.headroom(), after.headroom(),
                before.committed.get(), after.committed.get(), !still);
            if !still { let mut watch = RELEASE_WATCH.lock(); *watch = watch.freed(); }
            if still { break; }
        }
    }
}

unsafe fn leave_group(entry: *const u8, owner: u32, role: usize, group: usize) {
    let leave_started = now_us();
    let base = BASE.load(Ordering::Acquire);
    // Inbox 02:00Z: a PT pool still retiring is emptied before its holder is destroyed below.
    finish_retirement(base, role, owner, "left the cycle");
    finish_preparation(base, role, owner, "left the cycle");
    let Some(window) = BANKS[role].lock().as_ref()
        .filter(|bank| bank.entry == entry as usize && bank.owner == owner).map(|bank| *bank.members) else { return; };
    let copy = live_kirby_copy_kind(entry, owner, window);
    let mut owners = FixedVec::<(u32, Members), NATIVE_MEMBERS>::new();
    let (bundle, own, alias, members, before, mii_set) = {
        let state = BANKS[role].lock();
        let Some(bank) = state.as_ref().filter(|bank| bank.entry == entry as usize && bank.owner == owner) else { return; };
        let Some(g) = bank.groups.get(group).filter(|g| g.leaving && g.state != GroupState::Prefetching) else { return; };
        let mut detached = DetachedSnapshot::new(owner);
        match (g.state, g.saved) {
            (GroupState::Built, Some(saved)) => {
                for (i, kind) in g.members.kinds().iter().enumerate().take(g.members.count()) {
                    let body = g.bodies[i];
                    detached.bodies.push(BodyRef { id: body.id, kind: *kind, cell: i as u8,
                        activated: bank.activated.contains(&(body.address, body.id)),
                        valid: valid_parked_body(base, body, *kind, owner), registered: Some(*kind) });
                    owners.push((body.id, *g.members));
                }
                for context in saved.group_ctx.iter().copied().filter(|c| *c != 0) {
                    detached.contexts.push(graph::context_ref(context, valid_context(base, context, owner)));
                }
                detached.dispatch = Some(DispatcherRef { arrays: saved.dispatch,
                    valid: saved.dispatch.iter().all(|a| *a == 0 || dispatcher_array_ok(*a)) });
                if saved.records != [0, 0, 0] {
                    detached.records = Some(RecordsRef { records: saved.records,
                        valid: g.members.carries_records() && valid_special_object(base, saved.records[0], owner) });
                }
                // Inbox 00:18Z: its re-acquired object kind goes with it (the plan's key step).
                if g.object_held { detached.held_kinds.push(PT_OBJECT_KIND); }
            }
            (state, _) if state.holds_keys() => {
                let (kinds, count) = g.members.prefetch_kinds();
                for kind in kinds[..count].iter().copied() { detached.held_kinds.push(kind); }
            }
            _ => {}
        }
        let mii_set = g.saved.filter(|_| g.state == GroupState::Built).and_then(|w| w.mii_r);
        // Its keys go with its own words (`group_words`: a Mii's identity; X6, beside a live Mii window the baseline).
        let own = (g.state.holds_keys() || g.object_held)
            .then_some((g.identity, bank.identity.is_some(), bank.ordinary_identity.map(|(words, _)| words)));
        (detached, own, bank.alias, *g.members, g.state, mii_set)
    };
    // Without the alias R nothing is touched (`retire_groups`' rule); the bank's own retirement follows.
    if alias == 0 || resource(base, owner) != Some(alias) { return; }
    let has_bodies = !bundle.bodies.is_empty();
    let mut registry = if has_bodies { NativeRegistry::new(base, alias, owner) } else { None };
    if has_bodies && registry.is_none() {
        trace!("[Registry] leave owner={} group={} registry unreadable; the group stays leaving", owner, group);
        return;
    }
    let cells = registry.as_mut().map(|registry| {
        let before = registry.cells();
        copy_registry::teardown_begin(registry, window);
        (before, registry.cells())
    });
    let consumers = graph::probe_consumers(base, owner);
    let views = [bundle.view()];
    let plan = DetachedPlan { groups: &views, consumers: &consumers };
    let keys = detached_keys(&plan);
    let occupancy = occupancy_snapshot(role, copy);
    let words = own.and_then(|(identity, window_identity, baseline)|
        own_words(members, identity, window_identity, baseline, save_resource_words(alias)));
    let mut release = |_: &[u32]| {
        let mut outcome = (0usize, 0usize);
        with_words(alias, words, || {
            outcome = release_group_keys(base, role, Some(group), None, &occupancy, owner, "left the cycle");
        });
        outcome
    };
    let mut current: Option<Members> = None;
    // Inbox 22:17Z: when each destroy step started (`before_step`), for the step durations on the line below.
    let mut marks = FixedList::<u64, 16>::new(0);
    let pads_before = pad_controllers(base);
    let execute_started = now_us();
    let receipt = {
        let mut before_step = |body: Option<u32>| {
            marks.push(now_us());
            let Some(registry) = registry.as_mut() else { return; };
            let wanted = match body {
                Some(id) => owners.iter().find(|(body, _)| *body == id).map(|(_, group)| *group),
                None => Some(window),
            };
            if let Some(members) = wanted.filter(|m| current != Some(*m)) {
                copy_registry::teardown_group(registry, members);
                current = Some(members);
            }
        };
        graph::execute(base, &plan, &keys, owner, "left the cycle", &mut release, &mut before_step)
    };
    let execute_ended = now_us();
    let pads = pad_changes(&pads_before, &pad_controllers(base));
    if !pads.is_empty() {
        let restored = restore_live_pads(base, entry as usize, &pads_before);
        trace!("[GroupSwap] left cycle owner={} group={} pad controllers changed (slot, before, after)={:x?} live restored={:?}",
            owner, group, &*pads, &*restored);
    }
    let mut step_us = [0u64; 16];
    for i in 0..marks.len() {
        let next = if i + 1 < marks.len() { marks[i + 1] } else { execute_ended };
        step_us[i] = next.saturating_sub(marks[i]);
    }
    if let (Some(registry), Some((before_cells, emptied))) = (registry.as_mut(), cells) {
        copy_registry::teardown_end(registry, window);
        trace!("[Registry] leave owner={} group={} window={:x?} group={:x?} cells={:x?}->{:x?}->{:x?} header={:x?}",
            owner, group, &window.kinds()[..window.count()], &members.kinds()[..members.count()], before_cells, emptied,
            registry.cells(), registry.header());
    }
    // SPECIAL-GROUPS §1 (c): the group's own Mii set goes with it.
    if let Some(set) = mii_set { release_mii_r(base, set); }
    let held = r_handle::forget_group(ledger_role(role), group);
    {
        let mut state = BANKS[role].lock();
        if let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner) {
            if let Some(g) = bank.groups.get_mut(group) {
                g.state = GroupState::Released;
                g.leaving = false;
                // Its re-acquired object kind went with the group's other tokens above.
                g.object_held = false;
                g.saved = None;
                g.bodies = [Body::default(); NATIVE_MEMBERS];
            }
            GROUP_BODIES[role].store(bank.groups.iter().filter(|g| g.state.counts_bodies())
                .map(|g| g.members.count()).sum::<usize>().min(usize::from(u8::MAX)) as u8, Ordering::Release);
        }
    }
    trace!("[GroupSwap] left cycle owner={} group={} kinds={:x?} state={:?} released bodies={} contexts={} routes={} dispatchers={} records={} keys={:x?} skipped={} ledger_held={} map_after={} pool={:?} execute_us={} step_us={:?} us={}",
        owner, group, &members.kinds()[..members.count()], before, receipt.bodies, receipt.contexts, receipt.routes_cleared,
        receipt.dispatchers, receipt.records, &*keys, receipt.skipped, held,
        fields::read::<u64>(alias, 0x18).map_or(-1i64, |m| m as i64), pool_counts(base),
        execute_ended.saturating_sub(execute_started), &step_us[..marks.len()], now_us().saturating_sub(leave_started));
    r_handle::reconcile(ledger_role(role), "left the cycle");
    // Operator option C (F6): what this free left queued for the game's unload; the release watch waits for it, then
    // for a purge (`res_unload`). A Built group's bodies released their own handles above.
    let scan_started = now_us();
    if let Some((directories, files, complete)) = res_unload::capture() {
        trace!("[GroupSwap] release queued owner={} group={} directories={} files={} complete={} scan_us={}",
            owner, group, directories, files, complete, now_us().saturating_sub(scan_started));
    }
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
        r_handle::expect_free(ledger_role(role), owner, "mod reload");
    }
}

/// rc.6.1 target-first: called by `reload_at_pose` after [`before_reload`], with the descriptor native is about to load.
/// A canonical Reload that repeats a Ready one-member ordinary window's words (`body_bank::canonical_replaces_window`)
/// replaces that window, but `descriptor()` still matches it: retire the bank before native loads, as the tick's
/// canonical replacement would have. A bank Reload's incoming bank is Building and is left alone.
pub(super) unsafe fn retire_for_canonical(entry: *const u8, descriptor: &[u8; 0x70]) {
    if !aligned(entry) { return; }
    let owner = read::<u32>(entry, 0xe0);
    let Some(role) = role(owner) else { return; };
    let word = |at: usize| u32::from_le_bytes([descriptor[at], descriptor[at + 1], descriptor[at + 2], descriptor[at + 3]]);
    let kind = {
        let mut state = BANKS[role].lock();
        let Some(bank) = state.as_mut().filter(|bank| bank.entry == entry as usize && bank.owner == owner
            && canonical_replaces_window(*bank.members, bank.phase, word(0x14), [word(0x18), word(0x1c), word(0x20)], word(0x28)))
            else { return; };
        bank.phase = Phase::Retired;
        bank.exchange = None;
        bank.groups.clear();
        EXTRAS[role].store(0, Ordering::Release);
        GROUP_BODIES[role].store(0, Ordering::Release);
        bank.members.kinds()[0]
    };
    // Native's canonical path takes a free R (state 6): the ledger starts over, as at a canonical replacement.
    r_handle::reset(ledger_role(role), "canonical reload of a one-member window's shape");
    trace!("[BodyBank] canonical reload owner={} kind={:#x}: the one-member window is retired before native loads its shape",
        owner, kind);
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
        let state = BANKS[role(owner).unwrap_or(0)].lock();
        if !state.as_ref().is_some_and(|bank| matches!(bank.phase, Phase::Ready | Phase::Changing)) { return; }
    }
    let flags = read::<u8>(entry, 0x591f);
    let (contexts, aliases) = ([read::<usize>(entry, 0x4188), read::<usize>(entry, 0x4190), read::<usize>(entry, 0x4198)],
        [read::<usize>(entry, 0x4178), read::<usize>(entry, 0x4180)]);
    if flags & BANK_FLAG == 0 {
        trace!("[IcPair] teardown owner={} reason={} flags={:#x} INVARIANT VIOLATED: bit 2 clear while a destroyer can run (651660 frees +0x4188/90/98 only on its bit-2 path); not repaired (operator decision #6) index={} loader={:#x} outer={} contexts=[{:#x},{:#x},{:#x}] aliases=[{:#x},{:#x}] states=[{:?},{:?},{:?}]",
            owner, reason, flags, read::<u8>(entry, 0x5918), read::<u8>(entry, 0x591c), read::<u8>(entry, 0x5920),
            contexts[0], contexts[1], contexts[2], aliases[0], aliases[1],
            window_cells(entry as usize)[0].and_then(|b| fields::read::<u8>(b, 0x3a)),
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
    for value in &RELEASE_MEMBERS { value.store(0, Ordering::Release); }
    *RELEASE_WATCH.lock() = ReleaseWatch::Settled;
    res_unload::clear();
    // P-HOT B (§12): a start-up window the new scene's own configure published before this callback saw the scene
    // change belongs to the new scene; everything else departs as before.
    let now = crate::runtime::lobby::game_mode::current().map(|m| (m.mode, u32::from(m.melee)));
    let mut fresh = [false; 2];
    for (role, state) in BANKS.iter().enumerate() {
        if let Some(bank) = state.lock().as_mut() {
            fresh[role] = bank.scene.is_some() && bank.scene == now && bank.phase != Phase::Retired;
            if !fresh[role] { bank.departed = true; }
        }
    }
    for role in (0..2).filter(|role| !fresh[*role]) { r_handle::reset(ledger_role(role), "scene changed"); }
    // The scene loop resets every holder, G68 included: the copy holdings end without native calls.
    for role in (0..2).filter(|role| !fresh[*role]) {
        crate::runtime::roster::copy_packages::forget(role, role as u32, "scene changed");
    }
}

/// P-HOT B: called by `character_roster::scene_changed` after `mii_saved_alpha::scene_changed`, which opens the new
/// scene's owner generation. A start-up window kept above was stamped at H1 with the generation before it, so
/// `group_owner` refused its first group exchange (rc6-hot-xps1, 2026-11-08); it joins the new generation here. A
/// departed or retired bank keeps its stamp.
pub(super) fn adopt_scene_generation() {
    let now = crate::runtime::lobby::game_mode::current().map(|m| (m.mode, u32::from(m.melee)));
    for (role, state) in BANKS.iter().enumerate() {
        if let Some(bank) = state.lock().as_mut() {
            let fresh = !bank.departed && bank.phase != Phase::Retired && bank.scene.is_some() && bank.scene == now;
            bank.epoch = crate::engine::swap::entry_window::scene_generation(bank.epoch, super::mii_saved_alpha::current_generation(role as u32), fresh);
        }
    }
}

pub(super) fn on_destroy(entry: *const u8) {
    for (role, state) in BANKS.iter().enumerate() {
        if let Some(bank) = state.lock().as_mut() {
            if bank.entry == entry as usize {
                bank.phase = Phase::Retired; bank.departed = true;
                bank.exchange = None;
                r_handle::reset(ledger_role(role), "entry destroyed");
                crate::runtime::roster::copy_packages::forget(role, bank.owner, "entry destroyed");
                EXTRAS[role].store(0, Ordering::Release);
                GROUP_BODIES[role].store(0, Ordering::Release);
                WINDOW_WAITING[role].store(0, Ordering::Release);
                if !bank.groups.is_empty() || bank.prefetch.is_some() {
                    // Finalize ran before this destructor hook; the groups should be gone.
                    trace!("[GroupSwap] destroy owner={} with {} group records remaining (leak reported)", bank.owner, bank.groups.len());
                    bank.groups.clear();
                    bank.prefetch = None;
                }
                if let Some(background) = bank.background.take() {
                    // Finalize drains it; a record here is reported and its INSTALL token and skip bit end with the entry.
                    BACKGROUND_OWNERS.fetch_and(!owner_bit(bank.owner), Ordering::AcqRel);
                    if background.mii.is_some() { MII_REAL_R.store(0, Ordering::Release); }
                    super::finish_install(background.token, "background build: entry destroyed");
                    trace!("[GroupSwap] background destroy owner={} group={} UNEXPECTED: record remaining; tasks {:x?} leak (reported)",
                        bank.owner, background.group, &background.tasks[..background.members.count()]);
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
    // P-HOT B H4: a start-up window binds to its entry here, before the table hooks serve its cells.
    bind_pending(entry, role);
    let state = BANKS[role].lock();
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
        let started = now_us();
        original!()(entry, status);
        let native_us = now_us().saturating_sub(started);
        fields::write_unchecked(entry as usize, 0x591f, PAIR_ACTIVE_FLAG);
        trace!("[IcPair] N2 activate pair owner={} status={:#x} native_us={} flags {:#x}->2->6 popo_ctx={:#x} nana_ctx={:#x} +0x4178={:#x} +0x4180={:#x} states=[{:?},{:?}]",
            owner, status, native_us, before, read::<usize>(entry, 0x4188), nana, read::<usize>(entry, 0x4178),
            read::<usize>(entry, 0x4180),
            window_cells(entry as usize)[0].and_then(|b| fields::read::<u8>(b, 0x3a)),
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
            window_cells(entry as usize)[0].and_then(|b| fields::read::<u8>(b, 0x3a)),
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
                owner, id, state, saved, window_cells(entry as usize)[0].and_then(|b| fields::read::<u8>(b, 0x3a)),
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
    // rc.6 item 3: a background build's control init selects on the scratch copy of the entry (worker thread).
    let background = BACKGROUND_TABLE.load(Ordering::Acquire);
    if background != 0 && entry as usize == scratch_address() {
        ctx.registers[register].set_x(background as u64);
        return;
    }
    let Some(role) = role(read(entry, 0xe0)) else { return; };
    let state = BANKS[role].lock();
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
        let state = BANKS[role].lock();
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
    let mut supplemental = FixedList::<u32, NATIVE_MEMBERS>::new(0);
    for kind in members.kinds().iter().take(members.count()) {
        match r_handle::acquire(base, lrole, r_handle::WINDOW, *kind, Holder::Build(GroupId(u8::MAX)),
            member_recipe(members, *kind), &occupancy) {
            Ok(()) => { supplemental.push(*kind); }
            Err(LedgerRefusal::AlreadyHeld { .. }) => {}
            Err(other) => trace!("[Ledger] window acquisition refused owner={} kind={:#x} reason={:?}", owner, kind, other),
        }
    }
    trace!("[BodyBank] resource owner={} alias={:p} supplemental={:x?} native={:x?} flags r48={} r7d={} r7f={}",
        owner, resource, &*supplemental, DebugIter(native_window_requests(members.kinds(), flags).iter().map(|r| r.kind)),
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
                owner, resource, DebugIter(others.iter().map(|b| b.0))),
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
    crate::runtime::threads::headroom("finalize_entry");
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
/// P-HOT B H1: the scene configure's `str w13,[x20,#0x80]` (§12).
const SCENE_HEADER: usize = 0x66de14;
const SCENE_HEADER_SEAMS: &[&str] = &["body_bank.scene_header"];
const GROUP_SWAP_SEAMS: &[&str] = &["group_swap.dispatcher_destroy", "group_swap.sync_construct",
    "group_swap.destroy_body", "group_swap.select_control", "group_swap.release_kind", "group_swap.acquire_kind",
    "group_swap.prepare_entry", "group_swap.unprepare", "group_swap.roster_refresh",
    "group_swap.copy_acquire", "group_swap.copy_release"];
/// Phase B′ rows (`0x655320` is `capacity.load_655320`, the capacity module's row at the same site; `0x654dc0` is
/// `body_bank.prepare_models` in `BANK_SEAMS`).
const ASYNC_BUILD_SEAMS: &[&str] = &["group_swap.body_alloc", "group_swap.heap_alloc", "group_swap.functor_copy",
    "group_swap.task_schedule", "group_swap.task_callback", "group_swap.task_wait", "group_swap.control_init",
    "group_swap.construct_functor", "group_swap.control_callback", "group_swap.control_functor", "capacity.load_655320"];
/// rc.6 item 3, Mii groups (SPECIAL-GROUPS §1 (b)): the worker hooks of a background Mii build.
const BACKGROUND_MII_SEAMS: &[&str] = &["background_mii.init_enter", "background_mii.init_flags", "background_mii.init_leave",
    "background_mii.init_exit", "background_mii.construct_index_a", "background_mii.construct_index_b",
    "background_mii.construct_enter", "background_mii.construct_path", "background_mii.tail_release", "background_mii.construct_exit", "background_mii.reader_costume",
    "background_mii.reader_hat", "background_mii.reader_6e655c", "background_mii.reader_paths",
    "background_mii.reader_33c3468", "background_mii.trainer_enter", "background_mii.trainer_exit",
    "background_mii.reader_33c3370"];
const PAIR_SEAMS: &[&str] = &["ic_pair.partner_window", "ic_pair.pair_partner", "ic_pair.activate_entry",
    "ic_pair.commit_change", "ic_pair.outgoing_owner", "ic_pair.deactivate_body"];
const SPECIAL_SEAMS: &[&str] = &["special_groups.object_release", "special_groups.heap_free",
    "special_groups.release_key"];
/// Inbox 02:00Z: the PT exit's staged retirement (pass 2 of `0x3a6b80`, made by the bank).
const EXIT_SPLIT_SEAMS: &[&str] = &["special_groups.object_lookup", "special_groups.object_destroy",
    "special_groups.static_init"];
/// rc.6 item 3, SPECIAL-GROUPS §3: the X5 hook and native state 11's holder step.
const NATIVE_BACKGROUND_SEAMS: &[&str] = &["background_build.aegis_partner", "background_build.trainer_functor",
    "background_build.trainer_callback", "background_build.guard_acquire", "background_build.guard_release",
    "background_build.pool_build"];
/// rc.6 item 3, Ice Climbers: the construction tail's pair test for background bodies (`background_tail`).
const BACKGROUND_TAIL_SEAMS: &[&str] = &["background_build.construct_tail"];
/// rc.6 item 3, X4: the construction's info wrapper for background Ice Climbers bodies (`background_info`).
const BACKGROUND_INFO_SEAMS: &[&str] = &["background_build.info_wrapper"];

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
    for storage in &BANK_STORAGE {
        let mut storage = storage.lock();
        if storage.is_none() { *storage = Some(BankStorage::new()); }
    }
    skyline::install_hooks!(prepare_models, prepare_resources, create_contexts, select_context);
    ENABLED.store(true, Ordering::Release);
    // P-HOT B H1 (§12): the scene configure's header site, on its own row; without it Training starts as today.
    let scene_stock = fields::read_unchecked::<u32>(base, SCENE_HEADER) == 0xb900828d;
    if !scene_stock { trace!("[BodyBank] fingerprint mismatch +0x{:x}; no start-up window", SCENE_HEADER); }
    if gate(SEAM_GROUP, SCENE_HEADER_SEAMS, scene_stock) { skyline::install_hooks!(scene_header); }
    // WP-R: per-body identity view (all 18 selector exits or none; logs its own line).
    let identity_view = crate::runtime::roster::identity_view::install(base);
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
    // Inbox 02:00Z: pass 2's own calls, which a staged PT exit makes itself (`special_articles::destroy_saved`).
    let split_ok = gate(SEAM_GROUP, EXIT_SPLIT_SEAMS, special_ok
        && fingerprint(base, articles::OBJECT_LOOKUP, &articles::OBJECT_LOOKUP_WORDS, "SpecialGroups")
        && fingerprint(base, articles::OBJECT_DESTROY, &articles::OBJECT_DESTROY_WORDS, "SpecialGroups")
        && fingerprint(base, articles::STATIC_INIT, &articles::STATIC_INIT_WORDS, "SpecialGroups")
        && fingerprint(base, articles::GUARD_ACQUIRE, &[0xf000c6f0, 0xf9411e11, 0x9108e210, 0xd61f0220], "SpecialGroups")
        && fingerprint(base, articles::GUARD_RELEASE, &[0xf000c6f0, 0xf9412211, 0x91090210, 0xd61f0220], "SpecialGroups"))
        && special_ok;
    EXIT_SPLIT.store(split_ok, Ordering::Release);
    trace!("[SpecialGroups] PT exit split {}", if split_ok { "enabled; the pool retires over the next ticks" } else { "off; the exit's tick releases the whole pool" });
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
    // rc.6 item 3: background builds use the Phase B′ path and the identity view's body-init site (`background_skip`).
    BACKGROUND_ENABLED.store(async_ok && identity_view, Ordering::Release);
    // rc.6 item 3, Ice Climbers (SPECIAL-GROUPS §0 X1, §2), and the one tail hook a background Mii body's `x20` restore
    // also needs (22:52Z): the tail's pair test for background bodies, its first four
    // 13.0.5 words owned (`cmp w8, #0x4b; b.ne 0x60debc; ldr w8, [x9, #0x1c]; cmp w8, #0x4c`). Without it a live
    // `[Popo, Nana]` window keeps deferring the background builds.
    let tail_stock = fingerprint(base, BACKGROUND_TAIL, &[0x71012d1f, 0x54000281, 0xb9401d28, 0x7101311f], "GroupSwap");
    let tail_ok = gate(SEAM_GROUP, BACKGROUND_TAIL_SEAMS, async_ok && identity_view && tail_stock)
        && async_ok && identity_view && tail_stock;
    if tail_ok {
        skyline::install_hooks!(background_tail);
        BACKGROUND_PAIR_TAIL.store(true, Ordering::Release);
    }
    trace!("[GroupSwap] background pair tail {}",
        if tail_ok { "enabled; a live Ice Climbers pair window builds the other groups" } else { "off; a live Ice Climbers pair window waits" });
    // SPECIAL-GROUPS §1: the Mii groups' worker hooks, all or none, their first 13.0.5 words owned.
    // `0x60de68` is `background_tail`'s (row `background_build.construct_tail`, checked and installed above).
    // X6: Pokémon Trainer's pool task (`bg_trainer_enter`/`bg_trainer_exit`) takes the shadow beside a live Mii window.
    let mii_sites: [(usize, u32); 18] = [(0x607aac, 0xd102c3a0), (0x607d80, 0x1a9f311a), (0x607e74, 0xaa1803f4),
        (0x60b5c8, 0xa9577bfd), (0x60b7a8, 0x9343fd6b), (0x60b834, 0xcb0a016b), (0x60b98c, 0x9447780d),
        (0x60bf30, 0x52800283), (0x60dec4, 0x944769d3), (0x60df04, 0xa9444ff4),
        (0x33a05a8, 0xb9405909), (0x344fcd4, 0xf9404717), (0x6e655c, 0xb9405aa8), (0x6e6bc4, 0xb9401262),
        (0x33c3468, 0xb9405aa8), (0x651a70, 0x52800e42), (0x651b50, 0xb9400be1), (0x33c33a8, 0x2a0103f4)];
    let mii_stock = mii_sites.iter().all(|&(offset, word)| fingerprint(base, offset, &[word], "BackgroundMii"));
    // A background Mii body's `x20` is restored by `background_tail`, so the Mii hooks need it installed.
    let mii_ok = gate(SEAM_GROUP, BACKGROUND_MII_SEAMS, async_ok && identity_view && mii_stock && tail_ok)
        && async_ok && identity_view && tail_ok;
    if mii_ok {
        skyline::install_hooks!(bg_mii_init_enter, bg_mii_init_flags, bg_mii_init_leave, bg_mii_init_exit,
            bg_mii_construct_index_a, bg_mii_construct_index_b, bg_mii_construct_enter, bg_mii_construct_path,
            bg_mii_tail_release, bg_mii_construct_exit, bg_mii_reader_costume, bg_mii_reader_hat,
            bg_mii_reader_6e655c, bg_mii_reader_paths, bg_mii_reader_33c3468, bg_trainer_enter, bg_trainer_exit,
            bg_mii_reader_33c3370);
        BACKGROUND_MII_ENABLED.store(true, Ordering::Release);
    }
    trace!("[GroupSwap] background Mii builds {}", if mii_ok { "enabled" } else { "off; a Mii group builds at its first switch" });
    trace!("[GroupSwap] background build {} async={} identity_view={}",
        if async_ok && identity_view { "enabled" } else { "off; a group builds at its first switch" }, async_ok, identity_view);
    // SPECIAL-GROUPS §3: Pokémon Trainer and Pyra/Mythra groups in the background need the X5 site (`background_partner`)
    // and native state 11's holder step: the trainer functor through its relocated vtable slot, its callback, the guard
    // functions of the Pyra/Mythra table's lazy init and the pool build (in `special_ok`). Without them a native group
    // builds at its first switch, as before.
    let trainer_slot = fields::read_unchecked::<usize>(base, TRAINER_FUNCTOR_VTABLE + 0x30) == base + TRAINER_FUNCTOR;
    let native_ok = gate(SEAM_GROUP, NATIVE_BACKGROUND_SEAMS, async_ok && identity_view && special_ok && trainer_slot
        && fingerprint(base, PARTNER_SITE - 4, &[0xb9401ea8, 0x3100051f, 0x54000380, 0x52800ea2], "GroupSwap")
        && fingerprint(base, TRAINER_FUNCTOR, &[0xd10143ff, 0xf9000bf7, 0xa90257f6, 0xa9034ff4, 0xa9047bfd, 0x910103fd, 0xf9400415, 0x39424aa8], "GroupSwap")
        && fingerprint(base, TRAINER_CALLBACK, &[0x3941c008, 0x34000048, 0xd65f03c0, 0xf9403000, 0xb4000080, 0xf9400008, 0xf9401901, 0xd61f0020], "GroupSwap")
        && fingerprint(base, GUARD_ACQUIRE, &[0xf000c6f0, 0xf9411e11, 0x9108e210, 0xd61f0220], "GroupSwap")
        && fingerprint(base, GUARD_RELEASE, &[0xf000c6f0, 0xf9412211, 0x91090210, 0xd61f0220], "GroupSwap"))
        && async_ok && identity_view && special_ok && trainer_slot;
    if native_ok { skyline::install_hooks!(background_partner); }
    NATIVE_BACKGROUND.store(native_ok, Ordering::Release);
    trace!("[GroupSwap] background build of Pokémon Trainer and Pyra/Mythra groups {} trainer_slot={}",
        if native_ok { "enabled" } else { "off; they build at their first switch" }, trainer_slot);
    // Inbox 02:42Z: the prepared pool build's functor. The trainer functor vtable's relocated slots must be the clones it
    // replaces (`0x651970`, `0x6519f0`: they write the native vtable) and the functor (`+0x30`); the others are copied.
    let prepare_ok = native_ok
        && fields::read::<usize>(base, TRAINER_FUNCTOR_VTABLE + 0x10) == Some(base + TRAINER_CLONE_HEAP)
        && fields::read::<usize>(base, TRAINER_FUNCTOR_VTABLE + 0x18) == Some(base + TRAINER_CLONE_INTO)
        && fields::read::<usize>(base, TRAINER_FUNCTOR_VTABLE + 0x30) == Some(base + TRAINER_FUNCTOR)
        && fingerprint(base, TRAINER_CLONE_HEAP, &[0xd100c3ff, 0xf9000bf3, 0xa9027bfd, 0x910083fd], "GroupSwap")
        && fingerprint(base, TRAINER_CLONE_INTO, &[0x90024988, 0x91126108, 0xf9000028, 0xf9400408], "GroupSwap");
    if prepare_ok {
        for (i, slot) in PREPARE_VTABLE.iter().enumerate() {
            let value = match i * 8 {
                0x10 => prepare_clone_heap as *const () as usize,
                0x18 => prepare_clone_into as *const () as usize,
                0x30 => prepare_call as *const () as usize,
                offset => fields::read_unchecked::<usize>(base, TRAINER_FUNCTOR_VTABLE + offset),
            };
            slot.store(value, Ordering::Release);
        }
    }
    PREPARE_ENABLED.store(prepare_ok, Ordering::Release);
    trace!("[GroupSwap] prepared Pokémon Trainer pools {}",
        if prepare_ok { "enabled; a detached group's retired pool is built on the task worker" } else { "off; the attach rebuilds the pool" });
    let info_stock = fingerprint(base, BACKGROUND_INFO, &[0xf9400529, 0x39425129, 0x35001da9, 0xf945bd08], "GroupSwap");
    let info_ok = gate(SEAM_GROUP, BACKGROUND_INFO_SEAMS, async_ok && identity_view && info_stock)
        && async_ok && identity_view && info_stock;
    if info_ok {
        skyline::install_hooks!(background_info);
        BACKGROUND_X4.store(true, Ordering::Release);
    }
    trace!("[GroupSwap] background info wrapper {}",
        if info_ok { "enabled; a background Ice Climbers body takes its group's wrapper" } else { "off" });
    if groups_ok {
        // The `[GroupSwap] enabled` line follows the startup cycle-preload load
        // (`report_group_swap`), so it shows the switches this boot applies.
        GROUPS_ENABLED.store(true, Ordering::Release);
        // Operator option C: the release watch's rows (`res_unload`); without them a free never holds the planner.
        res_unload::install(base, gate("res_unload", res_unload::SEAMS, false));
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
