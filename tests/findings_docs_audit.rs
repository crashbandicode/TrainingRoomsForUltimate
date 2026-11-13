//! Code findings of the docs lane's audit (`artifacts/phase23/docs-audit-1113/CODE-FINDINGS.md`, 2026-11-13), one section
//! each.
//!
//! - #23: B closed the Recording pop-up and the same press reached the fighter (Mario's fireball), held or tapped for one
//!   frame. The pop-up now closes through the menu's close window (`flow::popup_closed`, `flow::close_done`), which also
//!   waits for the closing press to be released.
//! - #15: the Loop row's A list wrote the old two-value setting, so the count Play reads did not change, and Reset page to
//!   defaults left the count alone. The list and the reset now use the recording menu's per-role counts.
//! - #25: after a live fighter swap the Setup library refused a setup of exactly the fighters on screen: its fighter check
//!   read each entry's first body, the setup was saved from the running body. The check now reads the running body.

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::menu::input::Button;
use model::menu::{Env, Menu};
use model::settings::Settings;

// ---- #23: the pop-up's B close ----------------------------------------------------------------------------------------

use model::adapter::{bit, Adapter, Style};
use model::chords::Request;
use model::flow;
use model::frame::CLOSE_FRAMES;
use model::input_delay::{DelayQueue, MappedInputs, Owners, Sticks};
use model::menu::chord::Chord;

/// The raw buttons of one poll.
const B: u32 = 1 << bit::B;
const L: u32 = 1 << bit::L;
const LEFT: u32 = 1 << bit::DPAD_LEFT;
/// The game's mapped Special bit: what the closing B becomes when it reaches the fighter (a neutral special).
const SPECIAL: u32 = 1 << 1;
const NO_SHORTCUTS: [Chord; 2] = [Chord::NONE, Chord::NONE];

fn mapped(raw: u32) -> MappedInputs {
    MappedInputs { buttons: if raw & B != 0 { SPECIAL } else { 0 }, ..MappedInputs::NEUTRAL }
}

/// Where the menu worker (its own thread) handles a poll's presses, against that poll's two stages on the game thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Worker {
    /// Between the poll's MenuInput stage (`ui::input::on_p1_poll`) and its InputDelay stage (`input_delay::poll_with`):
    /// the race the docs lane hit with a one-frame B (`docs-rs4c`).
    RacePoint,
    /// After the poll's InputDelay stage, before the layout tick.
    AfterThePoll,
    /// After the layout tick.
    AfterTheTick,
}

const TIMINGS: [Worker; 3] = [Worker::RacePoint, Worker::AfterThePoll, Worker::AfterTheTick];
/// Input Delay 0 and 3: with a delay, a press that slipped into the queue would come out polls later.
const DELAYS: [u8; 2] = [0, 3];

/// The runtime's three actors with the model rules they call, in their order: the P1 poll hook (MenuInput, then
/// InputDelay), the menu worker and the layout tick. Each field stands for the atomic of `runtime::ui::state` (or
/// `published::INPUT_OWNERS`) named beside it.
struct Rig {
    menu: Menu,
    adapter: Adapter,
    queue: DelayQueue,
    delay: u8,
    worker: Worker,
    /// The worker's ring: one event per poll that sent presses or a release.
    events: Vec<Vec<Button>>,
    /// `INPUT_OWNERS`
    owners: Owners,
    /// `OPEN`
    open: bool,
    /// `POPUP`
    popup: bool,
    /// `CLOSING`
    closing: bool,
    /// `CLOSE_COUNT`
    count: u32,
    /// `CLOSE_HELD`
    held: u32,
    /// `CLOSE_FADES`
    fades: bool,
}

impl Rig {
    fn new(worker: Worker, delay: u8) -> Rig {
        Rig {
            menu: Menu::new(Settings::factory(), Env::default()),
            adapter: Adapter::new(),
            queue: DelayQueue::new(),
            delay,
            worker,
            events: Vec::new(),
            owners: Owners::default(),
            open: false,
            popup: false,
            closing: false,
            count: 0,
            held: 0,
            fades: true,
        }
    }

    /// `worker::toggle_popup` after the pop-up's chord.
    fn open_popup(&mut self) {
        self.menu.on_request(Request::RecordingPopup);
        self.publish_popup(self.menu.recording().popup_open());
        assert!(self.popup && self.owners.contains(Owners::MENU), "the pop-up owns Player 1");
    }

    /// The input hook's opening chord and the worker's `Event::Open`.
    fn open_menu(&mut self) {
        self.owners = self.owners.with(Owners::MENU);
        self.open = true;
        self.closing = false;
    }

    /// `worker::start_close_window`.
    fn start_close_window(&mut self, fades: bool) {
        self.held = flow::CLOSE_HELD_START;
        self.count = 0;
        self.fades = fades;
        self.closing = true;
    }

    /// `worker::publish_popup`, then `worker::sync_popup_owner`.
    fn publish_popup(&mut self, now: bool) {
        if flow::popup_closed(self.open, self.popup, now) {
            self.start_close_window(false);
        }
        self.popup = now;
        if self.popup && !self.open {
            self.owners = self.owners.with(Owners::MENU);
        } else if !flow::menu_owns_input(self.open, self.popup, self.closing, false) {
            self.owners = Owners::from_bits(self.owners.bits() & !Owners::MENU.bits());
        }
    }

    /// The worker's `Event::Poll` for every event in its ring: the presses (`worker::press`), the menu's close
    /// (`worker::close`), then the pop-up's publish.
    fn run_worker(&mut self) {
        for presses in std::mem::take(&mut self.events) {
            for button in presses {
                if self.open || self.popup {
                    self.menu.press(button);
                }
                if self.open && self.menu.is_closed() {
                    self.start_close_window(true);
                    self.open = false;
                }
            }
            self.publish_popup(self.menu.recording().popup_open());
        }
    }

    /// One frame: the P1 poll's two stages, the layout tick, and the worker at its point. Returns what the fighter reads.
    fn frame(&mut self, raw: u32) -> MappedInputs {
        // MenuInput (`input::on_p1_poll`): chords are refused while closing (`readiness::may_open`).
        let step = self.adapter.poll(raw, Style::Pro, self.open || self.popup, !self.closing, NO_SHORTCUTS);
        if self.closing {
            self.held &= u32::from(step.held);
        }
        if (self.open || self.popup) && (step.changed || step.presses().next().is_some()) {
            self.events.push(step.presses().collect());
        }
        if self.worker == Worker::RacePoint {
            self.run_worker();
        }
        // InputDelay (`input_delay::poll_with`).
        let record = mapped(raw);
        let (out, _) = self.queue.poll_with(self.delay, self.owners, record, Sticks::of_record(record));
        if self.worker == Worker::AfterThePoll {
            self.run_worker();
        }
        // The layout tick (`tick.rs`).
        if self.closing {
            self.count = self.count.saturating_add(1);
            if flow::close_done(self.count, self.held) {
                self.closing = false;
                self.owners = Owners::from_bits(self.owners.bits() & !Owners::MENU.bits());
            }
        }
        if self.worker == Worker::AfterTheTick {
            self.run_worker();
        }
        out
    }

    /// `chords::blocked()`: every owner but the stop chord's own bit keeps the chord detector off.
    fn chords_blocked(&self) -> bool {
        Owners::from_bits(self.owners.bits() & !Owners::STOP_CHORD.bits()).any()
    }

    /// After the window Player 1 is the game's again: a fresh B reaches the fighter (so the rig does pass presses).
    fn gives_player_1_back(&mut self, case: &str) {
        assert!(!self.closing && !self.owners.any(), "{case}: the window ended and released Player 1");
        let reached = (0..=self.delay).any(|_| self.frame(B).buttons & SPECIAL != 0);
        assert!(reached, "{case}: a B after the window reaches the fighter");
        for _ in 0..=self.delay {
            self.frame(0);
        }
    }
}

/// `docs-rs4c`: a B held for exactly one poll leaked when the worker closed the pop-up between that poll's MenuInput and
/// InputDelay stages. The close window starts before `POPUP` falls, so the owner bit never drops on that poll.
#[test]
fn popup_b_tap_at_the_race_point_never_reaches_the_fighter() {
    for worker in TIMINGS {
        for delay in DELAYS {
            let case = format!("{worker:?}, delay {delay}");
            let mut rig = Rig::new(worker, delay);
            rig.open_popup();
            for _ in 0..3 {
                assert_eq!(rig.frame(0), MappedInputs::NEUTRAL, "{case}");
            }
            assert_eq!(rig.frame(B), MappedInputs::NEUTRAL, "{case}: the poll of the B that closed the pop-up");
            assert!(!rig.menu.recording().popup_open(), "{case}: B closed the pop-up");
            for n in 0..CLOSE_FRAMES * 2 {
                assert_eq!(rig.frame(0), MappedInputs::NEUTRAL, "{case}: frame {n} after the tap");
            }
            rig.gives_player_1_back(&case);
        }
    }
}

/// `docs-rs4`: every B close was followed by `[FrameStrip] p1 … attack=Some(NeutralSpecial)`. A B held through the close
/// and three windows' worth of frames never reaches the fighter; its release ends the window.
#[test]
fn popup_b_held_past_the_close_window_never_reaches_the_fighter() {
    for worker in TIMINGS {
        for delay in DELAYS {
            let case = format!("{worker:?}, delay {delay}");
            let mut rig = Rig::new(worker, delay);
            rig.open_popup();
            for _ in 0..3 {
                rig.frame(0);
            }
            for n in 0..CLOSE_FRAMES * 3 {
                assert_eq!(rig.frame(B).buttons & SPECIAL, 0, "{case}: frame {n} of the held B");
            }
            assert!(!rig.menu.recording().popup_open(), "{case}: B closed the pop-up");
            assert!(rig.closing && rig.owners.contains(Owners::MENU), "{case}: the window waits for B's release");
            assert!(!rig.fades, "{case}: the pop-up vanishes, it does not fade");
            for _ in 0..=delay {
                assert_eq!(rig.frame(0), MappedInputs::NEUTRAL, "{case}: the release");
            }
            rig.gives_player_1_back(&case);
        }
    }
}

/// The menu's own close shares the window: a B held past its 15 frames no longer reaches the fighter either.
#[test]
fn a_menu_b_close_held_past_fifteen_frames_never_reaches_the_fighter() {
    for worker in TIMINGS {
        let case = format!("{worker:?}");
        let mut rig = Rig::new(worker, 0);
        rig.open_menu();
        rig.frame(0);
        for n in 0..CLOSE_FRAMES * 3 {
            assert_eq!(rig.frame(B), MappedInputs::NEUTRAL, "{case}: frame {n} of the held B");
        }
        assert!(rig.menu.is_closed() && !rig.open, "{case}: B closed the menu");
        assert!(rig.fades, "{case}: the menu's close fades");
        assert!(rig.closing, "{case}: the window waits for B's release");
        rig.frame(0);
        rig.gives_player_1_back(&case);
    }
}

/// The window latches the action buttons held at its first poll and waits for each; a direction held through it is
/// movement and is not waited for, and 15 frames stay the minimum.
#[test]
fn the_close_window_waits_for_every_action_button_held_when_it_began() {
    for worker in TIMINGS {
        let case = format!("{worker:?}");
        // L held on the pop-up (its role switch), then B, both still held at the window's first poll: both are latched.
        let mut rig = Rig::new(worker, 0);
        rig.open_popup();
        rig.frame(L);
        rig.frame(L | B);
        rig.frame(L | B);
        assert!(!rig.menu.recording().popup_open(), "{case}: B closed the pop-up");
        for _ in 0..CLOSE_FRAMES * 2 {
            assert_eq!(rig.frame(L).buttons & SPECIAL, 0, "{case}");
        }
        assert!(rig.closing && rig.owners.contains(Owners::MENU), "{case}: B is released, L is not");
        rig.frame(0);
        assert!(!rig.closing && !rig.owners.any(), "{case}: both released after 15 frames");

        // A direction held through the close does not hold the window; a tap still gets the full 15 frames.
        let mut rig = Rig::new(worker, 0);
        rig.open_popup();
        rig.frame(LEFT);
        rig.frame(LEFT | B);
        let mut frames = 0;
        while rig.closing || rig.popup {
            rig.frame(LEFT);
            frames += 1;
            assert!(frames <= CLOSE_FRAMES + 1, "{case}: a held direction kept the window");
        }
        assert!(frames >= CLOSE_FRAMES - 1, "{case}: the window ran {frames} frames, fewer than 15");
        assert!(!rig.owners.any(), "{case}");
    }
}

/// The finding's other note: the pop-up's own chord cannot close it, because every input owner blocks the chord detector
/// (`features/chords.rs` `blocked()`). This fix does not touch that: the detector stays blocked while the pop-up is open
/// and through its close window, as through the menu's, and `blocked()` is unchanged.
#[test]
fn the_popup_close_window_still_blocks_the_chords() {
    let mut rig = Rig::new(Worker::RacePoint, 0);
    rig.open_popup();
    assert!(rig.chords_blocked(), "the open pop-up blocks the chords (its own chord included)");
    rig.frame(B);
    assert!(rig.closing);
    while rig.closing {
        assert!(rig.chords_blocked(), "the close window blocks the chords");
        rig.frame(if rig.count > CLOSE_FRAMES * 2 { 0 } else { B });
    }
    assert!(!rig.chords_blocked());
    let chords = include_str!("../src/runtime/features/chords.rs");
    assert!(
        body(chords, "blocked").contains("Owners::from_bits(published::input_owners().bits() & !Owners::STOP_CHORD.bits()).any()"),
        "chords::blocked() changed"
    );
}

/// The body of `name` in `src`, from its `fn` line to the next line that is a lone `}` at column 0 (`tests/lockfree_wiring.rs`).
fn body<'a>(src: &'a str, name: &str) -> &'a str {
    let at = src.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("fn {name}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

fn ordered(src: &str, steps: &[&str]) {
    let mut from = 0;
    for step in steps {
        let at = src[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{src}"));
        from += at + step.len();
    }
}

/// The rig above is the runtime's order; this pins the runtime to the same rules (the runtime is Switch-only).
#[test]
fn the_runtime_uses_the_close_window_rules() {
    let worker = include_str!("../src/runtime/ui/worker.rs");
    ordered(body(worker, "publish_popup"), &["flow::popup_closed(", "start_close_window(false)", "POPUP.store(now", "sync_popup_owner()"]);
    assert!(
        body(worker, "sync_popup_owner").contains("flow::menu_owns_input(open, popup, CLOSING.load("),
        "the pop-up's owner is released while its close window runs"
    );
    ordered(body(worker, "start_close_window"), &[
        "CLOSE_HELD.store(flow::CLOSE_HELD_START",
        "CLOSE_COUNT.store(0",
        "CLOSE_FADES.store(fades",
        "CLOSING.store(true",
    ]);
    let close = body(worker, "close");
    assert!(close.contains("start_close_window(true)") && !close.contains("CLOSING.store(true"), "the menu's close uses the shared window");
    assert_eq!(worker.matches("POPUP.store(").count(), 2, "POPUP is published outside publish_popup and the session clear");
    assert_eq!(worker.matches("sync_popup_owner();").count(), 1, "the owner is synced without publish_popup's close window");
    ordered(body(worker, "toggle_popup"), &["on_request(", "publish_popup(", "post_recording_effect("]);
    ordered(body(worker, "handle"), &["press(button, s)", "publish_popup(menu.recording().popup_open())"]);

    let input = include_str!("../src/runtime/ui/input.rs");
    ordered(body(input, "on_p1_poll"), &["adapter.poll(", "state::CLOSING.load(", "state::CLOSE_HELD.fetch_and(u32::from(step.held)"]);

    let tick = include_str!("../src/runtime/ui/tick.rs");
    assert!(tick.contains("flow::close_done(count, state::CLOSE_HELD.load("), "the tick ends the window by flow::close_done");
    assert!(tick.contains("count < CLOSE_FRAMES && state::CLOSE_FADES.load("), "the fade runs 15 frames, for the menu only");
    assert!(!tick.contains("if count >= CLOSE_FRAMES {"), "the tick ends the window at 15 frames regardless again");
}

// ---- #15: the Loop row's list and reset --------------------------------------------------------------------------------

use model::frame::{self, Frame, Text};
use model::menu::options::LOOP_COUNTS;
use model::menu::table::{Control, ROWS};
use model::menu::{Entry, Screen, LOOP_ROW};
use model::recording_menu::{LoopMark, ViewRole};
use model::screens::frame_of;

fn row_of(key: &str) -> usize {
    ROWS.iter().position(|row| row.id == key).unwrap_or_else(|| panic!("no row {key}"))
}

fn goto_entry(m: &mut Menu, entry: Entry) {
    for _ in 0..=m.entry_count(m.page()) {
        if m.focus() == Some(entry) {
            return;
        }
        m.press(Button::Down);
    }
    panic!("could not focus {entry:?}");
}

/// A menu on the factory settings with the Loop row focused (`tests/findings_menu_close.rs`'s `goto`).
fn loop_menu() -> Menu {
    let mut m = Menu::new(Settings::factory(), Env::default());
    let row = row_of(LOOP_ROW);
    let page = ROWS[row].page();
    for _ in 0..6 {
        if m.page() == page {
            break;
        }
        m.press(Button::R);
    }
    assert_eq!(m.page(), page);
    let group = usize::from(ROWS[row].group);
    if !m.group_is_open(group) {
        goto_entry(&mut m, Entry::Group(group));
        m.press(Button::A);
    }
    goto_entry(&mut m, Entry::Row(row));
    m
}

/// ZR: the Recording page's CPU view. Its rows differ, so the cursor (an entry index) is put back on the Loop row.
fn cpu_view(m: &mut Menu) {
    m.press(Button::ZR);
    assert_eq!(m.recording().page_role(), ViewRole::Cpu);
    goto_entry(m, Entry::Row(row_of(LOOP_ROW)));
}

fn list_cursor(m: &Menu) -> usize {
    match m.screen() {
        Some(Screen::List { cursor, .. }) => *cursor,
        other => panic!("not the list: {other:?}, focus {:?}, depth {}, page role {:?}", m.focus(), m.depth(), m.recording().page_role()),
    }
}

/// A on the Loop row, the cursor to `index`, A.
fn pick(m: &mut Menu, index: usize) {
    m.press(Button::A);
    for _ in 0..LOOP_COUNTS.len() {
        if list_cursor(m) == index {
            break;
        }
        m.press(Button::Down);
    }
    assert_eq!(list_cursor(m), index);
    m.press(Button::A);
    assert!(m.screen().is_none(), "A sets and closes the list");
}

fn loop_of_selection(m: &Menu, role: ViewRole) -> LoopMark {
    let selection = m.recording().selection();
    match role {
        ViewRole::Player => selection.loop_player,
        ViewRole::Cpu => selection.loop_cpu,
    }
}

fn row_value(m: &Menu) -> String {
    let mut value = Text::new();
    frame::write_value(m, row_of(LOOP_ROW), &mut value);
    value.string_lossy()
}

/// The open list's lines: (label, value).
fn list_lines(m: &Menu) -> Vec<(String, String)> {
    let mut f = Box::new(Frame::new());
    frame_of(m, &mut f, |_| "Stage");
    f.rows.iter().map(|line| (line.label.string_lossy(), line.value.string_lossy())).filter(|(label, _)| !label.is_empty()).collect()
}

/// Every pick from the list is the page role's count: what the row shows and what the worker publishes to the index that
/// Play reads (`selection()` → `rr_host::publish_selection`). The other role keeps its count.
#[test]
fn the_loop_list_sets_the_count_play_reads() {
    for role in [ViewRole::Player, ViewRole::Cpu] {
        let mut m = loop_menu();
        if role == ViewRole::Cpu {
            cpu_view(&mut m);
        }
        assert_eq!(m.recording().page_role(), role);
        for (index, mark) in LoopMark::CYCLE.into_iter().enumerate() {
            pick(&mut m, index);
            assert_eq!(m.recording().loop_of(role), mark, "{role:?}: pick {index}");
            assert_eq!(loop_of_selection(&m, role), mark, "{role:?}: the count Play reads, pick {index}");
            assert_eq!(row_value(&m), format!("<  {}  >", mark.label()), "{role:?}: the row shows the pick");
            assert_eq!(m.recording().loop_of(role.other()), LoopMark::default(), "{role:?}: the other role's count");
        }
    }
}

/// The list opens on the page role's count and marks it "(current)".
#[test]
fn the_loop_list_opens_on_and_marks_the_current_count() {
    let mut m = loop_menu();
    m.recording_mut().adopt(None, [None, None], [LoopMark::Finite(3), LoopMark::Infinite], None);
    for (role, at) in [(ViewRole::Player, 3), (ViewRole::Cpu, 0)] {
        if role == ViewRole::Cpu {
            cpu_view(&mut m);
        }
        assert_eq!(m.recording().page_role(), role);
        m.press(Button::A);
        assert_eq!(list_cursor(&m), at, "{role:?}: the list opens on the count");
        let lines = list_lines(&m);
        let labels: Vec<&str> = lines.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, LOOP_COUNTS, "{role:?}");
        let current: Vec<&str> = lines.iter().filter(|(_, value)| value == "(current)").map(|(label, _)| label.as_str()).collect();
        assert_eq!(current, [LOOP_COUNTS[at]], "{role:?}: (current) marks the count");
        m.press(Button::B);
        assert!(m.screen().is_none());
    }
}

/// Reset page to defaults on the Recording page puts both roles' counts back to 1, the count of a new index.
#[test]
fn reset_page_restores_both_loop_counts() {
    let mut m = loop_menu();
    m.recording_mut().adopt(None, [None, None], [LoopMark::Finite(3), LoopMark::Infinite], None);
    let last = m.entry(m.page(), m.entry_count(m.page()) - 1).expect("the page's last line");
    assert!(matches!(last, Entry::ResetPage(_)), "{last:?}");
    goto_entry(&mut m, last);
    m.press(Button::A);
    m.press(Button::A);
    assert!(m.screen().is_none());
    assert_eq!(m.recording().loop_of(ViewRole::Player), LoopMark::Finite(1));
    assert_eq!(m.recording().loop_of(ViewRole::Cpu), LoopMark::Finite(1));
    assert_eq!(m.recording().selection().loop_player, LoopMark::Finite(1));
    assert_eq!(m.recording().selection().loop_cpu, LoopMark::Finite(1));
    let (_, changed) = m.take_page_reset().expect("the reset ran");
    assert!(changed.contains(&LOOP_ROW), "the reset reports the Loop row: {changed:?}");
}

/// The list's labels and the counts it sets line up one to one (`LOOP_COUNTS[i]` is `LoopMark::CYCLE[i]`).
#[test]
fn the_loop_list_labels_are_the_loop_counts() {
    let Control::Dropdown(labels) = ROWS[row_of(LOOP_ROW)].control else { panic!("the Loop row is a dropdown") };
    assert_eq!(labels, &LOOP_COUNTS);
    assert_eq!(LOOP_COUNTS.len(), LoopMark::CYCLE.len());
    for (label, mark) in LOOP_COUNTS.iter().zip(LoopMark::CYCLE) {
        assert_eq!(*label, mark.label());
    }
    let m = loop_menu();
    assert_eq!(m.option_count(row_of(LOOP_ROW)), LOOP_COUNTS.len());
}

// ---- #25: the Setup library's restore after a live swap ----------------------------------------------------------------

use model::entry_cells;
use model::state::machine::SituationIds;
use model::state::record::SavedState;
use model::state::setup_transaction::{export, Facts, Prepared, Refusal, Target};

const MARIO: i32 = 0x0;
const LUIGI: i32 = 0x9;

/// The CPU's entry after a live swap (CPU Character, a cycle chord, Fighter Mismatch's Switch), as the game lays it out
/// (`model::entry_cells`): the body bank put Mario in window cell 1 and made it the active cell (`+0x5918`, count `+0x4150`);
/// cell 0, the entry's first body, is still the Luigi the entry was built with. Returns the word and byte readers over that
/// memory and the two bodies.
fn swapped_cpu_entry(entry: usize) -> (impl Fn(usize) -> u64, impl Fn(usize) -> u8, usize, usize) {
    let (first, running) = (0x20000usize, 0x30000usize);
    let word = move |address: usize| match address - entry {
        0x4150 => 2,
        offset if offset == entry_cells::BODY_FIELD => first as u64,
        offset if offset == entry_cells::BODY_FIELD + 8 => running as u64,
        offset => panic!("unexpected entry word +{offset:#x}"),
    };
    let byte = move |address: usize| {
        assert_eq!(address - entry, 0x5918, "only the active-cell byte is read");
        1
    };
    (word, byte, first, running)
}

/// A setup saved by the Save chord after the swap is restored with the same fighters on screen. The chord records the body the
/// game runs a frame of (`states::on_fighter_frame`): Mario. The restore's fighter check is fed by `preview::observe`: read from
/// the running body (`entry_active_body`) it matches; read from the entry's first body (`entry_body`, before the fix) it saw
/// Luigi and refused with "Restore this setup with its saved fighters".
#[test]
fn a_setup_saved_after_a_live_swap_restores_with_the_fighters_on_screen() {
    const ENTRY: usize = 0x10000;
    let (word, byte, first, running) = swapped_cpu_entry(ENTRY);
    let kind_of = |body: usize| if body == running { MARIO } else { LUIGI };
    let pair: [SavedState; 2] = std::array::from_fn(|role| SavedState {
        fighter_kind: if role == 0 { MARIO } else { kind_of(running) },
        x: if role == 0 { -20.0 } else { 20.0 },
        saved_stage: Some(17),
        situation_kind: 0,
        ..SavedState::never_saved()
    });
    let prepared = Prepared::new(&export("Setup", &pair).unwrap(), [None::<u64>; 2]).unwrap();
    let target = Target { session: 4, slot: 0, enabled: true };
    let facts = |cpu_body: usize| Facts {
        session: 4,
        owned: true,
        busy: false,
        enabled: true,
        stage: 17,
        kinds: [MARIO, kind_of(cpu_body)],
        miis: [None::<u64>; 2],
        situations: SituationIds { ground: 0, air: 2, cliff: 5 },
    };

    let active = entry_cells::read_active_body(ENTRY, true, &word, &byte);
    assert_eq!(active, running, "the running body is the swapped-in Mario");
    assert_eq!(prepared.validate(target, &facts(active), true), Ok(()), "restored with Mario and Mario on screen");

    let first_body = word(ENTRY + entry_cells::BODY_FIELD) as usize;
    assert_eq!(first_body, first);
    assert_eq!(prepared.validate(target, &facts(first_body), true), Err(Refusal::Fighter), "the first-body read refused it");
    assert_eq!(prepared.validate(target, &facts(first_body), false), Ok(()), "Y binds without the fighter check");
}

/// The runtime half (Switch-only, so a source check like `tests/lockfree_wiring.rs`): the identity the Setup library's restore
/// and the replay preview both check reads each entry's running body.
#[test]
fn the_setup_restore_and_the_replay_preview_read_the_running_body() {
    let preview = include_str!("../src/runtime/features/states/preview.rs");
    let observe = body(preview, "observe");
    assert!(observe.contains("natives::entry_active_body(entries[index])"), "observe reads the running body:\n{observe}");
    assert!(!observe.contains("natives::entry_body("), "observe reads the entry's first body again");
    let setups = include_str!("../src/runtime/features/states/setups.rs");
    assert!(body(setups, "service").contains("preview::observe()"), "the restore's facts come from observe");
    assert!(body(preview, "request").contains("observe()"), "the replay preview's checkpoint comes from observe");
}
