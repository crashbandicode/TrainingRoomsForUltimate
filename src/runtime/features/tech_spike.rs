//! S0 diagnostic launcher. A rewritten ordinary CPU hit keeps the whole native damage path.
//! diag-tech-s0.req: real-hit | direct-post[-log|-full] | ordinary | direct-match | direct | walk | off | inspect.
//! No product mode, heap, blocking lock or new hook.
use super::tech_spike_guards;
use crate::model::cpu::hash::lua_bind_hash;
use crate::model::cpu::role::FrameReceipt;
use crate::model::tech_launch::{self as math, Vec2};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives::{self as state, Vector3f};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

static REQUEST: AtomicU8 = AtomicU8::new(0);
static GUARDED: AtomicU8 = AtomicU8::new(0);
static V1_GUARDED: AtomicU8 = AtomicU8::new(0);
static OBSERVING: AtomicBool = AtomicBool::new(false);
static NEXT_ATTEMPT: AtomicU32 = AtomicU32::new(1);
static RUN: hos_lock::Mutex<Spike> = hos_lock::Mutex::new(Spike::new());

pub fn request(text: &str) {
    let command = match text {
        "ordinary" => 1,
        "direct-match" => 2,
        "direct" => 3,
        "walk" => 4,
        "off" => 5,
        "inspect" => 6,
        "real-hit" => 7,
        "direct-post" => 8,
        "direct-post-log" => 9,
        "direct-post-full" => 10,
        _ => {
            diagnostic(format_args!("[TechDirect] request invalid"));
            return;
        }
    };
    REQUEST.store(command, Ordering::Release);
    diagnostic(format_args!(
        "[TrainingRooms] tech_s0_request command={command}"
    ));
}

#[derive(Clone, Copy)]
struct Input {
    origin: Vec2,
    direction: Vec2,
    kb: f32,
    angle: f32,
    percent: f32,
    weight: f32,
    lr: f32,
}
impl Input {
    const fn empty() -> Self {
        Self {
            origin: Vec2 { x: 0.0, y: 0.0 },
            direction: Vec2 { x: 0.0, y: 0.0 },
            kb: 0.0,
            angle: 0.0,
            percent: 0.0,
            weight: 0.0,
            lr: 1.0,
        }
    }
}
#[derive(Clone, Copy)]
struct Baseline {
    input: Input,
    hidden: Hidden,
    attempt: u32,
    scope: FrameReceipt,
    complete: bool,
}
/// Owned 13.0.5 energy layout and runtime-resolved work keys; no DamageInfo bytes are copied.
#[derive(Clone, Copy)]
struct Hidden {
    vptr: usize,
    reset_type: i32,
    enabled: u8,
    counter: u32,
    b8: u16,
    ba: u8,
    bb: u8,
    speed: [u32; 2],
    accel: [u32; 2],
    brake: [u32; 2],
    limit: [u32; 2],
    alternate: Option<([u32; 2], i32)>,
    floats: [f32; 3],
    flags: [bool; 4],
    motion: MotionContext,
}
/// Diagnostic observations only. Reading module state does not trace the
/// common module's instruction paths or reproduce its pending-motion effects.
#[derive(Clone, Copy)]
struct MotionContext {
    actual: u64,
    pending: u64,
    frame: u32,
    rate: u32,
    damage_rate: u32,
    init_reaction: u32,
    rotation: u32,
    roll_set_angle: bool,
    pierce_ground: bool,
    stopped_damage: bool,
    cursor: bool,
    cursor_frame: i32,
    paralyze_frame: i32,
}
unsafe fn motion_context(accessor: usize) -> Option<MotionContext> {
    let damage_stop: extern "C" fn(usize) -> bool =
        std::mem::transmute(game::text_base() + 0x2089220);
    let rate: extern "C" fn(usize) -> f32 = std::mem::transmute(game::text_base() + 0x205cc30);
    Some(MotionContext {
        actual: natives::motion_kind(accessor),
        pending: state::work_get_int64(accessor, id("FIGHTER_STATUS_DAMAGE_WORK_INT_MOTION_KIND")?)
            as u64,
        frame: natives::motion_frame(accessor).to_bits(),
        rate: rate(accessor).to_bits(),
        damage_rate: natives::get_float(
            accessor,
            id("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_DAMAGE_MOTION_RATE")?,
        )
        .to_bits(),
        init_reaction: natives::get_float(
            accessor,
            id("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_REACTION_FRAME")?,
        )
        .to_bits(),
        rotation: natives::get_float(accessor, id("FIGHTER_STATUS_DAMAGE_WORK_FLOAT_ROT_ANGLE")?)
            .to_bits(),
        roll_set_angle: natives::is_flag(
            accessor,
            id("FIGHTER_STATUS_DAMAGE_FLAG_FLY_ROLL_SET_ANGLE")?,
        ),
        pierce_ground: natives::is_flag(
            accessor,
            id("FIGHTER_INSTANCE_WORK_ID_FLAG_PIERCE_GROUND_DAMAGE")?,
        ),
        stopped_damage: damage_stop(accessor),
        cursor: natives::is_flag(accessor, id("FIGHTER_INSTANCE_WORK_ID_FLAG_CURSOR")?),
        cursor_frame: natives::get_int(accessor, id("FIGHTER_INSTANCE_WORK_ID_INT_CURSOR_FRAME")?),
        paralyze_frame: natives::get_int(
            accessor,
            id("FIGHTER_INSTANCE_WORK_ID_INT_INVALID_PARALYZE_FRAME")?,
        ),
    })
}
unsafe fn float_ids() -> Option<[i32; 3]> {
    Some([
        id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")?,
        id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME_LAST")?,
        id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_SPEED_UP_MAX_MAG")?,
    ])
}
unsafe fn flag_ids() -> Option<[i32; 4]> {
    Some([
        id("FIGHTER_INSTANCE_WORK_ID_FLAG_DAMAGE_SPEED_UP")?,
        id("FIGHTER_INSTANCE_WORK_ID_FLAG_DAMAGE_FLY_AIR")?,
        id("FIGHTER_STATUS_DAMAGE_FLAG_END_REACTION")?,
        id("FIGHTER_STATUS_DAMAGE_FLAG_GROUND")?,
    ])
}
unsafe fn hidden(accessor: usize) -> Option<Hidden> {
    let e = energy(accessor)?;
    let vptr = (e as *const usize).read();
    let offset = vptr.checked_sub(game::text_base())?;
    if offset != 0x4f88968 && offset != 0x4f88a18 {
        return None;
    }
    // Only the alternate energy has these trailing fields. Its virtual speed
    // getter selects +0xc0 while the signed +0xd0 counter is positive.
    let alternate = (offset == 0x4f88a18).then(|| {
        (
            ((e + 0xc0) as *const [u32; 2]).read_unaligned(),
            ((e + 0xd0) as *const i32).read(),
        )
    });
    Some(Hidden {
        vptr,
        reset_type: ((e + 0xa0) as *const i32).read(),
        enabled: ((e + 0x30) as *const u8).read(),
        counter: ((e + 0xb4) as *const u32).read(),
        b8: ((e + 0xb8) as *const u16).read(),
        ba: ((e + 0xba) as *const u8).read(),
        bb: ((e + 0xbb) as *const u8).read(),
        speed: ((e + 0x10) as *const [u32; 2]).read_unaligned(),
        accel: ((e + 0x40) as *const [u32; 2]).read_unaligned(),
        brake: ((e + 0x60) as *const [u32; 2]).read_unaligned(),
        limit: ((e + 0x70) as *const [u32; 2]).read_unaligned(),
        alternate,
        floats: float_ids()?.map(|key| natives::get_float(accessor, key)),
        flags: flag_ids()?.map(|key| natives::is_flag(accessor, key)),
        motion: motion_context(accessor)?,
    })
}
unsafe fn log_hidden(run: &Spike, accessor: usize, moment: &str) -> Option<Hidden> {
    let h = hidden(accessor)?;
    let kind = match run.mode {
        1 | 7 => "ordinary",
        2 => "match",
        _ => "fresh",
    };
    let (alternate_speed, alternate_frames) = h.alternate.unwrap_or(([0; 2], 0));
    diagnostic(format_args!(
        "[TechDirect] hidden attempt={} mode={} kind={kind} moment={moment} frame={} age={} vptr=0x{:x} vptr_offset=0x{:x} reset={} enabled={} b4={:08x} b8={:04x} ba={:02x} bb={:02x} speed=({:08x},{:08x}) accel=({:08x},{:08x}) brake=({:08x},{:08x}) limit=({:08x},{:08x}) alt_present={} alt_speed=({:08x},{:08x}) alt_frames={} reaction={:08x} reaction_last={:08x} speed_up={} speed_up_max={:08x} fly_air={} end_reaction={} ground={} status={} grounded={}",
        run.attempt,run.mode_name(),run.sample,run.age,h.vptr,h.vptr.wrapping_sub(game::text_base()),
        h.reset_type,h.enabled,h.counter,h.b8,h.ba,h.bb,
        h.speed[0],h.speed[1],h.accel[0],h.accel[1],h.brake[0],h.brake[1],h.limit[0],h.limit[1],
        h.alternate.is_some(),alternate_speed[0],alternate_speed[1],alternate_frames,
        h.floats[0].to_bits(),h.floats[1].to_bits(),h.flags[0],h.floats[2].to_bits(),
        h.flags[1],h.flags[2],h.flags[3],natives::status_kind(accessor),natives::is_grounded(accessor)
    ));
    let m = h.motion;
    diagnostic(format_args!(
        "[TechDirect] context attempt={} mode={} kind={kind} moment={moment} frame={} age={} motion_actual={:016x} motion_pending={:016x} motion_frame={:08x} motion_rate={:08x} damage_motion_rate={:08x} init_reaction={:08x} rotation={:08x} roll_set_angle={} pierce_ground={} stopped_damage={} stopped={} cursor={} cursor_frame={} paralyze_frame={} paths=fe844/ffd90/fe99c/feb84/fef5c path_execution=UNOBSERVED reset_type={} type5_info_read=UNOBSERVED",
        run.attempt,run.mode_name(),run.sample,run.age,m.actual,m.pending,m.frame,m.rate,
        m.damage_rate,m.init_reaction,m.rotation,m.roll_set_angle,m.pierce_ground,m.stopped_damage,
        natives::is_stop(accessor),m.cursor,m.cursor_frame,m.paralyze_frame,h.reset_type
    ));
    Some(h)
}
unsafe fn apply_derived(accessor: usize, observed: Hidden) -> Option<()> {
    for (key, value) in float_ids()?.into_iter().zip(observed.floats) {
        natives::set_float(accessor, value, key);
    }
    for (key, value) in flag_ids()?.into_iter().zip(observed.flags) {
        natives::set_flag(accessor, value, key);
    }
    Some(())
}
struct Spike {
    active: bool,
    heal_pending: bool,
    mode: u8,
    age: u32,
    phase: u8,
    attempt: u32,
    sample: u32,
    started: bool,
    arrange_cpu: bool,
    contact: bool,
    inspecting: bool,
    inspect_last: Option<(u32, u32, i32)>,
    player_scope: Option<FrameReceipt>,
    input: Input,
    baseline: Option<Baseline>,
    rewritten: u32,
    launch_reaction: f32,
}
impl Spike {
    const fn new() -> Self {
        Self {
            active: false,
            heal_pending: false,
            mode: 0,
            age: 0,
            phase: 0,
            attempt: 0,
            sample: 0,
            started: false,
            arrange_cpu: false,
            contact: false,
            inspecting: false,
            inspect_last: None,
            player_scope: None,
            input: Input::empty(),
            baseline: None,
            rewritten: 0,
            launch_reaction: 0.0,
        }
    }
    fn mode_name(&self) -> &'static str {
        if self.mode == 1 || self.mode == 7 {
            "ordinary"
        } else {
            "direct"
        }
    }
    fn post_init(&self) -> bool {
        matches!(self.mode, 8..=10)
    }
    fn stop(&mut self) {
        self.active = false;
        OBSERVING.store(false, Ordering::Release);
    }
}
unsafe fn id(name: &str) -> Option<i32> {
    game::lua_const(lua_bind_hash(name))
}
unsafe fn is_status(status: i32, name: &str) -> bool {
    id(name) == Some(status)
}
unsafe fn tumble(status: i32) -> bool {
    [
        "FIGHTER_STATUS_KIND_DAMAGE_FLY",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_ROLL",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_METEOR",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_D",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_U",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_LR",
    ]
    .iter()
    .any(|name| is_status(status, name))
}
unsafe fn guarded() -> bool {
    match GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut all = base != 0;
    if all {
        for &(name, offset, expected) in tech_spike_guards::SITES {
            let ok =
                std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected.as_slice();
            diagnostic(format_args!(
                "[TrainingRooms] tech_s0_guard site={name} ok={ok}"
            ));
            all &= ok;
        }
    }
    // Existing native wrappers must also be granted; none silently substitutes a missing writer.
    for offset in [
        crate::model::sites_pkg3::STATE_CLEAR_SPEED_ALL_OFFSET,
        crate::model::sites_pkg3::STATE_INIT_POS_OFFSET,
        crate::model::sites_pkg3::STATE_SET_LR_OFFSET,
        crate::model::sites_pkg3::STATE_HEAL_OFFSET,
        crate::model::sites_pkg3::STATE_ADD_DAMAGE_OFFSET,
        crate::model::sites_pkg3::STATE_SET_SITUATION_KIND_OFFSET,
        crate::model::sites_pkg3::STATE_WORK_GET_INT64_OFFSET,
        crate::model::sites_pkg2::CHANGE_STATUS_FORCE_OFFSET,
        crate::model::sites_pkg2::WORK_SET_FLOAT_OFFSET,
        crate::model::sites_pkg2::WORK_GET_FLOAT_OFFSET,
        crate::model::sites_pkg2::WORK_IS_FLAG_OFFSET,
        crate::model::sites_pkg2::WORK_SET_FLAG_OFFSET,
        crate::model::sites_pkg2::WORK_GET_INT_OFFSET,
        crate::model::sites_pkg2::MOTION_KIND_OFFSET,
        crate::model::sites_pkg2::MOTION_FRAME_OFFSET,
        crate::model::sites_pkg2::STAGE_ID_CALL_OFFSET,
        crate::model::sites_pkg2::STAGE_TABLE_OFFSET,
        crate::model::sites_pkg2::STATUS_KIND_OFFSET,
        crate::model::sites_pkg2::SITUATION_KIND_OFFSET,
        crate::model::sites_pkg2::POS_X_OFFSET,
        crate::model::sites_pkg2::POS_Y_OFFSET,
        crate::model::sites_pkg2::WORK_GET_PARAM_FLOAT_OFFSET,
        crate::model::sites_pkg2::DAMAGE_OF_OFFSET,
        crate::model::sites_pkg2::UTILITY_GET_KIND_OFFSET,
        crate::model::sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET,
        crate::model::sites_pkg4::POSTURE_LR_OFFSET,
        crate::model::sites_pkg4::STOP_IS_STOP_OFFSET,
        crate::model::sites_hitbox::HITBOX_DAMAGE_REACTION_OFFSET,
        crate::model::sites_hitbox::HITBOX_ACCESSOR_OFFSET,
        crate::model::sites_hitbox::HITBOX_IS_ATTACK_OFFSET,
        crate::model::sites_hitbox::HITBOX_ATTACK_DATA_OFFSET,
        crate::model::sites_hitbox::HITBOX_POWER_MUL_OFFSET,
    ] {
        all &= game::granted(offset);
    }
    all &= super::cpu_common::passive_ready();
    GUARDED.store(if all { 1 } else { 2 }, Ordering::Release);
    all
}

unsafe fn v1_guarded() -> bool {
    match V1_GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut all = base != 0;
    if all {
        for &(offset, expected) in super::tech_direct_v1_guards::DIRECT_V1_GUARDS {
            let ok = std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected;
            diagnostic(format_args!(
                "[TrainingRooms] tech_s0_guard site=direct_v1_{offset:x} ok={ok}"
            ));
            all &= ok;
        }
    }
    V1_GUARDED.store(if all { 1 } else { 2 }, Ordering::Release);
    all
}

/// Check the concrete Damage/Work virtual targets used by the new native calls.
unsafe fn v1_damage_module(accessor: usize) -> Option<()> {
    let base = game::text_base();
    for &(module_offset, slot, expected) in super::tech_direct_v1_guards::DIRECT_V1_VIRTUAL_TARGETS
    {
        let module = ((accessor + module_offset) as *const usize).read();
        if module == 0 || module & 7 != 0 {
            return None;
        }
        let vptr = (module as *const usize).read();
        if vptr == 0 || vptr & 7 != 0 {
            return None;
        }
        let target = ((vptr + slot) as *const usize).read();
        if !expected.iter().any(|&offset| target == base + offset) {
            diagnostic(format_args!(
                "[TechDirect] fault=v1_damage_target slot={slot:x} actual=0x{target:x}"
            ));
            return None;
        }
    }
    Some(())
}

fn complete_baseline(run: &mut Spike) {
    if !matches!(run.mode, 1 | 7) || run.sample < 20 || !run.contact {
        return;
    }
    let Some(baseline) = run.baseline.as_mut().filter(|b| !b.complete) else {
        return;
    };
    baseline.complete = true;
    let input = baseline.input;
    diagnostic(format_args!("[TechDirect] baseline_complete attempt={} command={} kb={:08x} direction_x={:08x} direction_y={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x} ground_at_hit={}",
        baseline.attempt,run.mode,input.kb.to_bits(),input.direction.x.to_bits(),input.direction.y.to_bits(),
        input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits(),baseline.hidden.flags[3]));
}

unsafe fn post_init_v1(run: &Spike, accessor: usize) -> Option<()> {
    if run.mode != 10 {
        return Some(());
    }
    let reaction_ids = float_ids()?;
    let flags = flag_ids()?;
    let wanted = [run.launch_reaction - 1.0, run.launch_reaction];
    let mut writes = 0u8;
    for (index, value) in wanted.into_iter().enumerate() {
        let key = reaction_ids[index];
        if natives::get_float(accessor, key).to_bits() != value.to_bits() {
            natives::set_float(accessor, value, key);
            writes |= 1 << index;
        }
    }
    for (index, value) in [(1usize, true), (2usize, false)] {
        if natives::is_flag(accessor, flags[index]) != value {
            natives::set_flag(accessor, value, flags[index]);
            writes |= 1 << (index + 1);
        }
    }
    diagnostic(format_args!("[TechDirect] derived attempt={} command={} writes={writes:02x} wanted_reaction={:08x} wanted_last={:08x} fly_air=true end_reaction=false",
        run.attempt,run.mode,wanted[0].to_bits(),wanted[1].to_bits()));

    v1_damage_module(accessor)?;
    let baseline = run.baseline?;
    // The native common helper passes math_toint(status reaction), then
    // as_number(), to this bool predicate. It does not return MAX_MAG.
    let observed_init = f32::from_bits(baseline.hidden.motion.init_reaction);
    if !observed_init.is_finite() || observed_init < 0.0 || observed_init > i32::MAX as f32 {
        return None;
    }
    let predicate_input = (observed_init as i64) as f32;
    let eligible: extern "C" fn(usize, f32) -> bool =
        std::mem::transmute(game::text_base() + 0x693e50);
    let speed_up = eligible(accessor, predicate_input);
    // S0's explicitly permitted same-boot seed: this is the game's own
    // MAX_MAG for the identical R1 input, never a guessed 2.0 multiplier.
    // The generic product calculation remains a follow-up to the trace.
    // The normal common helper's false branch explicitly clears MAX_MAG.
    let max_mag = if speed_up {
        baseline.hidden.floats[2]
    } else {
        0.0
    };
    if !max_mag.is_finite() {
        return None;
    }
    if natives::get_float(accessor, reaction_ids[2]).to_bits() != max_mag.to_bits() {
        natives::set_float(accessor, max_mag, reaction_ids[2]);
    }
    if natives::is_flag(accessor, flags[0]) != speed_up {
        natives::set_flag(accessor, speed_up, flags[0]);
    }
    diagnostic(format_args!("[TechDirect] speed_up_seed attempt={} command={} baseline_attempt={} predicate_reaction={:08x} baseline_init_reaction={:08x} calc_reaction={:08x} eligible={speed_up} baseline_eligible={} max_mag={:08x} mag_source=baseline_native inherited_info=true",
        run.attempt,run.mode,baseline.attempt,predicate_input.to_bits(),observed_init.to_bits(),run.launch_reaction.to_bits(),baseline.hidden.flags[0],max_mag.to_bits()));
    Some(())
}
/// The existing ATTACK hook calls this AFTER the game's command, before returning to ACMD.
/// Restrict edits to this attempt's fresh ordinary CPU accessor. Never copy AttackData or
/// manufacture a damage log, status, kinetic energy or collision result.
pub unsafe fn attack(lua_state: usize) {
    if !OBSERVING.load(Ordering::Acquire) {
        return;
    }
    let Some(mut run) = RUN.try_lock() else {
        return;
    };
    if !run.active || run.mode != 7 || run.baseline.is_some() || !guarded() {
        return;
    }
    if !crate::runtime::scene::is_native_training() {
        return;
    }
    let accessor = natives::lua_accessor(lua_state);
    if accessor == 0 || natives::module_accessor(1) != Some(accessor) {
        return;
    }
    let base = game::text_base();
    let module = ((accessor + 0xa0) as *const usize).read();
    if module == 0 {
        return;
    }
    let vptr = (module as *const usize).read();
    for (slot, offset) in [
        (0xf8, 0x3dd2f0),
        (0x108, 0x3dd360),
        (0x178, 0x3dd7d0),
        (0x188, 0x3dd830),
        (0x198, 0x3dd890),
    ] {
        if ((vptr + slot) as *const usize).read() != base + offset {
            diagnostic(format_args!("[TechHit] fault=setter_target slot={slot:x}"));
            run.stop();
            return;
        }
    }
    let power: extern "C" fn(usize, i32, f32, bool) = std::mem::transmute(base + 0x1fd0270);
    let angle: extern "C" fn(usize, i32, i32, bool) = std::mem::transmute(base + 0x1fd02b0);
    let growth: extern "C" fn(usize, i32, i32, bool) = std::mem::transmute(base + 0x1fd03c0);
    let fixed: extern "C" fn(usize, i32, i32, bool) = std::mem::transmute(base + 0x1fd0400);
    let knockback: extern "C" fn(usize, i32, i32, bool) = std::mem::transmute(base + 0x1fd0440);
    for slot in 0..crate::model::spots::SLOTS as i32 {
        if !natives::attack_is_attack(accessor, slot) {
            continue;
        }
        power(accessor, slot, 1.0, false);
        angle(accessor, slot, 90, false);
        growth(accessor, slot, 0, false);
        fixed(accessor, slot, 1, false);
        knockback(accessor, slot, 100, false);
        let data = natives::attack_data(accessor, slot);
        if data == 0 {
            diagnostic(format_args!("[TechHit] fault=slot_readback slot={slot}"));
            run.stop();
            return;
        }
        let damage = ((data + 0x20) as *const f32).read();
        let values = ((data + 0x28) as *const [i32; 4]).read_unaligned();
        let p1_damage = natives::module_accessor(0).map_or(f32::NAN, |a| natives::damage(a, 0));
        diagnostic(format_args!("[TechHit] rewrite attempt={} slot={slot} cpu_kind={} motion={:016x} motion_frame={:08x} damage={:08x} angle={} growth={} fixed={} base={} power_mul={:08x} p1_before={:08x}",
            run.attempt,natives::fighter_kind(accessor).unwrap_or(-1),natives::motion_kind(accessor),natives::motion_frame(accessor).to_bits(),damage.to_bits(),values[0],values[1],values[2],values[3],natives::attack_power_mul(accessor).to_bits(),p1_damage.to_bits()));
        if damage.to_bits() != 1.0f32.to_bits() || values != [90, 0, 1, 100] {
            diagnostic(format_args!("[TechHit] fault=setter_readback slot={slot}"));
            run.stop();
            return;
        }
        run.rewritten = run.rewritten.saturating_add(1);
    }
}
unsafe fn energy(accessor: usize) -> Option<usize> {
    let get: extern "C" fn(usize, i32) -> usize =
        std::mem::transmute(game::text_base() + 0x2043970);
    let value = get(accessor, id("FIGHTER_KINETIC_ENERGY_ID_DAMAGE")?);
    (value != 0 && value & 7 == 0).then_some(value)
}
unsafe fn velocity(accessor: usize) -> Option<Vec2> {
    let energy = energy(accessor)?;
    let x: extern "C" fn(usize) -> f32 = std::mem::transmute(game::text_base() + 0x20f6460);
    let y: extern "C" fn(usize) -> f32 = std::mem::transmute(game::text_base() + 0x20f6480);
    let v = Vec2 {
        x: x(energy),
        y: y(energy),
    };
    (v.x.is_finite() && v.y.is_finite()).then_some(v)
}
unsafe fn reaction(accessor: usize) -> Option<f32> {
    Some(natives::get_float(
        accessor,
        id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")?,
    ))
}
unsafe fn place(accessor: usize, input: Input) {
    state::clear_speed_all(accessor);
    state::init_pos(
        accessor,
        &Vector3f {
            x: input.origin.x,
            y: input.origin.y,
            z: 0.0,
        },
    );
    state::set_lr(accessor, input.lr);
}
unsafe fn setup(run: &Spike, accessor: usize) {
    let stage = natives::stage_id();
    let form = natives::stage_form(stage).unwrap_or(u32::MAX);
    let input = run.input;
    let baseline_attempt = run.baseline.map_or(0, |b| b.attempt);
    diagnostic(format_args!("[TechDirect] setup attempt={} mode={} baseline_attempt={baseline_attempt} p1_kind={} stage={stage} form={form} kb={:08x} angle={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x}",
        run.attempt,run.mode_name(),natives::fighter_kind(accessor).unwrap_or(-1),input.kb.to_bits(),input.angle.to_bits(),
        input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits()));
}
unsafe fn launch(run: &mut Spike, accessor: usize) -> Option<()> {
    let input = run.input;
    let calc: extern "C" fn(f32) -> f32 = std::mem::transmute(game::text_base() + 0x404ec0);
    let native_reaction = calc(input.kb);
    let reference =
        math::reaction_frames(input.kb, math::DAMAGE_FRAME_MUL, math::DAMAGE_FRAME_MIN)?;
    diagnostic(format_args!(
        "[TechDirect] reaction native={:08x} reference={:08x} equal={}",
        native_reaction.to_bits(),
        reference.to_bits(),
        native_reaction.to_bits() == reference.to_bits()
    ));
    if native_reaction.to_bits() != reference.to_bits() {
        return None;
    }
    run.launch_reaction = native_reaction;
    let reaction_id = id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")?;
    place(accessor, input);
    state::heal(accessor, -natives::damage(accessor, 0));
    state::add_damage(accessor, input.percent);
    let velocity = math::launch_velocity(input.kb, input.direction)?;
    #[repr(C, align(16))]
    struct NativeVector([f32; 4]);
    let enable: extern "C" fn(usize) = std::mem::transmute(game::text_base() + 0x20f6560);
    let damage = energy(accessor)?;
    let observed = if run.mode == 2 {
        Some(run.baseline?.hidden)
    } else {
        None
    };
    log_hidden(
        run,
        accessor,
        if observed.is_some() {
            "match_pre"
        } else {
            "cold_pre"
        },
    )?;
    let vector = NativeVector([velocity.x, velocity.y, 0.0, 0.0]);
    if let Some(observed) = observed {
        // The export dispatches through slot+0x40. Guard both export and actual target.
        let vptr = (damage as *const usize).read();
        let target = ((vptr + 0x40) as *const usize).read();
        if target != game::text_base() + 0x6d8560 {
            diagnostic(format_args!(
                "[TechDirect] fault=reset_target actual=0x{target:x}"
            ));
            return None;
        }
        let reset: extern "C" fn(usize, i32, *const NativeVector, *const NativeVector, usize) =
            std::mem::transmute(game::text_base() + 0x20f64d0);
        // x2 is read as q0 (16 bytes); x3 is unused by this guarded damage reset body.
        reset(
            damage,
            observed.reset_type,
            &vector,
            &NativeVector([0.0; 4]),
            accessor,
        );
        diagnostic(format_args!(
            "[TechDirect] seed attempt={} source=ordinary baseline_attempt={} reset={} before_status=true type5_info_read=UNOBSERVED",
            run.attempt,run.baseline?.attempt,observed.reset_type
        ));
    } else if run.post_init() {
        diagnostic(format_args!(
            "[TechDirect] seed attempt={} source=post_init_pending before_status=false",
            run.attempt
        ));
    } else {
        // Cold V1 precedes the first real hit. Observe this provisional state explicitly;
        // never invent a reset type or borrow a type from a later reference hit.
        let set: extern "C" fn(usize, *const NativeVector) =
            std::mem::transmute(game::text_base() + 0x20f8320);
        set(damage, &vector);
        diagnostic(format_args!(
            "[TechDirect] seed attempt={} source=cold_unseeded before_status=true",
            run.attempt
        ));
    }
    if !run.post_init() {
        enable(damage);
    }
    if matches!(run.mode, 9 | 10) {
        if !v1_guarded() {
            return None;
        }
        v1_damage_module(accessor)?;
        let overwrite: extern "C" fn(usize, f32) =
            std::mem::transmute(game::text_base() + 0x200b8b0);
        overwrite(accessor, native_reaction);
        diagnostic(format_args!(
            "[TechDirect] log_reaction attempt={} command={} reaction={:08x} before_status=true",
            run.attempt,
            run.mode,
            native_reaction.to_bits()
        ));
    }
    let ground_at_hit = run.post_init() && run.baseline?.hidden.flags[3];
    state::set_situation_kind(
        accessor,
        id(if ground_at_hit {
            "SITUATION_KIND_GROUND"
        } else {
            "SITUATION_KIND_AIR"
        })?,
    );
    if run.post_init() {
        natives::set_flag(
            accessor,
            ground_at_hit,
            id("FIGHTER_STATUS_DAMAGE_FLAG_GROUND")?,
        );
    }
    // D2 measures the log's effect on init; a manual reaction write would hide O1.
    if !matches!(run.mode, 9 | 10) {
        natives::set_float(accessor, native_reaction, reaction_id);
    }
    natives::change_status_force(accessor, id("FIGHTER_STATUS_KIND_DAMAGE_FLY")?, false);
    if !is_status(
        natives::status_kind(accessor),
        "FIGHTER_STATUS_KIND_DAMAGE_FLY",
    ) {
        return None;
    }
    if let Some(observed) = observed {
        apply_derived(accessor, observed)?;
    } else if !matches!(run.mode, 9 | 10) {
        natives::set_float(accessor, native_reaction, reaction_id);
    }
    setup(run, accessor);
    log_hidden(
        run,
        accessor,
        if observed.is_some() {
            "match_launch"
        } else {
            "cold_launch"
        },
    )?;
    let actual_velocity = self::velocity(accessor)?;
    let actual_reaction = reaction(accessor)?;
    diagnostic(format_args!(
        "[TechDirect] launched attempt={} mode=direct vx={:08x} vy={:08x} reaction={:08x} age={} requested_vx={:08x} requested_vy={:08x} requested_reaction={:08x}",
        run.attempt,
        actual_velocity.x.to_bits(),
        actual_velocity.y.to_bits(),
        actual_reaction.to_bits(),
        run.age,
        velocity.x.to_bits(),
        velocity.y.to_bits(),
        native_reaction.to_bits()
    ));
    run.started = !run.post_init();
    run.phase = if run.post_init() { 3 } else { 2 };
    Some(())
}

/// Existing common hook asks only for a fresh P1 accessor; no script/accessor pointer is retained.
pub unsafe fn watches_player(accessor: usize) -> bool {
    OBSERVING.load(Ordering::Acquire) && natives::module_accessor(0) == Some(accessor)
}
/// Called at the existing native status-decision seam, before the game's original status change.
pub unsafe fn decision(accessor: usize, status: i32, passive: bool) {
    let Some(mut run) = RUN.try_lock() else {
        return;
    };
    if !run.active || !run.started || run.contact {
        return;
    }
    let (surface, touch) = if [
        "FIGHTER_STATUS_KIND_PASSIVE",
        "FIGHTER_STATUS_KIND_PASSIVE_FB",
        "FIGHTER_STATUS_KIND_DOWN",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_D",
    ]
    .iter()
    .any(|n| is_status(status, n))
    {
        ("floor", "GROUND_TOUCH_ID_DOWN")
    } else if [
        "FIGHTER_STATUS_KIND_PASSIVE_CEIL",
        "FIGHTER_STATUS_KIND_STOP_CEIL",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_U",
    ]
    .iter()
    .any(|n| is_status(status, n))
    {
        ("ceiling", "GROUND_TOUCH_ID_UP")
    } else if [
        "FIGHTER_STATUS_KIND_PASSIVE_WALL",
        "FIGHTER_STATUS_KIND_PASSIVE_WALL_JUMP",
        "FIGHTER_STATUS_KIND_STOP_WALL",
        "FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_LR",
    ]
    .iter()
    .any(|n| is_status(status, n))
    {
        ("wall", "GROUND_TOUCH_ID_LEFT")
    } else {
        return;
    };
    let Some(touch) = id(touch) else { return };
    let get: extern "C" fn(usize, i32) -> usize =
        std::mem::transmute(game::text_base() + 0x2029fb0);
    let mut line = get(accessor, touch);
    if line == 0 && surface == "wall" {
        if let Some(right) = id("GROUND_TOUCH_ID_RIGHT") {
            line = get(accessor, right);
        }
    }
    if line == 0 || line & 7 != 0 {
        diagnostic(format_args!("[TechDirect] fault=contact_line"));
        run.stop();
        return;
    }
    let normal = ((line + 0xa0) as *const [f32; 2]).read_unaligned();
    if !normal.iter().all(|v| v.is_finite()) || normal == [0.0, 0.0] {
        diagnostic(format_args!("[TechDirect] fault=contact_normal"));
        run.stop();
        return;
    }
    diagnostic(format_args!("[TechDirect] contact attempt={} mode={} frame={} surface={surface} line=0x{line:x} x={:08x} y={:08x} nx={:08x} ny={:08x} passive={passive} status={status}",
        run.attempt,run.mode_name(),run.sample,natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits(),normal[0].to_bits(),normal[1].to_bits()));
    run.contact = true;
    complete_baseline(&mut run);
}

/// Layout reads are independently guarded by the exact native accessors' 64-byte windows.
/// Endpoints return SIMD q0 (not Rust's four-float HFA ABI), so read their guarded fields instead.
unsafe fn point(line: usize, right: bool) -> Option<[f32; 2]> {
    let vertex = ((line + if right { 0x90 } else { 0x88 }) as *const usize).read();
    if vertex == 0 || vertex & 7 != 0 {
        return None;
    }
    let value = ((vertex + 0x10) as *const [f32; 2]).read_unaligned();
    value.iter().all(|v| v.is_finite()).then_some(value)
}

unsafe fn walk(accessor: usize) {
    let Some(down) = id("GROUND_TOUCH_ID_DOWN") else {
        return;
    };
    // Owned main+15cbc88/15cbcdc load q1 from the Vector2f reference (16 bytes),
    // though the following distance reduction uses XY. Provide its complete native storage.
    #[repr(C, align(16))]
    struct NativePoint([f32; 4]);
    let touch: extern "C" fn(usize, i32) -> usize =
        std::mem::transmute(game::text_base() + 0x2029fb0);
    let next: extern "C" fn(usize, *const NativePoint) -> usize =
        std::mem::transmute(game::text_base() + 0x15cbc60);
    let mut line = touch(accessor, down);
    let seed = line;
    if !natives::is_grounded(accessor)
        || seed == 0
        || seed & 7 != 0
        || ((seed + 0xc5) as *const u8).read() & 0x20 != 0
    {
        diagnostic(format_args!("[TechDirect] fault=main_floor_seed"));
        diagnostic(format_args!("[TechPractice] walk closed=false count=0"));
        return;
    }
    diagnostic(format_args!(
        "[TechDirect] walk_seed line=0x{seed:x} passable=false grounded=true"
    ));
    let mut seen = [0usize; 64];
    let mut end = [0.0; 2];
    for index in 0..seen.len() {
        if line == 0 || line & 7 != 0 || seen[..index].contains(&line) {
            diagnostic(format_args!(
                "[TrainingRooms] tech_s0_walk end index={index} line=0x{line:x}"
            ));
            diagnostic(format_args!(
                "[TechPractice] walk closed={} count={index}",
                line != 0 && line == seed && index > 0
            ));
            return;
        }
        seen[index] = line;
        let (Some(left), Some(right)) = (point(line, false), point(line, true)) else {
            diagnostic(format_args!(
                "[TechPractice] walk closed=false count={index} invalid_vertex=true"
            ));
            return;
        };
        let normal = ((line + 0xa0) as *const [f32; 2]).read_unaligned();
        let flags = ((line + 0xc5) as *const u8).read();
        let floor = flags & 8 != 0;
        let wall = flags & 3 != 0;
        let kind = if floor || normal[1] >= 0.7 {
            "floor"
        } else if wall {
            "wall"
        } else if normal[1] < 0.0 {
            "ceiling"
        } else {
            "other"
        };
        diagnostic(format_args!(
            "[TechPractice] walk line={index} left=({:.5},{:.5}) right=({:.5},{:.5}) normal=({:.5},{:.5}) floor={floor} wall={wall} kind={kind} flags={flags:#x}",
            left[0], left[1], right[0], right[1], normal[0], normal[1]
        ));
        diagnostic(format_args!(
            "[TrainingRooms] tech_s0_line index={index} line=0x{line:x} left={left:?} right={right:?} normal={normal:?} flags={flags:#x}"
        ));
        let distance = |p: [f32; 2]| (p[0] - end[0]).powi(2) + (p[1] - end[1]).powi(2);
        end = if index == 0 || distance(left) < distance(right) {
            right
        } else {
            left
        };
        line = next(line, &NativePoint([end[0], end[1], 0.0, 0.0]));
    }
    diagnostic(format_args!("[TrainingRooms] tech_s0_walk cap=64"));
    diagnostic(format_args!(
        "[TechPractice] walk closed={} count=64",
        line == seed
    ));
}

/// Existing post-native fighter frame, P1 only except one ordinary-hit fixture placement.
pub unsafe fn frame(fighter: usize, receipt: Option<FrameReceipt>) {
    let Some(mut run) = RUN.try_lock() else {
        return;
    };
    if !crate::runtime::scene::is_native_training() {
        *run = Spike::new();
        OBSERVING.store(false, Ordering::Release);
        return;
    }
    if fighter == 0 {
        return;
    }
    let entry = ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    let accessor = natives::fighter_accessor(fighter);
    if accessor == 0 {
        return;
    }
    if entry == 1 {
        if run.active && run.arrange_cpu {
            let input = Input {
                origin: Vec2 {
                    x: run.input.origin.x - 12.0,
                    y: run.input.origin.y,
                },
                ..Input::empty()
            };
            place(accessor, input);
            run.arrange_cpu = false;
            diagnostic(format_args!(
                "[TechDirect] ordinary_cpu_fixture x={:08x} y={:08x} lr={:08x} unmodified_move={}",
                input.origin.x.to_bits(),
                input.origin.y.to_bits(),
                input.lr.to_bits(),
                run.mode != 7
            ));
        }
        return;
    }
    if entry != 0 {
        return;
    }
    if receipt.is_none() || ((run.active || run.heal_pending) && run.player_scope != receipt) {
        *run = Spike::new();
        OBSERVING.store(false, Ordering::Release);
        return;
    }
    if run.heal_pending {
        run.heal_pending = false;
        // Healing accumulates a native damage delta; observe it after the next native P1 frame.
        diagnostic(format_args!(
            "[TechDirect] healed damage={:08x} status={}",
            natives::damage(accessor, 0).to_bits(),
            natives::status_kind(accessor)
        ));
        diagnostic(format_args!("[TrainingRooms] tech_s0_off"));
        return;
    }
    let command = REQUEST.swap(0, Ordering::AcqRel);
    if command == 6 {
        if !guarded() {
            diagnostic(format_args!("[TechDirect] fault=guards"));
            return;
        }
        run.inspecting = true;
        run.inspect_last = None;
    }
    if run.inspecting {
        let generation = crate::runtime::published::live_generation();
        if let Some(live) = crate::runtime::published::live() {
            if generation == crate::runtime::published::live_generation() {
                let cpu_status =
                    natives::module_accessor(1).map_or(-1, |a| natives::status_kind(a));
                let cpu_wait = id("FIGHTER_STATUS_KIND_WAIT") == Some(cpu_status);
                let adopted = super::cpu::adopted_settings_generation();
                let observed = (generation, adopted, cpu_status);
                if run.inspect_last != Some(observed) {
                    // A generation identifies the immutable published array/trigger values.
                    // Re-emit adoption/status transitions, without per-frame array spam.
                    diagnostic(format_args!("[TechDirect] settings generation={generation} adopted={adopted} mash_state={:?} mash_triggers={} cpu_status={cpu_status} cpu_wait={cpu_wait}",live.cpu.mash_state.0,live.cpu.mash_triggers));
                    run.inspect_last = Some(observed);
                }
            }
        }
    }
    if command == 5 {
        if !guarded() {
            diagnostic(format_args!("[TechDirect] fault=off_guards"));
            return;
        }
        let before = natives::damage(accessor, 0);
        state::heal(accessor, -before);
        state::add_damage(accessor, 0.0);
        diagnostic(format_args!(
            "[TechHit] heal_queue damage={:08x} amount={:08x}",
            before.to_bits(),
            (-before).to_bits()
        ));
        let baseline = run.baseline;
        *run = Spike::new();
        run.baseline = baseline;
        run.player_scope = receipt;
        run.heal_pending = true;
        OBSERVING.store(false, Ordering::Release);
        return;
    }
    if command != 0 && command != 6 {
        if !guarded() {
            diagnostic(format_args!("[TechDirect] fault=guards"));
            return;
        }
        let stage = natives::stage_id();
        let form = natives::stage_form(stage).unwrap_or(u32::MAX);
        if command == 4 && (stage != 0 || form != 0) {
            diagnostic(format_args!("[TechDirect] fault=Battlefield_Normal"));
            return;
        }
        let baseline = run.baseline;
        *run = Spike {
            active: true,
            mode: command,
            attempt: NEXT_ATTEMPT.fetch_add(1, Ordering::Relaxed),
            player_scope: receipt,
            baseline,
            ..Spike::new()
        };
        OBSERVING.store(true, Ordering::Release);
        if command == 4 {
            diagnostic(format_args!("[TechPractice] fixture p1_kind={} cpu_kind={} stage={stage} damage={:.3} weight={:.3} form=0",
                natives::fighter_kind(accessor).unwrap_or(-1),natives::module_accessor(1).and_then(|a|natives::fighter_kind(a)).unwrap_or(-1),natives::damage(accessor,0),natives::weight(accessor)));
            walk(accessor);
            return;
        }
        let angle = if command == 7 { 90.0 } else { 315.0 };
        let Some(direction) = math::direction_degrees(angle) else {
            diagnostic(format_args!("[TechDirect] fault=direction"));
            run.stop();
            return;
        };
        let origin = Vec2 {
            x: natives::pos_x(accessor),
            y: natives::pos_y(accessor)
                + if command == 1 || command == 7 {
                    0.0
                } else {
                    30.0
                },
        };
        run.input = Input {
            origin,
            direction,
            kb: 100.0,
            angle,
            percent: if command == 1 { 20.0 } else { 60.0 },
            weight: natives::weight(accessor),
            lr: 1.0,
        };
        if matches!(command, 2 | 8..=10) {
            let Some(baseline) = baseline.filter(|b| b.complete && Some(b.scope) == receipt) else {
                diagnostic(format_args!("[TechDirect] fault=baseline_incomplete"));
                run.stop();
                return;
            };
            if baseline.input.weight.to_bits() != natives::weight(accessor).to_bits() {
                diagnostic(format_args!("[TechDirect] fault=weight_changed"));
                run.stop();
                return;
            }
            run.input = baseline.input;
            let input = run.input;
            diagnostic(format_args!("[TechDirect] baseline_adopted attempt={} command={} baseline_attempt={} kb={:08x} direction_x={:08x} direction_y={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x} ground_at_hit={}",
                run.attempt,command,baseline.attempt,input.kb.to_bits(),input.direction.x.to_bits(),input.direction.y.to_bits(),
                input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits(),baseline.hidden.flags[3]));
        }
        place(accessor, run.input);
        state::heal(accessor, -natives::damage(accessor, 0));
        state::add_damage(accessor, run.input.percent);
        if matches!(command, 1 | 2 | 7..=10) {
            let (Some(ground), Some(wait)) =
                (id("SITUATION_KIND_GROUND"), id("FIGHTER_STATUS_KIND_WAIT"))
            else {
                diagnostic(format_args!("[TechDirect] fault=ordinary_constants"));
                run.stop();
                return;
            };
            state::set_situation_kind(accessor, ground);
            natives::change_status_force(accessor, wait, false);
            if command == 1 || command == 7 {
                run.baseline = None;
                run.arrange_cpu = true;
                diagnostic(format_args!(
                    "[TechDirect] ordinary_armed attempt={}",
                    run.attempt
                ));
                if command == 7 {
                    diagnostic(format_args!("[TechHit] armed attempt={} damage=3f800000 angle=90 growth=0 fixed=1 base=100 percent=42700000 weight={:08x}",run.attempt,run.input.weight.to_bits()));
                }
            } else {
                run.phase = 1;
                diagnostic(format_args!(
                    "[TechDirect] match_armed attempt={} grounded={} status={}",
                    run.attempt,
                    natives::is_grounded(accessor),
                    natives::status_kind(accessor)
                ));
            }
        } else {
            run.phase = 1;
        }
        diagnostic(format_args!("[TechPractice] fixture p1_kind={} cpu_kind={} stage={stage} damage={:.3} weight={:.3} form={form}",
            natives::fighter_kind(accessor).unwrap_or(-1),natives::module_accessor(1).and_then(|a|natives::fighter_kind(a)).unwrap_or(-1),natives::damage(accessor,0),natives::weight(accessor)));
    }
    if !run.active {
        return;
    }
    run.age = run.age.saturating_add(1);
    let status = natives::status_kind(accessor);
    if (run.mode == 1 || run.mode == 7) && !run.started && tumble(status) {
        if natives::is_stop(accessor) && run.baseline.is_none() {
            if let Some(v) = velocity(accessor) {
                let speed = (v.x * v.x + v.y * v.y).sqrt();
                let kb = natives::damage_reaction(accessor);
                if speed > 0.0 && kb > 0.0 {
                    if run.mode == 7 {
                        if run.rewritten == 0 {
                            diagnostic(format_args!("[TechHit] fault=unmodified_contact"));
                            run.stop();
                            return;
                        }
                        let hit = crate::model::spots::Hit {
                            damage: 1.0,
                            growth: 0,
                            fixed: 1,
                            base: 100,
                        };
                        let expected = crate::model::spots::knockback(
                            &hit,
                            hit.damage,
                            natives::damage(accessor, 0),
                            run.input.weight,
                        );
                        let reference =
                            math::launch_velocity(expected, run.input.direction).unwrap();
                        let h = hidden(accessor);
                        diagnostic(format_args!("[TechHit] received attempt={} rewrites={} expected_kb={:08x} actual_kb={:08x} equal_kb={} expected_vx={:08x} expected_vy={:08x} base_vx={:08x} base_vy={:08x} virtual_vx={:08x} virtual_vy={:08x} percent={:08x} weight={:08x} age={}",
                            run.attempt,run.rewritten,expected.to_bits(),kb.to_bits(),expected.to_bits()==kb.to_bits(),reference.x.to_bits(),reference.y.to_bits(),h.map_or(u32::MAX,|h|h.speed[0]),h.map_or(u32::MAX,|h|h.speed[1]),v.x.to_bits(),v.y.to_bits(),natives::damage(accessor,0).to_bits(),run.input.weight.to_bits(),run.age));
                    }
                    run.input = Input {
                        origin: Vec2 {
                            x: natives::pos_x(accessor),
                            y: natives::pos_y(accessor),
                        },
                        direction: Vec2 {
                            x: v.x / speed,
                            y: v.y / speed,
                        },
                        kb,
                        angle: v.y.atan2(v.x).to_degrees(),
                        percent: natives::damage(accessor, 0),
                        weight: natives::weight(accessor),
                        lr: natives::lr(accessor),
                    };
                    let Some(observed) = log_hidden(&run, accessor, "real_hitlag") else {
                        diagnostic(format_args!("[TechDirect] fault=real_hidden"));
                        run.stop();
                        return;
                    };
                    run.baseline = Some(Baseline {
                        input: run.input,
                        hidden: observed,
                        attempt: run.attempt,
                        scope: receipt.unwrap(),
                        complete: false,
                    });
                    setup(&run, accessor);
                }
            }
        }
        if !natives::is_stop(accessor) {
            if run.baseline.is_none() {
                diagnostic(format_args!(
                    "[TechDirect] fault=no_stopped_launch_snapshot"
                ));
                run.stop();
                return;
            }
            run.started = true;
        }
    }
    let mut just_launched = false;
    if run.phase == 1 {
        place(accessor, run.input);
        if run.age >= 30 {
            if launch(&mut run, accessor).is_none() {
                diagnostic(format_args!("[TechDirect] fault=native_launch"));
                run.stop();
            }
            just_launched = true; // Sample0 follows the next original native integration; V1 still logs this frame.
        }
    } else if run.phase == 3 {
        // This callback follows the NEXT original native fighter frame after DAMAGE_FLY init.
        // Move only the speed/enable write; retain the native status, reaction and motion context.
        let Some(damage) = energy(accessor) else {
            run.stop();
            return;
        };
        let Some(wanted) = math::launch_velocity(run.input.kb, run.input.direction) else {
            run.stop();
            return;
        };
        let before = velocity(accessor);
        #[repr(C, align(16))]
        struct NativeVector([f32; 4]);
        let set: extern "C" fn(usize, *const NativeVector) =
            std::mem::transmute(game::text_base() + 0x20f8320);
        let enable: extern "C" fn(usize) = std::mem::transmute(game::text_base() + 0x20f6560);
        set(damage, &NativeVector([wanted.x, wanted.y, 0.0, 0.0]));
        enable(damage);
        if post_init_v1(&run, accessor).is_none() {
            diagnostic(format_args!("[TechDirect] fault=post_init_derived"));
            run.stop();
            return;
        }
        if log_hidden(&run, accessor, "post_init_write").is_none() {
            diagnostic(format_args!("[TechDirect] fault=post_init_hidden"));
            run.stop();
            return;
        }
        let actual = velocity(accessor);
        diagnostic(format_args!("[TechDirect] post_init_speed attempt={} age={} before_vx={:08x} before_vy={:08x} vx={:08x} vy={:08x} requested_vx={:08x} requested_vy={:08x}",
            run.attempt,run.age,before.map_or(u32::MAX,|v|v.x.to_bits()),before.map_or(u32::MAX,|v|v.y.to_bits()),actual.map_or(u32::MAX,|v|v.x.to_bits()),actual.map_or(u32::MAX,|v|v.y.to_bits()),wanted.x.to_bits(),wanted.y.to_bits()));
        run.started = true;
        run.phase = 2;
        just_launched = true;
    }
    let status = natives::status_kind(accessor);
    if let Some(button) = id("CONTROL_PAD_BUTTON_GUARD") {
        let count: extern "C" fn(usize, i32) -> u8 =
            std::mem::transmute(game::text_base() + 0x20013c0);
        let previous: extern "C" fn(usize, i32) -> u8 =
            std::mem::transmute(game::text_base() + 0x20013d0);
        let check: extern "C" fn(usize, i32) -> bool = std::mem::transmute(
            game::text_base() + crate::model::sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET,
        );
        if game::granted(crate::model::sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET) {
            let (c, p, guard_on) = (
                count(accessor, button),
                previous(accessor, button),
                check(accessor, button),
            );
            diagnostic(format_args!("[TechPractice] v1 frame={} c={c} p={p} guard_on={guard_on} status={status} grounded={} reaction={:.3} pos=({:.3},{:.3})",run.age,natives::is_grounded(accessor),reaction(accessor).unwrap_or(f32::NAN),natives::pos_x(accessor),natives::pos_y(accessor)));
        }
    }
    if run.started && !just_launched {
        let (Some(v), Some(reaction)) = (velocity(accessor), reaction(accessor)) else {
            diagnostic(format_args!("[TechDirect] fault=frame_read"));
            run.stop();
            return;
        };
        let input = run.input;
        if run.sample < 20
            && log_hidden(
                &run,
                accessor,
                if run.sample == 0 {
                    "first_sample"
                } else {
                    "sample"
                },
            )
            .is_none()
        {
            diagnostic(format_args!("[TechDirect] fault=sample_hidden"));
            run.stop();
            return;
        }
        diagnostic(format_args!("[TechDirect] frame attempt={} mode={} frame={} vx={:08x} vy={:08x} reaction={:08x} hitstun={:08x} status={status} tumble={} kb={:08x} angle={:08x} percent={:08x} weight={:08x} x={:08x} y={:08x}",
            run.attempt,run.mode_name(),run.sample,v.x.to_bits(),v.y.to_bits(),reaction.to_bits(),reaction.to_bits(),tumble(status),input.kb.to_bits(),input.angle.to_bits(),input.percent.to_bits(),input.weight.to_bits(),natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits()));
        run.sample = run.sample.saturating_add(1);
        complete_baseline(&mut run);
    }
}
