//! The Replay capture row on the Online Deluxe Plugins page (Q7; decision L1), Switch side. The row, its handler and the
//! outcome names are `engine::deluxe_row` (host-tested against a fake of the card's Deluxe 0.10.1 table).
//!
//! `register` runs once at boot from package 4's layer install (`macros::install`), after `replay_store::init` seeded the
//! preference from `replay_capture.ini` and before `live_replay::init`: a registered row makes the capture adapter resident
//! even while the INI says off, because a later On from the row cannot install Skyline hooks. It runs only when the layer
//! itself installed (its guarded closure, R2-1, passed): otherwise there is no preference to show and the install line says
//! `deluxe_row=not_installed reason=capture_layer_off`. Without Deluxe (`host_missing`) or with a host older than 0.5.3
//! (`host_older_than_0_5_3`) the plugin runs on with no row; nothing else changes.
//!
//! Registration allocates once at boot (the SDK copies the strings and boxes the handler); it is not a hook. The handler
//! then runs on the host's draw thread and allocates nothing (`engine::deluxe_row::CaptureRow`).

#[cfg(feature = "deluxe-sdk")]
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(feature = "deluxe-sdk")]
use crate::engine::deluxe_row::{CaptureRow, RowState, SPEC};
#[cfg(feature = "deluxe-sdk")]
use crate::runtime::diag_log::diagnostic;

/// `RowState` code; 0 (`LayerOff`) until `register` ran.
#[cfg(feature = "deluxe-sdk")]
static STATE: AtomicU8 = AtomicU8::new(0);

/// Register the row (boot, once). The SDK resolves the host at run time and sends descriptor version 4 exactly once.
#[cfg(feature = "deluxe-sdk")]
pub fn register() {
    let row = CaptureRow {
        toggle: crate::runtime::macros::replay_store::toggle,
        persist: crate::runtime::features::writer::persist_replay_capture,
    };
    let result = deluxe_plugin_sdk::register(&SPEC, row);
    let state = RowState::from_result(&result);
    STATE.store(state.code(), Ordering::Release);
    let (status, build) = host_identity();
    match result {
        Ok(handle) => diagnostic(format_args!(
            "[TrainingRooms] deluxe_row registered handle={} descriptor={} host_status={status:#x} host_build={build}",
            handle.0,
            deluxe_plugin_sdk::abi::ABI_VERSION
        )),
        Err(error) => diagnostic(format_args!(
            "[TrainingRooms] deluxe_row not registered reason={} error={error:?} host_status={status:#x} host_build={build}",
            state.reason().unwrap_or("")
        )),
    }
}

/// Without the SDK feature there is no row (the install line says so).
#[cfg(not(feature = "deluxe-sdk"))]
pub fn register() {}

/// True once a Deluxe host accepted the row (the capture adapter then installs at boot even while the INI says off).
#[cfg(feature = "deluxe-sdk")]
pub fn host_registered() -> bool {
    RowState::from_code(STATE.load(Ordering::Acquire)).registered()
}

/// `(installed, reason)` for the install line's `deluxe_row=` field.
pub fn report() -> (bool, Option<&'static str>) {
    #[cfg(feature = "deluxe-sdk")]
    {
        let state = RowState::from_code(STATE.load(Ordering::Acquire));
        (state.registered(), state.reason())
    }
    #[cfg(not(feature = "deluxe-sdk"))]
    {
        (false, Some("not_built"))
    }
}

/// The host's status bits and build string, when a host table exists (`("", 0)` otherwise). The build string is the host's,
/// NUL-terminated and valid for the process; it is shown up to 64 bytes and only when it is UTF-8. No allocation.
#[cfg(feature = "deluxe-sdk")]
fn host_identity() -> (u32, &'static str) {
    let Some(api) = deluxe_plugin_sdk::host() else { return (0, "") };
    // SAFETY: `host()` returns only a table that passed `table_matches` (32 bytes, table version 1); both functions take no
    // arguments, and `host_build` returns null or a string the host keeps for the life of the process.
    unsafe {
        let status = (api.host_status)();
        let build = (api.host_build)();
        if build.is_null() {
            return (status, "");
        }
        let bytes = std::ffi::CStr::from_ptr(build).to_bytes();
        let bytes = &bytes[..bytes.len().min(64)];
        (status, std::str::from_utf8(bytes).unwrap_or("non_utf8"))
    }
}
