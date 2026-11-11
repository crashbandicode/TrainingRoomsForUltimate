//! Pure policy for the helper-rendered Mii HUD portrait. Numbers in, decisions out.
//! The adapter (`mii_hud_portrait.rs`) reads every word fresh in the thread-80 service
//! callback, consults this module and acts at once. No native reads or calls live here.
//! Design: artifacts/enhancements/mii-hud54-design/DESIGN.md (A1/A2) and ROOT-FLAG-RESPONSE.md.

/// Candidate request subs on a helper row. Nothing here claims native code never uses
/// them: first use requires `RequestWords::idle_unregistered`, and an owned request that
/// stops reading live is disowned.
pub const PORTRAIT_SUBS: [u32; 3] = [7, 8, 9];
pub const VISIBLE_OWNERS: usize = 4;
/// Four exclusive renderer rows times three retained requests.
pub const OWNED_CAPACITY: usize = 12;

/// The ordinary P1/CPU exact-portrait channels in the shared source registry.
/// Configuration alone cannot identify a portrait: Kirby Mii hats carry it too.
pub const SOURCE_CHANNELS: [usize; 2] = [3, 4];
pub fn source_channel_admitted(channel: usize, exact: bool, menu_open: bool) -> bool {
    !menu_open || (SOURCE_CHANNELS.contains(&channel) && exact)
}

/// Count active helper acquisition attempts, not time deliberately spent behind
/// a native UI/restore/playback gate. No helper is leased in this phase. Once a
/// helper is leased or rendering, its original bounded lifetime still applies.
pub fn helper_wait_tick(waits: u16, admission_blocked: bool) -> (u16, bool) {
    if admission_blocked { return (waits, false); }
    let next = waits.saturating_add(1);
    (next, next >= 1200)
}

#[cfg(test)]
#[test]
fn open_editor_admits_exact_portraits_without_admitting_mii_hats_or_items() {
    let requests = [(0, true), (1, true), (2, false), (3, true), (4, true)];
    let admitted: Vec<_> = requests.iter().copied().filter(|&(c, exact)|
        source_channel_admitted(c, exact, true)).collect();
    assert_eq!(admitted, [(3, true), (4, true)]);
    assert!(!source_channel_admitted(3, false, true));
    assert!(!source_channel_admitted(4, false, true));
    assert!(!source_channel_admitted(5, true, true));
    assert!(!source_channel_admitted(6, true, true));
    assert!(!source_channel_admitted(7, true, true));
    for (c, exact) in requests { assert!(source_channel_admitted(c, exact, false)); }
}

#[cfg(test)]
#[test]
fn portrait_acquisition_budget_pauses_during_owned_playback_and_remains_bounded_after() {
    let mut waits = 1199;
    for _ in 0..3600 {
        let (next, expired) = helper_wait_tick(waits, true);
        assert_eq!(next, 1199);
        assert!(!expired);
        waits = next;
    }
    assert_eq!(helper_wait_tick(waits, false), (1200, true));
    assert_eq!(helper_wait_tick(u16::MAX, false), (u16::MAX, true));
}

/// A square face/shoulder window in the native 1024px Mii render. The small waiting
/// icon has no texture-SRT storage; crop its existing quad rather than allocating
/// a new material. Bilinear interpolation preserves a pane's native UV orientation.
pub fn waiting_portrait_uv(original: [f32; 8]) -> Option<[f32; 8]> {
    if original.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) {
        return None;
    }
    let mut result = [0.0; 8];
    for (i, &(u, v)) in [(0.32, 0.22), (0.68, 0.22), (0.32, 0.58), (0.68, 0.58)].iter().enumerate() {
        for axis in 0..2 {
            let top = original[axis] * (1.0 - u) + original[2 + axis] * u;
            let bottom = original[4 + axis] * (1.0 - u) + original[6 + axis] * u;
            result[i * 2 + axis] = top * (1.0 - v) + bottom * v;
        }
    }
    Some(result)
}

#[cfg(test)]
#[test]
fn waiting_crop_preserves_orientation_and_rejects_invalid_native_coordinates() {
    let ordinary = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let flipped = [1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let a = waiting_portrait_uv(ordinary).unwrap();
    let b = waiting_portrait_uv(flipped).unwrap();
    for i in 0..4 {
        assert!((a[2*i] + b[2*i] - 1.0).abs() < 1e-6);
        assert_eq!(a[2*i+1], b[2*i+1]);
    }
    assert!((a[2] - a[0] - (a[5] - a[1])).abs() < 1e-6);
    assert!(waiting_portrait_uv([f32::NAN;8]).is_none());
    assert!(waiting_portrait_uv([f32::INFINITY;8]).is_none());
    assert!(waiting_portrait_uv([-1.0;8]).is_none());
}

/// Run93's unused ordinary-origin renderer. Only this idle state may start the
/// native portrait-only setup; an existing or busy renderer remains native-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineStart {
    pub queue: Queue,
    pub ready: u8,
    pub portrait_flag: u8,
    pub scene_flag: u8,
    pub batch_present: bool,
    pub scene_object_present: bool,
}

impl PipelineStart {
    pub fn allowed(self) -> bool {
        self.queue == (Queue { count: 0, word: 0 })
            && self.ready == 0 && self.portrait_flag == 0 && self.scene_flag == 1
            && !self.batch_present && !self.scene_object_present
    }

    /// Only an owned, drained portrait-only pipeline may return to command 0.
    /// The adapter separately checks the source/batch/entry/thread identities and
    /// releases its visible pane before the native batch destructor can run.
    pub fn retirable(self, stack_base: u32, restart: u8, in_flight: bool) -> bool {
        self.queue == (Queue { count: 0, word: 3 }) && stack_base == 3
            && restart == 0 && !in_flight && self.ready == 1
            && self.portrait_flag == 1 && self.scene_flag == 0
            && self.batch_present && !self.scene_object_present
    }

    /// cmd0 frees native holders and resets flags but leaves ready latched at 1.
    /// Restore the originally borrowed ready=0 only after this readback, never
    /// on an existing/native-owned source or on a newly entered command3.
    pub fn reset_completed(self, stack_base: u32, flags: [u8; 5]) -> bool {
        self.queue == (Queue { count: 0, word: 0 }) && stack_base == 0
            && self.ready == 1 && self.portrait_flag == 0 && self.scene_flag == 1
            && !self.batch_present && !self.scene_object_present
            && flags == [1, 0, 0, 1, 1]
    }
}

#[cfg(test)]
#[test]
fn owned_pipeline_retirement_requires_idle_and_verified_native_reset() {
    let owned = PipelineStart { queue: Queue { count: 0, word: 3 }, ready: 1,
        portrait_flag: 1, scene_flag: 0, batch_present: true, scene_object_present: false };
    assert!(owned.retirable(3, 0, false));
    assert!(!owned.retirable(3, 0, true));
    assert!(!owned.retirable(3, 1, false));
    assert!(!owned.retirable(0, 0, false));
    for queue in [Queue { count: 1, word: 3 }, Queue { count: 0, word: 0xa }, Queue { count: 0, word: 0xb }] {
        assert!(!PipelineStart { queue, ..owned }.retirable(3, 0, false));
    }
    assert!(!PipelineStart { scene_object_present: true, ..owned }.retirable(3, 0, false));
    let retired = PipelineStart { queue: Queue { count: 0, word: 0 }, ready: 1,
        portrait_flag: 0, scene_flag: 1, batch_present: false, scene_object_present: false };
    assert!(retired.reset_completed(0, [1,0,0,1,1]));
    assert!(!owned.reset_completed(0, [1,0,0,1,1]));
    assert!(!PipelineStart { batch_present: true, ..retired }.reset_completed(0, [1,0,0,1,1]));
    assert!(!PipelineStart { scene_object_present: true, ..retired }.reset_completed(0, [1,0,0,1,1]));
    assert!(!PipelineStart { queue: Queue { count: 0, word: 3 }, ..retired }.reset_completed(0, [1,0,0,1,1]));
    assert!(!retired.reset_completed(0, [1,1,0,1,1]));
}

#[cfg(test)]
#[test]
fn unused_pipeline_admission_preserves_busy_and_existing_renderers() {
    let ordinary = PipelineStart { queue: Queue { count: 0, word: 0 }, ready: 0,
        portrait_flag: 0, scene_flag: 1, batch_present: false, scene_object_present: false };
    assert!(ordinary.allowed());
    assert!(!PipelineStart { queue: Queue { count: 1, word: 1 }, ..ordinary }.allowed());
    assert!(!PipelineStart { queue: Queue { count: 0, word: 3 }, ready: 1,
        portrait_flag: 1, scene_flag: 0, batch_present: true, ..ordinary }.allowed());
    assert!(!PipelineStart { scene_object_present: true, ..ordinary }.allowed());
    assert!(!PipelineStart { batch_present: true, ..ordinary }.allowed());
    assert!(!PipelineStart { portrait_flag: 1, ..ordinary }.allowed());
}

/// The retarget (`3304a74`) registers every request of a row at `table[row+0x12]`: the table
/// entry is per row, not per request. The native applier re-binds a pane from that entry.
pub fn table_index(row: u32) -> u32 { row + 0x12 }

/// One bust row per owner. Both visible owners' renders on the same helper row would register
/// into the same `table[0x12+row]`, and the native applier (`1418780/1418c80 -> 1b4ee40`)
/// re-binds a pane from `table[slot+0x1ac]` whenever it re-applies, so the second owner's
/// registration would replace the first owner's bust on the next re-apply. A row is free for
/// `owner` while no other owner's pane is bound to a request of that row.
pub fn row_free_for(bound: &[Option<Binding>; VISIBLE_OWNERS], owner: usize, row: u32) -> bool {
    bound.iter().enumerate().all(|(o, b)| o == owner || b.is_none_or(|b| b.request.row != row))
}

/// Native construction/render preset (`13438e4`). Storage subs7/8/9 are distinct from
/// this camera/draw preset; they must stay stable while their textures are bound.
pub fn class_sub(kind: u32) -> Option<u32> {
    match kind { 0x48 => Some(4), 0x4a => Some(5), 0x49 => Some(6), _ => None }
}

/// The renderer reads request.sub as both a storage identity and a presentation preset.
/// Only the owned render's register copies may use the class preset. Never rewrite the
/// stored request key: completion, template protection and texture lifetime use that key.
pub fn render_preset(request: Request, observed_sub: u32, kind: u32,
                     in_flight: Option<Job>) -> Option<u32> {
    let job = in_flight?;
    if job.contaminated || job.request != request || request.row >= 8
        || observed_sub != request.sub || !PORTRAIT_SUBS.contains(&request.sub) {
        return None;
    }
    class_sub(kind)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request { pub row: u32, pub sub: u32 }

/// `req+0x198`, `req+0x19c`, `req+0x1a8` read in the current callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestWords { pub index: i32, pub registered: u8, pub holder: usize }

impl RequestWords {
    /// Allocation state (`3302f7c/3302f80/3302f88`) or released by `32c4d90`. `+0xc/+0xe`
    /// are deliberately not consulted: their allocation values were not established.
    pub fn idle_unregistered(self) -> bool { self.index == -1 && self.registered == 0 && self.holder == 0 }
    /// What one of our registered requests must still read on this row.
    pub fn owned_live(self, row: u32) -> bool {
        self.index == table_index(row) as i32 && self.registered == 1 && self.holder != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding { pub request: Request, pub descriptor: usize }

/// A completed render reserved for a future active publication. The serial
/// distinguishes repeated preparation of the same request/descriptor. Native
/// source/configuration/scene identities are additionally sealed by the adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prepared { pub binding: Binding, pub generation: u64, pub serial: u64 }

/// One issued render. `superseded`, `contaminated` and `departed` are latches: the job
/// stays in flight (and its exclusive helper lease stays held) until the native queue
/// drains, whatever happened meanwhile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Job {
    pub owner: usize, pub request: Request, pub generation: u64, pub before_table: usize,
    pub superseded: bool, pub contaminated: bool, pub departed: bool,
    /// `+0x19c == 0` was read after issue: the `0xa` retarget (`3304b48`) ran for this render.
    pub saw_clear: bool,
}

impl Job {
    pub const fn new(owner: usize, request: Request, generation: u64, before_table: usize) -> Self {
        Self { owner, request, generation, before_table, superseded: false, contaminated: false, departed: false, saw_clear: false }
    }

    /// A live HUD update belongs on its service callback. Retirement still
    /// drains on pipeline/destructor callbacks, even after the event was sent
    /// and an asynchronous HUD asset update remains pending.
    pub const fn hud_service_required(self, withheld: bool, assets_pending: bool) -> bool {
        !self.departed && !self.superseded && !self.contaminated
            && (withheld || assets_pending)
    }
}

/// What `publish_mii_presentation` passes. Absence of a configuration is never an intent.
/// The adapter passes `Original` only (it refuses any other intent before the call); `Exact` is
/// the host tests' proof that an exact presentation never restores the original.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    #[cfg(test)]
    Exact,
    Original { original_kind: u32 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ownership {
    /// Requests this module registered in the current batch, as a (row, sub) set.
    pub owned: [Option<Request>; OWNED_CAPACITY],
    /// The request each visible pane currently draws. Cleared only by a successful rebind
    /// (`bound`) or by verified batch teardown (`teardown`), never by intent cancellation.
    pub bound: [Option<Binding>; VISIBLE_OWNERS],
    pub prepared: [Option<Prepared>; VISIBLE_OWNERS],
    preparation_serial: u64,
    pub in_flight: Option<Job>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Select { Request { request: Request, first_use: bool }, Defer(&'static str), Refuse(&'static str) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Wait, Bind { owner: usize, index: u32 }, Discard(&'static str) }

/// `S'+0x134` and `S'+0x138`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Queue { pub count: i32, pub word: u32 }

impl Queue {
    /// Both native items ran: `0xa` retargeted and rendered, `0xb` created the holder.
    pub fn drained(self) -> bool { self.count == 0 && (self.word | 1) != 0xb }
}

/// Queue plus `table[0x12+row]` under its mutex and the request's `+0x180` address.
#[derive(Clone, Copy, Debug)]
pub struct Completion { pub queue: Queue, pub table: usize, pub descriptor: usize }

impl Ownership {
    pub const fn new() -> Self {
        Self { owned: [None; OWNED_CAPACITY], bound: [None; VISIBLE_OWNERS],
            prepared: [None; VISIBLE_OWNERS], preparation_serial: 0, in_flight: None }
    }

    pub fn is_owned(&self, request: Request) -> bool { self.owned.iter().flatten().any(|r| *r == request) }
    #[cfg(test)]
    pub fn owned_count(&self) -> usize { self.owned.iter().flatten().count() }

    fn own(&mut self, request: Request) -> bool {
        if self.is_owned(request) { return true; }
        match self.owned.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => { *slot = Some(request); true }
            None => false,
        }
    }

    fn disown(&mut self, request: Request) {
        for slot in self.owned.iter_mut() { if *slot == Some(request) { *slot = None; } }
    }

    pub fn bound_by_any(&self, request: Request) -> bool {
        self.bound.iter().flatten().any(|b| b.request == request)
    }

    pub fn reserved_by_any(&self, request: Request) -> bool {
        self.bound_by_any(request)
            || self.prepared.iter().flatten().any(|p| p.binding.request == request)
    }

    pub fn row_free_for(&self, owner: usize, row: u32) -> bool {
        owner < VISIBLE_OWNERS && row_free_for(&self.bound, owner, row)
            && self.prepared.iter().enumerate().all(|(o, p)|
                o == owner || p.is_none_or(|p| p.binding.request.row != row))
    }

    /// Ordinary helper selection excludes every foreign displayed or retained row.
    /// The native backend separately admits only its parked helper pair.
    pub fn source_exclusions(&self, owner: usize, _training: bool) -> u8 {
        (0..8).fold(0, |mask, row|
            mask | if self.row_free_for(owner, row) { 0 } else { 1u8 << row })
    }

    pub fn prepare(&mut self, owner: usize, binding: Binding, generation: u64)
        -> Result<Prepared, &'static str> {
        if !self.row_free_for(owner, binding.request.row) { return Err("prepared row belongs to another owner"); }
        if self.prepared[owner].is_some() || self.reserved_by_any(binding.request)
            || self.in_flight.is_some() || !self.is_owned(binding.request)
            || binding.descriptor == 0 { return Err("request cannot be prepared"); }
        let serial = self.preparation_serial.checked_add(1).ok_or("preparation serial exhausted")?;
        self.preparation_serial = serial;
        let prepared = Prepared { binding, generation, serial };
        self.prepared[owner] = Some(prepared);
        Ok(prepared)
    }

    /// Pick the request to retarget for `row`; `words[i]` are fresh reads of `PORTRAIT_SUBS[i]`.
    /// Never a request bound by any pane, never while a job is in flight, never a request
    /// native code appears to use.
    pub fn select(&mut self, row: u32, words: [RequestWords; 3]) -> Select {
        if self.in_flight.is_some() { return Select::Refuse("a portrait render is already in flight"); }
        let mut fresh = None;
        for (i, sub) in PORTRAIT_SUBS.iter().copied().enumerate() {
            let request = Request { row, sub };
            if self.reserved_by_any(request) { continue; }
            if self.is_owned(request) {
                if words[i].owned_live(row) { return Select::Request { request, first_use: false }; }
                self.disown(request);
            }
            if fresh.is_none() && words[i].idle_unregistered() { fresh = Some(request); }
        }
        match fresh {
            Some(request) => Select::Request { request, first_use: true },
            None => Select::Defer("no free, idle request on the helper row"),
        }
    }

    /// Record the issued job; exactly one at a time across all owners.
    pub fn issue(&mut self, job: Job) -> Result<(), &'static str> {
        if self.in_flight.is_some() { return Err("publisher busy"); }
        if self.reserved_by_any(job.request) { return Err("would retarget a bound or prepared request"); }
        if !self.row_free_for(job.owner, job.request.row) { return Err("row belongs to another owner"); }
        self.in_flight = Some(job);
        Ok(())
    }

    /// A newer intent for the same owner while its job is in flight. The job keeps waiting.
    pub fn supersede(&mut self, owner: usize, generation: u64) {
        if let Some(job) = self.in_flight.as_mut() {
            if job.owner == owner && job.generation != generation { job.superseded = true; }
        }
    }

    /// Intent cancellation (owner entry destroyed, request withdrawn). The in-flight job is
    /// superseded and keeps waiting; the owner's bound record is untouched because the native
    /// pane may still draw that descriptor.
    pub fn cancel_owner(&mut self, owner: usize) {
        if owner < VISIBLE_OWNERS { self.prepared[owner] = None; }
        if let Some(job) = self.in_flight.as_mut() { if job.owner == owner { job.superseded = true; } }
    }

    /// Shared `S'+0x2e0/+0x2e4` changed before the queue drained: latch; keep waiting.
    #[cfg(test)]
    pub fn contaminate(&mut self) {
        if let Some(job) = self.in_flight.as_mut() { job.contaminated = true; }
    }

    /// Scene departure: the in-flight job may never bind, but it is not abandoned here.
    pub fn depart(&mut self) {
        self.prepared = [None; VISIBLE_OWNERS];
        if let Some(job) = self.in_flight.as_mut() { job.departed = true; }
    }

    /// Verified batch teardown (holder `[base+0x532f758]` nulled or replaced): every request
    /// and pane binding is gone. The in-flight record is kept, marked departed, until the
    /// native queue is seen drained or the source object is gone (`abandon_in_flight`).
    pub fn teardown(&mut self) {
        self.owned = [None; OWNED_CAPACITY];
        self.bound = [None; VISIBLE_OWNERS];
        self.depart();
    }

    /// Only after authoritative proof that nothing can drain (native source object gone).
    pub fn abandon_in_flight(&mut self) -> Option<Job> { self.in_flight.take() }

    /// Evaluate the in-flight job. `same_target` is `S'+0x2e0==row && S'+0x2e4==sub` this tick.
    /// Nothing ends before the queue drains; a contaminated, superseded or departed job is
    /// then discarded, never bound, even if the fields match again.
    pub fn complete(&mut self, c: Completion, words: RequestWords, same_target: bool) -> Outcome {
        self.complete_when_helper_idle(c, words, same_target, false)
    }

    /// A different helper may be loading while this leased helper's render drains.
    /// Keep ownership until its proof can be read again. Still latch contamination
    /// and observed registration clearing during that wait; a later idle callback
    /// must not turn a superseded or contaminated render into a valid publication.
    pub fn complete_when_helper_idle(&mut self, c: Completion, words: RequestWords,
        same_target: bool, helper_busy: bool) -> Outcome {
        let mut job = match self.in_flight { Some(job) => job, None => return Outcome::Wait };
        if !same_target && !job.contaminated { job.contaminated = true; self.in_flight = Some(job); }
        if words.registered == 0 && !job.saw_clear { job.saw_clear = true; self.in_flight = Some(job); }
        if !c.queue.drained() || helper_busy { return Outcome::Wait; }
        self.in_flight = None;
        if job.contaminated {
            self.disown(job.request);
            return Outcome::Discard(DISCARD_CONTAMINATED);
        }
        // Reusing the request that already occupies `table[0x12+row]` re-registers the same
        // descriptor, so an unchanged table pointer is fresh only if the retarget was seen.
        let registered = words.owned_live(job.request.row)
            && c.descriptor != 0 && c.table == c.descriptor
            && (job.saw_clear || c.table != job.before_table);
        if !registered {
            self.disown(job.request);
            return Outcome::Discard(DISCARD_UNREGISTERED);
        }
        if !self.own(job.request) { return Outcome::Discard(DISCARD_FULL); }
        if job.departed { return Outcome::Discard(DISCARD_DEPARTED); }
        if job.superseded { return Outcome::Discard(DISCARD_SUPERSEDED); }
        Outcome::Bind { owner: job.owner, index: table_index(job.request.row) }
    }

    /// After `141b8a0` succeeded and `slot+0x1ac` was verified. The owner's previous request
    /// becomes free (still owned); that is the only non-teardown way a bound record ends.
    pub fn bound(&mut self, owner: usize, binding: Binding) {
        if owner < VISIBLE_OWNERS {
            self.bound[owner] = Some(binding);
            self.prepared[owner] = None;
        }
    }

    /// After a verified explicit original rebind (pane read back on `0x12+owner`): the custom
    /// request is no longer drawn. It stays owned with its texture untouched; only a later
    /// `select` may retarget it. Never called on intent alone.
    pub fn unbind(&mut self, owner: usize) {
        if owner < VISIBLE_OWNERS { self.bound[owner] = None; }
    }

    /// Per-tick audit of owned requests on `row`; returns owners whose bound request was lost.
    #[cfg(test)]
    pub fn audit(&mut self, row: u32, words: [RequestWords; 3]) -> [bool; VISIBLE_OWNERS] {
        let mut lost = [false; VISIBLE_OWNERS];
        for (i, sub) in PORTRAIT_SUBS.iter().copied().enumerate() {
            let request = Request { row, sub };
            if !self.is_owned(request) || words[i].owned_live(row) { continue; }
            self.disown(request);
            for (owner, b) in self.bound.iter_mut().enumerate() {
                if b.is_some_and(|b| b.request == request) { *b = None; lost[owner] = true; }
            }
        }
        lost
    }
}

/// Words read before issuing the two native commands.
#[derive(Clone, Copy, Debug)]
pub struct Admission {
    pub sequence_pending: u8, pub sequence_busy: u8, pub source_ready: u8, // G+0xd, G+0x1dfe8, S'+0x328
    pub batch_busy: u32,                                                // native portrait batch +0xc8
    pub count: i32, pub word: u32,                                        // S'+0x134, S'+0x138
    pub template_gate: u8, pub scene_byte: u8,                            // internal+0xb20, base+0x52c51b0
    /// The 32c7404 scoped skip is installed (fingerprinted) and will be armed for this job.
    pub skip_armed: bool,
    pub install_active: bool, pub helper_parked: bool, pub helper_exact: bool, pub lease_valid: bool,
    pub pane_index: u32, pub pane_bound: bool,                            // slot+0x1ac, slot+0x1a8!=0 && +0x1a9==1
}

pub fn admit(a: Admission, owner: u32, row: u32) -> Result<(), &'static str> {
    admit_render(a, owner, row)?;
    if !a.pane_bound { return Err("visible slot has no bound bust pane"); }
    if a.pane_index >= 0x20 { return Err("visible pane index out of range"); }
    Ok(())
}

/// A fixed parked source has no visible pane yet. This admits rendering only;
/// publication must later satisfy the ordinary pane/bind and active-source proof.
pub fn admit_prepared(a: Admission, owner: u32, row: u32) -> Result<(), &'static str> {
    if !matches!(owner, 2 | 3) || row != owner { return Err("not a fixed parked portrait row"); }
    admit_render(a, owner, row)
}

fn admit_render(a: Admission, owner: u32, row: u32) -> Result<(), &'static str> {
    if !a.lease_valid { return Err("portrait lease not valid"); }
    if a.install_active { return Err("INSTALL active"); }
    if !a.helper_parked || !a.helper_exact { return Err("helper not parked/exact"); }
    // run42 observed the gate byte 01 in Training: the copy is prevented only by the scoped skip.
    if a.template_gate != 0 && !a.skip_armed {
        return Err("D template copy gate set and no scoped skip armed; render would rewrite helper D");
    }
    if a.scene_byte & 0xfe == 0x1c { return Err("alternate render path scene"); }
    if a.sequence_pending != 0 || a.sequence_busy != 0 { return Err("native HUD sequence busy"); }
    if a.source_ready == 0 { return Err("native source not ready"); }
    if a.batch_busy != 0 { return Err("native portrait batch busy"); }
    if a.count != 0 || (a.word | 1) == 0xb { return Err("native source busy"); }
    let _ = (owner, row);
    Ok(())
}

/// Explicit original restoration only: never inferred from an absent configuration.
pub fn restore_original(intent: Intent, now_kind: u32, owner: u32, w: RequestWords, table: usize, descriptor: usize) -> bool {
    match intent {
        Intent::Original { original_kind } =>
            now_kind == original_kind && class_sub(original_kind).is_some()
                && w.owned_live(owner) && table != 0 && table == descriptor,
        #[cfg(test)]
        Intent::Exact => false,
    }
}

/// What the visible HUD may do at publication. This decides timing only.
/// It does not identify which pixels the native portrait functor draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudIntent { NotMii, Exact, Original }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudRefresh {
    /// Deliver the portrait event at publication.
    Now,
    /// Keep the previous valid portrait until this generation's exact render is proved.
    DeferUntilPortrait,
}

/// Mii kinds are Brawler/Sword/Gunner. Only an active, in-scope, unhalted exact
/// portrait has a later render proof worth waiting for. Original restoration is
/// the native event itself, so it still publishes immediately.
pub fn hud_refresh(kind: u32, intent: HudIntent, active: bool, in_scope: bool, halted: bool) -> HudRefresh {
    if matches!(kind, 0x48..=0x4a)
        && matches!(intent, HudIntent::Exact)
        && active && in_scope && !halted
    {
        HudRefresh::DeferUntilPortrait
    } else {
        HudRefresh::Now
    }
}

/// One generation's withheld HUD event. No entry pointer or lease is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudHold {
    Published,
    Withheld { generation: u64 },
    Superseded { generation: u64 },
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudStep {
    /// Leave the visible portrait untouched.
    Keep,
    /// Event, then a fresh pane read, then the texture bind. One service callback.
    RefreshRereadBind,
    /// Event only. The exact texture will not be bound.
    RefreshFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudTerminal { Refusal, Retirement, Budget, SceneExit, Superseded, Completed }

impl HudHold {
    pub const fn start(decision: HudRefresh, generation: u64) -> Self {
        match decision {
            HudRefresh::Now => Self::Published,
            HudRefresh::DeferUntilPortrait => Self::Withheld { generation },
        }
    }

    pub const fn is_withheld(self) -> bool {
        matches!(self, Self::Withheld { .. })
    }

    /// A newer generation owns the event. This generation must not publish later.
    pub const fn supersede(self, newer: u64) -> Self {
        match self {
            Self::Withheld { generation } if newer != generation => Self::Superseded { generation },
            other => other,
        }
    }

    /// Scene departure drops the right to publish. A published event is already done.
    pub const fn retire(self) -> Self {
        match self {
            Self::Published => Self::Published,
            _ => Self::Retired,
        }
    }

    pub const fn on_proof(self, generation: u64, owner_current: bool, service_context: bool) -> (Self, HudStep) {
        match self {
            Self::Withheld { generation: held } if held == generation && owner_current && service_context => {
                (Self::Published, HudStep::RefreshRereadBind)
            }
            Self::Withheld { .. } if !service_context => (self, HudStep::Keep),
            Self::Withheld { generation: held } if held != generation => (Self::Superseded { generation: held }, HudStep::Keep),
            Self::Withheld { .. } => (Self::Retired, HudStep::Keep),
            other => (other, HudStep::Keep),
        }
    }

    pub const fn on_terminal(
        self, generation: u64, owner_current: bool, service_context: bool, terminal: HudTerminal,
    ) -> (Self, HudStep) {
        let Self::Withheld { generation: held } = self else {
            return (self, HudStep::Keep);
        };
        match terminal {
            HudTerminal::SceneExit | HudTerminal::Completed => (Self::Retired, HudStep::Keep),
            HudTerminal::Superseded => (Self::Superseded { generation: held }, HudStep::Keep),
            HudTerminal::Refusal | HudTerminal::Retirement | HudTerminal::Budget => {
                if held != generation {
                    (Self::Superseded { generation: held }, HudStep::Keep)
                } else if !owner_current {
                    (Self::Retired, HudStep::Keep)
                } else if !service_context {
                    (self, HudStep::Keep)
                } else {
                    (Self::Published, HudStep::RefreshFallback)
                }
            }
        }
    }
}

/// A queued successor owns the event, including when the drained render is discarded
/// for contamination or a missed registration rather than the supersede wording.
pub const fn terminal_for_finish(requested: HudTerminal, successor: bool) -> HudTerminal {
    if successor { HudTerminal::Superseded } else { requested }
}

pub const DISCARD_CONTAMINATED: &str = "contaminated: shared row/sub fields changed before the queue drained";
pub const DISCARD_UNREGISTERED: &str = "queue drained without a fresh registration of our request";
pub const DISCARD_FULL: &str = "ownership set full; request left unbound";
pub const DISCARD_DEPARTED: &str = "scene departed before completion; request kept free";
pub const DISCARD_SUPERSEDED: &str = "superseded before completion; request kept free";

/// `DISCARD_DEPARTED` is retirement, not a silent scene exit. `on_terminal`
/// then paints once only for this generation when the caller reports the owner
/// still current and passes the service token. Scene exit and a dead owner
/// have already retired the hold, and a failed owner check does not paint.
pub fn hud_terminal_for_discard(reason: &str) -> HudTerminal {
    if reason == DISCARD_SUPERSEDED {
        HudTerminal::Superseded
    } else if reason == DISCARD_DEPARTED {
        HudTerminal::Retirement
    } else {
        HudTerminal::Refusal
    }
}

/// The deferred request was not stored. Paint the kind now, or the previous portrait
/// would remain for this still-current owner. A foreign callback must not paint.
pub const fn rejected_deferral(deferred: bool, accepted: bool, owner_current: bool, service_context: bool) -> HudStep {
    if deferred && !accepted && owner_current && service_context {
        HudStep::RefreshFallback
    } else {
        HudStep::Keep
    }
}

/// Pane words read after the portrait event returned. The pre-event index is not an input:
/// the event may have changed bust mode, and the bind uses the proved table index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneRead { pub index: u32, pub part_ready: bool }

/// A portrait event queues native HUD asset work; its return is not a framing
/// barrier. Run146 caught the Gunner event key alongside the old G&W committed
/// key at 141b8a0. That binder reads +170 to choose framing. The native loader
/// commits that key at 1419ff8 and clears its channel state only after the
/// applier returns (141a830..141a868).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudAssets {
    pub committed_key: u64,
    pub event_dirty: bool,
    pub channel_busy: bool,
    pub update_busy: bool,
}

pub const fn hud_assets_settled(expected_key: u64, current: HudAssets) -> bool {
    expected_key != 0 && current.committed_key == expected_key
        && !current.event_dirty && !current.channel_busy && !current.update_busy
}

/// Where a drained Training render may bind once `hud_assets_settled` holds. The
/// native source tick (`3308f30`) runs in the scene node's own update (`3725350`),
/// after the HUD slot loader committed `+0x170` under that node's child traversal
/// (`3725340`; run151 `hud-order151`, thread 80). A bind there is never later than
/// the next entry service callback. Only a published hold qualifies: publication,
/// completion and the helper's return stay on the service callback, and a retiring
/// flight keeps draining there.
pub const fn tick_bind_allowed(
    flight: Job, hold: HudHold, assets_pending: bool, same_thread: bool, training: bool, halted: bool,
) -> bool {
    training && same_thread && assets_pending && !halted
        && matches!(hold, HudHold::Published)
        && !flight.departed && !flight.superseded && !flight.contaminated
}

/// After the asset wait the service callback binds unless the bind was refused or the
/// source tick already bound this pane; it still records the terminal and returns the helper.
pub const fn service_bind_after_wait(bind_refused: bool, tick_bound: bool) -> bool {
    !bind_refused && !tick_bound
}

/// An exact portrait this module bound into a Training HUD slot: the publication it
/// presents and its retained registration. Values only; identities are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown { pub entry: usize, pub physical: u32, pub kind: u32, pub generation: u64, pub binding: Binding }

/// The live Mii publication a native presentation rebuild (Training reset) found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Live { pub entry: usize, pub physical: u32, pub kind: u32, pub generation: u64 }

impl Shown {
    pub fn presents(self, live: Live) -> bool {
        self.entry == live.entry && self.physical == live.physical
            && self.kind == live.kind && self.generation == live.generation
    }
}

/// After native reset rebuilt the original CSS portrait over a live Mii publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Represent {
    /// A pending job still withholds this owner's event; it publishes and binds itself.
    Defer,
    /// Republish the live kind (event 0x1d) only; no retained exact texture applies.
    Paint,
    /// Republish, then rebind the retained texture once that event's assets settle.
    Rebind(Shown),
}

/// `pending_withheld` is None without a job, else whether its event is still withheld.
/// `retained` means fresh reads found the shown binding still this owner's bound record,
/// owned, and registered at `table[0x12+row]`. Nothing here renders or leases a helper.
pub fn represent(pending_withheld: Option<bool>, shown: Option<Shown>, live: Live, retained: bool, halted: bool) -> Represent {
    match pending_withheld {
        Some(true) => Represent::Defer,
        Some(false) => Represent::Paint,
        None => match shown {
            Some(shown) if shown.presents(live) && retained && !halted => Represent::Rebind(shown),
            _ => Represent::Paint,
        },
    }
}

/// Service callbacks a rebind waits for native assets and its qualified source tick.
pub const REBIND_SETTLE_CALLBACKS: u16 = 1200;

/// One armed rebind. `key` is the event key read right after the republish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RebindWait { pub shown: Shown, pub key: u64, pub waits: u16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebindStep { Wait, Bind(Binding), Drop(&'static str) }

impl RebindWait {
    /// One service callback of the shown owner. `live` is None when its body, scope or
    /// generation no longer qualifies. Only the retained binding is ever bound.
    pub fn step(&mut self, live: Option<Live>, settled: bool, retained: bool) -> RebindStep {
        if !live.is_some_and(|live| self.shown.presents(live)) {
            return RebindStep::Drop("owner body, scope or generation changed before the rebind");
        }
        if !settled {
            if self.waits >= REBIND_SETTLE_CALLBACKS {
                return RebindStep::Drop("native HUD rebind did not reach a qualified tick; exact texture left unbound");
            }
            self.waits += 1;
            return RebindStep::Wait;
        }
        if !retained {
            return RebindStep::Drop("retained exact texture is no longer registered or bound");
        }
        RebindStep::Bind(self.shown.binding)
    }
}



pub const fn after_event_pane(fresh: Option<PaneRead>) -> bool {
    match fresh {
        Some(pane) => pane.part_ready && pane.index < 0x20,
        None => false,
    }
}

/// Exclusive lease rule over the two helper slots (SourceBook facts the caller reads). Host-tested
/// design rule; the adapter does not consult it, so it compiles for tests only.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SlotState {
    pub ready: bool, pub leases: usize, pub same_configuration: bool,
    pub replaceable: bool, pub hat_eligible: bool, pub portrait_pending: bool,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice { Share(usize), Install(usize), Defer }

#[cfg(test)]
pub fn choose_slot(slots: [SlotState; 2]) -> Choice {
    for (i, s) in slots.iter().enumerate() {
        if s.ready && s.same_configuration && s.leases == 0 && !s.portrait_pending { return Choice::Share(i); }
    }
    for (i, s) in slots.iter().enumerate() {
        if s.replaceable && s.leases == 0 && s.hat_eligible && !s.portrait_pending { return Choice::Install(i); }
    }
    Choice::Defer
}

/// Hat/item `prepare` must skip a Ready slot while a portrait render is pending on it.
pub fn hat_may_share(portrait_pending: bool) -> bool { !portrait_pending }

/// What the `base+0x32c7404` inline callback sees. Registers come from the `InlineCtx`;
/// `armed_*` from the adapter's atomics (set at issue, cleared on any invalidation);
/// `fresh_*` are re-read from the symbols in the callback; `req_row/req_sub` are `[x20+0]/[x20+4]`.
#[derive(Clone, Copy, Debug)]
pub struct HookView {
    pub x19: usize, pub x20: usize, pub x21: usize,
    pub armed_request: usize, pub armed_entry: usize,
    pub req_row: i32, pub req_sub: i32, pub entry_owner: u32,
    pub fresh_internal: usize, pub fresh_entry: usize, pub fresh_request: usize,
}

/// True only for the one render that belongs to the in-flight, uncontaminated portrait job.
/// A superseded or departed job still qualifies on purpose: its render is still queued and the
/// helper's D block still deserves protection until the queue drains.
pub fn skip_template(v: HookView, in_flight: Option<Job>) -> bool {
    let job = match in_flight { Some(job) => job, None => return false };
    !job.contaminated
        && v.armed_request != 0 && v.x20 == v.armed_request && v.x20 == v.fresh_request
        && v.armed_entry != 0 && v.x19 == v.armed_entry && v.x19 == v.fresh_entry
        && v.x21 == v.fresh_internal
        && v.entry_owner == job.request.row
        && v.req_row == job.request.row as i32 && v.req_sub == job.request.sub as i32
}

/// Completion-time judgement of the helper's D after the render (host-tested rule; no runtime caller).
#[cfg(test)]
pub fn template_outcome(template_gate: u8, skipped: bool) -> Result<(), &'static str> {
    if template_gate != 0 && !skipped {
        return Err("render ran without the scoped skip; helper D assumed templated, re-proof decides");
    }
    Ok(())
}

/// The batch's request table index (`batch+0xd8+index*8`, eight rows of ten subs); the
/// pass keeps the index it is rendering at `batch+0x370`.
pub const fn request_index(request: Request) -> u32 { request.row * 10 + request.sub }

/// What the `32c52e4` inline callback sees. The native batch pass has just read its own
/// "stage already hidden" latch (`batch+0x358`, in `w9`) and is about to hide every stage
/// (`core+0x178 = 1` at `32c5380`, per-stage `+0x30/+0x34` counts) for the whole pass,
/// then set the latch (`32c5c24`). The pass cursor `batch+0x370` still reads 0 on that
/// first call (the terminal path at `32c55c8` is its only reset); only afterwards does the
/// advance loop (`32c6ed8..32c6f00`) skip slots whose request `+0xc` is clear, the flag the
/// `0xa` retarget sets at `3304b48`. Run152 traced the hide store with `x19` the live batch
/// and `x20` the observer's stage core, and the terminal unhide (`32c5700`) on the same
/// batch. Both unhides (`32c55c4`, `32c7b7c`) are gated by the latch.
#[derive(Clone, Copy, Debug)]
pub struct StageHideView {
    /// `x19`, the fresh `[base+0x532f758]` holder, and the batch recorded at issue.
    pub batch: usize, pub fresh_batch: usize, pub issued_batch: usize,
    /// `w9` as native code loaded it from `batch+0x358`.
    pub latch: u8,
    pub armed_request: usize,
    /// The request table as the pass will scan it.
    pub activity: BatchActivity,
}

/// Fold over the eighty request slots (`batch+0xd8+slot*8`) with the pass's own
/// criterion: a slot is active while its request's `+0xc` is set. The owned slot is
/// `request_index` of the in-flight request; a null slot is neither owned nor foreign.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchActivity {
    /// Slot whose pointer was folded as owned; activity does not end ownership.
    pub owned_slot: Option<u32>,
    /// The request pointer at the owned slot, 0 when absent.
    pub owned_request: usize,
    /// Active requests on every other slot.
    pub foreign_active: u32,
}

impl BatchActivity {
    pub fn note(&mut self, slot: u32, owned_slot: u32, request: usize, active: bool) {
        if slot == owned_slot {
            self.owned_slot = Some(slot);
            self.owned_request = request;
        } else if request != 0 && active {
            self.foreign_active += 1;
        }
    }
}

/// True only for the pass that renders the owned, uncontaminated request alone, on the
/// batch it was issued on. The adapter then reports the latch as set in `w9` so native code
/// takes its own already-hidden branch: no hide, no counts, no latch, and the latch-gated
/// unhides do nothing. A latch that already reads set has nothing to bypass. Any foreign
/// active request keeps the native hide for that pass. The cursor is not an input: the
/// first call hides before it moves. Superseded and departed jobs still execute their
/// queued render, so their pass qualifies too.
/// The owned request's activity byte is deliberately NOT an ownership test: native code
/// clears it at `32c6ecc` before the batch finishes. Run153 caught the subsequent hide
/// with that byte clear while the issued job, lease and request identity were still live.
/// Keep the bypass through that drain; a newly active foreign request still refuses it.
pub fn bypass_stage_hide(v: StageHideView, in_flight: Option<Job>) -> bool {
    let job = match in_flight { Some(job) => job, None => return false };
    !job.contaminated
        && v.latch == 0
        && v.armed_request != 0
        && v.activity.owned_slot == Some(request_index(job.request))
        && v.activity.owned_request == v.armed_request
        && v.activity.foreign_active == 0
        && v.batch != 0 && v.batch == v.fresh_batch && v.batch == v.issued_batch
        && job.request.row < 8 && PORTRAIT_SUBS.contains(&job.request.sub)
}




/// A helper avoided by this owner cannot satisfy or retain its pending want.
pub fn slot_may_install(avoid: u8, slot: u32) -> bool { slot < 8 && avoid & (1u8 << slot) == 0 }

/// Validate the part index before the adapter reads its child pointer.
pub fn part_offset(index: i32) -> Option<usize> {
    (0..8).contains(&index).then(|| index as usize * 0xd90 + 0x330)
}

/// Native portrait stores are admitted only in qualified practice scenes: Training,
/// local-wireless waiting and the online waiting routes admitted by the
/// shared PracticeScene policy. Matches, arenas and every other mode store nothing.
pub fn may_write_scene(melee: u8, mode: u32) -> bool {
    matches!((melee, mode), (0x20, 0x04040000) | (0x0d, 0x02010000))
        || (melee == 0x0d && crate::engine::mii::backend::Layout::quickplay_waiting_mode(mode))
}

