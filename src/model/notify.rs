//! HUD notifications (SPEC-input-05 §2.2, §5.1): a fixed-capacity queue of short messages shown one at
//! a time in the plugin's HUD slot `TrModDisp0`. Pure: the HUD tick (package 5's anchor) calls `tick`
//! with the facts and writes the panes the answer names.
//!
//! Decisions applied: B8 question 3 a notice only counts down while it is visible, so one posted
//! behind an open menu waits for the menu to close; length 0 is treated as 1 (question 1); headers are
//! static strings and bodies are copied into a fixed buffer, so posting allocates nothing
//! (question 2); the recorder posts once per phase change (§6.2).
//!
//! Standing notices (package 6's Ready and hot-swap notices): one entry per `id`, updated in place by
//! `upsert` and kept behind every plain notice that has not been drawn, so a plain notice never waits for
//! it. A standing notice is delivered when its whole visible time has run; its owner takes the id with
//! `take_delivered` and so tells "seen" from "wiped" (`clear`, session clear).

/// Entries the queue holds.
pub const CAPACITY: usize = 8;
/// Delivery records the queue keeps for its owners (a full ring drops the oldest).
pub const DELIVERED_CAPACITY: usize = 4;
/// Longest notice body in bytes.
pub const BODY_MAX: usize = 320;
/// Text colour used when a post gives none. Light, so it reads on the dark notice plate.
pub const DEFAULT_COLOR: [u8; 4] = super::panes::TEXT;

/// A UTF-8 body copied into the entry, independent of the producer's lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    bytes: [u8; BODY_MAX],
    len: u16,
}

impl Body {
    pub fn copied(text: &str) -> Body {
        let mut end = text.len().min(BODY_MAX);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut bytes = [0u8; BODY_MAX];
        bytes[..end].copy_from_slice(&text.as_bytes()[..end]);
        Body { bytes, len: end as u16 }
    }

    /// `parts` one after another, copied as far as they fit (cut at a character boundary). No allocation.
    pub fn joined(parts: &[&str]) -> Body {
        let mut bytes = [0u8; BODY_MAX];
        let mut len = 0;
        for part in parts {
            let mut end = part.len().min(BODY_MAX - len);
            while !part.is_char_boundary(end) {
                end -= 1;
            }
            bytes[len..len + end].copy_from_slice(&part.as_bytes()[..end]);
            len += end;
            if end < part.len() {
                break;
            }
        }
        Body { bytes, len: len as u16 }
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }
}

/// The notice a Mii editor Save posts when it matches a saved Mii (operator decision C, 2026-11-08): the reuse rule
/// stays (docs/MII.md rows 4 and 12), and the player is told which saved Mii it is.
pub const SAME_AS_SAVED_HEADER: &str = "Mii Fighter";
/// Five seconds of HUD ticks: the editor's return to Training is under way while it waits in the queue.
pub const SAME_AS_SAVED_FRAMES: u16 = 300;

/// `Some` for a Save that reused the saved Mii named `matched` ("Same as ‹saved name›, nothing new saved"), `None`
/// for a new save. Copies into a fixed body: no allocation.
pub fn save_notice(matched: Option<&str>) -> Option<Body> {
    matched.map(|name| Body::joined(&["Same as ", name, ", nothing new saved"]))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    header: &'static str,
    body: Body,
    /// UI frames left.
    length: u16,
    color: [u8; 4],
    drawn: bool,
    /// The id of a standing notice (`upsert`), `None` for a plain one.
    standing: Option<u8>,
}

/// What the pane writer writes when a notice is drawn for the first time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draw {
    pub header: &'static str,
    pub body: Body,
    pub color: [u8; 4],
}

/// What one HUD tick asks of the panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickOut {
    /// `TrModDisp0` visibility: a notice exists and the menu is closed.
    pub visible: bool,
    /// Write header, body and colour (the notice's first draw).
    pub draw: Option<Draw>,
}

#[derive(Clone, Copy, Debug)]
pub struct Queue {
    entries: [Option<Entry>; CAPACITY],
    len: usize,
    delivered: [u8; DELIVERED_CAPACITY],
    delivered_len: usize,
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub const fn new() -> Self {
        Self { entries: [None; CAPACITY], len: 0, delivered: [0; DELIVERED_CAPACITY], delivered_len: 0 }
    }

    #[cfg(test)]
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The header of the notice on screen, if any.
    #[cfg(test)]
    pub fn front_header(&self) -> Option<&'static str> {
        self.entries[0].map(|e| e.header)
    }

    /// The queue slot of the first standing notice that has not been drawn yet (plain notices go ahead of it).
    fn plain_position(&self) -> usize {
        (0..self.len).find(|&i| self.entries[i].is_some_and(|e| e.standing.is_some() && !e.drawn)).unwrap_or(self.len)
    }

    /// Make room for one more entry: a full queue drops its oldest waiting plain notice (never the one on
    /// screen, never a standing one). False when nothing can be dropped.
    fn make_room(&mut self) -> bool {
        if self.len < CAPACITY {
            return true;
        }
        match (1..self.len).find(|&i| self.entries[i].is_some_and(|e| e.standing.is_none())) {
            Some(at) => {
                self.remove(at);
                true
            }
            None => false,
        }
    }

    fn insert(&mut self, at: usize, entry: Entry) {
        for i in (at..self.len).rev() {
            self.entries[i + 1] = self.entries[i];
        }
        self.entries[at] = Some(entry);
        self.len += 1;
    }

    /// Append a plain notice (ahead of any standing notice not drawn yet). A full queue drops its oldest
    /// waiting plain notice (never the one on screen).
    pub fn post(&mut self, header: &'static str, body: Body, length: u16, color: [u8; 4]) {
        if !self.make_room() {
            return;
        }
        let at = self.plain_position();
        self.insert(at, Entry { header, body, length: length.max(1), color, drawn: false, standing: None });
    }

    pub fn post_static(&mut self, header: &'static str, body: &'static str, length: u16, color: [u8; 4]) {
        self.post(header, Body::copied(body), length, color);
    }

    fn remove(&mut self, at: usize) {
        for i in at..self.len - 1 {
            self.entries[i] = self.entries[i + 1];
        }
        self.entries[self.len - 1] = None;
        self.len -= 1;
    }

    /// Remove every queued notice with `header` (the recorder replaces its phase message).
    pub fn clear_header(&mut self, header: &str) {
        let mut i = 0;
        while i < self.len {
            if self.entries[i].is_some_and(|e| e.header == header) {
                self.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The layout was registered again: nothing this queue wrote is on screen, so the notice showing is drawn
    /// again on the next tick.
    pub fn on_layout_registered(&mut self) {
        if let Some(first) = self.entries[0].as_mut() {
            first.drawn = false;
        }
    }

    /// Clear all (session clear; Combo Display Off). Delivery records stay: they say something already
    /// happened.
    pub fn clear(&mut self) {
        let (delivered, delivered_len) = (self.delivered, self.delivered_len);
        *self = Self::new();
        self.delivered = delivered;
        self.delivered_len = delivered_len;
    }

    /// The standing notice `id`: created behind every plain notice (and behind the standing ones before
    /// it), or updated where it is with new text, colour and a full `length`, redrawn on the next tick. New
    /// content has not been delivered, so an earlier delivery record of `id` is dropped. A full queue with
    /// no plain notice to drop ignores a new id (an existing one is always updated).
    pub fn upsert(&mut self, id: u8, header: &'static str, body: Body, length: u16, color: [u8; 4]) {
        self.forget_delivery(id);
        let entry = Entry { header, body, length: length.max(1), color, drawn: false, standing: Some(id) };
        if let Some(at) = (0..self.len).find(|&i| self.entries[i].is_some_and(|e| e.standing == Some(id))) {
            self.entries[at] = Some(entry);
        } else if self.make_room() {
            self.insert(self.len, entry);
        }
    }

    /// The standing notice `id` on screen now, ahead of everything (rc.8's speed pop-up, shown while its combo is held):
    /// an existing `id` moves to the front with the new text. The notice that was on screen waits behind it with the
    /// time it had left and is drawn again when it returns. A full queue drops a waiting plain notice for a new `id`.
    pub fn show_now(&mut self, id: u8, header: &'static str, body: Body, length: u16, color: [u8; 4]) {
        self.forget_delivery(id);
        if let Some(at) = (0..self.len).find(|&i| self.entries[i].is_some_and(|e| e.standing == Some(id))) {
            self.remove(at);
        } else if !self.make_room() {
            return;
        }
        if let Some(first) = self.entries[0].as_mut() {
            first.drawn = false;
        }
        self.insert(0, Entry { header, body, length: length.max(1), color, drawn: false, standing: Some(id) });
    }

    /// Remove the standing notice `id` (queued or on screen) and its delivery record.
    pub fn clear_id(&mut self, id: u8) {
        if let Some(at) = (0..self.len).find(|&i| self.entries[i].is_some_and(|e| e.standing == Some(id))) {
            self.remove(at);
        }
        self.forget_delivery(id);
    }

    /// Whether the standing notice `id` is queued (not yet delivered, not cleared).
    pub fn is_queued(&self, id: u8) -> bool {
        (0..self.len).any(|i| self.entries[i].is_some_and(|e| e.standing == Some(id)))
    }

    /// The oldest delivery record: the id of a standing notice whose whole visible time has run.
    pub fn take_delivered(&mut self) -> Option<u8> {
        if self.delivered_len == 0 {
            return None;
        }
        let id = self.delivered[0];
        self.delivered.copy_within(1..self.delivered_len, 0);
        self.delivered_len -= 1;
        Some(id)
    }

    fn forget_delivery(&mut self, id: u8) {
        let mut kept = 0;
        for i in 0..self.delivered_len {
            if self.delivered[i] != id {
                self.delivered[kept] = self.delivered[i];
                kept += 1;
            }
        }
        self.delivered_len = kept;
    }

    fn record_delivery(&mut self, id: u8) {
        self.forget_delivery(id);
        if self.delivered_len == DELIVERED_CAPACITY {
            self.delivered.copy_within(1.., 0);
            self.delivered_len -= 1;
        }
        self.delivered[self.delivered_len] = id;
        self.delivered_len += 1;
    }

    /// One HUD tick (§5.1). `combo_display` is the game's option (false empties the queue);
    /// `menu_open` hides the slot and, by decision B8, stops the countdown.
    pub fn tick(&mut self, combo_display: bool, menu_open: bool) -> TickOut {
        if !combo_display {
            self.clear();
        }
        let Some(mut first) = self.entries[0] else {
            return TickOut { visible: false, draw: None };
        };
        if menu_open {
            return TickOut { visible: false, draw: None };
        }
        let mut draw = None;
        if !first.drawn {
            first.drawn = true;
            draw = Some(Draw { header: first.header, body: first.body, color: first.color });
        }
        // Completion is checked before the decrement could underflow; a notice posted for n frames is
        // drawn on n ticks.
        if first.length <= 1 {
            self.remove(0);
            if let Some(id) = first.standing {
                self.record_delivery(id);
            }
        } else {
            first.length -= 1;
            self.entries[0] = Some(first);
        }
        TickOut { visible: true, draw }
    }
}
