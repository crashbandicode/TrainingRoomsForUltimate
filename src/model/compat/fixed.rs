//! A fixed-capacity list for state kept in game callbacks (written for this repository, not carried): the carried Mii editor bridge
//! kept its trace in `Vec`s that grew inside hooks; this holds the same items in an array, never allocates and says when it is full.
//! It serialises as a plain sequence, so the files the bridge writes keep their format.

use serde::ser::{Serialize, SerializeSeq, Serializer};

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
}

impl<T: Copy, const N: usize> std::ops::Deref for FixedList<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.items[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_list_reads_as_the_slice_of_what_was_pushed() {
        let mut list = FixedList::<u32, 3>::new(0);
        assert!(list.is_empty());
        assert!(list.push(7) && list.push(8) && list.push(9) && !list.push(10));
        assert_eq!(&*list, &[7, 8, 9]);
        assert_eq!(list.iter().copied().max(), Some(9));
    }

    #[test]
    fn it_fills_to_its_capacity_and_then_refuses() {
        let mut list = FixedVec::<u8, 3>::new();
        assert!(list.is_empty() && list.last().is_none());
        assert!(list.push(1) && list.push(2) && list.push(3));
        assert!(!list.push(4));
        assert_eq!((list.len(), list.last().copied(), list.iter().copied().collect::<Vec<_>>()), (3, Some(3), vec![1, 2, 3]));
    }

    #[test]
    fn it_serialises_like_a_vec() {
        let mut list = FixedVec::<(usize, u8, u8), 4>::new();
        list.push((1, 2, 3));
        list.push((4, 5, 6));
        let vec = vec![(1usize, 2u8, 3u8), (4, 5, 6)];
        assert_eq!(serde_json::to_string(&list).unwrap(), serde_json::to_string(&vec).unwrap());
        assert_eq!(serde_json::to_string(&FixedVec::<u8, 2>::new()).unwrap(), "[]");
    }

    #[test]
    fn a_copy_is_independent() {
        let mut a = FixedVec::<u8, 2>::new();
        a.push(1);
        let mut b = a;
        b.push(2);
        assert_eq!((a.len(), b.len()), (1, 2));
    }
}
