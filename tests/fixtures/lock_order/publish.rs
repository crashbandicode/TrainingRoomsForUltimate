// tests/lock_order.rs fixture: the publishing path's lock.
use hos_lock::Mutex;

pub static B: Mutex<u32> = Mutex::new(0);

pub fn publish_like() {
    *B.lock() = 1;
}
