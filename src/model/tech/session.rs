//! Native-result grading and game-frame session/respawn scheduling (DESIGN §2/6).
use core::fmt::{self, Write};

pub const HOLD: u16 = 45;
pub const LATE: u16 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    InPlace,
    RollLeft,
    RollRight,
    Wall,
    WallJump,
    Ceiling,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Perfect(Option<u16>),
    Held,
    Early(u16),
    Locked(u16),
    Late(u16),
    Missed,
    AfterHitstun,
    Untechable,
    NoContact,
}
impl Grade {
    pub fn success(self) -> bool {
        matches!(self, Self::Perfect(_) | Self::Held)
    }
    pub fn counted(self) -> bool {
        !matches!(self, Self::Untechable | Self::NoContact)
    }
    pub fn write(self, out: &mut impl Write) -> fmt::Result {
        match self {
            Self::Perfect(Some(n)) => write!(out, "Perfect · {n}f"),
            // restriction: tech-calibration-unmeasured
            Self::Perfect(None) => out.write_str("Perfect · timing unavailable"),
            Self::Held => out.write_str("Tech (held)"),
            Self::Early(n) => write!(out, "Early by {n}f"),
            Self::Locked(n) => write!(out, "Locked out (2nd press {n}f after 1st)"),
            Self::Late(n) => write!(out, "Late by {n}f"),
            Self::Missed => out.write_str("Missed · no press"),
            Self::AfterHitstun => out.write_str("Missed (after hitstun)"),
            Self::Untechable => out.write_str("Untechable (not counted)"),
            Self::NoContact => out.write_str("No contact (not counted)"),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Decision {
    pub tech: Option<Direction>,
    pub count: u16,
    pub previous: u16,
    pub window: f32,
    pub lockout: u16,
    pub reaction: f32,
    pub damage_fall: bool,
    pub enabled: bool,
    /// V1 native press-frame calibration supplied by runtime, never guessed here.
    pub press_frame_offset: Option<u16>,
}
/// None waits for a new press for up to LATE frames after contact.
pub fn grade(d: Decision) -> Option<Grade> {
    if d.tech.is_some() {
        return Some(if d.reaction <= 0.0 || d.damage_fall {
            Grade::Held
        } else {
            Grade::Perfect(
                d.press_frame_offset
                    .map(|offset| d.count.saturating_add(offset)),
            )
        });
    }
    if !d.enabled {
        return Some(Grade::Untechable);
    }
    if d.damage_fall || d.reaction <= 0.0 {
        return Some(Grade::AfterHitstun);
    }
    let count = f32::from(d.count);
    if count < d.window && d.previous < d.lockout {
        return Some(Grade::Locked(d.previous));
    }
    if count >= d.window && count < d.window + f32::from(d.lockout) {
        // Count advances in whole frames; ceil(window) is the first early count.
        return Some(Grade::Early((count - d.window.ceil()) as u16 + 1));
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    Attempts(u16),
    Frames(u32),
    Free,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Off,
    Place,
    Delay(u16),
    Flight,
    Hold(u16),
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Place { reroll: bool },
    Launch,
    Summary,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Summary {
    pub attempts: u32,
    pub perfect: u32,
    pub held: u32,
    pub early: u32,
    pub late: u32,
    pub locked: u32,
    pub missed: u32,
    pub streak: u32,
    pub best: u32,
    pub offset_sum: u32,
    pub timed_perfect: u32,
}
impl Summary {
    pub fn record(&mut self, grade: Grade) {
        if !grade.counted() {
            return;
        }
        self.attempts += 1;
        if grade.success() {
            self.streak += 1;
            self.best = self.best.max(self.streak);
        } else {
            self.streak = 0;
        }
        match grade {
            Grade::Perfect(n) => {
                self.perfect += 1;
                if let Some(n) = n {
                    self.offset_sum += u32::from(n);
                    self.timed_perfect += 1;
                }
            }
            Grade::Held => self.held += 1,
            Grade::Early(_) => self.early += 1,
            Grade::Late(_) => self.late += 1,
            Grade::Locked(_) => self.locked += 1,
            _ => self.missed += 1,
        }
    }
    pub fn average(&self) -> Option<f32> {
        (self.timed_perfect > 0).then(|| self.offset_sum as f32 / self.timed_perfect as f32)
    }
    pub fn write(&self, out: &mut impl Write) -> fmt::Result {
        write!(
            out,
            "Done! {}: {} Perfect · {} Held · {} Early · {} Late · {} Missed · best {}",
            self.attempts,
            self.perfect,
            self.held,
            self.early,
            self.late,
            self.missed + self.locked,
            self.best
        )?;
        if let Some(avg) = self.average() {
            write!(out, " · avg {avg:.1}f")?;
        }
        Ok(())
    }
}
pub struct Session {
    pub phase: Phase,
    pub summary: Summary,
    pub result: Option<Grade>,
    pub length: Length,
    pub elapsed: u32,
    delay: u16,
    pending: bool,
}
impl Session {
    pub const IDLE: Self = Self {
        phase: Phase::Off,
        summary: Summary {
            attempts: 0,
            perfect: 0,
            held: 0,
            early: 0,
            late: 0,
            locked: 0,
            missed: 0,
            streak: 0,
            best: 0,
            offset_sum: 0,
            timed_perfect: 0,
        },
        result: None,
        length: Length::Free,
        elapsed: 0,
        delay: 30,
        pending: false,
    };
    pub fn start(&mut self, length: Length) -> Action {
        *self = Self {
            phase: Phase::Place,
            length,
            ..Self::IDLE
        };
        Action::Place { reroll: true }
    }
    pub fn placed(&mut self, delay: u16) {
        self.delay = delay;
        self.phase = Phase::Delay(delay);
    }
    pub fn stop(&mut self) -> Action {
        self.phase = Phase::Off;
        Action::Summary
    }
    fn expired(&self) -> bool {
        match self.length {
            Length::Attempts(n) => self.summary.attempts >= u32::from(n),
            Length::Frames(f) => self.elapsed >= f,
            Length::Free => false,
        }
    }
    pub fn contact(&mut self, d: Decision) {
        if self.phase != Phase::Flight {
            return;
        }
        self.result = grade(d);
        self.pending = self.result.is_none();
        if let Some(g) = self.result {
            self.summary.record(g);
        }
        self.phase = Phase::Hold(0);
    }
    /// No contact, KO or tumble escape repeats the same sampled setup immediately.
    pub fn abandon(&mut self) -> Action {
        if !matches!(self.phase, Phase::Flight | Phase::Delay(_)) {
            return Action::None;
        }
        self.result = Some(Grade::NoContact);
        if self.expired() {
            self.phase = Phase::Done;
            Action::Summary
        } else {
            self.phase = Phase::Place;
            Action::Place { reroll: false }
        }
    }
    /// Exactly one tick per P1 game frame. Menus pass paused=true.
    /// `new_guard_press` requires a separately proven native press event.
    /// A raw shoulder sample or counter decrease is not sufficient proof.
    pub fn tick(&mut self, paused: bool, new_guard_press: bool) -> Action {
        if paused || matches!(self.phase, Phase::Off | Phase::Done) {
            return Action::None;
        }
        self.elapsed = self.elapsed.saturating_add(1);
        match self.phase {
            Phase::Place => {
                if self.expired() {
                    self.phase = Phase::Done;
                    return Action::Summary;
                }
            }
            Phase::Delay(n) => {
                if self.expired() {
                    self.phase = Phase::Done;
                    return Action::Summary;
                }
                if n <= 1 {
                    self.phase = Phase::Flight;
                    return Action::Launch;
                }
                self.phase = Phase::Delay(n - 1);
            }
            Phase::Hold(n) => {
                let age = n + 1;
                if self.pending && (new_guard_press && age <= LATE || age == LATE) {
                    let g = if new_guard_press {
                        Grade::Late(age)
                    } else {
                        Grade::Missed
                    };
                    self.result = Some(g);
                    self.summary.record(g);
                    self.pending = false;
                }
                if age == HOLD {
                    if self.expired() {
                        self.phase = Phase::Done;
                        return Action::Summary;
                    }
                    self.phase = Phase::Place;
                    return Action::Place {
                        reroll: self.result.is_some_and(Grade::counted),
                    };
                }
                self.phase = Phase::Hold(age);
            }
            _ => {}
        }
        Action::None
    }
}

impl Direction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::InPlace => "In place",
            Self::RollLeft => "Roll left",
            Self::RollRight => "Roll right",
            Self::Wall => "Wall tech",
            Self::WallJump => "Wall jump",
            Self::Ceiling => "Ceiling tech",
        }
    }
}
impl Session {
    pub fn write_progress(&self, out: &mut impl Write) -> fmt::Result {
        match self.length {
            Length::Attempts(n) => write!(out, "{}/{}", self.summary.attempts, n),
            Length::Frames(total) => {
                let seconds = total.saturating_sub(self.elapsed).div_ceil(60);
                write!(out, "{}:{:02} left", seconds / 60, seconds % 60)
            }
            Length::Free => write!(out, "#{}", self.summary.attempts),
        }?;
        write!(out, " · streak {}", self.summary.streak)
    }
}
/// Best streaks persist for this process/session family, independently of the
/// current run's summary. Fixed storage follows the existing practice-game model.
pub struct Bests {
    entries: [u32; 128],
}
impl Bests {
    pub const EMPTY: Self = Self { entries: [0; 128] };
    pub fn record(&mut self, fighter: i32, streak: u32) -> u32 {
        let Some(best) = usize::try_from(fighter)
            .ok()
            .and_then(|kind| self.entries.get_mut(kind))
        else {
            return streak;
        };
        *best = (*best).max(streak);
        *best
    }
}
