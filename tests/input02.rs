//! SPEC-input-02 §9 host rows: the logger (rows, difference rules, frame counts, ring rotation), stick
//! binning, icons, the fade, and the renderer's change plan (counts per the fork's optimised behaviour).

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use model::input_delay::MappedInputs;
use model::input_display::*;
use model::settings::Snapshot;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn smash(buttons: u32) -> Candidate {
    Candidate { smash: MappedInputs { buttons, ..MappedInputs::NEUTRAL }, ..Candidate::default() }
}

fn smash_stick(x: i8, y: i8) -> Candidate {
    Candidate { smash: MappedInputs { lstick_x: x, lstick_y: y, ..MappedInputs::NEUTRAL }, ..Candidate::default() }
}

const SMASH: Settings = Settings { mode: Mode::Smash, status: false };

/// A log whose counters are running (the first poll only starts them, SPEC-input-02 §5.1).
fn started() -> Log {
    let mut log = Log::new();
    log.record(&SMASH, Candidate::default());
    log
}

/// One in-game frame then one poll.
fn frame(log: &mut Log, s: &Settings, c: Candidate) {
    log.tick_frame();
    log.record(s, c);
}

#[test]
fn menu_defaults() {
    assert_eq!(Settings::from_snapshot(&Snapshot::factory()), Settings { mode: Mode::Smash, status: false });
    let mut snap = Snapshot::factory();
    snap.set("input_display", &[0, 0, 1, 0]).unwrap();
    snap.set("input_display_status", &[1, 0]).unwrap();
    assert_eq!(Settings::from_snapshot(&snap), Settings { mode: Mode::Raw, status: true });
    snap.set("input_display", &[0, 0, 0, 1]).unwrap();
    assert_eq!(Settings::from_snapshot(&snap).mode, Mode::StatusOnly);
    snap.set("input_display", &[1, 0, 0, 0]).unwrap();
    assert_eq!(Settings::from_snapshot(&snap).mode, Mode::None);
}

#[test]
fn bin_stick_values() {
    let d = |x, y| bin_stick(x, y).map(|d| d.0);
    assert_eq!(d(0, 0), None);
    assert_eq!(d(21, 0), Some(0));
    assert_eq!(d(-21, 29), Some(180));
    assert_eq!(d(20, 0), None, "the walk threshold is exclusive");
    assert_eq!(d(0, 31), Some(90));
    assert_eq!(d(19, -31), Some(270));
    assert_eq!(d(0, 30), None, "the vertical dead zone is exclusive");
    assert_eq!(d(0, -30), None);
    assert_eq!(d(30, 40), Some(45));
    assert_eq!(d(-30, 40), Some(135));
    assert_eq!(d(-30, -40), Some(225));
    assert_eq!(d(30, -40), Some(315));
    // Not a direction between: |x| is 20 exactly with a vertical push.
    assert_eq!(d(20, 40), Some(90));
    assert_eq!(d(20, -40), Some(270));
    assert_eq!(raw_axis(1.0), 120);
    assert_eq!(raw_axis(-0.5), -60);
    assert_eq!(raw_axis(0.999), 119, "truncated");
}

#[test]
fn icon_order_and_colours() {
    // Smash: sticks first (left white, right yellow), then buttons in ascending bit order.
    let c = Candidate {
        smash: MappedInputs { buttons: 0x1 | 0x2 | 0x4 | 0x8 | 0x10, lstick_x: 100, lstick_y: 0, rstick_x: 0, rstick_y: 100 },
        ..Candidate::default()
    };
    let icons = icons_of(Mode::Smash, &c);
    let names: Vec<_> = icons.slots().iter().flatten().map(|s| (s.icon.name(), s.color)).collect();
    assert_eq!(
        names,
        [("right", Color::WHITE), ("up", Color::YELLOW), ("a", Color::GREEN), ("b", Color::RED), ("x", Color::CYAN)],
        "five at most"
    );
    // Duplicates are removed by name: ATTACK and ATTACK_RAW are one icon.
    let dup = icons_of(Mode::Smash, &smash(0x1 | 0x200 | 0x2 | 0x20000));
    assert_eq!(dup.count(), 2);
    // The remaining Smash icons.
    let rest = icons_of(Mode::Smash, &smash(0x8 | 0x10 | 0x100 | 0x400 | 0x1000));
    let names: Vec<_> = rest.slots().iter().flatten().map(|s| (s.icon.name(), s.color)).collect();
    assert_eq!(
        names,
        [("lb", Color::BLUE), ("zr", Color::PURPLE), ("plus", Color::WHITE), ("dpad_up", Color::WHITE), ("dpad_down", Color::WHITE)]
    );
    // Raw: the final order is the reverse of the list.
    let raw = Candidate { raw: RawInput { buttons: (1 << 5) | (1 << 6) | (1 << 4) | (1 << 9), ..RawInput::default() }, ..Candidate::default() };
    let names: Vec<_> = icons_of(Mode::Raw, &raw).slots().iter().flatten().map(|s| s.icon.name()).collect();
    assert_eq!(names, ["rb", "x", "b", "a"]);
    // The real digital trigger counts as L.
    let l = Candidate { raw: RawInput { buttons: 1 << 28, ..RawInput::default() }, ..Candidate::default() };
    assert_eq!(icons_of(Mode::Raw, &l).slots()[0].map(|s| s.icon.name()), Some("lb"));
    // Status only shows no icons; an unmapped bit shows none.
    assert_eq!(icons_of(Mode::StatusOnly, &smash(0xffff_ffff)).count(), 0);
    assert_eq!(icons_of(Mode::Smash, &smash(0x40)).count(), 0);
    // Every icon the display can produce is in the layout's list of 37.
    for icon in [Icon::A, Icon::B, Icon::X, Icon::Y, Icon::Lb, Icon::Rb, Icon::Zl, Icon::Zr, Icon::Plus, Icon::Minus, Icon::DpadUp, Icon::DownLeft] {
        assert!(LAYOUT_ICON_NAMES.contains(&icon.name()), "{}", icon.name());
    }
}

#[test]
fn difference_rules_smash_raw_status() {
    let a = smash(1);
    let b = smash(2);
    assert!(is_different(Mode::Smash, false, &a, &b));
    assert!(!is_different(Mode::Smash, false, &a, &a));
    // Smash: sticks compare binned (a small change inside a bin is not different).
    assert!(!is_different(Mode::Smash, false, &smash_stick(100, 0), &smash_stick(110, 0)));
    assert!(is_different(Mode::Smash, false, &smash_stick(100, 0), &smash_stick(-100, 0)));
    // Status differs only counts with Status On.
    let s1 = Candidate { status: 1, ..a };
    let s2 = Candidate { status: 2, ..a };
    assert!(!is_different(Mode::Smash, false, &s1, &s2));
    assert!(is_different(Mode::Smash, true, &s1, &s2));
    // Raw compares the raw buttons and binned raw sticks.
    let r1 = Candidate { raw: RawInput { buttons: 1, ..RawInput::default() }, ..Candidate::default() };
    let r2 = Candidate { raw: RawInput { buttons: 3, ..RawInput::default() }, ..Candidate::default() };
    assert!(is_different(Mode::Raw, false, &r1, &r2));
    assert!(!is_different(Mode::Raw, false, &a, &b), "raw ignores the mapped record");
    // Status only: nothing differs with Status Off.
    assert!(!is_different(Mode::StatusOnly, false, &s1, &s2));
    assert!(is_different(Mode::StatusOnly, true, &s1, &s2));
    assert!(!is_different(Mode::None, true, &s1, &s2));
}

#[test]
fn frame_count_cap_99() {
    let mut log = started();
    frame(&mut log, &SMASH, smash(1));
    for _ in 0..150 {
        frame(&mut log, &SMASH, smash(1));
    }
    assert_eq!(log.rows()[0].frames, 99);
    assert_eq!(log.rows()[1].ttl, 0, "no second row");
    // A held input keeps its ttl on overwrite and loses one per in-game frame.
    assert_eq!(log.rows()[0].ttl, ROW_TTL - 151);
}

#[test]
fn frame_counts_count_in_game_frames_not_polls() {
    let mut log = started();
    frame(&mut log, &SMASH, smash(1));
    // Repeated polls inside one game frame change nothing.
    for _ in 0..5 {
        log.record(&SMASH, smash(1));
    }
    assert_eq!(log.rows()[0].frames, 1);
    for n in 2..=10 {
        frame(&mut log, &SMASH, smash(1));
        assert_eq!(log.rows()[0].frames, n);
    }
    // Paused gameplay (no tick): polls change nothing, not even a different input.
    log.record(&SMASH, smash(2));
    assert_eq!(log.rows()[0].input.smash.buttons, 1);
}

#[test]
fn ring_rotation_physical_index() {
    let mut log = started();
    let mut panes = Vec::new();
    for n in 0..20u32 {
        frame(&mut log, &SMASH, smash(1 << (n % 7 + 8)));
        panes.push(log.physical(0));
    }
    // The newest row takes the pane the oldest held: the physical index of row 0 steps through the ring.
    assert_eq!(&panes[..4], &[14, 13, 12, 11]);
    assert_eq!(panes[14], 0);
    assert_eq!(panes[15], 14, "wraps");
    // Rows keep their panes while they age: row i of the log keeps pane (i + 15 - base) mod 15 stable
    // as base moves with the insert.
    let before: Vec<_> = (0..14).map(|i| log.physical(i)).collect();
    frame(&mut log, &SMASH, smash(0x30_0000));
    let after: Vec<_> = (1..15).map(|i| log.physical(i)).collect();
    assert_eq!(before, after);
}

#[test]
fn fade_alpha_curve() {
    assert_eq!(fade_alpha(600), 255);
    assert_eq!(fade_alpha(200), 255);
    assert_eq!(fade_alpha(0), 0);
    assert!(fade_alpha(199) >= 253 && fade_alpha(199) < 255);
    let mut last = 255;
    for ttl in (1..200).rev() {
        let a = fade_alpha(ttl);
        assert!(a <= last, "monotone at {ttl}");
        last = a;
    }
    assert!(fade_alpha(1) > 0 && fade_alpha(1) < 90);
    // A row is opaque for 400 frames, fades for 200, then disappears.
    let mut log = started();
    frame(&mut log, &SMASH, smash(1));
    for _ in 0..(ROW_TTL - 1) {
        frame(&mut log, &SMASH, smash(1));
    }
    assert_eq!(log.rows()[0].ttl, 0);
    assert_eq!(fade_alpha(log.rows()[0].ttl), 0);
}

#[test]
fn session_clear() {
    let mut log = started();
    for n in 0..5 {
        frame(&mut log, &SMASH, smash(1 << (n + 8)));
    }
    log.session_clear();
    assert_eq!((log.base(), log.overall(), log.rows()[0].ttl, log.rows()[0].frames), (0, 0, 0, 0));
    // Not counting until the next record.
    log.tick_frame();
    assert_eq!(log.overall(), 0);
}

#[test]
fn none_logs_nothing() {
    let mut log = Log::new();
    let off = Settings { mode: Mode::None, status: false };
    for _ in 0..10 {
        frame(&mut log, &off, smash(1));
    }
    assert_eq!(log.rows()[0].ttl, 0);
    assert_eq!(log.overall(), 0, "the counters are not started");
}

// ---- the renderer --------------------------------------------------------------------------------

fn count(plan: &Plan, f: impl Fn(&Op) -> bool) -> usize {
    plan.ops().iter().filter(|op| f(op)).count()
}

fn populated(n: u32) -> Log {
    let mut log = started();
    for k in 0..n {
        frame(&mut log, &SMASH, smash(1 << (k % 6 + 8)));
    }
    log
}

#[test]
fn unchanged_held_input_writes_nothing() {
    let mut log = populated(20);
    let mut r = Renderer::new();
    let first = r.plan(&log, &SMASH, true);
    assert!(count(&first, |op| matches!(op, Op::Icons { .. })) == 15, "the first draw is a full refresh");
    // Nothing changed: an empty plan (no lookups, no writes).
    assert!(r.plan(&log, &SMASH, true).is_empty());
    // The held newest input only advances its frame count: one text write.
    log = {
        let mut l = log;
        let held = l.rows()[0].input;
        frame(&mut l, &SMASH, held);
        l
    };
    let plan = r.plan(&log, &SMASH, true);
    assert_eq!(count(&plan, |op| matches!(op, Op::Frames { .. })), 1, "{:?}", plan.ops());
    assert_eq!(count(&plan, |op| matches!(op, Op::Icons { .. } | Op::Status { .. })), 0);
}

#[test]
fn rapid_changes_write_only_the_new_row_and_moves() {
    let mut log = populated(30);
    let mut r = Renderer::new();
    r.plan(&log, &SMASH, true);
    frame(&mut log, &SMASH, smash(0x19));
    let plan = r.plan(&log, &SMASH, true);
    // One new row's content; every row moved down by one slot: position writes, no content rewrite.
    assert_eq!(count(&plan, |op| matches!(op, Op::Icons { .. })), 1);
    // The frame count is written only when it differs from what the pane last showed (1 for a pane that
    // held a one-frame row).
    assert!(count(&plan, |op| matches!(op, Op::Frames { .. })) <= 1);
    assert!(count(&plan, |op| matches!(op, Op::Row { .. })) <= NUM_LOGS);
    // The newest row's content lands on the physical pane the oldest held.
    let newest = plan.ops().iter().find_map(|op| if let Op::Icons { phys, .. } = op { Some(*phys) } else { None }).unwrap();
    assert_eq!(usize::from(newest), log.physical(0));
}

#[test]
fn hidden_under_menus() {
    let log = populated(10);
    let mut r = Renderer::new();
    r.plan(&log, &SMASH, true);
    // Hiding writes the column visibility once and does no content work.
    let hidden = r.plan(&log, &SMASH, false);
    assert_eq!(hidden.ops(), &[Op::Column { visible: false }]);
    assert!(r.plan(&log, &SMASH, false).is_empty(), "a second hidden tick writes nothing");
    // Mode None hides too.
    assert!(r.plan(&log, &Settings { mode: Mode::None, status: false }, true).is_empty());
}

#[test]
fn refresh_after_hide_and_reregister() {
    let log = populated(12);
    let mut r = Renderer::new();
    r.plan(&log, &SMASH, true);
    r.plan(&log, &SMASH, false);
    // Visible again after a mode change while hidden: every retained row is redrawn in the new mode.
    let raw = Settings { mode: Mode::Raw, status: false };
    let plan = r.plan(&log, &raw, true);
    assert_eq!(count(&plan, |op| matches!(op, Op::Icons { .. })), 12);
    assert!(r.plan(&log, &raw, true).is_empty());
    // A registration, even at the same address: the cache is invalid and all rows are restored.
    r.on_layout_registered();
    let again = r.plan(&log, &raw, true);
    assert_eq!(count(&again, |op| matches!(op, Op::Icons { .. })), 12);
    assert!(matches!(again.ops()[0], Op::Column { visible: true }));
}

#[test]
fn status_toggle_follows_at_once_for_the_newest_row() {
    let mut log = started();
    frame(&mut log, &SMASH, Candidate { status: 7, fighter_kind: 3, ..smash(1) });
    let mut r = Renderer::new();
    r.plan(&log, &SMASH, true);
    let on = Settings { mode: Mode::Smash, status: true };
    let plan = r.plan(&log, &on, true);
    assert_eq!(count(&plan, |op| matches!(op, Op::Status { text: StatusText::Id { kind: 3, status: 7 }, .. })), 1);
    let off = r.plan(&log, &SMASH, true);
    assert_eq!(count(&off, |op| matches!(op, Op::Status { text: StatusText::Empty, .. })), 1);
}

#[test]
fn expired_rows_are_hidden_once() {
    let mut log = started();
    frame(&mut log, &SMASH, smash(1));
    for _ in 0..ROW_TTL {
        frame(&mut log, &SMASH, smash(1));
    }
    let mut r = Renderer::new();
    let plan = r.plan(&log, &SMASH, true);
    assert_eq!(count(&plan, |op| matches!(op, Op::RowHidden { .. })), 15);
    assert!(r.plan(&log, &SMASH, true).is_empty());
}

#[test]
fn a_missing_pane_disables_the_display() {
    let log = populated(5);
    let mut r = Renderer::new();
    r.disable();
    assert!(r.is_disabled());
    assert!(r.plan(&log, &SMASH, true).is_empty());
    r.session_clear();
    assert!(!r.is_disabled());
}

#[test]
fn bench_counts() {
    // Held input, frame count advancing: at most 3 writes (the fork's budget: lookups 3, text writes 1).
    let mut log = populated(40);
    let mut r = Renderer::new();
    r.plan(&log, &SMASH, true);
    let held = log.rows()[0].input;
    frame(&mut log, &SMASH, held);
    assert!(r.plan(&log, &SMASH, true).ops().len() <= 3);
    // The planning allocates nothing.
    let before = ALLOCS.with(Cell::get);
    for n in 0..500u32 {
        frame(&mut log, &SMASH, smash(1 << (n % 9 + 8)));
        let _ = r.plan(&log, &SMASH, n % 50 != 0);
    }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

#[test]
fn plan_buffer_holds_a_full_redraw() {
    let log = populated(40);
    let mut r = Renderer::new();
    let plan = r.plan(&log, &Settings { mode: Mode::Smash, status: true }, true);
    assert!(plan.ops().len() <= PLAN_MAX);
    assert_eq!(count(&plan, |op| matches!(op, Op::Row { .. })), 15);
}
