//! Generation and lifecycle regressions for the actual created receipt view.
use super::*;
use crate::engine::{mii::{backend::Layout,capacity::{Extras,Roots,ExtraFacts},
    ordinary_cpu::CreatedDestination,inventory::MiiClass,lifetime::{Binding,Configuration,Tokens},
    native_values::{RowIds,Mirrors}},swap::source_lifetime::{SourceBook,NativeEntry}};

#[test]
fn created_receipt_is_exact_source_bound_and_cleared_by_real_projection_operations() {
    let mut owners=std::array::from_fn(|id|(id<4).then_some(Owner {entry:0x1000+id*0x100,
        row:0x2000+id*0x100,information:0x3000+id*0x100,object:0x4000+id*0x100,
        index:0,selector:id as u8,alias:0}));
    let origin=Owner {entry:0x1700,row:0x2700,information:0x3700,object:0x4700,index:0,selector:7,alias:0};
    owners[7]=Some(origin);
    let extras=Extras::registered([Some(Roots {entry:origin.entry,resource:0x7000,
        alias:origin.object,selection:2,descriptor_selector:0}),None,None]).unwrap();
    let facts=ExtraFacts {entry_owner:7,row_owner:7,descriptor_selector:0,entry_index:0,
        information_index:0,object_index:0,entry_kind:9,object_kind:9,resource:0x7000,
        alias_root:origin.object,selection:2};
    let destination=CreatedDestination::capture(Layout::Training,extras,7,&owners,
        &owners.map(|o|o.map(|_|9)),facts,8).unwrap();
    let tokens=Tokens {outfit:0,hat:0,colour:4,voice:37,specials:[1,0,0,0]};
    let configuration=Configuration::from_observed(MiiClass::Swordfighter,tokens,[0;88],
        RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea}).unwrap();
    let key=SourceKey {kind:0x49,costume:1};
    let mut book=SourceBook::<8,1>::new();
    let load=book.begin_install(7,book.epoch(),NativeEntry(origin.entry as u64),key).unwrap();
    let mut view=View::EMPTY;
    view.project(book.epoch(),7,Helper::Unavailable {token:load.source(),phase:Phase::Loading});
    assert_eq!(view.ordinal(7),None);
    assert_eq!(view.created_ordinal(7),Some(0));
    assert!(view.loading(load.source()));
    assert_eq!(view.available_for(Some(load.source())),Ok(()));
    assert_eq!(view.available(),Err(BUSY));
    let ready=book.complete(load).unwrap();
    let owner=Owner {index:7,..origin};
    let proof=Proof {original:OriginalRow {row:origin.row,kind:9,index:7,selector:7,control:[0;7]},
        thread:0x8000,owner,kind:0x49,costume:1};
    let binding=Binding {epoch:53,owner:7,entry:owner.entry,row:owner.row,holder:0x9000,
        manager:0xa000,vector_begin:0xb000,vector_end:0xb000+8*104,record_index:7};
    let mut record=[0;104];record[88..96].copy_from_slice(&0x1000usize.to_le_bytes());
    record[96..104].copy_from_slice(&0x2000usize.to_le_bytes());
    let descriptor=destination.staged([0;0x70],configuration);
    let mii=MiiProof {binding,configuration,record,descriptor};
    view.project(book.epoch(),7,Helper::Ready {ready,key,proof});
    view.created_miis[0]=Some(CreatedReceipt {source:ready.source(),destination,mii});
    let live=ExtraFacts {entry_index:7,information_index:7,object_index:7,
        entry_kind:0x49,object_kind:0x49,..facts};
    assert!(view.accepts_created(7,proof,extras,live,true));
    assert!(!view.accepts_created(7,proof,extras,live,false));
    assert!(!view.accepts_created(7,proof,extras,ExtraFacts {descriptor_selector:7,..live},true));
    assert!(!view.accepts_created(7,Proof {owner:Owner {object:0x4800,..owner},..proof},extras,live,true));
    let mirrors=Mirrors {index:7,tokens,fe:[7,0,0,4,37],d:[7,0,0,4],d_specials:[1,0,0,0],
        owner_index:7,owner:[0,0,4,37]};
    assert!(mii.matches(binding,&descriptor,&record,mirrors));
    let mut changed=record;changed[40]^=1;
    assert!(!mii.matches(binding,&descriptor,&changed,mirrors));
    changed=record;changed[88..96].fill(0);
    assert!(!mii.matches(binding,&descriptor,&changed,mirrors));
    let next=book.begin_install(7,book.epoch(),NativeEntry(origin.entry as u64),key).unwrap();
    let replacement=book.complete(next).unwrap();
    assert_ne!(ready.source(),replacement.source());
    let mut stale=view;
    stale.created_slots[0]=Helper::Ready {ready:replacement,key,proof};
    assert!(!stale.accepts_created(7,proof,extras,live,true),"same FE/epoch cannot reuse another generation's receipt");
    for helper in [Helper::Unavailable {token:next.source(),phase:Phase::Loading},
        Helper::Unavailable {token:ready.source(),phase:Phase::Departed},Helper::Unmanaged] {
        let mut changed=view;changed.project(book.epoch(),7,helper);
        assert!(changed.created_miis[0].is_none());
        assert_eq!(changed.slots,view.slots);assert_eq!(changed.miis,view.miis);
        assert_eq!(changed.promoted,view.promoted);
        assert!(!changed.accepts_created(7,proof,extras,live,true));
    }
    let departed=View {epoch:Some(book.depart_scene().unwrap()),..view};
    assert!(!departed.accepts_created(7,proof,extras,live,true));
    assert!(!departed.loading(load.source()));
}
