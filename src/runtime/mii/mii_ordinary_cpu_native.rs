//! CPU1 state commits around the shared selector transaction reads.
use crate::runtime::mii::mii_native_access::*;
use super::super::mii_ordinary_player::native::{check,pointer,span,Scalars,mirrors,
    helpers,helper_snapshot,final_registry};
#[cfg(feature="native-body-bank-alpha")]
use super::super::mii_ordinary_player::native::{permanent,publish_refusal};
#[cfg(feature="native-body-bank-alpha")]
use crate::engine::swap::body_bank::mii_publish::PublishRefusal;
use super::super::mii_selector_native::{self as shared,Map,same_map};
use crate::engine::mii::{lifetime::{Binding,Configuration},native_values as values,
    ordinary_cpu as policy,ordinary_player::{SameClass,SameClassValues},role_owner,
    inventory::{MiiClass,Snapshot as Catalog}};
pub(super) use shared::{same_owned,Snapshot};
fn owner()->Result<usize,&'static str> {super::super::mii_backend::cpu().map(|id|id as usize).ok_or("CPU backend unavailable")}
unsafe fn map(entry:usize)->Result<Map,&'static str> {shared::map(entry,owner()?)}
unsafe fn pool(entry:usize,epoch:u64)->Result<Binding,&'static str> {shared::pool(entry,epoch,owner()?)}
unsafe fn current_locked(m:Map,b:Binding,kind:u32,staged:Option<(&[u8;0x70],usize)>,helpers:helpers::View)
    ->Result<Snapshot,&'static str> {shared::current_locked(m,b,kind,staged,helpers,None)}
pub(super) unsafe fn snapshot(entry:usize,epoch:u64,kind:u32)->Result<Snapshot,&'static str> {
    shared::snapshot(entry,epoch,kind,owner()?,None)
}
pub(super) unsafe fn target_rows(c:Configuration)->Result<(),&'static str> {shared::target_rows(c)}
/// Copied phase values under CPU1's own current map/protected-record union lock.
unsafe fn same_class_values(now:&Snapshot)->Result<SameClassValues,&'static str> {
    let o=now.owners[now.binding.owner as usize].ok_or("CPU same-class owner absent")?;
    check(span(o.object,0xb0,true),"CPU same-class writable O span")?;
    let task:usize=rd(o.object,0xa8);
    let task_idle=task==0 || (aligned(task)&&span(task,0x14,false)
        && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)==0);
    let result=SameClassValues {binding:now.binding,thread:now.thread,owner:o,
        header:rd(o.object,0x54),
        mirrors:values::Mirrors::decode(rd(o.row,0x1a8),rd(o.entry,0x46),
            rd(o.information,0x72),rd(o.information,0x390),rd(o.object,0x68)),
        initialized:rd::<u8>(o.object,0x4c)&1!=0,task_idle,
        rebuild_open:rd::<u8>(o.information,0x94)==0};
    let table=pointer(super::base(),OWNER_TABLE,0x130,false)?;
    check(rd::<usize>(table,0xe8+now.binding.owner as usize*8)==o.object && rd::<usize>(o.entry,0xf8)==o.information
        && rd::<usize>(o.object,0xa8)==task,"CPU same-class read authority changed")?;
    Ok(result)
}
pub(super) unsafe fn prepare_same_class(expected:Snapshot,before:Configuration,after:Configuration)
    ->Result<SameClass,&'static str> {
    let rows=[super::super::mii_ordinary_player::native::target_rows(before)?,
        super::super::mii_ordinary_player::native::target_rows(after)?];
    let b=pool(expected.binding.entry,expected.binding.epoch)?;let m=map(b.entry)?;
    check(b==expected.binding,"CPU same-class preflight G changed")?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(super::base(),b.manager);
    let now=current_locked(m,b,expected.original_kind,None,helpers)?;
    check(now==expected,"CPU same-class preflight snapshot changed")?;
    let at=values::record_address(b,b).map_err(|_|"CPU same-class record bounds")?;
    check(rd::<usize>(at,0x58)!=0 && rd::<usize>(at,0x60)!=0,
        "CPU same-class active Mii resource pair absent")?;
    let proof=SameClass::prepare_cpu_in(now.layout,before,after,same_class_values(&now)?,
        &now.descriptor,rows,&now.record)?;
    drop(g);Ok(proof)
}
/// Select the observed native configuration branch once. Failed proof never tries its peer.
pub(super) unsafe fn publish_same_class(track:&super::Track,selected:usize,
    evidence:&mut Option<Publication>)->Result<bool,&'static str> {
    let proof=track.same_class.ok_or("CPU same-class proof absent")?;
    check(track.publish==Some(proof.target()),"CPU same-class target changed")?;
    let rows=[super::super::mii_ordinary_player::native::target_rows(proof.before())?,
        super::super::mii_ordinary_player::native::target_rows(proof.target())?];
    check(aligned(selected)&&span(selected,0xb0,false),"CPU same-class selected O span")?;
    if rd::<u8>(selected,0x4c)&1==0 {
        // Bit0 clear: native configure runs, the gate installed nothing (U5).
        publish(track,selected,evidence,crate::engine::mii::identity::ExpectedMutation::None)?;return Ok(false);
    }
    let m=map(track.ticket.entry)?;let b=pool(track.ticket.entry,track.ticket.epoch)?;
    check(b==track.before.binding,"CPU same-class phase G changed")?;
    let helpers=helper_snapshot()?;
    check(helpers==track.before.helpers,"CPU same-class source provenance changed")?;
    let g=PoolLock::acquire(super::base(),b.manager);
    let now=current_locked(m,b,track.before.original_kind,
        Some((&track.descriptor,selected)),helpers)?;
    retained_origin(track)?;
    check(same_map(&now,&track.before) && now.protected==track.before.protected
        && now.records==track.before.records,"CPU same-class protected union changed")?;
    check(now.index==track.before.index && now.scalars==track.before.scalars
        && now.record==track.before.record && now.control==track.before.control,
        "CPU same-class old scalars/control/record changed")?;
    check(rd::<u8>(b.entry,0x5920)==3 && rd::<u8>(b.entry,0x591c)==6,
        "CPU same-class native loader6 required")?;
    let observed=same_class_values(&now)?;
    let update=proof.prepare_write(observed,&track.descriptor,rows,&now.record)?;
    let at=values::record_address(b,b).map_err(|_|"CPU same-class final record bounds")?;
    let targets=Targets {e:b.row,fe:b.entry,d:observed.owner.information,o:observed.owner.object};
    let mut state=super::STATE.lock().unwrap();
    let fresh=state.track.as_mut().ok_or("CPU same-class ticket retired")?;
    check(fresh.ticket==track.ticket && fresh.same_class==Some(proof)
        && fresh.destination==track.destination && fresh.gate.may_publish(track.ticket,6)
        && super::epoch()==track.ticket.epoch,"CPU same-class final ticket/epoch changed")?;
    let registry=final_registry(helpers,Some(track.token))?;
    // Existing same-class write set only: payload and outfit/colour/voice/specials.
    // No index/header/flag/task/resource-pair or vector write in this branch.
    copy_update(at,&update);write_tokens(&targets,update.tokens().into());
    fresh.retained_o=true;
    let published=fresh.gate.publish(track.ticket,6);debug_assert!(published);
    drop(registry);drop(state);drop(g);Ok(true)
}
pub(super) use shared::Publication;
/// The native origin can precede its selector destination (LDN). Keep that record
/// untouched and compare it under the shared transaction's current G lock.
unsafe fn retained_origin(track:&super::Track)->Result<(),&'static str> {
    if track.origin_index==track.before.binding.record_index {return Ok(());}
    let Some(expected)=track.origin_record else{return Ok(());};
    let b=Binding {record_index:track.origin_index,..track.before.binding};
    let at=values::record_address(b,b).map_err(|_|"CPU retained origin bounds")?;
    check(span(at,values::RECORD_LEN,false),"CPU retained origin span")?;
    check(rd::<[u8;104]>(at,0)==expected,"CPU retained origin record changed")
}
fn target(track:&super::Track)->shared::PublicationTarget<'_> {
    let published=u16::from_le_bytes([track.descriptor[0x46],track.descriptor[0x47]]);
    shared::PublicationTarget {before:&track.before,descriptor:&track.descriptor,
        // CharInfo is written at record_index. Keep the payload only for that
        // destination. Origin restore carries track.origin_index separately.
        payload:track.publish.filter(|_|policy::saved_payload_routes_to_destination(
            published,track.before.binding.record_index)),
        scalars:track.publish.map_or(track.origin_scalars,Scalars::saved),source:None,
        hat_change:track.hat_change,original_index:track.origin_index}
}
/// `allowed`: the gate writer's receipt (`ExpectedMutation::ConfigureSkipped`) when it installed the
/// configure-equivalent `O+0x68`/tokens on the skip path (U5, W-M2-4), otherwise `None`.
pub(super) unsafe fn publish(track:&super::Track,selected:usize,evidence:&mut Option<Publication>,
    allowed:crate::engine::mii::identity::ExpectedMutation)->Result<(),&'static str> {
    let publication=shared::publication(target(track),selected,evidence,allowed)?;
    retained_origin(track)?;
    let mut state=super::STATE.lock().unwrap();
    let fresh=state.track.as_mut().ok_or("CPU ticket retired")?;
    check(fresh.ticket==track.ticket && fresh.gate.may_publish(track.ticket,6)
        && fresh.destination==track.destination && super::epoch()==track.ticket.epoch,
        "CPU final ticket/epoch changed")?;
    let registry=final_registry(publication.observed.before.helpers,Some(track.token))?;
    publication.write();
    let published=fresh.gate.publish(track.ticket,6);debug_assert!(published);
    drop(registry);drop(state);drop(publication);Ok(())
}
pub(super) unsafe fn completed(track:&super::Track)->Result<Snapshot,&'static str> {
    let rows=track.publish.map(|c|super::super::mii_ordinary_player::native::target_rows(c)).transpose()?;
    let completion=shared::completed(target(track))?;
    let now=completion.now;
    retained_origin(track)?;
    if let (Some(c),Some(rows))=(track.publish,rows) {
        let own=now.owners[now.binding.owner as usize].ok_or("CPU ready owner absent")?;
        role_owner::validate_saved(c,now.index,mirrors(own),rows,&now.current_record)?;
        check(now.current_record[88..96]!=[0;8] && now.current_record[96..104]!=[0;8],
            "CPU native-consumed current resource pair absent")?;
    }
    if track.retained_o {
        let proof=track.same_class.ok_or("CPU same-class completion proof absent")?;
        proof.completed(same_class_values(&now)?,&track.descriptor)?;
    }
    let mut state=super::STATE.lock().unwrap();
    check(state.track.as_ref().is_some_and(|t|t.ticket==track.ticket&&t.gate.can_complete(track.ticket))
        && super::epoch()==track.ticket.epoch,"CPU ready ticket/epoch changed before commit")?;
    let owner=state.owner.as_mut().ok_or("CPU ready owner retired")?;
    let mut registry=final_registry(now.helpers,Some(track.token))?;
    owner.expected=now;owner.live=track.destination;
    registry.commit(now.authority(),now.original_kind,super::descriptor_kind(&now.descriptor));
    drop(registry);drop(state);drop(completion);Ok(now)
}
/// Terminal class of a CPU bank member settlement; see the Player adapter.
#[cfg(feature="native-body-bank-alpha")]
pub(super) enum BankRefusal { Busy, Kept(&'static str), Diverged(&'static str) }

/// What a CPU Mii one-member group switch published before its native request
/// (the destination record, `E+0x1a8` and the E/D scalars), for a rollback.
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy)]
pub(super) struct BankPublished { record:[u8;values::RECORD_LEN], scalars:Scalars, index:u16 }

#[cfg(feature="native-body-bank-alpha")]
fn bank_target<'a>(bank:&'a super::BankTrack,c:Configuration)->shared::PublicationTarget<'a> {
    let published=u16::from_le_bytes([bank.descriptor[0x46],bank.descriptor[0x47]]);
    shared::PublicationTarget {before:&bank.before,descriptor:&bank.descriptor,
        payload:Some(c).filter(|_|policy::saved_payload_routes_to_destination(published,bank.before.binding.record_index)),
        scalars:Scalars::saved(c),source:None,hat_change:bank.hat_change,original_index:bank.origin_index}
}
#[cfg(feature="native-body-bank-alpha")]
unsafe fn retained_origin_bank(bank:&super::BankTrack)->Result<(),&'static str> {
    if bank.origin_index==bank.before.binding.record_index {return Ok(());}
    let Some(expected)=bank.origin_record else{return Ok(());};
    let b=Binding {record_index:bank.origin_index,..bank.before.binding};
    let at=values::record_address(b,b).map_err(|_|"CPU retained origin bounds")?;
    check(span(at,values::RECORD_LEN,false),"CPU retained origin span")?;
    check(rd::<[u8;104]>(at,0)==expected,"CPU retained origin record changed")
}
/// MII-GROUPS-DESIGN §5.2 for CPU1: the shared selector publication in its bank
/// phase (native loader idle, the live initialized O, the group's hat as the
/// headgear proof), then the same write set as `publish`: the CharInfo at the
/// destination record, `E+0x1a8` and the E/D scalars. The identity words, entry
/// bytes and D mirror follow at the bank's build.
#[cfg(feature="native-body-bank-alpha")]
pub(super) unsafe fn bank_publish(bank:&super::BankTrack,c:Configuration,group_hat:u16,evidence:&mut Option<Publication>)
    ->Result<(),&'static str> {
    let target=bank_target(bank,c);
    check(target.payload.is_some(),"CPU Mii group payload does not route to the destination record")?;
    let selected=bank.before.owners[bank.before.binding.owner as usize].ok_or("CPU bank owner absent")?.object;
    let publication=shared::publication_in(target,selected,evidence,shared::PublicationPhase::BankSwitch {group_hat},
        crate::engine::mii::identity::ExpectedMutation::None)?;
    retained_origin_bank(bank)?;
    let mut state=super::STATE.lock().unwrap();
    check(!state.departed && state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token)
        && super::epoch()==bank.ticket.epoch,"CPU bank publication ticket/epoch changed")?;
    let track=state.bank.as_mut().ok_or("CPU bank publication track absent")?;
    let registry=final_registry(publication.observed.before.helpers,Some(bank.token))?;
    // MII-PHASEB: the track records what the write replaces in the same critical section;
    // every terminal arm compensates it (`bank_refused` -> `bank_restore`).
    let before=publication.observed.before;
    track.published=Some((c,BankPublished {record:before.record,scalars:before.scalars,index:before.index}));
    publication.write();
    drop(registry);drop(state);drop(publication);
    Ok(())
}
/// MII-GROUPS prefetch publication for CPU1: a planned Mii group's CharInfo goes
/// onto the destination record before the group's key request copies it
/// (`0x17e64d0` at `0x17e6c10`). The destination record is reservation-guarded,
/// so the write takes INSTALL and the record-1 reservation exactly as the exact
/// path does, releases INSTALL right after, and the owner's receipt adopts the
/// bytes. The switch's `bank_publish` rewrites them with `E+0x1a8`/scalars later.
#[cfg(feature="native-body-bank-alpha")]
pub(super) unsafe fn prefetch_publish(entry:usize,c:Configuration,identity:crate::engine::swap::body_bank::MiiIdentity)
    ->Result<(),PublishRefusal> {
    shared::target_rows(c).map_err(publish_refusal)?;
    let (expected,original_kind,retained)={
        let s=super::STATE.lock().unwrap();
        if s.departed || s.busy() || s.request.is_some() || s.capture || !s.menu_valid {return Err(PublishRefusal::Busy);}
        let o=s.owner.as_ref().ok_or(PublishRefusal::Permanent("CPU Mii group publication without an owner"))?;
        if o.origin.binding.entry!=entry || o.origin.binding.epoch!=super::epoch() {
            return Err(PublishRefusal::Permanent("CPU Mii group publication owner/entry/epoch differ"));
        }
        (o.expected,o.origin.original_kind,s.reservation.held())
    };
    permanent(identity.kind()==c.canonical().class as u32 && identity.index==expected.binding.record_index
        && identity.hat()==c.tokens().hat && identity.outfit()==c.tokens().outfit,
        "CPU Mii group publication identity differs from the configuration")?;
    let m=map(entry).map_err(publish_refusal)?;
    let b=pool(entry,expected.binding.epoch).map_err(publish_refusal)?;
    permanent(b==expected.binding,"CPU Mii group publication G changed")?;
    // INVARIANT (C116.2 mii-fix; `R/c1162/MII-PROVENANCE-REPORT.md`). The owner's stored `expected.helpers` is the
    // helper view of its LAST completion and is not compared with the live one: helper sources are replaced between
    // completions (the cycle preloader, Kirby hats), that gap is unfenced and legitimate, and the comparison refused
    // all 180 attempts of the first console boot of C116.1 once helper 3 had been reloaded. What authorizes the
    // CharInfo write is, within this one transaction:
    //  1. the owner still equals its last proven snapshot (`verify` of the WP5a identity receipt and `same_owned`
    //     in `prefetch_publish_owned`, plus the exact `o.expected==expected` under STATE) and its pool/entry/epoch
    //     are the group's (above);
    //  2. every non-destination helper row of the locked walk (`current_locked` -> `helper_kind`) is re-authorized
    //     against ONE view V, the fresh `helper_snapshot()` taken here, which is BUSY while any source loads;
    //  3. V is unchanged, with no source loading, from the start of the walk to the write (`check_helpers_for` at
    //     both ends of `current_locked`, then `final_registry(V, Some(token))` under the registry lock); and
    //  4. the only INSTALL held is this transaction's (`INSTALL.try_begin` here, `final_registry` checks `owns`).
    // The write (the 88-byte CharInfo at the owner's own destination record) neither reads nor writes a helper row,
    // so what must not move is V across the walk-to-write window, not the view the owner last completed with.
    let helpers=helper_snapshot().map_err(publish_refusal)?;
    let receipt=PrefetchReceipt {identity:prefetch_identity(&expected),expected,helpers,original_kind,retained,map:m};
    let Some(token)=super::super::INSTALL.try_begin(false) else {return Err(PublishRefusal::Busy);};
    let result=prefetch_publish_owned(receipt,c,token);
    super::super::finish_install(token,"CPU Mii group record publication");
    result
}
/// What `prefetch_publish` proved before it took INSTALL, handed to the locked write as one
/// value (WORKAROUND-DECISIONS row 24: no `too_many_arguments` allow). `identity` is WP5a's exact
/// receipt projected from the owner's `expected` snapshot: entry/owner/epoch from its binding,
/// the descriptor and the `O+0x68` index, the identity fields this site has always compared
/// (`same_owned`); the fields it never compared stay zero on both sides, as in WP5a's own
/// projections (`From<GroupOwner>`, `outgoing_object_matches_for`). `expected` carries the
/// owner's last proven snapshot, its binding included; its `helpers` field is the view of the owner's
/// last completion and is NOT used here. `helpers` is the fresh view V (`helper_snapshot()`, taken
/// after the binding proof and before INSTALL) the locked walk and the registry fence use
/// (see the invariant in `prefetch_publish`); `original_kind` is the owner's origin kind the locked
/// walk is keyed by, `retained` the reservation mode and `map` the map the caller walked.
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy)]
struct PrefetchReceipt {
    identity:crate::engine::mii::identity::Identity,
    expected:Snapshot,
    helpers:helpers::View,
    original_kind:u32,
    retained:bool,
    map:Map,
}
#[cfg(feature="native-body-bank-alpha")]
fn prefetch_identity(s:&Snapshot)->crate::engine::mii::identity::Identity {
    crate::engine::mii::identity::Identity {entry:s.binding.entry,owner:s.binding.owner,epoch:s.binding.epoch,
        descriptor:s.descriptor,index:s.index,..crate::engine::mii::identity::Identity::ZERO}
}
#[cfg(feature="native-body-bank-alpha")]
unsafe fn prefetch_publish_owned(receipt:PrefetchReceipt,c:Configuration,token:super::super::InstallToken)
    ->Result<(),PublishRefusal> {
    use crate::engine::mii::identity::{verify,ExpectedMutation};
    let PrefetchReceipt {identity,expected,helpers,original_kind,retained,map:m}=receipt;
    let (entry,b)=(identity.entry,expected.binding);
    let g=PoolLock::acquire(super::base(),b.manager);
    let now=current_locked(m,b,original_kind,None,helpers).map_err(publish_refusal)?;
    // One receipt compare (WP5a) names the identity field that moved; `same_owned` then proves
    // the rest of the owned state exactly as before (its rule already implies this one).
    verify(&identity,&prefetch_identity(&now),ExpectedMutation::None)
        .map_err(|_|PublishRefusal::Permanent("CPU Mii group publication identity changed"))?;
    permanent(same_owned(&now,&expected,retained),"CPU Mii group publication owned state changed")?;
    check(rd::<u8>(entry,0x5920)==6 && rd::<u8>(entry,0x591c)==14 && rd::<u8>(entry,0x591a)==0xff
        && rd::<u8>(entry,0x5934)==0 && rd::<u8>(entry,0x5935)==0,
        "CPU Mii group publication needs the native loader idle at 6/14").map_err(PublishRefusal::Race)?;
    let mut state=super::STATE.lock().unwrap();
    if state.departed || state.busy() || state.request.is_some() {return Err(PublishRefusal::Busy);}
    if !state.reservation.acquire(super::super::INSTALL.owns(token)) {
        return Err(PublishRefusal::Race("CPU reservation refused before the Mii group publication"));
    }
    let registry=final_registry(helpers,Some(token)).map_err(publish_refusal)?;
    let owner=state.owner.as_mut().filter(|o|o.expected==expected)
        .ok_or(PublishRefusal::Race("CPU Mii group publication owner changed"))?;
    let at=values::record_address(b,b).map_err(|_|PublishRefusal::Race("CPU destination record bounds"))?;
    for (off,byte) in c.charinfo().iter().enumerate() {wr(at,off,*byte);}
    owner.expected.record[..CHARINFO_LEN].copy_from_slice(c.charinfo());
    drop(registry);drop(state);drop(g);
    Ok(())
}
/// A verified restore: the receipt and the owner's state read back after the writes.
#[cfg(feature="native-body-bank-alpha")]
pub(super) struct Restored {
    pub receipt:crate::engine::mii::ordinary_player::RestoreReceipt,
    pub now:Snapshot,
    /// This owner claims its current record (`current_record_claim`, as `same_owned`).
    pub current_claimed:bool,
}

/// MII-PHASEB: the one compensation of `bank_publish` (see the Player `bank_restore`):
/// restore the destination record's CharInfo, `E+0x1a8` and the scalars, only while the
/// window is not the published group, they still read exactly as published and the
/// ticket, epoch and scene are the publication's. `E+0x1a8` names the destination while
/// FE/D name the outgoing index, so those reads are direct; after the writes the owner is
/// read back through the current walk (consistent again) and compared with the kept
/// receipt, adopting at most native's own pair. `helpers::BUSY` comes back only before any
/// write (the caller retries).
#[cfg(feature="native-body-bank-alpha")]
pub(super) unsafe fn bank_restore(bank:&super::BankTrack,c:Configuration,published:BankPublished)->Result<Restored,&'static str> {
    use crate::engine::mii::ordinary_player::{publication_rollback,restore_receipt,PublicationRollback};
    let before=&bank.before;
    let m=map(before.binding.entry)?;
    let b=pool(before.binding.entry,before.binding.epoch)?;
    check(b==before.binding,"CPU bank rollback G changed")?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(super::base(),b.manager);
    let d=pointer(before.binding.entry,0xf8,0x3a0,true)?;
    let at=values::record_address(b,b).map_err(|_|"CPU bank rollback record bounds")?;
    check(span(at,values::RECORD_LEN,true) && span(b.row,0x260,true),"CPU bank rollback spans")?;
    // Live only when the window *is* the published group (its identity).
    let identity=crate::engine::swap::body_bank::MiiIdentity::from_descriptor;
    let live=identity(&rd::<[u8;0x70]>(before.binding.entry,0)).is_some_and(|id|Some(id)==identity(&bank.descriptor));
    if publication_rollback(true,live)==PublicationRollback::KeepLive {
        return Err("the exchange made the published group the window; the publication is live");
    }
    check(rd::<u16>(b.row,0x1a8)==u16::from_le_bytes([bank.descriptor[0x46],bank.descriptor[0x47]])
        && rd::<[u8;CHARINFO_LEN]>(at,0)==*c.charinfo()
        && Scalars::read(b.row,d)==Scalars::saved(c),
        "CPU bank rollback state is not the published state")?;
    // The ticket/epoch/scene proof `bank_publish` made before it wrote (G -> STATE -> COMMITTED).
    let state=super::STATE.lock().unwrap();
    check(!state.departed && state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token)
        && super::epoch()==bank.ticket.epoch,"CPU bank rollback ticket/epoch changed or the scene departed")?;
    let registry=final_registry(helpers,Some(bank.token))?;
    // No fallible operation between here and the writes; nothing after them returns the retry sentinel.
    for (off,byte) in published.record[..CHARINFO_LEN].iter().enumerate() {wr(at,off,*byte);}
    wr::<u16>(b.row,0x1a8,published.index);
    published.scalars.write(b.row,d);
    drop(registry);drop(state);
    let after=current_locked(m,b,before.original_kind,None,helpers)
        .map_err(|_|"CPU bank publication restore read-back failed")?;
    drop(g);
    let rest=same_owned(&Snapshot {record:before.record,current_record:before.current_record,..after},before,bank.retained);
    let current_claimed=role_owner::current_record_claim(
        u32::from_le_bytes([before.descriptor[0x18],before.descriptor[0x19],before.descriptor[0x1a],before.descriptor[0x1b]]),before.index);
    let receipt=restore_receipt(&before.record,&after.record,rest);
    let receipt=if current_claimed {receipt.and(restore_receipt(&before.current_record,&after.current_record,rest))} else {receipt};
    if !receipt.verified() {return Err("CPU bank publication restore did not read back as the owner's receipt");}
    Ok(Restored {receipt,now:after,current_claimed})
}

/// Same-entry CPU bank member switch, current ready callback. The fresh snapshot
/// must be `same_owned` (same record mode as preflight) with the unchanged physical
/// receipt, with equal protected rows, records and provenance. `expected` stays
/// `before`; the registry is fenced, not written. Lock order G -> STATE -> COMMITTED.
/// A settled CPU bank member switch (WP-M2 settle split; see the Player `Settled`).
#[cfg(feature="native-body-bank-alpha")]
#[derive(Clone,Copy,Debug)]
pub(super) struct Settled {
    pub now:Snapshot,
    pub exact:Option<Configuration>,
    pub resource_pair:Option<bool>,
}
#[cfg(feature="native-body-bank-alpha")]
pub(super) unsafe fn bank_settled(bank:&super::BankTrack)->Result<Settled,BankRefusal> {
    let before=&bank.before;
    let m=map(before.binding.entry).map_err(BankRefusal::Diverged)?;
    let b=pool(before.binding.entry,before.binding.epoch).map_err(BankRefusal::Diverged)?;
    if b!=before.binding {return Err(BankRefusal::Diverged("CPU bank ready G identity changed"));}
    let helpers=match helper_snapshot() {
        Ok(view)=>view,
        Err(reason) if reason==helpers::BUSY=>return Err(BankRefusal::Busy),
        Err(reason)=>return Err(BankRefusal::Kept(reason)),
    };
    if helpers!=before.helpers {return Err(BankRefusal::Kept("CPU bank ready source provenance changed"));}
    let lock=PoolLock::acquire(super::base(),b.manager);
    let now=match current_locked(m,b,before.original_kind,None,helpers) {
        Ok(now)=>now,
        Err(reason)=>{drop(lock);return Err(BankRefusal::Diverged(reason));}
    };
    // A bank group swap changes only this owner's physical group words; the bank
    // module must report that exact current group, otherwise the state diverged.
    // A Mii group switch additionally published the destination record, `E+0x1a8`
    // and the scalars (MII-GROUPS-DESIGN §5.2): those are proven below against the
    // configuration instead of `before`.
    let regrouped=now.descriptor!=before.descriptor;
    let owner_slot=now.binding.owner as usize;
    let comparable=match bank.publish {
        Some(_)=>{
            // The publication moved `E+0x1a8` to the destination record (`index=0->1`,
            // B124PRE M8 L4606); the owner view carries that index too, so it is compared
            // at `before`'s value here and the new value is proven structurally below
            // (`e_index`). INFERENCE for the M8 clause `CPU bank ready owned state
            // changed`; the diagnostic line below names the differing fields otherwise.
            let mut owners=now.owners;
            if let (Some(mut o),Some(b))=(owners[owner_slot],before.owners[owner_slot]) {o.index=b.index;owners[owner_slot]=Some(o);}
            Snapshot {descriptor:before.descriptor,scalars:before.scalars,record:before.record,
                current_record:before.current_record,index:before.index,owners,..now}
        }
        None=>Snapshot {descriptor:before.descriptor,..now},
    };
    if !same_owned(&comparable,before,bank.retained)
        || (regrouped && !super::super::body_bank::group_rebaseline(before.binding.entry,before.binding.owner,
            before.binding.epoch,&before.descriptor,&now.descriptor)) {
        super::note(format_args!("[MiiOrdinaryCpu] bank settle diverged owner_before={:?} owner_now={:?} control_before={:?} control_now={:?} index_before={} index_now={} map_same={}",
            before.owners[owner_slot],now.owners[owner_slot],before.control,now.control,before.index,now.index,same_map(&comparable,before)));
        drop(lock);return Err(BankRefusal::Diverged("CPU bank ready owned state changed"));
    }
    if now.protected!=before.protected || now.records!=before.records || now.layout!=before.layout {
        drop(lock);return Err(BankRefusal::Kept("CPU bank ready protected owner state changed"));
    }
    let mut exact=None;let mut resource_pair=None;
    if let Some(c)=bank.publish {
        // The exact part of the canonical `completed()` proof without a Reload
        // (MII-GROUPS-DESIGN §5.3), split (WP-M2): structural and the hard resource
        // clause both decide adoption; the current record is the destination record.
        if !regrouped {drop(lock);return Err(BankRefusal::Diverged("CPU Mii group settled without a group exchange"));}
        let Some(own)=now.owners[owner_slot] else {drop(lock);return Err(BankRefusal::Diverged("CPU ready owner absent"));};
        let settle=crate::engine::mii::ordinary_player::ExactBankSettle {
            owner:now.binding.owner,descriptor:&now.descriptor,e_tokens:tokens(now.scalars.e),
            d_outfit:now.scalars.d_outfit,d_colour:now.scalars.d_colour,d_specials:now.scalars.d_specials,
            record:&now.record,d_mirror:rd(own.information,0x72),
            o_header:rd(own.object,0x54),o_index:rd(own.object,0x68),o_tokens:rd(own.object,0x6c),
            e_index:now.index,entry_flags:rd(before.binding.entry,0x591f),
        };
        let proof=settle.settle(&c);
        let outcome=proof.outcome();
        resource_pair=Some(proof.resources.is_ok());
        if let Some(reason)=outcome.diverged_reason() {drop(lock);return Err(BankRefusal::Diverged(reason));}
        if now.current_record!=now.record || now.index!=before.binding.record_index {
            drop(lock);return Err(BankRefusal::Diverged("CPU Mii group current record is not the destination record"));
        }
        exact=outcome.active(bank.publish);
    }
    let mut state=super::STATE.lock().unwrap();
    let current=!state.departed && state.bank.is_some_and(|now|now.ticket==bank.ticket && now.token==bank.token)
        && super::epoch()==before.binding.epoch;
    let owner=match state.owner.as_mut().filter(|o|current && same_owned(&o.expected,before,bank.retained)
        && o.live.target.saved().is_none()) {
        Some(owner)=>owner,
        None=>{drop(state);drop(lock);return Err(BankRefusal::Kept("CPU bank ticket/owner changed before settlement"));}
    };
    let mut registry=match final_registry(helpers,Some(bank.token)) {
        Ok(registry)=>registry,
        Err(reason)=>{drop(state);drop(lock);return Err(BankRefusal::Kept(reason));}
    };
    owner.live=bank.destination;
    if regrouped {
        // Adopt the swapped group's physical receipt exactly as a Reload completion does.
        owner.expected=now;
        registry.commit(now.authority(),now.original_kind,super::descriptor_kind(&now.descriptor));
        super::note("[GroupSwap] adapter rebaseline CPU descriptor/header/body proof passed");
    }
    state.bank=None;
    state.replay_result=bank.replay.filter(|r|r.current(super::epoch(),super::super::mii_replay::revision(r.source)))
        .map(|selection|crate::engine::mii::replay::SelectionReceipt {selection,outcome:Ok(())});
    drop(registry);drop(state);drop(lock);
    Ok(Settled {now,exact,resource_pair})
}

/// Reuse the native before/after proof while the generic roster still holds INSTALL.
pub(super) unsafe fn generic_completed(g:&super::Generic,kind:u32,cursor:Option<usize>)
    ->Result<Snapshot,&'static str> {
    // No origin is loaded here. shared::completed discards original_index;
    // this generic path does not publish a hat row.
    let completion=shared::completed(shared::PublicationTarget {
        before:&g.before,descriptor:&g.descriptor,payload:None,scalars:g.before.scalars,source:None,
        hat_change:false,original_index:0,
    })?;
    let now=completion.now;
    // Explicit bank receipt: an owned ready bank whose physical group is exactly this
    // descriptor publishes its proven live member. Every other receipt keeps +0x18.
    let published=super::bank_live(g.before.binding.entry as *const u8,g.before.binding.owner,&now.descriptor)
        .filter(|live|*live==kind).unwrap_or_else(||super::descriptor_kind(&now.descriptor));
    let mut state=super::STATE.lock().unwrap();
    let transaction_busy=state.departed || state.busy() || state.request.is_some()
        || super::epoch()!=g.before.binding.epoch;
    let reservation_held=state.reservation.held();
    let owner=state.owner.as_mut().filter(|o|o.expected==g.expected && o.live==g.live)
        .ok_or("generic CPU owner changed during INSTALL")?;
    let live_origin=policy::with_native_live_metadata(owner.origin.descriptor,&now.descriptor)?;
    let destination=policy::generic_ordinary_commit_in(now.layout,owner.origin.index,policy::GenericOrdinaryFacts {
        reservation_held,transaction_busy,origin_is_saved:owner.origin_saved.is_some(),
        live_is_saved:owner.live.target.saved().is_some(),kind,origin_kind:owner.origin.original_kind,
        published_kind:published,index:now.index,
        descriptor_equals_origin:now.descriptor==live_origin,roster_cursor:cursor,
    })?;
    let registry=final_registry(now.helpers,Some(g.token))?;
    owner.expected=now;owner.live=destination;
    drop(registry);drop(state);drop(completion);Ok(now)
}
/// Diagnostic only, after a refused completion: the fresh snapshot (or its refusal reason) plus
/// raw per-owner index mirrors and CPU O/D/task words that the snapshot does not carry.
/// Read-only; grants no authority and changes no decision or native state.
pub(super) struct Failure {
    pub now: Result<Snapshot,&'static str>,
    /// Per owner: E+1a8, FE+46, D+72, O+68.
    pub mirrors: [Option<[u16;4]>;8],
    /// Per owner: FE+18 kind, E+8 row kind.
    pub kinds: [Option<(u32,i8)>;8],
    pub fe_tokens: Option<[u16;5]>, pub d_tokens: Option<[u16;4]>,
    pub o_init: Option<u8>, pub o_words: Option<[u32;10]>,
    pub task: Option<usize>, pub outstanding: Option<u32>,
}
pub(super) unsafe fn completion_failure(track:&super::Track)->Option<Failure> {
    let m=map(track.ticket.entry).ok()?;let b=pool(track.ticket.entry,track.ticket.epoch).ok()?;
    let base=super::base();
    let helpers=helper_snapshot().ok()?;
    let g=PoolLock::acquire(base,b.manager);
    let mut f=Failure {now:current_locked(m,b,track.before.original_kind,None,helpers),
        mirrors:[None;8],kinds:[None;8],fe_tokens:None,d_tokens:None,
        o_init:None,o_words:None,task:None,outstanding:None};
    let table=pointer(base,OWNER_TABLE,0x130,false).ok();
    for (id,fe) in m.physical().iter().copied().enumerate() {
        let e=base+0x52c5730+id*0x260;
        if fe==0 || !aligned(fe) || !span(fe,0x5a88,false) || !span(e,0x260,false) {continue;}
        let (Ok(d),Some(o))=(pointer(fe,0xf8,0x3a0,false),
            table.and_then(|t|pointer(t,0xe8+id*8,0xb0,false).ok())) else {continue;};
        f.mirrors[id]=Some([rd(e,0x1a8),rd(fe,0x46),rd(d,0x72),rd(o,0x68)]);
        f.kinds[id]=Some((rd(fe,0x18),rd(e,8)));
        if id==b.owner as usize {
            f.fe_tokens=Some(rd(fe,0x46));f.d_tokens=Some(rd(d,0x72));
            f.o_init=Some(rd(o,0x4c));f.o_words=Some(rd(o,0x54));
            let task:usize=rd(o,0xa8);f.task=Some(task);
            if task!=0 && aligned(task) && span(task,0x14,false) {
                f.outstanding=Some(crate::engine::seams::native_fields::load_acquire_u32(task + 0x10));
            }
        }
    }
    drop(g);Some(f)
}

/// Saved CPU capture retains original helper tokens/CharInfo, even with certified ordinary FE kinds.
/// Later tickets retain their full protected owner/record union byte comparisons.
pub(super) unsafe fn capture_origin(entry:usize,epoch:u64,kind:u32,catalog:&Catalog,opening:u64)
    ->Result<(),helpers::CaptureError> {
    let previous={let state=super::STATE.lock().unwrap();
        state.owner.as_ref().map(|o|(o.expected,o.live,state.reservation,state.request,state.reservation.held()
            || o.live.target.publish(o.origin_saved).is_some()))};
    if let Some((expected,live,reservation,request,retained))=previous {
        let m=map(entry)?;let b=pool(entry,epoch)?;
        let helpers=helper_snapshot()?;
        let g=PoolLock::acquire(super::base(),b.manager);
        let now=current_locked(m,b,kind,None,helpers)?;
        let descriptor=match policy::recapture_rank(now.layout,expected.binding,now.binding,
            &expected.descriptor,&now.descriptor) {
            Ok(descriptor)=>descriptor,
            Err(reason)=>{
                // expected and now are owned copies. Release G before formatting,
                // the trace, or STATE. This branch does not read native memory again.
                drop(g);
                if reason=="CPU rank recapture changed non-rank descriptor bytes" {
                    super::note_non_rank_recapture(&expected,&now);
                }
                return Err(reason.into());
            }
        };
        // Only the CPU's live behavior bytes FE+0x34..=0x37 differ in this local
        // comparison value. All other same_owned checks stay exact.
        let comparison=Snapshot {descriptor,..expected};
        check(same_owned(&now,&comparison,retained),"menu current owner changed")?;
        check(current_locked(m,b,kind,None,helpers)?==now,"CPU menu snapshot changed before rank refresh")?;
        let mut state=super::STATE.lock().unwrap();
        check(super::super::mii_saved_alpha::menu_epoch()==opening
            && state.capture_at==(epoch,opening) && state.capture && !state.departed
            && !state.busy() && state.generic.is_none() && state.request==request
            && state.reservation==reservation && super::epoch()==epoch
            && state.owner.as_ref().is_some_and(|o|o.expected==expected && o.live==live),"menu owner retired")?;
        let registry=final_registry(now.helpers,None)?;
        // No whole-snapshot adoption: origin and historical foreign-owner values
        // remain unchanged. Later tickets freeze a fresh, fully qualified union.
        state.owner.as_mut().unwrap().expected.descriptor=descriptor;
        state.capture=false;state.menu_valid=true;
        drop(registry);drop(state);drop(g);return Ok(());
    }
    check(catalog.is_complete(),"CPU origin catalog incomplete")?;
    let before=snapshot(entry,epoch,kind)?;
    policy::native_rank(&before.descriptor)?;
    let c=if MiiClass::from_kind(kind).is_some() {
        check(policy::supported_kind(kind),"CPU origin class not qualified")?;
        let m=map(entry)?;let b=pool(entry,epoch)?;
        let helpers=helper_snapshot()?;
        let g=PoolLock::acquire(super::base(),b.manager);
        let fresh=current_locked(m,b,kind,None,helpers)?;
        check(fresh==before,"CPU origin changed before catalog resolution")?;
        let observed=mirrors(fresh.owners[b.owner as usize].unwrap());
        let saved=role_owner::saved_origin(catalog,kind,observed,&fresh.current_record);
        drop(g);
        let c=match saved {
            Some(c)=>c,
            None if fresh.layout.waiting()=>{
                // Native waiting CPUs may use a built-in Mii absent from the saved catalog.
                // Copy its actual values; do not synthesize a preset or write a saved file.
                // Native getters run outside G. The common second snapshot below must still
                // exactly match this source before its origin can be committed.
                let tokens=observed.tokens;
                let rows=values::RowIds {
                    outfit:lookup_row_id(super::base(),OUTFIT_TABLE,tokens.outfit)?,
                    hat:lookup_row_id(super::base(),HAT_TABLE,tokens.hat)?,
                };
                let mut charinfo=[0;88];
                charinfo.copy_from_slice(&fresh.current_record[..88]);
                Configuration::from_observed(MiiClass::from_kind(kind).unwrap(),tokens,charinfo,rows)
                    .map_err(|_|"native CPU origin values or row identity invalid")?
            },
            None=>return Err(helpers::CaptureError::Refused("no exact original saved CPU configuration")),
        };
        Some(c)
    } else {check(before.layout.waiting() || before.index==0,"ordinary CPU origin index changed")?;None};
    let rows=c.map(|c|super::super::mii_ordinary_player::native::target_rows(c)).transpose()?;
    let m=map(entry)?;let b=pool(entry,epoch)?;
    let helpers=helper_snapshot()?;
    let g=PoolLock::acquire(super::base(),b.manager);
    let now=current_locked(m,b,kind,None,helpers)?;
    check(now==before && now.thread!=0,"CPU origin changed across row validation")?;
    if let (Some(c),Some(rows))=(c,rows) {
        role_owner::validate_saved(c,now.index,mirrors(now.owners[b.owner as usize].unwrap()),rows,&now.current_record)?;
        // Training's helpers are saved-origin copies. LDN's independently indexed
        // current records are qualified/protected, never assumed to copy the CPU.
        for id in now.layout.helpers().iter().copied().map(|id|id as usize).filter(|_|now.layout==super::super::mii_backend::Layout::Training) {
            let owner=now.owners[id].ok_or("CPU origin helper absent")?;
            let record=now.records.iter().flatten().find(|(index,_)|*index==id as u16)
                .ok_or("CPU origin helper record absent")?;
            let original_kind=match now.helpers.slots[now.layout.helper(id).unwrap()] {
                helpers::Helper::Ready {proof,..}=>i64::from(proof.original.kind),
                helpers::Helper::Unmanaged=>i64::from(rd::<u32>(owner.entry,0x18)),
                helpers::Helper::Unavailable {..}=>return Err(helpers::CaptureError::Refused("CPU helper authority unavailable")),
            };
            check(original_kind==i64::from(kind),"CPU original helper class differs")?;
            role_owner::validate_saved(c,id as u16,mirrors(owner),rows,&record.1)?;
        }
    }
    let mut state=super::STATE.lock().unwrap();
    check(super::super::mii_saved_alpha::menu_epoch()==opening
        && state.capture && state.owner.is_none() && !state.busy() && !state.departed
        && super::epoch()==epoch,"CPU origin capture retired")?;
    let mut registry=final_registry(helpers,None)?;
    state.owner=Some(super::Owner{origin:now,origin_saved:c,expected:now,
        live:super::Destination{target:super::Target::ORIGINAL,cursor:None}});
    state.capture=false;state.menu_valid=true;
    registry.commit(now.authority(),kind,kind);
    drop(registry);drop(state);drop(g);
    super::super::observe_mii_origin(super::super::Role::Cpu,epoch,kind,super::descriptor_forms(&now.descriptor),c);
    Ok(())
}
