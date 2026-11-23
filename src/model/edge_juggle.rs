//! Shared attempt clock and scoring for Guard the Edge and Juggler (DESIGN v2).
use super::btt;
use super::crosshair::{Crosshair, Marks};
use super::panes::EGG_WHOLE;
use crate::model::practice_state::Fields;
use super::practice_games::{record_max, record_time, result_tick, Game};

#[path = "edge_juggle/perfect.rs"]
pub mod perfect;
#[path = "edge_juggle/picker.rs"]
pub mod picker;
#[path = "edge_juggle/zoom.rs"]
pub mod zoom;
#[cfg(test)]
#[path = "edge_juggle/crosshair_tests.rs"]
mod crosshair_tests;

const EDGE_TIMEOUT: u32 = 15 * 60;
const RESULT_FRAMES: u16 = 90;

/// The game does not hold a fighter on a floor's last point; see docs/design/gj4-player-start.md.
pub const PLAYER_INSET: f32 = 1.0;

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

    /// The control type the drill sets itself with the pause menu closed: the wanted selection and the live fighter
    /// are both AI kinds (not Control) and the operator flag already says CPU, so the game's own apply would change
    /// only the type. `None`: the stock pause apply has to make the change.
    pub fn direct(self, entry: u8, ai: u8, operation_cpu: bool, menu: BehaviorMenu, pause: BehaviorPause) -> Option<u8> {
        let control = self.control_type()?;
        (operation_cpu && menu == BehaviorMenu::Closed && pause == BehaviorPause::Idle
            && ![control, entry, ai].contains(&0x24)).then_some(control)
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

/// The waiting room has no pause panel: the first training partner's control type is the room's Training Partner
/// Behavior. Save the room's type once, ask for the stock CPU (type 0), give the saved type back at Stop.
#[derive(Default)]
pub struct PartnerLease(Option<(usize, u8)>);

impl PartnerLease {
    pub const fn new() -> Self { Self(None) }
    pub fn held(&self) -> bool { self.0.is_some() }

    /// `owner`: the partner's controller; `entry` and `ai`: its two live control types. The type to write now, if any.
    /// A Control type (0x24) or a fighter no CPU operates is never written.
    pub fn step(&mut self, level: Option<u8>, owner: usize, entry: u8, ai: u8, operation_cpu: bool) -> Option<u8> {
        if self.0.is_some_and(|(old, _)| old != owner) {
            self.0 = None;
        }
        if !operation_cpu || entry == 0x24 || ai == 0x24 {
            return None;
        }
        if level.is_some() {
            match &mut self.0 {
                None => self.0 = Some((owner, entry)),
                Some((_, saved)) if entry != 0 => *saved = entry,
                Some(_) => {}
            }
            return (entry != 0 || ai != 0).then_some(0);
        }
        let (_, saved) = self.0?;
        if entry == saved && ai == saved {
            self.0 = None;
            return None;
        }
        Some(saved)
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
    /// Player 1 is KO'd: Dead, Standby or Rebirth.
    pub player_ko: bool,
    pub grounded: bool,
    pub airborne: bool,
    pub wait: bool,
    pub fall: bool,
}

impl Placement {
    pub fn action(self, frame: PlacementFrame) -> PlacementAction {
        use PlacementAction as A;
        if frame.dead_or_rebirthing || frame.player_ko {
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

/// CPU settings affect both registered bodies of the scene's primary opposed entry.
pub const fn suspends_cpu_settings(active: bool, key: crate::model::cpu::role::FighterKey, primary: u8) -> bool {
    active && key.entry == primary
}

/// Runtime admits placement, then ticks only on eligible simulation frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Session {
    drill: Drill,
    limit: Option<u16>,
    last: Option<Outcome>,
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
            last: None,
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
        self.last = Some(outcome);
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
            let (step, point, player, value) = match wizard.step {
                WizardStep::Cpu => ("cpu", Some(wizard.setup.cpu), None, None),
                WizardStep::Percent => ("percent", None, None, Some(u32::from(wizard.setup.percent))),
                WizardStep::Player => ("player", Some(wizard.setup.p1), None, None),
                WizardStep::Preview => (
                    "preview",
                    Some(wizard.setup.cpu),
                    Some(wizard.setup.p1),
                    Some(u32::from(wizard.setup.percent)),
                ),
            };
            return Fields {
                phase: "wizard",
                wizard_step: step,
                wizard_point: point,
                wizard_player_point: player,
                wizard_value: value,
                ..Fields::default()
            };
        }

        let (phase, remaining_frames) = match self.phase {
            Phase::Place => ("place", None),
            Phase::Ready(left) => ("ready", Some(u32::from(left))),
            Phase::Live => ("live", None),
            Phase::Result(_, left) => ("result", Some(u32::from(left))),
            Phase::Done => ("done", None),
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
            last_outcome: self.last.map_or("", |outcome| outcome.bridge_name()),
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

    pub fn side(self, x: f32) -> usize {
        usize::from(x > self.ledges[0][0].midpoint(self.ledges[1][0]))
    }

    pub fn edge_cursor(self, point: [f32; 2]) -> Option<[f32; 2]> {
        let [left, right, top, bottom] = self.blast;
        clamp_point(point, [left + 1.0, right - 1.0, top - 1.0, bottom + 1.0])
    }

    pub fn off_stage(self, point: [f32; 2]) -> Option<[f32; 2]> {
        (self.edge_cursor(point) == Some(point)
            && (point[0] <= self.ledges[0][0] - 1.0 || point[0] >= self.ledges[1][0] + 1.0))
            .then_some(point)
    }

    fn lower_middle(self, side: usize) -> Option<[f32; 2]> {
        let [left, right, _, bottom] = self.blast;
        let [x, y] = self.ledges[side];
        let [lo, hi] = if side == 0 {
            [left + 1.0, x - 1.0]
        } else {
            [x + 1.0, right - 1.0]
        };
        self.off_stage([lo.midpoint(hi), (bottom + 1.0).midpoint(y)])
    }

    pub fn bucket(self, point: [f32; 2]) -> u8 {
        let side = self.side(point[0]);
        side as u8 + 2 * u8::from(point[1] >= self.ledges[side][1])
    }

    pub fn camera_point(self, point: [f32; 2]) -> Option<[f32; 2]> {
        clamp_point(point, self.camera)
    }

    pub fn player_x(self, drill: Drill, x: f32) -> f32 {
        if drill == Drill::Juggler {
            return x;
        }
        let lo = self.ledges[0][0] + PLAYER_INSET;
        let hi = self.ledges[1][0] - PLAYER_INSET;
        if lo <= hi {
            x.clamp(lo, hi)
        } else {
            self.ledges[0][0].midpoint(self.ledges[1][0])
        }
    }

    pub fn player_cursor(self, drill: Drill, point: [f32; 2]) -> Option<[f32; 2]> {
        let [x, y] = self.camera_point(point)?;
        let x = self.player_x(drill, x);
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
    Cpu,
    Percent,
    Player,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wizard {
    drill: Drill,
    area_offset: Option<[f32; 2]>,
    left_start: Option<[f32; 2]>,
    moved: bool,
    cross: Crosshair,
    pub step: WizardStep,
    pub setup: Setup,
}

impl Wizard {
    pub fn new(
        drill: Drill, stage: Stage, p1: [f32; 2], percent: u16,
        stage_key: (i32, u32), fighter: Option<i32>,
    ) -> Option<Self> {
        let link_start = drill == Drill::GuardEdge
            && fighter == Some(perfect::link::KIND)
            && perfect::link::in_region(stage_key, stage.ledges, [stage.ledges[0][0] - 30.0, stage.ledges[0][1] + 18.0], percent.into());
        let lucario_left_start = if drill == Drill::GuardEdge && fighter == Some(perfect::lucario::KIND) {
            perfect::lucario::wizard_offset(stage_key, percent)
        } else {
            None
        };
        let left_start = if let Some(offset) = lucario_left_start {
            Some(offset)
        } else if link_start {
            Some([30.0, 18.0])
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::greninja::KIND) {
            perfect::greninja::wizard_offset(stage_key)
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::mario::KIND)
            && perfect::mario::reference_stage(stage_key)
        {
            Some([34.0, -46.0])
        } else if drill == Drill::GuardEdge && fighter == Some(perfect::snake::KIND)
            && perfect::is_crossing_stage(stage_key)
        {
            Some([20.0, -4.0])
        } else if drill == Drill::GuardEdge && perfect::is_crossing_stage(stage_key) {
            match fighter {
                Some(perfect::ness::KIND) => perfect::ness::wizard_offset(stage_key),
                Some(perfect::lucas::KIND) if perfect::lucas::reference_stage(stage_key) => {
                    Some(perfect::lucas::WIZARD_OFFSET)
                }
                Some(perfect::sonic::KIND) => Some(perfect::sonic::WIZARD_OFFSET),
                Some(perfect::hero::KIND) if perfect::hero::reference_stage(stage_key) => {
                    Some(perfect::hero::WIZARD_OFFSET)
                }
                Some(perfect::steve::KIND) => Some([18.0, 30.0]),
                Some(perfect::pyra::PYRA) => Some([104.0, -50.0]),
                Some(perfect::incineroar::KIND) => Some([110.0, 10.0]),
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
        let cpu = stage.default_juggle(p1);
        let mut wizard = Self {
            drill,
            area_offset,
            left_start,
            moved: false,
            cross: Crosshair::new(cpu),
            step: WizardStep::Cpu,
            setup: Setup {
                cpu,
                p1: stage.player_cursor(drill, p1)?,
                percent: percent.min(300),
            },
        };
        if drill == Drill::GuardEdge {
            wizard.setup.cpu = wizard.default_cpu(stage, 0)?;
            wizard.cross.jump(wizard.setup.cpu);
        }
        Some(wizard)
    }

    pub fn default_cpu(&self, stage: Stage, side: usize) -> Option<[f32; 2]> {
        let offset = if side == 0 { self.left_start.or(self.area_offset) } else { self.area_offset };
        if let Some([outside, height]) = offset {
            let ledge = stage.ledges[side];
            let x = if side == 0 { ledge[0] - outside } else { ledge[0] + outside };
            let point = [x, ledge[1] + height];
            if stage.off_stage(point) == Some(point) {
                return Some(point);
            }
        }
        stage.lower_middle(side)
    }

    pub fn unmoved(&self) -> bool {
        self.drill == Drill::GuardEdge && !self.moved
    }

    pub fn cursor(&self) -> [f32; 2] {
        self.cross.at
    }

    pub fn marks(&self) -> Marks {
        let mut looks = [None; btt::LOOKS];
        let pointer = match self.step {
            WizardStep::Cpu => {
                looks[btt::LOOKS - 1] = Some(self.cross.look());
                Some(self.setup.cpu)
            }
            WizardStep::Percent => {
                looks[0] = Some((self.setup.cpu, EGG_WHOLE));
                None
            }
            WizardStep::Player | WizardStep::Preview => {
                looks[0] = Some((self.setup.cpu, EGG_WHOLE));
                looks[btt::LOOKS - 1] = Some(Crosshair::new(self.setup.p1).look());
                (self.step == WizardStep::Preview).then_some(self.setup.cpu)
            }
        };
        Marks { looks, pointer }
    }

    pub fn back_to_defaults(&mut self, defaults: Setup) {
        self.setup = defaults;
        self.moved = false;
        self.step = WizardStep::Preview;
        self.jump_page();
    }

    pub fn remembered(&self, stage: Stage, mut setup: Setup, unmoved: bool) -> Setup {
        if unmoved {
            if let Some(cpu) = self.default_cpu(stage, stage.side(setup.cpu[0])) {
                setup.cpu = cpu;
            }
        }
        setup
    }

    pub fn set_side_default(&mut self, stage: Stage, other: bool) {
        if self.drill != Drill::GuardEdge || self.step != WizardStep::Cpu {
            return;
        }
        let side = stage.side(self.setup.cpu[0]) ^ usize::from(other);
        if let Some(cpu) = self.default_cpu(stage, side) {
            self.setup.cpu = cpu;
            self.moved = false;
            self.cross.jump(cpu);
        }
    }

    pub fn steer(&mut self, stage: Stage, pad: &btt::Pad) {
        match self.step {
            WizardStep::Cpu => {
                let changed = self.cross.step(pad, |p| match self.drill {
                    Drill::GuardEdge => stage.edge_cursor(p),
                    Drill::Juggler => stage.camera_point(p),
                });
                if changed {
                    self.setup.cpu = self.cross.at;
                    self.moved = true;
                }
            }
            WizardStep::Player => {
                self.cross.step(pad, |p| stage.player_cursor(self.drill, p));
            }
            _ => {}
        }
    }

    pub fn accepts_cpu(&self, stage: Stage) -> bool {
        self.drill == Drill::Juggler || stage.off_stage(self.setup.cpu).is_some()
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
            WizardStep::Cpu if self.drill == Drill::Juggler => WizardStep::Percent,
            WizardStep::Cpu | WizardStep::Percent => WizardStep::Player,
            WizardStep::Player | WizardStep::Preview => WizardStep::Preview,
        };
        self.jump_page();
    }

    /// False means B on the first step cancels the wizard.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            WizardStep::Cpu => return false,
            WizardStep::Percent => WizardStep::Cpu,
            WizardStep::Player if self.drill == Drill::Juggler => WizardStep::Percent,
            WizardStep::Player => WizardStep::Cpu,
            WizardStep::Preview => WizardStep::Player,
        };
        self.jump_page();
        true
    }

    fn jump_page(&mut self) {
        self.cross.jump(match self.step {
            WizardStep::Cpu | WizardStep::Percent => self.setup.cpu,
            WizardStep::Player | WizardStep::Preview => self.setup.p1,
        });
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn edge_wizard_opens_on_the_left_default() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let fresh = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), None).unwrap();
        assert_eq!(fresh.step, WizardStep::Cpu);
        assert_eq!(fresh.setup.cpu, [-155.0, -69.5]);
        let diddy = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(perfect::diddy::KIND)).unwrap();
        assert_eq!(diddy.setup.cpu, [-94.0, -30.0]);
        let link = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(perfect::link::KIND)).unwrap();
        assert_eq!(link.setup.cpu, [-110.0, 18.0]);
        let lucas = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(perfect::lucas::KIND)).unwrap();
        assert_eq!(lucas.setup.cpu, [-98.0, 2.0]);
        let sonic = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (3, 0), Some(perfect::sonic::KIND)).unwrap();
        assert_eq!(sonic.setup.cpu, [-106.0, 34.0]);
    }

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
    fn placement_waits_for_a_knocked_out_player() {
        let cases = [
            (Placement::ResetResources, PlacementFrame::default(), PlacementAction::ResetResources),
            (Placement::Grounded, PlacementFrame { grounded: true, wait: true, ..PlacementFrame::default() },
             PlacementAction::PlaceStart),
            (Placement::Airborne, PlacementFrame { airborne: true, fall: true, ..PlacementFrame::default() },
             PlacementAction::HoldStart),
        ];
        for (placement, frame, normal) in cases {
            assert_eq!(placement.action(PlacementFrame { player_ko: true, ..frame }), PlacementAction::RestartPlacement);
            assert_eq!(placement.action(PlacementFrame { player_ko: false, ..frame }), normal);
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
        let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 7.0], 100, (0, 0), None).unwrap();
        let mut session = Session::new(Drill::GuardEdge, Some(1));
        let fields = session.practice_fields(Some(wizard), None);
        assert_eq!((fields.phase, fields.wizard_step), ("wizard", "cpu"));
        assert_eq!(fields.wizard_point, Some(wizard.setup.cpu));
        let mut wire = String::new();
        crate::model::practice_state::snapshot(Some(Drill::GuardEdge.game()), fields, [0; 8], &mut wire);
        assert!(wire.contains("phase=wizard\nwizard.step=cpu\n"));

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

        let mut juggler = release(Drill::Juggler, None);
        juggler.advance_attempt(Observation { grounded: true, ..Observation::default() });
        for _ in 0..RESULT_FRAMES {
            juggler.advance_attempt(Observation::default());
        }
        assert_eq!(juggler.practice_fields(None, None).last_outcome, "landed");
        juggler.restart_drill();
        assert_eq!(juggler.practice_fields(None, None).last_outcome, "");
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
        let initial = wizard.cursor();
        wizard.steer(stage, &btt::Pad { stick: [-1.0, 1.0], ..btt::Pad::default() });
        let moved = wizard.cursor();
        assert_ne!(moved, initial);
        wizard.advance();
        wizard.change_percent(10);
        let percent = session.practice_fields(Some(wizard), None);
        assert_eq!((percent.wizard_step, percent.wizard_value), ("percent", Some(70)));
        wizard.advance();
        assert_eq!(session.practice_fields(Some(wizard), None).wizard_step, "player");
        wizard.advance();
        let preview = session.practice_fields(Some(wizard), None);
        assert_eq!((preview.wizard_step, preview.wizard_value), ("preview", Some(70)));
        assert_eq!((preview.wizard_point, preview.wizard_player_point), (Some(moved), Some([0.0, 7.0])));

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
    fn drill_behavior_between_ai_kinds_is_set_directly() {
        use BehaviorMenu::Closed;
        let nine = CpuBehavior::stock(9);
        for menu in 0..8u32 {
            let from = CpuBehavior { menu, level: 0 };
            let kind = from.control_type().unwrap();
            assert_eq!(nine.direct(kind, kind, true, Closed, BehaviorPause::Idle), Some(0), "start from menu {menu}");
            assert_eq!(from.direct(0, 0, true, Closed, BehaviorPause::Idle), Some(kind), "restore to menu {menu}");
        }
        // A half-applied type (the entry's copy and the AI disagree) is still between AI kinds.
        assert_eq!(nine.direct(1, 5, true, Closed, BehaviorPause::Idle), Some(0));
    }

    #[test]
    fn control_or_an_open_menu_takes_the_stock_pause_apply() {
        use BehaviorMenu::{Closed, Open, Transition};
        let (nine, control, idle) = (CpuBehavior::stock(9), CpuBehavior { menu: 8, level: 2 }, BehaviorPause::Idle);
        assert_eq!(control.direct(0, 0, true, Closed, idle), None, "to Control");
        assert_eq!(nine.direct(0x24, 0x24, false, Closed, idle), None, "from Control");
        assert_eq!(nine.direct(0x24, 1, true, Closed, idle), None);
        assert_eq!(nine.direct(1, 0x24, true, Closed, idle), None);
        assert_eq!(nine.direct(1, 1, false, Closed, idle), None, "the operator flag is not CPU");
        assert_eq!(nine.direct(1, 1, true, Open, idle), None, "the player's own pause applies it on resume");
        assert_eq!(nine.direct(1, 1, true, Transition, idle), None);
        assert_eq!(nine.direct(1, 1, true, Closed, BehaviorPause::Opening), None, "a pause transaction is in flight");
        assert_eq!(nine.direct(1, 1, true, Closed, BehaviorPause::Closing), None);
        assert_eq!(CpuBehavior { menu: 9, level: 0 }.direct(1, 1, true, Closed, idle), None, "not a menu value");
    }

    #[test]
    fn the_waiting_room_partner_is_a_stock_cpu_for_the_drill_and_gets_its_type_back() {
        struct Game { owner: usize, entry: u8, ai: u8, cpu: bool }
        fn poll(game: &mut Game, lease: &mut PartnerLease, level: Option<u8>) -> Option<u8> {
            let write = lease.step(level, game.owner, game.entry, game.ai, game.cpu);
            if let Some(control) = write {
                game.entry = control;
                game.ai = control;
            }
            write
        }

        let mut game = Game { owner: 1, entry: 7, ai: 7, cpu: true };
        let mut lease = PartnerLease::new();
        assert_eq!(poll(&mut game, &mut lease, Some(9)), Some(0));
        assert_eq!(poll(&mut game, &mut lease, Some(9)), None);
        assert!(lease.held());
        assert_eq!(poll(&mut game, &mut lease, None), Some(7));
        assert_eq!(poll(&mut game, &mut lease, None), None);
        assert!(!lease.held());

        assert_eq!(poll(&mut game, &mut lease, Some(9)), Some(0));
        game.entry = 1;
        game.ai = 1;
        assert_eq!(poll(&mut game, &mut lease, Some(9)), Some(0));
        assert_eq!(poll(&mut game, &mut lease, None), Some(1));
        assert_eq!(poll(&mut game, &mut lease, None), None);
        assert!(!lease.held());

        game.entry = 0;
        game.ai = 0;
        assert_eq!(poll(&mut game, &mut lease, Some(9)), None);
        assert!(lease.held());
        assert_eq!(poll(&mut game, &mut lease, None), None);
        assert!(!lease.held());

        game.entry = 7;
        game.ai = 7;
        assert_eq!(poll(&mut game, &mut lease, Some(9)), Some(0));
        game.ai = 7;
        assert_eq!(poll(&mut game, &mut lease, Some(9)), Some(0));
        assert_eq!(poll(&mut game, &mut lease, None), Some(7));
        assert_eq!(poll(&mut game, &mut lease, None), None);

        for (entry, ai, cpu) in [(7, 7, false), (0x24, 7, true), (7, 0x24, true)] {
            let mut game = Game { owner: 1, entry, ai, cpu };
            let mut lease = PartnerLease::new();
            assert_eq!(poll(&mut game, &mut lease, Some(9)), None);
            assert!(!lease.held());
            assert_eq!(poll(&mut game, &mut lease, None), None);

            let mut held = PartnerLease::new();
            assert_eq!(held.step(Some(9), 1, 7, 7, true), Some(0));
            assert_eq!(held.step(None, 1, entry, ai, cpu), None);
            assert!(held.held());
        }
        let mut lease = PartnerLease::new();
        assert_eq!(lease.step(Some(9), 1, 7, 7, true), Some(0));
        assert_eq!(lease.step(None, 2, 1, 1, true), None);
        assert!(!lease.held());
    }

    #[test]
    fn lobby_practice_off_gives_the_partner_back_first_and_a_departure_writes_nothing() {
        use crate::model::lobby_flag::{self, FlagClock, Scene};
        use crate::model::edge_juggle::picker::{self, Settled, Tick};
        let room = Scene { initialized: true, melee: 0x0d, mode: 0x0701_0100 };
        let battle = Scene { initialized: true, melee: 0x0e, mode: 0x0701_0101 };
        let ready = || FlagClock { last_key: lobby_flag::scene_key(room), ui_ready: true, was_active: true };
        let held = || {
            let mut lease = PartnerLease::new();
            assert_eq!(lease.step(Some(9), 1, 7, 7, true), Some(0));
            lease
        };
        let mut clock = ready();
        let mut lease = held();
        let mut entry = 0;
        let mut ai = 0;
        let mut original = Some(10);
        let mut observed = Some(20);
        let mut selected = Some(20);
        let mut settled = Settled::new(selected);
        let mut releasing = false;
        let mut stops = 0;
        let mut behavior_writes = Vec::new();
        let mut fighter_requests = Vec::new();
        let mut ended = false;
        for tick in 0..10 {
            let holds = selected.is_some() || lease.held() || original.is_some();
            let (on, next_releasing, stop) = lobby_flag::release(false, clock.was_active, holds, releasing);
            releasing = next_releasing;
            let (next, decision) = lobby_flag::step(clock, Some(room), on);
            clock = next;
            if !decision.active {
                assert!(!holds && !stop);
                ended = true;
                break;
            }
            if stop {
                stops += 1;
                assert_eq!(tick, 0);
                selected = None;
            }
            if let Some(value) = lease.step(selected.map(|_| 9), 1, entry, ai, true) {
                behavior_writes.push(value);
                entry = value;
                ai = value;
            }
            let (next, tick) = picker::tick(settled, selected, original, true, || false, || observed);
            settled = next;
            if let Tick::Serve { target, observed: seen, step } = tick {
                if step.request {
                    fighter_requests.push(target);
                    observed = Some(target);
                } else if seen.is_some() && selected.is_none() {
                    original = None;
                }
            }
        }
        assert!(ended);
        assert_eq!(stops, 1);
        assert_eq!(behavior_writes, [7]);
        assert!(!lease.held());
        assert_eq!(fighter_requests, [10]);
        assert_eq!(original, None);

        for (setting, releasing) in [(true, false), (false, true)] {
            let clock = ready();
            let mut lease = held();
            assert!(lease.held());
            let (on, _, _) = lobby_flag::release(setting, clock.was_active, true, releasing);
            let (_, decision) = lobby_flag::step(clock, Some(battle), on);
            assert!(!decision.active);
            lease = PartnerLease::new();
            assert!(!lease.held());
            let (_, tick) = picker::tick(Settled::new(Some(20)), None, Some(10), false, || false, || Some(20));
            assert_eq!(tick, Tick::Left);
        }
        for replaced in [false, true] {
            let mut blocked = held();
            let mut clock = ready();
            let mut releasing = false;
            for tick in 0..20_001 {
                let (on, next_releasing, stop) = lobby_flag::release(false, clock.was_active, blocked.held(), releasing);
                releasing = next_releasing;
                let (next, decision) = lobby_flag::step(clock, Some(room), on);
                clock = next;
                assert!(decision.active);
                assert_eq!(stop, tick == 0);
                assert_eq!(blocked.step(None, 1, 0, 0, false), None);
                assert!(blocked.held());
            }
            assert!(blocked.held());
            if replaced {
                assert_eq!(blocked.step(None, 2, 0, 0, true), None);
                assert!(!blocked.held());
                let (on, releasing, stop) = lobby_flag::release(false, clock.was_active, blocked.held(), releasing);
                assert_eq!((on, releasing, stop), (false, false, false));
                let (_, decision) = lobby_flag::step(clock, Some(room), on);
                assert!(!decision.active);
            } else {
                let (on, _, _) = lobby_flag::release(false, clock.was_active, blocked.held(), releasing);
                let (_, decision) = lobby_flag::step(clock, Some(battle), on);
                assert!(!decision.active);
                blocked = PartnerLease::new();
                assert!(!blocked.held());
            }
        }
    }

    #[test]
    fn a_drill_between_ai_kinds_never_asks_for_the_pause() {
        // The game as the drill's poll sees it, and one poll written the way `step_drill_panel` composes the model.
        struct Game { panel: CpuBehavior, entry: u8, ai: u8, cpu: bool }
        fn poll(game: &mut Game, lease: &mut BehaviorLease, pause: &mut BehaviorPause, level: Option<u8>) -> bool {
            let applied = game.panel.applied(game.entry, game.ai, game.cpu);
            let desired = lease.step(level, 7, game.panel, applied);
            let needs_stock = desired.is_some_and(|d| !applied || d.menu != game.panel.menu);
            if let Some((d, control)) = desired.filter(|_| needs_stock)
                .and_then(|d| Some((d, d.direct(game.entry, game.ai, game.cpu, BehaviorMenu::Closed, *pause)?)))
            {
                (game.ai, game.entry, game.panel) = (control, control, d);
                return false;
            }
            if let Some(d) = desired.filter(|_| !needs_stock) { game.panel = d; }
            pause.pulse(needs_stock, BehaviorMenu::Closed, true)
        }
        for menu in 0..8u32 {
            let original = CpuBehavior { menu, level: 2 };
            let kind = original.control_type().unwrap();
            let mut game = Game { panel: original, entry: kind, ai: kind, cpu: true };
            let (mut lease, mut pause) = (BehaviorLease::new(), BehaviorPause::Idle);
            for _ in 0..4 { assert!(!poll(&mut game, &mut lease, &mut pause, Some(9)), "start, menu {menu}"); }
            assert_eq!((game.panel, game.entry, game.ai), (CpuBehavior::stock(9), 0, 0), "menu {menu}");
            assert!(lease.held());
            for _ in 0..4 { assert!(!poll(&mut game, &mut lease, &mut pause, None), "stop, menu {menu}"); }
            assert_eq!((game.panel, game.entry, game.ai), (original, kind, kind), "menu {menu}");
            assert!(!lease.held());
        }
        // Control still asks for the stock pause.
        let mut game = Game { panel: CpuBehavior { menu: 8, level: 2 }, entry: 0x24, ai: 0x24, cpu: false };
        let (mut lease, mut pause) = (BehaviorLease::new(), BehaviorPause::Idle);
        assert!(poll(&mut game, &mut lease, &mut pause, Some(9)));
        assert_eq!((game.entry, game.ai), (0x24, 0x24), "nothing written outside the pause");
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
    fn edge_wizard_cursor_uses_the_blast_box() {
        let stage = Stage::new([-160.0, 150.0, 110.0, -85.0], [-160.0, 150.0, 110.0, -85.0], [[-60.0, 5.0], [80.0, 8.0]]).unwrap();
        for (start, expected) in [
            ([300.0, 70.0], [149.0, 70.0]),
            ([0.0, -100.0], [0.0, -84.0]),
            ([0.0, -20.0], [0.0, -20.0]),
            ([200.0, 180.0], [149.0, 109.0]),
        ] {
            assert_eq!(stage.edge_cursor(start), Some(expected));
        }
        assert_eq!(stage.edge_cursor([f32::NAN, 0.0]), None);
    }

    #[test]
    fn edge_player_cursor_reverses_immediately_after_the_yoshi_story_lip() {
        let stage = Stage::new(
            [-180.0, 180.0, 120.0, -90.0],
            [-160.0, 160.0, 118.0, -80.0],
            [[-68.0, -4.25], [68.0, -4.25]],
        ).unwrap();
        for side in [-1.0, 1.0] {
            let lip = [side * 67.0, -4.25];
            let outside = [side * 130.0, -4.25];
            let cursor = stage.player_cursor(Drill::GuardEdge, outside).unwrap();
            assert_eq!(
                stage.player_cursor(Drill::GuardEdge, [cursor[0] - side * 2.0, cursor[1]]),
                Some([side * 65.0, -4.25]),
            );
            assert_eq!(cursor, lip);
            assert_eq!(stage.player_cursor(Drill::Juggler, outside), Some(outside));
        }
    }

    #[test]
    fn guard_player_point_stops_one_unit_inside_the_floor_ends() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-79.9856, 0.0], [79.9856, 0.0]],
        ).unwrap();
        for side in [-1.0_f32, 1.0] {
            let outside = [side * 150.0, 4.0];
            let inside = [side * (79.9856_f32 - 1.5), 4.0];
            assert_eq!(stage.player_cursor(Drill::GuardEdge, outside), Some([side * (79.9856_f32 - 1.0), 4.0]));
            assert_eq!(stage.player_cursor(Drill::GuardEdge, inside), Some(inside));
            assert_eq!(stage.player_cursor(Drill::Juggler, outside), Some(outside));
        }
    }

    #[test]
    fn a_main_floor_narrower_than_two_units_keeps_the_player_on_its_middle() {
        let stage = Stage::new(
            [-10.0, 10.0, 10.0, -10.0], [-9.0, 9.0, 9.0, -9.0],
            [[-0.5, 0.0], [1.0, 0.0]],
        ).unwrap();
        for x in [-8.0, 0.0, 8.0] {
            assert_eq!(stage.player_cursor(Drill::GuardEdge, [x, 0.0]), Some([0.25, 0.0]));
            assert_eq!(stage.player_cursor(Drill::Juggler, [x, 0.0]), Some([x, 0.0]));
        }
        let exactly_two = Stage::new(
            [-10.0, 10.0, 10.0, -10.0], [-9.0, 9.0, 9.0, -9.0],
            [[-1.0, 0.0], [1.0, 0.0]],
        ).unwrap();
        assert_eq!(exactly_two.player_cursor(Drill::GuardEdge, [8.0, 0.0]), Some([0.0, 0.0]));
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
            assert!(suspends_cpu_settings(true, key, 1));
            assert!(!suspends_cpu_settings(false, key, 1));
        }
        for entry in [0, 2, 4] {
            assert!(!suspends_cpu_settings(true, FighterKey::new(entry, false).unwrap(), 1));
        }
        for partner in [false, true] {
            assert!(suspends_cpu_settings(true, FighterKey::new(4, partner).unwrap(), 4));
            assert!(!suspends_cpu_settings(false, FighterKey::new(4, partner).unwrap(), 4));
            assert!(!suspends_cpu_settings(true, FighterKey::new(1, partner).unwrap(), 4));
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
                let seen = (wizard.step, wizard.marks().pointer.is_some());
                wizard.advance();
                seen
            })
        };
        assert_eq!(steps(Drill::GuardEdge), [
            (WizardStep::Cpu, true), (WizardStep::Player, false), (WizardStep::Preview, true), (WizardStep::Preview, true),
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
        a.steer(stage, &btt::Pad::default());
        a.steer(stage, &btt::Pad { stick: [f32::NAN, 0.0], ..btt::Pad::default() });
        assert_eq!(a, fresh);
        assert!(a.unmoved());

        a.steer(stage, &btt::Pad { stick: [1.0, 0.0], ..btt::Pad::default() });
        assert_ne!(a.setup.cpu, default);
        a.steer(stage, &btt::Pad { stick: [-1.0, 0.0], ..btt::Pad::default() });
        assert_eq!(a.setup.cpu, default);
        assert!(!a.unmoved());
        assert_eq!(diddy.remembered(stage, a.setup, a.unmoved()).cpu, default);

        let mut b = a;
        b.set_side_default(stage, true);
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
        a.steer(stage, &btt::Pad { stick: [1.0, 0.0], ..btt::Pad::default() });
        assert!(!a.unmoved());
        a.set_side_default(stage, true);
        assert_eq!(stage.side(a.setup.cpu[0]), 1);
        assert!(a.unmoved());
        let mut right_diddy = diddy;
        right_diddy.set_side_default(stage, true);
        assert_eq!(diddy.remembered(stage, a.setup, true).cpu, right_diddy.setup.cpu);
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
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [94.0, -30.0], "{key:?}");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [-94.0, -30.0], "{key:?}");
        }
        for key in [(0, 1), (89, 0)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(0x27)).unwrap();
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
        }
    }

    #[test]
    fn link_starts_facing_away_on_each_proven_side() {
        let stage = Stage::new(
            [-230.0, 230.0, 185.0, -140.0], [-160.0, 160.0, 100.0, -110.0],
            [[-80.0, 0.0], [80.0, 0.0]],
        ).unwrap();
        let left = Setup { cpu: [-102.0, -2.0], p1: [0.0, 0.0], percent: 100 };
        let right = Setup { cpu: [118.0, 38.0], percent: 105, ..left };
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
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, stage.lower_middle(1).unwrap());
        }
        for (stage_key, fighter) in [((44, 0), perfect::pyra::PYRA), ((3, 0), perfect::pyra::MYTHRA)] {
            let wizard = Wizard::new(
                Drill::GuardEdge, stage, [0.0; 2], 60, stage_key, Some(fighter),
            ).unwrap();
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
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
            assert_eq!(wizard.setup.cpu, [-98.0, 30.0], "native stage {key:?}");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, stage.lower_middle(1).unwrap());
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
            assert_eq!(wizard.setup.cpu, [-190.0, 10.0], "native stage {key:?}");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, stage.lower_middle(1).unwrap());
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
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
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
                else { stage.lower_middle(0).unwrap() };
            assert_eq!(wizard.setup.cpu, left);
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, stage.lower_middle(1).unwrap());
            wizard.set_side_default(stage, true);
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
                else { stage.lower_middle(0).unwrap() };
            assert_eq!(wizard.setup.cpu, left);
            assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)),
                perfect::is_crossing_stage(key).then_some(-1.0));
            assert_eq!(recovery_away_facing(Drill::Juggler, key, stage, wizard.setup, Some(0x22)), None);
            wizard.set_side_default(stage, true);
            let right = if key == (3, 0) { [100.0, -4.0] }
                else { stage.lower_middle(1).unwrap() };
            assert_eq!(wizard.setup.cpu, right);
            assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x22)),
                (key == (3, 0)).then_some(1.0));
            wizard.set_side_default(stage, true);
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
            if perfect::is_crossing_stage(key) {
                assert_eq!(wizard.setup.cpu, [-109.99, 18.0]);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)), Some(-1.0));
                assert_eq!(recovery_away_facing(Drill::Juggler, key, stage, wizard.setup, Some(0x2)), None);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, Setup { percent: 120, ..wizard.setup }, Some(0x2)), None);
            } else {
                assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
            }
            wizard.set_side_default(stage, true);
            if perfect::is_crossing_stage(key) {
                assert_eq!(wizard.setup.cpu, [117.99, 38.0]);
                assert_eq!(recovery_away_facing(Drill::GuardEdge, key, stage, wizard.setup, Some(0x2)), Some(1.0));
            } else {
                assert_eq!(wizard.setup.cpu, stage.lower_middle(1).unwrap());
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
            for (side, expected) in [(0, [-120.0, 20.0]), (1, [120.0, 20.0])] {
                if side == 1 {
                    wizard.set_side_default(stage, true);
                }
                let actual = if perfect::is_crossing_stage(key) {
                    expected
                } else {
                    stage.lower_middle(side).unwrap()
                };
                assert_eq!(wizard.setup.cpu, actual, "stage {key:?}, side {side:?}");
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
                let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, key, Some(fighter)).unwrap();
                let expected = if key.1 == 0 { [-110.0, 6.0] } else { stage.lower_middle(0).unwrap() };
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
                assert_eq!(wizard.setup.cpu, if perfect::is_crossing_stage(key) {
                    [-120.0, 20.0]
                } else {
                    stage.lower_middle(0).unwrap()
                });
                wizard.set_side_default(stage, true);
                assert_eq!(wizard.setup.cpu, if perfect::is_crossing_stage(key) {
                    [120.0, 20.0]
                } else {
                    stage.lower_middle(1).unwrap()
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
            assert_eq!(wizard.setup.cpu, [-106.0, 14.0]);
            wizard.set_side_default(stage, true);
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
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [100.0, -28.0], "{key:?}");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [-100.0, -28.0], "{key:?}");
        }
        for key in [(89, 0), (3, 1)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 0, key, pichu).unwrap();
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
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
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [100.0, -28.0], "stage {key:?}");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [-100.0, -28.0], "stage {key:?}");
        }
        let mut waiting_room =
            Wizard::new(Drill::GuardEdge, stage, [0.0; 2], 100, (311, 0), puff).unwrap();
        assert_eq!(waiting_room.setup.cpu, [-120.0, 20.0]);
        waiting_room.set_side_default(stage, true);
        assert_eq!(waiting_room.setup.cpu, [120.0, 20.0]);
        for key in [(3, 1), (0, 1), (311, 1)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, key, puff).unwrap();
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
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
            wizard.set_side_default(stage, true);
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
            wiz.set_side_default(stage, true);
            assert_eq!(wiz.setup.cpu, [98.0, -30.0]);
            wiz.set_side_default(stage, true);
            assert_eq!(wiz.setup.cpu, [-98.0, -30.0]);
        }
        for key in [(89, 0), (0, 1)] {
            let wiz = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, key, sheik).unwrap();
            assert_eq!(wiz.setup.cpu, stage.lower_middle(0).unwrap());
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
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [100.0, -55.0]);
            for _ in 0..12 {
                wizard.steer(stage, &btt::Pad { stick: [1.0, 0.625], ..btt::Pad::default() });
            }
            wizard.steer(stage, &btt::Pad { stick: [0.5, 0.0], ..btt::Pad::default() });
            assert_eq!(wizard.setup.cpu, [125.0, -40.0], "the wizard still edits the point");
            wizard.set_side_default(stage, true);
            assert_eq!(wizard.setup.cpu, [-100.0, -55.0]);
        }

        for (id, kind) in [((3, 1), pikachu), ((89, 0), pikachu), ((3, 0), Some(0x18)), ((3, 0), None)] {
            let wizard = Wizard::new(Drill::GuardEdge, stage, [0.0, 0.0], 100, id, kind).unwrap();
            assert_eq!(wizard.setup.cpu, stage.lower_middle(0).unwrap());
        }
        let juggle = Wizard::new(Drill::Juggler, stage, [0.0, 0.0], 100, (3, 0), pikachu).unwrap();
        assert_eq!(juggle.setup.cpu, stage.default_juggle([0.0, 0.0]));
    }

    #[test]
    fn juggle_wizard_percent_and_player_step() {
        let stage = Stage::new([-160.0, 150.0, 110.0, -85.0], [-160.0, 150.0, 110.0, -85.0], [[-60.0, 5.0], [80.0, 8.0]]).unwrap();
        let mut wizard = Wizard::new(Drill::Juggler, stage, [0.0, 7.0], 60, (3, 0), None).unwrap();
        assert_eq!(wizard.step, WizardStep::Cpu);
        for _ in 0..75 {
            wizard.steer(stage, &btt::Pad { stick: [-1.0, 0.0], ..btt::Pad::default() });
        }
        for _ in 0..15 {
            wizard.steer(stage, &btt::Pad { stick: [0.0, 1.0], ..btt::Pad::default() });
        }
        wizard.steer(stage, &btt::Pad { stick: [0.0, 0.75], ..btt::Pad::default() });
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
