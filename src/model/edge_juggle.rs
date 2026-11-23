//! Shared attempt clock and scoring for Guard the Edge and Juggler (DESIGN v2).
use crate::model::practice_state::Fields;
use super::practice_games::{record_max, record_time, result_tick, Game};

#[path = "edge_juggle/perfect.rs"]
pub mod perfect;
#[path = "edge_juggle/picker.rs"]
pub mod picker;
#[path = "edge_juggle/zoom.rs"]
pub mod zoom;

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
    pub const fn stock(level: u8) -> Self {
        Self { menu: 2, level: (level - 1) as u32 }
    }

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
        level: Option<u8>,
        panel: usize,
        current: CpuBehavior,
        applied: bool,
    ) -> Option<CpuBehavior> {
        if self.0.is_some_and(|(owner, _)| owner != panel) {
            self.0 = None;
        }
        let desired = if let Some(level) = level {
            self.0.get_or_insert((panel, current));
            CpuBehavior::stock(level)
        } else {
            self.0?.1
        };
        if desired != current || !applied {
            return Some(desired);
        }
        if level.is_none() {
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
    ZoomWin,
    ZoomMiss,
    Secured,
    RecoveredAtLedge,
    RecoveredOnStage,
    Unresolved,
    Landed,
    GrabbedLedge,
    KnockedOut,
}

impl Outcome {
    const fn bridge_name(self) -> &'static str {
        match self {
            Self::ZoomWin => "zoom_win",
            Self::ZoomMiss => "zoom_miss",
            Self::Secured => "secured",
            Self::RecoveredAtLedge => "recovered_at_ledge",
            Self::RecoveredOnStage => "recovered_on_stage",
            Self::Unresolved => "unresolved",
            Self::Landed => "landed",
            Self::GrabbedLedge => "grabbed_ledge",
            Self::KnockedOut => "knocked_out",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub dead_or_rebirthing: bool,
    pub grounded: bool,
    pub cliff_catch: bool,
    pub hit: bool,
}

/// Player 1 as the drill reads it around its start write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedPlayer {
    /// The game's own hit freeze.
    pub hit_freeze: bool,
    /// Standing idle, the status the start write asks for.
    pub wait: bool,
}

impl PlacedPlayer {
    /// A start write that finds Player 1 idle holds. One that finds it in a hit does not: a speed outlives the write
    /// and Player 1 slides off its point in Wait (docs/design/rc15-gte-wizard-start.md).
    pub const fn watch(self) -> bool {
        self.hit_freeze || !self.wait
    }

    /// On a frame between a watched start write and Ready: write the start again? Only while the hit still freezes
    /// Player 1 or it stands idle off its point; a status of its own is the player's. `spent` counts the rewrites of
    /// this attempt: at most one ready period of them, then the drill goes on as before.
    pub const fn rewrite(self, on_point: bool, drill: Drill, spent: u16) -> bool {
        spent < drill.ready_frames() && (self.hit_freeze || self.wait && !on_point)
    }

    /// One placement update between the start write and Ready. `watched` is, for a watched start, Player 1 as read on
    /// this update, whether it stands on its start (`None` when the floor under it could not be read), and the rewrites
    /// spent; `None` for a start that found it idle.
    /// Player 1 is judged on every update, also while the CPU is not in Fall yet. Returns (write the start again,
    /// start Ready): Ready starts on the first update with the CPU in Fall and nothing written.
    pub const fn before_ready(watched: Option<(Self, Option<bool>, u16)>, cpu_falling: bool, drill: Drill) -> (bool, bool) {
        let rewrite = match watched {
            // An unread floor shows no displacement: only a hit freeze is written again then.
            Some((player, on_start, spent)) => player.rewrite(
                match on_start {
                    Some(on_start) => on_start,
                    None => true,
                },
                drill,
                spent,
            ),
            None => false,
        };
        (rewrite, cpu_falling && !rewrite)
    }
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

/// The game's answer for an object that has no kind, or no status, yet.
pub const NO_NATIVE_VALUE: i32 = -1;

/// Entry 1's object as a run meets it on one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuObject {
    /// The object the run owns.
    Owned,
    /// A replacement the game made active before giving it a kind and a first status (the
    /// Aegis swap). A status request ends the current status and it has none, so nothing
    /// may be requested of it, and it is not a new CPU yet.
    Arriving,
    /// A replacement with a kind and a status.
    Replaced,
}

impl CpuObject {
    pub fn sighted(owned: bool, kind: i32, status: i32) -> Self {
        if owned {
            Self::Owned
        } else if kind == NO_NATIVE_VALUE || status == NO_NATIVE_VALUE {
            Self::Arriving
        } else {
            Self::Replaced
        }
    }
}

/// Placement has a native grounded resource reset before the airborne start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    ResetResources,
    Grounded,
    Airborne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementAction {
    RestartPlacement,
    ResetResources,
    PlaceStart,
    AwaitNativeWait,
    RequestFall,
    HoldStart,
}

#[derive(Clone, Copy, Default)]
pub struct PlacementFrame {
    pub dead_or_rebirthing: bool,
    pub grounded: bool,
    pub airborne: bool,
    pub wait: bool,
    pub fall: bool,
}

impl Placement {
    pub fn action(self, frame: PlacementFrame) -> PlacementAction {
        use PlacementAction as A;
        if frame.dead_or_rebirthing {
            return A::RestartPlacement;
        }
        match self {
            Self::ResetResources => A::ResetResources,
            Self::Grounded if frame.grounded && frame.wait => A::PlaceStart,
            Self::Grounded if frame.grounded => A::AwaitNativeWait,
            Self::Grounded => A::RestartPlacement,
            Self::Airborne if frame.airborne && frame.fall => A::HoldStart,
            Self::Airborne if frame.airborne => A::RequestFall,
            Self::Airborne => A::RestartPlacement,
        }
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
    rewrites: u16,
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
            rewrites: 0,
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
            self.rewrites = 0;
            self.frames = 0;
            self.hits = 0;
            self.phase = Phase::Ready(self.drill.ready_frames());
        }
    }

    /// Rewrites of Player 1's start spent since this session's last Ready. A placement that starts again keeps
    /// them; Ready and a new session (a Training reset, another CPU body) start a new allowance.
    pub const fn player_rewrites(&self) -> u16 {
        self.rewrites
    }

    pub fn player_rewritten(&mut self) {
        self.rewrites = self.rewrites.saturating_add(1);
    }

    pub fn advance_zoom(
        &mut self,
        attempt: &mut zoom::Attempt,
        observed: Observation,
        mut read: zoom::Observation,
        tolerance: u8,
    ) {
        if self.phase != Phase::Live {
            self.advance_attempt(observed);
            return;
        }
        self.frames = self.frames.saturating_add(1);
        read.ended = observed.dead_or_rebirthing || observed.cliff_catch || self.frames == EDGE_TIMEOUT;
        let Some(verdict) = attempt.step(self.frames, read, tolerance) else { return };
        self.finish_attempt(match verdict {
            zoom::Verdict::Win => Outcome::ZoomWin,
            zoom::Verdict::Miss => Outcome::ZoomMiss,
        });
    }

    fn finish_attempt(&mut self, outcome: Outcome) {
        if outcome != Outcome::Unresolved {
            self.attempts = self.attempts.saturating_add(1);
            if matches!(outcome, Outcome::Secured | Outcome::ZoomWin) {
                self.secured = self.secured.saturating_add(1);
                self.streak = self.streak.saturating_add(1);
                self.best_streak = self.best_streak.max(self.streak);
            } else {
                self.streak = 0;
            }
        }
        self.phase = Phase::Result(outcome, RESULT_FRAMES);
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
                self.finish_attempt(outcome);
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
            Outcome::ZoomWin | Outcome::ZoomMiss => return None,
        };
        Some((self.attempts, code))
    }

    /// Projects the same attempt state that drives the drill HUD onto the bridge.
    pub fn practice_fields(&self, wizard: Option<Wizard>, best: Option<u32>) -> Fields {
        if let Some(wizard) = wizard {
            let (step, selection, point, player, value) = match wizard.step {
                WizardStep::Area => (
                    "area",
                    wizard.setup.area.bridge_name(),
                    Some(wizard.setup.cpu),
                    None,
                    None,
                ),
                WizardStep::Cpu => ("cpu", "", Some(wizard.setup.cpu), None, None),
                WizardStep::Percent => ("percent", "", None, None, Some(u32::from(wizard.setup.percent))),
                WizardStep::Player => ("player", "", Some(wizard.setup.p1), None, None),
                WizardStep::Preview => (
                    "preview",
                    "",
                    Some(wizard.setup.cpu),
                    Some(wizard.setup.p1),
                    Some(u32::from(wizard.setup.percent)),
                ),
            };
            return Fields {
                phase: "wizard",
                wizard_step: step,
                wizard_selection: selection,
                wizard_point: point,
                wizard_player_point: player,
                wizard_value: value,
                ..Fields::default()
            };
        }

        let (phase, remaining_frames, last_outcome) = match self.phase {
            Phase::Place => ("place", None, ""),
            Phase::Ready(left) => ("ready", Some(u32::from(left)), ""),
            Phase::Live => ("live", None, ""),
            Phase::Result(outcome, left) => ("result", Some(u32::from(left)), outcome.bridge_name()),
            Phase::Done => ("done", None, ""),
        };
        let edge = self.drill == Drill::GuardEdge;
        Fields {
            phase,
            attempt_frames: Some(self.frames),
            remaining_frames,
            score: edge.then_some(u32::from(self.secured)),
            streak: edge.then_some(u32::from(self.streak)),
            best,
            attempts: Some(u32::from(self.attempts)),
            hits: (!edge).then_some(u32::from(self.hits)),
            last_outcome,
            ..Fields::default()
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

    const fn bridge_name(self) -> &'static str {
        match self {
            Self::LowerLeft => "lower_left",
            Self::LowerRight => "lower_right",
            Self::UpperLeft => "upper_left",
            Self::UpperRight => "upper_right",
        }
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

pub fn recovery_away_facing(
    drill: Drill,
    stage_key: (i32, u32),
    stage: Stage,
    setup: Setup,
    fighter: Option<i32>,
) -> Option<f32> {
    if drill != Drill::GuardEdge {
        return None;
    }
    let [left, right] = stage.ledges;
    let takes_away_facing_route = match fighter {
        Some(perfect::link::KIND) => {
            perfect::link::in_region(stage_key, stage.ledges, setup.cpu, setup.percent.into())
        }
        Some(perfect::snake::KIND) => perfect::snake::in_region(stage_key, stage.ledges, setup.cpu),
        _ => false,
    };
    takes_away_facing_route.then_some(if setup.cpu[0] < left[0].midpoint(right[0]) { -1.0 } else { 1.0 })
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
    moved: bool,
    pub step: WizardStep,
    pub setup: Setup,
}

impl Wizard {
    pub fn new(
        drill: Drill, stage: Stage, p1: [f32; 2], percent: u16,
        stage_key: (i32, u32), fighter: Option<i32>,
    ) -> Option<Self> {
        let area = Area::default();
        let link_start = drill == Drill::GuardEdge
            && fighter == Some(perfect::link::KIND)
            && perfect::link::in_region(stage_key, stage.ledges, [stage.ledges[0][0] - 30.0, stage.ledges[0][1] + 18.0], percent.into());
        let lucario_left_start = if drill == Drill::GuardEdge && fighter == Some(perfect::lucario::KIND) {
            perfect::lucario::wizard_offset(stage_key, percent)
        } else {
            None
        };
        let left_start = if let Some(offset) = lucario_left_start {
            Some((Area::LowerLeft, offset))
        } else if link_start {
            Some((Area::UpperLeft, [30.0, 18.0]))
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::greninja::KIND) {
            perfect::greninja::wizard_offset(stage_key).map(|offset| (Area::UpperLeft, offset))
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::mario::KIND)
            && perfect::mario::reference_stage(stage_key)
        {
            Some((Area::LowerLeft, [34.0, -46.0]))
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::snake::KIND)
            && perfect::is_crossing_stage(stage_key)
        {
            Some((Area::LowerLeft, [20.0, -4.0]))
        } else if drill == Drill::GuardEdge && perfect::is_crossing_stage(stage_key) {
            match fighter {
                Some(perfect::ness::KIND) => perfect::ness::wizard_offset(stage_key).map(|offset| (Area::LowerLeft, offset)),
                Some(perfect::lucas::KIND) if perfect::lucas::reference_stage(stage_key) => {
                    Some((Area::UpperLeft, perfect::lucas::WIZARD_OFFSET))
                }
                Some(perfect::sonic::KIND) => Some((Area::UpperLeft, perfect::sonic::WIZARD_OFFSET)),
                Some(perfect::hero::KIND) if perfect::hero::reference_stage(stage_key) => {
                    Some((Area::UpperLeft, perfect::hero::WIZARD_OFFSET))
                }
                Some(perfect::steve::KIND) => Some((Area::UpperLeft, [18.0, 30.0])),
                Some(perfect::pyra::PYRA) => Some((Area::LowerLeft, [104.0, -50.0])),
                Some(perfect::incineroar::KIND) => Some((Area::UpperLeft, [110.0, 10.0])),
                _ => None,
            }
        } else {
            None
        };
        let area_offset = match (drill, fighter) {
            (Drill::GuardEdge, Some(perfect::link::KIND)) if link_start => Some([38.0, 38.0]),
            (Drill::GuardEdge, Some(perfect::snake::KIND)) if stage_key == (3, 0) => {
                Some([20.0, -4.0])
            }
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
            moved: false,
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

    fn default_cpu(&self, stage: Stage, area: Area) -> Option<[f32; 2]> {
        Self::area_release(stage, area, Self::release_offset(area, self.left_start, self.area_offset))
    }

    pub fn unmoved(&self) -> bool {
        self.drill == Drill::GuardEdge && !self.moved
    }

    pub fn shows_cpu(&self) -> bool {
        matches!(self.step, WizardStep::Area | WizardStep::Cpu | WizardStep::Preview)
    }

    pub fn back_to_defaults(&mut self, defaults: Setup) {
        self.setup = defaults;
        self.moved = false;
        self.step = WizardStep::Preview;
    }

    pub fn remembered(&self, stage: Stage, mut setup: Setup, unmoved: bool) -> Setup {
        if unmoved {
            if let Some(cpu) = self.default_cpu(stage, setup.area) {
                setup.cpu = cpu;
            }
        }
        setup
    }

    pub fn change_area(&mut self, stage: Stage, delta: i32) {
        let area = self.setup.area.step(delta);
        let offset = Self::release_offset(area, self.left_start, self.area_offset);
        if let Some(cpu) = Self::area_release(stage, area, offset) {
            self.setup.area = area;
            self.setup.cpu = cpu;
            self.moved = false;
        }
    }

    pub fn move_cpu(&mut self, stage: Stage, point: [f32; 2]) {
        if let Some(point) = match self.drill {
            Drill::GuardEdge => stage.edge_start(self.setup.area, point),
            Drill::Juggler => stage.camera_point(point),
        } {
            self.moved |= point != self.setup.cpu;
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

    #[test]
    fn hit_player_start_is_rewritten_until_it_holds() {
        let idle = PlacedPlayer { hit_freeze: false, wait: true };
        let own_status = PlacedPlayer { hit_freeze: false, wait: false };
        let frozen_idle = PlacedPlayer { hit_freeze: true, wait: true };
        let frozen_other = PlacedPlayer { hit_freeze: true, wait: false };
        assert!(!idle.watch());
        assert!(own_status.watch());
        assert!(frozen_idle.watch());
        assert!(frozen_other.watch());

        assert!(idle.rewrite(false, Drill::GuardEdge, 0));
        assert!(!idle.rewrite(true, Drill::GuardEdge, 0));
        assert!(frozen_idle.rewrite(true, Drill::GuardEdge, 0));
        assert!(!own_status.rewrite(false, Drill::GuardEdge, 0));
        assert!(frozen_other.rewrite(false, Drill::GuardEdge, 0));

        assert_eq!(Drill::GuardEdge.ready_frames(), 60);
        assert_eq!(Drill::Juggler.ready_frames(), 45);
        assert!(idle.rewrite(false, Drill::GuardEdge, 59));
        assert!(!idle.rewrite(false, Drill::GuardEdge, 60));
        assert!(idle.rewrite(false, Drill::Juggler, 44));
        assert!(!idle.rewrite(false, Drill::Juggler, 45));

        let mut spent = 0;
        for (player, on_point) in [(frozen_idle, true); 8].into_iter().chain([(idle, false); 5]) {
            if player.rewrite(on_point, Drill::GuardEdge, spent) {
                spent += 1;
            }
        }
        assert_eq!(spent, 13);
        assert!(!idle.rewrite(true, Drill::GuardEdge, spent));
    }

    #[test]
    fn player_start_is_judged_on_every_update_before_ready() {
        let idle = PlacedPlayer { hit_freeze: false, wait: true };
        let own = PlacedPlayer { hit_freeze: false, wait: false };
        let frozen = PlacedPlayer { hit_freeze: true, wait: true };
        let g = Drill::GuardEdge;
        let j = Drill::Juggler;
        assert_eq!(PlacedPlayer::before_ready(None, false, g), (false, false));
        assert_eq!(PlacedPlayer::before_ready(None, true, g), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 0)), false, g), (true, false));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 0)), true, g), (true, false));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(true), 0)), false, g), (false, false));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(true), 0)), true, g), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((frozen, Some(true), 0)), true, g), (true, false));
        assert_eq!(PlacedPlayer::before_ready(Some((own, Some(false), 0)), true, g), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 59)), true, g), (true, false));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 60)), true, g), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 44)), true, j), (true, false));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), 45)), true, j), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, None, 0)), true, g), (false, true));
        assert_eq!(PlacedPlayer::before_ready(Some((idle, None, 0)), false, g), (false, false));
        assert_eq!(PlacedPlayer::before_ready(Some((frozen, None, 0)), true, g), (true, false));
    }

    #[test]
    fn player_rewrites_survive_a_placement_restart_within_a_session() {
        let idle = PlacedPlayer { hit_freeze: false, wait: true };
        let airborne = PlacementFrame { airborne: true, ..PlacementFrame::default() };
        let grounded = PlacementFrame { grounded: true, ..PlacementFrame::default() };
        let wait = PlacementFrame { wait: true, ..grounded };
        let fall = PlacementFrame { fall: true, ..airborne };
        for (drill, before_restart) in [(Drill::GuardEdge, 59), (Drill::Juggler, 44)] {
            let mut session = Session::new(drill, None);
            let mut placement = Placement::Airborne;
            let mut writes = 0;
            for _ in 0..before_restart {
                assert_eq!(placement.action(airborne), PlacementAction::RequestFall);
                assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), session.player_rewrites())), false, drill), (true, false));
                session.player_rewritten();
                writes += 1;
            }
            assert_eq!(placement.action(grounded), PlacementAction::RestartPlacement);
            placement = Placement::ResetResources;
            assert_eq!(placement.action(grounded), PlacementAction::ResetResources);
            placement = Placement::Grounded;
            assert_eq!(placement.action(wait), PlacementAction::PlaceStart);
            placement = Placement::Airborne;
            assert_eq!(session.player_rewrites(), before_restart);
            assert_eq!(placement.action(fall), PlacementAction::HoldStart);
            assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), session.player_rewrites())), true, drill), (true, false));
            session.player_rewritten();
            writes += 1;
            assert_eq!(placement.action(fall), PlacementAction::HoldStart);
            assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), session.player_rewrites())), true, drill), (false, true));
            assert_eq!(writes, drill.ready_frames());
            session.placed();
            assert_eq!(session.player_rewrites(), 0);
            assert_eq!(session.phase, Phase::Ready(drill.ready_frames()));
        }
    }

    #[test]
    fn a_new_session_and_ready_start_a_new_player_allowance() {
        let idle = PlacedPlayer { hit_freeze: false, wait: true };
        let drill = Drill::GuardEdge;
        let mut session = Session::new(drill, None);
        for _ in 0..59 {
            session.player_rewritten();
        }
        session.restart_drill();
        assert_eq!(session.player_rewrites(), 0);
        assert_eq!(PlacedPlayer::before_ready(Some((idle, Some(false), session.player_rewrites())), true, drill), (true, false));
        for _ in 0..59 {
            session.player_rewritten();
        }
        session.placed();
        assert_eq!(session.player_rewrites(), 0);
        assert_eq!(session.phase, Phase::Ready(drill.ready_frames()));
        session.discard_attempt();
        assert_eq!(session.player_rewrites(), 0);
        assert_eq!(session.phase, Phase::Place);
    }

    #[test]
    fn placement_defers_airborne_start_until_the_stock_escape_settles_in_wait() {
        use PlacementAction as A;
        let escape_back = PlacementFrame { grounded: true, ..PlacementFrame::default() };
        assert_eq!(Placement::Grounded.action(escape_back), A::AwaitNativeWait);
        let wait = PlacementFrame { wait: true, ..escape_back };
        assert_eq!(Placement::Grounded.action(wait), A::PlaceStart);
    }

    #[test]
    fn placement_restarts_refill_when_airborne_handoff_lands_before_fall() {
        use PlacementAction as A;
        let escape_air = PlacementFrame { airborne: true, ..PlacementFrame::default() };
        assert_eq!(Placement::Airborne.action(escape_air), A::RequestFall);
        let landing_light = PlacementFrame { grounded: true, ..PlacementFrame::default() };
        assert_eq!(Placement::Airborne.action(landing_light), A::RestartPlacement);
        assert_eq!(Placement::ResetResources.action(landing_light), A::ResetResources);
        let fall = PlacementFrame { fall: true, ..escape_air };
        assert_eq!(Placement::Airborne.action(fall), A::HoldStart);
        let stale_fall = PlacementFrame { fall: true, ..landing_light };
        assert_eq!(Placement::Airborne.action(stale_fall), A::RestartPlacement);
    }

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
    fn guard_edge_publishes_wizard_and_scored_attempt() {
        let stage = Stage::new(
            [-160.0, 150.0, 110.0, -85.0],
            [-160.0, 150.0, 110.0, -85.0],
            [[-60.0, 5.0], [80.0, 8.0]],
        ).unwrap();
        let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 7.0], 100, (0, 0), None).unwrap();
        let mut session = Session::new(Drill::GuardEdge, Some(1));
        wizard.change_area(stage, 1);
        let fields = session.practice_fields(Some(wizard), None);
        assert_eq!((fields.phase, fields.wizard_step, fields.wizard_selection), ("wizard", "area", "lower_right"));
        assert_eq!(fields.wizard_point, Some(wizard.setup.cpu));
        let mut wire = String::new();
        crate::model::practice_state::snapshot(Some(Drill::GuardEdge.game()), fields, [0; 8], &mut wire);
        assert!(wire.contains("phase=wizard\nwizard.step=area\nwizard.selection=lower_right\n"));

        assert_eq!(session.practice_fields(None, None).phase, "place");
        session.placed();
        let ready = session.practice_fields(None, None);
        assert_eq!((ready.phase, ready.remaining_frames), ("ready", Some(u32::from(Drill::GuardEdge.ready_frames()))));
        for _ in 0..Drill::GuardEdge.ready_frames() {
            session.advance_attempt(Observation::default());
        }
        let live = session.practice_fields(None, None);
        assert_eq!((live.phase, live.attempt_frames), ("live", Some(0)));
        session.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
        let result = session.practice_fields(None, Some(1));
        assert_eq!((result.phase, result.last_outcome), ("result", "secured"));
        assert_eq!((result.attempt_frames, result.score, result.attempts), (Some(1), Some(1), Some(1)));
        assert_eq!((result.streak, result.best), (Some(1), Some(1)));
        crate::model::practice_state::snapshot(Some(Drill::GuardEdge.game()), result, [0; 8], &mut wire);
        for line in ["game=guard_edge", "phase=result", "last.outcome=secured", "score=1", "streak=1", "best=1"] {
            assert!(wire.lines().any(|candidate| candidate == line));
        }
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.practice_fields(None, Some(1)).phase, "done");
    }

    #[test]
    fn drill_outcome_stays_between_attempts_and_clears_on_restart() {
        let mut session = release(Drill::GuardEdge, None);
        session.advance_attempt(Observation { dead_or_rebirthing: true, ..Observation::default() });
        assert_eq!(session.phase, Phase::Result(Outcome::Secured, RESULT_FRAMES));
        assert_eq!(session.practice_fields(None, None).last_outcome, "secured");
        for _ in 0..RESULT_FRAMES {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Place);
        assert_eq!(session.practice_fields(None, None).last_outcome, "secured");
        session.placed();
        assert!(matches!(session.phase, Phase::Ready(_)));
        assert_eq!(session.practice_fields(None, None).last_outcome, "secured");
        for _ in 0..Drill::GuardEdge.ready_frames() {
            session.advance_attempt(Observation::default());
        }
        assert_eq!(session.phase, Phase::Live);
        assert_eq!(session.practice_fields(None, None).last_outcome, "secured");
        session.discard_attempt();
        assert_eq!(session.phase, Phase::Place);
        assert_eq!(session.practice_fields(None, None).last_outcome, "secured");
        session.restart_drill();
        assert_eq!(session.practice_fields(None, None).last_outcome, "");
        assert_eq!(session.practice_fields(None, None).attempts, Some(0));
        assert_eq!(Session::new(Drill::GuardEdge, None).practice_fields(None, None).last_outcome, "");

        let mut unresolved = release(Drill::GuardEdge, None);
        for _ in 0..EDGE_TIMEOUT {
            unresolved.advance_attempt(Observation::default());
        }
        assert_eq!(unresolved.phase, Phase::Result(Outcome::Unresolved, RESULT_FRAMES));
        assert_eq!(unresolved.practice_fields(None, None).last_outcome, "unresolved");
        for _ in 0..RESULT_FRAMES {
            unresolved.advance_attempt(Observation::default());
        }
        assert_eq!(unresolved.practice_fields(None, None).last_outcome, "unresolved");
        assert_eq!(unresolved.practice_fields(None, None).attempts, Some(0));
    }

    #[test]
    fn juggler_publishes_percent_preview_and_airtime_result() {
        let stage = Stage::new(
            [-160.0, 150.0, 110.0, -85.0],
            [-160.0, 150.0, 110.0, -85.0],
            [[-60.0, 5.0], [80.0, 8.0]],
        ).unwrap();
        let mut wizard = Wizard::new(Drill::Juggler, stage, [0.0, 7.0], 60, (0, 0), None).unwrap();
        let mut session = release(Drill::Juggler, Some(1));
        assert_eq!(session.practice_fields(Some(wizard), None).wizard_step, "cpu");
        wizard.move_cpu(stage, [-150.0, 90.0]);
        wizard.advance();
        wizard.change_percent(10);
        let percent = session.practice_fields(Some(wizard), None);
        assert_eq!((percent.wizard_step, percent.wizard_value), ("percent", Some(70)));
        wizard.advance();
        assert_eq!(session.practice_fields(Some(wizard), None).wizard_step, "player");
        wizard.advance();
        let preview = session.practice_fields(Some(wizard), None);
        assert_eq!((preview.wizard_step, preview.wizard_value), ("preview", Some(70)));
        assert_eq!((preview.wizard_point, preview.wizard_player_point), (Some([-150.0, 90.0]), Some([0.0, 7.0])));

        session.advance_attempt(Observation { hit: true, ..Observation::default() });
        session.advance_attempt(Observation { grounded: true, ..Observation::default() });
        let result = session.practice_fields(None, Some(2));
        assert_eq!((result.phase, result.last_outcome), ("result", "landed"));
        assert_eq!((result.attempt_frames, result.hits, result.best), (Some(2), Some(1), Some(2)));
        let mut wire = String::new();
        crate::model::practice_state::snapshot(Some(Drill::Juggler.game()), result, [0; 8], &mut wire);
        for line in ["game=juggler", "phase=result", "last.outcome=landed", "attempt.frames=2", "hits=1", "best=2"] {
            assert!(wire.lines().any(|candidate| candidate == line));
        }
    }

    #[test]
    fn cpu_behavior_returns_to_its_saved_value_after_a_load() {
        let original = CpuBehavior { menu: 0, level: 3 };
        let changed_in_pause = CpuBehavior { menu: 4, level: 3 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(Some(9), 1, original, true), Some(CpuBehavior::stock(9)));
        assert_eq!(lease.step(Some(9), 1, CpuBehavior::stock(9), true), None);
        assert_eq!(lease.step(Some(9), 1, changed_in_pause, true), Some(CpuBehavior::stock(9)));
        assert_eq!(lease.step(None, 1, CpuBehavior::stock(9), true), Some(original));
        assert_eq!(lease.step(None, 1, original, true), None);
    }

    #[test]
    fn cpu_behavior_control_restore_waits_for_stock_apply() {
        let control = CpuBehavior { menu: 8, level: 2 };
        let cpu = CpuBehavior::stock(9);
        let mut lease = BehaviorLease::new();
        assert!(control.applied(0x24, 0x24, false));
        assert_eq!(lease.step(Some(9), 1, control, true), Some(cpu));
        assert_eq!(lease.step(Some(9), 1, cpu, cpu.applied(0, 0, false)), Some(cpu));
        assert_eq!(lease.step(Some(9), 1, cpu, cpu.applied(0, 0, true)), None);
        assert_eq!(lease.step(None, 1, cpu, true), Some(control));
        assert!(lease.held(), "the restore stays owned until stock acknowledges Control");
        assert_eq!(lease.step(None, 1, control, control.applied(0x24, 0x24, true)), Some(control));
        assert_eq!(lease.step(None, 1, control, control.applied(0x24, 0x24, false)), None);
        assert!(!lease.held());
    }

    #[test]
    fn cpu_behavior_does_not_write_the_old_settings_to_a_replaced_panel() {
        let a = CpuBehavior { menu: 0, level: 2 };
        let b = CpuBehavior { menu: 5, level: 4 };
        let mut lease = BehaviorLease::new();
        assert_eq!(lease.step(Some(9), 1, a, true), Some(CpuBehavior::stock(9)));
        assert_eq!(lease.step(None, 2, b, true), None);
        assert_eq!(lease.step(Some(9), 2, b, true), Some(CpuBehavior::stock(9)));
        assert_eq!(lease.step(None, 2, CpuBehavior::stock(9), true), Some(b));
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
    fn the_pointer_steps_are_the_ones_that_show_the_cpu_coordinates() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let steps = |drill| {
            let mut wizard = Wizard::new(drill, stage, [0.0; 2], 100, (0, 0), None).unwrap();
            [(); 4].map(|()| {
                let seen = (wizard.step, wizard.shows_cpu());
                wizard.advance();
                seen
            })
        };
        assert_eq!(steps(Drill::GuardEdge), [
            (WizardStep::Area, true), (WizardStep::Cpu, true), (WizardStep::Player, false), (WizardStep::Preview, true),
        ]);
        assert_eq!(steps(Drill::Juggler), [
            (WizardStep::Cpu, true), (WizardStep::Percent, false), (WizardStep::Player, false), (WizardStep::Preview, true),
        ]);
    }

    #[test]
    fn a_moved_start_stays_as_set_even_back_on_its_default() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let key = (0, 0);
        let fresh = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, None).unwrap();
        let diddy = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(perfect::diddy::KIND)).unwrap();
        let default = fresh.setup.cpu;
        assert_ne!(default, diddy.setup.cpu);

        let mut a = fresh;
        a.move_cpu(stage, default);
        a.move_cpu(stage, [f32::NAN, 0.0]);
        assert_eq!(a, fresh);
        assert!(a.unmoved());

        a.move_cpu(stage, [default[0] + 2.0, default[1]]);
        assert_ne!(a.setup.cpu, default);
        a.move_cpu(stage, default);
        assert_eq!(a.setup.cpu, default);
        assert!(!a.unmoved());
        assert_eq!(diddy.remembered(stage, a.setup, a.unmoved()).cpu, default);

        let mut b = a;
        b.change_area(stage, 1);
        assert!(b.unmoved());

        let mut c = a;
        c.step = WizardStep::Cpu;
        c.back_to_defaults(fresh.setup);
        assert!(c.unmoved());
        assert_eq!((c.setup, c.step), (fresh.setup, WizardStep::Preview));
        assert_eq!(diddy.remembered(stage, c.setup, c.unmoved()).cpu, diddy.setup.cpu);
    }

    #[test]
    fn unmoved_remembered_start_follows_the_current_fighter() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let key = (0, 0);
        let mut a = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, None).unwrap();
        let diddy = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(perfect::diddy::KIND)).unwrap();
        assert_eq!(diddy.setup.cpu, [-94.0, -30.0]);
        assert_ne!(a.setup.cpu, diddy.setup.cpu);
        assert_eq!(diddy.remembered(stage, a.setup, true).cpu, diddy.setup.cpu);
        assert!(a.unmoved());
        assert_eq!(diddy.remembered(stage, a.setup, false).cpu, a.setup.cpu);
        a.move_cpu(stage, [-100.0, -20.0]);
        assert!(!a.unmoved());
        a.change_area(stage, 2);
        assert_eq!(a.setup.area, Area::UpperLeft);
        assert!(a.unmoved());
        let mut upper_diddy = diddy;
        upper_diddy.change_area(stage, 2);
        assert_eq!(diddy.remembered(stage, a.setup, true).cpu, upper_diddy.setup.cpu);
        let juggler = Wizard::new(Drill::Juggler, stage, [0.0; 2], 100, key, None).unwrap();
        assert!(!juggler.unmoved());

        let lucario_100 = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(perfect::lucario::KIND)).unwrap();
        let lucario_130 = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 130, key, Some(perfect::lucario::KIND)).unwrap();
        assert!(lucario_100.unmoved());
        assert_ne!(lucario_100.setup.cpu, lucario_130.setup.cpu);
        assert_eq!(lucario_130.remembered(stage, lucario_100.setup, true).cpu, lucario_130.setup.cpu);
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
    fn link_starts_facing_away_on_each_proven_side() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let left = Setup { area: Area::LowerLeft, cpu: [-102.0, -2.0], p1: [0.0, 0.0], percent: 100 };
        let right = Setup { area: Area::UpperRight, cpu: [118.0, 38.0], percent: 105, ..left };
        let facing = |drill, stage_key, start| {
            recovery_away_facing(drill, stage_key, stage, start, Some(perfect::link::KIND))
        };
        assert_eq!(facing(Drill::GuardEdge, (3, 0), left), Some(-1.0));
        assert_eq!(facing(Drill::GuardEdge, (3, 0), right), Some(1.0));
        for key in [(3, 0), (0, 0), (311, 0)] {
            assert_eq!(facing(Drill::GuardEdge, key, right), Some(1.0), "{key:?}");
        }
        assert_eq!(facing(Drill::GuardEdge, (3, 1), right), None);
        assert_eq!(facing(Drill::Juggler, (3, 0), right), None);
    }

    #[test]
    fn pyra_wizard_offers_the_low_jump_first_reference_from_both_difficulties() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(
                Drill::GuardEdge, stage, [-75.0, 0.0], 60, key, Some(perfect::pyra::PYRA),
            ).unwrap();
            assert_eq!(wizard.setup.cpu, [-184.0, -50.0], "{key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerRight).unwrap());
        }
        for (stage_key, fighter) in [((44, 0), perfect::pyra::PYRA), ((3, 0), perfect::pyra::MYTHRA)] {
            let wizard = Wizard::new(
                Drill::GuardEdge, stage, [0.0; 2], 60, stage_key, Some(fighter),
            ).unwrap();
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::LowerLeft).unwrap());
        }
    }

    #[test]
    fn steve_wizard_places_the_left_elytra_release_in_a_flown_column() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 60, key, Some(0x58)).unwrap();
            wizard.change_area(stage, 2);
            assert_eq!(wizard.setup.cpu, [-98.0, 30.0], "native stage {key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperRight).unwrap());
        }
    }

    #[test]
    fn incineroar_upper_left_wizard_offers_the_measured_long_whip_start() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 60, key, Some(0x47)).unwrap();
            wizard.change_area(stage, 2);
            assert_eq!(wizard.setup.cpu, [-190.0, 10.0], "native stage {key:?}");
            wizard.change_area(stage, 1);
            assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperRight).unwrap());
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
    fn snake_wizard_offers_the_mirror_cypher_start_only_on_final_destination() {
        let stage = Stage::new(
            [-240.0, 240.0, 180.0, -140.0], [-180.0, 180.0, 110.0, -85.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0), (4, 0), (3, 1)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [-76.0, 0.0], 100, key, Some(0x22)).unwrap();
            let left = if perfect::is_crossing_stage(key) { [-100.0, -4.0] }
                else { stage.area_default(Area::LowerLeft).unwrap() };
            assert_eq!(wizard.setup.cpu, left);
            assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)),
                perfect::is_crossing_stage(key).then_some(-1.0));
            assert_eq!(recovery_away_facing(Drill::Juggler, key, stage, wizard.setup, Some(0x22)), None);
            wizard.change_area(stage, 1);
            let right = if key == (3, 0) { [100.0, -4.0] }
                else { stage.area_default(Area::LowerRight).unwrap() };
            assert_eq!(wizard.setup.cpu, right);
            assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)),
                (key == (3, 0)).then_some(1.0));
            wizard.change_area(stage, -1);
            assert_eq!(wizard.setup.cpu, left);
        }
    }

    #[test]
    fn link_wizard_places_the_proven_bomb_starts_in_both_difficulties() {
        let stage = Stage::new(
            [-240.0, 240.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-79.99, 0.0], [79.99, 0.0]],
        ).unwrap();
        for key in [(0, 0), (3, 0), (311, 0), (0, 1), (89, 0)] {
            let mut wizard = Wizard::new(Drill::GuardEdge, stage, [-76.0, 0.0], 100, key, Some(0x2)).unwrap();
            wizard.change_area(stage, 2);
            if perfect::is_crossing_stage(key) {
                assert_eq!(wizard.setup.cpu, [-109.99, 18.0]);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)), Some(-1.0));
                assert_eq!(recovery_away_facing(Drill::Juggler, key, stage, wizard.setup, Some(0x2)), None);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, Setup { percent: 120, ..wizard.setup }, Some(0x2)), None);
            } else {
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperLeft).unwrap());
            }
            wizard.change_area(stage, 1);
            if perfect::is_crossing_stage(key) {
                assert_eq!(wizard.setup.cpu, [117.99, 38.0]);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)), Some(1.0));
            } else {
                assert_eq!(wizard.setup.cpu, stage.area_default(Area::UpperRight).unwrap());
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)), None);
            }
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
