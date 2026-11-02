//! The menu worker: the one thread that owns the menu model. The input hook (game thread) pushes events
//! into a lock-free ring and wakes it; the worker presses the model's buttons, commits on close and
//! publishes each new frame for the layout tick to draw. The model allocates on input events, so that
//! work lives here and never in a hook (R5: hooks allocate nothing).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread::Thread;

use hos_lock::Mutex;

use crate::model::frame::Frame;
use crate::model::menu::grid::ConsoleMode;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::menu::input::Button;
use crate::model::menu::{Env, Menu};
use crate::model::settings::Settings;
use crate::model::screens::frame_of;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, notices, writer};
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
/// The session ended (leaving Training): drop the model, so the next opening starts a fresh one.
const EV_RESET: u8 = 32;

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

/// The menu session ends: the worker drops its model (uncommitted edits go with it).
pub fn push_reset() {
    push(EV_RESET);
    wake();
}

pub fn push_press(button: Button) {
    push(encode(button));
}

/// A typed view of the menu changed outside the menu (`compat::menu`): the worker writes it back into the settings.
static WRITEBACK: AtomicBool = AtomicBool::new(false);

/// Ask the worker to write the changed typed views back (any thread; no allocation).
pub fn request_writeback() {
    WRITEBACK.store(true, Ordering::Release);
    wake();
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
    let spawned = crate::runtime::threads::spawn(crate::model::thread_stacks::ThreadId::Menu, run);
    if let Err(error) = spawned {
        diagnostic(format_args!("[TrainingRooms] menu worker did not start: {error}"));
    }
}

/// The console's operation mode now (`nn::oe::GetOperationMode`: 0 handheld, 1 console/docked). SPEC-core-03 §2.5:
/// read each time a grid opens; a mode change while a grid is open applies at its next opening.
fn console_mode() -> ConsoleMode {
    // SAFETY: a plain query with no arguments.
    if unsafe { skyline::nn::oe::GetOperationMode() } == 0 {
        ConsoleMode::Handheld
    } else {
        ConsoleMode::Docked
    }
}

/// What the menu needs to know about the install now (SPEC-core-02 `Env`).
fn env() -> Env {
    Env {
        developer_page: published::developer_page(),
        pilot_present: crate::runtime::ui::state::PILOT_PRESENT.load(Ordering::Acquire),
        console: console_mode(),
        shortcuts: chords::shortcuts(),
        replay_capture: writer::replay_capture(),
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
        if WRITEBACK.swap(false, Ordering::AcqRel) {
            crate::runtime::compat::menu::flush_writeback();
        }
        // A settings publish (this thread's commit, or the diag reload) noted a roster re-plan: it runs here, on a thread, not in the
        // publish and never in a game hook (stream/stack).
        crate::runtime::compat::menu::run_roster_change();
        std::thread::park();
    }
}

fn handle(code: u8, menu: &mut Option<Menu>) {
    if code == EV_RESET {
        *menu = None;
        diagnostic(format_args!("[Ui] menu session cleared"));
        return;
    }
    if code == EV_OPEN {
        let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
        match menu {
            Some(m) => m.reopen(settings, env()),
            None => *menu = Some(Menu::new(settings, env())),
        }
        CLOSING.store(false, Ordering::Release);
        crate::runtime::ui::menu_app::note_open();
        publish_frame(menu.as_ref());
        diagnostic(format_args!("[Ui] menu opened"));
        return;
    }
    let (Some(button), Some(m)) = (decode(code), menu.as_mut()) else { return };
    if !OPEN.load(Ordering::Acquire) {
        return;
    }
    m.set_console(console_mode());
    m.press(button);
    if m.is_closed() {
        close(m);
    } else {
        publish_frame(Some(m));
    }
}

/// Package 6's stage list (the lobby's 117 selectable stages), by catalog index.
fn stage_label(i: usize) -> &'static str {
    crate::engine::stages::catalog::STAGES.get(i).map_or("-", |(_, label)| *label)
}

/// The frame is built here, in a buffer only this thread takes; `FRAME` is held for the copy alone, so the UI tick's `try_lock` skips a
/// draw only while a copy runs (stream/sync).
static SCRATCH: Mutex<Frame> = Mutex::new(Frame::new());

fn publish_frame(menu: Option<&Menu>) {
    let Some(menu) = menu else { return };
    let mut scratch = SCRATCH.lock();
    frame_of(menu, &mut scratch, stage_label);
    *FRAME.lock() = *scratch;
    FRAME_SEQ.fetch_add(1, Ordering::Release);
}

/// The write-back of decision D1: the carried engines changed their typed view of the menu (`compat::menu`, a roster pick the
/// cycle made, the preload rows after the INI overlay). The rows and kept keys that differ from the stored settings replace them, and
/// the settings file is rewritten by the writer thread. `defaults` addresses the saved-defaults snapshot instead of the current one.
/// Nothing is published to the other packages: they read none of these rows.
pub fn adopt_typed(defaults: bool, rows: &[(&'static str, Vec<u8>)], kept: &[(&'static str, serde_json::Value)]) {
    let json = {
        let mut guard = SETTINGS.lock();
        let Some(settings) = guard.as_mut() else { return };
        let target = if defaults { &mut settings.defaults } else { &mut settings.menu };
        let mut changed = false;
        for (id, elements) in rows {
            // A row this build has no definition for stays a kept key of the file (the Solo Popo cycle rows).
            if target.get(id).is_some_and(|current| current != elements.as_slice()) && target.set(id, elements).is_ok() {
                changed = true;
            }
        }
        for (key, value) in kept {
            if target.extra().get(*key) != Some(value) && target.set_kept(key, value.clone()).is_ok() {
                changed = true;
            }
        }
        changed.then(|| settings.to_json())
    };
    if let Some(json) = json {
        writer::request_settings(json);
    }
}

/// B on the page: commit (SPEC-core-03 §2.3), then the 15 frames of discarded input and fade run.
fn close(menu: &mut Menu) {
    let outbox = menu.take_outbox();
    let settings = menu.commit().clone();
    published::publish(&settings.menu, outbox.rerolls);
    // The carried engines read the typed view of the committed settings (decision D1).
    crate::runtime::compat::menu::apply_kept(&settings);
    let json = settings.to_json();
    *SETTINGS.lock() = Some(settings);
    // Every file write goes through the writer thread (`features::writer`): this only hands over the requests.
    if outbox.changed {
        writer::request_settings(json);
    }
    // The menu shortcuts (N7): each changed slot is checked and applied at once; the writer rewrites
    // `button_config.ini`. A refused edit changes nothing and says why.
    if let Some(wanted) = outbox.button_config {
        for (slot, chord) in wanted.into_iter().enumerate() {
            if chords::shortcuts()[slot] == chord {
                continue;
            }
            if let Err(refusal) = chords::commit_shortcut(slot, chord) {
                notices::post("Menu shortcuts", refusal.text(), 120, DEFAULT_COLOR);
                diagnostic(format_args!("[Ui] shortcut {} refused: {}", slot + 1, refusal.text()));
            }
        }
    }
    // Q7: the Replay capture toggle is live at once; the writer rewrites `replay_capture.ini`.
    if let Some(on) = outbox.replay_capture {
        writer::request_replay_capture(on);
    }
    CLOSE_COUNT.store(0, Ordering::Release);
    CLOSING.store(true, Ordering::Release);
    OPEN.store(false, Ordering::Release);
    crate::runtime::ui::menu_app::note_open();
    FRAME_SEQ.fetch_add(1, Ordering::Release);
    diagnostic(format_args!("[Ui] menu closed changed={} rerolls={:#x}", outbox.changed, outbox.rerolls));
}

