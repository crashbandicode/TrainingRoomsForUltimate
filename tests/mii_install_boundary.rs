//! Executes the same shared/editor installation dispatch used by native init().
#[path = "../src/runtime/mii/mii_editor_bridge/install.rs"]
mod install;
use std::cell::RefCell;

#[test]
fn unavailable_editor_installs_retained_support_and_no_editor_work() {
    let calls = RefCell::new(Vec::new());
    install::run(false, || { calls.borrow_mut().push("retained_base_catalog"); true },
        || panic!("editor-only guards/native reads must not run"),
        || calls.borrow_mut().push("training_stage"),
        || panic!("editor hooks/default/trace/lifecycle/keyboard/resume must not run"));
    assert_eq!(*calls.borrow(), ["retained_base_catalog", "training_stage"]);
}

#[test]
fn rejected_retained_contract_installs_nothing() {
    for available in [false, true] {
        install::run(available, || false, || panic!("no editor preflight"),
            || panic!("no stage hook"), || panic!("no editor hooks"));
    }
}

#[test]
fn available_editor_keeps_shared_then_editor_order() {
    let calls = RefCell::new(Vec::new());
    install::run(true, || { calls.borrow_mut().push("retained"); true },
        || { calls.borrow_mut().push("editor_preflight"); true },
        || calls.borrow_mut().push("training_stage"), || calls.borrow_mut().push("editor"));
    assert_eq!(*calls.borrow(), ["retained", "editor_preflight", "training_stage", "editor"]);
}

#[test]
fn rejected_editor_preflight_does_not_reject_retained_stage_or_catalog() {
    let calls = RefCell::new(Vec::new());
    install::run(true, || true, || false,
        || calls.borrow_mut().push("training_stage"), || panic!("rejected editor"));
    assert_eq!(*calls.borrow(), ["training_stage"]);
}

#[test]
fn availability_and_native_call_sites_bind_the_tested_boundary() {
    // The native editor is available (operator 2026-11-05, the Mii editor bring-over); the capability still gates every
    // native entry point, so switching it off again restores the tested refusals below.
    let menu = include_str!("../src/model/mii_root/mii_menu.rs");
    assert!(menu.contains("pub const NATIVE_EDITOR_AVAILABLE: bool = true;"));
    let bridge = include_str!("../src/runtime/mii/mii_editor_bridge/mod.rs");
    let init = bridge.split("pub(crate) fn init() {").nth(1).unwrap();
    let (shared, editor) = init.split_once("fn init_editor(base: usize) {").unwrap();
    assert!(shared.contains("install::run(crate::engine::mii_root::mii_menu::NATIVE_EDITOR_AVAILABLE,"));
    assert!(shared.contains("fingerprints::RETAINED_SEAMS"));
    assert!(shared.contains("BASE.store(base, Ordering::Release)"));
    assert!(shared.contains("RETAINED_ENABLED.store(true, Ordering::Release)"));
    assert!(shared.contains("skyline::install_hook!(training_state)"));
    for operation in ["native_default::init", "connected::init", "lifecycle::init", "ldn_lifecycle::init",
        "name_keyboard::init", "resume_adapter::init", "filesystem::init", "editor_parent", "edit_frame"] {
        assert!(!shared.contains(operation), "editor-only {operation} leaked into shared init");
        assert!(editor.contains(operation), "retained implementation lost {operation}");
    }
    let training = bridge.split("unsafe fn training_state(").nth(1).unwrap().split("unsafe fn materialize_default").next().unwrap();
    assert!(training.find("training_entry(context)").unwrap() < training.find("NATIVE_EDITOR_AVAILABLE").unwrap());
    for entry in ["pub(crate) fn request(", "fn begin_session("] {
        assert!(bridge.split(entry).nth(1).unwrap().split("NATIVE_EDITOR_AVAILABLE").next().unwrap().len() < 300);
    }
}
