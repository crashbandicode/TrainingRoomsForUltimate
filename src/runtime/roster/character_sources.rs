//! Shared, demand-driven native source ownership for copy abilities and items.
//! Native entries are borrowed only at their own loader callback. Requests,
//! leases and epochs contain values; every game accessor is resolved afresh.
#[cfg(feature = "managed-third-source-alpha")]
use super::native_source_capacity::managed as capacity;
use super::{aligned, finish_install, native_source, read, InstallToken};
use super::{ready_forms, INSTALL};
#[cfg(feature = "mii-managed-source-coexist-alpha")]
use super::mii_ordinary_player::native as coexist;
use crate::engine::swap::source_lifetime::{
    Holder, LeaseToken, LoadToken, NativeEntry, SceneEpoch, SourceBook,
    SourceToken,
};
use crate::engine::swap::source_lifetime::{Phase, SourceKey};
#[cfg(feature = "managed-third-source-alpha")]
use skyline::hooks::InlineCtx;
use super::native_service::ServiceEntryContext;
use smash::app::{self, lua_bind::*, sv_battle_object};
use smash::lib::lua_const::*;
use std::convert::TryFrom;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::engine::swap::source_load::{verdict, Unfinished, Verdict};
#[cfg(any(feature = "managed-third-source-alpha", feature = "mii-managed-source-coexist-alpha"))]
use crate::engine::swap::source_load::STALL_TICKS;
#[cfg(feature = "managed-third-source-alpha")]
use crate::engine::swap::capacity_creation::{self as creation, Refusal, Waiting};

/// The once-per-load reports at the bound (FAIL 2). Native has not completed the load and keeps it
/// with its INSTALL token; or native completed it, its managed proof failed, and the load is failed
/// with INSTALL released. Both keep the `stalled` prefix the Eden rows grep.
const STALL_RETAINED: &str = "stalled; native ownership retained";
const STALL_FAILED: &str = "stalled; native completion proven, scene proof failed: load failed, INSTALL released";
const NATIVE_WITHOUT_RESOURCES: &str = "native completion without usable resources";

// P1 hat, CPU hat, P1 item, then (portrait feature) P1 portrait, CPU portrait.
const CHANNELS: usize = if cfg!(feature = "mii-hud-portrait-alpha") { 5 } else { 3 };
/// Portrait wants carry the loaded configuration and avoid the other owner's bust row.
#[cfg(feature = "mii-hud-portrait-alpha")]
pub const PORTRAIT_CHANNELS: [usize; 2] = super::mii_hud_portrait_policy::SOURCE_CHANNELS;
const LEASES: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Want {
    epoch: SceneEpoch,
    kind: u32,
    configuration:Option<crate::engine::mii::lifetime::Configuration>,
    /// Portrait only. `None` keeps hat/item selection identical to the feature-off path.
    #[cfg(feature = "mii-hud-portrait-alpha")]
    avoid: Option<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Source {
    token: SourceToken,
    load: LoadToken,
    install: Option<InstallToken>,
    kind: u32,
    ticks: u16,
    prefetched: bool,
    measurement: Option<crate::engine::preload::preload_metrics::OperationStamp>,
    configuration:Option<crate::engine::mii::lifetime::Configuration>,
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    original: Option<coexist::helpers::OriginalRow>,
    /// GS-PANIC: committed process charge when the load was admitted; its measured charge is logged at ready.
    memory_committed: Option<u64>,
}

#[cfg(feature = "managed-third-source-alpha")]
#[derive(Clone, Copy)]
struct FirstLoad {
    wanted: Want,
    reservation: capacity::Reservation,
    previous: Option<Source>,
    before: capacity::Preflight,
    ticks: u16,
}

/// Event-only evidence; none of these fields participates in admission or release.
#[cfg(feature = "managed-third-source-alpha")]
#[derive(Clone, Copy, PartialEq, Eq)]
struct CleanupObservation {
    retiring: Option<Source>,
    #[cfg(feature = "managed-grouped-source-alpha")]
    current_actors: Option<capacity::Actors>,
    #[cfg(feature = "managed-grouped-source-alpha")]
    retiring_actors: Option<capacity::Actors>,
    #[cfg(feature = "managed-grouped-source-alpha")]
    masks: [(u8, u8); 2], // current/retiring: seen, valid; diagnostic only.
    #[cfg(feature = "managed-grouped-source-alpha")]
    cleaned: [[Option<capacity::Actor>; 3]; 2],
    last_actor: Option<(Option<SourceToken>, usize)>,
    ready_actors: u32,
    actor_events: u32,
    valid_actors: u32,
    ready_cleanups: u32,
    destroy_seen: bool,
    listeners_before: Option<u64>,
    listener_removed: Option<bool>,
}

#[cfg(feature = "managed-third-source-alpha")]
impl CleanupObservation {
    const fn new() -> Self {
        Self {
            retiring: None,
            #[cfg(feature = "managed-grouped-source-alpha")]
            current_actors: None,
            #[cfg(feature = "managed-grouped-source-alpha")]
            retiring_actors: None,
            #[cfg(feature = "managed-grouped-source-alpha")]
            masks: [(0, 0); 2],
            #[cfg(feature = "managed-grouped-source-alpha")]
            cleaned: [[None; 3]; 2],
            last_actor: None,
            ready_actors: 0,
            actor_events: 0,
            valid_actors: 0,
            ready_cleanups: 0,
            destroy_seen: false,
            listeners_before: None,
            listener_removed: None,
        }
    }
}

#[cfg(feature = "managed-third-source-alpha")]
struct Capacity {
    attempted: Option<SceneEpoch>,
    stable: Option<(Want, capacity::Preflight, u16, u8)>,
    reservation: Option<(SceneEpoch, capacity::Reservation)>,
    load: Option<(SourceToken, capacity::Preflight)>,
    first_load: Option<FirstLoad>,
    observation: CleanupObservation,
    /// lobby-preload D: which creation waits were logged in this scene.
    waiting: Waiting<SceneEpoch>,
}
#[cfg(feature = "managed-third-source-alpha")]
impl Capacity {
    const NEW:Self=Self {attempted:None,stable:None,reservation:None,load:None,
        first_load:None,observation:CleanupObservation::new(),
        waiting:Waiting::NEW};
}
#[cfg(feature = "managed-third-source-alpha")]
fn capacity_index(slot:u32)->usize {
    crate::engine::preload::cycle_preload_capacity::capacity_index(slot as usize)
        .expect("capacity access requires a validated extra slot")
}


#[derive(Clone, Copy)]
enum Consumer {
    Reserved,
    Hat { body: u32, kind: u32, slot: u32 },
    Item { body: u32, kind: i32 },
    /// Held only while a portrait render is queued. Scene departure and `release` end it.
    #[cfg(feature = "mii-hud-portrait-alpha")]
    Portrait,
}

#[derive(Clone, Copy)]
struct LiveLease {
    token: LeaseToken,
    source: SourceToken,
    consumer: Consumer,
    settle_polls: u8,
}

struct State {
    book: SourceBook<8, LEASES>,
    sources: [Option<Source>; 8],
    wants: [Option<Want>; CHANNELS],
    preload: [Option<Want>; crate::runtime::roster::cycle_preload::MAX_TARGETS],
    preload_failed: u128,
    preload_settled: bool,
    preload_limit: u8,
    #[cfg(feature="mii-managed-helper-alpha")]
    refusals: [Option<(Want, &'static str)>; CHANNELS],
    leases: [Option<LiveLease>; LEASES],
    failed: u128,
    #[cfg(feature = "managed-third-source-alpha")]
    capacities: [Capacity; crate::engine::preload::cycle_preload_capacity::ALL_EXTRA_SLOTS.len()],
    /// lobby-preload D: a creation preflight refusal standing in this scene and the refused
    /// extra's position in the creation order. Only the readiness line reads it.
    #[cfg(feature = "managed-third-source-alpha")]
    creation_hold: Option<(SceneEpoch, u8)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    book: SourceBook::new(),
    sources: [None; 8],
    wants: [None; CHANNELS],
    preload: [None; crate::runtime::roster::cycle_preload::MAX_TARGETS],
    preload_failed: 0,
    preload_settled: false,
    preload_limit: 0,
    #[cfg(feature="mii-managed-helper-alpha")]
    refusals: [None; CHANNELS],
    leases: [None; LEASES],
    failed: 0,
    #[cfg(feature = "managed-third-source-alpha")]
    capacities: [const { Capacity::NEW }; crate::engine::preload::cycle_preload_capacity::ALL_EXTRA_SLOTS.len()],
    #[cfg(feature = "managed-third-source-alpha")]
    creation_hold: None,
});
static HAS_CONSUMERS: AtomicBool = AtomicBool::new(false);
/// LOG-REFUSED: the (slot, reason) pairs whose memory refusal of a kind was already logged in this scene, one
/// bit per kind (`reason` = what × admission, [`refusal_reason`]); cleared by `scene_changed`. A preload refused on
/// every loader callback logs once per scene, slot, kind and reason.
static MEMORY_REFUSED: Mutex<[[u128; 4]; 8]> = Mutex::new([[0; 4]; 8]);

/// The reason index of a refusal line: what was refused (a preload or the helper owner created for one) × the
/// admission (refused or deferred).
fn refusal_reason(what: &str, admission: crate::engine::swap::body_bank::memory::Admission) -> usize {
    usize::from(what != "preload") * 2 + usize::from(admission == crate::engine::swap::body_bank::memory::Admission::Defer)
}

/// GS-PANIC (`process_memory`): what the helper loads in flight hold against the process budget, the whole
/// measured cost of each until it settles (ready, failed or departed releases its INSTALL token). Takes the
/// source lock; never call it with that lock held.
pub(super) fn helper_outstanding() -> u64 {
    use crate::engine::swap::body_bank::memory;
    STATE.lock().unwrap().sources.iter().flatten().filter(|source| source.install.is_some())
        .map(|source| memory::helper_outstanding(memory::helper_cost(source.kind)))
        .fold(0, u64::saturating_add)
}

/// GS-PANIC: `[CharacterSources] memory … phase=helper`: the committed charge of one helper load from its admission
/// to ready, next to its estimate (`tools/group_memory_costs.py` reads these lines as exact observations).
fn log_helper_memory(slot: u32, kind: u32, before: Option<u64>) {
    use crate::engine::swap::body_bank::memory;
    let after = crate::engine::preload::preload_metrics::MemorySample::capture();
    let committed = after.committed.get();
    let (members, count) = memory::helper_members(kind);
    let delta = before.zip(committed).map(|(b, a)| i128::from(a) - i128::from(b));
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] memory slot={} kinds={:x?} phase=helper committed={:?}->{:?} delta={} estimate={} headroom={:?}",
        slot, &members[..count], before, committed, delta.map_or_else(|| "na".to_string(), |d| d.to_string()),
        memory::helper_cost(kind).total(), after.headroom()));
}

/// GS-PANIC: a speculative helper load the process cannot hold now stays in the plan. LOG-REFUSED: the line uses the
/// shipped `[ProcessMemory]` tag (release builds print it, Surface R-GSP7), once per scene, slot, kind and reason.
fn memory_refused(slot: u32, kind: u32, decision: crate::engine::swap::body_bank::memory::Decision,
    cost: crate::engine::swap::body_bank::memory::GroupCost, what: &str) {
    let reason = refusal_reason(what, decision.admission);
    let first = {
        let mut seen = MEMORY_REFUSED.lock().unwrap();
        let bits = &mut seen[(slot as usize).min(7)][reason];
        let bit = 1u128 << (kind & 127);
        let first = *bits & bit == 0;
        *bits |= bit;
        first
    };
    if !first { return; }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ProcessMemory] refused consumer=helper slot={} kind=0x{:x} what={} admission={:?} headroom={:?} outstanding={} need={} prefetch={} build={}; stays planned",
        slot, kind, what, decision.admission, decision.headroom, decision.outstanding, decision.need, cost.prefetch, cost.build));
}

/// Retires only speculative intent. Native loads/leases keep their normal owner.
pub(crate) fn pause_preload() {
    let mut state=STATE.lock().unwrap(); state.preload=[None;crate::runtime::roster::cycle_preload::MAX_TARGETS];state.preload_settled=false;
}
pub(crate) fn preload_epoch()->SceneEpoch {STATE.lock().unwrap().book.epoch()}

/// The helper load holding INSTALL now, for a queued visible request's waiting line (FAIL 2):
/// (slot, kind, loader callbacks, speculative). None: a visible transaction or a capacity
/// creation holds it, or nothing does. Values only.
#[cfg(feature = "mii-ordinary-cpu-alpha")]
pub(crate) fn install_holder() -> Option<(u32, u32, u16, bool)> {
    let state = STATE.lock().unwrap();
    state.sources.iter().flatten().find(|source| source.install.is_some())
        .map(|source| (source.token.slot() as u32, source.kind, source.ticks, source.prefetched))
}

/// Current retained helper generations consume warm capacity even when leased
/// or finishing a load. Do not admit a body bank by counting only idle Ready
/// speculative sources; that would undercount a hat/portrait consumer.
#[cfg(feature = "native-body-bank-alpha")]
pub(super) fn warm_occupancy() -> usize {
    let state = STATE.lock().unwrap();
    state.sources.iter().flatten()
        .filter(|source| state.book.is_current(source.token.epoch())).count()
}

pub(crate) fn metrics() -> crate::engine::preload::preload_metrics::Preload {
    let settings=crate::runtime::roster::cycle_preload_config::snapshot();
    let state=STATE.lock().unwrap();
    let mut result=crate::engine::preload::preload_metrics::Preload {slots:settings.slots,effective:state.preload_limit,
        fighters:settings.fighters,hats:settings.hats,..Default::default()};
    for (slot,source) in state.sources.iter().enumerate().filter_map(|(slot,s)|s.map(|s|(slot,s))) {
        if !source.prefetched || !state.book.is_current(source.token.epoch()) {continue;}
        result.busy|=source.install.is_some();
        if state.leases.iter().flatten().any(|lease|lease.source==source.token) {continue;}
        if state.book.phase(slot)==Ok(Phase::Ready) {
            result.entries|=1<<slot;
            if usize::from(result.resident)<result.kinds.len() {
                result.kinds[usize::from(result.resident)]=Some(source.kind);
            }
            result.resident=result.resident.saturating_add(1);
        }
    }
    result
}

pub(crate) fn resident(target:crate::runtime::roster::cycle_preload::Target,hat:bool)->bool {
    let state=STATE.lock().unwrap();
    state.sources.iter().enumerate().any(|(slot,s)|s.is_some_and(|s|
        (s.kind==target.kind || (!hat && target.configuration.is_none()
            && crate::engine::preload::cycle_preload_policy::fighter_group(s.kind)==crate::engine::preload::cycle_preload_policy::fighter_group(target.kind)))
            && s.configuration==target.configuration
            && state.book.is_current(s.token.epoch()) && state.book.phase(slot)==Ok(Phase::Ready)))
}

/// Read-only readiness of the active speculative plan: one lock, no native
/// traversal, allocation, or scheduler change. A resident counts only when it is
/// a current-epoch Ready source matching an active plan target, so stale
/// residents, failed loads, and still-loading sources (leased or not) do not.
/// `set_preload_plan` drops a failed kind from `state.preload`, so the caller's
/// unfiltered `desired` plan (index-aligned with it) keeps failed targets in the
/// denominator instead of letting the plan quietly shrink to "all ready".
pub(crate) fn preload_readiness(blocked:bool,
    desired:&[Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS])->crate::engine::preload::cycle_preload_policy::Readiness {
    let configured=crate::runtime::roster::cycle_preload_config::snapshot().slots;
    let state=STATE.lock().unwrap();
    let failed=state.preload_failed|state.failed;
    let plan:[Option<(u32,_)>;crate::runtime::roster::cycle_preload::MAX_TARGETS]=std::array::from_fn(|i|
        state.preload[i].map(|want|(want.kind,want.configuration)).or_else(||desired[i]
            .filter(|target|failed&(1u128<<target.kind)!=0)
            .map(|target|(target.kind,target.configuration))));
    let ready:[Option<(u32,_)>;8]=std::array::from_fn(|slot|state.sources[slot]
        .filter(|s|state.book.is_current(s.token.epoch()) && state.book.phase(slot)==Ok(Phase::Ready))
        .map(|s|(s.kind,s.configuration)));
    crate::engine::preload::cycle_preload_policy::readiness(configured,shown_limit(&state),blocked,plan,
        |(kind,_)|failed&(1u128<<kind)!=0,ready)
}

/// The readiness line's capacity (lobby-preload D). While a creation preflight refusal stands in
/// this scene, the extras from the refused one on cannot be created, so the line counts what the
/// scene holds (`limited`) instead of reporting `Loading` for the whole visit. Planning, metrics
/// and creation keep `preload_limit`.
fn shown_limit(state:&State)->u8 {
    #[cfg(feature="managed-third-source-alpha")]
    if let Some((_,position))=state.creation_hold.filter(|(epoch,_)|state.book.is_current(*epoch)) {
        return creation::held_limit(usize::from(state.preload_limit),
            crate::engine::preload::cycle_preload_capacity::BASE_HELPERS,Some(usize::from(position))) as u8;
    }
    state.preload_limit
}

pub(crate) unsafe fn preload_hat_target(role:usize,kind:u32)->Option<crate::runtime::roster::cycle_preload::Target> {
    if !(0x48..=0x4a).contains(&kind) {return Some(crate::runtime::roster::cycle_preload::Target::fighter(kind));}
    #[cfg(feature="mii-managed-helper-alpha")]
    if super::mii_managed_helper::admission_ready(true) {
        return super::mii_managed_helper::configuration(role,kind).ok()
            .map(|configuration|crate::runtime::roster::cycle_preload::Target {kind,configuration:Some(configuration)});
    }
    let _=role;
    None
}

pub(crate) unsafe fn set_preload_plan(epoch:SceneEpoch,targets:[Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS]) {
    let Ok(backend)=qualified_backend() else {return;};
    let limit=preload_limit(backend);
    let targets=std::array::from_fn::<_,{crate::runtime::roster::cycle_preload::MAX_TARGETS},_>(|i|
        targets[i].filter(|_|i<limit));
    // Resolve exact helper compatibility outside STATE and without publishing a
    // consumer request. A speculative Mii never substitutes another appearance.
    let targets=targets.map(|target|target.and_then(|target| {
        crate::engine::swap::trio::TrioFighter::from_kind(target.kind)?;
        #[cfg(feature="mii-managed-helper-alpha")]
        let target=if (0x48..=0x4a).contains(&target.kind) {
            if !super::mii_managed_helper::admission_ready(true) {return None;}
            let hats=super::mii_ordinary_cpu::helper_origin_hats()?;
            crate::runtime::roster::cycle_preload::Target {configuration:Some(
                crate::engine::mii::ordinary_cpu::helper_configuration(target.configuration?,hats).ok()?),..target}
        }else{target};
        #[cfg(not(feature="mii-managed-helper-alpha"))]
        if (0x48..=0x4a).contains(&target.kind) {return None;}
        Some(target)
    }));
    let mut state=STATE.lock().unwrap();
    if !state.book.is_current(epoch) {return;}
    let plan=targets.map(|t|t.filter(|t|(state.preload_failed|state.failed) & (1u128<<t.kind)==0)
        .map(|t|Want {epoch:state.book.epoch(),kind:t.kind,configuration:t.configuration,
            #[cfg(feature="mii-hud-portrait-alpha")] avoid:None}));
    if plan!=state.preload {state.preload_settled=false;state.preload=plan;}
}

fn note_preload_settled() {
    let settings=crate::runtime::roster::cycle_preload_config::snapshot();
    if !settings.measurements {return;}
    let settled={let mut state=STATE.lock().unwrap();
        let settled=!state.preload_settled && state.preload.iter().any(Option::is_some)
            && state.preload.iter().flatten().all(|want|state.sources.iter().enumerate().any(|(i,s)|
                s.is_some_and(|s|s.kind==want.kind && s.configuration==want.configuration
                    && state.book.is_current(s.token.epoch()) && state.book.phase(i)==Ok(Phase::Ready))));
        if settled {state.preload_settled=true;} settled};
    if settled {crate::engine::preload::preload_metrics::snapshot(true,crate::engine::preload::preload_metrics::Label::Settled,
        crate::runtime::roster::cycle_preload::frame(),metrics());}
}

fn want_exact(want:Want)->bool {
    crate::engine::preload::cycle_preload_policy::exact_mii(want.kind,want.configuration.is_some())
}

/// Demand wins even if it needs the speculative source's slot. Speculation only
/// fills qualified helpers without taking a consumer lease. Extra owners remain
/// ordinary-only. On the native pair, slot-local order reserves exact Miis
/// ahead of ordinary targets; the stored global plan is not reordered.
fn selected_request(state:&State,slot:u32,menu_open:bool)->Option<(Want,bool)> {
    #[cfg(feature="managed-third-source-alpha")]
    let extra_owned=reservation_for(state,slot).is_some();
    #[cfg(not(feature="managed-third-source-alpha"))]
    let extra_owned=false;
    let supported=|want:Want| !extra_owned || !want_exact(want);
    if let Some(want)=pending_request_for(state,Some(slot),menu_open).filter(|want|supported(*want)) {
        return Some((want,false));
    }
    let settings=crate::runtime::roster::cycle_preload_config::snapshot();
    if menu_open || (!CALLBACK_SLOTS.contains(&slot) && !extra_owned) || settings.slots==0 {
        return None;
    }
    let sources=std::array::from_fn(|i|state.sources[i].filter(|s|state.book.is_current(s.token.epoch())
        && state.book.phase(s.token.slot())!=Ok(Phase::Failed)).map(|s|Want {
            epoch:s.token.epoch(),kind:s.kind,configuration:s.configuration,
            #[cfg(feature="mii-hud-portrait-alpha")] avoid:None}));
    let targets:[Option<Want>;crate::runtime::roster::cycle_preload::MAX_TARGETS]=std::array::from_fn(|i|state.preload[i].filter(|w|i<usize::from(settings.slots)
        && (state.preload_failed|state.failed)&(1u128<<w.kind)==0));
    // Existing native admission validates the backend pair. Created owners
    // are the only eligible slots that cannot hold an exact Mii.
    let targets=crate::engine::preload::cycle_preload_policy::schedulable(targets,!extra_owned,want_exact);
    crate::engine::preload::cycle_preload_policy::missing(targets,sources,slot as usize,
        state.book.can_replace(slot as usize)==Ok(true),state.wants.iter().any(Option::is_some))
        .map(|want|(want,true))
}

#[derive(Clone, Copy)]
pub struct Lease {
    token: LeaseToken,
    pub slot: u32,
    pub kind: u32,
}

/// Three qualified backends share one book. Training keeps parked CPUs 2/3 beside
/// visible 0/1. The waiting backends (exact local wireless and Quickplay, one shared
/// PracticeScene shape) keep witnessed entries 5/6 beside visible 0/4. Arenas and
/// every other slot stay out. Callback dispatch is the union, so a departing owned
/// actor is still serviced after its scene ends.
const CALLBACK_SLOTS: [u32; 4] = [2, 3, 5, 6];

use crate::engine::mii::backend::Layout as Backend;

fn callback_slot(slot: u32) -> bool {
    CALLBACK_SLOTS.contains(&slot) || (cfg!(feature = "managed-third-source-alpha") && slot == 7)
}

fn candidates(backend: Backend) -> [u32; crate::runtime::roster::cycle_preload::MAX_TARGETS] {
    crate::engine::preload::cycle_preload_capacity::slots_with_waiting(backend,
        cfg!(feature="managed-third-source-alpha"),cfg!(feature="managed-ldn-third-source-alpha"),
        cfg!(feature="cycle-preload-extended"),cfg!(feature="lobby-preload-extended"))
}

fn preload_limit(backend: Backend) -> usize {
    let slots = crate::runtime::roster::cycle_preload_config::snapshot().slots;
    #[cfg(feature = "native-body-bank-alpha")]
    let slots = super::body_bank::helper_budget(slots);
    crate::engine::preload::cycle_preload_capacity::target_count_with_waiting(
        slots, backend,
        cfg!(feature="managed-third-source-alpha"),cfg!(feature="managed-ldn-third-source-alpha"),
        cfg!(feature="cycle-preload-extended"),cfg!(feature="lobby-preload-extended"))
}

/// Called on the visible practice frame's sparse planning boundary, before its
/// baseline/settings snapshot. Metrics read this value without native traversal.
pub(crate) unsafe fn refresh_preload_capacity() -> u8 {
    let limit=qualified_backend().map_or(0,preload_limit) as u8;
    STATE.lock().unwrap().preload_limit=limit;
    limit
}

/// Scene-specific created capacity. Quickplay and the additional waiting seats
/// require the lobby candidate; every use still checks the retained native owner.
#[cfg(feature = "managed-third-source-alpha")]
fn third_source_backend(backend: Backend) -> bool {
    backend == Backend::Training
        || (backend == Backend::Ldn && cfg!(feature = "managed-ldn-third-source-alpha"))
        || (backend == Backend::Quickplay && cfg!(feature = "lobby-preload-extended"))
}

#[cfg(feature = "managed-third-source-alpha")]
fn reservation(state: &State) -> Option<capacity::Reservation> {reservation_for(state,7)}

#[cfg(feature = "managed-third-source-alpha")]
fn reservation_for(state:&State,slot:u32)->Option<capacity::Reservation> {
    let index=crate::engine::preload::cycle_preload_capacity::capacity_index(slot as usize)?;
    state.capacities[index].reservation
        .filter(|(epoch,value)|state.book.is_current(*epoch) && !value.entry_destroyed && !value.owner_retired)
        .map(|(_,value)|value)
}

/// Waiting's native helpers 5/6 are not created capacity. Only an exact retained
/// reservation identifies a created entry, including during departure.
fn is_capacity_entry(entry:*const u8,slot:u32)->bool {
    if slot==7 {return cfg!(feature="managed-third-source-alpha");}
    #[cfg(feature="cycle-preload-extended")]
    if let Some(index)=crate::engine::preload::cycle_preload_capacity::capacity_index(slot as usize) {
        return STATE.lock().unwrap().capacities[index].reservation
            .is_some_and(|(_,value)|value.entry==entry as usize);
    }
    let _=entry;
    false
}


unsafe fn usable(entry: *const u8, slot: u32, kind: u32, backend: Backend) -> bool {
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity_entry(entry,slot) {
        let current = reservation_for(&STATE.lock().unwrap(),slot);
        return third_source_backend(backend)
            && current.is_some_and(|value| capacity::ready(value, entry, kind));
    }
    native_source::ready(entry, slot, kind).is_some()
        && (!backend.waiting() || waiting_layout(entry, slot, backend))
}

/// The witnessed waiting shape (manager 0/4/5/6, CPU selection rows) that local
/// wireless and Quickplay share. An extended scene is admitted only through our exact
/// current epoch/reservation; an isolated diagnostic claimant never becomes a
/// managed helper backend.
unsafe fn waiting_layout(entry: *const u8, slot: u32, backend: Backend) -> bool {
    if native_source::exact_waiting_layout(entry, slot) { return true; }
    #[cfg(feature = "managed-ldn-third-source-alpha")]
    if backend == Backend::Ldn || (backend == Backend::Quickplay && cfg!(feature="lobby-preload-extended")) {
        let owner = reservation(&STATE.lock().unwrap());
        if owner.is_some_and(|owner| capacity::owned_ldn_helper(owner, entry, slot)) {
            return true;
        }
    }
    #[cfg(not(feature = "managed-ldn-third-source-alpha"))]
    let _ = backend;
    false
}

/// Initialized melee 0x0d with lobby practice already active, in one of the two
/// waiting families the shared PracticeScene setup builds: local wireless
/// (`0x02010000`) or Quickplay (`0x070101xx`). Arenas and every other family stay out.
fn waiting_backend() -> Option<Backend> {
    let mode = crate::runtime::lobby::game_mode::current()?;
    Backend::scene(mode.initialized, mode.melee, mode.mode, super::lobby::active())
        .filter(|backend| backend.waiting())
}

/// Scene qualification only. The waiting manager and selection table are checked
/// later, on the target's own loader callback, and only when a request needs
/// a new install.
unsafe fn qualified_backend() -> Result<Backend, &'static str> {
    if !cfg!(feature = "managed-sources") {
        return Err("Character resource loading is not available in this scene yet");
    }
    if crate::runtime::compat::common::is_native_training_mode() {
        return if native_source::training_cpu_quantity() == Some(1) {
            Ok(Backend::Training)
        } else {
            Err("Character resources currently need one visible CPU")
        };
    }
    waiting_backend().ok_or("Character resource loading is not available in this scene yet")
}

/// Returns None while the native loader works. A queue entry does not pin an old
/// source; the returned lease does, even before the consumer has been created.
pub unsafe fn prepare(
    channel: usize,
    kind: u32,
    holder: Holder,
) -> Result<Option<Lease>, &'static str> {
    #[cfg(feature = "mii-hud-portrait-alpha")]
    if PORTRAIT_CHANNELS.contains(&channel) {
        return Err("Portrait channels require prepare_portrait");
    }
    prepare_with(channel, kind, holder, None, None)
}

/// Exclusive portrait lease. The configuration is the one the visible owner loaded.
/// Nothing here resolves a different saved identity to fit a helper.
#[cfg(feature = "mii-hud-portrait-alpha")]
pub unsafe fn prepare_portrait(
    channel: usize,
    kind: u32,
    configuration: crate::engine::mii::lifetime::Configuration,
    holder: Holder,
    avoid: Option<u32>,
) -> Result<Option<Lease>, &'static str> {
    if !PORTRAIT_CHANNELS.contains(&channel) {
        return Err("Invalid portrait channel");
    }
    if !(0x48..=0x4a).contains(&kind) || configuration.canonical().class as u32 != kind {
        return Err("Portrait requires the loaded Mii class and its exact configuration");
    }
    prepare_with(channel, kind, holder, Some(configuration), avoid)
}

unsafe fn prepare_with(
    channel: usize,
    kind: u32,
    holder: Holder,
    exact: Option<crate::engine::mii::lifetime::Configuration>,
    avoid: Option<u32>,
) -> Result<Option<Lease>, &'static str> {
    #[cfg(not(feature = "mii-hud-portrait-alpha"))]
    let _ = (exact, avoid);
    if channel >= CHANNELS {
        return Err("Invalid source request");
    }
    let Some(kind_bit) = 1u128
        .checked_shl(kind)
        .filter(|_| (!(0x48..=0x4a).contains(&kind) || cfg!(feature="mii-managed-helper-alpha"))
            && crate::engine::swap::trio::TrioFighter::from_kind(kind).is_some())
    else {
        return Err("This character's resources are unsupported");
    };
    // Unqualified scenes leave requests, channels and counters untouched.
    let backend = qualified_backend()?;
    #[cfg(feature="mii-managed-helper-alpha")]
    {
        // A consumer polls prepare until success/error. Deliver its exact
        // request's refusal before it can resolve/requeue or read native rows.
        let state = STATE.lock().unwrap();
        if let Some(refusal) = state.refusals[channel].as_ref()
            .filter(|(want, _)| want.kind == kind && state.book.is_current(want.epoch)) {
            return Err(refusal.1);
        }
    }
    #[cfg(not(feature="mii-managed-helper-alpha"))]
    let configuration=None;
    #[cfg(feature="mii-managed-helper-alpha")]
    let configuration=if (0x48..=0x4a).contains(&kind) {
        if channel == 2 {return Err("Mii helpers are limited to copy hats");}
        let queued={
            #[cfg(feature = "mii-hud-portrait-alpha")]
            if let Some(c) = exact {
                if !super::mii_managed_helper::admission_ready(true) {
                    return Ok(None);
                }
                let hats = super::mii_ordinary_cpu::helper_origin_hats()
                    .ok_or("Captured helper hat tokens are unavailable")?;
                Some(crate::engine::mii::ordinary_cpu::helper_configuration(c, hats)?)
            } else {
                let state=STATE.lock().unwrap();
                state.wants[channel]
                    .filter(|w|w.kind==kind&&state.book.is_current(w.epoch)).and_then(|w|w.configuration)
            }
            #[cfg(not(feature = "mii-hud-portrait-alpha"))]
            {
                let state=STATE.lock().unwrap();
                state.wants[channel]
                    .filter(|w|w.kind==kind&&state.book.is_current(w.epoch)).and_then(|w|w.configuration)
            }
        };
        Some(match queued {
            Some(c) => c,
            None => {
                // Automatic lobby restoration can arrive before the new
                // lifetime's catalog and immutable role captures. This is the
                // same transient admission gate used by helper installation,
                // not a failed configuration. Keep the consumer pending until
                // capture finishes; resolve real configuration errors afterward.
                if !super::mii_managed_helper::admission_ready(true) {
                    return Ok(None);
                }
                super::mii_managed_helper::configuration(channel, kind)?
            }
        })
    } else {None};
    let wanted = {
        let mut state = STATE.lock().unwrap();
        if state.failed & kind_bit != 0 {
            state.wants[channel] = None;
            return Err("This character's resources failed to load in this practice session");
        }
        let wanted = Want {
            epoch: state.book.epoch(),
            kind,configuration,
            #[cfg(feature = "mii-hud-portrait-alpha")]
            avoid,
        };
        #[cfg(feature="mii-managed-helper-alpha")]
        { state.refusals[channel] = None; }
        state.wants[channel] = Some(wanted);
        wanted
    };
    for slot in candidates(backend).iter().copied().filter(|slot| *slot < 8 && slot_allowed(avoid, *slot)) {
        let helper = slot as usize;
        let ready = {
            let state = STATE.lock().unwrap();
            let Ok(ready) = state.book.ready(helper) else {
                continue;
            };
            if state.book.key(helper).ok().flatten().map(|key| key.kind) != Some(kind)
                || state.sources[helper].is_none_or(|s|s.configuration!=wanted.configuration) {
                continue;
            }
            #[cfg(feature = "mii-hud-portrait-alpha")]
            {
                // A portrait never shares any live lease. Hats and items skip a helper
                // only while its portrait render is still the consumer.
                let portrait = PORTRAIT_CHANNELS.contains(&channel);
                if state.leases.iter().flatten().any(|live| live.source == ready.source()
                    && (portrait || !super::mii_hud_portrait_policy::hat_may_share(
                        matches!(live.consumer, Consumer::Portrait))))
                {
                    continue;
                }
            }
            ready
        };
        // Native readiness can reenter hooks. Hold no bookkeeping mutex here.
        let Some(entry) = native_source::entry(slot) else {
            continue;
        };
        if ready.source().entry() != NativeEntry(entry as u64)
            || !usable(entry, slot, kind, backend)
        {
            continue;
        }
        #[cfg(feature="mii-managed-helper-alpha")]
        if let Some(c)=wanted.configuration {
            if !super::mii_managed_helper::ready(ready,c) {continue;}
        }
        let mut state = STATE.lock().unwrap();
        if state.wants[channel] != Some(wanted)
            || !state.book.is_current(wanted.epoch)
            || state.book.ready(helper) != Ok(ready)
        {
            continue;
        }
        let Some(index) = state.leases.iter().position(Option::is_none) else {
            state.wants[channel] = None;
            return Err("Too many live character items or abilities");
        };
        let token = match state.book.acquire(ready, holder) {
            Ok(token) => token,
            Err(_) => {
                state.wants[channel] = None;
                return Err("Character resource lease could not be acquired");
            }
        };
        state.leases[index] = Some(LiveLease {
            token,
            source: ready.source(),
            consumer: Consumer::Reserved,
            settle_polls: 0,
        });
        state.wants[channel] = None;
        HAS_CONSUMERS.store(true, Ordering::Release);
        return Ok(Some(Lease { token, slot, kind }));
    }
    Ok(None)
}

/// True only for an otherwise serviceable demand blocked by actual leases on
/// every created source in the selected capacity backend. Loading, native busy,
/// failed/unknown ownership and mere queued wants retain the normal timeout.
pub fn waiting_for_capacity(channel: usize) -> bool {
    #[cfg(not(feature = "managed-third-source-alpha"))]
    {
        let _ = channel;
        false
    }
    #[cfg(feature = "managed-third-source-alpha")]
    unsafe {
        if channel >= CHANNELS || admission_blocked() { return false; }
        let Ok(backend) = qualified_backend() else { return false; };
        let slots = candidates(backend);
        if !third_source_backend(backend) { return false; }
        let (wanted, sources) = {
            let state = STATE.lock().unwrap();
            let Some(wanted) = state.wants[channel] else {
                return false;
            };
            if pending_request(&state) != Some(wanted)
                || !capacity::source_kind(wanted.kind)
                || state.failed & (1u128 << wanted.kind) != 0
                || reservation(&state).is_none()
            {
                return false;
            }
            let mut sources = [None; crate::runtime::roster::cycle_preload::MAX_TARGETS];
            for (i, slot) in slots.iter().copied().map(|slot| slot as usize).enumerate() {
                if slot >= state.sources.len() {continue;}
                if !backend.helpers().contains(&(slot as u32))
                    && reservation_for(&state,slot as u32).is_none() {continue;}
                let Some(source) = state.sources[slot] else {
                    return false;
                };
                if source.install.is_some()
                    || !state.book.is_current(source.token.epoch())
                    || state.book.phase(slot) != Ok(Phase::Ready)
                    || state.book.can_replace(slot) != Ok(false)
                {
                    return false;
                }
                sources[i] = Some(source);
            }
            (wanted, sources)
        };
        for (slot, source) in slots.iter().copied().zip(sources)
            .filter_map(|(slot, source)|source.map(|source|(slot,source))) {
            let Some(entry) = native_source::entry(slot) else {
                return false;
            };
            if source.token.entry() != NativeEntry(entry as u64)
                || !usable(entry, slot, source.kind, backend)
            {
                return false;
            }
        }
        if admission_blocked() {
            return false;
        }
        let state = STATE.lock().unwrap();
        state.wants[channel] == Some(wanted)
            && state.book.is_current(wanted.epoch)
            && slots.iter().copied().map(|slot| slot as usize)
                .zip(sources)
                .filter_map(|(slot,source)|source.map(|source|(slot,source)))
                .all(|(slot, source)| {
                    state.sources[slot] == Some(source)
                        && state.book.phase(slot) == Ok(Phase::Ready)
                        && state.book.can_replace(slot) == Ok(false)
                })
    }
}

pub fn cancel(channel: usize) {
    let mut state = STATE.lock().unwrap();
    if let Some(want) = state.wants.get_mut(channel) {
        *want = None;
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    if let Some(refusal) = state.refusals.get_mut(channel) {
        *refusal = None;
    }
    #[cfg(feature = "managed-third-source-alpha")]
    {
        for capacity in &mut state.capacities {capacity.stable=None;capacity.first_load=None;}
    }
}

pub fn release(lease: Lease) {
    let mut state = STATE.lock().unwrap();
    state.book.release(lease.token);
    for live in &mut state.leases {
        if live.is_some_and(|live| live.token == lease.token) { *live = None; }
    }
    HAS_CONSUMERS.store(state.leases.iter().any(Option::is_some), Ordering::Release);
    drop(state);
}

fn retain(lease: Lease, consumer: Consumer) {
    let mut state = STATE.lock().unwrap();
    if let Some(live) = state.leases.iter_mut().flatten().find(|live| live.token == lease.token) {
        live.consumer = consumer;
    }
    drop(state);
}

pub fn retain_hat(lease: Lease, body: u32) {
    retain(lease, Consumer::Hat { body, kind: lease.kind, slot: lease.slot });
}

/// A live hat lease names this Kirby body (its copy source is retained for it).
/// Value receipt for the body-bank departure gate: a Kirby with such a lease is
/// not parked, because a parked copy source lease is unqualified. Takes the
/// source lock briefly; never call while holding the bank lock.
#[cfg(feature = "native-body-bank-alpha")] // Its only caller is the body bank.
pub fn hat_lease_on_body(body: u32) -> bool {
    STATE.lock().unwrap().leases.iter().flatten()
        .any(|live| matches!(live.consumer, Consumer::Hat { body: id, .. } if id == body))
}

pub fn retain_item(lease: Lease, body: u32, kind: i32) {
    retain(lease, Consumer::Item { body, kind });
}

/// The portrait lease stays live until the adapter releases it or the scene departs.
#[cfg(feature = "mii-hud-portrait-alpha")]
pub fn retain_portrait(lease: Lease) {
    retain(lease, Consumer::Portrait);
}

#[cfg(feature = "mii-hud-portrait-alpha")]
pub fn portrait_lease_valid(lease: Lease, holder: Holder) -> bool {
    let state = STATE.lock().unwrap();
    state.book.validate_lease(lease.token, holder)
        && state.leases.iter().flatten().any(|live| live.token == lease.token)
}

#[cfg(feature = "mii-hud-portrait-alpha")]
pub fn portrait_ready(lease: Lease, holder: Holder) -> Option<crate::engine::swap::source_lifetime::ReadyToken> {
    let state = STATE.lock().unwrap();
    if !state.book.validate_lease(lease.token, holder) {
        return None;
    }
    state.book.ready(lease.slot as usize).ok()
}

/// Native copy changes retain earlier kinds' object groups. Copy loss destroys
/// their objects and clears the groups; unused native entries have key -1.
/// Read only from a freshly resolved Kirby, and retain on an unknown layout.
/// The bounds cover the roster and observed native records without allowing an
/// unexpected count to turn a frame callback into an unbounded memory walk.
pub(crate) unsafe fn retained_copy_kinds(body: *mut app::BattleObjectModuleAccessor) -> Option<u128> {
    let context = WorkModule::get_int64(body, 0x10000106) as *const u8;
    if !aligned(context) { return None; }
    let container = read::<*const u8>(context, 0x258);
    if !aligned(container) { return None; }
    let count = usize::try_from(read::<i32>(container, 0x10)).ok()?;
    if count > 128 { return None; }
    let entries = read::<*const u8>(container, 0x18);
    if count != 0 && !aligned(entries) { return None; }
    let mut kinds = 0u128;
    for index in 0..count {
        let entry = entries.add(index * 0x10);
        let key = read::<i32>(entry, 0);
        let group = read::<*const u8>(entry, 8);
        if key == -1 && group.is_null() { continue; }
        let bit = if key == -1 { 0 } else {
            1u128.checked_shl(u32::try_from(key).ok()?)?
        };
        if !aligned(group) { return None; }
        let count = usize::try_from(read::<i32>(group, 0x10)).ok()?;
        if count > 32 { return None; }
        let records = read::<*const u8>(group, 0x18);
        if count != 0 && !aligned(records) { return None; }
        for record in 0..count {
            let objects = read::<i64>(records, record * 0x348 + 0x18);
            if objects < 0 { return None; }
            if key == -1 && objects != 0 { return None; }
            if objects != 0 { kinds |= bit; break; }
        }
    }
    Some(kinds)
}

/// Called after Kirby's original frame callback, while every old source is still
/// leased. Use native full copy loss (the silent KO variant), including its
/// kind-specific cleanup, instead of clearing fields or just changing the hat.
/// This candidate requires the source-reuse/KO runtime regression before release.
pub unsafe fn clear_hat_for_switch(body: *mut app::BattleObjectModuleAccessor)
    -> Result<(), &'static str>
{
    if !WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY) {
        return if retained_copy_kinds(body) == Some(0) { Ok(()) }
            else { Err("Kirby's previous copy is still being cleared") };
    }
    let Some(before) = retained_copy_kinds(body) else {
        return Err("Kirby's previous copy resources are not ready");
    };
    let object = crate::runtime::compat::common::try_get_battle_object((*body).battle_object_id)
        .filter(|object| object.module_accessor == body)
        .ok_or("Kirby is still changing characters")?;
    let base = super::BASE.load(Ordering::Acquire);
    // Seam row `sources.kirby_copy_clear` (Called). The `character_sources` group is authoritative (SEAMS-ALL):
    // the row decides; the 4-word check is the shadow compare. One check per user request, not per frame.
    if base == 0 || !crate::runtime::seams::seam_install::site("sources.kirby_copy_clear",
        std::slice::from_raw_parts((base + 0xb96770) as *const u32, 4)
            == [0xd10243ff, 0xa9036ffc, 0xa90467fa, 0xa9055ff8])
    { return Err("Native Kirby copy cleanup is unavailable for this game build"); }
    let clear: unsafe extern "C" fn(*mut app::Fighter, i32) =
        std::mem::transmute(base + 0xb96770);
    // The P1 sweep can run later in this same frame. Pin through that sweep and
    // the next one if this request is cancelled before its next Kirby callback.
    {
        let mut state = STATE.lock().unwrap();
        for live in state.leases.iter_mut().flatten() {
            if matches!(live.consumer, Consumer::Hat { body: id, .. }
                if id == (*body).battle_object_id) { live.settle_polls = 2; }
        }
    }
    clear(object as *const app::BattleObject as *mut app::Fighter, 0);
    let after = retained_copy_kinds(body);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[KirbyHats] native cleanup body=0x{:x} groups_before=0x{:x} groups_after={:?}",
        (*body).battle_object_id, before, after));
    if WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY)
        || after != Some(0)
    { return Err("Kirby's previous copy is still being cleared"); }
    Ok(())
}

/// Recheck on a later Kirby callback before preparing a replacement. Native
/// cleanup has returned and had a frame to settle; no new copy owns these leases.
/// `leases_released` belongs to the same unstarted pending request/body.
/// A waiting request has not acquired its replacement hat lease.
pub unsafe fn finish_hat_clear(body: *mut app::BattleObjectModuleAccessor, leases_released: bool)
    -> Result<(), &'static str>
{
    if WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY)
        || retained_copy_kinds(body) != Some(0)
    { return Err("Kirby's previous copy is still being cleared"); }
    // Native copy state must still be validated before skipping completed work.
    if leases_released { return Ok(()); }
    let mut state = STATE.lock().unwrap();
    let mut released = 0;
    for index in 0..LEASES {
        let Some(live) = state.leases[index] else { continue; };
        if matches!(live.consumer, Consumer::Hat { body: id, .. }
            if id == (*body).battle_object_id)
        {
            state.book.release(live.token);
            state.leases[index] = None;
            released += 1;
        }
    }
    HAS_CONSUMERS.store(state.leases.iter().any(Option::is_some), Ordering::Release);
    if released != 0 {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[KirbyHats] cleanup settled body=0x{:x} released={}",
            (*body).battle_object_id, released));
    }
    Ok(())
}

// At most one group walk per P1/CPU Kirby in this callback, even when several
// old hats retain leases. No pointer or observation survives the callback.
type HatObservation = Option<(u32, Option<u128>)>;

unsafe fn consumer_alive(consumer: Consumer, hats: &mut [HatObservation; 2]) -> bool {
    let (id, category) = match consumer {
        Consumer::Reserved => return true,
        // Released by the portrait adapter after the queue drains, or by scene departure.
        #[cfg(feature = "mii-hud-portrait-alpha")]
        Consumer::Portrait => return true,
        Consumer::Hat { body, .. } => (body, *BATTLE_OBJECT_CATEGORY_FIGHTER),
        Consumer::Item { body, .. } => (body, *BATTLE_OBJECT_CATEGORY_ITEM),
    };
    if !sv_battle_object::is_active(id) { return false; }
    let body = sv_battle_object::module_accessor(id);
    if body.is_null() || app::utility::get_category(&mut *body) != category { return false; }
    match consumer {
        Consumer::Reserved => true,
        #[cfg(feature = "mii-hud-portrait-alpha")]
        Consumer::Portrait => true,
        Consumer::Item { kind, .. } => app::utility::get_kind(&mut *body) == kind,
        Consumer::Hat { kind, slot, .. } => {
            if app::utility::get_kind(&mut *body) != *FIGHTER_KIND_KIRBY { return false; }
            // Keep both old and new sources across native copy reconstruction.
            if WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY_ON_START)
                || (WorkModule::is_flag(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_FLAG_COPY)
                    && WorkModule::get_int(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA) as u32 == kind
                    && WorkModule::get_int(body, *FIGHTER_KIRBY_INSTANCE_WORK_ID_INT_COPY_CHARA_SLOT_NO) as u32 == slot)
            { return true; }
            let observed = if let Some((_, kinds)) = hats.iter().flatten().find(|(seen, _)| *seen == id) {
                *kinds
            } else if let Some(observation) = hats.iter_mut().find(|seen| seen.is_none()) {
                let kinds = retained_copy_kinds(body);
                *observation = Some((id, kinds));
                kinds
            } else {
                None // Another live body is departing; wait for native destruction.
            };
            observed.is_none_or(|kinds| kinds & (1u128 << kind) != 0)
        }
    }
}

/// Once on the current P1 frame, not once per helper or per fighter. Thrown
/// items retain their lease until their actual native item object ends.
pub unsafe fn on_frame(body: *mut app::BattleObjectModuleAccessor) {
    if !HAS_CONSUMERS.load(Ordering::Acquire) { return; }
    if crate::runtime::compat::common::try_get_module_accessor(crate::runtime::compat::common::consts::FighterId::Player)
        != Some(body) { return; }
    let mut state = STATE.lock().unwrap();
    let mut hats = [None; 2];
    for index in 0..LEASES {
        let Some(mut live) = state.leases[index] else { continue; };
        if live.settle_polls != 0 {
            live.settle_polls -= 1;
            state.leases[index] = Some(live);
            continue;
        }
        if !consumer_alive(live.consumer, &mut hats) {
            state.book.release(live.token);
            state.leases[index] = None;
        }
    }
    HAS_CONSUMERS.store(state.leases.iter().any(Option::is_some), Ordering::Release);
    drop(state);
}

pub fn scene_changed() {
    #[cfg(feature="mii-managed-helper-alpha")]
    super::mii_managed_helper::depart();
    // LOG-REFUSED: refusal lines are bounded per scene.
    *MEMORY_REFUSED.lock().unwrap() = [[0; 4]; 8];
    let mut state = STATE.lock().unwrap();
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let mut registry = coexist::registry();
    state.wants = [None; CHANNELS];
    state.preload = [None; crate::runtime::roster::cycle_preload::MAX_TARGETS];
    state.preload_failed = 0;
    state.preload_settled = false;
    state.preload_limit = 0;
    #[cfg(feature="mii-managed-helper-alpha")]
    { state.refusals = [None; CHANNELS]; }
    state.leases = [None; LEASES];
    state.failed = 0;
    #[cfg(feature = "managed-third-source-alpha")]
    {
        for capacity in &mut state.capacities {capacity.stable=None;capacity.attempted=None;capacity.first_load=None;}
        // Keep the old reservation until both actual native boundaries arrive.
        state.creation_hold = None;
    }
    state
        .book
        .depart_scene()
        .expect("native scene epoch exhausted");
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    {
        for slot in registry.helpers.layout.helpers().map(|id|id as usize) {
            let helper = state.sources[slot].map_or(coexist::helpers::Helper::Unmanaged,
                |source| coexist::helpers::Helper::Unavailable { token: source.token, phase: Phase::Departed });
            registry.source(state.book.epoch(), slot, helper);
        }
        drop(registry);
    }
    HAS_CONSUMERS.store(false, Ordering::Release);
    // In-flight native loads stay owned until their callback completes or the
    // real native destructor runs. No old intent is published after departure.
    drop(state);
    crate::runtime::roster::cycle_preload::scene_end();
}

/// Preserve the old probe's Training owner2 automation suppression independently
/// of its diagnostics. This is not resource admission or native source ownership.
#[cfg(any(feature = "source-owner-probe", feature = "managed-sources"))]
#[cfg(feature = "source-owner-probe")]
pub unsafe fn legacy_training_owner_reserved(body: &mut app::BattleObjectModuleAccessor) -> bool {
    crate::runtime::compat::common::is_native_training_mode()
        && app::utility::get_category(body) == *BATTLE_OBJECT_CATEGORY_FIGHTER
        && WorkModule::get_int(body, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) == 2
}

/// Finish a load this entry already owns. True means this callback must not
/// start another install. Pending Mii completion reaches the normal tick commit;
/// a SourceBook complete() failure keeps the previous tick copy.
unsafe fn service_owned(_ctx: &ServiceEntryContext, entry: *mut u8, slot: u32) -> bool {
    let is_capacity=is_capacity_entry(entry,slot);
    let index = slot as usize;
    let (original, departed) = {
        let state = STATE.lock().unwrap();
        let Some(source) = state.sources[index] else {
            return false;
        };
        if source.token.entry() != NativeEntry(entry as u64) {
            return true;
        }
        if source.install.is_none() {
            return state.book.phase(index) == Ok(Phase::Departed);
        }
        (source, !state.book.is_current(source.token.epoch()))
    };
    #[cfg(feature="mii-managed-helper-alpha")]
    if original.configuration.is_some()
        && super::mii_managed_helper::retained_terminal(original.token) {
        return true;
    }
    let mut source = original;
    source.ticks = source.ticks.saturating_add(1);
    if let Some(stamp)=source.measurement.as_mut() {stamp.frame(false);}
    let native_done = native_source::parked_ready(entry, slot, source.kind);
    #[cfg(feature = "managed-third-source-alpha")]
    let capacity_observation = if is_capacity {
        let state = STATE.lock().unwrap();
        (
            reservation_for(&state,slot),
            state
                .capacities[capacity_index(slot)]
                .load
                .filter(|(token, _)| *token == source.token)
                .map(|(_, before)| before),
        )
    } else {
        (None, None)
    };
    #[cfg(feature = "managed-third-source-alpha")]
    let complete = if is_capacity {
        // Scene departure never releases a pending created-entry load: full
        // acceptance or its actual destructor does. A load native has completed
        // whose proof never holds ends at `source_load::verdict` (FAIL 2).
        !departed
            && native_done
            && capacity_observation
                .0
                .zip(capacity_observation.1)
                .is_some_and(|(owner, before)| {
                    capacity::loaded(_ctx, entry, owner, before, source.kind)
                })
    } else {
        native_done && (departed || native_source::ready(entry, slot, source.kind).is_some())
    };
    #[cfg(not(feature = "managed-third-source-alpha"))]
    let complete =
        native_done && (departed || native_source::ready(entry, slot, source.kind).is_some());
    #[cfg(feature = "managed-third-source-alpha")]
    let stalled = if is_capacity && source.ticks == STALL_TICKS {
        Some(capacity::stalled_checks(
            _ctx,
            entry,
            capacity_observation.0,
            capacity_observation.1,
            source.kind,
        ))
    } else {
        None
    };
    // FAIL 2: on a created entry, native completion is the entry's own actor,
    // control and archive readiness beside `native_done`. Read only at the bound.
    #[cfg(feature = "managed-third-source-alpha")]
    let entry_ready = is_capacity && native_done && source.ticks >= STALL_TICKS
        && capacity_observation.0.is_some_and(|owner| capacity::ready(owner, entry, source.kind));
    #[cfg(not(feature = "managed-third-source-alpha"))]
    let entry_ready = false;
    #[cfg(feature = "managed-grouped-source-alpha")]
    let ready_actors = (is_capacity && complete).then(|| capacity::actors_at(entry, slot, source.kind)).flatten();
    #[cfg(feature = "managed-grouped-source-alpha")]
    let complete = complete && (!is_capacity || ready_actors.is_some());
    #[cfg(feature="mii-managed-helper-alpha")]
    let mii_proof=if source.configuration.is_some() && complete && !departed {
        match super::mii_managed_helper::completed(source.token,source.ticks) {
            Ok(proof)=>proof,Err(reason)=>{
                super::mii_managed_helper::refuse_completion(source.token,reason);
                return true;
            }
        }
    } else {None};
    #[cfg(feature="mii-managed-helper-alpha")]
    let complete=complete && (source.configuration.is_none() || mii_proof.is_some());
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let proof = if complete && !departed {
        coexist::helpers::observe(entry as usize, slot).filter(|proof| {
            #[cfg(feature="mii-managed-helper-alpha")]
            if mii_proof.is_some() {
                return source.original.is_some_and(|old|coexist::helpers::OriginalRow {
                    index:proof.original.index,..old}==proof.original);
            }
            Some(proof.original)==source.original
        })
    } else { None };
    #[cfg(feature="mii-managed-helper-alpha")]
    if mii_proof.is_some() && proof.is_none() {
        super::mii_managed_helper::refuse_completion(source.token,"managed original row/provenance changed at completion");
        return true;
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    let complete=complete && (source.configuration.is_none() || proof.is_some());
    let mut state = STATE.lock().unwrap();
    if state.sources[index] != Some(original) {
        return true;
    }
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    if !source.install.is_some_and(|token| INSTALL.owns(token)) { return true; }
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let mut registry = (!is_capacity && (complete || (native_done && source.ticks >= STALL_TICKS && !departed)))
        .then(coexist::registry);
    let departed = !state.book.is_current(source.token.epoch());
    let mut finished = None;
    let mut event = None;
    if complete && (!cfg!(feature = "managed-third-source-alpha") || !is_capacity || !departed)
        && (source.configuration.is_none() || !departed) {
        if !departed {
            if state.book.complete(source.load).is_err() {
                return true;
            }
            event = Some("ready");
        }
        finished = source.install.take();
    } else {
        // One rule for both helper kinds: native completed and the managed proof
        // still fails at the bound -> fail the load and release INSTALL; native
        // not completed -> keep the token and report once (FAIL 2).
        let unfinished = Unfinished {
            ticks: source.ticks, native_done, entry_ready, departed,
            created: is_capacity, exact_mii: source.configuration.is_some(),
        };
        match verdict(unfinished) {
            Verdict::Fail => {
                if state.book.fail(source.load).is_ok() {
                    finished = source.install.take();
                    // A native helper without usable resources retires the kind for the
                    // scene; a created entry's failed scene proof fails only this attempt,
                    // and the planner retries the kind (`Unfinished::fails_kind`).
                    if unfinished.fails_kind() {
                        state.failed |= 1u128 << source.kind;
                        for want in &mut state.wants {
                            if want.is_some_and(|want| want.kind == source.kind) {
                                *want = None;
                            }
                        }
                    }
                    event = Some(if is_capacity { STALL_FAILED } else { NATIVE_WITHOUT_RESOURCES });
                }
            }
            Verdict::Retain => event = Some(STALL_RETAINED),
            Verdict::Wait => {}
        }
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    if complete && !departed && source.configuration.is_some() {
        // Only a completed selector transaction can advance this managed row's index anchor.
        // The visible roles' immutable saved-original snapshots remain untouched.
        source.original=proof.map(|p|p.original);
    }
    state.sources[index] = Some(source);
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    {
        if let Some(registry) = registry.as_mut() {
            let helper = match (state.book.ready(index), state.book.key(index), proof) {
                #[cfg(feature="mii-managed-helper-alpha")]
                (Ok(ready),Ok(Some(key)),Some(proof))
                    if !departed && ready.source()==source.token && proof.kind==key.kind
                        && proof.costume==key.costume && mii_proof.is_some() =>
                    coexist::helpers::Helper::Ready {ready,key,proof},
                (Ok(ready), Ok(Some(key)), Some(proof))
                    if !departed && ready.source() == source.token && proof.kind == key.kind
                        && proof.costume == key.costume && !(0x48..=0x4a).contains(&key.kind) =>
                    coexist::helpers::Helper::Ready { ready, key, proof },
                _ => coexist::helpers::Helper::Unavailable { token: source.token,
                    phase: state.book.phase(index).expect("registered source slot") },
            };
            registry.source(state.book.epoch(), index, helper);
            #[cfg(feature="mii-managed-helper-alpha")]
            if matches!(helper,coexist::helpers::Helper::Ready {..}) {
                if let Some(ordinal)=registry.helpers.ordinal(index) {registry.helpers.miis[ordinal]=mii_proof;}
            }
        }
        drop(registry);
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    if finished.is_some() && source.configuration.is_some() {
        super::mii_managed_helper::release(source.token);
    }
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity && finished.is_some() {
        state.capacities[capacity_index(slot)].load = None;
        if event == Some("ready") {
            #[cfg(feature = "managed-grouped-source-alpha")]
            let count = ready_actors.map_or(0, |actors| actors.count());
            #[cfg(not(feature = "managed-grouped-source-alpha"))]
            let count = 1;
            state.capacities[capacity_index(slot)].observation.ready_actors =
                state.capacities[capacity_index(slot)].observation.ready_actors.saturating_add(count);
            #[cfg(feature = "managed-grouped-source-alpha")]
            { state.capacities[capacity_index(slot)].observation.current_actors = ready_actors; }
        }
    }
    drop(state);
    #[cfg(feature = "managed-grouped-source-alpha")]
    if is_capacity && event == Some("ready") {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[ManagedSourceCleanup] canonical ready source={:?} kind=0x{:x} actors={:x?}",
            source.token, source.kind, ready_actors));
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    if event==Some("ready") {
        if let Some(proof)=mii_proof {
            crate::engine::compat::logging::diagnostic(format_args!("[MiiHelper] exact parked completion source={:?} proof={:?}",source.token,proof));
        }
    }
    if let Some(token) = finished {
        finish_install(token, "managed source completion");
        if let Some(stamp)=source.measurement {
            stamp.finish(u32::from(source.ticks),if departed {crate::engine::preload::preload_metrics::Outcome::Departed}
                else if event==Some("ready") {crate::engine::preload::preload_metrics::Outcome::Ready}
                else {crate::engine::preload::preload_metrics::Outcome::Failed},metrics());
        }
        note_preload_settled();
        if event == Some("ready") {
            log_helper_memory(slot, source.kind, source.memory_committed);
        }
    }
    if let Some(event) = event {
        // The bound's checks, with what differed from the commit-time scene and input rows
        // (FAIL 2: names the other owner's change), then the verdict.
        #[cfg(feature = "managed-third-source-alpha")]
        if let Some(checks) = stalled.filter(|_| matches!(event, STALL_RETAINED | STALL_FAILED)) {
            crate::engine::compat::logging::diagnostic(format_args!(
                "[CharacterSources] stalled slot={} kind=0x{:x} ticks={} reservation={} preflight={} owned={} snapshot={:?} scene={:?} original_resources={:?} inputs={:?} ready={:?} native_done={} departed={} entry_ready={} scene_diff_hex={:x?} input_rows_hex={:x?} input_fields_equal={:?} changed_rows={:x?}; {}",
                slot, source.kind, source.ticks, checks.reservation, checks.preflight, checks.owned,
                checks.snapshot, checks.scene, checks.original_resources, checks.inputs,
                checks.ready, native_done, departed, entry_ready, checks.scene_diff, checks.input_rows,
                checks.input_fields_equal, checks.changed_rows,
                event.strip_prefix("stalled; ").unwrap_or(event)));
            return true;
        }
        crate::engine::compat::logging::diagnostic(format_args!(
            "[CharacterSources] {} slot={} kind=0x{:x} ticks={}",
            event, slot, source.kind, source.ticks
        ));
    }
    true
}

/// Only a proven configuration mismatch retires an unsubmitted exact intent.
/// Keep a per-channel receipt so its consumer observes Err instead of requeueing.
#[cfg(feature="mii-managed-helper-alpha")]
fn refuse_want(wanted: Want, slot: u32, reason: &'static str) {
    let mut state = STATE.lock().unwrap();
    let mut refused = false;
    for channel in 0..CHANNELS {
        if state.wants[channel] == Some(wanted) {
            state.wants[channel] = None;
            state.refusals[channel] = Some((wanted, reason));
            refused = true;
        }
    }
    drop(state);
    if refused {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[MiiHelper] admission refused slot={} epoch={:?} kind={:#x} reason={}",
            slot, wanted.epoch, wanted.kind, reason));
    }
}

fn slot_allowed(avoid: Option<u32>, slot: u32) -> bool {
    #[cfg(feature = "mii-hud-portrait-alpha")]
    { super::mii_hud_portrait_policy::slot_may_install(avoid, slot) }
    #[cfg(not(feature = "mii-hud-portrait-alpha"))]
    { let _ = (avoid, slot); true }
}

#[cfg(not(feature = "mii-hud-portrait-alpha"))]
fn pending_request_for(state: &State, slot: Option<u32>, menu_open: bool) -> Option<Want> {
    let _ = slot;
    if menu_open { None } else { pending_request(state) }
}

#[cfg(not(feature = "mii-hud-portrait-alpha"))]
fn pending_request(state: &State) -> Option<Want> {
    state.wants.iter().flatten().copied().find(|want| {
        state.book.is_current(want.epoch) && !state.sources.iter().flatten().any(|source|
            source.kind == want.kind && source.configuration==want.configuration
                && state.book.is_current(source.token.epoch())
                && state.book.phase(source.token.slot()) != Ok(Phase::Failed))
    })
}

#[cfg(all(feature = "mii-hud-portrait-alpha", feature = "managed-third-source-alpha"))]
fn pending_request(state: &State) -> Option<Want> {
    pending_request_for(state, None, false)
}

/// The first unsatisfied want this slot may install. An avoided slot neither
/// satisfies the want nor installs it. `slot == None` is the previous global walk.
#[cfg(feature = "mii-hud-portrait-alpha")]
fn pending_request_for(state: &State, slot: Option<u32>, menu_open: bool) -> Option<Want> {
    // Exact portrait requests depend on the loaded fighter, not menu preferences.
    // Hats/items still wait for menu close, even when a hat carries a Mii payload.
    // Filter before finding a want so a pending hat cannot hide a portrait.
    state.wants.iter().enumerate().filter_map(|(channel, want)| {
        want.filter(|want| super::mii_hud_portrait_policy::source_channel_admitted(
            channel, want.configuration.is_some(), menu_open))
    }).find(|want| {
        slot.is_none_or(|slot| super::mii_hud_portrait_policy::slot_may_install(want.avoid, slot))
            && state.book.is_current(want.epoch) && !state.sources.iter().flatten().any(|source|
            source.kind == want.kind && source.configuration==want.configuration
                && super::mii_hud_portrait_policy::slot_may_install(want.avoid, source.token.slot() as u32)
                && state.book.is_current(source.token.epoch())
                && state.book.phase(source.token.slot()) != Ok(Phase::Failed))
    })
}

#[cfg(not(feature = "mii-hud-portrait-alpha"))]
fn ready_still_requested(state: &State, index: usize) -> bool {
    state.book.phase(index) == Ok(Phase::Ready) && state.sources[index].is_some_and(|source|
        state.wants.iter().flatten().any(|want| want.kind == source.kind && want.configuration==source.configuration))
}

#[cfg(feature = "mii-hud-portrait-alpha")]
fn ready_still_requested(state: &State, index: usize) -> bool {
    state.book.phase(index) == Ok(Phase::Ready) && state.sources[index].is_some_and(|source|
        state.wants.iter().flatten().any(|want|
            super::mii_hud_portrait_policy::slot_may_install(want.avoid, index as u32)
                && want.kind == source.kind && want.configuration==source.configuration))
}

/// Hard gates apply to every source. Only an exact portrait may pass the
/// separate mod-menu gate; it never relaxes native UI, save/preview ownership,
/// native readiness, or the single installer. Also used by the portrait wait
/// budget so deliberately deferred acquisition cannot expire during playback.
unsafe fn admission_blocker() -> Option<&'static str> {
    if INSTALL.active() { Some("another source install") }
    else if !crate::runtime::compat::common::is_ready_go() { Some("ready-go") }
    else if super::lobby::native_ui_busy() { Some("native UI") }
    else if super::training::save_states::is_loading() { Some("save-state restore") }
    else if super::training::recording_or_playback() { Some("recording or playback") }
    else if crate::runtime::compat::replay_preview::occupied() { Some("replay preview") }
    else { None }
}

#[cfg(feature = "mii-hud-portrait-alpha")]
pub unsafe fn portrait_admission_blocker() -> Option<&'static str> {
    admission_blocker()
}

#[cfg(feature = "managed-third-source-alpha")]
unsafe fn admission_blocked() -> bool {
    admission_blocker().is_some() || super::menu::captures_input()
}

unsafe fn visible_ready(backend: Backend) -> bool {
    let [left, right] = backend.visible();
    let Some(first) = native_source::entry(left) else { return false; };
    let Some(second) = native_source::entry(right) else { return false; };
    if backend.waiting() && first == second { return false; }
    ready_forms(first, left).is_some() && ready_forms(second, right).is_some()
}

/// Training's first install still requires outer 0, guard 0, this entry, and
/// visible 0/1. Its later installs require parked_ready of the managed kind.
/// A waiting first install also requires loader 14 and parked_ready of descriptor
/// +0x18. Every new waiting install requires distinct ready 0/4 and the witnessed
/// manager shape before any selection-table read.
unsafe fn native_admission(
    entry: *const u8, slot: u32, backend: Backend, managed: Option<u32>,
) -> bool {
    if let Some(kind) = managed {
        if !native_source::parked_ready(entry, slot, kind) { return false; }
        if backend.waiting() && !visible_ready(backend) { return false; }
    } else {
        if read::<u8>(entry, 0x5920) != 0 || read::<u8>(entry, 0x5935) != 0 { return false; }
        if backend.waiting() && read::<u8>(entry, 0x591c) != 14 { return false; }
        if native_source::entry(slot) != Some(entry) { return false; }
        if !visible_ready(backend) { return false; }
        if backend.waiting()
            && !native_source::parked_ready(entry, slot, read::<u32>(entry, 0x18))
        { return false; }
    }
    !backend.waiting() || waiting_layout(entry, slot, backend)
}

#[cfg(feature = "managed-third-source-alpha")]
fn helpers_unavailable(state: &State, backend: Backend) -> bool {
    backend.helpers().iter().copied().all(|slot| {
        let index = slot as usize;
        ready_still_requested(state, index) || state.book.can_replace(index) != Ok(true)
    })
}

#[cfg(feature="managed-third-source-alpha")]
#[derive(Clone,Copy,PartialEq,Eq)]
struct CreationIntent {extra:u32,wanted:Want,speculative:bool}

/// One shared admission policy for every additional helper. A requested cache
/// count never makes an unowned native entry usable.
#[cfg(feature="managed-third-source-alpha")]
fn creation_intent(state:&State,backend:Backend,speculative:bool)->Option<CreationIntent> {
    use crate::engine::preload::cycle_preload_capacity as policy;
    let settings=crate::runtime::roster::cycle_preload_config::snapshot();
    let slots = settings.slots;
    #[cfg(feature = "native-body-bank-alpha")]
    let slots = super::body_bank::helper_budget(slots);
    let limit=policy::target_count_with_waiting(slots,backend,true,
        cfg!(feature="managed-ldn-third-source-alpha"),cfg!(feature="cycle-preload-extended"),cfg!(feature="lobby-preload-extended"));
    if !speculative {
        // The preload candidate must preserve the original two-helper behavior
        // when extra capacity was not requested, including the Off baseline.
        if cfg!(feature="cycle-preload-extended") && limit<=policy::BASE_HELPERS {return None;}
        let wanted=pending_request(state)?;
        let extra=policy::extra_slots(backend)[0] as u32;
        let owner=&state.capacities[capacity_index(extra)];
        return (third_source_backend(backend) && helpers_unavailable(state,backend)
            && owner.reservation.is_none() && owner.attempted!=Some(wanted.epoch)
            && !crate::engine::preload::cycle_preload_policy::exact_mii(wanted.kind,wanted.configuration.is_some())
            && capacity::source_kind(wanted.kind))
            .then_some(CreationIntent {extra,wanted,speculative});
    }
    if state.wants.iter().any(Option::is_some) {return None;}
    let (position,extra)=policy::extra_slots(backend).iter().copied().enumerate()
        .take(limit.saturating_sub(backend.helpers().len()))
        .find(|(_,slot)|state.capacities[capacity_index(*slot as u32)].reservation.is_none())?;
    let owner=&state.capacities[capacity_index(extra as u32)];
    if owner.attempted==Some(state.book.epoch()) {return None;}
    // Existing helpers must first be filled or actively leased. Never create a
    // new native owner merely because an existing free helper has not run yet.
    for slot in candidates(backend).iter().copied().take(backend.helpers().len()+position) {
        let source=state.sources[slot as usize]?;
        if !state.book.is_current(source.token.epoch()) || state.book.phase(slot as usize)!=Ok(Phase::Ready) {return None;}
        let targeted=state.preload.iter().flatten().any(|want|
            want.kind==source.kind && want.configuration==source.configuration);
        if !targeted && state.book.can_replace(slot as usize)==Ok(true) {return None;}
    }
    // An extra is created for a missing ordinary target. Exact Miis stay on the native pair.
    let wanted=state.preload.iter().copied().flatten().find(|want|
        !crate::engine::preload::cycle_preload_policy::exact_mii(want.kind,want.configuration.is_some())
        && capacity::source_kind(want.kind) && !state.sources.iter().enumerate().any(|(slot,source)|
            source.is_some_and(|source|source.kind==want.kind && source.configuration==want.configuration
                && state.book.is_current(source.token.epoch()) && state.book.phase(slot)!=Ok(Phase::Failed))))?;
    Some(CreationIntent {extra:extra as u32,wanted,speculative})
}

/// Retain the native claim until real retirement, including a failed post-call
/// qualification. Each slot has one attempt per scene and serializes via INSTALL.
#[cfg(feature="managed-third-source-alpha")]
unsafe fn create_capacity(ctx:&ServiceEntryContext,entry:*const u8,slot:u32,intent:CreationIntent) {
    let grace = if intent.speculative {
        let Some(generation) = crate::runtime::roster::cycle_preload::speculative_admission() else { return; };
        Some(generation)
    } else { None };
    let Ok(backend)=qualified_backend() else {return;};
    let helpers=backend.helpers();
    if !helpers.contains(&slot) || !candidates(backend).contains(&intent.extra)
        || admission_blocked() {return;}
    let index=capacity_index(intent.extra);
    let wanted=intent.wanted;
    {let state=STATE.lock().unwrap();
     if creation_intent(&state,backend,intent.speculative)!=Some(intent) {return;}}
    let before=match capacity::preflight_checked(ctx,entry,slot,intent.extra) {
        Ok(before)=>before,
        Err(refusal)=>{
            let first={
                let mut state=STATE.lock().unwrap();
                state.capacities[index].stable=None;
                // lobby-preload D: the readiness line counts what the scene holds while this stands.
                let position=crate::engine::preload::cycle_preload_capacity::extra_slots(backend).iter()
                    .position(|extra|*extra as u32==intent.extra).unwrap_or(0);
                state.creation_hold=Some((wanted.epoch,position as u8));
                state.capacities[index].waiting.noted(wanted.epoch,refusal)
            };
            if first {creation_waiting(intent,slot,refusal);}
            return;
        }
    };
    let (stable,note)={
        let mut state=STATE.lock().unwrap();
        if creation_intent(&state,backend,intent.speculative)!=Some(intent) {
            state.capacities[index].stable=None;return;
        }
        state.creation_hold=None;
        let owner=&mut state.capacities[index];
        let restarted=owner.stable.is_some_and(|(old,observed,_,_)|old==wanted && observed!=before);
        let (ticks,seen)=owner.stable.filter(|(old,observed,_,_)|*old==wanted && *observed==before)
            .map_or((1,0),|(_,_,ticks,seen)|(ticks.saturating_add(1),seen));
        let seen=seen|u8::from(slot==helpers[0])|(u8::from(slot==helpers[1])<<1);
        owner.stable=Some((wanted,before,ticks,seen));
        // lobby-preload D: a preflight that passes but never stays stable, or is seen from one
        // helper only, is the other silent wait; noted once per scene, never a refusal.
        let note=if restarted && owner.waiting.restarted(wanted.epoch) {Some(Refusal::Unstable)}
            else if backend.waiting() && seen!=3 && ticks==creation::ONE_HELPER_TICKS {Some(Refusal::OneHelper)}
            else {None};
        (ticks>=30 && (!backend.waiting() || seen==3),
            note.filter(|refusal|owner.waiting.noted(wanted.epoch,*refusal)))
    };
    if let Some(refusal)=note {creation_waiting(intent,slot,refusal);}
    if !stable {return;}
    // GS-PANIC (`process_memory`): a helper owner created for a speculative target exists only to host that load,
    // so it is admitted with that load's measured cost; a creation for a demand load is not refused here.
    if intent.speculative {
        let cost = crate::engine::swap::body_bank::memory::helper_cost(wanted.kind);
        let (decision, _) = super::process_memory::admit(cost);
        if decision.admission != crate::engine::swap::body_bank::memory::Admission::Admit {
            memory_refused(intent.extra, wanted.kind, decision, cost, "capacity creation");
            return;
        }
    }
    let Some(install)=INSTALL.try_begin(false) else {return;};
    {
        let mut state=STATE.lock().unwrap();
        if creation_intent(&state,backend,intent.speculative)!=Some(intent)
            || grace.is_some_and(|generation| !crate::runtime::roster::cycle_preload::speculative_admission_current(generation)) {
            drop(state);finish_install(install,"capacity intent changed");return;
        }
        // Creating an extra owner does not consume the speculative install opportunity.
        state.capacities[index].attempted=Some(wanted.epoch);
        state.capacities[index].stable=None;
    }
    let created=capacity::create_reserved(ctx,entry,slot,before);
    // lobby-preload A, the R-LP-1 safety evidence: every visible exact Mii the preflight proved is
    // proved again after the native call (same committed authority, descriptor and record).
    let visible_mii_kept=if created.is_some() {capacity::visible_mii_kept(&before)} else {[None;2]};
    // lobby-preload B, the R-LP-B1 safety evidence: every visible body-bank window admitted by its
    // committed bank is captured again after the native call and must be unchanged.
    let visible_bank_kept=if created.is_some() {capacity::visible_bank_kept(ctx,entry,slot,&before)} else {[None;2]};
    if let Some(value)=created {
        let mut state=STATE.lock().unwrap();
        state.capacities[index].reservation=Some((wanted.epoch,value));
        state.capacities[index].first_load=None;
        state.capacities[index].observation=CleanupObservation::new();
    }
    finish_install(install,"native capacity creation returned");
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] capacity creation epoch={:?} backend={:?} slot={} speculative={} qualified={} entry=0x{:x} owner=0x{:x} visible_mii_kept={:?} visible_bank_kept={:?}",
        wanted.epoch,backend,intent.extra,intent.speculative,created.is_some(),
        created.map_or(0,|r|r.entry),created.map_or(0,|r|r.resource),visible_mii_kept,visible_bank_kept));
    if visible_mii_kept.contains(&Some(false)) {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[CharacterSources] capacity creation changed a visible exact Mii slot={} visible_mii_kept={:?}: native creation beside a visible Mii is NOT safe (R-LP-1 fail)",
            intent.extra,visible_mii_kept));
    }
    if visible_bank_kept.contains(&Some(false)) {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[CharacterSources] capacity creation changed a visible body bank slot={} visible_bank_kept={:?}: native creation beside a visible body bank is NOT safe (R-LP-B1 fail)",
            intent.extra,visible_bank_kept));
    }
}

/// lobby-preload D: once per (scene, extra, reason). A refused preflight is evaluated again on
/// every helper loader callback (the designed wait, not a retry loop); this line reports it.
#[cfg(feature="managed-third-source-alpha")]
fn creation_waiting(intent:CreationIntent,slot:u32,refusal:Refusal) {
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] capacity creation waiting slot={} callback={} epoch={:?} want=0x{:x} speculative={} reason={}; preflight repeats on each helper callback",
        intent.extra,slot,intent.wanted.epoch,intent.wanted.kind,intent.speculative,refusal));
}

/// Only called with the current native loader callback's argument.
/// Already-owned loads run before any scene, mode or ownership gate.
pub unsafe fn service(ctx: &ServiceEntryContext, entry: *mut u8, slot: u32) {
    // Diag (F9 research): one line per change of No. of CPUs seen by the native loader callback, with the slot it served, so an
    // Eden row tells whether this callback runs while the pause menu is open (before the game activates a parked entry).
    #[cfg(feature = "diag")]
    {
        static SEEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
        let quantity = native_source::training_cpu_quantity().unwrap_or(u32::MAX - 1);
        if SEEN.swap(quantity, Ordering::Relaxed) != quantity {
            crate::engine::compat::logging::diagnostic(format_args!("[CharacterSources] cpu_quantity={quantity} seen_by_service slot={slot}"));
        }
    }
    let is_capacity=is_capacity_entry(entry,slot);
    if !cfg!(feature = "managed-sources") || !callback_slot(slot) {
        return;
    }
    let index = slot as usize;
    // Every entry7 callback starts with no saved stability. Restore it only
    // after a valid first-load observation; any early decline resets the run.
    #[cfg(feature = "managed-third-source-alpha")]
    let prior_first = if is_capacity {
        STATE.lock().unwrap().capacities[capacity_index(slot)].first_load.take()
    } else {
        None
    };
    if service_owned(ctx, entry, slot) {
        return;
    }
    #[cfg(feature="managed-third-source-alpha")]
    if usize::from(crate::runtime::roster::cycle_preload_config::snapshot().slots)>crate::engine::preload::cycle_preload_capacity::BASE_HELPERS && !super::preload_blocked()
        && !super::menu::captures_input() {
        if let Ok(backend)=qualified_backend() {
            let intent={creation_intent(&STATE.lock().unwrap(),backend,true)};
            if let Some(intent)=intent {create_capacity(ctx,entry,slot,intent);}
        }
    }
    let initial = {
        let state = STATE.lock().unwrap();
        // Idle: no native snapshots, log formatting, or allocation.
        // Per-slot: a want this helper may take is not hidden by an earlier avoided want.
        let Some(request) = selected_request(&state, slot, false) else {
            return;
        };
        (
            request,
            !ready_still_requested(&state, index) && state.book.can_replace(index) == Ok(true),
            state.sources[index],
        )
    };
    // Check the menu only after finding work: idle loader callbacks retain the
    // original allocation-free fast path. Re-select under the same channel rule
    // used at commit, without holding the source mutex while reading menu state.
    let menu_open = super::menu::captures_input();
    let ((wanted, speculative), replaceable, previous) = if menu_open {
        let state = STATE.lock().unwrap();
        let Some(request) = selected_request(&state, slot, true) else { return; };
        (request, !ready_still_requested(&state, index) && state.book.can_replace(index) == Ok(true),
            state.sources[index])
    } else { initial };
    if speculative && super::preload_blocked() { return; }
    let grace = if speculative {
        let Some(generation) = crate::runtime::roster::cycle_preload::speculative_admission() else { return; };
        Some(generation)
    } else { None };
    if !replaceable {
        #[cfg(feature = "managed-third-source-alpha")]
        if !speculative {
            if let Ok(backend)=qualified_backend() {
                let intent={creation_intent(&STATE.lock().unwrap(),backend,false)};
                if let Some(intent)=intent {create_capacity(ctx,entry,slot,intent);}
            }
        }
        return;
    }
    if admission_blocker().is_some() {
        return;
    }
    let Ok(backend) = qualified_backend() else {
        return;
    };
    #[cfg(feature="mii-managed-helper-alpha")]
    if (backend==Backend::Training || wanted.configuration.is_some())
        && !super::mii_managed_helper::admission_ready(wanted.configuration.is_some()) {return;}
    if !candidates(backend).contains(&slot) {
        return;
    }
    #[cfg(feature = "managed-third-source-alpha")]
    let (capacity_load, first_load_callbacks) = if is_capacity {
        // Only this module's current reservation can admit 7. An isolated
        // R3+X experiment's entry never becomes a managed source.
        if !capacity::source_kind(wanted.kind) {
            return;
        }
        let owner = { reservation_for(&STATE.lock().unwrap(),slot) };
        let Some(owner) = owner else {
            return;
        };
        let Some(before) =
            capacity::load_preflight(ctx, entry, owner, previous.map(|source| source.kind))
        else {
            return;
        };
        let ticks = if previous.is_none() {
            let ticks = prior_first
                .filter(|old| {
                    old.wanted == wanted
                        && old.reservation == owner
                        && old.previous == previous
                        && old.before == before
                })
                .map_or(1, |old| old.ticks.saturating_add(1));
            let mut state = STATE.lock().unwrap();
            if selected_request(&state, slot, menu_open) != Some((wanted,speculative))
                || state.sources[index] != previous
                || reservation_for(&state,slot) != Some(owner)
            {
                return;
            }
            state.capacities[capacity_index(slot)].first_load = Some(FirstLoad {
                wanted,
                reservation: owner,
                previous,
                before,
                ticks,
            });
            if ticks < 30 {
                return;
            }
            ticks
        } else {
            0
        };
        (Some(before), ticks)
    } else {
        (None, 0)
    };
    if !is_capacity && !native_admission(entry, slot, backend, previous.map(|source| source.kind)) {
        return;
    }
    let Some(costume) = native_source::inherited_costume(slot) else {
        return;
    };
    #[cfg(feature="mii-managed-helper-alpha")]
    if wanted.configuration.is_some_and(|c| !super::mii_managed_helper::slot_eligible(slot,c)) {
        return;
    }
    // GS-PANIC (`process_memory`): a helper load is a native Reload of a helper entry (files and bodies; s30 L8292
    // `[TrioBoundary] reload … owner=2 incoming=(1, [52 …` at 63 MiB aborted in the page commit). A speculative load
    // is discretionary: it starts only when the headroom covers its measured cost beside what every other consumer
    // still holds; otherwise it stays in the plan and a later callback tries again. A demand load (a visible request
    // waits for it) is not refused here. Both hold their cost while in flight (`helper_outstanding`).
    let memory_cost = crate::engine::swap::body_bank::memory::helper_cost(wanted.kind);
    let memory_committed = if speculative {
        let (decision, sample) = super::process_memory::admit(memory_cost);
        if decision.admission != crate::engine::swap::body_bank::memory::Admission::Admit {
            memory_refused(slot, wanted.kind, decision, memory_cost, "preload");
            return;
        }
        sample.committed.get()
    } else {
        crate::engine::preload::preload_metrics::MemorySample::capture().committed.get()
    };
    let Some(install) = INSTALL.try_begin(false) else {
        return;
    };
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        let owner = reservation_for(&STATE.lock().unwrap(),slot);
        if owner.and_then(|owner| {
            capacity::load_preflight(ctx, entry, owner, previous.map(|source| source.kind))
        }) != capacity_load
            || capacity_load.is_none()
        {
            STATE.lock().unwrap().capacities[capacity_index(slot)].first_load = None;
            finish_install(install, "capacity load preflight changed");
            return;
        }
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    let mii_prepared=match wanted.configuration {
        Some(c)=>match super::mii_managed_helper::prepare(entry as usize,slot,c) {
            Ok(p)=>Some(p),Err(reason)=>{
                if speculative {STATE.lock().unwrap().preload_failed|=1u128<<wanted.kind;}
                // A resolved row/hash mismatch belongs to this exact payload.
                // Table availability, live hat, snapshot and provenance errors
                // remain pending; a different helper callback may be eligible.
                if reason == "saved appearance hint/hash mismatch" {
                    refuse_want(wanted,slot,reason);
                }
                finish_install(install,reason);
                return;
            }
        },None=>None,
    };
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let original = match previous {
        // Missing first-install proof stays missing for this native lifetime.
        Some(source) => {
            if source.original.is_some_and(|original|
                coexist::helpers::observe(entry as usize, slot)
                    .is_none_or(|now| now.original != original))
            {
                finish_install(install, "managed original row changed");
                return;
            }
            source.original
        }
        None => if is_capacity {None} else {coexist::helpers::original(entry as usize, slot)},
    };
    let measurement=crate::engine::preload::preload_metrics::OperationStamp::begin(
        speculative && crate::runtime::roster::cycle_preload_config::snapshot().measurements,
        crate::engine::preload::preload_metrics::Operation::Background,crate::engine::preload::preload_metrics::Target::Fighter,
        wanted.kind,false,0);
    let mut state = STATE.lock().unwrap();
    // Same per-slot selection as initiation. Slot 7 capacity probes stay on the global walk.
    if selected_request(&state, slot, menu_open) != Some((wanted,speculative))
        || grace.is_some_and(|generation| !crate::runtime::roster::cycle_preload::speculative_admission_current(generation))
        || state.sources[index] != previous
        || ready_still_requested(&state, index)
        || state.book.can_replace(index) != Ok(true)
    {
        #[cfg(feature = "managed-third-source-alpha")]
        if is_capacity {
            state.capacities[capacity_index(slot)].first_load = None;
        }
        drop(state);
        finish_install(install, "managed source intent changed");
        return;
    }
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    if !INSTALL.owns(install) {
        drop(state);
        finish_install(install, "managed exact install owner changed");
        return;
    }
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let mut registry = (!is_capacity).then(coexist::registry);
    let load = match state.book.begin_install(
        index,
        wanted.epoch,
        NativeEntry(entry as u64),
        SourceKey {
            kind: wanted.kind,
            costume,
        },
    ) {
        Ok(load) => load,
        Err(_) => {
            #[cfg(feature = "managed-third-source-alpha")]
            if is_capacity {
                state.capacities[capacity_index(slot)].first_load = None;
            }
            #[cfg(feature = "mii-managed-source-coexist-alpha")]
            drop(registry);
            drop(state);
            finish_install(install, "managed source admission rejected");
            return;
        }
    };
    if let Some(generation) = grace {
        crate::runtime::roster::cycle_preload::speculative_admission_committed(generation);
    }
    state.sources[index] = Some(Source {
        token: load.source(),
        load,
        install: Some(install),
        kind: wanted.kind,
        ticks: 0,
        prefetched: speculative,
        measurement,
        configuration:wanted.configuration,
        #[cfg(feature = "mii-managed-source-coexist-alpha")]
        original,
        memory_committed,
    });
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    {
        if let Some(registry)=registry.as_mut() {
            registry.source(state.book.epoch(), index,
                coexist::helpers::Helper::Unavailable { token: load.source(), phase: Phase::Loading });
        }
        drop(registry);
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    if let Some(prepared)=mii_prepared {
        if let Err(reason)=super::mii_managed_helper::bind(prepared,load,install) {
            drop(state);
            crate::engine::compat::logging::diagnostic(format_args!("[MiiHelper] bind refused; INSTALL retained source={:?} reason={}",load.source(),reason));
            return;
        }
    }
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        state.capacities[capacity_index(slot)].load = capacity_load.map(|before| (load.source(), before));
        state.capacities[capacity_index(slot)].first_load = None;
        // A replacement publishes its new source before native release of the
        // old actor. Retain the old generation solely to attribute that event.
        state.capacities[capacity_index(slot)].observation.retiring = previous;
        #[cfg(feature = "managed-grouped-source-alpha")]
        {
            let observed = &mut state.capacities[capacity_index(slot)].observation;
            observed.retiring_actors = observed.current_actors.take();
            observed.masks = [(0, 0), observed.masks[0]];
            observed.cleaned = [[None; 3], observed.cleaned[0]];
        }
    }
    drop(state);
    #[cfg(feature = "managed-third-source-alpha")]
    if first_load_callbacks != 0 {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[CharacterSources] first capacity load stable slot={} callbacks={}",
            slot, first_load_callbacks
        ));
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] install slot={} kind=0x{:x}",
        slot, wanted.kind
    ));
    #[cfg(feature="mii-managed-helper-alpha")]
    if wanted.configuration.is_some() {
        if !super::mii_managed_helper::install(load.source()) {
            crate::engine::compat::logging::diagnostic(format_args!("[MiiHelper] native submit refused; INSTALL retained source={:?}",load.source()));
        }
        return;
    }
    native_source::install(entry, wanted.kind);
}

/// Association uses the native destructor's current actor and current entry7,
/// including old actors released while a new managed install is pending.
#[cfg(feature = "managed-third-source-alpha")]
pub(super) unsafe fn capacity_actor_cleanup(ctx: &InlineCtx) {
    for slot in crate::engine::preload::cycle_preload_capacity::ALL_EXTRA_SLOTS {capacity_actor_cleanup_at(ctx,slot as u32);}
}
#[cfg(feature="managed-third-source-alpha")]
unsafe fn capacity_actor_cleanup_at(ctx: &InlineCtx, slot:u32) {
    let (saved, current, observation) = {
        let state = STATE.lock().unwrap();
        let Some(saved) = state.capacities[capacity_index(slot)].reservation else {
            return;
        };
        (saved, state.sources[slot as usize], state.capacities[capacity_index(slot)].observation)
    };
    #[cfg(feature = "managed-grouped-source-alpha")]
    let association = {
        let actor = ctx.registers[20].x() as usize;
        if observation.retiring_actors.is_some_and(|set| set.unretired(actor, observation.masks[1].0)) {
            observation.retiring.map(|source| (source, observation.retiring_actors, 1))
        } else {
            current.filter(|_| observation.current_actors.is_none_or(|set| set.contains(actor)))
                .map(|source| (source, observation.current_actors, 0))
        }
    };
    #[cfg(feature = "managed-grouped-source-alpha")]
    let event = association.and_then(|(source, actors, group)|
        capacity::actor_cleanup_observation(ctx, saved.1, source.kind, actors, observation.cleaned[group][0]));
    #[cfg(not(feature = "managed-grouped-source-alpha"))]
    let event = capacity::actor_cleanup_observation(ctx, saved.1);
    let Some(event) = event else {
        return;
    };
    #[cfg(feature = "managed-grouped-source-alpha")]
    let source = association.map(|(source, _, _)| source);
    #[cfg(not(feature = "managed-grouped-source-alpha"))]
    let source = current
        .filter(|source| source.kind == event.kind)
        .or_else(|| {
            observation
                .retiring
                .filter(|source| source.kind == event.kind)
        });
    let source = source.filter(|source| {
            source.token.epoch() == saved.0
                && source.token.entry() == NativeEntry(saved.1.entry as u64)
        });
    let key = (source.map(|source| source.token), event.actor);
    #[cfg(feature = "managed-grouped-source-alpha")]
    let group = association.map(|(_, _, group)| group).unwrap_or(0);
    #[cfg(feature = "managed-grouped-source-alpha")]
    if observation.masks[group].0 & (1 << event.index) != 0 { return; }
    if observation.last_actor == Some(key) {
        return;
    }
    let valid = event.valid && source.is_some();
    let mut state = STATE.lock().unwrap();
    if state.capacities[capacity_index(slot)].reservation != Some(saved)
        || state.sources[slot as usize] != current
        || state.capacities[capacity_index(slot)].observation != observation
    {
        return;
    }
    let observed = &mut state.capacities[capacity_index(slot)].observation;
    observed.last_actor = Some(key);
    #[cfg(feature = "managed-grouped-source-alpha")]
    {
        observed.masks[group].0 |= 1 << event.index;
        observed.cleaned[group][event.index] = event.record;
        if valid { observed.masks[group].1 |= 1 << event.index; }
    }
    observed.actor_events = observed.actor_events.saturating_add(1);
    if valid {
        observed.valid_actors = observed.valid_actors.saturating_add(1);
        if source.is_some_and(|source| source.install.is_none()) {
            observed.ready_cleanups = observed.ready_cleanups.saturating_add(1);
        }
    }
    #[cfg(feature = "managed-grouped-source-alpha")]
    let masks = observed.masks;
    drop(state);
    #[cfg(feature = "managed-grouped-source-alpha")]
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ManagedSourceCleanup] canonical cleanup source={:?} retiring={} index={} masks={:?} record={:x?}",
        key.0, group == 1, event.index, masks, event.record));
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ManagedSourceCleanup] actor epoch={:?} entry=0x{:x} owner=0x{:x} actor=0x{:x} kind=0x{:x} source={:?} pending={:?} identity={} entry_associated=true resource_associated={} native_return={} valid={} input={:x?}",
        saved.0, saved.1.entry, saved.1.resource, event.actor, event.kind,
        key.0, source.map(|source| source.install.is_some()),
        event.identity, event.resource_associated, event.returned, valid, event.input));
}

/// The entry destructor owns entry now; read the current service outside STATE.
#[cfg(feature = "managed-third-source-alpha")]
unsafe fn capacity_destroying(entry: *const u8,slot:u32) {
    let saved = STATE.lock().unwrap().capacities[capacity_index(slot)].reservation;
    let Some(saved) = saved.filter(|(_, value)| {
        value.entry == entry as usize && !value.entry_destroyed && !value.owner_retired
    }) else {
        return;
    };
    let before = capacity::listener_count(saved.1);
    let mut state = STATE.lock().unwrap();
    if state.capacities[capacity_index(slot)].reservation == Some(saved) {
        state.capacities[capacity_index(slot)].observation.destroy_seen = true;
        state.capacities[capacity_index(slot)].observation.listeners_before = before;
    }
}

#[cfg(feature = "managed-third-source-alpha")]
pub(super) unsafe fn capacity_listener_removed(ctx: &InlineCtx) {
    for slot in crate::engine::preload::cycle_preload_capacity::ALL_EXTRA_SLOTS {capacity_listener_removed_at(ctx,slot as u32);}
}
#[cfg(feature="managed-third-source-alpha")]
unsafe fn capacity_listener_removed_at(ctx: &InlineCtx, slot:u32) {
    let (saved, observation) = {
        let state = STATE.lock().unwrap();
        let Some(saved) = state.capacities[capacity_index(slot)].reservation else {
            return;
        };
        (saved, state.capacities[capacity_index(slot)].observation)
    };
    if ctx.registers[19].x() as usize != saved.1.entry
        || !saved.1.entry_destroyed
        || !observation.destroy_seen
        || observation.listener_removed.is_some()
    {
        return;
    }
    let after = capacity::listener_after(ctx, saved.1);
    let removed = observation
        .listeners_before
        .zip(after)
        .is_some_and(|(before, after)| before.checked_sub(1) == Some(after));
    let mut state = STATE.lock().unwrap();
    if state.capacities[capacity_index(slot)].reservation != Some(saved) || state.capacities[capacity_index(slot)].observation != observation {
        return;
    }
    state.capacities[capacity_index(slot)].observation.listener_removed = Some(removed);
    drop(state);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ManagedSourceCleanup] listener epoch={:?} entry=0x{:x} owner=0x{:x} before={:?} after={:?} removed={}",
        saved.0, saved.1.entry, saved.1.resource, observation.listeners_before, after, removed));
}

/// Numeric association at the actual owner cleanup boundary, after its native
/// destructor body. Never dereference a saved owner, including allocator reuse.
#[cfg(feature = "managed-third-source-alpha")]
pub(super) fn capacity_owner_retired(owner: usize)->bool {
    crate::engine::preload::cycle_preload_capacity::ALL_EXTRA_SLOTS.iter().any(|slot|capacity_owner_retired_at(owner,*slot as u32))
}
#[cfg(feature="managed-third-source-alpha")]
fn capacity_owner_retired_at(owner: usize, slot:u32) -> bool {
    let mut state = STATE.lock().unwrap();
    let Some((epoch, mut value)) = state.capacities[capacity_index(slot)].reservation else {
        return false;
    };
    if value.resource != owner || value.owner_retired {
        return false;
    }
    value.owner_retired = true;
    state.capacities[capacity_index(slot)].first_load = None;
    state.capacities[capacity_index(slot)].reservation = if value.entry_destroyed {
        None
    } else {
        Some((epoch, value))
    };
    let observed = state.capacities[capacity_index(slot)].observation;
    let cleared = state.capacities[capacity_index(slot)].reservation.is_none();
    drop(state);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] capacity owner retired owner=0x{:x} entry_destroyed={}",
        owner, value.entry_destroyed
    ));
    crate::engine::compat::logging::diagnostic(format_args!(
        "[ManagedSourceCleanup] retirement epoch={:?} entry=0x{:x} owner=0x{:x} entry_destroyed={} reservation_cleared={} ready_actors={} actor_events={} valid_actors={} ready_cleanups={} listener_removed={:?}",
        epoch, value.entry, owner, value.entry_destroyed, cleared, observed.ready_actors,
        observed.actor_events, observed.valid_actors, observed.ready_cleanups, observed.listener_removed));
    true
}

/// The synchronous native entry destructor owns this argument. Keep the full
/// admission token beside it until this callback, including across mode changes.
pub unsafe fn on_destroy(entry: *const u8, slot: u32) {
    #[cfg(any(feature="managed-third-source-alpha",feature="mii-managed-source-coexist-alpha"))]
    let is_capacity=is_capacity_entry(entry,slot);
    if slot >= 8 {
        return;
    }
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        capacity_destroying(entry,slot);
    }
    let mut state = STATE.lock().unwrap();
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        if let Some((epoch, mut value)) = state.capacities[capacity_index(slot)].reservation {
            if value.entry == entry as usize {
                state.capacities[capacity_index(slot)].first_load = None;
                value.entry_destroyed = true;
                state.capacities[capacity_index(slot)].reservation = if value.owner_retired {
                    None
                } else {
                    Some((epoch, value))
                };
            }
        }
    }
    let Some(source) = state.sources[slot as usize] else {
        return;
    };
    if source.token.entry() != NativeEntry(entry as u64) {
        return;
    }
    let current=state.book.is_current(source.token.epoch());
    #[cfg(feature = "managed-third-source-alpha")]
    let destruction = (
        state.book.phase(slot as usize),
        !state.book.is_current(source.token.epoch()),
    );
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    let mut registry = (!is_capacity).then(coexist::registry);
    let released = match state.book.native_destroyed(source.token) {
        Ok(released) => released,
        Err(error) => {
            #[cfg(feature = "mii-managed-source-coexist-alpha")]
            drop(registry);
            drop(state);
            crate::engine::compat::logging::diagnostic(format_args!(
                "[CharacterSources] destructor identity rejected slot={} error={:?}",
                slot, error
            ));
            return;
        }
    };
    for live in &mut state.leases {
        if live.is_some_and(|live| live.source == source.token) {
            *live = None;
        }
    }
    HAS_CONSUMERS.store(state.leases.iter().any(Option::is_some), Ordering::Release);
    state.sources[slot as usize] = None;
    #[cfg(feature = "mii-managed-source-coexist-alpha")]
    {
        if let Some(registry)=registry.as_mut() {
            registry.source(state.book.epoch(), slot as usize, coexist::helpers::Helper::Unmanaged);
        }
        drop(registry);
    }
    #[cfg(feature="mii-managed-helper-alpha")]
    super::mii_managed_helper::release(source.token);
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        state.capacities[capacity_index(slot)].load = None;
    }
    drop(state);
    #[cfg(feature = "managed-third-source-alpha")]
    if is_capacity {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[ManagedSourceCleanup] entry destruction source={:?} phase={:?} ticks={} departed={} pending_install={:?} gate_active={}",
            source.token, destruction.0, source.ticks, destruction.1, source.install, INSTALL.active()));
    }
    if let Some(install) = source.install {
        finish_install(install, "managed source destroyed");
        if let Some(stamp)=source.measurement {
            stamp.finish(u32::from(source.ticks),crate::engine::preload::preload_metrics::Outcome::Departed,metrics());
        }
    }
    if source.prefetched && current {
        let now=metrics();
        if now.resident==0 && !now.busy {
            crate::engine::preload::preload_metrics::snapshot(crate::runtime::roster::cycle_preload_config::snapshot().measurements,
                crate::engine::preload::preload_metrics::Label::Retired,crate::runtime::roster::cycle_preload::frame(),now);
        }
    }
    crate::engine::compat::logging::diagnostic(format_args!(
        "[CharacterSources] destroyed slot={} tracked_leases={}",
        slot, released
    ));
}
