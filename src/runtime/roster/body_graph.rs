//! Native execution of the teardown protocol (`swap_model::lifecycle`): the ONLY
//! caller of the body destroyer `0x3aeb50`, the context destructor (vtable `+8`,
//! as `651660`), the dispatcher-array destructor `0x37ae5e0`, the object-record
//! destroyer pair `0x3a6b80`/`0x392eb40` and the pad-route null (D-PTA1c).
//!
//! The plan is computed by `lifecycle::destroy_plan` from value records the body
//! bank validated in the current callback; `execute` runs the steps in plan order
//! with no bank lock held and re-reads the one word it conditionally writes (the
//! route slot) right before writing it. Facts, with their sources:
//! * Pad manager `P = *(main+0x52b6fd8)`, `S = *P`, `S+0x98` → 16 route objects
//!   `A_i` (0x20 bytes each); `A->+0` = the routed controller (`ctx+0x58`),
//!   `A->+8` = tick disabled, `A->+0x14` = frame counter; the tick `main+0x2f7b80`
//!   dereferences `**(A->+0)` whenever `A->+8 == 0`; only `313b00(mgr, idx, flag,
//!   obj != 0)` writes `A->+0`; the context destructor `6b7920` frees the
//!   controller and leaves `A->+0` stale (D-PTA1BC-FIX §2.3–§2.5). Nulling the
//!   slot turns the tick into its no-route path (`cbz` → counter only), validated
//!   live in session dpta5 (§2.1: 0 halts in 40 s with the route nulled).
//! * `0x3aeb50(id)` derives the slab from the id (`ubfx w8,w0,#16,#12`, stride
//!   `0xf940`) and dereferences a null lookup for a stale id (GROUP-SWAP-IMPL
//!   §4.1): the bank validates `valid_parked_body` before a `DestroyBody` step.
//! * `0x37ae5e0(info+8)` reads only `+8/+0x10` of the 0x20-byte dispatcher struct
//!   and writes `+0/+8/+0x10/+0x18`, so a stack copy of the two array pointers is
//!   what finalize's own call sees (`0x6531bc`; GROUP-SWAP-IMPL §14.1).
//! * Finalize destroys a native group's object through `0x3a6b80(obj)` then
//!   `0x392eb40(obj)` and frees `+0x5a98` (`0x652f5c-0x652f84`); loader state 1
//!   does the same (`0x651288`).
//! * WP-K (`refactor/wp-k/RE-KIRBY-UNLOAD.md`): `unprepare 0x17f26c0`'s Kirby
//!   branch walks the per-`(entry, sub)` copy-tuple list, which construction
//!   fills assuming Kirby is sub 0; no native step repairs a Kirby at cell 1/2 at
//!   teardown, so the plan only reports them (`kirby_misplaced=`) and the planner
//!   keeps Kirby at cell 0 or canonical-only (K-rows decide, WORKPLAN §2.14).
//!   The window and every detached group share those per-`(entry, sub)` lists; U18
//!   (`swap_model::copy_registry`) keeps them the attached window's, so a detached
//!   body unloads over empty cells (`execute_with`'s hook, `retire_groups`).
use crate::engine::seams::native_fields as fields;
use crate::engine::swap::swap_model::lifecycle::{AttachedPlan, ContextRef, DetachedPlan, DestroyStep, NativeConsumers, RouteRef, PAD_SLOTS};
use crate::engine::compat::fixed::FixedList;

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

pub(crate) const DESTROY_BODY: usize = 0x3aeb50;
pub(crate) const DISPATCHER_DESTROY: usize = 0x37ae5e0;
pub(crate) const SPECIAL_OBJECT_RELEASE: usize = 0x3a6b80;
pub(crate) const HEAP_FREE: usize = 0x392eb40;
pub(crate) const PAD_MANAGER: usize = 0x52b6fd8;
pub(crate) const PAD_ROUTE_TABLE: usize = 0x98;
/// `ctx+0x58`: the controller object the route names (`ctx+0x6c` holds the pad slot, which the
/// plan does not use: a Nana context is routed at `owner+8`, so routes are matched by controller).
pub(crate) const CONTEXT_CONTROLLER: usize = 0x58;
/// The window's context words: the selected context `+0x4178`, the pair alias
/// `+0x4180`, and the per-cell contexts `+0x4188/+0x4190/+0x4198`.
pub(crate) const WINDOW_CONTEXT_OFFSETS: [usize; 5] = [0x4178, 0x4180, 0x4188, 0x4190, 0x4198];

/// `S->+0x98[slot]`, validated link by link; `None` when the chain does not resolve.
unsafe fn route_object(base: usize, slot: u8) -> Option<usize> {
    if usize::from(slot) >= PAD_SLOTS { return None; }
    let state = fields::chain(base, &[PAD_MANAGER, 0])?;
    fields::chain(state, &[PAD_ROUTE_TABLE, usize::from(slot) * 8])
}

/// Values only: every route slot's controller word and Nana's `owner+8` word.
pub(crate) unsafe fn probe_consumers(base: usize, owner: u32) -> NativeConsumers {
    let mut consumers = NativeConsumers::default();
    for slot in 0..PAD_SLOTS as u8 {
        let Some(route) = route_object(base, slot) else { continue; };
        if let Some(controller) = fields::read::<usize>(route, 0) {
            if controller != 0 { consumers.pad_routes[usize::from(slot)] = Some(RouteRef { slot, controller }); }
        }
    }
    let nana = owner.saturating_add(8);
    if (nana as usize) < PAD_SLOTS { consumers.nana_owner_plus_8 = consumers.pad_routes[nana as usize].map(|r| r.controller); }
    consumers
}

/// A plan record for one context: the controller word is read only when the
/// caller's validation passed (an unvalidated context has no known controller,
/// so its route is left to native).
pub(crate) unsafe fn context_ref(context: usize, valid: bool) -> ContextRef {
    let controller = if valid { fields::read::<usize>(context, CONTEXT_CONTROLLER).unwrap_or(0) } else { 0 };
    ContextRef { ctx: context, controller, valid }
}

/// The attached window's contexts, deduplicated across `+0x4178/+0x4180/+0x4188/90/98`
/// (the aliases select among the cell contexts; GROUP-SWAP-FEASIBILITY §2.2).
pub(crate) unsafe fn window_contexts(entry: usize, mut valid: impl FnMut(usize) -> bool)
    -> FixedList<ContextRef, { WINDOW_CONTEXT_OFFSETS.len() }> {
    let mut out = FixedList::new(ContextRef { ctx: 0, controller: 0, valid: false });
    for offset in WINDOW_CONTEXT_OFFSETS {
        let Some(context) = fields::child(entry, offset) else { continue; };
        if out.iter().any(|c| c.ctx == context) { continue; }
        let ok = valid(context);
        out.push(context_ref(context, ok));
    }
    out
}

/// D-PTA1c: null `S->+0x98[slot]->+0` if it still equals `controller`. `Some(true)` = cleared,
/// `Some(false)` = the slot no longer names this controller (native re-pointed it, or an earlier
/// plan cleared it), `None` = the chain did not validate (nothing written). The only write is the
/// 8-byte null; every read is a validated field.
pub(crate) unsafe fn release_pad_route(base: usize, slot: u8, controller: usize) -> Option<bool> {
    if controller == 0 { return Some(false); }
    let route = route_object(base, slot)?;
    if fields::read::<usize>(route, 0)? != controller { return Some(false); }
    fields::write_unchecked(route, 0, 0usize);
    Some(true)
}

unsafe fn destroy_body(base: usize, id: u32) {
    let destroy: unsafe extern "C" fn(u32) = std::mem::transmute(base + DESTROY_BODY);
    destroy(id);
}

unsafe fn free_context(context: usize) {
    let vtable = fields::read_unchecked::<usize>(context, 0);
    let free: unsafe extern "C" fn(*mut u8) = std::mem::transmute(fields::read_unchecked::<usize>(vtable, 8));
    free(context as *mut u8);
}

/// Destroy a detached group's dispatcher arrays with the native destructor on a
/// stack copy of the 0x20-byte struct. Zero pointers count as absent; the plan
/// validated the arrays (`dispatcher_array_ok`) in the same callback.
unsafe fn destroy_dispatcher_arrays(base: usize, arrays: [usize; 2]) -> bool {
    if arrays == [0, 0] { return true; }
    let mut dispatcher: [u64; 4] = [1, arrays[0] as u64, arrays[1] as u64, 0];
    let destroy: unsafe extern "C" fn(*mut u64) = std::mem::transmute(base + DISPATCHER_DESTROY);
    destroy(dispatcher.as_mut_ptr());
    dispatcher[1] == 0 && dispatcher[2] == 0
}

/// Finalize's destruction of the records (`0x652f5c-0x652f84`): children through
/// `0x3a6b80`, the object freed, then the PT auxiliary object freed.
unsafe fn destroy_special_records(base: usize, records: [usize; 3]) {
    let release: unsafe extern "C" fn(usize) = std::mem::transmute(base + SPECIAL_OBJECT_RELEASE);
    let free: unsafe extern "C" fn(usize) = std::mem::transmute(base + HEAP_FREE);
    if records[0] != 0 {
        release(records[0]);
        free(records[0]);
    }
    if records[2] != 0 && fields::object(records[2]) { free(records[2]); }
}

/// What `execute` did, for the caller's teardown line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DestroyReceipt {
    pub routes_cleared: usize,
    /// Route steps whose slot no longer named the controller at execution time.
    pub routes_stale: usize,
    pub bodies: usize,
    pub contexts: usize,
    pub dispatchers: usize,
    pub records: usize,
    pub keys_released: usize,
    pub keys_kept: usize,
    /// Obligations refused: the plan's `skipped` plus execution-time refusals.
    pub skipped: usize,
}

/// Attached-window route-only plan. Native performs the later destruction;
/// there are no body/context/dispatcher/record/key actions to materialize.
pub(crate) unsafe fn execute_attached(base: usize, plan: &AttachedPlan, owner: u32, reason: &str) -> DestroyReceipt {
    let mut receipt = DestroyReceipt::default();
    for route in plan.routes.iter() {
        match release_pad_route(base, route.slot, route.controller) {
            Some(true) => receipt.routes_cleared += 1,
            Some(false) => receipt.routes_stale += 1,
            None => receipt.skipped += 1,
        }
    }
    trace!("[Destroy] plan owner={} reason={} steps={} routes_cleared={} attached=NativeRetiring routes_stale={} kirby={} kirby_misplaced={} kirby_branch_mismatch={} bodies=0 contexts=0 skipped={}",
        owner, reason, plan.routes.len(), receipt.routes_cleared, receipt.routes_stale,
        plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch, receipt.skipped);
    receipt
}

/// Run the plan in order. `release_keys` is the bank's ledger release (WP4:
/// `r_handle::release` through `release_group_keys`, the orphan keys first for
/// D-SG1, the live-consumer check for D-KIRBY1), returning `(released, kept)`.
/// Never called with a bank lock held. Logs the interface line
/// `[Destroy] plan owner={} reason={} steps={} routes_cleared={} attached=NativeRetiring|None …`.
/// `before_step` receives a body id for body destruction, None for every other phase: U18's detached teardown
/// (`swap_model::copy_registry::teardown_group`) installs each group's own header before that
/// group's first `DestroyBody`, and the window's again before the first later step.
pub(crate) unsafe fn execute(base: usize, plan: &DetachedPlan<'_>, keys: &[u32], owner: u32, reason: &str,
    release_keys: &mut dyn FnMut(&[u32]) -> (usize, usize), before_step: &mut dyn FnMut(Option<u32>)) -> DestroyReceipt {
    let mut receipt = DestroyReceipt { skipped: plan.skipped().count(), ..DestroyReceipt::default() };
    let mut steps = 0;
    for step in plan.steps() {
        steps += 1;
        before_step(match &step { DestroyStep::DestroyBody { id } => Some(*id), _ => None });
        let step = &step;
        match step {
            DestroyStep::ReleasePadRoute { slot, controller } => match release_pad_route(base, *slot, *controller) {
                Some(true) => receipt.routes_cleared += 1,
                Some(false) => receipt.routes_stale += 1,
                None => receipt.skipped += 1,
            },
            DestroyStep::DestroyBody { id } => { destroy_body(base, *id); receipt.bodies += 1; }
            DestroyStep::FreeContext { ctx } => { free_context(*ctx); receipt.contexts += 1; }
            DestroyStep::DestroyDispatchers { arrays } => {
                if destroy_dispatcher_arrays(base, *arrays) { receipt.dispatchers += 1; } else { receipt.skipped += 1; }
            }
            DestroyStep::DestroyRecords { records } => { destroy_special_records(base, *records); receipt.records += 1; }
            DestroyStep::ReleaseKeys { kinds } => {
                let (released, kept) = release_keys(kinds);
                receipt.keys_released += released;
                receipt.keys_kept += kept;
                receipt.skipped += kept;
            }
        }
    }
    if !keys.is_empty() {
        steps += 1;
        before_step(None);
        let (released, kept) = release_keys(keys);
        receipt.keys_released += released;
        receipt.keys_kept += kept;
        receipt.skipped += kept;
    }
    let [kirby_cells, kirby_misplaced, kirby_branch_mismatch] = plan.kirby_counts();
    trace!("[Destroy] plan owner={} reason={} steps={} routes_cleared={} attached={} routes_stale={} kirby={} kirby_misplaced={} kirby_branch_mismatch={} bodies={} contexts={} skipped={}",
        owner, reason, steps, receipt.routes_cleared, "None",
        receipt.routes_stale, kirby_cells, kirby_misplaced, kirby_branch_mismatch, receipt.bodies, receipt.contexts, receipt.skipped);
    receipt
}
