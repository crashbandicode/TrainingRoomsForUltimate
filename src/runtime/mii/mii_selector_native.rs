//! Shared selector1/2/3 transaction reads. No slot allocation or lifetime owner.
//! Callers retain the exact INSTALL/source ticket and commit under their own state lock.
use super::mii_ordinary_player::native::{
    authority, check, check_helpers_for, foreign_kind, helper_kind, helper_snapshot_for, helpers,
    pointer, span, Scalars,
};
use crate::runtime::mii::mii_native_access::*;
use crate::engine::swap::source_lifetime::SourceToken;
use std::convert::TryInto;
use crate::engine::mii::{
    identity::ExpectedMutation,
    lifetime::{Binding, Configuration},
    native_values as values, ordinary_cpu as policy,
    ordinary_player::Owner,
    role_owner::{self, Authority},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Protected {
    owner: Owner,
    descriptor: [u8; 0x70],
    e: [u8; 14],
    d: [u8; 8],
    specials: [u32; 4],
    o: [u8; 20],
    control: [u8; 7],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub layout: super::mii_backend::Layout,
    pub helpers: helpers::View,
    pub created: Option<policy::CreatedDestination>,
    pub extras: crate::engine::mii::capacity::Extras,
    pub binding: Binding,
    pub thread: usize,
    pub original_kind: u32,
    pub descriptor: [u8; 0x70],
    /// Outgoing O header `R+0x54..0x68`, read under the same lock as `descriptor`.
    /// A same-R group exchange leaves it at the original window's words while
    /// FE+0x18.. names the exchanged group (`GroupExchange::new`), so publication
    /// compares the selected O against this capture, never against the descriptor
    /// words (D-PTA1).
    pub header: [u32; 5],
    pub scalars: Scalars,
    pub index: u16,
    pub record: [u8; values::RECORD_LEN],
    pub current_record: [u8; values::RECORD_LEN],
    pub(crate) control: [u8; 7],
    pub(crate) symbol: usize,
    pub(crate) manager: usize,
    pub(crate) internal: usize,
    pub(crate) entries: [usize; 8],
    pub(crate) owners: [Option<Owner>; 8],
    pub(crate) protected: [Option<Protected>; 8],
    pub(crate) records: [Option<(u16, [u8; values::RECORD_LEN])>; 8],
}
impl Snapshot {
    #[cfg(feature = "mii-managed-helper-alpha")]
    pub(crate) fn helper_hats(&self) -> Option<[u16; 2]> {
        let hat = |id: usize| self.protected[id].map(|p| u16::from_le_bytes([p.e[4], p.e[5]]));
        let [a,b]=self.layout.helpers();Some([hat(a as usize)?, hat(b as usize)?])
    }
    pub(crate) fn authority(&self) -> Authority {
        authority(
            self.binding,
            self.thread,
            [self.symbol, self.manager, self.internal],
            self.entries,
            self.binding.owner,
            self.owners[self.binding.owner as usize].unwrap(),
        )
    }
}
/// `entries` is the preserved owner view: registered created entries are validated
/// against their exact identities and left out, so captures that predate their
/// creation still compare equal. `physical()` is the exact native map to recheck.
#[derive(Clone, Copy)]
pub(crate) struct Map {
    pub(crate) layout: super::mii_backend::Layout,
    pub(crate) symbol: usize,
    pub(crate) manager: usize,
    pub(crate) internal: usize,
    pub(crate) entries: [usize; 8],
    pub(crate) extras: crate::engine::mii::capacity::Extras,
    created: bool,
}
impl Map {
    pub(crate) fn physical(&self) -> [usize; 8] {
        self.extras.physical(&self.entries)
    }
}
/// Native setup's row/control sources; retained independently of the adapter-owned scalars.
unsafe fn control(row: usize) -> [u8; 7] {
    [0xa, 0x6b, 0x79, 0x78, 0x19, 0x1a, 0xa7].map(|offset| rd(row, offset))
}
pub(crate) unsafe fn map(entry: usize, owner: usize) -> Result<Map, &'static str> {
    map_in(entry,owner,false)
}
unsafe fn map_in(entry:usize,owner:usize,created:bool)->Result<Map,&'static str> {
    let layout=super::mii_backend::current().ok_or("Mii backend unavailable")?;
    let symbol = crate::runtime::compat::sync::read(&crate::runtime::compat::common::FIGHTER_MANAGER_ADDR);
    let manager = pointer(symbol, 0, 8, false)?;
    let internal = pointer(manager, 0, 0xa4, false)?;
    let physical = std::array::from_fn(|id| {
        smash::app::lua_bind::FighterManager::get_fighter_entry(
            manager as *mut smash::app::FighterManager,
            smash::app::FighterEntryID(id as i32),
        ) as usize
    });
    // The four layout owners plus exactly the registered created entries; an
    // extra is never a selector destination.
    let extras = super::mii_backend::extras();
    check(
        owner<8 && (if created {layout==super::mii_backend::Layout::Training && extras.entry(owner)==Some(entry)}
                else {layout.selector(owner) && extras.entry(owner).is_none()})
            && layout.shape_with(&physical, extras)
            && physical[owner] == entry
            && rd::<[usize; 8]>(internal, 0x20) == physical
            && rd::<[usize; 8]>(internal, 0x60) == [0; 8]
            && rd::<u32>(internal, 0xa0) == layout.count_with(extras),
        "CPU current four-owner map differs",
    )?;
    if layout.waiting() {
        check(super::native_source::exact_waiting_layout_with(physical[5] as *const u8,5,layout,extras),"waiting row layout changed")?;
    }
    let entries = extras.view(&physical).ok_or("registered created entry identity differs")?;
    Ok(Map {
        layout,
        symbol,
        manager,
        internal,
        entries,
        extras,
        created,
    })
}
pub(crate) unsafe fn pool(entry: usize, epoch: u64, owner: usize) -> Result<Binding, &'static str> {
    pool_in(entry,epoch,owner,None)
}
unsafe fn pool_for(m:Map,entry:usize,epoch:u64,owner:usize)->Result<Binding,&'static str> {
    pool_in(entry,epoch,owner,m.created.then_some(m.extras))
}
unsafe fn pool_in(entry:usize,epoch:u64,owner:usize,
    created:Option<crate::engine::mii::capacity::Extras>)->Result<Binding,&'static str> {
    let base = super::BASE.load(std::sync::atomic::Ordering::Acquire);
    let holder = pointer(base, POOL_HOLDER, 8, false)?;
    let manager = pointer(holder, 0, 0x24e0, true)?;
    let layout=super::mii_backend::current().ok_or("Mii backend unavailable")?;
    check(owner<8 && match created {
        None=>layout.selector(owner),
        Some(extras)=>layout==super::mii_backend::Layout::Training && extras.entry(owner)==Some(entry),
    }, "selector outside qualified backend destination")?;
    let row = base + 0x52c5730 + owner * 0x260;
    check(
        span(row, 0x260, true) && span(entry, 0x5a88, true),
        "CPU FE/E spans",
    )?;
    check(
        usize::from(rd::<u8>(row, 0xa7)) == owner,
        "CPU native selector is not1",
    )?;
    let b = Binding {
        epoch,
        owner: owner as u32,
        entry,
        row,
        holder,
        manager,
        vector_begin: rd(manager, 0x24d0),
        vector_end: rd(manager, 0x24d8),
        record_index: u16::from(rd::<u8>(row, 0xa7)),
    };
    let at = values::record_address(b, b).map_err(|_| "CPU selected G1 outside existing vector")?;
    check(span(at, values::RECORD_LEN, true), "CPU G1 writable span")?;
    Ok(b)
}
pub(crate) fn same_map(a: &Snapshot, b: &Snapshot) -> bool {
    a.layout == b.layout && a.binding == b.binding && a.created == b.created
        && a.thread != 0
        && a.thread == b.thread
        && a.symbol == b.symbol
        && a.manager == b.manager
        && a.internal == b.internal
        && a.entries == b.entries
}
pub(crate) fn same_owned(a: &Snapshot, b: &Snapshot, retained: bool) -> bool {
    same_map(a, b)
        && a.index == b.index
        && a.descriptor == b.descriptor
        && a.scalars == b.scalars
        && (!retained || a.record == b.record)
        && (!role_owner::current_record_claim(u32::from_le_bytes(a.descriptor[0x18..0x1c].try_into().unwrap()),a.index)
            || a.current_record == b.current_record)
        && a.owners[a.binding.owner as usize] == b.owners[b.binding.owner as usize]
        && a.control == b.control
}
pub(crate) unsafe fn current_locked(
    m: Map,
    b: Binding,
    original_kind: u32,
    staged: Option<(&[u8; 0x70], usize)>,
    helpers: helpers::View,
    source: Option<SourceToken>,
) -> Result<Snapshot, &'static str> {
    check(!m.created,"created destination requires its explicit phase capability")?;
    current_locked_in(m,b,original_kind,staged,helpers,source,None)
}
unsafe fn current_locked_in(
    m:Map,b:Binding,original_kind:u32,staged:Option<(&[u8;0x70],usize)>,
    helpers:helpers::View,source:Option<SourceToken>,
    mut created:Option<policy::CreatedDestination>,
)->Result<Snapshot,&'static str> {
    check_helpers_for(helpers, source)?;
    if let Some(destination)=created {
        check(m.created && destination.registered_in(m.layout,m.extras,b.owner as usize,b.entry),
            "created destination registration changed")?;
    }
    check(helpers.layout==m.layout,"selector helper backend changed")?;
    let base = super::BASE.load(std::sync::atomic::Ordering::Acquire);
    check(
        rd::<usize>(m.symbol, 0) == m.manager
            && rd::<usize>(m.manager, 0) == m.internal
            && rd::<[usize; 8]>(m.internal, 0x20) == m.physical()
            && rd::<[usize; 8]>(m.internal, 0x60) == [0; 8]
            && pool_for(m,b.entry, b.epoch, b.owner as usize)? == b,
        "CPU current map/G changed under lock",
    )?;
    let own_id = b.owner as usize;
    let table = pointer(base, OWNER_TABLE, 0x130, false)?;
    let mut owners = [None; 8];
    let mut protected = [None; 8];
    let mut kinds = [None; 8];
    for (id, fe) in m.physical().iter().copied().enumerate() {
        if fe == 0 {
            continue;
        }
        let extra = m.extras.entry(id) == Some(fe);
        check(
            (m.layout.owners().contains(&id) || extra) && aligned(fe) && span(fe, 0x5a88, false),
            "CPU extra/invalid owner",
        )?;
        let e = base + 0x52c5730 + id * 0x260;
        check(span(e, 0x260, false), "CPU owner E span")?;
        let d = pointer(fe, 0xf8, 0x3a0, id == own_id)?;
        let o = pointer(table, 0xe8 + id * 8, 0xb0, false)?;
        let index: u16 = rd(e, 0x1a8);
        check(
            rd::<u32>(fe, 0xe0) == id as u32 && rd::<u32>(e, 0x14) == id as u32,
            "CPU owner row identity",
        )?;
        // Only an exact registered created entry replaces this mirror with
        // Extras::admit's "word1 == its creation descriptor" below.
        check(
            extra || rd::<u32>(fe, 4) == u32::from(rd::<u8>(e, 0xa7)),
            "CPU descriptor selector differs",
        )?;
        if id == own_id {
            check(
                role_owner::original_row_matches(original_kind,rd(e,8)),
                "CPU original row kind changed",
            )?;
        }
        if let Some((descriptor, selected)) = staged.filter(|_| id == own_id) {
            let target_index = u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap());
            check(
                (match created {
                    Some(destination)=>destination.publication_indices(target_index,index,rd(fe,0x46),rd(d,0x72)),
                    None=>policy::publication_indices_in(m.layout,own_id,index,target_index,index,rd(fe,0x46),rd(d,0x72)),
                }) && rd::<[u8; 0x70]>(fe, 0) == *descriptor
                    && o == selected,
                "CPU staged FE / before E-D or selected O mismatch",
            )?;
        } else {
            check(
                rd::<u16>(fe, 0x46) == index
                    && rd::<u16>(d, 0x72) == index
                    && rd::<u16>(o, 0x68) == index,
                "CPU current index mirrors differ",
            )?;
        }
        let owner = Owner {
            entry: fe,
            row: e,
            information: d,
            object: o,
            index,
            selector: rd(e, 0xa7),
            alias: rd(table, 0x128 + id),
        };
        owners[id] = Some(owner);
        let kind = rd::<u32>(fe, 0x18);
        // During loader6 the own FE is staged, while E/D/O still describe outgoing.
        kinds[id] = Some(if id == own_id && staged.is_some() {
            rd(o, 0x58)
        } else {
            kind
        });
        if !(id == own_id && staged.is_some()) {
            check(
                super::mii_ordinary_player::native::owner_header_matches(fe,id as u32,o,kind),
                "CPU current owner header kind differs",
            )?;
        }
        if extra {
            // Shared exact admission with the Player0 walk; raw reads under G. A
            // protected non-destination; names the failed predicate.
            let facts=super::mii_backend::extra_facts(table,id,fe,e,d,o);
            if id==own_id {
                match created {
                    Some(destination)=>check(if staged.is_some() {
                        destination.staged_owner(owner,facts,kind)
                    } else {destination.completed_owner(owner,facts,kind)},"created destination phase changed")?,
                    None=>m.extras.admit(id,&owner,facts)?,
                }
            } else {
                super::mii_ordinary_player::native::created_kind(helpers,m.extras,id,owner,kind,facts)?;
            }
        } else if id != own_id && m.layout.logical(id as u32).is_some() {
            let current = authority(
                b,
                skyline::nn::os::GetCurrentThread() as usize,
                [m.symbol, m.manager, m.internal],
                m.entries,
                id as u32,
                owner,
            );
            let original = super::original_kind(if id == 0 {
                super::Role::Player
            } else {
                super::Role::Cpu
            })
            .ok_or("P1 original absent")?;
            check(
                foreign_kind(current, original, rd(e, 8), kind),
                "CPU protected original row/kind authority changed",
            )?;
        } else if id != own_id && m.layout.helper(id).is_some() {
            check(
                helper_kind(helpers, id, owner, kind),
                "CPU helper row kind/provenance changed",
            )?;
        }
        if id != own_id {
            protected[id] = Some(Protected {
                owner,
                descriptor: rd(fe, 0),
                e: rd(e, 0x1a8),
                d: rd(d, 0x72),
                specials: rd(d, 0x390),
                o: rd(o, 0x68),
                control: control(e),
            });
        }
    }
    check(
        role_owner::claims_in_with(m.layout, m.extras, &owners, &kinds).is_some(),
        "CPU selector-owned claim conflict",
    )?;
    if m.created && created.is_none() {
        check(staged.is_none(),"created origin cannot be captured after staging")?;
        let owner=owners[own_id].ok_or("created owner absent")?;
        let bytes=b.vector_end.checked_sub(b.vector_begin).ok_or("created G vector reversed")?;
        check(bytes.is_multiple_of(values::RECORD_LEN),"created G vector size differs")?;
        created=Some(policy::CreatedDestination::capture(m.layout,m.extras,own_id,&owners,&kinds,
            super::mii_backend::extra_facts(table,own_id,owner.entry,owner.row,owner.information,owner.object),
            bytes/values::RECORD_LEN).ok_or("created ordinary destination is not qualified")?);
    }
    let protected_indices = match created {
        Some(destination)=>destination.protected_now(m.layout,m.extras,&owners,&kinds),
        None=>policy::assignment_with(m.layout,m.extras,own_id,&owners),
    }.ok_or("CPU destination/selector/owner conflict")?;
    let mut records = [None; 8];
    for (slot, index) in protected_indices.iter().enumerate() {
        if let Some(index) = index {
            let record_binding = Binding {
                record_index: *index,
                ..b
            };
            let at = values::record_address(record_binding, record_binding)
                .map_err(|_| "CPU protected union bounds")?;
            check(
                span(at, values::RECORD_LEN, false),
                "CPU protected union span",
            )?;
            records[slot] = Some((*index, rd(at, 0)));
        }
    }
    let own = owners[own_id].ok_or("CPU owner absent")?;
    let header: [u32; 5] = rd(own.object, 0x54);
    let current_binding=Binding {record_index:own.index,..b};
    let current_at=values::record_address(current_binding,current_binding).map_err(|_|"current index bounds")?;
    check(span(current_at,values::RECORD_LEN,false),"current record span")?;
    let at = values::record_address(b, b).map_err(|_| "CPU destination changed")?;
    check(
        rd::<usize>(m.symbol, 0) == m.manager
            && rd::<usize>(m.manager, 0) == m.internal
            && rd::<[usize; 8]>(m.internal, 0x20) == m.physical()
            && rd::<[usize; 8]>(m.internal, 0x60) == [0; 8]
            && rd::<usize>(base, OWNER_TABLE) == table
            && rd::<usize>(table, 0xe8 + own_id * 8) == own.object
            && rd::<usize>(b.entry, 0xf8) == own.information,
        "CPU final current authority changed",
    )?;
    check_helpers_for(helpers, source)?;
    Ok(Snapshot {
        extras:m.extras,
        created,
        layout:m.layout,
        helpers,
        binding: b,
        thread: skyline::nn::os::GetCurrentThread() as usize,
        original_kind,
        descriptor: rd(b.entry, 0),
        header,
        scalars: Scalars::read(b.row, own.information),
        index: own.index,
        record: rd(at, 0),
        current_record: rd(current_at,0),
        control: control(b.row),
        symbol: m.symbol,
        manager: m.manager,
        internal: m.internal,
        entries: m.entries,
        owners,
        protected,
        records,
    })
}
pub(crate) unsafe fn snapshot(
    entry: usize,
    epoch: u64,
    original_kind: u32,
    owner: usize,
    source: Option<SourceToken>,
) -> Result<Snapshot, &'static str> {
    let helpers = helper_snapshot_for(source)?;
    let m = map(entry, owner)?;
    let b = pool(entry, epoch, owner)?;
    let g = PoolLock::acquire(
        super::BASE.load(std::sync::atomic::Ordering::Acquire),
        b.manager,
    );
    let result = current_locked(m, b, original_kind, None, helpers, source);
    drop(g);
    result
}
/// Initial read only; the resulting capability is usable only after the caller
/// binds the full SourceBook token. Epoch0 is not CPU1 authority and is never
/// compared to its mutable generation; created lifetime checks use SourceToken.
#[cfg(feature="mii-managed-helper-alpha")]
pub(crate) unsafe fn snapshot_created(entry:usize,original_kind:u32,owner:usize)
    ->Result<Snapshot,&'static str> {
    let helpers=helper_snapshot_for(None)?;
    let m=map_in(entry,owner,true)?;
    let b=pool_for(m,entry,0,owner)?;
    let g=PoolLock::acquire(super::BASE.load(std::sync::atomic::Ordering::Acquire),b.manager);
    let receipt=helpers.created_ordinal(owner).and_then(|i|helpers.created_miis[i]);
    let mut now=current_locked_in(m,b,original_kind,None,helpers,None,receipt.map(|r|r.destination))?;
    if let Some(receipt)=receipt {
        let own=now.owners[owner].ok_or("created replacement owner absent")?;
        let table=pointer(super::BASE.load(std::sync::atomic::Ordering::Acquire),OWNER_TABLE,0x130,false)?;
        let facts=super::mii_backend::extra_facts(table,owner,own.entry,own.row,own.information,own.object);
        let kind=rd::<u32>(own.entry,0x18);
        super::mii_ordinary_player::native::created_kind(helpers,m.extras,owner,own,kind,facts)?;
        now.created=Some(receipt.destination.reuse_completed(own,facts,kind)
            .ok_or("created replacement lacks completed owner authority")?);
    }
    check_helpers_for(helpers,None)?;
    drop(g);
    Ok(now)
}
/// Revalidation of an already captured created destination, never recapture of
/// an existing Mii. The full ready source receipt is checked by the caller.
#[cfg(feature="mii-managed-helper-alpha")]
pub(crate) unsafe fn created_ready_snapshot(binding:Binding,original_kind:u32,
    destination:policy::CreatedDestination,helpers:helpers::View)
    ->Result<CompletionGuard,&'static str> {
    let owner=binding.owner as usize;
    let m=map_in(binding.entry,owner,true)?;
    let b=pool_for(m,binding.entry,binding.epoch,owner)?;
    check(b==binding,"created ready G identity changed")?;
    let g=PoolLock::acquire(super::BASE.load(std::sync::atomic::Ordering::Acquire),b.manager);
    let now=current_locked_in(m,b,original_kind,None,helpers,None,Some(destination))?;
    Ok(CompletionGuard {now,_pool:g})
}

pub(crate) unsafe fn target_rows(c: Configuration) -> Result<(), &'static str> {
    super::mii_ordinary_player::native::target_rows(c).map(|_| ())
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Publication {
    pub before: Snapshot,
    pub selected: usize,
    pub initialized: bool,
    pub header: [u32; 5],
    pub o_index: u16,
    pub o_tokens: [u32; 4],
    pub task: usize,
    pub outstanding: Option<u32>,
    pub d_index: u16,
    pub rebuild: u8,
}
/// G remains held through the caller's exact ticket/INSTALL/registry commit.
/// This owns no lifetime or scheduler; each adapter uses the same checked write set.
pub(crate) struct PublicationGuard {
    pub observed: Publication,
    _pool: PoolLock,
    address: usize,
    scalars: Scalars,
    target_index: u16,
    payload: Option<Configuration>,
}
#[derive(Clone, Copy)]
pub(crate) struct PublicationTarget<'a> {
    pub before: &'a Snapshot,
    pub descriptor: &'a [u8; 0x70],
    pub payload: Option<Configuration>,
    pub scalars: Scalars,
    pub source: Option<SourceToken>,
    /// Fresh-O for this layout entry's own row. Retained same-class stays equal-hat.
    pub hat_change: bool,
    /// Immutable origin E+0x1a8 from the owning track. Not the active snapshot index.
    pub original_index: u16,
}
/// Which native phase a publication runs in. `Reload`: the canonical exact
/// path at loader state 6 of an active Reload (the FE already holds the staged
/// descriptor, the selected O is fresh). `BankSwitch(group_hat)`: a Mii
/// one-member group switch before the bank's native request (MII-GROUPS-DESIGN
/// §5.2): the loader is idle at 6/14, the FE still holds the outgoing window,
/// the selected O is the live, initialized resource owner whose Mii keys the
/// group prefetched for `group_hat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublicationPhase {
    Reload,
    BankSwitch { group_hat: u16 },
}

/// The canonical (`Reload`) publication. `allowed` is what the captured-O proof admits besides
/// equality: the gate writer's `ExpectedMutation::ConfigureSkipped` when it installed the
/// configure-equivalent `O+0x68`/tokens on the configure-skip path (U5, W-M2-4), otherwise `None`.
pub(crate) unsafe fn publication(
    target: PublicationTarget<'_>,
    selected: usize,
    evidence: &mut Option<Publication>,
    allowed: ExpectedMutation,
) -> Result<PublicationGuard, &'static str> {
    publication_in(target, selected, evidence, PublicationPhase::Reload, allowed)
}

pub(crate) unsafe fn publication_in(
    target: PublicationTarget<'_>,
    selected: usize,
    evidence: &mut Option<Publication>,
    phase: PublicationPhase,
    allowed: ExpectedMutation,
) -> Result<PublicationGuard, &'static str> {
    let PublicationTarget {
        before,
        descriptor,
        payload,
        scalars,
        source,
        hat_change,
        original_index,
    } = target;
    if let Some(c) = payload {
        target_rows(c)?;
    }
    let owner = before.binding.owner as usize;
    let m = map_in(before.binding.entry, owner,before.created.is_some())?;
    let b = pool_for(m,before.binding.entry, before.binding.epoch, owner)?;
    check(b == before.binding, "selector phase G identity changed")?;
    let helpers = helper_snapshot_for(source)?;
    check(
        helpers == before.helpers,
        "selector phase source provenance changed",
    )?;
    let g = PoolLock::acquire(
        super::BASE.load(std::sync::atomic::Ordering::Acquire),
        b.manager,
    );
    // A bank switch publishes before any native write: the FE is still the
    // outgoing window and every mirror is the current one, so the full current
    // walk applies (no staged branch); the exact FE/header equality is below.
    let staged = match phase {
        PublicationPhase::Reload => Some((descriptor, selected)),
        PublicationPhase::BankSwitch { .. } => None,
    };
    let now = current_locked_in(m,b,before.original_kind,staged,helpers,source,before.created)?;
    check(
        same_map(&now, before)
            && now.protected == before.protected
            && now.records == before.records,
        "selector phase protected owner/record union changed",
    )?;
    check(
        now.index == before.index
            && now.scalars == before.scalars
            && now.record == before.record
            && now.control == before.control,
        "selector old scalars/control/destination changed",
    )?;
    let own = now.owners[owner].ok_or("selected owner absent")?;
    let before_owner = before.owners[owner].ok_or("before owner absent")?;
    check(
        own.information == before_owner.information,
        "D ownership changed during INSTALL",
    )?;
    match phase {
        PublicationPhase::Reload => check(
            rd::<u8>(b.entry, 0x5920) == 3 && rd::<u8>(b.entry, 0x591c) == 6,
            "native loader6 required",
        )?,
        PublicationPhase::BankSwitch { .. } => {
            check(
                rd::<u8>(b.entry, 0x5920) == 6 && rd::<u8>(b.entry, 0x591c) == 14
                    && rd::<u8>(b.entry, 0x591a) == 0xff
                    && rd::<u8>(b.entry, 0x5934) == 0 && rd::<u8>(b.entry, 0x5935) == 0,
                "bank switch publication needs the native loader idle at 6/14",
            )?;
            check(
                now.descriptor == before.descriptor && now.header == before.header
                    && own.object == before_owner.object && own.object == selected,
                "bank switch outgoing entry/header changed",
            )?;
            check(
                crate::engine::swap::body_bank::MiiIdentity::from_descriptor(descriptor)
                    .is_some_and(|identity| identity.index == u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap())
                        && payload.is_some_and(|c| c.canonical().class as u32 == identity.kind())),
                "bank switch descriptor is not the payload's Mii identity",
            )?;
        }
    }
    let task: usize = rd(selected, 0xa8);
    let outstanding = if task == 0 {
        Some(0)
    } else if aligned(task) && span(task, 0x14, false) {
        Some(
            crate::engine::seams::native_fields::load_acquire_u32(task + 0x10),
        )
    } else {
        None
    };
    let observed = Publication {
        before: now,
        selected,
        initialized: rd::<u8>(selected, 0x4c) & 1 != 0,
        header: rd(selected, 0x54),
        o_index: rd(selected, 0x68),
        o_tokens: rd(selected, 0x6c),
        task,
        outstanding,
        d_index: rd(own.information, 0x72),
        rebuild: rd(own.information, 0x94),
    };
    *evidence = Some(observed);
    let t = before.scalars.e;
    // The expectation is the header this owner snapshot captured, not the FE group
    // words: after a group exchange the shared R keeps the window's kinds (D-PTA1). `allowed` is the
    // gate writer's receipt on the configure-skip path (U5), otherwise `None`.
    check(
        match before.created {
            Some(destination)=>allowed==ExpectedMutation::None && destination.outgoing(before.header,
                [t.outfit,t.hat,t.colour,t.voice],policy::OutgoingObject {
                    object:selected,header:observed.header,index:observed.o_index,tokens:observed.o_tokens,
                },observed.initialized,observed.outstanding),
            None=>policy::outgoing_object_matches_with(before_owner,before.header,
                [t.outfit,t.hat,t.colour,t.voice],policy::OutgoingObject {
                    object:selected,header:observed.header,index:observed.o_index,tokens:observed.o_tokens,
                },allowed),
        },
        "outgoing O identity/header/mirrors changed",
    )?;
    // The canonical Reload selects a fresh O; a bank switch keeps the live,
    // initialized resource owner and rewrites its index/header/tokens itself at
    // the build (MII-GROUPS-DESIGN §3.2), so that check is the phase's.
    if phase == PublicationPhase::Reload {
        check(
            !observed.initialized,
            "selected O initialized; no index reassignment",
        )?;
    }
    check(observed.outstanding == Some(0), "old O task outstanding")?;
    if payload.is_some() {
        check(observed.rebuild == 0, "Mii rebuild closed")?;
    }
    let hat = policy::HatPublication {
        layout: now.layout, owner: b.owner, source: source.is_some(), opt_in: hat_change,
        initialized: observed.initialized,
        fingerprints: hat_change && super::mii_ordinary_player::native::cpu_headgear_fingerprints(),
        before: now.scalars.e.hat, target: scalars.e.hat,
        destination_index: b.record_index, original_index,
        descriptor, payload,
    };
    // The canonical phase keeps the fresh-O headgear proof (the `}.validate()?;`
    // write boundary the replay185 wiring test anchors on); the bank phase proves
    // the group's prefetched hat instead (`validate_bank`).
    if let Some(destination)=before.created {
        check(phase==PublicationPhase::Reload,"created destination has no bank publication authority")?;
        destination.validate_hat(&hat)?;
    } else {
        match phase {
            PublicationPhase::Reload => { policy::HatPublication { ..hat }.validate()?; }
            PublicationPhase::BankSwitch { group_hat } => { hat.validate_bank(group_hat)?; }
        }
    }
    let address = values::record_address(b, b).map_err(|_| "selected G bounds")?;
    Ok(PublicationGuard {
        observed,
        _pool: g,
        address,
        scalars,
        target_index: u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap()),
        payload,
    })
}
impl PublicationGuard {
    /// Only after caller holds its current ticket state and exact final registry/INSTALL.
    pub(crate) unsafe fn write(&self) {
        if let Some(c) = self.payload {
            for (offset, byte) in c.charinfo().iter().enumerate() {
                wr(self.address, offset, *byte);
            }
        }
        let b = self.observed.before.binding;
        wr::<u16>(b.row, 0x1a8, self.target_index);
        self.scalars.write(
            b.row,
            self.observed.before.owners[b.owner as usize]
                .unwrap()
                .information,
        );
    }
}

pub(crate) struct CompletionGuard {
    pub now: Snapshot,
    _pool: PoolLock,
}
pub(crate) unsafe fn completed(
    target: PublicationTarget<'_>,
) -> Result<CompletionGuard, &'static str> {
    let PublicationTarget {
        before,
        descriptor,
        payload,
        scalars,
        source,
        hat_change: _,
        // Ignored. Ready completion compares the published descriptor and the
        // selector record. Hat restore uses publication(), which keeps this field.
        original_index: _,
    } = target;
    let owner = before.binding.owner as usize;
    let m = map_in(before.binding.entry, owner,before.created.is_some())?;
    let b = pool_for(m,before.binding.entry, before.binding.epoch, owner)?;
    check(b == before.binding, "ready G identity changed")?;
    let helpers = helper_snapshot_for(source)?;
    check(helpers == before.helpers, "ready source provenance changed")?;
    let g = PoolLock::acquire(
        super::BASE.load(std::sync::atomic::Ordering::Acquire),
        b.manager,
    );
    let now = current_locked_in(m, b, before.original_kind, None, helpers, source,before.created)?;
    check(
        same_map(&now, before)
            && now.protected == before.protected
            && now.records == before.records,
        "ready protected union changed",
    )?;
    let index = u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap());
    check(
        now.index == index
            && now.descriptor == *descriptor
            && now.scalars == scalars
            && now.control == before.control,
        "ready original/target configuration differs",
    )?;
    let expected = payload
        .map(|c| *c.charinfo())
        .unwrap_or_else(|| before.record[..CHARINFO_LEN].try_into().unwrap());
    check(
        now.record[..CHARINFO_LEN] == expected,
        "ready selected G payload differs",
    )?;
    if payload.is_none() {
        check(
            now.record == before.record,
            "return changed retained G resource",
        )?;
    }
    let own = now.owners[owner].ok_or("ready owner absent")?;
    let word =
        |offset: usize| u32::from_le_bytes(descriptor[offset..offset + 4].try_into().unwrap());
    check(
        rd::<[u32; 5]>(own.object, 0x54)
            == [
                owner as u32,
                word(0x18),
                word(0x1c),
                word(0x20),
                u32::from(descriptor[0x41]),
            ],
        "native O header differs",
    )?;
    let task: usize = rd(own.object, 0xa8);
    check(
        task == 0
            || (aligned(task)
                && span(task, 0x14, false)
                && crate::engine::seams::native_fields::load_acquire_u32(task + 0x10)
                    == 0),
        "ready task outstanding",
    )?;
    let tokens: [u16; 5] = rd(b.entry, 0x46);
    check(
        rd::<[u16; 4]>(own.information, 0x72) == tokens[..4]
            && rd::<[u32; 4]>(own.object, 0x6c)
                == [
                    u32::from(tokens[1]),
                    u32::from(tokens[2]),
                    u32::from(tokens[3]),
                    u32::from(tokens[4]),
                ],
        "native D/O propagation differs",
    )?;
    if payload.is_some() {
        check(
            usize::from_le_bytes(now.record[88..96].try_into().unwrap()) != 0
                && usize::from_le_bytes(now.record[96..104].try_into().unwrap()) != 0,
            "native resource pair absent",
        )?;
    }
    Ok(CompletionGuard { now, _pool: g })
}
