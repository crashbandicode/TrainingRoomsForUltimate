//! Training Rooms for Ultimate: a Skyline plugin for Super Smash Bros. Ultimate 13.0.5.
//!
//! Layout (playbook): `model` is pure and host-tested; `runtime` holds everything that touches the
//! game and exists only for the Switch. The crate is a `cdylib` only, so tests include `model` by
//! path.

pub mod model;

// The carried fork engines (package 6): each area is test-only until its consumer lands, except those engine/mod.rs leaves open.
pub mod engine;

#[cfg(target_os = "switch")]
mod runtime;
