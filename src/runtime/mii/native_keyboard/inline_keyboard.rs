//! Shared native adapter. The SDK's Calc dispatches callbacks synchronously.
//! Mii editing and waiting naming share one lease and callback signals. The caller
//! owns its keyboard on one UI thread and never persists its pointers.
//! ABI evidence: private SDK-INLINE-REPORT.md, owned SDK build 5fd4f531.
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use hos_lock::Mutex;

use crate::engine::mii::inline_keyboard::{Limit, Signals};
pub(crate) use crate::engine::mii::inline_keyboard::{Decision, Text};

// Native calls never run with this lock held. Only our SDK callbacks and the
// single owning UI-thread poll access it; Finalize finishes before the next owner.
static SIGNALS: Mutex<Signals> = Mutex::new(Signals {
    limit: Limit::Mii, text: None, decision: None,
    initialized: false, finished: false, invalid: false,
});
static LEASED: AtomicBool = AtomicBool::new(false);
pub(crate) fn active() -> bool { LEASED.load(Ordering::Acquire) }

// Also held around ordinary Training's synchronous ShowKeyboard. There is one
// physical keyboard lease across both transports and the Mii compatibility shim.
pub(super) struct Lease;
impl Lease {
    pub(super) fn acquire() -> Option<Self> {
        LEASED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).ok()?;
        Some(Self)
    }
}
impl Drop for Lease {
    fn drop(&mut self) { LEASED.store(false, Ordering::Release); }
}

pub(super) fn current_thread() -> usize { unsafe { skyline::nn::os::GetCurrentThread() as usize } }

#[repr(C)]
struct Changed {
    text: [u16; 502],
    len: i32,
    dictionary_start: i32,
    dictionary_end: i32,
    cursor: i32,
}

#[repr(C)]
struct Moved {
    text: [u16; 502],
    len: i32,
    cursor: i32,
}

#[repr(C)]
struct Entered {
    text: [u16; 502],
    len: i32,
}

unsafe extern "C" fn changed(arg: *const Changed) {
    let Some(arg) = arg.as_ref() else { return; };
    SIGNALS.lock().change(&arg.text, arg.len, arg.cursor);
}

unsafe extern "C" fn moved(arg: *const Moved) {
    let Some(arg) = arg.as_ref() else { return; };
    SIGNALS.lock().change(&arg.text, arg.len, arg.cursor);
}

unsafe extern "C" fn entered(arg: *const Entered) {
    let Some(arg) = arg.as_ref() else { return; };
    SIGNALS.lock().enter(&arg.text, arg.len);
}

unsafe extern "C" fn cancelled() { SIGNALS.lock().cancel(); }

unsafe extern "C" fn initialized() { SIGNALS.lock().initialized = true; }
unsafe extern "C" fn finished() { SIGNALS.lock().finished = true; }

#[repr(C)]
struct Object { implementation: usize, work: usize }

#[repr(C, align(8))]
struct Work([u8; 0x698]);

struct WorkStorage(UnsafeCell<Work>);
// SAFETY: only the exclusive keyboard Lease can expose or clear this region.
// The lease stays owned through Finalize/Destruct; an orphan keeps it forever.
unsafe impl Sync for WorkStorage {}
static WORK: WorkStorage = WorkStorage(UnsafeCell::new(Work([0; 0x698])));

impl Lease {
    fn prepare_work(&mut self) -> *mut std::ffi::c_void {
        let work = WORK.0.get();
        // SAFETY: acquiring this lease excludes both inline and synchronous
        // keyboard owners. The previous owner finalized before releasing it.
        unsafe { work.write(Work([0; 0x698])); }
        work.cast()
    }
}

#[repr(C)]
struct InputText([u16; 506]);

#[repr(C)]
struct Appear {
    keyboard_type: u32,
    ok_text: [u16; 9],
    optional_left: u16,
    optional_right: u16,
    prediction: u8,
    disable_cancel: u8,
    disabled_keys: u32,
    max_length: i32,
    min_length: i32,
    enable_return: u8,
    padding0: [u8; 3],
    flags: u32,
    use_save_data: u8,
    padding1: [u8; 7],
    user_id: [u8; 16],
}

const _: () = {
    assert!(std::mem::size_of::<Object>() == 0x10);
    assert!(std::mem::size_of::<Work>() == 0x698);
    assert!(std::mem::size_of::<Appear>() == 0x48);
    assert!(std::mem::offset_of!(Appear, flags) == 0x2c);
    assert!(std::mem::offset_of!(Appear, user_id) == 0x38);
    assert!(std::mem::size_of::<Changed>() == 0x3fc);
    assert!(std::mem::size_of::<Moved>() == 0x3f4);
    assert!(std::mem::size_of::<Entered>() == 0x3f0);
};

pub(crate) struct Poll {
    pub text: Text,
    pub decision: Option<Decision>,
    #[cfg(feature = "mii-editor-alpha")]
    pub initialized: bool,
    pub finished: bool,
    pub invalid: bool,
    #[cfg(feature = "mii-editor-alpha")]
    pub state: i32,
}

pub(crate) struct Keyboard {
    object: Object,
    text: Text,
    thread: usize,
    active: bool,
    initialized: bool,
    invalid: bool,
    lease: Option<Lease>,
}

impl Keyboard {
    /// Must run on the bound UI thread. Applet-drawn mode still needs the
    /// operator's console acceptance; the normal inline mode requires a renderer.
    pub(crate) unsafe fn open(text: Text, thread: usize) -> Option<Self> {
        if thread != current_thread() || sdk_work_size() != 0x698 {
            return None;
        }
        let mut lease = Lease::acquire()?;
        *SIGNALS.lock() = Signals { limit: text.limit(), ..Signals::default() };
        let work = lease.prepare_work();
        let mut object = Object { implementation: 0, work: 0 };
        sdk_construct(&mut object);
        if !sdk_initialize(&mut object, work, true, 1) {
            sdk_destruct(&mut object);
            return None;
        }
        sdk_changed(&mut object, changed);
        sdk_moved(&mut object, moved);
        sdk_entered(&mut object, entered);
        sdk_cancelled(&mut object, cancelled);
        sdk_initialized(&mut object, initialized);
        sdk_finished(&mut object, finished);
        let mut input = InputText([0; 506]);
        sdk_text_set(&mut input, text.terminated().as_ptr());
        sdk_input_text(&mut object, &input);
        sdk_cursor(&mut object, text.cursor() as i32);
        let appear = Appear {
            keyboard_type: 0, ok_text: [0; 9], optional_left: 0, optional_right: 0,
            prediction: 0, disable_cancel: 0, disabled_keys: 0,
            max_length: text.limit().units() as i32, min_length: 1,
            enable_return: 0, padding0: [0; 3], flags: 0, use_save_data: 0,
            padding1: [0; 7], user_id: [0; 16],
        };
        sdk_appear(&mut object, &appear);
        Some(Self { object, text, thread, active: true,
                    initialized: false, invalid: false, lease: Some(lease) })
    }

    pub(super) fn on_owner_thread(&self) -> bool { self.thread == current_thread() }

    pub(crate) unsafe fn poll(&mut self) -> Option<Poll> {
        if !self.active || !self.on_owner_thread() { return None; }
        #[cfg(feature = "mii-editor-alpha")]
        let state = sdk_calc(&mut self.object);
        #[cfg(not(feature = "mii-editor-alpha"))]
        sdk_calc(&mut self.object);
        let signals = std::mem::replace(&mut *SIGNALS.lock(),
            Signals { limit: self.text.limit(), ..Signals::default() });
        if let Some(text) = signals.text { self.text = text; }
        self.initialized |= signals.initialized;
        self.invalid |= signals.invalid;
        Some(Poll { text: self.text, decision: signals.decision,
                    #[cfg(feature = "mii-editor-alpha")]
                    initialized: self.initialized,
                    finished: signals.finished, invalid: self.invalid,
                    #[cfg(feature = "mii-editor-alpha")]
                    state })
    }

    /// Caller first copies pending values. Finalize owns its native join; never
    /// free the work buffer or release the editor context before it returns.
    pub(crate) unsafe fn close(&mut self) -> bool {
        if !self.on_owner_thread() { return false; }
        if self.active {
            sdk_disappear(&mut self.object);
            sdk_finalize(&mut self.object);
            sdk_destruct(&mut self.object);
            self.active = false;
            *SIGNALS.lock() = Signals::default();
            self.lease = None;
        }
        true
    }
}

impl Drop for Keyboard {
    fn drop(&mut self) {
        if self.active {
            // A mistaken owner drop must not finalize on a different thread.
            // Keep the static work region leased and unavailable: otherwise
            // an orphaned callback could corrupt the next keyboard's signals.
            // Normal completion and suspension explicitly close on the owner.
            if let Some(lease) = self.lease.take() { std::mem::forget(lease); }
            crate::engine::compat::logging::diagnostic(format_args!(
                "[NativeKeyboard] keyboard_owner_dropped_before_finalize; lease retained"));
        }
    }
}

extern "C" {
    #[link_name = "_ZN2nn5swkbd14InlineKeyboardC1Ev"]
    fn sdk_construct(this: *mut Object);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboardD1Ev"]
    fn sdk_destruct(this: *mut Object);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard25GetRequiredWorkBufferSizeEv"]
    fn sdk_work_size() -> u64;
    #[link_name = "_ZN2nn5swkbd30InlineKeyboardForLibraryApplet10InitializeEPvbi"]
    fn sdk_initialize(this: *mut Object, work: *mut std::ffi::c_void, display: bool, mode: i32) -> bool;
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard8FinalizeEv"]
    fn sdk_finalize(this: *mut Object);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard4CalcEv"]
    fn sdk_calc(this: *mut Object) -> i32;
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard6AppearERKNS0_9AppearArgE"]
    fn sdk_appear(this: *mut Object, arg: *const Appear);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard9DisappearEv"]
    fn sdk_disappear(this: *mut Object);
    #[link_name = "_ZN2nn5swkbd9InputText3SetEPKDs"]
    fn sdk_text_set(this: *mut InputText, text: *const u16);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard12SetInputTextERKNS0_9InputTextE"]
    fn sdk_input_text(this: *mut Object, text: *const InputText);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard12SetCursorPosEi"]
    fn sdk_cursor(this: *mut Object, cursor: i32);
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard24SetChangedStringCallbackEPFvPKNS0_16ChangedStringArgEE"]
    fn sdk_changed(this: *mut Object, callback: unsafe extern "C" fn(*const Changed));
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard22SetMovedCursorCallbackEPFvPKNS0_14MovedCursorArgEE"]
    fn sdk_moved(this: *mut Object, callback: unsafe extern "C" fn(*const Moved));
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard23SetDecidedEnterCallbackEPFvPKNS0_15DecidedEnterArgEE"]
    fn sdk_entered(this: *mut Object, callback: unsafe extern "C" fn(*const Entered));
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard24SetDecidedCancelCallbackEPFvvE"]
    fn sdk_cancelled(this: *mut Object, callback: unsafe extern "C" fn());
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard29SetFinishedInitializeCallbackEPFvvE"]
    fn sdk_initialized(this: *mut Object, callback: unsafe extern "C" fn());
    #[link_name = "_ZN2nn5swkbd14InlineKeyboard27SetFinishedKeyboardCallbackEPFvvE"]
    fn sdk_finished(this: *mut Object, callback: unsafe extern "C" fn());
}

