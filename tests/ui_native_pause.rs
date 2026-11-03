//! R3-3: production layout callback + pane walker + real input-delay adapter.
//! Native structs use a host shim; native ABI qualification is a separate gate.
extern crate self as skyline;
#[path = "../src/model.rs"]
#[allow(dead_code)]
pub mod model;
// The exact pkg6 value-only protocol is the tested subject, not its unrelated
// mode classifier. This dependency shim matches the existing observed family;
// it is not native/mode qualification or a replacement lifetime producer.
pub mod engine { pub mod mii { pub mod backend {
    pub struct Layout;
    impl Layout {
        pub fn quickplay_waiting_mode(mode: u32) -> bool {
            matches!(mode & 0x7fff_ffff, 0x0701_0100..=0x0701_01ff | 0x0701_0400)
        }
    }
} } }
#[path = "../src/model/lobby/practice_policy.rs"]
mod practice_policy;
pub mod nn { pub mod ui2d {
    #[repr(C)] pub struct PaneNode { pub prev: *mut PaneNode, pub next: *mut PaneNode }
    #[repr(C)] pub struct Pane {
        pub link: PaneNode, pub children_list: PaneNode, pub name: [u8; 25],
        pub alpha: u8, pub flags: u8, pub pos_x: f32, pub pos_y: f32,
        pub size_x: f32, pub size_y: f32, pub rot_z: f32,
    }
    impl Pane { pub fn set_visible(&mut self, on: bool) { self.flags = u8::from(on); } }
    pub enum PaneFlag { IsGlobalMatrixDirty = 1 }
    #[repr(C)] pub struct Picture { pub pane: Pane, pub vertex_colors: [[u8; 4]; 4] }
    #[repr(C)] pub struct TextBox {
        pub pane: Pane, pub text_buf_len: u16, pub text_len: u16, pub text_buf: *mut u16,
        pub bits: u32, pub text_position: u32,
    }
    impl TextBox { pub fn set_color(&mut self, _: u8, _: u8, _: u8, _: u8) {} }
    pub struct Layout { pub root_pane: *mut Pane, pub name: &'static [u8] }
} }
#[path = "../src/runtime/ui/pane.rs"] pub mod pane;
#[path = "../src/runtime/ui/state.rs"] pub mod state;
#[path = "../src/runtime/ui/tick.rs"] pub mod tick;
#[path = "../src/runtime/ui/native_pause.rs"] pub mod native_pause;
pub use runtime::ui::menu_app;

pub mod runtime {
    pub mod alloc { pub use crate::model::hook_stats::{HookId, HookScope}; }
    pub mod diag_log { pub fn diagnostic(_: core::fmt::Arguments<'_>) {} }
    pub mod published {
        pub static INPUT_OWNERS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
        pub fn input_delay() -> u8 { 4 }
    }
    pub mod game {
        pub static TRAINING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
        pub fn is_training_mode() -> bool { TRAINING.load(std::sync::atomic::Ordering::Acquire) }
    }
    pub mod features { pub mod input_delay {
        use crate::model::{input_adapter::InputAdapter, input_delay::MappedInputs};
        pub static ADAPTER: std::sync::Mutex<InputAdapter> = std::sync::Mutex::new(InputAdapter::new());
        pub fn native_pause_epoch() -> u32 { ADAPTER.lock().unwrap().pause_epoch() }
        pub fn native_pause_sample(epoch: u32, layout: usize, registration: bool, active: Option<bool>) -> bool {
            let mut adapter = ADAPTER.lock().unwrap();
            let accepted = adapter.native_pause_sample(&crate::runtime::published::INPUT_OWNERS, epoch, layout, registration, active, 4).is_some();
            if accepted { crate::state::PAUSE_ACTIVE.store(crate::model::input_adapter::owners(&crate::runtime::published::INPUT_OWNERS).contains(crate::model::input_delay::Owners::VANILLA_PAUSE), std::sync::atomic::Ordering::Release); }
            accepted
        }
        pub fn retire_pause_layout(epoch: u32, layout: usize) {
            ADAPTER.lock().unwrap().retire_pause_layout(&crate::runtime::published::INPUT_OWNERS, epoch, layout, 4);
        }
        pub fn retire_native_pause() {
            ADAPTER.lock().unwrap().retire_native_pause(&crate::runtime::published::INPUT_OWNERS, 4);
            crate::state::PAUSE_ACTIVE.store(false, std::sync::atomic::Ordering::Release);
        }
        pub fn poll(input: MappedInputs) -> MappedInputs {
            ADAPTER.lock().unwrap().poll(&crate::runtime::published::INPUT_OWNERS, 4, input).0
        }
    } }
    pub mod macros { pub fn after_layout_draw() {} }
    pub mod lobby {
        pub mod game_mode { pub fn popup_busy() -> Option<bool> { Some(false) } }
        pub mod training { pub mod lobby {
            pub fn requested() -> bool { false }
            pub fn set_ui_ready(_: bool) {}
            pub fn set_native_ui_busy(_: bool) {}
        } }
    }
    pub mod ui {
        pub use crate::{native_pause, pane, state, tick};
        pub mod input { pub static POLLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0); }
        pub mod menu_app { pub fn captures_input() -> bool { false } pub fn note_open() {} }
        pub mod registry {
            use crate::nn::ui2d::Layout;
            pub unsafe fn layout_name<'a>(layout: *mut Layout) -> Option<&'a [u8]> { Some((*layout).name) }
            pub fn holders() -> &'static [*const u8] { &[] }
        }
        pub mod hud {
            pub unsafe fn run(_: *mut crate::nn::ui2d::Layout, _: &[u8]) {}
            pub unsafe fn hud_rule(_: *mut crate::nn::ui2d::Layout, _: &[u8]) {}
        }
        pub mod view {
            pub struct Panes { pub container: *mut crate::nn::ui2d::Pane,
                pub all: [*mut crate::nn::ui2d::Pane; crate::model::panes::MENU_COUNT] }
            impl Panes { pub unsafe fn draw(&self, _: &crate::model::frame::Frame) {} }
        }
        pub mod texture { pub fn tick() {} }
        pub mod requests { pub fn run_menu_requests() {} }
        pub mod thumbs {
            pub fn note_frame(_: &crate::model::frame::Frame) {}
            pub unsafe fn lose_layout() {}
            pub unsafe fn hide() {}
            pub unsafe fn run(_: &super::view::Panes) -> bool { false }
        }
        pub mod chips {
            pub fn note_frame(_: &crate::model::frame::Frame) {}
            pub unsafe fn lose_layout() {}
            pub unsafe fn hide() {}
            pub unsafe fn run(_: &super::view::Panes) -> bool { false }
        }
    }
}

use model::input_delay::{MappedInputs, Owners};
use nn::ui2d::{Layout, Pane};
use runtime::features::input_delay::{poll, ADAPTER};
use runtime::published::INPUT_OWNERS;
use std::sync::atomic::Ordering;
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn stock_tree() -> Vec<Pane> {
    let mut nodes: Vec<Pane> = ["RootPane", "L_staying_help"].into_iter().map(|name| {
        let mut p: Pane = unsafe { core::mem::zeroed() };
        p.name[..name.len()].copy_from_slice(name.as_bytes()); p
    }).collect();
    for p in &mut nodes {
        let head = core::ptr::addr_of_mut!(p.children_list);
        p.children_list.prev = head; p.children_list.next = head;
    }
    let head = core::ptr::addr_of_mut!(nodes[0].children_list);
    let child = core::ptr::addr_of_mut!(nodes[1].link);
    unsafe { (*head).prev = child; (*head).next = child; (*child).prev = head; (*child).next = head; }
    nodes[1].pos_y = -80.0;
    nodes
}

fn queue_input() {
    *ADAPTER.lock().unwrap() = model::input_adapter::InputAdapter::new();
    INPUT_OWNERS.store(0, Ordering::Release);
    runtime::game::TRAINING.store(true, Ordering::Release);
    state::INSTALLED.store(false, Ordering::Release);
    let actual = MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL };
    assert_eq!(poll(actual), MappedInputs::NEUTRAL);
    assert_eq!(ADAPTER.lock().unwrap().queued(), 1);
}

#[test]
fn r3_native_pause_clears_delay_without_any_injected_panes() {
    let _serial = SERIAL.lock().unwrap();
    queue_input();
    let mut tree = stock_tree();
    let mut layout = Layout { root_pane: tree.as_mut_ptr(), name: b"info_training" };
    unsafe { tick::tick(&mut layout, true); }
    tree[1].pos_y = 0.0;
    unsafe { tick::tick(&mut layout, false); }
    assert_ne!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0,
        "native pause is independent of TrModMenu/archive injection");
    assert_eq!(ADAPTER.lock().unwrap().queued(), 0, "takeover clears without a mapping poll");
    tree[1].pos_y = -80.0;
    unsafe { tick::tick(&mut layout, false); }
    assert_eq!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
    let actual = MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL };
    for _ in 0..4 { assert_eq!(poll(actual), MappedInputs::NEUTRAL); }
    assert_eq!(poll(actual), actual);
}

#[test]
fn r3_native_pause_survives_renderer_failure_and_loss_after_begin() {
    let _serial = SERIAL.lock().unwrap();
    queue_input();
    let mut tree = stock_tree();
    let mut layout = Layout { root_pane: tree.as_mut_ptr(), name: b"info_training" };
    tree[1].pos_y = 0.0;
    // True with a stock tree exercises actual failed pane resolution; false
    // represents rejected renderer installation. Neither changes native ownership.
    for installed in [true, false, true, false] {
        state::INSTALLED.store(installed, Ordering::Release);
        unsafe { tick::tick(&mut layout, false); }
        assert_ne!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
        assert_eq!(ADAPTER.lock().unwrap().queued(), 0);
    }
    tree[1].pos_y = -80.0;
    unsafe { tick::tick(&mut layout, false); }
    assert_eq!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
}

#[test]
fn r3_registration_and_scene_retirement_cancel_old_layout_observations() {
    let _serial = SERIAL.lock().unwrap();
    queue_input();
    let mut tree = stock_tree();
    let mut layout = Layout { root_pane: tree.as_mut_ptr(), name: b"info_training" };
    tree[1].pos_y = 0.0;
    unsafe { tick::tick(&mut layout, true); }
    let stale = runtime::features::input_delay::native_pause_epoch();
    // The live native help is gone; missing is unknown, not an invented resume.
    let head = core::ptr::addr_of_mut!(tree[0].children_list);
    unsafe { (*head).next = head; (*head).prev = head; tick::tick(&mut layout, false); }
    assert_ne!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
    // Registration is authoritative even when the allocation address is reused.
    unsafe { tick::tick(&mut layout, true); }
    assert_eq!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
    assert!(!runtime::features::input_delay::native_pause_sample(stale, &mut layout as *mut Layout as usize, false, Some(true)));
    let stale = runtime::features::input_delay::native_pause_epoch();
    runtime::features::input_delay::retire_native_pause();
    assert!(!runtime::features::input_delay::native_pause_sample(stale, &mut layout as *mut Layout as usize, false, Some(true)));
    assert_eq!(ADAPTER.lock().unwrap().queued(), 0);
}

#[test]
fn r3_inactive_old_layout_cannot_retire_the_current_layout() {
    let _serial = SERIAL.lock().unwrap();
    queue_input();
    let mut tree_a = stock_tree();
    let mut tree_b = stock_tree();
    let mut a = Layout { root_pane: tree_a.as_mut_ptr(), name: b"info_training" };
    let mut b = Layout { root_pane: tree_b.as_mut_ptr(), name: b"info_training" };
    tree_b[1].pos_y = 0.0;
    unsafe { tick::tick(&mut a, true); tick::tick(&mut b, true); }
    runtime::game::TRAINING.store(false, Ordering::Release);
    unsafe { tick::tick(&mut a, false); }
    assert_ne!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
    unsafe { tick::tick(&mut b, false); }
    assert_eq!(INPUT_OWNERS.load(Ordering::Acquire) & Owners::VANILLA_PAUSE.bits(), 0);
}

#[test]
fn r3_shared_departure_clears_pause_and_queue_without_lease_or_cpu_frame() {
    let _serial = SERIAL.lock().unwrap();
    for paused in [false, true] {
        queue_input();
        let mut lease = practice_policy::EntryLease::new();
        assert!(!lease.active(), "ordinary idle Training has no operation lease");
        let mut tree = stock_tree();
        let mut layout = Layout { root_pane: tree.as_mut_ptr(), name: b"info_training" };
        tree[1].pos_y = if paused { 0.0 } else { -80.0 };
        unsafe { tick::tick(&mut layout, true); }
        if !paused { poll(MappedInputs { buttons: 2, ..MappedInputs::NEUTRAL }); }
        assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), if paused { Owners::VANILLA_PAUSE.bits() } else { 0 });
        if !paused { assert!(ADAPTER.lock().unwrap().queued() > 0); }
        let stale = runtime::features::input_delay::native_pause_epoch();
        let order = std::cell::RefCell::new(Vec::new());
        runtime::game::TRAINING.store(false, Ordering::Release);
        // Exact production protocol, not direct leaf retirement. No CPU body
        // frame or subsequent old-layout callback occurs across this boundary.
        practice_policy::depart_session(|| {
            assert!(!lease.retire());
            order.borrow_mut().push("lease_released");
        }, || {
            assert_eq!(*order.borrow(), ["lease_released"]);
            runtime::features::input_delay::retire_native_pause();
            order.borrow_mut().push("pause_consumer");
        });
        assert_eq!(*order.borrow(), ["lease_released", "pause_consumer"]);
        assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), 0);
        assert!(!state::PAUSE_ACTIVE.load(Ordering::Acquire));
        assert_eq!(ADAPTER.lock().unwrap().queued(), 0);
        runtime::game::TRAINING.store(true, Ordering::Release);
        // Reentry precedes any fresh Calculate callback.
        assert!(!runtime::features::input_delay::native_pause_sample(
            stale, &mut layout as *mut Layout as usize, false, Some(true)));
        assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), 0);
        assert_eq!(poll(MappedInputs { buttons: 4, ..MappedInputs::NEUTRAL }), MappedInputs::NEUTRAL);
        // Same allocation address, newly registered lifetime.
        tree[1].pos_y = 0.0;
        unsafe { tick::tick(&mut layout, true); }
        assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), Owners::VANILLA_PAUSE.bits());
        assert_eq!(ADAPTER.lock().unwrap().queued(), 0);
        assert!(!runtime::features::input_delay::native_pause_sample(
            stale, &mut layout as *mut Layout as usize, false, Some(false)));
        assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), Owners::VANILLA_PAUSE.bits());
    }
}

#[test]
fn r3_shared_departure_releases_held_lease_before_real_pause_consumer() {
    let _serial = SERIAL.lock().unwrap();
    queue_input();
    let lease = std::cell::RefCell::new(practice_policy::EntryLease::new());
    assert!(lease.borrow_mut().acquire(practice_policy::Entries {
        context: practice_policy::Context::Training, player: 0x1000, cpu: 0x2000,
    }));
    let mut tree = stock_tree();
    let mut layout = Layout { root_pane: tree.as_mut_ptr(), name: b"info_training" };
    tree[1].pos_y = 0.0;
    unsafe { tick::tick(&mut layout, true); }
    let calls = std::cell::Cell::new(0);
    practice_policy::depart_session(|| { assert!(lease.borrow_mut().retire()); }, || {
        assert!(!lease.borrow().active(), "consumer runs after lease guard is dropped");
        calls.set(calls.get() + 1);
        runtime::features::input_delay::retire_native_pause();
    });
    assert_eq!(calls.get(), 1);
    assert_eq!(INPUT_OWNERS.load(Ordering::Acquire), 0);
    assert_eq!(ADAPTER.lock().unwrap().queued(), 0);
}
