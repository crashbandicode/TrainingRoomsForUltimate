//! Package 6, the roster cycle (worker A): hot character swap, the Mii and Kirby sources it uses, the preload, Kirby hats and
//! character items. One strongly connected cycle of carried, game-facing files (carried by `tools/carry_pkg6.py` from the fork's
//! stage; the module declarations here are ours). Switch only.
//!
//! `character_roster` declares its own children (`sources`, `body_bank`, the Mii files under `../mii/`, the install and probe
//! files) with the fork's `#[path]` lines, which the carry tool re-points; everything else of the cycle is declared here, flat. The
//! fork's `swap_native` directory is flat too: the manifest maps `swap_native::<x>` to `runtime::roster::<x>`. Feature gates are the
//! fork's (`swap_native/mod.rs`): the native-body-bank files need `native-body-bank-alpha`, the presentation readback
//! `mii-hud-portrait-alpha`.

/// Diag meter of the group build's game-thread time (stream f6; empty without `diag`).
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod build_split;
pub(crate) mod character_items;
pub(crate) mod character_lifecycle;
// Carried, not declared (no clean-room consumer, single owner): `effect_hook_chain` (the fork's chaining of the shared EffectModule
// entries: packages 2-4 hook those themselves). `heap_fields` (the fork's install-line field) is not carried: package 1's install
// line owns `heap_allocations=`. `heap_metrics` is the diag meter's reader (the byte meter of package 1's allocator).
pub(crate) mod character_roster;
/// F9: the hold between a CPU-count change and the Reload that follows the activation frame.
pub(crate) mod f9_gate;
pub(crate) mod cycle_preload;
pub(crate) mod cycle_preload_config;
#[cfg(feature = "hang-trace")]
pub(crate) mod hang_trace;
/// Without the `hang-trace` diag feature the four probes the carried callbacks hit are no-ops (the fork's disabled build: "Disabled
/// builds leave hook probes inactive"); the tracing code itself is only compiled with the feature.
#[cfg(not(feature = "hang-trace"))]
pub(crate) mod hang_trace {
    pub struct Probe;

    impl Probe {
        #[inline(always)]
        pub fn hit(&self) {}
    }

    pub static ROSTER_SERVICE: Probe = Probe;
    pub static ALLOCATION: Probe = Probe;
    pub static RELOAD: Probe = Probe;
    pub static DESTROY: Probe = Probe;
}
#[cfg(feature = "plugin-heap-meter")]
pub(crate) mod heap_metrics;

pub(crate) mod kirby_hats;
/// The lock-free runtime: what the menu worker asks of the roster, run on the game frame (ours, not carried).
pub(crate) mod roster_requests;
pub(crate) mod native_gc_guard;
pub(crate) mod native_unlink_install;
#[cfg(feature = "native-unlink-probe")]
pub(crate) mod native_unlink_probe;
pub(crate) mod practice_trace;
#[cfg(feature = "console-practice-trace")]
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
/// Operator option C (2026-11-09 17:57Z): the release watch's read of the game's resource unload (own code).
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod res_unload;
#[cfg(feature = "native-body-bank-alpha")]
pub(crate) mod special_articles;
#[cfg(feature = "mii-hud-portrait-alpha")]
pub(crate) mod presentation;

/// The rows of other packages' tables (and its own `roster_*` rows of `sites_pkg6`) that cover the `smash::` calls of the cycle's
/// release code (`tests/pkg6_smash_calls.rs` maps each call to its row). The cycle starts only when all of them passed their guard.
pub(crate) const CALLED_ROWS: [&str; 16] = [
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
    let _scope = crate::model::hook_stats::HookScope::enter_hook(crate::model::hook_stats::HookId::RosterInput);
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
    let _scope = crate::model::hook_stats::HookScope::enter_hook(crate::model::hook_stats::HookId::RosterInput);
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

/// The cycle started (every game function it calls passed its guard and boot called `init`): the owners of moments the cycle's
/// engines serve (package 3's save-state load hands Kirby's hat to the hat engine) check it before they ask.
static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// F9: the guarded rows of the No. of CPUs path (`sites_pkg6`): the game's root load, its reader of the field and the pause menu's
/// writer. `native_source::training_cpu_quantity` reads the field only when all three passed at boot.
pub(crate) const CPU_QUANTITY_ROWS: [&str; 3] = ["roster_cpu_quantity_root", "roster_cpu_quantity_reader", "roster_cpu_quantity_writer"];
static CPU_QUANTITY_GUARDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn set_cpu_quantity_guarded(ok: bool) {
    CPU_QUANTITY_GUARDED.store(ok, std::sync::atomic::Ordering::Release);
}

/// Whether the No. of CPUs field may be read (its rows passed their guards).
pub(crate) fn cpu_quantity_guarded() -> bool {
    CPU_QUANTITY_GUARDED.load(std::sync::atomic::Ordering::Acquire)
}

/// Whether the roster cycle is running.
pub(crate) fn running() -> bool {
    STARTED.load(std::sync::atomic::Ordering::Acquire)
}

/// The nro-load provider's registration function (`libnro_hook`, docs/HOOK-SHARING.md): one pointer-sized argument, the callback.
type AddNroLoadHook = extern "C" fn(usize);

/// The turnip identity read needs the loaded `item` module's image (every read re-checks its code, `held_face`): ask the provider to
/// tell us when it loads. Without the provider nothing registers and a turnip is saved as unsupported (the item engine's rule).
fn register_nro_load() {
    let mut address = 0usize;
    // SAFETY: the name is a NUL-terminated literal; the loader writes one address.
    unsafe { skyline::nn::ro::LookupSymbol(&mut address, b"add_nro_load_hook\0".as_ptr()) };
    if address == 0 {
        crate::engine::compat::logging::diagnostic(format_args!("[TrainingRooms] turnip identity: no nro-load provider"));
        return;
    }
    // SAFETY: the provider exports `add_nro_load_hook(callback)`.
    let add: AddNroLoadHook = unsafe { std::mem::transmute(address) };
    add(on_nro_load as *const () as usize);
}

extern "Rust" fn on_nro_load(info: &skyline::nro::NroInfo<'_>) {
    // SAFETY: called by the provider with the loader's own NRO record.
    unsafe { turnip_identity::on_load(info) };
}

/// Start the cycle: sync the preload cache from the already published settings.json before anything asks for a snapshot,
/// then the trace recorder, native patches (unlink guard, Lua GC guard), lifecycle observations and last the roster
/// itself, whose `init` installs the service callback, the sources, the body bank and the Mii gates. Each file checks its own seam
/// rows through `seams::seam_install::gate` and installs nothing when they are refused. Called once from boot, before the install
/// line.
pub(crate) fn init() {
    STARTED.store(true, std::sync::atomic::Ordering::Release);
    register_nro_load();
    cycle_preload_config::init();
    practice_trace::init();
    native_unlink_install::init();
    native_gc_guard::init();
    // Capture the full pristine load contract before reload_trace patches its interior.
    // Activation remains after the ordinary Mii guards in character_roster::init.
    #[cfg(feature = "managed-third-source-alpha")]
    let load_contract = character_roster::prepare_load_contract();
    character_lifecycle::init();
    character_roster::init(#[cfg(feature = "managed-third-source-alpha")] load_contract);
}
