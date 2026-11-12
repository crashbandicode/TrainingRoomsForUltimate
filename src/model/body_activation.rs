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
    lib_unit_tests! {
    use super::*;
    fn warm_exchange_keeps_activation_with_the_exact_body() {
        let mut history = History::new();
        history.observe([(100, 1), (200, 2), (300, 3)], (100, 1));
        history.observe([(300, 3), (200, 2), (100, 1)], (300, 3));
        assert!(history.contains(&(100, 1)) && history.contains(&(300, 3)));
        assert!(!history.contains(&(200, 2)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("warm_exchange_keeps_activation_with_the_exact_body", warm_exchange_keeps_activation_with_the_exact_body as fn()),
        ]);
    }
    }
}
