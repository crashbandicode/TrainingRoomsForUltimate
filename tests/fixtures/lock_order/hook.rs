// tests/lock_order.rs fixture: a game-thread hook that takes the two locks in the other order.
#[skyline::hook(offset = 0x10)]
unsafe fn on_frame() {
    match *publish::B.lock() {
        1 => {
            *worker::A.lock() = 2;
        }
        _ => {}
    }
}
