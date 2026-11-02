//! Package 6 seam guards: the fork's authoritative site table (rows of the hot-swap engine, the Mii bridge, the automation helper, replay, native unlink, stages and the identity view) with its install API, plus the native-field and unlink guards.
//!
//! Carried from the fork (stage m5o) by tools/carry_pkg6.py: the files below are generated from the manifest
//! docs/provenance/pkg6-carry-manifest.json and are never edited by hand (CARRY-OVER.md, pkg6 section).

pub mod native_fields;
pub mod native_unlink_core;
pub mod native_unlink_guard;
pub mod native_seams;
