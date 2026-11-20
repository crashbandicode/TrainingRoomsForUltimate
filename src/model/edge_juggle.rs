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

/// Training's pause-menu selection and the CPU's corresponding native control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuBehavior {
    pub menu: u32,
    pub level: u32,
    pub control: u8,
}

impl CpuBehavior {
    pub const LEVEL_NINE: Self = Self {
        menu: 2,
        level: 8,
        control: 0,
    };
}

/// Save the original selection once for the lifetime of its Training panel.
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
        safe: bool,
        panel: usize,
        current: CpuBehavior,
    ) -> Option<CpuBehavior> {
        if !safe {
            return None;
        }
        if self.0.is_some_and(|(owner, _)| owner != panel) {
            self.0 = None;
        }
        let desired = if running {
            self.0.get_or_insert((panel, current));
            CpuBehavior::LEVEL_NINE
        } else {
            self.0.take()?.1
        };
        (desired != current).then_some(desired)
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
    pub step: WizardStep,
    pub setup: Setup,
}

impl Wizard {
    pub fn new(drill: Drill, stage: Stage, p1: [f32; 2], percent: u16) -> Option<Self> {
        let area = Area::default();
        let (step, cpu) = match drill {
            Drill::GuardEdge => (WizardStep::Area, stage.area_default(area)?),
            Drill::Juggler => (WizardStep::Cpu, stage.default_juggle(p1)),
        };
        Some(Self {
            drill,
            step,
            setup: Setup {
                area,
                cpu,
                p1: stage.camera_point(p1)?,
                percent: percent.min(300),
            },
        })
    }

    pub fn change_area(&mut self, stage: Stage, delta: i32) {
        let area = self.setup.area.step(delta);
        if let Some(cpu) = stage.area_default(area) {
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
    fn cpu_behavior_returns_to_its_saved_value_after_a_load() {
        let original = CpuBehavior { menu: 0, level: 3, control: 1 };
        let changed_in_pause = CpuBehavior { menu: 4, level: 3, control: 5 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(true, true, 1, original), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(true, true, 1, CpuBehavior::LEVEL_NINE), None);
        assert_eq!(lease.step(true, true, 1, changed_in_pause), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, false, 1, CpuBehavior::LEVEL_NINE), None);
        assert_eq!(lease.step(false, true, 1, CpuBehavior::LEVEL_NINE), Some(original));
        assert_eq!(lease.step(false, true, 1, original), None);
    }

    #[test]
    fn cpu_behavior_does_not_write_the_old_settings_to_a_replaced_panel() {
        let a = CpuBehavior { menu: 0, level: 2, control: 1 };
        let b = CpuBehavior { menu: 5, level: 4, control: 3 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(true, true, 1, a), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, true, 2, b), None);
        assert_eq!(lease.step(true, true, 2, b), Some(CpuBehavior::LEVEL_NINE));
        assert_eq!(lease.step(false, true, 2, CpuBehavior::LEVEL_NINE), Some(b));
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
    fn juggle_wizard_percent_and_player_step() {
        let stage = Stage::new([-160.0, 150.0, 110.0, -85.0], [-160.0, 150.0, 110.0, -85.0], [[-60.0, 5.0], [80.0, 8.0]]).unwrap();
        let mut wizard = Wizard::new(Drill::Juggler, stage, [0.0, 7.0], 60).unwrap();
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
