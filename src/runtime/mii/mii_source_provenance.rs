//! Value projection of SourceBook ownership; never a second source lifetime book.
use crate::engine::swap::source_lifetime::{Phase, SceneEpoch, SourceToken};
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
            if self.created_miis[_i].is_some() && !matches!(helper,Helper::Ready {ready,..}
                if Some(ready.source().epoch())==self.epoch) {return Err(NO_AUTHORITY);}
        }
        Ok(())
    }

    pub fn accepts(&self, id: usize, now: Proof,
        parked: bool) -> bool {
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

#[cfg(target_os = "switch")]
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

#[cfg(target_os = "switch")]
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

#[cfg(target_os = "switch")]
pub(crate) unsafe fn original(entry: usize, slot: u32) -> Option<OriginalRow> {
    observe(entry, slot).filter(|p| crate::engine::mii::role_owner::original_row_matches(
        p.kind, p.original.kind))
        .map(|p| p.original)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::swap::source_lifetime::{NativeEntry, SourceBook};

    fn same_bytes_replacement_changes_provenance_and_departure_refuses() {
        let mut book = SourceBook::<4, 1>::new();
        let key = SourceKey { kind: 7, costume: 0 };
        let owner = Owner { entry: 17, row: 18, information: 19, object: 20,
            index: 0, selector: 2, alias: 0 };
        let proof = Proof { original: OriginalRow { row: 18, kind: 0x49, index: 0,
            selector: 2, control: [0; 7] }, thread: 21, owner, kind: 7, costume: 0 };
        let load = book.begin_install(2, book.epoch(), NativeEntry(17), key).unwrap();
        let ready = book.complete(load).unwrap();
        let old = View { epoch: Some(book.epoch()), slots: [Helper::Ready { ready, key, proof }, Helper::Unmanaged],..View::EMPTY };
        assert!(old.accepts(2, proof, true));
        assert!(!old.accepts(2, Proof { thread: 22, ..proof }, true));
        let next = book.begin_install(2, book.epoch(), NativeEntry(17), key).unwrap();
        let loading = View { slots: [Helper::Unavailable { token: next.source(), phase: Phase::Loading }, Helper::Unmanaged], ..old };
        assert_eq!(loading.available(), Err(BUSY));
        assert!(!loading.accepts(2, proof, true));
        let ready = book.complete(next).unwrap();
        let current = View { slots: [Helper::Ready { ready, key, proof }, Helper::Unmanaged], ..old };
        assert_ne!(old, current);
        assert!(current.accepts(2, proof, true));
        let departed = View { epoch: Some(book.depart_scene().unwrap()), ..current };
        assert!(!departed.accepts(2, proof, true));
        let unqualified = View { slots: [Helper::Unavailable { token: ready.source(), phase: Phase::Ready }, Helper::Unmanaged], ..current };
        assert!(unqualified.available().is_err());
        assert!(!unqualified.accepts(2, Proof { original: OriginalRow { kind: 7, ..proof.original }, ..proof }, true));
        assert!(matches!(CaptureError::from(BUSY), CaptureError::Busy));
        assert!(matches!(CaptureError::from(CAPTURE_BUSY), CaptureError::Busy));
        assert!(matches!(CaptureError::from(unqualified.available().unwrap_err()), CaptureError::Refused(_)));
        assert_eq!(unqualified.available(), Err(NO_AUTHORITY));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("same_bytes_replacement_changes_provenance_and_departure_refuses", same_bytes_replacement_changes_provenance_and_departure_refuses as fn()),
        ]);
    }
    }
}

#[cfg(test)]
mod mii_tests {
    lib_unit_tests! {
    use super::*;
    use std::convert::TryInto;
    use crate::engine::swap::source_lifetime::{SourceBook,NativeEntry,Holder};
    use crate::engine::mii::{inventory::{build_snapshot,RawSlot,MiiClass,TaggedId},
        lifetime::{Binding,Configuration},native_values::Mirrors};
    fn ordinary_helper_mii_exact_payload_resources_and_generation_are_independent_of_class() {
        let raw=RawSlot {index:0,slot_id:4155,name_len:1,name_units:[65,0,0,0,0,0,0,0,0,0],
            colour:4,voice:37,specials:[0;4],tag_outfit:TaggedId(0x1300291bfef7a5f7),
            tag_hat:TaggedId(0x5a00000fa9efa5ea),
            tag_chara:TaggedId(0xc100000000000000|MiiClass::Brawler.chara_hash40()),charinfo:[0;88]};
        let catalog=build_snapshot(vec![raw]);let c=Configuration::from_saved(&catalog.configs[0]).unwrap();
        let mut draft=crate::engine::mii::draft::Draft::from_template(&catalog.configs[0]).unwrap();
        draft.set_specials([1,0,0,0]).unwrap();let alt=draft.configuration().unwrap();
        assert_eq!(c.canonical().class,alt.canonical().class);assert_ne!(c,alt);
        // The exact payload is kept. A different hat is not replaced by a
        // captured helper hat. LDN slot 2 is not a waiting helper.
        let hat=c.tokens().hat;
        assert_eq!(crate::engine::mii::ordinary_cpu::helper_configuration(c,[hat.wrapping_add(1),hat]),Ok(c));
        assert_eq!(crate::engine::mii::ordinary_cpu::helper_configuration(c,[hat.wrapping_add(1),hat.wrapping_add(2)]),Ok(c));
        assert_eq!(crate::engine::mii::ordinary_cpu::helper_configuration(alt,[hat;2]),Ok(alt));
        assert_eq!(crate::engine::mii::ordinary_cpu::headgear_reload(
            crate::engine::mii::backend::Layout::Ldn,2,hat,hat.wrapping_add(1)),
            Err("headgear row is not on this layout"));
        let b=Binding {epoch:4,owner:2,entry:0x1200,row:0x2200,holder:0x5000,manager:0x6000,
            vector_begin:0x7000,vector_end:0x7000+4*104,record_index:2};
        let mut record=[0u8;104];record[..88].copy_from_slice(c.charinfo());
        record[88..96].copy_from_slice(&0x8000u64.to_le_bytes());record[96..].copy_from_slice(&0x9000u64.to_le_bytes());
        let descriptor=crate::engine::mii::ordinary_cpu::staged_descriptor_for(2,[0;0x70],Some(c)).unwrap();
        assert_eq!(u16::from_le_bytes(descriptor[0x46..0x48].try_into().unwrap()),2);
        let t=c.tokens();
        let mirrors=Mirrors {index:2,tokens:t,fe:[2,t.outfit,t.hat,t.colour,t.voice],
            d:[2,t.outfit,t.hat,t.colour],d_specials:t.specials.map(u32::from),owner_index:2,
            owner:[t.outfit,t.hat,t.colour,t.voice].map(u32::from)};
        let mii=MiiProof {binding:b,configuration:c,record,descriptor};
        assert!(mii.matches(b,&descriptor,&record,mirrors));
        assert!(!MiiProof {configuration:alt,..mii}.matches(b,&descriptor,&record,mirrors));
        let mut changed=record;changed[96]^=1;
        assert!(!mii.matches(b,&descriptor,&changed,mirrors));
        assert!(!mii.matches(Binding {vector_end:b.vector_end+104,..b},&descriptor,&record,mirrors));
        let mut book=SourceBook::<4,1>::new();let key=SourceKey {kind:0x48,costume:0};
        let load=book.begin_install(2,book.epoch(),NativeEntry(b.entry as u64),key).unwrap();
        let view=View {epoch:Some(book.epoch()),slots:[Helper::Unavailable {token:load.source(),phase:Phase::Loading},Helper::Unmanaged],..View::EMPTY};
        assert_eq!(view.available(),Err(BUSY));assert_eq!(view.available_for(Some(load.source())),Ok(()));
        assert!(book.ready(2).is_err());
        let ready=book.complete(load).unwrap();
        let owner=Owner {entry:b.entry,row:b.row,information:0x3200,object:0x4200,index:2,selector:2,alias:0};
        let proof=Proof {original:OriginalRow {row:b.row,kind:9,index:2,selector:2,control:[0;7]},
            thread:21,owner,kind:0x48,costume:0};
        let committed=View {slots:[Helper::Ready {ready,key,proof},Helper::Unmanaged],miis:[Some(mii),None],..view};
        assert!(committed.accepts(2,proof,true));
        assert!(!committed.accepts(2,proof,false));
        assert!(!View {miis:[None;2],..committed}.accepts(2,proof,true));
        assert!(!View {miis:[Some(MiiProof {binding:Binding {record_index:3,..b},..mii}),None],
            ..committed}.accepts(2,proof,true));
        let lease=book.acquire(ready,Holder {category:1,id:17}).unwrap();
        assert_eq!(book.leased_ready(lease,Holder {category:1,id:17}),Some(ready));
        assert!(book.begin_install(2,book.epoch(),NativeEntry(b.entry as u64),key).is_err());
        assert!(book.release(lease));let next=book.begin_install(2,book.epoch(),NativeEntry(b.entry as u64),key).unwrap();
        assert_ne!(next,load);assert!(view.available_for(Some(next.source())).is_err());
        let next_ready=book.complete(next).unwrap();
        assert!(View {slots:[Helper::Ready {ready:next_ready,key,proof},Helper::Unmanaged],..committed}.accepts(2,proof,true));
        assert!(book.complete(load).is_err());
        let departed=View {epoch:Some(book.depart_scene().unwrap()),..committed};
        assert!(!departed.accepts(2,proof,true));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("ordinary_helper_mii_exact_payload_resources_and_generation_are_independent_of_class", ordinary_helper_mii_exact_payload_resources_and_generation_are_independent_of_class as fn()),
        ]);
    }
    }
}

#[cfg(test)]
#[path="created_source_provenance_tests.rs"]
mod created_source_provenance_tests;
