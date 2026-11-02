//! Activation receipts belong to currently owned bodies, not retired IDs.
//! The caller supplies its complete native window plus detached-group cells.
#[derive(Clone, Copy, Debug)]
pub struct History<T: Copy + Eq, const N: usize> { active: [Option<T>; N] }
impl<T: Copy + Eq, const N: usize> History<T, N> {
    pub const fn new() -> Self { Self { active: [None; N] } }
    pub fn contains(&self, body: &T) -> bool { self.active.contains(&Some(*body)) }
    pub fn observe(&mut self, live: [T; N], activated: T) {
        self.active = live.map(|body| (body == activated || self.contains(&body)).then_some(body));
    }
}
impl<T: Copy + Eq, const N: usize> Default for History<T, N> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn warm_exchange_keeps_activation_with_the_exact_body() {
        let mut history = History::new();
        history.observe([(100, 1), (200, 2), (300, 3)], (100, 1));
        history.observe([(300, 3), (200, 2), (100, 1)], (300, 3));
        assert!(history.contains(&(100, 1)) && history.contains(&(300, 3)));
        assert!(!history.contains(&(200, 2)));
    }
    #[test]
    fn replacement_and_reused_ids_do_not_inherit_retired_activation() {
        let mut history = History::new();
        history.observe([(100, 1), (200, 2)], (100, 1));
        history.observe([(300, 1), (200, 2)], (200, 2));
        assert!(!history.contains(&(100, 1)) && !history.contains(&(300, 1)));
        assert!(history.contains(&(200, 2)));
        // The full set can be replaced indefinitely; this is not a swap cap.
        for generation in 1..1000 {
            let live = [(generation * 1000, 1), (generation * 1000 + 1, 2)];
            history.observe(live, live[0]);
            assert!(history.contains(&live[0]) && !history.contains(&live[1]));
        }
    }
}
