//! A fixed-capacity list for state kept in game callbacks (written for this repository, not carried): the carried Mii editor bridge
//! kept its trace in `Vec`s that grew inside hooks; this holds the same items in an array, never allocates and says when it is full.
//! It serialises as a plain sequence, so the files the bridge writes keep their format.

use serde::ser::{Serialize, SerializeSeq, Serializer};

/// Format a repeatable iterator as a debug list without collecting its items.
pub struct DebugIter<I>(pub I);
impl<I: Iterator + Clone> std::fmt::Debug for DebugIter<I> where I::Item: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.0.clone()).finish()
    }
}

/// Up to `N` copies of `T`.
#[derive(Clone, Copy, Debug)]
pub struct FixedVec<T: Copy, const N: usize> {
    items: [Option<T>; N],
    len: usize,
}

impl<T: Copy, const N: usize> FixedVec<T, N> {
    pub const fn new() -> Self {
        Self { items: [None; N], len: 0 }
    }

    /// Append `item`; `false` (and nothing stored) when the list is full.
    pub fn push(&mut self, item: T) -> bool {
        if self.len == N {
            return false;
        }
        self.items[self.len] = Some(item);
        self.len += 1;
        true
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn last(&self) -> Option<&T> {
        self.len.checked_sub(1).and_then(|i| self.items[i].as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items[..self.len].iter().flatten()
    }
}

impl<T: Copy, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Serialize, const N: usize> Serialize for FixedVec<T, N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.len))?;
        for item in self.iter() {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

/// Up to `N` copies of `T` held in an array (no heap), read as a slice: the roster and hat lists the preload planner builds on
/// every planning tick inside the fighter-frame hook (R5, fix ledger F10). `fill` is the value of the unused tail.
#[derive(Clone, Copy, Debug)]
pub struct FixedList<T: Copy, const N: usize> {
    items: [T; N],
    len: usize,
}

impl<T: Copy, const N: usize> FixedList<T, N> {
    pub const fn new(fill: T) -> Self {
        Self { items: [fill; N], len: 0 }
    }

    /// Append `item`; `false` (and nothing stored) when the list is full.
    pub fn push(&mut self, item: T) -> bool {
        if self.len == N {
            return false;
        }
        self.items[self.len] = item;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<T> {
        self.len = self.len.checked_sub(1)?;
        Some(self.items[self.len])
    }

    pub fn clear(&mut self) { self.len = 0; }

    /// Remove one initialized item, preserving the order of the remaining items.
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);
        let item = self.items[index];
        self.items.copy_within(index + 1..self.len, index);
        self.len -= 1;
        item
    }
}

impl<T: Copy, const N: usize> std::ops::Deref for FixedList<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.items[..self.len]
    }
}

impl<T: Copy, const N: usize> std::ops::DerefMut for FixedList<T, N> {
    fn deref_mut(&mut self) -> &mut [T] { &mut self.items[..self.len] }
}

/// Up to `N` owned values in arrival order, held in an array (no heap): what a game callback hands to a plugin thread when the
/// values are not `Copy` (hos-lock's `Mailbox` carries `Copy` messages). A full queue hands the value back and counts it.
pub struct FixedQueue<T, const N: usize> {
    items: [Option<T>; N],
    head: usize,
    len: usize,
    refused: u32,
}

impl<T, const N: usize> FixedQueue<T, N> {
    pub const fn new() -> Self {
        Self { items: [const { None }; N], head: 0, len: 0, refused: 0 }
    }

    /// Append `item` after the others; a full queue hands it back.
    pub fn push(&mut self, item: T) -> Result<(), T> {
        if self.len == N {
            self.refused = self.refused.saturating_add(1);
            return Err(item);
        }
        self.items[(self.head + self.len) % N] = Some(item);
        self.len += 1;
        Ok(())
    }

    /// The oldest value.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let item = self.items[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Values handed back since the last call.
    pub fn take_refused(&mut self) -> u32 {
        std::mem::take(&mut self.refused)
    }
}

impl<T, const N: usize> Default for FixedQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn a_queue_gives_its_values_back_in_order_and_refuses_past_its_capacity() {
        let mut queue = FixedQueue::<Vec<u8>, 3>::new();
        assert!(queue.is_empty() && queue.pop().is_none());
        for round in 0..4u8 {
            assert!(queue.push(vec![round]).is_ok() && queue.push(vec![round, 1]).is_ok() && queue.push(vec![round, 2]).is_ok());
            assert_eq!(queue.push(vec![9]), Err(vec![9]), "the fourth value is handed back");
            assert_eq!((queue.len(), queue.take_refused(), queue.take_refused()), (3, 1, 0));
            assert_eq!(queue.pop(), Some(vec![round]));
            assert!(queue.push(vec![round, 3]).is_ok(), "a freed slot takes the next value");
            let rest: Vec<_> = std::iter::from_fn(|| queue.pop()).collect();
            assert_eq!(rest, [vec![round, 1], vec![round, 2], vec![round, 3]]);
        }
    }

    fn a_fixed_list_reads_as_the_slice_of_what_was_pushed() {
        let mut list = FixedList::<u32, 3>::new(0);
        assert!(list.is_empty());
        assert!(list.push(7) && list.push(8) && list.push(9) && !list.push(10));
        assert_eq!(&*list, &[7, 8, 9]);
        assert_eq!(list.iter().copied().max(), Some(9));
    }

    fn it_fills_to_its_capacity_and_then_refuses() {
        let mut list = FixedVec::<u8, 3>::new();
        assert!(list.is_empty() && list.last().is_none());
        assert!(list.push(1) && list.push(2) && list.push(3));
        assert!(!list.push(4));
        assert_eq!((list.len(), list.last().copied(), list.iter().copied().collect::<Vec<_>>()), (3, Some(3), vec![1, 2, 3]));
    }

    fn it_serialises_like_a_vec() {
        let mut list = FixedVec::<(usize, u8, u8), 4>::new();
        list.push((1, 2, 3));
        list.push((4, 5, 6));
        let vec = vec![(1usize, 2u8, 3u8), (4, 5, 6)];
        assert_eq!(serde_json::to_string(&list).unwrap(), serde_json::to_string(&vec).unwrap());
        assert_eq!(serde_json::to_string(&FixedVec::<u8, 2>::new()).unwrap(), "[]");
    }

    fn a_copy_is_independent() {
        let mut a = FixedVec::<u8, 2>::new();
        a.push(1);
        let mut b = a;
        b.push(2);
        assert_eq!((a.len(), b.len()), (1, 2));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_queue_gives_its_values_back_in_order_and_refuses_past_its_capacity", a_queue_gives_its_values_back_in_order_and_refuses_past_its_capacity as fn()),
            ("a_fixed_list_reads_as_the_slice_of_what_was_pushed", a_fixed_list_reads_as_the_slice_of_what_was_pushed as fn()),
            ("it_fills_to_its_capacity_and_then_refuses", it_fills_to_its_capacity_and_then_refuses as fn()),
            ("it_serialises_like_a_vec", it_serialises_like_a_vec as fn()),
            ("a_copy_is_independent", a_copy_is_independent as fn()),
        ]);
    }
    }
}
