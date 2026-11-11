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

