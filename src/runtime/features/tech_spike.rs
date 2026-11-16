//! S0 direct launcher: retain native post-init residual, add the launch, same callback.
//! diag-tech-s0.req: direct-v3 | ordinary | walk | off | inspect.
//! No product mode or new hook; P1 capture uses try_lock, roster requests reuse its game-thread service.
#[path = "tech_reference.rs"]
mod reference;
#[path = "tech_info_seed.rs"]
mod info_seed;
#[path = "tech_info_guards.rs"]
mod info_guards;
use super::tech_spike_guards;
use crate::model::cpu::hash::lua_bind_hash;
use crate::model::cpu::role::FrameReceipt;
use crate::model::tech_launch::{self as math, Vec2};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives::{self as state, Vector3f};
use reference::{Identity as ReferenceIdentity, Tracker as ReferenceTracker};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

static REQUEST: AtomicU8 = AtomicU8::new(0);
static GUARDED: AtomicU8 = AtomicU8::new(0);
static V1_GUARDED: AtomicU8 = AtomicU8::new(0);
static V3_GUARDED: AtomicU8 = AtomicU8::new(0);
static INFO_GUARDED: AtomicU8 = AtomicU8::new(0);
static OBSERVING: AtomicBool = AtomicBool::new(false);
static NEXT_ATTEMPT: AtomicU32 = AtomicU32::new(1);
static RUN: hos_lock::Mutex<Spike> = hos_lock::Mutex::new(Spike::new());
// TR roster table: model/swap/trio.rs (0x15, "Marth"); one body, ordinary forward smash.
// A separate diag producer uses this mailbox, never the menu's SPSC request ring.
const REFERENCE_CPU_KIND: u32 = 0x15;
static REFERENCE_CPU: AtomicU8 = AtomicU8::new(0);

/// Existing MainThread roster service seam, before the diagnostic RUN lock.
pub(crate) fn service_reference_cpu() {
    use crate::runtime::roster::character_roster as roster;
    let phase = REFERENCE_CPU.load(Ordering::Acquire);
    if phase == 0 || !roster::cpu_reload_ready() {
        return;
    }
    // Same already-guarded native reads as the existing diagnostic frame.
    let actual = unsafe {
        if !guarded() {
            REFERENCE_CPU.store(0, Ordering::Release);
            diagnostic(format_args!(
                "[TechDirect] reference_cpu_invalid reason=guards"
            ));
            return;
        }
        natives::module_accessor(1).and_then(|a| natives::fighter_kind(a))
    };
    if phase == 1 && actual != Some(REFERENCE_CPU_KIND as i32) {
        let accepted = roster::request_cpu_kind(REFERENCE_CPU_KIND);
        diagnostic(format_args!(
            "[TechDirect] reference_cpu_accepted kind={REFERENCE_CPU_KIND} accepted={accepted}"
        ));
        REFERENCE_CPU.store(if accepted { 2 } else { 0 }, Ordering::Release);
        if !accepted {
            diagnostic(format_args!(
                "[TechDirect] reference_cpu_invalid reason=admission"
            ));
        }
        return;
    }
    REFERENCE_CPU.store(0, Ordering::Release);
    if actual == Some(REFERENCE_CPU_KIND as i32) {
        diagnostic(format_args!(
            "[TechDirect] reference_cpu_ready kind={REFERENCE_CPU_KIND} quiescent=true changed={}",
            phase == 2
        ));
    } else {
        diagnostic(format_args!(
            "[TechDirect] reference_cpu_invalid reason=completed_wrong_kind actual={actual:?}"
        ));
    }
}

pub fn request(text: &str) {
    if text == "reference-cpu" {
        if REFERENCE_CPU
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            diagnostic(format_args!(
                "[TechDirect] reference_cpu_requested kind={REFERENCE_CPU_KIND}"
            ));
        } else {
            diagnostic(format_args!(
                "[TechDirect] reference_cpu_invalid reason=pending"
            ));
        }
        return;
    }
    let command = match text {
        "ordinary" => 1,
        "ordinary-front" => 14,
        "walk" => 4,
        "off" => 5,
        "inspect" => 6,
        "direct-v3" => 13,
        "direct-v4-behind" => 15,
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
    hitlag_velocity: Vec2,
    attempt: u32,
    scope: FrameReceipt,
    complete: bool,
    capture_age: u32,
    capture_status: i32,
    capture_grounded: bool,
    identity: ReferenceIdentity,
    damage_info: [u8; 0xa0],
}
/// Owned 13.0.5 energy layout and runtime-resolved work keys; observations only.
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
    reserve_damage_lr: u32,
    lr: u32,
    motion_speed: Option<[u32; 2]>,
}
/// Missing observation is not a missing damage/reference state.
struct MotionSpeed(Option<[u32; 2]>);
impl std::fmt::Display for MotionSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some([x, y]) => write!(f, "({x:08x},{y:08x})"),
            None => f.write_str("absent"),
        }
    }
}
fn motion_fault(reason: &str) {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        diagnostic(format_args!(
            "[TechDirect] observation=motion_speed_absent fault={reason}"
        ));
    }
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
        reserve_damage_lr: natives::get_float(
            accessor,
            id("FIGHTER_STATUS_WORK_ID_FLOAT_RESERVE_DAMAGE_LR")?,
        )
        .to_bits(),
        lr: natives::lr(accessor).to_bits(),
        motion_speed: motion_velocity(accessor).map(|v| [v.x.to_bits(), v.y.to_bits()]),
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
    log_hidden_snapshot(
        run,
        h,
        moment,
        run.age,
        natives::status_kind(accessor),
        natives::is_grounded(accessor),
        natives::is_stop(accessor),
    );
    Some(h)
}
fn log_hidden_snapshot(
    run: &Spike,
    h: Hidden,
    moment: &str,
    age: u32,
    status: i32,
    grounded: bool,
    stopped: bool,
) {
    let kind = if run.ordinary() { "ordinary" } else { "fresh" };
    let (alternate_speed, alternate_frames) = h.alternate.unwrap_or(([0; 2], 0));
    diagnostic(format_args!(
        "[TechDirect] hidden attempt={} mode={} kind={kind} moment={moment} frame={} age={} vptr=0x{:x} vptr_offset=0x{:x} reset={} enabled={} b4={:08x} b8={:04x} ba={:02x} bb={:02x} speed=({:08x},{:08x}) accel=({:08x},{:08x}) brake=({:08x},{:08x}) limit=({:08x},{:08x}) alt_present={} alt_speed=({:08x},{:08x}) alt_frames={} reaction={:08x} reaction_last={:08x} speed_up={} speed_up_max={:08x} fly_air={} end_reaction={} ground={} status={} grounded={}",
        run.attempt,run.mode_name(),run.sample,age,h.vptr,h.vptr.wrapping_sub(game::text_base()),
        h.reset_type,h.enabled,h.counter,h.b8,h.ba,h.bb,
        h.speed[0],h.speed[1],h.accel[0],h.accel[1],h.brake[0],h.brake[1],h.limit[0],h.limit[1],
        h.alternate.is_some(),alternate_speed[0],alternate_speed[1],alternate_frames,
        h.floats[0].to_bits(),h.floats[1].to_bits(),h.flags[0],h.floats[2].to_bits(),
        h.flags[1],h.flags[2],h.flags[3],status,grounded
    ));
    let m = h.motion;
    diagnostic(format_args!(
        "[TechDirect] context attempt={} mode={} kind={kind} moment={moment} frame={} age={} motion_actual={:016x} motion_pending={:016x} motion_frame={:08x} motion_rate={:08x} damage_motion_rate={:08x} init_reaction={:08x} rotation={:08x} roll_set_angle={} pierce_ground={} stopped_damage={} stopped={} cursor={} cursor_frame={} paralyze_frame={} reserve_damage_lr={:08x} lr={:08x} motion_speed={} paths=fe844/ffd90/fe99c/feb84/fef5c path_execution=UNOBSERVED reset_type={} type5_info_read=UNOBSERVED",
        run.attempt,run.mode_name(),run.sample,age,m.actual,m.pending,m.frame,m.rate,
        m.damage_rate,m.init_reaction,m.rotation,m.roll_set_angle,m.pierce_ground,m.stopped_damage,
        stopped,m.cursor,m.cursor_frame,m.paralyze_frame,m.reserve_damage_lr,m.lr,MotionSpeed(m.motion_speed),h.reset_type
    ));
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
    reference: ReferenceTracker,
    reference_last: Option<(bool, ReferenceIdentity)>,
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
            reference: ReferenceTracker::new(),
            reference_last: None,
        }
    }
    fn ordinary(&self) -> bool {
        matches!(self.mode, 1 | 14)
    }
    fn mode_name(&self) -> &'static str {
        if self.ordinary() {
            "ordinary"
        } else {
            "direct"
        }
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
        crate::model::sites_hitbox::DAMAGE_RECORD_OFFSET,
        crate::model::sites_hitbox::DAMAGE_HIT_IDS_OFFSET,
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

/// Raw observation of the inline Info returned by the guarded slot 0x188.
/// No game call or write: the concrete target is add x0,x0,#0x80; ret.
unsafe fn damage_info(accessor: usize) -> Option<[u8; 0xa0]> {
    if !v1_guarded() || accessor < 0x1000 || accessor & 7 != 0 || accessor >> 47 != 0 {
        return None;
    }
    let module = ((accessor + 0xa8) as *const usize).read();
    if module < 0x1000 || module & 7 != 0 || module >> 47 != 0 {
        return None;
    }
    v1_damage_module(accessor)?;
    Some(((module + 0x80) as *const [u8; 0xa0]).read_unaligned())
}
struct DamageInfoHex<'a>(&'a [u8; 0xa0]);
impl std::fmt::Display for DamageInfoHex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
fn log_damage_info(run: &Spike, moment: &str, age: u32, bytes: &[u8; 0xa0]) {
    diagnostic(format_args!(
        "[TechDirect] damage_info attempt={} mode={} moment={moment} age={age} size=a0 raw={}",
        run.attempt,
        run.mode_name(),
        DamageInfoHex(bytes)
    ));
}

fn complete_baseline(run: &mut Spike) {
    if !run.ordinary() || run.sample < 20 || !run.contact {
        return;
    }
    let Some(baseline) = run.baseline.as_mut().filter(|b| !b.complete) else {
        return;
    };
    baseline.complete = true;
    let input = baseline.input;
    diagnostic(format_args!("[TechDirect] baseline_complete attempt={} command={} kb={:08x} direction_x={:08x} direction_y={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x} ground_at_hit={} hitlag_vx={:08x} hitlag_vy={:08x}",
        baseline.attempt,run.mode,input.kb.to_bits(),input.direction.x.to_bits(),input.direction.y.to_bits(),
        input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits(),baseline.hidden.flags[3],baseline.hitlag_velocity.x.to_bits(),baseline.hitlag_velocity.y.to_bits()));
}

/// Same guarded record layout as input_display::struck/probe_damage. Raw reads only.
unsafe fn reference_identity(accessor: usize) -> Option<ReferenceIdentity> {
    use crate::model::sites_hitbox::damage_record as r;
    if !v1_guarded() || v1_damage_module(accessor).is_none() {
        return None;
    }
    let pointer = |p: usize| p >= 0x1000 && p & 7 == 0 && p >> 47 == 0;
    if !pointer(accessor) {
        return None;
    }
    let module = ((accessor + r::MODULE) as *const usize).read();
    if !pointer(module) {
        return None;
    }
    let record = ((module + r::RECORDS) as *const usize).read();
    if !pointer(record) {
        return None;
    }
    Some(ReferenceIdentity {
        attacker: ((record + r::ATTACKER) as *const u32).read(),
        slot: ((record + r::SLOT) as *const u8).read(),
        damage: natives::damage(accessor, 0).to_bits(),
        kb: ((record + 0x10) as *const u32).read(),
    })
}
fn invalid_reference(run: &mut Spike, field: &str) {
    diagnostic(format_args!(
        "[TechDirect] reference_invalid attempt={} age={} field={field}",
        run.attempt, run.age
    ));
    run.baseline = None;
    run.stop();
}

unsafe fn kinetic_energy(accessor: usize, name: &str) -> Option<usize> {
    let get: extern "C" fn(usize, i32) -> usize =
        std::mem::transmute(game::text_base() + 0x2043970);
    let value = get(accessor, id(name)?);
    (value != 0 && value & 7 == 0).then_some(value)
}
/// Native constructor registers this concrete energy as id0; reject other virtual layouts.
/// Owned ELF constructor6887d4, module table4f88568 and energy table4f888b8.
unsafe fn motion_velocity(accessor: usize) -> Option<Vec2> {
    let base = game::text_base();
    let pointer = |p: usize| p >= 0x1000 && p & 7 == 0 && p >> 47 == 0;
    let module = ((accessor + 0x68) as *const usize).read();
    if !pointer(module) {
        motion_fault("motion_module_pointer");
        return None;
    }
    let vptr = (module as *const usize).read();
    if vptr != base + 0x4f88568 || ((vptr + 0x60) as *const usize).read() != base + 0x479d40 {
        motion_fault("motion_module_type");
        return None;
    }
    let Some(value) = kinetic_energy(accessor, "FIGHTER_KINETIC_ENERGY_ID_MOTION") else {
        motion_fault("motion_energy_absent");
        return None;
    };
    let vptr = (value as *const usize).read();
    if vptr != base + 0x4f888b8 || ((vptr + 0x20) as *const usize).read() != base + 0x3b3540 {
        motion_fault("motion_energy_type");
        return None;
    }
    let velocity = energy_velocity(value);
    if velocity.is_none() {
        motion_fault("motion_speed_nonfinite");
    }
    velocity
}
unsafe fn energy(accessor: usize) -> Option<usize> {
    kinetic_energy(accessor, "FIGHTER_KINETIC_ENERGY_ID_DAMAGE")
}
unsafe fn velocity(accessor: usize) -> Option<Vec2> {
    energy_velocity(energy(accessor)?)
}
unsafe fn energy_velocity(energy: usize) -> Option<Vec2> {
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
/// Guard the whole helper body and both PLT windows; reject unresolved/foreign main targets.
unsafe fn v3_guarded() -> bool {
    match V3_GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut ok = base != 0;
    if ok {
        for &(offset, expected) in super::tech_direct_v3_guards::DIRECT_V3_GUARDS {
            let equal = std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected;
            diagnostic(format_args!(
                "[TechDirect] v3_guard offset={offset:x} ok={equal}"
            ));
            ok &= equal;
        }
        for offset in [0x529f2c8, 0x529f2d0] {
            let target = ((base + offset) as *const usize).read();
            let outside = target != 0
                && target & 3 == 0
                && !(base..base + super::tech_direct_v3_guards::MAIN_IMAGE_END).contains(&target);
            diagnostic(format_args!(
                "[TechDirect] v3_import slot={offset:x} target={target:x} outside_main={outside}"
            ));
            ok &= outside;
        }
    }
    if ok {
        let first: extern "C" fn(f32) -> f32 = std::mem::transmute(base + 0x39c07d0);
        let second: extern "C" fn(f32) -> f32 = std::mem::transmute(base + 0x39c07e0);
        let (x0, y0) = (first(0.0), second(0.0));
        let (x90, y90) = (
            first(std::f32::consts::FRAC_PI_2),
            second(std::f32::consts::FRAC_PI_2),
        );
        // Identify order at known inputs. The rounded quarter-turn has a small negative cosine;
        // sign/order checks here are not a tolerance in the frame-by-frame fidelity comparison.
        ok = x0.to_bits() == 1.0f32.to_bits()
            && y0.to_bits() == 0
            && y90.to_bits() == 1.0f32.to_bits()
            && x90 < 0.0
            && x90 > -1.0;
        diagnostic(format_args!("[TechDirect] v3_trig_identity zero=({:08x},{:08x}) quarter=({:08x},{:08x}) cos_then_sin={ok} radians=true",x0.to_bits(),y0.to_bits(),x90.to_bits(),y90.to_bits()));
    }
    V3_GUARDED.store(if ok { 1 } else { 2 }, Ordering::Release);
    ok
}
unsafe fn native_direction(degrees: f32) -> Option<(Vec2, f32)> {
    if !degrees.is_finite() || !v3_guarded() {
        return None;
    }
    let radians = degrees.to_radians();
    let cos: extern "C" fn(f32) -> f32 = std::mem::transmute(game::text_base() + 0x39c07d0);
    let sin: extern "C" fn(f32) -> f32 = std::mem::transmute(game::text_base() + 0x39c07e0);
    let v = Vec2 {
        x: cos(radians),
        y: sin(radians),
    };
    (v.x.is_finite() && v.y.is_finite()).then_some((v, radians))
}
struct LaunchVector {
    residual: Vec2,
    component: Vec2,
    wanted: Vec2,
    magnitude: f32,
    radians: f32,
    direction: Vec2,
    matched: bool,
}
unsafe fn launch_vector(run: &Spike, residual: Vec2) -> Option<LaunchVector> {
    let (direction, radians) = native_direction(run.input.angle)?;
    // The helper consumes w2/w3 and s0/s1/s2. Separate AAPCS integer/FP banks need
    // two unused integer placeholders. Angle id0 selects the ordinary multiplier,
    // excluding its361/367 special paths; the chosen direction is applied below.
    let speed: extern "C" fn(usize, usize, i32, i32, f32, f32, f32) -> f32 =
        std::mem::transmute(game::text_base() + 0x403a20);
    let magnitude = speed(0, 0, 0, 0, run.input.kb, 0.0, 0.0);
    if !magnitude.is_finite() || magnitude < 0.0 {
        return None;
    }
    let baseline = run.baseline.filter(|b| b.complete);
    // Only the matched diagnostic imports a real-hit vector. Cold/product inputs
    // use the native magnitude and trig, without a hardcoded speed multiplier.
    let component = baseline.map_or(
        Vec2 {
            x: magnitude * direction.x,
            y: magnitude * direction.y,
        },
        |b| Vec2 {
            x: b.hitlag_velocity.x - residual.x,
            y: b.hitlag_velocity.y - residual.y,
        },
    );
    let wanted = Vec2 {
        x: residual.x + component.x,
        y: residual.y + component.y,
    };
    (wanted.x.is_finite() && wanted.y.is_finite()).then_some(LaunchVector {
        residual,
        component,
        wanted,
        magnitude,
        radians,
        direction,
        matched: baseline.is_some(),
    })
}
unsafe fn write_speed(run: &Spike, accessor: usize, vector: &LaunchVector) -> Option<()> {
    let damage = energy(accessor)?;
    let wanted = vector.wanted;
    diagnostic(format_args!("[TechDirect] residual_launch attempt={} command={} age={} baseline_attempt={} source={} r_vx={:08x} r_vy={:08x} magnitude={:08x} angle={:08x} radians={:08x} direction_x={:08x} direction_y={:08x} launch_vx={:08x} launch_vy={:08x} wanted_vx={:08x} wanted_vy={:08x}",
        run.attempt,run.mode,run.age,run.baseline.map_or(0,|b|b.attempt),if vector.matched {"baseline_vector"} else {"native_formula"},
        vector.residual.x.to_bits(),vector.residual.y.to_bits(),vector.magnitude.to_bits(),run.input.angle.to_bits(),vector.radians.to_bits(),vector.direction.x.to_bits(),vector.direction.y.to_bits(),vector.component.x.to_bits(),vector.component.y.to_bits(),wanted.x.to_bits(),wanted.y.to_bits()));
    #[repr(C, align(16))]
    struct NativeVector([f32; 4]);
    let set: extern "C" fn(usize, *const NativeVector) =
        std::mem::transmute(game::text_base() + 0x20f8320);
    let enable: extern "C" fn(usize) = std::mem::transmute(game::text_base() + 0x20f6560);
    set(damage, &NativeVector([wanted.x, wanted.y, 0.0, 0.0]));
    enable(damage);
    log_hidden(run, accessor, "post_init_write")?;
    let actual = velocity(accessor);
    diagnostic(format_args!(
        "[TechDirect] speed_write attempt={} command={} age={} timing=same_callback",
        run.attempt, run.mode, run.age
    ));
    diagnostic(format_args!("[TechDirect] post_init_speed attempt={} age={} before_vx={:08x} before_vy={:08x} vx={:08x} vy={:08x} requested_vx={:08x} requested_vy={:08x}",
        run.attempt,run.age,vector.residual.x.to_bits(),vector.residual.y.to_bits(),actual.map_or(u32::MAX,|v|v.x.to_bits()),actual.map_or(u32::MAX,|v|v.y.to_bits()),wanted.x.to_bits(),wanted.y.to_bits()));
    Some(())
}
/// Mapping guards supplement the typed DamageModule slot guard; no new native call.
unsafe fn info_guarded() -> bool {
    match INFO_GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut ok = base != 0;
    if ok {
        for &(name, offset, expected) in info_guards::DAMAGE_INFO_LAYOUT_GUARDS {
            let equal = std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected;
            diagnostic(format_args!("[TechDirect] info_guard name={name} offset={offset:x} ok={equal}"));
            ok &= equal;
        }
    }
    INFO_GUARDED.store(if ok { 1 } else { 2 }, Ordering::Release);
    ok
}
unsafe fn seed_info(run: &Spike, accessor: usize) -> Option<()> {
    if !info_guarded() { return None; }
    // damage_info validates the accessor, module pointer and concrete slot188 target.
    let before = damage_info(accessor)?;
    let heights = [id("HIT_HEIGHT_LOW")?, id("HIT_HEIGHT_CENTER")?, id("HIT_HEIGHT_HIGH")?];
    let baseline = run.baseline.filter(|b| b.complete);
    let seed = if let Some(baseline) = baseline {
        info_seed::Seed::from_info(&baseline.damage_info)
    } else {
        let vector = launch_vector(run, Vec2 { x: 0.0, y: 0.0 })?;
        info_seed::Seed::derived(heights[1], vector.magnitude, [vector.direction.x, vector.direction.y], run.input.angle)
    };
    if !seed.valid(heights) { return None; }
    log_damage_info(run, "info_pre_write", run.age, &before);
    let module = ((accessor + 0xa8) as *const usize).read();
    if !seed.write((module + 0x80) as *mut u8, heights) { return None; }
    let after = damage_info(accessor)?;
    log_damage_info(run, "info_post_write", run.age, &after);
    diagnostic(format_args!("[TechDirect] info_seed attempt={} mode={} source={} bytes=16 height={} speed_x={:08x} speed_y={:08x} angle={:08x} mirror_past_vertical=UNVERIFIED",
        run.attempt,run.mode_name(),if baseline.is_some() { "baseline_fields" } else { "native_formula" },
        seed.height,seed.speed[0].to_bits(),seed.speed[1].to_bits(),seed.angle.to_bits()));
    Some(())
}
unsafe fn launch(run: &mut Spike, accessor: usize) -> Option<()> {
    if !v1_guarded() || !v3_guarded() {
        return None;
    }
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
    place(accessor, input);
    state::heal(accessor, -natives::damage(accessor, 0));
    state::add_damage(accessor, input.percent);
    // The first native energy update must take its air branch, even for a grounded real hit.
    state::set_situation_kind(accessor, id("SITUATION_KIND_AIR")?);
    log_hidden(run, accessor, "cold_pre")?;
    log_damage_info(run, "cold_pre", run.age, &damage_info(accessor)?);
    v1_damage_module(accessor)?;
    let overwrite: extern "C" fn(usize, f32) = std::mem::transmute(game::text_base() + 0x200b8b0);
    overwrite(accessor, native_reaction);
    diagnostic(format_args!(
        "[TechDirect] log_reaction attempt={} command={} reaction={:08x} before_status=true",
        run.attempt,
        run.mode,
        native_reaction.to_bits()
    ));
    // v4 behind diagnostic only: observe whether the native status init retains this input.
    let reserve = if run.mode == 15 {
        let baseline = run.baseline.filter(|b| b.complete)?;
        let value = f32::from_bits(baseline.hidden.motion.reserve_damage_lr);
        natives::set_float(
            accessor,
            value,
            id("FIGHTER_STATUS_WORK_ID_FLOAT_RESERVE_DAMAGE_LR")?,
        );
        diagnostic(format_args!("[TechDirect] reserve_lr_write attempt={} command={} baseline_attempt={} value={:08x} readback={:08x} before_status=true",
            run.attempt,run.mode,baseline.attempt,value.to_bits(),
            natives::get_float(accessor,id("FIGHTER_STATUS_WORK_ID_FLOAT_RESERVE_DAMAGE_LR")?).to_bits()));
        Some(value.to_bits())
    } else {
        None
    };
    seed_info(run, accessor)?;
    let damage_fly = id("FIGHTER_STATUS_KIND_DAMAGE_FLY")?;
    let reserve_key = id("FIGHTER_STATUS_WORK_ID_FLOAT_RESERVE_DAMAGE_LR")?;
    natives::change_status_force(accessor, damage_fly, false);
    natives::set_float(accessor, 0.0, reserve_key);
    diagnostic(format_args!(
        "[TechDirect] reserve_lr_consumed attempt={} command={} seeded={} actual={:08x} motion={:016x} after_status=true",
        run.attempt, run.mode, reserve.is_some(),
        natives::get_float(accessor, reserve_key).to_bits(), natives::motion_kind(accessor)
    ));
    if !is_status(
        natives::status_kind(accessor),
        "FIGHTER_STATUS_KIND_DAMAGE_FLY",
    ) {
        return None;
    }
    setup(run, accessor);
    log_hidden(run, accessor, "cold_launch")?;
    log_damage_info(run, "cold_launch", run.age, &damage_info(accessor)?);
    let actual = velocity(accessor)?;
    let vector = launch_vector(run, actual)?;
    let wanted = vector.wanted;
    diagnostic(format_args!(
        "[TechDirect] launched attempt={} mode=direct vx={:08x} vy={:08x} reaction={:08x} age={} requested_vx={:08x} requested_vy={:08x} requested_reaction={:08x}",
        run.attempt, actual.x.to_bits(), actual.y.to_bits(), reaction(accessor)?.to_bits(), run.age,
        wanted.x.to_bits(), wanted.y.to_bits(), native_reaction.to_bits()
    ));
    write_speed(run, accessor, &vector)?;
    run.started = true;
    run.phase = 2;
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
    let (Some(left), Some(right)) = (point(line, false), point(line, true)) else {
        diagnostic(format_args!("[TechDirect] fault=contact_endpoints"));
        run.stop();
        return;
    };
    diagnostic(format_args!("[TechDirect] contact attempt={} mode={} frame={} surface={surface} line=0x{line:x} x={:08x} y={:08x} nx={:08x} ny={:08x} left_x={:08x} left_y={:08x} right_x={:08x} right_y={:08x} passive={passive} status={status}",
        run.attempt,run.mode_name(),run.sample,natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits(),normal[0].to_bits(),normal[1].to_bits(),
        left[0].to_bits(),left[1].to_bits(),right[0].to_bits(),right[1].to_bits()));
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
                    x: run.input.origin.x
                        + if run.mode == 14 {
                            12.0 * run.input.lr
                        } else {
                            -12.0 * run.input.lr
                        },
                    y: run.input.origin.y,
                },
                lr: if run.mode == 14 {
                    -run.input.lr
                } else {
                    run.input.lr
                },
                ..Input::empty()
            };
            place(accessor, input);
            run.arrange_cpu = false;
            if let Some(p1) = natives::module_accessor(0) {
                diagnostic(format_args!("[TechDirect] reference_side attempt={} command={} side={} p1_x={:08x} p1_y={:08x} p1_lr={:08x} cpu_x={:08x} cpu_y={:08x} cpu_lr={:08x}",
                    run.attempt,run.mode,if run.mode == 14 {"front"} else {"behind"},
                    natives::pos_x(p1).to_bits(),natives::pos_y(p1).to_bits(),natives::lr(p1).to_bits(),
                    natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits(),natives::lr(accessor).to_bits()));
            }
            diagnostic(format_args!(
                "[TechDirect] ordinary_cpu_fixture x={:08x} y={:08x} lr={:08x} unmodified_move=true",
                input.origin.x.to_bits(),
                input.origin.y.to_bits(),
                input.lr.to_bits()
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
        let angle = 315.0;
        let direction = if matches!(command, 13 | 15) {
            native_direction(angle).map(|v| v.0)
        } else {
            Some(Vec2::default())
        };
        let Some(direction) = direction else {
            diagnostic(format_args!("[TechDirect] fault=direction"));
            run.stop();
            return;
        };
        let origin = Vec2 {
            x: natives::pos_x(accessor),
            y: natives::pos_y(accessor) + if matches!(command, 1 | 14) { 0.0 } else { 30.0 },
        };
        run.input = Input {
            origin,
            direction,
            kb: 100.0,
            angle,
            percent: if matches!(command, 1 | 14) {
                20.0
            } else {
                60.0
            },
            weight: natives::weight(accessor),
            lr: 1.0,
        };
        if matches!(command, 13 | 15) && baseline.is_some() {
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
            diagnostic(format_args!("[TechDirect] baseline_adopted attempt={} command={} baseline_attempt={} kb={:08x} direction_x={:08x} direction_y={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x} ground_at_hit={} hitlag_vx={:08x} hitlag_vy={:08x}",
                run.attempt,command,baseline.attempt,input.kb.to_bits(),input.direction.x.to_bits(),input.direction.y.to_bits(),
                input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits(),baseline.hidden.flags[3],baseline.hitlag_velocity.x.to_bits(),baseline.hitlag_velocity.y.to_bits()));
        } else if matches!(command, 13 | 15) {
            let input = run.input;
            diagnostic(format_args!("[TechDirect] baseline_cold attempt={} command={} baseline_attempt=0 kb={:08x} direction_x={:08x} direction_y={:08x} percent={:08x} weight={:08x} start_x={:08x} start_y={:08x} lr={:08x} source=fixed",
                run.attempt,command,input.kb.to_bits(),input.direction.x.to_bits(),input.direction.y.to_bits(),
                input.percent.to_bits(),input.weight.to_bits(),input.origin.x.to_bits(),input.origin.y.to_bits(),input.lr.to_bits()));
        }
        place(accessor, run.input);
        state::heal(accessor, -natives::damage(accessor, 0));
        state::add_damage(accessor, run.input.percent);
        if matches!(command, 1 | 13 | 14 | 15) {
            let (Some(ground), Some(wait)) =
                (id("SITUATION_KIND_GROUND"), id("FIGHTER_STATUS_KIND_WAIT"))
            else {
                diagnostic(format_args!("[TechDirect] fault=ordinary_constants"));
                run.stop();
                return;
            };
            state::set_situation_kind(accessor, ground);
            natives::change_status_force(accessor, wait, false);
            if run.ordinary() {
                run.baseline = None;
                run.arrange_cpu = true;
                diagnostic(format_args!(
                    "[TechDirect] ordinary_armed attempt={}",
                    run.attempt
                ));
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
    if run.ordinary() && !run.started && (tumble(status) || run.baseline.is_some()) {
        if natives::module_accessor(1).and_then(|a| natives::fighter_kind(a))
            != Some(REFERENCE_CPU_KIND as i32)
        {
            invalid_reference(&mut run, "cpu_kind");
            return;
        }
        let Some(record) = reference_identity(accessor) else {
            invalid_reference(&mut run, "record_unreadable");
            return;
        };
        let stopped = natives::is_stop(accessor);
        if run.reference_last != Some((stopped, record)) {
            diagnostic(format_args!("[TechDirect] reference_record attempt={} age={} stopped={stopped} attacker={:08x} slot={} damage={:08x} kb={:08x}",
                run.attempt, run.age, record.attacker, record.slot, record.damage, record.kb));
            run.reference_last = Some((stopped, record));
        }
        if stopped {
            let identity = match run.reference.stopped(record) {
                Ok(value) => value,
                Err(reason) => {
                    invalid_reference(&mut run, reason.field());
                    return;
                }
            };
            let Some(v) = velocity(accessor) else {
                invalid_reference(&mut run, "velocity_unreadable");
                return;
            };
            let speed = (v.x * v.x + v.y * v.y).sqrt();
            if !speed.is_finite() || speed <= 0.0 || !f32::from_bits(identity.damage).is_finite() {
                invalid_reference(&mut run, "launch_values");
                return;
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
                kb: f32::from_bits(identity.kb),
                angle: v.y.atan2(v.x).to_degrees(),
                percent: f32::from_bits(identity.damage),
                weight: natives::weight(accessor),
                lr: natives::lr(accessor),
            };
            let Some(observed) = hidden(accessor) else {
                invalid_reference(&mut run, "hidden_unreadable");
                return;
            };
            let Some(info) = damage_info(accessor) else {
                invalid_reference(&mut run, "damage_info_unreadable");
                return;
            };
            // Refresh every stopped callback, including after native record KB clears.
            // Emit only the selected final snapshot at the first non-stop boundary.
            run.baseline = Some(Baseline {
                input: run.input,
                hidden: observed,
                hitlag_velocity: v,
                attempt: run.attempt,
                scope: receipt.unwrap(),
                complete: false,
                capture_age: run.age,
                capture_status: status,
                capture_grounded: natives::is_grounded(accessor),
                identity,
                damage_info: info,
            });
        } else {
            let Some(baseline) = run.baseline else {
                invalid_reference(&mut run, "no_stopped_launch_snapshot");
                return;
            };
            if !tumble(status) {
                invalid_reference(&mut run, "status");
                return;
            }
            if let Err(reason) = run.reference.validate(record) {
                invalid_reference(&mut run, reason.field());
                return;
            }
            if baseline.capture_age.checked_add(1) != Some(run.age) {
                invalid_reference(&mut run, "nonadjacent_snapshot");
                return;
            }
            let i = baseline.identity;
            diagnostic(format_args!("[TechDirect] reference_final attempt={} capture_age={} k0_age={} attacker={:08x} slot={} damage={:08x} kb={:08x} record_kb={:08x} valid=true",
                run.attempt, baseline.capture_age, run.age, i.attacker, i.slot, i.damage, i.kb, record.kb));
            log_damage_info(
                &run,
                "reference_final",
                baseline.capture_age,
                &baseline.damage_info,
            );
            log_hidden_snapshot(
                &run,
                baseline.hidden,
                "real_hitlag",
                baseline.capture_age,
                baseline.capture_status,
                baseline.capture_grounded,
                true,
            );
            setup(&run, accessor);
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
