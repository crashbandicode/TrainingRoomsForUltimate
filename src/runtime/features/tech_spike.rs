//! S0 DIRECT only. Native status/energy launch; fidelity is an explicit runtime comparison.
//! diag-tech-s0.req: ordinary | direct-match | direct | walk | off | inspect.
//! No product mode, CPU launcher, heap, blocking lock or new hook.
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
    attempt: u32,
    scope: FrameReceipt,
    complete: bool,
}
struct Spike {
    active: bool,
    mode: u8,
    age: u32,
    phase: u8,
    attempt: u32,
    sample: u32,
    started: bool,
    arrange_cpu: bool,
    contact: bool,
    inspecting: bool,
    player_scope: Option<FrameReceipt>,
    input: Input,
    baseline: Option<Baseline>,
}
impl Spike {
    const fn new() -> Self {
        Self {
            active: false,
            mode: 0,
            age: 0,
            phase: 0,
            attempt: 0,
            sample: 0,
            started: false,
            arrange_cpu: false,
            contact: false,
            inspecting: false,
            player_scope: None,
            input: Input::empty(),
            baseline: None,
        }
    }
    fn mode_name(&self) -> &'static str {
        if self.mode == 1 {
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
            let ok = std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected;
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
        crate::model::sites_pkg2::CHANGE_STATUS_FORCE_OFFSET,
        crate::model::sites_pkg2::WORK_SET_FLOAT_OFFSET,
        crate::model::sites_pkg2::WORK_GET_FLOAT_OFFSET,
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
    ] {
        all &= game::granted(offset);
    }
    all &= super::cpu_common::passive_ready();
    GUARDED.store(if all { 1 } else { 2 }, Ordering::Release);
    all
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
    let reaction_id = id("FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME")?;
    place(accessor, input);
    state::heal(accessor, -natives::damage(accessor, 0));
    state::add_damage(accessor, input.percent);
    state::set_situation_kind(accessor, id("SITUATION_KIND_AIR")?);
    natives::set_float(accessor, native_reaction, reaction_id);
    natives::change_status_force(accessor, id("FIGHTER_STATUS_KIND_DAMAGE_FLY")?, false);
    // The native status initializes first. No DamageInfo/energy bytes from the reference hit are cloned.
    if !is_status(
        natives::status_kind(accessor),
        "FIGHTER_STATUS_KIND_DAMAGE_FLY",
    ) {
        return None;
    }
    let velocity = math::launch_velocity(input.kb, input.direction)?;
    #[repr(C, align(16))]
    struct NativeVector([f32; 4]);
    let set: extern "C" fn(usize, *const NativeVector) =
        std::mem::transmute(game::text_base() + 0x20f8320);
    let enable: extern "C" fn(usize) = std::mem::transmute(game::text_base() + 0x20f6560);
    let damage = energy(accessor)?;
    // Owned full-body proof: this setter's only write is the same speed store as Damage's own setter.
    set(damage, &NativeVector([velocity.x, velocity.y, 0.0, 0.0]));
    enable(damage);
    natives::set_float(accessor, native_reaction, reaction_id);
    setup(run, accessor);
    diagnostic(format_args!(
        "[TechDirect] launched attempt={} mode=direct vx={:08x} vy={:08x} reaction={:08x}",
        run.attempt,
        velocity.x.to_bits(),
        velocity.y.to_bits(),
        native_reaction.to_bits()
    ));
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
    diagnostic(format_args!("[TechDirect] contact attempt={} mode={} frame={} surface={surface} line=0x{line:x} x={:08x} y={:08x} nx={:08x} ny={:08x} passive={passive} status={status}",
        run.attempt,run.mode_name(),run.sample,natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits(),normal[0].to_bits(),normal[1].to_bits()));
    run.contact = true;
    if run.mode == 1 && run.sample >= 20 {
        if let Some(b) = &mut run.baseline {
            b.complete = true;
        }
    }
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
                origin: Vec2 { x: -12.0, y: 0.01 },
                ..Input::empty()
            };
            place(accessor, input);
            run.arrange_cpu = false;
            diagnostic(format_args!("[TechDirect] ordinary_cpu_fixture x={:08x} y={:08x} lr={:08x} unmodified_move=true",(-12.0f32).to_bits(),0.01f32.to_bits(),1.0f32.to_bits()));
        }
        return;
    }
    if entry != 0 {
        return;
    }
    if receipt.is_none() || (run.active && run.player_scope != receipt) {
        *run = Spike::new();
        OBSERVING.store(false, Ordering::Release);
        return;
    }
    let command = REQUEST.swap(0, Ordering::AcqRel);
    if command == 6 {
        if !guarded() {
            diagnostic(format_args!("[TechDirect] fault=guards"));
            return;
        }
        run.inspecting = true;
    }
    if run.inspecting {
        let generation = crate::runtime::published::live_generation();
        if let Some(live) = crate::runtime::published::live() {
            if generation == crate::runtime::published::live_generation() {
                let cpu_status =
                    natives::module_accessor(1).map_or(-1, |a| natives::status_kind(a));
                let cpu_wait = id("FIGHTER_STATUS_KIND_WAIT") == Some(cpu_status);
                let adopted = super::cpu::adopted_settings_generation();
                diagnostic(format_args!("[TechDirect] settings generation={generation} adopted={adopted} mash_state={:?} mash_triggers={} cpu_status={cpu_status} cpu_wait={cpu_wait}",live.cpu.mash_state.0,live.cpu.mash_triggers));
            }
        }
    }
    if command == 5 {
        let baseline = run.baseline;
        *run = Spike::new();
        run.baseline = baseline;
        OBSERVING.store(false, Ordering::Release);
        diagnostic(format_args!("[TrainingRooms] tech_s0_off"));
        return;
    }
    if command != 0 && command != 6 {
        if !guarded() {
            diagnostic(format_args!("[TechDirect] fault=guards"));
            return;
        }
        let stage = natives::stage_id();
        if stage != 0 || natives::stage_form(stage) != Some(0) {
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
        run.input = Input {
            origin: Vec2 {
                x: 0.0,
                y: if command == 1 { 0.01 } else { 30.0 },
            },
            direction: Vec2 { x: 0.0, y: -1.0 },
            kb: 100.0,
            angle: 270.0,
            percent: 60.0,
            weight: natives::weight(accessor),
            lr: 1.0,
        };
        if command == 2 {
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
        }
        place(accessor, run.input);
        state::heal(accessor, -natives::damage(accessor, 0));
        state::add_damage(accessor, run.input.percent);
        if command == 1 {
            let (Some(ground), Some(wait)) =
                (id("SITUATION_KIND_GROUND"), id("FIGHTER_STATUS_KIND_WAIT"))
            else {
                diagnostic(format_args!("[TechDirect] fault=ordinary_constants"));
                run.stop();
                return;
            };
            state::set_situation_kind(accessor, ground);
            natives::change_status_force(accessor, wait, false);
            run.baseline = None;
            run.arrange_cpu = true;
            diagnostic(format_args!(
                "[TechDirect] ordinary_armed attempt={}",
                run.attempt
            ));
        } else {
            run.phase = 1;
        }
        diagnostic(format_args!("[TechPractice] fixture p1_kind={} cpu_kind={} stage={stage} damage={:.3} weight={:.3} form=0",
            natives::fighter_kind(accessor).unwrap_or(-1),natives::module_accessor(1).and_then(|a|natives::fighter_kind(a)).unwrap_or(-1),natives::damage(accessor,0),natives::weight(accessor)));
    }
    if !run.active {
        return;
    }
    run.age = run.age.saturating_add(1);
    let status = natives::status_kind(accessor);
    if run.mode == 1 && !run.started && tumble(status) {
        if natives::is_stop(accessor) && run.baseline.is_none() {
            if let Some(v) = velocity(accessor) {
                let speed = (v.x * v.x + v.y * v.y).sqrt();
                let kb = natives::damage_reaction(accessor);
                if speed > 0.0 && kb > 0.0 {
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
                    run.baseline = Some(Baseline {
                        input: run.input,
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
        diagnostic(format_args!("[TechDirect] frame attempt={} mode={} frame={} vx={:08x} vy={:08x} reaction={:08x} hitstun={:08x} status={status} tumble={} kb={:08x} angle={:08x} percent={:08x} weight={:08x} x={:08x} y={:08x}",
            run.attempt,run.mode_name(),run.sample,v.x.to_bits(),v.y.to_bits(),reaction.to_bits(),reaction.to_bits(),tumble(status),input.kb.to_bits(),input.angle.to_bits(),input.percent.to_bits(),input.weight.to_bits(),natives::pos_x(accessor).to_bits(),natives::pos_y(accessor).to_bits()));
        run.sample = run.sample.saturating_add(1);
        if run.mode == 1 && run.sample >= 20 && run.contact {
            if let Some(b) = &mut run.baseline {
                b.complete = true;
            }
        }
    }
}
