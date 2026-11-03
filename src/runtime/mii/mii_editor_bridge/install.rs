//! The real bridge installation boundary, executable with host operation spies.
//! Shared stage/catalog support must not install the unavailable editor's hooks.
pub fn run(
    editor_available: bool,
    retained: impl FnOnce() -> bool,
    editor_preflight: impl FnOnce() -> bool,
    training_stage: impl FnOnce(),
    editor: impl FnOnce(),
) {
    if !retained() { return; }
    // Read every applicable guard before any shared hook mutates its site.
    let editor_ready = editor_available && editor_preflight();
    training_stage();
    if editor_ready { editor(); }
}
