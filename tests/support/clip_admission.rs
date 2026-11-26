//! Production worker service and both queue endpoints; the roster is an admission oracle, not a loader here.
use std::sync::{Mutex, atomic::Ordering};
pub mod model { #[path = "PROJECT_ROOT/src/model/drop_ring.rs"] pub mod drop_ring; }
pub mod engine {
    pub mod compat { pub mod logging { pub fn diagnostic(_: std::fmt::Arguments) {} } }
    pub mod macro_ui { pub mod scrub_track {
        pub struct Slot;
        pub static SLOT: Slot = Slot;
        impl Slot { pub fn released(&self) -> bool { true } }
    } }
}
pub mod runtime {
    pub mod macros { pub mod port { pub mod sync {
        pub fn assign(flag: &std::sync::atomic::AtomicBool, value: bool) { flag.store(value, std::sync::atomic::Ordering::Release); }
    } } }
    pub mod compat { pub mod common { pub mod menu {
        pub static MENU_RECEIVED_INPUT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    } } }
    pub mod features { pub mod edge_juggle { pub mod practice_cpu { pub fn service_practice_cpu_selection() {} } } }
    pub mod ui { pub mod worker { pub fn wake() {} } }
    pub mod diag_log { pub fn diagnostic(_: std::fmt::Arguments) {} }
}
#[path = "PROJECT_ROOT/src/runtime/roster/roster_requests.rs"] mod roster_requests;
mod character_roster {
    pub fn player_kind() -> Option<u32> { Some(6) }
    pub fn request_player_kind(kind: u32) -> bool { kind == 6 }
    pub fn request_cpu_kind(kind: u32) -> bool { kind == 7 }
}
#[derive(Default)]
struct Editor { wanted: Option<(usize, i32)>, loading: Option<(usize, i32)>, notice: String }
impl Editor {
    fn wanted_fighter(&self) -> Option<(usize, i32)> { self.wanted }
    fn take_wanted_fighter(&mut self) -> Option<(bool, i32)> {
        self.loading = self.wanted.take();
        self.loading.map(|(role, kind)| (role == 0, kind))
    }
    fn fighter_loading(&self) -> Option<(usize, i32)> { self.loading }
    fn tick_fighter_load(&mut self) -> bool { false }
    fn cancel_fighter(&mut self) -> bool { self.loading.take().is_some() }
}
static EDITOR: Mutex<Option<Editor>> = Mutex::new(None);
fn notify(message: String) { EDITOR.lock().unwrap().as_mut().unwrap().notice = message; }
// WORKER
fn ask(role: usize, kind: i32) {
    *EDITOR.lock().unwrap() = Some(Editor { wanted: Some((role, kind)), ..Editor::default() });
    service_wanted_fighter();
}
fn main() {
    for (role, refused, accepted) in [(0, 7, 6), (1, 6, 7)] {
        ask(role, refused);
        roster_requests::request_player_kind_replied(6);
        roster_requests::service();
        service_wanted_fighter();
        {
            let guard = EDITOR.lock().unwrap();
            let e = guard.as_ref().unwrap();
            assert_eq!(e.loading, None, "refused role {role} still loading after reply");
            assert!(e.notice.to_lowercase().contains("refus"), "no refusal notice for role {role}: {}", e.notice);
        }
        assert_eq!(roster_requests::take_reply(), Some((6, true)), "combo answer was consumed");
        assert_eq!(roster_requests::take_reply(), None, "clip answer leaked into combo replies");
        ask(role, refused);
        EDITOR.lock().unwrap().as_mut().unwrap().cancel_fighter();
        roster_requests::service();
        service_wanted_fighter();
        assert!(EDITOR.lock().unwrap().as_ref().unwrap().notice.is_empty(), "B-cancel received a stale refusal");
        // An old refused request and a newer accepted one are serviced in the same game pass.
        ask(role, refused);
        ask(role, accepted);
        roster_requests::service();
        service_wanted_fighter();
        let guard = EDITOR.lock().unwrap();
        let e = guard.as_ref().unwrap();
        assert_eq!(e.loading, Some((role, accepted)), "accepted must wait for actual settlement");
        assert!(e.notice.is_empty(), "stale refusal poisoned new request");
    }
}
