//! Switch side of the recording worker. Decisions live in `engine::rr_host`, which the host tests compile.

use std::path::Path;

use crate::engine::library::LegacySlot;
use crate::engine::macros::InputFrame;
use crate::engine::rr_host::{self, Tick};
use crate::model::input_delay::MappedInputs;

const NAME_TOKEN: u64 = 1 << 62;
const NOTICE_ID: u8 = 0x2C;
const NOTICE_COLOR: [u8; 4] = [255, 255, 255, 255];

pub fn on_chords(record: bool, play_player: bool, play_cpu: bool) {
    // A Record that starts keeps both fighters as they are now: the start save is where the macro starts
    // (RECORDING-REDESIGN, "Start save"), and the macro keeps the recorded role's speed (PLAY-MOMENTUM).
    if record && rr_host::capture_role() == 0 {
        // SAFETY: the chord step runs on the game thread (the AI epilogue), where the fighters' accessors are live.
        rr_host::note_record_start(unsafe { record_start() });
    }
    rr_host::on_chords(record, play_player, play_cpu);
}

/// Both fighters' live pose and summed speed. Atomics and kinetic reads only; no allocation.
///
/// # Safety
/// Game thread.
unsafe fn record_start() -> rr_host::RecordStart {
    rr_host::RecordStart {
        poses: [super::states::live_pose(0), super::states::live_pose(1)],
        speeds: [live_speed(0), live_speed(1)],
    }
}

/// A fighter's summed speed (`KineticModule::get_sum_speed_x/y`, the named layer's `snapshot_start` reading); at rest
/// before its first frame. The kinetic rows are the named-macro unit's (R2-1): read only once that unit's closure passed.
///
/// # Safety
/// Game thread.
unsafe fn live_speed(slot: usize) -> (f32, f32) {
    use smash::app::{lua_bind::KineticModule, BattleObjectModuleAccessor};
    use smash::lib::lua_const::KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN;
    let accessor = super::states::fighter_accessor(slot);
    if !crate::runtime::macros::ready() || accessor == 0 {
        return (0.0, 0.0);
    }
    let module = accessor as *mut BattleObjectModuleAccessor;
    let x = KineticModule::get_sum_speed_x(module, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN);
    let y = KineticModule::get_sum_speed_y(module, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN);
    // A start state must be finite (`StartState::validate`), or the whole recording is refused.
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    (finite(x), finite(y))
}

/// PLAY-MOMENTUM (primary 2026-11-08 23:03Z, reading (b)): on the game frame a run's first input after its warp is
/// injected, the playing role gets the start speed it is owed (`rr_host::take_start_speed`): the recorded speed with
/// "Keep recorded speed", zero with "Start at rest". As the named layer's `apply_ready_speed`: x relative to the facing it
/// was recorded with; in the air the control energy's x and the gravity's y, on the ground the motion energy's x.
///
/// # Safety
/// Game thread.
unsafe fn apply_start_speed(role: crate::engine::library::Role) {
    use smash::app::{lua_bind::*, BattleObjectModuleAccessor};
    use smash::lib::lua_const::*;
    let Some(start) = rr_host::take_start_speed(role) else { return };
    // The kinetic rows (kinetic_get_energy and the two set_speed thunks) are the named-macro unit's (R2-1).
    if !crate::runtime::macros::ready() {
        return;
    }
    let slot = match role {
        crate::engine::library::Role::Player => 0,
        crate::engine::library::Role::Cpu => 1,
    };
    let accessor = super::states::fighter_accessor(slot);
    if accessor == 0 {
        return;
    }
    let module = accessor as *mut BattleObjectModuleAccessor;
    let x = start.speed.0 * start.facing * PostureModule::lr(module);
    let air = StatusModule::situation_kind(module) == *SITUATION_KIND_AIR;
    if air {
        set_normal_speed(module, *FIGHTER_KINETIC_ENERGY_ID_CONTROL, x);
        let gravity = KineticModule::get_energy(module, *FIGHTER_KINETIC_ENERGY_ID_GRAVITY)
            as *mut smash::app::FighterKineticEnergyGravity;
        if !gravity.is_null() {
            FighterKineticEnergyGravity::set_speed(gravity, start.speed.1);
        }
    } else {
        set_normal_speed(module, *FIGHTER_KINETIC_ENERGY_ID_MOTION, x);
    }
    #[cfg(feature = "diag")]
    {
        crate::runtime::diag_log::diagnostic(format_args!(
            "[RecordingHost] start_speed slot={slot} air={air} x={x} y={} pos_x={} sum_x={} sum_y={}",
            start.speed.1,
            PostureModule::pos_x(module),
            KineticModule::get_sum_speed_x(module, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN),
            KineticModule::get_sum_speed_y(module, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN)
        ));
        OBSERVE[slot].store(8, core::sync::atomic::Ordering::Relaxed);
    }
}

/// Diag build: how many more injected frames of each role log their motion after a start speed (the PLAY-MOMENTUM Eden
/// row compares "Keep recorded speed" with "Start at rest").
#[cfg(feature = "diag")]
static OBSERVE: [core::sync::atomic::AtomicU8; 2] = [const { core::sync::atomic::AtomicU8::new(0) }; 2];

/// Diag build: one motion line of the role's run while `OBSERVE` lasts.
///
/// # Safety
/// Game thread.
#[cfg(feature = "diag")]
unsafe fn observe_start(role: crate::engine::library::Role) {
    use smash::app::{lua_bind::*, BattleObjectModuleAccessor};
    use smash::lib::lua_const::KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN;
    let slot = match role {
        crate::engine::library::Role::Player => 0,
        crate::engine::library::Role::Cpu => 1,
    };
    let left = OBSERVE[slot].load(core::sync::atomic::Ordering::Relaxed);
    let accessor = super::states::fighter_accessor(slot);
    if left == 0 || accessor == 0 || !crate::runtime::macros::ready() {
        return;
    }
    OBSERVE[slot].store(left - 1, core::sync::atomic::Ordering::Relaxed);
    let module = accessor as *mut BattleObjectModuleAccessor;
    crate::runtime::diag_log::diagnostic(format_args!(
        "[RecordingHost] start_observed slot={slot} n={} pos_x={} sum_x={} status={}",
        9 - left,
        PostureModule::pos_x(module),
        KineticModule::get_sum_speed_x(module, *KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN),
        StatusModule::status_kind(module)
    ));
}

#[cfg(not(feature = "diag"))]
unsafe fn observe_start(_role: crate::engine::library::Role) {}

/// # Safety
/// Game thread; `module` is a live fighter's accessor.
unsafe fn set_normal_speed(module: *mut smash::app::BattleObjectModuleAccessor, energy_id: i32, x: f32) {
    use smash::app::lua_bind::{KineticEnergyNormal, KineticModule};
    let energy = KineticModule::get_energy(module, energy_id) as *mut smash::app::KineticEnergyNormal;
    if !energy.is_null() {
        KineticEnergyNormal::set_speed(energy, &smash::phx::Vector2f { x, y: 0.0 });
    }
}

pub fn cpu_role_after_chord(facing: i8) {
    rr_host::cpu_role_after_chord(facing);
}

/// # Safety
/// `out` is null or the live mapped-input record for this poll.
pub unsafe fn cpu_role_poll(out: *mut MappedInputs) -> bool {
    if out.is_null() {
        return false;
    }
    rr_host::cpu_role_poll(unsafe { &mut *out })
}

pub fn cpu_role_control(ctx: &crate::model::recording::ControlCtx) -> crate::model::recording::ControlOut {
    rr_host::cpu_role_control(ctx)
}

pub fn cpu_role_phase() -> Option<crate::model::recording::Phase> {
    rr_host::cpu_role_phase()
}

pub fn playback_controller(frame: InputFrame) -> crate::model::recording::Inject {
    rr_host::playback_controller(frame)
}

pub fn note_mash() {
    rr_host::note_mash();
}

pub fn cpu_playback(
    recorder: crate::model::cpu::playback::PlaybackState,
) -> crate::model::cpu::playback::PlaybackState {
    rr_host::cpu_playback(recorder)
}

pub fn push_capture(side: u8, frame: InputFrame) {
    rr_host::push_capture(side, frame);
}

/// The CPU run's frame for this game frame (the AI epilogue, game thread); its first one after a warp gets the start speed.
pub(crate) fn take_cpu_frame() -> Option<InputFrame> {
    let frame = rr_host::take_cpu_frame();
    if frame.is_some() {
        // SAFETY: called from the AI epilogue on the game thread (`recorder::host_cpu`).
        unsafe {
            observe_start(crate::engine::library::Role::Cpu);
            apply_start_speed(crate::engine::library::Role::Cpu);
        }
    }
    frame
}

/// # Safety
/// `out` is null or a live mapped-input record for this poll.
pub unsafe fn on_player_poll(out: *mut MappedInputs) {
    if rr_host::on_player_poll(out) {
        observe_start(crate::engine::library::Role::Player);
        apply_start_speed(crate::engine::library::Role::Player);
    }
}

pub fn service() {
    let root = Path::new(crate::model::storage::ROOT);
    if !root.is_dir() {
        let _ = std::fs::create_dir_all(root);
        if !root.is_dir() {
            return;
        }
    }
    take_mismatch_answer();
    let player = super::states::live_saved(0);
    let cpu = super::states::live_saved(1);
    if player.fighter_kind >= 0 {
        rr_host::set_role_fighter(crate::engine::library::Role::Player, player.fighter_kind);
    }
    if cpu.fighter_kind >= 0 {
        rr_host::set_role_fighter(crate::engine::library::Role::Cpu, cpu.fighter_kind);
    }
    rr_host::set_strip_sample(super::combo::strip_sample());
    if player.fighter_kind >= 0 {
        rr_host::set_fighter_label(&crate::runtime::macros::named_macros::fighter_label(player.fighter_kind));
    }
    let online = keyboard_closed();
    // Migration waits for the old slot image. A record, a play, or a menu effect does not.
    if !rr_host::migrated() {
        if let Some(slots) = super::recorder::copied_slots() {
            let tick = rr_host::service_at(root, &slots, &rr_host::today(), player, cpu, online);
            if rr_host::migrated() {
                super::recorder::release_slot_image();
            }
            apply(&tick);
            sync_bound(root);
            return;
        }
    }
    sync_bound(root);
    if !rr_host::pending() {
        return;
    }
    let tick = rr_host::service_pending(root, player, cpu, online);
    apply(&tick);
}

/// Save writes the bound save; the save-state machine's active slot holds the bound save (RECORDING.md, "Save states
/// only warp").
fn sync_bound(root: &Path) {
    rr_host::write_saved(root);
    if let Some(pair) = rr_host::bound_sync(root) {
        super::states::post_bound(pair);
    }
}

fn take_mismatch_answer() {
    let Some(choice) = crate::runtime::macros::named_macros::take_recording_fighter_choice() else {
        return;
    };
    let answer = match choice {
        Some(crate::engine::macro_ui::macro_panel::FighterChoice::Switch) => {
            rr_host::MismatchChoice::Switch
        }
        Some(crate::engine::macro_ui::macro_panel::FighterChoice::Keep) => rr_host::MismatchChoice::Keep,
        None => rr_host::MismatchChoice::Cancel,
    };
    rr_host::answer_mismatch(answer);
}

fn apply(tick: &Tick) {
    if let Some(ask) = &tick.mismatch {
        let recorded = if ask.recorded_kind >= 0 {
            crate::runtime::macros::named_macros::fighter_label(ask.recorded_kind)
        } else {
            ask.recorded_label.clone()
        };
        let current = if ask.current_kind >= 0 {
            crate::runtime::macros::named_macros::fighter_label(ask.current_kind)
        } else {
            ask.current_label.clone()
        };
        let _ = crate::runtime::macros::named_macros::open_recording_fighter_prompt(
            crate::engine::macro_ui::macro_panel::FighterOrigin::Recording,
            recorded,
            current,
        );
    }
    if let Some(kind) = tick.switch_kind {
        let requested = if tick.switch_player {
            crate::runtime::roster::character_roster::request_player_kind(kind as u32)
        } else {
            crate::runtime::roster::character_roster::request_cpu_kind(kind as u32)
        };
        let _ = requested;
    }
    if let Some(name) = &tick.switch_name {
        if let Some(kind) = (0..128u32).find(|kind| {
            crate::runtime::macros::named_macros::fighter_label(*kind as i32) == name.as_str()
        }) {
            let _ = crate::runtime::roster::character_roster::request_player_kind(kind);
        }
    }
    if let Some(id) = &tick.warp {
        let root = std::path::Path::new(crate::model::storage::ROOT);
        let saves = crate::model::store::Store::<crate::engine::library::NamedSave>::new(
            root.join(crate::engine::library::SAVES_DIR),
        );
        if let Ok(save) = saves.load(id) {
            // Edit placement's offset for the playing role (primary 2026-11-08 #70 (a)).
            super::states::request_named_load(rr_host::warp_pair(save, tick.warp_place));
        } else {
            // No restore was asked for, so none will land.
            rr_host::warp_landed();
        }
    }
    for text in &tick.notices {
        super::notices::upsert(NOTICE_ID, "Recording", text, 240, NOTICE_COLOR);
    }
    if let Some(offer) = &tick.keyboard {
        crate::runtime::macros::named_macros::offer_external_name_titled(
            NAME_TOKEN,
            offer.header,
            &offer.draft,
            "Name",
        );
    }
    emit(&tick.diags);
}

fn emit(lines: &[String]) {
    #[cfg(feature = "diag")]
    {
        for line in lines {
            crate::runtime::diag_log::diagnostic(format_args!("[PracticeRecording] {line}"));
        }
    }
    #[cfg(not(feature = "diag"))]
    let _ = lines;
}

pub(crate) fn keyboard_is_closed() -> bool {
    keyboard_closed()
}

pub(crate) fn take_typed_name() -> Option<String> {
    crate::runtime::macros::named_macros::take_confirmed_name_for(NAME_TOKEN)
}

fn keyboard_closed() -> bool {
    let mode = crate::runtime::lobby::game_mode::current()
        .map(|mode| mode.mode)
        .unwrap_or(0);
    !crate::model::scene::combo_name_keyboard(crate::runtime::ui::facts::fact_waiting_scene(), mode)
}
