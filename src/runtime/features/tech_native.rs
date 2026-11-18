//! Guarded native launch primitives shared by product and diagnostic launchers.
#[path = "tech_info_guards.rs"]
mod info_guards;
#[path = "tech_stop_guards.rs"]
mod stop_guards;
use crate::model::cpu::hash::lua_bind_hash;
use crate::model::tech_launch::Vec2;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game::{self, natives};
use std::sync::atomic::{AtomicU8, Ordering};
static V1_GUARDED: AtomicU8 = AtomicU8::new(0);
static V3_GUARDED: AtomicU8 = AtomicU8::new(0);
static STOP_GUARDED: AtomicU8 = AtomicU8::new(0);
static INFO_GUARDED: AtomicU8 = AtomicU8::new(0);
pub(super) unsafe fn id(name: &str) -> Option<i32> {
    game::lua_const(lua_bind_hash(name))
}
pub(super) unsafe fn v1_guarded() -> bool {
    match V1_GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut all = base != 0;
    if all {
        for &(offset, expected) in crate::model::sites_tech_launch::v1::DIRECT_V1_GUARDS {
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
pub(super) unsafe fn v1_damage_module(accessor: usize) -> Option<()> {
    let base = game::text_base();
    for &(module_offset, slot, expected) in
        crate::model::sites_tech_launch::v1::DIRECT_V1_VIRTUAL_TARGETS
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

pub(super) unsafe fn damage_info(accessor: usize) -> Option<[u8; 0xa0]> {
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

pub(super) unsafe fn v3_guarded() -> bool {
    match V3_GUARDED.load(Ordering::Acquire) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let base = game::text_base();
    let mut ok = base != 0;
    if ok {
        for &(offset, expected) in crate::model::sites_tech_launch::v3::DIRECT_V3_GUARDS {
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
                && !(base..base + crate::model::sites_tech_launch::v3::MAIN_IMAGE_END)
                    .contains(&target);
            diagnostic(format_args!(
                "[TechDirect] v3_import slot={offset:x} target={target:x} outside_main={outside}"
            ));
            ok &= outside;
        }
    }
    if ok {
        let values = (
            natives::tech_cos(0.0),
            natives::tech_sin(0.0),
            natives::tech_cos(std::f32::consts::FRAC_PI_2),
            natives::tech_sin(std::f32::consts::FRAC_PI_2),
        );
        let (Some(x0), Some(y0), Some(x90), Some(y90)) = values else {
            return false;
        };
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
pub(super) unsafe fn native_direction(degrees: f32) -> Option<(Vec2, f32)> {
    if !degrees.is_finite() || !v3_guarded() {
        return None;
    }
    let radians = degrees.to_radians();
    let v = Vec2 {
        x: natives::tech_cos(radians)?,
        y: natives::tech_sin(radians)?,
    };
    (v.x.is_finite() && v.y.is_finite()).then_some((v, radians))
}

pub(super) unsafe fn stop_module(accessor: usize) -> Option<usize> {
    let base = game::text_base();
    if STOP_GUARDED.load(Ordering::Acquire) == 0 {
        let mut ok = base != 0;
        if ok {
            for &(offset, expected) in stop_guards::STOP_GUARDS {
                let equal =
                    std::slice::from_raw_parts((base + offset) as *const u8, 64) == expected;
                diagnostic(format_args!(
                    "[TechDirect] stop_guard offset={offset:x} ok={equal}"
                ));
                ok &= equal;
            }
        }
        STOP_GUARDED.store(if ok { 1 } else { 2 }, Ordering::Release);
    }
    if STOP_GUARDED.load(Ordering::Acquire) != 1
        || accessor < 0x1000
        || accessor & 7 != 0
        || accessor >> 47 != 0
    {
        return None;
    }
    let module = ((accessor + 0x90) as *const usize).read();
    if module < 0x1000 || module & 7 != 0 || module >> 47 != 0 {
        return None;
    }
    let vptr = (module as *const usize).read();
    if vptr != base + 0x4f8a0e0 {
        return None;
    }
    for (slot, target) in [
        (0xa0, 0x4df440),
        (0xd0, 0x4df690),
        (0xc0, 0x6ebb20),
        (0x100, 0x6ebd50),
    ] {
        if ((vptr + slot) as *const usize).read() != base + target {
            return None;
        }
    }
    Some(module)
}

pub(super) unsafe fn info_guarded() -> bool {
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
            diagnostic(format_args!(
                "[TechDirect] info_guard name={name} offset={offset:x} ok={equal}"
            ));
            ok &= equal;
        }
    }
    INFO_GUARDED.store(if ok { 1 } else { 2 }, Ordering::Release);
    ok
}

pub(super) use crate::model::sites_tech_launch::core::CORE_GUARDS;

static GUARDED: AtomicU8 = AtomicU8::new(0);
pub(super) unsafe fn launch_ready(accessor: usize) -> bool {
    if GUARDED.load(Ordering::Acquire) == 0 {
        let base = game::text_base();
        let ok = base != 0
            && CORE_GUARDS.iter().all(|&(offset, bytes)| {
                std::slice::from_raw_parts((base + offset) as *const u8, 64) == bytes.as_slice()
            });
        GUARDED.store(if ok { 1 } else { 2 }, Ordering::Release);
    }
    use crate::model::{sites_pkg2 as p2, sites_pkg3 as p3, sites_pkg4 as p4};
    GUARDED.load(Ordering::Acquire) == 1
        && crate::model::sites_tech_launch::SITES
            .iter()
            .all(|s| game::granted(s.offset))
        && game::granted(crate::model::sites_tech_launch::GET_ENERGY_OFFSET)
        && game::granted(crate::model::sites_hitbox::HITBOX_HIT_STOP_OFFSET)
        && v1_guarded()
        && v3_guarded()
        && info_guarded()
        && stop_module(accessor).is_some()
        && damage_info(accessor).is_some()
        && [
            p3::STATE_SET_SLEEP_OFFSET,
            p3::STATE_CLEAR_SPEED_ALL_OFFSET,
            p3::STATE_INIT_POS_OFFSET,
            p3::STATE_SET_POS_OFFSET,
            p3::STATE_SET_LR_OFFSET,
            p3::STATE_HEAL_OFFSET,
            p3::STATE_ADD_DAMAGE_OFFSET,
            p3::STATE_SET_SITUATION_KIND_OFFSET,
            p2::CHANGE_STATUS_FORCE_OFFSET,
            p2::WORK_SET_FLOAT_OFFSET,
            p4::WORK_SET_INT_OFFSET,
            p2::WORK_GET_INT_OFFSET,
            p2::STATUS_KIND_OFFSET,
            p2::SITUATION_KIND_OFFSET,
            p2::POS_X_OFFSET,
            p2::POS_Y_OFFSET,
            p2::DAMAGE_OF_OFFSET,
            p4::POSTURE_LR_OFFSET,
            p4::STOP_IS_STOP_OFFSET,
        ]
        .iter()
        .all(|&x| game::granted(x))
}

pub(super) unsafe fn damage_velocity(accessor: usize) -> Option<Vec2> {
    let base = game::text_base();
    let module = ((accessor + 0x68) as *const usize).read();
    if module < 0x1000 || module & 7 != 0 || module >> 47 != 0 {
        return None;
    }
    let vptr = (module as *const usize).read();
    if vptr != base + 0x4f88568 || ((vptr + 0x60) as *const usize).read() != base + 0x479d40 {
        return None;
    }
    let energy = natives::tech_energy(accessor, id("FIGHTER_KINETIC_ENERGY_ID_DAMAGE")?)?;
    if energy < 0x1000 || energy & 7 != 0 || energy >> 47 != 0 {
        return None;
    }
    let offset = (energy as *const usize)
        .read()
        .checked_sub(game::text_base())?;
    let target = match offset {
        0x4f88968 => 0x3b3540,
        0x4f88a18 => 0x6d8cc0,
        _ => return None,
    };
    if (((base + offset) + 0x20) as *const usize).read() != base + target {
        return None;
    }
    let (x, y) = natives::tech_speed(energy)?;
    let speed = Vec2 { x, y };
    (speed.x.is_finite() && speed.y.is_finite()).then_some(speed)
}
