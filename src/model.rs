//! The pure half of the plugin (playbook layout: pure logic with host tests here, hooks in
//! `runtime`, which exists only for the Switch). Nothing in this tree reads or writes the game or
//! depends on the Switch target. Tests include these files by path (see `tests/`), because the crate
//! is a `cdylib` only and so cannot be linked by integration tests. The `#[path]` attributes name each
//! submodule file relative to `src/`, which resolves the same whether this file is the crate's own
//! module or is included by path from a test.

#[path = "model/ui/adapter.rs"]
pub mod adapter;
#[path = "model/ui/events.rs"]
pub mod ui_events;
#[path = "model/ui/readiness.rs"]
pub mod readiness;
#[path = "model/ui/catalog.rs"]
pub mod catalog;
#[path = "model/archive_name.rs"]
pub mod archive_name;
#[path = "model/archive_recipe.rs"]
pub mod archive_recipe;
#[path = "model/archive_retire.rs"]
pub mod archive_retire;
#[path = "model/build_split.rs"]
pub mod build_split;
#[path = "model/cpu/mod.rs"]
pub mod cpu;
#[path = "model/capture_settings.rs"]
pub mod capture_settings;
#[path = "model/capabilities.rs"]
pub mod capabilities;
#[path = "model/chords.rs"]
pub mod chords;
#[path = "model/damage_display.rs"]
pub mod damage_display;
#[path = "model/developer_ini.rs"]
pub mod developer_ini;
#[path = "model/developer_debug.rs"]
pub mod developer_debug;
#[path = "model/developer_reload.rs"]
pub mod developer_reload;
#[path = "model/diag_ring.rs"]
pub mod diag_ring;
#[path = "model/factory.rs"]
pub mod factory;
#[path = "model/guard.rs"]
pub mod guard;
#[path = "model/hook_stats.rs"]
pub mod hook_stats;
#[path = "model/input_adapter.rs"]
pub mod input_adapter;
#[path = "model/input_pending.rs"]
pub mod input_pending;
#[path = "model/ini_files.rs"]
pub mod ini_files;
#[path = "model/input_delay.rs"]
pub mod input_delay;
#[path = "model/frame_strip.rs"]
pub mod frame_strip;
#[path = "model/frame_strip_plan.rs"]
pub mod frame_strip_plan;
#[path = "model/install_line.rs"]
pub mod install_line;
#[path = "model/lra_reset.rs"]
pub mod lra_reset;
#[path = "model/macro_handoff.rs"]
pub mod macro_handoff;
#[path = "model/menu/mod.rs"]
pub mod menu;
#[path = "model/menu_shortcuts.rs"]
pub mod menu_shortcuts;
#[path = "model/migration.rs"]
pub mod migration;
#[path = "model/scene.rs"]
pub mod scene;
#[path = "model/migration_run.rs"]
pub mod migration_run;
#[path = "model/notify.rs"]
pub mod notify;
#[path = "model/recording.rs"]
pub mod recording;
#[path = "model/pause_helper.rs"]
pub mod pause_helper;
#[path = "model/settings.rs"]
pub mod settings;
#[path = "model/settings_file.rs"]
pub mod settings_file;
#[path = "model/sha256.rs"]
pub mod sha256;
#[path = "model/hitbox.rs"]
pub mod hitbox;
#[path = "model/hitbox_orbs.rs"]
pub mod hitbox_orbs;
#[path = "model/sites_hitbox.rs"]
pub mod sites_hitbox;
#[path = "model/sites_u4.rs"]
pub mod sites_u4;
#[path = "model/sites_u4b.rs"]
pub mod sites_u4b;
#[path = "model/effect_texture.rs"]
pub mod effect_texture;
#[path = "model/effect_chips.rs"]
pub mod effect_chips;
#[path = "model/ui_texture.rs"]
pub mod ui_texture;
#[path = "model/ui/stage_textures.rs"]
pub mod stage_textures;
#[path = "model/sites_ui.rs"]
pub mod sites_ui;
#[path = "model/sites.rs"]
pub mod sites;
#[path = "model/sites_pkg3.rs"]
pub mod sites_pkg3;
#[path = "model/sites_pkg4.rs"]
pub mod sites_pkg4;
#[path = "model/sites_pkg6.rs"]
pub mod sites_pkg6;
#[path = "model/sites_pkg2.rs"]
pub mod sites_pkg2;
#[path = "model/stage_hazards.rs"]
pub mod stage_hazards;
#[path = "model/store.rs"]
pub mod store;
#[path = "model/state/mod.rs"]
pub mod state;
#[path = "model/storage.rs"]
pub mod storage;
#[path = "model/ui/hash40.rs"]
pub mod hash40;
#[path = "model/ui/flow.rs"]
pub mod flow;
#[path = "model/ui/frame.rs"]
pub mod frame;
#[path = "model/ui/screens.rs"]
pub mod screens;
#[path = "model/ui/panes.rs"]
pub mod panes;
#[path = "model/version.rs"]
pub mod version;
#[path = "model/thread_stacks.rs"]
pub mod thread_stacks;
/// F9: Training's parked CPU entries and the roster cycle's helper sources (the give-back decision).
#[path = "model/parked_entries.rs"]
pub mod parked_entries;
/// F11: the running cell of a fighter entry's window (the game's active-cell index).
#[path = "model/entry_cells.rs"]
pub mod entry_cells;

/// Startup-allocated catalog reply transport; native callers never allocate channels.
#[path = "model/catalog_reply.rs"]
pub mod catalog_reply;

/// Boot-allocated storage for immutable native transaction snapshots.
#[path = "model/reusable_snapshot.rs"]
pub mod reusable_snapshot;

#[path = "model/body_activation.rs"]
pub mod body_activation;

/// Boot-reserved replay request identities and completion lifecycle.
#[path = "model/replay_jobs.rs"]
pub mod replay_jobs;

/// Native publication ownership transferred to the UI worker.
#[path = "model/catalog_mailbox.rs"]
pub mod catalog_mailbox;

/// Reply endpoints returned only after both request owners retire.
#[path = "model/pooled_reply.rs"]
pub mod pooled_reply;

/// Owned menu close intents retained until their matching native transition.
#[path = "model/close_handoff.rs"]
pub mod close_handoff;

#[path = "model/scalar_tls.rs"]
pub mod scalar_tls;

#[path = "model/boot_notices.rs"]
pub mod boot_notices;
