//! Consecutive native stopped callbacks, not deduplicated log records.
#[derive(Clone, Copy)]
pub struct Span {
    first: Option<u32>,
    last: Option<u32>,
    count: u32,
}
impl Span {
    pub const fn new() -> Self {
        Self {
            first: None,
            last: None,
            count: 0,
        }
    }
    pub fn observe(&mut self, age: u32) -> Option<u32> {
        if self
            .last
            .is_some_and(|last| last.checked_add(1) != Some(age))
        {
            return None;
        }
        let count = self.count.checked_add(1)?;
        self.first.get_or_insert(age);
        self.last = Some(age);
        self.count = count;
        Some(count)
    }
    pub fn first(&self) -> Option<u32> {
        self.first
    }
    pub fn count(&self) -> u32 {
        self.count
    }
}
