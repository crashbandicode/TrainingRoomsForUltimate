// The producer runs before the worker. Keep only the editor, lifecycle event and other-draft effects in the oracle.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
#[derive(Clone, Copy)]
// EVENT
struct Queue(Mutex<Vec<Lifecycle>>);
impl Queue {
    fn drain(&self, f: impl FnMut(Lifecycle)) { self.0.lock().unwrap().drain(..).for_each(f); }
    fn take_dropped(&self) -> usize { 0 }
}
static LIFECYCLE: Queue = Queue(Mutex::new(Vec::new()));
static SESSION_GENERATION: AtomicU64 = AtomicU64::new(0);
static OPEN: AtomicBool = AtomicBool::new(true);
static TRANSIENT_CLOSED: AtomicBool = AtomicBool::new(false);
struct Ui;
impl Ui {
    fn close_transient(&mut self) { TRANSIENT_CLOSED.store(true,Ordering::Relaxed); }
    fn invalidate_practice_context(&mut self) {}
}
static UI: Mutex<Ui> = Mutex::new(Ui);
use std::sync::MutexGuard;
fn lock_write<T>(m:&Mutex<T>)->MutexGuard<'_,T>{m.lock().unwrap()}
static CONFIRMED_NAME: Mutex<Option<()>>=Mutex::new(None);
static LIST_ACTION: Mutex<Option<()>>=Mutex::new(None);
static LIST_CHOICE: Mutex<Option<()>>=Mutex::new(None);
static LIST_RENAME: Mutex<Option<()>>=Mutex::new(None);
fn post_lifecycle(e:Lifecycle){LIFECYCLE.0.lock().unwrap().push(e);}
fn suspended()->bool{false}
fn retire_name(_:bool){}
fn cancel_character_selection(){}
mod engine {pub mod rr_host {pub fn drop_preview_home(){}}}
mod runtime {pub mod macros {pub mod replay_capture {
    use super::super::super::*;
    pub fn owns_close_input(_:bool,_:bool){}
    pub fn close_editor(){OPEN.store(false,Ordering::Relaxed);}
    pub fn discard_waiting(){close_editor();}
    pub fn suspend_waiting(){}
}}}
// PRODUCER
// DRAIN
fn main() {
    for (own, expected_open) in [(true,true),(false,false)] {
        OPEN.store(true,Ordering::Relaxed);
        TRANSIENT_CLOSED.store(false,Ordering::Relaxed);
        clear_for_test(own);
        worker_lifecycle();
        assert_eq!(OPEN.load(Ordering::Relaxed),expected_open,"own install: {own}; scene departure must still close");
        assert!(TRANSIENT_CLOSED.load(Ordering::Relaxed),"other transient state still resets");
    }
}
