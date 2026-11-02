//! The menu worker: the one thread that owns the menu model. The input hook (game thread) pushes events
//! into a lock-free ring and wakes it; the worker presses the model's buttons, commits on close and
//! publishes each new frame for the layout tick to draw. The model allocates on input events, so that
//! work lives here and never in a hook (R5: hooks allocate nothing).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread::Thread;

use hos_lock::Mutex;

use crate::model::menu::grid::ConsoleMode;
use crate::model::notify::DEFAULT_COLOR;
use crate::editors::clip::clip_frame;
use crate::model::flow::{self, Phase, Seen, Step};
use crate::model::frame::Frame;
use crate::model::menu::input::Button;
use crate::model::menu::table::Editor;
use crate::model::menu::{Env, Menu, Screen};
use crate::model::settings::Settings;
use crate::model::screens::{frame_of, panel_frame};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, input_delay, notices, writer};
use crate::runtime::published;
use crate::runtime::macros::{named_macros, replay_capture};
use crate::runtime::ui::{external, requests};
use crate::runtime::ui::state::{CLOSE_COUNT, CLOSING, EXT_SHOWN, FRAME, FRAME_SEQ, OPEN};

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

/// What the worker owns: the menu model and which screen has the buttons (`model::flow`).
#[derive(Default)]
struct Session {
    menu: Option<Menu>,
    phase: Phase,
    /// The panel row the menu is on while `phase` is `Panel`.
    panel: Option<Editor>,
    /// Why the last panel refused to open, shown on the row's help line until the next press.
    refusal: Option<&'static str>,
}

impl Session {
    /// The worker has something to poll (the layer changes its panels on the game thread, the keyboard returns there).
    fn watching(&self) -> bool {
        self.phase != Phase::Menu
    }
}

/// How often the worker looks at a panel the layer may change on its own (a keyboard that returned, a clip's preview).
const POLL: std::time::Duration = std::time::Duration::from_millis(33);

fn run() {
    let _ = WORKER.set(std::thread::current());
    let mut session = Session::default();
    loop {
        // Drain first: an event pushed before this thread registered has no unpark waiting for it.
        while let Some(code) = pop() {
            handle(code, &mut session);
        }
        if WRITEBACK.swap(false, Ordering::AcqRel) {
            crate::runtime::compat::menu::flush_writeback();
        }
        // The named-macro layer's model is this thread's (design C): its worker tick runs before the panels are looked at.
        crate::runtime::macros::port::worker_tick();
        sync_panels(&mut session);
        if session.watching() {
            std::thread::park_timeout(POLL);
        } else {
            std::thread::park();
        }
    }
}

fn handle(code: u8, s: &mut Session) {
    if code == EV_RESET {
        s.menu = None;
        s.panel = None;
        s.refusal = None;
        if s.phase == Phase::Clip {
            EXT_SHOWN.store(false, Ordering::Release);
        }
        s.phase = Phase::Menu;
        diagnostic(format_args!("[Ui] menu session cleared"));
        return;
    }
    if code == EV_OPEN {
        let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
        match &mut s.menu {
            Some(m) => m.reopen(settings, env()),
            None => s.menu = Some(Menu::new(settings, env())),
        }
        s.panel = None;
        s.refusal = None;
        if s.phase == Phase::Panel {
            s.phase = Phase::Menu;
        }
        CLOSING.store(false, Ordering::Release);
        crate::runtime::ui::menu_app::QUICK_MENU_APP.lock().set_open(true);
        publish_frame(s);
        diagnostic(format_args!("[Ui] menu opened"));
        return;
    }
    let Some(button) = decode(code) else { return };
    if s.menu.is_none() || !OPEN.load(Ordering::Acquire) || s.phase == Phase::Clip {
        return;
    }
    s.refusal = None;
    let panel = s.phase == Phase::Panel;
    let closed = {
        let Some(m) = s.menu.as_mut() else { return };
        m.set_console(console_mode());
        if panel {
            // The layer's panel owns the buttons; the menu stays on its row until the panel closes.
            external::press_panel(button);
        } else {
            m.press(button);
            if let Some(item) = m.take_give_item() {
                requests::give_item(item);
            }
            if let Some((tile, button)) = m.take_stage_press() {
                stage_press(m, tile, button, &mut s.refusal);
            }
        }
        m.is_closed()
    };
    if closed {
        if let Some(m) = s.menu.as_mut() {
            close(m);
        }
    } else {
        sync_panels(s);
    }
}

/// Follow the layer's panels (`model::flow`): open the one the menu just entered, hand the menu over to the clip editor, step back out
/// of a panel that closed, and publish the frame when it changed.
fn sync_panels(s: &mut Session) {
    // A keyboard name the frame wrapper confirmed: its library write is file I/O, done here.
    if s.menu.is_none() {
        return;
    }
    let open = OPEN.load(Ordering::Acquire);
    if open && s.phase == Phase::Menu {
        let entered = s.menu.as_ref().and_then(|m| match m.screen() {
            Some(Screen::External(editor)) if external::is_layer_panel(*editor) => Some(*editor),
            _ => None,
        });
        if let Some(editor) = entered {
            let slot = s.menu.as_ref().and_then(record_slot);
            if external::open_panel(editor, slot) {
                s.phase = Phase::Panel;
                s.panel = Some(editor);
            } else {
                s.refusal = Some(external::refusal_of(editor, slot));
                if let Some(m) = s.menu.as_mut() {
                    m.press(Button::B);
                }
            }
        }
    }
    let seen = Seen { menu_open: open, panel_visible: external::panel_up(), clip_visible: external::clip_up() };
    let (phase, step) = flow::next(s.phase, seen);
    s.phase = phase;
    match step {
        Step::Nothing => {}
        Step::CloseLayer => {
            // The menu lost its renderer or the session ended: the layer's panel goes with it.
            external::close_panel();
            s.panel = None;
        }
        Step::PopRow => {
            s.panel = None;
            if let Some(m) = s.menu.as_mut() {
                m.press(Button::B);
            }
        }
        Step::HandOverToClip => {
            // The capture browser opened a clip: the editor reads Player 1 itself, so the menu closes and the editor draws alone.
            s.panel = None;
            if let Some(m) = s.menu.as_mut() {
                m.press(Button::B);
                close(m);
            }
        }
        Step::HideClip => {
            EXT_SHOWN.store(false, Ordering::Release);
            FRAME_SEQ.fetch_add(1, Ordering::Release);
        }
    }
    if s.phase == Phase::Clip {
        EXT_SHOWN.store(true, Ordering::Release);
    }
    if open || s.phase == Phase::Clip {
        publish_frame(s);
    }
}
/// A, X or Y on the Lobby Stages grid: the stage editor applies it to the menu's kept values; a Direct pick is requested from the
/// waiting room's practice stage (state only: the request is made by the stage hooks on the game's side).
fn stage_press(m: &mut Menu, tile: u16, button: Button, note: &mut Option<&'static str>) {
    use crate::editors::stages::{Effect, StageEditor, ORDER_KEY, PREFERENCES_KEY};
    let mut editor = StageEditor::read(m.kept(ORDER_KEY), m.top_extra(PREFERENCES_KEY));
    let effect = editor.press(usize::from(tile), button);
    if effect == Effect::Nothing {
        return;
    }
    m.set_kept(ORDER_KEY, editor.order_value());
    m.set_top_extra(PREFERENCES_KEY, editor.preferences_value());
    if let Effect::Request(selection) = effect {
        *note = Some(match crate::runtime::lobby::practice_stage_request::request(selection) {
            Ok(_) => "Stage requested: it loads in the waiting room",
            Err(why) => why,
        });
    }
}

/// The page's Record Slot (the row `recording_slot`, one-hot over the five slots).
fn record_slot(menu: &Menu) -> Option<usize> {
    menu.settings().menu.get("recording_slot")?.iter().position(|v| *v != 0)
}

/// Package 6's stage list (the lobby's 117 selectable stages), by catalog index.
fn stage_label(i: usize) -> &'static str {
    crate::engine::stages::catalog::STAGES.get(i).map_or("-", |(_, label)| *label)
}

/// Build the frame of what is on screen (the clip editor, a layer panel, or the menu's own page and sub-screen) and hand it to the tick
/// when it differs from the last one.
fn publish_frame(s: &Session) {
    let Some(menu) = s.menu.as_ref() else { return };
    let mut next = Frame::new();
    if s.phase == Phase::Clip {
        let Some(view) = replay_capture::clip_view() else { return };
        clip_frame(&view, &named_macros::fighter_label(view.fighter_kind), replay_capture::scrub_pace(), &mut next);
    } else if s.phase == Phase::Panel {
        // The layer's page may be between two views (a capture loading, a keyboard open): keep what is on screen.
        let Some(view) = external::panel_view_now() else { return };
        panel_frame(menu, &view.title, &view.footer, &view.labels, view.selected, &mut next);
    } else {
        frame_of(menu, &mut next, stage_label);
        if let Some(Screen::Grid { layout, page, cell }) = menu.screen().copied() {
            if layout.kind == crate::model::menu::grid::GridKind::Stages {
                let editor = crate::editors::stages::StageEditor::read(
                    menu.kept(crate::editors::stages::ORDER_KEY),
                    menu.top_extra(crate::editors::stages::PREFERENCES_KEY),
                );
                editor.decorate(&mut next, layout.tile(page, cell).unwrap_or(0));
            }
        }
        if let Some(reason) = s.refusal {
            next.help[1].clear();
            let _ = core::fmt::Write::write_str(&mut next.help[1], reason);
        }
    }
    // The last frame handed over (only this thread takes it): an unchanged frame is not handed over again.
    let mut last = LAST_FRAME.lock();
    if *last != next {
        *last = next;
        FRAME.write(next);
        FRAME_SEQ.fetch_add(1, Ordering::Release);
    }
}

/// The frame last handed to the tick (the worker's own copy; the tick reads `FRAME`).
static LAST_FRAME: Mutex<Frame> = Mutex::new(Frame::new());

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
    crate::runtime::ui::menu_app::QUICK_MENU_APP.lock().set_open(false);
    FRAME_SEQ.fetch_add(1, Ordering::Release);
    diagnostic(format_args!("[Ui] menu closed changed={} rerolls={:#x}", outbox.changed, outbox.rerolls));
    let _ = input_delay::clear;
}

