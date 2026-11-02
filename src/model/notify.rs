//! HUD notifications (SPEC-input-05 §2.2, §5.1): a fixed-capacity queue of short messages shown one at
//! a time in the plugin's HUD slot `TrModDisp0`. Pure: the HUD tick (package 5's anchor) calls `tick`
//! with the facts and writes the panes the answer names.
//!
//! Decisions applied: B8 question 3 a notice only counts down while it is visible, so one posted
//! behind an open menu waits for the menu to close; length 0 is treated as 1 (question 1); headers are
//! static strings and bodies are static or a small fixed buffer, so posting allocates nothing
//! (question 2); the recorder posts once per phase change (§6.2).

/// Entries the queue holds.
pub const CAPACITY: usize = 8;
/// Longest dynamic body in bytes.
pub const BODY_MAX: usize = 48;
/// Text colour used when a post gives none (black).
pub const DEFAULT_COLOR: [u8; 4] = [0, 0, 0, 255];

/// A body: a static string or bytes copied into the entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Static(&'static str),
    Copied { bytes: [u8; BODY_MAX], len: u8 },
}

impl Body {
    pub fn copied(text: &str) -> Body {
        let mut end = text.len().min(BODY_MAX);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut bytes = [0u8; BODY_MAX];
        bytes[..end].copy_from_slice(&text.as_bytes()[..end]);
        Body::Copied { bytes, len: end as u8 }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Body::Static(text) => text,
            Body::Copied { bytes, len } => std::str::from_utf8(&bytes[..usize::from(*len)]).unwrap_or(""),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    header: &'static str,
    body: Body,
    /// UI frames left.
    length: u16,
    color: [u8; 4],
    drawn: bool,
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
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

impl Queue {
    pub const fn new() -> Self {
        Self { entries: [None; CAPACITY], len: 0 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The header of the notice on screen, if any.
    pub fn front_header(&self) -> Option<&'static str> {
        self.entries[0].map(|e| e.header)
    }

    /// Append a notice. A full queue drops its oldest waiting notice (never the one on screen).
    pub fn post(&mut self, header: &'static str, body: Body, length: u16, color: [u8; 4]) {
        let entry = Entry { header, body, length: length.max(1), color, drawn: false };
        if self.len == CAPACITY {
            self.remove(1);
        }
        self.entries[self.len] = Some(entry);
        self.len += 1;
    }

    pub fn post_static(&mut self, header: &'static str, body: &'static str, length: u16, color: [u8; 4]) {
        self.post(header, Body::Static(body), length, color);
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

    /// Clear all (session clear; Combo Display Off).
    pub fn clear(&mut self) {
        *self = Self::new();
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
        } else {
            first.length -= 1;
            self.entries[0] = Some(first);
        }
        TickOut { visible: true, draw }
    }
}
