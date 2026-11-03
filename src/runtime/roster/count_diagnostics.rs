//! Value-only diagnostic limiter. Exhaustion is observable and never admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { Changed, Overflow, Quiet }

pub struct Changes<T> { last: Option<T>, remaining: u8, overflow: bool }
impl<T: Copy + Eq> Changes<T> {
    pub const fn new(remaining: u8) -> Self { Self {last: None, remaining, overflow: false} }
    pub fn observe(&mut self, value: T) -> Event {
        if self.last == Some(value) { return Event::Quiet; }
        self.last = Some(value);
        if self.remaining != 0 { self.remaining -= 1; Event::Changed }
        else if !self.overflow { self.overflow = true; Event::Overflow }
        else { Event::Quiet }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_promotion_changed_tuples_have_a_fixed_budget_and_one_explicit_overflow() {
        let mut changes = Changes::new(2);
        assert_eq!(changes.observe((1, false)), Event::Changed);
        for _ in 0..100 { assert_eq!(changes.observe((1, false)), Event::Quiet); }
        assert_eq!(changes.observe((1, true)), Event::Changed);
        assert_eq!(changes.observe((2, false)), Event::Overflow);
        assert_eq!(changes.observe((3, true)), Event::Quiet);
        assert_eq!(changes.observe((1, false)), Event::Quiet);
    }
}
