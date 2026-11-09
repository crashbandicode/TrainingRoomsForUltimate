// tests/lock_order.rs fixture (never compiled): the menu worker's entry (`worker::run`), holding the hook's locks.
pub fn run() {
    *hook::HELD.lock() = 2;
    if let Some(mut g) = hook::TRIED.try_lock() {
        *g = 2;
    }
    *hook::POLLED.lock() = 2;
    let _ = read(&MENU);
    assign(&MENU, 3);
    let role = 0;
    let _g = hook::ROLES[role].lock();
}
