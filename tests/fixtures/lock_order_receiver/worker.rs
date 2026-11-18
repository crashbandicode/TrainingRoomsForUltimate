// Source-only resolver fixture: the model Ring is not the runtime Vm.
use crate::model::ui_events::Ring;
use crate::runtime::lua_vm::Vm;
static INPUT: Ring<64> = Ring::new();
static NATIVE: Vm = Vm::new();
static UNKNOWN: Unresolved = make_unknown();
static FOREIGN_LOCK: hos_lock::Mutex<Ring<64>> = make_foreign();
static RUNTIME_LOCK: hos_lock::Mutex<Vm> = make_runtime();
static UNKNOWN_LOCK: hos_lock::Mutex<Unresolved> = make_unknown();

pub fn foreign_static() { INPUT.reset(); }
pub fn runtime_static() { NATIVE.reset(); }
pub fn unresolved_static() { UNKNOWN.reset(); }
pub fn foreign_guard() { if let Some(mut guard) = FOREIGN_LOCK.try_lock() { guard.reset(); } }
pub fn runtime_guard() { if let Some(mut guard) = RUNTIME_LOCK.try_lock() { guard.reset(); } }
pub fn unresolved_guard() { if let Some(mut guard) = UNKNOWN_LOCK.try_lock() { guard.reset(); } }
