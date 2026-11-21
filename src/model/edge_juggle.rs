//! Shared attempt clock and scoring for Guard the Edge and Juggler (DESIGN v2).
use super::practice_games::{record_max, record_time, result_tick, Game};

#[path = "edge_juggle/perfect.rs"]
pub mod perfect;

const EDGE_TIMEOUT: u32 = 15 * 60;
const RESULT_FRAMES: u16 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drill {
    GuardEdge,
    Juggler,
}

impl Drill {
    pub const fn from_game(game: Game) -> Option<Self> {
        match game {
            Game::GuardEdge => Some(Self::GuardEdge),
            Game::Juggler => Some(Self::Juggler),
            _ => None,
        }
    }

    pub const fn game(self) -> Game {
        match self {
            Self::GuardEdge => Game::GuardEdge,
            Self::Juggler => Game::Juggler,
        }
    }

    /// The Edge recovery start is already airborne, so only aerial jumps are restored.
    pub const fn placed_jump_count(self) -> i32 {
        match self {
            Self::GuardEdge => 1,
            Self::Juggler => 0,
        }
    }

    pub const fn ready_frames(self) -> u16 {
        match self {
            Self::GuardEdge => 60,
            Self::Juggler => 45,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    LevelNine,
    Perfect,
}

impl Difficulty {
    pub const fn from_option(option: usize) -> Self {
        if option == 1 {
            Self::Perfect
        } else {
            Self::LevelNine
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    FramePerfect,
    Human,
    Custom(u8),
}

impl Reaction {
    /// The custom row stores a zero-based index for 1–30 frames.
    pub fn from_settings(option: usize, frames_index: usize) -> Self {
        match option {
            0 => Self::FramePerfect,
            2 => Self::Custom((frames_index.min(29) + 1) as u8),
            _ => Self::Human,
        }
    }

    pub const fn frames(self) -> u8 {
        match self {
            Self::FramePerfect => 0,
            Self::Human => 12,
            Self::Custom(frames) => frames,
        }
    }
}

pub struct DifficultyRows {
    pub difficulty: &'static str,
    pub reaction: &'static str,
    pub frames: &'static str,
}

pub const EDGE_ROWS: DifficultyRows = DifficultyRows {
    difficulty: "edge_difficulty",
    reaction: "edge_reaction",
    frames: "edge_reaction_frames",
};
pub const JUGGLE_ROWS: DifficultyRows = DifficultyRows {
    difficulty: "juggle_difficulty",
    reaction: "juggle_reaction",
    frames: "juggle_reaction_frames",
};

impl Drill {
    pub const fn difficulty_rows(self) -> &'static DifficultyRows {
        match self {
            Self::GuardEdge => &EDGE_ROWS,
            Self::Juggler => &JUGGLE_ROWS,
        }
    }
}

pub fn reaction_rows(id: &str) -> Option<&'static DifficultyRows> {
    [&EDGE_ROWS, &JUGGLE_ROWS]
        .into_iter()
        .find(|rows| rows.reaction == id)
}

/// Training's pause-menu CPU selection. The native apply maps its menu value to a controller type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuBehavior {
    pub menu: u32,
    pub level: u32,
}

impl CpuBehavior {
    pub const LEVEL_NINE: Self = Self { menu: 2, level: 8 };

    pub fn control_type(self) -> Option<u8> {
        [1, 1, 0, 2, 5, 3, 0x19, 0x1a, 0x24].get(self.menu as usize).copied()
    }

    pub fn applied(self, entry: u8, ai: u8, operation_cpu: bool) -> bool {
        self.control_type().is_some_and(|control| {
            control == entry && control == ai && operation_cpu == (control != 0x24)
        })
    }
}

/// Save the selection once. Retain it through Stop until the game applies the restored selection.
#[derive(Default)]
pub struct BehaviorLease(Option<(usize, CpuBehavior)>);

impl BehaviorLease {
    pub const fn new() -> Self {
        Self(None)
    }

    pub fn held(&self) -> bool {
        self.0.is_some()
    }

    pub fn step(
        &mut self,
        running: bool,
        panel: usize,
        current: CpuBehavior,
        applied: bool,
    ) -> Option<CpuBehavior> {
        if self.0.is_some_and(|(owner, _)| owner != panel) {
            self.0 = None;
        }
        let desired = if running {
            self.0.get_or_insert((panel, current));
            CpuBehavior::LEVEL_NINE
        } else {
            self.0?.1
        };
        if desired != current || !applied {
            return Some(desired);
        }
        if !running {
            self.0 = None;
        }
        None
    }
}

/// The guarded Training HUD state used by the stock pause input handler.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BehaviorMenu {
    Closed,
    Transition,
    Open,
}

impl BehaviorMenu {
    pub fn from_stock(state: u32, pause_owner: bool) -> Self {
        match state {
            2 if !pause_owner => Self::Closed,
            5 => Self::Open,
            _ => Self::Transition,
        }
    }
}

/// Drive the native pause/resume once when a behavior requires the stock apply.
/// A pause opened by the player is never closed by the drill.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BehaviorPause {
    Idle,
    Opening,
    Closing,
}

impl BehaviorPause {
    pub fn pulse(&mut self, needs_stock: bool, menu: BehaviorMenu, quiet: bool) -> bool {
        match *self {
            Self::Idle if needs_stock && menu == BehaviorMenu::Closed && quiet => {
                *self = Self::Opening;
                true
            }
            Self::Opening if menu == BehaviorMenu::Open && quiet => {
                *self = Self::Closing;
                true
            }
            Self::Closing if menu == BehaviorMenu::Closed => {
                *self = Self::Idle;
                false
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Secured,
    RecoveredAtLedge,
    RecoveredOnStage,
    Unresolved,
    Landed,
    GrabbedLedge,
    KnockedOut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub dead_or_rebirthing: bool,
    pub grounded: bool,
    pub cliff_catch: bool,
    pub hit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Place,
    Ready(u16),
    Live,
    Result(Outcome, u16),
    Done,
}

impl Phase {
    /// Keep stock stick and buttons neutral until the attempt releases the CPU.
    pub fn held_pad(self, buttons: u32) -> Option<perfect::Correction> {
        matches!(self, Self::Place | Self::Ready(_)).then_some(perfect::Correction {
            stick: Some([0.0, 0.0]),
            release: buttons & !perfect::FLICK_JUMP,
            ..perfect::Correction::default()
        })
    }
}

/// Which live pad owner runs at the AI epilogue.
pub const fn drill_owns_pad(owns_primary: bool, recorder_active: bool) -> bool {
    owns_primary && !recorder_active
}

/// CPU settings affect both registered bodies of the opposed entry.
pub const fn suspends_cpu_settings(active: bool, key: crate::model::cpu::role::FighterKey) -> bool {
    active && key.entry == 1
}

/// Runtime admits placement, then ticks only on eligible simulation frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    drill: Drill,
    limit: Option<u16>,
    pub phase: Phase,
    pub attempts: u16,
    pub secured: u16,
    pub streak: u16,
    pub best_streak: u16,
    pub frames: u32,
    pub hits: u16,
}

impl Session {
    pub const fn new(drill: Drill, limit: Option<u16>) -> Self {
        Self {
            drill,
            limit,
            phase: Phase::Place,
            attempts: 0,
            secured: 0,
            streak: 0,
            best_streak: 0,
            frames: 0,
            hits: 0,
        }
    }

    pub fn restart_drill(&mut self) {
        *self = Self::new(self.drill, self.limit);
    }

    pub fn discard_attempt(&mut self) {
        if matches!(self.phase, Phase::Ready(_) | Phase::Live) {
            self.phase = Phase::Place;
        }
    }

    pub fn placed(&mut self) {
        if self.phase == Phase::Place {
            self.frames = 0;
            self.hits = 0;
            self.phase = Phase::Ready(self.drill.ready_frames());
        }
    }

    /// The transition to Live is the release frame (time zero).
    pub fn advance_attempt(&mut self, observed: Observation) {
        match self.phase {
            Phase::Ready(mut left) => {
                self.phase = if result_tick(&mut left, true) {
                    Phase::Live
                } else {
                    Phase::Ready(left)
                };
            }
            Phase::Live => {
                self.frames = self.frames.saturating_add(1);
                self.hits = self.hits.saturating_add(u16::from(observed.hit));
                let outcome = match (self.drill, observed) {
                    (
                        Drill::GuardEdge,
                        Observation {
                            dead_or_rebirthing: true,
                            ..
                        },
                    ) => Outcome::Secured,
                    (
                        Drill::GuardEdge,
                        Observation {
                            cliff_catch: true, ..
                        },
                    ) => Outcome::RecoveredAtLedge,
                    (Drill::GuardEdge, Observation { grounded: true, .. }) => {
                        Outcome::RecoveredOnStage
                    }
                    (Drill::GuardEdge, _) if self.frames == EDGE_TIMEOUT => Outcome::Unresolved,
                    (
                        Drill::Juggler,
                        Observation {
                            dead_or_rebirthing: true,
                            ..
                        },
                    ) => Outcome::KnockedOut,
                    (
                        Drill::Juggler,
                        Observation {
                            cliff_catch: true, ..
                        },
                    ) => Outcome::GrabbedLedge,
                    (Drill::Juggler, Observation { grounded: true, .. }) => Outcome::Landed,
                    _ => return,
                };
                if outcome != Outcome::Unresolved {
                    self.attempts = self.attempts.saturating_add(1);
                    if outcome == Outcome::Secured {
                        self.secured = self.secured.saturating_add(1);
                        self.streak = self.streak.saturating_add(1);
                        self.best_streak = self.best_streak.max(self.streak);
                    } else {
                        self.streak = 0;
                    }
                }
                self.phase = Phase::Result(outcome, RESULT_FRAMES);
            }
            Phase::Result(outcome, mut left) => {
                self.phase = if !result_tick(&mut left, true) || observed.dead_or_rebirthing {
                    Phase::Result(outcome, left)
                } else if self.limit.is_some_and(|limit| self.attempts >= limit) {
                    Phase::Done
                } else {
                    Phase::Place
                };
            }
            Phase::Done | Phase::Place => {}
        }
    }

    /// The scored window starts at release and ends at its first result.
    /// Return the attempt number and the stable event code for the trace.
    pub fn trace_event(self, before: Phase) -> Option<(u16, u8)> {
        let outcome = match (before, self.phase) {
            (Phase::Ready(_), Phase::Live) => return Some((self.attempts.saturating_add(1), 0)),
            (Phase::Live, Phase::Result(outcome, _)) => outcome,
            _ => return None,
        };
        let code = match outcome {
            Outcome::Secured => 1,
            Outcome::RecoveredAtLedge => 2,
            Outcome::RecoveredOnStage => 3,
            Outcome::Unresolved => return Some((self.attempts.saturating_add(1), 4)),
            Outcome::Landed => 5,
            Outcome::GrabbedLedge => 6,
            Outcome::KnockedOut => 7,
        };
        Some((self.attempts, code))
    }

    pub fn record_result<K: Copy + Eq, const N: usize>(
        &self,
        bests: &mut [Option<(K, u32)>; N],
        key: K,
    ) -> Option<u32> {
        match self.phase {
            Phase::Result(Outcome::Secured, RESULT_FRAMES) => {
                Some(record_time(bests, key, self.frames))
            }
            Phase::Result(
                Outcome::Landed | Outcome::GrabbedLedge | Outcome::KnockedOut,
                RESULT_FRAMES,
            ) => Some(record_max(bests, key, self.frames)),
            _ => None,
        }
    }
}

/// The stage is split at its centre and at each side's ledge height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Area {
    #[default]
    LowerLeft,
    LowerRight,
    UpperLeft,
    UpperRight,
}

impl Area {
    const ALL: [Self; 4] = [
        Self::LowerLeft,
        Self::LowerRight,
        Self::UpperLeft,
        Self::UpperRight,
    ];

    pub fn step(self, delta: i32) -> Self {
        Self::ALL[(self as i32 + delta).rem_euclid(4) as usize]
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::LowerLeft => "Lower left",
            Self::LowerRight => "Lower right",
            Self::UpperLeft => "Upper left",
            Self::UpperRight => "Upper right",
        }
    }
}

/// Coordinates come from the live main floor, blast bounds, and camera limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stage {
    blast: [f32; 4],
    camera: [f32; 4],
    pub ledges: [[f32; 2]; 2],
}

impl Stage {
    pub const fn blast(self) -> [f32; 4] {
        self.blast
    }

    pub fn new(blast: [f32; 4], camera: [f32; 4], ledges: [[f32; 2]; 2]) -> Option<Self> {
        let [left, right, top, bottom] = blast;
        let [[lx, ly], [rx, ry]] = ledges;
        (blast
            .into_iter()
            .chain(ledges.into_iter().flatten())
            .all(f32::is_finite)
            && clamp_point([lx, ly], camera).is_some()
            && left < lx
            && lx < rx
            && rx < right
            && [ly, ry].into_iter().all(|y| bottom < y && y < top))
        .then_some(Self {
            blast,
            camera,
            ledges,
        })
    }

    fn area_bounds(self, area: Area) -> [f32; 4] {
        let [left, right, top, bottom] = self.blast;
        let is_left = matches!(area, Area::LowerLeft | Area::UpperLeft);
        let [x, y] = self.ledges[usize::from(!is_left)];
        // Leave one world unit between the start and the ledge or blast boundary.
        let [lo, hi] = if is_left {
            [left + 1.0, x - 1.0]
        } else {
            [x + 1.0, right - 1.0]
        };
        if matches!(area, Area::LowerLeft | Area::LowerRight) {
            [lo, hi, y, bottom + 1.0]
        } else {
            [lo, hi, top - 1.0, y]
        }
    }

    pub fn edge_start(self, area: Area, point: [f32; 2]) -> Option<[f32; 2]> {
        clamp_point(point, self.area_bounds(area))
    }

    pub fn area_default(self, area: Area) -> Option<[f32; 2]> {
        let [lo, hi, top, bottom] = self.area_bounds(area);
        self.edge_start(area, [lo.midpoint(hi), bottom.midpoint(top)])
    }

    pub fn camera_point(self, point: [f32; 2]) -> Option<[f32; 2]> {
        clamp_point(point, self.camera)
    }

    pub fn player_cursor(self, drill: Drill, point: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = self.camera_point(point)?;
        let x = match drill {
            Drill::GuardEdge => x.clamp(self.ledges[0][0], self.ledges[1][0]),
            Drill::Juggler => x,
        };
        Some([x, y])
    }

    pub fn default_juggle(self, p1: [f32; 2]) -> [f32; 2] {
        [p1[0], p1[1].midpoint(self.camera[2])]
    }
}

fn clamp_point([x, y]: [f32; 2], [left, right, top, bottom]: [f32; 4]) -> Option<[f32; 2]> {
    (x.is_finite() && y.is_finite() && left <= right && bottom <= top)
        .then(|| [x.clamp(left, right), y.clamp(bottom, top)])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setup {
    pub area: Area,
    pub cpu: [f32; 2],
    pub p1: [f32; 2],
    pub percent: u16,
}

pub fn recovery_faces_away(
    drill: Drill,
    stage_key: (i32, u32),
    stage: Stage,
    setup: Setup,
    fighter: Option<i32>,
) -> bool {
    if drill != Drill::GuardEdge {
        return false;
    }
    let near = stage.ledges[0];
    match fighter {
        Some(perfect::link::KIND) => perfect::link::in_region(stage_key, near, setup.cpu, setup.percent.into()),
        Some(perfect::snake::KIND) => perfect::snake::in_region(stage_key, near, setup.cpu),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WizardStep {
    Area,
    Cpu,
    Percent,
    Player,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wizard {
    drill: Drill,
    area_offset: Option<[f32; 2]>,
    left_start: Option<(Area, [f32; 2])>,
    pub step: WizardStep,
    pub setup: Setup,
}

impl Wizard {
    pub fn new(
        drill: Drill, stage: Stage, p1: [f32; 2], percent: u16,
        stage_key: (i32, u32), fighter: Option<i32>,
    ) -> Option<Self> {
        let area = Area::default();
        let link_left_start = drill == Drill::GuardEdge
            && fighter == Some(perfect::link::KIND)
            && perfect::link::in_region(stage_key, stage.ledges[0], [stage.ledges[0][0] - 30.0, stage.ledges[0][1] + 18.0], percent.into());
        let lucario_left_start = if drill == Drill::GuardEdge && fighter == Some(perfect::lucario::KIND) {
            perfect::lucario::wizard_offset(stage_key, percent)
        } else {
            None
        };
        let left_start = if let Some(offset) = lucario_left_start {
            Some((Area::LowerLeft, offset))
        } else if link_left_start {
            Some((Area::UpperLeft, [30.0, 18.0]))
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::mario::KIND)
            && perfect::mario::reference_stage(stage_key)
        {
            Some((Area::LowerLeft, [34.0, -46.0]))
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::snake::KIND)
            && perfect::is_crossing_stage(stage_key)
        {
            Some((Area::LowerLeft, [20.0, -4.0]))
        } else {
            None
        };
        let area_offset = match (drill, fighter) {
            (Drill::GuardEdge, Some(0x8)) if perfect::is_crossing_stage(stage_key) => {
                Some([20.0, -55.0])
            }
            (Drill::GuardEdge, Some(0xc)) if stage_key == (311, 0) => {
                Some([40.0, 20.0])
            }
            (Drill::GuardEdge, Some(0xc)) if perfect::is_crossing_stage(stage_key) => {
                Some([20.0, -28.0])
            }
            (Drill::GuardEdge, Some(0xd | 0xe | perfect::rob::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([40.0, 20.0])
            }
            (Drill::GuardEdge, Some(0x1d..=0x1f)) if perfect::is_crossing_stage(stage_key) => {
                Some([30.0, 6.0])
            }
            (Drill::GuardEdge, Some(0x10)) if perfect::is_crossing_stage(stage_key) => {
                Some([18.0, -30.0])
            }
            (Drill::GuardEdge, Some(0x13)) if perfect::is_crossing_stage(stage_key) => {
                Some([20.0, -28.0])
            }
            (Drill::GuardEdge, Some(perfect::sora_flight::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([20.0, -28.0])
            }
            (Drill::GuardEdge, Some(perfect::olimar::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([26.0, -26.0])
            }
            (Drill::GuardEdge, Some(perfect::kart::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([22.0, -42.0])
            }
            (Drill::GuardEdge, Some(perfect::diddy::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([14.0, -30.0])
            }
            (Drill::GuardEdge, Some(perfect::pacman::KIND)) if perfect::is_crossing_stage(stage_key) => {
                Some([26.0, 14.0])
            }
            _ => None,
        };
        let (step, cpu) = match drill {
            Drill::GuardEdge => (WizardStep::Area, Self::area_release(stage, area, Self::release_offset(area, left_start, area_offset))?),
            Drill::Juggler => (WizardStep::Cpu, stage.default_juggle(p1)),
        };
        Some(Self {
            drill,
            area_offset,
            left_start,
            step,
            setup: Setup {
                area,
                cpu,
                p1: stage.player_cursor(drill, p1)?,
                percent: percent.min(300),
            },
        })
    }

    fn area_release(stage: Stage, area: Area, area_offset: Option<[f32; 2]>) -> Option<[f32; 2]> {
        if let Some([outside, height]) = area_offset {
            let lower = matches!(area, Area::LowerLeft | Area::LowerRight);
            if lower == (height < 0.0) {
                let left = matches!(area, Area::LowerLeft | Area::UpperLeft);
                let ledge = stage.ledges[usize::from(!left)];
                let x = if left { ledge[0] - outside } else { ledge[0] + outside };
                let point = [x, ledge[1] + height];
                // Preserve the ordinary default on stages that would clip this release.
                if stage.edge_start(area, point) == Some(point) {
                    return Some(point);
                }
            }
        }
        stage.area_default(area)
    }

    fn release_offset(area: Area, left: Option<(Area, [f32; 2])>, shared: Option<[f32; 2]>) -> Option<[f32; 2]> {
        left.filter(|(side, _)| *side == area).map(|(_, offset)| offset).or(shared)
    }

    pub fn change_area(&mut self, stage: Stage, delta: i32) {
        let area = self.setup.area.step(delta);
        let offset = Self::release_offset(area, self.left_start, self.area_offset);
        if let Some(cpu) = Self::area_release(stage, area, offset) {
            self.setup.area = area;
            self.setup.cpu = cpu;
        }
    }

    pub fn move_cpu(&mut self, stage: Stage, point: [f32; 2]) {
        if let Some(point) = match self.drill {
            Drill::GuardEdge => stage.edge_start(self.setup.area, point),
            Drill::Juggler => stage.camera_point(point),
        } {
            self.setup.cpu = point;
        }
    }

    pub fn player_floor(
        &self,
        stage: Stage,
        cursor: [f32; 2],
        confirm: bool,
        mut sample: impl FnMut([f32; 2]) -> Option<[f32; 2]>,
    ) -> Option<[f32; 2]> {
        sample(cursor).or_else(|| {
            let previous = self.setup.p1;
            // A ray at a shared lip can hit the wall. Confirm the displayed
            // floor point only after its own fresh sample succeeds.
            if confirm
                && self.drill == Drill::GuardEdge
                && cursor[0] != previous[0]
                && stage.ledges.iter().any(|ledge| cursor[0] == ledge[0])
            {
                sample(previous)
            } else {
                None
            }
        })
    }

    pub fn change_percent(&mut self, delta: i16) {
        self.setup.percent =
            (i32::from(self.setup.percent) + i32::from(delta)).clamp(0, 300) as u16;
    }

    pub fn advance(&mut self) {
        self.step = match self.step {
            WizardStep::Area => WizardStep::Cpu,
            WizardStep::Cpu if self.drill == Drill::Juggler => WizardStep::Percent,
            WizardStep::Cpu | WizardStep::Percent => WizardStep::Player,
            WizardStep::Player | WizardStep::Preview => WizardStep::Preview,
        };
    }

    /// False means B on the first step cancels the wizard.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            WizardStep::Area => return false,
            WizardStep::Cpu if self.drill == Drill::Juggler => return false,
            WizardStep::Cpu => WizardStep::Area,
            WizardStep::Percent => WizardStep::Cpu,
            WizardStep::Player if self.drill == Drill::Juggler => WizardStep::Percent,
            WizardStep::Player => WizardStep::Cpu,
            WizardStep::Preview => WizardStep::Player,
        };
        true
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn release(drill: Drill, limit: Option<u16>) -> Session {
        let mut session = Session::new(drill, limit);
        session.placed();
        for _ in 0..drill.ready_frames() {
            session.advance_attempt(Observation::default());
        }
        assert_eq!((session.phase, session.frames), (Phase::Live, 0));
        session
    }

    #[test]
    fn edge_free_fall_restores_stock_aerial_jump_reserve() {
        // Stock jump_count_max includes the ground jump. The verified Sheik,
        // Pit and Meta Knight starts have one, three and five aerial jumps.
        for (native_max, reference_reserve) in [(2, 1), (4, 3), (6, 5)] {
            assert_eq!(native_max - Drill::GuardEdge.placed_jump_count(), reference_reserve);
        }
        assert_eq!(Drill::Juggler.placed_jump_count(), 0);
    }

    #[test]
    fn cpu_behavior_returns_to_its_saved_value_after_a_load() {
        let original = CpuBehavior { menu: 0, level: 3 };
        let changed_in_pause = CpuBehavior { menu: 4, level: 3 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(true, 1, original, true), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(true, 1, CpuBehavior::LEVEL_NINE, true), None);
        assert_eq!(lease.step(true, 1, changed_in_pause, true), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, 1, CpuBehavior::LEVEL_NINE, true), Some(original));
        assert_eq!(lease.step(false, 1, original, true), None);
    }

    #[test]
    fn cpu_behavior_control_restore_waits_for_stock_apply() {
        let control = CpuBehavior { menu: 8, level: 2 };
        let cpu = CpuBehavior::LEVEL_NINE;
        let mut lease = BehaviorLease::new();
        assert!(control.applied(0x24, 0x24, false));
        assert_eq!(lease.step(true, 1, control, true), Some(cpu));
        assert_eq!(lease.step(true, 1, cpu, cpu.applied(0, 0, false)), Some(cpu));
        assert_eq!(lease.step(true, 1, cpu, cpu.applied(0, 0, true)), None);
        assert_eq!(lease.step(false, 1, cpu, true), Some(control));
        assert!(lease.held(), "the restore stays owned until stock acknowledges Control");
        assert_eq!(lease.step(false, 1, control, control.applied(0x24, 0x24, true)), Some(control));
        assert_eq!(lease.step(false, 1, control, control.applied(0x24, 0x24, false)), None);
        assert!(!lease.held());
    }

    #[test]
    fn cpu_behavior_does_not_write_the_old_settings_to_a_replaced_panel() {
        let a = CpuBehavior { menu: 0, level: 2 };
        let b = CpuBehavior { menu: 5, level: 4 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(true, 1, a, true), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, 2, b, true), None);
        assert_eq!(lease.step(true, 2, b, true), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, 2, CpuBehavior::LEVEL_NINE, true), Some(b));
    }

    #[test]
    fn behavior_pause_pulses_once_per_edge_and_leaves_a_player_pause_open() {
        use BehaviorMenu::{Closed, Open, Transition};
        let mut pause = BehaviorPause::Idle;
        assert!(!pause.pulse(true, Open, true));
        assert!(!pause.pulse(true, Closed, false));
        assert!(pause.pulse(true, Closed, true));
        assert!(!pause.pulse(true, Closed, true));
        assert!(!pause.pulse(true, Open, false));
        assert!(pause.pulse(false, Open, true));
        assert!(!pause.pulse(false, Open, true));
        assert!(!pause.pulse(true, Transition, true));
        assert!(!pause.pulse(false, Closed, true));
        assert!(!pause.pulse(false, Closed, true));
    }

    #[test]
    fn behavior_resume_waits_for_stock_pause_input() {
        let mut pause = BehaviorPause::Idle;
        assert!(pause.pulse(true, BehaviorMenu::from_stock(2, false), true));
        assert!(
            !pause.pulse(true, BehaviorMenu::from_stock(3, true), true),
            "the moving pause pane appears before stock accepts resume"
        );
        assert!(pause.pulse(true, BehaviorMenu::from_stock(5, true), true));
        assert!(!pause.pulse(true, BehaviorMenu::from_stock(21, false), true));
        assert!(!pause.pulse(true, BehaviorMenu::from_stock(2, false), true));
        assert!(pause.pulse(true, BehaviorMenu::from_stock(2, false), true), "a subsequent transaction can open");
    }

    #[test]
    fn reaction_setting_three_values() {
        assert_eq!(Reaction::from_settings(0, 29).frames(), 0);
        assert_eq!(Reaction::from_settings(1, 0).frames(), 12);
        assert_eq!(Reaction::from_settings(2, 0).frames(), 1);
        assert_eq!(Reaction::from_settings(2, 29).frames(), 30);
        assert_eq!(Reaction::from_settings(2, usize::MAX).frames(), 30);
    }

    #[test]
    fn drill_status_uses_the_existing_standing_notice_lifetime() {
        // The text is only posted again when it changes. A player can leave the
        // wizard on one step, or the finished set on screen, for several seconds.
        let adapter = include_str!("../runtime/features/edge_juggle.rs");
        assert!(
            adapter.contains("notices::upsert(NOTICE, header, text.as_str(), u16::MAX, DEFAULT_COLOR)"),
            "an unchanged drill notice must survive the ordinary transient notice lifetime"
        );
    }

    #[test]
    fn edge_wizard_four_areas() {
        let stage = Stage::new([-160.0, 150.0, 110.0, -85.0], [-160.0, 150.0, 110.0, -85.0], [[-60.0, 5.0], [80.0, 8.0]]).unwrap();
        for (area, start, expected) in [
            (Area::LowerLeft, [300.0, 70.0], [-61.0, 5.0]),
            (Area::LowerRight, [0.0, -100.0], [81.0, -84.0]),
            (Area::UpperLeft, [0.0, -20.0], [-61.0, 5.0]),
            (Area::UpperRight, [200.0, 180.0], [149.0, 109.0]),
        ] {
            assert_eq!(stage.edge_start(area, start), Some(expected));
        }
        assert_eq!(stage.edge_start(Area::LowerLeft, [f32::NAN, 0.0]), None);
    }

    #[test]
    fn edge_player_cursor_reverses_immediately_after_the_yoshi_story_lip() {
        let stage = Stage::new(
            [-180.0, 180.0, 120.0, -90.0],
            [-160.0, 160.0, 118.0, -80.0],
            [[-68.0, -4.25], [68.0, -4.25]],
        ).unwrap();
        for side in [-1.0, 1.0] {
            let lip = [side * 68.0, -4.25];
            let outside = [side * 130.0, -4.25];
            let cursor = stage.player_cursor(Drill::GuardEdge, outside).unwrap();
            assert_eq!(
                stage.player_cursor(Drill::GuardEdge, [cursor[0] - side * 2.0, cursor[1]]),
                Some([side * 66.0, -4.25]),
            );
            assert_eq!(cursor, lip);
            assert_eq!(stage.player_cursor(Drill::Juggler, outside), Some(outside));
        }
    }

    #[test]
    fn edge_player_confirms_the_last_solid_floor_when_the_endpoint_is_a_wall() {
        let stage = Stage::new(
            [-180.0, 180.0, 120.0, -90.0],
            [-160.0, 160.0, 118.0, -80.0],
            [[-68.0, -4.25], [68.0, -4.25]],
        ).unwrap();
        let previous = [-66.140_594, -3.862];
        let edge = Wizard::new(Drill::GuardEdge, stage, previous, 100, (0, 0), Some(0)).unwrap();
        let lip = [-68.0, -3.862];
        let mut queries = Vec::new();
        assert_eq!(edge.player_floor(stage, lip, true, |point| {
            queries.push(point);
            (point == previous).then_some(previous)
        }), Some(previous));
        assert_eq!(queries, [lip, previous]);
        assert!(edge.player_floor(stage, lip, false, |point| {
            assert_eq!(point, lip, "only fresh confirmation revalidates the prior point");
            None
        }).is_none());
        assert!(edge.player_floor(stage, lip, true, |_| None).is_none());
        assert!(edge.player_floor(stage, [-60.0, 0.0], true, |point| {
            (point == previous).then_some(previous)
        }).is_none(), "a failed interior sample needs a new cursor location");
        let juggler = Wizard::new(Drill::Juggler, stage, previous, 60, (0, 0), Some(0)).unwrap();
        assert!(juggler.player_floor(stage, lip, true, |point| {
            (point == previous).then_some(previous)
        }).is_none(), "Juggler selects the current platform height");
    }

    #[test]
    fn both_ice_climbers_yield_cpu_settings_but_only_primary_owns_the_pad() {
        use crate::model::cpu::role::FighterKey;

        let primary = FighterKey::new(1, false).unwrap();
        let partner = FighterKey::new(1, true).unwrap();
        for key in [primary, partner] {
            assert!(suspends_cpu_settings(true, key));
            assert!(!suspends_cpu_settings(false, key));
        }
        for entry in [0, 2, 4] {
            assert!(!suspends_cpu_settings(true, FighterKey::new(entry, false).unwrap()));
        }
        assert!(drill_owns_pad(true, false));
        assert!(!drill_owns_pad(false, false));
    }

    #[test]
    fn pre_release_drift_and_jump_cannot_spend_recovery_resources() {
        use perfect::{Correction, FLICK_JUMP, JUMP, SPECIAL};

        let stock = FLICK_JUMP | JUMP | SPECIAL;
        let neutral = Some(Correction {
            stick: Some([0.0, 0.0]),
            release: JUMP | SPECIAL,
            ..Correction::default()
        });
        for drill in [Drill::GuardEdge, Drill::Juggler] {
            let mut session = Session::new(drill, None);
            assert_eq!(session.phase.held_pad(stock), neutral);
            session.placed();
            for _ in 0..drill.ready_frames() {
                assert_eq!(session.phase.held_pad(stock), neutral);
                session.advance_attempt(Observation::default());
            }
            assert_eq!(session.phase, Phase::Live);
            assert_eq!(session.phase.held_pad(stock), None);
        }
    }

    #[test]
    fn attempt_trace_follows_release_and_first_scored_contact() {
        let mut session = Session::new(Drill::GuardEdge, Some(2));
        session.placed();
        let before = session.phase;
        session.advance_attempt(Observation::default());
        assert_eq!(session.trace_event(before), None);
        session = release(Drill::GuardEdge, Some(2));
        assert_eq!(session.trace_event(Phase::Ready(1)), Some((1, 0)));
        assert_eq!(session.trace_event(Phase::Live), None);

        session.advance_attempt(Observation { grounded: true, ..Observation::default() });
        assert_eq!(session.trace_event(Phase::Live), Some((1, 3)));
        assert_eq!(session.frames, 1);
        let before = session.phase;
        session.advance_attempt(Observation { cliff_catch: true, ..Observation::default() });
        assert_eq!(session.trace_event(before), None);

        let mut juggle = release(Drill::Juggler, None);
        juggle.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
        assert_eq!(juggle.trace_event(Phase::Live), Some((1, 7)));

        let mut timeout = release(Drill::GuardEdge, None);
        for _ in 0..EDGE_TIMEOUT {
            timeout.advance_attempt(Observation::default());
        }
        assert_eq!(timeout.trace_event(Phase::Live), Some((1, 4)));
        assert_eq!(timeout.attempts, 0);
    }

    #[test]
    fn edge_scores_ko_and_waits_for_respawn() {
        let mut session = release(Drill::GuardEdge, None);
        let dead = Observation { dead_or_rebirthing: true, ..Observation::default() };
        session.advance_attempt(dead);
        assert_eq!((session.secured, session.attempts, session.streak), (1, 1, 1));
        let bests = &mut [None; 4];
        assert_eq!(session.record_result(bests, 42), Some(1));
        for _ in 0..=RESULT_FRAMES {
            session.advance_attempt(dead);
        }
        assert_eq!(session.phase, Phase::Result(Outcome::Secured, 0));
        session.advance_attempt(Observation::default());
        assert_eq!(session.phase, Phase::Place);
        session.restart_drill();
        assert_eq!(session, Session::new(Drill::GuardEdge, None));
        assert_eq!(super::super::practice_games::best_time(bests, 42), Some(1));
    }

    #[test]
    fn edge_timeout_does_not_spend_an_attempt() {
        let mut session = release(Drill::GuardEdge, Some(10));
        for _ in 0..EDGE_TIMEOUT {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Result(Outcome::Unresolved, RESULT_FRAMES));
        assert_eq!((session.secured, session.attempts), (0, 0));
    }

    #[test]
    fn diddy_lower_wizard_defaults_to_the_measured_rocketbarrel_release() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(0x27)).unwrap();
            assert_eq!(wizard.setup.cpu, [-94.0, -30.0], "{key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, [94.0, -30.0], "{key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
        }
        for key in [(0, 1), (89, 0)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(0x27)).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn lucario_default_shares_the_flown_starts_at_measured_aura() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            for (damage, expected) in [(100, [-102.0, -14.0]), (130, [-102.0, -22.0])] {
                let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], damage, key, Some(0x2c)).unwrap();
                assert_eq!(wizard.setup.cpu, expected, "{key:?}, {damage}%");
            }
        }
        for (key, damage) in [((89, 0), 100), ((3, 1), 100), ((3, 0), 65)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], damage, key, Some(0x2c)).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn mario_lower_left_wizard_offers_the_flown_cape_start() {
        let stage = Stage::new(
            [-240.0, 240.0, 180.0, -140.0], [-180.0, 180.0, 110.0, -85.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0), (4, 0), (3, 1)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [-76.0, 0.0], 100, key, Some(0)).unwrap();
            let left = if perfect::mario::reference_stage(key) { [-114.0, -46.0] }
                else { stage.area_default(Area::LowerLeft).unwrap() };
            assert_eq!(wizard.setup.cpu, left);
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerRight).unwrap());
            wizard.change_area(stage, -1);
            assert_eq!(wizard.setup.cpu, left);
        }
    }

    #[test]
    fn snake_wizard_offers_only_the_measured_left_cypher_start() {
        let stage = Stage::new(
            [-240.0, 240.0, 180.0, -140.0], [-180.0, 180.0, 110.0, -85.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0), (4, 0), (3, 1)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [-76.0, 0.0], 100, key, Some(0x22)).unwrap();
            let left = if perfect::is_crossing_stage(key) { [-100.0, -4.0] }
                else { stage.area_default(Area::LowerLeft).unwrap() };
            assert_eq!(wizard.setup.cpu, left);
            assert_eq!(recovery_faces_away(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)), perfect::is_crossing_stage(key));
            assert!(!recovery_faces_away(Drill::Juggler, key, stage, wizard.setup, Some(0x22)));
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerRight).unwrap());
            assert!(!recovery_faces_away(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)));
            wizard.change_area(stage, -1);
            assert_eq!(wizard.setup.cpu, left);
        }
    }

    #[test]
    fn link_wizard_places_the_proven_left_bomb_start_in_both_difficulties() {
        let stage = Stage::new(
            [-240.0, 240.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-79.99, 0.0], [79.99, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0), (0, 1), (89, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [-76.0, 0.0], 100, key, Some(0x2)).unwrap();
            wizard.change_area(stage, 2);
            if perfect::is_crossing_stage(key) {
                assert_eq!(wizard.setup.cpu, [-109.99, 18.0]);
                assert!(recovery_faces_away(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)));
                assert!(!recovery_faces_away(Drill::Juggler, key, stage, wizard.setup, Some(0x2)));
                assert!(!recovery_faces_away(Drill::GuardEdge, key, stage, Setup { percent: 120, ..wizard.setup }, Some(0x2)));
            } else {
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
            }
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperRight).unwrap());
            assert!(!recovery_faces_away(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)));
        }
    }

    #[test]
    fn rob_upper_release_uses_the_flown_cell_on_each_allowed_stage() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0],
            [-200.0, 200.0, 120.0, -120.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(3, 0), (0, 0), (311, 0), (0, 1), (89, 0)] {
            let mut wizard = Wizard::new(
                Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(0x2d),
            ).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
            for (area, expected) in [(Area::UpperLeft, [-120.0, 20.0]),
                                     (Area::UpperRight, [120.0, 20.0])] {
                while wizard.setup.area != area {
                    wizard.change_area(stage, 1);
                }
                let actual = if perfect::is_crossing_stage(key) {
                    expected
                } else {
                    stage.area_default(area).unwrap()
                };
                assert_eq!(wizard.setup.cpu, actual, "stage {key:?}, area {area:?}");
            }
        }
    }

    #[test]
    fn wing_wizard_defaults_to_the_measured_upper_release_on_each_normal_crossing_stage() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for fighter in [0x1d, 0x1e, 0x1f] {
            for key in [(0, 0), (3, 0), (311, 0), (0, 1)] {
                let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(fighter)).unwrap();
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
                wizard.change_area(stage, 2);
                let expected = if key.1 == 0 { [-110.0, 6.0] } else { stage.area_default(Area::UpperLeft).unwrap() };
                assert_eq!(wizard.setup.cpu, expected);
            }
        }
    }

    #[test]
    fn peach_and_daisy_wizard_offers_the_measured_parasol_start_on_both_sides() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for fighter in [0xd, 0xe] {
            for key in [(0, 0), (3, 0), (311, 0), (0, 1), (89, 0)] {
                let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(fighter)).unwrap();
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
                wizard.change_area(stage, 2);
                assert_eq!(wizard.setup.cpu, if perfect::is_crossing_stage(key) {
                    [-120.0, 20.0]
                } else {
                    stage.area_default(Area::UpperLeft).unwrap()
                });
                wizard.change_area(stage, 1);
                assert_eq!(wizard.setup.cpu, if perfect::is_crossing_stage(key) {
                    [120.0, 20.0]
                } else {
                    stage.area_default(Area::UpperRight).unwrap()
                });
            }
        }
    }

    #[test]
    fn pacman_upper_wizard_defaults_to_the_measured_pellet_start() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0],
            [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(0x37)).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
            while wizard.setup.area != Area::UpperLeft {
                wizard.change_area(stage, 1);
            }
            assert_eq!(wizard.setup.cpu, [-106.0, 14.0]);
            while wizard.setup.area != Area::UpperRight {
                wizard.change_area(stage, 1);
            }
            assert_eq!(wizard.setup.cpu, [106.0, 14.0]);
        }
    }

    #[test]
    fn pichu_lower_wizard_starts_in_its_measured_skull_bash_region() {
        let stage = Stage::new(
            [-240.0, 240.0, 160.0, -140.0],
            [-160.0, 160.0, 110.0, -105.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let pichu = Some(0x13);
        for key in [(3, 0), (0, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 0, key, pichu).unwrap();
            assert_eq!(wizard.setup.cpu, [-100.0, -28.0], "{key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, [100.0, -28.0], "{key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
        }
        for key in [(89, 0), (3, 1)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 0, key, pichu).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn puff_wizard_releases_inside_the_measured_three_stage_ring() {
        let stage = Stage::new(
            [-240.0, 240.0, 160.0, -140.0],
            [-160.0, 160.0, 110.0, -105.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let puff = Some(0xc);
        for key in [(3, 0), (0, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, key, puff)
                .unwrap();
            assert_eq!(wizard.setup.cpu, [-100.0, -28.0], "stage {key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, [100.0, -28.0], "stage {key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
        }
        let mut waiting_room =
            Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (311, 0), puff).unwrap();
        assert_eq!(waiting_room.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        waiting_room.change_area(stage, 2);
        assert_eq!(waiting_room.setup.cpu, [-120.0, 20.0]);
        waiting_room.change_area(stage, 1);
        assert_eq!(waiting_room.setup.cpu, [120.0, 20.0]);
        for key in [(3, 1), (0, 1), (311, 1)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, key, puff).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn sora_wizard_uses_the_widened_default_release_on_the_three_measured_stages() {
        let stage = Stage::new(
            [-200.0, 200.0, 180.0, -140.0],
            [-160.0, 160.0, 120.0, -100.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        )
        .unwrap();
        for id in [0, 3, 311] {
            let mut wizard = Wizard::new(
                Drill::GuardEdge,
                stage,
                [0.0, 0.0],
                100,
                (id, 0),
                Some(perfect::sora_flight::KIND),
            )
            .unwrap();
            assert_eq!(wizard.setup.cpu, [-100.0, -28.0]);
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, [100.0, -28.0]);
        }
    }

    #[test]
    fn sheik_lower_wizard_release_remains_in_the_proven_fish_region() {
        let stage = Stage::new(
            [-200.0, 200.0, 180.0, -140.0],
            [-160.0, 160.0, 120.0, -100.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let sheik = Some(0x10);
        for id in [0, 3, 311] {
            let mut wiz = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, (id, 0), sheik).unwrap();
            assert_eq!(wiz.setup.cpu, [-98.0, -30.0]);
            wiz.change_area(stage, 1);
            assert_eq!(wiz.setup.cpu, [98.0, -30.0]);
            wiz.change_area(stage, 1);
            assert_eq!(wiz.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
        }
        for key in [(89, 0), (0, 1)] {
            let wiz = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, key, sheik).unwrap();
            assert_eq!(wiz.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn pikachu_low_wizard_release_starts_below_the_cliff_inside_the_crossing_region() {
        let stage = Stage::new(
            [-240.0, 240.0, 160.0, -140.0],
            [-160.0, 160.0, 110.0, -105.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let pikachu = Some(0x8);
        for stage_key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, stage_key, pikachu)
                .unwrap();
            assert_eq!(wizard.setup.cpu, [-100.0, -55.0]);
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, [100.0, -55.0]);
            wizard.advance();
            wizard.move_cpu(stage, [125.0, -40.0]);
            assert_eq!(wizard.setup.cpu, [125.0, -40.0], "the wizard still edits the point");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
        }

        for (id, kind) in [((3, 1), pikachu), ((89, 0), pikachu), ((3, 0), Some(0x18)), ((3, 0), None)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, id, kind).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
        let juggle = Wizard::new(Drill::Juggler, stage, [0.0, 0.0], 100, (3, 0), pikachu).unwrap();
        assert_eq!(juggle.setup.cpu, stage.default_juggle([0.0, 0.0]));
    }

    #[test]
    fn juggle_wizard_percent_and_player_step() {
        let stage = Stage::new([-160.0, 150.0, 110.0, -85.0], [-160.0, 150.0, 110.0, -85.0], [[-60.0, 5.0], [80.0, 8.0]]).unwrap();
        let mut wizard = Wizard::new(Drill::Juggler, stage, [0.0, 7.0], 60, (3, 0), None).unwrap();
        assert_eq!(wizard.step, WizardStep::Cpu);
        wizard.move_cpu(stage, [-150.0, 90.0]);
        wizard.advance();
        assert_eq!(wizard.step, WizardStep::Percent);
        assert_eq!(wizard.setup.cpu, [-150.0, 90.0]);
        wizard.change_percent(300);
        assert_eq!(wizard.setup.percent, 300);
        wizard.change_percent(-301);
        assert_eq!(wizard.setup.percent, 0);
        wizard.advance();
        assert_eq!(wizard.step, WizardStep::Player);
        wizard.back();
        assert_eq!(wizard.step, WizardStep::Percent);
    }

    #[test]
    fn juggle_clock_starts_at_release() {
        let mut session = release(Drill::Juggler, Some(1));
        for _ in 0..25 {
            session.advance_attempt(Observation::default());
        }
        session.advance_attempt(Observation { hit: true, ..Observation::default() });
        session.advance_attempt(Observation { grounded: true, ..Observation::default() });
        assert_eq!(session.phase, Phase::Result(Outcome::Landed, RESULT_FRAMES));
        assert_eq!((session.frames, session.hits), (27, 1));
        let bests = &mut [Some((0, 15)), None];
        assert_eq!(session.record_result(bests, 0), Some(27));
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Done);
    }

    #[test]
    fn menu_interruption_discards_the_current_attempt_only() {
        let mut session = release(Drill::GuardEdge, None);
        session.advance_attempt(Observation { cliff_catch: true, ..Observation::default() });
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        session.placed();
        session.discard_attempt();
        assert_eq!((session.phase, session.attempts), (Phase::Place, 1));
    }
    }
}
