//! Native observations for the niche route pad sequence.

use super::{
    airborne_recovery, attack_reaches_core, konst, natives, perfect,
    recovery_snapshot, remaining_jumps, special_reach, state, C, UNRESOLVED,
};
use crate::{
    model::{sites_pkg2, sites_pkg3},
    runtime::game,
};
use perfect::diddy::{self, BarrelFlight, BarrelSample};
use perfect::fly::{puff_active_status, FlyState, PuffFlight};
use perfect::greninja::{self, HydroFlight, HydroSample};
use perfect::incineroar::{self, WhipBeat, WhipReturn};
use perfect::kart::{KartFlight, KartState};
use perfect::link::{self, LinkBeat, LinkFlight, OwnedBomb};
use perfect::lucario::{self, AuraFlight, AuraSample};
use perfect::mario::{self, CapeBeat, CapeStall};
use perfect::recovery::Zone;
use perfect::niche::{self, Facts, Motion, Move, NearReach, Row, Sample, Sequence};
use perfect::olimar::{self, OlimarBeat, OlimarFlight};
use perfect::pacman::{self, PacBeat, PacFlight};
use perfect::peach::{FloatBeat, FloatFlight};
use perfect::pichu::{PichuBeat, PichuFakeOut, PichuFlight};
use perfect::pikachu::{self, FakeOutFlight, FakeOutSample, SkullBashFlight};
use perfect::pyra::{self, Beat as PyraBeat, SwapReturn};
use perfect::rob::{self, BurnerFlight, BurnerState};
use perfect::sheik::{SheikBeat, SheikFlight};
use perfect::snake::{self, CypherFlight, CypherSample};
use perfect::sora_flight::{self, SoraBeat, SoraFlight};
use perfect::steve::{self, ElytraBeat, ElytraFlight};
use perfect::wings::{self, WingsFlight};
use perfect::{AirPhysics, Correction, Outline, Point, RouteFacts, SpecialReach};

pub(super) struct NicheGuide {
    sequence: Sequence,
    fighter: i32,
    ledge: Point,
    measured: Option<MeasuredFlight>,
    body_clearance: f32,
}

enum MeasuredFlight {
    SkullBash(SkullBashFlight),
    FakeOut(FakeOutFlight),
    Puff(PuffFlight),
    Parasol(FloatFlight),
    Burner(BurnerFlight),
    Wings(WingsFlight),
    Pichu(PichuFlight),
    PichuFakeOut(PichuFakeOut),
    Olimar(OlimarFlight),
    Sheik(SheikFlight),
    Sora(SoraFlight),
    Kart(KartFlight),
    Link(LinkFlight),
    Cypher(CypherFlight),
    Cape(CapeStall),
    Aura(AuraFlight),
    Hydro(HydroFlight),
    Diddy(BarrelFlight),
    PacMan(PacFlight),
    Elytra(ElytraFlight),
    Whip(WhipReturn),
    Swap(SwapReturn),
}

/// What the plan already read for the row's need and its crossing budget.
pub(super) struct RowInputs {
    pub frame: u32,
    pub zone: Zone,
    pub draw: u32,
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
        let (near, direction) = if x < center { (left, 1.0) } else { (right, -1.0) };
        let outside = (near[0] - x) * direction;
        if !(outside > 0.0 && facts.player.into_iter().all(f32::is_finite)) {
            return None;
        }
        if fighter == pyra::PYRA {
            let row = niche::measured_row(pyra::ROW, niche::CANDIDATE_ROWS)?;
            let flight = SwapReturn::start(facts, PyraBeat {
                frame: inputs.frame,
                fighter,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_jump: niche_start_enabled(cpu, Move::Jump, true),
                can_swap: inputs.special.is_some() && niche_start_enabled(cpu, Move::Swap, true),
                can_ray: false,
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: near,
                measured: Some(MeasuredFlight::Swap(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == mario::KIND {
            let row = niche::measured_row(mario::ROW, niche::CANDIDATE_ROWS)?;
            let flight = CapeStall::start(facts, inputs.zone, inputs.draw, CapeBeat {
                frame: inputs.frame,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_cape: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: near,
                measured: Some(MeasuredFlight::Cape(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == steve::KIND {
            let row = niche::measured_row(steve::ROW, niche::CANDIDATE_ROWS)?;
            let flight = ElytraFlight::start(facts, ElytraBeat {
                frame: inputs.frame,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Glide, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Elytra(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == incineroar::KIND {
            let row = niche::measured_row(incineroar::ROW, niche::CANDIDATE_ROWS)?;
            let flight = WhipReturn::start(facts, inputs.draw, WhipBeat {
                frame: inputs.frame,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_whip: niche_start_enabled(cpu, Move::Side, true),
                can_jump: false,
                can_chop: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: near,
                measured: Some(MeasuredFlight::Whip(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
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
                measured: Some(MeasuredFlight::SkullBash(SkullBashFlight::new(
                    path, outline, facts.blast[3],
                ).with_measured_thunder(facts.ledges, facts.position, inputs.speed, natives::lr(cpu)))),
                body_clearance: facts.body_clearance,
            };
            return Some((guide, 1.0));
        }
        if fighter == 0xc {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED || natives::status_kind(cpu) != fall || natives::lr(cpu) != direction {
                return None;
            }
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
                measured: Some(MeasuredFlight::Puff(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if matches!(fighter, 0xd | 0xe) {
            let row = niche::measured_row(if fighter == 0xd { 24 } else { 25 }, niche::CANDIDATE_ROWS)?;
            let flight = FloatFlight::start(facts, FloatBeat {
                frame: 0,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_float: peach_float_unspent(cpu),
                can_jump: niche_start_enabled(cpu, Move::Jump, true),
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Parasol(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == rob::KIND {
            band?;
            let row = niche::measured_row(rob::ROW, niche::CANDIDATE_ROWS)?;
            let flight = BurnerFlight::start(facts, BurnerState {
                frame: 0,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                full_fuel: full_burner(cpu),
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_start: inputs.special.is_some() && niche_start_enabled(cpu, Move::Hover, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Burner(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if (0x1d..=0x1f).contains(&fighter) {
            let row = niche::measured_row((fighter - 0x1d + 18) as u8, niche::CANDIDATE_ROWS)?;
            let flight = WingsFlight::start(facts, wings::Sample {
                entry: wings::Entry {
                    frame: 0,
                    position: facts.position,
                    velocity: inputs.speed,
                    remaining: inputs.jumps,
                    facing: natives::lr(cpu),
                    can_special: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
                },
                status: natives::status_kind(cpu),
                can_jump: false,
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Wings(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == 0x10 {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED || natives::status_kind(cpu) != fall || natives::lr(cpu) != direction {
                return None;
            }
            let row = niche::measured_row(12, niche::CANDIDATE_ROWS)?;
            let flight = SheikFlight::start(facts, SheikBeat {
                frame: 0,
                status: fall,
                position: facts.position,
                velocity: inputs.speed,
                jumps: inputs.jumps,
                can_jump: false,
                down: niche_start_enabled(cpu, Move::Down, true),
                up: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Sheik(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == greninja::KIND {
            band?;
            let row = niche::measured_row(greninja::ROW, niche::CANDIDATE_ROWS)?;
            let flight = HydroFlight::start(facts, HydroSample {
                frame: inputs.frame,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_jump: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Travel, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Hydro(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == 0x13 {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED || natives::status_kind(cpu) != fall || natives::lr(cpu) != direction {
                return None;
            }
            let row = niche::measured_row(16, niche::CANDIDATE_ROWS)?;
            let flight = PichuFlight::start(facts, PichuBeat {
                frame: 0,
                status: fall,
                position: facts.position,
                velocity: inputs.speed,
                jumps: inputs.jumps,
                can_jump: false,
                side: niche_start_enabled(cpu, Move::Side, true),
                up: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Pichu(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == sora_flight::KIND {
            band?;
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED || natives::status_kind(cpu) != fall || natives::lr(cpu) != direction {
                return None;
            }
            let row = niche::measured_row(8, niche::CANDIDATE_ROWS)?;
            let flight = SoraFlight::start(facts, SoraBeat {
                frame: 0,
                status: fall,
                position: facts.position,
                velocity: inputs.speed,
                player_x: facts.player[0],
                jumps: inputs.jumps,
                can_jump: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::SoraChain, true),
                can_blade: niche_start_enabled(cpu, Move::Side, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Sora(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == olimar::KIND {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED || natives::status_kind(cpu) != fall {
                return None;
            }
            let row = niche::measured_row(30, niche::CANDIDATE_ROWS)?;
            let flight = OlimarFlight::start(facts, OlimarBeat {
                frame: 0,
                status: fall,
                position: facts.position,
                velocity: inputs.speed,
                jumps: inputs.jumps,
                pikmin: super::super::state_restore::olimar_pikmin_count(cpu),
                can_jump: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
                facing: natives::lr(cpu),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Olimar(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == perfect::kart::KIND {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED
                || natives::status_kind(cpu) != fall
                || inputs.jumps != 1
                || inputs.special.is_none()
                || !niche_start_enabled(cpu, Move::Side, true)
                || !niche_start_enabled(cpu, Move::Rise, true)
            {
                return None;
            }
            let row = niche::measured_row(31, niche::CANDIDATE_ROWS)?;
            let flight = KartFlight::start(facts, inputs.speed)?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Kart(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == diddy::KIND {
            let fall = konst(C::StatusFall);
            if fall != diddy::FALL || natives::status_kind(cpu) != fall {
                return None;
            }
            let row = niche::measured_row(diddy::ROW, niche::CANDIDATE_ROWS)?;
            let flight = BarrelFlight::start(facts, BarrelSample {
                frame: 0,
                status: fall,
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Barrel, true),
            })?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Diddy(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == pacman::KIND {
            let fall = konst(C::StatusFall);
            if fall == UNRESOLVED
                || natives::status_kind(cpu) != fall
                || inputs.jumps != 1
                || inputs.special.is_none()
                || !niche_start_enabled(cpu, Move::Side, true)
                || !niche_start_enabled(cpu, Move::Rise, true)
            {
                return None;
            }
            let row = niche::measured_row(21, niche::CANDIDATE_ROWS)?;
            let flight = PacFlight::start(facts, inputs.speed, natives::lr(cpu))?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::PacMan(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == lucario::KIND {
            let flight = AuraFlight::start(facts, AuraSample {
                frame: inputs.frame,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                damage: natives::damage(cpu, 0),
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_jump: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            })?;
            let row = niche::measured_row(lucario::ROW, niche::CANDIDATE_ROWS)?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Aura(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == snake::KIND {
            let sample = CypherSample {
                frame: 0,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                can_jump: false,
                can_rise: inputs.special.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            };
            let flight = CypherFlight::start(facts, sample)?;
            let row = niche::measured_row(snake::ROW, niche::CANDIDATE_ROWS)?;
            return Some((Self {
                sequence: Sequence::new(row),
                fighter,
                ledge: flight.exit_point(),
                measured: Some(MeasuredFlight::Cypher(flight)),
                body_clearance: facts.body_clearance,
            }, 1.0));
        }
        if fighter == link::KIND && inputs.special.is_some() {
            let sample = LinkBeat {
                frame: 0,
                status: natives::status_kind(cpu),
                position: facts.position,
                velocity: inputs.speed,
                damage: natives::damage(cpu, 0),
                facing: natives::lr(cpu),
                jumps: inputs.jumps,
                bomb: owned_link_bomb(cpu).map(|(owned, _)| owned),
                can_pull: niche_start_enabled(cpu, Move::Bomb, true),
                can_jump: false,
                can_spin: niche_start_enabled(cpu, Move::Rise, true),
            };
            if let Some(flight) = LinkFlight::start(facts, sample) {
                let row = niche::measured_row(link::ROW, niche::CANDIDATE_ROWS)?;
                return Some((Self {
                    sequence: Sequence::new(row),
                    fighter,
                    ledge: flight.exit_point(),
                    measured: Some(MeasuredFlight::Link(flight)),
                    body_clearance: facts.body_clearance,
                }, 1.0));
            }
        }
        let row_facts = Facts {
            fighter,
            corridor: None,
            covered_ledge: false,
            player_over_crossing: false,
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
            budget: None,
        };
        let row = niche::select(row_facts, niche::EXTENSION_ROWS)?;
        let margin = row.margin(row_facts)?;
        let guide = Self {
            sequence: Sequence::new(row),
            fighter,
            ledge: near,
            measured: None,
            body_clearance: facts.body_clearance,
        };
        Some((guide, margin))
    }

    /// The target of a measured flight.
    pub(super) fn exit_point(&self) -> Point {
        self.ledge
    }

    pub(super) fn crossing_frames(&self) -> Option<u16> {
        match &self.measured {
            // Measured from the Skull Bash press to the exit: 13+39+46+27+33.
            Some(MeasuredFlight::SkullBash(_)) => Some(158),
            // The FLY-L ring bake runs in at most 283 frames at the cell centers.
            Some(MeasuredFlight::Puff(_)) => Some(300),
            Some(MeasuredFlight::Parasol(_)) => Some(400),
            Some(MeasuredFlight::Burner(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Wings(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Pichu(_)) => Some(200),
            Some(MeasuredFlight::Olimar(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Sheik(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Sora(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Kart(flight)) => Some(flight.crossing_frames()),
            // 72 to the blast, 88 to Spin Attack, and up to 37 to catch; allow for the neutral wait.
            Some(MeasuredFlight::Link(_)) => Some(235),
            Some(MeasuredFlight::Cypher(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Aura(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Hydro(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Diddy(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::PacMan(flight)) => Some(flight.crossing_frames()),
            Some(MeasuredFlight::Elytra(flight)) => Some(flight.crossing_frames()),
            _ => None,
        }
    }

    pub(super) fn fake_out(mut self, position: Point, velocity: Point) -> Option<Self> {
        let (row, ledge, flight) = match self.measured.take()? {
            MeasuredFlight::SkullBash(flight) => {
                let flight = flight.into_fake_out();
                (41, flight.near_ledge(), MeasuredFlight::FakeOut(flight))
            }
            MeasuredFlight::Pichu(flight) => {
                let flight = flight.into_fake_out(position, velocity)?;
                (43, flight.exit_point(), MeasuredFlight::PichuFakeOut(flight))
            }
            _ => return None,
        };
        self.ledge = ledge;
        self.sequence = Sequence::new(niche::measured_row(row, niche::CANDIDATE_ROWS)?);
        self.measured = Some(flight);
        Some(self)
    }

    pub(super) fn using_bomb(&self) -> bool {
        self.sequence.action() == Some(Move::Bomb)
    }

    pub(super) fn accepts_form(&self, fighter: Option<i32>) -> bool {
        self.sequence.accepts_form(fighter)
    }

    pub(super) fn waiting_for_form(&self, frame: u32) -> bool {
        match &self.measured {
            Some(MeasuredFlight::Swap(flight)) => flight.waiting_for_mythra(frame),
            _ => self.accepts_form(None),
        }
    }

    pub(super) fn follow_measured_form(&mut self, frame: u32, fighter: i32, status: i32) -> bool {
        match &mut self.measured {
            Some(MeasuredFlight::Swap(flight)) => flight.follow(frame, fighter, status),
            _ => false,
        }
    }

    pub(super) fn correction_row(&self) -> u8 {
        if matches!(&self.measured, Some(MeasuredFlight::SkullBash(flight))
            if flight.thunder.as_ref().is_some_and(pikachu::thunder::Thunder::active))
        {
            pikachu::thunder::ROW
        } else {
            self.selected_niche_row().id
        }
    }

    pub(super) fn selected_niche_row(&self) -> &'static Row {
        self.sequence.row()
    }

    pub(super) unsafe fn correct_niche_pad(
        &mut self,
        ai: usize,
        cpu: usize,
        p1: usize,
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
        if let Some(MeasuredFlight::Swap(flight)) = &mut self.measured {
            let fighter = natives::fighter_kind(cpu)?;
            let needs_jump = flight.needs_jump(frame);
            return flight.step(PyraBeat {
                frame,
                fighter,
                status,
                position: [x, y],
                velocity: [0.0; 2],
                facing: 0.0,
                jumps: if needs_jump { remaining_jumps(cpu)? } else { 0 },
                can_jump: needs_jump && niche_start_enabled(cpu, Move::Jump, true),
                can_swap: flight.needs_swap(frame) && niche_start_enabled(cpu, Move::Swap, true),
                can_ray: flight.needs_ray(frame) && reach.is_some()
                    && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::Elytra(flight)) = &mut self.measured {
            let waiting = flight.needs_enabled_special();
            let ((vx, vy), facing) = if waiting {
                (super::super::recording_host::fighter_speed(cpu)?, natives::lr(cpu))
            } else {
                ((0.0, 0.0), 0.0)
            };
            return flight.step(ElytraBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing,
                can_rise: waiting && reach.is_some() && niche_start_enabled(cpu, Move::Glide, available),
            });
        }
        if let Some(MeasuredFlight::Whip(flight)) = &mut self.measured {
            let jump = flight.jump_frame(frame);
            let chop = flight.chop_frame(frame);
            let (vx, vy) = if chop {
                super::super::recording_host::fighter_speed(cpu)?
            } else {
                (0.0, 0.0)
            };
            return flight.step(WhipBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing: if jump || chop { natives::lr(cpu) } else { 0.0 },
                jumps: if jump || chop { remaining_jumps(cpu)? } else { 0 },
                can_whip: flight.whip_frame(frame) && niche_start_enabled(cpu, Move::Side, true),
                can_jump: jump && niche_start_enabled(cpu, Move::Jump, true),
                can_chop: chop && reach.is_some() && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::Hydro(flight)) = &mut self.measured {
            let jump = flight.jump_frame(frame);
            let rise = flight.rise_frame(frame);
            return flight.step(HydroSample {
                frame,
                status,
                position: [x, y],
                velocity: [0.0; 2],
                facing: if jump || rise { natives::lr(cpu) } else { 0.0 },
                jumps: remaining_jumps(cpu)?,
                can_jump: jump && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: rise && reach.is_some() && niche_start_enabled(cpu, Move::Travel, available),
            });
        }
        if let Some(MeasuredFlight::Cape(flight)) = &mut self.measured {
            let free = flight.awaiting_press();
            let ((vx, vy), jumps, facing) = if free {
                (super::super::recording_host::fighter_speed(cpu)?, remaining_jumps(cpu)?, natives::lr(cpu))
            } else {
                ((0.0, 0.0), 0, 0.0)
            };
            return flight.step(CapeBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing,
                jumps,
                can_cape: free && niche_start_enabled(cpu, Move::Side, true),
                can_rise: free && reach.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            });
        }
        if let Some(MeasuredFlight::Link(flight)) = &mut self.measured {
            let sample_bomb = flight.needs_bomb_sample(frame);
            let velocity = if flight.waiting_to_pull() {
                let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
                [vx, vy]
            } else {
                [0.0; 2]
            };
            return flight.step(LinkBeat {
                frame,
                status,
                position: [x, y],
                velocity,
                damage: natives::damage(cpu, 0),
                facing: natives::lr(cpu),
                jumps: remaining_jumps(cpu)?,
                bomb: if sample_bomb { owned_link_bomb(cpu).map(|(owned, _)| owned) } else { None },
                can_pull: flight.waiting_to_pull() && niche_start_enabled(cpu, Move::Bomb, true),
                can_jump: flight.is_jump_frame(frame) && niche_start_enabled(cpu, Move::Jump, true),
                can_spin: flight.needs_spin_sample(frame) && reach.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            });
        }
        if let Some(MeasuredFlight::Aura(flight)) = &mut self.measured {
            let free = matches!(status, lucario::FALL | lucario::AERIAL_JUMP);
            return flight.step(AuraSample {
                frame,
                status,
                position: [x, y],
                velocity: [0.0; 2],
                damage: natives::damage(cpu, 0),
                facing: if free { natives::lr(cpu) } else { 0.0 },
                jumps: if free { remaining_jumps(cpu)? } else { 0 },
                can_jump: flight.jump_frame(frame) && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: flight.rise_frame(frame) && reach.is_some()
                    && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::Cypher(flight)) = &mut self.measured {
            let free = matches!(status, snake::FALL | snake::JUMP_AERIAL);
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            return flight.step(CypherSample {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing: natives::lr(cpu),
                jumps: if free { remaining_jumps(cpu)? } else { 0 },
                can_jump: status == snake::FALL && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: status == snake::JUMP_AERIAL && reach.is_some()
                    && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
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
        if let Some(MeasuredFlight::Parasol(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let airborne = airborne_recovery(cpu, status);
            return flight.step(FloatBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing: natives::lr(cpu),
                jumps: remaining_jumps(cpu)?,
                can_float: flight.needs_float() && peach_float_unspent(cpu),
                can_jump: airborne && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: airborne && reach.is_some() && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::Burner(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let takeoff = flight.needs_fuel_sample();
            return flight.step(BurnerState {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                full_fuel: takeoff && full_burner(cpu),
                facing: if takeoff { natives::lr(cpu) } else { 0.0 },
                jumps: if takeoff { remaining_jumps(cpu)? } else { 0 },
                can_start: takeoff && reach.is_some() && niche_start_enabled(cpu, Move::Hover, available),
            });
        }
        if let Some(MeasuredFlight::Wings(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let free = matches!(status, 0xd..=0xf);
            return flight.step(wings::Sample {
                entry: wings::Entry {
                    frame,
                    position: [x, y],
                    velocity: [vx, vy],
                    remaining: if free { remaining_jumps(cpu)? } else { 0 },
                    facing: natives::lr(cpu),
                    can_special: free && reach.is_some() && niche_start_enabled(cpu, Move::Rise, available),
                },
                status,
                can_jump: free && niche_start_enabled(cpu, Move::Jump, true),
            });
        }
        if let Some(MeasuredFlight::Sora(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let free = matches!(status, sora_flight::FALL | sora_flight::FALL_AERIAL);
            let player_x = if status == sora_flight::FALL { natives::pos_x(p1) } else { 0.0 };
            return flight.step(SoraBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                player_x,
                jumps: if free { remaining_jumps(cpu)? } else { 0 },
                can_jump: status == sora_flight::FALL && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: free && reach.is_some() && niche_start_enabled(cpu, Move::SoraChain, available),
                can_blade: free && niche_start_enabled(cpu, Move::Side, true),
            });
        }
        if let Some(MeasuredFlight::Olimar(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let rise_status = [olimar::FALL, olimar::JUMP_AERIAL, olimar::FALL_AERIAL].contains(&status);
            return flight.step(OlimarBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                pikmin: super::super::state_restore::olimar_pikmin_count(cpu),
                can_jump: status == fall && niche_start_enabled(cpu, Move::Jump, true),
                can_rise: rise_status && reach.is_some() && niche_start_enabled(cpu, Move::Rise, available),
                facing: natives::lr(cpu),
            });
        }
        if let Some(MeasuredFlight::Sheik(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let aerial = konst(C::StatusJumpAerial);
            return flight.step(SheikBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                can_jump: status == perfect::sheik::FISH_END
                    && niche_start_enabled(cpu, Move::Jump, true),
                down: status == fall && niche_start_enabled(cpu, Move::Down, true),
                up: (status == fall || status == aerial) && reach.is_some()
                    && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::Pichu(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let aerial = konst(C::StatusJumpAerial);
            return flight.step(PichuBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                can_jump: status == perfect::pichu::BASH_END
                    && niche_start_enabled(cpu, Move::Jump, true),
                side: status == fall && niche_start_enabled(cpu, Move::Side, true),
                up: status == aerial && reach.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            });
        }
        if let Some(MeasuredFlight::PichuFakeOut(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let aerial = konst(C::StatusJumpAerial);
            return flight.step(PichuBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                can_jump: status == fall && niche_start_enabled(cpu, Move::Jump, true),
                side: false,
                up: status == aerial && reach.is_some() && niche_start_enabled(cpu, Move::Rise, true),
            });
        }
        if let Some(MeasuredFlight::Kart(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let aerial = konst(C::StatusJumpAerial);
            let side = status == fall && niche_start_enabled(cpu, Move::Side, available);
            let up = reach.is_some() && (status == fall || status == aerial)
                && niche_start_enabled(cpu, Move::Rise, available);
            return flight.step(KartState {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                side,
                up,
            });
        }
        if let Some(MeasuredFlight::Diddy(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            return flight.step(BarrelSample {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                facing: natives::lr(cpu),
                can_rise: status == diddy::FALL && reach.is_some()
                    && niche_start_enabled(cpu, Move::Barrel, available),
            });
        }
        if let Some(MeasuredFlight::PacMan(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let fall = konst(C::StatusFall);
            let aerial = konst(C::StatusJumpAerial);
            return flight.step(PacBeat {
                frame,
                status,
                position: [x, y],
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                side: status == fall && niche_start_enabled(cpu, Move::Side, available),
                up: status == aerial && reach.is_some() && niche_start_enabled(cpu, Move::Rise, available),
            });
        }
        if let Some(MeasuredFlight::FakeOut(flight)) = &mut self.measured {
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let position = [x, y];
            let motion = niche_motion_phase(cpu, self.fighter, flight.movement(), status);
            let can_start = flight.start_check(position, vy).is_some_and(|movement| {
                matches!(motion, Motion::Free | Motion::Travel)
                    && niche_start_enabled(cpu, movement, available)
            });
            return flight.step(FakeOutSample {
                frame,
                motion,
                position,
                velocity: [vx, vy],
                jumps: remaining_jumps(cpu)?,
                can_start,
            });
        }
        if let Some(MeasuredFlight::SkullBash(skull_bash)) = &mut self.measured {
            use pikachu::thunder;
            let (vx, vy) = super::super::recording_host::fighter_speed(cpu)?;
            let position = [x, y];
            let velocity = [vx, vy];
            let jumps = remaining_jumps(cpu)?;
            if let Some(flight) = &mut skull_bash.thunder {
                if flight.active() {
                    let can_down = flight.cast_due(frame) && niche_start_enabled(cpu, Move::Down, true);
                    return flight.step(frame, status, can_down, reach.is_some());
                }
                if flight.jump_due(frame) {
                    let ground = konst(C::SituationGround);
                    let wait = konst(C::StatusWait);
                    let guarder = (ground != UNRESOLVED && wait != UNRESOLVED
                        && natives::situation_kind(p1) == ground && natives::status_kind(p1) == wait)
                        .then(|| [natives::pos_x(p1), natives::pos_y(p1)]);
                    if let Some(pad) = flight.begin(thunder::Jump {
                        frame, status, position, velocity, jumps, guarder, up_available: reach.is_some(),
                    }) {
                        return Some(pad);
                    }
                }
            }
            let motion = niche_motion_phase(cpu, self.fighter, movement, status);
            let pad = skull_bash.step(&mut self.sequence, Sample {
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
            }, position, vy, jumps)?;
            if movement == Move::Side && motion == Motion::Free && pad.press == perfect::SPECIAL {
                skull_bash.observe_bash_pad(thunder::Bash { frame, position, velocity });
            }
            return Some(pad);
        }

        let direction = (self.ledge[0] - x).signum();
        let mut aim = normalized_stick([self.ledge[0] - x, self.ledge[1] - y]);
        let native_phase = (movement != Move::Bomb)
            .then(|| niche_motion_phase(cpu, self.fighter, movement, status));
        let (phase, actionable) = match native_phase {
            Some(phase) => (phase, phase == Motion::Free),
            None => link_bomb_phase(cpu, status, [x, y], direction, self.body_clearance)?,
        };
        if matches!(movement, Move::Bolt) && phase == Motion::Charging {
            aim = bolt_input(cpu, self.fighter, [x, y], direction, aim)?;
        }
        if matches!(movement, Move::Rise | Move::SoraChain)
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
            at_exit: true,
            can_enter: true,
            low: false,
            hold_over: true,
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

pub(super) unsafe fn niche_start_enabled(cpu: usize, movement: Move, available: bool) -> bool {
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
        | (0x1e | 0x1f, Travel, 0x1e7)
        | (0x22, Hover, 0x1f4)
        | (0x27, Side, 0x1e5 | 0x1ee)
        | (0x27, Barrel, 0x1f2)
        | (0x2b, Hover, 0x1e5)
        | (0x2c, Travel, 0x1e6)
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
    if !link_bomb_reads_granted() || !crate::runtime::roster::running() || status == UNRESOLVED {
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
    let (ownership, bomb) = owned_link_bomb(cpu)?;
    match ownership {
        OwnedBomb::Absent => return Some((Motion::Free, true)),
        OwnedBomb::Held => return Some((Motion::ItemHeld, false)),
        OwnedBomb::Loose => {}
    }
    let bomb = bomb?;
    let dx = (x - natives::pos_x(bomb)) * inward;
    let dy = y - natives::pos_y(bomb);
    // Let the dropped bomb fall behind Link so the launch bends toward the stage.
    (dx.hypot(dy) < clearance).then_some((Motion::ItemLoose, dx > 0.0 && dy >= 0.0))
}

unsafe fn owned_link_bomb(cpu: usize) -> Option<(OwnedBomb, Option<usize>)> {
    use smash::app::{lua_bind::ItemModule, BattleObjectModuleAccessor};

    if !link_bomb_reads_granted() || !crate::runtime::roster::running() {
        return None;
    }
    let body = cpu as *mut BattleObjectModuleAccessor;
    let held = ItemModule::is_have_item(body, 0).then(|| ItemModule::get_have_item_id(body, 0));
    // FIGHTER_LINK_INSTANCE_WORK_ID_INT_BOMB_OBJECT_ID.
    let id = u32::try_from(natives::get_int(cpu, 0x1000_00c3)).ok()?;
    if id == 0x5000_0000 || !state::object_is_active(id) {
        return held.is_none().then_some((OwnedBomb::Absent, None));
    }
    let bomb = state::object_accessor(id);
    if bomb == 0 || natives::fighter_kind(bomb) != Some(0x78) {
        return None;
    }
    if let Some(held) = held {
        return (held == u64::from(id)).then_some((OwnedBomb::Held, Some(bomb)));
    }
    Some((OwnedBomb::Loose, Some(bomb)))
}

pub(super) fn pyra_swapping(status: i32) -> bool {
    status != UNRESOLVED
        && (status == konst(C::StatusSpecialLw) || matches!(status, 0x1e1 | 0x1e2))
}

unsafe fn peach_float_unspent(cpu: usize) -> bool {
    // Peach and Daisy set UNIQ_FLOAT as fuwafuwa_start begins. The native
    // float decision admits the action only while the same flag is clear.
    game::granted(sites_pkg2::WORK_IS_FLAG_OFFSET) && !natives::is_flag(cpu, 0x2000_00e1)
}

unsafe fn full_burner(cpu: usize) -> bool {
    // FIGHTER_ROBOT_INSTANCE_WORK_ID_FLOAT_BURNER_ENERGY_VALUE
    game::granted(sites_pkg2::WORK_GET_FLOAT_OFFSET)
        && rob::full_fuel(natives::get_float(cpu, 0x4e))
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
