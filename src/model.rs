//! The pure half of the plugin (playbook layout: pure logic with host tests here, hooks in
//! `runtime`, which exists only for the Switch). Nothing in this tree reads or writes the game or
//! depends on the Switch target. Tests include these files by path (see `tests/`), because the crate
//! is a `cdylib` only and so cannot be linked by integration tests. The `#[path]` attributes name each
//! submodule file relative to `src/`, which resolves the same whether this file is the crate's own
//! module or is included by path from a test.

#[path = "model/cpu/mod.rs"]
pub mod cpu;
#[path = "model/developer_ini.rs"]
pub mod developer_ini;
#[path = "model/diag_ring.rs"]
pub mod diag_ring;
#[path = "model/factory.rs"]
pub mod factory;
#[path = "model/guard.rs"]
pub mod guard;
#[path = "model/input_delay.rs"]
pub mod input_delay;
#[path = "model/install_line.rs"]
pub mod install_line;
#[path = "model/lra_reset.rs"]
pub mod lra_reset;
#[path = "model/menu/mod.rs"]
pub mod menu;
#[path = "model/migration.rs"]
pub mod migration;
#[path = "model/settings.rs"]
pub mod settings;
#[path = "model/sites.rs"]
pub mod sites;
// Carried fork areas (package 6): compiled for tests only until their runtime consumers are carried (docs/PACKAGE-6-PLAN.md\n// section 5); each area's cfg(test) is removed with the step that lands its consumer, so the release build never carries\n// unused code and clippy -D warnings stays clean without an llow.
#[cfg(test)]
#[path = "model/preload/mod.rs"]
pub mod preload;
#[cfg(test)]
#[path = "model/seams/mod.rs"]
pub mod seams;
#[path = "model/stage_hazards.rs"]
pub mod stage_hazards;
#[cfg(test)]
#[path = "model/swap/mod.rs"]
pub mod swap;
#[path = "model/storage.rs"]
pub mod storage;
#[path = "model/version.rs"]
pub mod version;
