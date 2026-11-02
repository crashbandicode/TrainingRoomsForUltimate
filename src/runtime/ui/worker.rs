//! The menu worker: the one thread that owns the menu model. The input hook (game thread) pushes events
//! into a lock-free ring and wakes it; the worker presses the model's buttons, commits on close and
//! publishes each new frame for the layout tick to draw. The model allocates on input events, so that
//! work lives here and never in a hook (R5: hooks allocate nothing).

use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread::Thread;

use parking_lot::Mutex;

use crate::model::menu::input::Button;
use crate::model::menu::{Env, Menu};
use crate::model::settings::Settings;
use crate::model::storage::{self, FixedPath};
use crate::model::screens::frame_of;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::input_delay;
use crate::runtime::published;
use crate::runtime::ui::state::{CLOSE_COUNT, CLOSING, FRAME, FRAME_SEQ, OPEN};

const RING: usize = 64;
static SLOTS: [AtomicU8; RING] = [const { AtomicU8::new(0) }; RING];
static HEAD: AtomicUsize = AtomicUsize::new(0);
static TAIL: AtomicUsize = AtomicUsize::new(0);
static WORKER: OnceLock<Thread> = OnceLock::new();
/// Events the ring had no room for (it should never be non-zero: a frame has a dozen presses at most).
pub static DROPPED: AtomicU32 = AtomicU32::new(0);

const EV_OPEN: u8 = 1;
const EV_PRESS: u8 = 2;

fn encode(button: Button) -> u8 {
    EV_PRESS + button as u8
}

fn decode(code: u8) -> Option<Button> {
    const ALL: [Button; 12] = [
        Button::Up,
        Button::Down,
        Button::Left,
        Button::Right,
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::L,
        Button::R,
        Button::ZL,
        Button::ZR,
    ];
    ALL.get(usize::from(code.checked_sub(EV_PRESS)?)).copied()
}

/// Called by the input hook: no allocation, no lock.
fn push(code: u8) {
    let head = HEAD.load(Ordering::Relaxed);
    if head.wrapping_sub(TAIL.load(Ordering::Acquire)) >= RING {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    SLOTS[head % RING].store(code, Ordering::Relaxed);
    HEAD.store(head.wrapping_add(1), Ordering::Release);
}

fn pop() -> Option<u8> {
    let tail = TAIL.load(Ordering::Relaxed);
    if tail == HEAD.load(Ordering::Acquire) {
        return None;
    }
    let code = SLOTS[tail % RING].load(Ordering::Relaxed);
    TAIL.store(tail.wrapping_add(1), Ordering::Release);
    Some(code)
}

pub fn push_open() {
    push(EV_OPEN);
}

pub fn push_press(button: Button) {
    push(encode(button));
}

pub fn wake() {
    if let Some(thread) = WORKER.get() {
        thread.unpark();
    }
}

/// The settings the menu opens on and commits to: what was loaded at boot, then each commit.
static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

pub fn start(settings: Settings) {
    *SETTINGS.lock() = Some(settings);
    let spawned = std::thread::Builder::new().name("tr-menu".into()).stack_size(256 * 1024).spawn(run);
    if let Err(error) = spawned {
        diagnostic(format_args!("[TrainingRooms] menu worker did not start: {error}"));
    }
}

/// What the menu needs to know about the install now (SPEC-core-02 `Env`).
fn env() -> Env {
    Env {
        developer_page: published::developer_page(),
        pilot_present: crate::runtime::ui::state::PILOT_PRESENT.load(Ordering::Acquire),
        ..Env::default()
    }
}

fn run() {
    let _ = WORKER.set(std::thread::current());
    let mut menu: Option<Menu> = None;
    loop {
        // Drain first: an event pushed before this thread registered has no unpark waiting for it.
        while let Some(code) = pop() {
            handle(code, &mut menu);
        }
        std::thread::park();
    }
}

fn handle(code: u8, menu: &mut Option<Menu>) {
    if code == EV_OPEN {
        let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
        match menu {
            Some(m) => m.reopen(settings, env()),
            None => *menu = Some(Menu::new(settings, env())),
        }
        CLOSING.store(false, Ordering::Release);
        publish_frame(menu.as_ref());
        diagnostic(format_args!("[Ui] menu opened"));
        return;
    }
    let (Some(button), Some(m)) = (decode(code), menu.as_mut()) else { return };
    if !OPEN.load(Ordering::Acquire) {
        return;
    }
    m.press(button);
    if m.is_closed() {
        close(m);
    } else {
        publish_frame(Some(m));
    }
}

fn publish_frame(menu: Option<&Menu>) {
    let Some(menu) = menu else { return };
    let mut frame = FRAME.lock();
    frame_of(menu, &mut frame);
    FRAME_SEQ.fetch_add(1, Ordering::Release);
}

/// B on the page: commit (SPEC-core-03 §2.3), then the 15 frames of discarded input and fade run.
fn close(menu: &mut Menu) {
    let outbox = menu.take_outbox();
    let settings = menu.commit().clone();
    published::publish(&settings.menu, outbox.rerolls);
    let json = settings.to_json();
    *SETTINGS.lock() = Some(settings);
    if outbox.changed {
        write_settings(&json);
    }
    // The INI files the menu edits are written by the input package's chord code (N7: chords::commit_shortcut;
    // Q7: the replay capture file); until that lands the requests are only recorded.
    if let Some(chords) = outbox.button_config {
        diagnostic(format_args!("[Ui] outbox button_config shortcut1={} shortcut2={} (write pending the input package)", chords[0], chords[1]));
    }
    if let Some(on) = outbox.replay_capture {
        diagnostic(format_args!("[Ui] outbox replay_capture={on} (write pending the input package)"));
    }
    CLOSE_COUNT.store(0, Ordering::Release);
    CLOSING.store(true, Ordering::Release);
    OPEN.store(false, Ordering::Release);
    FRAME_SEQ.fetch_add(1, Ordering::Release);
    diagnostic(format_args!("[Ui] menu closed changed={} rerolls={:#x}", outbox.changed, outbox.rerolls));
    let _ = input_delay::clear;
}

/// Write `settings.json` through a temporary file and a rename. (The storage-transaction API of the
/// package-1 review fix replaces this when it lands; SPEC-core-04 §3.3.)
fn write_settings(json: &str) {
    let (Some(temp), Some(target)) =
        (FixedPath::<128>::in_root("settings.json.tmp"), FixedPath::<128>::in_root(storage::file::SETTINGS))
    else {
        return;
    };
    if let Err(error) = std::fs::write(temp.as_str(), json) {
        diagnostic(format_args!("[Ui] settings write failed: {error}"));
        return;
    }
    if std::fs::rename(temp.as_str(), target.as_str()).is_err() {
        // The Switch's rename does not replace: remove the old file, then move the new one in.
        let _ = std::fs::remove_file(target.as_str());
        if let Err(error) = std::fs::rename(temp.as_str(), target.as_str()) {
            diagnostic(format_args!("[Ui] settings rename failed: {error}"));
        }
    }
}