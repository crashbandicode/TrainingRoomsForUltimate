//! Reads the stock attack forecast during a live Perfect Juggler attempt.
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    engine::seams::native_fields as field,
    model::{
        edge_juggle::perfect::startup_dodge::{
            neutral_dodge_window, AttackForecast, MotionSample, ThreatProjection,
        },
        hash40::hash40,
        sites_pkg2::WORK_GET_INT_OFFSET,
    },
    runtime::{game, roster::body_graph::PAD_MANAGER, seams::seam_install},
};

use super::{air_physics, konst, natives, C, UNRESOLVED};

static BASE: AtomicUsize = AtomicUsize::new(0);
const ESCAPE_PENALTY_FRAME: i32 = 0x1000_0079;

pub(super) fn install(installed: bool) {
    let base = if installed
        && seam_install::gate(
            "edge_juggle",
            &[
                "edge_juggle.attack_forecast_manager",
                "edge_juggle.attack_forecast_slots",
                "edge_juggle.attack_forecast_owner",
                "edge_juggle.attack_forecast_first",
                "edge_juggle.attack_forecast_last",
                "edge_juggle.attack_forecast_bounds",
                "edge_juggle.attack_forecast_scale",
            ],
            false,
        ) {
        game::text_base()
    } else {
        0
    };
    BASE.store(base, Ordering::Release);
}

/// Game thread, in the existing AI epilogue with both Training bodies live.
unsafe fn player_forecast(p1: usize, status: i32) -> Option<AttackForecast> {
    let base = BASE.load(Ordering::Acquire);
    if base == 0 {
        return None;
    }
    let object = natives::entry_active_body(natives::fighter_entry(0));
    let kind = natives::fighter_kind(p1)?;
    if object == 0 || natives::fighter_accessor(object) != p1 || kind < 0 || status < 0 {
        return None;
    }

    // Stock uses a fixed 16-slot table of 0x280-byte snapshots. Compare
    // against the live entry body before reading data from its slot.
    let slots = field::chain(base, &[PAD_MANAGER, 0, 0x90])?;
    for i in 0..16 {
        let slot = slots + i * 0x280;
        if field::read::<i32>(slot, 0x28)? != kind || field::read::<usize>(slot, 0x10)? != object {
            continue;
        }
        return Some(AttackForecast {
            source: MotionSample {
                status: field::read(slot, 0x2c)?,
                motion: field::read(slot, 0x38)?,
                frame: field::read(slot, 0x48)?,
            },
            flags: field::read(slot, 0x54)?,
            countdown: [field::read(slot, 0x120)?, field::read(slot, 0x124)?],
            bounds: [field::read(slot, 0x140)?, field::read(slot, 0x150)?],
        });
    }
    None
}

/// Only predict an unstaled neutral dodge against a grounded, currently
/// forecast attack. Active hitboxes retain the existing capsule path.
pub(super) unsafe fn covers_contact(p1: usize, cpu: usize, status: i32) -> bool {
    let ground = konst(C::SituationGround);
    if ground == UNRESOLVED
        || natives::situation_kind(p1) != ground
        || !game::granted(WORK_GET_INT_OFFSET)
        || natives::get_int(cpu, ESCAPE_PENALTY_FRAME) != 0
    {
        return false;
    }
    let qualify = || {
        let forecast = player_forecast(p1, status)?;
        let motion = MotionSample {
            status,
            motion: natives::motion_kind(p1),
            frame: natives::motion_frame(p1),
        };
        let (remaining_active, attack) = forecast.for_motion(motion, natives::lr(p1))?;
        let origin = [natives::pos_x(p1), natives::pos_y(p1)];
        let [hx, hy, _] = natives::joint_position(cpu, hash40("hip"), &[0.0; 3])?;
        let [tx, ty, _] = natives::joint_position(cpu, hash40("head"), &[0.0; 3])?;
        let (cx, cy) = crate::runtime::features::recording_host::fighter_speed(cpu)?;
        let (px, py) = crate::runtime::features::recording_host::fighter_speed(p1)?;
        Some(ThreatProjection {
            remaining_active,
            attack,
            torso: [
                [hx - origin[0], hy - origin[1]],
                [tx - origin[0], ty - origin[1]],
            ],
            cpu_speed: [cx, cy],
            attacker_speed: [px, py],
            cpu_air: air_physics(cpu),
            invulnerable: neutral_dodge_window(natives::fighter_kind(cpu)?)?,
        })
    };
    qualify().is_some_and(ThreatProjection::dodge_covers_contact)
}
