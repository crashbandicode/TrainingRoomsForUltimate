use super::frame_strip::Move;

pub const ROW_MOVE_NAMES: &str = "move_names";

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normal {
    Unknown, Jab1, Jab2, Jab3, RapidJab, RapidJabFinisher, DashAttack, ForwardTilt, UpTilt, DownTilt,
    ForwardSmash, UpSmash, DownSmash, NeutralAir, ForwardAir, BackAir, UpAir, DownAir, Grab, DashGrab,
    PivotGrab, Pummel, ForwardThrow, BackThrow, UpThrow, DownThrow, LedgeAttack, GetupAttack, TripAttack,
}

impl Normal {
    pub fn of(_: u64) -> Self { Self::Unknown }
    pub const fn name(self) -> Option<&'static str> { None }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot { Neutral, Side, Up, Down, Final }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterId {
    pub chara: u64,
    pub specials: [u8; 4],
}

#[derive(Debug)]
pub struct Refused;

pub struct Names;

impl Names {
    pub fn build(_: &[u8]) -> Result<Self, Refused> { Ok(Self) }
    pub fn special(&self, _: u64, _: Slot, _: u8) -> Option<&str> { None }
    pub fn bytes(&self) -> usize { 0 }
}

#[derive(Clone, Copy)]
pub struct Name {
    bytes: [u8; 96],
    len: usize,
}

impl Name {
    pub fn new(source: &str) -> Self {
        let mut out = Self { bytes: [0; 96], len: 0 };
        for ch in source.chars() {
            if out.len + ch.len_utf8() > out.bytes.len() { break; }
            ch.encode_utf8(&mut out.bytes[out.len..]);
            out.len += ch.len_utf8();
        }
        out
    }
    pub fn as_str(&self) -> &str { core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("") }
}

pub struct Published;
pub static PUBLISHED: Published = Published;

impl Published {
    pub fn set_on(&self, _: bool) {}
    pub fn publish(&self, _: Box<Names>) {}
    pub fn set_player(&self, _: Option<(i32, FighterId)>) {}
    pub fn player(&self) -> Option<(i32, FighterId)> { None }
    pub fn set_target(&self, _: Option<FighterId>) {}
}

pub fn recorded_identity(_: Option<(i32, FighterId)>, _: i32) -> Option<FighterId> { None }

#[derive(Clone, Copy)]
pub struct Namer<'a> { names: Option<&'a Names> }

impl<'a> Namer<'a> {
    pub const OFF: Namer<'static> = Namer { names: None };
    pub fn new(_: bool, names: Option<&'a Names>, _: Option<FighterId>) -> Self { Self { names } }
    pub fn live() -> Namer<'static> { Self::OFF }
    pub fn target() -> Namer<'static> { Self::OFF }
    pub fn recorded(_: Option<FighterId>) -> Namer<'static> { Self::OFF }
    pub fn name(&self, mv: Move, _: Normal) -> Name {
        let _ = self.names;
        Name::new(mv.name())
    }
    pub fn final_smash(&self) -> Name { Name::new("Final Smash") }
}

#[cfg(test)]
#[path = "move_names_tests.rs"]
mod tests;
