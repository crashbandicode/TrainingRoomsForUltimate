//! Bounded production-policy checks for the observed LDN owner/index split and return.
use {crate::engine::mii::backend::Layout, crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId}, crate::engine::mii::lifetime::Configuration, crate::engine::mii::ordinary_player::Owner, crate::engine::mii::remembered::Value, crate::engine::mii::role_owner};

use crate::engine::swap::character_memory;

fn owners() -> [Option<Owner>; 8] {
    // run18's four native selectors/current indices; addresses are synthetic.
    let mut rows=[None;8];
    for (id,index) in [(0,0),(4,26),(5,27),(6,28)] {
        rows[id]=Some(Owner {entry:0x1000+id*0x100,row:0x2000+id*0x100,
            information:0x3000+id*0x100,object:0x4000+id*0x100,index,selector:id as u8,alias:0});
    }
    rows
}

fn backend20_ordinary_shared_zero_does_not_claim_player_payload() {
    let rows=owners().map(|row|row.map(|o|Owner {index:0,..o}));
    let mut kinds=rows.map(|row|row.map(|_|9));kinds[0]=Some(0x49);
    assert!(role_owner::claims_in(Layout::Ldn,&rows,&kinds).is_some());
    assert!(!role_owner::current_record_claim(9,0));
    assert!(role_owner::current_record_claim(0x49,0));
    assert!(role_owner::current_record_claim(9,26));
    kinds[4]=Some(0x49);
    assert!(role_owner::claims_in(Layout::Ldn,&rows,&kinds).is_none());
}

fn configurations() -> (Configuration,Configuration) {
    let catalog=build_snapshot(vec![RawSlot {index:0,slot_id:2622,name_len:1,
        name_units:[65,0,0,0,0,0,0,0,0,0],colour:4,voice:37,specials:[1,0,0,0],
        tag_outfit:TaggedId(0x13002b147bc59435),tag_hat:TaggedId(0x5a00000fa9efa5ea),
        tag_chara:TaggedId(0xc1004d15d7061f77),charinfo:[0;88]}]);
    let saved=catalog.configs[0].clone();
    let original=Configuration::from_saved(&saved).unwrap();
    let mut different=saved;different.charinfo[0]=1;
    (original,Configuration::from_saved(&different).unwrap())
}

fn backend20_same_kind_same_canonical_return_keeps_exact_configuration_and_cursor() {
    let (original,accepted)=configurations();
    assert_eq!(original.canonical(),accepted.canonical());assert_ne!(original,accepted);
    let native=Value {fighter:0x49,forms:[0x49,u32::MAX,u32::MAX],cursor:None,
        configuration:Some(original),owned:true,original:true};
    let wanted=Value {configuration:Some(accepted),cursor:Some(2),original:false,..native};
    let mut memory=character_memory::WaitingSession::empty();
    memory.scene_changed(true,13,0x02010000);
    memory.observe(0,native);memory.observe(1,native);
    memory.commit(1,wanted); // Only the accepted completion is remembered.
    memory.player_departed();memory.scene_changed(true,1,0x02010000);
    memory.scene_changed(true,13,0x02010000);memory.observe(1,native);
    assert_eq!(memory.restore_target(1),Some(wanted));
    assert!(!role_owner::payload_already_active(native.configuration,wanted.configuration,true));
    assert!(!Value {original:true,..wanted}.matches_original(native.fighter,native.forms,native.configuration));
    assert!(native.matches_original(native.fighter,native.forms,native.configuration));
    memory.manual_request(1); // Interrupted replacement never overwrites the accepted value.
    memory.scene_changed(true,1,0x02010000);memory.scene_changed(true,13,0x02010000);
    assert_eq!(memory.restore_target(1),Some(wanted));
    memory.commit(1,wanted);assert_eq!(memory.restore_target(1),None);
    memory.player_departed();assert!(memory.observe(0,native));
    assert_eq!(memory.restore_target(1),None); // Fresh room forgets both roles.
    memory.scene_changed(true,1,0x02010000);
    assert_eq!(memory.restore_target(0),Some(native));assert_eq!(memory.restore_target(1),None);
}

/// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("backend20_ordinary_shared_zero_does_not_claim_player_payload", backend20_ordinary_shared_zero_does_not_claim_player_payload as fn()),
        ("backend20_same_kind_same_canonical_return_keeps_exact_configuration_and_cursor", backend20_same_kind_same_canonical_return_keeps_exact_configuration_and_cursor as fn()),
    ]);
}
