use super::btt;
use super::chords::logical;
use super::cpu::ids::C;
use super::crosshair::{Crosshair, Marks};
use super::floor_hop;
use super::panes::{EggLook, EGG_WHOLE};
use super::stage_geometry::{classify, default_spot, nearest_ledge, Probe};
use super::state::record::{SavedState, StartState};

pub const READY_FRAMES: u16 = 60;
pub const CHECK_FRAMES: u16 = 60;
pub const PRACTICE_ACTIVE: &str = "Stop the practice game to place fighters";

pub const DOWN_BOUND_U: u64 = crate::model::hash40::hash40("down_bound_u");
pub const DOWN_BOUND_D: u64 = crate::model::hash40::hash40("down_bound_d");
pub const DOWN_WAIT_U: u64 = crate::model::hash40::hash40("down_wait_u");
pub const DOWN_WAIT_D: u64 = crate::model::hash40::hash40("down_wait_d");
pub const LIVE_FRAMES: [u16; 2] = [1, 40];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownFace { pub up: bool, pub motion: u64 }

pub const fn down_face(state: StartState) -> Option<DownFace> {
    match state {
        StartState::MissedTechUp => Some(DownFace { up: true, motion: DOWN_BOUND_U }),
        StartState::MissedTechDown => Some(DownFace { up: false, motion: DOWN_BOUND_D }),
        _ => None,
    }
}

pub fn down_face_fix(want: DownFace, flag_up: bool, motion: u64) -> Option<DownFace> {
    (flag_up != want.up || motion != want.motion).then_some(want)
}

pub trait DownBody {
    fn force_down(&mut self);
    /// The face flag and motion kind, or None when the fighter is not in DOWN after the forced change.
    fn face(&self) -> Option<(bool, u64)>;
    fn set_face(&mut self, up: bool, motion: u64);
}

pub fn enter_down(body: &mut impl DownBody, want: DownFace) {
    body.force_down();
    if let Some(face) = body.face().and_then(|(flag_up, motion)| down_face_fix(want, flag_up, motion)) {
        body.set_face(face.up, face.motion);
    }
}

#[derive(Default)]
pub struct Watch { seen: [u16; 2] }

impl Watch {
    pub const fn new() -> Self { Self { seen: [0; 2] } }

    pub fn step(&mut self, who: Who) -> Option<usize> {
        let seen = &mut self.seen[who.index()];
        *seen = seen.saturating_add(1);
        LIVE_FRAMES.iter().position(|&frame| frame == *seen)
    }

    pub fn done(&self) -> bool {
        self.seen.iter().all(|&seen| seen >= LIVE_FRAMES[1])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord { Pass(bool), Wizard }

pub const fn chord(record: bool, capturing: bool, place_on: bool) -> Chord {
    if record && !capturing && place_on {
        Chord::Wizard
    } else {
        Chord::Pass(record)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who { P1, Cpu }

impl Who {
    pub const fn index(self) -> usize {
        match self { Self::P1 => 0, Self::Cpu => 1 }
    }

    const fn other(self) -> Self {
        match self { Self::P1 => Self::Cpu, Self::Cpu => Self::P1 }
    }

    const fn name(self) -> &'static str {
        match self { Self::P1 => "P1", Self::Cpu => "CPU" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place { OnStage, OffStage }

pub const fn first_status(state: StartState, place: Place) -> C {
    match state {
        StartState::Standard => match place { Place::OnStage => C::StatusWait, Place::OffStage => C::StatusFall },
        StartState::Tumble => C::StatusDamageFall,
        StartState::Helpless => C::StatusFallSpecial,
        StartState::Footstool => C::StatusTreadDamageAir,
        StartState::MissedTechUp | StartState::MissedTechDown => C::StatusDown,
        StartState::Trip => C::StatusSlip,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub place: Place,
    pub at: [f32; 2],
    pub state: StartState,
    pub percent: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status { Standard, State, Other }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub usable: bool,
    pub requested: bool,
    pub status: Status,
    pub grounded: Option<bool>,
    pub face_up: bool,
    pub motion: u64,
    pub at: [f32; 2],
    pub damage: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act { None, Write, Hold, Request }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End { Started, Refused(Who) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orders {
    pub acts: [Act; 2],
    pub live: bool,
    pub count: Option<u8>,
    pub end: Option<End>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase { Initial, Place, Confirm, Ended(End) }

pub struct Sync {
    plans: [Plan; 2],
    phase: Phase,
    left: u16,
    waited: u16,
    count_started: bool,
    last: Orders,
}

impl Sync {
    pub fn new(plans: [Plan; 2], count: u16) -> Self {
        Self {
            plans,
            phase: Phase::Initial,
            left: count,
            waited: 0,
            count_started: false,
            last: Orders { acts: [Act::None; 2], live: false, count: None, end: None },
        }
    }

    pub fn plan(&self, who: Who) -> &Plan {
        &self.plans[who.index()]
    }

    fn placed(&self, index: usize, seen: Option<Seen>) -> bool {
        let plan = self.plans[index];
        seen.is_some_and(|s| s.usable && s.status == Status::Standard
            && s.grounded == Some(plan.place == Place::OnStage) && s.at == plan.at
            && plan.percent.is_none_or(|percent| s.damage == percent))
    }

    fn holding(&self, index: usize) -> Act {
        match self.plans[index].place { Place::OnStage => Act::Write, Place::OffStage => Act::Hold }
    }

    fn count(&self) -> Option<u8> {
        (self.count_started && self.left > 0).then_some(u32::from(self.left).div_ceil(20).min(u32::from(u8::MAX)) as u8)
    }

    fn answer(&mut self, orders: Orders) -> Orders {
        self.last = orders;
        orders
    }

    fn end(&mut self, end: End) -> Orders {
        self.phase = Phase::Ended(end);
        self.answer(Orders { acts: [Act::None; 2], live: false, count: None, end: Some(end) })
    }

    pub fn tick(&mut self, seen: [Option<Seen>; 2], ready: bool) -> Orders {
        if seen.iter().all(Option::is_none) {
            return Orders { live: false, ..self.last };
        }
        match self.phase {
            Phase::Initial => {
                if seen.iter().all(|report| report.is_some_and(|s| s.usable)) {
                    self.phase = Phase::Place;
                    self.waited = 0;
                    return self.answer(Orders { acts: [Act::Write; 2], live: false, count: None, end: None });
                }
                self.waited = self.waited.saturating_add(1);
                if self.waited > CHECK_FRAMES {
                    let who = if seen[0].is_none_or(|s| !s.usable) { Who::P1 } else { Who::Cpu };
                    return self.end(End::Refused(who));
                }
                self.answer(Orders { acts: [Act::None; 2], live: false, count: None, end: None })
            }
            Phase::Ended(end) => self.end(end),
            Phase::Confirm => {
                for who in [Who::P1, Who::Cpu] {
                    let plan = self.plans[who.index()];
                    let valid = seen[who.index()].is_some_and(|s| {
                        let status = if plan.state == StartState::Standard { Status::Standard } else { Status::State };
                        s.requested && s.usable && s.status == status
                            && s.grounded == Some(plan.place == Place::OnStage)
                            && s.at == plan.at && plan.percent.is_none_or(|percent| s.damage == percent)
                            && down_face(plan.state).is_none_or(|want| s.face_up == want.up && s.motion == want.motion)
                    });
                    if !valid { return self.end(End::Refused(who)); }
                }
                self.phase = Phase::Ended(End::Started);
                self.answer(Orders { acts: [Act::None; 2], live: true, count: None, end: Some(End::Started) })
            }
            Phase::Place => {
                let placed = [self.placed(0, seen[0]), self.placed(1, seen[1])];
                let acts = std::array::from_fn(|i| if placed[i] { self.holding(i) } else { Act::Write });
                if !placed[0] || !placed[1] || self.left == 0 && !ready {
                    self.waited = self.waited.saturating_add(1);
                    if self.waited > CHECK_FRAMES {
                        let who = if !placed[0] { Who::P1 } else if !placed[1] { Who::Cpu } else { Who::P1 };
                        return self.end(End::Refused(who));
                    }
                    return self.answer(Orders { acts, live: false, count: self.count(), end: None });
                }
                if self.left > 0 {
                    self.count_started = true;
                    let count = self.count();
                    self.left -= 1;
                    return self.answer(Orders { acts, live: false, count, end: None });
                }
                self.phase = Phase::Confirm;
                self.answer(Orders { acts: [Act::Request; 2], live: false, count: None, end: None })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub place: Place,
    pub at: [f32; 2],
    pub state: StartState,
    pub percent: u16,
}

pub type Setup = [Fighter; 2];

/// `Ask` comes before the five pages, and only on a save that macros start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page { Ask, Where, Spot, State, Percent, Preview }

/// What the wizard is opened for: a Record start, or a save state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Record, Save }

/// A save that macros start from (operator 2026-11-25, PC5): how many macro files link it, and whether the picker
/// was opened from one of them (its own screen or the Replays row) or from the save itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shared {
    pub macros: u8,
    pub from_macro: bool,
}

/// The Advanced setting "Shared start save": ask each time, or always take one answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SharedStart {
    #[default]
    Prompt,
    Change,
    Copy,
}

/// How a picker was left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    /// B on the first page, or a save that could not be written: the fighters go back.
    Cancel,
    /// Anything else: a Record start or a load places the fighters, or the scene is gone.
    Other,
}

/// What leaving does with one body the picker held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// Not the body the picker took: it is not written, woken or moved.
    Nothing,
    /// Woken where it is.
    Wake,
    /// Woken and put back where it stood when the picker took it.
    PutBack,
}

/// `same_body`: the role's body is still the one the picker took. `loading`: a save state is loading, and a load
/// places the fighters itself.
pub const fn release(leave: Leave, same_body: bool, loading: bool) -> Release {
    if !same_body {
        Release::Nothing
    } else if matches!(leave, Leave::Cancel) && !loading {
        Release::PutBack
    } else {
        Release::Wake
    }
}

/// The worker's answer to a confirmed placement: the visit it belongs to, and the written pair to load once.
/// `None`: the save could not be written.
pub struct Answer {
    pub ticket: u32,
    pub pair: Option<[SavedState; 2]>,
}

/// The posted answer, for the visit `ticket` that still waits for it: its pair, moved out of the box, which stays
/// where it is for the worker to drop, marked as taken. An answer for any other visit is left unread, so a visit
/// that ended gets no load and the next visit does not inherit one. A fighter frame; no allocation and no free.
pub fn take_answer(posted: &mut Option<Box<Answer>>, ticket: u32) -> Option<Option<[SavedState; 2]>> {
    let answer = posted.as_deref_mut().filter(|answer| ticket != 0 && answer.ticket == ticket)?;
    answer.ticket = 0;
    Some(answer.pair.take())
}

/// How far a spot may go: on stage the camera's range, off stage up to 1 unit inside the blast line (Guard the
/// Edge's limit), `[left, right, top, bottom]`. Without the blast line the camera's range stands in.
pub fn reach(place: Place, camera: [f32; 4], blast: Option<[f32; 4]>) -> [f32; 4] {
    match (place, blast) {
        (Place::OffStage, Some([left, right, top, bottom])) => [left + 1.0, right - 1.0, top - 1.0, bottom + 1.0],
        _ => camera,
    }
}

/// Where a fighter switched to Off stage goes (operator 2026-11-25, T1 A). `None`: its spot stays. A spot that is
/// off stage by the rule stays, in open air or under a solid, and nothing more is read for it. Any other spot goes
/// to the default spot beside the nearest ledge with open air. A stage that cannot be read, or that has no such
/// ledge or spot, moves nothing. `at` is the crosshair's point on the Spot page, and the fighter's spot on the Where page.
pub fn off_stage_spot(stage: &mut impl Probe, blast: [f32; 4], at: [f32; 2]) -> Option<[f32; 2]> {
    if classify(stage, blast, at).ok()?.off_stage(true) {
        return None;
    }
    let ledge = nearest_ledge(stage, blast, at).ok()??;
    default_spot(stage, blast, ledge).ok()?
}

/// The game's situation kinds a save records.
pub const SITUATION_GROUND: i32 = 0;
pub const SITUATION_CLIFF: i32 = 1;
pub const SITUATION_AIR: i32 = 2;

/// One half of a save as the wizard shows it, and whether it was saved hanging on a ledge.
pub fn saved_fighter(saved: &SavedState) -> (Fighter, bool) {
    let place = if saved.situation_kind == SITUATION_GROUND { Place::OnStage } else { Place::OffStage };
    let percent = saved.percent.round().clamp(0.0, 300.0) as u16;
    (Fighter { place, at: [saved.x, saved.y], state: saved.start_state, percent }, saved.situation_kind == SITUATION_CLIFF)
}

/// A confirmed placement written into a save's two halves: only what was edited, field by field. An edited spot
/// takes its place and the stage it was placed on; everything else in the record stays as saved.
pub fn patch(pair: [&mut SavedState; 2], setup: &Setup, edits: [Edits; 2], stage: i32) {
    for ((saved, fighter), edit) in pair.into_iter().zip(setup).zip(edits) {
        if edit.spot {
            [saved.x, saved.y] = fighter.at;
            saved.situation_kind = if fighter.place == Place::OnStage { SITUATION_GROUND } else { SITUATION_AIR };
            saved.saved_stage = Some(stage);
        }
        if edit.state {
            saved.start_state = fighter.state;
        }
        if edit.percent {
            saved.percent = f32::from(fighter.percent);
        }
    }
}

/// The approved refusal for a save that could not be written (tr-primary-2, 2026-11-25).
pub fn write_not_saved(name: &str, out: &mut impl core::fmt::Write) -> core::fmt::Result {
    write!(out, "Not saved · {name} could not be written")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raw {
    pub status: i32,
    pub situation: i32,
    pub at: [f32; 2],
    pub damage: f32,
    pub lr: f32,
    pub motion_frame: f32,
    pub face_up: bool,
    pub motion: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Live {
    pub frames: u16,
    pub tick: u32,
    pub raw: Raw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Idle, Pages, Start }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub stage: Stage,
    pub page: Page,
    pub edit: Who,
    pub recorded: Who,
    pub plans: [Plan; 2],
    pub ends: u32,
    pub end: Option<(bool, End, bool)>,
    pub frames: [[Option<Raw>; 3]; 2],
    pub live: [[Option<Live>; 2]; 2],
}

impl Shown {
    pub const IDLE: Self = Self {
        stage: Stage::Idle,
        page: Page::Spot,
        edit: Who::P1,
        recorded: Who::P1,
        plans: [Plan { place: Place::OnStage, at: [0.0; 2], state: StartState::Standard, percent: None }; 2],
        ends: 0,
        end: None,
        frames: [[None; 3]; 2],
        live: [[None; 2]; 2],
    };
}

fn who_key(who: Who) -> &'static str {
    match who { Who::P1 => "p1", Who::Cpu => "cpu" }
}

pub fn write_shown(shown: &Shown, first: bool, popup: bool, role: Who, out: &mut String) {
    use core::fmt::Write as _;
    use crate::model::bridge_state::push_str;
    let _ = writeln!(out, "place.first={}", u8::from(first));
    let _ = writeln!(out, "place.popup={}", u8::from(popup));
    push_str(out, "place.role", who_key(role));
    push_str(out, "place.phase", match shown.stage { Stage::Idle => "idle", Stage::Pages => "pages", Stage::Start => "start" });
    push_str(out, "place.page", if shown.stage == Stage::Pages {
        match shown.page {
            Page::Ask => "ask", Page::Where => "where", Page::Spot => "spot", Page::State => "state",
            Page::Percent => "percent", Page::Preview => "preview",
        }
    } else { "" });
    push_str(out, "place.edit", if shown.stage == Stage::Pages { who_key(shown.edit) } else { "" });
    push_str(out, "place.recorded", if shown.stage == Stage::Idle { "" } else { who_key(shown.recorded) });
    for (key, plan) in ["place.p1", "place.cpu"].into_iter().zip(shown.plans) {
        let value = if shown.stage == Stage::Idle {
            String::new()
        } else {
            let percent = plan.percent.map_or_else(|| "-".to_string(), |p| p.to_string());
            format!("{} {} {} {} {}", if plan.place == Place::OnStage { "on" } else { "off" },
                plan.at[0], plan.at[1], plan.state.key(), percent)
        };
        push_str(out, key, &value);
    }
    let _ = writeln!(out, "place.ends={}", shown.ends);
    let end = shown.end.map_or_else(String::new, |(record, end, live)| {
        let (result, who) = match end {
            End::Started => ("started", "-"),
            End::Refused(who) => ("refused", who_key(who)),
        };
        format!("{} {result} {who} live={}", if record { "record" } else { "load" }, u8::from(live))
    });
    push_str(out, "place.end", &end);
    for (key, frames) in ["place.p1.frames", "place.cpu.frames"].into_iter().zip(shown.frames) {
        let mut text = String::new();
        for raw in frames.into_iter().flatten() {
            if !text.is_empty() { text.push(';'); }
            let _ = write!(text, "{},{},{},{},{},{},{},{},0x{:x}", raw.status, raw.situation, raw.at[0], raw.at[1],
                raw.damage, raw.lr, raw.motion_frame, u8::from(raw.face_up), raw.motion);
        }
        push_str(out, key, &text);
    }
    for (key, samples) in ["place.p1.live", "place.cpu.live"].into_iter().zip(shown.live) {
        let mut text = String::new();
        for sample in samples.into_iter().flatten() {
            if !text.is_empty() { text.push(';'); }
            let raw = sample.raw;
            let _ = write!(text, "{},{},{},{},{},{},{},{},{},{},0x{:x}",
                sample.frames, sample.tick, raw.status, raw.situation, raw.at[0], raw.at[1],
                raw.damage, raw.lr, raw.motion_frame, u8::from(raw.face_up), raw.motion);
        }
        push_str(out, key, &text);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome { Open, Start, Closed }

/// Which parts of one fighter differ from what the wizard opened with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edits {
    /// The place or the spot.
    pub spot: bool,
    pub state: bool,
    pub percent: bool,
}

impl Edits {
    pub fn any(self) -> bool {
        self.spot || self.state || self.percent
    }
}

/// A placement confirmed on a save, as the picker hands it to the worker that writes the file. `ticket`: the
/// visit it belongs to. `copy`: it goes into a copy of the save. `stage`: the stage it was placed on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Commit {
    pub ticket: u32,
    pub copy: bool,
    pub setup: Setup,
    pub edits: [Edits; 2],
    pub stage: i32,
    /// The macros that start from the save, as the picker was told: the Saved notice says how many.
    pub macros: u8,
}

pub struct Wizard {
    pub page: Page,
    pub edit: Who,
    pub setup: Setup,
    defaults: Setup,
    mode: Mode,
    ledge: [bool; 2],
    home: Option<[f32; 2]>,
    shared: Option<Shared>,
    copy: bool,
    cross: Crosshair,
    hop: floor_hop::Hop,
    was: u16,
    repeat: u8,
}

impl Wizard {
    pub fn new(defaults: Setup, remembered: Option<Setup>) -> Self {
        let setup = remembered.unwrap_or(defaults);
        Self { page: Page::Where, edit: Who::P1, setup, defaults, mode: Mode::Record, ledge: [false; 2], home: None,
            shared: None, copy: false, cross: Crosshair::new(setup[0].at), hop: floor_hop::Hop::new(), was: 0, repeat: 0 }
    }

    /// Opened on a save state: its spots are the defaults (X puts them back). `ledge`: saved hanging on a ledge.
    pub fn for_save(saved: Setup, ledge: [bool; 2], edit: Who) -> Self {
        Self { edit, mode: Mode::Save, ledge, cross: Crosshair::new(saved[edit.index()].at), ..Self::new(saved, None) }
    }

    /// The save is a start save of `shared.macros` macros (operator 2026-11-25, PC5). `Prompt` opens on the
    /// question: change the save for all of them, or copy it. The other two settings answer it unasked. A save no
    /// macro starts from is changed, unasked.
    pub fn sharing(self, shared: Shared, rule: SharedStart) -> Self {
        if shared.macros == 0 {
            return self;
        }
        let page = if rule == SharedStart::Prompt { Page::Ask } else { self.page };
        Self { page, shared: Some(shared), copy: rule == SharedStart::Copy, ..self }
    }

    /// The answer to the question: the placement goes into a copy of the save, not the save.
    pub fn copies(&self) -> bool {
        self.copy
    }

    /// What a save of this visit writes.
    pub fn commit(&self, ticket: u32, stage: i32) -> Commit {
        Commit { ticket, copy: self.copy, setup: self.setup, edits: self.edits(), stage, macros: self.shared.map_or(0, |shared| shared.macros) }
    }

    /// The main floor's point under the middle of the stage, where a fighter with no floor under it goes on stage.
    pub fn set_home(&mut self, home: Option<[f32; 2]>) {
        self.home = home;
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Which fighters differ from what the wizard opened with, by value. A fighter moved and brought back, or reset
    /// with X, is not edited.
    pub fn edited(&self) -> [bool; 2] {
        self.edits().map(Edits::any)
    }

    /// What differs, field by field: a save is rewritten only in these.
    pub fn edits(&self) -> [Edits; 2] {
        [0, 1].map(|i| {
            let (now, was) = (self.setup[i], self.defaults[i]);
            Edits {
                spot: now.place != was.place || now.at != was.at,
                state: now.state != was.state,
                percent: now.percent != was.percent,
            }
        })
    }

    /// A fighter saved hanging on a ledge reads so until its place or spot is edited.
    fn on_ledge(&self, i: usize) -> bool {
        self.ledge[i] && !self.edits()[i].spot
    }

    /// On stage / off stage for the edited fighter. Off: the spot stays when it is off stage by the stage rule, else
    /// `off` gives the default spot beside the nearest ledge with open air (T1 A). On: the floor under it, else `home`,
    /// else nothing changes. A Record start begins Standard at the new place. A save keeps its state unless that state
    /// cannot exist there (a missed tech or a trip in the air; a tumble, helpless or footstool start on a floor).
    fn toggle_place(&mut self, at: [f32; 2], floor: impl Fn([f32; 2]) -> Option<[f32; 2]>, home: Option<[f32; 2]>,
        off: impl FnOnce([f32; 2]) -> Option<[f32; 2]>) {
        let fighter = &mut self.setup[self.edit.index()];
        let (place, at) = match fighter.place {
            Place::OnStage => (Place::OffStage, off(at).unwrap_or(at)),
            Place::OffStage => match floor(at).or(home) {
                Some(at) => (Place::OnStage, at),
                None => return,
            },
        };
        (fighter.place, fighter.at) = (place, at);
        let impossible = fighter.state.grounded().is_some_and(|grounded| grounded != (place == Place::OnStage));
        if self.mode == Mode::Record || impossible {
            fighter.state = StartState::Standard;
        }
    }

    pub fn cursor(&self) -> [f32; 2] {
        self.cross.at
    }

    /// `step_on` before the stage's floors are traced. No stage rule is read: the spot stays.
    pub fn step(&mut self, pad: &btt::Pad, fit: impl Fn([f32; 2]) -> Option<[f32; 2]>,
        floor: impl Fn([f32; 2]) -> Option<[f32; 2]>) -> Outcome
    {
        self.step_on(pad, fit, floor, |_| None, &[])
    }

    /// One frame of the pad. `floors`: the traced floor lines of the stage (Tech practice's surface walk). With
    /// them an on-stage fighter rides the floors: Left / Right along its floor, Up / Down to the floor above or
    /// below (operator 2026-11-25, PC1 B). Without (the walk is not finished, or the fighter stands on a floor it
    /// did not trace), the fighter stands on the floor under a free crosshair.
    pub fn step_on(&mut self, pad: &btt::Pad, fit: impl Fn([f32; 2]) -> Option<[f32; 2]>,
        floor: impl Fn([f32; 2]) -> Option<[f32; 2]>, off: impl FnOnce([f32; 2]) -> Option<[f32; 2]>,
        floors: &[floor_hop::Floor]) -> Outcome
    {
        let was = self.was;
        self.was = pad.held;
        let pressed = |key| pad.pressed & (1u16 << key) != 0;
        if self.page == Page::Ask {
            // Two choices and B: nothing else acts on the question, + included.
            if pressed(logical::B) { return Outcome::Closed; }
            if pressed(logical::A) {
                self.page = Page::Where;
            } else if pressed(logical::DPAD_UP) || pressed(logical::DPAD_DOWN) {
                self.copy = !self.copy;
            }
            return Outcome::Open;
        }
        if pressed(logical::PLUS) { return Outcome::Start; }
        if pressed(logical::B) {
            self.page = match self.page {
                Page::Ask | Page::Where => return Outcome::Closed,
                Page::Spot => Page::Where,
                Page::State => Page::Spot,
                Page::Percent => Page::State,
                Page::Preview => Page::Percent,
            };
            return Outcome::Open;
        }
        if pressed(logical::A) {
            self.page = match self.page {
                Page::Ask | Page::Where => {
                    self.cross.jump(self.setup[self.edit.index()].at);
                    Page::Spot
                }
                Page::Spot => Page::State,
                Page::State => Page::Percent,
                Page::Percent => Page::Preview,
                Page::Preview => return Outcome::Start,
            };
            return Outcome::Open;
        }
        let other = pressed(logical::L) || pressed(logical::R)
            || self.page == Page::Where && (pressed(logical::DPAD_UP) || pressed(logical::DPAD_DOWN));
        if self.page != Page::Preview && other {
            self.edit = self.edit.other();
            self.cross.jump(self.setup[self.edit.index()].at);
            return Outcome::Open;
        }
        let i = self.edit.index();
        if pressed(logical::X) {
            if self.page == Page::State && matches!(self.setup[i].state, StartState::MissedTechUp | StartState::MissedTechDown) {
                self.setup[i].state = if self.setup[i].state == StartState::MissedTechUp {
                    StartState::MissedTechDown
                } else {
                    StartState::MissedTechUp
                };
            } else {
                self.setup = self.defaults;
                self.cross.jump(self.setup[i].at);
            }
            return Outcome::Open;
        }
        if self.page == Page::Spot && pressed(logical::Y) {
            self.toggle_place(self.cross.at, floor, None, off);
            if self.setup[i].place == Place::OffStage { self.cross.jump(self.setup[i].at); }
            return Outcome::Open;
        }
        if self.page == Page::Where && (pressed(logical::DPAD_LEFT) || pressed(logical::DPAD_RIGHT)) {
            self.toggle_place(self.setup[i].at, floor, self.home, off);
            self.cross.jump(self.setup[i].at);
            return Outcome::Open;
        }
        match self.page {
            Page::Spot if self.setup[i].place == Place::OnStage && floor_hop::on(floors, self.setup[i].at) => {
                // The cursor is the fighter, on a traced floor. The vertical part of the pad is the hop's alone.
                let at = self.setup[i].at;
                let up = self.hop.vertical(pad);
                let vertical = (1u16 << logical::DPAD_UP) | (1u16 << logical::DPAD_DOWN);
                let level = btt::Pad { stick: [pad.stick[0], 0.0], held: pad.held & !vertical, ..*pad };
                self.cross.at = at;
                self.cross.step(&level, |to| fit(to).map(|to| floor_hop::along(floors, at, to[0] - at[0])));
                if let Some(to) = floor_hop::hop(floors, self.cross.at, up > 0).filter(|_| up != 0) {
                    self.cross.at = to;
                }
                self.setup[i].at = self.cross.at;
            }
            Page::Spot => {
                let moved = self.cross.step(pad, fit);
                if self.setup[i].place == Place::OffStage {
                    self.setup[i].at = self.cross.at;
                } else if self.mode == Mode::Record || moved {
                    // A save's spot follows the floor only once the crosshair moves: reading the floor is not an edit.
                    if let Some(at) = floor(self.cross.at) {
                        self.setup[i].at = at;
                    }
                }
            }
            Page::State => {
                let direction = i32::from(pressed(logical::DPAD_RIGHT)) - i32::from(pressed(logical::DPAD_LEFT));
                // A fighter that reads Ledge keeps its saved state: only its spot takes it off the ledge.
                if direction != 0 && !self.on_ledge(i) {
                    let states: &[StartState] = match self.setup[i].place {
                        Place::OnStage => &[StartState::Standard, StartState::MissedTechUp, StartState::Trip],
                        Place::OffStage => &[StartState::Standard, StartState::Tumble, StartState::Helpless, StartState::Footstool],
                    };
                    let state = match self.setup[i].state {
                        StartState::MissedTechDown => StartState::MissedTechUp,
                        state => state,
                    };
                    let current = states.iter().position(|&value| value == state).unwrap_or(0) as i32;
                    self.setup[i].state = states[(current + direction).rem_euclid(states.len() as i32) as usize];
                }
            }
            Page::Percent => {
                let pad = btt::Pad { stick: [0.0; 2], ..*pad };
                let motion = btt::placing_motion(&pad, was, &mut self.repeat);
                let delta = motion[0] as i32 + 10 * motion[1] as i32;
                self.setup[i].percent = (i32::from(self.setup[i].percent) + delta).clamp(0, 300) as u16;
            }
            Page::Ask | Page::Where | Page::Preview => {}
        }
        Outcome::Open
    }

    /// The marks to draw. `held`: that fighter's own body stands at its spot, so it gets no mark.
    pub fn marks(&self, held: [bool; 2]) -> Marks {
        let mut looks = [None; btt::LOOKS];
        for who in [Who::P1, Who::Cpu] {
            let i = who.index();
            let fighter = self.setup[i];
            let editing_spot = self.page == Page::Spot && self.edit == who;
            if !held[i] && (!editing_spot || fighter.place == Place::OnStage) {
                let scale = if editing_spot || who == Who::P1 { 0.3 } else { EGG_WHOLE.scale };
                looks[i] = Some((fighter.at, EggLook { scale, ..EGG_WHOLE }));
            }
        }
        if self.page == Page::Spot {
            looks[btt::LOOKS - 1] = Some(self.cross.look());
        }
        // The border pointer is for the fighter being edited, off stage; never for the other one.
        let edited = self.setup[self.edit.index()];
        let pointer = (edited.place == Place::OffStage).then_some(edited.at);
        Marks { looks, pointer }
    }

    /// The notice. `name`: the save's name when the wizard is opened on one.
    pub fn write_text(&self, recorded: Who, name: &str, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        let (number, title) = match self.page {
            Page::Ask => return self.write_ask(name, out),
            Page::Where => (1, "Where"), Page::Spot => (2, "Spot"), Page::State => (3, "State"),
            Page::Percent => (4, "Percent"), Page::Preview => (5, "Preview"),
        };
        let save = self.mode == Mode::Save;
        // What X and + do, in the words of each use: a save goes back to its saved values and is saved.
        let (reset, plus) = if save { ("X as saved", "+ save") } else { ("X defaults", "+ start") };
        if save {
            writeln!(out, "{name} · {number}/5 {title}")?;
        } else {
            writeln!(out, "Record ({}) · {number}/5 {title}", recorded.name())?;
        }
        for who in [Who::P1, Who::Cpu] {
            let fighter = self.setup[who.index()];
            let name = if who == Who::P1 { "P1  " } else { "CPU " };
            if self.page != Page::Preview {
                write!(out, "{} {name} ", if self.edit == who { ">" } else { " " })?;
            } else {
                write!(out, "{name} ")?;
            }
            match self.page {
                Page::Ask => {}
                Page::Where | Page::Spot | Page::Preview => {
                    let place = if self.on_ledge(who.index()) { "Ledge" }
                        else if fighter.place == Place::OnStage { "On stage" } else { "Off stage" };
                    write!(out, "{place} ({}, {})", fighter.at[0].round() as i32, fighter.at[1].round() as i32)?;
                    if self.page == Page::Preview {
                        write!(out, " · {} · {}%", fighter.state.title(), fighter.percent)?;
                    }
                }
                Page::State => write!(out, "{}", fighter.state.title())?,
                Page::Percent => write!(out, "{}%", fighter.percent)?,
            }
            writeln!(out)?;
        }
        match self.page {
            Page::Ask => Ok(()),
            Page::Where => write!(out, "L / R fighter · ← → on / off stage · A next\n{reset} · {plus} · B cancel"),
            Page::Spot if save => write!(out, "Stick or D-pad move · L / R fighter · A next\n{reset} · Y on / off stage · {plus} · B back"),
            Page::Spot => write!(out, "Stick or D-pad move · L / R fighter · A next\n{reset} · Y on / off stage · B back · {plus}"),
            Page::State => {
                let face = matches!(self.setup[self.edit.index()].state, StartState::MissedTechUp | StartState::MissedTechDown);
                let x = if face { "X face up / down" } else { reset };
                write!(out, "L / R fighter · ← → change · {x} · A next · B back · {plus}")
            }
            Page::Percent => write!(out, "L / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · {reset} · {plus}"),
            Page::Preview => write!(out, "A / {plus} · B back"),
        }
    }

    /// The question before the pages: two choices and B.
    fn write_ask(&self, name: &str, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        let Shared { macros, from_macro } = self.shared.unwrap_or(Shared { macros: 0, from_macro: false });
        let mark = |chosen: bool| if chosen { ">" } else { " " };
        if macros == 1 {
            writeln!(out, "{name} · 1 macro starts here\n{} Change it for its macro", mark(!self.copy))?;
        } else {
            writeln!(out, "{name} · {macros} macros start here\n{} Change it for all {macros} macros", mark(!self.copy))?;
        }
        let copy = if from_macro { "Copy it for this macro" } else { "Copy it to a new save" };
        write!(out, "{} {copy}\n↑ ↓ choose · A next · B cancel", mark(self.copy))
    }

    pub fn plans(&self) -> [Plan; 2] {
        self.setup.map(|fighter| Plan { place: fighter.place, at: fighter.at, state: fighter.state, percent: Some(f32::from(fighter.percent)) })
    }
}

pub fn defaults(live: [([f32; 2], f32, Option<[f32; 2]>); 2],
    fit: impl Fn([f32; 2]) -> Option<[f32; 2]>) -> Option<Setup>
{
    let fighter = |(at, damage, floor): ([f32; 2], f32, Option<[f32; 2]>)| {
        let (place, at) = if let Some(point) = floor { (Place::OnStage, point) } else { (Place::OffStage, fit(at)?) };
        Some(Fighter { place, at, state: StartState::Standard, percent: damage.round().clamp(0.0, 300.0) as u16 })
    };
    Some([fighter(live[0])?, fighter(live[1])?])
}

#[derive(Default)]
pub struct Kept {
    entries: [Option<((i32, u32), Setup)>; 8],
}

impl Kept {
    pub const fn new() -> Self { Self { entries: [None; 8] } }

    pub fn get(&self, stage: (i32, u32)) -> Option<Setup> {
        self.entries.iter().flatten().find(|(key, _)| *key == stage).map(|(_, setup)| *setup)
    }

    pub fn put(&mut self, stage: (i32, u32), setup: Setup) {
        if let Some(entry) = self.entries.iter_mut().flatten().find(|(key, _)| *key == stage) {
            *entry = (stage, setup);
        } else {
            self.entries.rotate_left(1);
            self.entries[7] = Some((stage, setup));
        }
    }
}

pub fn write_refusal(recorded: bool, who: Who, state: StartState, out: &mut impl core::fmt::Write) -> core::fmt::Result {
    write!(out, "{} · {} could not start in {}", if recorded { "Not recorded" } else { "Not placed" }, who.name(), state.title())
}

pub fn write_ready(count: u8, out: &mut impl core::fmt::Write) -> core::fmt::Result {
    write!(out, "Ready · {count}")
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::model::state::record::StartState;

    /// The saved-macro editor routes previews through Recording and hides the clip page.
    fn the_crop_preview_is_wired_to_recordings_playback() {
        let capture = include_str!("../runtime/macros/replay_capture.rs");
        assert!(!capture.contains("macro_file(recording, false)"));
        assert!(capture.contains("rr_host::request_preview("));
        assert!(capture.contains("rr_host::stop_preview()"));
        let vault = capture.split("Some(ClipEffect::Preview(recording))").nth(1).unwrap()
            .split("Some(ClipEffect::PreviewFile").next().unwrap();
        assert!(vault.contains("request_clip_preview("));
        let host = include_str!("../runtime/features/recording_host.rs");
        let apply = host.split("fn apply(").nth(1).unwrap().split("\n}").next().unwrap();
        let home = apply.split("if let Some(home) = tick.home {").nth(1).unwrap().split('}').next().unwrap();
        assert!(home.contains("request_preview_home("));
        let checkpoint = include_str!("../runtime/features/states/preview.rs");
        let request = checkpoint.split("unsafe fn request(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(request.contains("read_record(&raws[0])"));
        assert!(request.contains("read_record(&raws[1])"));
        let take = checkpoint.split("unsafe fn take_home()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(take.find("LOADING.load(").unwrap() < take.find("ASK.swap(").unwrap());
        let returning = take.split("if ask == 3 {").nth(1).unwrap().split("} else").next().unwrap();
        assert!(returning.contains("CHECKPOINT.lock().clear()"));
        let states = include_str!("../runtime/features/states.rs");
        let session = states.split("fn clear_session()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(session.contains("preview::clear()"));
        let named = include_str!("../runtime/macros/named_macros.rs");
        let publish = named.split("pub fn publish_queries()").nth(1).unwrap().split("\n}").next().unwrap();
        let clip_previewing = publish.split("let clip_previewing =").nth(1).unwrap().split(';').next().unwrap();
        assert!(clip_previewing.contains("rr_host::preview_away()"));
        assert!(!named.contains("fn begin_restore()"));
        let clear = named.split("fn clear_session()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(clear.contains("rr_host::drop_preview_home()"));
        let hold = named.split("pub unsafe fn hold_panel_fighter(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(hold.contains("clip_previewing"));
        let worker = include_str!("../runtime/ui/worker.rs");
        let clip = worker.split("if s.phase == Phase::Clip {").nth(1).unwrap().split("\n    }").next().unwrap();
        let store = clip.split("EXT_SHOWN.store(").nth(1).unwrap().split(';').next().unwrap();
        assert!(store.starts_with("true"));
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_crop_preview_is_wired_to_recordings_playback", the_crop_preview_is_wired_to_recordings_playback as fn()),
        ]);
    }

    #[test]
    fn a_missed_tech_start_takes_the_asked_face_whatever_the_pose_gives() {
        struct Fake { pose_up: bool, flag: bool, motion: u64, restarts: u8 }
        impl DownBody for Fake {
            fn force_down(&mut self) {
                self.flag = self.pose_up;
                self.motion = if self.pose_up { DOWN_BOUND_U } else { DOWN_BOUND_D };
            }
            fn face(&self) -> Option<(bool, u64)> { Some((self.flag, self.motion)) }
            fn set_face(&mut self, up: bool, motion: u64) {
                self.flag = up;
                self.motion = motion;
                self.restarts += 1;
            }
        }
        let mut failures = Vec::new();
        for pose_up in [false, true] {
            for asked in [StartState::MissedTechUp, StartState::MissedTechDown] {
                let want = down_face(asked).unwrap();
                let mut fake = Fake { pose_up, flag: false, motion: 0, restarts: 0 };
                enter_down(&mut fake, want);
                if fake.face() != Some((want.up, want.motion)) || fake.restarts != u8::from(pose_up != want.up) {
                    failures.push((pose_up, asked, fake.face(), fake.restarts));
                }
            }
        }
        assert!(failures.is_empty(), "pose, asked, actual face and restarts: {failures:?}");
    }

    #[test]
    fn a_fighter_that_did_not_enter_down_gets_no_face_write() {
        struct Fake { forces: u8, writes: u8 }
        impl DownBody for Fake {
            fn force_down(&mut self) { self.forces += 1; }
            fn face(&self) -> Option<(bool, u64)> { None }
            fn set_face(&mut self, _: bool, _: u64) { self.writes += 1; }
        }
        for asked in [StartState::MissedTechUp, StartState::MissedTechDown] {
            let mut fake = Fake { forces: 0, writes: 0 };
            enter_down(&mut fake, down_face(asked).unwrap());
            assert_eq!(fake.forces, 1);
            assert_eq!(fake.writes, 0);
        }
    }

    #[test]
    fn the_down_motions_are_the_games_names() {
        assert_eq!(DOWN_BOUND_U, 0xcc24d7e43);
        assert_eq!(DOWN_BOUND_D, 0xca8fd5eb1);
        assert_eq!(DOWN_WAIT_U, 0xb95b751c9);
        assert_eq!(DOWN_WAIT_D, 0xbff07713b);
    }

    #[test]
    fn down_face_fix_checks_both_flag_and_motion() {
        let up = DownFace { up: true, motion: DOWN_BOUND_U };
        let down = DownFace { up: false, motion: DOWN_BOUND_D };
        for (want, flag, motion, expected) in [
            (down, true, DOWN_BOUND_U, Some(down)),
            (up, false, DOWN_BOUND_D, Some(up)),
            (down, false, DOWN_BOUND_D, None),
            (up, true, DOWN_BOUND_U, None),
            (up, true, DOWN_BOUND_D, Some(up)),
            (down, false, DOWN_BOUND_U, Some(down)),
            (up, false, DOWN_BOUND_U, Some(up)),
            (down, true, DOWN_BOUND_D, Some(down)),
        ] {
            assert_eq!(down_face_fix(want, flag, motion), expected);
        }
        for state in [StartState::Standard, StartState::Tumble, StartState::Helpless, StartState::Footstool, StartState::Trip] {
            assert_eq!(down_face(state), None);
        }
    }

    #[test]
    fn the_live_watch_samples_each_fighter_on_its_first_and_fortieth_frame() {
        let mut watch = Watch::new();
        assert!(!watch.done());
        for frame in 1..=40 {
            assert_eq!(watch.step(Who::P1), match frame { 1 => Some(0), 40 => Some(1), _ => None });
            assert!(!watch.done());
        }
        for frame in 1..=40 {
            assert_eq!(watch.step(Who::Cpu), match frame { 1 => Some(0), 40 => Some(1), _ => None });
            assert_eq!(watch.done(), frame == 40);
        }
        assert_eq!(watch.step(Who::P1), None);
        assert_eq!(watch.step(Who::Cpu), None);
    }

    #[test]
    fn each_start_state_names_its_first_status() {
        use crate::model::cpu::ids::C;
        for (state, expected) in [
            (StartState::Tumble, C::StatusDamageFall),
            (StartState::Helpless, C::StatusFallSpecial),
            (StartState::Footstool, C::StatusTreadDamageAir),
            (StartState::MissedTechUp, C::StatusDown),
            (StartState::MissedTechDown, C::StatusDown),
            (StartState::Trip, C::StatusSlip),
        ] {
            for place in [Place::OnStage, Place::OffStage] {
                assert_eq!(first_status(state, place), expected);
            }
        }
    }

    #[test]
    fn a_standard_start_follows_the_place() {
        use crate::model::cpu::ids::C;
        assert_eq!(first_status(StartState::Standard, Place::OnStage), C::StatusWait);
        assert_eq!(first_status(StartState::Standard, Place::OffStage), C::StatusFall);
    }

    #[test]
    fn record_placement_runs_wherever_practice_is_live() {
        let place = include_str!("../runtime/features/record_place.rs");
        let recorder = include_str!("../runtime/features/recorder.rs");
        assert!(!place.contains("is_native_training"));
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        assert!(frame.contains("scene::current().is_none()"));
        assert!(frame.contains("scene::current().is_some()"));
        let chord = recorder.split("unsafe fn chord_step(").nth(1).unwrap().split("unsafe fn begin_playback(").next().unwrap();
        assert!(!chord.contains("is_native_training"));
    }

    #[test]
    fn record_place_is_wired_into_the_poll_the_frame_the_camera_and_the_load() {
        let chords = include_str!("../runtime/features/chords.rs");
        let cpu = include_str!("../runtime/features/cpu.rs");
        let camera = include_str!("../runtime/features/btt_camera.rs");
        let states = include_str!("../runtime/features/states.rs");
        let place = include_str!("../runtime/features/record_place.rs");
        assert!(chords.find("record_place::poll(").unwrap() < chords.find("egg_zone::poll(").unwrap());
        assert!(cpu.find("states::frame(").unwrap() < cpu.find("record_place::frame(").unwrap());
        assert!(camera.contains("RECORD_PLACE_OWNER"));
        let targets = states.split("if let Some(start) = out.start").nth(1).unwrap().split("\n    }").next().unwrap();
        assert!(targets.contains("record_place::load_target("));
        let cancelled = states.split("pub fn cancel_pending()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(cancelled.contains("load_cancelled()"));
        assert_eq!(place.matches("WIZARDS.set_up(Kind::RecordPlacing").count(), 1);
        let refused = place.split("End::Refused(who)").nth(1).unwrap();
        assert!(refused.find("abort_warp()").unwrap() < refused.find("finish_start()").unwrap());
        let poll = place.split("pub unsafe fn poll(").nth(1).unwrap().split("unsafe fn stage(").next().unwrap();
        let cancel = poll.split("Session::Start(mut start) =>").nth(1).unwrap().split("let ready").next().unwrap();
        assert!(cancel.contains("!block") && cancel.contains("!start.live_given") && cancel.contains("StartKind::Record"));
        assert!(cancel.contains("key::B") && cancel.contains("leave(&mut engine, Leave::Other)"));
        let clearing = place.split("fn clear_engine(").nth(1).unwrap().split("pub(super) fn open(").next().unwrap();
        assert!(clearing.contains("CANCEL_LOAD") && clearing.contains("WAKE.fetch_or") && clearing.contains("abort_warp()"));
        let cancel_load = place.split("pub(super) fn load_cancelled()").nth(1).unwrap().split("pub(super) fn session_clear()").next().unwrap();
        assert!(cancel_load.find("CANCEL_LOAD.store").unwrap() < cancel_load.find("if dropped").unwrap());
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        assert!(frame.contains("WAKE.swap") && frame.contains("wake(["));
        let request = place.split("Act::Request => {").nth(1).unwrap().split("pub unsafe fn frame(").next().unwrap();
        assert!(request.find("placement::ground(").unwrap() < request.find("change_status_force(").unwrap());
        assert!(!request.contains("status_request("));
        assert!(request.contains("record_place::enter_down("));
        assert!(!request.contains("set_flag("));
        let face = place.split("fn face(&self)").nth(1).unwrap().split("fn set_face(").next().unwrap();
        assert!(face.find("natives::status_kind(").unwrap() < face.find("natives::is_flag(").unwrap());
        assert!(face.contains("== konst(C::StatusDown)") && face.contains(".then(||"));
        let granted = place.split("fn granted(").nth(1).unwrap().split("struct DownAccess").next().unwrap();
        assert!(granted.contains("MOTION_CHANGE_HOOK_OFFSET"));
        assert!(frame.contains("watch.step(who)") && frame.contains("engine.shown.live[who.index()][i]"));
        assert!(frame.contains("rr_host::captured_frames()") && frame.contains("rr_host::played_frames()"));
        let publishing = place.split("fn publish(").nth(1).unwrap().split("pub fn shown(").next().unwrap();
        assert!(publishing.contains("shown != engine.published") && publishing.contains("SHOWN.write(shown)"));
        assert!(frame.find("C::StatusDead").unwrap() < frame.find("!act(").unwrap());
        let worker = include_str!("../runtime/ui/worker.rs");
        assert!(worker.contains("record_place::write_shown(") && worker.contains("features::record_place::shown()"));
        assert_eq!(place.matches("SHOWN.write(").count(), 1);
    }

    fn plans() -> [Plan; 2] {
        [
            Plan { place: Place::OnStage, at: [10.0, 2.0], state: StartState::MissedTechUp, percent: Some(80.0) },
            Plan { place: Place::OffStage, at: [-18.0, 22.0], state: StartState::Footstool, percent: Some(45.0) },
        ]
    }

    fn placed(plan: Plan) -> Seen {
        Seen {
            usable: true,
            requested: false,
            status: Status::Standard,
            grounded: Some(plan.place == Place::OnStage),
            face_up: plan.state == StartState::MissedTechUp,
            motion: down_face(plan.state).map_or(0, |face| face.motion),
            at: plan.at,
            damage: plan.percent.unwrap_or(0.0),
        }
    }

    fn reports(plans: [Plan; 2]) -> [Option<Seen>; 2] {
        [Some(placed(plans[0])), Some(placed(plans[1]))]
    }

    fn started(plans: [Plan; 2]) -> [Option<Seen>; 2] {
        let mut seen = reports(plans);
        for i in 0..2 {
            seen[i].as_mut().unwrap().requested = true;
            if plans[i].state != StartState::Standard {
                seen[i].as_mut().unwrap().status = Status::State;
            }
        }
        seen
    }

    fn initial(sync: &mut Sync) {
        assert_eq!(sync.tick([None, None], true).acts, [Act::None; 2]);
        let plans = [*sync.plan(Who::P1), *sync.plan(Who::Cpu)];
        let order = sync.tick(reports(plans), true);
        assert_eq!(order.acts, [Act::Write; 2]);
        assert!(!order.live && order.end.is_none());
    }

    fn request_both(sync: &mut Sync, plans: [Plan; 2]) {
        initial(sync);
        let order = sync.tick(reports(plans), true);
        assert_eq!(order.acts, [Act::Request; 2]);
        assert!(!order.live && order.end.is_none());
    }

    #[test]
    fn no_write_is_ordered_while_either_fighter_is_dead_or_respawning() {
        let plans = plans();
        let mut sync = Sync::new(plans, 0);
        assert_eq!(sync.tick([None, None], true).acts, [Act::None; 2]);
        for who in [Who::P1, Who::Cpu] {
            let mut seen = reports(plans);
            seen[who.index()].as_mut().unwrap().usable = false;
            let order = sync.tick(seen, true);
            assert_eq!(order.acts, [Act::None; 2]);
            assert!(!order.live && order.end.is_none());
        }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Write; 2]);

        for who in [Who::P1, Who::Cpu] {
            let mut sync = Sync::new(plans, 0);
            let mut seen = reports(plans);
            seen[who.index()].as_mut().unwrap().usable = false;
            for i in 0..=CHECK_FRAMES {
                let order = sync.tick(seen, true);
                assert_eq!(order.acts, [Act::None; 2]);
                assert!(!order.live);
                assert_eq!(order.end, (i == CHECK_FRAMES).then_some(End::Refused(who)));
            }
        }
    }

    #[test]
    fn the_frame_checks_eligibility_before_acting() {
        let place = include_str!("../runtime/features/record_place.rs");
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        assert!(frame.find("C::StatusDead").unwrap() < frame.find("!act(").unwrap());
    }

    #[test]
    fn place_start_rewrites_an_on_stage_fighter_every_frame_until_its_request() {
        let plans = plans();
        let mut sync = Sync::new(plans, 3);
        initial(&mut sync);
        for _ in 0..3 {
            let order = sync.tick(reports(plans), true);
            assert_eq!(order.acts, [Act::Write, Act::Hold]);
            assert!(!order.live && order.end.is_none());
        }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);

    }

    #[test]
    fn the_bridge_lines_name_the_page_the_fighter_edited_and_both_plans() {
        let mut plans = plans();
        plans[1].at = [-18.5, 22.25];
        let shown = Shown {
            stage: Stage::Pages, page: Page::State, edit: Who::Cpu, recorded: Who::Cpu, plans, ..Shown::IDLE
        };
        let mut out = String::new();
        write_shown(&shown, true, false, Who::Cpu, &mut out);
        assert_eq!(out, concat!(
            "place.first=1\n",
            "place.popup=0\n",
            "place.role=\"cpu\"\n",
            "place.phase=\"pages\"\n",
            "place.page=\"state\"\n",
            "place.edit=\"cpu\"\n",
            "place.recorded=\"cpu\"\n",
            "place.p1=\"on 10 2 missed_tech_up 80\"\n",
            "place.cpu=\"off -18.5 22.25 footstool 45\"\n",
            "place.ends=0\n",
            "place.end=\"\"\n",
            "place.p1.frames=\"\"\n",
            "place.cpu.frames=\"\"\n",
            "place.p1.live=\"\"\n",
            "place.cpu.live=\"\"\n",
        ));
    }

    #[test]
    fn the_bridge_lines_carry_the_live_samples() {
        let raw = Raw {
            status: 470, situation: 2, at: [1.25, -0.5], damage: 12.5,
            lr: -1.0, motion_frame: 0.0, face_up: true, motion: DOWN_BOUND_U,
        };
        let shown = Shown {
            live: [
                [Some(Live { frames: 1, tick: 0, raw }), Some(Live { frames: 40, tick: 39, raw })],
                [None, None],
            ],
            ..Shown::IDLE
        };
        let mut out = String::new();
        write_shown(&shown, false, false, Who::P1, &mut out);
        assert!(out.contains("place.p1.live=\"1,0,470,2,1.25,-0.5,12.5,-1,0,1,0xcc24d7e43;40,39,470,2,1.25,-0.5,12.5,-1,0,1,0xcc24d7e43\"\n"));
        assert!(out.contains("place.cpu.live=\"\"\n"));
    }

    #[test]
    fn the_bridge_lines_with_all_live_samples_fit_the_bridge_bound() {
        let raw = Raw {
            status: 470, situation: 2, at: [-123.45678, -123.45678], damage: 300.12345,
            lr: -1.0, motion_frame: 123.45678, face_up: true, motion: 0xff_ffff_ffff,
        };
        let sample = Some(Live { frames: 40, tick: u32::MAX, raw });
        let plan = Plan {
            place: Place::OffStage, at: raw.at, state: StartState::MissedTechDown, percent: Some(300.0),
        };
        let shown = Shown {
            stage: Stage::Pages, page: Page::Percent, edit: Who::Cpu, recorded: Who::Cpu,
            plans: [plan; 2], ends: u32::MAX, end: Some((true, End::Refused(Who::Cpu), true)),
            frames: [[Some(raw); 3]; 2], live: [[sample; 2]; 2],
        };
        let mut out = String::new();
        write_shown(&shown, true, true, Who::Cpu, &mut out);
        assert!(out.len() <= crate::model::bridge_state::MAX, "placement bridge bytes: {}", out.len());
    }

    #[test]
    fn the_bridge_lines_keep_the_last_three_reports_of_an_ended_start() {
        let raw = |status, situation, at, damage, lr, motion_frame, face_up| Raw {
            status, situation, at, damage, lr, motion_frame, face_up, motion: 0xab
        };
        let reports = [
            raw(1, 2, [1.25, -0.5], 12.5, 1.0, 0.0, true),
            raw(3, 4, [2.0, 3.5], 40.0, -1.0, 1.25, false),
            raw(5, 6, [-9.0, 0.0], 0.0, 1.0, 2.0, true),
            raw(7, 8, [4.0, 5.0], 60.0, -1.0, 0.5, false),
            raw(9, 10, [6.0, 7.0], 80.0, 1.0, 1.5, true),
        ];
        let mut shown = Shown {
            ends: 2,
            end: Some((true, End::Started, true)),
            frames: [
                [Some(reports[0]), Some(reports[1]), Some(reports[2])],
                [None, Some(reports[3]), Some(reports[4])],
            ],
            ..Shown::IDLE
        };
        let mut out = String::new();
        write_shown(&shown, false, true, Who::P1, &mut out);
        assert!(out.contains("place.end=\"record started - live=1\"\n"));
        assert!(out.contains("place.p1.frames=\"1,2,1.25,-0.5,12.5,1,0,1,0xab;3,4,2,3.5,40,-1,1.25,0,0xab;5,6,-9,0,0,1,2,1,0xab\"\n"));
        assert!(out.contains("place.cpu.frames=\"7,8,4,5,60,-1,0.5,0,0xab;9,10,6,7,80,1,1.5,1,0xab\"\n"));
        shown.end = Some((false, End::Refused(Who::Cpu), false));
        out.clear();
        write_shown(&shown, false, false, Who::P1, &mut out);
        assert!(out.contains("place.end=\"load refused cpu live=0\"\n"));
    }

    #[test]
    fn the_bridge_lines_stay_small() {
        let plan = Plan {
            place: Place::OffStage,
            at: [-123.45678, -123.45678],
            state: StartState::MissedTechDown,
            percent: Some(300.0),
        };
        let raw = Raw {
            status: 470,
            situation: 2,
            at: [-123.45678, -123.45678],
            damage: 300.12345,
            lr: -1.0,
            motion_frame: 123.45678,
            face_up: true,
            motion: 0xff_ffff_ffff,
        };
        let shown = Shown {
            stage: Stage::Pages,
            page: Page::Percent,
            edit: Who::Cpu,
            recorded: Who::Cpu,
            plans: [plan; 2],
            ends: u32::MAX,
            end: Some((true, End::Refused(Who::Cpu), true)),
            frames: [[Some(raw); 3]; 2],
            live: [[None; 2]; 2],
        };
        let mut out = String::new();
        write_shown(&shown, true, true, Who::Cpu, &mut out);
        println!("placement bridge bytes: {}", out.len());
        assert!(out.len() <= 800, "placement bridge bytes: {}", out.len());
    }

    #[test]
    fn an_idle_placement_publishes_empty_fields() {
        let mut out = String::new();
        write_shown(&Shown::IDLE, false, false, Who::P1, &mut out);
        assert_eq!(out, concat!(
            "place.first=0\n",
            "place.popup=0\n",
            "place.role=\"p1\"\n",
            "place.phase=\"idle\"\n",
            "place.page=\"\"\n",
            "place.edit=\"\"\n",
            "place.recorded=\"\"\n",
            "place.p1=\"\"\n",
            "place.cpu=\"\"\n",
            "place.ends=0\n",
            "place.end=\"\"\n",
            "place.p1.frames=\"\"\n",
            "place.cpu.frames=\"\"\n",
            "place.p1.live=\"\"\n",
            "place.cpu.live=\"\"\n",
        ));
    }

    #[test]
    fn record_chord_with_place_off_starts_at_once() {
        assert_eq!(chord(true, false, false), Chord::Pass(true));
        assert_eq!(chord(false, false, false), Chord::Pass(false));
    }

    #[test]
    fn record_chord_with_place_on_opens_the_wizard() {
        assert_eq!(chord(true, false, true), Chord::Wizard);
        assert_eq!(chord(false, false, true), Chord::Pass(false));
        assert_eq!(PRACTICE_ACTIVE, "Stop the practice game to place fighters");
    }

    #[test]
    fn a_record_chord_that_stops_a_capture_never_opens_the_wizard() {
        for place in [false, true] {
            assert_eq!(chord(true, true, place), Chord::Pass(true));
            assert_eq!(chord(false, true, place), Chord::Pass(false));
        }
    }

    fn wizard_defaults() -> Setup {
        [
            Fighter { place: Place::OnStage, at: [-40.0, 0.0], state: StartState::Standard, percent: 0 },
            Fighter { place: Place::OffStage, at: [-118.0, 22.0], state: StartState::Tumble, percent: 45 },
        ]
    }

    fn button(key: u8) -> crate::model::btt::Pad {
        crate::model::btt::Pad { held: 1 << key, pressed: 1 << key, ..Default::default() }
    }

    fn wizard_press(wizard: &mut Wizard, key: u8) -> Outcome {
        wizard.step(&button(key), Some, |at| Some([at[0], 0.0]))
    }

    /// A Record wizard on its Spot page: one A from Where.
    fn spot_wizard(defaults: Setup, remembered: Option<Setup>) -> Wizard {
        let mut w = Wizard::new(defaults, remembered);
        assert_eq!(w.page, Page::Where);
        assert_eq!(wizard_press(&mut w, crate::model::chords::logical::A), Outcome::Open);
        w
    }

    /// A save's spots: Player 1 on a platform, the CPU hanging on the left ledge.
    fn saved() -> Setup {
        [
            Fighter { place: Place::OnStage, at: [-30.0, 30.0], state: StartState::Standard, percent: 12 },
            Fighter { place: Place::OffStage, at: [-80.0, -11.0], state: StartState::Standard, percent: 40 },
        ]
    }

    fn save_wizard() -> Wizard {
        Wizard::for_save(saved(), [false, true], Who::P1)
    }

    /// Floors of the tests: a platform at 30 over the ground at 0, both from x -60 to 60.
    fn floors(at: [f32; 2]) -> Option<[f32; 2]> {
        if at[0].abs() > 60.0 { None }
        else if at[1] + 1.0 >= 30.0 { Some([at[0], 30.0]) }
        else if at[1] + 1.0 >= 0.0 { Some([at[0], 0.0]) }
        else { None }
    }

    #[test]
    fn place_wizard_pages_in_order() {
        use crate::model::chords::logical::{A, B, PLUS};
        let mut w = Wizard::new(wizard_defaults(), None);
        assert_eq!((w.page, w.edit, w.mode()), (Page::Where, Who::P1, Mode::Record));
        assert_eq!(wizard_press(&mut w, A), Outcome::Open);
        assert_eq!((w.page, w.edit, w.cursor()), (Page::Spot, Who::P1, [-40.0, 0.0]));
        assert_eq!(wizard_press(&mut w, B), Outcome::Open);
        assert_eq!(w.page, Page::Where);
        wizard_press(&mut w, A);
        for page in [Page::State, Page::Percent, Page::Preview] {
            assert_eq!(wizard_press(&mut w, A), Outcome::Open);
            assert_eq!(w.page, page);
        }
        assert_eq!(wizard_press(&mut w, A), Outcome::Start);
        assert_eq!(wizard_press(&mut w, B), Outcome::Open);
        assert_eq!(w.page, Page::Percent);
        assert_eq!(wizard_press(&mut w, PLUS), Outcome::Start);
    }

    #[test]
    fn place_wizard_role_toggle_edits_either_fighter() {
        use crate::model::chords::logical::{A, L, R};
        let mut w = spot_wizard(wizard_defaults(), None);
        for page in [Page::Spot, Page::State, Page::Percent] {
            assert_eq!(w.page, page);
            let before = w.setup;
            let mut pad = button(L);
            pad.stick = [1.0, 1.0];
            assert_eq!(w.step(&pad, Some, |_| Some([0.0, 0.0])), Outcome::Open);
            assert_eq!(w.edit, Who::Cpu);
            assert_eq!(w.cursor(), before[1].at);
            assert_eq!(w.setup, before);
            wizard_press(&mut w, R);
            assert_eq!(w.edit, Who::P1);
            assert_eq!(w.cursor(), before[0].at);
            wizard_press(&mut w, A);
        }
        assert_eq!(w.page, Page::Preview);
        wizard_press(&mut w, L);
        assert_eq!(w.edit, Who::P1);
    }

    #[test]
    fn place_wizard_keeps_each_fighters_place_apart() {
        use crate::model::btt;
        use crate::model::chords::logical::{A, L};
        use crate::model::panes::{EggLook, EGG_WHOLE};
        let setup = wizard_defaults();
        let mut w = spot_wizard(setup, None);
        let marks = w.marks([false; 2]);
        assert_eq!(marks.looks[0], Some((setup[0].at, EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[1], Some((setup[1].at, EGG_WHOLE)));
        assert_eq!(marks.pointer, None);
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        wizard_press(&mut w, L);
        let marks = w.marks([false; 2]);
        assert_eq!(marks.looks[0], Some((setup[0].at, EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[1], None);
        assert_eq!(marks.pointer, Some(setup[1].at));
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        for _ in 0..3 { wizard_press(&mut w, A); }
        assert_eq!(w.setup, setup);
        let marks = w.marks([false; 2]);
        assert_eq!(marks.looks[0], Some((setup[0].at, EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[1], Some((setup[1].at, EGG_WHOLE)));
        assert_eq!(marks.looks[btt::LOOKS - 1], None);
        assert_eq!(marks.pointer, Some(setup[1].at));
        assert_eq!(w.plans(), [
            Plan { place: Place::OnStage, at: [-40.0, 0.0], state: StartState::Standard, percent: Some(0.0) },
            Plan { place: Place::OffStage, at: [-118.0, 22.0], state: StartState::Tumble, percent: Some(45.0) },
        ]);
    }

    #[test]
    fn place_wizard_y_switches_stage_and_resets_state() {
        use crate::model::chords::logical::Y;
        let mut setup = wizard_defaults();
        setup[0].state = StartState::Trip;
        let mut w = spot_wizard(setup, None);
        let mut pad = button(Y);
        pad.stick = [1.0, 0.0];
        w.step(&pad, Some, |_| Some([0.0, -9.0]));
        assert_eq!((w.setup[0].place, w.setup[0].at, w.setup[0].state),
                   (Place::OffStage, [-40.0, 0.0], StartState::Standard));
        w.setup[0].state = StartState::Helpless;
        w.step(&pad, Some, |_| None);
        assert_eq!((w.setup[0].place, w.setup[0].state), (Place::OffStage, StartState::Helpless));
        w.step(&pad, Some, |_| Some([-40.0, -9.0]));
        assert_eq!((w.setup[0].place, w.setup[0].at, w.setup[0].state),
                   (Place::OnStage, [-40.0, -9.0], StartState::Standard));
    }

    #[test]
    fn place_on_stage_moves_as_the_tech_point() {
        use crate::model::btt::{self, Pad};
        use crate::model::chords::logical::{DPAD_RIGHT, ZL};
        use crate::model::panes::{EggLook, EGG_WHOLE};
        let mut w = spot_wizard(wizard_defaults(), None);
        let mut expected = w.cursor();
        let (mut was, mut repeat) = (0, 0);
        let mut samples = vec![
            Pad { stick: [0.299, -0.299], ..Pad::default() },
            Pad { stick: [0.3, -0.3], ..Pad::default() },
            Pad { stick: [-0.9, 0.75], ..Pad::default() },
        ];
        samples.extend(vec![Pad { held: 1 << DPAD_RIGHT, ..Pad::default() }; 32]);
        samples.push(Pad { held: (1 << DPAD_RIGHT) | (1 << ZL), ..Pad::default() });
        for pad in samples {
            let delta = btt::placing_motion(&pad, was, &mut repeat);
            expected = [expected[0] + delta[0], expected[1] + delta[1]];
            was = pad.held;
            w.step(&pad, Some, |at| Some([at[0], 0.0]));
            assert_eq!(w.cursor().map(f32::to_bits), expected.map(f32::to_bits));
            assert!(w.marks([false; 2]).looks.iter().flatten().any(|(at, look)|
                *at == w.setup[0].at && *look == EggLook { scale: 0.3, ..EGG_WHOLE }));
        }
    }

    #[test]
    fn place_on_stage_mark_takes_the_floor_under_the_crosshair() {
        let mut w = spot_wizard(wizard_defaults(), None);
        let pad = crate::model::btt::Pad { stick: [1.0, 1.0], ..Default::default() };
        w.step(&pad, Some, |at| Some([at[0], at[1] - 8.0]));
        assert_eq!(w.setup[0].at, [w.cursor()[0], w.cursor()[1] - 8.0]);
        let previous = w.setup[0].at;
        w.step(&pad, Some, |_| None);
        assert_eq!(w.setup[0].at, previous);
    }

    #[test]
    fn place_on_stage_down_goes_below_a_platform() {
        let mut setup = wizard_defaults();
        setup[0].at = [0.0, 32.0];
        let mut w = spot_wizard(setup, None);
        let floor = |at: [f32; 2]| {
            if at[1] + 1.0 >= 30.0 { Some([at[0], 30.0]) }
            else if at[1] + 1.0 >= 0.0 { Some([at[0], 0.0]) }
            else { None }
        };
        w.step(&Default::default(), Some, floor);
        assert_eq!(w.setup[0].at[1], 30.0);
        let down = crate::model::btt::Pad { stick: [0.0, -1.0], ..Default::default() };
        for _ in 0..10 { w.step(&down, Some, floor); }
        assert!(w.cursor()[1] < 29.0);
        assert_eq!(w.setup[0].at[1], 0.0);
    }

    #[test]
    fn place_off_stage_moves_as_btt_placing() {
        use crate::model::btt::{self, Pad};
        use crate::model::chords::logical::{DPAD_LEFT, L};
        let mut w = spot_wizard(wizard_defaults(), None);
        wizard_press(&mut w, L);
        let mut expected = w.cursor();
        let (mut was, mut repeat) = (0, 0);
        let mut samples = vec![Pad { stick: [0.3, -0.9], ..Pad::default() }];
        samples.extend(vec![Pad { held: 1 << DPAD_LEFT, ..Pad::default() }; 30]);
        for pad in samples {
            let delta = btt::placing_motion(&pad, was, &mut repeat);
            expected = [expected[0] + delta[0], expected[1] + delta[1]];
            was = pad.held;
            w.step(&pad, Some, |_| None);
            assert_eq!(w.cursor().map(f32::to_bits), expected.map(f32::to_bits));
            assert_eq!(w.setup[1].at, expected);
        }
    }

    #[test]
    fn place_states_offered_off_stage() {
        use crate::model::chords::logical::{A, DPAD_LEFT, DPAD_RIGHT, L};
        let mut w = spot_wizard(wizard_defaults(), None);
        w.setup[1].state = StartState::Standard;
        wizard_press(&mut w, L);
        wizard_press(&mut w, A);
        for state in [StartState::Tumble, StartState::Helpless, StartState::Footstool, StartState::Standard] {
            wizard_press(&mut w, DPAD_RIGHT);
            assert_eq!(w.setup[1].state, state);
        }
        wizard_press(&mut w, DPAD_LEFT);
        assert_eq!(w.setup[1].state, StartState::Footstool);
        let held = crate::model::btt::Pad { held: 1 << DPAD_LEFT, stick: [1.0, 0.0], ..Default::default() };
        for _ in 0..30 { w.step(&held, Some, |_| None); }
        assert_eq!(w.setup[1].state, StartState::Footstool);
    }

    #[test]
    fn place_states_offered_on_stage() {
        use crate::model::chords::logical::{A, DPAD_LEFT, DPAD_RIGHT};
        let mut w = spot_wizard(wizard_defaults(), None);
        wizard_press(&mut w, A);
        for state in [StartState::MissedTechUp, StartState::Trip, StartState::Standard] {
            wizard_press(&mut w, DPAD_RIGHT);
            assert_eq!(w.setup[0].state, state);
        }
        wizard_press(&mut w, DPAD_LEFT);
        assert_eq!(w.setup[0].state, StartState::Trip);
        w.setup[0].state = StartState::MissedTechDown;
        wizard_press(&mut w, DPAD_RIGHT);
        assert_eq!(w.setup[0].state, StartState::Trip);
    }

    #[test]
    fn place_wizard_x_flips_missed_tech_face() {
        use crate::model::chords::logical::{A, DPAD_RIGHT, X};
        let mut w = spot_wizard(wizard_defaults(), None);
        wizard_press(&mut w, A);
        wizard_press(&mut w, DPAD_RIGHT);
        wizard_press(&mut w, X);
        assert_eq!(w.setup[0].state, StartState::MissedTechDown);
        wizard_press(&mut w, X);
        assert_eq!(w.setup[0].state, StartState::MissedTechUp);
    }

    #[test]
    fn place_wizard_percent_page_steps() {
        use crate::model::chords::logical::{A, DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT, DPAD_UP};
        let mut w = spot_wizard(wizard_defaults(), None);
        wizard_press(&mut w, A);
        wizard_press(&mut w, A);
        let mut held = button(DPAD_RIGHT);
        held.stick = [1.0, 1.0];
        w.step(&held, Some, |_| None);
        assert_eq!(w.setup[0].percent, 1);
        held.pressed = 0;
        for _ in 0..15 { w.step(&held, Some, |_| None); }
        assert_eq!(w.setup[0].percent, 2);
        wizard_press(&mut w, DPAD_UP);
        assert_eq!(w.setup[0].percent, 12);
        wizard_press(&mut w, DPAD_DOWN);
        assert_eq!(w.setup[0].percent, 2);
        w.setup[0].percent = 299;
        wizard_press(&mut w, DPAD_UP);
        assert_eq!(w.setup[0].percent, 300);
        w.setup[0].percent = 0;
        wizard_press(&mut w, DPAD_LEFT);
        assert_eq!(w.setup[0].percent, 0);
    }

    #[test]
    fn place_wizard_opens_on_the_last_setup_of_the_stage() {
        let defaults = wizard_defaults();
        let mut kept = Kept::new();
        for stage in 0..8 {
            let mut setup = defaults;
            setup[0].percent = stage as u16;
            kept.put((stage, 0), setup);
        }
        assert_eq!(kept.get((0, 0)).unwrap()[0].percent, 0);
        kept.put((8, 0), defaults);
        assert_eq!(kept.get((0, 0)), None);
        let mut alternate = defaults;
        alternate[0].percent = 177;
        kept.put((8, 1), alternate);
        assert_eq!(kept.get((8, 0)), Some(defaults));
        assert_eq!(kept.get((8, 1)), Some(alternate));
        let w = Wizard::new(defaults, kept.get((8, 1)));
        assert_eq!(w.setup, alternate);
        assert_eq!(w.cursor(), alternate[0].at);
    }

    #[test]
    fn leaving_the_place_wizard_records_nothing() {
        use crate::model::chords::logical::B;
        let mut w = spot_wizard(wizard_defaults(), None);
        assert_eq!(wizard_press(&mut w, B), Outcome::Open);
        assert_eq!(w.page, Page::Where);
        assert_eq!(wizard_press(&mut w, B), Outcome::Closed);
    }

    #[test]
    fn place_wizard_x_puts_both_fighters_back_to_the_defaults() {
        use crate::model::chords::logical::{A, L, X};
        let defaults = wizard_defaults();
        let mut remembered = defaults;
        remembered[0].at = [9.0, 3.0];
        remembered[1].percent = 123;
        let mut w = spot_wizard(defaults, Some(remembered));
        wizard_press(&mut w, A);
        wizard_press(&mut w, L);
        wizard_press(&mut w, X);
        assert_eq!(w.page, Page::State);
        assert_eq!(w.setup, defaults);
        assert_eq!(w.cursor(), defaults[1].at);
    }

    #[test]
    fn place_wizard_texts_match_the_design() {
        use crate::model::chords::logical::{A, L};
        let mut w = Wizard::new(wizard_defaults(), None);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, "", &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 1/5 Where\n> P1   On stage (-40, 0)\n  CPU  Off stage (-118, 22)\nL / R fighter · ← → on / off stage · A next\nX defaults · + start · B cancel"));
        }
        wizard_press(&mut w, A);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, "", &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 2/5 Spot\n> P1   On stage (-40, 0)\n  CPU  Off stage (-118, 22)\nStick or D-pad move · L / R fighter · A next\nX defaults · Y on / off stage · B back · + start"));
        }
        wizard_press(&mut w, A);
        let mut ordinary = String::new();
        w.write_text(Who::P1, "", &mut ordinary).unwrap();
        assert_eq!(ordinary, "Record (P1) · 3/5 State\n> P1   Standard\n  CPU  Tumble\nL / R fighter · ← → change · X defaults · A next · B back · + start");
        wizard_press(&mut w, L);
        w.setup[1].state = StartState::MissedTechUp;
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, "", &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 3/5 State\n  P1   Standard\n> CPU  Missed tech · face up\nL / R fighter · ← → change · X face up / down · A next · B back · + start"));
        }
        wizard_press(&mut w, A);
        wizard_press(&mut w, L);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, "", &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 4/5 Percent\n> P1   0%\n  CPU  45%\nL / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · X defaults · + start"));
        }
        w.setup[1].state = StartState::Tumble;
        wizard_press(&mut w, A);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, "", &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 5/5 Preview\nP1   On stage (-40, 0) · Standard · 0%\nCPU  Off stage (-118, 22) · Tumble · 45%\nA / + start · B back"));
        }
    }

    fn text(w: &Wizard, name: &str) -> String {
        let mut out = String::new();
        w.write_text(Who::P1, name, &mut out).unwrap();
        out
    }

    #[test]
    fn the_placement_picker_opens_on_the_where_page() {
        use crate::model::chords::logical::{A, B, PLUS};
        let mut w = save_wizard();
        assert_eq!((w.page, w.mode(), w.edit, w.cursor()), (Page::Where, Mode::Save, Who::P1, [-30.0, 30.0]));
        let cpu = Wizard::for_save(saved(), [false, true], Who::Cpu);
        assert_eq!((cpu.page, cpu.edit, cpu.cursor()), (Page::Where, Who::Cpu, [-80.0, -11.0]));
        // The operator's PC3: all five pages, and + saves from any of them with the values of the pages not visited.
        assert_eq!(wizard_press(&mut w, B), Outcome::Closed);
        for page in [Page::Where, Page::Spot, Page::State, Page::Percent, Page::Preview] {
            assert_eq!(w.page, page);
            assert_eq!(wizard_press(&mut w, PLUS), Outcome::Start, "{page:?}");
            assert_eq!((w.page, w.setup, w.edited()), (page, saved(), [false; 2]));
            let next = wizard_press(&mut w, A);
            assert_eq!(next, if page == Page::Preview { Outcome::Start } else { Outcome::Open });
        }
        for page in [Page::Percent, Page::State, Page::Spot, Page::Where] {
            assert_eq!(wizard_press(&mut w, B), Outcome::Open);
            assert_eq!(w.page, page);
        }
    }

    #[test]
    fn a_save_carries_the_state_and_percent_of_an_edited_fighter() {
        use crate::model::chords::logical::{A, DPAD_RIGHT, DPAD_UP, L, X};
        let none = Edits::default();
        let mut w = save_wizard();
        assert_eq!(w.edits(), [none; 2]);
        for _ in 0..2 { wizard_press(&mut w, A); }
        assert_eq!(w.page, Page::State);
        wizard_press(&mut w, DPAD_RIGHT);
        assert_eq!(w.setup[0].state, StartState::MissedTechUp);
        assert_eq!(w.edits(), [Edits { state: true, ..none }, none]);
        assert_eq!(w.edited(), [true, false]);
        // A fighter that reads Ledge keeps its saved state: only its spot can take it off the ledge.
        wizard_press(&mut w, L);
        wizard_press(&mut w, DPAD_RIGHT);
        assert_eq!((w.setup[1].state, w.edits()[1]), (StartState::Standard, none));
        wizard_press(&mut w, A);
        wizard_press(&mut w, DPAD_UP);
        assert_eq!(w.setup[1].percent, 50);
        assert_eq!(w.edits(), [Edits { state: true, ..none }, Edits { percent: true, ..none }]);
        assert_eq!(text(&w, "Jab start"), "Jab start · 4/5 Percent\n  P1   12%\n> CPU  50%\nL / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · X as saved · + save");
        // X puts every page's values back, and nothing is edited.
        wizard_press(&mut w, X);
        assert_eq!((w.setup, w.edits()), (saved(), [none; 2]));
    }

    #[test]
    fn a_saved_state_that_cannot_exist_at_the_new_place_becomes_standard() {
        use crate::model::chords::logical::DPAD_RIGHT;
        let mut setup = saved();
        setup[0].state = StartState::Trip;
        setup[1].state = StartState::Tumble;
        let none = Edits::default();
        let mut w = Wizard::for_save(setup, [false; 2], Who::P1);
        w.set_home(Some([0.0, 0.0]));
        // A trip moved into the air cannot be: it becomes Standard, and the save is written with that.
        w.step(&button(DPAD_RIGHT), Some, floors);
        assert_eq!((w.setup[0].place, w.setup[0].state), (Place::OffStage, StartState::Standard));
        assert_eq!(w.edits()[0], Edits { spot: true, state: true, ..none });
        // A tumble moved onto a floor cannot be either.
        let mut w = Wizard::for_save(setup, [false; 2], Who::Cpu);
        w.set_home(Some([0.0, 0.0]));
        w.step(&button(DPAD_RIGHT), Some, floors);
        assert_eq!((w.setup[1].place, w.setup[1].state), (Place::OnStage, StartState::Standard));
        // A state that can exist at both places is kept: a save's Standard, and its damage.
        let mut w = save_wizard();
        w.step(&button(DPAD_RIGHT), Some, floors);
        assert_eq!((w.setup[0].place, w.setup[0].state, w.setup[0].percent), (Place::OffStage, StartState::Standard, 12));
        assert_eq!(w.edits()[0], Edits { spot: true, ..none });
    }

    /// Traced floors of the tests, as `floors` reads them: the ground and the platform, and one more above.
    fn traced() -> Vec<crate::model::floor_hop::Floor> {
        vec![[[-60.0, 0.0], [60.0, 0.0]], [[-60.0, 30.0], [60.0, 30.0]], [[-35.0, 55.0], [-20.0, 55.0]]]
    }

    fn hop_press(w: &mut Wizard, key: u8) -> Outcome {
        w.step_on(&button(key), Some, floors, |_| None, &traced())
    }

    #[test]
    fn on_stage_up_and_down_step_between_the_traced_floors() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{A, DPAD_DOWN, DPAD_UP};
        for mut w in [spot_wizard(saved(), None), { let mut w = save_wizard(); wizard_press(&mut w, A); w }] {
            assert_eq!((w.page, w.cursor()), (Page::Spot, [-30.0, 30.0]));
            hop_press(&mut w, DPAD_UP);
            assert_eq!((w.setup[0].place, w.setup[0].at, w.cursor()), (Place::OnStage, [-30.0, 55.0], [-30.0, 55.0]));
            // No further floor: the fighter stays.
            hop_press(&mut w, DPAD_UP);
            assert_eq!(w.setup[0].at, [-30.0, 55.0]);
            hop_press(&mut w, DPAD_DOWN);
            hop_press(&mut w, DPAD_DOWN);
            assert_eq!((w.setup[0].at, w.cursor()), ([-30.0, 0.0], [-30.0, 0.0]));
            hop_press(&mut w, DPAD_DOWN);
            assert_eq!(w.setup[0].at, [-30.0, 0.0]);
            // One flick of the stick is one floor, however long it is held.
            for _ in 0..30 { w.step_on(&Pad { stick: [0.0, 1.0], ..Pad::default() }, Some, floors, |_| None, &traced()); }
            assert_eq!(w.setup[0].at, [-30.0, 30.0]);
        }
    }

    #[test]
    fn on_stage_left_and_right_move_along_the_floor() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{DPAD_LEFT, DPAD_UP};
        let mut w = spot_wizard(saved(), None);
        hop_press(&mut w, DPAD_UP);
        // The stick: 2 units a frame, as Break the Targets; the D-pad: one unit a press.
        for _ in 0..3 { w.step_on(&Pad { stick: [1.0, 0.0], ..Pad::default() }, Some, floors, |_| None, &traced()); }
        assert_eq!((w.setup[0].at, w.cursor()), ([-24.0, 55.0], [-24.0, 55.0]));
        hop_press(&mut w, DPAD_LEFT);
        assert_eq!(w.setup[0].at, [-25.0, 55.0]);
        // The cursor cannot leave a floor: it stops at the floor's end.
        for _ in 0..20 { w.step_on(&Pad { stick: [1.0, 0.0], ..Pad::default() }, Some, floors, |_| None, &traced()); }
        assert_eq!((w.setup[0].place, w.setup[0].at, w.cursor()), (Place::OnStage, [-20.0, 55.0], [-20.0, 55.0]));
    }

    #[test]
    fn a_diagonal_stick_keeps_the_floor() {
        use crate::model::btt::Pad;
        let mut w = spot_wizard(saved(), None);
        // Past the tilt, but more sideways than up: no hop, and the fighter slides 2 units a frame.
        for _ in 0..5 { w.step_on(&Pad { stick: [1.0, 0.75], ..Pad::default() }, Some, floors, |_| None, &traced()); }
        assert_eq!((w.setup[0].at, w.cursor()), ([-20.0, 30.0], [-20.0, 30.0]));
        // More down than sideways, but short of the tilt: no hop either.
        for _ in 0..5 { w.step_on(&Pad { stick: [-0.5, -0.6875], ..Pad::default() }, Some, floors, |_| None, &traced()); }
        assert_eq!((w.setup[0].at, w.cursor()), ([-25.0, 30.0], [-25.0, 30.0]));
    }

    #[test]
    fn off_stage_and_an_unfinished_walk_keep_the_free_crosshair() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::L;
        let up = Pad { stick: [0.0, 1.0], ..Pad::default() };
        // Off stage the traced floors change nothing: the crosshair is free, as Break the Targets.
        let mut w = spot_wizard(saved(), None);
        hop_press(&mut w, L);
        for _ in 0..3 { w.step_on(&up, Some, floors, |_| None, &traced()); }
        assert_eq!((w.setup[1].place, w.setup[1].at), (Place::OffStage, [-80.0, -5.0]));
        // Without traced floors the fighter stands on the floor under a free crosshair.
        let mut w = spot_wizard(saved(), None);
        for _ in 0..3 { w.step_on(&up, Some, floors, |_| None, &[]); }
        assert_eq!((w.cursor(), w.setup[0].at), ([-30.0, 36.0], [-30.0, 30.0]));
    }

    #[test]
    fn where_left_right_switches_on_and_off_stage() {
        use crate::model::chords::logical::{DPAD_DOWN, DPAD_LEFT, DPAD_RIGHT};
        let mut w = save_wizard();
        assert_eq!(w.step(&button(DPAD_RIGHT), Some, floors), Outcome::Open);
        assert_eq!((w.page, w.setup[0].place, w.setup[0].at), (Page::Where, Place::OffStage, [-30.0, 30.0]));
        w.step(&button(DPAD_LEFT), Some, floors);
        assert_eq!((w.setup[0].place, w.setup[0].at), (Place::OnStage, [-30.0, 30.0]));
        // The CPU hangs past the floor's end: with no floor under it, it stays off stage unless the stage has a home floor.
        w.step(&button(DPAD_DOWN), Some, floors);
        assert_eq!(w.edit, Who::Cpu);
        w.step(&button(DPAD_LEFT), Some, floors);
        assert_eq!((w.setup[1].place, w.setup[1].at), (Place::OffStage, [-80.0, -11.0]));
        w.set_home(Some([0.0, 0.0]));
        w.step(&button(DPAD_LEFT), Some, floors);
        assert_eq!((w.setup[1].place, w.setup[1].at, w.cursor()), (Place::OnStage, [0.0, 0.0], [0.0, 0.0]));
        assert_eq!(w.setup[0], saved()[0]);
    }

    #[test]
    fn change_placement_role_switch_places_either_fighter() {
        use crate::model::chords::logical::{A, DPAD_DOWN, DPAD_UP, L, R};
        let mut w = save_wizard();
        for (key, who) in [(L, Who::Cpu), (R, Who::P1), (DPAD_DOWN, Who::Cpu), (DPAD_UP, Who::P1), (DPAD_UP, Who::Cpu)] {
            assert_eq!(w.step(&button(key), Some, floors), Outcome::Open);
            assert_eq!((w.page, w.edit, w.cursor()), (Page::Where, who, saved()[who.index()].at));
        }
        wizard_press(&mut w, A);
        assert_eq!((w.page, w.edit, w.cursor()), (Page::Spot, Who::Cpu, [-80.0, -11.0]));
        w.step(&button(L), Some, floors);
        assert_eq!((w.page, w.edit, w.cursor()), (Page::Spot, Who::P1, [-30.0, 30.0]));
        // On the Spot page the D-pad moves the crosshair; it does not switch the fighter.
        w.step(&button(DPAD_UP), Some, floors);
        assert_eq!(w.edit, Who::P1);
        assert_eq!(w.setup, saved());
        assert_eq!(w.edited(), [false; 2]);
    }

    #[test]
    fn both_fighters_are_placed_in_one_visit() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{A, L};
        let mut w = save_wizard();
        wizard_press(&mut w, A);
        let right = Pad { stick: [1.0, 0.0], ..Pad::default() };
        for _ in 0..5 { w.step(&right, Some, floors); }
        assert_eq!(w.setup[0].at, [-20.0, 30.0]);
        w.step(&button(L), Some, floors);
        let up = Pad { stick: [0.0, 1.0], ..Pad::default() };
        for _ in 0..3 { w.step(&up, Some, floors); }
        assert_eq!((w.setup[1].place, w.setup[1].at), (Place::OffStage, [-80.0, -5.0]));
        assert_eq!(w.edited(), [true, true]);
        assert_eq!(w.step(&button(crate::model::chords::logical::PLUS), Some, floors), Outcome::Start);
        assert_eq!(w.setup.map(|f| (f.place, f.at)), [(Place::OnStage, [-20.0, 30.0]), (Place::OffStage, [-80.0, -5.0])]);
    }

    #[test]
    fn change_placement_moves_as_break_the_targets() {
        use crate::model::btt::{self, Pad};
        use crate::model::chords::logical::{A, DPAD_LEFT, DPAD_UP};
        let mut w = Wizard::for_save(saved(), [false, false], Who::Cpu);
        wizard_press(&mut w, A);
        let mut expected = w.cursor();
        let (mut was, mut repeat) = (0, 0);
        let mut samples = vec![Pad { stick: [0.299, -0.299], ..Pad::default() }, Pad { stick: [0.3, -0.9], ..Pad::default() }];
        samples.extend(vec![Pad { held: 1 << DPAD_LEFT, ..Pad::default() }; 30]);
        samples.extend(vec![Pad { held: 1 << DPAD_UP, ..Pad::default() }; 2]);
        for pad in samples {
            let delta = btt::placing_motion(&pad, was, &mut repeat);
            expected = [expected[0] + delta[0], expected[1] + delta[1]];
            was = pad.held;
            w.step(&pad, Some, |_| None);
            assert_eq!(w.cursor().map(f32::to_bits), expected.map(f32::to_bits));
            assert_eq!(w.setup[1].at, expected);
        }
        assert_eq!(w.edited(), [false, true]);
    }

    /// "The same as Record placement": one table of pads through both wizards, the same floor read.
    #[test]
    fn change_placement_on_stage_is_record_placements_floor_rule() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{DPAD_DOWN, DPAD_RIGHT};
        let mut record = spot_wizard(saved(), None);
        let mut save = Wizard::for_save(saved(), [false, false], Who::P1);
        wizard_press(&mut save, crate::model::chords::logical::A);
        let mut pads = vec![Pad { stick: [1.0, 0.0], ..Pad::default() }; 4];
        pads.extend(vec![Pad { stick: [0.0, -1.0], ..Pad::default() }; 6]);
        pads.extend(vec![Pad { held: 1 << DPAD_RIGHT, ..Pad::default() }; 20]);
        pads.extend(vec![Pad { stick: [1.0, 1.0], ..Pad::default() }; 50]);
        pads.push(Pad { held: 1 << DPAD_DOWN, ..Pad::default() });
        for pad in pads {
            record.step(&pad, Some, floors);
            save.step(&pad, Some, floors);
            assert_eq!((save.cursor(), save.setup[0].place, save.setup[0].at), (record.cursor(), record.setup[0].place, record.setup[0].at));
        }
        // Past the floors' end the fighter stays on its last floor point.
        assert!(save.cursor()[0] > 60.0);
        assert!(save.setup[0].at[0] <= 60.0);
    }

    #[test]
    fn a_saved_platform_spot_can_be_moved_to_the_floor_under_it() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::A;
        let mut w = save_wizard();
        wizard_press(&mut w, A);
        // Nothing moves until the crosshair does: a floor read that differs from the saved height is not an edit.
        w.step(&Pad::default(), Some, |at| Some([at[0], 29.5]));
        assert_eq!((w.setup[0].at, w.edited()), ([-30.0, 30.0], [false; 2]));
        let down = Pad { stick: [0.0, -1.0], ..Pad::default() };
        for _ in 0..10 { w.step(&down, Some, floors); }
        assert_eq!((w.setup[0].place, w.setup[0].at), (Place::OnStage, [-30.0, 0.0]));
        assert_eq!(w.edited(), [true, false]);
        assert_eq!(w.edits()[0], Edits { spot: true, ..Edits::default() });
        assert_eq!(w.step(&button(crate::model::chords::logical::PLUS), Some, floors), Outcome::Start);
    }

    #[test]
    fn x_back_to_saved_leaves_nothing_edited() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{A, DPAD_LEFT, L, X};
        let mut w = save_wizard();
        w.set_home(Some([0.0, 0.0]));
        w.step(&button(L), Some, floors);
        w.step(&button(DPAD_LEFT), Some, floors);
        wizard_press(&mut w, A);
        for _ in 0..7 { w.step(&Pad { stick: [-1.0, 0.0], ..Pad::default() }, Some, floors); }
        assert_eq!(w.edited(), [false, true]);
        assert!(text(&w, "Jab start").contains("> CPU  On stage (-14, 0)"));
        w.step(&button(X), Some, floors);
        assert_eq!((w.setup, w.edited(), w.cursor()), (saved(), [false; 2], [-80.0, -11.0]));
        assert!(text(&w, "Jab start").contains("> CPU  Ledge (-80, -11)"));
        // Moved away and brought back by hand is not an edit either.
        w.step(&Pad { stick: [1.0, 0.0], ..Pad::default() }, Some, floors);
        assert_eq!(w.edited(), [false, true]);
        w.step(&Pad { stick: [-1.0, 0.0], ..Pad::default() }, Some, floors);
        assert_eq!(w.edited(), [false; 2]);
    }

    #[test]
    fn the_border_pointer_follows_the_edited_fighter() {
        use crate::model::btt;
        use crate::model::chords::logical::{A, L};
        use crate::model::panes::{EggLook, EGG_WHOLE};
        let mut w = save_wizard();
        // Player 1, on stage, is edited: nothing points at the CPU, off stage though it is.
        assert_eq!(w.marks([false; 2]).pointer, None);
        wizard_press(&mut w, A);
        assert_eq!(w.marks([false; 2]).pointer, None);
        w.step(&button(L), Some, floors);
        let marks = w.marks([false; 2]);
        assert_eq!(marks.pointer, Some([-80.0, -11.0]));
        assert_eq!(marks.looks[0], Some(([-30.0, 30.0], EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        // A fighter whose own body stands at its spot gets no mark; the crosshair stays.
        let held = w.marks([true, true]);
        assert_eq!((held.looks[0], held.looks[1]), (None, None));
        assert_eq!(held.looks[btt::LOOKS - 1], Some(w.cross.look()));
        assert_eq!(held.pointer, Some([-80.0, -11.0]));
        assert_eq!(w.marks([false, true]).looks[0], Some(([-30.0, 30.0], EggLook { scale: 0.3, ..EGG_WHOLE })));
    }

    #[test]
    fn an_old_save_opens_on_its_saved_spots() {
        use crate::model::chords::logical::{DPAD_RIGHT, L};
        let mut w = save_wizard();
        assert_eq!((w.setup, w.edited()), (saved(), [false; 2]));
        assert_eq!(text(&w, "Jab start"), "Jab start · 1/5 Where\n> P1   On stage (-30, 30)\n  CPU  Ledge (-80, -11)\nL / R fighter · ← → on / off stage · A next\nX as saved · + save · B cancel");
        // A ledge reading ends when that fighter is changed, and comes back with its saved place.
        w.set_home(Some([0.0, 0.0]));
        w.step(&button(L), Some, floors);
        w.step(&button(DPAD_RIGHT), Some, floors);
        assert!(text(&w, "Jab start").contains("> CPU  On stage (0, 0)"));
        assert_eq!(w.edited(), [false, true]);
    }

    #[test]
    fn change_placement_texts_match_the_design() {
        use crate::model::chords::logical::{A, L};
        let mut w = Wizard::for_save(wizard_defaults(), [false; 2], Who::P1);
        assert_eq!(text(&w, "Jab start"), "Jab start · 1/5 Where\n> P1   On stage (-40, 0)\n  CPU  Off stage (-118, 22)\nL / R fighter · ← → on / off stage · A next\nX as saved · + save · B cancel");
        wizard_press(&mut w, A);
        wizard_press(&mut w, L);
        assert_eq!(text(&w, "Jab start"), "Jab start · 2/5 Spot\n  P1   On stage (-40, 0)\n> CPU  Off stage (-118, 22)\nStick or D-pad move · L / R fighter · A next\nX as saved · Y on / off stage · + save · B back");
        wizard_press(&mut w, A);
        assert_eq!(text(&w, "Jab start"), "Jab start · 3/5 State\n  P1   Standard\n> CPU  Tumble\nL / R fighter · ← → change · X as saved · A next · B back · + save");
        w.setup[1].state = StartState::MissedTechUp;
        assert_eq!(text(&w, "Jab start"), "Jab start · 3/5 State\n  P1   Standard\n> CPU  Missed tech · face up\nL / R fighter · ← → change · X face up / down · A next · B back · + save");
        w.setup[1].state = StartState::Tumble;
        wizard_press(&mut w, A);
        assert_eq!(text(&w, "Jab start"), "Jab start · 4/5 Percent\n  P1   0%\n> CPU  45%\nL / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · X as saved · + save");
        wizard_press(&mut w, A);
        assert_eq!(text(&w, "Jab start"), "Jab start · 5/5 Preview\nP1   On stage (-40, 0) · Standard · 0%\nCPU  Off stage (-118, 22) · Tumble · 45%\nA / + save · B back");
        // Every page says + save, and the longest text fits the notice: a 40-byte name and four-digit spots.
        w.setup = [Fighter { at: [-1234.0, -1234.0], state: StartState::MissedTechDown, percent: 300, ..w.setup[0] },
            Fighter { at: [-1234.0, -1234.0], ..w.setup[1] }];
        for _ in 0..5 {
            let page = text(&w, &"n".repeat(40));
            assert!(page.contains("+ save") && page.len() <= 224, "{} {page}", page.len());
            wizard_press(&mut w, crate::model::chords::logical::B);
        }
        // Record placement keeps + start.
        let mut record = Wizard::new(wizard_defaults(), None);
        for _ in 0..5 {
            let mut page = String::new();
            record.write_text(Who::P1, "", &mut page).unwrap();
            assert!(page.contains("+ start") && !page.contains("save"), "{page}");
            wizard_press(&mut record, A);
        }
    }

    #[test]
    fn b_puts_back_only_the_body_the_picker_took() {
        assert_eq!(release(Leave::Cancel, true, false), Release::PutBack);
        // A body that is no longer the one the picker took is not written, woken or moved, however it is left.
        for leave in [Leave::Cancel, Leave::Other] {
            for loading in [false, true] {
                assert_eq!(release(leave, false, loading), Release::Nothing);
            }
        }
    }

    #[test]
    fn a_competing_load_ends_the_picker_without_a_put_back() {
        // A loading save places the fighters itself: the held body is only woken, B or not.
        assert_eq!(release(Leave::Cancel, true, true), Release::Wake);
        for loading in [false, true] {
            assert_eq!(release(Leave::Other, true, loading), Release::Wake);
        }
        let place = include_str!("../runtime/features/record_place.rs");
        let target = place.split("pub(super) fn load_target(").nth(1).unwrap().split("pub(super) fn load_cancelled()").next().unwrap();
        assert!(target.contains("leave(&mut engine, Leave::Other)"));
    }

    fn answer(ticket: u32, x: Option<f32>) -> Box<Answer> {
        let pair = x.map(|x| {
            let mut p1 = SavedState::never_saved();
            (p1.x, p1.fighter_kind) = (x, 4);
            [p1, SavedState::never_saved()]
        });
        Box::new(Answer { ticket, pair })
    }

    /// rv-edge-juggle-sol 2026-11-25 16:38:12Z: the answer is posted while its visit waits, the visit ends, and
    /// only then is the answer taken on a fighter frame.
    #[test]
    fn a_late_completion_for_a_dropped_ticket_is_discarded() {
        let mut posted = None;
        assert_eq!(take_answer(&mut posted, 4), None, "nothing posted");
        // Visit 4's pair is posted; visit 4 was left and visit 5 waits now. It does not inherit the old result,
        // and no visit can ever take it: visit 4 is over, and no later visit has its ticket.
        posted = Some(answer(4, Some(-30.0)));
        assert_eq!(take_answer(&mut posted, 5), None);
        assert_eq!(take_answer(&mut posted, 0), None);
        assert!(posted.as_ref().is_some_and(|left| left.ticket == 4 && left.pair.is_some()), "left for the worker to drop");
        // The visit that still waits takes its own pair, once; the box stays for the worker, emptied.
        posted = Some(answer(5, Some(12.0)));
        let pair = take_answer(&mut posted, 5).unwrap().unwrap();
        assert_eq!((pair[0].x, pair[0].fighter_kind), (12.0, 4));
        assert_eq!(take_answer(&mut posted, 5), None);
        assert!(posted.as_ref().is_some_and(|left| left.pair.is_none()));
        // A save that could not be written is an answer with no pair.
        posted = Some(answer(6, None));
        assert_eq!(take_answer(&mut posted, 6), Some(None));
        // The load is started by the picker that waits, on Player 1's frame: the worker posts the answer and never
        // asks for the load itself, so a pair a Play posted for its warp is never replaced by a placement.
        let host = include_str!("../runtime/features/recording_host.rs");
        let commit = host.split("fn place_commit(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(commit.contains("record_place::post_placed(") && !commit.contains("request_named_load("));
        let place = include_str!("../runtime/features/record_place.rs");
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        assert!(frame.find("record_place::take_answer(&mut engine.answer, ticket)").unwrap() < frame.find("states::begin_placed(").unwrap());
        assert_eq!(place.matches("states::begin_placed(").count(), 1);
    }

    #[test]
    fn a_failed_save_puts_the_same_bodies_back_and_says_so() {
        let mut text = String::new();
        write_not_saved("Jab start", &mut text).unwrap();
        assert_eq!(text, "Not saved · Jab start could not be written");
        let place = include_str!("../runtime/features/record_place.rs");
        // The worker's answer has no pair: the picker that still waits for that visit puts the fighters back.
        let failed = place.split("record_place::take_answer(&mut engine.answer, ticket)").nth(1).unwrap()
            .split("Some(None) => {").nth(1).unwrap().split("return;").next().unwrap();
        assert!(failed.find("record_place::write_not_saved(").unwrap() < failed.find("leave(&mut engine, Leave::Cancel);").unwrap());
        assert!(failed.contains("// restriction: place-save-write"));
        let host = include_str!("../runtime/features/recording_host.rs");
        let commit = host.split("fn place_commit(").nth(1).unwrap().split("\n}").next().unwrap();
        // A picker the worker could not reach still hears it: the worker posts the same text itself.
        assert!(commit.contains("Placed::Refused { ticket, name }") && commit.contains("if !super::record_place::post_placed(ticket, None)"));
        assert!(commit.contains("record_place::write_not_saved(") && commit.contains("// restriction: place-save-write"));
    }

    /// tr-primary-2 2026-11-25 16:48:42Z and 17:50Z: flat BSS, and "on stage reuses the tracing stage". The picker
    /// keeps no outline and no walk of its own, and adds no static: what it hands the worker and takes from it
    /// lies in its engine, where the worker puts, replaces and drops it and a fighter frame only reads it.
    #[test]
    fn the_picker_keeps_no_outline_of_its_own() {
        let place = include_str!("../runtime/features/record_place.rs");
        for gone in ["Surfaces", "Discovery", "NativeSource", "AtomicU32", "Mailbox", "BoxHandoff"] {
            assert!(!place.contains(gone), "{gone}");
        }
        // The statics of `main` `a945b0c1`, and no more.
        assert_eq!(place.matches("\nstatic ").count(), 12);
        // Each box has one writer, a worker function; no fighter frame empties or drops one.
        assert_eq!(place.matches("&mut engine.asked").count() + place.matches("engine.asked = ").count(), 1);
        assert_eq!(place.matches("core::mem::replace(&mut engine.answer").count() + place.matches("engine.answer = ").count(), 1);
        for worker in ["pub(crate) fn open_save(", "pub(crate) fn post_placed("] {
            let body = place.split(worker).nth(1).unwrap().split("\n}").next().unwrap();
            assert!(body.contains("worker_engine()") && body.contains("core::mem::replace(") && body.contains("drop(old);"));
            assert!(body.find("drop(engine);").unwrap() < body.find("drop(old);").unwrap(), "dropped outside the lock");
        }
        assert!(!place.contains("ENGINE.lock()"));
        let tech = include_str!("../runtime/features/tech.rs");
        let view = tech.split("struct ViewState {").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(view.contains("surfaces: Surfaces,") && view.contains("discovery: Discovery,"));
        let run = tech.split("struct Practice {").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(!run.contains("Discovery"));
    }

    #[test]
    fn both_fighters_stand_at_their_spots_while_placing() {
        use crate::model::btt;
        use crate::model::chords::logical::A;
        // Each fighter's own body is at its spot, so neither gets a mark; the crosshair is the picker's and stays.
        let mut w = save_wizard();
        wizard_press(&mut w, A);
        let marks = w.marks([true, true]);
        assert_eq!((marks.looks[0], marks.looks[1]), (None, None));
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        let place = include_str!("../runtime/features/record_place.rs");
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        assert!(frame.contains("placement::hold_at(&mut holds[who.index()]"));
        let look = place.split("pub fn look(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(look.contains("wizard.marks(held)"));
    }

    #[test]
    fn an_off_stage_fighter_is_held_at_the_crosshair() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{A, L};
        use crate::model::crosshair::{hold_step, Origin, PreviewAct, PREVIEW_SETTLE};
        let mut w = save_wizard();
        wizard_press(&mut w, A);
        w.step(&button(L), Some, floors);
        for _ in 0..4 { w.step(&Pad { stick: [-1.0, 1.0], ..Pad::default() }, Some, floors); }
        assert_eq!((w.setup[1].place, w.setup[1].at, w.cursor()), (Place::OffStage, [-88.0, -3.0], [-88.0, -3.0]));
        // The body is lifted there, then held asleep: the shared hold of the Guard the Edge and Juggler pickers.
        let mut slot = None;
        let origin = || Origin { at: [9.0, 0.0], grounded: true, facing: 1.0 };
        let acts: Vec<_> = (0..PREVIEW_SETTLE + 1).map(|_| hold_step(&mut slot, 3, w.setup[1].place == Place::OnStage, origin)).collect();
        assert_eq!(acts.last(), Some(&PreviewAct::Hold));
        assert_eq!(hold_step(&mut slot, 3, w.setup[0].place == Place::OnStage, origin), PreviewAct::Stand);
    }

    #[test]
    fn the_dotted_pointer_is_not_drawn_while_either_fighter_stands_at_its_point() {
        use crate::model::chords::logical::{A, L};
        let mut w = save_wizard();
        wizard_press(&mut w, A);
        w.step(&button(L), Some, floors);
        assert_eq!(w.marks([true, true]).pointer, Some([-80.0, -11.0]));
        // Held, the game shows the body or its own magnifier of it: the border pointer is left out.
        assert_eq!(w.marks([true, true]).shown(true).pointer, None);
        let place = include_str!("../runtime/features/record_place.rs");
        let look = place.split("pub fn look(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(look.contains(".shown(held[wizard.edit.index()] && published::hud())"));
    }

    #[test]
    fn off_stage_reaches_one_unit_inside_the_blast_line() {
        let camera = [-150.0, 150.0, 120.0, -40.0];
        let blast = [-240.0, 240.0, 190.0, -140.0];
        assert_eq!(reach(Place::OnStage, camera, Some(blast)), camera);
        assert_eq!(reach(Place::OffStage, camera, Some(blast)), [-239.0, 239.0, 189.0, -139.0]);
        assert_eq!(reach(Place::OffStage, camera, None), camera);
    }

    #[test]
    fn leaving_change_placement_puts_both_fighters_back() {
        let place = include_str!("../runtime/features/record_place.rs");
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        let closed = frame.split("Outcome::Closed => {").nth(1).unwrap().split("Outcome::Start").next().unwrap();
        assert!(closed.contains("leave(&mut engine, Leave::Cancel)"));
        let leave = place.split("unsafe fn leave(").nth(1).unwrap().split("unsafe fn end(").next().unwrap();
        assert!(leave.contains("record_place::release(") && leave.contains("states::is_loading()"));
        assert!(leave.contains("placement::restore(") && leave.contains("natives::battle_object_id("));
    }

    #[test]
    fn guard_juggler_and_placement_hold_through_one_preview_path() {
        let placement = include_str!("../runtime/features/placement.rs");
        let preview = placement.split("pub(super) unsafe fn preview(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(preview.contains("hold_at(slot, body, id, at, false)"));
        assert_eq!(placement.matches("crosshair::hold_step(").count(), 1);
        assert!(include_str!("../runtime/features/edge_juggle.rs").contains("placement::preview("));
        assert!(include_str!("../runtime/features/record_place.rs").contains("placement::hold_at("));
    }

    #[test]
    fn the_pickers_share_one_floor_read() {
        let place = include_str!("../runtime/features/record_place.rs");
        let tech = include_str!("../runtime/features/tech.rs");
        assert!(place.contains("placement::floor_under("));
        assert!(include_str!("../runtime/features/edge_juggle.rs").contains("placement::floor_under("));
        // The traced floors are Tech practice's one outline, walked by one function: Tech's own run and the picker.
        assert_eq!(tech.matches(".discovery.walk_step(&mut source, range, &mut view.surfaces)").count(), 2);
        assert!(!tech.contains("PlatformScan::new(") && !tech.contains("Bootstrap::new("));
        assert_eq!(place.matches("super::tech::floors(").count(), 1);
        // The outline is never waited for, and a floor list is given only when the walk and its platform scan
        // are both done, for this stage, form and Training session, with no Training reset since.
        let floors = tech.split("pub(super) unsafe fn floors(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(floors.contains("VIEW.try_lock()?") && !floors.contains(".lock()"));
        assert!(floors.contains("RESET.fetch_and(!RESET_OUTLINE") && floors.contains("stale || view.surfaces.epoch() != Some(epoch)"));
        assert!(floors.find("view.discovery.walked(&view.surfaces)").unwrap() < floors.find("Some(SurfaceKind::Floor)").unwrap());
        // The walk's native calls stay Tech practice's own: the picker stops at its gate, before anything else.
        let gate = floors.find("if !super::tech_adapter::tech_ready() {\n        return Some(0);").unwrap();
        assert!(gate < floors.find("VIEW.try_lock()").unwrap());
        // A Training reset marks the outline stale in the byte Tech's own reset already has: no static is added.
        let reset = tech.split("pub fn reset() {").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(reset.contains("RESET.store(RESET_RUN | RESET_OUTLINE"));
        assert_eq!(tech.matches("\nstatic ").count(), 16);
    }

    #[test]
    fn change_placement_runs_wherever_practice_is_live() {
        let place = include_str!("../runtime/features/record_place.rs");
        assert!(!place.contains("is_native_training") && !place.contains("lobby_active"));
        let frame = place.split("pub unsafe fn frame(").nth(1).unwrap().split("pub fn look(").next().unwrap();
        let open = frame.split("OPEN.swap(0").nth(1).unwrap();
        assert!(open.contains("scene::current().is_some()") && open.contains("engine.asked.as_deref()"));
    }

    #[test]
    fn a_shared_start_save_asks_change_or_copy() {
        use crate::model::chords::logical::{A, B, DPAD_DOWN, DPAD_UP, PLUS, X};
        let shared = Shared { macros: 2, from_macro: true };
        let mut w = save_wizard().sharing(shared, SharedStart::Prompt);
        assert_eq!((w.page, w.copies()), (Page::Ask, false));
        assert_eq!(text(&w, "Jab start"),
            "Jab start · 2 macros start here\n> Change it for all 2 macros\n  Copy it for this macro\n↑ ↓ choose · A next · B cancel");
        // Two choices and B: + does not save from the question, and no other key acts on it.
        for key in [PLUS, X] {
            assert_eq!(wizard_press(&mut w, key), Outcome::Open);
            assert_eq!((w.page, w.copies(), w.setup), (Page::Ask, false, saved()));
        }
        wizard_press(&mut w, DPAD_DOWN);
        assert!(w.copies());
        assert!(text(&w, "Jab start").contains("\n  Change it for all 2 macros\n> Copy it for this macro\n"));
        wizard_press(&mut w, DPAD_UP);
        assert!(!w.copies());
        wizard_press(&mut w, DPAD_DOWN);
        assert_eq!(wizard_press(&mut w, A), Outcome::Open);
        assert_eq!((w.page, w.copies()), (Page::Where, true));
        // B on the first page cancels; the question is not asked again on the way back.
        assert_eq!(wizard_press(&mut w, B), Outcome::Closed);
        let mut cancelled = save_wizard().sharing(shared, SharedStart::Prompt);
        assert_eq!(wizard_press(&mut cancelled, B), Outcome::Closed);
        // From the save's own screen a copy is a new save that no macro uses; one macro reads in the singular.
        let own = save_wizard().sharing(Shared { macros: 1, from_macro: false }, SharedStart::Prompt);
        assert_eq!(text(&own, "Jab start"),
            "Jab start · 1 macro starts here\n> Change it for its macro\n  Copy it to a new save\n↑ ↓ choose · A next · B cancel");
        let long = save_wizard().sharing(Shared { macros: 255, from_macro: true }, SharedStart::Prompt);
        assert!(text(&long, &"n".repeat(40)).len() <= 224);
    }

    #[test]
    fn always_change_and_always_copy_skip_the_question() {
        let shared = Shared { macros: 3, from_macro: true };
        let change = save_wizard().sharing(shared, SharedStart::Change);
        assert_eq!((change.page, change.copies()), (Page::Where, false));
        let copy = save_wizard().sharing(shared, SharedStart::Copy);
        assert_eq!((copy.page, copy.copies()), (Page::Where, true));
        // A save that no macro starts from is changed, whatever the setting says.
        for rule in [SharedStart::Prompt, SharedStart::Change, SharedStart::Copy] {
            let alone = save_wizard().sharing(Shared { macros: 0, from_macro: false }, rule);
            assert_eq!((alone.page, alone.copies()), (Page::Where, false));
        }
        assert_eq!(SharedStart::default(), SharedStart::Prompt);
    }

    /// A save's two halves, as an old file holds them: Player 1 standing, the CPU on a ledge at 6.3 %.
    fn saved_pair() -> [SavedState; 2] {
        let mut p1 = SavedState::never_saved();
        (p1.x, p1.y, p1.percent, p1.lr, p1.fighter_kind, p1.saved_stage) = (-30.0, 30.0, 12.0, -1.0, 4, Some(7));
        let mut cpu = SavedState::never_saved();
        (cpu.x, cpu.y, cpu.percent, cpu.situation_kind, cpu.fighter_kind, cpu.saved_stage) = (-80.0, -11.0, 6.3, SITUATION_CLIFF, 9, Some(7));
        [p1, cpu]
    }

    fn opened(pair: &[SavedState; 2]) -> Wizard {
        let [p1, cpu] = [saved_fighter(&pair[0]), saved_fighter(&pair[1])];
        Wizard::for_save([p1.0, cpu.0], [p1.1, cpu.1], Who::P1)
    }

    fn patched(pair: &[SavedState; 2], w: &Wizard, stage: i32) -> [SavedState; 2] {
        let [mut p1, mut cpu] = pair.clone();
        patch([&mut p1, &mut cpu], &w.setup, w.edits(), stage);
        [p1, cpu]
    }

    #[test]
    fn a_save_opens_as_the_file_holds_it() {
        let pair = saved_pair();
        assert_eq!(saved_fighter(&pair[0]), (Fighter { place: Place::OnStage, at: [-30.0, 30.0], state: StartState::Standard, percent: 12 }, false));
        // A ledge save is off stage and reads Ledge; its damage shows rounded, and the file keeps its own.
        assert_eq!(saved_fighter(&pair[1]), (Fighter { place: Place::OffStage, at: [-80.0, -11.0], state: StartState::Standard, percent: 6 }, true));
        let mut air = pair[0].clone();
        (air.situation_kind, air.start_state, air.percent) = (SITUATION_AIR, StartState::Tumble, 999.0);
        assert_eq!(saved_fighter(&air), (Fighter { place: Place::OffStage, at: [-30.0, 30.0], state: StartState::Tumble, percent: 300 }, false));
        assert!(text(&opened(&pair), "Jab start").contains("  CPU  Ledge (-80, -11)"));
    }

    #[test]
    fn a_saved_placement_rewrites_only_the_moved_fighters_spot() {
        use crate::model::btt::Pad;
        use crate::model::chords::logical::{A, PLUS};
        let pair = saved_pair();
        let mut w = opened(&pair);
        wizard_press(&mut w, A);
        for _ in 0..10 { w.step(&Pad { stick: [0.0, -1.0], ..Pad::default() }, Some, floors); }
        assert_eq!(wizard_press(&mut w, PLUS), Outcome::Start);
        let [p1, cpu] = patched(&pair, &w, 31);
        assert_eq!((p1.x, p1.y, p1.situation_kind, p1.saved_stage), (-30.0, 0.0, SITUATION_GROUND, Some(31)));
        // The fighter that was not edited keeps its whole record: its ledge, its stage and its 6.3 %.
        assert_eq!(cpu, pair[1]);
        assert_eq!(cpu.percent.to_bits(), 6.3f32.to_bits());
    }

    #[test]
    fn a_placement_save_keeps_every_other_field_of_the_file() {
        use crate::model::chords::logical::{A, DPAD_RIGHT, L};
        let pair = saved_pair();
        let mut w = opened(&pair);
        w.set_home(Some([0.0, 0.0]));
        // Player 1 goes off stage; the CPU comes off its ledge onto the stage.
        wizard_press(&mut w, DPAD_RIGHT);
        w.step(&button(L), Some, floors);
        w.step(&button(DPAD_RIGHT), Some, floors);
        wizard_press(&mut w, A);
        let [p1, cpu] = patched(&pair, &w, 31);
        assert_eq!((p1.x, p1.y, p1.situation_kind, p1.saved_stage), (-30.0, 30.0, SITUATION_AIR, Some(31)));
        assert_eq!((cpu.x, cpu.y, cpu.situation_kind, cpu.saved_stage), (0.0, 0.0, SITUATION_GROUND, Some(31)));
        for (now, was) in [(p1, &pair[0]), (cpu, &pair[1])] {
            let kept = SavedState { x: was.x, y: was.y, situation_kind: was.situation_kind, saved_stage: was.saved_stage, ..now };
            assert_eq!(&kept, was);
        }
    }

    #[test]
    fn an_untouched_ledge_fighter_keeps_its_record_on_another_stage() {
        use crate::model::chords::logical::{A, DPAD_RIGHT};
        let pair = saved_pair();
        // Opened on another stage, the ledge spot is shown fitted to that stage's range. That is not an edit.
        let [p1, cpu] = [saved_fighter(&pair[0]), saved_fighter(&pair[1])];
        let mut w = Wizard::for_save([p1.0, Fighter { at: [-60.0, -11.0], ..cpu.0 }], [p1.1, cpu.1], Who::P1);
        assert_eq!(w.edited(), [false; 2]);
        for _ in 0..2 { wizard_press(&mut w, A); }
        wizard_press(&mut w, DPAD_RIGHT);
        let [p1, cpu] = patched(&pair, &w, 31);
        assert_eq!(cpu, pair[1]);
        // A state edit alone writes the state and nothing else: the spot and its stage stay as saved.
        assert_eq!(p1, SavedState { start_state: StartState::MissedTechUp, ..pair[0].clone() });
    }

    #[test]
    fn plus_from_every_page_saves_only_what_was_edited() {
        use crate::model::chords::logical::{A, DPAD_UP, L, PLUS};
        let pair = saved_pair();
        // + on every page with nothing touched, the Percent page visited: the file is unchanged, 6.3 % included.
        let mut w = opened(&pair);
        for _ in 0..5 {
            assert_eq!(wizard_press(&mut w, PLUS), Outcome::Start);
            assert_eq!(patched(&pair, &w, 31), pair);
            wizard_press(&mut w, A);
        }
        // A damage edit alone writes that fighter's damage and nothing else.
        let mut w = opened(&pair);
        for _ in 0..3 { wizard_press(&mut w, A); }
        assert_eq!(w.page, Page::Percent);
        w.step(&button(L), Some, floors);
        wizard_press(&mut w, DPAD_UP);
        assert_eq!(wizard_press(&mut w, PLUS), Outcome::Start);
        let [p1, cpu] = patched(&pair, &w, 31);
        assert_eq!(p1, pair[0]);
        assert_eq!(cpu, SavedState { percent: 16.0, ..pair[1].clone() });
    }

    #[test]
    fn a_confirmed_placement_carries_its_ticket_and_the_answer() {
        use crate::model::chords::logical::{A, DPAD_DOWN, DPAD_RIGHT};
        let mut w = save_wizard().sharing(Shared { macros: 2, from_macro: true }, SharedStart::Prompt);
        wizard_press(&mut w, DPAD_DOWN);
        wizard_press(&mut w, A);
        wizard_press(&mut w, DPAD_RIGHT);
        let commit = w.commit(9, 31);
        assert_eq!(commit, Commit { ticket: 9, copy: true, setup: w.setup, edits: w.edits(), stage: 31, macros: 2 });
        assert_eq!(commit.edits[0], Edits { spot: true, ..Edits::default() });
    }

    #[test]
    fn place_refusals_name_the_fighter_and_the_state() {
        for (who, label) in [(Who::P1, "P1"), (Who::Cpu, "CPU")] {
            for state in [StartState::Standard, StartState::Tumble, StartState::Helpless, StartState::Footstool,
                StartState::MissedTechUp, StartState::MissedTechDown, StartState::Trip]
            {
                for (recorded, prefix) in [(true, "Not recorded"), (false, "Not placed")] {
                    let mut out = String::new();
                    write_refusal(recorded, who, state, &mut out).unwrap();
                    assert_eq!(out, format!("{prefix} · {label} could not start in {}", state.title()));
                }
            }
        }
        let mut ready = String::new();
        write_ready(3, &mut ready).unwrap();
        assert_eq!(ready, "Ready · 3");
    }

    #[test]
    fn place_defaults_take_each_fighter_where_it_is() {
        let live = [([1.0, 2.0], 34.6, Some([1.0, 0.0])), ([500.0, 40.0], 999.0, None)];
        let setup = defaults(live, |at| Some([at[0].clamp(-200.0, 200.0), at[1]])).unwrap();
        assert_eq!(setup, [
            Fighter { place: Place::OnStage, at: [1.0, 0.0], state: StartState::Standard, percent: 35 },
            Fighter { place: Place::OffStage, at: [200.0, 40.0], state: StartState::Standard, percent: 300 },
        ]);
        assert!(defaults(live, |_| None).is_none());
    }

    #[test]
    fn a_fighter_already_on_its_spot_is_still_written_and_starts_at_its_percent() {
        let mut plans = plans();
        plans[0].state = StartState::Standard;
        let mut sync = Sync::new(plans, 2);
        initial(&mut sync);
        let mut wrong = reports(plans);
        wrong[0].as_mut().unwrap().damage = 0.0;
        for i in 0..=CHECK_FRAMES {
            let order = sync.tick(wrong, true);
            assert!(!order.live);
            if i < CHECK_FRAMES {
                assert_eq!(order.acts[0], Act::Write);
                assert!(order.end.is_none());
            } else {
                assert_eq!(order.end, Some(End::Refused(Who::P1)));
            }
        }
        let mut sync = Sync::new(plans, 2);
        initial(&mut sync);
        for _ in 0..2 {
            assert!(sync.tick(reports(plans), true).count.is_some());
        }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);
        let confirmed = sync.tick(started(plans), true);
        assert!(confirmed.live);
        assert_eq!(confirmed.end, Some(End::Started));
        assert_eq!(sync.plan(Who::P1).percent, Some(80.0));
    }

    #[test]
    fn place_start_reads_back_the_exact_spot_and_percent() {
        let plans = plans();
        for case in 0..4 {
            let mut sync = Sync::new(plans, READY_FRAMES);
            initial(&mut sync);
            let mut seen = reports(plans);
            let p1 = seen[0].as_mut().unwrap();
            match case {
                0 => p1.at[0] += 0.75,
                1 => p1.at[1] += 0.75,
                2 => p1.at[0] = f32::from_bits(p1.at[0].to_bits() + 1),
                _ => p1.damage += 1.0,
            }
            for _ in 0..5 {
                let order = sync.tick(seen, true);
                assert_eq!(order.acts[0], Act::Write);
                assert_eq!(order.count, None);
                assert!(!order.live && order.end.is_none());
            }
        }
    }

    #[test]
    fn place_start_holds_airborne_fighters_through_ready() {
        let plans = plans();
        let mut sync = Sync::new(plans, 3);
        initial(&mut sync);
        for _ in 0..3 {
            let order = sync.tick(reports(plans), true);
            assert_eq!(order.acts, [Act::Write, Act::Hold]);
            assert!(order.count.is_some());
        }
    }

    #[test]
    fn place_start_counts_only_while_both_are_placed() {
        let plans = plans();
        let mut sync = Sync::new(plans, READY_FRAMES);
        initial(&mut sync);
        for frame in 0..READY_FRAMES {
            if frame == 25 {
                let mut displaced = reports(plans);
                displaced[1].as_mut().unwrap().at[1] += 1.0;
                let order = sync.tick(displaced, true);
                assert_eq!(order.acts[1], Act::Write);
                assert!(!order.live);
            }
            let order = sync.tick(reports(plans), true);
            assert_eq!(order.count, Some(3 - (frame / 20) as u8));
            assert!(!order.live && order.end.is_none());
        }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);
    }

    #[test]
    fn live_is_given_only_on_the_tick_that_confirms_both_states() {
        let plans = plans();
        let mut sync = Sync::new(plans, 2);
        initial(&mut sync);
        for _ in 0..2 {
            let order = sync.tick(reports(plans), true);
            assert!(!order.live && order.end.is_none());
        }
        let request = sync.tick(reports(plans), true);
        assert_eq!(request.acts, [Act::Request; 2]);
        assert!(!request.live && request.end.is_none());
        let confirmed = sync.tick(started(plans), true);
        assert!(confirmed.live);
        assert_eq!(confirmed.end, Some(End::Started));
    }

    #[test]
    fn a_state_not_entered_on_the_request_frame_refuses_with_no_live() {
        let plans = plans();
        for who in [Who::P1, Who::Cpu] {
            let mut sync = Sync::new(plans, 0);
            initial(&mut sync);
            let request = sync.tick(reports(plans), true);
            assert_eq!(request.acts, [Act::Request; 2]);
            assert!(!request.live);
            let mut seen = started(plans);
            seen[who.index()].as_mut().unwrap().status = Status::Standard;
            let refusal = sync.tick(seen, true);
            assert!(!refusal.live);
            assert_eq!(refusal.end, Some(End::Refused(who)));
        }
    }

    #[test]
    fn a_report_from_a_later_frame_does_not_confirm() {
        let plans = plans();
        for who in [Who::P1, Who::Cpu] {
            let mut sync = Sync::new(plans, 0);
            request_both(&mut sync, plans);
            let mut seen = started(plans);
            seen[who.index()].as_mut().unwrap().requested = false;
            let order = sync.tick(seen, true);
            assert!(!order.live);
            assert_eq!(order.end, Some(End::Refused(who)));
        }
    }

    #[test]
    fn a_missing_report_on_the_confirm_tick_refuses_that_fighter() {
        let plans = plans();
        for who in [Who::P1, Who::Cpu] {
            let mut sync = Sync::new(plans, 0);
            request_both(&mut sync, plans);
            let mut seen = started(plans);
            seen[who.index()] = None;
            let order = sync.tick(seen, true);
            assert!(!order.live);
            assert_eq!(order.end, Some(End::Refused(who)));
        }
    }

    #[test]
    fn the_wrong_spot_percent_situation_or_face_on_the_request_frame_refuses() {
        let mut plans = plans();
        plans[0].state = StartState::MissedTechDown;
        for case in 0..4 {
            let mut sync = Sync::new(plans, 0);
            request_both(&mut sync, plans);
            let mut seen = started(plans);
            let p1 = seen[0].as_mut().unwrap();
            match case {
                0 => p1.at[0] += 1.0,
                1 => p1.damage += 1.0,
                2 => p1.grounded = Some(false),
                _ => p1.face_up = true,
            }
            let order = sync.tick(seen, true);
            assert!(!order.live);
            assert_eq!(order.end, Some(End::Refused(Who::P1)));
        }
    }

    #[test]
    fn a_missed_tech_confirmation_requires_the_asked_bound_motion_for_both_fighters() {
        for who in [Who::P1, Who::Cpu] {
            for state in [StartState::MissedTechUp, StartState::MissedTechDown] {
                let mut plans = plans();
                plans[who.index()].state = state;
                let mut sync = Sync::new(plans, 0);
                request_both(&mut sync, plans);
                let mut seen = started(plans);
                let report = seen[who.index()].as_mut().unwrap();
                report.motion = if state == StartState::MissedTechUp { DOWN_BOUND_D } else { DOWN_BOUND_U };
                let refused = sync.tick(seen, true);
                assert!(!refused.live);
                assert_eq!(refused.end, Some(End::Refused(who)));

                let mut sync = Sync::new(plans, 0);
                request_both(&mut sync, plans);
                let confirmed = sync.tick(started(plans), true);
                assert!(confirmed.live);
                assert_eq!(confirmed.end, Some(End::Started));
            }
        }
    }

    #[test]
    fn place_start_waits_for_ready_inside_the_limit() {
        let plans = plans();
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        for _ in 0..CHECK_FRAMES {
            let order = sync.tick(reports(plans), false);
            assert!(!order.acts.contains(&Act::Request));
            assert!(order.end.is_none() && !order.live);
        }
        assert_eq!(sync.tick(reports(plans), false).end, Some(End::Refused(Who::P1)));
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        for _ in 0..3 { assert!(sync.tick(reports(plans), false).end.is_none()); }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);
    }

    #[test]
    fn place_start_does_not_advance_without_a_game_frame() {
        let plans = plans();
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        assert_eq!(sync.tick([None, None], true).acts, [Act::Write; 2]);
        let request = sync.tick(reports(plans), true);
        assert_eq!(request.acts, [Act::Request; 2]);
        for _ in 0..3 {
            let repeated = sync.tick([None, None], true);
            assert_eq!(repeated.acts, request.acts);
            assert!(!repeated.live);
        }
        let confirmed = sync.tick(started(plans), true);
        assert!(confirmed.live);
        assert_eq!(confirmed.end, Some(End::Started));
        for _ in 0..3 {
            let repeated = sync.tick([None, None], true);
            assert_eq!(repeated.acts, [Act::None; 2]);
            assert!(!repeated.live && repeated.end == Some(End::Started));
        }
        assert_eq!(sync.tick(started(plans), true).end, Some(End::Started));
    }

    #[test]
    fn place_start_waits_for_both_bodies_to_be_usable() {
        let plans = plans();
        let mut absent = reports(plans);
        absent[1].as_mut().unwrap().usable = false;
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        for _ in 0..CHECK_FRAMES {
            let order = sync.tick(absent, true);
            assert_eq!(order.acts[1], Act::Write);
            assert!(order.end.is_none());
        }
        assert_eq!(sync.tick(absent, true).end, Some(End::Refused(Who::Cpu)));
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        for _ in 0..3 { assert!(sync.tick(absent, true).end.is_none()); }
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);
        let confirmed = sync.tick(started(plans), true);
        assert!(confirmed.live);
        assert_eq!(confirmed.end, Some(End::Started));
        let mut sync = Sync::new(plans, 0);
        initial(&mut sync);
        let mut missing = reports(plans);
        missing[1] = None;
        let order = sync.tick(missing, true);
        assert_eq!(order.acts[1], Act::Write);
        assert!(!order.live && order.end.is_none());
        assert_eq!(sync.tick(reports(plans), true).acts, [Act::Request; 2]);
    }
    }
}

#[cfg(test)]
#[path = "record_place/switch_tests.rs"]
mod switch_tests;
