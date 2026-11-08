//! Default-off Player0 adapter with an immutable ordinary or saved origin.
//! INSTALL submission and phase-time publication are distinct commits.
use super::*;
use std::sync::Arc;
use hos_lock::Mutex as OwnerMutex;
use std::convert::TryInto;
use crate::engine::mii::{
    inventory::{MiiClass,Snapshot as Catalog}, lifetime::Configuration,
    ordinary_player::{Gate,Phase,Request,SameClass,Ticket},
    selection::{Action,Origin,PlayerLive,PlayerOrigin},
    identity::Identity as TxnIdentity,
    transition::{Outcome,ReadyReceipt,Refusal},
};
use crate::engine::mii_root::mii_menu::Context as MenuContext;
use crate::engine::swap::mii_cycle::{self,Identity,Live};
#[path="mii_ordinary_native.rs"] pub(super) mod native;
#[path="mii_ordinary_gate.rs"] mod hook;

static ENABLED:AtomicBool=AtomicBool::new(false);
// Const kernel lock: the first native callback must not allocate a pthread
// mutex. Keep exclusion for all callers; no game-thread-only assumption.
static STATE:OwnerMutex<State>=OwnerMutex::new(State {
    owner:None,track:None,catalog:None,capture:false,capture_at:(0,0),menu_valid:false,request:None,next:1,
    generic:None,restore:None,
    #[cfg(feature="native-body-bank-alpha")]
    bank:None,
    #[cfg(feature="native-body-bank-alpha")]
    mii_exit:None,
});
struct Owner {
    origin:native::Snapshot,
    expected:native::Snapshot,
    origin_saved:Option<Configuration>,
    active:Option<Configuration>,
    cursor:Option<usize>,
}
#[derive(Clone)]
struct Track {
    ticket:Ticket, gate:Gate, token:InstallToken, installing:bool,
    retained_o:bool,
    /// WP5c: set once when a committed track can no longer be adopted and waits for exact
    /// destruction (`transition::Recovery::AwaitDestruction`); the reason names the outcome.
    recovering:Option<&'static str>,
    data:Arc<Prepared>,
}
/// WP5c: a newly noted recovery, traced by the caller after the STATE lock is dropped.
#[derive(Clone,Copy)]
struct RecoveringNote {ticket:Ticket,reason:&'static str,loader:Option<u8>,published:bool}
impl std::ops::Deref for Track {
    type Target=Prepared;
    fn deref(&self)->&Prepared {&self.data}
}
struct Prepared {
    before:native::Snapshot, origin_scalars:native::Scalars,
    descriptor:[u8;0x70], publish:Option<Configuration>,
    destination:mii_cycle::Step,
    same_class:Option<SameClass>,
    /// Physical same-entry bank build (laid out by the planner; the target at its
    /// own cell, `ReloadTarget`). Completion is the normal phase-6 proof; a
    /// terminal refusal releases INSTALL and retires authority.
    bank:bool,
}
impl Prepared {
    /// The cell native activates at Ready and the logical target this transaction settles on.
    fn reload_target(&self)->crate::engine::swap::body_bank::ReloadTarget {
        crate::engine::swap::body_bank::ReloadTarget::of(&self.descriptor,self.bank)
    }
}
/// Same-entry bank member switch. No phase-6 publication, no new physical receipt:
/// `before` is the owner's current physical snapshot and is never replaced by it.
/// `logical` is this token's requested member; the durable value is `ready_kind`.
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy)]
struct BankTrack {
    ticket:Ticket, token:InstallToken, before:native::Snapshot, logical:u32,
    destination:mii_cycle::Step, departed:bool,
    /// A Mii one-member group switch (MII-GROUPS-DESIGN §5): the exact configuration
    /// published onto the owner's record before the switch and proven at settlement;
    /// `descriptor` is the request's staged exact-Mii bytes naming its identity.
    publish:Option<Configuration>, descriptor:[u8;0x70], hat_change:bool,
    /// MII-PHASEB: what `bank_publish` wrote (the configuration and the exact pre-publish
    /// record and scalars), recorded under G and STATE in the same critical section as the
    /// write. The one terminal arm (`bank_refused`) puts it back before INSTALL goes.
    published:Option<(Configuration,native::BankPublished)>,
    /// rc.6 warm exit: this switch leaves the live Mii group for the window it displaced (`MiiExit`).
    exit:bool,
}
/// rc.6 warm exit (operator decision 2026-11-08): a settled Mii group switch's publication, kept while the
/// owner's receipt is the one that switch adopted, so the exit to the window it displaced can put it back
/// (`native::exit_settled`).
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy)]
struct MiiExit {mii:BankTrack,configuration:Configuration,published:native::BankPublished,adopted:native::Snapshot}
/// Read-only receipt for an ordinary Player0 load kept on the generic roster path.
/// It carries no authority; `generic_completed` re-proves everything against `before`.
struct Generic {
    token:InstallToken,before:native::Snapshot,expected:native::Snapshot,descriptor:[u8;0x70],
}
struct State {
    owner:Option<Owner>,track:Option<Track>,catalog:Option<Arc<Catalog>>,
    capture:bool,capture_at:(u64,u64),menu_valid:bool,request:Option<Request>,next:u64,
    generic:Option<Generic>,
    restore:Option<WaitingValue>,
    #[cfg(feature="native-body-bank-alpha")]
    bank:Option<BankTrack>,
    #[cfg(feature="native-body-bank-alpha")]
    mii_exit:Option<MiiExit>,
}
impl State {
    /// An owned phase-6 transaction or bank member switch holds this adapter.
    fn busy(&self)->bool {
        #[cfg(feature="native-body-bank-alpha")]
        if self.bank.is_some() {return true;}
        self.track.is_some()
    }
}
fn base()->usize { BASE.load(Ordering::Acquire) }
fn epoch()->u64 { mii_saved_alpha::current_generation(0) }
/// lobby-preload A: Player0's visible exact Mii for extra-capacity creation. Only the authority
/// this adapter committed for owner 0, rederived now under G, proves it. Reads only.
#[cfg(all(feature="mii-ordinary-cpu-alpha",feature="managed-third-source-alpha"))]
pub(super) unsafe fn visible_exact(current:*const u8,kind:u32)->Result<native::VisibleMii,&'static str> {
    let original=original_kind(Role::Player).ok_or("Player original absent")?;
    native::visible_exact(current as usize,epoch(),original,kind)
}
fn note(reason:impl std::fmt::Display) { trace!("[MiiOrdinaryPlayer] {reason}"); }

pub(super) fn preload_pending()->bool {
    let s=STATE.lock(); s.capture || s.request.is_some() || s.busy()
}
pub(super) fn preload_targets(menu:&TrainingModpackMenu)->Option<[Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS]> {
    let (original,live,catalog,saved)={
        let s=STATE.lock();
        if !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
        let o=s.owner.as_ref()?;
        if o.origin.binding.epoch!=epoch() {return None;}
        let identity=o.active.map_or(Identity::Fighter(unsafe {logical_kind(&o.expected)}),|c|Identity::Saved(c.canonical()));
        let live=if unsafe {at_origin(&o.origin,&o.expected,o.active,o.origin_saved,o.cursor)} {Live::Original}
            else {Live::Numbered {identity,cursor:o.cursor}};
        (o.origin_saved.map_or(Identity::Fighter(o.origin.original_kind),|c|Identity::Saved(c.canonical())),
            live,Arc::clone(s.catalog.as_ref()?),o.origin_saved)
    };
    Some(crate::runtime::roster::cycle_preload::mixed_targets(menu,0,original,live,&catalog,saved))
}

pub(super) unsafe fn init(base:usize)->bool {
    // WP-A2: both gates are checked and recorded (64-byte rows decide; the FNV/word verdicts are the shadow compare),
    // before the gate's own patch and before the entry service hook at 0x650010 goes in.
    let saved=mii_saved_fingerprints::valid(base);
    let gate=hook::gate(base);
    hook::seal_headgear(base);
    if !saved || !gate {
        note("disabled: prequeue/service fingerprint mismatch");
        return false;
    }
    hook::install(base);
    ENABLED.store(true,Ordering::Release);
    note("enabled: staged Player0 alpha; one owner for ordinary and saved origins");
    true
}
pub(super) fn catalog_ready(catalog:Arc<Catalog>,generation:u64,opening:u64)->bool {
    if !ENABLED.load(Ordering::Acquire) {return false;}
    let mut s=STATE.lock();
    if epoch()!=generation || mii_saved_alpha::menu_epoch()!=opening {return false;}
    let complete=catalog.is_complete();
    s.catalog=Some(catalog);s.menu_valid=false;s.capture=complete && !s.busy();
    s.capture_at=(generation,opening);
    s.capture
}
#[cfg(feature="mii-ordinary-cpu-alpha")]
pub(super) fn library_catalog() -> Result<Option<Arc<Catalog>>, ()> {
    let s = STATE.try_lock().ok_or(())?;
    Ok(s.catalog.as_ref().map(Arc::clone))
}
#[cfg(feature="mii-ordinary-cpu-alpha")]
pub(super) fn publish_library(expected: &Arc<Catalog>, prepared: &Arc<Catalog>) -> Result<bool, ()> {
    let mut s = STATE.try_lock().ok_or(())?;
    if !s.catalog.as_ref().is_some_and(|catalog| Arc::ptr_eq(catalog, expected)) { return Ok(false); }
    s.catalog = Some(Arc::clone(prepared));
    Ok(true)
}
pub(super) unsafe fn menu_capture_pending(entry:*const u8)->bool {
    let s=STATE.lock();
    s.capture && (s.owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry as usize)
        || original_kind(Role::Player)==Some(read::<u32>(entry,0x18)))
}
#[cfg(feature="mii-managed-helper-alpha")]
pub(super) fn helper_origin_captured()->bool {
    let s=STATE.lock();
    !s.capture && !s.busy() && s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch())
}
/// Settled loaded configuration for this Player entry. Original, saved and
/// discovered captures all live in `active` after they are the loaded body.
/// A transaction, a non-Mii, or a configuration whose class is not the loaded
/// descriptor kind returns None.
pub(super) fn current_configuration()->Option<Configuration> {
    let s=STATE.lock();
    if !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
    let owner=s.owner.as_ref().filter(|o|
        o.origin.binding.epoch==epoch() && o.origin.layout.logical(o.origin.binding.owner)==Some(0))?;
    let configuration=owner.active?;
    (configuration.canonical().class as u32==descriptor_kind(&owner.expected.descriptor))
        .then_some(configuration)
}
pub(super) fn player_context()->Option<(PlayerOrigin,PlayerLive)> {
    let s=STATE.lock();
    if !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
    s.owner.as_ref().filter(|o|o.origin.binding.epoch==epoch()).map(|o| {
        let original=o.origin_saved.map_or(PlayerOrigin::NotMii,
            |c|PlayerOrigin::Mii(crate::engine::mii::selection::Origin(c.canonical())));
        let live=if o.cursor.is_none() && (o.expected.descriptor==o.origin.descriptor
            || o.expected.descriptor==crate::engine::swap::entry_window::original_request(o.origin.descriptor,o.origin.original_kind))
            && o.active==o.origin_saved {PlayerLive::Origin}
            else {o.active.map_or(PlayerLive::Origin,|c|
                PlayerLive::Cycled{class:c.canonical().class,canonical:c.canonical()})};
        (original,live)
    })
}
/// Consume ordinary Player actions even when refused, so they cannot fall through to SavedRole.
pub(super) fn menu_close(context:MenuContext,action:Action)->bool {
    if !matches!(action,Action::ReloadPlayer(_)) {return false;}
    let mut s=STATE.lock();
    let Some(owner)=s.owner.as_ref() else {
        return enabled();
    };
    if !ENABLED.load(Ordering::Acquire) || !s.menu_valid || s.busy() || s.request.is_some() {
        drop(s);note("menu-close request refused: ordinary owner busy");return true;
    }
    let original=owner.origin_saved.map_or(PlayerOrigin::NotMii,
        |c|PlayerOrigin::Mii(crate::engine::mii::selection::Origin(c.canonical())));
    if context.original!=original {drop(s);note("menu original identity changed");return true;}
    let request=Request::from_menu(mii_saved_alpha::menu_epoch(),context.epoch,
        context.original,context.active.player,owner.active.map(|c|c.canonical()),action);
    s.request=request;s.restore=None;
    if request.is_some() {WAITING_MEMORY.lock().manual_request(Role::Player as usize);}
    if request.is_none() {
        drop(s);note("menu-close request refused: ordinary active configuration changed or out of scope");
    }
    true
}
fn admit_hat(layout:super::mii_backend::Layout,owner:usize,before:u16,target:Configuration)
    ->Result<(),&'static str> {
    if !crate::engine::mii::ordinary_cpu::supported_kind(target.canonical().class as u32) {
        return Err("Mii class not qualified");
    }
    let changed=crate::engine::mii::ordinary_cpu::headgear_reload(
        layout,owner,before,target.tokens().hat)?;
    if changed && !unsafe { native::cpu_headgear_fingerprints() } {
        return Err("Player headgear fingerprint mismatch");
    }
    Ok(())
}
fn stage_saved(origin:&native::Snapshot,target:Configuration)->[u8;0x70] {
    let mut descriptor=crate::engine::mii::ordinary_player::staged_descriptor(origin.descriptor,Some(target));
    descriptor[0x46..0x48].copy_from_slice(&origin.binding.record_index.to_le_bytes());
    descriptor
}
fn clear_request(request:Request) {
    let mut s=STATE.lock();
    let restore_refused=s.request==Some(request) && s.restore.is_some();
    if s.request==Some(request) {s.request=None;s.restore=None;}
    drop(s);
    if restore_refused {
        WAITING_MEMORY.lock().manual_request(Role::Player as usize);
        notify(Role::Player,"Practice fighter could not be restored".into());
    }
}
pub(super) fn enabled()->bool {ENABLED.load(Ordering::Acquire)}
pub(super) fn owns(entry:usize)->bool {
    STATE.lock().owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry)
}
pub(super) fn request_restore(value:WaitingValue)->bool {
    let mut s=STATE.lock();
    if !mii_backend::current().is_some_and(|l|l.waiting()) || !enabled() {return false;}
    // Qualification precedes continuation, ordinary values included: the remembered
    // choice replaces the CSS-origin body, the only body capture can qualify.
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if unsafe {super::mii_auto_init::origin_capture_pending(Role::Player,
        s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch()),s.capture)} {return true;}
    // Mixed rosters are owned by intercept even when the remembered fighter is ordinary.
    // Restore the exact remembered value rather than resolving a new menu choice.
    let mixed=setting(&MENU).player_roster.fighters().iter()
        .any(|f|MiiClass::from_kind(f.kind()).is_some());
    let needed=value.owned || original_kind(Role::Player).is_some_and(|k|MiiClass::from_kind(k).is_some())
        || s.owner.as_ref().is_some_and(|o|o.active.is_some() || o.cursor.is_some() || mixed);
    if !needed {return false;}
    if s.capture || s.busy() || s.request.is_some()
        || s.owner.as_ref().is_none_or(|o|o.origin.binding.epoch!=epoch()) {return true;}
    if WAITING_MEMORY.lock().restore_target(Role::Player as usize)!=Some(value) {return true;}
    s.restore=Some(value);s.request=Some(Request::CYCLE);
    true
}
pub(super) fn intercept(role:Role,selection:Selection,user_request:bool)->Option<bool> {
    if role!=Role::Player || !ENABLED.load(Ordering::Acquire) {return None;}
    let mut s=STATE.lock();
    let Some(owner)=s.owner.as_ref() else {
        #[cfg(feature="mii-ordinary-cpu-alpha")]
        // A manual ordinary swap can replace the same origin needed by capture.
        // Consume it without enqueuing a stale choice; the existing loading notice
        // asks the user to retry once the shared catalog is qualified.
        if unsafe {super::mii_auto_init::origin_capture_pending(Role::Player,false,s.capture)} {
            drop(s);
            if user_request {notify(Role::Player,"Wait for the practice character to finish loading".into());}
            return Some(true);
        }
        #[cfg(feature="mii-ordinary-cpu-alpha")]
        let automatic=super::mii_auto_init::pending();
        #[cfg(not(feature="mii-ordinary-cpu-alpha"))]
        let automatic=false;
        let qualifying=unsafe {mii_backend::ready()} && (automatic || s.capture);
        let roster=qualifying && matches!(selection,Selection::Cycle) && setting(&MENU).player_roster.fighters_excluding_fixed(None).iter()
            .any(|f|MiiClass::from_kind(f.kind()).is_some());
        let direct=if let Selection::Direct(f)=selection {Some(f.kind())} else {None};
        if crate::engine::mii::auto_init::requires_owner(original_kind(Role::Player),roster,direct) {
            drop(s);note("Mii request refused: original assignment not captured");
            if user_request && mii_backend::current().is_some() {
                notify(Role::Player,"Wait for the practice character to finish loading".into());
            }
            return Some(true);
        }
        return None;
    };
    // A captured mixed roster remains in this adapter, including its ordinary entries.
    let has_mii=setting(&MENU).player_roster.fighters_excluding_fixed(None).iter()
        .any(|f|MiiClass::from_kind(f.kind()).is_some());
    // Same-entry banks: a qualified, captured owner (current epoch, valid menu and
    // catalog, no capture pending) also keeps ordinary-only cycles and direct picks,
    // so one adapter owns the physical receipt, the bank switch and the cursor. A
    // direct pick resolves without a cursor (`mii_cycle::direct`), so a pick made
    // straight after another pick stays a warm group swap instead of a canonical
    // generic reload that retires the groups (C118 L1). A missing or unqualified
    // owner still takes the fallback above or the generic roster below.
    #[cfg(feature="native-body-bank-alpha")]
    let ordinary_request=matches!(selection,Selection::Cycle|Selection::Direct(_)) && s.menu_valid && !s.capture
        && s.catalog.is_some() && owner.origin.binding.epoch==epoch();
    #[cfg(not(feature="native-body-bank-alpha"))]
    let ordinary_request=false;
    let handles=owner.origin_saved.is_some() || owner.active.is_some() || owner.cursor.is_some() || has_mii
        || ordinary_request
        || matches!(selection,Selection::Direct(f) if MiiClass::from_kind(f.kind()).is_some());
    if !handles {return None;}
    // The roster reconciles partner count every frame. Coalesce that automatic
    // request while an owned operation is pending; do not sync a refusal to SD.
    if matches!(selection,Selection::RefreshPartners) && (s.request.is_some() || s.busy()) {
        return Some(true);
    }
    let request=match selection {
        Selection::Cycle=>Some(Request::CYCLE),
        Selection::Direct(f)=>Request::direct(mii_saved_alpha::menu_epoch(),f.kind()),
        Selection::RefreshPartners=>Some(Request::REFRESH_PARTNERS),
    };
    let manual_direct=user_request && matches!(selection,Selection::Direct(_));
    // Before INSTALL begins, an explicit pick supersedes a pending lobby restore.
    let replace_restore=manual_direct && s.restore.is_some();
    if !s.busy() && (s.request.is_none() || replace_restore) && request.is_some() {
        s.restore=None;
        if user_request {WAITING_MEMORY.lock().manual_request(Role::Player as usize);}
        s.request=request;
    } else {
        drop(s);note("request refused: owned Player request busy or unsupported");
        if manual_direct {notify(Role::Player,"Finish the current practice action first".into());}
    }
    Some(true)
}

unsafe fn permitted()->bool {
    #[cfg(feature="mii-ordinary-cpu-alpha")]
    if super::mii_ordinary_cpu::writers_reserved(0,0) {return false;}
    mii_backend::ready() && crate::runtime::compat::common::is_ready_go()
        && !menu::captures_input() && !lobby::native_ui_busy()
        && !button_config::has_pending_request() && !INSTALL.active()
        && !training::save_states::is_loading() && !training::recording_or_playback()
        && !crate::runtime::compat::replay_preview::occupied()
        && STATES.iter().all(|s| {let s=s.lock();s.active.is_none()&&s.pending.is_none()})
}
unsafe fn ready(entry:*const u8,kind:u32)->bool {
    if MiiClass::from_kind(kind).is_some() {ready_single_mii(entry,0)==Some(kind)}
    else {ready_forms(entry,0).is_some_and(|(_,k)|k==kind)}
}
fn descriptor_kind(d:&[u8;0x70])->u32 {u32::from_le_bytes(d[0x18..0x1c].try_into().unwrap())}
fn descriptor_forms(d:&[u8;0x70])->Forms {
    let kind=descriptor_kind(d);
    if kind==0x4b && u32::from_le_bytes(d[0x14..0x18].try_into().unwrap())==1 {
        Forms::solo_popo()
    } else {Forms::starting(kind)}
}
unsafe fn ready_descriptor(entry:*const u8,d:&[u8;0x70])->bool {
    if MiiClass::from_kind(descriptor_kind(d)).is_some() {ready(entry,descriptor_kind(d))}
    else {ready_forms(entry,0).is_some_and(|(forms,_)|forms==descriptor_forms(d))
        || bank_live(entry,d).is_some()}
}
/// Live member of this owner's ready same-entry bank when `d` is exactly that
/// bank's physical group (count, member kinds, one actor). Value only: the bank
/// module proves the current descriptor and every body receipt under its lock.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_live(entry:*const u8,d:&[u8;0x70])->Option<u32> {
    let members=super::body_bank::ready_members(entry,0)?;
    let word=|at:usize|u32::from_le_bytes(d[at..at+4].try_into().unwrap());
    if word(0x14) as usize==members.count() && word(0x28)==members.actor_count()
        && (0..3).all(|i|word(0x18+i*4)==members.kinds()[i]) {
        super::body_bank::ready_kind(entry,0)
    } else {None}
}
#[cfg(not(feature="native-body-bank-alpha"))]
unsafe fn bank_live(_entry:*const u8,_d:&[u8;0x70])->Option<u32> {None}
/// The live fighter of this owner's entry (the outgoing one of a request): the ready bank's live
/// member when `d` is that bank's physical group, never `+0x18[0]` of a PinCell0 window (Kirby or
/// Solo Popo at cell 0 whatever member is live); the descriptor's first kind otherwise. Current
/// native callback only, before the request changes the bank.
unsafe fn live_kind(entry:*const u8,d:&[u8;0x70])->u32 {bank_live(entry,d).unwrap_or_else(||descriptor_kind(d))}
/// Logical form of a bank member from the physical window words: a pair-lead
/// window's Popo is the normal pair, otherwise the member's single form.
fn bank_member_forms(d:&[u8;0x70],kind:u32)->Forms {
    let word=|at:usize|u32::from_le_bytes(d[at..at+4].try_into().unwrap());
    crate::engine::swap::body_bank::member_forms_in(word(0x14),[word(0x18),word(0x1c),word(0x20)],word(0x28),kind)
}
/// Logical fighter of the owned physical receipt, for value planning only (preload
/// and cycle identity). The bank's value receipt reports a Ready, non-departed bank's
/// last observed live member for exactly this physical descriptor without reading the
/// entry, so this is safe outside the native callback. Every other receipt (Mii,
/// saved, original, canonical) keeps its first kind. Request/settlement proofs use
/// the strict `bank_live` inside the owner's native callback instead.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn logical_kind(expected:&native::Snapshot)->u32 {
    super::body_bank::cached_kind(expected.binding.entry,0,&expected.descriptor)
        .unwrap_or_else(||descriptor_kind(&expected.descriptor))
}
#[cfg(not(feature="native-body-bank-alpha"))]
unsafe fn logical_kind(expected:&native::Snapshot)->u32 {descriptor_kind(&expected.descriptor)}
/// The live body is the immutable original: cursor None, only the origin's own
/// saved configuration, and either the origin descriptor itself, the Ice Climbers
/// kind rule, or (original bank on, EXCLUSIONS row 4) an owned ready bank whose
/// live member is the origin's ordinary kind although the window words differ
/// from the canonical origin descriptor. Value-only; never follows a pointer.
unsafe fn at_origin(origin:&native::Snapshot,expected:&native::Snapshot,active:Option<Configuration>,
    origin_saved:Option<Configuration>,cursor:Option<usize>)->bool {
    cursor.is_none() && active==origin_saved && (expected.descriptor==origin.descriptor
        || expected.descriptor==crate::engine::swap::entry_window::original_request(origin.descriptor,origin.original_kind)
        || (origin.original_kind==0x4b && descriptor_kind(&expected.descriptor)==0x4b)
        || (origin_saved.is_none() && logical_kind(expected)==origin.original_kind
            // WORKAROUND-DECISIONS row 8 (operator-approved root fix): the banked original is
            // recognised by its exact descriptor — every byte equal to the canonical origin except
            // the exchanged group words — not by kind alone.
            && crate::engine::mii::identity::verify_descriptor(&origin.descriptor,&expected.descriptor,
                crate::engine::mii::identity::ExpectedMutation::GroupExchange {
                    layout:crate::engine::mii::identity::GroupLayout::of(&expected.descriptor),
                    bodies:[(0,0);crate::engine::mii::identity::BODIES]}).is_ok()
            && crate::engine::swap::body_bank::original_bankable(origin.original_kind)
            && crate::runtime::roster::cycle_preload_config::snapshot().original_bank_enabled()))
}
/// The Player cycle's Mii admission (`cycle_members::player_admission`) from the owner's state: qualified as `begin`
/// serves the cycle (menu valid, capture finished, this epoch, a catalog), and the captured saved original is a Mii
/// origin. The menu shows it (`publish_admission`) and every Player walker walks it (P1-CYCLE).
fn cycle_admission(s:&State)->crate::engine::swap::cycle_members::MiiAdmission {
    let (Some(o),Some(catalog))=(s.owner.as_ref(),s.catalog.as_ref()) else {return crate::engine::swap::cycle_members::NO_MII;};
    let qualified=s.menu_valid && !s.capture && o.origin.binding.epoch==epoch();
    if !qualified {return crate::engine::swap::cycle_members::NO_MII;}
    let origin=match o.origin_saved {Some(c)=>PlayerOrigin::Mii(Origin(c.canonical())),None=>PlayerOrigin::NotMii};
    crate::engine::swap::cycle_members::player_admission(catalog,origin)
}
static ADMISSION:core::sync::atomic::AtomicU8=core::sync::atomic::AtomicU8::new(0);
fn publish_admission(admitted:crate::engine::swap::cycle_members::MiiAdmission) {
    ADMISSION.store(admitted.iter().enumerate().fold(0u8,|acc,(i,on)|acc|(u8::from(*on)<<i)),Ordering::Release);
}
pub(super) fn published_admission()->crate::engine::swap::cycle_members::MiiAdmission {
    let bits=ADMISSION.load(Ordering::Acquire);
    [bits&1!=0,bits&2!=0,bits&4!=0]
}
#[inline(never)]
pub(super) unsafe fn service(entry:*mut u8,owner:u32)->bool {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080001);
    if owner!=0 || !ENABLED.load(Ordering::Acquire) {return false;}
    // P1-CYCLE: the admission the menu shows is the one the walkers walk, refreshed every frame from this state.
    publish_admission(cycle_admission(&STATE.lock()));
    #[cfg(feature="native-body-bank-alpha")]
    {
        let bank=STATE.lock().bank;
        if let Some(b)=bank {
            if b.ticket.entry!=entry as usize {return false;}
            bank_service(entry,b);
            return true;
        }
    }
    let tracked=STATE.lock().track.clone();
    if let Some(t)=tracked {
        if t.ticket.entry!=entry as usize {return false;}
        if matches!(t.gate.phase(),Phase::Refused|Phase::Departed) {
            // A refused bank build still finished its native Reload; once native is
            // settled release INSTALL instead of retaining it for this cache. A track whose
            // refusal continued natively (row 7 / D-PTA1b) settles the same way.
            if matches!(t.gate.phase(),Phase::Refused) && settled(entry) && (t.bank || t.gate.native_only()) {
                bank_build_refused(&t,if t.bank {"bank build publication refused"}
                    else {"publication refused; native Reload settled"});
            }
            return true;
        }
        // Native is ready at the staged starting cell (`ReloadTarget`): the target's own cell
        // of a PinCell0 window, 0 for a fighter request.
        if t.gate.can_complete(t.ticket) && ready_descriptor(entry,&t.descriptor)
            && t.reload_target().ready_at(read::<u8>(entry,0x5918)) {complete(entry,&t);}
        else if t.bank && t.gate.can_complete(t.ticket) && (bank_failed(entry,0)
            || (settled(entry) && !ready_descriptor(entry,&t.descriptor))) {
            bank_build_refused(&t,"native settled without the staged bank");
        }
        return true;
    }
    let needs_capture=STATE.lock().capture;
    if needs_capture {capture(entry);}
    #[cfg(feature="native-body-bank-alpha")]
    mii_group_record_publication(entry);
    if STATE.lock().request.is_none() {return false;}
    begin(entry)
}

/// A planned Mii group's CharInfo goes onto this owner's selector record before
/// the bank requests the group's keys (the key request copies the record at
/// `R+0x68`, MII-GROUPS-DESIGN §1.4/§5.2). Idle owner only. A busy refusal is
/// retried next frame at no cost; a read/write race counts against the group; a
/// refusal that cannot clear while the owner idles fails the group at once
/// (`crate::engine::swap::body_bank::mii_publish`). The refusal reason is noted once.
#[cfg(feature="native-body-bank-alpha")]
#[inline(never)] // stream/stack: its own frame, not `service`'s
unsafe fn mii_group_record_publication(entry:*mut u8) {
    use crate::engine::swap::body_bank::mii_publish::PublishRefusal;
    let Some((group,identity,c))=super::body_bank::pending_mii_publication(0) else {return;};
    if INSTALL.active() || !mii_backend::ready() {return;}
    let outcome=native::prefetch_publish(entry as usize,c,identity);
    match outcome {
        Ok(())=>trace!("[MiiOrdinaryPlayer] Mii group record published group={} kind={:#x} index={} hat={}",
            group,identity.kind(),identity.index,identity.hat()),
        Err(PublishRefusal::Busy)=>return,
        Err(_)=>{}
    }
    if let Some(reason)=super::body_bank::mii_charinfo_published(0,group,identity,outcome) {note(reason);}
}

#[inline(never)]
unsafe fn complete(entry:*const u8,t:&Track) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080004);
            let result=native::completed(t);
            let mut s=STATE.lock();
            if !s.track.as_ref().is_some_and(|now|now.ticket==t.ticket) {return;}
            match result {
                Ok(now) if epoch()==t.ticket.epoch => {
                    let Some(o)=s.owner.as_mut() else {
                        s.track.as_mut().unwrap().gate.refuse();return;
                    };
                    // Native completion committed owner/foreign-kind evidence under G -> STATE.
                    debug_assert_eq!(o.expected,now);
                    s.track=None;s.request=None;s.restore=None;
                    drop(s);
                    // The delivered fighter: the member at the staged starting cell for a bank
                    // build (never `+0x18[0]`, a pinned Kirby/Solo Popo), the request's kind otherwise.
                    let target=t.reload_target().kind;
                    #[cfg(feature="mii-hud-portrait-alpha")]
                    publish_mii_presentation(entry,0,t.ticket.epoch,target,
                        if t.destination.target==mii_cycle::Target::Original {
                            super::PortraitIntent::Original {
                                original_kind: super::original_kind(super::Role::Player).unwrap_or(u32::MAX),
                            }
                        } else if let Some(c)=t.publish {
                            super::PortraitIntent::Exact(c)
                        } else {
                            super::PortraitIntent::NotMii
                        });
                    #[cfg(not(feature="mii-hud-portrait-alpha"))]
                    publish_mii_presentation(entry,0,t.ticket.epoch,target);
                    crate::runtime::roster::cycle_preload::foreground_settled();
                    let released=settle_outcome(Outcome::Applied {receipt:receipt_of(&now,target),lease:t.token},t.ticket,None,true);
                    crate::runtime::roster::cycle_preload::completed(0,false,target);
                    state_for(Role::Player).lock().cursor=t.destination.cursor;
                    let forms=if t.bank {bank_member_forms(&t.descriptor,target)}
                        else {descriptor_forms(&t.descriptor)};
                    publish_partners(Role::Player,forms);
                    remember_owned(Role::Player,t.ticket.epoch,target,forms,
                        t.publish,t.destination.cursor,t.destination.target==mii_cycle::Target::Original);
                    if let Some(fighter)=TrioFighter::from_kind(target) {
                        Role::Player.sync_selection(fighter);
                    }
                    trace!("[MiiOrdinaryPlayer] ready ticket={:?} target={:#x} index={} resource={:02x?} released={} saved_files_written=false",
                        t.ticket,target,now.binding.record_index,
                        &now.record[88..104],released);
                    notify(Role::Player,match t.destination.target {
                        mii_cycle::Target::Saved(_)=>"Saved Mii applied",
                        mii_cycle::Target::Original=>"Original fighter restored",
                        mii_cycle::Target::Fighter(_)=>"Player fighter applied",
                    }.into());
                }
                _ if t.bank => {drop(s);bank_build_refused(t,"terminal bank ready comparison refusal");}
                _ => {
                    let rec={let now_t=s.track.as_mut().unwrap();now_t.gate.refuse();
                        note_recovering(now_t,"terminal ready comparison refusal; INSTALL retained until exact destruction",None)};
                    drop(s);trace_recovering(rec);note("terminal ready comparison refusal; INSTALL retained");
                }
            }
}

/// A bank build is a cache load with native already settled. Never adopt the new
/// physical receipt: drop authority (no native writes) and release INSTALL.
fn bank_build_refused(t:&Track,reason:&'static str) {
    let mut s=STATE.lock();
    if !s.track.as_ref().is_some_and(|now|now.ticket==t.ticket) {return;}
    s.track=None;s.owner=None;s.menu_valid=false;s.request=None;s.restore=None;
    drop(s);
    native::retire_authority(0,t.ticket.entry);
    let released=settle_outcome(Outcome::NativeSettledUnowned {reason,lease:t.token},t.ticket,None,t.gate.published());
    trace!("[MiiOrdinaryPlayer] bank build refused ticket={:?} INSTALL_released={} reason={}",t.ticket,released,reason);
}

#[inline(never)]
unsafe fn capture(entry:*mut u8) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080002);
    let kind=read::<u32>(entry,0x18);
    let captured=STATE.lock().owner.as_ref().map(|o|o.origin.original_kind);
    let Some(original)=captured.or_else(||original_kind(Role::Player)) else {return;};
    // An owned bank can display any member while +0x18 remains member zero.
    // Prove native readiness here; capture_origin below compares the physical
    // ownership receipt and finishes a stale receipt as refused, not busy.
    let ready_now=match captured {
        Some(_)=>ready_forms(entry,0).is_some() || ready_single_mii(entry,0).is_some(),
        None=>ready(entry,kind),
    };
    if (captured.is_none() && original!=kind) || !ready_now
        || INSTALL.active() || !mii_backend::ready() {return;}
    let (catalog,at)={let s=STATE.lock();(s.catalog.as_ref().map(Arc::clone),s.capture_at)};
    let Some(catalog)=catalog else {return;};
    // Keep the arm's generation/opening through native capture; never adopt a later lifetime.
    let result=if at==(epoch(),mii_saved_alpha::menu_epoch()) {
        native::capture_origin(entry as usize,at.0,original,&catalog,at.1)
    } else {Err(native::helpers::CaptureError::Refused("catalog capture retired"))};
    match result {
        Ok(())=>note("catalog owner qualified; immutable origin retained"),
        Err(native::helpers::CaptureError::Busy)=>{},
        Err(native::helpers::CaptureError::Refused(reason))=>{
            let mut s=STATE.lock();
            if s.capture_at==at {s.capture=false;}
            drop(s);note(reason);
        }
    }
}

#[inline(never)]
unsafe fn begin(entry:*mut u8)->bool {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080003);
    if !permitted() {return false;}
    let pending={
        let s=STATE.lock();
        let admitted=cycle_admission(&s);
        s.owner.as_ref().zip(s.request).map(|(o,request)|
             (o.origin,o.origin_saved,o.expected,o.active,o.cursor,s.catalog.as_ref().map(Arc::clone),request,s.restore,admitted))
    };
    let Some((origin,origin_saved,expected,active,cursor,Some(catalog),request,restore,admitted))=pending else{return false;};
    if !request.current(mii_saved_alpha::menu_epoch(),active.map(|c|c.canonical())) {
        clear_request(request);note("menu-close request expired before preparation");
        if request.direct_kind().is_some() {notify(Role::Player,"Character choice expired; select it again".into());}
        return false;
    }
    if !ready_descriptor(entry,&expected.descriptor) {return false;}
    let menu=setting(&MENU);
    // Only catalog/target-row qualification may skip a numbered choice. Current
    // owner, native publication and completion failures never select a fallback.
    let qualify=|step:mii_cycle::Step|->Result<(Option<Configuration>,[u8;0x70]),&'static str> {
        match step.target {
            mii_cycle::Target::Original=>{
                if let Some(c)=origin_saved {native::target_rows(c)?;}
                // P-HOT B: an origin captured after H3 carries the start-up window; the original is its single form.
                let mut descriptor=crate::engine::swap::entry_window::original_request(origin.descriptor,origin.original_kind);
                if origin.original_kind==0x4b {
                    let forms=requested_forms_at(&menu,Role::Player,0x4b,None);
                    descriptor[0x14..0x18].copy_from_slice(&forms.count.to_le_bytes());
                    for (i,k) in forms.kinds.iter().enumerate() {
                        descriptor[0x18+i*4..0x1c+i*4].copy_from_slice(&k.to_le_bytes());
                    }
                    descriptor[0x28..0x2c].copy_from_slice(&forms.actor_count.to_le_bytes());
                }
                Ok((step.target.publish(origin_saved,None)?,descriptor))
            },
            mii_cycle::Target::Saved(choice)=>{
                let target=catalog.configuration(choice)
                    .ok_or("selected saved configuration unavailable")?;
                admit_hat(origin.layout,origin.binding.owner as usize,origin.scalars.e.hat,target)?;
                native::target_rows(target)?;
                Ok((step.target.publish(origin_saved,Some(target))?,stage_saved(&origin,target)))
            }
            mii_cycle::Target::Fighter(kind)=>{
                let forms=restore.map_or_else(||requested_forms_at(&menu,Role::Player,kind,step.cursor),|v|v.forms);
                let mut descriptor=origin.descriptor;
                descriptor[0x14..0x18].copy_from_slice(&forms.count.to_le_bytes());
                for (i,k) in forms.kinds.iter().enumerate() {
                    descriptor[0x18+i*4..0x1c+i*4].copy_from_slice(&k.to_le_bytes());
                }
                descriptor[0x28..0x2c].copy_from_slice(&forms.actor_count.to_le_bytes());
                descriptor[0x43]=0;
                descriptor[0x46..0x48].copy_from_slice(&origin.binding.record_index.to_le_bytes());
                Ok((step.target.publish(origin_saved,None)?,descriptor))
            }
        }
    };
    let resolved=if let Some(value)=restore {
        let original=TrioFighter::from_kind(origin.original_kind).is_some_and(|fighter|
            value.matches_original(fighter,descriptor_forms(&origin.descriptor),origin_saved));
        let step=mii_cycle::Step {cursor:value.cursor,target:if original {mii_cycle::Target::Original}
            else if let Some(c)=value.configuration {mii_cycle::Target::Saved(crate::engine::mii::selection::Choice {
                canonical:c.canonical(),slot_id:None,source:crate::engine::mii::selection::Source::Preference})}
            else {mii_cycle::Target::Fighter(value.fighter.kind())}};
        let result=if original {qualify(step)} else if let Some(c)=value.configuration {
            admit_hat(origin.layout,origin.binding.owner as usize,origin.scalars.e.hat,c)
                .and_then(|_|native::target_rows(c))
                .map(|_|(Some(c),stage_saved(&origin,c)))
        } else {qualify(step)};
        match result {Ok(value)=>Some((step,value)),Err(reason)=>{clear_request(request);note(reason);return false;}}
    } else if request.refresh_partners() {
        // A completed generic ordinary swap updates the roster slot, but does
        // not grant this adapter a cursor. Use the same proven live slot as the
        // refresh trigger; the normal snapshot/INSTALL checks still gate commit.
        let slot=state_for(Role::Player).lock().cursor;
        mii_cycle::refresh_partners(logical_kind(&expected),origin.original_kind,slot,true)
            .and_then(|step|match qualify(step) {
                Ok(value)=>Some((step,value)),
                Err(reason)=>{note(reason);None}
            })
    } else if let Some(kind)=request.direct_kind() {
        let result=mii_cycle::direct(kind,&catalog,&menu::mii_preferences(),
            crate::engine::mii::selection::Role::Player,
            origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())))
            .ok_or("selected Player character unavailable")
            .and_then(|step|qualify(step).map(|value|(step,value)));
        match result {Ok(value)=>Some(value),Err(reason)=>{
            clear_request(request);note(reason);
            notify(Role::Player,"This Player character cannot be loaded".into());return false;
        }}
    } else if let Some(choice)=request.choice() {
        // A menu selection applies that exact configuration and retains order.
        let step=mii_cycle::Step{target:mii_cycle::Target::Saved(choice),cursor};
        match qualify(step) {
            Ok(value)=>Some((step,value)),
            Err(reason)=>{
                clear_request(request);note(reason);
                notify(Role::Player,"This Player Mii configuration cannot be loaded".into());
                return false;
            }
        }
    } else {
        let identity=active.map_or(Identity::Fighter(logical_kind(&expected)),
            |c|Identity::Saved(c.canonical()));
        let live=if at_origin(&origin,&expected,active,origin_saved,cursor) {
            Live::Original
        } else {Live::Numbered{identity,cursor}};
        // P1-CYCLE: the cycle the menu numbers (`cycle_members` with this owner's admission), not the unfiltered roster.
        mii_cycle::next_qualified_player_with_ice_climbers(&crate::engine::swap::cycle_members::cycle_members(menu.player_roster,admitted),&catalog,&menu::mii_preferences(),
            origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())),live,
            menu.solo_popo_original!=menu.solo_popo_player_cycle,|step|match qualify(step) {
                Ok(value)=>Some(value),
                Err(reason)=>{
                    if let mii_cycle::Target::Saved(choice)=step.target {
                        trace!("[MiiOrdinaryPlayer] skipped unqualified cycle choice class={:?} slot={:?} reason={}",
                            choice.canonical.class,choice.slot_id,reason);
                        notify_args(Role::Player,format_args!("Player {:?} choice skipped",choice.canonical.class));
                    }
                    None
                }
            })
    };
    let Some((destination,(publish,descriptor)))=resolved else {
        clear_request(request);
        if !request.refresh_partners() {note("Player cycle has no qualified choice");}
        return false;
    };
    let current=match native::snapshot(entry as usize,epoch(),origin.original_kind) {
        Ok(now) if native::same_owned(&now,&expected)=>now,
        Err(native::helpers::BUSY)=>return false,
        Err(reason)=>{clear_request(request);note(reason);return false;}
        Ok(_)=>{clear_request(request);note("preflight current configuration changed");return false;}
    };
    // A ready owned bank whose live member is already this ordinary target is the
    // same visible fighter: an order update only, never a switch or INSTALL.
    let bank_same=bank_live(entry,&current.descriptor).is_some_and(|live|
        live==descriptor_kind(&descriptor) && descriptor_forms(&descriptor)==bank_member_forms(&current.descriptor,live));
    // Identical configuration/group needs only an order update, never INSTALL.
    if (bank_same || (descriptor==current.descriptor && ready(entry,descriptor_kind(&descriptor))))
        && crate::engine::mii::role_owner::payload_already_active(active,publish,
            restore.is_some() || destination.target==mii_cycle::Target::Original) {
        let mut s=STATE.lock();
        if !s.busy() && s.request==Some(request)
            && request.current(mii_saved_alpha::menu_epoch(),active.map(|c|c.canonical())) {
            if let Some(o)=s.owner.as_mut().filter(|o|
                o.expected==expected && o.active==active && o.cursor==cursor) {
                let Ok(registry)=native::final_registry(current.helpers,None) else{return false;};
                o.cursor=destination.cursor;s.request=None;s.restore=None;drop(registry);drop(s);
                state_for(Role::Player).lock().cursor=destination.cursor;
                publish_partners(Role::Player,descriptor_forms(&descriptor));
                remember_owned(Role::Player,current.binding.epoch,descriptor_kind(&descriptor),descriptor_forms(&descriptor),
                    active,destination.cursor,destination.target==mii_cycle::Target::Original);
                if let Some(fighter)=TrioFighter::from_kind(descriptor_kind(&descriptor)) {
                    Role::Player.sync_selection(fighter);
                }
                if !request.refresh_partners() {notify(Role::Player,"Player choice already active".into());}
            }
        }
        return false;
    }
    // A changed hat cannot reuse the retained O resource key. Skip same-class
    // and let publish() take the fresh-O constructor. Equal hats keep the proof.
    let hat_change=match (active,publish) {
        (Some(before),Some(after))=>before.tokens().hat!=after.tokens().hat,
        _=>false,
    };
    // A bank-built Mii window (`+0x591f == 4`) has no retained canonical resource pair
    // for `prepare_same_class` (WP-M2): its same-class change takes the fresh-O path.
    let mii_window=MiiClass::from_kind(descriptor_kind(&descriptor)).is_some()
        && read::<u8>(current.binding.entry as *const u8,0x591f)==crate::engine::mii::ordinary_player::MII_BANK_WINDOW_FLAGS;
    let same_class=if hat_change || mii_window {None} else {match (active,publish) {
        (Some(before),Some(after)) if before.canonical().class==after.canonical().class=>
            match native::prepare_same_class(current,before,after) {
                Ok(proof)=>Some(proof),
                Err(reason)=>{clear_request(request);note(reason);return false;}
            },
        _=>None,
    }};
    // Ordinary numbered/direct/restored fighters may use this owner's same-entry
    // bank, and so may an ordinary single-body original while `original_bank` is
    // on (EXCLUSIONS row 4): its return is then a window reuse or a group swap
    // instead of the canonical Reload. Saved, same-class and every Mii keep the
    // phase-6 path. The bank planner owns eligibility (Miis and native groups stay
    // canonical; Kirby parks only copy-free) and budget; upcoming kinds come from
    // the real mixed cycle after this destination.
    // rc.6 warm exit: out of a live Mii group whose publication this owner kept, the window it displaced is warm.
    #[cfg(feature="native-body-bank-alpha")]
    let exit=active.is_some() && publish.is_none()
        && STATE.lock().mii_exit.is_some_and(|x|x.mii.ticket.epoch==current.binding.epoch
            && x.adopted==expected);
    #[cfg(feature="native-body-bank-alpha")]
    let plan={
        let after=match destination.target {
            mii_cycle::Target::Fighter(kind) if publish.is_none() && same_class.is_none()
                && descriptor_kind(&descriptor)==kind =>
                Some(Live::Numbered {identity:Identity::Fighter(kind),cursor:destination.cursor}),
            mii_cycle::Target::Original if publish.is_none() && same_class.is_none() && origin_saved.is_none()
                && descriptor_kind(&descriptor)==origin.original_kind
                && crate::engine::swap::body_bank::original_bankable(origin.original_kind)
                && crate::runtime::roster::cycle_preload_config::snapshot().original_bank_enabled() => Some(Live::Original),
            // A saved/preset Mii with no same-class proof (another class, or a hat
            // change) may be served by its own one-member group (MII-GROUPS-DESIGN
            // §5.1 G10): the bank planner admits only a reachable group carrying
            // exactly this identity; everything else stays the phase-6 path.
            mii_cycle::Target::Saved(choice) if publish.is_some() && same_class.is_none()
                && MiiClass::from_kind(descriptor_kind(&descriptor)).is_some()
                && crate::runtime::roster::cycle_preload_config::snapshot().mii_groups_enabled()
                && super::body_bank::reachable_mii_group(0,descriptor_kind(&descriptor)).is_some_and(|(_,c)|Some(c)==publish)
                // Interim rule (WP-M2): the native Mii-setup preflight must hold before
                // any group mutation; otherwise this target stays on the canonical path.
                && match super::body_bank::mii_setup_admitted(current.binding.entry,0,descriptor_kind(&descriptor)) {
                    Ok(())=>true,
                    Err(reason)=>{note(format_args!("[MiiOrdinaryPlayer] Mii group canonical before mutation: {}",reason));false}
                } =>
                Some(Live::Numbered {identity:Identity::Saved(choice.canonical),cursor:destination.cursor}),
            _=>None,
        };
        after.map(|after| {
            let upcoming:Vec<u32>=crate::runtime::roster::cycle_preload::mixed_targets(&menu,0,
                origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())),
                after,&catalog,origin_saved).iter().flatten()
                .filter(|target|target.configuration.is_none()).map(|target|target.kind).collect();
            if exit {super::body_bank::plan_mii_exit(entry,0,descriptor,&upcoming)}
            else {super::body_bank::plan(entry,0,descriptor,&upcoming)}
        })
    };
    // Physical bytes INSTALL copies (a bank build replaces the group words) and
    // whether this is a warm member switch with no Reload at all.
    #[cfg(feature="native-body-bank-alpha")]
    let (physical,bank_build,reuse)=plan.map_or((descriptor,false,false),
        |p|(p.descriptor,!p.reuse && p.is_bank(),p.reuse));
    #[cfg(not(feature="native-body-bank-alpha"))]
    let (physical,bank_build,reuse)=(descriptor,false,false);
    if !permitted(){return false;}
    let Some(pose)=capture_reload_pose(entry,0) else {
        clear_request(request);note("Player position not ready before reload");return false;
    };
    let Some(token)=INSTALL.try_begin(true) else{return false;};
    // The preflight predates INSTALL ownership; freeze all foreign bytes/provenance anew.
    if native::snapshot(entry as usize,current.binding.epoch,origin.original_kind)!=Ok(current) {
        refused_before_submit(token,"Player preflight changed before INSTALL ownership");return false;
    }
    let track={
        let mut s=STATE.lock();
        if s.busy() || s.request!=Some(request) || epoch()!=current.binding.epoch
            || !request.current(mii_saved_alpha::menu_epoch(),
                s.owner.as_ref().and_then(|o|o.active.map(|c|c.canonical())))
            || !s.owner.as_ref().is_some_and(|o|
                o.expected==expected && o.active==active && o.cursor==cursor) {
            drop(s);refused_before_submit(token,"ordinary pre-submit refusal");return false;
        }
        let sequence=s.next;
        let Some(next)=sequence.checked_add(1) else {
            s.request=None;drop(s);refused_before_submit(token,"ordinary ticket exhausted");return false;
        };
        let ticket=Ticket{epoch:epoch(),entry:entry as usize,sequence};
        let registry=match native::final_registry(current.helpers,Some(token)) {
            Ok(registry)=>registry,
            Err(_)=>{drop(s);refused_before_submit(token,"Player source provenance changed before submit");return false;}
        };
        s.next=next;s.request=None;
        if reuse {
            // Published before the native request; settled only by bank_service.
            #[cfg(feature="native-body-bank-alpha")]
            {s.bank=Some(BankTrack {ticket,token,before:current,logical:descriptor_kind(&descriptor),
                destination,departed:false,publish,descriptor,hat_change,published:None,exit:exit && reuse});}
            drop(registry);None
        } else {
            let t=Track {ticket,gate:Gate::new(ticket).unwrap(),token,installing:true,retained_o:false,recovering:None,
                data:Arc::new(Prepared {before:current,origin_scalars:origin.scalars,
                    descriptor:physical,publish,destination,same_class,bank:bank_build})};
            s.track=Some(t.clone());drop(registry);Some(t)
        }
    };
    let Some(track)=track else {
        #[cfg(feature="native-body-bank-alpha")]
        return submit_bank(entry,plan,pose);
        #[cfg(not(feature="native-body-bank-alpha"))]
        return false;
    };
    // Outgoing FE is still untouched. INSTALL copies the separately staged 0x70-byte descriptor.
    let staged=native_fighter_install::Descriptor::from_bytes(&track.descriptor);
    training::clear_session_state();
    trace!("[MiiOrdinaryPlayer] submit ticket={:?} outgoing={:#x} target={:#x} index={} bank_build={} resource_before={:02x?} saved_files_written=false",
        track.ticket,live_kind(entry,&current.descriptor),track.reload_target().kind,current.binding.record_index,
        track.bank,&current.record[88..104]);
    #[cfg(feature="native-body-bank-alpha")]
    let submitted=match plan {
        Some(p) if track.bank => super::body_bank::submit(entry,0,p,pose),
        _ => {reload_at_pose(base(),entry,&staged,pose);true}
    };
    #[cfg(not(feature="native-body-bank-alpha"))]
    let submitted={reload_at_pose(base(),entry,&staged,pose);true};
    if !submitted {
        // The bank refused before any native write: no Reload was issued.
        let owned={
            let mut s=STATE.lock();
            let owned=s.track.as_ref().is_some_and(|t|t.ticket==track.ticket);
            if owned {s.track=None;}
            owned
        };
        if owned {settle_refusal(Refusal {reason:"Player bank build refused before native Reload",lease:track.token},Some(track.ticket));}
        note("bank build refused before native Reload; request cleared");
        return false;
    }
    if let Some(current)=STATE.lock().track.as_mut().filter(|t|t.ticket==track.ticket) {
        current.installing=false;
    }
    true
}

/// Warm member switch after the BankTrack is published, INSTALL and pose owned.
/// A refusal performed no native write; release only this token's ownership.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn submit_bank(entry:*mut u8,plan:Option<super::body_bank::Plan>,
    pose:super::native_reload_pose::ReloadPose)->bool {
    let bank=STATE.lock().bank;
    let Some(b)=bank.filter(|b|b.ticket.entry==entry as usize) else {return false;};
    // A Mii group switch publishes the exact record before the native request
    // (MII-GROUPS-DESIGN §5.2), with the group's own identity as the hat proof. The
    // publication is recorded in the track as it is written; every refusal from here on
    // (this one, Phase B's rollback, a settle refusal) puts it back in `bank_refused`.
    if let Some(c)=b.publish {
        let Some((identity,group_c))=super::body_bank::reachable_mii_group(0,b.logical) else {
            bank_refused(b,"no reachable Mii group for the published configuration",false);
            return false;
        };
        if group_c!=c {
            bank_refused(b,"reachable Mii group was planned for another configuration",false);
            return false;
        }
        match native::bank_publish(&b,c,identity.hat()) {
            Ok(())=>trace!("[MiiOrdinaryPlayer] bank publish ticket={:?} target={:#x} index={} hat={} charinfo_written=true scalars_written=true",
                b.ticket,b.logical,b.before.binding.record_index,identity.hat()),
            Err(reason)=>{bank_refused(b,reason,false);return false;}
        }
    }
    // The outgoing member, read while the bank is still Ready (`switch` clears its observation).
    let outgoing=live_kind(entry,&b.before.descriptor);
    let switched=match plan {
        Some(p) if p.reuse && p.target==b.logical => super::body_bank::submit(entry,0,p,pose),
        _ => false,
    };
    if !switched {
        bank_refused(b,"bank member switch refused before the native request",false);
        return false;
    }
    training::clear_session_state();
    trace!("[MiiOrdinaryPlayer] bank switch ticket={:?} outgoing={:#x} target={:#x} index={} mii={}",
        b.ticket,outgoing,b.logical,b.before.binding.record_index,b.publish.is_some());
    true
}

/// Settle a member switch in this owner's ready callback. Pending native work is
/// never forced; a refusal releases INSTALL only once native is settled.
#[cfg(feature="native-body-bank-alpha")]
#[inline(never)] // stream/stack: its locals live in its own frame, not in every `service` call's
unsafe fn bank_service(entry:*mut u8,b:BankTrack) {
    let live=super::body_bank::ready_kind(entry,0);
    if b.departed {
        if live.is_some() || settled(entry) {bank_refused(b,"scene departed during bank switch",false);}
        return;
    }
    // The bank module may still report a ready current body after its identity proof
    // failed; a same-kind new body generation is never adopted as success. `failed`
    // is set only after native settlement, so releasing INSTALL here forces nothing.
    if super::body_bank::failed(entry,0) {
        bank_refused(b,"bank identity failed after native settlement",true);
        return;
    }
    match live {
        Some(kind) if kind==b.logical => bank_settle(entry,b),
        // The bank module returned to Ready on another member with the same owned
        // physical group and bodies: nothing to adopt; keep expected, release INSTALL.
        Some(_) => bank_refused(b,"bank settled on another member",false),
        // Native settled without a ready owned bank: a different generation.
        None if settled(entry) => bank_refused(b,"native settled without the requested bank member",true),
        None => {},
    }
}

#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_settle(entry:*mut u8,b:BankTrack) {
    // rc.6 warm exit: a Mii group switch's publication, read before the settle drops its track, is kept for the
    // exit to the window it displaced; that exit settles through the kept record.
    let published=STATE.lock().bank.filter(|t|t.ticket==b.ticket && t.token==b.token).and_then(|t|t.published);
    let exit=STATE.lock().mii_exit;
    let settled=match if b.exit {native::exit_settled(&b,exit)} else {native::bank_settled(&b)} {
        Ok(settled)=>settled,
        Err(native::BankRefusal::Busy)=>return,
        Err(native::BankRefusal::Kept(reason))=>{bank_refused(b,reason,false);return;}
        Err(native::BankRefusal::Diverged(reason))=>{bank_refused(b,reason,true);return;}
    };
    let now=settled.now;let exact=settled.exact;
    STATE.lock().mii_exit=match (exact,published) {
        (Some(c),Some((published_c,published))) if published_c==c=>Some(MiiExit {mii:b,configuration:c,published,adopted:now}),
        _=>None,
    };
    let logical=b.logical;
    // A Mii group switch whose exact proof was adopted publishes its configuration
    // (the exact HUD portrait); an ordinary switch publishes the live kind. The intent
    // is the active configuration itself, never the publication attempt.
    #[cfg(feature="mii-hud-portrait-alpha")]
    publish_mii_presentation(entry,0,b.ticket.epoch,logical,
        exact.map_or(super::PortraitIntent::NotMii,super::PortraitIntent::Exact));
    #[cfg(not(feature="mii-hud-portrait-alpha"))]
    publish_mii_presentation(entry,0,b.ticket.epoch,logical);
    crate::runtime::roster::cycle_preload::foreground_settled();
    let released=settle_outcome(Outcome::Applied {receipt:receipt_of(&now,logical),lease:b.token},b.ticket,None,true);
    crate::runtime::roster::cycle_preload::completed(0,false,logical);
    state_for(Role::Player).lock().cursor=b.destination.cursor;
    let forms=bank_member_forms(&now.descriptor,logical);
    let original=b.destination.target==mii_cycle::Target::Original;
    publish_partners(Role::Player,forms);
    remember_owned(Role::Player,b.ticket.epoch,logical,forms,exact,b.destination.cursor,original);
    if let Some(fighter)=TrioFighter::from_kind(logical) {Role::Player.sync_selection(fighter);}
    trace!("[MiiOrdinaryPlayer] bank ready ticket={:?} target={:#x} physical={:#x} index={} original={} mii={} resource_pair={:?} released={}",
        b.ticket,logical,descriptor_kind(&now.descriptor),now.binding.record_index,original,exact.is_some(),settled.resource_pair,released);
    notify(Role::Player,crate::engine::mii::ordinary_player::bank_notice(true,exact.is_some(),original).into());
}

/// Terminal BankTrack refusal. The pending track, request and restore go; the owner
/// (origin and expected) is kept and no authority is retired, whatever the reason
/// (WP-M2 cascade guard, `bank_refusal_effects`): B124PRE D-MII2 followed from a
/// `diverged` refusal dropping the owner (`original assignment not captured`). The
/// physical receipt is never adopted here.
///
/// MII-PHASEB: it is also the one compensation point of the switch-time publication. Before
/// INSTALL goes, a recorded `bank_publish` is put back and read back under G
/// (`native::bank_restore`); the kept receipt then describes native again, so the next
/// request is admitted (Surface s2 #192/#409/#411 refused every request after the M3
/// refusal). Only native's own pair from a crossed Mii setup is adopted into the receipt.
/// A restore that cannot verify (the exchange made the publication live, or anything else
/// moved) is reported and the refusal is the diverged class. A restore that met the
/// transient `helpers::BUSY` before writing keeps the track and INSTALL: the next
/// `bank_service` callback reaches this arm again and retries.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_refused(b:BankTrack,reason:&'static str,diverged:bool) {
    let effects=crate::engine::mii::ordinary_player::bank_refusal_effects(diverged);
    let (published,departed)=match STATE.lock().bank {
        Some(now) if now.ticket==b.ticket && now.token==b.token=>(now.published,now.departed),
        _=>return,
    };
    // Lock order G -> STATE: the restore takes G and STATE itself; STATE is not held across it.
    let restored=published.map(|(c,p)|native::bank_restore(&b,c,p));
    if matches!(restored,Some(Err(why)) if why==native::helpers::BUSY) {
        trace!("[MiiOrdinaryPlayer] bank publication restore deferred ticket={:?} why=busy reason={}",b.ticket,reason);
        return;
    }
    let mut s=STATE.lock();
    if !s.bank.is_some_and(|now|now.ticket==b.ticket && now.token==b.token) {return;}
    if effects.drop_bank {s.bank=None;}
    if effects.drop_request {s.request=None;s.restore=None;}
    if !effects.keep_owner {s.owner=None;s.menu_valid=false;}
    // Native's own pair goes into the receipt only of the owner the restore compared against.
    if let (Some(Ok(r)),Some(o))=(&restored,s.owner.as_mut()) {
        if r.receipt==crate::engine::mii::ordinary_player::RestoreReceipt::NativePairAdopted
            && native::same_owned(&o.expected,&b.before) {o.expected.record=r.now.record;}
    }
    let owner_kept=s.owner.is_some();
    drop(s);
    match &restored {
        Some(Ok(r))=>trace!("[MiiOrdinaryPlayer] bank publication restored ticket={:?} verified=true receipt={:?} reason={}",b.ticket,r.receipt,reason),
        Some(Err(why))=>trace!("[MiiOrdinaryPlayer] bank publication NOT restored ticket={:?} departed={} why={} reason={}",b.ticket,departed,why,reason),
        None=>{}
    }
    // A publication that could not be put back leaves the owner's receipt stale: diverged
    // (owner kept, NativeSettledUnowned), unless the scene itself departed.
    let diverged=diverged || (matches!(restored,Some(Err(_))) && !departed);
    if effects.retire_authority {native::retire_authority(0,b.ticket.entry);}
    // WP5c: diverged = native settled on a body this owner cannot adopt; otherwise nothing native
    // ran, or what ran is compensated (the bank's Phase B rollback, the publication restore above).
    let released=if diverged {settle_outcome(Outcome::NativeSettledUnowned {reason,lease:b.token},b.ticket,None,true)}
        else {settle_refusal(Refusal {reason,lease:b.token},Some(b.ticket))};
    trace!("[MiiOrdinaryPlayer] bank switch refused ticket={:?} diverged={} owner_kept={} INSTALL_released={} reason={}",
        b.ticket,diverged,owner_kept,released,reason);
}

/// The bank module disproved this owner's operation after native settlement.
#[cfg(feature="native-body-bank-alpha")]
fn bank_failed(entry:*const u8,owner:u32)->bool {super::body_bank::failed(entry,owner)}
#[cfg(not(feature="native-body-bank-alpha"))]
fn bank_failed(_entry:*const u8,_owner:u32)->bool {false}
/// Native loader idle with no pending/active change: nothing remains to force.
unsafe fn settled(entry:*const u8)->bool {
    read::<u8>(entry,0x5920)==6 && read::<u8>(entry,0x591c)==14 && read::<u8>(entry,0x591a)==0xff
        && read::<u8>(entry,0x5934)==0 && read::<u8>(entry,0x5935)==0
}

/// WP5c: the one INSTALL release site for Player tracks after their first native effect. Every
/// terminal path builds the `transition::Outcome` that names it; the lease travels inside the
/// outcome, so a path can neither release twice nor forget to release, and this emits the single
/// `[Txn] outcome=` line the Eden rows grep.
fn settle_outcome(outcome:Outcome<InstallToken>,ticket:Ticket,loader:Option<u8>,published:bool)->bool {
    let name=outcome.name();let reason=outcome.reason();
    let released=finish_install(outcome.into_lease(),name);
    trace!("[Txn] outcome={} role=player ticket={:?} reason={} loader={:?} published={} INSTALL_released={} INSTALL_active={}",
        name,ticket,reason,loader,published,released,INSTALL.active());
    released
}
/// WP5c: a refusal before any native effect (preflight/submit): nothing to compensate, the lease
/// comes straight back (`transition::Refusal`).
fn settle_refusal(refusal:Refusal<InstallToken>,ticket:Option<Ticket>)->bool {
    let released=finish_install(refusal.lease,refusal.reason);
    trace!("[Txn] outcome=Refused role=player ticket={:?} reason={} INSTALL_released={} INSTALL_active={}",
        ticket,refusal.reason,released,INSTALL.active());
    released
}
fn refused_before_submit(token:InstallToken,reason:&'static str)->bool {
    settle_refusal(Refusal {reason,lease:token},None)
}
/// `kind` is the delivered fighter (the transaction's target or the bank's live member), never
/// `+0x18[0]`: a PinCell0 window holds Kirby or Solo Popo there whatever member is live.
fn receipt_of(now:&native::Snapshot,kind:u32)->ReadyReceipt {
    ReadyReceipt {identity:TxnIdentity {entry:now.binding.entry,owner:now.binding.owner,epoch:now.binding.epoch,
        descriptor:now.descriptor,index:now.binding.record_index,..TxnIdentity::ZERO},
        kind}
}
/// WP5c: a committed track whose native side cannot be adopted enters a named recovery exactly
/// once (`AwaitDestruction`: INSTALL stays owned until exact destruction, as before) instead of
/// an anonymous `Refused` stall. Exact-Mii tracks (`RecoveryPlan::MII_PAYLOAD`) have no native
/// continuation, so this is their only recovery.
fn note_recovering(t:&mut Track,reason:&'static str,loader:Option<u8>)->Option<RecoveringNote> {
    if t.recovering.is_some() {return None;}
    t.recovering=Some(reason);
    Some(RecoveringNote {ticket:t.ticket,reason,loader,published:t.gate.published()})
}
fn trace_recovering(note:Option<RecoveringNote>) {
    if let Some(n)=note {
        trace!("[Txn] recovering role=player ticket={:?} how=AwaitDestruction reason={} loader={:?} published={} INSTALL_active={}",
            n.ticket,n.reason,n.loader,n.published,INSTALL.active());
    }
}

/// Wrapper ownership is separate from the observer/service callback's Rust early return.
pub(super) unsafe fn enter_native(entry:*mut u8)->bool {
    let mut s=STATE.lock();
    let Some(t)=s.track.as_mut().filter(|t|t.ticket.entry==entry as usize) else{return true;};
    if t.installing {return false;} // Suppress a same-entry callback nested inside INSTALL.
    if !native::span(entry as usize,0x5a88,false) || epoch()!=t.ticket.epoch {
        t.gate.refuse();return false;
    }
    t.gate.enter(t.ticket,read::<u8>(entry,0x591c))
}
pub(super) unsafe fn leave_native(entry:*mut u8) {
    let event={
        let mut s=STATE.lock();
        s.track.as_mut().filter(|t|t.ticket.entry==entry as usize).and_then(|t| {
            let loader=read::<u8>(entry,0x591c);
            t.gate.leave(t.ticket,loader);
            // WP5c: the native loader refused the request (>=7) before publication: name the
            // recovery once (the Player adapter has no native continuation path).
            let note=if loader>=7 && matches!(t.gate.phase(),Phase::Refused) && !t.gate.published() && !t.gate.native_only() {
                note_recovering(t,"native loader refused the request before publication",Some(loader))
            } else {None};
            matches!(t.gate.phase(),Phase::Refused|Phase::Departed)
                .then_some((t.ticket,t.gate.phase(),loader,t.gate.published(),note))
        })
    };
    // This executes after the original service actually returned through its native epilogue.
    // Subsequent refused calls never enter native and cannot flood this event.
    if let Some((ticket,phase,loader,published,note))=event {
        trace!("[MiiOrdinaryPlayer] native_return ticket={:?} phase={:?} loader={} published={} INSTALL_active={}",
            ticket,phase,loader,published,INSTALL.active());
        trace_recovering(note);
    }
}
pub(super) fn tracked(entry:usize)->bool {
    STATE.lock().track.as_ref().is_some_and(|t|t.ticket.entry==entry)
}
/// The tracked request's staged descriptor when it carries a Mii payload, and whether it is the
/// same-class retained-O arm (U5: the gate's configure-skip writer reads it; cloned under STATE).
pub(super) fn mii_payload_descriptor(entry:usize)->Option<([u8;0x70],bool)> {
    STATE.lock().track.as_ref().filter(|t|t.ticket.entry==entry && t.publish.is_some())
        .map(|t|(t.descriptor,t.same_class.is_some()))
}
/// WORKAROUND-DECISIONS row 7 (operator-approved root fix), the Player side of D-PTA1b: a prequeue
/// refusal after `reload_at_pose` must not strand the entry at loader 6 with its window already
/// destroyed. Every unpublished refused track continues natively — the staged descriptor is what
/// the native state-6 tail installs (an exact-Mii payload installs with the native-current Mii
/// rows, its publication skipped); the track stays `Refused`, is never published, and `service`
/// releases it as `NativeSettledUnowned` when native settles (loader 14). Eden E2 validates it.
pub(super) fn continue_natively(entry:usize)->bool {
    let mut s=STATE.lock();
    let event=s.track.as_mut().filter(|t|t.ticket.entry==entry).and_then(|t| {
        t.gate.continue_natively()
            .then_some((t.ticket,t.reload_target().kind,t.bank,t.publish.is_some() || t.same_class.is_some()))
    });
    drop(s);
    if let Some((ticket,target,bank,mii_payload))=event {
        trace!("[MiiOrdinaryPlayer] refusal continues natively ticket={:?} target={:#x} bank_build={} INSTALL_active={}",
            ticket,target,bank,INSTALL.active());
        trace!("[Txn] recovering role=player ticket={:?} how=NativeContinue reason=prequeue refused; staged descriptor installs natively target={:#x} mii_payload={} INSTALL_active={}",
            ticket,target,mii_payload,INSTALL.active());
        true
    } else {false}
}
pub(super) unsafe fn prequeue(entry:usize,selected:usize)->bool {
    let track={
        let s=STATE.lock();
        let Some(t)=s.track.as_ref().filter(|t|t.ticket.entry==entry) else{return true;};
        if !t.gate.may_publish(t.ticket,6) {drop(s);refuse(entry,"duplicate/reentrant gate");return false;}
        t.clone()
    };
    // WP-E (U14): the Player Mii publication writes native records/scalars; it runs in Training and
    // waiting rooms only. A refusal is the ordinary prequeue refusal (forward continuation follows).
    if !crate::runtime::roster::scope_gate::admitted("mii_ordinary_player::prequeue") {
        refuse(entry,"publication outside Training/waiting rooms (scope)");return false;
    }
    // Player `native::publish` refuses an initialised O before any O index/token compare, so the gate's
    // configure-skip words (U5) need no receipt on this side.
    let result=if track.same_class.is_some() {
        native::publish_same_class(&track,selected)
    } else {
        native::publish(&track,selected).map(|()|false)
    };
    match result {
        Ok(retained_o)=>{
            trace!("[MiiOrdinaryPlayer] published ticket={:?} target={:#x} O={:#x} index={} retained_same_class_O={} native_resources_untouched=true",
                track.ticket,track.reload_target().kind,selected,track.before.binding.record_index,retained_o);
            true
        }
        Err(reason)=>{refuse(entry,reason);false}
    }
}
fn refuse(entry:usize,reason:&str) {
    let mut s=STATE.lock();
    let event=s.track.as_mut().filter(|t|t.ticket.entry==entry).map(|t| {
        t.gate.refuse();(t.ticket,t.gate.phase(),t.gate.published())
    });
    drop(s);
    if let Some((ticket,phase,published))=event {
        trace!("[MiiOrdinaryPlayer] refused ticket={:?} phase={:?} published={} INSTALL_active={} reason={}",
            ticket,phase,published,INSTALL.active(),reason);
    }
}
/// Capture the qualified owner at the generic roster's INSTALL boundary, including already
/// committed helper/CPU changes. A missing receipt never alters the load or grants authority.
#[inline(never)]
pub(super) unsafe fn begin_generic(entry:*mut u8,token:InstallToken,descriptor:&[u8;0x70]) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080005);
    if !enabled() {return;}
    let pending={
        let mut s=STATE.lock();
        s.generic=None;
        let busy=s.busy() || s.request.is_some();
        s.owner.as_ref().filter(|o|o.origin.binding.entry==entry as usize).map(|o|(
            o.origin.original_kind,o.expected,
            busy || o.origin_saved.is_some() || o.active.is_some() || o.cursor.is_some()))
    };
    let Some((original_kind,expected,busy))=pending else {return;};
    if busy || MiiClass::from_kind(descriptor_kind(descriptor)).is_some() {
        note("generic Player0 begin not recorded: owned configuration, transaction or Mii target");return;
    }
    let before=match native::snapshot(entry as usize,epoch(),original_kind) {
        Ok(now) if native::same_owned(&now,&expected)=>now,
        Ok(_)=>{note("generic Player0 began from an unqualified owner state");return;}
        Err(reason)=>{note(reason);return;}
    };
    let mut s=STATE.lock();
    if s.busy() || s.request.is_some() || epoch()!=before.binding.epoch
        || !s.owner.as_ref().is_some_and(|o|o.expected==expected && o.origin_saved.is_none()
            && o.active.is_none() && o.cursor.is_none()) {
        drop(s);note("generic Player0 begin retired");return;
    }
    s.generic=Some(Generic {token,before,expected,descriptor:*descriptor});
}
#[inline(never)]
pub(super) unsafe fn complete_generic(entry:*mut u8,token:InstallToken,kind:u32) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080006);
    if !enabled() {return;}
    let generic={
        let mut s=STATE.lock();
        if s.generic.as_ref().is_some_and(|g|g.token==token && g.before.binding.entry==entry as usize) {
            s.generic.take()
        } else {None}
    };
    let Some(g)=generic else {return;};
    match native::generic_completed(&g,kind) {
        Ok(now)=>trace!("[MiiOrdinaryPlayer] generic Player0 owner committed kind={:#x} published={:#x}",
            kind,descriptor_kind(&now.descriptor)),
        Err(reason)=>trace!("[MiiOrdinaryPlayer] generic Player0 commit refused; owner unchanged reason={}",reason),
    }
}
pub(super) fn scene_changed() {
    publish_admission(crate::engine::swap::cycle_members::NO_MII);
    let mut s=STATE.lock();s.capture=false;s.menu_valid=false;s.catalog=None;s.request=None;
    s.generic=None;s.restore=None;
    #[cfg(feature="native-body-bank-alpha")]
    if let Some(b)=s.bank.as_mut() {b.departed=true;}
    #[cfg(feature="native-body-bank-alpha")]
    let bank_open=s.bank.is_some();
    #[cfg(not(feature="native-body-bank-alpha"))]
    let bank_open=false;
    if let Some(t)=s.track.as_mut(){t.gate.depart();}else if !bank_open{s.owner=None;}
}
pub(super) fn on_destroy(entry:*const u8) {
    let token={
        let mut s=STATE.lock();
        if s.generic.as_ref().is_some_and(|g|g.before.binding.entry==entry as usize) {s.generic=None;}
        if s.owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry as usize) {
            s.owner=None;s.request=None;s.restore=None;s.capture=false;s.menu_valid=false;
        }
        #[cfg(feature="native-body-bank-alpha")]
        if let Some(b)=s.bank.filter(|b|b.ticket.entry==entry as usize) {
            s.bank=None;
            let released=settle_outcome(Outcome::Destroyed {reason:"ordinary Player0 bank switch destructor",lease:b.token},
                b.ticket,None,true);
            trace!("[MiiOrdinaryPlayer] destroyed bank ticket={:?} INSTALL_released={} cleanup_writes=false",
                b.ticket,released);
        }
        if s.track.as_ref().is_some_and(|t|t.ticket.entry==entry as usize) {
            s.track.take().map(|t|(t.token,t.ticket,t.gate.phase(),t.gate.published(),t.recovering))
        } else {None}
    };
    native::retire_authority(0,entry as usize);
    if let Some((token,ticket,phase,published,recovering))=token {
        // A recovering track ends here with the reason it was waiting on.
        let released=settle_outcome(Outcome::Destroyed {reason:recovering.unwrap_or("ordinary Player0 exact destructor"),lease:token},
            ticket,None,published);
        trace!("[MiiOrdinaryPlayer] destroyed ticket={:?} phase={:?} INSTALL_released={} cleanup_writes=false",
            ticket,phase,released);
    }
}
