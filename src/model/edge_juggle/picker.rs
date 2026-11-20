//! Fighter, behavior and settings shared by the two practice games.
use core::fmt::{self, Write};

use crate::model::{fighter_choices::CHOICES, settings::Snapshot, tech_settings::selected};

use super::Drill;

pub const HERO: u32 = 0x53;
pub const FIGHTER_CHOICES: usize = CHOICES.len() + 1;
pub const MODE_CHOICES: usize = 3;
pub const DEFAULT_WINDOW: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillId {
    Zoom,
}

pub fn drills(fighter: Option<u32>) -> &'static [DrillId] {
    match fighter {
        Some(HERO) => &[DrillId::Zoom],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Cpu(u8),
    Perfect,
    Drill(DrillId),
}

impl Mode {
    pub fn write(self, out: &mut impl Write) -> fmt::Result {
        match self {
            Self::Cpu(level) => write!(out, "CPU {level}"),
            Self::Perfect => out.write_str("Perfect"),
            Self::Drill(DrillId::Zoom) => out.write_str("Zoom"),
        }
    }

    pub const fn stock_level(self) -> u8 {
        match self {
            Self::Cpu(level) => level,
            Self::Perfect | Self::Drill(_) => 9,
        }
    }
}

pub struct Rows {
    pub fighter: &'static str,
    pub level: &'static str,
    pub window: &'static str,
}

pub const fn rows(game: Drill) -> &'static Rows {
    match game {
        Drill::GuardEdge => &Rows {
            fighter: "edge_cpu_fighter",
            level: "edge_cpu_level",
            window: "edge_zoom_window",
        },
        Drill::Juggler => &Rows {
            fighter: "juggle_cpu_fighter",
            level: "juggle_cpu_level",
            window: "juggle_zoom_window",
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picker {
    pub fighter: Option<u32>,
    pub level: u8,
    pub choice: usize,
}

impl Picker {
    pub const DEFAULT: Self = Self { fighter: None, level: 9, choice: 0 };

    pub fn read(snapshot: &Snapshot, game: Drill) -> Self {
        let rows = rows(game);
        let mut pick = Self {
            fighter: selected(snapshot, rows.fighter)
                .checked_sub(1)
                .and_then(|index| CHOICES.get(index))
                .map(|&(kind, _)| kind),
            level: (selected(snapshot, rows.level).min(8) + 1) as u8,
            choice: selected(snapshot, game.difficulty_rows().difficulty),
        };
        pick.select_fighter(pick.fighter);
        pick
    }

    pub fn select_fighter(&mut self, fighter: Option<u32>) {
        self.fighter = fighter;
        if self.choice >= self.count() {
            self.choice = 0;
        }
    }

    pub fn fighter_index(self) -> usize {
        CHOICES.iter().position(|&(kind, _)| Some(kind) == self.fighter).map_or(0, |i| i + 1)
    }

    pub fn count(self) -> usize {
        2 + drills(self.fighter).len()
    }

    pub fn mode_at(self, choice: usize) -> Mode {
        match choice {
            1 => Mode::Perfect,
            _ => choice.checked_sub(2)
                .and_then(|i| drills(self.fighter).get(i))
                .map_or(Mode::Cpu(self.level), |&drill| Mode::Drill(drill)),
        }
    }

    pub fn mode(self) -> Mode {
        self.mode_at(self.choice)
    }

    pub fn cycle_mode(&mut self, forward: bool) {
        let count = self.count();
        self.choice = (self.choice + if forward { 1 } else { count - 1 }) % count;
    }

    pub fn cycle_level(&mut self) {
        if matches!(self.mode(), Mode::Cpu(_)) {
            self.level = self.level % 9 + 1;
        }
    }
}
