//! The integration suite keeps each former target in its own module.
//! Run a group with `cargo test --test all -- module_name::`.

macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}

macro_rules! callgraph_self_tests {
    ($($body:tt)*) => { $($body)* };
}

#[path = "../support/scratch.rs"]
mod test_scratch;

#[path = "../common/callgraph.rs"]
mod callgraph;
#[path = "../common/mod.rs"]
mod common;
#[path = "../common/tech_callbacks.rs"]
mod tech_callbacks;

#[path = "../../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "../support/playback_engine.rs"]
#[allow(dead_code, unused_imports)]
pub mod engine;
mod allocation;
mod input_json;
mod slice_load;
#[path = "../scrub_chain_heap.rs"]
mod scrub_chain_heap;
#[path = "../scrub_play_timing.rs"]
mod scrub_play_timing;
#[path = "../generated_idle.rs"]
mod generated_idle;
#[path = "../scrub_save_heap.rs"]
mod scrub_save_heap;
#[path = "../capture_fresh_process.rs"]
mod capture_fresh_process;
#[path = "../../src/test_table.rs"]
mod test_table;
#[path = "../build_tag.rs"]
mod build_tag;
#[path = "../capabilities.rs"]
mod capabilities;
#[path = "../capture_completion.rs"]
mod capture_completion;
#[path = "../carried_diag_ring.rs"]
mod carried_diag_ring;
#[path = "../carried_guard.rs"]
mod carried_guard;
#[path = "../carried_model_facade.rs"]
mod carried_model_facade;
#[path = "../carry_pkg4.rs"]
mod carry_pkg4;
#[path = "../carry_pkg6.rs"]
mod carry_pkg6;
#[path = "../catalog_reply.rs"]
mod catalog_reply;
#[path = "../core01.rs"]
mod core01;
#[path = "../core02_menu.rs"]
mod core02_menu;
#[path = "../core04_recovery.rs"]
mod core04_recovery;
#[path = "../core04_settings.rs"]
mod core04_settings;
#[path = "../core06_hitbox.rs"]
mod core06_hitbox;
#[path = "../core06_orbs.rs"]
mod core06_orbs;
#[path = "../cpu01.rs"]
mod cpu01;
#[path = "../cpu02.rs"]
mod cpu02;
#[path = "../cpu03.rs"]
mod cpu03;
#[path = "../cpu04.rs"]
mod cpu04;
#[path = "../cpu05.rs"]
mod cpu05;
#[path = "../cpu06.rs"]
mod cpu06;
#[path = "../cpu07.rs"]
mod cpu07;
#[path = "../cpu_common.rs"]
mod cpu_common;
#[path = "../cpu_core.rs"]
mod cpu_core;
#[path = "../cpu_expiry_owner.rs"]
mod cpu_expiry_owner;
#[path = "../cpu_hide.rs"]
mod cpu_hide;
#[path = "../cpu_receipt.rs"]
mod cpu_receipt;
#[path = "../cpu_sites.rs"]
mod cpu_sites;
#[path = "../developer_features.rs"]
mod developer_features;
#[path = "../docs_gates.rs"]
mod docs_gates;
#[path = "../entry_cells.rs"]
mod entry_cells;
#[path = "../findings_docs_audit.rs"]
mod findings_docs_audit;
#[path = "../findings_menu_close.rs"]
mod findings_menu_close;
#[path = "../findings_single_settings.rs"]
mod findings_single_settings;
#[path = "../frame_strip_settings.rs"]
mod frame_strip_settings;
#[path = "../fs027_busy_sees_rr_host.rs"]
mod fs027_busy_sees_rr_host;
#[path = "../game_thread_handoffs.rs"]
mod game_thread_handoffs;
#[path = "../temp_dir_scan.rs"]
mod temp_dir_scan;
#[path = "../hook_alloc.rs"]
mod hook_alloc;
#[path = "../hook_locks.rs"]
mod hook_locks;
#[path = "../hook_tls.rs"]
mod hook_tls;
#[path = "../inline_keyboard.rs"]
mod inline_keyboard;
#[path = "../input01.rs"]
mod input01;
#[path = "../input04.rs"]
mod input04;
#[path = "../input05.rs"]
mod input05;
#[path = "../input_delay.rs"]
mod input_delay;
#[path = "../input_mapping_callers.rs"]
mod input_mapping_callers;
#[path = "../input_pause.rs"]
mod input_pause;
#[path = "../input_pending.rs"]
mod input_pending;
#[path = "../input_sites.rs"]
mod input_sites;
#[path = "../intro_once.rs"]
mod intro_once;
#[path = "../kirby_hat_copy_pair.rs"]
mod kirby_hat_copy_pair;
#[path = "../kirby_hat_cpu.rs"]
mod kirby_hat_cpu;
#[path = "../lint_allows.rs"]
mod lint_allows;
#[path = "../lock_order.rs"]
mod lock_order;
#[path = "../lockfree_wiring.rs"]
mod lockfree_wiring;
#[path = "../lockon_items.rs"]
mod lockon_items;
#[path = "../loop_picker.rs"]
mod loop_picker;
#[path = "../mashout_clatter_set.rs"]
mod mashout_clatter_set;
#[path = "../mashout_egg.rs"]
mod mashout_egg;
#[path = "../mashout_random_roll.rs"]
mod mashout_random_roll;
#[path = "../mashout_reload.rs"]
mod mashout_reload;
#[path = "../menu_catalog_cache.rs"]
mod menu_catalog_cache;
#[path = "../menu_nav.rs"]
mod menu_nav;
#[path = "../menu_nonblocking.rs"]
mod menu_nonblocking;
#[path = "../menu_writeback.rs"]
mod menu_writeback;
#[path = "../migration.rs"]
mod migration;
#[path = "../migration_run.rs"]
mod migration_run;
#[path = "../mii_bank_groups.rs"]
mod mii_bank_groups;
#[path = "../mii_bank_publication.rs"]
mod mii_bank_publication;
#[path = "../mii_bust_context.rs"]
mod mii_bust_context;
#[path = "../mii_configuration.rs"]
mod mii_configuration;
#[path = "../mii_helper147_headgear.rs"]
mod mii_helper147_headgear;
#[path = "../mii_ldn_handshake.rs"]
mod mii_ldn_handshake;
#[path = "../mii_player146_headgear_ownership.rs"]
mod mii_player146_headgear_ownership;
#[path = "../mii_replay185_headgear.rs"]
mod mii_replay185_headgear;
#[path = "../mii_role_adapter.rs"]
mod mii_role_adapter;
#[path = "../mii_same_notice.rs"]
mod mii_same_notice;
#[path = "../mii_setup_preflight.rs"]
mod mii_setup_preflight;
#[path = "../native_service_wiring.rs"]
mod native_service_wiring;
#[path = "../native_wrappers.rs"]
mod native_wrappers;
#[path = "../no_spin_locks.rs"]
mod no_spin_locks;
#[path = "../p1_core.rs"]
mod p1_core;
#[path = "../p1_pilot_hooks.rs"]
mod p1_pilot_hooks;
#[path = "../pane_recipe_guard.rs"]
mod pane_recipe_guard;
#[path = "../pause_helper_sites.rs"]
mod pause_helper_sites;
#[path = "../perfect_rule_counters.rs"]
mod perfect_rule_counters;
#[path = "../phase2_pkg6_storage.rs"]
mod phase2_pkg6_storage;
#[path = "../pkg6_deps.rs"]
mod pkg6_deps;
#[path = "../pkg6_features.rs"]
mod pkg6_features;
#[path = "../pkg6_sites.rs"]
mod pkg6_sites;
#[path = "../pkg6_smash_calls.rs"]
mod pkg6_smash_calls;
#[path = "../playback_hook.rs"]
mod playback_hook;
#[path = "../practice_menu_runtime.rs"]
mod practice_menu_runtime;
#[path = "../pristine_load.rs"]
mod pristine_load;
#[path = "../release_warnings.rs"]
mod release_warnings;
#[path = "../roster_request_handoff.rs"]
mod roster_request_handoff;
#[path = "../rows_tables.rs"]
mod rows_tables;
#[path = "../sav002_load_stops_rr_playback.rs"]
mod sav002_load_stops_rr_playback;
#[path = "../size_budget.rs"]
mod size_budget;
#[path = "../stack_marks.rs"]
mod stack_marks;
#[path = "../state01_json.rs"]
mod state01_json;
#[path = "../state01_machine.rs"]
mod state01_machine;
#[path = "../state01_setup.rs"]
mod state01_setup;
#[path = "../state02_charges.rs"]
mod state02_charges;
#[path = "../state03_buffs.rs"]
mod state03_buffs;
#[path = "../state04_items.rs"]
mod state04_items;
#[path = "../state05_restore.rs"]
mod state05_restore;
#[path = "../state06_07.rs"]
mod state06_07;
#[path = "../state_sites.rs"]
mod state_sites;
#[path = "../svc_clobber_abi.rs"]
mod svc_clobber_abi;
#[path = "../tech_delivery_boundary.rs"]
mod tech_delivery_boundary;
#[path = "../tech_hitlag.rs"]
mod tech_hitlag;
#[path = "../tech_hitlag_reader.rs"]
mod tech_hitlag_reader;
#[path = "../tech_info_seed.rs"]
mod tech_info_seed;
#[path = "../tech_launch.rs"]
mod tech_launch;
#[path = "../tech_product_contract.rs"]
mod tech_product_contract;
#[path = "../tech_reference.rs"]
mod tech_reference;
#[path = "../tech_spike_attack_accessor.rs"]
mod tech_spike_attack_accessor;
#[path = "../tech_stop_span.rs"]
mod tech_stop_span;
#[path = "../thread_stacks.rs"]
mod thread_stacks;
#[path = "../u4_sites.rs"]
mod u4_sites;
#[path = "../u4b_sites.rs"]
mod u4b_sites;
#[path = "../ui_chord_capture.rs"]
mod ui_chord_capture;
#[path = "../ui_owner.rs"]
mod ui_owner;
#[path = "../ui_readiness.rs"]
mod ui_readiness;
#[path = "../ui_sites.rs"]
mod ui_sites;
