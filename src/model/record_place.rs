use super::btt;
use super::chords::logical;
use super::cpu::ids::C;
use super::crosshair::{Crosshair, Marks};
use super::panes::{EggLook, EGG_WHOLE};
use super::state::record::StartState;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page { Spot, State, Percent, Preview }

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
        match shown.page { Page::Spot => "spot", Page::State => "state", Page::Percent => "percent", Page::Preview => "preview" }
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

pub struct Wizard {
    pub page: Page,
    pub edit: Who,
    pub setup: Setup,
    defaults: Setup,
    cross: Crosshair,
    was: u16,
    repeat: u8,
}

impl Wizard {
    pub fn new(defaults: Setup, remembered: Option<Setup>) -> Self {
        let setup = remembered.unwrap_or(defaults);
        Self { page: Page::Spot, edit: Who::P1, setup, defaults,
            cross: Crosshair::new(setup[0].at), was: 0, repeat: 0 }
    }

    pub fn cursor(&self) -> [f32; 2] {
        self.cross.at
    }

    pub fn step(&mut self, pad: &btt::Pad, fit: impl Fn([f32; 2]) -> Option<[f32; 2]>,
        floor: impl Fn([f32; 2]) -> Option<[f32; 2]>) -> Outcome
    {
        let was = self.was;
        self.was = pad.held;
        let pressed = |key| pad.pressed & (1u16 << key) != 0;
        if pressed(logical::PLUS) { return Outcome::Start; }
        if pressed(logical::B) {
            self.page = match self.page {
                Page::Spot => return Outcome::Closed,
                Page::State => Page::Spot,
                Page::Percent => Page::State,
                Page::Preview => Page::Percent,
            };
            return Outcome::Open;
        }
        if pressed(logical::A) {
            self.page = match self.page {
                Page::Spot => Page::State,
                Page::State => Page::Percent,
                Page::Percent => Page::Preview,
                Page::Preview => return Outcome::Start,
            };
            return Outcome::Open;
        }
        if self.page != Page::Preview && (pressed(logical::L) || pressed(logical::R)) {
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
            match self.setup[i].place {
                Place::OnStage => {
                    self.setup[i].place = Place::OffStage;
                    self.setup[i].at = self.cross.at;
                    self.setup[i].state = StartState::Standard;
                }
                Place::OffStage => {
                    if let Some(at) = floor(self.cross.at) {
                        self.setup[i].place = Place::OnStage;
                        self.setup[i].at = at;
                        self.setup[i].state = StartState::Standard;
                    }
                }
            }
            return Outcome::Open;
        }
        match self.page {
            Page::Spot => {
                self.cross.step(pad, fit);
                if self.setup[i].place == Place::OffStage {
                    self.setup[i].at = self.cross.at;
                } else if let Some(at) = floor(self.cross.at) {
                    self.setup[i].at = at;
                }
            }
            Page::State => {
                let direction = i32::from(pressed(logical::DPAD_RIGHT)) - i32::from(pressed(logical::DPAD_LEFT));
                if direction != 0 {
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
            Page::Preview => {}
        }
        Outcome::Open
    }

    pub fn marks(&self) -> Marks {
        let mut looks = [None; btt::LOOKS];
        for who in [Who::P1, Who::Cpu] {
            let i = who.index();
            let fighter = self.setup[i];
            let editing_spot = self.page == Page::Spot && self.edit == who;
            if !editing_spot || fighter.place == Place::OnStage {
                let scale = if editing_spot || who == Who::P1 { 0.3 } else { EGG_WHOLE.scale };
                looks[i] = Some((fighter.at, EggLook { scale, ..EGG_WHOLE }));
            }
        }
        if self.page == Page::Spot {
            looks[btt::LOOKS - 1] = Some(self.cross.look());
        }
        let pointer = looks[Who::Cpu.index()].map(|(at, _)| at);
        Marks { looks, pointer }
    }

    pub fn write_text(&self, recorded: Who, out: &mut impl core::fmt::Write) -> core::fmt::Result {
        let (number, title) = match self.page {
            Page::Spot => (1, "Spot"), Page::State => (2, "State"),
            Page::Percent => (3, "Percent"), Page::Preview => (4, "Preview"),
        };
        writeln!(out, "Record ({}) · {number}/4 {title}", recorded.name())?;
        for who in [Who::P1, Who::Cpu] {
            let fighter = self.setup[who.index()];
            let name = if who == Who::P1 { "P1  " } else { "CPU " };
            if self.page != Page::Preview {
                write!(out, "{} {name} ", if self.edit == who { ">" } else { " " })?;
            } else {
                write!(out, "{name} ")?;
            }
            match self.page {
                Page::Spot | Page::Preview => {
                    let place = if fighter.place == Place::OnStage { "On stage" } else { "Off stage" };
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
            Page::Spot => write!(out, "L / R fighter · Y on / off stage · A next · B back\nX defaults · + start"),
            Page::State => {
                let face = matches!(self.setup[self.edit.index()].state, StartState::MissedTechUp | StartState::MissedTechDown);
                let x = if face { "X face up / down" } else { "X defaults" };
                write!(out, "L / R fighter · ← → change · {x} · A next · B back · + start")
            }
            Page::Percent => write!(out, "L / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · X defaults · + start"),
            Page::Preview => write!(out, "A / + start · B back"),
        }
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
        let restore = named.split("fn begin_restore()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(restore.find("rr_host::previewing()").unwrap() < restore.find("source_is_named()").unwrap());
        let clear = named.split("fn clear_session()").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(clear.contains("rr_host::drop_preview_home()"));
        let hold = named.split("pub unsafe fn hold_panel_fighter(").nth(1).unwrap().split("\n}").next().unwrap();
        assert!(hold.contains("clip_previewing"));
        let worker = include_str!("../runtime/ui/worker.rs");
        let clip = worker.split("if s.phase == Phase::Clip {").nth(1).unwrap().split("\n    }").next().unwrap();
        let store = clip.split("EXT_SHOWN.store(").nth(1).unwrap().split(';').next().unwrap();
        assert!(store.contains("hides_page()"));
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
        assert!(cancel.contains("key::B") && cancel.contains("leave(&mut engine)"));
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

    #[test]
    fn place_wizard_pages_in_order() {
        use crate::model::chords::logical::{A, B, PLUS};
        let mut w = Wizard::new(wizard_defaults(), None);
        assert_eq!((w.page, w.edit, w.cursor()), (Page::Spot, Who::P1, [-40.0, 0.0]));
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(setup, None);
        let marks = w.marks();
        assert_eq!(marks.looks[0], Some((setup[0].at, EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[1], Some((setup[1].at, EGG_WHOLE)));
        assert_eq!(marks.pointer, Some(setup[1].at));
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        wizard_press(&mut w, L);
        let marks = w.marks();
        assert_eq!(marks.looks[0], Some((setup[0].at, EggLook { scale: 0.3, ..EGG_WHOLE })));
        assert_eq!(marks.looks[1], None);
        assert_eq!(marks.pointer, None);
        assert_eq!(marks.looks[btt::LOOKS - 1], Some(w.cross.look()));
        for _ in 0..3 { wizard_press(&mut w, A); }
        assert_eq!(w.setup, setup);
        let marks = w.marks();
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
        let mut w = Wizard::new(setup, None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
            assert!(w.marks().looks.iter().flatten().any(|(at, look)|
                *at == w.setup[0].at && *look == EggLook { scale: 0.3, ..EGG_WHOLE }));
        }
    }

    #[test]
    fn place_on_stage_mark_takes_the_floor_under_the_crosshair() {
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(setup, None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
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
        let mut w = Wizard::new(wizard_defaults(), None);
        assert_eq!(wizard_press(&mut w, B), Outcome::Closed);
    }

    #[test]
    fn place_wizard_x_puts_both_fighters_back_to_the_defaults() {
        use crate::model::chords::logical::{A, L, X};
        let defaults = wizard_defaults();
        let mut remembered = defaults;
        remembered[0].at = [9.0, 3.0];
        remembered[1].percent = 123;
        let mut w = Wizard::new(defaults, Some(remembered));
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
            w.write_text(recorded, &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 1/4 Spot\n> P1   On stage (-40, 0)\n  CPU  Off stage (-118, 22)\nL / R fighter · Y on / off stage · A next · B back\nX defaults · + start"));
        }
        wizard_press(&mut w, A);
        let mut ordinary = String::new();
        w.write_text(Who::P1, &mut ordinary).unwrap();
        assert_eq!(ordinary, "Record (P1) · 2/4 State\n> P1   Standard\n  CPU  Tumble\nL / R fighter · ← → change · X defaults · A next · B back · + start");
        wizard_press(&mut w, L);
        w.setup[1].state = StartState::MissedTechUp;
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 2/4 State\n  P1   Standard\n> CPU  Missed tech · face up\nL / R fighter · ← → change · X face up / down · A next · B back · + start"));
        }
        wizard_press(&mut w, A);
        wizard_press(&mut w, L);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 3/4 Percent\n> P1   0%\n  CPU  45%\nL / R fighter · ← → 1 · ↑ ↓ 10 · A next · B back · X defaults · + start"));
        }
        w.setup[1].state = StartState::Tumble;
        wizard_press(&mut w, A);
        for recorded in [Who::P1, Who::Cpu] {
            let header = if recorded == Who::P1 { "P1" } else { "CPU" };
            let mut out = String::new();
            w.write_text(recorded, &mut out).unwrap();
            assert_eq!(out, format!("Record ({header}) · 4/4 Preview\nP1   On stage (-40, 0) · Standard · 0%\nCPU  Off stage (-118, 22) · Tumble · 45%\nA / + start · B back"));
        }
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
