// tests/lock_order.rs fixture (never compiled): a game-thread hook that waits on two locks, polls a third, and reads a typed static.
use hos_lock::Mutex;

pub static HELD: Mutex<u32> = Mutex::new(0);
pub static TRIED: Mutex<u32> = Mutex::new(0);
pub static POLLED: Mutex<u32> = Mutex::new(0);
pub static ROLES: [Mutex<u32>; 2] = [Mutex::new(0), Mutex::new(0)];

#[skyline::hook(offset = 0x10)]
unsafe fn on_frame() {
    *HELD.lock() += 1;
    *TRIED.lock() += 1;
    if let Some(mut g) = POLLED.try_lock() {
        *g += 1;
    }
    let _ = read(&MENU);
    *ROLES[1].lock() += 1;
}
