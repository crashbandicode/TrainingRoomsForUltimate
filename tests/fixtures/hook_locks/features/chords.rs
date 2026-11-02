//! Fixture (never compiled).
static STATE: Mutex<State> = Mutex::new(State::new());

/// Seeded violation: the worker reaches this, and it takes the hook's lock with a method call.
pub fn refresh_rows(rows: u8) {
    STATE.lock().rows = rows;
}

/// The hook: takes STATE every frame (not reachable from the worker, must not be reported).
pub fn on_poll() {
    let _state = STATE.lock();
}

pub fn tick() {
    let _state = STATE.lock();
}