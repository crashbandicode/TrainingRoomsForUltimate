//! Exercise the actual native adapter against a synchronous SDK boundary double.
//! This proves our storage/lease protocol, not the SDK's internal allocation behavior.
extern crate self as skyline;

#[allow(dead_code, unused_imports)]
#[path = "../src/model/mii/mod.rs"]
pub mod mii;
mod engine {
    pub use super::mii;
    pub mod compat {
        pub mod logging {
            pub fn diagnostic(_: std::fmt::Arguments<'_>) {}
        }
    }
}
#[allow(dead_code)]
#[path = "../src/runtime/mii/native_keyboard/inline_keyboard.rs"]
mod adapter;

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
static THREAD: AtomicUsize = AtomicUsize::new(1);
static WORK: AtomicUsize = AtomicUsize::new(0);
static PHASE: AtomicUsize = AtomicUsize::new(0);
static FAIL: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicUsize = AtomicUsize::new(0);
static INITIALIZED: AtomicUsize = AtomicUsize::new(0);

pub mod nn {
    pub mod os {
        /// # Safety
        /// Test-only native-thread identity; no pointed-to object is read.
        #[allow(non_snake_case)]
        pub unsafe fn GetCurrentThread() -> *mut std::ffi::c_void {
            crate::THREAD.load(crate::SeqCst) as *mut _
        }
    }
}

#[export_name = "_ZN2nn5swkbd14InlineKeyboardC1Ev"]
extern "C" fn construct(_: *mut c_void) {
    assert!(adapter::active());
    assert_eq!(PHASE.swap(1, SeqCst), 0);
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboardD1Ev"]
extern "C" fn destruct(_: *mut c_void) {
    assert!(adapter::active(), "lease must remain owned during destruction");
    let phase = PHASE.swap(0, SeqCst);
    assert!(phase == 4 || (phase == 1 && FAIL.load(SeqCst)));
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard25GetRequiredWorkBufferSizeEv"]
extern "C" fn required_size() -> u64 { 0x698 }
#[export_name = "_ZN2nn5swkbd30InlineKeyboardForLibraryApplet10InitializeEPvbi"]
unsafe extern "C" fn initialize(_: *mut c_void, work: *mut c_void, display: bool, mode: i32) -> bool {
    assert_eq!(PHASE.load(SeqCst), 1);
    assert!(display);
    assert_eq!(mode, 1);
    assert_eq!(work as usize % 8, 0);
    let bytes = std::slice::from_raw_parts_mut(work.cast::<u8>(), 0x698);
    assert!(bytes.iter().all(|b| *b == 0), "every owner receives freshly zeroed work");
    let previous = WORK.swap(work as usize, SeqCst);
    assert!(previous == 0 || previous == work as usize, "static region stays stable");
    bytes.fill(0xa5);
    if FAIL.load(SeqCst) { return false; }
    PHASE.store(2, SeqCst);
    true
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard9DisappearEv"]
extern "C" fn disappear(_: *mut c_void) { assert_eq!(PHASE.swap(3, SeqCst), 2); }
#[export_name = "_ZN2nn5swkbd14InlineKeyboard8FinalizeEv"]
unsafe extern "C" fn finalize(_: *mut c_void) {
    assert!(adapter::Lease::acquire().is_none(), "Finalize still owns the physical keyboard");
    assert_eq!(PHASE.swap(4, SeqCst), 3);
    let bytes = std::slice::from_raw_parts(WORK.load(SeqCst) as *const u8, 0x698);
    assert!(bytes.iter().all(|b| *b == 0xa5), "work must survive until Finalize returns");
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard4CalcEv"]
unsafe extern "C" fn calc(_: *mut c_void) -> i32 {
    let callback: unsafe extern "C" fn() = std::mem::transmute(INITIALIZED.load(SeqCst));
    callback();
    let callback: unsafe extern "C" fn() = std::mem::transmute(CANCEL.load(SeqCst));
    callback();
    3
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard6AppearERKNS0_9AppearArgE"]
extern "C" fn appear(_: *mut c_void, _: *const c_void) {}
#[export_name = "_ZN2nn5swkbd9InputText3SetEPKDs"]
extern "C" fn text_set(_: *mut c_void, _: *const u16) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard12SetInputTextERKNS0_9InputTextE"]
extern "C" fn input_text(_: *mut c_void, _: *const c_void) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard12SetCursorPosEi"]
extern "C" fn cursor(_: *mut c_void, _: i32) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard24SetChangedStringCallbackEPFvPKNS0_16ChangedStringArgEE"]
extern "C" fn changed(_: *mut c_void, _: unsafe extern "C" fn(*const c_void)) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard22SetMovedCursorCallbackEPFvPKNS0_14MovedCursorArgEE"]
extern "C" fn moved(_: *mut c_void, _: unsafe extern "C" fn(*const c_void)) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard23SetDecidedEnterCallbackEPFvPKNS0_15DecidedEnterArgEE"]
extern "C" fn entered(_: *mut c_void, _: unsafe extern "C" fn(*const c_void)) {}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard24SetDecidedCancelCallbackEPFvvE"]
extern "C" fn cancelled(_: *mut c_void, callback: unsafe extern "C" fn()) {
    CANCEL.store(callback as usize, SeqCst);
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard29SetFinishedInitializeCallbackEPFvvE"]
extern "C" fn initialized(_: *mut c_void, callback: unsafe extern "C" fn()) {
    INITIALIZED.store(callback as usize, SeqCst);
}
#[export_name = "_ZN2nn5swkbd14InlineKeyboard27SetFinishedKeyboardCallbackEPFvvE"]
extern "C" fn finished(_: *mut c_void, _: unsafe extern "C" fn()) {}

#[test]
fn actual_adapter_preserves_storage_and_exclusive_lifetime() {
    use adapter::{Keyboard, Lease, Text};
    let text = Text::new(&[65, 66], 2).unwrap();
    unsafe {
        assert!(Keyboard::open(text, 2).is_none(), "wrong owner cannot acquire");
        let lease = Lease::acquire().unwrap();
        assert!(Keyboard::open(text, 1).is_none(), "synchronous owner excludes inline");
        drop(lease);
        FAIL.store(true, SeqCst);
        assert!(Keyboard::open(text, 1).is_none());
        assert!(!adapter::active(), "failed Initialize destructs before release");
        FAIL.store(false, SeqCst);
        for _ in 0..3 {
            let mut keyboard = Keyboard::open(text, 1).unwrap();
            assert!(Keyboard::open(text, 1).is_none());
            let poll = keyboard.poll().unwrap();
            assert_eq!(poll.text.units(), &[65, 66]);
            assert!(matches!(poll.decision, Some(adapter::Decision::Cancel)));
            THREAD.store(2, SeqCst);
            assert!(keyboard.poll().is_none());
            assert!(!keyboard.close(), "foreign thread cannot finalize");
            assert!(Lease::acquire().is_none());
            THREAD.store(1, SeqCst);
            assert!(keyboard.close());
            assert!(keyboard.close(), "closed owner is idempotent");
            assert!(!adapter::active());
        }
        // Interrupted transport polls its final values then closes on the owner.
        let mut interrupted = Keyboard::open(text, 1).unwrap();
        let snapshot = interrupted.poll().unwrap().text;
        assert!(interrupted.close());
        let orphan = Keyboard::open(snapshot, 1).unwrap();
        THREAD.store(2, SeqCst);
        drop(orphan);
        assert!(adapter::active(), "orphan retains the lease and SDK-owned storage");
        assert!(Lease::acquire().is_none());
        assert_eq!(PHASE.load(SeqCst), 2, "Drop must not finalize on a foreign thread");
    }
}
