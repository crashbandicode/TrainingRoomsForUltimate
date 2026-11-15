//! S0 only: real hidden-CPU hit, plugin-observed diag counters, and a bounded ground walk.
//! Not a product mode or a product Training-only restriction. No hook, worker, or blocking lock added.
//! `diag-tech-s0.req`: hidden | invisible | camera | walk | off. Each launch fixture is 60%, (0,30).

use std::sync::atomic::{AtomicU8, Ordering};

use crate::model::cpu::hash::{hash40, lua_bind_hash};
use crate::model::cpu::role::FrameReceipt;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game::{self, natives};
use crate::runtime::state_natives::{self as state, Vector3f};

use super::{hide_cpu, tech_spike_guards};

static REQUEST: AtomicU8 = AtomicU8::new(0);
static GUARDED: AtomicU8 = AtomicU8::new(0);
static RUN: hos_lock::Mutex<Spike> = hos_lock::Mutex::new(Spike::new());

/// Called only by the existing diag file-poll thread; hooks consume the atomic, never its lock.
pub fn request(text: &str) {
    let command = match text {
        "hidden" => 1,
        "invisible" => 2,
        "camera" => 3,
        "walk" => 4,
        "off" => 5,
        _ => {
            diagnostic(format_args!("[TrainingRooms] tech_s0_request invalid"));
            return;
        }
    };
    REQUEST.store(command, Ordering::Release);
    diagnostic(format_args!("[TrainingRooms] tech_s0_request command={command}"));
}

struct Spike {
    active: bool,
    mode: u8,
    phase: u8,
    age: u32,
    weight: f32,
    player_scope: Option<FrameReceipt>,
    cpu_scope: Option<FrameReceipt>,
    status_before: i32,
    launch_left: u8,
}

impl Spike {
    const fn new() -> Self {
        Self {
            active: false,
            mode: 0,
            phase: 0,
            age: 0,
            weight: 0.0,
            player_scope: None,
            cpu_scope: None,
            status_before: -1,
            launch_left: 0,
        }
    }
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
            let actual = std::slice::from_raw_parts((base + offset) as *const u8, 64);
            let ok = actual == expected;
            diagnostic(format_args!("[TrainingRooms] tech_s0_guard site={name} ok={ok}"));
            all &= ok;
        }
    }
    GUARDED.store(if all { 1 } else { 2 }, Ordering::Release);
    all
}

unsafe fn id(name: &str) -> Option<i32> {
    game::lua_const(lua_bind_hash(name))
}

/// Native ATTACK body main+2284b80 constructs at sp+20, passes x3=sp+20 at 2285ba4.
/// Its last store is sp+11a; this is the 0x100-byte INPUT, not the 0x130-byte attack slot.
/// Constructor defaults: 2284ba4..2284c9c. Argument stores: 2284ca0..2285ad0.
#[repr(C, align(16))]
struct AttackData([u8; 0x100]);

impl AttackData {
    fn word(&mut self, at: usize, value: u32) {
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn float(&mut self, at: usize, value: f32) {
        self.word(at, value.to_bits());
    }

    fn hash(&mut self, at: usize, value: u64) {
        self.0[at..at + 8].copy_from_slice(&value.to_le_bytes());
    }

    unsafe fn new(weight: f32) -> Option<Self> {
        if !weight.is_finite() || weight <= 0.0 {
            return None;
        }
        // Vanilla KB formula at 60%, damage .1, target KB 100; BKB and FKB zero.
        let growth = (10_000.0 / ((6.0 + 60.0 * 0.1 / 20.0) * 200.0 / (weight + 100.0) * 1.4 + 18.0)).round() as u32;
        let mut data = Self([0; 0x100]);
        data.float(0x04, 6.0); // top-relative sphere center; radius 10 reaches P1's hurtboxes.
        data.float(0x20, 0.1);
        data.float(0x24, 10.0);
        data.word(0x28, 45);
        data.word(0x2c, growth);
        data.float(0x40, 1.0); // SDI; hitlag at +3c stays zero, internal hitlag<=0 flag below.
        data.hash(0x48, hash40("top"));
        let category = id("COLLISION_CATEGORY_MASK_FIGHTER")? as u16;
        data.0[0x50..0x52].copy_from_slice(&category.to_le_bytes());
        data.0[0x52] = id("COLLISION_SITUATION_MASK_GA")? as u8;
        data.0[0x54] = id("COLLISION_PART_MASK_ALL")? as u8;
        data.hash(0x58, hash40("collision_attr_normal"));
        data.0[0x60] = id("ATTACK_SOUND_LEVEL_S")? as u8;
        data.0[0x61] = id("COLLISION_SOUND_ATTR_NONE")? as u8;
        data.0[0x62] = id("ATTACK_SETOFF_KIND_OFF")? as u8;
        // Shield damage 0: ATTACK writes flag sp+84=1 AND amount i16 at sp+94 (2285554..557c).
        data.0[0x64] = 1;
        data.0[0x67] = 1; // direct; reflectable/absorbable/flinchless/disable-hitlag remain false.
        data.0[0x6a] = id("ATTACK_LR_CHECK_POS")? as u8;
        data.0[0x6d] = 1; // hitlag multiplier <= 0 (2285474..548c).
        data.0[0x71] = id("ATTACK_REGION_NONE")? as u8;
        data.0[0x76] = 0x0e;
        // Arg20=set_weight false is sp+9e (+7e), not +63; the zero default preserves it.
        data.hash(0x88, hash40("top"));
        // +90 is nn::util::Vector4f::ConstantZero, the imported constructor vector.
        data.float(0xa8, 1.0);
        data.float(0xac, 1.0);
        data.hash(0xb8, 0x425cbfc4f);
        data.hash(0xc0, 0x7fb997a80);
        data.float(0xd0, -1.0);
        data.word(0xec, u32::MAX);
        data.0[0xf0] = 3;
        data.float(0xf4, 1.0);
        Some(data)
    }
}

unsafe fn place(accessor: usize, x: f32, y: f32) {
    state::clear_speed_all(accessor);
    state::init_pos(accessor, &Vector3f { x, y, z: 0.0 });
}

/// Use the existing guarded, installed button-on seam, including TR's own pass-through hook for P1.
unsafe fn guard_on(accessor: usize, button: i32) -> Option<bool> {
    let offset = crate::model::sites_pkg2::CHECK_BUTTON_ON_HOOK_OFFSET;
    if !game::granted(offset) {
        return None;
    }
    let on: extern "C" fn(usize, i32) -> bool = std::mem::transmute(game::text_base() + offset);
    Some(on(accessor, button))
}

/// The immediate post-call sample and the next three native P1 frame observations. No retained accessor.
unsafe fn launch_sample(run: &Spike, player: usize, sample: u8) {
    let Some(all) = id("KINETIC_ENERGY_RESERVE_ATTRIBUTE_ALL") else {
        diagnostic(format_args!("[TechPractice] launch unavailable=energy_id"));
        return;
    };
    let speed_x: extern "C" fn(usize, i32) -> f32 = std::mem::transmute(game::text_base() + 0x2043a00);
    diagnostic(format_args!(
        "[TechPractice] launch cpu_hidden={} set_attack=ok p1_status_before={} p1_status_after={} reaction={:.3} speed=({:.5},{:.5}) frame={} sample={sample} mode={}",
        hide_cpu::hidden(), run.status_before, natives::status_kind(player), natives::damage_reaction(player),
        speed_x(player, all), natives::sum_speed_y(player, all), run.age, run.mode
    ));
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
    let Some(down) = id("GROUND_TOUCH_ID_DOWN") else { return };
    // Owned main+15cbc88/15cbcdc load q1 from the Vector2f reference (16 bytes),
    // though the following distance reduction uses XY. Provide its complete native storage.
    #[repr(C, align(16))]
    struct NativePoint([f32; 4]);
    let touch: extern "C" fn(usize, i32) -> usize = std::mem::transmute(game::text_base() + 0x2029fb0);
    let next: extern "C" fn(usize, *const NativePoint) -> usize = std::mem::transmute(game::text_base() + 0x15cbc60);
    let mut line = touch(accessor, down);
    let seed = line;
    let mut seen = [0usize; 64];
    let mut end = [0.0; 2];
    for index in 0..seen.len() {
        if line == 0 || line & 7 != 0 || seen[..index].contains(&line) {
            diagnostic(format_args!("[TrainingRooms] tech_s0_walk end index={index} line=0x{line:x}"));
            diagnostic(format_args!(
                "[TechPractice] walk closed={} count={index}",
                line != 0 && line == seed && index > 0
            ));
            return;
        }
        seen[index] = line;
        let (Some(left), Some(right)) = (point(line, false), point(line, true)) else {
            diagnostic(format_args!("[TechPractice] walk closed=false count={index} invalid_vertex=true"));
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
    diagnostic(format_args!("[TechPractice] walk closed={} count=64", line == seed));
}

/// Game thread only, from the existing CPU frame after Hide CPU and Eggs/BTT. No accessor is retained across frames.
pub unsafe fn frame(fighter: usize, receipt: Option<FrameReceipt>, launcher: bool) {
    let Some(mut run) = RUN.try_lock() else { return };
    if !crate::runtime::scene::is_native_training() {
        if run.active {
            hide_cpu::restore();
        }
        *run = Spike::new();
        return;
    }
    if fighter == 0 {
        return;
    }
    let entry = ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    // Eggs publishes its request on every body's pass, including parked helper bodies.
    // Reassert this research owner after each pass without admitting those helpers as launchers.
    if run.active && run.mode != 4 {
        hide_cpu::hide(true);
    }
    if !(0..=1).contains(&entry) {
        return;
    }
    if receipt.is_none()
        || (entry == 0 && run.active && run.player_scope != receipt)
        || (entry == 1 && run.active && run.cpu_scope.is_some() && run.cpu_scope != receipt)
    {
        if run.active {
            hide_cpu::restore();
        }
        *run = Spike::new();
        return;
    }
    let at = natives::fighter_accessor(fighter);
    if at == 0 {
        return;
    }
    if entry == 0 {
        let command = REQUEST.swap(0, Ordering::AcqRel);
        if command == 5 {
            hide_cpu::restore();
            *run = Spike::new();
            diagnostic(format_args!("[TrainingRooms] tech_s0_off"));
        } else if command != 0 && guarded() {
            walk(at); // Seed while P1 still touches the fixture's floor.
            *run = Spike {
                active: true,
                mode: command,
                phase: if command <= 2 { 1 } else { 4 },
                age: 0,
                weight: natives::weight(at),
                player_scope: receipt,
                cpu_scope: None,
                status_before: -1,
                launch_left: 0,
            };
            if command <= 2 {
                state::heal(at, -natives::damage(at, 0));
                state::add_damage(at, 60.0);
                if let Some(fall) = id("FIGHTER_STATUS_KIND_FALL") {
                    state::status_request(at, fall);
                }
                place(at, 0.0, 30.0);
            }
            diagnostic(format_args!(
                "[TrainingRooms] tech_s0_prepared mode={command} p1={:.3},{:.3} damage={:.3} weight={:.3}",
                natives::pos_x(at),
                natives::pos_y(at),
                natives::damage(at, 0),
                natives::weight(at)
            ));
            let cpu_kind = natives::module_accessor(1).and_then(|cpu| natives::fighter_kind(cpu)).unwrap_or(-1);
            diagnostic(format_args!(
                "[TechPractice] fixture p1_kind={} cpu_kind={cpu_kind} stage={} damage={:.3} weight={:.3}",
                natives::fighter_kind(at).unwrap_or(-1),
                natives::stage_id(),
                natives::damage(at, 0),
                natives::weight(at)
            ));
        }
        if run.active {
            run.age = run.age.saturating_add(1);
            if run.launch_left > 0 {
                launch_sample(&run, at, 4 - run.launch_left);
                run.launch_left -= 1;
            }
            if run.phase <= 2 {
                place(at, 0.0, 30.0);
            }
            if let Some(guard) = id("CONTROL_PAD_BUTTON_GUARD") {
                let count: extern "C" fn(usize, i32) -> u8 = std::mem::transmute(game::text_base() + 0x20013c0);
                let previous: extern "C" fn(usize, i32) -> u8 = std::mem::transmute(game::text_base() + 0x20013d0);
                let (c, p) = (count(at, guard), previous(at, guard));
                if let Some(guard_on) = guard_on(at, guard) {
                    diagnostic(format_args!(
                        "[TechPractice] v1 frame={} c={c} p={p} guard_on={guard_on} status={} grounded={} reaction={:.3} pos=({:.3},{:.3})",
                        run.age,
                        natives::status_kind(at),
                        natives::is_grounded(at),
                        natives::damage_reaction(at),
                        natives::pos_x(at),
                        natives::pos_y(at)
                    ));
                } else {
                    diagnostic(format_args!("[TechPractice] v1 unavailable=button_guard frame={}", run.age));
                }
                if run.age <= 40 || c <= 3 || run.age % 60 == 0 {
                    diagnostic(format_args!(
                        "[TrainingRooms] tech_s0_p1 age={} c={c} p={p} status={} damage={:.3} pos={:.3},{:.3} reaction={:.3}",
                        run.age,
                        natives::status_kind(at),
                        natives::damage(at, 0),
                        natives::pos_x(at),
                        natives::pos_y(at),
                        natives::damage_reaction(at)
                    ));
                }
            }
        }
    } else if run.active && launcher {
        run.cpu_scope = receipt;
        let clear: extern "C" fn(usize) = std::mem::transmute(game::text_base() + 0x1fd00c0);
        match run.phase {
            1 => run.phase = 2, // First let the ordinary hidden-body step run once.
            2 => {
                place(at, 0.0, 30.0);
                state::set_lr(at, 1.0);
                state::heal(at, -natives::damage(at, 0));
                if let Some(data) = AttackData::new(run.weight) {
                    let Some(player) = natives::module_accessor(0) else {
                        diagnostic(format_args!("[TechPractice] launch unavailable=player"));
                        run.phase = 4;
                        return;
                    };
                    run.status_before = natives::status_kind(player);
                    if run.mode == 2 {
                        // Approved DESIGN §8 variant: still invisible, but not frozen for this launch frame.
                        let stop: extern "C" fn(usize, i32, bool) = std::mem::transmute(game::text_base() + 0x2089260);
                        stop(at, 0, false);
                    }
                    let set: extern "C" fn(usize, i32, i32, *const AttackData) = std::mem::transmute(game::text_base() + 0x1fd0110);
                    set(at, 0, 0, &data);
                    launch_sample(&run, player, 0);
                    run.launch_left = 3;
                    diagnostic(format_args!(
                        "[TrainingRooms] tech_s0_launch mode={} age={} cpu={:.3},{:.3} stop={}",
                        run.mode,
                        run.age,
                        natives::pos_x(at),
                        natives::pos_y(at),
                        natives::is_stop(at)
                    ));
                } else {
                    diagnostic(format_args!("[TrainingRooms] tech_s0_launch unavailable constants_or_weight"));
                }
                run.phase = 3;
            }
            3 => {
                clear(at);
                place(at, 35.0, -15.0);
                run.phase = 4;
                diagnostic(format_args!(
                    "[TrainingRooms] tech_s0_park cpu={:.3},{:.3}",
                    natives::pos_x(at),
                    natives::pos_y(at)
                ));
            }
            _ if run.mode == 3 => place(at, 70.0, -15.0),
            _ => {}
        }
    }
    if run.active && run.mode != 4 {
        hide_cpu::hide(true);
    }
}
