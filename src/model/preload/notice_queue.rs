//! Value-only queue behind the Training UI's one notice pane (`TrModDisp0`). The pane shows the
//! first entry only. A plain notice counts down every frame, even while the quick menu hides the
//! pane, exactly as before this module existed.
//!
//! A *standing* notice (`upsert`) is one entry per header that is updated in place and kept behind
//! every plain notice, so a swap notice never waits for it and it never gives way and disappears.
//! It counts down only while the pane is visible, and it is recorded as delivered only when its
//! whole visible time has run, so its owner can tell "seen" from "wiped by another writer".

/// One queued notice. `C` is the draw colour, copied through untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice<C> {
    pub header: String,
    pub message: String,
    pub color: C,
    length: u32,
    drawn: bool,
    standing: bool,
}

/// What the pane does on one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame<C> {
    /// The pane is shown: there is an entry and the quick menu is closed.
    pub visible: bool,
    /// Header, message and colour to write into the pane's text boxes (first frame of an entry,
    /// or of an updated standing entry).
    pub draw: Option<(String, String, C)>,
}

#[derive(Debug)]
pub struct NoticeQueue<C> {
    items: Vec<Notice<C>>,
    delivered: Vec<String>,
}

impl<C: Copy> NoticeQueue<C> {
    pub const fn new() -> Self {
        Self { items: Vec::new(), delivered: Vec::new() }
    }

    /// A plain notice goes to the back of the plain notices, ahead of any standing notice that has
    /// not been drawn yet. Behind a standing notice that is on screen it waits for that notice.
    pub fn push(&mut self, header: String, message: String, length: u32, color: C) {
        let at = self.items.iter().position(|n| n.standing && !n.drawn).unwrap_or(self.items.len());
        self.items.insert(at, Notice { header, message, color, length, drawn: false, standing: false });
    }

    /// The one standing notice for `header`: created at the back, or updated where it is (new text,
    /// colour and a full `length` again, redrawn on the next frame). A new content has not been
    /// delivered, so an earlier delivery record of the header is dropped.
    pub fn upsert(&mut self, header: &str, message: String, length: u32, color: C) {
        self.delivered.retain(|h| h != header);
        if let Some(n) = self.items.iter_mut().find(|n| n.standing && n.header == header) {
            n.message = message;
            n.color = color;
            n.length = length;
            n.drawn = false;
            return;
        }
        self.items.push(Notice { header: header.to_string(), message, color, length, drawn: false, standing: true });
    }

    pub fn clear(&mut self, header: &str) {
        self.items.retain(|n| n.header != header);
        self.delivered.retain(|h| h != header);
    }

    /// Wipes the queue. Delivery records stay: they say something already happened.
    pub fn clear_all(&mut self) {
        self.items.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn is_queued(&self, header: &str) -> bool {
        self.items.iter().any(|n| n.header == header)
    }

    /// Something to clear for `header`: a queued entry or an unread delivery record.
    pub fn holds(&self, header: &str) -> bool {
        self.is_queued(header) || self.delivered.iter().any(|h| h == header)
    }

    /// True once, after the standing notice of `header` was visible for its whole time.
    pub fn take_delivered(&mut self, header: &str) -> bool {
        let before = self.delivered.len();
        self.delivered.retain(|h| h != header);
        self.delivered.len() != before
    }

    /// One draw frame. `hidden` = the quick menu covers the pane.
    pub fn step(&mut self, hidden: bool) -> Frame<C> {
        let Some(first) = self.items.first_mut() else {
            return Frame { visible: false, draw: None };
        };
        let counting = !(first.standing && hidden);
        if counting {
            first.length = first.length.saturating_sub(1);
        }
        let draw = if first.drawn {
            None
        } else {
            first.drawn = true;
            Some((first.header.clone(), first.message.clone(), first.color))
        };
        if counting && first.length <= 1 {
            let done = self.items.remove(0);
            if done.standing {
                self.delivered.push(done.header);
            }
        }
        Frame { visible: !hidden, draw }
    }
}

impl<C: Copy> Default for NoticeQueue<C> {
    fn default() -> Self {
        Self::new()
    }
}

