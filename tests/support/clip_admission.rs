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
/// The roster as the ticket sees it: a role takes its own fighter (6 for Player 1, 7 for the CPU) while nothing waits.
mod character_roster {
    use std::sync::Mutex;
    pub static PENDING: Mutex<[Option<u32>; 2]> = Mutex::new([None; 2]);
    pub static BEGUN: Mutex<[bool; 2]> = Mutex::new([false; 2]);
    pub fn player_kind() -> Option<u32> { Some(6) }
    fn ask(role: usize, kind: u32, takes: u32) -> bool {
        let mut pending = PENDING.lock().unwrap();
        if kind != takes || pending[role].is_some() { return false; }
        pending[role] = Some(kind);
        true
    }
    pub fn request_player_kind(kind: u32) -> bool { ask(0, kind, 6) }
    pub fn request_cpu_kind(kind: u32) -> bool { ask(1, kind, 7) }
    /// A waiting direct request for `kind` with no load begun is removed.
    pub fn withdraw_direct(player: bool, kind: u32) -> bool {
        let role = usize::from(!player);
        let mut pending = PENDING.lock().unwrap();
        if pending[role] != Some(kind) || BEGUN.lock().unwrap()[role] { return false; }
        pending[role] = None;
        true
    }
    pub fn notice_blocked() -> bool { PENDING.lock().unwrap().iter().any(Option::is_some) }
    pub fn reset() { *PENDING.lock().unwrap() = [None; 2]; *BEGUN.lock().unwrap() = [false; 2]; }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FighterOutcome { Loading, Committed, Refused, Withdrawn }
const FIGHTER_REFUSED: &str = "The roster refused the fighter switch";
#[derive(Default)]
struct Editor { wanted: Option<(usize, i32)>, loading: Option<(usize, i32)>, notice: String, committed: bool, withdrawing: bool }
impl Editor {
    fn note_fighter_outcome(&mut self, outcome: FighterOutcome) -> bool {
        match outcome {
            FighterOutcome::Loading => false,
            FighterOutcome::Committed => !std::mem::replace(&mut self.committed, true),
            FighterOutcome::Refused | FighterOutcome::Withdrawn => self.loading.take().is_some(),
        }
    }
    fn wanted_fighter(&self) -> Option<(usize, i32)> { self.wanted }
    fn take_wanted_fighter(&mut self) -> Option<(bool, i32)> {
        self.loading = self.wanted.take();
        self.loading.map(|(role, kind)| (role == 0, kind))
    }
    fn fighter_loading(&self) -> Option<(usize, i32)> { self.loading }
    fn cancel_fighter(&mut self) -> bool {
        if self.wanted.take().is_some() { return false; }
        self.loading.is_some() && !std::mem::replace(&mut self.withdrawing, true)
    }
}
static EDITOR: Mutex<Option<Editor>> = Mutex::new(None);
fn notify(message: String) { EDITOR.lock().unwrap().as_mut().unwrap().notice = message; }
// WORKER
fn ask(role: usize, kind: i32) {
    *EDITOR.lock().unwrap() = Some(Editor { wanted: Some((role, kind)), ..Editor::default() });
    service_wanted_fighter();
}
/// B in the editor while a switch is asked, as `replay_capture::on_input` does it.
fn press_b() {
    let asked = EDITOR.lock().unwrap().as_mut().unwrap().cancel_fighter();
    if asked { roster_requests::withdraw_clip(); }
}
fn loading() -> Option<(usize, i32)> { EDITOR.lock().unwrap().as_ref().unwrap().loading }
fn notice() -> String { EDITOR.lock().unwrap().as_ref().unwrap().notice.clone() }
fn pending(role: usize) -> Option<u32> { character_roster::PENDING.lock().unwrap()[role] }
/// The cases of the toggle fix (tr-primary-2 2026-11-27 00:40:25Z, 01:02:26Z), for one role and the fighter it takes.
fn toggle_cases(role: usize, kind: i32, failed: &mut Vec<String>) {
    let mut check = |name: &str, ok: bool| if !ok { failed.push(format!("role {role}: {name}")); };
    let admitted = || { character_roster::reset(); ask(role, kind); roster_requests::service(); service_wanted_fighter(); };
    // The worker can post between the game frame's mailbox drain and scene teardown.
    character_roster::reset();
    ask(role, kind);
    roster_requests::note_drop(role == 0, None);
    roster_requests::service();
    service_wanted_fighter();
    check("scene loss before admission must not dispatch the old request", pending(role).is_none());
    check("old scene's asked ticket terminates", loading().is_none() && notice() == FIGHTER_REFUSED);
    // A full production mailbox discards its oldest request, not necessarily a legacy request.
    character_roster::reset();
    ask(role, kind);
    for _ in 0..8 { roster_requests::request_player_kind(99); }
    roster_requests::service();
    service_wanted_fighter();
    check("evicted clip request ends with the roster's refusal", loading().is_none() && notice() == FIGHTER_REFUSED);
    character_roster::reset();
    for _ in 0..8 { roster_requests::request_player_kind(99); }
    ask(role, kind);
    roster_requests::service();
    service_wanted_fighter();
    check("an older evicted message is not this ticket", pending(role) == Some(kind as u32) && loading().is_some());
    // The roster keeps the request for as long as it likes: the editor keeps loading and says nothing.
    admitted();
    for _ in 0..2_000 { roster_requests::service(); service_wanted_fighter(); }
    check("2,000 passes: still loading, no line", loading() == Some((role, kind)) && notice().is_empty());
    // The roster gives the request up and says so.
    admitted();
    // Another fighter's request given up for this role is not this switch's.
    roster_requests::note_drop(role == 0, Some(kind as u32 + 1));
    service_wanted_fighter();
    check("another fighter's drop is not this switch's", loading() == Some((role, kind)) && notice().is_empty());
    character_roster::PENDING.lock().unwrap()[role] = None;
    roster_requests::note_drop(role == 0, Some(kind as u32));
    service_wanted_fighter();
    check("a dropped switch ends the load", loading().is_none());
    check("a dropped switch shows the refusal", notice() == FIGHTER_REFUSED);
    // B before the request reached the roster: it is never sent.
    character_roster::reset();
    ask(role, kind);
    press_b();
    roster_requests::service();
    service_wanted_fighter();
    check("B before the request is sent: the roster never hears of it", pending(role).is_none());
    check("B before the request is sent: no line", loading().is_none() && notice().is_empty());
    // B while the request waits in the roster with no load begun: it is removed.
    admitted();
    press_b();
    check("B keeps the editor loading until the roster answers", loading() == Some((role, kind)));
    roster_requests::service();
    service_wanted_fighter();
    check("B before the roster began: the request is withdrawn", pending(role).is_none());
    check("B before the roster began: no line, no load", loading().is_none() && notice().is_empty());
    // B after the roster began: the editor follows the roster.
    admitted();
    character_roster::BEGUN.lock().unwrap()[role] = true;
    press_b();
    roster_requests::service();
    service_wanted_fighter();
    check("B after the roster began: still loading", loading() == Some((role, kind)));
    check("B after the roster began: the editor knows it is committed", EDITOR.lock().unwrap().as_ref().unwrap().committed);
    check("B after the roster began: the request stays", pending(role) == Some(kind as u32));
    // A scene change or the entry's destruction after that: a marked drop ends it.
    character_roster::PENDING.lock().unwrap()[role] = None;
    roster_requests::note_drop(role == 0, None);
    service_wanted_fighter();
    check("a scene change or a destroyed entry ends a committed load", loading().is_none() && notice() == FIGHTER_REFUSED);
    // A drop of the other role's request is not this ticket's.
    admitted();
    roster_requests::note_drop(role != 0, None);
    service_wanted_fighter();
    check("the other role's drop is not this switch's", loading() == Some((role, kind)) && notice().is_empty());
    character_roster::reset();
}
fn main() {
    for (role, refused, accepted) in [(0, 7, 6), (1, 6, 7)] {
        character_roster::reset();
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
        character_roster::reset();
        ask(role, refused);
        press_b();
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
    let mut failed = Vec::new();
    for (role, kind) in [(0, 6), (1, 7)] { toggle_cases(role, kind, &mut failed); }
    assert!(failed.is_empty(), "toggle cases failed:\n{}", failed.join("\n"));
}
