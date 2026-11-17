//! Fixed-capacity wizard presentation. The four trajectory dots are drawing
//! only, using the existing approximate flight; native outcomes stay authoritative.
use super::picker::arrow_points;
use super::wizard::{Step, Wizard};
use super::{add, scale, Vec2};
use crate::model::btt::LOOKS;
use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
use crate::model::tech_launch::{flight, launch_velocity, FlightParams};

pub fn marks(
    wizard: &Wizard,
    cursor: Vec2,
    params: Option<FlightParams>,
    admitted: bool,
) -> [Option<([f32; 2], EggLook)>; LOOKS] {
    let mut out = [None; LOOKS];
    // The Spot cursor belongs to input positioning, independently of a valid
    // hover/selection. Keep it even when later target validation returns early.
    if wizard.step == Step::Spot {
        out[LOOKS - 1] = Some((
            [cursor.x, cursor.y],
            EggLook {
                picture: EggPicture::Cursor,
                ..EGG_WHOLE
            },
        ));
    }
    let Some(spot) = wizard.selected else {
        return out;
    };
    let marker = |p: Vec2, size, alpha| {
        Some((
            [p.x, p.y],
            EggLook {
                scale: size,
                alpha,
                ..EGG_WHOLE
            },
        ))
    };
    out[0] = marker(spot.point, 0.3, 255);
    let Some(angle) = wizard.world_angle() else {
        return out;
    };
    let radians = angle.to_radians();
    let direction = Vec2 {
        x: radians.cos(),
        y: radians.sin(),
    };
    let tail = if matches!(wizard.step, Step::Distance | Step::Hit | Step::Preview) {
        wizard.start_marker().unwrap_or(spot.point)
    } else {
        add(spot.point, scale(direction, -7.))
    };
    let tip = add(tail, scale(direction, 7.));
    let mut arrow = arrow_points(tip, direction, 7., 2.);
    // Retain the original tail exactly: subtracting the stem from its computed
    // tip can round differently from the actual planner start.
    arrow[0] = tail;
    for (slot, p) in out[1..5].iter_mut().zip(arrow) {
        *slot = marker(p, 0.12, 255);
    }
    if matches!(wizard.step, Step::Distance | Step::Hit | Step::Preview) {
        if let Some(start) = wizard.start_marker() {
            out[5] = marker(start, 0.65, 110);
            if wizard.step == Step::Preview && admitted {
                // Ten Whole panes total, within the existing typed BTT pool.
                if let Some(params) = params {
                    if let Some(v) = launch_velocity(wizard.speed, direction) {
                        for (slot, frame) in out[6..10].iter_mut().zip([3, 6, 9, 12]) {
                            if let Some(flight) = flight(v, params, frame) {
                                *slot = marker(
                                    add(
                                        start,
                                        Vec2 {
                                            x: flight.displacement.x,
                                            y: flight.displacement.y,
                                        },
                                    ),
                                    0.1,
                                    160,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    out
}
