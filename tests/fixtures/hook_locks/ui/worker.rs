//! Fixture (never compiled): a plugin thread that reaches a hook's lock through a method call and through a helper.
pub fn run() {
    let settings = Settings::default();
    // A method call whose name is unique among the runtime's functions: followed to chords::refresh_rows.
    settings.refresh_rows(1);
    // A path call to a function that hands MOTION by reference to a lock helper.
    crate::runtime::macros::named_macros::reserve();
    // A method name two runtime functions carry: not followed, listed as ambiguous.
    settings.tick();
    // A local function and an atomic method of the same name as another module's lock taker: neither reaches named_macros::store.
    store(1);
    FLAG.store(true, Ordering::Release);
    // This model method cannot call a runtime free function merely sharing its name.
    settings.free_only();
}

pub fn direct() {
    crate::runtime::features::chords::free_only();
}

fn store(value: u8) {
    let _ = value;
}
