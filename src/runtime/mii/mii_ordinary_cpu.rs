//! Default-off ordinary CPU1 cycle adapter. Original index0 and retained destination1 are distinct.
//! INSTALL submission and phase-time publication are distinct commits.
use super::*;
use std::sync::Arc;
use std::convert::TryInto;
use crate::engine::mii::{
    inventory::{MiiClass,Snapshot as Catalog}, lifetime::Configuration,
    ordinary_player::{Gate,Phase,SameClass,Ticket},
    ordinary_cpu::{Destination,NonRankDiff,RecaptureFlags,Request,Target,TargetKind,
        non_rank_descriptor_diff,recapture_observation_seen,write_recapture_diagnostic,
        RECAPTURE_DIAGNOSTIC_BYTES},
    selection::Action,
    identity::Identity as TxnIdentity,
    transition::{Outcome,ReadyReceipt,Refusal},
};
use crate::engine::mii_root::mii_menu::Context as MenuContext;
use crate::engine::swap::mii_cycle::{self,Identity,Live};
use crate::engine::swap::source_load::{Blocker,Waiting};
#[path="mii_ordinary_cpu_native.rs"] mod native;
use super::mii_ordinary_player::native::{self as provenance,Scalars};

static ENABLED:AtomicBool=AtomicBool::new(false);
static STATE:Mutex<State>=Mutex::new(State {
    owner:None,track:None,catalog:None,capture:false,capture_at:(0,0),menu_valid:false,request:None,next:1,
    reservation:crate::engine::mii::ordinary_cpu::Reservation::empty(),departed:false,replay_result:None,generic:None,restore:None,
    recapture_log:None,waiting:Waiting::new(),
    #[cfg(feature="native-body-bank-alpha")]
    bank:None,
});
struct Owner {
    origin:native::Snapshot,
    expected:native::Snapshot,
    origin_saved:Option<Configuration>,
    live:Destination,
}
#[derive(Clone)]
struct Track {
    ticket:Ticket, gate:Gate, token:InstallToken, installing:bool, retained_o:bool,
    /// WP5b: set once when a committed track can no longer be adopted and waits for exact
    /// destruction (`transition::Recovery::AwaitDestruction`); the reason names the outcome.
    recovering:Option<&'static str>,
    data:Arc<Prepared>,
}
/// WP5b: a newly noted recovery, traced by the caller after the STATE lock is dropped.
#[derive(Clone,Copy)]
struct RecoveringNote {ticket:Ticket,reason:&'static str,loader:Option<u8>,published:bool}
impl std::ops::Deref for Track {
    type Target=Prepared;
    fn deref(&self)->&Prepared {&self.data}
}
struct Prepared {
    before:native::Snapshot, origin_scalars:Scalars,
    hat_change:bool,
    origin_index:u16,origin_record:Option<[u8;104]>,
    descriptor:[u8;0x70], publish:Option<Configuration>, destination:Destination, same_class:Option<SameClass>, replay:Option<crate::engine::mii::replay::Selection>,
    /// Physical same-entry bank build (the target at its own cell, `ReloadTarget`); see the
    /// Player adapter.
    bank:bool,
}
impl Prepared {
    /// The cell native activates at Ready and the logical target this transaction settles on.
    fn reload_target(&self)->crate::engine::swap::body_bank::ReloadTarget {
        crate::engine::swap::body_bank::ReloadTarget::of(&self.descriptor,self.bank)
    }
}
/// Same-entry bank member switch: no phase-6 publication and no new physical
/// receipt. `before` stays the owner's expected snapshot; logical is this token's
/// requested member and `retained` the same_owned record mode used at preflight.
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy)]
struct BankTrack {
    ticket:Ticket, token:InstallToken, before:native::Snapshot, logical:u32,
    destination:Destination, retained:bool, replay:Option<crate::engine::mii::replay::Selection>,
    /// A Mii one-member group switch (MII-GROUPS-DESIGN §5): the exact configuration
    /// published onto the destination record before the switch and proven at
    /// settlement; `descriptor` is the request's staged exact-Mii bytes naming its
    /// identity; the origin index/record are the retained-origin proof's inputs.
    publish:Option<Configuration>, descriptor:[u8;0x70], hat_change:bool,
    origin_index:u16, origin_record:Option<[u8;104]>,
    /// MII-PHASEB: what `bank_publish` wrote (configuration, pre-publish record, `E+0x1a8`
    /// and scalars), recorded with the write; `bank_refused` puts it back (see the Player).
    published:Option<(Configuration,native::BankPublished)>,
}
/// Read-only receipt for an ordinary load kept on the generic roster path.
struct Generic {
    token:InstallToken,before:native::Snapshot,expected:native::Snapshot,live:Destination,
    descriptor:[u8;0x70],
}
struct State {
    owner:Option<Owner>,track:Option<Track>,catalog:Option<Arc<Catalog>>,
    capture:bool,capture_at:(u64,u64),menu_valid:bool,request:Option<Request>,next:u64,
    reservation:crate::engine::mii::ordinary_cpu::Reservation,departed:bool,
    replay_result:Option<crate::engine::mii::replay::SelectionReceipt>,
    generic:Option<Generic>,
    restore:Option<WaitingValue>,
    /// Last printed non-rank recapture tuple. Not read on an accepted capture.
    recapture_log:Option<(u64,NonRankDiff,RecaptureFlags)>,
    /// FAIL 2: the queued request's last noted blocker; one line per change, not per callback.
    waiting:Waiting<Request>,
    #[cfg(feature="native-body-bank-alpha")]
    bank:Option<BankTrack>,
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
fn epoch()->u64 { mii_saved_alpha::current_generation(1) }
fn note(reason:impl std::fmt::Display) { trace!("[MiiOrdinaryCpu] {reason}"); }
/// lobby-preload A: the CPU's visible exact Mii for extra-capacity creation. Only the authority
/// this adapter committed for the CPU owner (1 Training, 4 waiting), rederived now under G with
/// the CPU's original row kind, proves it. Reads only.
#[cfg(feature="managed-third-source-alpha")]
pub(super) unsafe fn visible_exact(current:*const u8,kind:u32)->Result<provenance::VisibleMii,&'static str> {
    let original=original_kind(Role::Cpu).ok_or("CPU original absent")?;
    let now=native::snapshot(current as usize,epoch(),original)?;
    let authority=now.authority();
    provenance::check(provenance::visible_committed(authority,original,kind),
        "CPU visible Mii is not its committed exact authority")?;
    Ok(provenance::VisibleMii {authority,descriptor:now.descriptor,record:now.record})
}

pub(super) fn preload_pending()->bool {
    let s=STATE.lock().unwrap(); s.capture || s.request.is_some() || s.busy()
}
pub(super) fn preload_targets(menu:&TrainingModpackMenu)->Option<[Option<crate::runtime::roster::cycle_preload::Target>;crate::runtime::roster::cycle_preload::MAX_TARGETS]> {
    let (original,live,catalog,saved)={
        let s=STATE.lock().unwrap();
        if s.departed || !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
        let o=s.owner.as_ref()?;
        if o.origin.binding.epoch!=epoch() {return None;}
        let identity=o.live.target.publish(o.origin_saved).map_or(
            Identity::Fighter(unsafe {logical_kind(&o.expected)}),|c|Identity::Saved(c.canonical()));
        let live=if o.live.cursor.is_none() && o.live.target==Target::ORIGINAL {Live::Original}
            else {Live::Numbered {identity,cursor:o.live.cursor}};
        (o.origin_saved.map_or(Identity::Fighter(o.origin.original_kind),|c|Identity::Saved(c.canonical())),
            live,Arc::clone(s.catalog.as_ref()?),o.origin_saved)
    };
    Some(crate::runtime::roster::cycle_preload::mixed_targets(menu,1,original,live,&catalog,saved))
}
/// Menu recapture already refused the descriptor. Log the locked snapshots once
/// per owner epoch and observed tuple. Does not accept bytes or change slots.
fn note_non_rank_recapture(expected:&native::Snapshot, now:&native::Snapshot) {
    let diff=non_rank_descriptor_diff(&expected.descriptor,&now.descriptor);
    if diff.count==0 {return;}
    let flags=RecaptureFlags {
        index_eq:expected.index==now.index,
        scalars_eq:expected.scalars==now.scalars,
        selector_record_eq:expected.record==now.record,
        current_record_eq:expected.current_record==now.current_record,
        captured_index:expected.index,
        live_index:now.index,
    };
    let epoch=expected.binding.epoch;
    if recapture_observation_seen(STATE.lock().unwrap().recapture_log,epoch,diff,flags) {return;}
    let mut buf=[0u8;RECAPTURE_DIAGNOSTIC_BYTES];
    let Some(n)=write_recapture_diagnostic(&mut buf,&diff,&flags) else {return;};
    let Ok(line)=core::str::from_utf8(&buf[..n]) else {return;};
    let print={
        let mut s=STATE.lock().unwrap();
        if recapture_observation_seen(s.recapture_log,epoch,diff,flags) {false}
        else {s.recapture_log=Some((epoch,diff,flags));true}
    };
    if print {note(line);}
}

/// One line per refused preflight, naming which part of the owner snapshot moved
/// after the idle behavior refresh. Read-only; grants nothing.
fn note_preflight_mismatch(expected:&native::Snapshot, now:&native::Snapshot) {
    let diff=non_rank_descriptor_diff(&expected.descriptor,&now.descriptor);
    let pairs=&diff.pairs[..usize::from(diff.count.min(8))];
    let controller=|d:&[u8;0x70]| i32::from_le_bytes(d[0x10..0x14].try_into().unwrap());
    trace!("[MiiOrdinaryCpu] preflight current configuration changed: descriptor n={} {:02x?} rank={}>{} row_control={:02x?}>{:02x?} index_eq={} scalars_eq={} record_eq={} current_record_eq={} owner_eq={} binding_eq={} controller={}>{}",
        diff.count,pairs,expected.descriptor[0x37],now.descriptor[0x37],expected.control,now.control,
        expected.index==now.index,expected.scalars==now.scalars,expected.record==now.record,
        expected.current_record==now.current_record,
        expected.owners[expected.binding.owner as usize]==now.owners[now.binding.owner as usize],
        expected.binding==now.binding,controller(&expected.descriptor),controller(&now.descriptor));
}

// Called only after the existing ordinary gate's fingerprint/install succeeds.
pub(super) fn init() {
    ENABLED.store(true,Ordering::Release);
    note("enabled: CPU1 ordinary index0/native selector1 cycle alpha; ordered targets and same-class menu reload");
}
pub(super) fn enabled()->bool {ENABLED.load(Ordering::Acquire)}
pub(super) fn writers_reserved(role:usize,record:u16)->bool {
    let s=STATE.lock().unwrap();
    let physical=s.owner.as_ref().map_or(1,|o|o.origin.binding.owner as usize);
    s.reservation.held() && (role==1 || role==physical || record==physical as u16)
}
pub(super) fn catalog_ready(catalog:Arc<Catalog>,generation:u64,opening:u64)->bool {
    if !ENABLED.load(Ordering::Acquire) {return false;}
    let mut s=STATE.lock().unwrap();
    if epoch()!=generation || mii_saved_alpha::menu_epoch()!=opening {return false;}
    let complete=catalog.is_complete();
    s.catalog=Some(catalog);s.menu_valid=false;s.capture=complete && !s.busy();
    s.capture_at=(generation,opening);
    s.capture
}
pub(super) fn refresh_library(library:&crate::engine::mii::draft::Library) {
    let mut s=STATE.lock().unwrap();
    if let Some(catalog)=s.catalog.take() {s.catalog=Some(Arc::new((*catalog).clone().with_library(library)));}
}
pub(super) unsafe fn menu_capture_pending(entry:*const u8)->bool {
    let s=STATE.lock().unwrap();
    s.capture && (s.owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry as usize)
        || original_kind(Role::Cpu)==Some(read::<u32>(entry,0x18)))
}
#[cfg(feature="mii-managed-helper-alpha")]
pub(super) fn helper_origin_captured()->bool {
    let s=STATE.lock().unwrap();
    !s.departed && !s.capture && !s.busy()
        && s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch())
}
#[cfg(feature="mii-managed-helper-alpha")]
pub(super) fn helper_origin_hats()->Option<[u16;2]> {
    let s=STATE.lock().unwrap();
    s.owner.as_ref().filter(|o|!s.departed && o.origin.binding.epoch==epoch())
        .and_then(|o|o.origin.helper_hats())
}
pub(super) fn menu_qualified()->bool {
    let s=STATE.lock().unwrap();
    !s.departed && s.menu_valid && !s.capture && !s.busy() && s.request.is_none()
        && s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch())
}
/// Only the ordinary adapter supplies active CPU canonical identity to the existing menu.
pub(super) fn cpu_context()->Option<(MiiClass,crate::engine::mii::inventory::Canonical)> {
    let s=STATE.lock().unwrap();
    if s.departed || !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
    s.owner.as_ref().filter(|o|o.origin.binding.epoch==epoch())
        .and_then(|o|o.live.target.publish(o.origin_saved)).map(|c|(c.canonical().class,c.canonical()))
}
/// Captured saved-Mii original for menu numbered-cycle exposure.
/// The live class is not an original. None leaves the menu unresolved.
pub(super) fn cpu_original_canonical()->Option<crate::engine::mii::inventory::Canonical> {
    let s=STATE.lock().unwrap();
    if s.departed || !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
    s.owner.as_ref().filter(|o|o.origin.binding.epoch==epoch())
        .and_then(|o|o.origin_saved).map(|configuration|configuration.canonical())
}
pub(super) fn replay_catalog()->Option<Arc<Catalog>> {
    let s=STATE.lock().unwrap();
    (!s.departed && s.menu_valid && !s.capture && !s.busy() && s.request.is_none()
        && s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch()))
        .then(||s.catalog.as_ref().filter(|c|c.is_complete()).map(Arc::clone)).flatten()
}
/// Settled loaded CPU configuration. This is the live original, saved or
/// discovered body, and only when its class is the loaded descriptor kind.
/// A replay receipt that has not become that body is not returned.
pub(super) fn current_configuration()->Option<Configuration> {
    let s=STATE.lock().unwrap();
    if s.departed || !s.menu_valid || s.capture || s.busy() || s.request.is_some() {return None;}
    let owner=s.owner.as_ref().filter(|o|
        o.origin.binding.epoch==epoch() && o.origin.layout.logical(o.origin.binding.owner)==Some(1))?;
    let configuration=owner.live.target.publish(owner.origin_saved)?;
    (configuration.canonical().class as u32==descriptor_kind(&owner.expected.descriptor))
        .then_some(configuration)
}
pub(super) fn replay_current()->Option<Configuration> {
    let s=STATE.lock().unwrap();
    s.owner.as_ref().filter(|o|!s.departed && !s.busy() && s.request.is_none()
        && o.origin.binding.epoch==epoch()).and_then(|o|o.live.target.publish(o.origin_saved))
}
pub(super) fn replay_ready(selection:crate::engine::mii::replay::Selection,kind:u32)->bool {
    let s=STATE.lock().unwrap();
    s.replay_result.and_then(|receipt|receipt.for_selection(selection,epoch(),super::mii_replay::revision(selection.source)))==Some(Ok(()))
        && !s.departed && !s.busy() && s.request.is_none()
        && s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch()
            && selection.ready(kind,o.live.target.publish(o.origin_saved)))
}
pub(super) fn replay_refusal(selection:crate::engine::mii::replay::Selection)->Option<&'static str> {
    STATE.lock().unwrap().replay_result
        .and_then(|receipt|receipt.for_selection(selection,epoch(),super::mii_replay::revision(selection.source)))
        .and_then(Result::err)
}
fn refuse_request(request:Request,reason:&'static str) {
    let mut s=STATE.lock().unwrap();
    let mut restore_refused=false;
    let mut cycle_refused=false;
    if s.request==Some(request) {
        if let Some(selection)=request.replay() {
            s.replay_result=Some(crate::engine::mii::replay::SelectionReceipt {
                selection,outcome:Err(reason)});
        }
        s.request=None;
        restore_refused=s.restore.is_some();
        cycle_refused=request==Request::CYCLE && !restore_refused;
        s.restore=None;
    }
    drop(s);
    if restore_refused {
        WAITING_MEMORY.lock().unwrap().manual_request(Role::Cpu as usize);
        notify(Role::Cpu,"Practice fighter could not be restored".into());
    } else if cycle_refused {
        notify(Role::Cpu,"This CPU character cannot be loaded".into());
    }
}
pub(super) fn request_replay(selection:crate::engine::mii::replay::Selection)->bool {
    request_replay_intent(selection,false)
}
pub(super) fn request_reacknowledge(selection:crate::engine::mii::replay::Selection)->bool {
    request_replay_intent(selection,true)
}
fn request_replay_intent(selection:crate::engine::mii::replay::Selection,receipt_only:bool)->bool {
    if TrioFighter::from_kind(selection.target.kind()).is_none() {return false;}
    let mut s=STATE.lock().unwrap();
    if !enabled() || s.departed || !s.menu_valid || s.capture || s.busy() || s.request.is_some()
        || !selection.current(epoch(),super::mii_replay::revision(selection.source))
        || s.owner.as_ref().is_none_or(|o|o.origin.binding.epoch!=epoch()) {return false;}
    s.restore=None;
    if receipt_only {
        let current=s.owner.as_ref().and_then(|o|o.live.target.publish(o.origin_saved));
        if selection.target.configuration().is_none() || current!=selection.target.configuration()
            || s.replay_result.and_then(|r|r.for_selection(selection,epoch(),super::mii_replay::revision(selection.source)))==Some(Ok(())) {
            return false;
        }
        s.request=Request::acknowledge_replay(selection);
    } else {s.request=Request::from_replay(selection);}
    if s.request.is_some() {s.replay_result=None;true} else {false}
}
fn request_current(request:Request,active:Option<Configuration>)->bool {
    request.current(mii_saved_alpha::menu_epoch(),active.map(|c|c.canonical()))
        && request.replay().is_none_or(|r|r.current(epoch(),super::mii_replay::revision(r.source)))
}
/// Consume ordinary-owner actions even when rejected; never fall through to SavedRole.
pub(super) fn menu_close(context:MenuContext,action:Action)->bool {
    if !matches!(action,Action::ReloadCpu(_)) {return false;}
    let mut s=STATE.lock().unwrap();
    let Some(o)=s.owner.as_ref() else {return enabled();};
    if !ENABLED.load(Ordering::Acquire) || !s.menu_valid || s.departed || s.busy() || s.request.is_some() {
        drop(s);note("CPU menu-close refused: owned transaction busy");return true;
    }
    let request=Request::from_menu(mii_saved_alpha::menu_epoch(),context.epoch,
        context.active.cpu,o.live.target.publish(o.origin_saved).map(|c|c.canonical()),action);
    s.request=request;s.restore=None;
    if request.is_some() {WAITING_MEMORY.lock().unwrap().manual_request(Role::Cpu as usize);}
    if request.is_none() {drop(s);note("CPU menu-close refused: active configuration/epoch changed");}
    true
}
fn clear_request(request:Request) {
    let mut s=STATE.lock().unwrap();
    let restore_refused=s.request==Some(request) && s.restore.is_some();
    if s.request==Some(request) {s.request=None;s.restore=None;}
    drop(s);
    if restore_refused {
        WAITING_MEMORY.lock().unwrap().manual_request(Role::Cpu as usize);
        notify(Role::Cpu,"Practice fighter could not be restored".into());
    }
}
pub(super) fn owns(entry:usize)->bool {
    STATE.lock().unwrap().owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry)
}
pub(super) fn request_restore(value:WaitingValue)->bool {
    let mut s=STATE.lock().unwrap();
    if !mii_backend::current().is_some_and(|l|l.waiting()) || !enabled() {return false;}
    // Qualification precedes continuation, ordinary values included (see Player).
    if unsafe {super::mii_auto_init::origin_capture_pending(Role::Cpu,
        s.owner.as_ref().is_some_and(|o|o.origin.binding.epoch==epoch()),s.capture)} {return true;}
    // A mixed CPU roster is intercepted even when neither current nor remembered body is Mii.
    let mixed=setting(&MENU).cpu_roster.fighters().iter()
        .any(|f|MiiClass::from_kind(f.kind()).is_some());
    let needed=value.owned || original_kind(Role::Cpu).is_some_and(|k|MiiClass::from_kind(k).is_some())
        || s.reservation.held() || mixed || s.owner.as_ref().is_some_and(|o|o.live.target.saved().is_some());
    if !needed {return false;}
    if s.departed || s.capture || s.busy() || s.request.is_some()
        || s.owner.as_ref().is_none_or(|o|o.origin.binding.epoch!=epoch()) {return true;}
    if WAITING_MEMORY.lock().unwrap().restore_target(Role::Cpu as usize)!=Some(value) {return true;}
    s.restore=Some(value);s.request=Some(Request::CYCLE);
    true
}
pub(super) fn intercept(role:Role,selection:Selection,user_request:bool)->Option<bool> {
    if role!=Role::Cpu || !ENABLED.load(Ordering::Acquire) {return None;}
    let mut s=STATE.lock().unwrap();
    let Some(owner)=s.owner.as_ref() else {
        // A manual ordinary swap can replace the same origin needed by capture.
        // Consume it without enqueuing a stale choice; the existing loading notice
        // asks the user to retry once the shared catalog is qualified.
        if unsafe {super::mii_auto_init::origin_capture_pending(Role::Cpu,false,s.capture)} {
            drop(s);
            if user_request {notify(Role::Cpu,"Wait for the practice character to finish loading".into());}
            return Some(true);
        }
        let wants_mii=setting(&MENU).cpu_roster.fighters().iter()
            .any(|f|MiiClass::from_kind(f.kind()).is_some());
        let direct=if let Selection::Direct(f)=selection {Some(f.kind())} else {None};
        if crate::engine::mii::auto_init::requires_owner(original_kind(Role::Cpu),wants_mii,direct) {
            drop(s);note("CPU cycle refused: original assignment not captured");
            if user_request && mii_backend::current().is_some() {
                notify(Role::Cpu,"Wait for the practice character to finish loading".into());
            }
            return Some(true);
        }
        return None;
    };
    // Same-entry banks: a qualified, captured owner (current epoch, valid menu and
    // catalog, no capture pending, not departed) also keeps ordinary-only cycles. A
    // missing or unqualified owner still takes the fallback above or the roster.
    #[cfg(feature="native-body-bank-alpha")]
    let ordinary_cycle=matches!(selection,Selection::Cycle) && !s.departed && s.menu_valid && !s.capture
        && s.catalog.is_some() && owner.origin.binding.epoch==epoch();
    #[cfg(not(feature="native-body-bank-alpha"))]
    let ordinary_cycle=false;
    let requested=setting(&MENU).cpu_roster.fighters().iter()
        .any(|f|MiiClass::from_kind(f.kind()).is_some())
        || ordinary_cycle
        || matches!(selection,Selection::Direct(f) if MiiClass::from_kind(f.kind()).is_some());
    if !s.reservation.routes(requested,owner.origin_saved.is_some() || owner.live.target.saved().is_some()) {return None;}
    // Automatic partner reconciliation may repeat while an owned operation
    // waits. Coalesce it without a per-frame synchronous diagnostic write.
    if matches!(selection,Selection::RefreshPartners)
        && (s.departed || s.request.is_some() || s.busy()) {return Some(true);}
    // Reservation starts at the INSTALL ticket, never at this menu/catalog capture.
    let request=match selection {
        Selection::Cycle=>Some(Request::CYCLE),
        Selection::Direct(f)=>Request::direct(mii_saved_alpha::menu_epoch(),f.kind()),
        Selection::RefreshPartners=>Some(Request::REFRESH_PARTNERS),
    };
    let manual_direct=user_request && matches!(selection,Selection::Direct(_));
    // Before INSTALL begins, an explicit pick supersedes a pending lobby restore.
    let replace_restore=manual_direct && s.restore.is_some();
    if !s.departed && !s.busy() && (s.request.is_none() || replace_restore) && request.is_some() {
        s.restore=None;
        if user_request {WAITING_MEMORY.lock().unwrap().manual_request(Role::Cpu as usize);}
        s.request=request;
        s.waiting=Waiting::new();
        drop(s);
        // FAIL 2: an accepted request has a line too; automatic partner refreshes stay quiet.
        if !matches!(selection,Selection::RefreshPartners) {
            trace!("[MiiOrdinaryCpu] request accepted selection={:?} user={} install_active={}",
                selection,user_request,INSTALL.active());
        }
    } else {
        drop(s);note("request refused: owned CPU request busy or unsupported");
        if manual_direct {notify(Role::Cpu,"Finish the current practice action first".into());}
    }
    Some(true)
}

/// What `begin` waits on, in `permitted()`'s order.
unsafe fn begin_blocker()->Option<Blocker> {
    if !mii_backend::ready() {Some(Blocker::Backend)}
    else if !crate::runtime::compat::common::is_ready_go() {Some(Blocker::ReadyGo)}
    else if menu::captures_input() {Some(Blocker::Menu)}
    else if lobby::native_ui_busy() {Some(Blocker::NativeUi)}
    else if button_config::has_pending_request() {Some(Blocker::Shortcut)}
    else if INSTALL.active() {Some(Blocker::Install)}
    else if training::save_states::is_loading() {Some(Blocker::SaveState)}
    else if training::recording_or_playback() {Some(Blocker::Recording)}
    else if crate::runtime::compat::replay_preview::occupied() {Some(Blocker::ReplayPreview)}
    else if !STATES.iter().all(|s| {let s=s.lock().unwrap();s.active.is_none()&&s.pending.is_none()}) {Some(Blocker::Roster)}
    else {None}
}
unsafe fn permitted()->bool {begin_blocker().is_none()}

/// FAIL 2: one line when the queued request starts waiting or its blocker changes. Values only;
/// the INSTALL holder is read after STATE is released.
fn note_waiting(blocker:Option<Blocker>) {
    let noted={
        let mut s=STATE.lock().unwrap();
        let Some(request)=s.request else {return;};
        s.waiting.note(request,blocker).map(|blocker|(request,blocker))
    };
    if let Some((request,blocker))=noted {
        let holder=if blocker==Blocker::Install {sources::install_holder()} else {None};
        trace!("[MiiOrdinaryCpu] request waiting request={:?} on=\"{}\" install_holder(slot,kind,ticks,speculative)={:x?}",
            request,blocker.reason(),holder);
    }
}
unsafe fn ready(entry:*const u8,kind:u32)->bool {
    let Some(owner)=mii_backend::cpu() else{return false;};
    if MiiClass::from_kind(kind).is_some() {ready_single_mii(entry,owner)==Some(kind)}
    else {ready_forms(entry,owner).is_some_and(|(_,k)|k==kind)}
}
fn descriptor_forms(d:&[u8;0x70])->Forms {
    let kind=descriptor_kind(d);
    if kind==0x4b && u32::from_le_bytes(d[0x14..0x18].try_into().unwrap())==1 {
        Forms::solo_popo()
    } else {Forms::starting(kind)}
}
unsafe fn ready_descriptor(entry:*const u8,d:&[u8;0x70])->bool {
    if MiiClass::from_kind(descriptor_kind(d)).is_some() {ready(entry,descriptor_kind(d))}
    else {mii_backend::cpu().is_some_and(|owner|ready_forms(entry,owner).is_some_and(|(forms,_)|forms==descriptor_forms(d))
        || bank_live(entry,owner,d).is_some())}
}
/// Live member of this CPU owner's ready same-entry bank when `d` is exactly that
/// bank's physical group. Value only; the bank module proves bodies under its lock.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_live(entry:*const u8,owner:u32,d:&[u8;0x70])->Option<u32> {
    let members=super::body_bank::ready_members(entry,owner)?;
    let word=|at:usize|u32::from_le_bytes(d[at..at+4].try_into().unwrap());
    if word(0x14) as usize==members.count() && word(0x28)==members.actor_count()
        && (0..3).all(|i|word(0x18+i*4)==members.kinds()[i]) {
        super::body_bank::ready_kind(entry,owner)
    } else {None}
}
#[cfg(not(feature="native-body-bank-alpha"))]
unsafe fn bank_live(_entry:*const u8,_owner:u32,_d:&[u8;0x70])->Option<u32> {None}
/// The live fighter of this CPU owner's entry (the outgoing one of a request); see the Player
/// adapter. Current native callback only, before the request changes the bank.
unsafe fn live_kind(entry:*const u8,owner:u32,d:&[u8;0x70])->u32 {bank_live(entry,owner,d).unwrap_or_else(||descriptor_kind(d))}
/// Logical form of a bank member from the physical window words: a pair-lead
/// window's Popo is the normal pair, otherwise the member's single form.
fn bank_member_forms(d:&[u8;0x70],kind:u32)->Forms {
    let word=|at:usize|u32::from_le_bytes(d[at..at+4].try_into().unwrap());
    crate::engine::swap::body_bank::member_forms_in(word(0x14),[word(0x18),word(0x1c),word(0x20)],word(0x28),kind)
}
/// Logical CPU fighter of the owned physical receipt, for value planning only. Uses
/// the bank's value receipt (no entry read), so it is safe outside the native
/// callback; request/settlement proofs keep the strict `bank_live`.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn logical_kind(expected:&native::Snapshot)->u32 {
    super::body_bank::cached_kind(expected.binding.entry,expected.binding.owner,&expected.descriptor)
        .unwrap_or_else(||descriptor_kind(&expected.descriptor))
}
#[cfg(not(feature="native-body-bank-alpha"))]
unsafe fn logical_kind(expected:&native::Snapshot)->u32 {descriptor_kind(&expected.descriptor)}
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
fn descriptor_kind(d:&[u8;0x70])->u32 {u32::from_le_bytes(d[0x18..0x1c].try_into().unwrap())}
#[inline(never)]
pub(super) unsafe fn service(entry:*mut u8,owner:u32)->bool {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080011);
    if !matches!(owner,1|4) || !ENABLED.load(Ordering::Acquire) {return false;}
    #[cfg(feature="native-body-bank-alpha")]
    {
        let bank=STATE.lock().unwrap().bank;
        if let Some(b)=bank {
            if b.ticket.entry!=entry as usize {return false;}
            bank_service(entry,b);
            return true;
        }
    }
    let tracked=STATE.lock().unwrap().track.clone();
    if let Some(t)=tracked {
        if t.ticket.entry!=entry as usize {return false;}
        if matches!(t.gate.phase(),Phase::Refused|Phase::Departed) {
            // A refused bank build still finished its native Reload; once native is
            // settled release INSTALL instead of retaining it for this cache. A plain-kind
            // track whose refusal continued natively (D-PTA1b) settles the same way.
            if matches!(t.gate.phase(),Phase::Refused) && settled(entry) && (t.bank || t.gate.native_only()) {
                bank_build_refused(&t,if t.bank {"CPU bank build publication refused"}
                    else {"CPU publication refused; native Reload settled"});
            }
            return true;
        }
        // Native is ready at the staged starting cell (`ReloadTarget`): the target's own cell
        // of a PinCell0 window, 0 for a fighter request.
        if t.gate.can_complete(t.ticket) && ready_descriptor(entry,&t.descriptor)
            && t.reload_target().ready_at(read::<u8>(entry,0x5918)) {complete(entry,&t);}
        else if t.bank && t.gate.can_complete(t.ticket) && (bank_failed(entry,owner)
            || (settled(entry) && !ready_descriptor(entry,&t.descriptor))) {
            bank_build_refused(&t,"CPU native settled without the staged bank");
        }
        return true;
    }
    if STATE.lock().unwrap().departed {return true;}
    if mii_backend::cpu()!=Some(owner) {return false;}
    let needs_capture=STATE.lock().unwrap().capture;
    if needs_capture {capture(entry);}
    #[cfg(feature="native-body-bank-alpha")]
    mii_group_record_publication(entry,owner);
    {
        let mut s=STATE.lock().unwrap();
        // FAIL 2: an idle owner forgets the last noted wait, so the next request's wait is noted.
        if s.request.is_none() {s.waiting=Waiting::new();return false;}
    }
    begin(entry)
}

/// A planned CPU Mii group's CharInfo goes onto the destination record before the
/// bank requests the group's keys (the key request copies the record at `R+0x68`,
/// MII-GROUPS-DESIGN §1.4/§5.2). Idle owner only. A busy refusal is retried next
/// frame at no cost; a read/write race counts against the group; a refusal that
/// cannot clear while the owner idles fails the group at once
/// (`crate::engine::swap::body_bank::mii_publish`). The refusal reason is noted once.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn mii_group_record_publication(entry:*mut u8,owner:u32) {
    use crate::engine::swap::body_bank::mii_publish::PublishRefusal;
    let Some((group,identity,c))=super::body_bank::pending_mii_publication(owner) else {return;};
    if INSTALL.active() || !mii_backend::ready() {return;}
    let outcome=native::prefetch_publish(entry as usize,c,identity);
    match outcome {
        Ok(())=>trace!("[MiiOrdinaryCpu] Mii group record published owner={} group={} kind={:#x} index={} hat={}",
            owner,group,identity.kind(),identity.index,identity.hat()),
        Err(PublishRefusal::Busy)=>return,
        Err(_)=>{}
    }
    if let Some(reason)=super::body_bank::mii_charinfo_published(owner,group,identity,outcome) {note(reason);}
}

#[inline(never)]
unsafe fn complete(entry:*const u8,t:&Track) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080014);
            let result=native::completed(t);
            // Diagnostic only: bounded read-only values for a refused comparison, read under G
            // before STATE (existing G->STATE order). No decision depends on them.
            let failure=if result.is_err() {native::completion_failure(t)} else {None};
            let mut s=STATE.lock().unwrap();
            if !s.track.as_ref().is_some_and(|now|now.ticket==t.ticket) {return;}
            match result {
                Ok(now) if epoch()==t.ticket.epoch => {
                    let Some(o)=s.owner.as_mut() else {
                        s.track.as_mut().unwrap().gate.refuse();return;
                    };
                    // Native completion committed owner/foreign-kind evidence under G -> STATE.
                    debug_assert_eq!(o.expected,now);s.track=None;s.request=None;s.restore=None;
                    s.replay_result=t.replay.filter(|r|r.current(epoch(),super::mii_replay::revision(r.source)))
                        .map(|selection|crate::engine::mii::replay::SelectionReceipt {selection,outcome:Ok(())});
                    drop(s);
                    // The delivered fighter: the member at the staged starting cell for a bank
                    // build (never `+0x18[0]`, a pinned Kirby/Solo Popo), the request's kind otherwise.
                    let target=t.reload_target().kind;
                    #[cfg(feature="mii-hud-portrait-alpha")]
                    publish_mii_presentation(entry,t.before.binding.owner,t.ticket.epoch,target,
                        if t.destination.target==Target::ORIGINAL {
                            super::PortraitIntent::Original {
                                original_kind: super::original_kind(super::Role::Cpu).unwrap_or(u32::MAX),
                            }
                        } else if let Some(c)=t.publish {
                            super::PortraitIntent::Exact(c)
                        } else {
                            super::PortraitIntent::NotMii
                        });
                    #[cfg(not(feature="mii-hud-portrait-alpha"))]
                    publish_mii_presentation(entry,t.before.binding.owner,t.ticket.epoch,target);
                    crate::runtime::roster::cycle_preload::foreground_settled();
                    let released=settle_outcome(Outcome::Applied {receipt:receipt_of(&now,target),lease:t.token},t.ticket,None,true);
                    if t.replay.is_none() {state_for(Role::Cpu).lock().unwrap().cursor=t.destination.cursor;}
                    let forms=if t.bank {bank_member_forms(&t.descriptor,target)}
                        else {descriptor_forms(&t.descriptor)};
                    publish_partners(Role::Cpu,forms);
                    if t.replay.is_none() {remember_owned(Role::Cpu,t.ticket.epoch,target,forms,
                        t.publish,t.destination.cursor,t.destination.target==Target::ORIGINAL);}
                    if let Some(fighter)=TrioFighter::from_kind(target).filter(|_|t.replay.is_none()) {
                        Role::Cpu.sync_selection(fighter);
                    }
                    crate::runtime::roster::cycle_preload::completed(1,false,target);
                    trace!("[MiiOrdinaryCpu] ready ticket={:?} target={:#x} index={} resource={:02x?} released={} saved_files_written=false",
                        t.ticket,target,now.index,
                        &now.record[88..104],released);
                    trace!("[MiiOrdinaryCpu] completion ticket={:?} current={:?}",t.ticket,now);
                    notify(Role::Cpu,match t.destination.target.kind() {
                        TargetKind::Saved=>"Saved Mii applied",TargetKind::Original=>"Original fighter restored",
                        TargetKind::Fighter(_)=>"CPU fighter applied"}.into());
                }
                _ if t.bank => {drop(s);bank_build_refused(t,"CPU terminal bank ready comparison refusal");}
                other => {
                    let note={let now_t=s.track.as_mut().unwrap();now_t.gate.refuse();
                        note_recovering(now_t,"terminal ready comparison refusal; INSTALL retained until exact destruction",None)};
                    drop(s);trace_recovering(note);
                    let reason=other.err().unwrap_or("ticket epoch changed before completion");
                    match failure {
                        Some(native::Failure {now,mirrors,kinds,fe_tokens,d_tokens,o_init,o_words,task,outstanding}) => {
                            trace!("[MiiOrdinaryCpu] terminal ready comparison refusal; INSTALL retained ticket={:?} reason={} mirrors(E1a8,FE46,D72,O68)={:?} kinds(FE18,E8)={:?} FEtokens={:?} Dtokens={:?} Oinit={:?} O54..7b={:x?} task={:x?} outstanding={:?} now={:?}",
                                t.ticket,reason,mirrors,kinds,fe_tokens,d_tokens,o_init,o_words,task,outstanding,now);
                        }
                        None => {
                            trace!("[MiiOrdinaryCpu] terminal ready comparison refusal; INSTALL retained ticket={:?} reason={} failure_values=unavailable",
                                t.ticket,reason);
                        }
                    }
                }
            }
}

#[inline(never)]
unsafe fn capture(entry:*mut u8) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080012);
    let kind=read::<u32>(entry,0x18);
    let captured=STATE.lock().unwrap().owner.as_ref().map(|o|o.origin.original_kind);
    let Some(original)=captured.or_else(||original_kind(Role::Cpu)) else {return;};
    // An owned bank can display any member while +0x18 remains member zero.
    // Prove native readiness here; capture_origin below compares the physical
    // ownership receipt and finishes a stale receipt as refused, not busy.
    let ready_now=match captured {
        Some(_)=>mii_backend::cpu().is_some_and(|owner|
            ready_forms(entry,owner).is_some() || ready_single_mii(entry,owner).is_some()),
        None=>ready(entry,kind),
    };
    if (captured.is_none() && original!=kind) || !ready_now
        || INSTALL.active() || !mii_backend::ready() {return;}
    let (catalog,at)={let s=STATE.lock().unwrap();(s.catalog.as_ref().map(Arc::clone),s.capture_at)};
    let Some(catalog)=catalog else {return;};
    // Keep the arm's generation/opening through native capture; never adopt a later lifetime.
    let result=if at==(epoch(),mii_saved_alpha::menu_epoch()) {
        native::capture_origin(entry as usize,at.0,original,&catalog,at.1)
    } else {Err(provenance::helpers::CaptureError::Refused("catalog capture retired"))};
    match result {
        Ok(())=>note("catalog owner qualified; immutable origin retained"),
        Err(provenance::helpers::CaptureError::Busy)=>{},
        Err(provenance::helpers::CaptureError::Refused(reason))=>{
            let mut s=STATE.lock().unwrap();
            if s.capture_at==at {s.capture=false;}
            drop(s);note(reason);
        }
    }
}

/// One idle snapshot. A busy read, a failed protected-union compare, or a
/// changed reservation/epoch leaves `expected` unchanged. The returned value
/// replaces only the descriptor; records, scalars, owners, and control stay
/// the caller's expected snapshot.
unsafe fn refresh_idle_metadata(entry:*mut u8,kind:u32,expected:native::Snapshot,live:Destination,request:Request,retained:bool,active_present:bool)->native::Snapshot {
    let generation=epoch();
    let reservation={
        let state=STATE.lock().unwrap();
        if INSTALL.active() || state.reservation.held()!=retained {return expected;}
        state.reservation
    };
    let Ok(now)=native::snapshot(entry as usize,generation,kind) else {return expected;};
    if now.binding.epoch!=generation {return expected;}
    let Ok(descriptor)=crate::engine::mii::ordinary_cpu::recapture_rank(
        now.layout,expected.binding,now.binding,&expected.descriptor,&now.descriptor) else {return expected;};
    if descriptor==expected.descriptor {return expected;}
    let comparison=native::Snapshot {descriptor,..expected};
    // Protected union first. A mismatch returns the caller's expected snapshot
    // and does not write STATE.
    if !native::same_owned(&now,&comparison,retained || active_present) {return expected;}
    let mut state=STATE.lock().unwrap();
    if INSTALL.active() || state.reservation!=reservation || epoch()!=generation
        || state.departed || state.busy() || state.generic.is_some() || state.request!=Some(request)
        || !state.owner.as_ref().is_some_and(|owner|owner.expected==expected && owner.live==live) {
        return expected;
    }
    state.owner.as_mut().unwrap().expected.descriptor=comparison.descriptor;
    drop(state);
    comparison
}

#[inline(never)]
unsafe fn begin(entry:*mut u8)->bool {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080013);
    let blocker=begin_blocker();
    note_waiting(blocker);
    if blocker.is_some() {return false;}
    let pending={
        let s=STATE.lock().unwrap();
        s.owner.as_ref().zip(s.request).map(|(o,r)|
             (o.origin,o.origin_saved,o.expected,o.live,r,s.catalog.as_ref().map(Arc::clone),s.reservation.held(),s.restore))
    };
    let Some((origin,origin_saved,expected,live,request,Some(catalog),retained,restore))=pending else{return false;};
    let active=live.target.publish(origin_saved);
    if !request_current(request,active) {
        clear_request(request);note("CPU request epoch/configuration changed");
        if request.direct_kind().is_some() {notify(Role::Cpu,"Character choice expired; select it again".into());}
        return false;
    }
    if !ready_descriptor(entry,&expected.descriptor) {return false;}
    // No ticket exists yet. Native Training CPU behavior (stop, attack, level,
    // Control) and waiting practice can change the CPU's live behavior bytes after
    // capture: the controller word FE+0x10..0x14 (Behavior=Control assigns a pad,
    // C104 #336: ffffffff>00000001) and FE+0x34..=0x37. Use the same descriptor-only
    // qualification as menu recapture. Every other byte and protected owner field must still match;
    // commit rechecks INSTALL, reservation, and epoch. The later fresh preflight
    // and all transaction phases remain byte-exact; origin is never updated.
    let expected=refresh_idle_metadata(entry,origin.original_kind,expected,live,request,retained,active.is_some());
    // Keep the immutable identity origin, but carry the CPU's live behavior so the
    // install preserves it. The fresh preflight and every transaction phase
    // below remain byte-exact against this refreshed expected descriptor.
    let live_origin=match crate::engine::mii::ordinary_cpu::with_native_live_metadata(origin.descriptor,&expected.descriptor) {
        Ok(descriptor)=>descriptor,
        Err(reason)=>{refuse_request(request,reason);note(reason);return false;}
    };
    let menu=setting(&MENU);
    // Only source/catalog/row target qualification can skip a cycle choice.
    // Current-owner, SameClass, INSTALL and publication checks remain below this resolver.
    let mii_target=|c:Configuration|->Result<(Target,[u8;0x70]),&'static str> {
        if !crate::engine::mii::ordinary_cpu::supported_kind(c.canonical().class as u32) {
            return Err("CPU class not qualified");
        }
        if crate::engine::mii::ordinary_cpu::cpu_hat_route(origin.layout,origin.binding.owner as usize,expected.scalars.e.hat,c.tokens().hat)?
            && !provenance::cpu_headgear_fingerprints() {return Err("CPU headgear fingerprint mismatch");}
        native::target_rows(c)?;
        Ok((Target::mii(c),crate::engine::mii::ordinary_cpu::staged_descriptor_in(origin.layout,origin.binding.owner as usize,live_origin,Some(c)).ok_or("CPU backend selector changed")?))
    };
    let qualify=|step:mii_cycle::Step|->Result<(Target,[u8;0x70]),&'static str> {
        let mut descriptor=live_origin;
        let target=match step.target {
            mii_cycle::Target::Saved(choice)=>{
                let c=catalog.configuration(choice)
                    .ok_or("saved target unavailable")?;
                return mii_target(c);
            }
            mii_cycle::Target::Original=>{
                if let Some(c)=origin_saved {native::target_rows(c)?;}
                Target::ORIGINAL
            },
            mii_cycle::Target::Fighter(kind)=>{
                let forms=restore.map_or_else(||requested_forms(&menu,Role::Cpu,kind),|v|v.forms);
                descriptor[0x14..0x18].copy_from_slice(&forms.count.to_le_bytes());
                for (i,k) in forms.kinds.iter().enumerate() {
                    descriptor[0x18+i*4..0x1c+i*4].copy_from_slice(&k.to_le_bytes());
                }
                descriptor[0x28..0x2c].copy_from_slice(&forms.actor_count.to_le_bytes());
                descriptor[0x43]=0;
                let index=if origin.layout.waiting() {origin.index}else{0};
                descriptor[0x46..0x48].copy_from_slice(&index.to_le_bytes());
                if kind==origin.original_kind && descriptor==live_origin {Target::ORIGINAL}
                else {Target::fighter(kind)}
            }
        };
        Ok((target,descriptor))
    };
    let resolved=if let Some(value)=restore {
        let original=TrioFighter::from_kind(origin.original_kind).is_some_and(|fighter|
            value.matches_original(fighter,descriptor_forms(&origin.descriptor),origin_saved));
        let step=mii_cycle::Step {cursor:value.cursor,target:if original {mii_cycle::Target::Original}
            else {mii_cycle::Target::Fighter(value.fighter.kind())}};
        let result=if original {qualify(step)} else if let Some(c)=value.configuration {mii_target(c)}else{qualify(step)};
        match result {Ok(value)=>Some((step,value)),Err(reason)=>{refuse_request(request,reason);note(reason);return false;}}
    } else if request.refresh_partners() {
        mii_cycle::refresh_partners(logical_kind(&expected),origin.original_kind,live.cursor,false)
            .and_then(|step|match qualify(step) {
                Ok(value)=>Some((step,value)),
                Err(reason)=>{note(reason);None}
            })
    } else if let Some(replay)=request.replay() {
        let step=mii_cycle::Step {target:mii_cycle::Target::Fighter(replay.target.kind()),cursor:live.cursor};
        let result=match replay.target.configuration() {Some(c)=>mii_target(c),None=>qualify(step)};
        match result {Ok(value)=>Some((step,value)),Err(reason)=>{
            refuse_request(request,reason);note(reason);return false;
        }}
    } else if let Some(kind)=request.direct_kind() {
        let result=mii_cycle::direct(kind,&catalog,&menu::mii_preferences(),
            crate::engine::mii::selection::Role::Cpu,
            origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())))
            .ok_or("selected CPU character unavailable")
            .and_then(|step|qualify(step).map(|value|(step,value)));
        match result {Ok(value)=>Some(value),Err(reason)=>{
            refuse_request(request,reason);note(reason);
            notify(Role::Cpu,"This CPU character cannot be loaded".into());return false;
        }}
    } else if let Some(choice)=request.choice() {
        let step=mii_cycle::Step{target:mii_cycle::Target::Saved(choice),cursor:live.cursor};
        match qualify(step) {
            Ok(value)=>Some((step,value)),
            Err(reason)=>{
                clear_request(request);
                trace!("[MiiOrdinaryCpu] menu-close choice refused class={:?} slot={:?} reason={}",
                    choice.canonical.class,choice.slot_id,reason);
                notify(Role::Cpu,"This CPU Mii configuration cannot be loaded".into());
                return false;
            }
        }
    } else {
        let identity=active.map_or(Identity::Fighter(logical_kind(&expected)),
            |c|Identity::Saved(c.canonical()));
        let current=if live.cursor.is_none() && live.target==Target::ORIGINAL {Live::Original}
            else {Live::Numbered{identity,cursor:live.cursor}};
        mii_cycle::next_qualified_cpu(&menu.cpu_roster.fighters(),&catalog,&menu::mii_preferences(),
            origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())),current,|step|match qualify(step) {
                Ok(value)=>Some(value),
                Err(reason)=>{
                    // Source-only refusal; no ticket, live cursor or preference publication.
                    if let mii_cycle::Target::Saved(choice)=step.target {
                        trace!("[MiiOrdinaryCpu] skipped unqualified cycle choice class={:?} slot={:?} reason={}",
                            choice.canonical.class,choice.slot_id,reason);
                        notify(Role::Cpu,format!("CPU {:?} choice skipped",choice.canonical.class).into());
                    }
                    None
                }
            })
    };
    let Some((step,(target,descriptor)))=resolved else {
        clear_request(request);
        if !request.refresh_partners() {
            note("CPU cycle has no qualified choice");
            notify(Role::Cpu,"No available CPU cycle characters".into());
        }
        return false;
    };
    let destination=Destination{target,cursor:step.cursor};
    let publish=target.publish(origin_saved);
    let current=match native::snapshot(entry as usize,epoch(),origin.original_kind) {
        Ok(now) if native::same_owned(&now,&expected,retained || active.is_some())=>now,
        Err(provenance::helpers::BUSY)=>return false,
        Err(reason)=>{refuse_request(request,reason);note(reason);return false;}
        Ok(now)=>{refuse_request(request,"preflight current configuration changed");note_preflight_mismatch(&expected,&now);return false;}
    };
    // Include saved/ordinary returns. Admission and phase publication both recheck
    // the current-to-target transition; the origin's hat alone is insufficient.
    let hat_change=match crate::engine::mii::ordinary_cpu::cpu_hat_route(current.layout,
        current.binding.owner as usize,current.scalars.e.hat,publish.map_or(origin.scalars.e.hat,|c|c.tokens().hat)) {
        Ok(changed) if !changed || provenance::cpu_headgear_fingerprints()=>changed,
        Ok(_)=>{refuse_request(request,"CPU headgear fingerprint mismatch");note("CPU headgear fingerprint mismatch");return false;}
        Err(reason)=>{refuse_request(request,reason);note(reason);return false;}
    };
    // A qualified already-active group/configuration advances only its numbered cursor;
    // no INSTALL or record publication is needed.
    // A ready owned bank whose live member is already this ordinary target needs
    // only the same cursor/receipt update, never a member switch or INSTALL.
    let bank_same=bank_live(entry,current.binding.owner,&current.descriptor).is_some_and(|live|
        live==descriptor_kind(&descriptor) && descriptor_forms(&descriptor)==bank_member_forms(&current.descriptor,live));
    if (bank_same || (descriptor==current.descriptor && ready(entry,descriptor_kind(&descriptor))))
        && if request.replay().is_some() || restore.is_some() {active==publish} else {
            crate::engine::mii::role_owner::payload_already_active(active,publish,
                destination.target==Target::ORIGINAL)
        } {
        let mut s=STATE.lock().unwrap();
        if !s.departed && !s.busy() && s.request==Some(request)
            && request_current(request,active) {
            if let Some(o)=s.owner.as_mut().filter(|o|o.expected==expected && o.live==live) {
                let Ok(registry)=provenance::final_registry(current.helpers,None) else{return false;};
                // A deduplicated canonical choice may name another saved-row copy.
                // Keep the actually qualified payload; only its cycle position changes.
                o.live=Destination {cursor:destination.cursor,..o.live};
                s.replay_result=request.replay().map(|selection|crate::engine::mii::replay::SelectionReceipt {selection,outcome:Ok(())});s.request=None;s.restore=None;drop(registry);drop(s);
                if request.replay().is_none() {state_for(Role::Cpu).lock().unwrap().cursor=destination.cursor;}
                publish_partners(Role::Cpu,descriptor_forms(&descriptor));
                if request.replay().is_none() {remember_owned(Role::Cpu,current.binding.epoch,descriptor_kind(&descriptor),descriptor_forms(&descriptor),
                    active,destination.cursor,destination.target==Target::ORIGINAL);}
                if let Some(fighter)=TrioFighter::from_kind(descriptor_kind(&descriptor)).filter(|_|request.replay().is_none()) {
                    Role::Cpu.sync_selection(fighter);
                }
                if !request.refresh_partners() {notify(Role::Cpu,"CPU choice already active".into());}
            }
        }
        return false;
    }
    // Receipt renewal cannot become a fighter change if user intent or native
    // state changed since it was queued. Only the qualified no-INSTALL branch above applies.
    if !request.can_install() {clear_request(request);return false;}
    // A bank-built Mii window (`+0x591f == 4`) has no retained canonical resource pair
    // for `prepare_same_class` (WP-M2): its same-class change takes the fresh-O path.
    let mii_window=MiiClass::from_kind(descriptor_kind(&descriptor)).is_some()
        && read::<u8>(current.binding.entry as *const u8,0x591f)==crate::engine::mii::ordinary_player::MII_BANK_WINDOW_FLAGS;
    let same_class=match (active,publish) {
        (Some(before),Some(after)) if !hat_change && !mii_window
            && crate::engine::mii::ordinary_cpu::retained_same_class_candidate(before,after)
            && current.index==current.binding.record_index
            && u16::from_le_bytes([descriptor[0x46],descriptor[0x47]])==current.binding.record_index=>
            match native::prepare_same_class(current,before,after) {
                Ok(proof)=>Some(proof),Err(reason)=>{refuse_request(request,reason);note(reason);return false;}
            },
        _=>None,
    };
    // Ordinary cycle/direct/restored/replayed fighters may use this owner's same-entry
    // bank, and so may an ordinary single-body original while `original_bank` is on
    // (EXCLUSIONS row 4); saved, same-class and every Mii keep the phase-6 path. The
    // bank planner owns eligibility and budget; upcoming kinds come from the real
    // mixed CPU cycle after this destination.
    #[cfg(feature="native-body-bank-alpha")]
    let plan={
        let after=match destination.target.kind() {
            TargetKind::Fighter(kind) if publish.is_none() && same_class.is_none()
                && descriptor_kind(&descriptor)==kind =>
                Some(Live::Numbered {identity:Identity::Fighter(kind),cursor:destination.cursor}),
            TargetKind::Original if publish.is_none() && same_class.is_none() && origin_saved.is_none()
                && descriptor_kind(&descriptor)==origin.original_kind
                && crate::engine::swap::body_bank::original_bankable(origin.original_kind)
                && crate::runtime::roster::cycle_preload_config::snapshot().original_bank_enabled() => Some(Live::Original),
            // A saved/preset Mii with no same-class proof may be served by its own
            // one-member group (MII-GROUPS-DESIGN §5.1 G10); a replay "switch to the
            // recorded fighter" is not a lane target and stays canonical.
            TargetKind::Saved => match publish {
                Some(c) if same_class.is_none() && request.replay().is_none()
                    && MiiClass::from_kind(descriptor_kind(&descriptor)).is_some()
                    && crate::runtime::roster::cycle_preload_config::snapshot().mii_groups_enabled()
                    && super::body_bank::reachable_mii_group(current.binding.owner,descriptor_kind(&descriptor))
                        .is_some_and(|(_,group_c)|group_c==c)
                    // Interim rule (WP-M2): the native Mii-setup preflight must hold
                    // before any group mutation; otherwise canonical.
                    && match super::body_bank::mii_setup_admitted(current.binding.entry,current.binding.owner,descriptor_kind(&descriptor)) {
                        Ok(())=>true,
                        Err(reason)=>{note(format_args!("[MiiOrdinaryCpu] Mii group canonical before mutation: {}",reason));false}
                    } =>
                    Some(Live::Numbered {identity:Identity::Saved(c.canonical()),cursor:destination.cursor}),
                _=>None,
            },
            _=>None,
        };
        after.map(|after| {
            let upcoming:Vec<u32>=crate::runtime::roster::cycle_preload::mixed_targets(&menu,1,
                origin_saved.map_or(Identity::Fighter(origin.original_kind),|c|Identity::Saved(c.canonical())),
                after,&catalog,origin_saved).iter().flatten()
                .filter(|target|target.configuration.is_none()).map(|target|target.kind).collect();
            super::body_bank::plan(entry,current.binding.owner,descriptor,&upcoming)
        })
    };
    #[cfg(feature="native-body-bank-alpha")]
    let (physical,bank_build,reuse)=plan.map_or((descriptor,false,false),
        |p|(p.descriptor,!p.reuse && p.is_bank(),p.reuse));
    #[cfg(not(feature="native-body-bank-alpha"))]
    let (physical,bank_build,reuse)=(descriptor,false,false);
    if !permitted(){return false;}
    let Some(pose)=capture_reload_pose(entry,current.binding.owner) else {
        refuse_request(request,"CPU position not ready before reload");return false;
    };
    let Some(token)=INSTALL.try_begin(true) else{return false;};
    if native::snapshot(entry as usize,current.binding.epoch,origin.original_kind)!=Ok(current) {
        refused_before_submit(token,"CPU preflight changed before INSTALL ownership");return false;
    }
    let track={
        let mut s=STATE.lock().unwrap();
        if s.departed || s.busy() || s.request!=Some(request) || epoch()!=current.binding.epoch
            || !request_current(request,active)
            || !s.owner.as_ref().is_some_and(|o|o.expected==expected && o.live==live) {
            drop(s);refused_before_submit(token,"ordinary pre-submit refusal");return false;
        }
        let sequence=s.next;
        let Some(next)=sequence.checked_add(1) else {
            s.request=None;drop(s);refused_before_submit(token,"ordinary ticket exhausted");return false;
        };
        let ticket=Ticket{epoch:epoch(),entry:entry as usize,sequence};
        let registry=match provenance::final_registry(current.helpers,Some(token)) {
            Ok(registry)=>registry,
            Err(_)=>{drop(s);refused_before_submit(token,"CPU source provenance changed before submit");return false;}
        };
        if !s.reservation.acquire(INSTALL.owns(token)) {
            drop(registry);drop(s);refused_before_submit(token,"CPU reservation refused before INSTALL");return false;
        }
        s.next=next;s.request=None;
        if reuse {
            // Published before the native request; settled only by bank_service.
            #[cfg(feature="native-body-bank-alpha")]
            {s.bank=Some(BankTrack {ticket,token,before:current,logical:descriptor_kind(&descriptor),
                destination,retained:retained || active.is_some(),replay:request.replay(),
                publish,descriptor,hat_change,origin_index:origin.index,
                origin_record:crate::engine::mii::role_owner::current_record_claim(origin.original_kind,origin.index)
                    .then_some(origin.current_record),published:None});}
            drop(registry);None
        } else {
            let t=Track {ticket,gate:Gate::new(ticket).unwrap(),token,installing:true,retained_o:false,recovering:None,
                data:Arc::new(Prepared {before:current,origin_scalars:origin.scalars,hat_change,
                    origin_index:origin.index,origin_record:crate::engine::mii::role_owner::current_record_claim(origin.original_kind,origin.index)
                        .then_some(origin.current_record),
                    descriptor:physical,publish,destination,same_class,replay:request.replay(),bank:bank_build})};
            s.track=Some(t.clone());drop(registry);Some(t)
        }
    };
    let Some(track)=track else {
        #[cfg(feature="native-body-bank-alpha")]
        return submit_bank(entry,plan,pose);
        #[cfg(not(feature="native-body-bank-alpha"))]
        return false;
    };
    let staged=native_fighter_install::Descriptor::from_bytes(&track.descriptor);
    training::clear_session_state();
    trace!("[MiiOrdinaryCpu] submit ticket={:?} outgoing={:#x} target={:#x} index={} cursor={:?} resource_before={:02x?} saved_files_written=false",
        track.ticket,live_kind(entry,current.binding.owner,&current.descriptor),track.reload_target().kind,current.index,
        track.destination.cursor,&current.record[88..104]);
    trace!("[MiiOrdinaryCpu] staged ticket={:?} bank_build={} descriptor={:02x?}",track.ticket,track.bank,track.descriptor);
    #[cfg(feature="native-body-bank-alpha")]
    let submitted=match plan {
        Some(p) if track.bank => super::body_bank::submit(entry,current.binding.owner,p,pose),
        _ => {reload_at_pose(base(),entry,&staged,pose);true}
    };
    #[cfg(not(feature="native-body-bank-alpha"))]
    let submitted={reload_at_pose(base(),entry,&staged,pose);true};
    if !submitted {
        // The bank refused before any native write: no Reload was issued.
        let owned={
            let mut s=STATE.lock().unwrap();
            let owned=s.track.as_ref().is_some_and(|t|t.ticket==track.ticket);
            if owned {
                s.track=None;
                if let Some(selection)=track.replay {
                    s.replay_result=Some(crate::engine::mii::replay::SelectionReceipt {
                        selection,outcome:Err("CPU bank build refused")});
                }
            }
            owned
        };
        if owned {settle_refusal(Refusal {reason:"CPU bank build refused before native Reload",lease:track.token},Some(track.ticket));}
        note("bank build refused before native Reload; request cleared");
        return false;
    }
    if let Some(current)=STATE.lock().unwrap().track.as_mut().filter(|t|t.ticket==track.ticket) {
        current.installing=false;
    }
    true
}

/// Warm member switch after the BankTrack is published, INSTALL and pose owned.
/// A refusal performed no native write; release only this token's ownership.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn submit_bank(entry:*mut u8,plan:Option<super::body_bank::Plan>,
    pose:super::native_reload_pose::ReloadPose)->bool {
    let bank=STATE.lock().unwrap().bank;
    let Some(b)=bank.filter(|b|b.ticket.entry==entry as usize) else {return false;};
    let owner=b.before.binding.owner;
    // A Mii group switch publishes the exact record before the native request
    // (MII-GROUPS-DESIGN §5.2), with the group's own identity as the hat proof. The
    // publication is recorded in the track as it is written; every refusal from here on
    // puts it back in `bank_refused` (MII-PHASEB).
    if let Some(c)=b.publish {
        let Some((identity,group_c))=super::body_bank::reachable_mii_group(owner,b.logical) else {
            bank_refused(b,"no reachable CPU Mii group for the published configuration",false);
            return false;
        };
        if group_c!=c {
            bank_refused(b,"reachable CPU Mii group was planned for another configuration",false);
            return false;
        }
        let mut evidence=None;
        match native::bank_publish(&b,c,identity.hat(),&mut evidence) {
            Ok(())=>trace!("[MiiOrdinaryCpu] bank publish ticket={:?} target={:#x} index={}->{} hat={} evidence={} charinfo_written=true scalars_written=true",
                b.ticket,b.logical,b.before.index,b.before.binding.record_index,identity.hat(),evidence.is_some()),
            Err(reason)=>{bank_refused(b,reason,false);return false;}
        }
    }
    // The outgoing member, read while the bank is still Ready (`switch` clears its observation).
    let outgoing=live_kind(entry,owner,&b.before.descriptor);
    let switched=match plan {
        Some(p) if p.reuse && p.target==b.logical => super::body_bank::submit(entry,owner,p,pose),
        _ => false,
    };
    if !switched {
        bank_refused(b,"CPU bank member switch refused before the native request",false);
        return false;
    }
    training::clear_session_state();
    trace!("[MiiOrdinaryCpu] bank switch ticket={:?} outgoing={:#x} target={:#x} index={} cursor={:?} mii={}",
        b.ticket,outgoing,b.logical,b.before.index,b.destination.cursor,b.publish.is_some());
    true
}

/// Settle a CPU member switch in this owner's ready callback; never forced.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_service(entry:*mut u8,b:BankTrack) {
    let live=super::body_bank::ready_kind(entry,b.before.binding.owner);
    if STATE.lock().unwrap().departed {
        if live.is_some() || settled(entry) {bank_refused(b,"CPU scene departed during bank switch",false);}
        return;
    }
    // A failed identity proof is terminal even if the current body reports the same
    // kind; never adopt a new body generation. Set only after native settlement.
    if super::body_bank::failed(entry,b.before.binding.owner) {
        bank_refused(b,"CPU bank identity failed after native settlement",true);
        return;
    }
    match live {
        Some(kind) if kind==b.logical => bank_settle(entry,b),
        // Ready on another member with the same owned bank: nothing to adopt.
        Some(_) => bank_refused(b,"CPU bank settled on another member",false),
        None if settled(entry) => bank_refused(b,"CPU native settled without the requested bank member",true),
        None => {},
    }
}

#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_settle(entry:*mut u8,b:BankTrack) {
    let settled=match native::bank_settled(&b) {
        Ok(settled)=>settled,
        Err(native::BankRefusal::Busy)=>return,
        Err(native::BankRefusal::Kept(reason))=>{bank_refused(b,reason,false);return;}
        Err(native::BankRefusal::Diverged(reason))=>{bank_refused(b,reason,true);return;}
    };
    let now=settled.now;let exact=settled.exact;
    let logical=b.logical;
    // A Mii group switch whose exact proof was adopted publishes its configuration
    // (the exact HUD portrait); an ordinary switch publishes the live kind.
    #[cfg(feature="mii-hud-portrait-alpha")]
    publish_mii_presentation(entry,b.before.binding.owner,b.ticket.epoch,logical,
        exact.map_or(super::PortraitIntent::NotMii,super::PortraitIntent::Exact));
    #[cfg(not(feature="mii-hud-portrait-alpha"))]
    publish_mii_presentation(entry,b.before.binding.owner,b.ticket.epoch,logical);
    crate::runtime::roster::cycle_preload::foreground_settled();
    let released=settle_outcome(Outcome::Applied {receipt:receipt_of(&now,logical),lease:b.token},b.ticket,None,true);
    if b.replay.is_none() {state_for(Role::Cpu).lock().unwrap().cursor=b.destination.cursor;}
    let forms=bank_member_forms(&now.descriptor,logical);
    let original=b.destination.target==Target::ORIGINAL;
    publish_partners(Role::Cpu,forms);
    if b.replay.is_none() {remember_owned(Role::Cpu,b.ticket.epoch,logical,forms,exact,b.destination.cursor,original);}
    if let Some(fighter)=TrioFighter::from_kind(logical).filter(|_|b.replay.is_none()) {
        Role::Cpu.sync_selection(fighter);
    }
    crate::runtime::roster::cycle_preload::completed(1,false,logical);
    trace!("[MiiOrdinaryCpu] bank ready ticket={:?} target={:#x} physical={:#x} index={} original={} mii={} resource_pair={:?} released={}",
        b.ticket,logical,descriptor_kind(&now.descriptor),now.index,original,exact.is_some(),settled.resource_pair,released);
    notify(Role::Cpu,crate::engine::mii::ordinary_player::bank_notice(false,exact.is_some(),original).into());
}

/// Terminal BankTrack refusal. The pending track, request and restore go; the owner
/// is kept and no authority is retired, whatever the reason (WP-M2 cascade guard,
/// `bank_refusal_effects`; B124PRE D-MII2 `CPU cycle refused: original assignment not
/// captured` followed from the owner drop). The retained G1 reservation still ends
/// only at exact destruction.
///
/// MII-PHASEB: the one compensation point of the switch-time publication, as in the Player
/// adapter: a recorded `bank_publish` is put back and read back under G before INSTALL goes;
/// a restore that met the transient `helpers::BUSY` before writing keeps the track and retries.
#[cfg(feature="native-body-bank-alpha")]
unsafe fn bank_refused(b:BankTrack,reason:&'static str,diverged:bool) {
    let effects=crate::engine::mii::ordinary_player::bank_refusal_effects(diverged);
    let published=match STATE.lock().unwrap().bank {
        Some(now) if now.ticket==b.ticket && now.token==b.token=>now.published,
        _=>return,
    };
    // Lock order G -> STATE: the restore takes G and STATE itself; STATE is not held across it.
    let restored=published.map(|(c,p)|native::bank_restore(&b,c,p));
    if matches!(restored,Some(Err(why)) if why==provenance::helpers::BUSY) {
        trace!("[MiiOrdinaryCpu] bank publication restore deferred ticket={:?} why=busy reason={}",b.ticket,reason);
        return;
    }
    let mut s=STATE.lock().unwrap();
    if !s.bank.is_some_and(|now|now.ticket==b.ticket && now.token==b.token) {return;}
    if effects.drop_bank {s.bank=None;}
    if let Some(selection)=b.replay {
        s.replay_result=Some(crate::engine::mii::replay::SelectionReceipt {selection,outcome:Err(reason)});
    }
    if effects.drop_request {s.request=None;s.restore=None;}
    if !effects.keep_owner {s.owner=None;s.menu_valid=false;}
    // Native's own pair goes into the receipt only of the owner the restore compared against,
    // and the current record only when this owner claims it (`current_record_claim`).
    if let (Some(Ok(r)),Some(o))=(&restored,s.owner.as_mut()) {
        if r.receipt==crate::engine::mii::ordinary_player::RestoreReceipt::NativePairAdopted
            && native::same_owned(&o.expected,&b.before,b.retained) {
            o.expected.record=r.now.record;
            if r.current_claimed {o.expected.current_record=r.now.current_record;}
        }
    }
    let owner_kept=s.owner.is_some();
    let departed=s.departed;
    drop(s);
    match &restored {
        Some(Ok(r))=>trace!("[MiiOrdinaryCpu] bank publication restored ticket={:?} verified=true receipt={:?} reason={}",b.ticket,r.receipt,reason),
        Some(Err(why))=>trace!("[MiiOrdinaryCpu] bank publication NOT restored ticket={:?} departed={} why={} reason={}",b.ticket,departed,why,reason),
        None=>{}
    }
    let diverged=diverged || (matches!(restored,Some(Err(_))) && !departed);
    if effects.retire_authority {for id in [1,4] {super::mii_ordinary_player::native::retire_authority(id,b.ticket.entry);}}
    // WP5b: diverged = native settled on a body this owner cannot adopt; otherwise nothing native
    // ran, or what ran is compensated (the bank's Phase B rollback, the publication restore above).
    let released=if diverged {settle_outcome(Outcome::NativeSettledUnowned {reason,lease:b.token},b.ticket,None,true)}
        else {settle_refusal(Refusal {reason,lease:b.token},Some(b.ticket))};
    trace!("[MiiOrdinaryCpu] bank switch refused ticket={:?} diverged={} owner_kept={} INSTALL_released={} reason={}",
        b.ticket,diverged,owner_kept,released,reason);
    if b.replay.is_none() {notify(Role::Cpu,"This CPU character cannot be loaded".into());}
}

/// A bank build is a cache load with native already settled. Never adopt the new
/// physical receipt: drop authority (no native writes) and release INSTALL.
fn bank_build_refused(t:&Track,reason:&'static str) {
    let mut s=STATE.lock().unwrap();
    if !s.track.as_ref().is_some_and(|now|now.ticket==t.ticket) {return;}
    s.track=None;s.owner=None;s.menu_valid=false;s.request=None;s.restore=None;
    if let Some(selection)=t.replay {
        s.replay_result=Some(crate::engine::mii::replay::SelectionReceipt {selection,outcome:Err(reason)});
    }
    drop(s);
    for id in [1,4] {super::mii_ordinary_player::native::retire_authority(id,t.ticket.entry);}
    let released=settle_outcome(Outcome::NativeSettledUnowned {reason,lease:t.token},t.ticket,None,t.gate.published());
    trace!("[MiiOrdinaryCpu] bank build refused ticket={:?} INSTALL_released={} reason={}",t.ticket,released,reason);
}

/// WP5b: the one INSTALL release site for CPU tracks after their first native effect. Every
/// terminal path builds the `transition::Outcome` that names it; the lease travels inside the
/// outcome, so a path can neither release twice nor forget to release, and this emits the single
/// `[Txn] outcome=` line the Eden rows grep.
fn settle_outcome(outcome:Outcome<InstallToken>,ticket:Ticket,loader:Option<u8>,published:bool)->bool {
    let name=outcome.name();let reason=outcome.reason();
    let released=finish_install(outcome.into_lease(),name);
    trace!("[Txn] outcome={} role=cpu ticket={:?} reason={} loader={:?} published={} INSTALL_released={} INSTALL_active={}",
        name,ticket,reason,loader,published,released,INSTALL.active());
    released
}
/// WP5b: a refusal before any native effect (preflight/submit): nothing to compensate, the lease
/// comes straight back (`transition::Refusal`).
fn settle_refusal(refusal:Refusal<InstallToken>,ticket:Option<Ticket>)->bool {
    let released=finish_install(refusal.lease,refusal.reason);
    trace!("[Txn] outcome=Refused role=cpu ticket={:?} reason={} INSTALL_released={} INSTALL_active={}",
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
        header:now.header,descriptor:now.descriptor,index:now.index,..TxnIdentity::ZERO},
        kind}
}
/// WP5b: a committed track whose native side cannot be adopted enters a named recovery exactly
/// once (`AwaitDestruction`: INSTALL stays owned until exact destruction, as before) instead of
/// an anonymous `Refused` stall — the DEFECT #2 shape `phase=Refused loader=7 INSTALL_active=true`.
fn note_recovering(t:&mut Track,reason:&'static str,loader:Option<u8>)->Option<RecoveringNote> {
    if t.recovering.is_some() {return None;}
    t.recovering=Some(reason);
    Some(RecoveringNote {ticket:t.ticket,reason,loader,published:t.gate.published()})
}
fn trace_recovering(note:Option<RecoveringNote>) {
    if let Some(n)=note {
        trace!("[Txn] recovering role=cpu ticket={:?} how=AwaitDestruction reason={} loader={:?} published={} INSTALL_active={}",
            n.ticket,n.reason,n.loader,n.published,INSTALL.active());
    }
}

/// Wrapper ownership is separate from the observer/service callback's Rust early return.
pub(super) unsafe fn enter_native(entry:*mut u8)->bool {
    let mut s=STATE.lock().unwrap();
    let Some(t)=s.track.as_mut().filter(|t|t.ticket.entry==entry as usize) else{return true;};
    if t.installing {return false;} // Suppress a same-entry callback nested inside INSTALL.
    if !super::mii_ordinary_player::native::span(entry as usize,0x5a88,false) || epoch()!=t.ticket.epoch {
        t.gate.refuse();return false;
    }
    t.gate.enter(t.ticket,read::<u8>(entry,0x591c))
}
pub(super) unsafe fn leave_native(entry:*mut u8) {
    let event={
        let mut s=STATE.lock().unwrap();
        s.track.as_mut().filter(|t|t.ticket.entry==entry as usize).and_then(|t| {
            let loader=read::<u8>(entry,0x591c);
            t.gate.leave(t.ticket,loader);
            // WP5b / DEFECT #2: the native loader refused the request (>=7) before publication and
            // this track is not the D-PTA1b native continuation: name the recovery once.
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
        trace!("[MiiOrdinaryCpu] native_return ticket={:?} phase={:?} loader={} published={} INSTALL_active={}",
            ticket,phase,loader,published,INSTALL.active());
        trace_recovering(note);
    }
}
pub(super) fn tracked(entry:usize)->bool {
    STATE.lock().unwrap().track.as_ref().is_some_and(|t|t.ticket.entry==entry)
}
/// The tracked request's staged descriptor when it carries a Mii payload, and whether it is the
/// same-class retained-O arm (U5: the gate's configure-skip writer reads it; cloned under STATE).
pub(super) fn mii_payload_descriptor(entry:usize)->Option<([u8;0x70],bool)> {
    STATE.lock().unwrap().track.as_ref().filter(|t|t.ticket.entry==entry && t.publish.is_some())
        .map(|t|(t.descriptor,t.same_class.is_some()))
}
/// `installed`: the configure-equivalent words the gate wrote into the selected O on the skip path
/// (U5, W-M2-4); the publication proof admits exactly them (`ExpectedMutation::ConfigureSkipped`).
pub(super) unsafe fn prequeue(entry:usize,selected:usize,
    installed:Option<crate::engine::swap::identity_view::ConfiguredMiiWords>)->bool {
    let track={
        let s=STATE.lock().unwrap();
        let Some(t)=s.track.as_ref().filter(|t|t.ticket.entry==entry) else{return true;};
        if !t.gate.may_publish(t.ticket,6) {drop(s);refuse(entry,"duplicate/reentrant gate");return false;}
        t.clone()
    };
    // WP-E (U14): the CPU Mii publication writes native records/scalars; it runs in Training and
    // waiting rooms only. A refusal is the ordinary prequeue refusal (forward continuation follows).
    if !crate::runtime::roster::scope_gate::admitted("mii_ordinary_cpu::prequeue") {
        refuse(entry,"publication outside Training/waiting rooms (scope)");return false;
    }
    let allowed=installed.map_or(crate::engine::mii::identity::ExpectedMutation::None,
        |w|crate::engine::mii::identity::ExpectedMutation::ConfigureSkipped {index:w.index,tokens:w.tokens});
    let mut evidence=None;
    let result=if track.same_class.is_some() {
        native::publish_same_class(&track,selected,&mut evidence)
    } else {native::publish(&track,selected,&mut evidence,allowed).map(|()|false)};
    if let Some(e)=evidence {
        trace!("[MiiOrdinaryCpu] prequeue ticket={:?} selected={:#x} initialized={} header={:x?} Oindex={} Otokens={:x?} task={:#x} outstanding={:?} Dindex={} rebuild={} current={:?}",
            track.ticket,e.selected,e.initialized,e.header,e.o_index,e.o_tokens,e.task,e.outstanding,e.d_index,e.rebuild,e.before);
    }
    match result {
        Ok(retained_o)=>{
            trace!("[MiiOrdinaryCpu] published ticket={:?} target={:#x} O={:#x} index={} retained_same_class_O={} native_resources_untouched=true",
                track.ticket,track.reload_target().kind,selected,track.before.binding.record_index,retained_o);
            true
        }
        Err(reason)=>{refuse(entry,reason);false}
    }
}
pub(super) fn refuse(entry:usize,reason:&str) {
    let mut s=STATE.lock().unwrap();
    let event=s.track.as_mut().filter(|t|t.ticket.entry==entry).map(|t| {
        t.gate.refuse();(t.ticket,t.gate.phase(),t.gate.published())
    });
    drop(s);
    if let Some((ticket,phase,published))=event {
        trace!("[MiiOrdinaryCpu] refused ticket={:?} phase={:?} published={} INSTALL_active={} reason={}",
            ticket,phase,published,INSTALL.active(),reason);
    }
}
/// D-PTA1b, generalised by WORKAROUND-DECISIONS row 7 (operator-approved root fix): a prequeue
/// refusal after `reload_at_pose` must not strand the entry at loader 6 with its window already
/// destroyed, so every unpublished refused track continues natively — the staged descriptor is what
/// the native state-6 tail installs (a Mii payload installs with the native-current Mii rows, its
/// publication skipped); the track stays `Refused`, is never published, and `service` releases it
/// as `NativeSettledUnowned` when native settles (loader 14). Eden E2 validates the Mii cases.
pub(super) fn continue_natively(entry:usize)->bool {
    let mut s=STATE.lock().unwrap();
    let event=s.track.as_mut().filter(|t|t.ticket.entry==entry).and_then(|t| {
        t.gate.continue_natively()
            .then_some((t.ticket,t.reload_target().kind,t.bank,t.publish.is_some() || t.same_class.is_some()))
    });
    drop(s);
    if let Some((ticket,target,bank,mii_payload))=event {
        trace!("[MiiOrdinaryCpu] refusal continues natively ticket={:?} target={:#x} bank_build={} INSTALL_active={}",
            ticket,target,bank,INSTALL.active());
        // WP5b: the forward continuation (`RecoveryPlan::{PLAIN_KIND, MII_PAYLOAD}.late_mismatch`);
        // settles in `service` as `NativeSettledUnowned` once the loader is idle.
        trace!("[Txn] recovering role=cpu ticket={:?} how=NativeContinue reason=prequeue refused; staged descriptor installs natively target={:#x} mii_payload={} INSTALL_active={}",
            ticket,target,mii_payload,INSTALL.active());
        true
    } else {false}
}
/// Capture the current qualified owner, including already committed helper/P1
/// changes. A missing receipt never alters the generic load or grants authority.
#[inline(never)]
pub(super) unsafe fn begin_generic(entry:*mut u8,token:InstallToken,descriptor:&[u8;0x70]) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080015);
    if !enabled() {return;}
    let pending={
        let mut s=STATE.lock().unwrap();
        s.generic=None;
        let busy=s.departed || s.busy() || s.request.is_some() || s.reservation.held();
        s.owner.as_ref().filter(|o|o.origin.binding.entry==entry as usize).map(|o|(
            o.origin.original_kind,o.expected,o.live,
            busy || o.origin_saved.is_some() || o.live.target.saved().is_some()))
    };
    let Some((original_kind,expected,live,busy))=pending else {return;};
    if busy || MiiClass::from_kind(descriptor_kind(descriptor)).is_some() {
        note("generic CPU begin not recorded: owned transaction or Mii target");return;
    }
    let before=match native::snapshot(entry as usize,epoch(),original_kind) {
        Ok(now) if native::same_owned(&now,&expected,false) && now.index==expected.index=>now,
        Ok(_)=>{note("generic CPU began from an unqualified owner state");return;}
        Err(reason)=>{note(reason);return;}
    };
    let mut s=STATE.lock().unwrap();
    if s.departed || s.busy() || s.request.is_some() || s.reservation.held()
        || epoch()!=before.binding.epoch
        || !s.owner.as_ref().is_some_and(|o|o.expected==expected && o.live==live) {
        drop(s);note("generic CPU begin retired");return;
    }
    s.generic=Some(Generic {token,before,expected,live,descriptor:*descriptor});
}
#[inline(never)]
pub(super) unsafe fn complete_generic(entry:*mut u8,token:InstallToken,kind:u32,cursor:Option<usize>) {
    #[cfg(feature = "console-swap-trace")]
    let _swap_stack = crate::runtime::roster::practice_trace::scope(0x1080016);
    if !enabled() {return;}
    let generic={
        let mut s=STATE.lock().unwrap();
        if s.generic.as_ref().is_some_and(|g|g.token==token && g.before.binding.entry==entry as usize) {
            s.generic.take()
        } else {None}
    };
    let Some(g)=generic else {return;};
    match native::generic_completed(&g,kind,cursor) {
        Ok(now)=>trace!("[MiiOrdinaryCpu] generic CPU owner committed kind={:#x} index={} cursor={:?}",
            kind,now.index,cursor),
        Err(reason)=>trace!("[MiiOrdinaryCpu] generic CPU commit refused; owner unchanged reason={}",reason),
    }
}
pub(super) fn scene_changed() {
    let mut s=STATE.lock().unwrap();s.capture=false;s.menu_valid=false;s.catalog=None;s.request=None;s.replay_result=None;s.recapture_log=None;
    s.departed=true;s.generic=None;s.restore=None;
    // A pending bank switch observes departed and settles without publication.
    if let Some(t)=s.track.as_mut(){t.gate.depart();}
    // A completed or returned transaction still retains native G1 until exact destruction.
    if !s.reservation.held() {s.owner=None;s.departed=false;}
}
pub(super) fn on_destroy(entry:*const u8) {
    let token={
        let mut s=STATE.lock().unwrap();
        if s.generic.as_ref().is_some_and(|g|g.before.binding.entry==entry as usize) {s.generic=None;}
        let exact=s.owner.as_ref().is_some_and(|o|o.origin.binding.entry==entry as usize);
        if exact {s.owner=None;s.request=None;s.restore=None;s.replay_result=None;s.capture=false;s.menu_valid=false;s.recapture_log=None;s.reservation.retire(true);s.departed=false;}
        #[cfg(feature="native-body-bank-alpha")]
        if let Some(b)=s.bank.filter(|b|b.ticket.entry==entry as usize) {
            s.bank=None;
            let released=settle_outcome(Outcome::Destroyed {reason:"ordinary CPU1 bank switch destructor",lease:b.token},
                b.ticket,None,true);
            trace!("[MiiOrdinaryCpu] destroyed bank ticket={:?} INSTALL_released={} cleanup_writes=false",
                b.ticket,released);
        }
        if s.track.as_ref().is_some_and(|t|t.ticket.entry==entry as usize) {
            s.track.take().map(|t|(t.token,t.ticket,t.gate.phase(),t.gate.published(),t.recovering))
        } else {None}
    };
    for id in [1,4] {super::mii_ordinary_player::native::retire_authority(id,entry as usize);}
    if let Some((token,ticket,phase,published,recovering))=token {
        // A recovering track ends here with the reason it was waiting on.
        let released=settle_outcome(Outcome::Destroyed {reason:recovering.unwrap_or("ordinary CPU1 exact destructor"),lease:token},
            ticket,None,published);
        trace!("[MiiOrdinaryCpu] destroyed ticket={:?} phase={:?} INSTALL_released={} cleanup_writes=false",
            ticket,phase,released);
    }
}
