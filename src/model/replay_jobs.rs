//! Boot-reserved replay resolution slots. The caller supplies synchronization.
//! Native submission, completion, cancellation and polling never grow storage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub epoch: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ticket(u64);

#[derive(Debug, Eq, PartialEq)]
pub enum Poll<R> {
    Pending,
    Retired,
    Ready(R),
}

struct Slot<Q, R> {
    ticket: Ticket,
    identity: Option<Identity>,
    request: Option<Q>,
    result: Option<R>,
}

pub struct Work<Q> {
    pub ticket: Ticket,
    pub identity: Option<Identity>,
    pub request: Q,
}

pub struct Queue<Q, R> {
    slots: Vec<Option<Slot<Q, R>>>,
    sequence: u64,
}

impl<Q, R> Queue<Q, R> {
    /// Startup only. Capacity is fixed for the lifetime of this queue.
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        slots.resize_with(capacity, || None);
        Self { slots, sequence: 0 }
    }

    pub fn submit(&mut self, request: Q, identity: Option<Identity>) -> Result<Ticket, Q> {
        let Some(slot) = self.slots.iter_mut().find(|slot| slot.is_none()) else {
            return Err(request);
        };
        // Never let an ancient handle alias a newly issued request.
        let Some(sequence) = self.sequence.checked_add(1) else { return Err(request); };
        self.sequence = sequence;
        let ticket = Ticket(sequence);
        *slot = Some(Slot { ticket, identity, request: Some(request), result: None });
        Ok(ticket)
    }

    /// Selected work has priority, newest first; durable imports remain FIFO.
    pub fn take_next(&mut self) -> Option<Work<Q>> {
        let index = self.slots.iter().enumerate()
            .filter_map(|(i, slot)| slot.as_ref().filter(|s| s.request.is_some()).map(|s| (i, s)))
            .max_by_key(|(_, s)| (s.identity.is_some(), if s.identity.is_some() {
                s.ticket.0
            } else { u64::MAX - s.ticket.0 }))
            .map(|(i, _)| i)?;
        let slot = self.slots[index].as_mut().unwrap();
        Some(Work { ticket: slot.ticket, identity: slot.identity, request: slot.request.take().unwrap() })
    }

    /// A stale completion cannot publish into a reused slot. Durable imports
    /// have no reader and release their slot as soon as the worker completes.
    pub fn complete(&mut self, ticket: Ticket, result: R) -> bool {
        let Some(place) = self.slots.iter_mut().find(|s| s.as_ref().is_some_and(|s| s.ticket == ticket)) else {
            return false;
        };
        let slot = place.as_mut().unwrap();
        if slot.request.is_some() || slot.result.is_some() { return false; }
        if slot.identity.is_none() { *place = None; } else { slot.result = Some(result); }
        true
    }

    pub fn poll(&mut self, ticket: Ticket) -> Poll<R> {
        let Some(place) = self.slots.iter_mut().find(|s| s.as_ref().is_some_and(|s| s.ticket == ticket)) else {
            return Poll::Retired;
        };
        let Some(result) = place.as_mut().unwrap().result.take() else { return Poll::Pending; };
        *place = None;
        Poll::Ready(result)
    }

    /// Session/selection retirement cancels queued, in-flight and completed
    /// selected work. Durable companion imports survive selection changes.
    pub fn retire(&mut self, current: Identity) {
        for slot in &mut self.slots {
            if slot.as_ref().is_some_and(|s| s.identity.is_some_and(|id| id != current)) {
                *slot = None;
            }
        }
    }

    pub fn has_work(&self) -> bool {
        self.slots.iter().flatten().any(|s| s.result.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const A: Identity = Identity { epoch: 1, revision: 3 };
    const B: Identity = Identity { epoch: 1, revision: 4 };

    #[test]
    fn retired_inflight_cannot_complete_into_reused_slot() {
        let mut q = Queue::new(1);
        let old = q.submit(10, Some(A)).unwrap();
        assert_eq!(q.take_next().unwrap().request, 10);
        q.retire(B);
        let new = q.submit(20, Some(B)).unwrap();
        assert_ne!(old, new);
        assert!(!q.complete(old, 11));
        assert_eq!(q.poll(old), Poll::Retired);
        assert_eq!(q.poll(new), Poll::Pending);
        q.take_next().unwrap();
        assert!(q.complete(new, 21));
        assert!(!q.complete(new, 22));
        assert_eq!(q.poll(new), Poll::Ready(21));
        assert_eq!(q.poll(new), Poll::Retired);
    }

    #[test]
    fn selected_priority_and_durable_fifo_survive_retirement() {
        let mut q = Queue::<_, ()>::new(4);
        let durable = q.submit(1, None).unwrap();
        q.submit(2, None).unwrap();
        q.submit(3, Some(A)).unwrap();
        let chosen = q.submit(4, Some(A)).unwrap();
        assert_eq!(q.submit(5, Some(A)), Err(5));
        let next = q.take_next().unwrap();
        assert_eq!((next.ticket, next.identity, next.request), (chosen, Some(A), 4));
        q.retire(B);
        assert!(!q.complete(chosen, ()));
        assert_eq!(q.take_next().unwrap().request, 1);
        assert!(q.complete(durable, ()));
        assert_eq!(q.poll(durable), Poll::Retired);
        assert_eq!(q.take_next().unwrap().request, 2);
    }

    #[test]
    fn epoch_retirement_discards_unread_completion_and_queued_request() {
        let mut q = Queue::new(2);
        let done = q.submit(1, Some(A)).unwrap();
        q.take_next().unwrap();
        assert!(q.complete(done, 2));
        let waiting = q.submit(3, Some(A)).unwrap();
        q.retire(Identity { epoch: 2, ..A });
        assert_eq!(q.poll(done), Poll::Retired);
        assert_eq!(q.poll(waiting), Poll::Retired);
        assert!(!q.has_work());
    }
}