//! Scope dispatcher (WP-E, WORKPLAN §2.6d): asks the `swap_model::scope` rule whether a declared
//! callback may run in the current scene, logging a refusal once per callback (`[Scope] refused
//! name=… now=…`, a diag line). The declarations (U14, M4b) are one `if !scope_gate::admitted("…")`
//! at the start of each gated callback in `crate::engine::swap::swap_model::scope::CALLBACKS`; the
//! row-31 host test (`training_mod_consts/tests/scope_callbacks.rs`) checks that every call names a
//! row. Teardown, release and continuation rows (`Exception::MustComplete`) are declared and never
//! gated. Reads only `scope::GATES` (name, scopes, read-only, excused), so the rows' review text
//! stays out of the NRO. Never allocates or panics; reads only the native Training flag and the
//! three scene words `game_mode::current()` already reads, and not even those for a row admitted in
//! every scope.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::engine::swap::swap_model::scope::{self, Scope};

/// One bit per `GATES` row: refusal already logged.
static REFUSED_LOGGED: AtomicU64 = AtomicU64::new(0);
const _: () = assert!(scope::GATE_COUNT <= 64, "REFUSED_LOGGED holds one bit per row");

/// The current scope (`Match` while the scene words are unavailable).
pub(crate) fn current() -> Scope {
    let native_training = unsafe { crate::runtime::compat::common::is_native_training_mode() };
    match crate::runtime::lobby::game_mode::current() {
        Some(mode) => scope::scope_of(native_training, mode.melee, mode.mode),
        None if native_training => Scope::Training,
        None => Scope::Match,
    }
}

/// Whether the named callback may run now. A name without a row is admitted (the table is a
/// contract for the callbacks it lists; the row-31 host test keeps every call on a row).
pub(crate) fn admitted(name: &'static str) -> bool {
    let Some(index) = scope::GATES.iter().position(|g| g.name == name) else {
        return true;
    };
    let gate = &scope::GATES[index];
    if gate.admitted_everywhere() {
        return true;
    }
    let now = current();
    let ok = gate.admitted(now);
    if !ok {
        let bit = 1u64 << index;
        if REFUSED_LOGGED.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
            crate::engine::compat::logging::diagnostic(format_args!("[Scope] refused name={} now={}", name, now.name()));
        }
    }
    ok
}
