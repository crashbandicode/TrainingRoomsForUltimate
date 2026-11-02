//! One teardown protocol for the bodies, contexts, dispatcher arrays, object
//! records and resource keys the body bank owns (WORKPLAN §2.4, SYNTHESIS
//! agree-4, Astra §5, Fable §2.2).
//!
//! Every destroy path — `retire_groups` (mod Reload, loader-0 Reload, finalize:
//! match start, Quit Training, room close), the `perform_swap` rollback and the
//! attached-window teardown before a Reload — computes one [`DestroyPlan`] here
//! and lets `swap_native::body_graph::execute` run it. This module is pure: it
//! takes value records (addresses and ids are identity tokens, never
//! dereferenced) and returns the ordered steps plus everything it refused.
//!
//! Facts the plan encodes, each with its source:
//! * **Route before free (D-PTA1c; D-PTA1BC-FIX §0.2/§3).** The pad manager's
//!   per-frame tick `main+0x2f7b80` dereferences `**(A->+0)` for every enabled
//!   route slot `A = S->+0x98[i]`; the context destructor `0x6b7920` frees the
//!   controller `ctx+0x58` that `A->+0` names and never clears `A->+0`. Only an
//!   activation (`313b00(obj)`) re-points it. So every route naming a controller
//!   of a context about to be freed — by us or by native — is nulled first.
//!   A Nana context is bound to route `owner+8` (`6b6f10`, U2 FACT), so the
//!   match is by controller over all sixteen slots, never by `ctx+0x6c` alone.
//! * **Finalize's order (GROUP-SWAP-FEASIBILITY §4.1; `0x6524e0`).** Bodies
//!   (`0x3aeb50` per id, `0x652ebc`), then the contexts (vtable `+8`, as
//!   `651660`), then the dispatcher arrays (`0x37ae5e0`, `0x6531bc`), then the
//!   object records (`0x3a6b80` + `0x392eb40`, `0x652f5c`), then the keys only
//!   registered (`release_prefetched`: orphan key first, D-SG1-RCA). The plan of
//!   several detached groups keeps that order across the groups, as
//!   `retire_groups` did.
//! * **The attached window is native's (Astra §5 step 4).** A Reload (states
//!   1–3) and finalize destroy the window themselves; our plan only clears its
//!   routes and marks it [`Attached::NativeRetiring`]. B123-pre Yoga l123a
//!   (B123PRE-LOBBY.md): 64 `Unmapped Read64 @ 0x0` inside the match-start
//!   finalize of an attached Phase-B window `[2d,2c,0]` with no route clear.
//! * **Aliases (`+0x4178/+0x4180`) are selections into `+0x4188/90/98`**, not
//!   separately freeable contexts: every context appears once in a plan.
//! * **Kirby residency (WP-K; `refactor/wp-k/RE-KIRBY-UNLOAD.md`, D-HANG1-SEPARATION
//!   §1, D-KIRBY1-ROB1-RCA §3.1).** The body unload `0x610f20` calls
//!   `unprepare 0x17f26c0(G68, kind, entry, sub)` with `sub = body+0xf816` (the
//!   body's cell in its entry); its Kirby branch (`0x17f2784–0x17f2884`) walks
//!   the per-`(entry, sub)` copy-tuple list at `G68+0x1d258+entry*0x48+sub*0x18`,
//!   which the G68 constructor `0x17eb550` initialises at scene setup and which
//!   Kirby's construction (`0x658af0` → `0x3ae080` → `0x6079d0` → `0x607f30`)
//!   fills — activation writes nothing. The game assumes Kirby is sub 0 of its
//!   entry: a Kirby constructed at window/group cell 1 or 2 registers tuples that
//!   point back at its own cell or the cell-0 member's, so unloading removes one
//!   node twice, the count reaches −1, the sentinel is freed and the walk spins
//!   (S-B `[5,6,7]` 3/3 hung, D-KIRBY1 `[7,6,8]` hung; S-K `[6,7,8]` completed).
//!   No native step repairs this at teardown, so the plan **reports** every
//!   Kirby body outside cell 0 ([`DestroyPlan::kirby_misplaced`], the
//!   `[Destroy] plan … kirby_misplaced=` field) as the runtime invariant of the
//!   planner's `CellRule::PinCell0` (WP1): Kirby always occupies physical cell 0
//!   of any window or group that contains it, the rotation order is decoupled
//!   from the cell layout and the target becomes active via `set_active_index`,
//!   so the entries the bank builds are ones native builds itself. A second
//!   route to the same hazard (WP-R, INTEGRATION "per-body identity view"):
//!   init `0x607eb4` registers a body under the home window's `R+0x58[sub]`,
//!   not its own kind, and a Phase-B group keeps the home header, so a
//!   non-Kirby group body under a Kirby home (or a Kirby group body under a
//!   non-Kirby home) registers on one `prepare_entry` branch and unloads on the
//!   other. The plan reports it ([`DestroyPlan::kirby_branch_mismatch`], the
//!   `[Destroy] plan … kirby_branch_mismatch=` field); WP-R's `identity_view`
//!   (the init fed from the body's own identity) is the fix.
//! * Objects that fail their validation are [`Skipped`], never destroyed: they
//!   leak rather than corrupt (`retire_groups` since GROUP-SWAP-IMPL §4.1).

use crate::engine::swap::body_bank::KIRBY;

/// Pad-manager route slots: `S->+0x98` holds sixteen 0x20-byte route objects
/// (`0x311db0` ticks slots `i` and `i+8` for entries 0..7).
pub const PAD_SLOTS: usize = 16;

/// A route slot naming a controller object: `S->+0x98[slot]->+0 == controller`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteRef { pub slot: u8, pub controller: usize }

/// One fighter body the bank owns. `cell` is its index in its entry's window
/// (or saved window) — native's `sub` (`body+0xf816`); `activated` is whether
/// this body was ever the window's live member (the bank's receipt); `registered`
/// is the kind native's init registered the body under — the home window's
/// `R+0x58[sub]` that `0x607eb4` passes to `prepare_entry`, not the body's own
/// kind (WP-R FACT) — when the bank read it, `None` when unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyRef { pub id: u32, pub kind: u32, pub cell: u8, pub activated: bool, pub valid: bool, pub registered: Option<u32> }

impl BodyRef {
    /// A Kirby body outside cell 0: the D-HANG1/D-KIRBY1 class (module doc).
    pub fn misplaced_kirby(&self) -> bool { self.kind == KIRBY && self.cell != 0 }
    /// Registered on one `prepare_entry` branch, unloaded on the other: the body's
    /// own kind (`unprepare` reads `body+0xc`) differs from the home header's
    /// `R+0x58[sub]` it was registered under, and one of the two is Kirby (WP-R's
    /// second route to the K-R1 hazard; WP-R's `identity_view` is the fix).
    pub fn kirby_branch_mismatch(&self) -> bool {
        self.registered.is_some_and(|registered| registered != self.kind && (registered == KIRBY || self.kind == KIRBY))
    }
}

/// One fighter context. `controller` is `ctx+0x58` (0 when unknown or absent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextRef { pub ctx: usize, pub controller: usize, pub valid: bool }

/// A detached group's entry-info dispatcher arrays (`info+0x10`, `info+0x18`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatcherRef { pub arrays: [usize; 2], pub valid: bool }

/// A native group's object records (`+0x5a88/+0x5a90/+0x5a98`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordsRef { pub records: [usize; 3], pub valid: bool }

/// Native readers holding a pointer to one of our objects (Fable §2.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeConsumers {
    /// `S->+0x98[i]->+0` for every slot, values only.
    pub pad_routes: [Option<RouteRef>; PAD_SLOTS],
    /// The controller routed at slot `owner+8` (Nana's route, U2) when the probe
    /// read it separately from the table walk; redundant with `pad_routes`.
    pub nana_owner_plus_8: Option<usize>,
}

/// The window group natively attached to its entry: cells `+0x4160/68/70` and
/// the deduplicated contexts `+0x4178/+0x4180/+0x4188/90/98`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachedGroup { pub owner: u32, pub cells: Vec<BodyRef>, pub contexts: Vec<ContextRef> }

/// A detached group (saved window fields) or a partially built group being
/// rolled back: everything we must destroy ourselves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetachedGroup {
    pub owner: u32,
    pub bodies: Vec<BodyRef>,
    pub contexts: Vec<ContextRef>,
    pub dispatch: Option<DispatcherRef>,
    pub records: Option<RecordsRef>,
    /// Kinds whose prefetch keys are still registered on R (never constructed).
    pub held_kinds: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bundle { Attached(AttachedGroup), Detached(DetachedGroup) }

impl Bundle {
    pub fn bodies(&self) -> &[BodyRef] {
        match self { Bundle::Attached(group) => &group.cells, Bundle::Detached(group) => &group.bodies }
    }
}

/// One native action, executed in plan order by `body_graph::execute`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DestroyStep {
    /// Null `S->+0x98[slot]->+0` if it still equals `controller` (D-PTA1c).
    ReleasePadRoute { slot: u8, controller: usize },
    /// `0x3aeb50(id)` after `valid_parked_body`.
    DestroyBody { id: u32 },
    /// Context vtable `+8` after `valid_context`.
    FreeContext { ctx: usize },
    /// `0x37ae5e0` on a stack copy of the dispatcher struct (after the bodies).
    DestroyDispatchers { arrays: [usize; 2] },
    /// `0x3a6b80` then `0x392eb40` (+ the PT auxiliary object), after the contexts.
    DestroyRecords { records: [usize; 3] },
    /// The kinds whose prefetch keys the bundles still hold. The bank's arm
    /// returns the ledger's tokens through `r_handle::release` (orphan key first;
    /// WP4, the only caller of `0x17e2fe0`/`0x17e8610`).
    ReleaseKeys { kinds: Vec<u32> },
}

/// What happens to the attached window after the plan: nothing (no attached
/// bundle), or native destroys it (Reload states 1–3, finalize).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Attached { #[default] None, NativeRetiring }

/// An obligation the plan refused and reports instead of executing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skipped { Body(u32), Context(usize), Dispatchers([usize; 2]), Records([usize; 3]) }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DestroyPlan {
    pub steps: Vec<DestroyStep>,
    pub skipped: Vec<Skipped>,
    pub attached: Attached,
    /// Kirby bodies in the bundles (any cell): the residency class under test.
    pub kirby_cells: usize,
    /// Kirby bodies outside cell 0: the hazard the planner must never create
    /// (module doc). Reported, never repaired here.
    pub kirby_misplaced: usize,
    /// Bodies registered under the other `prepare_entry` branch than the one
    /// their unload takes (`BodyRef::kirby_branch_mismatch`): WP-R's route to the
    /// same hazard. Reported; `identity_view` removes the cause.
    pub kirby_branch_mismatch: usize,
}

impl DestroyPlan {
    pub fn routes_cleared(&self) -> usize {
        self.steps.iter().filter(|s| matches!(s, DestroyStep::ReleasePadRoute { .. })).count()
    }
    pub fn bodies(&self) -> usize { self.steps.iter().filter(|s| matches!(s, DestroyStep::DestroyBody { .. })).count() }
    pub fn contexts(&self) -> usize { self.steps.iter().filter(|s| matches!(s, DestroyStep::FreeContext { .. })).count() }
    pub fn dispatchers(&self) -> usize { self.steps.iter().filter(|s| matches!(s, DestroyStep::DestroyDispatchers { .. })).count() }
    pub fn records(&self) -> usize { self.steps.iter().filter(|s| matches!(s, DestroyStep::DestroyRecords { .. })).count() }
    pub fn keys(&self) -> &[u32] {
        self.steps.iter().find_map(|s| match s { DestroyStep::ReleaseKeys { kinds } => Some(kinds.as_slice()), _ => None })
            .unwrap_or(&[])
    }
    /// Nothing to execute and nothing refused.
    pub fn is_empty(&self) -> bool { self.steps.is_empty() && self.skipped.is_empty() }
}

/// Phase buckets of one or more bundles, merged in finalize's order.
#[derive(Default)]
struct Phases {
    routes: Vec<(u8, usize)>,
    bodies: Vec<u32>,
    contexts: Vec<usize>,
    dispatchers: Vec<[usize; 2]>,
    records: Vec<[usize; 3]>,
    keys: Vec<u32>,
    skipped: Vec<Skipped>,
    attached: Attached,
    kirby_cells: usize,
    kirby_misplaced: usize,
    kirby_branch_mismatch: usize,
}

impl Phases {
    fn route(&mut self, slot: u8, controller: usize) {
        if !self.routes.iter().any(|(s, _)| *s == slot) { self.routes.push((slot, controller)); }
    }

    /// Every route slot naming a validated context's controller is cleared
    /// first (D-PTA1c), without allocating a separate controller list.
    fn routes_for(&mut self, owner: u32, contexts: &[ContextRef], consumers: &NativeConsumers) {
        let owns = |controller| unique_contexts(contexts)
            .any(|context| context.valid && context.controller == controller);
        for route in consumers.pad_routes.iter().flatten() {
            if route.controller != 0 && owns(route.controller) { self.route(route.slot, route.controller); }
        }
        // Nana's route (`owner+8`, U2): the same rule from the separately probed word.
        if let Some(controller) = consumers.nana_owner_plus_8 {
            let slot = owner.saturating_add(8);
            if controller != 0 && slot < PAD_SLOTS as u32 && owns(controller) { self.route(slot as u8, controller); }
        }
    }

    /// Kirby fields count only `valid` refs: an invalid ref's `kind` is the planned kind, not the
    /// body's own `+0xc` that `unprepare` reads (Yoga `m4b3` #140: the incoming window's Kirby
    /// paired with the outgoing Mario body and Mario's header word reported a false mismatch).
    fn kirby(&mut self, bodies: &[BodyRef]) {
        for body in bodies.iter().filter(|b| b.valid) {
            if body.kind == KIRBY { self.kirby_cells += 1; }
            if body.misplaced_kirby() { self.kirby_misplaced += 1; }
            if body.kirby_branch_mismatch() { self.kirby_branch_mismatch += 1; }
        }
    }

    fn detached_contexts(&mut self, contexts: &[ContextRef]) {
        for context in unique_contexts(contexts) {
            if !context.valid {
                self.skipped.push(Skipped::Context(context.ctx));
                continue;
            }
            self.contexts.push(context.ctx);
        }
    }

    fn attached(&mut self, group: &AttachedGroup, consumers: &NativeConsumers) {
        self.routes_for(group.owner, &group.contexts, consumers);
        self.kirby(&group.cells);
        self.attached = Attached::NativeRetiring;
    }

    fn detached(&mut self, group: &DetachedGroup, consumers: &NativeConsumers) {
        self.detached_contexts(&group.contexts);
        self.routes_for(group.owner, &group.contexts, consumers);
        self.kirby(&group.bodies);
        for body in &group.bodies {
            if body.valid { self.bodies.push(body.id); } else { self.skipped.push(Skipped::Body(body.id)); }
        }
        if let Some(dispatch) = group.dispatch {
            if dispatch.valid { self.dispatchers.push(dispatch.arrays); } else { self.skipped.push(Skipped::Dispatchers(dispatch.arrays)); }
        }
        if let Some(records) = group.records {
            if records.valid { self.records.push(records.records); } else { self.skipped.push(Skipped::Records(records.records)); }
        }
        for kind in &group.held_kinds {
            if !self.keys.contains(kind) { self.keys.push(*kind); }
        }
    }

    fn plan(self) -> DestroyPlan {
        let mut steps = Vec::new();
        steps.extend(self.routes.into_iter().map(|(slot, controller)| DestroyStep::ReleasePadRoute { slot, controller }));
        steps.extend(self.bodies.into_iter().map(|id| DestroyStep::DestroyBody { id }));
        steps.extend(self.contexts.into_iter().map(|ctx| DestroyStep::FreeContext { ctx }));
        steps.extend(self.dispatchers.into_iter().map(|arrays| DestroyStep::DestroyDispatchers { arrays }));
        steps.extend(self.records.into_iter().map(|records| DestroyStep::DestroyRecords { records }));
        if !self.keys.is_empty() { steps.push(DestroyStep::ReleaseKeys { kinds: self.keys }); }
        DestroyPlan { steps, skipped: self.skipped, attached: self.attached, kirby_cells: self.kirby_cells,
            kirby_misplaced: self.kirby_misplaced, kirby_branch_mismatch: self.kirby_branch_mismatch }
    }
}

/// Native context aliases use the first occurrence, including its validation
/// result. A later duplicate cannot upgrade an earlier refused context.
fn unique_contexts(contexts: &[ContextRef]) -> impl Iterator<Item = &ContextRef> {
    contexts.iter().enumerate().filter_map(|(index, context)| {
        (context.ctx != 0 && !contexts[..index].iter().any(|prior| prior.ctx == context.ctx))
            .then_some(context)
    })
}

/// The plan for one bundle: an attached window yields its route releases plus
/// [`Attached::NativeRetiring`]; a detached group yields finalize's order after
/// the route releases.
pub fn destroy_plan(bundle: &Bundle, consumers: &NativeConsumers) -> DestroyPlan {
    destroy_plan_many(std::slice::from_ref(bundle), consumers)
}

/// One plan for several bundles (every detached group of an entry at
/// `retire_groups`, with or without the attached window): routes of all of
/// them first, then all bodies, all contexts, all dispatcher arrays, all
/// records, one key release — `retire_groups`' order since GROUP-SWAP-IMPL.
pub fn destroy_plan_many(bundles: &[Bundle], consumers: &NativeConsumers) -> DestroyPlan {
    let mut phases = Phases::default();
    for bundle in bundles {
        match bundle {
            Bundle::Attached(group) => phases.attached(group, consumers),
            Bundle::Detached(group) => phases.detached(group, consumers),
        }
    }
    phases.plan()
}

/// Host model of the consumers after a plan ran: the route table with the
/// released slots nulled and the controllers and bodies native (or we) freed.
/// Used by the tests and by WP2's replay; `body_graph::execute` is the native
/// counterpart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConsumerModel {
    pub routes: [usize; PAD_SLOTS],
    pub freed_controllers: Vec<usize>,
    pub destroyed_bodies: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// A route slot still names a freed controller: the tick would read it
    /// (`Unmapped Read64 @ 0x0`, D-PTA1c).
    DanglingRoute { slot: u8 },
    /// A Kirby body outside cell 0 was destroyed: `unprepare`'s Kirby branch
    /// removes a node twice and spins (D-HANG1 S-B, D-KIRBY1).
    KirbyListWalk { body: u32 },
    /// A body registered under the other branch than its unload takes was
    /// destroyed (WP-R: init `0x607eb4` used the home header's `R+0x58[sub]`).
    KirbyBranchMismatch { body: u32 },
}

impl ConsumerModel {
    pub fn from_consumers(consumers: &NativeConsumers) -> Self {
        let mut routes = [0usize; PAD_SLOTS];
        for route in consumers.pad_routes.iter().flatten() { routes[usize::from(route.slot)] = route.controller; }
        Self { routes, ..Self::default() }
    }

    /// Apply the plan's own effects: route releases, destructions, and the
    /// controllers of every context the plan frees.
    pub fn apply(&mut self, plan: &DestroyPlan, bundles: &[Bundle]) {
        for step in &plan.steps {
            match step {
                DestroyStep::ReleasePadRoute { slot, controller } => {
                    if self.routes[usize::from(*slot)] == *controller { self.routes[usize::from(*slot)] = 0; }
                }
                DestroyStep::DestroyBody { id } => self.destroyed_bodies.push(*id),
                DestroyStep::FreeContext { ctx } => {
                    for bundle in bundles {
                        if let Bundle::Detached(group) = bundle {
                            for context in group.contexts.iter().filter(|c| c.ctx == *ctx && c.controller != 0) {
                                self.freed_controllers.push(context.controller);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Native retires the attached window: its contexts' controllers are freed
    /// and its cells destroyed (Reload states 1–3 / finalize).
    pub fn native_retire(&mut self, group: &AttachedGroup) {
        for context in &group.contexts {
            if context.controller != 0 { self.freed_controllers.push(context.controller); }
        }
        for cell in &group.cells { self.destroyed_bodies.push(cell.id); }
    }

    /// The pad tick: every enabled slot dereferences its controller.
    pub fn tick(&self) -> Result<(), Fault> {
        for (slot, controller) in self.routes.iter().enumerate() {
            if *controller != 0 && self.freed_controllers.contains(controller) { return Err(Fault::DanglingRoute { slot: slot as u8 }); }
        }
        Ok(())
    }

    /// The body unload's `unprepare` over every destroyed body of `bundles`: the
    /// Kirby branch's double removal (cell ≠ 0) or a registration on the other
    /// branch (WP-R) faults.
    pub fn kirby_walks(&self, bundles: &[Bundle]) -> Result<(), Fault> {
        for body in bundles.iter().flat_map(|b| b.bodies().iter()).filter(|b| b.valid) {
            if !self.destroyed_bodies.contains(&body.id) { continue; }
            if body.misplaced_kirby() { return Err(Fault::KirbyListWalk { body: body.id }); }
            if body.kirby_branch_mismatch() { return Err(Fault::KirbyBranchMismatch { body: body.id }); }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: u32 = 0;

    fn body(id: u32, kind: u32, cell: u8, activated: bool) -> BodyRef { BodyRef { id, kind, cell, activated, valid: true, registered: Some(kind) } }
    fn ctx(ctx: usize, controller: usize) -> ContextRef { ContextRef { ctx, controller, valid: true } }
    fn route(slot: u8, controller: usize) -> Option<RouteRef> { Some(RouteRef { slot, controller }) }
    /// A window's three cells with the first one live.
    fn cells(kinds: [u32; 3], ids: [u32; 3]) -> Vec<BodyRef> {
        (0..3).map(|i| body(ids[i], kinds[i], i as u8, i == 0)).collect()
    }

    /// D-PTA1BC-FIX §2.4 (dpta5): the CPU window's three contexts and controllers,
    /// the active one routed at slot 1.
    fn consumers_dpta5() -> NativeConsumers {
        let mut consumers = NativeConsumers::default();
        consumers.pad_routes[0] = route(0, 0x1180e93400);
        consumers.pad_routes[1] = route(1, 0x1180e94200);
        consumers
    }

    fn detached_dpta5() -> DetachedGroup {
        DetachedGroup {
            owner: 1,
            bodies: cells([0x21, 0x1e, 0x2f], [0x90000, 0xa0000, 0xb0000]),
            contexts: vec![ctx(0x1187044b00, 0x1180e91800), ctx(0x1187044b80, 0x1180e94200), ctx(0x1187044a00, 0x1180e95e00)],
            dispatch: Some(DispatcherRef { arrays: [0x11e22b0000, 0x11e22b0100], valid: true }),
            records: None,
            held_kinds: vec![],
        }
    }

    fn position(plan: &DestroyPlan, f: impl Fn(&DestroyStep) -> bool) -> Option<usize> { plan.steps.iter().position(f) }

    #[test]
    fn destroy_plan_releases_the_pad_route_before_freeing_a_routed_context() {
        let plan = destroy_plan(&Bundle::Detached(detached_dpta5()), &consumers_dpta5());
        let route = position(&plan, |s| *s == DestroyStep::ReleasePadRoute { slot: 1, controller: 0x1180e94200 }).expect("route release present");
        let first_body = position(&plan, |s| matches!(s, DestroyStep::DestroyBody { .. })).unwrap();
        let first_free = position(&plan, |s| matches!(s, DestroyStep::FreeContext { .. })).unwrap();
        let dispatch = position(&plan, |s| matches!(s, DestroyStep::DestroyDispatchers { .. })).unwrap();
        assert!(route < first_body && first_body < first_free && first_free < dispatch, "{:?}", plan);
        assert_eq!(plan.routes_cleared(), 1, "slot 0 names another entry's controller and stays");
        assert_eq!((plan.bodies(), plan.contexts(), plan.dispatchers()), (3, 3, 1));
        assert_eq!(plan.attached, Attached::None);
        assert!(plan.skipped.is_empty());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced), (0, 0));
    }

    #[test]
    fn nana_route_at_owner_plus_8_is_cleared_with_the_pair_window() {
        // A pair-lead window parked as a detached group: Nana's context is bound to
        // route `owner+8` (6b6f10, U2), Popo's to `owner`.
        let mut consumers = NativeConsumers::default();
        consumers.pad_routes[0] = route(0, 0xa000);
        consumers.pad_routes[8] = route(8, 0xb000);
        let group = DetachedGroup { owner: OWNER, bodies: cells([0x4b, 0x4c, 0x0a], [1, 2, 3]),
            contexts: vec![ctx(0x100, 0xa000), ctx(0x200, 0xb000), ctx(0x300, 0xc000)], ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(group.clone()), &consumers);
        assert_eq!(&plan.steps[..2], &[DestroyStep::ReleasePadRoute { slot: 0, controller: 0xa000 },
            DestroyStep::ReleasePadRoute { slot: 8, controller: 0xb000 }]);
        // The separately probed `owner+8` word yields the same single step (no duplicate).
        let mut probed = NativeConsumers { nana_owner_plus_8: Some(0xb000), ..Default::default() };
        probed.pad_routes[8] = route(8, 0xb000);
        let plan = destroy_plan(&Bundle::Detached(group), &probed);
        assert_eq!(plan.routes_cleared(), 1);
        assert_eq!(plan.steps[0], DestroyStep::ReleasePadRoute { slot: 8, controller: 0xb000 });
    }

    #[test]
    fn aliased_window_contexts_are_freed_once_and_routed_once() {
        // `+0x4178` aliases `+0x4188` (the selected context) and `+0x4180` is 0.
        let mut consumers = NativeConsumers::default();
        consumers.pad_routes[1] = route(1, 0x5000);
        let group = DetachedGroup { owner: 1, bodies: vec![body(7, 0x07, 0, true)],
            contexts: vec![ctx(0x700, 0x5000), ctx(0, 0), ctx(0x700, 0x5000), ctx(0x800, 0x6000)], ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(group), &consumers);
        assert_eq!(plan.routes_cleared(), 1);
        assert_eq!(plan.contexts(), 2, "{plan:?}");
        assert_eq!(plan.steps.iter().filter(|s| **s == DestroyStep::FreeContext { ctx: 0x700 }).count(), 1);
    }

    #[test]
    fn attached_window_plan_clears_routes_and_lets_native_retire() {
        let mut consumers = NativeConsumers::default();
        consumers.pad_routes[0] = route(0, 0x1180e93400);
        let window = AttachedGroup { owner: OWNER, cells: cells([0x2d, 0x2c, 0x00], [0x50000, 0x60000, 0x70000]),
            contexts: vec![ctx(0x900, 0x1180e93400), ctx(0, 0), ctx(0x900, 0x1180e93400), ctx(0x910, 0x1180e93500), ctx(0x920, 0x1180e93600)] };
        let plan = destroy_plan(&Bundle::Attached(window), &consumers);
        assert_eq!(plan.steps, vec![DestroyStep::ReleasePadRoute { slot: 0, controller: 0x1180e93400 }]);
        assert_eq!(plan.attached, Attached::NativeRetiring);
        assert_eq!((plan.bodies(), plan.contexts(), plan.dispatchers(), plan.records()), (0, 0, 0, 0));
    }

    /// B123-pre Yoga l123a (B123PRE-LOBBY.md): match-start finalize with the
    /// R.O.B. Phase-B window `[2d, 2c, 0]` attached; 64 `Unmapped Read64 @ 0x0`
    /// followed the `teardown … reason=finalize` line. Old policy: no route
    /// clear for the attached window. New: the attached arm runs at finalize.
    #[test]
    fn finalize_with_an_attached_group_leaves_no_route_dangling() {
        let mut consumers = NativeConsumers::default();
        consumers.pad_routes[0] = route(0, 0x1180e93400);
        consumers.pad_routes[9] = route(9, 0x1180e97000); // the CPU entry's slot, untouched
        let window = AttachedGroup { owner: OWNER, cells: cells([0x2d, 0x2c, 0x00], [0x50000, 0x60000, 0x70000]),
            contexts: vec![ctx(0x900, 0x1180e93400), ctx(0x910, 0x1180e93500), ctx(0x920, 0x1180e93600)] };
        // Old policy: finalize destroys the window; the route still names its controller.
        let mut old = ConsumerModel::from_consumers(&consumers);
        old.native_retire(&window);
        assert_eq!(old.tick(), Err(Fault::DanglingRoute { slot: 0 }), "l123a L17286–L17350");
        // New policy: the plan's route release runs before native retires the window.
        let bundles = [Bundle::Attached(window.clone())];
        let plan = destroy_plan_many(&bundles, &consumers);
        let mut new = ConsumerModel::from_consumers(&consumers);
        new.apply(&plan, &bundles);
        new.native_retire(&window);
        assert_eq!(new.tick(), Ok(()));
        assert_eq!(new.routes[9], 0x1180e97000, "another entry's route is never touched");
    }

    /// D-PTA1 (dpta1/dpta5): the mod Reload of a CPU window with a detached
    /// group; the window's active context is the routed one. Both the detached
    /// group and the attached window go through one plan before `reload_at_pose`.
    #[test]
    fn mod_reload_clears_the_window_route_and_destroys_the_detached_groups_first() {
        let consumers = consumers_dpta5();
        let window = AttachedGroup { owner: 1, cells: cells([0x2f, 0x21, 0x1e], [0x1, 0x2, 0x3]),
            contexts: vec![ctx(0x1187044b80, 0x1180e94200), ctx(0, 0), ctx(0x1187044b00, 0x1180e91800), ctx(0x1187044b80, 0x1180e94200), ctx(0x1187044a00, 0x1180e95e00)] };
        let mut detached = detached_dpta5();
        detached.contexts = vec![ctx(0x1187045900, 0x1180e98000), ctx(0x1187045700, 0x1180e98100), ctx(0x1187045800, 0x1180e98200)];
        let bundles = [Bundle::Attached(window.clone()), Bundle::Detached(detached)];
        let plan = destroy_plan_many(&bundles, &consumers);
        assert_eq!(plan.steps[0], DestroyStep::ReleasePadRoute { slot: 1, controller: 0x1180e94200 });
        assert_eq!(plan.attached, Attached::NativeRetiring);
        assert_eq!((plan.bodies(), plan.contexts(), plan.dispatchers()), (3, 3, 1));
        let mut model = ConsumerModel::from_consumers(&consumers);
        model.apply(&plan, &bundles);
        model.native_retire(&window);
        assert_eq!(model.tick(), Ok(()));
    }

    #[test]
    fn invalid_objects_are_skipped_not_destroyed() {
        let mut consumers = consumers_dpta5();
        consumers.pad_routes[2] = route(2, 0x1180e95e00);
        let group = DetachedGroup { owner: 1,
            bodies: vec![body(0x90000, 0x21, 0, true), BodyRef { id: 0xa0000, kind: 0x1e, cell: 1, activated: false, valid: false, registered: Some(0x1e) }],
            contexts: vec![ctx(0x1187044b80, 0x1180e94200), ContextRef { ctx: 0x1187044a00, controller: 0, valid: false }],
            dispatch: Some(DispatcherRef { arrays: [0x11e22b0000, 0x11e22b0100], valid: false }),
            records: Some(RecordsRef { records: [0x11f0000000, 0, 0x11f0000300], valid: false }),
            held_kinds: vec![0x2f] };
        let plan = destroy_plan(&Bundle::Detached(group), &consumers);
        assert_eq!(plan.skipped, vec![Skipped::Context(0x1187044a00), Skipped::Body(0xa0000),
            Skipped::Dispatchers([0x11e22b0000, 0x11e22b0100]), Skipped::Records([0x11f0000000, 0, 0x11f0000300])]);
        assert_eq!(plan.bodies(), 1);
        assert_eq!(plan.contexts(), 1);
        assert_eq!(plan.routes_cleared(), 1, "an unvalidated context has no known controller: its route is left to native");
        assert_eq!(plan.keys(), &[0x2f]);
    }

    /// The `perform_swap` rollback of a partially built native group: what exists
    /// is destroyed in finalize's order and the unbuilt members' keys (plus the
    /// object kind's) are released last.
    #[test]
    fn rollback_of_a_partial_build_releases_unbuilt_keys_after_destroying_what_exists() {
        let group = DetachedGroup { owner: OWNER,
            bodies: vec![body(0x1, 0x24, 0, false)], contexts: vec![ctx(0x100, 0x1000)],
            dispatch: Some(DispatcherRef { arrays: [0x2000, 0x2100], valid: true }),
            records: Some(RecordsRef { records: [0x3000, 0, 0x3300], valid: true }),
            held_kinds: vec![0x25, 0x26, 0x72] };
        let plan = destroy_plan(&Bundle::Detached(group), &NativeConsumers::default());
        assert_eq!(plan.steps, vec![
            DestroyStep::DestroyBody { id: 0x1 },
            DestroyStep::FreeContext { ctx: 0x100 },
            DestroyStep::DestroyDispatchers { arrays: [0x2000, 0x2100] },
            DestroyStep::DestroyRecords { records: [0x3000, 0, 0x3300] },
            DestroyStep::ReleaseKeys { kinds: vec![0x25, 0x26, 0x72] },
        ]);
    }

    #[test]
    fn plans_of_several_groups_merge_in_finalizes_order_with_one_key_release() {
        let a = DetachedGroup { owner: OWNER, bodies: vec![body(1, 0x07, 0, true), body(2, 0x08, 1, false)],
            contexts: vec![ctx(0x100, 0x1000), ctx(0x200, 0x2000)],
            dispatch: Some(DispatcherRef { arrays: [0x900, 0x910], valid: true }), records: None, held_kinds: vec![] };
        let b = DetachedGroup { owner: OWNER, bodies: vec![], contexts: vec![], dispatch: None, records: None, held_kinds: vec![0x5a, 0x5b, 0x75] };
        let c = DetachedGroup { owner: OWNER, bodies: vec![body(3, 0x24, 0, false)], contexts: vec![ctx(0x300, 0x3000)],
            dispatch: Some(DispatcherRef { arrays: [0, 0], valid: true }),
            records: Some(RecordsRef { records: [0x4000, 0, 0x4300], valid: true }), held_kinds: vec![0x5a] };
        let plan = destroy_plan_many(&[Bundle::Detached(a), Bundle::Detached(b), Bundle::Detached(c)], &NativeConsumers::default());
        let kinds: Vec<&str> = plan.steps.iter().map(|s| match s {
            DestroyStep::ReleasePadRoute { .. } => "route", DestroyStep::DestroyBody { .. } => "body",
            DestroyStep::FreeContext { .. } => "ctx", DestroyStep::DestroyDispatchers { .. } => "dispatch",
            DestroyStep::DestroyRecords { .. } => "records", DestroyStep::ReleaseKeys { .. } => "keys" }).collect();
        assert_eq!(kinds, ["body", "body", "body", "ctx", "ctx", "ctx", "dispatch", "dispatch", "records", "keys"]);
        assert_eq!(plan.keys(), &[0x5a, 0x5b, 0x75], "a kind held by two groups is released once");
    }

    /// WP-K (RE-KIRBY-UNLOAD §0/§3): a Kirby body outside cell 0 is the hazard,
    /// activated or not; a Kirby at cell 0 is not. The plan reports, never repairs.
    #[test]
    fn a_kirby_outside_cell_0_is_reported_as_the_d_hang1_hazard() {
        // S-B: `[5, 6, 7]`, Kirby parked at cell 1, never activated (3/3 hung).
        let s_b = AttachedGroup { owner: OWNER, cells: cells([0x05, KIRBY, 0x07], [0x50, 0x60, 0x70]), contexts: vec![] };
        let plan = destroy_plan(&Bundle::Attached(s_b), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced), (1, 1));
        assert_eq!(plan.attached, Attached::NativeRetiring);
        // K6: a group-built Kirby at cell 1 destroyed by a group teardown.
        let k6 = DetachedGroup { owner: OWNER, bodies: cells([0x05, KIRBY, 0x07], [0x10, 0x20, 0x30]),
            contexts: vec![ctx(0x100, 0x1000), ctx(0x200, 0x2000), ctx(0x300, 0x3000)], ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(k6), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced), (1, 1));
        assert_eq!(plan.bodies(), 3, "nothing is withheld: the hazard is reported, the planner prevents it");
        // The deciding row (RE §0): an ACTIVATED hatless Kirby at cell 1 is the same hazard.
        let mut warm = cells([0x05, KIRBY, 0x07], [0x50, 0x60, 0x70]);
        warm[0].activated = true; warm[1].activated = true;
        let plan = destroy_plan(&Bundle::Attached(AttachedGroup { owner: OWNER, cells: warm, contexts: vec![] }), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced), (1, 1));
        // S-K: Kirby at cell 0 (`[6, 7, 8]`) is counted but not misplaced (completed 1/1).
        let s_k = AttachedGroup { owner: OWNER, cells: cells([KIRBY, 0x07, 0x08], [0x60, 0x70, 0x80]), contexts: vec![] };
        let plan = destroy_plan(&Bundle::Attached(s_k), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced), (1, 0));
        // S-A: no Kirby anywhere.
        let s_a = AttachedGroup { owner: OWNER, cells: cells([0x07, 0x08, 0x09], [0x70, 0x80, 0x90]), contexts: vec![] };
        assert_eq!(destroy_plan(&Bundle::Attached(s_a), &NativeConsumers::default()).kirby_cells, 0);
    }

    /// The host walk model: S-B and K6 spin, S-K and S-A do not (D-HANG1-SEPARATION §1).
    #[test]
    fn s_b_and_k6_walk_the_registry_and_s_k_s_a_do_not() {
        let s_b = Bundle::Attached(AttachedGroup { owner: OWNER, cells: cells([0x05, KIRBY, 0x07], [0x50, 0x60, 0x70]), contexts: vec![] });
        let k6 = Bundle::Detached(DetachedGroup { owner: OWNER, bodies: cells([0x05, KIRBY, 0x07], [0x10, 0x20, 0x30]), ..Default::default() });
        let s_k = Bundle::Attached(AttachedGroup { owner: OWNER, cells: cells([KIRBY, 0x07, 0x08], [0x60, 0x70, 0x80]), contexts: vec![] });
        let s_a = Bundle::Attached(AttachedGroup { owner: OWNER, cells: cells([0x07, 0x08, 0x09], [0x70, 0x80, 0x90]), contexts: vec![] });
        for (bundle, expected) in [(s_b, Err(Fault::KirbyListWalk { body: 0x60 })), (k6, Err(Fault::KirbyListWalk { body: 0x20 })), (s_k, Ok(())), (s_a, Ok(()))] {
            let bundles = [bundle];
            let plan = destroy_plan_many(&bundles, &NativeConsumers::default());
            let mut model = ConsumerModel::default();
            model.apply(&plan, &bundles);
            if let Bundle::Attached(window) = &bundles[0] { model.native_retire(window); }
            assert_eq!(model.kirby_walks(&bundles), expected, "{bundles:?}");
        }
    }

    /// WP-R's second route (INTEGRATION "per-body identity view"): init `0x607eb4` registers a
    /// body under the home header's `R+0x58[sub]` while `unprepare` reads the body's own kind, and a
    /// Phase-B group keeps the home header. K14: a non-Kirby group body under a PinCell0 Kirby home;
    /// K15: a Kirby group body under a non-Kirby home. Both are reported and fault the walk model;
    /// with `identity_view` (registered == kind) neither does.
    #[test]
    fn a_group_body_registered_under_the_other_kirby_branch_is_reported() {
        // K14: home window [Kirby, Yoshi, Fox] (R+0x58 = [6, 5, 7]); a Phase-B group [Chrom, Lucina, Marth]
        // built under it: cell 0 registered as Kirby while the body is Chrom.
        let k14 = DetachedGroup { owner: OWNER, bodies: vec![
            BodyRef { id: 1, kind: 0x1b, cell: 0, activated: false, valid: true, registered: Some(KIRBY) },
            BodyRef { id: 2, kind: 0x1c, cell: 1, activated: false, valid: true, registered: Some(0x05) },
            BodyRef { id: 3, kind: 0x1d, cell: 2, activated: false, valid: true, registered: Some(0x07) }],
            ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(k14.clone()), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch), (0, 0, 1));
        let bundles = [Bundle::Detached(k14)];
        let mut model = ConsumerModel::default();
        model.apply(&plan, &bundles);
        assert_eq!(model.kirby_walks(&bundles), Err(Fault::KirbyBranchMismatch { body: 1 }));
        // K15: home [Yoshi, Fox, Falco] (R+0x58 = [5, 7, 0x14]); a group [Kirby, Chrom] under it: Kirby at
        // cell 0 (PinCell0) registered as Yoshi.
        let k15 = DetachedGroup { owner: OWNER, bodies: vec![
            BodyRef { id: 4, kind: KIRBY, cell: 0, activated: true, valid: true, registered: Some(0x05) },
            BodyRef { id: 5, kind: 0x1b, cell: 1, activated: false, valid: true, registered: Some(0x07) }],
            ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(k15.clone()), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch), (1, 0, 1));
        let bundles = [Bundle::Detached(k15)];
        let mut model = ConsumerModel::default();
        model.apply(&plan, &bundles);
        assert_eq!(model.kirby_walks(&bundles), Err(Fault::KirbyBranchMismatch { body: 4 }));
        // With identity_view the init registers the body's own kind: nothing reported, no fault.
        let fixed = DetachedGroup { owner: OWNER, bodies: vec![
            BodyRef { id: 1, kind: 0x1b, cell: 0, activated: false, valid: true, registered: Some(0x1b) },
            BodyRef { id: 4, kind: KIRBY, cell: 0, activated: true, valid: true, registered: Some(KIRBY) }],
            ..Default::default() };
        let plan = destroy_plan(&Bundle::Detached(fixed.clone()), &NativeConsumers::default());
        assert_eq!((plan.kirby_misplaced, plan.kirby_branch_mismatch), (0, 0));
        let bundles = [Bundle::Detached(fixed)];
        let mut model = ConsumerModel::default();
        model.apply(&plan, &bundles);
        assert_eq!(model.kirby_walks(&bundles), Ok(()));
        // A mismatch between two non-Kirby kinds takes the generic branch both ways: not this hazard.
        let generic = BodyRef { id: 9, kind: 0x1b, cell: 1, activated: false, valid: true, registered: Some(0x05) };
        assert!(!generic.kirby_branch_mismatch());
        // Unknown registration (no R read) reports nothing rather than guessing.
        let unknown = BodyRef { id: 10, kind: KIRBY, cell: 0, activated: true, valid: true, registered: None };
        assert!(!unknown.kirby_branch_mismatch());
    }

    /// Yoga `m4b3` #140: a mod Reload into `[6, 5, 7]` planned the retiring window with the incoming
    /// kinds while cell 0 still held the outgoing Mario body (`valid=false`, `body+0xc` = 0) under
    /// Mario's header word. Such a ref names no Kirby body: no Kirby count, no walk fault. With the
    /// adapter's kind read from `body+0xc` the same cell is a valid Mario ref and also reports nothing.
    #[test]
    fn an_invalid_ref_is_not_a_kirby_hazard() {
        let planned = AttachedGroup { owner: OWNER, cells: vec![
            BodyRef { id: 1, kind: KIRBY, cell: 0, activated: true, valid: false, registered: Some(0x00) }],
            contexts: vec![] };
        let plan = destroy_plan(&Bundle::Attached(planned.clone()), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch), (0, 0, 0));
        let bundles = [Bundle::Attached(planned)];
        let mut model = ConsumerModel::default();
        model.apply(&plan, &bundles);
        if let Bundle::Attached(window) = &bundles[0] { model.native_retire(window); }
        assert_eq!(model.kirby_walks(&bundles), Ok(()));
        let read = AttachedGroup { owner: OWNER, cells: vec![
            BodyRef { id: 1, kind: 0x00, cell: 0, activated: true, valid: true, registered: Some(0x00) }],
            contexts: vec![] };
        let plan = destroy_plan(&Bundle::Attached(read), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_misplaced, plan.kirby_branch_mismatch), (0, 0, 0));
        // A valid Kirby body registered under another header word is still reported.
        let real = AttachedGroup { owner: OWNER, cells: vec![
            BodyRef { id: 2, kind: KIRBY, cell: 0, activated: true, valid: true, registered: Some(0x00) }],
            contexts: vec![] };
        let plan = destroy_plan(&Bundle::Attached(real), &NativeConsumers::default());
        assert_eq!((plan.kirby_cells, plan.kirby_branch_mismatch), (1, 1));
    }

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
        fn below(&mut self, n: u64) -> u64 { self.next() % n }
    }

    /// Property: after the plan ran, no route names a controller the plan freed
    /// or native will free, a foreign route survives, invalid bodies are never
    /// destroyed, and the Kirby report equals the misplaced Kirby bodies, whose
    /// destruction is exactly what the walk model faults on.
    #[test]
    fn no_consumer_references_a_destroyed_object_after_the_plan() {
        let kinds = [0x00u32, 0x05, KIRBY, 0x07, 0x1e, 0x21, 0x2f, 0x4b, 0x4c];
        for seed in 1..400u64 {
            let mut rng = Lcg(seed);
            let mut consumers = NativeConsumers::default();
            let mut bundles = Vec::new();
            let mut next_ctrl = 0x1000usize;
            let mut next_id = 1u32;
            let mut mk_contexts = |rng: &mut Lcg, consumers: &mut NativeConsumers, owner: u32| -> Vec<ContextRef> {
                let n = 1 + rng.below(3) as usize;
                let mut contexts = Vec::new();
                for i in 0..n {
                    next_ctrl += 0x100;
                    let valid = rng.below(8) != 0;
                    let context = ContextRef { ctx: 0x8000 + next_ctrl, controller: if valid { next_ctrl } else { 0 }, valid };
                    contexts.push(context);
                    if i == 0 { contexts.push(context); } // the `+0x4178` alias of the selected context
                    // Routes: this owner's slot, Nana's `owner+8`, or a random other slot naming it.
                    let slot = match rng.below(4) { 0 => owner, 1 => owner + 8, 2 => rng.below(16) as u32, _ => continue };
                    if valid && consumers.pad_routes[slot as usize].is_none() { consumers.pad_routes[slot as usize] = route(slot as u8, next_ctrl); }
                }
                contexts
            };
            let owner = rng.below(2) as u32;
            let mut mk_bodies = |rng: &mut Lcg| -> Vec<BodyRef> {
                (0..1 + rng.below(3)).map(|cell| {
                    let kind = kinds[rng.below(kinds.len() as u64) as usize];
                    let id = next_id; next_id += 1;
                    // Mostly registered under its own kind; one in twelve under another home header kind (WP-R).
                    let registered = if rng.below(12) == 0 { Some(kinds[rng.below(kinds.len() as u64) as usize]) } else { Some(kind) };
                    BodyRef { id, kind, cell: cell as u8, activated: rng.below(2) == 0, valid: rng.below(10) != 0, registered }
                }).collect()
            };
            if rng.below(2) == 0 {
                let contexts = mk_contexts(&mut rng, &mut consumers, owner);
                bundles.push(Bundle::Attached(AttachedGroup { owner, cells: mk_bodies(&mut rng), contexts }));
            }
            for _ in 0..rng.below(3) {
                let contexts = mk_contexts(&mut rng, &mut consumers, owner);
                bundles.push(Bundle::Detached(DetachedGroup { owner, bodies: mk_bodies(&mut rng), contexts,
                    dispatch: Some(DispatcherRef { arrays: [0x900, 0x910], valid: true }), records: None,
                    held_kinds: if rng.below(2) == 0 { vec![0x75] } else { vec![] } }));
            }
            // A foreign route (another entry's live controller) must survive every plan.
            consumers.pad_routes[15] = route(15, 0xdead0);
            let plan = destroy_plan_many(&bundles, &consumers);
            let mut model = ConsumerModel::from_consumers(&consumers);
            model.apply(&plan, &bundles);
            for bundle in &bundles {
                if let Bundle::Attached(window) = bundle { model.native_retire(window); }
            }
            assert_eq!(model.tick(), Ok(()), "seed {seed}: {plan:?}");
            assert_eq!(model.routes[15], 0xdead0, "seed {seed}: foreign route cleared");
            // Invalid detached bodies are never destroyed by us.
            for bundle in &bundles {
                if let Bundle::Detached(group) = bundle {
                    for body in group.bodies.iter().filter(|b| !b.valid) {
                        assert!(!plan.steps.contains(&DestroyStep::DestroyBody { id: body.id }), "seed {}", seed);
                    }
                }
            }
            // The Kirby report equals the bundles' Kirby bodies, and the walk model faults exactly
            // when a misplaced Kirby body was destroyed (by us or by native).
            let all: Vec<BodyRef> = bundles.iter().flat_map(|b| b.bodies().iter().copied()).filter(|b| b.valid).collect();
            assert_eq!(plan.kirby_cells, all.iter().filter(|b| b.kind == KIRBY).count(), "seed {seed}");
            assert_eq!(plan.kirby_misplaced, all.iter().filter(|b| b.misplaced_kirby()).count(), "seed {seed}");
            assert_eq!(plan.kirby_branch_mismatch, all.iter().filter(|b| b.kirby_branch_mismatch()).count(), "seed {seed}");
            let destroyed_hazard = all.iter().any(|b| (b.misplaced_kirby() || b.kirby_branch_mismatch()) && model.destroyed_bodies.contains(&b.id));
            assert_eq!(model.kirby_walks(&bundles).is_err(), destroyed_hazard, "seed {seed}");
        }
    }
}
