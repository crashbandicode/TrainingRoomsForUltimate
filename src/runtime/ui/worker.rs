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
use crate::editors::clip::clip_frame;
use crate::model::flow::{self, Phase, Seen, Step};
use crate::model::menu::input::Button;
use crate::model::menu::table::Editor;
use crate::model::menu::{Env, Menu, Screen};
use crate::model::settings::Settings;
use crate::model::screens::{frame_of, panel_frame};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, notices, writer};
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
    /// The open roster grid's editor (role, previewed forms), kept across presses.
    roster: Option<crate::editors::roster::RosterEditor>,
    mii: Option<crate::editors::mii::MiiList>,
    #[cfg(feature = "mii-saved-alpha")]
    opening: u32,
    #[cfg(feature = "mii-saved-alpha")]
    pending_close: Option<crate::runtime::mii::menu_handoff::CloseIntent>,
}

impl Session {
    /// The worker has something to poll (the layer changes its panels on the game thread, the keyboard returns there).
    fn watching(&self) -> bool {
        #[cfg(feature = "mii-saved-alpha")]
        if self.pending_close.is_some() { return true; }
        self.phase != Phase::Menu || self.menu.as_ref().is_some_and(|m| matches!(m.screen(), Some(Screen::External(Editor::MiiList(_)))))
    }
}

/// How often the worker looks at a panel the layer may change on its own (a keyboard that returned, a clip's preview).
const POLL: std::time::Duration = std::time::Duration::from_millis(33);

fn run() {
    let _ = WORKER.set(std::thread::current());
    let mut session = Session::default();
    loop {
        // Retire departing layer state before accepting a new scene's panel input.
        crate::runtime::macros::port::worker_events();
        // Drain first: an event pushed before this thread registered has no unpark waiting for it.
        while let Some(code) = pop() {
            handle(code, &mut session);
        }
        if WRITEBACK.swap(false, Ordering::AcqRel) {
            crate::runtime::compat::menu::flush_writeback();
        }
        // A settings publish (this thread's commit, or the diag reload) noted a roster re-plan: it runs here, on a thread, not in the
        // publish and never in a game hook (stream/stack).
        crate::runtime::compat::menu::run_roster_change();
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
        s.roster = None;
        s.mii = None;
        #[cfg(feature = "mii-saved-alpha")]
        { s.pending_close = None; }
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
        s.roster = None;
        s.mii = None;
        #[cfg(feature = "mii-saved-alpha")]
        { s.pending_close = None; }
        if s.phase == Phase::Panel {
            s.phase = Phase::Menu;
        }
        CLOSING.store(false, Ordering::Release);
        crate::runtime::ui::menu_app::note_open();
        #[cfg(feature = "mii-saved-alpha")]
        { s.opening = crate::runtime::ui::menu_app::sequence(); }
        publish_frame(s);
        diagnostic(format_args!("[Ui] menu opened"));
        return;
    }
    let Some(button) = decode(code) else { return };
    if s.menu.is_none() || !OPEN.load(Ordering::Acquire) || s.phase == Phase::Clip {
        return;
    }
    #[cfg(feature = "mii-saved-alpha")]
    if s.pending_close.is_some() { return; }
    s.refusal = None;
    #[cfg(feature = "mii-editor-alpha")]
    let mut editor_request = None;
    let panel = s.phase == Phase::Panel;
    let closed = {
        let Some(m) = s.menu.as_mut() else { return };
        m.set_console(console_mode());
        if panel {
            // The layer's panel owns the buttons; the menu stays on its row until the panel closes.
            external::press_panel(button);
        } else if matches!(m.screen(), Some(Screen::External(Editor::MiiList(_)))) && button != Button::B {
            if let Some(list) = s.mii.as_mut() {
                #[cfg(feature = "mii-editor-alpha")]
                if button == Button::A {
                    editor_request = list.editor_request();
                }
                #[cfg(feature = "mii-editor-alpha")]
                if editor_request.is_some() {
                    // Add New leaves the list and closes the menu; its native
                    // request is posted with the captured epoch before OPEN falls.
                    m.press(Button::B);
                    m.press(Button::B);
                } else {
                    list.press(button, m);
                }
                #[cfg(not(feature = "mii-editor-alpha"))]
                list.press(button, m);
            }
        } else {
            m.press(button);
            if let Some(item) = m.take_give_item() {
                requests::give_item(item);
            }
            if let Some((tile, button)) = m.take_stage_press() {
                stage_press(m, tile, button, &mut s.refusal);
            }
            if let Some((tile, button)) = m.take_roster_press() {
                roster_press(m, &mut s.roster, tile, button);
            }
        }
        m.is_closed()
    };
    if closed {
        begin_close(s,
            #[cfg(feature = "mii-editor-alpha")]
            editor_request,
        );
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
    #[cfg(feature = "mii-saved-alpha")]
    if s.pending_close.is_some() {
        retry_close(s);
        if s.pending_close.is_some() { return; }
    }
    let open = OPEN.load(Ordering::Acquire);
    if open {
        sync_mii(s);
    }
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
            }
            begin_close(s,
                #[cfg(feature = "mii-editor-alpha")]
                None,
            );
            #[cfg(feature = "mii-saved-alpha")]
            if s.pending_close.is_some() { return; }
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

/// Take the owned catalog published by package 6; never inspect a native owner from the worker.
// Catalog construction scratch is dead before the subsequent capture-browser I/O.
#[inline(never)]
fn sync_mii(s: &mut Session) {
    let Some(menu) = s.menu.as_ref() else { return };
    let class = match menu.screen() {
        Some(Screen::External(Editor::MiiList(class))) => Some(*class),
        _ => None,
    };
    #[cfg(feature = "mii-saved-alpha")]
    if s.mii.is_none() {
        if let Some(publication) = crate::runtime::ui::menu_app::take_catalog() {
            // Provider rejects stale epoch/opening sequence without the native App guard.
            // Deep catalog copies belong only to this worker.
            s.mii = crate::editors::mii::MiiList::new(
                (*publication.catalog).clone(), publication.context, publication.epoch, menu,
            );
            #[cfg(feature = "mii-ordinary-cpu-alpha")]
            if let Some(list) = s.mii.as_mut() { list.qualify_cpu(publication.ordinary_cpu); }
        }
    }
    if let Some(list) = s.mii.as_mut() {
        if let Some(class) = class { list.open(class); } else { list.leave(); }
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

/// A press on the portrait roster grid: package 6's roster rules on the active kept key (`editors::roster`).
fn roster_press(m: &mut Menu, editor: &mut Option<crate::editors::roster::RosterEditor>, tile: u16, button: Button) {
    use crate::editors::roster::RosterEditor;
    let Some(Screen::Grid { layout, .. }) = m.screen().copied() else { return };
    let crate::model::menu::grid::GridKind::Roster(use_) = layout.kind else { return };
    if editor.is_none_or(|e| e.use_() != use_) {
        *editor = Some(RosterEditor::new(use_));
    }
    let Some(e) = editor.as_mut() else { return };
    e.apply(m, usize::from(tile), button);
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
    let mut next = NEXT_FRAME.lock();
    next.reset();
    if s.phase == Phase::Clip {
        let Some(view) = replay_capture::clip_view() else { return };
        clip_frame(&view, &named_macros::fighter_label(view.fighter_kind), replay_capture::scrub_pace(), &mut next);
    } else if s.phase == Phase::Panel {
        // The layer's page may be between two views (a capture loading, a keyboard open): keep what is on screen.
        let Some(view) = external::panel_view_now() else { return };
        panel_frame(menu, &view.title, &view.footer, &view.labels, view.selected, &mut next);
    } else if matches!(menu.screen(), Some(Screen::External(Editor::MiiList(_)))) {
        if let Some(list) = s.mii.as_ref() {
            list.frame(menu, &mut next);
        } else {
            frame_of(menu, &mut next, stage_label);
            next.help = [crate::model::frame::Text::new(); 2];
            let _ = core::fmt::Write::write_str(&mut next.help[0], "Reading saved and replay Miis…");
        }
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
            if let crate::model::menu::grid::GridKind::Roster(use_) = layout.kind {
                let editor = s.roster.filter(|e| e.use_() == use_).unwrap_or_else(|| crate::editors::roster::RosterEditor::new(use_));
                let values = editor.values(menu.kept(editor.key()));
                editor.decorate(&mut next, &values, editor.solo_enabled(menu));
            }
        }
        if let Some(reason) = s.refusal {
            next.help[1].clear();
            let _ = core::fmt::Write::write_str(&mut next.help[1], reason);
        }
    }
    // The last frame handed over (only this thread takes it): an unchanged frame is not handed over again.
    let mut last = LAST_FRAME.lock();
    if *last != *next {
        *last = *next;
        FRAME.write(*next);
        FRAME_SEQ.fetch_add(1, Ordering::Release);
    }
}

/// The frame last handed to the tick (the worker's own copy; the tick reads `FRAME`).
static LAST_FRAME: Mutex<Frame> = Mutex::new(Frame::new());

/// Scratch owned only by the menu worker. Keeping it here removes a full Frame
/// temporary from every nested screen builder without allocating in a hook.
static NEXT_FRAME: Mutex<Frame> = Mutex::new(Frame::new());

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


/// Construct once on the worker. No settings/outbox side effects occur until
/// the native handoff owns the intent; a full queue keeps this exact value.
fn begin_close(
    s: &mut Session,
    #[cfg(feature = "mii-editor-alpha")]
    editor: Option<crate::engine::mii_root::mii_menu::EditorRequest>,
) {
    #[cfg(feature = "mii-saved-alpha")]
    if let Some(list) = s.mii.as_ref() {
        s.pending_close = Some(crate::runtime::mii::menu_handoff::CloseIntent {
            opening: s.opening,
            epoch: list.epoch(),
            selection: list.close(),
            #[cfg(feature = "mii-editor-alpha")]
            editor,
        });
        retry_close(s);
        return;
    }
    if let Some(menu) = s.menu.as_mut() { close(menu); }
}

#[cfg(feature = "mii-saved-alpha")]
fn retry_close(s: &mut Session) {
    let Some(intent) = s.pending_close.take() else { return };
    // Renderer loss/reset/reopen invalidates the captured opening. Never let a
    // retained old close commit settings or close a newly opened menu.
    if !OPEN.load(Ordering::Acquire)
        || crate::runtime::ui::menu_app::sequence() != s.opening
    { return; }
    if intent.epoch != crate::runtime::roster::character_roster::mii_saved_alpha::menu_epoch() {
        // This opening's native context retired without a queued UI reset.
        // Abort its closed model without committing; the next open rebuilds it.
        s.mii = None;
        crate::runtime::ui::state::release_menu();
        return;
    }
    match crate::runtime::mii::menu_handoff::post_close(intent) {
        Ok(()) => {
            if let Some(menu) = s.menu.as_mut() { close(menu); }
        }
        Err(intent) => s.pending_close = Some(intent),
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

