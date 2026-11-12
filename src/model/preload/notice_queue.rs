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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    type Queue = NoticeQueue<u8>;

    fn q() -> Queue {
        Queue::new()
    }
    fn plain(q: &mut Queue, header: &str, length: u32) {
        q.push(header.to_string(), format!("{} text", header), length, 7);
    }
    fn first(q: &Queue) -> Option<&str> {
        q.items.first().map(|n| n.header.as_str())
    }
    fn headers(q: &Queue) -> Vec<&str> {
        q.items.iter().map(|n| n.header.as_str()).collect()
    }
    fn run(q: &mut Queue, frames: u32, hidden: bool) {
        for _ in 0..frames {
            q.step(hidden);
        }
    }

    fn plain_notices_behave_as_before_the_queue_was_extracted() {
        // Old `draw`: tick, then complete when the length is 1 or less. A length of 3 is drawn twice.
        let mut q = q();
        plain(&mut q, "a", 3);
        plain(&mut q, "b", 2);
        let f = q.step(false);
        assert!(f.visible);
        assert_eq!(f.draw, Some(("a".to_string(), "a text".to_string(), 7)));
        assert_eq!(first(&q), Some("a"));
        assert_eq!(q.step(false).draw, None);
        assert_eq!(first(&q), Some("b"));
        let f = q.step(false);
        assert_eq!(f.draw.map(|d| d.0), Some("b".to_string()));
        assert!(q.is_empty());
        assert_eq!(q.step(false), Frame { visible: false, draw: None });
    }

    fn a_plain_notice_still_counts_down_while_the_quick_menu_hides_the_pane() {
        let mut q = q();
        plain(&mut q, "a", 5);
        let f = q.step(true);
        assert!(!f.visible);
        assert!(f.draw.is_some(), "text is written on the first frame even while hidden, as before");
        run(&mut q, 3, true);
        assert!(q.is_empty());
        assert!(!q.take_delivered("a"), "a plain notice is never recorded as delivered");
    }

    fn a_standing_notice_waits_behind_plain_notices_and_new_plain_ones_stay_ahead_of_it() {
        let mut q = q();
        plain(&mut q, "welcome", 3);
        q.upsert("Fighters", "Swap once to load".to_string(), 10, 1);
        assert_eq!(headers(&q), ["welcome", "Fighters"]);
        plain(&mut q, "swap", 3);
        assert_eq!(headers(&q), ["welcome", "swap", "Fighters"]);
        // Once the standing notice is on screen a later plain notice waits for it instead.
        run(&mut q, 4, false);
        assert_eq!(first(&q), Some("Fighters"));
        q.step(false);
        plain(&mut q, "late", 3);
        assert_eq!(headers(&q), ["Fighters", "late"]);
    }

    fn upsert_updates_in_place_and_restarts_the_full_time() {
        let mut q = q();
        q.upsert("Fighters", "Loading 1/5".to_string(), 10, 1);
        plain(&mut q, "swap", 4);
        q.upsert("Fighters", "Loading 2/5".to_string(), 10, 2);
        assert_eq!(headers(&q), ["swap", "Fighters"], "no second entry, no reordering");
        assert_eq!(q.items[1].message, "Loading 2/5");
        // Once the plain notice is gone the standing one draws; an update then rewrites the text.
        run(&mut q, 3, false);
        let f = q.step(false);
        assert_eq!(f.draw, Some(("Fighters".to_string(), "Loading 2/5".to_string(), 2)));
        run(&mut q, 4, false);
        q.upsert("Fighters", "All loaded".to_string(), 10, 3);
        assert_eq!(q.step(false).draw, Some(("Fighters".to_string(), "All loaded".to_string(), 3)));
        run(&mut q, 7, false);
        assert!(q.is_queued("Fighters"), "9 of 10 frames ran: not yet delivered");
        assert!(!q.take_delivered("Fighters"));
    }

    fn delivery_needs_the_full_visible_time_and_the_quick_menu_does_not_consume_it() {
        let mut q = q();
        q.upsert("Fighters", "All loaded".to_string(), 10, 1);
        // 9 visible frames leave it queued (completes when the length reaches 1).
        run(&mut q, 4, false);
        // The quick menu hides the pane for a long time: no countdown, nothing delivered, not lost.
        let f = q.step(true);
        assert!(!f.visible);
        run(&mut q, 500, true);
        assert!(q.is_queued("Fighters"));
        assert!(!q.take_delivered("Fighters"));
        run(&mut q, 4, false);
        assert!(q.is_queued("Fighters"));
        q.step(false);
        assert!(!q.is_queued("Fighters"));
        assert!(q.take_delivered("Fighters"));
        assert!(!q.take_delivered("Fighters"), "a delivery is read once");
    }

    fn a_standing_notice_of_length_one_is_not_delivered_unseen() {
        let mut q = q();
        q.upsert("Fighters", "x".to_string(), 1, 1);
        run(&mut q, 20, true);
        assert!(q.is_queued("Fighters"));
        assert!(!q.take_delivered("Fighters"));
        q.step(false);
        assert!(q.take_delivered("Fighters"));
    }

    fn a_wipe_by_another_writer_is_not_a_delivery() {
        let mut q = q();
        q.upsert("Fighters", "All loaded".to_string(), 10, 1);
        run(&mut q, 3, false);
        q.clear_all();
        assert!(!q.is_queued("Fighters"));
        assert!(!q.take_delivered("Fighters"));
        q.upsert("Fighters", "All loaded".to_string(), 10, 1);
        run(&mut q, 3, false);
        q.clear("Fighters");
        assert!(!q.is_queued("Fighters"));
        assert!(!q.take_delivered("Fighters"));
    }

    fn a_new_content_drops_the_earlier_delivery_record() {
        let mut q = q();
        q.upsert("Fighters", "a".to_string(), 2, 1);
        run(&mut q, 2, false);
        q.upsert("Fighters", "b".to_string(), 2, 1);
        assert!(!q.take_delivered("Fighters"), "the record belonged to the old content");
        run(&mut q, 2, false);
        assert!(q.take_delivered("Fighters"));
    }

    fn a_delivery_survives_a_wipe_that_follows_it() {
        let mut q = q();
        q.upsert("Fighters", "a".to_string(), 2, 1);
        run(&mut q, 2, false);
        q.clear_all();
        assert!(q.take_delivered("Fighters"), "it was seen before the wipe");
    }

    fn clearing_one_header_leaves_the_others() {
        let mut q = q();
        plain(&mut q, "a", 9);
        q.upsert("Fighters", "x".to_string(), 9, 1);
        plain(&mut q, "b", 9);
        q.clear("a");
        assert_eq!(headers(&q), ["b", "Fighters"]);
        assert!(q.is_queued("b") && !q.is_queued("a"));
    }

    fn holds_covers_a_queued_entry_and_an_unread_delivery() {
        let mut q = q();
        assert!(!q.holds("Fighters"));
        q.upsert("Fighters", "x".to_string(), 2, 1);
        assert!(q.holds("Fighters"));
        run(&mut q, 1, false);
        assert!(!q.is_queued("Fighters"));
        assert!(q.holds("Fighters"), "the delivery record has not been read");
        q.clear("Fighters");
        assert!(!q.holds("Fighters"));
        assert!(!q.take_delivered("Fighters"), "clearing drops the record");
    }

    fn a_swap_notice_burst_never_loses_the_standing_notice() {
        // Ten swap notices of 3 s each arrive while the standing notice is queued; it is still there,
        // carries the latest content and is shown once they have all run.
        let mut q = q();
        q.upsert("Fighters", "Loading 1/5".to_string(), 180, 1);
        for _ in 0..10 {
            plain(&mut q, "swap", 180);
        }
        q.upsert("Fighters", "All loaded (5/5, 2 instant)".to_string(), 180, 2);
        run(&mut q, 10 * 179, false);
        assert_eq!(headers(&q), ["Fighters"]);
        let f = q.step(false);
        assert_eq!(f.draw.map(|d| d.1), Some("All loaded (5/5, 2 instant)".to_string()));
        run(&mut q, 179, false);
        assert!(q.take_delivered("Fighters"));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("plain_notices_behave_as_before_the_queue_was_extracted", plain_notices_behave_as_before_the_queue_was_extracted as fn()),
            ("a_plain_notice_still_counts_down_while_the_quick_menu_hides_the_pane", a_plain_notice_still_counts_down_while_the_quick_menu_hides_the_pane as fn()),
            ("a_standing_notice_waits_behind_plain_notices_and_new_plain_ones_stay_ahead_of_it", a_standing_notice_waits_behind_plain_notices_and_new_plain_ones_stay_ahead_of_it as fn()),
            ("upsert_updates_in_place_and_restarts_the_full_time", upsert_updates_in_place_and_restarts_the_full_time as fn()),
            ("delivery_needs_the_full_visible_time_and_the_quick_menu_does_not_consume_it", delivery_needs_the_full_visible_time_and_the_quick_menu_does_not_consume_it as fn()),
            ("a_standing_notice_of_length_one_is_not_delivered_unseen", a_standing_notice_of_length_one_is_not_delivered_unseen as fn()),
            ("a_wipe_by_another_writer_is_not_a_delivery", a_wipe_by_another_writer_is_not_a_delivery as fn()),
            ("a_new_content_drops_the_earlier_delivery_record", a_new_content_drops_the_earlier_delivery_record as fn()),
            ("a_delivery_survives_a_wipe_that_follows_it", a_delivery_survives_a_wipe_that_follows_it as fn()),
            ("clearing_one_header_leaves_the_others", clearing_one_header_leaves_the_others as fn()),
            ("holds_covers_a_queued_entry_and_an_unread_delivery", holds_covers_a_queued_entry_and_an_unread_delivery as fn()),
            ("a_swap_notice_burst_never_loses_the_standing_notice", a_swap_notice_burst_never_loses_the_standing_notice as fn()),
        ]);
    }
    }
}
