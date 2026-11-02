//! Package 6, the roster cycle (worker A): hot character swap, the Mii and Kirby sources it uses, the preload, Kirby hats and
//! character items. One strongly connected cycle of carried, game-facing files (carried by `tools/carry_pkg6.py` from the fork's
//! stage; the module declarations here are ours). Switch only.
//!
//! `character_roster` declares its own children (`sources`, `body_bank`, the Mii files under `../mii/`, the install and probe
//! files) with the fork's `#[path]` lines, which the carry tool re-points; everything else of the cycle is declared here, flat. The
//! fork's `swap_native` directory is flat too: the manifest maps `swap_native::<x>` to `runtime::roster::<x>`. Feature gates are the
//! fork's (`swap_native/mod.rs`): the native-body-bank files need `native-body-bank-alpha`, the presentation readback
//! `mii-hud-portrait-alpha`.

pub(crate) mod character_items;
pub(crate) mod character_lifecycle;
pub(crate) mod character_roster;
pub(crate) mod cycle_preload;
pub(crate) mod cycle_preload_config;
pub(crate) mod effect_hook_chain;
pub(crate) mod hang_trace;
pub(crate) mod heap_fields;
#[cfg(feature = "plugin-heap-meter")]
pub(crate) mod heap_metrics;
pub(crate) mod kirby_hats;
pub(crate) mod native_gc_guard;
pub(crate) mod native_unlink_install;
pub(crate) mod native_unlink_probe;
pub(crate) mod practice_trace;
pub(crate) mod practice_trace_core;
pub(crate) mod scope_gate;
pub(crate) mod turnip_identity;

#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod body_graph;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod copy_packages;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod identity_view;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod mii_setup;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod observe;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod r_handle;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod special_articles;
#[cfg(feature = "mii-hud-portrait-alpha")]
pub(crate) mod presentation;

/// The rows of other packages' tables (and its own `roster_*` rows of `sites_pkg6`) that cover the `smash::` calls of the cycle's
/// release code (`tests/pkg6_smash_calls.rs` maps each call to its row). The cycle starts only when all of them passed their guard.
pub(crate) const CALLED_ROWS: [&str; 17] = [
    "state_get_have_item_id",
    "state_have_item",
    "state_is_have_item",
    "state_remove_item",
    "state_set_have_item_action",
    "state_work_get_int64",
    "state_object_is_active",
    "work_set_int",
    "work_is_flag",
    "utility_get_category",
    "posture_lr",
    "stop_is_stop",
    "roster_have_item_kind",
    "roster_posture_pos",
    "roster_status_is_changing",
    "roster_find_battle_object",
    "state_kirby_copy_setup",
];

/// Player 1's mapped input record before the input delay (called by the input stage for slot 0, on its poll): the roster's cycle
/// chords and the Kirby hat shortcut read and edit a copy of the record, which is written back. `controller` is the game's
/// `Controller` of player 0, or 0 when the poll passed none (the stick clicks then read as not pressed).
///
/// # Safety
/// `out` is the 8-byte record the game just wrote for player 0; `controller` is the live `Controller` pointer or 0.
pub(crate) unsafe fn handle_p1_input(out: *mut crate::model::input_delay::MappedInputs, controller: usize) {
    use crate::model::chords::{bit, field};
    let held = if controller == 0 { 0 } else { ((controller + field::CURRENT_BUTTONS) as *const u32).read_volatile() };
    let mut record = crate::runtime::compat::input::MappedInputs::from(*out);
    character_roster::handle_input(&mut record);
    kirby_hats::handle_input(&mut record, held & bit::STICK_L != 0, held & bit::STICK_R != 0);
    *out = record.into();
}

/// The mapped input record of the controller that operates the CPU under native Behavior = Control (called by the input stage for
/// slot 1, before anything else touches it): that controller's own CPU cycle chord queues the CPU role only, and the Kirby hat
/// shortcut of its stick clicks cycles the CPU's hat. Nothing happens unless the practice engine says this is the operator.
///
/// # Safety
/// `out` is the 8-byte record the game just wrote for slot 1; `controller` is the live `Controller` pointer or 0.
pub(crate) unsafe fn handle_operator_input(out: *mut crate::model::input_delay::MappedInputs, controller: usize) {
    use crate::model::chords::{bit, field};
    if !crate::runtime::lobby::training::practice::cpu_operator(1) {
        return;
    }
    let held = if controller == 0 { 0 } else { ((controller + field::CURRENT_BUTTONS) as *const u32).read_volatile() };
    let cycle = crate::runtime::features::chords::operator_cycle(controller);
    let mut record = crate::runtime::compat::input::MappedInputs::from(*out);
    character_roster::handle_cpu_operator_input(1, &mut record, cycle);
    kirby_hats::handle_cpu_operator_input(&mut record, held & bit::STICK_L != 0, held & bit::STICK_R != 0);
    *out = record.into();
}

/// Start the cycle, in the order the fork's `main` did: the preload cache first (the INI is read once, before anything asks for a
/// snapshot), the trace recorder, the native patches (unlink guard, Lua GC guard), the lifecycle observations and last the roster
/// itself, whose `init` installs the service callback, the sources, the body bank and the Mii gates. Each file checks its own seam
/// rows through `seams::seam_install::gate` and installs nothing when they are refused. Called once from boot, before the install
/// line.
pub(crate) fn init() {
    cycle_preload_config::init();
    practice_trace::init();
    native_unlink_install::init();
    native_gc_guard::init();
    character_lifecycle::init();
    character_roster::init();
}
