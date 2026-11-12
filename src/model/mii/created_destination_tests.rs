//! Original created-destination regressions. These exercise the value policies
//! used at the native write boundary; they do not claim native execution.
lib_unit_tests! {
use super::*;
use crate::engine::mii::{backend::Layout,capacity::{Extras,Roots,ExtraFacts},
    inventory::MiiClass,lifetime::Tokens,native_values::RowIds};

fn fixture(slot:usize)->(CreatedDestination,Extras,[Option<Owner>;8],[Option<u32>;8],ExtraFacts) {
    let owners=std::array::from_fn(|id|Some(Owner {
        entry:0x1000+id*0x100,row:0x2000+id*0x100,information:0x3000+id*0x100,
        object:0x4000+id*0x100,index:0,selector:id as u8,alias:0}));
    let extras=Extras::registered([7,6,5,4].map(|id|Some(Roots {
        entry:owners[id].unwrap().entry,resource:0x7000+id*0x100,
        alias:owners[id].unwrap().object,selection:2,descriptor_selector:0}))).unwrap();
    let kinds=owners.map(|o|o.map(|_|9));
    let facts=ExtraFacts {entry_owner:slot as u32,row_owner:slot as u32,descriptor_selector:0,
        entry_index:0,information_index:0,object_index:0,entry_kind:9,object_kind:9,
        resource:0x7000+slot*0x100,alias_root:owners[slot].unwrap().object,selection:2};
    (CreatedDestination::capture(Layout::Training,extras,slot,&owners,&kinds,facts,8).unwrap(),
        extras,owners,kinds,facts)
}
fn configuration(class:MiiClass,hat:u16)->Configuration {
    Configuration::from_observed(class,Tokens {outfit:0,hat,colour:4,voice:37,specials:[1,0,0,0]},
        [0;88],RowIds {outfit:0x1300001bfef7a5f7,hat:0x5a00000fa9efa5ea | (u64::from(hat)<<40)}).unwrap()
}

fn four_created_rows_keep_creation_word_and_all_foreign_claims_through_each_class() {
    for slot in [7,6,5,4] {
        let (destination,extras,owners,kinds,facts)=fixture(slot);
        assert!(!Layout::Training.selector(slot));
        assert!(assignment_with(Layout::Training,extras,slot,&owners).is_none());
        let protected=destination.protected_now(Layout::Training,extras,&owners,&kinds).unwrap();
        for record in [0,1,2,3,4,5,6,7].into_iter().filter(|r|*r!=slot) {
            assert!(protected.contains(&Some(record as u16)));
        }
        for class in [MiiClass::Brawler,MiiClass::Swordfighter,MiiClass::Gunner] {
            for hat in [0,17] {
                let c=configuration(class,hat);
                let mut original=[0;0x70];original[0x37]=73;
                let descriptor=destination.staged(original,c);
                assert_eq!(&descriptor[4..8],&0u32.to_le_bytes());
                assert_eq!(descriptor[0x37],73);
                let staged=ExtraFacts {entry_index:slot as u16,entry_kind:class as u32,..facts};
                assert!(destination.staged_owner(owners[slot].unwrap(),staged,class as u32));
                assert!(!destination.staged_owner(owners[slot].unwrap(),ExtraFacts {object_kind:class as u32,..staged},class as u32));
                let mut live=owners;live[slot].as_mut().unwrap().index=slot as u16;
                let mut live_kinds=kinds;live_kinds[slot]=Some(class as u32);
                let published=ExtraFacts {information_index:slot as u16,object_index:slot as u16,
                    object_kind:class as u32,..staged};
                assert!(destination.completed_owner(live[slot].unwrap(),published,class as u32));
                assert_eq!(destination.protected_now(Layout::Training,extras,&live,&live_kinds),Some(protected));
                let reused=destination.reuse_completed(live[slot].unwrap(),published,class as u32).unwrap();
                let next=configuration(MiiClass::Gunner,hat+1);
                let next_descriptor=reused.staged(descriptor,next);
                let next_phase=ExtraFacts {entry_kind:MiiClass::Gunner as u32,..published};
                assert!(reused.staged_owner(live[slot].unwrap(),next_phase,MiiClass::Gunner as u32));
                assert!(reused.publication_indices(slot as u16,slot as u16,slot as u16,slot as u16));
                assert_eq!(reused.validate_hat(&HatPublication {layout:Layout::Training,owner:slot as u32,
                    source:true,opt_in:true,initialized:false,fingerprints:true,before:hat,target:hat+1,
                    destination_index:slot as u16,original_index:slot as u16,
                    descriptor:&next_descriptor,payload:Some(next)}),Ok(()));
                assert!(destination.reuse_completed(live[slot].unwrap(),ExtraFacts {selection:3,..published},class as u32).is_none());
                assert!(extras.admit(slot,&live[slot].unwrap(),published).is_err());
                assert!(CreatedDestination::capture(Layout::Training,extras,slot,&live,&live_kinds,published,8).is_none());
                let publication=HatPublication {layout:Layout::Training,owner:slot as u32,source:true,
                    opt_in:hat!=0,initialized:false,fingerprints:true,before:0,target:hat,
                    destination_index:slot as u16,original_index:0,descriptor:&descriptor,payload:Some(c)};
                assert_eq!(destination.validate_hat(&publication),Ok(()));
                assert!(destination.validate_hat(&HatPublication {initialized:true,..publication}).is_err());
                assert!(destination.validate_hat(&HatPublication {source:false,..publication}).is_err());
                assert!(destination.validate_hat(&HatPublication {payload:Some(configuration(class,hat+1)),..publication}).is_err());
                if hat!=0 {assert!(destination.validate_hat(&HatPublication {fingerprints:false,..publication}).is_err());}
                assert!(!destination.completed_owner(live[slot].unwrap(),ExtraFacts {resource:facts.resource+8,..published},class as u32));
            }
        }
        for foreign in [0,1,2,3,4,5,6,7].into_iter().filter(|id|*id!=slot) {
            let mut changed=owners;changed[foreign].as_mut().unwrap().index=slot as u16;
            assert!(destination.protected_now(Layout::Training,extras,&changed,&kinds).is_none());
            changed=owners;changed[foreign].as_mut().unwrap().selector=slot as u8;
            assert!(destination.protected_now(Layout::Training,extras,&changed,&kinds).is_none());
        }
        assert!(!destination.registered_in(Layout::Ldn,extras,slot,owners[slot].unwrap().entry));
        assert!(!destination.registered_in(Layout::Training,Extras::NONE,slot,owners[slot].unwrap().entry));
    }
}

fn created_fresh_o_rejects_pointer_header_tokens_initialization_and_task_changes() {
    for slot in [7,6,5,4] {
        let (destination,_,_,_,_)=fixture(slot);
        let header=[slot as u32,9,u32::MAX,u32::MAX,1];
        let now=OutgoingObject {object:destination.owner().object,header,index:0,tokens:[0;4]};
        assert!(destination.outgoing(header,[0;4],now,false,Some(0)));
        assert!(!destination.outgoing(header,[0;4],now,true,Some(0)));
        for task in [None,Some(1),Some(u32::MAX)] {assert!(!destination.outgoing(header,[0;4],now,false,task));}
        assert!(!destination.outgoing(header,[0;4],OutgoingObject {object:now.object+8,..now},false,Some(0)));
        assert!(!destination.outgoing(header,[0;4],OutgoingObject {index:slot as u16,..now},false,Some(0)));
        for field in 0..5 {
            let mut changed=now;changed.header[field]^=1;
            assert!(!destination.outgoing(header,[0;4],changed,false,Some(0)));
        }
        for field in 0..4 {
            let mut changed=now;changed.tokens[field]=1;
            assert!(!destination.outgoing(header,[0;4],changed,false,Some(0)));
        }
    }
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("four_created_rows_keep_creation_word_and_all_foreign_claims_through_each_class", four_created_rows_keep_creation_word_and_all_foreign_claims_through_each_class as fn()),
        ("created_fresh_o_rejects_pointer_header_tokens_initialization_and_task_changes", created_fresh_o_rejects_pointer_header_tokens_initialization_and_task_changes as fn()),
    ]);
}
}
