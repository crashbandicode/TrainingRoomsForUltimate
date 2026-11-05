//! The fork's Mii host tests that read more than the Mii crate (`training_mod_mii/tests/{ldn_backend,
//! quickplay_backend}.rs`, carried by pkg6): lib test modules, so `crate::engine` is the whole engine, as the fork's
//! `training_mod_consts` was to them.
mod ldn_backend;
mod quickplay_backend;
