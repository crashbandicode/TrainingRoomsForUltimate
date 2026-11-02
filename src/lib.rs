//! Training Rooms for Ultimate: a Skyline plugin for Super Smash Bros. Ultimate 13.0.5.
//!
//! Layout (playbook): `model` is pure and host-tested; `runtime` holds everything that touches the
//! game and exists only for the Switch. The crate is a `cdylib` only, so tests include `model` by
//! path.

pub mod model;

#[cfg(target_os = "switch")]
mod runtime;
