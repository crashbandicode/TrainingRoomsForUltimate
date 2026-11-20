//! Native observations for the niche route pad sequence.

use super::{
    airborne_recovery, attack_reaches_core, konst, natives, perfect,
    recovery_snapshot, remaining_jumps, special_reach, state, C, UNRESOLVED,
};
use crate::{
    model::{sites_pkg2, sites_pkg3},
    runtime::game,
};
use perfect::fly::{puff_active_status, FlyState, PuffFlight};
use perfect::niche::{self, Budget, Facts, Motion, Move, NearReach, Row, Sample, Sequence};
use perfect::pikachu::{self, SkullBashFlight};
use perfect::{recovery, AirPhysics, Correction, Outline, Point, Route, RouteFacts, SpecialReach};

pub(super) struct NicheGuide {
    sequence: Sequence,
    fighter: i32,
    ledge: Point,
    across: Option<Crossing>,
    measured: Option<MeasuredFlight>,
    body_clearance: f32,
}

enum MeasuredFlight {
    SkullBash(SkullBashFlight),
    Puff(PuffFlight),
}

struct Crossing {
    near: f32,
    far: f32,
    band: Point,
    direction: f32,
    exit: f32,
    air: AirPhysics,
}

/// What the plan already read for the row's need and its crossing budget.
pub(super) struct RowInputs {
    pub short: bool,
    pub speed: Point,
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
        if !(outside > 0.0 && facts.player.into_iter().all(f32::is_finite)) {
            return None;
        }
        if fighter == 0x8 {
            band?;
            let path = pikachu::skull_bash_start(
                facts.stage, facts.outline, facts.ledges, facts.position, facts.blast[3],
                facts.body_clearance,
            )?;
            let row = niche::measured_row(3, niche::CANDIDATE_ROWS)?;
            if inputs.special.is_none()
                || inputs.jumps == 0
                || inputs.speed[0].abs() > inputs.air.horizontal_acceleration
                || inputs.speed[1] > 0.0
                || ![Move::Side, Move::Jump, Move::QuickAttack]
                    .into_iter()
                    .all(|movement| niche_start_enabled(cpu, movement, true))
            {
                return None;
            }
            let mut outline = Outline::new();
            for &line in facts.outline {
                outline.add(line);
            }
            let guide = Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: path.far,
                across: None,
                measured: Some(MeasuredFlight::SkullBash(SkullBashFlight::new(
                    path, outline, facts.blast[3],
                ))),
                body_clearance: facts.body_clearance,
            };
            return Some((guide, 1.0));
        }
        if fighter == 0xc {
            let row = niche::measured_row(17, niche::CANDIDATE_ROWS)?;
            let flight = PuffFlight::start(facts, FlyState {
                position: facts.position,
                velocity: inputs.speed,
                remaining: inputs.jumps,
                can_jump: false,
                frame: 0,
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                across: None,
                measured: Some(MeasuredFlight::Puff(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        // A crossing starts below the near ledge. An extension row sees every height and
        // applies its own rule (`climb`).
        let band = band.filter(|_| y < near[1]);
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
            near: inputs.special.map(|special| NearReach {
                gap: outside,
                climb: near[1] - y,
                body: facts.body_clearance,
                special,
                ledge_covered: (facts.player[0] - near[0]).hypot(facts.player[1] - near[1])
                    <= facts.player_reach,
            }),
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
                air: inputs.air,
            })
        } else {
            None
        };
        let guide = Self {
            sequence: Sequence::new(row),
            fighter,
            ledge: if across.is_some() { far } else { near },
            across,
            measured: None,
            body_clearance: facts.body_clearance,
        };
        Some((guide, margin))
    }

    /// Where a crossing leaves the stage: the far ledge plus one body.
    pub(super) fn exit_point(&self) -> Point {
        self.across
            .as_ref()
            .map_or(self.ledge, |crossing| [crossing.exit, self.ledge[1]])
    }

    pub(super) fn crossing_frames(&self) -> Option<u16> {
        match &self.measured {
            // Measured from the Skull Bash press to the exit: 13+39+46+27+33.
            Some(MeasuredFlight::SkullBash(_)) => Some(158),
            _ => None,
        }
    }

    pub(super) fn using_bomb(&self) -> bool {
        self.sequence.action() == Some(Move::Bomb)
    }

    pub(super) fn accepts_form(&self, fighter: Option<i32>) -> bool {
        self.sequence.accepts_form(fighter)
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
        let reach = if available { Some(special_reach(ai, snapshot)?) } else { None };
        if let Some(MeasuredFlight::Puff(flight)) = &mut self.measured {
            if !puff_active_status(status, airborne_recovery(cpu, status)) {
                return None;
            }
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let jump = konst(C::TermContJumpAerial);
            return flight.step(FlyState {
                position: [x, y],
                velocity: [vx, vy],
                remaining: remaining_jumps(cpu)?,
                can_jump: jump != UNRESOLVED && natives::is_enable_transition_term(cpu, jump),
                frame,
            });
        }
        if let Some(MeasuredFlight::SkullBash(skull_bash)) = &mut self.measured {
            let (_, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let motion = niche_motion_phase(cpu, self.fighter, movement, status);
            return skull_bash.step(&mut self.sequence, Sample {
                frame,
                motion,
                can_start: motion == Motion::Free && niche_start_enabled(cpu, movement, available),
                valid: true,
                at_exit: false,
                can_enter: false,
                low: false,
                hold_over: false,
                direction: 0.0,
                aim: [0.0; 2],
            }, [x, y], vy, remaining_jumps(cpu)?);
        }

        let (mut direction, mut aim, mut at_exit, mut can_enter) = (
            (self.ledge[0] - x).signum(),
            normalized_stick([self.ledge[0] - x, self.ledge[1] - y]),
            true,
            true,
        );
        let (mut low, mut hold_over) = (false, true);
        let native_phase = (movement != Move::Bomb)
            .then(|| niche_motion_phase(cpu, self.fighter, movement, status));
        if let Some(crossing) = self.across.as_ref() {
            let entered = (x - crossing.near) * crossing.direction >= 0.0;
            let beyond = (x - crossing.far) * crossing.direction > 0.0;
            let [bottom, top] = crossing.band;
            if !beyond && (y < bottom || entered && y > top) {
                return None;
            }
            // Outside the near ledge the crossing can still be given up: read its budget again.
            if !entered && native_phase == Some(Motion::Free) {
                let budget = Budget {
                    air: crossing.air,
                    jumps: remaining_jumps(cpu)?,
                    special: reach,
                    crossing: recovery::Crossing {
                        span: (crossing.exit - x).abs(),
                        band: crossing.band,
                        foot: y,
                        ledge: self.ledge[1],
                    },
                };
                if !self.sequence.still_crosses(budget) {
                    return None;
                }
            }
            at_exit = (x - crossing.exit) * crossing.direction >= 0.0;
            let launch_clears = beyond || self.selected_niche_row().launch_clears(y, top);
            can_enter = if movement == Move::Hover { launch_clears } else { beyond || y <= top };
            if !at_exit {
                direction = crossing.direction;
                aim = [direction, 0.0];
            }
            hold_over = at_exit;
            if movement == Move::Hold && !at_exit {
                let air = crossing.air;
                let rise = air.jump_speed.powi(2) / (2.0 * air.gravity);
                // A jump from the hold line peaks at the band top; a taller jump waits for the exit.
                low = air.valid() && rise <= top - bottom && y < top - rise;
                // An aimed special starts where its level travel still leaves the climb to the ledge.
                let remaining = (crossing.exit - x) * crossing.direction;
                hold_over = launch_clears
                    && self.selected_niche_row().aimed()
                    && reach.is_some_and(|reach| {
                        remaining <= recovery::level_run(reach, self.ledge[1] - y)
                    });
            }
        }
        let (phase, actionable) = match native_phase {
            Some(phase) => (phase, phase == Motion::Free),
            None => link_bomb_phase(cpu, status, [x, y], direction, self.body_clearance)?,
        };
        if movement == Move::Hover && self.fighter == 0x2d && !has_burner(cpu) {
            return None;
        }
        if matches!(movement, Move::Bolt) && phase == Motion::Charging {
            aim = bolt_input(cpu, self.fighter, [x, y], direction, aim)?;
        }
        if matches!(movement, Move::Rise | Move::SoraChain)
            && at_exit
            && reach.is_none_or(|reach| !reach.reaches([x, y], self.ledge))
            && phase == Motion::Free
        {
            return None;
        }
        self.sequence.step(Sample {
            frame,
            motion: phase,
            can_start: actionable && niche_start_enabled(cpu, movement, available),
            valid: true,
            at_exit,
            can_enter,
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
        Down | Counter | Bomb | Swap => C::TermContSpecialLw,
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
        Down | Counter | Bomb | Swap => C::StatusSpecialLw,
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
        (0x28, Bolt, 0x1e4) => Charging,
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
        | (0x28, Bolt, 0x1e6)
        | (0x47, Rise, 0x1e6..=0x1e8)
        | (0x5a, Swap, 0x1e1 | 0x1e2)
        | (0x5b, Side, 0x1e9)
        | (0x5b, Rise, 0x1eb..=0x1ed) => Motion::Travel,
        (0x2, Rise, 0x1e3)
        | (0x8 | 0x13, QuickAttack, 0x1e2)
        | (0x8, Side, 0x1e6)
        | (0x28, Bolt, 0x1e5)
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
        | (0x5d, SoraChain, 0x1ee)
        | (0x47, Side, 0x1e4 | 0x1e5)
        | (0x47, Rise, 0x1e9 | 0x1ea)
        | (0x5b, Side, 0x1ea)
        | (0x5b, Rise, 0x1ee) => Finish,
        _ => Interrupted,
    }
}

fn link_bomb_reads_granted() -> bool {
    [
        sites_pkg2::WORK_GET_INT_OFFSET,
        sites_pkg3::STATE_OBJECT_IS_ACTIVE_OFFSET,
        sites_pkg3::STATE_OBJECT_MODULE_ACCESSOR_OFFSET,
    ]
    .into_iter()
    .all(game::granted)
}

unsafe fn link_bomb_phase(
    cpu: usize,
    status: i32,
    [x, y]: Point,
    inward: f32,
    clearance: f32,
) -> Option<(Motion, bool)> {
    use smash::app::{lua_bind::ItemModule, BattleObjectModuleAccessor};

    if !link_bomb_reads_granted() || !crate::runtime::roster::running() {
        return None;
    }
    if status == UNRESOLVED {
        return None;
    }
    if status == konst(C::StatusSpecialLw) {
        return Some((Motion::Starting, false));
    }
    if status == 0x1e5 {
        // FIGHTER_LINK_STATUS_KIND_SPECIAL_LW_BLAST.
        return Some((Motion::BombBlast, false));
    }
    if matches!(status, 0x48..=0x4b) {
        // Native DamageAir / DamageFly family. The model requires a preceding detonation.
        return Some((Motion::BombHit, false));
    }
    if !airborne_recovery(cpu, status) {
        return None;
    }
    let body = cpu as *mut BattleObjectModuleAccessor;
    let held = ItemModule::is_have_item(body, 0)
        .then(|| ItemModule::get_have_item_id(body, 0));
    // FIGHTER_LINK_INSTANCE_WORK_ID_INT_BOMB_OBJECT_ID.
    let id = u32::try_from(natives::get_int(cpu, 0x1000_00c3)).ok()?;
    if id == 0x5000_0000 || !state::object_is_active(id) {
        return held.is_none().then_some((Motion::Free, true));
    }
    let bomb = state::object_accessor(id);
    if bomb == 0 || natives::fighter_kind(bomb) != Some(0x78) {
        return None;
    }
    if let Some(held) = held {
        return (held == u64::from(id)).then_some((Motion::ItemHeld, false));
    }
    let dx = (x - natives::pos_x(bomb)) * inward;
    let dy = y - natives::pos_y(bomb);
    // Let the dropped bomb fall behind Link so the launch bends toward the stage.
    (dx.hypot(dy) < clearance).then_some((Motion::ItemLoose, dx > 0.0 && dy >= 0.0))
}

pub(super) fn pyra_swapping(status: i32) -> bool {
    status != UNRESOLVED
        && (status == konst(C::StatusSpecialLw) || matches!(status, 0x1e1 | 0x1e2))
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
    if fighter != 0x28 || !bolt_reads_granted() {
        return None;
    }
    const PK_THUNDER: i32 = 2;
    const LUCAS_THUNDER_KIND: i32 = 0x163;
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
    if bolt == 0 || natives::fighter_kind(bolt) != Some(LUCAS_THUNDER_KIND) {
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
