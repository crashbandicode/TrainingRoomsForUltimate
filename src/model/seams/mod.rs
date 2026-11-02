//! Package 6 seam guards: the fork's authoritative site table (rows of the hot-swap engine, the Mii bridge, the automation helper, replay, native unlink, stages and the identity view) with its install API, plus the native-field and unlink guards.
//!
//! Carried from the fork (stage m5o) by tools/carry_pkg6.py: the files below are generated from the manifest
//! docs/provenance/pkg6-carry-manifest.json and are never edited by hand (CARRY-OVER.md, pkg6 section).
//!
//! The site table (`native_seams`) is unconditional: the runtime seam installer reads it at boot. The field and unlink guards have
//! no runtime consumer yet (their callers are the roster cycle and the unlink installer) and stay test-only until those land.

// Its consumers are the Switch-only runtime (untime::p1::cpu_pilot, later the roster cycle); on the host only the tests use it.
#[cfg(any(test, target_os = "switch"))]
pub mod native_fields;
#[cfg(test)]
pub mod native_unlink_core;
#[cfg(test)]
pub mod native_unlink_guard;
pub mod native_seams;