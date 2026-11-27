//! A fixed pool of nodes allocated once, with lock-free free and queued stacks: any thread pushes a `Copy` value without
//! allocating or waiting on a lock, and `drain` hands values over in push order and returns their nodes.
//! Nodes are free, owned by a producer until publication, queued, or owned by a drainer until returned. Links are atomic
//! because a suspended pop can still read a node another thread took. Only the owner writes a payload before publication;
//! only its drainer reads it after the swap. The generation never wraps: at its last value the pool is retired and every
//! later push is refused, so no head value is stored twice with a pop in between. Checkout is Acquire, return Release;
//! publication is Release and detachment Acquire.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{Acquire, AcqRel, Relaxed, Release}};

struct Node<T> {
    /// The next node of the stack this node is on: its index plus one, 0 for none.
    next: AtomicU32,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct PoolList<T: Copy> {
    nodes: Box<[Node<T>]>,
    /// The free stack's head: bits 0..16 the top node's index plus one (0: none free), bits 16..64 a generation that every pop raises.
    free: AtomicU64,
    /// The queued stack's head: the newest node's index plus one (0: nothing queued).
    queued: AtomicU32,
    last_generation: u64,
}

// SAFETY: each node's payload has one producer or drainer owner, transferred through atomic stack heads.
unsafe impl<T: Copy + Send> Sync for PoolList<T> {}
// SAFETY: each node's payload has one producer or drainer owner, transferred through atomic stack heads.
unsafe impl<T: Copy + Send> Send for PoolList<T> {}

impl<T: Copy> PoolList<T> {
    /// `None` for zero or more than `0xfffe` nodes, or a failed allocation.
    pub fn new(capacity: usize) -> Option<Self> {
        Self::make(capacity, (1 << 48) - 1)
    }

    #[cfg(test)]
    /// A test pool that retires after `last` checkouts.
    fn with_last_generation(capacity: usize, last: u64) -> Option<Self> {
        Self::make(capacity, last)
    }

    fn make(capacity: usize, last_generation: u64) -> Option<Self> {
        if capacity == 0 || capacity > 0xfffe { return None; }
        let mut nodes = Vec::new();
        nodes.try_reserve_exact(capacity).ok()?;
        for i in 0..capacity {
            nodes.push(Node {
                next: AtomicU32::new(if i + 1 < capacity { i as u32 + 2 } else { 0 }),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }
        Some(Self { nodes: nodes.into_boxed_slice(), free: AtomicU64::new(1), queued: AtomicU32::new(0), last_generation })
    }

    pub fn capacity(&self) -> usize {
        self.nodes.len()
    }

    fn free_top(&self) -> Option<(u64, u32)> {
        let head = self.free.load(Acquire);
        let index = (head & 0xffff) as u32;
        (index != 0 && head >> 16 != self.last_generation).then(|| (head, self.nodes[index as usize - 1].next.load(Relaxed)))
    }

    fn take_free(&self, seen: (u64, u32)) -> bool {
        let next = (seen.0.wrapping_add(1 << 16) & !0xffff) | u64::from(seen.1);
        self.free.compare_exchange(seen.0, next, AcqRel, Relaxed).is_ok()
    }

    fn return_free(&self, index: u32) {
        let node = &self.nodes[index as usize - 1];
        let mut head = self.free.load(Relaxed);
        loop {
            node.next.store((head & 0xffff) as u32, Relaxed);
            let next = (head & !0xffff) | u64::from(index);
            match self.free.compare_exchange_weak(head, next, Release, Relaxed) {
                Ok(_) => return,
                Err(actual) => head = actual,
            }
        }
    }

    /// False when no node is free or the pool is retired; nothing else changes.
    pub fn push(&self, value: T) -> bool {
        let index = loop {
            let Some(seen) = self.free_top() else { return false };
            if self.take_free(seen) { break (seen.0 & 0xffff) as u32; }
        };
        let node = &self.nodes[index as usize - 1];
        // SAFETY: this producer owns the node until it publishes it.
        unsafe { (*node.value.get()).write(value); }
        let mut head = self.queued.load(Relaxed);
        loop {
            node.next.store(head, Relaxed);
            match self.queued.compare_exchange_weak(head, index, Release, Relaxed) {
                Ok(_) => return true,
                Err(actual) => head = actual,
            }
        }
    }

    /// Any thread can hand over each queued value once in push order; returns the count.
    pub fn drain(&self, mut f: impl FnMut(T)) -> usize {
        let mut head = self.queued.swap(0, Acquire);
        let mut reversed = 0;
        while head != 0 {
            let node = &self.nodes[head as usize - 1];
            let next = node.next.load(Relaxed);
            node.next.store(reversed, Relaxed);
            reversed = head;
            head = next;
        }
        let mut count = 0;
        while reversed != 0 {
            let index = reversed;
            let node = &self.nodes[index as usize - 1];
            reversed = node.next.load(Relaxed);
            // SAFETY: the swap gave this drainer the published node and its initialized payload.
            let value = unsafe { (*node.value.get()).assume_init_read() };
            self.return_free(index);
            count += 1;
            f(value);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use std::sync::Arc;

    fn values_come_out_in_push_order_and_their_nodes_return_to_the_pool() {
        let list = PoolList::new(8).unwrap();
        for n in 1..=8 { assert!(list.push(n)); }
        let mut values = Vec::new();
        assert_eq!(list.drain(|n| values.push(n)), 8);
        assert_eq!(values, (1..=8).collect::<Vec<_>>());
        for n in 1..=list.capacity() { assert!(list.push(n)); }
        assert!(!list.push(9));
    }

    fn an_empty_pool_refuses_a_push_and_takes_it_after_a_drain() {
        let list = PoolList::new(1).unwrap();
        assert!(list.push(1));
        assert!(!list.push(2));
        assert_eq!(list.drain(|n| assert_eq!(n, 1)), 1);
        assert!(list.push(2));
    }

    fn new_refuses_no_nodes_and_more_than_the_index_holds() {
        assert!(PoolList::<u8>::new(0).is_none());
        assert!(PoolList::<u8>::new(0xffff).is_none());
        assert!(PoolList::<u8>::new(0xfffe).is_some());
    }

    fn a_pop_suspended_across_a_reuse_of_its_node_takes_nothing_stale() {
        let list = PoolList::new(2).unwrap();
        let seen = list.free_top().unwrap();
        assert!(list.push(1));
        assert!(list.push(2));
        assert_eq!(list.drain(|n| {
            // The second node is queued again while the originally observed first node is free.
            if n == 2 { assert!(list.push(3)); }
        }), 2);
        assert_eq!(list.free.load(Relaxed) & 0xffff, seen.0 & 0xffff);
        assert!(!list.take_free(seen));
        assert_eq!(list.drain(|n| assert_eq!(n, 3)), 1);
        assert!(list.push(4));
        assert!(list.push(5));
        assert!(!list.push(6));
        let mut values = Vec::new();
        assert_eq!(list.drain(|n| values.push(n)), 2);
        assert_eq!(values, [4, 5]);
    }

    fn the_last_generation_retires_the_free_stack_before_it_can_wrap() {
        // Reduced-generation control: a wrapping counter of the same width would restore the first head value after its fourth pop.
        let list = PoolList::with_last_generation(2, 3).unwrap();
        let seen = list.free_top().unwrap();
        let mut values = Vec::new();
        for n in 1..=3 {
            assert!(list.push(n));
            assert!(!list.take_free(seen));
            assert_eq!(list.drain(|value| values.push(value)), 1);
            assert!(!list.take_free(seen));
        }
        assert_eq!(values, [1, 2, 3]);
        assert!(!list.push(4));
        assert!(!list.push(5));
        assert_eq!(list.drain(|_| panic!("no command queued")), 0);
    }

    fn pushes_from_several_threads_lose_nothing_and_keep_each_threads_order() {
        let list = Arc::new(PoolList::new(8).unwrap());
        std::thread::scope(|scope| {
            for who in 0..4 {
                let list = Arc::clone(&list);
                scope.spawn(move || for n in 0..2000 {
                    while !list.push((who, n)) { std::thread::yield_now(); }
                });
            }
            let mut next = [0; 4];
            let mut count = 0;
            while count < 8000 {
                count += list.drain(|(who, n)| {
                    assert_eq!(n, next[who]);
                    next[who] += 1;
                });
                std::thread::yield_now();
            }
            assert_eq!(next, [2000; 4]);
        });
        for n in 0..8 { assert!(list.push((0, n))); }
        assert!(!list.push((0, 8)));
    }

    #[test]
    fn case_table() {
        values_come_out_in_push_order_and_their_nodes_return_to_the_pool();
        an_empty_pool_refuses_a_push_and_takes_it_after_a_drain();
        new_refuses_no_nodes_and_more_than_the_index_holds();
        a_pop_suspended_across_a_reuse_of_its_node_takes_nothing_stale();
        the_last_generation_retires_the_free_stack_before_it_can_wrap();
        pushes_from_several_threads_lose_nothing_and_keep_each_threads_order();
    }
    }
}
