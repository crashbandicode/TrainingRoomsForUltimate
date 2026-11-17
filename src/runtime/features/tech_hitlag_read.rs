//! Shared live-reader algorithm; the production backend owns memory/native safety.
use super::tech_adapter::Error;
use crate::model::tech_hitlag::{self as arithmetic, Parameters};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ParamPair { pub ops: usize, pub value: usize }
pub trait Native {
    type Context: PartialEq;
    fn hitlag_held(&mut self) -> Result<Self::Context, Error>;
    fn hitlag_base(&self) -> usize;
    fn hitlag_admitted(&self) -> bool;
    fn hitlag_word(&mut self, address: usize) -> Option<usize>;
    fn hitlag_float(&mut self, address: usize) -> Option<f32>;
    fn hitlag_byte(&mut self, address: usize) -> Option<u8>;
    fn hitlag_readable(&mut self, address: usize, size: usize) -> bool;
    fn hitlag_pair(&mut self, object: usize, hash: u64) -> Option<ParamPair>;
    fn hitlag_get_float(&mut self, pair: &ParamPair) -> Option<f32>;
    fn hitlag_get_int(&mut self, pair: &ParamPair) -> Option<i32>;
    fn hitlag_k(&mut self, map: usize, hash: u64) -> Option<f32>;
}
fn slot(n: &mut impl Native, table: usize, offset: usize, expected: usize) -> Option<()> {
    (n.hitlag_word(table.checked_add(offset)?)? == expected).then_some(())
}
fn checked_pair(n: &mut impl Native, object: usize, hash: u64, field: usize, integer: bool) -> Option<ParamPair> {
    let base=n.hitlag_base();
    let pair=n.hitlag_pair(object,hash)?;
    let (ops, offset, target)=if integer { (0x4f69490,0x30,0x3bce20) }
        else { (0x4f62460,0x38,0x3a4a30) };
    if pair.ops != base+ops || pair.value != object.checked_add(field)? || !n.hitlag_readable(pair.value,4) {
        return None;
    }
    slot(n,pair.ops,offset,base+target)?;
    Some(pair)
}
pub fn multiplier(n: &mut impl Native) -> Option<f32> {
    let base=n.hitlag_base();
    if base==0 || !n.hitlag_admitted() { return None; }
    let root=n.hitlag_word(base+0x52b94f8)?;
    let global=n.hitlag_word(root)?;
    let value=n.hitlag_float(global.checked_add(0xf0)?)?;
    (value.is_finite() && value>0.0).then_some(value)
}
fn parameters(n: &mut impl Native) -> Option<Parameters> {
    let base=n.hitlag_base();
    if base==0 || !n.hitlag_admitted() || n.hitlag_byte(base+0x52c51d8)? != 0 { return None; }
    let holder=n.hitlag_word(base+0x52b8540)?;
    let object=n.hitlag_word(holder)?;
    let table=n.hitlag_word(object)?;
    if table != base+0x4f69b78 { return None; }
    slot(n,table,0x10,base+0x3bd350)?;
    let max=checked_pair(n,object,0x11cc94003c,0x108,false)?;
    let mul=checked_pair(n,object,0x11f8e00314,0x110,false)?;
    let add=checked_pair(n,object,0x11acf85152,0x10c,false)?;
    let threshold=checked_pair(n,object,0x23d2d966eb,0x16c,true)?;
    let map=n.hitlag_word(holder.checked_add(0x20)?)?;
    if !n.hitlag_readable(map,0x20) { return None; }
    Some(Parameters {
        maximum:n.hitlag_get_float(&max)?, multiplier:n.hitlag_get_float(&mul)?, add:n.hitlag_get_float(&add)?,
        threshold:n.hitlag_get_int(&threshold)?, object_multiplier:n.hitlag_k(map,0x192e58d48b)?,
        percent_multiplier:multiplier(n)?,
    })
}
/// Production and host probes execute this same read/revalidate/f32 pipeline.
pub fn resolve(n: &mut impl Native, base_damage: f32) -> Result<i32,Error> {
    let held=n.hitlag_held()?;
    if !n.hitlag_admitted() { return Err(Error::Guards); }
    let params=parameters(n).ok_or(Error::HitlagUnavailable)?;
    let frames=arithmetic::frames(base_damage,1.0,params).ok_or(Error::HitlagUnavailable)?;
    if n.hitlag_held()? != held { return Err(Error::NotPlaced); }
    Ok(frames)
}
