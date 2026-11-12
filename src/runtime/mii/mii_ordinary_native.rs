//! Native reads/publication for ordinary-origin Player0 only.
//! No converter, record allocation, flag/reset/handle writes, or saved-file IO.
use crate::runtime::mii::mii_native_access::*;
use std::convert::TryInto;
use crate::engine::mii::{
    lifetime::{Binding, Configuration},
    native_values::{self as values, RowIds},
    ordinary_player::{Owner, SameClass, SameClassValues},
    role_owner::{self, Authority, Committed},
    inventory::{MiiClass,Snapshot as Catalog},
};

// Native publication: G -> role/helper transaction STATE -> COMMITTED.
// SourceBook updates: source STATE -> helper transaction STATE -> COMMITTED,
// with registry released before helper release; no G acquisition under source STATE.
// Reads release COMMITTED before STATE access. Registry operations call no native code.
#[path="mii_source_provenance.rs"]
pub(crate) mod helpers;
pub(crate) struct Registry {
    visible: [Option<Committed>;8],
    pub(crate) helpers: helpers::View,
}
static COMMITTED: hos_lock::Mutex<Registry> = hos_lock::Mutex::new(Registry {
    visible: [None;8], helpers: helpers::View::EMPTY,
});
pub(crate) fn registry()->hos_lock::MutexGuard<'static,Registry> { COMMITTED.lock() }

pub(crate) fn helper_snapshot()->Result<helpers::View,&'static str> {
    let layout=super::super::mii_backend::current().ok_or("Mii backend unavailable")?;
    let mut r=registry();
    if r.helpers.slots==[helpers::Helper::Unmanaged;2] && r.helpers.created_slots==[helpers::Helper::Unmanaged;4] {r.helpers.layout=layout;}
    let view=r.helpers;check(view.layout==layout,"old helper backend retained")?;view.available()?;Ok(view)
}
pub(crate) fn helper_snapshot_for(source:Option<crate::engine::swap::source_lifetime::SourceToken>)->Result<helpers::View,&'static str> {
    if source.is_none() {return helper_snapshot();}
    let view=registry().helpers;
    view.available_for(source)?;Ok(view)
}
pub(crate) fn check_helpers(view:&helpers::View)->Result<(),&'static str> {check_helpers_for(view,None)}
pub(crate) fn check_helpers_for(view:&helpers::View,source:Option<crate::engine::swap::source_lifetime::SourceToken>)->Result<(),&'static str> {
    let guard=registry();
    guard.helpers.available_for(source)?;
    check(guard.helpers==*view,"source provenance changed during native read")
}
pub(crate) fn final_registry(view:&helpers::View,token:Option<super::InstallToken>)
    ->Result<hos_lock::MutexGuard<'static,Registry>,&'static str> {final_registry_for(view,token,None)}
pub(crate) fn final_registry_for(view:&helpers::View,token:Option<super::InstallToken>,source:Option<crate::engine::swap::source_lifetime::SourceToken>)
    ->Result<hos_lock::MutexGuard<'static,Registry>,&'static str> {
    let guard=registry();
    match token {
        Some(token)=>check(super::INSTALL.owns(token),"exact INSTALL owner changed")?,
        None=>check(!super::INSTALL.active(),helpers::CAPTURE_BUSY)?,
    }
    guard.helpers.available_for(source)?;
    check(guard.helpers==*view,"source provenance changed before commit")?;
    Ok(guard)
}
impl Registry {
    pub(crate) fn commit(&mut self,authority:Authority,original_kind:u32,kind:u32) {
        self.visible[authority.binding.owner as usize]=Some(Committed{authority,original_kind,kind});
    }
}
pub(crate) fn authority(b:Binding,thread:usize,map:[usize;3],entries:[usize;8],id:u32,owner:Owner)->Authority {
    Authority {binding:Binding {owner:id,entry:owner.entry,row:owner.row,
        epoch:super::super::mii_saved_alpha::current_generation(if id==0 {0}else{1}),
        record_index:u16::from(owner.selector),..b},thread,map,entries,owner}
}
/// Called with freshly rederived current authority and G held. Original E identity
/// remains fixed; only changed Mii kinds need an exact committed registration.
pub(crate) fn foreign_kind(now:Authority,original_kind:u32,row_kind:i8,kind:u32)->bool {
    let registry=COMMITTED.lock();
    role_owner::foreign_kind(now,original_kind,row_kind,kind,
        registry.visible.get(now.binding.owner as usize).copied().flatten())
}
/// Preserve the ordinary equality rule unless an exact bank exchange owns the
/// unchanged resource header and the current descriptor/body generation. The
/// receipt path is `GroupExchange::permits` = `identity::verify(after, now,
/// ExpectedMutation::NativeBehavior)` (WP5a): a per-owner mismatch fails that
/// owner's claim only and never widens tolerance for another owner.
pub(crate) unsafe fn owner_header_matches(entry:usize,id:u32,object:usize,kind:u32)->bool {
    if let Some(matches)=super::super::body_bank::group_header(entry,id,object) {return matches;}
    rd::<u32>(object,0x58)==kind
}
pub(crate) fn retire_authority(id:usize,entry:usize) {
    let mut registry=COMMITTED.lock();
    if registry.visible[id].is_some_and(|c|c.authority.binding.entry==entry) {registry.visible[id]=None;}
}
/// lobby-preload A: a visible exact Mii as extra-capacity creation proved it: the owner's
/// authority rederived under G (equal to the one committed for it), its descriptor and its
/// selected record. `native_source_capacity` compares it again after the native creation call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VisibleMii {
    pub(crate) authority:Authority,
    pub(crate) descriptor:[u8;0x70],
    pub(crate) record:[u8;values::RECORD_LEN],
}
/// lobby-preload A: true only when `now`, rederived under G, is exactly the authority committed
/// for its owner (`Committed::permits`: binding, epoch, thread, map, entries, owner) with this
/// original row kind and committed Mii kind. A native or uncommitted Mii never qualifies.
pub(crate) fn visible_committed(now:Authority,original_kind:u32,kind:u32)->bool {
    COMMITTED.lock().visible.get(now.binding.owner as usize).copied().flatten()
        .is_some_and(|c|c.permits(now,original_kind,kind))
}
/// lobby-preload A: Player0's visible exact Mii, proven from a fresh snapshot. Reads only.
pub(super) unsafe fn visible_exact(entry:usize,epoch:u64,original_kind:u32,kind:u32)->Result<VisibleMii,&'static str> {
    let now=snapshot(entry,epoch,original_kind)?;
    let authority=now.authority();
    check(visible_committed(authority,original_kind,kind),"Player visible Mii is not its committed exact authority")?;
    Ok(VisibleMii {authority,descriptor:now.descriptor,record:now.record})
}
pub(crate) unsafe fn mirrors(o:Owner)->values::Mirrors {
    values::Mirrors::decode(rd(o.row,0x1a8),rd(o.entry,0x46),rd(o.information,0x72),
        rd(o.information,0x390),rd(o.object,0x68))
}
pub(crate) unsafe fn control(row:usize)->[u8;7] {
    [0xa,0x6b,0x79,0x78,0x19,0x1a,0xa7].map(|offset|rd(row,offset))
}
pub(crate) unsafe fn helper_kind(view:&helpers::View,id:usize,owner:Owner,kind:u32)->bool {
    let Some(ordinal)=view.ordinal(id) else {
        return false;
    };
    let now=helpers::Proof {original:helpers::OriginalRow {row:owner.row,
        kind:rd(owner.row,8),index:owner.index,selector:owner.selector,control:control(owner.row)},
        thread:skyline::nn::os::GetCurrentThread() as usize,owner,kind,costume:rd(owner.object,0x64)};
    let managed=matches!(view.slots.get(ordinal),Some(helpers::Helper::Ready{..}));
    let parked=!managed || (rd::<u32>(owner.object,0x54)==id as u32
        && super::super::native_source::parked_ready(owner.entry as *const u8,id as u32,kind));
    // TASK-OPUS-rc6-open-fixes item 1, reading A (primary 2026-11-08 23:03Z): a promoted F9 seat the game plays as a CPU
    // (slot <= No. of CPUs) is a CPU owner for this snapshot, not a parked helper row. Its source must still be exactly
    // the published one (the proof, key, token and Mii binding below); it is only not parked while it plays (Eden
    // covrest-12: the recorded proof equal to the row read now, `parked=false`).
    let owner_live=crate::runtime::roster::f9_gate::seat_live(id as u32);
    if !view.accepts(id,now, parked || owner_live) {
        note_refused_proof(view,id,ordinal,now,parked,"proof");
        return false;
    }
    // Item 1 (primary 2026-11-09 00:09Z, 00:23Z): going live rewrites the seat's descriptor (every re-proof logged
    // `descriptor_changed=true`), so a live seat keeps its Mii by reading B's identity in place of byte equality of the
    // descriptor and the record (`MiiProof::holds`). The binding checks below and the mirrors' tokens stay exact.
    let live=owner_live && !parked;
    if let Some(Some(mii))=view.miis.get(ordinal) {
        let b=mii.binding;
        if b.epoch != super::super::mii_saved_alpha::current_generation(1)
            || rd::<usize>(super::base(),POOL_HOLDER)!=b.holder
            || !span(b.holder,8,false) || rd::<usize>(b.holder,0)!=b.manager
            || !span(b.manager,0x24e0,false) || rd::<usize>(b.manager,0x24d0)!=b.vector_begin
            || rd::<usize>(b.manager,0x24d8)!=b.vector_end {
            note_refused_proof(view,id,ordinal,now,parked,"mii binding");
            return false;
        }
        let Ok(at)=values::record_address(b,b) else {
            note_refused_proof(view,id,ordinal,now,parked,"mii record address");
            return false;
        };
        if !span(at,values::RECORD_LEN,false) {
            note_refused_proof(view,id,ordinal,now,parked,"mii record");
            return false;
        }
        let (descriptor,record):([u8;0x70],[u8;values::RECORD_LEN])=(rd(owner.entry,0),rd(at,0));
        if live {note_live_bytes(id,*mii,&descriptor,&record);}
        if !mii.holds(b,&descriptor,&record,mirrors(owner),live) {
            note_refused_proof(view,id,ordinal,now,parked,"mii record");
            return false;
        }
    }
    true
}

/// Diag builds (primary 2026-11-09 00:23Z): the descriptor and record offsets of a live seat that differ from its
/// recorded Mii proof, and whether reading B's identity still holds, for the first live snapshots of a boot.
#[cfg(feature = "diag")]
static LIVE_BYTE_NOTES: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
fn note_live_bytes(id:usize,mii:helpers::MiiProof,descriptor:&[u8;0x70],record:&[u8;values::RECORD_LEN]) {
    #[cfg(not(feature = "diag"))]
    let _ = (id, mii, descriptor, record);
    #[cfg(feature = "diag")]
    {
        if mii.descriptor==*descriptor && mii.record==*record {return;}
        if LIVE_BYTE_NOTES.fetch_add(1,core::sync::atomic::Ordering::Relaxed)>=12 {return;}
        crate::engine::compat::logging::diagnostic(format_args!(
            "[MiiHelperProof] live id={} descriptor_offsets={:?} record_offsets={:?} identity={}",
            id,ByteRanges::of(&mii.descriptor,descriptor),ByteRanges::of(&mii.record,record),
            crate::model::parked_entries::mii_record_identity(record,mii.configuration.charinfo())));
    }
}
/// Up to eight differing byte ranges of two equal-length arrays, printed without allocating.
#[cfg(feature = "diag")]
struct ByteRanges {spans:[(usize,usize);8],len:usize,more:usize}
#[cfg(feature = "diag")]
impl ByteRanges {
    fn of(a:&[u8],b:&[u8])->Self {
        let mut r=ByteRanges {spans:[(0,0);8],len:0,more:0};
        let n=a.len().min(b.len());
        let mut i=0;
        while i<n {
            if a[i]==b[i] {i+=1;continue;}
            let start=i;
            while i<n && a[i]!=b[i] {i+=1;}
            if r.len<r.spans.len() {r.spans[r.len]=(start,i);r.len+=1;} else {r.more+=1;}
        }
        r
    }
}
#[cfg(feature = "diag")]
impl core::fmt::Debug for ByteRanges {
    fn fmt(&self,f:&mut core::fmt::Formatter<'_>)->core::fmt::Result {
        f.write_str("[")?;
        for (n,(start,end)) in self.spans[..self.len].iter().enumerate() {
            if n>0 {f.write_str(",")?;}
            write!(f,"{start:#x}..{end:#x}")?;
        }
        if self.more>0 {write!(f,",+{}",self.more)?;}
        f.write_str("]")
    }
}

/// Diag builds: the recorded helper proof beside the row read now, for the first refusals of a boot (primary
/// 2026-11-08 16:44Z: which fields an F9 promotion and re-park change).
#[cfg(feature = "diag")]
static REFUSED_PROOF_NOTES: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
fn note_refused_proof(view:&helpers::View,id:usize,ordinal:usize,now:helpers::Proof,parked:bool,why:&str) {
    #[cfg(not(feature = "diag"))]
    let _ = (view, id, ordinal, now, parked, why);
    #[cfg(feature = "diag")]
    {
        if REFUSED_PROOF_NOTES.fetch_add(1,core::sync::atomic::Ordering::Relaxed)>=12 {return;}
        crate::engine::compat::logging::diagnostic(format_args!(
            "[MiiHelperProof] mismatch id={} why={} parked={} recorded={:?} now={:?} mii={:?}",
            id,why,parked,view.slots.get(ordinal),now,view.miis.get(ordinal).copied().flatten().map(|m|m.binding)));
    }
}

/// Created Miis require their separate committed SourceBook receipt. Ordinary
/// extras continue through the original strict admission; no generic widening.
/// G is held; this consumes only the captured View and raw native reads.
pub(crate) unsafe fn created_kind(view:&helpers::View,extras:crate::engine::mii::capacity::Extras,
    id:usize,owner:Owner,kind:u32,facts:crate::engine::mii::capacity::ExtraFacts)
    ->Result<(),&'static str> {
    if MiiClass::from_kind(kind).is_none() {return extras.admit(id,&owner,facts);}
    {
    let ordinal=view.created_ordinal(id).ok_or("created Mii backend differs")?;
    let receipt=view.created_miis[ordinal].ok_or("created Mii receipt absent")?;
    let now=helpers::Proof {original:helpers::OriginalRow {row:owner.row,
        kind:rd(owner.row,8),index:owner.index,selector:owner.selector,control:control(owner.row)},
        thread:skyline::nn::os::GetCurrentThread() as usize,owner,kind,costume:rd(owner.object,0x64)};
    let parked=super::super::native_source::parked_created_ready(owner.entry as *const u8,
        id as u32,kind,receipt.destination);
    check(view.accepts_created(id,now,extras,facts,parked),"created Mii source/roots/readiness changed")?;
    let b=receipt.mii.binding;
    check(rd::<usize>(super::base(),POOL_HOLDER)==b.holder
        && span(b.holder,8,false) && rd::<usize>(b.holder,0)==b.manager
        && span(b.manager,0x24e0,false) && rd::<usize>(b.manager,0x24d0)==b.vector_begin
        && rd::<usize>(b.manager,0x24d8)==b.vector_end,"created Mii G identity changed")?;
    let at=values::record_address(b,b).map_err(|_|"created Mii record bounds")?;
    check(span(at,values::RECORD_LEN,false)
        && receipt.mii.matches(b,&rd(owner.entry,0),&rd(at,0),mirrors(owner)),"created Mii exact payload changed")
    }
}

#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    address: u64, size: u64, kind: u32, attributes: u32, permission: u32,
    ipc: u32, device: u32, padding: u32,
}
/// Horizon QueryMemory, the same pinned SVC ABI as the local Deluxe SDK reader.
/// One finite span per object; does not pin lifetime. Current callback/map/thread supplies lifetime.
pub(crate) unsafe fn span(address: usize, size: usize, write: bool) -> bool {
    let Some(end) = address.checked_add(size) else { return false };
    if address < 0x10000 || size == 0 { return false; }
    let mut info = MemoryInfo::default();
    let result: u64;
    core::arch::asm!("svc 0x6",
        inout("x0") &mut info as *mut MemoryInfo as u64 => result,
        lateout("x1") _, in("x2") address as u64,
        clobber_abi("C"), options(nostack));
    let required = if write { 3 } else { 1 };
    result as u32 == 0 && info.permission & required == required
        && info.address <= address as u64
        && info.address.checked_add(info.size).is_some_and(|e| end as u64 <= e)
}
pub(crate) fn check(ok: bool, reason: &'static str) -> Result<(), &'static str> {
    if ok { Ok(()) } else { Err(reason) }
}
/// Classify a refusal that came from a native read or a registry fence while a planned Mii group's CharInfo is
/// published (`crate::engine::swap::body_bank::mii_publish`): a busy sentinel clears by itself and costs no attempt,
/// a helper with no ordinary authority cannot clear while the owner idles, and every other read/write failure is a
/// race the next frame reads again (counted).
pub(crate) fn publish_refusal(reason: &'static str) -> crate::engine::swap::body_bank::mii_publish::PublishRefusal {
    use crate::engine::swap::body_bank::mii_publish::PublishRefusal;
    if reason == helpers::BUSY || reason == helpers::CAPTURE_BUSY { PublishRefusal::Busy }
    else if reason == helpers::NO_AUTHORITY { PublishRefusal::Permanent(reason) }
    else { PublishRefusal::Race(reason) }
}
/// A refusal that cannot clear while the owner idles (`PublishRefusal::Permanent`) unless `ok`.
pub(crate) fn permanent(ok: bool, reason: &'static str)
    -> Result<(), crate::engine::swap::body_bank::mii_publish::PublishRefusal> {
    if ok { Ok(()) } else { Err(crate::engine::swap::body_bank::mii_publish::PublishRefusal::Permanent(reason)) }
}
pub(crate) unsafe fn cpu_headgear_fingerprints()->bool {super::hook::headgear_valid(super::base())}
pub(crate) unsafe fn pointer(at: usize, offset: usize, size: usize, write: bool) -> Result<usize, &'static str> {
    let slot=at.checked_add(offset).ok_or("pointer overflow")?;
    check(span(slot,8,false), "pointer slot unmapped")?;
    let p: usize = rd(slot,0);
    check(aligned(p) && span(p, size, write), "target span unavailable")?;
    Ok(p)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Scalars {
    pub e: Tokens,
    pub d_outfit: u16,
    pub d_colour: u16,
    pub d_specials: [u32; 4],
}
impl Scalars {
    pub(crate) unsafe fn read(e: usize, d: usize) -> Self {
        Self { e: Tokens::from_row(e), d_outfit: rd(d,0x74),
            d_colour: rd(d,0x78), d_specials: rd(d,0x390) }
    }
    pub(crate) fn saved(c: Configuration) -> Self {
        let t=c.tokens();
        Self { e:t.into(), d_outfit:t.outfit,d_colour:t.colour,d_specials:t.specials.map(u32::from) }
    }
    /// E+1ac is a native hat source. D+76 and O+70 remain native-owned;
    /// CPU headgear changes require the shared publisher's explicit fresh-O proof.
    pub(crate) unsafe fn write(self,e:usize,d:usize) {
        wr(e,0x1aa,self.e.outfit);wr(e,0x1ac,self.e.hat);wr(e,0x1ae,self.e.colour);wr(e,0x1b0,self.e.voice);
        for (i,v) in self.e.specials.iter().enumerate() { wr(e,0x1b2+i,*v); }
        wr(d,0x74,self.d_outfit);wr(d,0x78,self.d_colour);
        for (i,v) in self.d_specials.iter().enumerate() { wr(d,0x390+i*4,*v); }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Protected {
    owner: Owner,
    descriptor: [u8; 0x70],
    e: [u8;14], d: [u8;8], specials:[u32;4], o:[u8;20], control:[u8;7],
    record:[u8;values::RECORD_LEN],
    current_record:Option<[u8;values::RECORD_LEN]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub layout:super::super::mii_backend::Layout,
    pub helpers: helpers::View,
    pub binding: Binding,
    pub thread: usize,
    pub original_kind: u32,
    pub descriptor: [u8;0x70],
    pub scalars: Scalars,
    pub record:[u8;values::RECORD_LEN],
    control:[u8;7],
    symbol:usize, manager:usize, internal:usize,
    entries:[usize;8], owners:[Option<Owner>;8], protected:[Option<Protected>;8],
}
impl Snapshot {
    fn authority(&self)->Authority {
        authority(self.binding,self.thread,[self.symbol,self.manager,self.internal],
            self.entries,0,self.owners[0].unwrap())
    }
    /// rc.6 item 3 (a Mii group built in the background, `body_bank::start_background`): the bank's Mii setup changed
    /// selector record `index` from `before` to `after` (its pair) while this owner was idle. Each copy of that record
    /// here (the destination, a protected owner's record or current record) that still holds `before` takes `after`.
    /// Returns how many copies changed.
    pub(super) fn adopt_record(&mut self,index:u16,before:&[u8;values::RECORD_LEN],after:&[u8;values::RECORD_LEN])->usize {
        let mut changed=0;
        if self.binding.record_index==index && self.record==*before {self.record=*after;changed+=1;}
        for p in self.protected.iter_mut().flatten() {
            if u16::from(p.owner.selector)==index && p.record==*before {p.record=*after;changed+=1;}
            if p.owner.index==index && p.current_record==Some(*before) {p.current_record=Some(*after);changed+=1;}
        }
        changed
    }
}
pub(super) fn same_owned(a:&Snapshot,b:&Snapshot)->bool {
    a.binding==b.binding && a.thread!=0 && a.thread==b.thread
        && a.symbol==b.symbol && a.manager==b.manager && a.internal==b.internal
        && a.entries==b.entries && a.owners[0]==b.owners[0] && a.control==b.control
        && a.descriptor==b.descriptor && a.scalars==b.scalars && a.record==b.record
}
/// `entries` is the preserved owner view: registered created entries are validated
/// against their exact identities and left out, so a capture that predates their
/// creation still compares equal. `physical()` is the exact native map to recheck.
#[derive(Clone, Copy)]
struct Map { layout:super::super::mii_backend::Layout,symbol:usize, manager:usize, internal:usize, entries:[usize;8],
    extras:crate::engine::mii::capacity::Extras }
impl Map {
    fn physical(&self)->[usize;8] {self.extras.physical(&self.entries)}
}
unsafe fn map(entry: usize) -> Result<Map,&'static str> {
    let layout=super::super::mii_backend::current().ok_or("Mii backend unavailable")?;
    let symbol=crate::runtime::compat::sync::read(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    let manager=pointer(symbol,0,8,false)?;
    let internal=pointer(manager,0,0xa4,false)?;
    let physical=std::array::from_fn(|id| {
        smash::app::lua_bind::FighterManager::get_fighter_entry(
            manager as *mut smash::app::FighterManager,
            smash::app::FighterEntryID(id as i32)) as usize
    });
    check(physical[0]==entry && rd::<[usize;8]>(internal,0x20)==physical
        && rd::<[usize;8]>(internal,0x60)==[0;8],"current primary owner map")?;
    check(rd::<u32>(internal,0xa0) as usize==physical.iter().filter(|p| **p!=0).count(),
        "current map count")?;
    let extras=super::super::mii_backend::extras();
    check(layout.shape_with(&physical,extras),"Mii backend owner shape differs")?;
    if layout.waiting() {
        check(super::super::native_source::exact_waiting_layout_with(physical[5] as *const u8,5,layout,extras),"waiting row layout changed")?;
    }
    let entries=extras.view(&physical).ok_or("registered created entry identity differs")?;
    Ok(Map{layout,symbol,manager,internal,entries,extras})
}
unsafe fn pool(base:usize, entry:usize, epoch:u64) -> Result<Binding,&'static str> {
    let holder=pointer(base,POOL_HOLDER,8,false)?;
    let manager=pointer(holder,0,0x24e0,true)?;
    let row=base.checked_add(0x52c5730).ok_or("E overflow")?;
    check(span(row,0x260,true)&&span(entry,0x5a88,true),"current FE/E spans")?;
    let b=Binding {epoch,owner:0,entry,row,holder,manager,
        vector_begin:rd(manager,0x24d0),vector_end:rd(manager,0x24d8),
        record_index:rd(row,0x1a8)};
    let record=values::record_address(b,b).map_err(|_|"native-selected record bounds")?;
    check(span(record,values::RECORD_LEN,true),"selected record span")?;
    Ok(b)
}
unsafe fn current_locked(base:usize, m:Map, locked:Binding,
    original_kind:u32, staged:Option<(&[u8;0x70],usize)>, helpers:helpers::View)
    -> Result<Snapshot,&'static str> {
    let entry=locked.entry;let epoch=locked.epoch;
    check_helpers(&helpers)?;
    check(rd::<usize>(m.symbol,0)==m.manager && rd::<usize>(m.manager,0)==m.internal
        && rd::<[usize;8]>(m.internal,0x20)==m.physical()
        && rd::<[usize;8]>(m.internal,0x60)==[0;8],"owner map changed")?;
    let b=pool(base,entry,epoch)?;
    check(helpers.layout==m.layout,"helper backend changed")?;
    check(b==locked,"G/vector/index changed under lock")?;
    let table=pointer(base,OWNER_TABLE,0x130,false)?;
    let mut owners=[None;8];
    let mut kinds=[None;8];
    let mut protected=[None;8];
    // Every physical entry, including registered created entries, gets the same
    // FE/E/D/O, index and mirror checks and a protected-record capture below.
    for (id,fe) in m.physical().iter().copied().enumerate() {
        if fe==0 { continue; }
        let e=base+0x52c5730+id*0x260;
        check(aligned(fe)&&span(fe,0x5a88,false)&&span(e,0x260,false),"other owner FE/E span")?;
        let d=pointer(fe,0xf8,0x3a0,id==0)?;
        let o=pointer(table,0xe8+id*8,0xb0,false)?;
        let index: u16=rd(e,0x1a8);
        // Only an exact registered created entry replaces the "word1 == row selector"
        // mirror with Extras::admit's "word1 == its creation descriptor" below.
        let extra=m.extras.entry(id)==Some(fe);
        check(rd::<u32>(fe,0xe0)==id as u32 && rd::<u32>(e,0x14)==id as u32
            && (extra || rd::<u32>(fe,4)==u32::from(rd::<u8>(e,0xa7)))
            && rd::<u16>(fe,0x46)==index && rd::<u16>(d,0x72)==index,
            "owner/index mirror changed")?;
        if id==0 {check(role_owner::original_row_matches(original_kind,rd(e,8)),
            "original row identity changed")?;}
        if !(id==0 && staged.is_some()) {
            check(rd::<u16>(o,0x68)==index,"owner object index mismatch")?;
        }
        let owner=Owner {entry:fe,row:e,information:d,object:o,index,
            selector:rd(e,0xa7),alias:rd(table,0x128+id)};
        owners[id]=Some(owner);
        let kind=rd::<u32>(fe,0x18);
        kinds[id]=Some(kind);
        if !(id==0 && staged.is_some()) {
            check(owner_header_matches(fe,id as u32,o,kind),"current owner header kind differs")?;
        }
        if id!=0 {
            if extra {
                // Shared exact admission (roots, owners, object, creation descriptor
                // word, indices, kinds, ordinary-only claim facts); raw reads under G.
                // Never a helper ordinal or Mii owner; names the failed predicate.
                created_kind(&helpers,m.extras,id,owner,kind,super::super::mii_backend::extra_facts(table,id,fe,e,d,o))?;
            } else if m.layout.logical(id as u32).is_some() {
                let current=authority(b,skyline::nn::os::GetCurrentThread() as usize,
                    [m.symbol,m.manager,m.internal],m.entries,id as u32,owner);
                let original=super::super::original_kind(super::super::Role::Cpu)
                    .ok_or("CPU original absent")?;
                check(foreign_kind(current,original,rd(e,8),kind),"foreign original row/kind authority changed")?;
            } else {check(helper_kind(&helpers,id,owner,kind),"helper row kind/provenance changed")?;}
            let other=Binding {owner:id as u32,entry:fe,row:e,record_index:u16::from(owner.selector),..b};
            let at=values::record_address(other,other).map_err(|_|"protected record bounds")?;
            check(span(at,values::RECORD_LEN,false),"protected record span")?;
            let cur=Binding {record_index:index,..other};
            let current_at=values::record_address(cur,cur).map_err(|_|"protected current record bounds")?;
            check(span(current_at,values::RECORD_LEN,false),"protected current record span")?;
            protected[id]=Some(Protected{owner,descriptor:rd(fe,0),e:rd(e,0x1a8),
                d:rd(d,0x72),specials:rd(d,0x390),o:rd(o,0x68),control:control(e),record:rd(at,0),
                // Ordinary index0 is a placeholder, not a foreign claim on Player0's G0.
                current_record:role_owner::current_record_claim(kind,index).then(||rd(current_at,0))});
        }
    }
    check(if m.layout.waiting() {role_owner::claims_in_with(m.layout,m.extras,&owners,&kinds).is_some()}
        else {crate::engine::mii::ordinary_player::unique_player(&owners,&kinds)},
        "owner/selector/index alias or inactive owner conflict")?;
    let owner=owners[0].ok_or("Player0 absent")?;
    if let Some((_,o))=staged { check(owner.object==o,"selected native O differs")?; }
    let descriptor:[u8;0x70]=rd(entry,0);
    if let Some((expected,_))=staged { check(descriptor==*expected,"incoming descriptor differs")?; }
    let at=values::record_address(b,b).map_err(|_|"selected bounds changed")?;
    check(rd::<usize>(m.symbol,0)==m.manager && rd::<usize>(m.manager,0)==m.internal
        && rd::<[usize;8]>(m.internal,0x20)==m.physical()
        && rd::<[usize;8]>(m.internal,0x60)==[0;8]
        && rd::<usize>(base,OWNER_TABLE)==table
        && rd::<usize>(table,0xe8)==owner.object
        && rd::<usize>(entry,0xf8)==owner.information,
        "owner authority changed during bounded read")?;
    check_helpers(&helpers)?;
    Ok(Snapshot {layout:m.layout,helpers,binding:b,thread:skyline::nn::os::GetCurrentThread() as usize,
        original_kind,descriptor,scalars:Scalars::read(b.row,owner.information),record:rd(at,0),control:control(b.row),
        symbol:m.symbol,manager:m.manager,internal:m.internal,entries:m.entries,owners,protected})
}
pub(super) unsafe fn snapshot(entry:usize,epoch:u64,original_kind:u32) -> Result<Snapshot,&'static str> {
    let helpers=helper_snapshot()?;
    let base=super::base();
    let m=map(entry)?;
    let b=pool(base,entry,epoch)?;
    let g=PoolLock::acquire(base,b.manager);
    let result=current_locked(base,m,b,original_kind,None,helpers);
    drop(g);
    result
}
pub(crate) unsafe fn target_rows(c:Configuration) -> Result<RowIds,&'static str> {
    let base=super::base();let t=c.tokens();
    let ids=RowIds {outfit:lookup_row_id(base,OUTFIT_TABLE,t.outfit)?,
        hat:lookup_row_id(base,HAT_TABLE,t.hat)?};
    ids.validate(c).map_err(|_|"saved appearance hint/hash mismatch")?;
    Ok(ids)
}

/// Call only inside this adapter's freshly rederived all-owner/G critical section.
/// The returned values are comparison evidence, never a saved pointer to replay later.
unsafe fn same_class_values(now:&Snapshot)->Result<SameClassValues,&'static str> {
    let o=now.owners[0].ok_or("same-class Player0 absent")?;
    check(span(o.object,0xb0,true),"same-class writable O span")?;
    let task:usize=rd(o.object,0xa8);
    let task_idle=task==0 || (aligned(task)&&span(task,0x24,false)
        && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)==0);
    let values=SameClassValues {
        binding:now.binding,thread:now.thread,owner:o,
        header:rd(o.object,0x54),
        mirrors:values::Mirrors::decode(rd(o.row,0x1a8),rd(o.entry,0x46),
            rd(o.information,0x72),rd(o.information,0x390),rd(o.object,0x68)),
        initialized:rd::<u8>(o.object,0x4c)&1!=0,task_idle,
        rebuild_open:rd::<u8>(o.information,0x94)==0,
    };
    let table=pointer(super::base(),OWNER_TABLE,0x130,false)?;
    check(rd::<usize>(table,0xe8)==o.object && rd::<usize>(o.entry,0xf8)==o.information
        && rd::<usize>(o.object,0xa8)==task,"same-class read authority changed")?;
    Ok(values)
}

/// Preflight: every mirror still describes the BEFORE configuration. No writes.
pub(super) unsafe fn prepare_same_class(expected:Snapshot,before:Configuration,after:Configuration)
    ->Result<SameClass,&'static str> {
    let rows=[target_rows(before)?,target_rows(after)?];
    let base=super::base();let b=expected.binding;
    let m=map(b.entry)?;let current=pool(base,b.entry,b.epoch)?;
    check(current==b,"same-class preflight binding changed")?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,expected.original_kind,None,helpers)?;
    check(now==expected,"same-class preflight snapshot changed")?;
    let at=values::record_address(b,b).map_err(|_|"same-class record bounds")?;
    check(rd::<usize>(at,0x58)!=0 && rd::<usize>(at,0x60)!=0,
        "same-class active Mii resource pair absent")?;
    let proof=SameClass::prepare(before,after,same_class_values(&now)?,
        &now.descriptor,rows,&now.record)?;
    drop(g);
    Ok(proof)
}

/// Explicit native branch selection, never a fallback after a failed proof. The unchanged
/// publisher rechecks bit0 clear under current authority; the retained arm requires bit0 set.
pub(super) unsafe fn publish_same_class(track:&super::Track,selected:usize)
    ->Result<bool,&'static str> {
    let proof=track.same_class.ok_or("same-class proof absent")?;
    check(track.publish==Some(proof.target()),"same-class target changed")?;
    // Row getters are native calls and run before acquiring G or adapter STATE, for either arm.
    let rows=[target_rows(proof.before())?,target_rows(proof.target())?];
    check(aligned(selected)&&span(selected,0xb0,false),"same-class selected O span")?;
    if rd::<u8>(selected,0x4c)&1==0 {
        // Native itself will publish its header and FE tokens to this selected O.
        // A flag/authority change during rederivation refuses inside publish; do not switch arms.
        publish(track,selected)?;
        return Ok(false);
    }
    let base=super::base();let entry=track.ticket.entry;
    let m=map(entry)?;let b=pool(base,entry,track.ticket.epoch)?;
    check(b==track.before.binding,"same-class phase binding differs")?;
    let helpers=helper_snapshot()?;
    check(helpers==track.before.helpers,"same-class source provenance changed")?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,track.before.original_kind,
        Some((&track.descriptor,selected)),helpers)?;
    check(now.thread!=0 && now.thread==track.before.thread
        && now.symbol==track.before.symbol && now.manager==track.before.manager
        && now.internal==track.before.internal && now.entries==track.before.entries
        && now.protected==track.before.protected,"same-class protected authority changed")?;
    check(now.record==track.before.record && now.scalars==track.before.scalars,
        "same-class owned record/scalars changed")?;
    check(rd::<u8>(entry,0x5920)==3 && rd::<u8>(entry,0x591c)==6,
        "same-class not active native loader6")?;
    let observed=same_class_values(&now)?;
    let update=proof.prepare_write(observed,&track.descriptor,rows,&now.record)?;
    let record=values::record_address(b,b).map_err(|_|"same-class record changed")?;
    let targets=Targets {e:b.row,fe:entry,d:observed.owner.information,o:observed.owner.object};
    let mut state=super::STATE.lock();
    let fresh=state.track.as_mut().ok_or("same-class ticket retired")?;
    check(fresh.ticket==track.ticket && fresh.same_class==Some(proof)
        && fresh.gate.may_publish(track.ticket,6) && super::epoch()==track.ticket.epoch,
        "same-class phase ticket/epoch changed")?;
    let registry=final_registry(&helpers,Some(track.token))?;
    // No fallible operation, row getter or native callback after the first write.
    copy_update(record,&update);
    write_tokens(&targets,update.tokens().into());
    fresh.retained_o=true;
    let published=fresh.gate.publish(track.ticket,6);
    debug_assert!(published);
    drop(registry);drop(state);drop(g);
    Ok(true)
}

/// All checks precede the write. Lock order: G -> final adapter state lock, solely for the
/// bounded raw publication. No path holds adapter state while acquiring G or calling native.
pub(super) unsafe fn publish(track:&super::Track, selected:usize) -> Result<(),&'static str> {
    if let Some(c)=track.publish {target_rows(c)?;}
    let base=super::base();let entry=track.ticket.entry;
    let m=map(entry)?;
    let b=pool(base,entry,track.ticket.epoch)?;
    check(b==track.before.binding,"phase binding differs")?;
    let helpers=helper_snapshot()?;
    check(helpers==track.before.helpers,"phase source provenance changed")?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,track.before.original_kind,
        Some((&track.descriptor,selected)),helpers)?;
    check(now.thread!=0 && now.thread==track.before.thread,"phase native thread differs")?;
    check(now.symbol==track.before.symbol && now.manager==track.before.manager
        && now.internal==track.before.internal && now.entries==track.before.entries
        && now.protected==track.before.protected,"protected owner state changed")?;
    check(now.record==track.before.record && now.scalars==track.before.scalars,
        "owned record/scalars changed before publication")?;
    let o=now.owners[0].ok_or("Player0 absent")?;
    check(rd::<u8>(entry,0x5920)==3 && rd::<u8>(entry,0x591c)==6,
        "not active native loader6")?;
    check(rd::<u8>(selected,0x4c)&1==0,"selected O already initialized")?;
    let task:usize=rd(selected,0xa8);
    // O+a8 owns a native 0x80-byte job: +0x10 is the outstanding counter.
    // Queue3549720 increments it; worker354a7e0 decrements after the callback.
    // Match that release with Acquire; +0x20 is not this job's status field.
    check(task==0 || (aligned(task)&&span(task,0x24,false)
        && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)==0),
        "old O task not quiescent")?;
    if track.publish.is_some() {
        check(rd::<u8>(o.information,0x94)==0,"native Mii rebuild gate is closed")?;
    }
    // State/epoch is checked again immediately at the publication linearization point.
    let mut state=super::STATE.lock();
    let fresh=state.track.as_mut().ok_or("ticket retired")?;
    check(fresh.ticket==track.ticket && fresh.gate.may_publish(track.ticket,6)
        && super::epoch()==track.ticket.epoch,"phase ticket/epoch changed")?;
    let record=values::record_address(b,b).map_err(|_|"record changed")?;
    let scalars=track.publish.map_or(track.origin_scalars,Scalars::saved);
    if scalars.e.hat!=now.scalars.e.hat {
        crate::engine::mii::ordinary_cpu::HatPublication {
            layout:now.layout,owner:b.owner,source:false,opt_in:true,
            initialized:rd::<u8>(selected,0x4c)&1!=0,
            fingerprints:cpu_headgear_fingerprints(),
            before:now.scalars.e.hat,target:scalars.e.hat,
            // Player binding stays on its captured row across publication.
            // current_locked proves its owner/index mirrors and b == before.binding.
            destination_index:b.record_index,original_index:track.before.binding.record_index,
            descriptor:&track.descriptor,payload:track.publish,
        }.validate()?;
    }
    let registry=final_registry(&helpers,Some(track.token))?;
    // No fallible operation or native callback after this point.
    if track.publish.is_some() {
        let update=crate::engine::mii::ordinary_player::retained_record(now.record,track.publish);
        for (off,byte) in update[..CHARINFO_LEN].iter().enumerate() {wr(record,off,*byte);}
    }
    scalars.write(b.row,o.information);
    let published=fresh.gate.publish(track.ticket,6);
    debug_assert!(published);
    drop(registry);drop(state);
    drop(g);
    Ok(())
}
/// What a Mii one-member group switch published before its native request, so a
/// switch refused afterwards can put the owner's exact record and scalars back.
#[derive(Clone,Copy)]
pub(super) struct BankPublished { record:[u8;values::RECORD_LEN], scalars:Scalars }

/// MII-GROUPS-DESIGN §5.2: the canonical exact publication (`publish`) for a Mii
/// one-member group switch, before the bank's native request. Every check runs
/// against the owner's unchanged physical receipt, with the two phase relaxations
/// the bank path needs checked rather than skipped: the native loader is idle at
/// 6/14 (no Reload runs; the bank's `+0x591a` commit follows) and the selected O
/// is the live, initialized resource owner whose Mii keys the group prefetched
/// for `group_hat` (`HatPublication::validate_bank`). Writes the CharInfo onto the
/// owner's selector record and the E/D scalars; the identity words follow at the
/// build. Lock order G -> adapter STATE -> COMMITTED.
pub(super) unsafe fn bank_publish(bank:&super::BankTrack,c:Configuration,group_hat:u16)->Result<(),&'static str> {
    target_rows(c)?;
    let before=&bank.before;
    let base=super::base();let entry=before.binding.entry;
    let m=map(entry)?;
    let b=pool(base,entry,before.binding.epoch)?;
    check(b==before.binding,"bank publication binding differs")?;
    let helpers=helper_snapshot()?;
    check(helpers==before.helpers,"bank publication source provenance changed")?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,before.original_kind,None,helpers)?;
    check(same_owned(&now,before) && now.protected==before.protected && now.layout==before.layout,
        "bank publication owned state changed")?;
    let o=now.owners[0].ok_or("Player0 absent")?;
    check(rd::<u8>(entry,0x5920)==6 && rd::<u8>(entry,0x591c)==14 && rd::<u8>(entry,0x591a)==0xff
        && rd::<u8>(entry,0x5934)==0 && rd::<u8>(entry,0x5935)==0,
        "bank publication needs the native loader idle at 6/14")?;
    let task:usize=rd(o.object,0xa8);
    check(task==0 || (aligned(task)&&span(task,0x24,false)
        && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)==0),
        "bank publication O task not quiescent")?;
    check(rd::<u8>(o.information,0x94)==0,"native Mii rebuild gate is closed")?;
    let identity=crate::engine::swap::body_bank::MiiIdentity::from_descriptor(&bank.descriptor)
        .ok_or("bank publication descriptor is not a Mii identity")?;
    check(identity.kind()==c.canonical().class as u32 && identity.index==b.record_index
        && identity.hat()==c.tokens().hat && identity.outfit()==c.tokens().outfit,
        "bank publication identity differs from the payload")?;
    let scalars=Scalars::saved(c);
    crate::engine::mii::ordinary_cpu::HatPublication {
        layout:now.layout,owner:b.owner,source:false,opt_in:scalars.e.hat!=now.scalars.e.hat || bank.hat_change,
        initialized:rd::<u8>(o.object,0x4c)&1!=0,
        fingerprints:cpu_headgear_fingerprints(),
        before:now.scalars.e.hat,target:scalars.e.hat,
        destination_index:b.record_index,original_index:before.binding.record_index,
        descriptor:&bank.descriptor,payload:Some(c),
    }.validate_bank(group_hat)?;
    let mut state=super::STATE.lock();
    check(state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token && !now.departed)
        && super::epoch()==bank.ticket.epoch,"bank publication ticket/epoch changed")?;
    let record=values::record_address(b,b).map_err(|_|"record changed")?;
    let track=state.bank.as_mut().ok_or("bank publication track absent")?;
    let registry=final_registry(&helpers,Some(bank.token))?;
    // No fallible operation or native callback after this point. The track records what
    // it is about to replace in the same critical section (MII-PHASEB): from here on every
    // terminal arm compensates it (`bank_refused` -> `bank_restore`).
    track.published=Some((c,BankPublished {record:now.record,scalars:now.scalars}));
    let update=crate::engine::mii::ordinary_player::retained_record(now.record,Some(c));
    for (off,byte) in update[..CHARINFO_LEN].iter().enumerate() {wr(record,off,*byte);}
    scalars.write(b.row,o.information);
    drop(registry);drop(state);drop(g);
    Ok(())
}

/// MII-GROUPS prefetch publication: a planned Mii group's CharInfo goes onto the
/// owner's selector record before the group's key request copies that record
/// (`0x17e64d0` at `0x17e6c10` builds the face from it). No INSTALL is active
/// (none may be: `final_registry(.., None)`), G is held, the owner's unchanged
/// receipt is re-proven and then adopts the bytes; nothing else is written. The
/// switch's `bank_publish` rewrites the same bytes with the scalars later.
pub(super) unsafe fn prefetch_publish(entry:usize,c:Configuration,identity:crate::engine::swap::body_bank::MiiIdentity)
    ->Result<(),crate::engine::swap::body_bank::mii_publish::PublishRefusal> {
    use crate::engine::swap::body_bank::mii_publish::PublishRefusal;
    target_rows(c).map_err(publish_refusal)?;
    let (expected,original_kind)={
        let s=super::STATE.lock();
        if s.busy() || s.request.is_some() || s.capture || !s.menu_valid {return Err(PublishRefusal::Busy);}
        let o=s.owner.as_ref().ok_or(PublishRefusal::Permanent("Mii group publication without an owner"))?;
        if o.origin.binding.entry!=entry || o.origin.binding.epoch!=super::epoch() {
            return Err(PublishRefusal::Permanent("Mii group publication owner/entry/epoch differ"));
        }
        (o.expected,o.origin.original_kind)
    };
    permanent(identity.kind()==c.canonical().class as u32 && identity.index==expected.binding.record_index
        && identity.hat()==c.tokens().hat && identity.outfit()==c.tokens().outfit,
        "Mii group publication identity differs from the configuration")?;
    let base=super::base();
    let m=map(entry).map_err(publish_refusal)?;
    let b=pool(base,entry,expected.binding.epoch).map_err(publish_refusal)?;
    permanent(b==expected.binding,"Mii group publication binding differs")?;
    // INVARIANT (C116.2 mii-fix; `R/c1162/MII-PROVENANCE-REPORT.md`). The owner's stored `expected.helpers` is the
    // helper view of its LAST completion and is not compared with the live one: helper sources are replaced between
    // completions (the cycle preloader, Kirby hats), that gap is unfenced and legitimate, and the comparison refused
    // every frame once a helper was reloaded. What authorizes this write is, within this one transaction:
    //  1. the owner still equals its last proven snapshot (`same_owned` below, plus the exact `o.expected==expected`
    //     under STATE) and the pool/entry/epoch are the group's;
    //  2. every non-destination helper row of the locked walk (`current_locked` -> `helper_kind`) is re-authorized
    //     against ONE view V, the fresh `helper_snapshot()` taken here (BUSY while any source loads);
    //  3. V is unchanged, with no source loading, from the start of the walk to the write (`check_helpers_for` at
    //     both ends of `current_locked`, then `final_registry(V, ..)` under the registry lock); and
    //  4. no INSTALL is held by anyone else (`final_registry(.., None)`).
    // The write (the 88-byte CharInfo at the owner's own destination record) neither reads nor writes a helper row.
    let helpers=helper_snapshot().map_err(publish_refusal)?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,original_kind,None,helpers).map_err(publish_refusal)?;
    permanent(same_owned(&now,&expected),"Mii group publication owned state changed")?;
    check(rd::<u8>(entry,0x5920)==6 && rd::<u8>(entry,0x591c)==14 && rd::<u8>(entry,0x591a)==0xff
        && rd::<u8>(entry,0x5934)==0 && rd::<u8>(entry,0x5935)==0,
        "Mii group publication needs the native loader idle at 6/14").map_err(PublishRefusal::Race)?;
    let mut state=super::STATE.lock();
    if state.busy() || state.request.is_some() {return Err(PublishRefusal::Busy);}
    let registry=final_registry(&helpers,None).map_err(publish_refusal)?;
    let owner=state.owner.as_mut().filter(|o|o.expected==expected)
        .ok_or(PublishRefusal::Race("Mii group publication owner changed"))?;
    let record=values::record_address(b,b).map_err(|_|PublishRefusal::Race("record changed"))?;
    for (off,byte) in c.charinfo().iter().enumerate() {wr(record,off,*byte);}
    owner.expected.record[..CHARINFO_LEN].copy_from_slice(c.charinfo());
    drop(registry);drop(state);drop(g);
    Ok(())
}

/// A verified restore: the receipt and the owner's state read back after the writes.
pub(super) struct Restored {
    pub receipt:crate::engine::mii::ordinary_player::RestoreReceipt,
    pub now:Snapshot,
}

/// MII-PHASEB: the one compensation of `bank_publish`, run by `bank_refused` on every
/// terminal arm while INSTALL is still held (entry destruction drops the owner instead).
/// The window must not be the published group (`publication_rollback`; a live exchange
/// keeps the publication), the owner's record and scalars must still read exactly as
/// published and the ticket, epoch and scene must be the publication's; then the
/// pre-publish CharInfo and scalars go back and are read back under the same G hold. The
/// receipt is verified when the owner's kept receipt (`before`) describes native again,
/// with at most native's own pair at `record+0x58` (a crossed Phase B Mii setup) to adopt.
/// `helpers::BUSY` comes back only before any write: the caller keeps the track and retries.
pub(super) unsafe fn bank_restore(bank:&super::BankTrack,c:Configuration,published:BankPublished)->Result<Restored,&'static str> {
    use crate::engine::mii::ordinary_player::{publication_rollback,restore_receipt,PublicationRollback};
    let before=&bank.before;
    let base=super::base();let entry=before.binding.entry;
    let m=map(entry)?;
    let b=pool(base,entry,before.binding.epoch)?;
    check(b==before.binding,"bank rollback binding differs")?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,before.original_kind,None,helpers)?;
    let o=now.owners[0].ok_or("Player0 absent")?;
    // Live only when the window *is* the published group (its identity), not on any other
    // descriptor difference; those fall to the published-state check and the read-back.
    let identity=crate::engine::swap::body_bank::MiiIdentity::from_descriptor;
    let live=identity(&now.descriptor).is_some_and(|id|Some(id)==identity(&bank.descriptor));
    if publication_rollback(true,live)==PublicationRollback::KeepLive {
        return Err("the exchange made the published group the window; the publication is live");
    }
    check(now.scalars==Scalars::saved(c) && now.record[..CHARINFO_LEN]==c.charinfo()[..],
        "bank rollback state is not the published state")?;
    let record=values::record_address(b,b).map_err(|_|"record changed")?;
    // The ticket/epoch/scene proof `bank_publish` made before it wrote (G -> STATE -> COMMITTED).
    let state=super::STATE.lock();
    check(state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token && !now.departed)
        && super::epoch()==bank.ticket.epoch,"bank rollback ticket/epoch changed or the scene departed")?;
    let registry=final_registry(&helpers,Some(bank.token))?;
    // No fallible operation between here and the writes; nothing after them returns the retry sentinel.
    for (off,byte) in published.record[..CHARINFO_LEN].iter().enumerate() {wr(record,off,*byte);}
    published.scalars.write(b.row,o.information);
    drop(registry);drop(state);
    let after=current_locked(base,m,b,before.original_kind,None,helpers)
        .map_err(|_|"bank publication restore read-back failed")?;
    drop(g);
    let rest=same_owned(&Snapshot {record:before.record,..after},before);
    let receipt=restore_receipt(&before.record,&after.record,rest);
    if !receipt.verified() {return Err("bank publication restore did not read back as the owner's receipt");}
    Ok(Restored {receipt,now:after})
}

/// Current ready callback after native construction. Native replaces resource/control words;
/// compare G payload and all owned mirrors, never fabricate or clear its shared_ptr.
pub(super) unsafe fn completed(track:&super::Track) -> Result<Snapshot,&'static str> {
    let base=super::base();
    let m=map(track.ticket.entry)?;
    let b=pool(base,track.ticket.entry,track.ticket.epoch)?;
    let helpers=helper_snapshot()?;
    check(helpers==track.before.helpers,"ready source provenance changed")?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,track.before.original_kind,None,helpers)?;
    check(now.binding==track.before.binding && now.thread==track.before.thread
        && now.symbol==track.before.symbol && now.manager==track.before.manager
        && now.internal==track.before.internal && now.entries==track.before.entries
        && now.control==track.before.control
        && now.protected==track.before.protected,"ready authority/protected state changed")?;
    check(now.descriptor==track.descriptor,"ready descriptor changed")?;
    let wanted=track.publish.map_or(track.origin_scalars,Scalars::saved);
    check(now.scalars==wanted,"ready E/D configuration mismatch")?;
    let owner=now.owners[0].ok_or("ready Player0 absent")?;
    let at=values::record_address(b,b).map_err(|_|"ready record bounds")?;
    let bytes:[u8;values::RECORD_LEN]=rd(at,0);
    let expected=track.publish.map(|c|*c.charinfo())
        .unwrap_or_else(||track.before.record[..CHARINFO_LEN].try_into().unwrap());
    check(bytes[..CHARINFO_LEN]==expected,"ready retained/target CharInfo mismatch")?;
    let tokens:[u16;5]=rd(track.ticket.entry,0x46);
    check(rd::<[u16;4]>(owner.information,0x72)==tokens[..4],
        "native D descriptor mirror mismatch")?;
    check(rd::<[u32;4]>(owner.object,0x6c)==[
        u32::from(tokens[1]),u32::from(tokens[2]),u32::from(tokens[3]),u32::from(tokens[4])],
        "native O descriptor tokens mismatch")?;
    if track.publish.is_some() {
        check(rd::<usize>(at,0x58)!=0 && rd::<usize>(at,0x60)!=0,"native Mii resource pair absent")?;
    }
    if track.retained_o {
        let proof=track.same_class.ok_or("same-class completion proof absent")?;
        proof.completed(same_class_values(&now)?,&track.descriptor)?;
    }
    let mut state=super::STATE.lock();
    check(state.track.as_ref().is_some_and(|t|t.ticket==track.ticket
        && t.gate.can_complete(track.ticket)) && super::epoch()==track.ticket.epoch,
        "ready ticket/epoch changed before commit")?;
    let owner=state.owner.as_mut().ok_or("ready owner retired")?;
    let mut registry=final_registry(&helpers,Some(track.token))?;
    owner.expected=now;owner.active=track.publish;owner.cursor=track.destination.cursor;
    registry.commit(now.authority(),now.original_kind,super::descriptor_kind(&now.descriptor));
    drop(registry);drop(state);drop(g);
    Ok(now)
}

/// Terminal class of a bank member settlement. `Diverged` means the owner's
/// physical receipt no longer describes native state; `Kept` leaves it valid.
pub(super) enum BankRefusal { Busy, Kept(&'static str), Diverged(&'static str) }

/// Same-entry bank member switch, current ready callback. Nothing is adopted: the
/// fresh snapshot must be `same_owned` with the owner's unchanged physical receipt,
/// and protected rows and source provenance must equal it too. `expected` stays
/// `before`; the registry is fenced, not written (the physical kind is unchanged).
/// A settled bank member switch (WP-M2 settle split): the current snapshot, the
/// configuration that stays active (the published one only when the exact proof was
/// adopted), the degrade reason when the body was proven but a resource clause failed,
/// and whether the record's native pair was present.
#[derive(Clone,Copy,Debug)]
pub(super) struct Settled {
    pub now:Snapshot,
    pub exact:Option<Configuration>,
    pub resource_pair:Option<bool>,
}
/// Lock order G -> adapter STATE -> COMMITTED.
#[inline(never)] // rc.8 §7: its snapshots and MiiExit copies live in its own frame, not in bank_service's
pub(super) unsafe fn bank_settled(bank:&super::BankTrack) -> Result<Settled,BankRefusal> {
    let before=&bank.before;
    let base=super::base();
    let m=map(before.binding.entry).map_err(BankRefusal::Diverged)?;
    let b=pool(base,before.binding.entry,before.binding.epoch).map_err(BankRefusal::Diverged)?;
    if b!=before.binding {return Err(BankRefusal::Diverged("bank ready G identity changed"));}
    let helpers=match helper_snapshot() {
        Ok(view)=>view,
        Err(reason) if reason==helpers::BUSY=>return Err(BankRefusal::Busy),
        Err(reason)=>return Err(BankRefusal::Kept(reason)),
    };
    if helpers!=before.helpers {return Err(BankRefusal::Kept("bank ready source provenance changed"));}
    let lock=PoolLock::acquire(base,b.manager);
    let now=match current_locked(base,m,b,before.original_kind,None,helpers) {
        Ok(now)=>now,
        Err(reason)=>{drop(lock);return Err(BankRefusal::Diverged(reason));}
    };
    // A bank group swap changes only this owner's physical group words; the bank
    // module must report that exact current group, otherwise the state diverged.
    // A Mii group switch additionally published the exact record and scalars
    // (MII-GROUPS-DESIGN §5.2): those are proven against the configuration below.
    let regrouped=now.descriptor!=before.descriptor;
    let comparable=match bank.publish {
        Some(_)=>Snapshot {descriptor:before.descriptor,scalars:before.scalars,record:before.record,..now},
        None=>Snapshot {descriptor:before.descriptor,..now},
    };
    if !same_owned(&comparable,before)
        || (regrouped && !super::super::body_bank::group_rebaseline(before.binding.entry,0,
            before.binding.epoch,&before.descriptor,&now.descriptor)) {
        super::note(format_args!("[MiiOrdinaryPlayer] bank settle diverged owner_before={:?} owner_now={:?} control_before={:?} control_now={:?} index_before={} index_now={}",
            before.owners[0],now.owners[0],before.control,now.control,before.binding.record_index,now.binding.record_index));
        drop(lock);return Err(BankRefusal::Diverged("bank ready owned state changed"));
    }
    if now.protected!=before.protected || now.layout!=before.layout {
        drop(lock);return Err(BankRefusal::Kept("bank ready protected owner state changed"));
    }
    let mut exact=None;let mut resource_pair=None;
    if let Some(c)=bank.publish {
        // The exact part of the canonical `completed()` proof without a Reload
        // (MII-GROUPS-DESIGN §5.3), split (WP-M2): the structural half (scalars,
        // record payload, D mirror, O header/index/tokens, E index, bank flag) and the
        // hard resource half (the native pair the Mii-setup adapter published); either
        // failing diverges, and the refusal keeps the owner (cascade guard).
        if !regrouped {drop(lock);return Err(BankRefusal::Diverged("Mii group settled without a group exchange"));}
        let Some(own)=now.owners[0] else {drop(lock);return Err(BankRefusal::Diverged("ready Player0 absent"));};
        let settle=crate::engine::mii::ordinary_player::ExactBankSettle {
            owner:0,descriptor:&now.descriptor,e_tokens:tokens(now.scalars.e),
            d_outfit:now.scalars.d_outfit,d_colour:now.scalars.d_colour,d_specials:now.scalars.d_specials,
            record:&now.record,d_mirror:rd(own.information,0x72),
            o_header:rd(own.object,0x54),o_index:rd(own.object,0x68),o_tokens:rd(own.object,0x6c),
            e_index:rd(before.binding.row,0x1a8),entry_flags:rd(before.binding.entry,0x591f),
        };
        let proof=settle.settle(&c);
        let outcome=proof.outcome();
        resource_pair=Some(proof.resources.is_ok());
        if let Some(reason)=outcome.diverged_reason() {drop(lock);return Err(BankRefusal::Diverged(reason));}
        exact=outcome.active(bank.publish);
    }
    let mut state=super::STATE.lock();
    let current=state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token)
        && super::epoch()==before.binding.epoch;
    let owner=match state.owner.as_mut().filter(|o|current && same_owned(&o.expected,before) && o.active.is_none()) {
        Some(owner)=>owner,
        None=>{drop(state);drop(lock);return Err(BankRefusal::Kept("bank ticket/owner changed before settlement"));}
    };
    let mut registry=match final_registry(&helpers,Some(bank.token)) {
        Ok(registry)=>registry,
        Err(reason)=>{drop(state);drop(lock);return Err(BankRefusal::Kept(reason));}
    };
    owner.cursor=bank.destination.cursor;
    if regrouped {
        // Adopt the swapped group's physical receipt exactly as a Reload completion
        // does; a Mii group's published configuration becomes the active one when its
        // exact proof was adopted, otherwise the proven body is a plain fighter.
        owner.expected=now;owner.active=exact;
        registry.commit(now.authority(),now.original_kind,super::descriptor_kind(&now.descriptor));
        super::note("[GroupSwap] adapter rebaseline owner=0 descriptor/header/body proof passed");
    }
    state.bank=None;
    drop(registry);drop(state);drop(lock);
    Ok(Settled {now,exact,resource_pair})
}

/// rc.8 R5: what the warm exit carries across `bank_restore`. `exit_plan` builds it in its own frame, so the kept
/// `MiiExit`, the snapshot read under G and their copies are gone before the restore runs.
struct ExitPlan {
    restore:super::BankTrack, configuration:Configuration, published:BankPublished, adopted:Snapshot, helpers:helpers::View,
}

/// rc.6 warm exit from a Mii group (operator decision 2026-11-08): the bank exchanged the live Mii group for the
/// window it displaced and wrote that window's `R` words, entry bytes and D mirror back (`perform_swap`). In this
/// settle tick the bank's exchange receipt proves the descriptor (`group_rebaseline`), the Mii switch's publication
/// goes back through `bank_restore` (the window is no longer the published group), whose read-back proves the rest
/// against the receipt from before the Mii with the descriptor as it is now; then the owner adopts the window with no
/// active configuration. `helpers::BUSY` before any write retries on the next callback.
#[inline(never)] // rc.8 §7: its snapshots and MiiExit copies live in its own frame, not in bank_service's
pub(super) unsafe fn exit_settled(bank:&super::BankTrack)->Result<Settled,BankRefusal> {
    // rc.8 R5: the proof before the restore (`exit_plan`) and the adoption after it (`exit_adopt`) each have their own
    // frame; this one holds the plan and the restore's result, borrowed.
    let mut plan=None;
    exit_plan(bank,&mut plan)?;
    let Some(plan)=plan.as_ref() else {return Err(BankRefusal::Diverged("warm exit without its kept Mii publication"))};
    let restored=bank_restore(&plan.restore,plan.configuration,plan.published);
    let now=match &restored {
        Ok(restored)=>&restored.now,
        Err(why) if *why==helpers::BUSY=>return Err(BankRefusal::Busy),
        Err(why)=>return Err(BankRefusal::Diverged(why)),
    };
    exit_adopt(bank,plan,now)
}

/// rc.8 R5: `exit_settled` before the restore: the kept exit, the G identity, the helpers' view, the bank's exchange
/// receipt; then the restore's track.
#[inline(never)]
unsafe fn exit_plan(bank:&super::BankTrack,plan:&mut Option<ExitPlan>)->Result<(),BankRefusal> {
    let before=&bank.before;
    // rc.8 §7, R5: the kept exit is read here once, straight into the plan (no copy of the whole `MiiExit`); the
    // restore's track takes the descriptor as it is now, after the proof below.
    {
        let state=super::STATE.lock();
        let exit=state.mii_exit.as_ref().filter(|x|x.mii.ticket.epoch==before.binding.epoch)
            .ok_or(BankRefusal::Diverged("warm exit without its kept Mii publication"))?;
        *plan=Some(ExitPlan {restore:super::BankTrack {before:exit.mii.before,descriptor:exit.mii.descriptor,..*bank},
            configuration:exit.configuration,published:exit.published,adopted:exit.adopted,helpers:helpers::View::EMPTY});
    }
    let base=super::base();
    let m=map(before.binding.entry).map_err(BankRefusal::Diverged)?;
    let b=pool(base,before.binding.entry,before.binding.epoch).map_err(BankRefusal::Diverged)?;
    if b!=before.binding {return Err(BankRefusal::Diverged("warm exit G identity changed"));}
    let helpers=match helper_snapshot() {
        Ok(view)=>view,
        Err(reason) if reason==helpers::BUSY=>return Err(BankRefusal::Busy),
        Err(reason)=>return Err(BankRefusal::Kept(reason)),
    };
    let lock=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,before.original_kind,None,helpers).map_err(BankRefusal::Diverged)?;
    drop(lock);
    if now.protected!=before.protected || now.layout!=before.layout {
        return Err(BankRefusal::Kept("warm exit protected owner state changed"));
    }
    if now.descriptor==before.descriptor || !super::super::body_bank::group_rebaseline(before.binding.entry,0,
        before.binding.epoch,&before.descriptor,&now.descriptor) {
        return Err(BankRefusal::Diverged("warm exit without its group exchange"));
    }
    let Some(plan)=plan.as_mut() else {return Err(BankRefusal::Diverged("warm exit without its kept Mii publication"))};
    // The restore's track: the receipt from before the Mii, with the descriptor as it is now.
    plan.restore.before.descriptor=now.descriptor;
    plan.helpers=helpers;
    Ok(())
}

/// rc.8 R5: `exit_settled` after the restore: the owner adopts the window (`now` is the restore's read-back).
#[inline(never)]
unsafe fn exit_adopt(bank:&super::BankTrack,plan:&ExitPlan,now:&Snapshot)->Result<Settled,BankRefusal> {
    let before=&bank.before;
    let mut state=super::STATE.lock();
    let current=state.bank.is_some_and(|t|t.ticket==bank.ticket && t.token==bank.token && !t.departed)
        && super::epoch()==before.binding.epoch;
    let Some(owner)=state.owner.as_mut().filter(|o|current && o.expected==plan.adopted && o.active==Some(plan.configuration)) else {
        return Err(BankRefusal::Diverged("warm exit ticket/owner changed after the restore"));
    };
    let mut registry=final_registry(&plan.helpers,Some(bank.token)).map_err(BankRefusal::Diverged)?;
    owner.cursor=bank.destination.cursor;
    owner.expected=*now;owner.active=None;
    registry.commit(now.authority(),now.original_kind,super::descriptor_kind(&now.descriptor));
    super::note("[GroupSwap] warm exit: Player Mii publication restored; descriptor/header/body proof passed");
    state.bank=None;state.mii_exit=None;
    drop(registry);drop(state);
    Ok(Settled {now:*now,exact:None,resource_pair:None})
}

/// Ready proof for the generic roster loader, which writes nothing here. Compares the fresh
/// state only with the receipt's `before` snapshot and the staged generic descriptor; arbitrary
/// current state is never adopted. Native may replace the O object, so O identity is not compared.
/// Lock order G -> adapter STATE -> COMMITTED; the registry is only fenced, never written.
pub(super) unsafe fn generic_completed(g:&super::Generic,kind:u32) -> Result<Snapshot,&'static str> {
    let before=&g.before;
    let base=super::base();
    let m=map(before.binding.entry)?;
    let b=pool(base,before.binding.entry,before.binding.epoch)?;
    check(b==before.binding,"generic ready G identity changed")?;
    let helpers=helper_snapshot()?;
    check(helpers==before.helpers,"generic ready source provenance changed")?;
    let lock=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,before.original_kind,None,helpers)?;
    check(now.binding==before.binding && now.thread==before.thread
        && now.symbol==before.symbol && now.manager==before.manager
        && now.internal==before.internal && now.entries==before.entries
        && now.control==before.control
        && now.protected==before.protected,"generic ready authority/protected state changed")?;
    check(now.descriptor==g.descriptor,"generic ready descriptor differs")?;
    check(now.scalars==before.scalars,"generic ready E/D configuration changed")?;
    check(now.record[..CHARINFO_LEN]==before.record[..CHARINFO_LEN],"generic ready CharInfo changed")?;
    check(now.record==before.record,"generic ready retained G resource changed")?;
    let (old,own)=(before.owners[0].ok_or("generic ready prior Player0 absent")?,
        now.owners[0].ok_or("generic ready Player0 absent")?);
    check(own.entry==old.entry && own.row==old.row && own.selector==old.selector
        && own.alias==old.alias,"generic ready Player0 identity changed")?;
    let word=|offset:usize|u32::from_le_bytes(g.descriptor[offset..offset+4].try_into().unwrap());
    check(rd::<[u32;5]>(own.object,0x54)==[0,word(0x18),word(0x1c),word(0x20),u32::from(g.descriptor[0x41])],
        "generic ready native O header differs")?;
    let task:usize=rd(own.object,0xa8);
    check(task==0 || (aligned(task)&&span(task,0x24,false)
        && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)==0),"generic ready task outstanding")?;
    let tokens:[u16;5]=rd(before.binding.entry,0x46);
    check(rd::<[u16;4]>(own.information,0x72)==tokens[..4]
        && rd::<[u32;4]>(own.object,0x6c)==[
            u32::from(tokens[1]),u32::from(tokens[2]),u32::from(tokens[3]),u32::from(tokens[4])],
        "generic ready native D/O propagation differs")?;
    // Explicit bank receipt: an owned ready bank whose physical group is exactly this
    // descriptor publishes its proven live member. Every other receipt keeps +0x18.
    let published=super::bank_live(before.binding.entry as *const u8,&now.descriptor)
        .filter(|live|*live==kind).unwrap_or_else(||super::descriptor_kind(&now.descriptor));
    let mut state=super::STATE.lock();
    let transaction_busy=state.busy() || state.request.is_some()
        || super::epoch()!=before.binding.epoch;
    let owner=state.owner.as_mut().filter(|o|o.expected==g.expected)
        .ok_or("generic Player0 owner changed during INSTALL")?;
    crate::engine::mii::ordinary_player::generic_ordinary_commit(
        crate::engine::mii::ordinary_player::GenericOrdinaryFacts {
            transaction_busy,origin_is_saved:owner.origin_saved.is_some(),
            active_is_saved:owner.active.is_some(),cursor_owned:owner.cursor.is_some(),
            kind,published_kind:published,
            index:own.index,original_index:old.index,
        })?;
    let registry=final_registry(&helpers,Some(g.token))?;
    owner.expected=now;
    drop(registry);drop(state);drop(lock);
    Ok(now)
}

/// Exact catalog origin capture reuses the same reader and claim authority as later tickets.
/// Row getters run outside G; the final full snapshot is rederived before installing an owner.
pub(super) unsafe fn capture_origin(entry:usize,epoch:u64,kind:u32,catalog:&Catalog,opening:u64)
    ->Result<(),helpers::CaptureError> {
    let previous=super::STATE.lock().owner.as_ref().map(|o|o.expected);
    if let Some(expected)=previous {
        let now=snapshot(entry,epoch,kind)?;
        check(same_owned(&now,&expected),"menu current owner changed")?;
        let mut state=super::STATE.lock();
        check(super::super::mii_saved_alpha::menu_epoch()==opening
            && state.capture && !state.busy() && super::epoch()==epoch
            && state.owner.as_ref().is_some_and(|o|o.expected==expected),"menu owner retired")?;
        let registry=final_registry(&now.helpers,None)?;
        state.capture=false;state.menu_valid=true;
        drop(registry);return Ok(());
    }
    let before=snapshot(entry,epoch,kind)?;
    let c=if MiiClass::from_kind(kind).is_some() {
        let o=before.owners[0].ok_or("saved origin owner absent")?;
        // This first copied selection is not publication authority; it is rechecked below.
        let base=super::base();let m=map(entry)?;let b=pool(base,entry,epoch)?;
        let helpers=helper_snapshot()?;
        let g=PoolLock::acquire(base,b.manager);
        let fresh=current_locked(base,m,b,kind,None,helpers)?;
        check(fresh==before,"origin changed before catalog resolution")?;
        let c=role_owner::saved_origin(catalog,kind,mirrors(o),&fresh.record)
            .ok_or("no exact original saved configuration")?;
        drop(g);Some(c)
    } else {None};
    let rows=c.map(|c|target_rows(c)).transpose()?;
    let base=super::base();let m=map(entry)?;let b=pool(base,entry,epoch)?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(base,b.manager);
    let now=current_locked(base,m,b,kind,None,helpers)?;
    check(now==before && now.thread!=0,"origin changed across row validation")?;
    if let (Some(c),Some(rows))=(c,rows) {
        role_owner::validate_saved(c,0,mirrors(now.owners[0].unwrap()),rows,&now.record)?;
    }
    let mut state=super::STATE.lock();
    check(super::super::mii_saved_alpha::menu_epoch()==opening
        && state.capture && state.owner.is_none() && !state.busy()
        && super::epoch()==epoch,"origin capture retired")?;
    let mut registry=final_registry(&helpers,None)?;
    state.owner=Some(super::Owner{origin:now,origin_saved:c,expected:now,active:c,cursor:None});
    state.capture=false;state.menu_valid=true;
    registry.commit(now.authority(),kind,kind);
    drop(registry);drop(state);drop(g);
    super::super::observe_mii_origin(super::super::Role::Player,epoch,kind,super::descriptor_forms(&now.descriptor),c);
    Ok(())
}

#[cfg(test)]
mod scalar_write_tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn scalar_write_adds_only_e_hat_and_never_writes_native_d_hat() {
        // Actual writer over synthetic local memory. No Horizon calls or game bytes.
        #[repr(align(8))]
        struct Aligned<const N:usize>([u8;N]);
        let mut e_memory=Aligned([0xa5u8;0x260]);let mut d_memory=Aligned([0x5au8;0x3a0]);
        let e=&mut e_memory.0;let d=&mut d_memory.0;
        let scalars=Scalars {e:Tokens {outfit:75,hat:125,colour:0,voice:37,specials:[2,2,2,1]},
            d_outfit:75,d_colour:0,d_specials:[2,2,2,1]};
        unsafe {scalars.write(e.as_mut_ptr() as usize,d.as_mut_ptr() as usize);}
        assert_eq!(&e[0x1aa..0x1b6],&[75,0,125,0,0,0,37,0,2,2,2,1]);
        assert!(e[..0x1aa].iter().chain(&e[0x1b6..]).all(|b|*b==0xa5));
        assert_eq!(&d[0x74..0x7a],&[75,0,0x5a,0x5a,0,0]);
        assert_eq!(&d[0x390..0x3a0],&[2,0,0,0,2,0,0,0,2,0,0,0,1,0,0,0]);
        for (i,byte) in d.iter().enumerate() {
            if !(0x74..0x76).contains(&i) && !(0x78..0x7a).contains(&i) && !(0x390..0x3a0).contains(&i) {
                assert_eq!(*byte,0x5a,"native-owned D byte {i:#x}");
            }
        }
        // Return to a distinct value exercises the same source writer, not a 125 special case.
        unsafe {Scalars {e:Tokens {hat:0,..scalars.e},..scalars}.write(e.as_mut_ptr() as usize,d.as_mut_ptr() as usize);}
        assert_eq!(&e[0x1ac..0x1ae],&[0,0]);
        assert_eq!(&d[0x76..0x78],&[0x5a,0x5a]);
    }
    }
}
