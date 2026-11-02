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