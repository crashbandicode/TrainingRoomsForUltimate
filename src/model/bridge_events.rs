use core::fmt::Write;
use std::collections::VecDeque;

pub use crate::model::adapter::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Hold { dir: Direction, polls: u32, fires: u32, jump: bool },
    RoomChange { row: i32, index: u32 },
    RoomApplied { partners: u32, behavior: u32 },
}

impl Event {
    pub fn text(self) -> String {
        match self {
            Self::Hold { dir, polls, fires, jump } => {
                let dir = match dir {
                    Direction::Up => "up",
                    Direction::Right => "right",
                    Direction::Down => "down",
                    Direction::Left => "left",
                };
                format!("hold dir={dir} polls={polls} fires={fires} jump={}", u8::from(jump))
            }
            Self::RoomChange { row, index } => format!("room.panel row={row} index={index}"),
            Self::RoomApplied { partners, behavior } => format!("room.panel applied partners={partners} behavior={behavior}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub ctx: &'static str,
    pub total: u32,
    pub start: u32,
    pub end: u32,
    pub cursor: u32,
    pub playing: bool,
    pub handle: &'static str,
    pub running: bool,
}

impl Clip {
    pub fn text(self) -> String {
        format!("clip ctx={} total={} in={} out={} cursor={} playing={} handle={} running={}",
            self.ctx, self.total, self.start, self.end, self.cursor, u8::from(self.playing), self.handle, u8::from(self.running))
    }
}

#[derive(Default)]
pub struct ClipSeen {
    seen: Option<Clip>,
    unsent: bool,
    skipped: u32,
}

impl ClipSeen {
    pub fn send(&mut self, current: Option<Clip>, mut send: impl FnMut(&str) -> i32) {
        if current != self.seen {
            if self.unsent { self.skipped = self.skipped.saturating_add(1); }
            self.seen = current;
            self.unsent = true;
        }
        if !self.unsent { return; }
        if self.skipped != 0 {
            if send(&format!("clip lost={}", self.skipped)) != 0 { return; }
            self.skipped = 0;
        }
        let text = self.seen.map_or_else(|| "clip closed".to_string(), Clip::text);
        if send(&text) == 0 { self.unsent = false; }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fed {
    pub p1: u32,
    pub cpu: u32,
    pub preview: bool,
    pub hides: bool,
}

pub struct FedText {
    bytes: [u8; 64],
    len: usize,
}

impl FedText {
    pub const fn new() -> Self { Self { bytes: [0; 64], len: 0 } }
    pub fn as_str(&self) -> &str { core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("") }
}

impl Default for FedText {
    fn default() -> Self { Self::new() }
}

impl core::fmt::Write for FedText {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = s.len().min(self.bytes.len() - self.len);
        self.bytes[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FedSeen {
    seen: Fed,
    unsent: bool,
    late: u32,
}

impl FedSeen {
    pub fn sample(&mut self, now: Fed, mut send: impl FnMut(&str) -> i32) {
        if now != self.seen {
            self.seen = now;
            self.unsent = true;
        }
        if !self.unsent { return; }
        if self.late != 0 {
            let mut text = FedText::new();
            let _ = write!(text, "fed lost={}", self.late);
            if send(text.as_str()) != 0 {
                self.late = self.late.saturating_add(1);
                return;
            }
            self.late = 0;
        }
        let mut text = FedText::new();
        let _ = write!(text, "fed p1={} cpu={} preview={} hides={}", self.seen.p1, self.seen.cpu, u8::from(self.seen.preview), u8::from(self.seen.hides));
        if send(text.as_str()) == 0 { self.unsent = false; } else { self.late = 1; }
    }

    pub fn words(self) -> [u64; 2] {
        [u64::from(self.seen.p1) << 32 | u64::from(self.seen.cpu),
            u64::from(self.late) << 32 | u64::from(self.seen.preview) | u64::from(self.seen.hides) << 1 | u64::from(self.unsent) << 2]
    }

    pub fn from_words(words: [u64; 2]) -> Self {
        Self {
            seen: Fed { p1: (words[0] >> 32) as u32, cpu: words[0] as u32, preview: words[1] & 1 != 0, hides: words[1] & 2 != 0 },
            unsent: words[1] & 4 != 0,
            late: (words[1] >> 32) as u32,
        }
    }
}

#[derive(Default)]
pub struct Outbox {
    entries: VecDeque<(Event, u64)>,
    lost: u64,
}

impl Outbox {
    pub fn accept(&mut self, event: Event, now_ms: u64) {
        if self.entries.len() == 16 {
            self.entries.pop_front();
            self.lost(1);
        }
        self.entries.push_back((event, now_ms));
    }

    pub fn lost(&mut self, count: u32) {
        self.lost = self.lost.saturating_add(u64::from(count));
    }

    pub fn flush(&mut self, now_ms: u64, mut send: impl FnMut(&str) -> i32) {
        loop {
            while self.entries.front().is_some_and(|(_, at)| now_ms.saturating_sub(*at) > 2000) {
                self.entries.pop_front();
                self.lost(1);
            }
            if self.lost != 0 {
                if send(&format!("events lost={}", self.lost)) != 0 { return; }
                self.lost = 0;
            }
            let Some((event, _)) = self.entries.front().copied() else { return };
            if send(&event.text()) != 0 { return; }
            self.entries.pop_front();
        }
    }
}

#[cfg(test)]
#[path = "bridge_events_tests.rs"]
mod tests;
