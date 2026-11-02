//! SPEC-input-04 §10 host rows: the chord rule, the button mapping by controller style, request flags,
//! suppression, Plus, glyphs, the INI shortcuts file (fail-closed reads, transactional writes with
//! filesystem-backed tests including a write failure) and the N7 editor rules.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use model::chords::*;
use model::ini_files::{self, EditRefusal, FileRead, Reject};
use model::menu::chord::Chord;
use model::menu_shortcuts::{self, MenuShortcuts, DEFAULT_INI, DEFAULT_SHORTCUTS};
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

fn c(buttons: &[u8]) -> Chord {
    Chord(buttons.iter().fold(0u16, |m, b| m | (1 << b)))
}

fn sample(held: &[u8], pressed: &[u8]) -> Sample {
    Sample { held: c(held).0, pressed: c(pressed).0 }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("trainingrooms-input04-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn menu_defaults() {
    let set = ChordSet::factory();
    let snap = Snapshot::factory();
    assert_eq!(ChordSet::factory().with_rows(&snap), set);
    assert_eq!(set.save_state, c(&[logical::ZL, logical::DPAD_DOWN]));
    assert_eq!(set.load_state, c(&[logical::ZL, logical::DPAD_UP]));
    assert_eq!(set.input_record, c(&[logical::ZR, logical::DPAD_DOWN]));
    assert_eq!(set.input_playback, c(&[logical::ZR, logical::DPAD_UP]));
    assert_eq!(set.cpu_cycle, c(&[logical::ZL, logical::DPAD_LEFT]));
    assert_eq!(set.player_cycle, c(&[logical::ZL, logical::DPAD_RIGHT]));
    assert_eq!(set.shortcuts, DEFAULT_SHORTCUTS);
}

#[test]
fn hold_press_rule() {
    let chord = c(&[logical::ZR, logical::DPAD_DOWN]);
    // Either button may be the held one.
    assert!(passes(chord, sample(&[logical::ZR, logical::DPAD_DOWN], &[logical::DPAD_DOWN])));
    assert!(passes(chord, sample(&[logical::ZR, logical::DPAD_DOWN], &[logical::ZR])));
    // Both pressed on the same poll passes.
    assert!(passes(chord, sample(&[logical::ZR, logical::DPAD_DOWN], &[logical::ZR, logical::DPAD_DOWN])));
    // Held for a while: nothing pressed, no pass.
    assert!(!passes(chord, sample(&[logical::ZR, logical::DPAD_DOWN], &[])));
    // One button missing.
    assert!(!passes(chord, sample(&[logical::ZR], &[logical::ZR])));
    // Extra unrelated buttons do not disqualify.
    assert!(passes(chord, sample(&[logical::ZR, logical::DPAD_DOWN, logical::A, logical::B], &[logical::DPAD_DOWN, logical::A])));
    // An empty set never passes.
    assert!(!passes(Chord::NONE, sample(&[logical::A], &[logical::A])));
}

#[test]
fn single_button_semantics() {
    // B8 (question 1): a one-button chord fires on the press, not on every poll while held.
    let chord = c(&[logical::Y]);
    assert!(passes(chord, sample(&[logical::Y], &[logical::Y])));
    assert!(!passes(chord, sample(&[logical::Y], &[])));
    // Three buttons: two pressed this poll, the third held before.
    let three = c(&[logical::A, logical::B, logical::X]);
    assert!(passes(three, sample(&[logical::A, logical::B, logical::X], &[logical::B, logical::X])));
    assert!(!passes(three, sample(&[logical::A, logical::B, logical::X], &[logical::X])));
}

#[test]
fn button_mapping_by_style() {
    let pro = 5;
    assert_eq!(logical_set(pro, bit::ZL), 1 << logical::ZL);
    assert_eq!(logical_set(pro, bit::LEFT_SL), 1 << logical::ZL);
    assert_eq!(logical_set(pro, bit::RIGHT_SL), 1 << logical::ZL);
    assert_eq!(logical_set(pro, bit::ZR | bit::LEFT_SR), 1 << logical::ZR);
    assert_eq!(logical_set(pro, bit::L), 1 << logical::L);
    assert_eq!(logical_set(pro, bit::R), 1 << logical::R);
    assert_eq!(logical_set(pro, bit::MINUS), 1 << logical::MINUS);
    assert_eq!(logical_set(pro, bit::PLUS | bit::STICK_L | bit::STICK_R), c(&[logical::PLUS, logical::STICK_L, logical::STICK_R]).0);
    // GameCube: Z arrives as zr and is the logical R; the triggers are logical ZL/ZR; L and Minus never.
    let gc = STYLE_GAMECUBE;
    assert_eq!(logical_set(gc, bit::ZR), 1 << logical::R);
    assert_eq!(logical_set(gc, bit::L), 1 << logical::ZL);
    assert_eq!(logical_set(gc, bit::REAL_DIGITAL_L), 1 << logical::ZL);
    assert_eq!(logical_set(gc, bit::R), 1 << logical::ZR);
    assert_eq!(logical_set(gc, bit::REAL_DIGITAL_R), 1 << logical::ZR);
    assert_eq!(logical_set(gc, bit::MINUS), 0);
    assert_eq!(logical_set(gc, bit::X | bit::A | bit::DPAD_UP), c(&[logical::X, logical::A, logical::DPAD_UP]).0);
    // On a GameCube the default chords still work as Z/L + D-pad.
    let s = Sample::of(gc, bit::R | bit::DPAD_DOWN, bit::DPAD_DOWN);
    assert!(passes(ChordSet::factory().input_record, s));
    assert!(unusable_on(c(&[logical::L, logical::A]), gc));
    assert!(unusable_on(c(&[logical::MINUS]), gc));
    assert!(!unusable_on(c(&[logical::ZL]), gc));
    assert!(!unusable_on(c(&[logical::MINUS]), pro));
}

#[test]
fn glyph_table() {
    assert_eq!(glyph(logical::A, 5), Some(0xE0E0));
    assert_eq!(glyph(logical::B, 5), Some(0xE0E0));
    assert_eq!(glyph(logical::X, 5), Some(0xE0E2));
    assert_eq!(glyph(logical::X, STYLE_GAMECUBE), Some(0xE206));
    assert_eq!(glyph(logical::Y, STYLE_GAMECUBE), Some(0xE207));
    assert_eq!(glyph(logical::L, 5), Some(0xE0E4));
    assert_eq!(glyph(logical::L, STYLE_GAMECUBE), None);
    assert_eq!(glyph(logical::R, STYLE_GAMECUBE), Some(0xE205));
    assert_eq!(glyph(logical::ZL, 5), Some(0xE0E6));
    assert_eq!(glyph(logical::ZL, STYLE_GAMECUBE), Some(0xE204));
    assert_eq!(glyph(logical::ZR, STYLE_GAMECUBE), Some(0xE208));
    assert_eq!(
        [logical::DPAD_UP, logical::DPAD_DOWN, logical::DPAD_LEFT, logical::DPAD_RIGHT].map(|b| glyph(b, 5).unwrap()),
        [0xE079, 0xE07A, 0xE07B, 0xE07C]
    );
    assert_eq!(
        [logical::DPAD_UP, logical::DPAD_DOWN, logical::DPAD_LEFT, logical::DPAD_RIGHT].map(|b| glyph(b, STYLE_GAMECUBE).unwrap()),
        [0xE209, 0xE20A, 0xE20B, 0xE20C]
    );
    assert_eq!((glyph(logical::PLUS, 5), glyph(logical::PLUS, STYLE_GAMECUBE)), (Some(0xE0EF), Some(0xE20D)));
    assert_eq!((glyph(logical::MINUS, 5), glyph(logical::MINUS, STYLE_GAMECUBE)), (Some(0xE0F0), None));
    assert_eq!((glyph(logical::STICK_L, 5), glyph(logical::STICK_R, 5)), (Some(0xE104), Some(0xE105)));
    assert_eq!(glyph(logical::STICK_L, STYLE_GAMECUBE), None);
}

#[test]
fn request_latch_and_clear() {
    let set = ChordSet::factory();
    let mut d = Detector::new();
    let record = sample(&[logical::ZR, logical::DPAD_DOWN], &[logical::DPAD_DOWN]);
    d.poll(&set, record, false);
    assert!(d.has_pending_request());
    // A second poll cannot double-fire; the request stays until consumed.
    d.poll(&set, record, false);
    assert!(d.is_pending(Request::InputRecord));
    assert!(d.consume(Request::InputRecord));
    assert!(!d.consume(Request::InputRecord));
    assert!(!d.has_pending_request());
    // Both chords in one poll: both requests are raised (the recorder drops the record one).
    let both = sample(&[logical::ZR, logical::DPAD_DOWN, logical::DPAD_UP], &[logical::DPAD_DOWN, logical::DPAD_UP]);
    d.poll(&set, both, false);
    assert!(d.is_pending(Request::InputRecord) && d.is_pending(Request::InputPlayback));
    d.clear_session();
    assert!(!d.has_pending_request());
}

#[test]
fn suppressed_under_menus() {
    let set = ChordSet::factory();
    let mut d = Detector::new();
    let load = sample(&[logical::ZL, logical::DPAD_UP], &[logical::DPAD_UP]);
    assert_eq!(d.poll(&set, load, true), 0);
    assert!(!d.has_pending_request());
    // Requests already pending are not cleared by suppression.
    d.poll(&set, load, false);
    d.poll(&set, sample(&[], &[]), true);
    assert!(d.is_pending(Request::LoadState));
}

#[test]
fn plus_passthrough_and_suppression() {
    let set = ChordSet::factory();
    let plus = sample(&[logical::PLUS], &[logical::PLUS]);
    // A plain Plus is not an opener and reaches the game's pause.
    let mut d = Detector::new();
    d.poll(&set, plus, false);
    assert!(!d.is_pending(Request::OpenMenu));
    assert!(!plus_shortcut_accepted(&set, plus));
    assert!(!clear_plus_bits(false, false));
    // An explicit Plus shortcut consumes its pause press.
    let mut with_plus = set;
    with_plus.shortcuts = with_plus.shortcuts.with_slot(1, c(&[logical::PLUS]));
    assert!(plus_shortcut_accepted(&with_plus, plus));
    d.poll(&with_plus, plus, false);
    assert!(d.is_pending(Request::OpenMenu));
    assert!(clear_plus_bits(false, true));
    // A modal owner clears the Plus bits.
    assert!(clear_plus_bits(true, false));
}

#[test]
fn permanent_chord_and_minus_open_the_menu() {
    let set = ChordSet::factory();
    let mut d = Detector::new();
    d.poll(&set, sample(&[logical::B, logical::DPAD_UP], &[logical::B]), false);
    assert!(d.consume(Request::OpenMenu));
    d.poll(&set, sample(&[logical::MINUS], &[logical::MINUS]), false);
    assert!(d.consume(Request::OpenMenu));
    d.poll(&set, sample(&[logical::MINUS], &[]), false);
    assert!(!d.consume(Request::OpenMenu), "edge rule: holding does not repeat");
}

#[test]
fn detection_allocates_nothing() {
    let set = ChordSet::factory();
    let mut d = Detector::new();
    let before = ALLOCS.with(Cell::get);
    for n in 0..1000u32 {
        let s = Sample::of(5, (n.wrapping_mul(2654435761)) & 0x000f_ffff, n.wrapping_mul(40503) & 0x000f_ffff);
        d.poll(&set, s, false);
        d.consume(Request::SaveState);
        d.consume(Request::OpenMenu);
    }
    assert_eq!(ALLOCS.with(Cell::get), before);
}

// ---- the INI file --------------------------------------------------------------------------------

#[test]
fn ini_missing_file_is_created_with_the_default_text() {
    let dir = temp_dir("missing");
    let path = dir.join("button_config.ini");
    let (shortcuts, error) = ini_files::load_shortcuts(&path);
    assert!(error.is_none());
    assert_eq!(shortcuts, DEFAULT_SHORTCUTS);
    assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_INI);
    // A second load reads it and writes nothing.
    let stamp = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(ini_files::load_shortcuts(&path).0, DEFAULT_SHORTCUTS);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), stamp);
}

#[test]
fn ini_reads_fail_closed() {
    let dir = temp_dir("failclosed");
    let path = dir.join("button_config.ini");
    // Oversized: defaults, even though the first lines are valid.
    let mut big = String::from("[menu]\nshortcut_1=X\n");
    while big.len() <= menu_shortcuts::MAX_BYTES {
        big.push_str("# padding padding padding\n");
    }
    fs::write(&path, &big).unwrap();
    assert_eq!(ini_files::read_capped(&path, menu_shortcuts::MAX_BYTES), FileRead::Rejected(Reject::TooLarge));
    assert_eq!(ini_files::load_shortcuts(&path).0, DEFAULT_SHORTCUTS);
    // Exactly at the cap is accepted.
    let mut exact = String::from("[menu]\nshortcut_1=X\n");
    while exact.len() < menu_shortcuts::MAX_BYTES - 1 {
        exact.push('#');
    }
    exact.push('\n');
    assert_eq!(exact.len(), menu_shortcuts::MAX_BYTES);
    fs::write(&path, &exact).unwrap();
    assert_eq!(ini_files::load_shortcuts(&path).0.slots()[0], c(&[logical::X]));
    // Invalid UTF-8 anywhere: defaults.
    let mut bytes = b"[menu]\nshortcut_1=X\n".to_vec();
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    fs::write(&path, &bytes).unwrap();
    assert_eq!(ini_files::load_shortcuts(&path).0, DEFAULT_SHORTCUTS);
    // Unreadable (a directory where the file should be): defaults, and nothing is written over it.
    let dpath = dir.join("dir.ini");
    fs::create_dir(&dpath).unwrap();
    assert_eq!(ini_files::read_capped(&dpath, 100), FileRead::Rejected(Reject::Unreadable));
    assert_eq!(ini_files::load_shortcuts(&dpath).0, DEFAULT_SHORTCUTS);
    assert!(dpath.is_dir());
}

#[test]
fn ini_written_only_on_confirmed_change() {
    let dir = temp_dir("written");
    let path = dir.join("button_config.ini");
    fs::write(&path, "# my notes\n[menu]\nshortcut_1=Minus\nshortcut_2=disabled\nunknown=1\n").unwrap();
    let before = fs::read_to_string(&path).unwrap();
    // Loading changes nothing.
    ini_files::load_shortcuts(&path);
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    // A confirmed change rewrites only the two lines.
    let changed = DEFAULT_SHORTCUTS.with_slot(1, c(&[logical::ZL, logical::R]));
    ini_files::write_shortcuts(&path, changed).unwrap();
    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("# my notes\n") && after.contains("unknown=1\n"));
    assert_eq!(MenuShortcuts::parse(&after), changed);
    // No stray temporary or backup file remains.
    let names: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    assert_eq!(names, ["button_config.ini"]);
    // The survived-restart row: a fresh load gives the same shortcuts.
    assert_eq!(ini_files::load_shortcuts(&path).0, changed);
}

#[test]
fn ini_write_failure_keeps_the_old_file() {
    let dir = temp_dir("writefail");
    let path = dir.join("button_config.ini");
    fs::write(&path, DEFAULT_INI).unwrap();
    // The temporary name is taken by a directory: the write fails before it touches the file.
    fs::create_dir(dir.join("button_config.ini.tmp")).unwrap();
    let error = ini_files::write_shortcuts(&path, DEFAULT_SHORTCUTS.with_slot(0, c(&[logical::X])));
    assert!(error.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_INI);
    fs::remove_dir(dir.join("button_config.ini.tmp")).unwrap();
    // A read-only folder (the card is full or locked): the write fails, the old content stays and no
    // temporary file is left.
    let ro = dir.join("ro");
    fs::create_dir(&ro).unwrap();
    fs::write(ro.join("keep.ini"), "old").unwrap();
    let mut perms = fs::metadata(&ro).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o555);
    fs::set_permissions(&ro, perms.clone()).unwrap();
    if fs::File::create(ro.join("probe")).is_err() {
        assert!(ini_files::write_atomic(&ro.join("keep.ini"), b"new").is_err());
        assert_eq!(fs::read_to_string(ro.join("keep.ini")).unwrap(), "old");
        assert!(!ro.join("keep.ini.tmp").exists());
    }
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    fs::set_permissions(&ro, perms).unwrap();    // A missing folder fails and leaves nothing.
    assert!(ini_files::write_atomic(&dir.join("nope").join("x.ini"), b"y").is_err());
    // An orphaned backup (a crash inside the swap) is put back.
    let orphan = dir.join("orphan.ini");
    fs::write(dir.join("orphan.ini.bak"), "old").unwrap();
    ini_files::restore_orphaned_backup(&orphan);
    assert_eq!(fs::read_to_string(&orphan).unwrap(), "old");
}

#[test]
fn shortcut_rows_open_editor_and_permanent_chord_fixed() {
    let menu = model::menu::table::ROWS.iter().filter(|r| matches!(r.control, model::menu::table::Control::Editor(model::menu::table::Editor::ShortcutSlot(_)))).count();
    assert_eq!(menu, 2, "two shortcut rows");
    // The permanent chord cannot be edited into a slot, and a duplicate slot is refused.
    let cur = DEFAULT_SHORTCUTS;
    assert_eq!(ini_files::check_edit(cur, 1, menu_shortcuts::DEFAULT_OPEN_MENU_CONFIG), Err(EditRefusal::Permanent));
    assert_eq!(ini_files::check_edit(cur, 1, cur.slots()[0]), Err(EditRefusal::SameAsOther));
    assert!(ini_files::check_edit(cur, 1, Chord::NONE).is_ok(), "an empty chord disables the slot");
    let ok = ini_files::check_edit(cur, 1, c(&[logical::X, logical::Y])).unwrap();
    assert_eq!(ok.slots(), [cur.slots()[0], c(&[logical::X, logical::Y])]);
    // The six chord rows may overlap a shortcut.
    assert!(ini_files::check_edit(cur, 1, ChordSet::factory().save_state).is_ok());
}

#[test]
fn shortcut_change_applies_immediately() {
    // The new chord opens the menu as soon as it is in the set; the old one no longer does.
    let mut set = ChordSet::factory();
    let mut d = Detector::new();
    let minus = sample(&[logical::MINUS], &[logical::MINUS]);
    d.poll(&set, minus, false);
    assert!(d.consume(Request::OpenMenu));
    set.shortcuts = set.shortcuts.with_slot(0, c(&[logical::X]));
    d.poll(&set, minus, false);
    assert!(!d.consume(Request::OpenMenu));
    d.poll(&set, sample(&[logical::X], &[logical::X]), false);
    assert!(d.consume(Request::OpenMenu));
}

/// R2-3 (review R2): the Save and Load chords are raised by the detector and pulled once each by their consumer; a
/// request stays pending until its consumer runs, and a blocked poll raises nothing.
#[test]
fn r2_3_save_and_load_chords_are_pulled_once_each_and_wait_for_their_consumer() {
    let set = ChordSet::factory();
    let mut detector = Detector::new();
    let save = sample(&[logical::ZL, logical::DPAD_DOWN], &[logical::DPAD_DOWN]);
    let load = sample(&[logical::ZL, logical::DPAD_UP], &[logical::DPAD_UP]);
    detector.poll(&set, save, false);
    assert!(detector.is_pending(Request::SaveState));
    assert_eq!(detector.poll(&set, load, true), 0, "a blocked poll raises nothing");
    assert!(!detector.is_pending(Request::LoadState));
    detector.poll(&set, load, false);
    assert!(detector.is_pending(Request::LoadState));
    // Several polls later the requests are still pending (the consumer is the save-state fighter step).
    for _ in 0..30 {
        detector.poll(&set, sample(&[], &[]), false);
    }
    assert!(detector.consume(Request::SaveState));
    assert!(!detector.consume(Request::SaveState), "once");
    assert!(detector.is_pending(Request::LoadState), "a load nobody is eligible for yet stays pending");
    assert!(detector.consume(Request::LoadState));
}

/// R2-6 (review R2): a nondefault INI shortcut raises the OpenMenu request, also a Plus shortcut, from the raw
/// sample (the Plus bits are cleared only afterwards), and the permanent chord is not a shortcut.
#[test]
fn r2_6_an_ini_shortcut_raises_the_open_menu_request_including_plus() {
    let mut set = ChordSet::factory();
    set.shortcuts = MenuShortcuts::parse("[menu]\nshortcut_1=Plus\nshortcut_2=X + Y\n");
    let mut detector = Detector::new();
    let plus = sample(&[logical::PLUS], &[logical::PLUS]);
    detector.poll(&set, plus, false);
    assert!(detector.is_pending(Request::OpenMenu));
    assert!(plus_shortcut_accepted(&set, plus), "an accepted Plus shortcut consumes the pause press");
    assert!(detector.consume(Request::OpenMenu));
    let xy = sample(&[logical::X, logical::Y], &[logical::Y]);
    detector.poll(&set, xy, false);
    assert!(detector.is_pending(Request::OpenMenu));
    assert!(detector.consume(Request::OpenMenu));
    // The default shortcut is gone once the file says otherwise; the permanent chord always opens.
    let minus = sample(&[logical::MINUS], &[logical::MINUS]);
    detector.poll(&set, minus, false);
    assert!(!detector.is_pending(Request::OpenMenu));
    let permanent = sample(&[logical::B, logical::DPAD_UP], &[logical::DPAD_UP]);
    detector.poll(&set, permanent, false);
    assert!(detector.is_pending(Request::OpenMenu));
}
#[test]
fn the_operator_controller_cycle_chord_reads_its_own_sample_and_touches_no_pending_flag() {
    let set = ChordSet::factory();
    let d = Detector::new();
    let cycle = sample(&[logical::ZL, logical::DPAD_LEFT], &[logical::DPAD_LEFT]);
    assert!(Detector::operator_cycle(&set, cycle));
    assert!(!Detector::operator_cycle(&set, sample(&[logical::ZL], &[logical::ZL])));
    assert!(!Detector::operator_cycle(&set, sample(&[logical::ZR, logical::DPAD_DOWN], &[logical::DPAD_DOWN])));
    assert!(!d.has_pending_request());
}
