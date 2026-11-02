//! The fork's `FighterId` (its consts crate): which of the two practice fighters a call means.

/// Player 1 or the practice CPU, by the fork's discriminants (`role as i32` is the entry id outside the waiting room).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum FighterId {
    Player = 0,
    CPU = 1,
}