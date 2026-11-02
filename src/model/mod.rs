//! The pure half of the plugin (playbook layout: pure logic with host tests here, hooks in
//! `runtime`, which exists only for the Switch). Nothing in this tree reads or writes the game or
//! depends on the Switch target. Tests include these files by path (see `tests/`), because the crate
//! is a `cdylib` only and so cannot be linked by integration tests.

pub mod developer_ini;
pub mod diag_ring;
pub mod guard;
pub mod input_delay;
pub mod install_line;
pub mod lra_reset;
pub mod settings;
pub mod sites;
pub mod stage_hazards;
pub mod storage;
pub mod version;
