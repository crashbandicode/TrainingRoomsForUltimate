//! Fixture (never compiled).
static MOTION: RwLock<Motion> = RwLock::new(Motion::new());

/// Seeded violation: the lock handed by reference to the carried helper.
pub fn reserve() {
    let mut motion = lock_write(&MOTION);
    motion.reserve();
}

pub fn tick() {
    let _motion = lock_read(&MOTION);
}