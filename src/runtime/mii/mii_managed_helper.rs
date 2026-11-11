//! Helper2/3 adapter around shared selector transactions. SourceBook owns all leases/lifetimes.
use super::mii_ordinary_player::native::{self as provenance, check, helpers, Scalars};
use super::mii_selector_native::{self as native, PublicationTarget, Snapshot};
use super::{mii_saved_alpha, native_fighter_install, native_source, InstallToken, BASE, INSTALL};
use crate::engine::swap::source_lifetime::{LoadToken, SourceToken, SceneEpoch};
use std::sync::atomic::Ordering;
use hos_lock::Mutex;
use crate::model::reusable_snapshot::{Storage, Snapshot as SharedSnapshot};
use crate::engine::mii::{
    inventory::MiiClass,
    lifetime::Configuration,
    ordinary_cpu as policy,
    ordinary_player::{Gate, Phase, Ticket},
    selection::{self, PlayerOrigin},
};
struct State {
    storage: Option<Storage<Prepared>>,
    track: Option<Track>,
    next: u64,
    capture_wait_noted: bool,
}
static STATE: Mutex<State> = Mutex::new(State {
    storage: None,
    track: None,
    next: 1,
    capture_wait_noted: false,
});
#[derive(Clone)]
struct Track {
    ticket: Ticket,
    gate: Gate,
    source: SourceToken,
    install: InstallToken,
    data: SharedSnapshot<Prepared>,
}
pub(super) struct Prepared {
    before: Snapshot,
    descriptor: [u8; 0x70],
    configuration: Configuration,
    player_epoch: u64,
    created_epoch: Option<SceneEpoch>,
    hat_change: bool,
}
impl Prepared {
    pub(super) fn created_destination(&self)->Option<policy::CreatedDestination> {self.before.created}
    pub(super) fn descriptors(&self)->([u8;0x70],[u8;0x70]) {(self.before.descriptor,self.descriptor)}
    /// Original row captured under G before the SourceBook replacement begins.
    pub(super) fn original(&self)->helpers::OriginalRow {
        let owner=self.before.owners[self.before.binding.owner as usize].unwrap();
        helpers::OriginalRow {row:owner.row,kind:self.before.original_kind as i8,index:owner.index,
            selector:owner.selector,control:self.before.control}
    }
}
/// Called once at boot, before native hooks can admit a helper.
pub(super) fn init_storage() {
    let mut state = STATE.lock();
    if state.storage.is_none() { state.storage = Some(Storage::new()); }
}
pub(super) fn origins_captured() -> bool {
    super::mii_ordinary_player::helper_origin_captured()
        && super::mii_ordinary_cpu::helper_origin_captured()
}
/// Ordinary helpers only need CPU capture when its original Mii copies must
/// survive. Actual Mii transactions still require both qualified role captures.
pub(super) fn admission_ready(mii_request: bool) -> bool {
    let original_cpu = super::original_kind(super::Role::Cpu);
    if policy::helper_captures_ready(mii_request, original_cpu, false, false) {
        return true;
    }
    let player = mii_request && super::mii_ordinary_player::helper_origin_captured();
    let cpu = super::mii_ordinary_cpu::helper_origin_captured();
    let ready = policy::helper_captures_ready(mii_request, original_cpu, player, cpu);
    let mut s = STATE.lock();
    let report = !ready && !s.capture_wait_noted;
    s.capture_wait_noted = !ready;
    drop(s);
    if report {
        crate::engine::compat::logging::diagnostic(format_args!(
            "[MiiHelper] waiting for original capture mii_request={} original_cpu={:?} player={} cpu={}",
            mii_request, original_cpu, player, cpu));
    }
    ready
}
/// Once per queued request; no preference/cursor write and no native/file scan.
pub(super) fn configuration(role: usize, kind: u32) -> Result<Configuration, &'static str> {
    let class = MiiClass::from_kind(kind).ok_or("Mii source class absent")?;
    check(
        role < 2 && origins_captured(),
        "Wait for both original characters to be captured",
    )?;
    let catalog =
        super::mii_ordinary_cpu::replay_catalog().ok_or("Mii source catalog is not qualified")?;
    let role = if role == 0 {
        selection::Role::Player
    } else {
        selection::Role::Cpu
    };
    let choice = selection::resolve(
        &catalog,
        &super::menu::mii_preferences(),
        role,
        class,
        PlayerOrigin::NotMii,
    )
    .ok_or("No saved Mii configuration is available for this hat")?;
    let c = catalog
        .configuration(choice)
        .ok_or("Saved Mii source configuration is unavailable")?;
    let hats = super::mii_ordinary_cpu::helper_origin_hats()
        .ok_or("Captured helper hat tokens are unavailable")?;
    policy::helper_configuration(c, hats)
}
/// Captured origin hats are the before-value. Shared reload policy admits an
/// equal hat on every layout and a changed hat on Training. The live row,
/// fingerprints, and lease are still rechecked by prepare().
pub(super) fn slot_eligible(slot: u32, c: Configuration) -> bool {
    let Some(layout) = super::mii_backend::current() else { return false };
    let Some(hats) = super::mii_ordinary_cpu::helper_origin_hats() else { return false };
    policy::helper_slot_admission(layout, slot as usize, hats, c.tokens().hat)
}
/// Scheduling hint only. prepare_created still proves every native root,
/// record claim, outgoing resource and exact source receipt before INSTALL.
pub(super) fn created_slot_eligible(slot:u32,c:Configuration)->bool {
    crate::engine::preload::cycle_preload_policy::created_mii_sources()
        && origins_captured() && super::mii_backend::current()==Some(super::mii_backend::Layout::Training)
        && super::mii_backend::extras().entry(slot as usize).is_some()
        && policy::supported_kind(c.canonical().class as u32) && c.tokens().hat!=u16::MAX
}
pub(super) unsafe fn prepare(
    entry: usize,
    slot: u32,
    c: Configuration,
) -> Result<Prepared, &'static str> {
    prepare_in(entry,slot,c,None)
}
/// Explicit created-source entry point. Caller supplies its current SourceBook
/// epoch; bind checks the eventual full source token under Source STATE.
pub(super) unsafe fn prepare_created(entry:usize,slot:u32,c:Configuration,epoch:SceneEpoch)
    ->Result<Prepared,&'static str> {
    check(crate::engine::preload::cycle_preload_policy::created_mii_sources(),
        "Created exact-Mii sources are deferred in this RC")?;
    prepare_in(entry,slot,c,Some(epoch))
}
unsafe fn prepare_in(entry:usize,slot:u32,c:Configuration,created_epoch:Option<SceneEpoch>)
    ->Result<Prepared,&'static str> {
    check(
        super::mii_backend::current().is_some_and(|l| if created_epoch.is_some() {
            l==super::mii_backend::Layout::Training && super::mii_backend::extras().entry(slot as usize)==Some(entry)
        } else {l.helper(slot as usize).is_some()}) && origins_captured(),
        "helper original capture/slot unavailable",
    )?;
    check(
        {
            let state = STATE.lock();
            state.track.is_none() && state.storage.as_ref().is_some_and(Storage::available)
        },
        "helper transaction still retained",
    )?;
    native::target_rows(c)?;
    let row = BASE.load(Ordering::Acquire) + 0x52c5730 + slot as usize * 0x260;
    check(
        provenance::span(row, 0x260, false),
        "helper original row span",
    )?;
    let original_kind=crate::runtime::mii::mii_native_access::rd::<i8>(row,8) as u32;
    let before=if created_epoch.is_some() {
        native::snapshot_created(entry,original_kind,slot as usize)?
    } else {native::snapshot(entry,mii_saved_alpha::current_generation(1),original_kind,slot as usize,None)?};
    if !policy::supported_kind(c.canonical().class as u32) {
        return Err("Mii class not qualified");
    }
    // The helper's own physical slot. Training is 2/3; waiting is 5/6.
    let hat_change=match before.created {
        Some(destination)=>destination.headgear_reload(before.scalars.e.hat,c.tokens().hat)?,
        None=>policy::headgear_reload(before.layout,slot as usize,before.scalars.e.hat,c.tokens().hat)?,
    };
    if hat_change && !provenance::cpu_headgear_fingerprints() {
        return Err("helper headgear fingerprint mismatch");
    }
    let descriptor=match before.created {
        Some(destination)=>destination.staged(before.descriptor,c),
        None=>policy::staged_descriptor_in(before.layout,slot as usize,before.descriptor,Some(c))
            .ok_or("helper selector unavailable")?,
    };
    Ok(Prepared {
        before,
        descriptor,
        configuration: c,
        player_epoch: if created_epoch.is_some() {0} else {mii_saved_alpha::current_generation(0)},
        created_epoch,
        hat_change,
    })
}
/// Source STATE held, no G/registry lock. Failure retains the load and never calls native INSTALL.
pub(super) fn bind(
    mut data: Prepared,
    load: LoadToken,
    install: InstallToken,
) -> Result<(), &'static str> {
    let source = load.source();
    let mut s = STATE.lock();
    check(
        s.track.is_none()
            && INSTALL.owns(install)
            && source.slot() == data.before.binding.owner as usize
            && source.entry().0 == data.before.binding.entry as u64,
        "helper transaction binding changed",
    )?;
    if let Some(epoch)=data.created_epoch {
        check(source.epoch()==epoch && data.before.created.is_some(),"created source epoch changed")?;
        // Use the existing transaction sequence for the inert binding stamp.
        // Full source token equality, not this number, owns the created lifetime.
        data.before.binding.epoch=s.next;
    } else {
        check(data.before.binding.epoch==mii_saved_alpha::current_generation(1)
            && data.player_epoch==mii_saved_alpha::current_generation(0),"helper original generation changed")?;
    }
    let view = provenance::helper_snapshot_for(Some(source))?;
    for i in 0..2 {
        if Some(i) != view.ordinal(source.slot()) {
            check(
                view.slots[i] == data.before.helpers.slots[i]
                    && view.miis[i] == data.before.helpers.miis[i],
                "foreign source projection changed",
            )?;
        }
    }
    for i in 0..view.created_slots.len() {
        if Some(i)!=view.created_ordinal(source.slot()) {
            check(view.created_slots[i]==data.before.helpers.created_slots[i]
                && view.created_miis[i]==data.before.helpers.created_miis[i],"foreign created source projection changed")?;
        }
    }
    data.before.helpers = view;
    let ticket = Ticket {
        epoch: data.before.binding.epoch,
        entry: data.before.binding.entry,
        sequence: s.next,
    };
    let next = s.next.checked_add(1).ok_or("helper ticket exhausted")?;
    let gate = Gate::new(ticket).ok_or("helper ticket invalid")?;
    let data = s.storage.as_mut().ok_or("helper storage not initialized")?
        .publish(data).map_err(|_| "helper snapshot reader still retained")?;
    s.track = Some(Track {
        ticket,
        gate,
        source,
        install,
        data,
    });
    s.next = next;
    Ok(())
}
fn current(t: &Track) -> bool {
    INSTALL.owns(t.install) && match t.data.created_epoch {
        Some(epoch)=>t.source.epoch()==epoch && t.data.before.created.is_some()
            && provenance::check_helpers_for(&t.data.before.helpers,Some(t.source)).is_ok(),
        None=>t.ticket.epoch==mii_saved_alpha::current_generation(1)
            && t.data.player_epoch==mii_saved_alpha::current_generation(0),
    }
}
fn target(t: &Track) -> PublicationTarget<'_> {
    PublicationTarget {
        before: &t.data.before,
        descriptor: &t.data.descriptor,
        payload: Some(t.data.configuration),
        scalars: Scalars::saved(t.data.configuration),
        source: Some(t.source),
        hat_change: t.data.hat_change,
        // Saved helper payload always targets the slot record. This publisher
        // does not restore; the pre-request mirror is not the write address.
        original_index: t.data.before.index,
    }
}
pub(super) unsafe fn install(source: SourceToken) -> bool {
    let descriptor = {
        let s = STATE.lock();
        let Some(t) = s
            .track
            .as_ref()
            .filter(|t| t.source == source && current(t))
        else {
            return false;
        };
        t.data.descriptor
    };
    let staged = native_fighter_install::Descriptor::from_bytes(&descriptor);
    native_fighter_install::install(
        BASE.load(Ordering::Acquire),
        source.entry().0 as *mut u8,
        &staged,
        native_fighter_install::Flags::Parked,
    );
    true
}
pub(super) fn tracked(entry: usize) -> bool {
    STATE
        .lock()
        .track
        .as_ref()
        .is_some_and(|t| t.ticket.entry == entry)
}
pub(super) unsafe fn enter_native(entry: *mut u8) -> bool {
    let mut s = STATE.lock();
    let Some(t) = s
        .track
        .as_mut()
        .filter(|t| t.ticket.entry == entry as usize)
    else {
        return true;
    };
    if !current(t) {
        t.gate.refuse();
        return false;
    }
    t.gate.enter(
        t.ticket,
        crate::runtime::mii::mii_native_access::rd(entry as usize, 0x591c),
    )
}
pub(super) unsafe fn leave_native(entry: *mut u8) {
    let mut s = STATE.lock();
    if let Some(t) = s
        .track
        .as_mut()
        .filter(|t| t.ticket.entry == entry as usize)
    {
        t.gate.leave(
            t.ticket,
            crate::runtime::mii::mii_native_access::rd(entry as usize, 0x591c),
        );
    }
}
pub(super) fn refuse(entry: usize, reason: &str) {
    let mut s = STATE.lock();
    if let Some(t) = s.track.as_mut().filter(|t| t.ticket.entry == entry) {
        t.gate.refuse();
        drop(s);
        crate::engine::compat::logging::diagnostic(format_args!(
            "[MiiHelper] refused entry={:#x} reason={}",
            entry, reason
        ));
    }
}
pub(super) unsafe fn prequeue(entry: usize, selected: usize) -> bool {
    let track = {
        let s = STATE.lock();
        let Some(t) = s.track.as_ref().filter(|t| t.ticket.entry == entry) else {
            return false;
        };
        t.clone()
    };
    let mut evidence = None;
    let result = (|| {
        // The gate's configure-skip writer (U5) runs for the ordinary CPU/Player tracks only.
        let publication = native::publication(target(&track), selected, &mut evidence,
            crate::engine::mii::identity::ExpectedMutation::None)?;
        let mut s = STATE.lock();
        let t = s.track.as_mut().ok_or("helper ticket retired")?;
        check(
            t.source == track.source
                && t.ticket == track.ticket
                && current(t)
                && t.gate.may_publish(t.ticket, 6),
            "helper final ticket changed",
        )?;
        let registry = provenance::final_registry_for(
            &publication.observed.before.helpers,
            Some(t.install),
            Some(t.source),
        )?;
        publication.write();
        let published = t.gate.publish(t.ticket, 6);
        debug_assert!(published);
        drop(registry);
        drop(s);
        drop(publication);
        Ok::<_, &'static str>(())
    })();
    if let Some(e) = evidence {
        crate::engine::compat::logging::diagnostic(format_args!("[MiiHelper] prequeue source={:?} selected={:#x} init={} header={:x?} index={} tokens={:x?} task={:#x} outstanding={:?} Dindex={} rebuild={} before={:?}",
        track.source,e.selected,e.initialized,e.header,e.o_index,e.o_tokens,e.task,e.outstanding,e.d_index,e.rebuild,e.before));
    }
    match result {
        Ok(()) => true,
        Err(reason) => {
            refuse(entry, reason);
            false
        }
    }
}
/// Terminal transactions skip further SourceBook readiness/G reads; INSTALL
/// remains owned until the actual destructor. Missing tracks cover a refused bind.
pub(super) fn retained_terminal(source: SourceToken) -> bool {
    let s = STATE.lock();
    s.track
        .as_ref()
        .filter(|t| t.source == source)
        .is_none_or(|t| matches!(t.gate.phase(), Phase::Refused | Phase::Departed))
}
pub(super) fn refuse_completion(source: SourceToken, reason: &str) {
    let mut s = STATE.lock();
    let Some(t) = s.track.as_mut().filter(|t| t.source == source) else {
        return;
    };
    if matches!(t.gate.phase(), Phase::Refused | Phase::Departed) {
        return;
    }
    t.gate.refuse();
    drop(s);
    crate::engine::compat::logging::diagnostic(format_args!(
        "[MiiHelper] completion refused; INSTALL retained source={:?} reason={}",
        source, reason
    ));
}
/// INSTALL stays held until SourceBook's Ready commit, so no writer can intervene.
pub(super) struct Completion {
    pub mii:helpers::MiiProof,
    pub proof:helpers::Proof,
    pub created:Option<helpers::CreatedReceipt>,
}
pub(super) unsafe fn completed_details(
    source: SourceToken,
    ticks: u16,
) -> Result<Option<Completion>, &'static str> {
    let track = {
        let s = STATE.lock();
        let t = s.track.as_ref().ok_or("helper ticket absent")?;
        check(
            t.source == source && current(t) && t.gate.can_complete(t.ticket),
            "helper not published/current",
        )?;
        t.clone()
    };
    let kind=track.data.configuration.canonical().class as u32;
    let ready=match track.data.before.created {
        Some(destination)=>native_source::parked_created_ready(source.entry().0 as *const u8,
            source.slot() as u32,kind,destination),
        None=>native_source::parked_ready(source.entry().0 as *const u8,source.slot() as u32,kind),
    };
    if !ready {
        return Ok(None);
    }
    native::target_rows(track.data.configuration)?;
    let completion = match native::completed(target(&track)) {
        // Only this native job-counter lag is asynchronous. Returning None
        // follows service_owned's normal tick commit, including tick 899.
        Err("ready task outstanding") if ticks < 900 => return Ok(None),
        result => result?,
    };
    let s = STATE.lock();
    let t = s.track.as_ref().ok_or("helper completion retired")?;
    check(
        t.source == source
            && t.ticket == track.ticket
            && current(t)
            && t.gate.can_complete(t.ticket),
        "helper completion token changed",
    )?;
    let registry =
        provenance::final_registry_for(&completion.now.helpers, Some(t.install), Some(source))?;
    let proof = helpers::MiiProof {
        binding: completion.now.binding,
        configuration: track.data.configuration,
        descriptor: completion.now.descriptor,
        record: completion.now.record,
    };
    let owner=completion.now.owners[source.slot()].ok_or("completed source owner absent")?;
    let native_proof=helpers::Proof {
        original:helpers::OriginalRow {row:owner.row,kind:completion.now.original_kind as i8,
            index:owner.index,selector:owner.selector,control:completion.now.control},
        thread:completion.now.thread,owner,kind,
        costume:crate::runtime::mii::mii_native_access::rd(owner.object,0x64),
    };
    let created=completion.now.created.map(|destination|helpers::CreatedReceipt {source,destination,mii:proof});
    drop(registry);
    drop(s);
    drop(completion);
    Ok(Some(Completion {mii:proof,proof:native_proof,created}))
}
pub(super) fn release(source: SourceToken) {
    let mut s = STATE.lock();
    if s.track.as_ref().is_some_and(|t| t.source == source) {
        s.track = None;
    }
}
pub(super) fn depart() {
    let mut s = STATE.lock();
    s.capture_wait_noted = false;
    if let Some(t) = s.track.as_mut() {
        t.gate.depart();
    }
}

/// Recheck exact committed payload/resource identity before SourceBook grants a lease.
pub(super) unsafe fn ready(ready: crate::engine::swap::source_lifetime::ReadyToken, c: Configuration) -> bool {
    ready_checked(ready, c).is_ok()
}

/// Copied capacity preflight facts only after the current source and full native
/// G payload have been revalidated. The caller rechecks its ReadyToken before
/// replacing the source; this snapshot is not a second lifetime authority.
pub(super) unsafe fn created_ready_receipt(
    ready: crate::engine::swap::source_lifetime::ReadyToken, c: Configuration,
) -> Result<helpers::CreatedReceipt, &'static str> {
    ready_checked(ready, c)?;
    let view = provenance::helper_snapshot()?;
    let i = view.created_ordinal(ready.source().slot()).ok_or("created backend differs")?;
    let receipt = view.created_miis[i].ok_or("created receipt absent")?;
    check(receipt.source == ready.source() && receipt.mii.configuration == c
        && matches!(view.created_slots[i], helpers::Helper::Ready {ready: current, ..} if current == ready),
        "created preflight source changed")?;
    Ok(receipt)
}

/// F9 (Eden covrest-11, 2026-11-08): a helper seat the game activated as CPU 2 or 3 and parked again keeps its
/// identity, but its Mii record differs from the recorded proof, so every Mii owner snapshot refused the row. Once
/// the seat is parked ready, its Mii proof is taken again from the row, only when the identity is unchanged: the
/// same ready source and configuration, the same binding, owner objects and scalars, and a record that still carries
/// the configuration's charinfo and a resource pair (`parked_entries::reproof`). `Ok(None)`: not parked yet;
/// `Ok(Some((descriptor_changed, record_changed)))`: the proof matches the row now.
pub(super) unsafe fn reproof_parked(
    ready: crate::engine::swap::source_lifetime::ReadyToken,
    c: Configuration,
) -> Result<Option<(bool, bool)>, &'static str> {
    use crate::model::parked_entries::Reproof;
    let source = ready.source();
    let view = provenance::helper_snapshot()?;
    let ordinal = view.ordinal(source.slot()).ok_or("helper backend differs")?;
    let Some(helpers::Helper::Ready { ready: current, proof, .. }) = view.slots.get(ordinal).copied() else {
        return Err("helper source is not ready");
    };
    let mii = view.miis[ordinal].ok_or("helper exact payload is absent")?;
    let kind = c.canonical().class as u32;
    let parked = native_source::parked_ready(source.entry().0 as *const u8, source.slot() as u32, kind);
    if crate::model::parked_entries::reproof(parked, true, true) == Reproof::Wait {
        return Ok(None);
    }
    let m = native::map(source.entry().0 as usize, source.slot())?;
    let b = native::pool(source.entry().0 as usize, mii.binding.epoch, source.slot())?;
    let g = crate::runtime::mii::mii_native_access::PoolLock::acquire(BASE.load(Ordering::Acquire), b.manager);
    let now = native::current_locked(m, b, proof.original.kind as u32, None, view, None);
    drop(g);
    let now = now?;
    let identity = current == ready
        && mii.configuration == c
        && now.binding == mii.binding
        && now.owners[source.slot()] == Some(proof.owner)
        && now.scalars == Scalars::saved(c)
        && crate::model::parked_entries::mii_record_identity(&now.record, c.charinfo());
    let changed = (now.descriptor != mii.descriptor, now.record != mii.record);
    match crate::model::parked_entries::reproof(parked, identity, changed == (false, false)) {
        Reproof::Wait => Ok(None),
        Reproof::Unchanged => Ok(Some(changed)),
        Reproof::Refuse => Err("promoted seat identity changed"),
        Reproof::Rerecord => {
            let mut registry = provenance::registry();
            check(
                registry.helpers.slots.get(ordinal).copied() == view.slots.get(ordinal).copied()
                    && registry.helpers.miis[ordinal] == Some(mii),
                "helper proof changed during the re-record",
            )?;
            registry.helpers.miis[ordinal] =
                Some(helpers::MiiProof { descriptor: now.descriptor, record: now.record, ..mii });
            Ok(Some(changed))
        }
    }
}

/// Preserve a transient shared-installer refusal for consumers that already hold
/// a lease. Another helper loading does not invalidate this helper's payload.
/// Callers must retry BUSY without releasing the lease; every other failure is
/// still a failed proof. No native read proceeds past an unavailable projection.
pub(super) unsafe fn ready_checked(
    ready: crate::engine::swap::source_lifetime::ReadyToken,
    c: Configuration,
) -> Result<(), &'static str> {
    let source = ready.source();
    let view = provenance::helper_snapshot()?;
    if let Some(i)=view.created_ordinal(source.slot()) {
        let helpers::Helper::Ready {ready:current,proof,..}=view.created_slots[i]
            else {return Err("created source is not ready");};
        let receipt=view.created_miis[i].ok_or("created exact payload absent")?;
        check(current==ready && receipt.source==source && receipt.mii.configuration==c,
            "created exact source/payload changed")?;
        let guard=native::created_ready_snapshot(receipt.mii.binding,proof.original.kind as u32,
            receipt.destination,view)?;
        let now=&guard.now;
        check(now.record==receipt.mii.record && now.descriptor==receipt.mii.descriptor
            && now.scalars==Scalars::saved(c) && now.owners[source.slot()]==Some(proof.owner),
            "created exact configuration changed")?;
        let table=provenance::pointer(BASE.load(Ordering::Acquire),
            crate::runtime::mii::mii_native_access::OWNER_TABLE,0x130,false)?;
        provenance::created_kind(&view,now.extras,source.slot(),proof.owner,
            c.canonical().class as u32,super::mii_backend::extra_facts(table,source.slot(),
                proof.owner.entry,proof.owner.row,proof.owner.information,proof.owner.object))?;
        return Ok(());
    }
    let ordinal = view.ordinal(source.slot()).ok_or("helper backend differs")?;
    let Some(helpers::Helper::Ready {
        ready: current,
        proof,
        ..
    }) = view.slots.get(ordinal).copied()
    else {
        return Err("helper source is not ready");
    };
    let Some(mii) = view.miis[ordinal] else {
        return Err("helper exact payload is absent");
    };
    if current != ready || mii.configuration != c {
        return Err("helper exact payload identity changed");
    }
    let m = native::map(source.entry().0 as usize, source.slot())?;
    let b = native::pool(source.entry().0 as usize, mii.binding.epoch, source.slot())?;
    let g = crate::runtime::mii::mii_native_access::PoolLock::acquire(BASE.load(Ordering::Acquire), b.manager);
    let result = native::current_locked(m, b, proof.original.kind as u32, None, view, None)
        .and_then(|now| check(
            now.binding == mii.binding
                && now.record == mii.record
                && now.descriptor == mii.descriptor
                && now.scalars == Scalars::saved(c)
                && now.owners[source.slot()] == Some(proof.owner)
                && provenance::helper_kind(
                    &view,
                    source.slot(),
                    proof.owner,
                    c.canonical().class as u32,
                )
            , "helper configuration not exact"));
    drop(g);
    result
}
