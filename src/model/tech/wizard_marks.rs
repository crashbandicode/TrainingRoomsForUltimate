//! Shared fixed-capacity Tech picker/wizard marks. The four flight dots are drawing
//! only, using the existing approximate flight; native outcomes stay authoritative.
use super::picker::{arrow_points, Spot};
use super::wizard::{Step, Wizard};
use super::{add, scale, Vec2};
use crate::model::btt::LOOKS;
use crate::model::panes::{EggLook, EggPicture, EGG_WHOLE};
use crate::model::tech_launch::{flight, launch_velocity, FlightParams};

pub fn marks(
    spot: Option<Spot>,
    cursor: Option<Vec2>,
    wizard: Option<&Wizard>,
    params: Option<FlightParams>,
    admitted: bool,
) -> [Option<([f32; 2], EggLook)>; LOOKS] {
    let mut out = [None; LOOKS];
    // Cursor visibility is independent of hover/selection validity. The caller
    // supplies it only for full placing or the wizard's Spot step.
    if let Some(cursor) = cursor {
        out[LOOKS - 1] = Some((
            [cursor.x, cursor.y],
            EggLook {
                picture: EggPicture::Cursor,
                ..EGG_WHOLE
            },
        ));
    }
    let Some(spot) = spot else {
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
    let (direction, tail, tip, stem, head) = if let Some(wizard) = wizard {
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
        (direction, tail, add(tail, scale(direction, 7.)), 7., 2.)
    } else {
        if cursor.is_none() {
            return out;
        }
        let normal = spot.line.normal;
        (
            normal,
            add(spot.point, scale(normal, 2.5)),
            add(spot.point, scale(normal, 5.)),
            2.5,
            1.5,
        )
    };
    let mut arrow = arrow_points(tip, direction, stem, head);
    // Keep both paths' original stem arithmetic rather than subtracting from
    // a rounded tip; the wizard stem may be the exact planner start.
    arrow[0] = tail;
    for (slot, p) in out[1..5].iter_mut().zip(arrow) {
        *slot = marker(p, 0.12, 255);
    }
    if let Some(wizard) =
        wizard.filter(|w| matches!(w.step, Step::Distance | Step::Hit | Step::Preview))
    {
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
