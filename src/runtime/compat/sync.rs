//! The carried code's `training_mod_sync::{read, lock_write, assign}` (the fork's lock helper crate is not kept, decision B9).
//!
//! `read(&X)` copies the value out of whatever the clean room keeps behind `X`: the typed menu (`MENU`), the fighter manager's
//! slot address (`FIGHTER_MANAGER_ADDR`). A value is returned, never a guard, so no lock is held across a native call and a read
//! cannot deadlock with the commit that replaces the menu.

/// A global the carried code reads with [`read`].
pub trait Readable {
    type Value;
    fn read_value(&self) -> Self::Value;
}

/// The current value of `source`.
pub fn read<T: Readable>(source: &T) -> T::Value {
    source.read_value()
}
