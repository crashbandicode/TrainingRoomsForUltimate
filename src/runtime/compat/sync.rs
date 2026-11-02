//! The carried code's `training_mod_sync::{read, assign, lock_write}` (the fork's lock helper crate is not kept, decision B9).
//!
//! `read(&X)` copies the value out of whatever the clean room keeps behind `X`: the typed menu (`MENU`), the fighter manager's
//! slot address (`FIGHTER_MANAGER_ADDR`). A value is returned, never a guard, so no lock is held across a native call and a read
//! cannot deadlock with the commit that replaces the menu. `assign(&X, value)` replaces the value behind `X`; `lock_write(&X)`
//! gives a guard that edits it in place (the typed menu's guard writes the change back when it drops, decision D1).

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Mutex, MutexGuard};

use crate::runtime::ui::menu_app::App;

/// A global the carried code reads with [`read`].
pub trait Readable {
    type Value;
    fn read_value(&self) -> Self::Value;
}

/// The current value of `source`.
pub fn read<T: Readable>(source: &T) -> T::Value {
    source.read_value()
}

/// A global the carried code replaces with [`assign`].
pub trait Assignable {
    type Value;
    fn assign_value(&self, value: Self::Value);
}

/// Replace the value behind `target`.
pub fn assign<T: Assignable>(target: &T, value: T::Value) {
    target.assign_value(value);
}

/// A global the carried code edits in place with [`lock_write`].
pub trait Lockable {
    type Guard;
    fn lock_write(&'static self) -> Self::Guard;
}

/// A guard that edits the value behind `target`.
pub fn lock_write<T: Lockable>(target: &'static T) -> T::Guard {
    target.lock_write()
}

impl Assignable for AtomicBool {
    type Value = bool;
    fn assign_value(&self, value: bool) {
        self.store(value, Ordering::Release);
    }
}

impl Lockable for Mutex<App> {
    type Guard = MutexGuard<'static, App>;
    fn lock_write(&'static self) -> Self::Guard {
        self.lock()
    }
}
