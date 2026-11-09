//! The menu worker: the one thread that owns the menu model. The input hook (game thread) pushes events
//! into a lock-free ring and wakes it; the worker presses the model's buttons, commits on close and
//! publishes each new frame for the layout tick to draw. The model allocates on input events, so that
//! work lives here and never in a hook (R5: hooks allocate nothing).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
use crate::model::ui_events::{Event, Packet, Ring};
use crate::model::screens::{fighter_question_frame, frame_of, panel_frame};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{chords, notices, writer};
use crate::runtime::published;
use crate::runtime::macros::{named_macros, replay_capture};
use crate::runtime::ui::{combos, external, requests};
use crate::model::input_delay::Owners;
use crate::runtime::ui::state::{CLOSE_COUNT, CLOSING, EXT_SHOWN, FRAME, FRAME_SEQ, JUMPS, LAYOUT_READY, OPEN, POPUP, POPUP_REQ, PROMPT};

static INPUT: Ring<64> = Ring::new();
static WORKER: OnceLock<Thread> = OnceLock::new();
/// Complete polls/openings refused by the bounded ring; never half a physical poll.
pub static DROPPED: AtomicU32 = AtomicU32::new(0);

/// Called by the input hook: no allocation, no lock.
fn push(generation: u32, event: Event) -> bool {
    if !INPUT.push(generation, event) {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return false;
    }
    true
}

pub fn input_generation() -> u32 { INPUT.generation() }

pub fn push_open(generation: u32) -> bool {
    push(generation, Event::Open)
}

/// The menu session ends: the worker drops its model (uncommitted edits go with it).
pub fn push_reset() {
    INPUT.reset();
    wake();
}

pub fn push_poll(generation: u32, packet: Packet) -> bool {
    push(generation, Event::Poll(packet))
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

/// Keep the boot settings for the writer even when the menu thread does not start.
pub fn remember(settings: Settings) {
    *SETTINGS.lock() = Some(settings);
}

/// The text the writer thread stores. When `mark_intro` is set, the intro flag is recorded first.
pub fn settings_json_for_write(mark_intro: bool) -> Option<String> {
    let mut guard = SETTINGS.lock();
    let settings = guard.as_mut()?;
    if mark_intro {
        settings.mark_intro_seen();
    }
    Some(settings.to_json())
}

pub fn start(settings: Settings) {
    remember(settings);
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
        combo_target: crate::runtime::features::combo::labels().0,
        combo_draft: crate::runtime::features::combo::labels().1,
        ..Env::default()
    }
}

/// A Combos row's result (SPEC-input-07 §6): a notice, and the names the open menu shows.
fn combo_note(s: &mut Session, note: &str) {
    // The action note replaces the skipped-file notice (`model::combo_record::apply_combo_action`).
    notices::clear(crate::model::combo_record::ComboNotices::SKIPPED_ID);
    notices::upsert(crate::model::combo_record::ComboNotices::ACTION_ID, "Combos", note, 180, DEFAULT_COLOR);
    let (target, draft) = crate::runtime::features::combo::labels();
    if let Some(m) = s.menu.as_mut() {
        m.set_combo_labels(target, draft);
    }
}

/// What the worker owns: the menu model and which screen has the buttons (`model::flow`).
#[derive(Default)]
struct Session {
    input_generation: u32,
    menu: Option<Menu>,
    /// Where the menu was when its model was last dropped (every close resets the input generation, which drops the
    /// model): the next opening comes back here. Kept for the rest of the boot, across scenes and Training resets.
    keeper: crate::model::menu::PlaceKeeper,
    /// The open menu already has this catalog generation. A close drops the menu,
    /// so the watch reloads on the next open even when no file was written.
    catalog_watch: crate::model::recording_menu::CatalogWatch,
    setups: super::setups::Ui,
    setup_note: Option<String>,
    combos: combos::Ui,
    stage_context: Option<crate::runtime::lobby::practice_stage_request::MenuContext>,
    /// The stage picked to go to: requested when the menu has closed (`close_session`).
    stage_close: Option<crate::engine::stages::stage_forms::Selection>,
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
        if self.setups.watching() { return true; }
        #[cfg(feature = "mii-saved-alpha")]
        if self.pending_close.is_some() { return true; }
        self.phase != Phase::Menu || self.menu.as_ref().is_some_and(|m| matches!(m.screen(), Some(Screen::External(Editor::MiiList(_)))))
    }
}

/// How often the worker looks at a panel the layer may change on its own (a keyboard that returned, a clip's preview).
const POLL: std::time::Duration = std::time::Duration::from_millis(33);
#[cfg(feature = "diag")]
static LAST_DIAG: Mutex<String> = Mutex::new(String::new());

fn run() {
    let _ = WORKER.set(std::thread::current());
    let mut session = Session::default();
    let mut developer_reload = crate::runtime::developer_reload::Worker::default();
    loop {
        // Retire departing layer state before accepting a new scene's panel input.
        crate::runtime::macros::port::worker_events();
        // Drain first: an event pushed before this thread registered has no unpark waiting for it.
        sync_input_generation(&mut session);
        while let Some((generation, event)) = INPUT.pop() {
            sync_input_generation(&mut session);
            if generation == session.input_generation { handle(event, &mut session); }
        }
        if WRITEBACK.swap(false, Ordering::AcqRel) {
            crate::runtime::compat::menu::flush_writeback();
        }
        if POPUP_REQ.swap(false, Ordering::AcqRel) {
            toggle_popup(&mut session);
        }
        // A settings publish (this thread's commit, or the diag reload) noted a roster re-plan: it runs here, on a thread, not in the
        // publish and never in a game hook (stream/stack).
        crate::runtime::compat::menu::run_roster_change();
        // The named-macro layer's model is this thread's (design C): its worker tick runs before the panels are looked at.
        crate::runtime::macros::port::worker_tick();
        sync_sequence_drill();
        if refresh_recording(&mut session) {
            publish_frame(&session);
        }
        sync_panels(&mut session);
        let developer_pending = developer_reload.step();
        #[cfg(feature = "diag")]
        crate::runtime::stall_watch::worker_step(0);
        if session.watching() || developer_pending {
            std::thread::park_timeout(POLL);
        } else {
            std::thread::park();
        }
    }
}

fn sync_input_generation(s: &mut Session) {
    let generation = INPUT.generation();
    if generation != s.input_generation {
        s.input_generation = generation;
        s.keeper.drop_model(&mut s.menu);
        s.catalog_watch = s.catalog_watch.on_menu_dropped();
        s.stage_context = None;
        s.stage_close = None;
        // Practice cleanup can suspend a same-lobby draft. The provider and
        // external-name owner distinguish actual retirement on the worker.
        s.setup_note = None;
        s.panel = None;
        s.refusal = None;
        s.roster = None;
        s.mii = None;
        #[cfg(feature = "mii-saved-alpha")]
        { s.pending_close = None; }
        if s.phase == Phase::Prompt {
            // The next `sync_panels` draws a question that is still up again, on a new model.
            hide_prompt();
        }
        POPUP.store(false, Ordering::Release);
        // The clip editor outlives the menu (the hand-over's own close lands here) and keeps drawing; `sync_panels`
        // hides it when the layer closes it.
        s.phase = flow::after_session_reset(s.phase);
        publish_jumps(s);
        diagnostic(format_args!("[Ui] menu session cleared"));
    }
}

fn handle(event: Event, s: &mut Session) {
    if event == Event::Open {
        s.stage_context = crate::runtime::lobby::practice_stage_request::menu_context();
        s.stage_close = None;
        let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
        s.keeper.open(&mut s.menu, settings, env());
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
        publish_jumps(s);
        #[cfg(feature = "diag")]
        if let Some(m) = s.menu.as_ref() {
            diagnostic(format_args!("[Ui] menu opened page={} cursor={} depth={} screen={:?}", m.page().id(), m.cursor(), m.depth(), m.screen()));
        }
        #[cfg(not(feature = "diag"))]
        diagnostic(format_args!("[Ui] menu opened"));
        return;
    }
    let Event::Poll(packet) = event else { return };
    let Some(menu) = s.menu.as_mut() else { return };
    let buttons = packet.presses(menu);
    for button in buttons {
        if INPUT.generation() != s.input_generation { break; }
        press(button, s);
    }
    if let Some(menu) = s.menu.as_ref() {
        POPUP.store(menu.recording().popup_open(), Ordering::Release);
        sync_popup_owner();
    }
    // Release-only packets also advance capture and update its hint/preview.
    if OPEN.load(Ordering::Acquire) {
        sync_panels(s);
    } else if POPUP.load(Ordering::Acquire) {
        publish_frame(s);
    }
    publish_jumps(s);
}

/// RecordingPopup toggles the pop-up without setting `OPEN`. A closed menu stays closed.
fn toggle_popup(s: &mut Session) {
    if s.menu.is_none() {
        let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
        s.keeper.open(&mut s.menu, settings, env());
    }
    refresh_recording(s);
    if let Some(menu) = s.menu.as_mut() {
        menu.on_request(crate::model::chords::Request::RecordingPopup);
        POPUP.store(menu.recording().popup_open(), Ordering::Release);
        post_recording_effect(menu);
    }
    sync_popup_owner();
    publish_frame(s);
    publish_jumps(s);
}

fn sync_popup_owner() {
    let popup = POPUP.load(Ordering::Acquire);
    let open = OPEN.load(Ordering::Acquire);
    if popup && !open {
        crate::runtime::published::INPUT_OWNERS.fetch_or(Owners::MENU.bits(), Ordering::AcqRel);
    } else if !flow::menu_owns_input(open, popup, CLOSING.load(Ordering::Acquire), PROMPT.load(Ordering::Acquire)) {
        crate::runtime::published::INPUT_OWNERS.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
    }
}

/// Tell the input hook whether ZL + Up / Down are jumps on what shows now (`Menu::takes_jumps`): only the menu's own
/// pages and popup lists take them, never a layer panel or the clip editor (review MN-1).
fn publish_jumps(s: &Session) {
    let jumps = POPUP.load(Ordering::Acquire) || (s.phase == Phase::Menu && s.menu.as_ref().is_none_or(Menu::takes_jumps));
    JUMPS.store(jumps, Ordering::Release);
}

fn press(button: Button, s: &mut Session) {
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(1);
    let popup = POPUP.load(Ordering::Acquire);
    let open = OPEN.load(Ordering::Acquire);
    if s.phase == Phase::Prompt && !open {
        // The fighter question owns Player 1 until it is answered (A), cancelled (B) or retired.
        external::press_panel(button);
        sync_panels(s);
        return;
    }
    if s.menu.is_none() || s.phase == Phase::Clip || (!open && !popup) {
        return;
    }
    refresh_recording(s);
    if popup && !open {
        if let Some(menu) = s.menu.as_mut() {
            menu.set_console(console_mode());
            menu.press(button);
            post_recording_effect(menu);
        }
        return;
    }
    #[cfg(feature = "mii-saved-alpha")]
    if s.pending_close.is_some() { return; }
    s.refusal = None;
    s.setup_note = None;
    #[cfg(feature = "mii-editor-alpha")]
    let mut editor_request = None;
    let panel = s.phase == Phase::Panel;
    let closed = {
        let Some(m) = s.menu.as_mut() else { return };
        m.set_console(console_mode());
        if panel {
            // The layer's panel owns the buttons; the menu stays on its row until the panel closes.
            if s.panel.is_some_and(is_setup) {
                s.setups.press(button);
            } else if s.panel.is_some_and(combos::is_combo) {
                s.combos.press(button);
            } else {
                external::press_panel(button);
            }
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
            // Eden reads where each press left the cursor (the section jump, the remembered place).
            #[cfg(feature = "diag")]
            diagnostic(format_args!(
                "[Ui] press {:?} page={} cursor={} depth={} screen={:?} row={:?} recording={}",
                button,
                m.page().id(),
                m.cursor(),
                m.depth(),
                m.screen(),
                m.help_row().map(|r| r.title),
                m.recording().focused_file_title().unwrap_or_default()
            ));
            // Shipped builds write it too: a page reset is the player's explicit, confirmed choice.
            if let Some((page, changed)) = m.take_page_reset() {
                diagnostic(format_args!("[TrainingRooms] page reset page={} rows={} changed={}", page.id(), changed.len(), changed.join(",")));
            }
            if let Some(item) = m.take_give_item() {
                requests::give_item(item);
            }
            if let Some((tile, button)) = m.take_stage_press() {
                s.stage_close = stage_press(m, tile, button);
                // A stage to go to closes the menu; the request is made when it has closed (`close_session`).
                for _ in 0..8 {
                    if s.stage_close.is_none() || m.is_closed() { break; }
                    m.press(Button::B);
                }
            }
            if let Some((tile, button)) = m.take_roster_press() {
                roster_press(m, &mut s.roster, tile, button, mii_tiles(&s.mii));
            }
        }
        m.is_closed()
    };
    // Cancellation/close invalidates every remaining poll (and the rest of this
    // poll), before it can publish an old editor's effect into a later opening.
    if INPUT.generation() != s.input_generation { return; }
    if let Some(menu) = s.menu.as_mut() {
        #[cfg(feature = "diag")]
        crate::runtime::stall_watch::worker_step(2);
        post_recording_effect(menu);
        #[cfg(feature = "diag")]
        crate::runtime::stall_watch::worker_step(3);
        apply_shortcuts(menu);
    }
    if closed {
        begin_close(s,
            #[cfg(feature = "mii-editor-alpha")]
            editor_request,
        );
    } else {
        sync_panels(s);
    }
}

/// The recording menu names the file work. This thread posts it and the pop-up's role.
fn post_recording_effect(menu: &mut Menu) {
    let selection = menu.recording().selection();
    crate::engine::rr_host::publish_menu_role(selection.record_role);
    let drill = selection.drill.clone();
    let kind = drill.as_ref().and_then(|id| {
        if menu.recording().catalog().combos.iter().any(|file| &file.id == id) {
            Some(crate::engine::library::DrillKind::Combo)
        } else if menu.recording().catalog().sequences.iter().any(|file| &file.id == id) {
            Some(crate::engine::library::DrillKind::Sequence)
        } else {
            None
        }
    });
    crate::engine::rr_host::note_drill(drill.clone());
    apply_grade_target(drill.as_deref(), kind);
    crate::engine::rr_host::publish_selection(std::path::Path::new(crate::model::storage::ROOT), &selection, kind);
    crate::engine::rr_host::post_effect(menu.recording_mut().take_effect());
}

/// What the frame hook drills: the kind and id, and for a sequence the plan it was given (to see a change on the card).
struct Applied {
    kind: crate::engine::library::DrillKind,
    id: String,
    plan: Option<crate::engine::rr_host::SequencePlan>,
    /// The catalog generation the plan was read at, and the plan's generation on the frame hook.
    catalog: u64,
    generation: u32,
}

static APPLIED: Mutex<Option<Applied>> = Mutex::new(None);

/// The one drill becomes what the frame hook grades: a combo drill its combo, a sequence drill its plan. Either
/// replaces the other. The same drill is not applied twice, and a file that is not readable yet is retried on the next
/// post.
fn apply_grade_target(drill: Option<&str>, kind: Option<crate::engine::library::DrillKind>) {
    use crate::engine::rr_host::{follow_drill, DrillFollow};
    use crate::runtime::features::combo;

    let root = std::path::Path::new(crate::model::storage::ROOT);
    let mut applied = APPLIED.lock();
    let now = applied.as_ref().map(|a| (a.kind, a.id.as_str()));
    match follow_drill(now, kind, drill) {
        DrillFollow::Same => {}
        DrillFollow::Clear => {
            *applied = None;
            combo::stage_sequence(None);
            combo::stage_target(None);
        }
        DrillFollow::Combo(id) => {
            let Ok(record) = crate::model::combo_record::ComboLibrary::new(root.join("combos")).load(id) else {
                return;
            };
            combo::stage_sequence(None);
            combo::stage_target(Some((record.combo, record.name.as_str())));
            *applied = Some(Applied {
                kind: crate::engine::library::DrillKind::Combo,
                id: id.to_owned(),
                plan: None,
                catalog: 0,
                generation: 0,
            });
        }
        DrillFollow::Sequence(id) => {
            let catalog = crate::engine::rr_host::catalog_generation();
            let Some(plan) = crate::engine::rr_host::sequence_plan(root, id) else {
                return;
            };
            let generation = combo::stage_sequence(Some((&plan, false)));
            *applied = Some(Applied {
                kind: crate::engine::library::DrillKind::Sequence,
                id: id.to_owned(),
                plan: Some(plan),
                catalog,
                generation,
            });
        }
    }
}

/// Every worker pass: a drilled sequence's streak and best go to its file, and a change on the card (a renamed,
/// deleted or reordered combo, Edit combos) reaches the frame hook. A rename or the saved streak keeps the run going.
fn sync_sequence_drill() {
    use crate::runtime::features::combo;

    let root = std::path::Path::new(crate::model::storage::ROOT);
    let mut applied = APPLIED.lock();
    let Some(drill) = applied.as_mut().filter(|a| a.plan.is_some()) else {
        let _ = combo::take_progress();
        return;
    };
    if let Some(progress) = combo::take_progress().filter(|p| p.generation == drill.generation) {
        let saved = crate::engine::rr_host::save_sequence_progress(root, &drill.id, progress.streak, progress.best);
        combo::tr_seq_saved_v1(u32::from(saved), progress.streak, progress.best, 0);
    }
    let catalog = crate::engine::rr_host::catalog_generation();
    if catalog == drill.catalog {
        return;
    }
    drill.catalog = catalog;
    let Some(plan) = crate::engine::rr_host::sequence_plan(root, &drill.id) else {
        return;
    };
    let same = drill.plan.as_ref().is_some_and(|old| old.same_steps(&plan));
    if drill.plan.as_ref() != Some(&plan) {
        drill.generation = combo::stage_sequence(Some((&plan, same)));
        drill.plan = Some(plan);
    }
}

/// Reload the pop-up after the recording worker writes a file, and return a typed name.
fn refresh_recording(s: &mut Session) -> bool {
    if s.menu.is_none() {
        return false;
    }
    let gen = crate::engine::rr_host::catalog_generation();
    let (watch, due) = s.catalog_watch.take_load(gen);
    s.catalog_watch = watch;
    let Some(menu) = s.menu.as_mut() else { return false };
    menu.recording_mut().set_keyboard_blocked(crate::runtime::features::recording_host::keyboard_is_closed());
    let mut changed = false;
    if due {
        if let Ok(loaded) = crate::engine::rr_runtime::load_catalog(std::path::Path::new(crate::model::storage::ROOT)) {
            menu.recording_mut().set_catalog(loaded.catalog);
            menu.recording_mut().adopt(loaded.bound, loaded.active, loaded.loops, loaded.drill);
            changed = true;
        }
    }
    if let Some(name) = crate::runtime::features::recording_host::take_typed_name() {
        menu.recording_mut().submit_name(&name);
        post_recording_effect(menu);
        changed = true;
    }
    emit_recording_diag(menu);
    changed
}

#[cfg(feature = "diag")]
fn emit_recording_diag(menu: &Menu) {
    use crate::model::menu::table::Page;
    use crate::model::recording_menu::Face;
    let rec = menu.recording();
    let popup = rec.popup_open();
    let on_page = menu.page() == Page::Recording;
    if !popup && !on_page {
        // A closed pop-up forgets the last line, so the next open emits it again. The same line
        // while it stays open is still one write.
        LAST_DIAG.lock().clear();
        return;
    }
    let face = if popup { Face::Popup } else { Face::Page };
    let line = rec.diag_line(face, menu.recording_page_group());
    let mut last = LAST_DIAG.lock();
    if *last == line {
        return;
    }
    *last = line.clone();
    diagnostic(format_args!("[PracticeRecording] {line}"));
}

#[cfg(not(feature = "diag"))]
fn emit_recording_diag(_menu: &Menu) {}

/// Editor confirmation, not whole-menu close, owns this immediate INI effect.
fn apply_shortcuts(menu: &mut Menu) {
    if let Some(wanted) = menu.take_button_config() {
        for (slot, chord) in wanted.into_iter().enumerate() {
            if chords::shortcuts()[slot] == chord { continue; }
            if let Err(refusal) = chords::commit_shortcut(slot, chord) {
                notices::post("Menu shortcuts", refusal.text(), 120, DEFAULT_COLOR);
                diagnostic(format_args!("[Ui] shortcut {} refused: {}", slot + 1, refusal.text()));
            }
        }
        menu.shortcuts_applied(chords::shortcuts());
    }
}

/// Follow the layer's panels (`model::flow`): open the one the menu just entered, hand the menu over to the clip editor, step back out
/// of a panel that closed, and publish the frame when it changed.
fn sync_panels(s: &mut Session) {
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(9);
    // Consume synchronous list/name effects before flow sees their closed panel,
    // and retire accepted jobs even after the disposable menu was reset.
    if let Some(note) = s.setups.tick() {
        notices::upsert(0x29, "Setup library", &note, 180, DEFAULT_COLOR);
        s.setup_note = Some(note);
    }
    if let Some(note) = s.combos.tick() {
        combo_note(s, &note);
    }
    // A keyboard name the frame wrapper confirmed: its library write is file I/O, done here.
    // Drawn only where a renderer exists, so Player 1 is never held by a question nobody can see (R2-2).
    let prompt_visible = named_macros::fighter_prompt_up() && LAYOUT_READY.load(Ordering::Acquire);
    // A closed menu has dropped its model; a fighter question opened since then still needs the flow below (ShowPrompt
    // builds the model it is drawn on). The clip editor's own question too: the hand-over closed the menu (#68).
    let prompt_phase = s.phase == Phase::Menu || s.phase == Phase::Clip;
    if s.menu.is_none() && !(prompt_phase && prompt_visible && !OPEN.load(Ordering::Acquire)) {
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
    if open && s.phase == Phase::Menu && s.setups.panel_up() {
        s.phase = Phase::Panel;
        s.panel = s.setups.active_editor();
    }
    if open && s.phase == Phase::Menu {
        let entered = s.menu.as_ref().and_then(|m| match m.screen() {
            Some(Screen::External(editor))
                if external::is_layer_panel(*editor) || is_setup(*editor) || combos::is_combo(*editor) => Some(*editor),
            _ => None,
        });
        if let Some(editor) = entered.filter(|e| combos::is_combo(*e)) {
            // The Combos group (SPEC-input-07 §6): a list or name page opens as a panel; Save runs and the row closes.
            match s.combos.open(editor) {
                Ok(combos::Opened::Panel) => {
                    s.phase = Phase::Panel;
                    s.panel = Some(editor);
                }
                Ok(combos::Opened::Done(note)) | Err(note) => {
                    combo_note(s, &note);
                    if let Some(m) = s.menu.as_mut() {
                        m.press(Button::B);
                    }
                }
            }
        } else if let Some(editor) = entered {
            let slot = s.menu.as_ref().and_then(record_slot);
            let opened = if is_setup(editor) {
                match s.menu.as_ref().map(|m| s.setups.open(editor, m)) {
                    Some(Ok(())) => true,
                    Some(Err(why)) => { s.setup_note = Some(why); false }
                    None => false,
                }
            } else { external::open_panel(editor, slot) };
            if opened {
                s.phase = Phase::Panel;
                s.panel = Some(editor);
            } else {
                if !is_setup(editor) { s.refusal = Some(external::refusal_of(editor, slot)); }
                if let Some(m) = s.menu.as_mut() {
                    m.press(Button::B);
                }
            }
        }
    }
    let seen = Seen {
        menu_open: open,
        panel_visible: external::panel_up() || s.setups.panel_up(),
        clip_visible: external::clip_up(),
        clip_foreground: named_macros::clip_foreground(),
        prompt_visible,
    };
    let (phase, step) = flow::next(s.phase, seen);
    s.phase = phase;
    match step {
        Step::Nothing => {}
        Step::CloseLayer => {
            // The menu lost its renderer or the session ended: the layer's panel goes with it.
            external::close_panel();
            s.setups.leave_panel();
            s.combos.leave_panel();
            s.panel = None;
        }
        Step::PopRow => {
            s.setups.leave_panel();
            s.combos.leave_panel();
            s.panel = None;
            if let Some(m) = s.menu.as_mut() {
                // A resumed external name can overlay the reopened page.
                if matches!(m.screen(), Some(Screen::External(_))) { m.press(Button::B); }
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
        Step::ShowPrompt => {
            // The frame is built from a menu model (its panel frame); a closed menu has none, so open one as the pop-up does.
            if s.menu.is_none() {
                let settings = SETTINGS.lock().clone().unwrap_or_else(Settings::factory);
                s.keeper.open(&mut s.menu, settings, env());
            }
            crate::runtime::published::INPUT_OWNERS.fetch_or(Owners::MENU.bits(), Ordering::AcqRel);
            PROMPT.store(true, Ordering::Release);
            diagnostic(format_args!("[Ui] fighter prompt shown"));
        }
        Step::HidePrompt => {
            hide_prompt();
            diagnostic(format_args!("[Ui] fighter prompt closed"));
        }
    }
    if s.phase == Phase::Clip {
        EXT_SHOWN.store(true, Ordering::Release);
    }
    if open || s.phase == Phase::Clip || s.phase == Phase::Prompt {
        publish_frame(s);
    }
}

/// Stop drawing the closed-menu fighter question and give Player 1 back, unless the menu or the pop-up holds it.
fn hide_prompt() {
    if !PROMPT.swap(false, Ordering::AcqRel) {
        return;
    }
    if !flow::menu_owns_input(OPEN.load(Ordering::Acquire), POPUP.load(Ordering::Acquire), CLOSING.load(Ordering::Acquire), false) {
        crate::runtime::published::INPUT_OWNERS.fetch_and(!Owners::MENU.bits(), Ordering::AcqRel);
    }
    // The A or B that answered is still held: it must not become a jab or a special in the match.
    crate::runtime::features::recorder::swallow_p1(15);
    FRAME_SEQ.fetch_add(1, Ordering::Release);
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
/// A, X or Y on the Lobby Stages or Change Stage grid: the stage editor applies it to the menu's kept values. A stage to go to (A on
/// Change Stage, or A in Direct on Lobby Stages) is returned: the menu closes and `close_session` requests it then.
fn stage_press(m: &mut Menu, tile: u16, button: Button) -> Option<crate::engine::stages::stage_forms::Selection> {
    use crate::editors::stages::{Effect, StageEditor, ORDER_KEY, PREFERENCES_KEY};
    let change = matches!(m.screen(), Some(Screen::Grid { layout, .. })
        if layout.kind == crate::model::menu::grid::GridKind::StageChange);
    let mut editor = StageEditor::read(m.kept(ORDER_KEY), m.top_extra(PREFERENCES_KEY));
    let effect = if change { editor.change(usize::from(tile), button) } else { editor.press(usize::from(tile), button) };
    if effect == Effect::Nothing {
        return None;
    }
    m.set_kept(ORDER_KEY, editor.order_value());
    m.set_top_extra(PREFERENCES_KEY, editor.preferences_value());
    match effect {
        Effect::Request(selection) => Some(selection),
        _ => None,
    }
}

/// Close the menu, then request the stage picked in it, as the fork does (`src/common/menu.rs`, `stage_close`): with the
/// practice stage the menu opened on (`menu_context`), so the native update serves it once the menu is gone. A refusal is
/// a notice; nothing is half-applied (the request is all or nothing, `practice_stage_request::queue`).
fn close_session(s: &mut Session) {
    if let Some(menu) = s.menu.as_mut() { close(menu, &mut s.setups); }
    // A combo's Switch (SPEC-input-07 §4a) goes to the roster loader once the menu has closed.
    s.combos.after_close();
    let Some(selection) = s.stage_close.take() else { return };
    let result = s.stage_context.ok_or("no practice stage when this menu opened")
        .and_then(|context| crate::runtime::lobby::practice_stage_request::request_in(context, selection));
    diagnostic(format_args!("[PracticeStageRequest] menu_close selection={selection:?} result={result:?}"));
    if let Err(why) = result {
        notices::post("Practice Stage", why, 180, DEFAULT_COLOR);
    }
}

/// A press on the portrait roster grid: package 6's roster rules on the active kept key (`editors::roster`).
/// This opening's Mii tiles for the roster grids: what the mounted catalog admits (none before it arrives).
fn mii_tiles(list: &Option<crate::editors::mii::MiiList>) -> crate::editors::roster::MiiTiles {
    let mut tiles = list.as_ref().map(crate::editors::mii::MiiList::tiles).unwrap_or_default();
    // P-CYCLE: the CPU cycle's Mii admission and both slot 0s are the runtime's, so the grid numbers what cycles.
    (tiles.player_cycle, tiles.cpu_cycle, tiles.slot_zero) = crate::runtime::roster::character_roster::cycle_facts();
    tiles
}

fn roster_press(m: &mut Menu, editor: &mut Option<crate::editors::roster::RosterEditor>, tile: u16, button: Button,
    mii: crate::editors::roster::MiiTiles) {
    use crate::editors::roster::RosterEditor;
    let Some(Screen::Grid { layout, .. }) = m.screen().copied() else { return };
    let crate::model::menu::grid::GridKind::Roster(use_) = layout.kind else { return };
    if editor.is_none_or(|e| e.use_() != use_) {
        *editor = Some(RosterEditor::new(use_));
    }
    let Some(e) = editor.as_mut() else { return };
    e.set_mii(mii);
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
    if !flow::frame_needs_menu(s.phase) {
        // The clip editor: the hand-over closed the menu and the close dropped its model, so it draws from its own view.
        let Some(view) = replay_capture::clip_view() else { return };
        let mut next = NEXT_FRAME.lock();
        next.reset();
        clip_frame(&view, &named_macros::fighter_label(view.fighter_kind), replay_capture::scrub_pace(), &mut next);
        hand_over(&next);
        return;
    }
    let Some(menu) = s.menu.as_ref() else { return };
    let mut next = NEXT_FRAME.lock();
    next.reset();
    if s.phase == Phase::Panel || s.phase == Phase::Prompt {
        // The layer's page may be between two views (a capture loading, a keyboard open): keep what is on screen.
        let Some(view) = (if s.panel.is_some_and(is_setup) {
            s.setups.view()
        } else if s.panel.is_some_and(combos::is_combo) {
            s.combos.view()
        } else { external::panel_view_now() }) else { return };
        if named_macros::fighter_prompt_up() {
            fighter_question_frame(menu, &view.title, &view.footer, &view.labels, view.selected, &mut next);
        } else {
            panel_frame(menu, &view.title, &view.footer, &view.labels, view.selected, &mut next);
        }
    } else if matches!(menu.screen(), Some(Screen::External(Editor::MiiList(_)))) {
        if let Some(list) = s.mii.as_ref() {
            list.frame(menu, &mut next);
        } else {
            frame_of(menu, &mut next, stage_label);
            next.help = [crate::model::frame::Text::new(); 2];
            #[cfg(feature = "mii-saved-alpha")]
            let none = crate::runtime::ui::menu_app::no_catalog(s.opening);
            #[cfg(not(feature = "mii-saved-alpha"))]
            let none = true;
            let _ = core::fmt::Write::write_str(&mut next.help[0], if none {
                "Mii lists open in Training and in waiting practice."
            } else {
                "Reading saved and replay Miis…"
            });
        }
    } else {
        frame_of(menu, &mut next, stage_label);
        if let Some(Screen::Grid { layout, page, cell }) = menu.screen().copied() {
            use crate::model::menu::grid::GridKind;
            if matches!(layout.kind, GridKind::Stages | GridKind::StageChange) {
                let editor = crate::editors::stages::StageEditor::read(
                    menu.kept(crate::editors::stages::ORDER_KEY),
                    menu.top_extra(crate::editors::stages::PREFERENCES_KEY),
                );
                let focused = layout.tile(page, cell).unwrap_or(0);
                if layout.kind == GridKind::StageChange {
                    editor.decorate_change(&mut next, focused);
                } else {
                    editor.decorate(&mut next, focused);
                }
            }
            if let crate::model::menu::grid::GridKind::Roster(use_) = layout.kind {
                let mut editor = s.roster.filter(|e| e.use_() == use_).unwrap_or_else(|| crate::editors::roster::RosterEditor::new(use_));
                editor.set_mii(mii_tiles(&s.mii));
                let values = editor.values(menu.kept(editor.key()));
                editor.decorate(&mut next, &values);
                let numbers = editor.decorate_badges(&mut next, menu);
                if editor.use_() == crate::model::menu::grid::RosterUse::Cycles {
                    for (role, n) in numbers.iter().enumerate() {
                        let slot_zero = n.slot_zero.map_or(0, |kind| (kind & 0xff) + 1);
                        tr_cycle_badges_v1(n.mask[0], n.mask[1], n.mask[2], role as u32 | u32::from(n.count) << 8 | slot_zero << 16);
                    }
                }
            }
        }
        if let Some(reason) = s.refusal {
            next.help[1].clear();
            let _ = core::fmt::Write::write_str(&mut next.help[1], reason);
        }
    }
    if let Some(note) = &s.setup_note {
        next.help[1].clear();
        let _ = core::fmt::Write::write_str(&mut next.help[1], note);
    }
    hand_over(&next);
}

/// Hand a built frame to the tick. The last frame handed over (only this thread takes it): an unchanged frame is not
/// handed over again.
fn hand_over(next: &Frame) {
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
    let changed = {
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
        changed
    };
    if changed {
        writer::request_settings();
    }
}

/// A publish made outside the menu (the diag reload, diag builds): the committed settings the menu
/// opens on take the published rows and top-level keys, so a close keeps them (decision MENU-CLOSE).
/// The saved defaults are the player's and stay.
#[cfg(feature = "diag")]
pub fn adopt_published(settings: &Settings) {
    let mut guard = SETTINGS.lock();
    if let Some(current) = guard.as_mut() {
        current.menu = settings.menu.clone();
        current.extra = settings.extra.clone();
    } else {
        *guard = Some(settings.clone());
    }
}


/// The portrait grid drew a cycle's numbers (P-CYCLE; the rc.5 rows read it with an Eden trace point, no diag print):
/// which catalog kinds carry a number, as a mask over `trio::CHOICES` indices (x0 bits 0..64, x1 64..128, x2 128..), and
/// x3 = role (0 Player, 1 CPU) | the highest number << 8 | (slot-0 kind + 1) << 16 (0: none).
#[no_mangle]
#[inline(never)]
pub extern "C" fn tr_cycle_badges_v1(lo: u64, mid: u64, hi: u64, role_count: u32) {
    core::hint::black_box((0x4359_434c_u32, lo, mid, hi, role_count));
}

/// Construct once on the worker. No settings/outbox side effects occur until
/// the native handoff owns the intent; a full queue keeps this exact value.
fn begin_close(
    s: &mut Session,
    #[cfg(feature = "mii-editor-alpha")]
    editor: Option<crate::engine::mii_root::mii_menu::EditorRequest>,
) {
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(4);
    #[cfg(feature = "mii-saved-alpha")]
    if let Some(list) = s.mii.as_ref() {
        // A portrait pick this opening (its kept value differs from the live settings) resolves that role's choice itself.
        let picked = s.menu.as_ref().map_or([false; 2], |menu| {
            let live = SETTINGS.lock();
            let changed = |key: &str| live.as_ref().is_some_and(|live| live.menu.extra().get(key) != menu.kept(key));
            [changed("player_character"), changed("cpu_character")]
        });
        s.pending_close = Some(crate::runtime::mii::menu_handoff::CloseIntent {
            opening: s.opening,
            epoch: list.epoch(),
            #[cfg(feature = "mii-editor-alpha")]
            selection: list.close_for(editor.is_some(), picked),
            #[cfg(not(feature = "mii-editor-alpha"))]
            selection: list.close_for(false, picked),
            #[cfg(feature = "mii-editor-alpha")]
            editor,
        });
        retry_close(s);
        return;
    }
    close_session(s);
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
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(5);
    match crate::runtime::mii::menu_handoff::post_close(intent) {
        Ok(()) => close_session(s),
        Err(intent) => s.pending_close = Some(intent),
    }
}

/// B on the page: commit (SPEC-core-03 §2.3), then the 15 frames of discarded input and fade run.
fn close(menu: &mut Menu, setups: &mut super::setups::Ui) {
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(6);
    let outbox = menu.take_outbox();
    // Decision MENU-CLOSE (`review/menu-proposal` `9a9cc47`): the close commits only the rows the
    // player changed this session; every other row keeps its live value in `SETTINGS` — the D1
    // write-backs and publishes adopted while the menu was open. The player's edit wins.
    let settings = match SETTINGS.lock().clone() {
        Some(live) => menu.commit_onto(&live),
        None => menu.commit().clone(),
    };
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(7);
    published::publish(&settings.menu, outbox.rerolls);
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(10);
    // The carried engines read the typed view of the committed settings (decision D1).
    crate::runtime::compat::menu::apply_kept(&settings);
    *SETTINGS.lock() = Some(settings);
    // Every file write goes through the writer thread (`features::writer`): this only hands over the requests.
    // The writer serializes the live settings, including an intro flag stored on that object.
    if outbox.changed {
        writer::request_settings();
    }
    // Q7: the Replay capture toggle is live at once; the writer rewrites `replay_capture.ini`.
    if let Some(on) = outbox.replay_capture {
        writer::request_replay_capture(on);
    }
    // This path is reached only after the Mii close intent was accepted.
    // A refusal changes no setup and does not trap the player in the menu.
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(11);
    if let Err(why) = setups.close_menu() {
        notices::post("Setup library", why, 180, DEFAULT_COLOR);
    }
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(12);
    CLOSE_COUNT.store(0, Ordering::Release);
    CLOSING.store(true, Ordering::Release);
    INPUT.reset();
    OPEN.store(false, Ordering::Release);
    crate::runtime::ui::menu_app::note_open();
    FRAME_SEQ.fetch_add(1, Ordering::Release);
    diagnostic(format_args!("[Ui] menu closed changed={} rerolls={:#x}", outbox.changed, outbox.rerolls));
    #[cfg(feature = "diag")]
    crate::runtime::stall_watch::worker_step(8);
}


fn is_setup(editor: Editor) -> bool {
    matches!(editor, Editor::SetupSave | Editor::SetupLibrary)
}
