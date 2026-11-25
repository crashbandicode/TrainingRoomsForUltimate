//! Merge 7 frame-strip scrollback history, written on the game thread's frame step and read by the HUD while scrolled.
//! Pure, with fixed storage.

use super::{Aim, Cell, Column, Link, Mark, Press, Tag, Tracker, HISTORY, WINDOW};

/// Columns kept in history.
pub const COLUMNS: usize = 600;
/// Presses kept in history.
pub const PRESSES: usize = 128;
/// Direction marks, spot marks and grade tags kept in each history ring.
pub const NOTES: usize = 64;
/// Links kept in history.
pub const LINKS: usize = 16;

struct Ring<T: Copy + PartialEq, const N: usize> {
    items: [Option<T>; N],
    head: usize,
}

impl<T: Copy + PartialEq, const N: usize> Ring<T, N> {
    const fn new() -> Self { Self { items: [None; N], head: 0 } }

    fn clear(&mut self) {
        self.items.fill(None);
        self.head = 0;
    }

    fn push(&mut self, item: T) {
        self.items[self.head] = Some(item);
        self.head = (self.head + 1) % N;
    }

    fn newest_first(&self) -> impl Iterator<Item = T> + '_ {
        (0..N).filter_map(|i| self.items[(self.head + N - 1 - i) % N])
    }

    fn contains_recent(&self, item: &T, n: usize) -> bool {
        self.newest_first().take(n).any(|seen| &seen == item)
    }
}

fn record_recent<T: Copy + PartialEq, const N: usize, const K: usize>(
    ring: &mut Ring<T, N>, items: impl Iterator<Item = T>,
) {
    let mut pending = [None; K];
    for (slot, item) in pending.iter_mut().zip(items) {
        if ring.contains_recent(&item, K) { break; }
        *slot = Some(item);
    }
    for item in pending.into_iter().rev().flatten() {
        ring.push(item);
    }
}

fn restore_recent<T: Copy + PartialEq, const N: usize, const K: usize>(
    ring: &Ring<T, N>, target: &mut [Option<T>; K], accept: impl Fn(&T) -> bool,
) -> usize {
    target.fill(None);
    let mut pending = [None; K];
    let mut len = 0;
    for (index, (slot, item)) in pending.iter_mut().zip(ring.newest_first().filter(accept)).enumerate() {
        *slot = Some(item);
        len = index + 1;
    }
    for (slot, item) in target.iter_mut().zip(pending[..len].iter().rev()) {
        *slot = *item;
    }
    len % K
}

/// What the strip showed, kept past its 60 columns.
pub struct Back {
    columns: [Column; COLUMNS],
    head: usize,
    kept: usize,
    count: u32,
    frame: u32,
    presses: Ring<Press, PRESSES>,
    aims: Ring<Aim, NOTES>,
    marks: Ring<Mark, NOTES>,
    tags: Ring<Tag, NOTES>,
    links: Ring<Link, LINKS>,
}

impl Back {
    pub const fn new() -> Self {
        Self {
            columns: [Column { frame: 0, p1: Cell::Actionable, cpu: Cell::Actionable, stick: None }; COLUMNS],
            head: 0, kept: 0, count: 0, frame: 0,
            presses: Ring::new(), aims: Ring::new(), marks: Ring::new(), tags: Ring::new(), links: Ring::new(),
        }
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.kept = 0;
        self.count = 0;
        self.frame = 0;
        self.presses.clear();
        self.aims.clear();
        self.marks.clear();
        self.tags.clear();
        self.links.clear();
    }

    /// Columns recorded since the last clear, and how many are kept.
    pub fn span(&self) -> (u32, u32) { (self.count, self.kept as u32) }

    /// Append what the tracker gained on the frame just stepped.
    pub fn record(&mut self, tracker: &Tracker) {
        if tracker.frame() <= self.frame && (self.frame != 0 || self.count != 0) {
            self.clear();
        }
        let last = (self.kept != 0).then(|| self.columns[(self.head + COLUMNS - 1) % COLUMNS].frame);
        for column in tracker.columns().filter(|column| last.is_none_or(|frame| column.frame > frame)) {
            self.columns[self.head] = column;
            self.head = (self.head + 1) % COLUMNS;
            self.kept = (self.kept + 1).min(COLUMNS);
            self.count = self.count.saturating_add(1);
        }
        record_recent::<Press, PRESSES, { super::PRESSES }>(&mut self.presses, tracker.presses());
        record_recent::<Aim, NOTES, { super::AIMS }>(&mut self.aims, tracker.aims());
        record_recent::<Mark, NOTES, { super::MARKS }>(&mut self.marks, tracker.marks());
        record_recent::<Tag, NOTES, { super::TAGS }>(&mut self.tags, tracker.tags().copied());
        let mut pending = [None; HISTORY];
        let mut len = 0;
        for link in tracker.history() {
            if let Some(old) = self.links.items.iter_mut().flatten().find(|old| old.hit == link.hit) {
                *old = link;
            } else if let Some(slot) = pending.get_mut(len) {
                *slot = Some(link);
                len += 1;
            }
        }
        for link in pending.into_iter().rev().flatten() {
            self.links.push(link);
        }
        self.frame = tracker.frame();
    }

    /// Turn a tracker copy into the strip at `anchor`, clamped to the oldest full window. Returns frames behind live,
    /// or leaves the tracker untouched when no view is available.
    pub fn view(&self, tracker: &mut Tracker, anchor: u32) -> Option<u32> {
        if self.count == 0 || anchor == 0 || anchor > self.count || tracker.frame() < self.frame { return None; }
        let live_newest = tracker.newest()?;
        let first_kept = self.count.saturating_sub(self.kept as u32) + 1;
        let oldest_view = first_kept.saturating_add((WINDOW - 1) as u32).min(self.count);
        let anchor = anchor.max(oldest_view);
        let first = anchor.saturating_sub((WINDOW - 1) as u32).max(first_kept);
        let len = (anchor - first + 1) as usize;
        let start = (self.head + COLUMNS - self.kept + (first - first_kept) as usize) % COLUMNS;
        for (offset, slot) in tracker.columns[..len].iter_mut().enumerate() {
            *slot = self.columns[(start + offset) % COLUMNS];
        }
        tracker.len = len;
        tracker.head = len % WINDOW;
        let oldest = tracker.columns[0].frame;
        let newest = tracker.columns[len - 1].frame;
        tracker.press_head = restore_recent(&self.presses, &mut tracker.presses, |press| oldest <= press.frame && press.frame <= newest);
        tracker.aim_head = restore_recent(&self.aims, &mut tracker.aims, |aim| aim.frame <= newest);
        tracker.mark_head = restore_recent(&self.marks, &mut tracker.marks, |mark| mark.frame <= newest);
        tracker.tag_head = restore_recent(&self.tags, &mut tracker.tags, |tag| tag.frame <= newest);
        tracker.history.fill(None);
        for (slot, link) in tracker.history.iter_mut().zip(self.links.newest_first().filter(|link| link.hit <= newest)) {
            *slot = Some(link);
        }
        tracker.advantage = tracker.advantage.filter(|advantage| {
            (oldest..=newest).contains(&advantage.p1) && (oldest..=newest).contains(&advantage.cpu)
        });
        Some(live_newest.saturating_sub(newest))
    }
}

impl Default for Back {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
#[path = "strip_back_tests.rs"]
mod tests;
