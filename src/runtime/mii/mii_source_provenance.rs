//! Value projection of SourceBook ownership; never a second source lifetime book.
use crate::engine::swap::source_lifetime::{Phase, SceneEpoch, SourceToken};
#[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))]
use crate::engine::swap::source_lifetime::{ReadyToken, SourceKey};
use crate::engine::mii::ordinary_player::Owner;

pub(crate) const BUSY: &str = "managed helper load still owns native installation";
pub(crate) const CAPTURE_BUSY: &str = "native installation began during origin capture";
/// A helper slot that is unavailable for a reason other than a load in progress (an unqualified ready source, or a
/// loader of a departed epoch). It never clears while the owner idles: callers do not retry it.
pub(crate) const NO_AUTHORITY: &str = "managed helper has no current ordinary authority";

#[derive(Debug)]
pub(crate) enum CaptureError { Busy, Refused(&'static str) }
impl From<&'static str> for CaptureError {
    fn from(reason: &'static str) -> Self {
        if reason == BUSY || reason == CAPTURE_BUSY { Self::Busy }
        else { Self::Refused(reason) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginalRow {
    pub row: usize, pub kind: i8, pub index: u16, pub selector: u8, pub control: [u8; 7],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Proof {
    pub original: OriginalRow, pub thread: usize, pub owner: Owner,
    pub kind: u32, pub costume: u32,
}
/// Exact payload/resource identity committed by the shared selector transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MiiProof {
    pub binding:crate::engine::mii::lifetime::Binding,
    pub configuration:crate::engine::mii::lifetime::Configuration,
    pub record:[u8;crate::engine::mii::native_values::RECORD_LEN],
    pub descriptor:[u8;0x70],
}
impl MiiProof {
    /// Copied values only. Caller supplies current native map/G lock authority.
    pub fn matches(&self,binding:crate::engine::mii::lifetime::Binding,
        descriptor:&[u8;0x70],record:&[u8;crate::engine::mii::native_values::RECORD_LEN],
        mirrors:crate::engine::mii::native_values::Mirrors)->bool {
        self.holds(binding,descriptor,record,mirrors,false)
    }
    /// TASK-OPUS-rc6-open-fixes item 1 (primary 2026-11-09 00:09Z, 00:23Z): a live F9 seat (slot <= No. of CPUs) is the
    /// same Mii by reading B's identity predicate in place of byte equality of both the descriptor and the record: going
    /// live rewrites them (every re-proof logged `descriptor_changed=true`). Binding and mirrors' tokens stay exact.
    pub fn holds(&self,binding:crate::engine::mii::lifetime::Binding,
        descriptor:&[u8;0x70],record:&[u8;crate::engine::mii::native_values::RECORD_LEN],
        mirrors:crate::engine::mii::native_values::Mirrors,live:bool)->bool {
        self.binding==binding
            && (live || (self.descriptor==*descriptor && self.record==*record))
            && crate::model::parked_entries::mii_record_identity(record,self.configuration.charinfo())
            && mirrors.validate(binding.record_index,self.configuration.tokens()).is_ok()
    }
}
/// Receipt committed only alongside the matching SourceBook ready token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CreatedReceipt {
    pub source:SourceToken,
    pub destination:crate::engine::mii::ordinary_cpu::CreatedDestination,
    pub mii:MiiProof,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Helper {
    Unmanaged,
    Unavailable { token: SourceToken, phase: Phase },
    #[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))]
    Ready { ready: ReadyToken, key: SourceKey, proof: Proof },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct View {
    pub layout:crate::engine::mii::backend::Layout,
    pub epoch: Option<SceneEpoch>, pub slots: [Helper; 2],
    pub miis:[Option<MiiProof>;2],
    /// Training creation order 7/6/5/4, never native helper ordinals. SourceBook
    /// projects these tokens; this value view owns no independent lifetime.
    pub created_slots:[Helper;4],
    pub created_miis:[Option<CreatedReceipt>;4],
}
impl View {
    pub const EMPTY: Self = Self { layout:crate::engine::mii::backend::Layout::Training,epoch: None, slots: [Helper::Unmanaged; 2],miis:[None;2],created_slots:[Helper::Unmanaged;4],created_miis:[None;4] };
    pub fn ordinal(&self,id:usize)->Option<usize> {self.layout.helper(id)}
    pub fn created_ordinal(&self,id:usize)->Option<usize> {
        (self.layout==crate::engine::mii::backend::Layout::Training)
            .then(||crate::engine::mii::capacity::EXTRA_SLOTS.iter().position(|slot|*slot==id)).flatten()
    }
    pub fn source_at(&self,id:usize)->Option<Helper> {
        self.ordinal(id).map(|i|self.slots[i])
            .or_else(||self.created_ordinal(id).map(|i|self.created_slots[i]))
    }
    /// Full source token equality includes generation, even at a reused FE.
    pub fn loading(&self,token:SourceToken)->bool {
        Some(token.epoch())==self.epoch && self.source_at(token.slot())
            ==Some(Helper::Unavailable {token,phase:Phase::Loading})
    }
    #[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))]
    pub fn accepts_created(&self,id:usize,now:Proof,
        extras:crate::engine::mii::capacity::Extras,
        facts:crate::engine::mii::capacity::ExtraFacts,parked:bool)->bool {
        let Some(i)=self.created_ordinal(id) else{return false;};
        let Helper::Ready {ready,key,proof}=self.created_slots[i] else{return false;};
        let Some(CreatedReceipt {source,destination,mii})=self.created_miis[i] else{return false;};
        let token=ready.source();
        source==token && Some(token.epoch())==self.epoch && token.slot()==id
            && token.entry().0==now.owner.entry as u64 && now==proof && parked && now.thread!=0
            && key.kind==now.kind && key.costume==now.costume
            && destination.registered_in(self.layout,extras,id,now.owner.entry)
            && destination.completed_owner(now.owner,facts,now.kind)
            && mii.configuration.canonical().class as u32==key.kind
            && mii.binding.owner as usize==id && mii.binding.entry==now.owner.entry
            && mii.binding.row==now.owner.row && mii.binding.record_index==destination.record()
    }


    /// Source STATE projects actual lifecycle changes through this one leaf
    /// operation. Replacement/departure/destruction discard exact receipts.
    pub fn project(&mut self,epoch:SceneEpoch,slot:usize,helper:Helper) {
        self.epoch=Some(epoch);
        if let Some(i)=self.ordinal(slot) {
            self.slots[i]=helper;self.miis[i]=None;
        } else if let Some(i)=self.created_ordinal(slot) {
            self.created_slots[i]=helper;self.created_miis[i]=None;
        }
    }

    pub fn available(&self) -> Result<(), &'static str> { self.available_for(None) }
    pub fn available_for(&self, own:Option<SourceToken>) -> Result<(), &'static str> {
        if let Some(token)=own {
            if !self.loading(token) {
                return Err("exact managed helper transaction absent");
            }
        }
        // Phase::Ready in Unavailable is an actual ready SourceBook slot whose
        // additional coexistence proof was absent; it is never a retry signal.
        for helper in self.slots {
            if matches!(helper,Helper::Unavailable {token,phase:Phase::Loading} if Some(token)==own) {continue;}
            match helper {
                Helper::Unavailable { token, phase: Phase::Loading }
                    if Some(token.epoch()) == self.epoch => return Err(BUSY),
                Helper::Unavailable { .. } => return Err(NO_AUTHORITY),
                _ => {}
            }
        }
        for (_i,helper) in self.created_slots.into_iter().enumerate() {
            if matches!(helper,Helper::Unavailable {token,phase:Phase::Loading} if Some(token)==own) {continue;}
            if matches!(helper,Helper::Unavailable {token,phase:Phase::Loading} if Some(token.epoch())==self.epoch) {
                return Err(BUSY);
            }
            // Ordinary created sources have no Mii receipt and retain the
            // existing Extras::admit checks. They cannot authorize a Mii.
            #[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))]
            if self.created_miis[_i].is_some() && !matches!(helper,Helper::Ready {ready,..}
                if Some(ready.source().epoch())==self.epoch) {return Err(NO_AUTHORITY);}
        }
        Ok(())
    }

    pub fn accepts(&self, id: usize, now: Proof,
        #[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))] parked: bool) -> bool {
        let Some(ordinal)=self.ordinal(id) else{return false;};
        let helper=&self.slots[ordinal];
        match *helper {
            // Native waiting helpers inherit the CSS selection too: Ice Climbers
            // keeps E+8=0x6e while its first loaded actor is Popo (0x4b). Use
            // the same exact native-row identity policy as visible owners.
            // Managed sources below still require their complete committed proof.
            Helper::Unmanaged => crate::engine::mii::role_owner::original_row_matches(
                now.kind, now.original.kind),
            Helper::Unavailable { .. } => false,
            #[cfg(any(test, feature = "mii-managed-source-coexist-alpha"))]
            Helper::Ready { ready, key, proof } => {
                let token = ready.source();
                token.slot() == id && Some(token.epoch()) == self.epoch
                    && token.entry().0 == now.owner.entry as u64
                    && proof == now && now.thread != 0 && parked
                    && key.kind == now.kind && key.costume == now.costume
                    && match self.miis[ordinal] {
                        None=>!(0x48..=0x4a).contains(&key.kind),
                        Some(mii)=>mii.configuration.canonical().class as u32==key.kind
                            && mii.binding.owner as usize==id && mii.binding.entry==now.owner.entry
                            && mii.binding.row==now.owner.row && mii.binding.record_index==id as u16
                            && now.owner.index==id as u16 && now.owner.selector as usize==id,
                    }
            }
        }
    }
}

#[cfg(all(target_os = "switch", feature = "mii-managed-source-coexist-alpha"))]
impl super::Registry {
    /// Source STATE is held by the caller; registry is the leaf lock.
    pub(crate) fn source(&mut self, epoch: SceneEpoch, slot: usize, helper: Helper) {
        let backend=super::super::super::mii_backend::current();
        // A slot that is not a helper of the current backend (e.g. a Training created
        // entry 5/6/7) never selects another layout's helper ordinals.
        if self.helpers.ordinal(slot).is_none() && self.helpers.slots==[Helper::Unmanaged;2]
            && self.helpers.created_slots==[Helper::Unmanaged;4]
            && backend.is_none_or(|l|l.helper(slot).is_some()) {
            use crate::engine::mii::backend::Layout;
            // Both waiting families share helpers 5/6; only the current scene names the family.
            let current=backend.filter(|l|l.helper(slot).is_some());
            if let Some(layout)=current.or_else(||[Layout::Training,Layout::Ldn,Layout::Quickplay]
                .iter().copied().find(|l|l.helper(slot).is_some())) {self.helpers.layout=layout;}
        }
        self.helpers.project(epoch,slot,helper);
    }
}

#[cfg(all(target_os = "switch", feature = "mii-managed-source-coexist-alpha"))]
pub(crate) unsafe fn observe(entry: usize, slot: u32) -> Option<Proof> {
    use crate::runtime::mii::mii_native_access::*;
    use super::{control, map, pointer, span};
    let layout=super::super::super::mii_backend::current()?;
    layout.helper(slot as usize)?;
    let base = super::super::base();
    let p0 = super::super::super::native_source::entry(0)? as usize;
    let m = map(p0).ok()?;
    // Helper ordinals stay exactly two; registered created entries are accepted
    // only as exact extra identities beside them.
    if m.entries[slot as usize] != entry || !layout.shape_with(&m.physical(), m.extras)
    { return None; }
    let row = base + 0x52c5730 + slot as usize * 0x260;
    if !span(entry, 0x5a88, false) || !span(row, 0x260, false) { return None; }
    let table = pointer(base, OWNER_TABLE, 0x130, false).ok()?;
    let information = pointer(entry, 0xf8, 0x3a0, false).ok()?;
    let object = pointer(table, 0xe8 + slot as usize * 8, 0xb0, false).ok()?;
    let index = rd::<u16>(row, 0x1a8);
    let owner = Owner { entry, row, information, object, index,
        selector: rd(row, 0xa7), alias: rd(table, 0x128 + slot as usize) };
    let kind = rd::<u32>(entry, 0x18);
    if owner.selector != slot as u8 || owner.alias != 0
        || rd::<u32>(entry, 4) != slot || rd::<u32>(entry, 0xe0) != slot
        || rd::<u32>(row, 0x14) != slot || rd::<u16>(entry, 0x46) != index
        || rd::<u16>(information, 0x72) != index || rd::<u16>(object, 0x68) != index
        || rd::<u32>(object, 0x54) != slot || rd::<u32>(object, 0x58) != kind
    { return None; }
    let proof = Proof { original: OriginalRow {row, kind: rd(row, 8), index,
        selector: owner.selector, control: control(row)}, owner, kind,
        costume: rd(object, 0x64), thread: skyline::nn::os::GetCurrentThread() as usize };
    if proof.thread == 0 || rd::<usize>(m.symbol, 0) != m.manager
        || rd::<usize>(m.manager, 0) != m.internal
        || rd::<[usize; 8]>(m.internal, 0x20) != m.physical()
        || rd::<[usize; 8]>(m.internal, 0x60) != [0; 8]
        || rd::<usize>(base, OWNER_TABLE) != table
        || rd::<usize>(table, 0xe8 + slot as usize * 8) != object
        || rd::<usize>(entry, 0xf8) != information
    { return None; }
    Some(proof)
}

#[cfg(all(target_os = "switch", feature = "mii-managed-source-coexist-alpha"))]
pub(crate) unsafe fn original(entry: usize, slot: u32) -> Option<OriginalRow> {
    observe(entry, slot).filter(|p| crate::engine::mii::role_owner::original_row_matches(
        p.kind, p.original.kind))
        .map(|p| p.original)
}
