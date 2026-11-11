#[path = "../src/model/nowait_channel.rs"]
mod nowait_channel;
#[path = "../src/model/catalog_reply.rs"]
mod catalog_reply;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct Count;
fn allocation() {
    let _ = TRACK.try_with(|track| {
        if track.get() { let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1)); }
    });
}
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
fn native<T>(f: impl FnOnce() -> T) -> T {
    ALLOCATIONS.with(|n| n.set(0));
    TRACK.with(|n| n.set(true));
    let result = f();
    TRACK.with(|n| n.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0, "native transport allocated");
    result
}

#[test]
fn native_submit_full_retry_poll_and_reuse_allocate_nothing() {
    let mut replies = catalog_reply::Replies::new();
    for n in 0..16 {
        let job = native(|| catalog_reply::Job::new(replies.begin().unwrap()));
        native(|| replies.rejected(job.rejected()));
        let job = native(|| catalog_reply::Job::new(replies.begin().unwrap()));
        // Snapshot construction occurs on the worker, outside the native counter.
        let value = Arc::new(vec![n; 4096]);
        job.complete(value);
        let received = native(|| replies.poll().unwrap().unwrap());
        assert_eq!(received[0], n);
    }
    let job = native(|| catalog_reply::Job::new(replies.begin().unwrap()));
    drop(job); // a failed/disconnected worker drops its queued jobs
    assert!(native(|| replies.poll().unwrap()).is_err());
    let job = native(|| catalog_reply::Job::new(replies.begin().unwrap()));
    job.complete(Arc::new(vec![23]));
    assert_eq!(native(|| replies.poll().unwrap().unwrap())[0], 23);
}

#[path = "../src/model/reusable_snapshot.rs"]
mod reusable_snapshot;

#[test]
fn helper_snapshot_publish_reader_retirement_and_reuse_allocate_nothing() {
    let mut storage = reusable_snapshot::Storage::new(); // boot allocation
    for n in 0..32u64 {
        let transaction = native(|| storage.publish([n; 512]).unwrap());
        let reader = native(|| transaction.clone());
        native(|| drop(transaction));
        assert!(!storage.available());
        assert_eq!(native(|| storage.publish([99; 512]).err()), Some([99; 512]));
        assert_eq!(reader[511], n);
        native(|| drop(reader));
        assert!(storage.available());
    }
}
#[path = "../src/model/replay_jobs.rs"]
mod replay_jobs;

#[test]
fn replay_request_retirement_completion_and_poll_reuse_allocate_nothing() {
    use replay_jobs::{Identity, Poll, Queue};
    let mut queue = Queue::new(3);
    for revision in 0..64 {
        let id = Identity { epoch: 7, revision };
        native(|| queue.retire(id));
        let old = native(|| queue.submit([revision; 128], Some(id)).unwrap());
        let work = native(|| queue.take_next().unwrap());
        assert_eq!(work.request[127], revision);
        assert_eq!(native(|| queue.poll(old)), Poll::Pending);
        assert!(native(|| queue.complete(old, Ok::<_, ()>(revision))));
        assert_eq!(native(|| queue.poll(old)), Poll::Ready(Ok(revision)));
        assert_eq!(native(|| queue.poll(old)), Poll::Retired);
    }
}

#[test]
fn boot_reserved_import_box_and_endpoint_survive_full_retry_and_completion() {
    let mut replies = catalog_reply::Replies::new();
    let mut payload = Some(Box::new([0u64; 256]));
    let address = payload.as_ref().unwrap().as_ptr();
    for n in 0..32 {
        let (value, job) = native(|| (payload.take().unwrap(), catalog_reply::Job::new(replies.begin().unwrap())));
        native(|| { payload = Some(value); replies.rejected(job.rejected()); });
        let (mut value, job) = native(|| (payload.take().unwrap(), catalog_reply::Job::new(replies.begin().unwrap())));
        value[255] = n; // existing file worker owns the payload while processing
        job.complete(value);
        payload = Some(native(|| replies.poll().unwrap().unwrap()));
        assert_eq!(payload.as_ref().unwrap().as_ptr(), address);
        assert_eq!(payload.as_ref().unwrap()[255], n);
    }
}

#[path = "../src/model/catalog_mailbox.rs"]
mod catalog_mailbox;
#[path = "../src/model/close_handoff.rs"]
mod close_handoff;

#[test]
fn native_close_first_full_cancel_reopen_and_retry_allocate_nothing() {
    use close_handoff::{channel, Intent};
    let (producer, mut consumer) = channel(1);
    for epoch in 0..32 {
        let opening = 1 + epoch as u32 * 4;
        let value = Arc::new(vec![epoch; 128]);
        let make = |opening| Intent { opening, epoch, value: Arc::clone(&value) };
        assert!(producer.post(make(opening)).is_ok());
        let retry = producer.post(make(opening + 2)).err().unwrap();
        assert!(native(|| consumer.observe(opening, epoch, true)).is_none());
        assert!(producer.post(retry).is_ok());
        // Coalesced reopen cancels the old close and retains the current one.
        assert!(native(|| consumer.observe(opening + 2, epoch, true)).is_none());
        let received = native(|| consumer.observe(opening + 3, epoch, false)).unwrap();
        assert!(Arc::ptr_eq(&received, &value));
        assert!(native(|| consumer.observe(opening + 3, epoch, false)).is_none());
    }
}
#[test]
fn native_catalog_post_full_retry_and_worker_take_allocate_nothing() {
    use catalog_mailbox::{channel, Publication};
    let (producer, mut consumer) = channel(1);
    for epoch in 0..32 {
        let catalog = Arc::new(vec![epoch; 256]); // file worker
        let make = || Publication { sequence: 4, epoch, value: Arc::clone(&catalog) };
        assert!(native(|| producer.post(make())).is_ok());
        let retained = native(|| producer.post(make())).err().unwrap();
        assert!(Arc::ptr_eq(&consumer.take(4, epoch, true).unwrap(), &catalog));
        assert!(native(|| producer.post(retained)).is_ok());
        assert!(Arc::ptr_eq(&consumer.take(4, epoch, true).unwrap(), &catalog));
    }
}

#[path = "../src/model/pooled_reply.rs"]
mod pooled_reply;
#[test]
fn reply_pool_first_use_full_drop_and_reuse_allocate_nothing() {
    let pool = pooled_reply::Pool::new(1);
    for n in 0..64 {
        let (ticket, job) = native(|| pool.claim().unwrap());
        assert!(native(|| pool.claim()).is_none());
        native(|| drop(ticket));
        assert!(native(|| pool.claim()).is_none());
        native(|| job.complete(n));
        let (ticket, job) = native(|| pool.claim().unwrap());
        assert!(matches!(native(|| ticket.try_recv()), Err(nowait_channel::TryRecvError::Empty)));
        native(|| drop(job));
        assert_eq!(native(|| ticket.try_recv()), Ok(Err(())));
        native(|| drop(ticket));
    }
}
