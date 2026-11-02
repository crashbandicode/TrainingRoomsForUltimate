//! Fixture (never compiled).
static MOTION: RwLock<Motion> = RwLock::new(Motion::new());

/// Seeded violation: the lock handed by reference to the carried helper.
pub fn reserve() {
    let mut motion = lock_write(&MOTION);
    motion.reserve();
}

impl OtherSettings {
pub fn tick(&self) {
    let _motion = lock_read(&MOTION);
}
}
/// Must not be reported: the worker's `store(..)` is its own local function.
pub fn store() {
    let _motion = lock_write(&MOTION);
}
