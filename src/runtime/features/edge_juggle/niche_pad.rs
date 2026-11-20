//! Native observations for the niche route pad sequence.

use super::{
    air_physics, airborne_recovery, attack_reaches_core, konst, natives, perfect,
    recovery_snapshot, remaining_jumps, special_reach, state, C, UNRESOLVED,
};
use crate::{
    model::{sites_pkg2, sites_pkg3},
    runtime::game,
};
use perfect::niche::{self, Budget, Facts, Motion, Move, Row, Sample, Sequence};
use perfect::{recovery, AirPhysics, Correction, Point, Route, RouteFacts, SpecialReach};

pub(super) struct NicheGuide {
    sequence: Sequence,
    fighter: i32,
    ledge: Point,
    across: Option<Crossing>,
    player_x: f32,
}

struct Crossing {
    near: f32,
    far: f32,
    band: Point,
    direction: f32,
    exit: f32,
}

/// What the plan already read for the row's need and its crossing budget.
pub(super) struct RowInputs {
    pub short: bool,
    pub air: AirPhysics,
    pub jumps: u8,
    pub special: Option<SpecialReach>,
}

impl NicheGuide {
    /// The fighter's row for this plan with its spare climb. The caller rolls a crossing among
    /// the routes; an extension row is a need.
    pub(super) unsafe fn prepare_niche(
        cpu: usize,
        p1: usize,
        fighter: i32,
        facts: &RouteFacts<'_>,
        band: Option<Point>,
        inputs: RowInputs,
    ) -> Option<(Self, f32)> {
        let [left, right] = facts.ledges;
        let center = left[0].midpoint(right[0]);
        let [x, y] = facts.position;
        let (near, far, direction) = if x < center {
            (left, right, 1.0)
        } else {
            (right, left, -1.0)
        };
        let outside = (near[0] - x) * direction;
        if !(outside > 0.0 && y < near[1] && facts.player.into_iter().all(f32::is_finite)) {
            return None;
        }
        let exit = far[0] + direction * facts.body_clearance;
        let row_facts = Facts {
            fighter,
            corridor: band,
            // The zone weights decide how often a feasible crossing is flown.
            covered_ledge: true,
            player_over_crossing: (facts.player[0] - center).abs() <= facts.player_reach,
            long_approach: outside > facts.body_clearance * 2.0,
            counter_contact: fighter == 0x34 && attack_reaches_core(p1, cpu) == Some(true),
            short: inputs.short,
            climb: near[1] - y,
            budget: band.map(|band| Budget {
                air: inputs.air,
                jumps: inputs.jumps,
                special: inputs.special,
                crossing: recovery::Crossing {
                    span: (exit - x).abs(),
                    band,
                    foot: y,
                    ledge: far[1],
                },
            }),
        };
        let row = niche::select(row_facts, niche::CANDIDATE_ROWS)?;
        let margin = row.margin(row_facts)?;
        let across = if row.route == Route::Opposite {
            Some(Crossing {
                near: near[0],
                far: far[0],
                band: band?,
                direction,
                exit,
            })
        } else {
            None
        };
        let guide = Self {
            sequence: Sequence::new(row),
            fighter,
            ledge: if across.is_some() { far } else { near },
            across,
            player_x: facts.player[0],
        };
        Some((guide, margin))
    }

    /// Where a crossing leaves the stage: the far ledge plus one body.
    pub(super) fn exit_point(&self) -> Point {
        self.across
            .as_ref()
            .map_or(self.ledge, |crossing| [crossing.exit, self.ledge[1]])
    }

    pub(super) fn selected_niche_row(&self) -> &'static Row {
        self.sequence.row()
    }

    pub(super) unsafe fn correct_niche_pad(
        &mut self,
        ai: usize,
        cpu: usize,
        [x, y]: Point,
        status: i32,
        frame: u32,
    ) -> Option<Correction> {
        let movement = self.sequence.action()?;
        let air = konst(C::SituationAir);
        if !x.is_finite()
            || !y.is_finite()
            || air == UNRESOLVED
            || natives::situation_kind(cpu) != air
        {
            return None;
        }
        let (snapshot, available) = recovery_snapshot(ai)?;
        let reach = special_reach(ai, snapshot)?;
        let phase = niche_motion_phase(cpu, self.fighter, movement, status);
        let (mut direction, mut aim, mut at_exit, mut can_enter) = (
            (self.ledge[0] - x).signum(),
            normalized_stick([self.ledge[0] - x, self.ledge[1] - y]),
            true,
            true,
        );
        let (mut low, mut hold_over) = (false, true);
        if let Some(crossing) = self.across.as_ref() {
            let entered = (x - crossing.near) * crossing.direction >= 0.0;
            let beyond = (x - crossing.far) * crossing.direction > 0.0;
            let [bottom, top] = crossing.band;
            if !beyond && (y < bottom || entered && y > top) {
                return None;
            }
            at_exit = (x - crossing.exit) * crossing.direction >= 0.0;
            can_enter = beyond || y <= top;
            if !at_exit {
                direction = crossing.direction;
                aim = [direction, 0.0];
            }
            hold_over = at_exit;
            if movement == Move::Hold && !at_exit {
                let air = air_physics(cpu);
                let rise = air.jump_speed.powi(2) / (2.0 * air.gravity);
                // A jump from the hold line peaks at the band top; a taller jump waits for the exit.
                low = air.valid() && rise <= top - bottom && y < top - rise;
                // An aimed special starts where its level travel still leaves the climb to the ledge.
                let remaining = (crossing.exit - x) * crossing.direction;
                hold_over = available
                    && self.selected_niche_row().aimed()
                    && remaining <= recovery::level_run(reach, self.ledge[1] - y);
            }
        }
        if movement == Move::Hover && self.fighter == 0x2d && !has_burner(cpu) {
            return None;
        }
        let approaching =
            self.selected_niche_row().id == 2 && movement == Move::Down && (self.player_x - x) * direction > 0.0;
        if matches!(movement, Move::Bolt) && phase == Motion::Charging {
            aim = bolt_input(cpu, self.fighter, [x, y], direction, aim)?;
        }
        if matches!(movement, Move::Rise | Move::SoraChain)
            && at_exit
            && !reach.reaches([x, y], self.ledge)
            && phase == Motion::Free
        {
            return None;
        }
        self.sequence.step(Sample {
            frame,
            motion: phase,
            can_start: phase == Motion::Free && niche_start_enabled(cpu, movement, available),
            valid: true,
            at_exit,
            can_enter,
            approaching,
            low,
            hold_over,
            direction,
            aim,
        })
    }
}

fn normalized_stick([x, y]: Point) -> Point {
    let length = x.hypot(y);
    if length.is_finite() && length > 0.0 {
        [x / length, y / length]
    } else {
        [0.0, 1.0]
    }
}

unsafe fn niche_start_enabled(cpu: usize, movement: Move, available: bool) -> bool {
    use Move::*;
    if movement == SoraChain && !game::granted(sites_pkg2::WORK_IS_FLAG_OFFSET) {
        return false;
    }
    let term = match movement {
        Drift => return true,
        Hold | Jump | Float if remaining_jumps(cpu).is_some_and(|jumps| jumps != 0) => {
            C::TermContJumpAerial
        }
        Hold | Jump | Float => return false,
        Side | ChargedSide | Missile => C::TermContSpecialS,
        Down | Counter | Bomb => C::TermContSpecialLw,
        _ if available => C::TermContSpecialHi,
        _ => return false,
    };
    let term = konst(term);
    term != UNRESOLVED && natives::is_enable_transition_term(cpu, term)
}

/// These move-specific status enums belong to the exact fighter body checked by the caller.
/// Values come from the pinned 13.0.5 lua constants. Common status IDs use the existing resolver.
/// An unrecognized status (including damage and special-fall) returns to the stock controller.
unsafe fn niche_motion_phase(cpu: usize, fighter: i32, movement: Move, status: i32) -> Motion {
    use Motion::{AimAgain, Charging, Finish, Free, Interrupted, Starting};
    use Move::*;
    if status == UNRESOLVED {
        return Interrupted;
    }
    if matches!(movement, Hold | Jump) && status == konst(C::StatusJumpAerial) {
        return Motion::Travel;
    }
    if airborne_recovery(cpu, status) {
        return Free;
    }
    let start = match movement {
        Side | ChargedSide | Missile => C::StatusSpecialS,
        Down | Counter | Bomb => C::StatusSpecialLw,
        Drift | Hold | Jump | Float => {
            return match (fighter, movement, status) {
                (0xd | 0xe, Float, 0x1e8) => Starting,
                (0xd | 0xe, Float, 0x1e9) => Motion::Travel,
                _ => Interrupted,
            }
        }
        _ => C::StatusSpecialHi,
    };
    if status == konst(start) {
        if movement == SoraChain {
            return if !game::granted(sites_pkg2::WORK_IS_FLAG_OFFSET) {
                Interrupted
            } else if natives::is_flag(cpu, 0x2100_0010) {
                // TRAIL_STATUS_SPECIAL_HI_FLAG_COMMAND_ACCEPT opens Sonic Blade.
                AimAgain
            } else {
                Starting
            };
        }
        return Starting;
    }
    if movement == SoraChain && status == konst(C::StatusSpecialS) {
        return Motion::Travel;
    }
    match (fighter, movement, status) {
        (0x8 | 0x13, QuickAttack, 0x1e1) => {
            // FIGHTER_PIKACHU_STATUS_WORK_ID_INT_QUICK_ATTACK_COUNT, shared with Pichu.
            if !game::granted(sites_pkg2::WORK_GET_INT_OFFSET) {
                return Interrupted;
            }
            match natives::get_int(cpu, 0x1100_0005) {
                0 => Motion::Travel,
                1 | 2 => AimAgain,
                _ => Interrupted,
            }
        }
        (0x27, Barrel, 0x1f0) | (0x9, Missile, 0x1e1) => Charging,
        (0xa | 0x28, Bolt, 0x1e4) => Charging,
        (0x5d, SoraChain, 0x1ec | 0x1ed) => AimAgain,
        (0x8, Side, 0x1e3..=0x1e5)
        | (0x8, Down, 0x1e7)
        | (0x10, Down, 0x1e9)
        | (0x10, Travel, 0x1e7)
        | (0x1d, Side, 0x1e3)
        | (0x1e | 0x1f, Travel, 0x1e7)
        | (0x22, Hover, 0x1f4)
        | (0x27, Side, 0x1e5 | 0x1ee)
        | (0x27, Barrel, 0x1f2)
        | (0x2b, Hover, 0x1e5)
        | (0x2c, Travel, 0x1e6)
        | (0x2d, Hover, 0x1e3)
        | (0x34, Side, 0x1e8 | 0x1ea)
        | (0x34, Rise, 0x1ec | 0x1ed)
        | (0x34, Counter, 0x1ee)
        | (0x35, ChargedSide, 0x1e5)
        | (0x35, Travel, 0x1e7)
        | (0x37, Travel, 0x1ec)
        | (0x3a, Side, 0x1e3 | 0x1e4)
        | (0x3a, Rise, 0x1e7)
        | (0x40, Side, 0x1e8 | 0x1e9)
        | (0x40, Travel, 0x1ed)
        | (0x29, Side, 0x1e8..=0x1ea)
        | (0x29, Rise, 0x1ef)
        | (0x53, Rise, 0x1eb | 0x1ec)
        | (0x58, Glide, 0x20a..=0x20c)
        | (0x5d, SoraChain, 0x1eb)
        | (0x9, Missile, 0x1e2)
        | (0xa | 0x28, Bolt, 0x1e6)
        | (0x2, Bomb, 0x1e5) => Motion::Travel,
        (0x2, Rise, 0x1e3)
        | (0x8 | 0x13, QuickAttack, 0x1e2)
        | (0x8, Side, 0x1e6)
        | (0xa | 0x28, Bolt, 0x1e5)
        | (0xd | 0xe, Rise, 0x1e4..=0x1e6)
        | (0x10, Down, 0x1ea | 0x1eb)
        | (0x10, Travel, 0x1e8)
        | (0x1c, Rise, 0x1e1 | 0x1e2)
        | (0x1d, Side, 0x1e4)
        | (0x1e | 0x1f, Travel, 0x1e8)
        | (0x22, Hover, 0x1f5)
        | (0x2c, Travel, 0x1e7)
        | (0x34, Side, 0x1e9 | 0x1eb)
        | (0x35, ChargedSide, 0x1e6)
        | (0x35, Travel, 0x1e9)
        | (0x37, Travel, 0x1ed)
        | (0x3a, Rise, 0x1e8)
        | (0x5d, SoraChain, 0x1ee) => Finish,
        _ => Interrupted,
    }
}

unsafe fn has_burner(cpu: usize) -> bool {
    // FIGHTER_ROBOT_INSTANCE_WORK_ID_FLOAT_BURNER_ENERGY_VALUE
    if !game::granted(sites_pkg2::WORK_GET_FLOAT_OFFSET) {
        return false;
    }
    let fuel = natives::get_float(cpu, 0x4e);
    fuel.is_finite() && fuel > 0.0
}

fn bolt_reads_granted() -> bool {
    [
        sites_pkg3::STATE_ARTICLE_IS_EXIST_OFFSET,
        sites_pkg3::STATE_ARTICLE_GET_ARTICLE_OFFSET,
        sites_pkg3::STATE_ARTICLE_OBJECT_ID_OFFSET,
        sites_pkg3::STATE_OBJECT_MODULE_ACCESSOR_OFFSET,
        sites_pkg3::STATE_OBJECT_IS_ACTIVE_OFFSET,
    ]
    .into_iter()
    .all(game::granted)
}

unsafe fn bolt_input(
    cpu: usize,
    fighter: i32,
    player: Point,
    direction: f32,
    launch: Point,
) -> Option<Point> {
    if !bolt_reads_granted() {
        return None;
    }
    const PK_THUNDER: i32 = 2;
    let kind = match fighter {
        0xa => 0x61,
        0x28 => 0x163,
        _ => return None,
    };
    if !state::article_is_exist(cpu, PK_THUNDER) {
        // Charge has begun. Wait for the native article to spawn.
        return Some([-direction, 0.0]);
    }
    let article = state::article_get(cpu, PK_THUNDER);
    if article == 0 {
        return None;
    }
    let id = state::article_object_id(article);
    if id == 0 || !state::object_is_active(id) {
        return None;
    }
    let bolt = state::object_accessor(id);
    if bolt == 0 || natives::fighter_kind(bolt) != Some(kind) {
        return None;
    }
    let [x, y] = player;
    let [bx, by] = [natives::pos_x(bolt), natives::pos_y(bolt)];
    let scale = super::fighter_height(cpu, y)?;
    if !bx.is_finite() || !by.is_finite() {
        return None;
    }
    let behind = [x - launch[0] * scale, y - launch[1] * scale];
    if (bx - x) * launch[0] + (by - y) * launch[1] < -scale {
        Some(normalized_stick([x - bx, y - by]))
    } else {
        Some(normalized_stick([behind[0] - bx, behind[1] - by]))
    }
}
