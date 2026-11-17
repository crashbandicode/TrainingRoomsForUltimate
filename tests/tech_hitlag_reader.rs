#[path = "../src/model/tech_hitlag.rs"]
pub mod arithmetic;
mod model { pub use crate::arithmetic as tech_hitlag; }
mod tech_adapter {
    #[derive(Debug,PartialEq,Eq)]
    pub enum Error { Busy, Guards, HitlagUnavailable, NotPlaced }
}
#[path = "../src/runtime/features/tech_hitlag_read.rs"]
mod reader;
use reader::{Native,ParamPair};
use tech_adapter::Error;
use std::collections::BTreeMap;
const BASE:usize=0x10000000;
const HOLDER:usize=0x20000000;
const OBJECT:usize=0x20001000;
const MAP:usize=0x20002000;
const ROOT:usize=0x20003000;
const GLOBAL:usize=0x20004000;
struct Mock {
    words:BTreeMap<usize,usize>, floats:BTreeMap<usize,f32>, denied:Option<usize>,
    admitted:bool, special:u8, pair_fault:u8, k:Option<f32>, stale:bool, busy:bool,
    held_calls:usize, k_calls:usize, multiplier_reads:usize, getter_calls:usize,
}
impl Mock {
    fn valid()->Self {
        Self { words:BTreeMap::from([
            (BASE+0x52b8540,HOLDER),(HOLDER,OBJECT),(OBJECT,BASE+0x4f69b78),
            (BASE+0x4f69b88,BASE+0x3bd350),(HOLDER+0x20,MAP),
            (BASE+0x52b94f8,ROOT),(ROOT,GLOBAL),
            (BASE+0x4f62498,BASE+0x3a4a30),(BASE+0x4f694c0,BASE+0x3bce20)]),
            floats:BTreeMap::from([(OBJECT+0x108,30.0),(OBJECT+0x110,0.65),
                (OBJECT+0x10c,6.0),(GLOBAL+0xf0,1.2)]),
            denied:None,admitted:true,special:0,pair_fault:0,k:Some(1.0),stale:false,busy:false,
            held_calls:0,k_calls:0,multiplier_reads:0,getter_calls:0 }
    }
}
impl Native for Mock {
    type Context=u64;
    fn hitlag_held(&mut self)->Result<u64,Error> {
        self.held_calls+=1;
        if self.busy {Err(Error::Busy)} else {Ok(if self.stale && self.held_calls>1 {2} else {1})}
    }
    fn hitlag_base(&self)->usize {BASE}
    fn hitlag_admitted(&self)->bool {self.admitted}
    fn hitlag_readable(&mut self,address:usize,_size:usize)->bool {self.denied!=Some(address) && address!=0}
    fn hitlag_word(&mut self,address:usize)->Option<usize> {
        self.hitlag_readable(address,8).then(||self.words.get(&address).copied()).flatten()
    }
    fn hitlag_float(&mut self,address:usize)->Option<f32> {
        if address==GLOBAL+0xf0 {self.multiplier_reads+=1;}
        self.hitlag_readable(address,4).then(||self.floats.get(&address).copied()).flatten()
    }
    fn hitlag_byte(&mut self,address:usize)->Option<u8> {
        assert_eq!(address,BASE+0x52c51d8);
        self.hitlag_readable(address,1).then_some(self.special)
    }
    fn hitlag_pair(&mut self,object:usize,hash:u64)->Option<ParamPair> {
        assert_eq!(object,OBJECT);
        let (offset,ops)=match hash {
            0x11cc94003c=>(0x108,0x4f62460),0x11f8e00314=>(0x110,0x4f62460),
            0x11acf85152=>(0x10c,0x4f62460),0x23d2d966eb=>(0x16c,0x4f69490),_=>panic!() };
        Some(ParamPair {ops:BASE+ops+usize::from(self.pair_fault==1),
            value:OBJECT+offset+usize::from(self.pair_fault==2)})
    }
    fn hitlag_get_float(&mut self,pair:&ParamPair)->Option<f32> {self.getter_calls+=1;self.hitlag_float(pair.value)}
    fn hitlag_get_int(&mut self,pair:&ParamPair)->Option<i32> {self.getter_calls+=1;assert_eq!(pair.value,OBJECT+0x16c);Some(10)}
    fn hitlag_k(&mut self,map:usize,hash:u64)->Option<f32> {
        assert_eq!((map,hash),(MAP,0x192e58d48b));self.k_calls+=1;self.k
    }
}
#[test]
fn actual_reader_rereads_live_k_and_multiplier_every_resolve() {
    let mut n=Mock::valid();
    assert_eq!(reader::resolve(&mut n,30.0),Ok(25));
    n.k=Some(0.925);n.floats.insert(GLOBAL+0xf0,1.3);
    assert_eq!(reader::resolve(&mut n,30.0),Ok(23));
    assert_eq!((n.k_calls,n.multiplier_reads,n.held_calls),(2,2,4));
    n.floats.insert(GLOBAL+0xf0,0.0);
    assert_eq!(reader::resolve(&mut n,30.0),Err(Error::HitlagUnavailable));
    assert_eq!(n.multiplier_reads,3);
}
#[test]
fn actual_reader_refuses_null_unreadable_and_wrong_native_types_before_getters() {
    for address in [BASE+0x52b8540,HOLDER,OBJECT,BASE+0x4f69b88,HOLDER+0x20,
        BASE+0x52b94f8,ROOT,OBJECT+0x108,GLOBAL+0xf0,BASE+0x52c51d8,MAP] {
        let mut n=Mock::valid();n.denied=Some(address);
        assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable),"{address:x}");
    }
    for address in [BASE+0x52b8540,HOLDER,HOLDER+0x20,BASE+0x52b94f8,ROOT] {
        let mut n=Mock::valid();n.words.insert(address,0);
        assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable),"null at {address:x}");
    }
    for address in [OBJECT,BASE+0x4f69b88,BASE+0x4f62498,BASE+0x4f694c0] {
        let mut n=Mock::valid();n.words.insert(address,0xbad);
        assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable));
        assert_eq!(n.getter_calls,0);
    }
    for fault in [1,2] {
        let mut n=Mock::valid();n.pair_fault=fault;
        assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable));
        assert_eq!(n.getter_calls,0);
    }
}
#[test]
fn actual_reader_refuses_missing_k_special_mode_and_invalid_live_values() {
    for k in [None,Some(0.0),Some(-1.0),Some(f32::NAN),Some(f32::INFINITY)] {
        let mut n=Mock::valid();n.k=k;
        assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable));
    }
    let mut n=Mock::valid();n.special=1;
    assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable));
    assert_eq!(n.getter_calls,0);
    let mut n=Mock::valid();n.admitted=false;
    assert_eq!(reader::resolve(&mut n,12.0),Err(Error::Guards));
    assert_eq!(n.getter_calls,0);
}
#[test]
fn actual_reader_preserves_busy_and_rejects_stale_held_identity() {
    let mut n=Mock::valid();n.busy=true;
    assert_eq!(reader::resolve(&mut n,12.0),Err(Error::Busy));
    assert_eq!(n.k_calls,0);
    let mut n=Mock::valid();n.stale=true;
    assert_eq!(reader::resolve(&mut n,12.0),Err(Error::NotPlaced));
    assert_eq!(n.held_calls,2);
}

#[test]
fn actual_reader_refuses_malformed_g_and_map_pointer_values() {
    for (slot,good) in [(ROOT,GLOBAL),(HOLDER+0x20,MAP)] {
        for bad in [0,good+4,0x1_0000_0000_0000,usize::MAX] {
            let mut n=Mock::valid();n.words.insert(slot,bad);
            // A readable four-aligned float cannot establish a valid G pointer.
            n.floats.insert(bad.wrapping_add(0xf0),1.2);
            assert_eq!(reader::resolve(&mut n,12.0),Err(Error::HitlagUnavailable),"slot={slot:x} bad={bad:x}");
            if slot==HOLDER+0x20 {assert_eq!(n.k_calls,0);}
        }
    }
}
