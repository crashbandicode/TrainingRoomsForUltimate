use crate::engine::{library, macros, playback, rr_runtime};
use crate::model::combo::{Aim, Combo, Event, LinkRec};
use crate::model::combo_record::{ComboLibrary, ComboRecord};
use crate::model::frame_strip::{Button as InputButton, Move, Verdict};
use crate::model::hud_row::{self, Take};
use crate::engine::macro_ui::clip_marks::Span;
use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEditor, ClipEffect};
use crate::model::menu::input::Button;
use crate::model::menu::table::{Control, COMBOS_SEQ, ROWS, SAVES_MACROS};
use crate::model::recording_menu::{Effect, Face, FileKind, RecordingScreens};
use crate::model::settings::{self, Snapshot};
use crate::model::state::record::SavedState;
use crate::model::trim::Trim;
use library::{Chain, LoopCount, NamedSave, RecordingIndex, Role, Sequence, MACROS_DIR, SAVES_DIR, SEQUENCES_DIR};
use macros::{InputFrame, Library, MacroRecording, StartState};
use playback::{Book, Playback};
use crate::model::store::Store;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::engine::rr_host as host;

#[test]
fn switching_tracks_shows_the_selected_handle_on_the_new_track() {
    let track = |entry, len| macros::ReplayTrack { entry, quantized_analog: false, fighter: None,
        frames: vec![macros::ReplayFrame { start: StartState::neutral(entry, 0, 1.0),
            input: InputFrame::default(), x: 0.0, y: 0.0 }; len] };
    let session = macros::ReplaySession { tracks: vec![track(0, 10), track(1, 30)], ..Default::default() };
    let mut editor = ClipEditor::new(session, vec![0]).unwrap();
    editor.on_action(ClipAction::SelectOut);
    editor.on_action(ClipAction::NextFighter);
    let view = editor.view();
    assert_eq!((view.end, view.cursor, view.shown), (30, 29, Some(29)));
    editor.on_action(ClipAction::Nudge(-4));
    editor.on_action(ClipAction::PreviousFighter);
    let view = editor.view();
    assert_eq!((view.end, view.cursor, view.shown), (10, 9, Some(9)));
    editor.on_action(ClipAction::SelectOut);
    editor.on_action(ClipAction::Nudge(-3));
    editor.on_action(ClipAction::NextFighter);
    let view = editor.view();
    assert_eq!((view.cursor, view.shown), (6, None));
}

#[test]
fn install5_previewable_capture_windows_remain_previewable() {
    use macros::{MoveFamily, MoveStartRules, ReplayFrame, ReplaySession, ReplayTrack};

    const CLEAN: [i32; 3] = [0, 14, 15];
    let rules = MoveStartRules {
        families: vec![MoveFamily { fighter: None, statuses: vec![45, 46, 47], buttons: 1 }],
        setup: vec![7], ground: 0, air: 2, wait: 0, fall: 14, trigger_buttons: 1,
    };
    let frames: Vec<_> = (0..48).map(|i| {
        let status = match i {
            0..=3 | 40..=47 => 0, 4..=7 => 14, 8..=11 => 7, 12..=15 | 32..=34 => 45,
            16..=18 | 35..=36 => 46, 19 | 37 => 47, 20..=27 => 15, 28..=31 => 7, _ => 99,
        };
        ReplayFrame {
            input: InputFrame { buttons: u32::from([5, 10, 31].contains(&i)), lstick_x: i as i8, ..Default::default() },
            start: StartState { status, situation: if (4..=27).contains(&i) { 2 } else { 0 },
                ..StartState::neutral(if i < 40 { 74 } else { 8 }, 0, 1.0) }, x: 0.0, y: 0.0,
        }
    }).collect();
    let session = ReplaySession { tracks: vec![ReplayTrack {
        entry: 1, frames, quantized_analog: false, fighter: None,
    }], breaks: vec![24], truncated: false };
    let track = &session.tracks[0];
    let marks = crate::engine::macro_ui::clip_marks::Marks::from_inputs(track.frames.iter().map(|f| f.input));
    let mut accepted = [0; 3];
    for out in 1..=48 {
        for at in 0..out {
            for (mode, key) in [None, Some(ClipAction::Snap), Some(ClipAction::UseLeadIn)].into_iter().enumerate() {
                // The old Y only applied the marked boundary. A refused lead-in
                // left In unchanged, so X could still preview an ordinary clean In.
                let mv = if matches!(key, Some(ClipAction::UseLeadIn)) {
                    rules.snap(track, at).ok().filter(|mv| mv.action_frame < out)
                } else { None };
                let (start, context) = match mv {
                    Some(mv) => (mv.frame, mv.context),
                    _ => {
                        let n = if matches!(key, Some(ClipAction::Snap)) {
                            marks.snap_in(at).unwrap_or(at).min(out - 1)
                        } else { at };
                        let context = track.frames[n as usize].start;
                        if !CLEAN.contains(&context.status) { continue; }
                        (n, context)
                    }
                };
                if (start < 24 && 24 < out) || (start < 40 && 40 < out) { continue; }
                let mut editor = ClipEditor::new(session.clone(), CLEAN.to_vec()).unwrap().with_move_rules(rules.clone());
                editor.on_action(ClipAction::SelectOut);
                editor.on_action(ClipAction::Nudge(out as i32 - 48));
                editor.on_action(ClipAction::SelectIn);
                editor.on_action(ClipAction::Nudge(at as i32));
                if let Some(key) = key { editor.on_action(key); }
                let ClipEffect::Preview(take) = editor.on_action(ClipAction::TogglePlay) else {
                    panic!("lost an install 5 preview: {key:?} {at}..{out} (In {start})")
                };
                assert_eq!(take.start, context, "{key:?} {at}..{out} (In {start})");
                assert!(take.frames.iter().copied().eq(track.frames[start as usize..out as usize].iter().map(|f| f.input)));
                accepted[mode] += 1;
            }
        }
    }
    assert!(accepted.iter().all(|&count| count > 0), "{accepted:?}");
}

static NEXT: AtomicUsize = AtomicUsize::new(0);
static HOST: Mutex<()> = Mutex::new(());

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tr-trim-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct HostGuard(MutexGuard<'static, ()>);

impl HostGuard {
    fn new() -> Self {
        let guard = HOST.lock().unwrap_or_else(|error| error.into_inner());
        host::slot_stop();
        host::warp_landed();
        host::note_load();
        host::stop_preview();
        Self(guard)
    }
}

impl Drop for HostGuard {
    fn drop(&mut self) {
        host::slot_stop();
        host::warp_landed();
        host::note_load();
        host::stop_preview();
        let _ = &self.0;
    }
}

fn frame(buttons: u32) -> InputFrame {
    InputFrame { buttons, ..InputFrame::default() }
}

fn recording(name: &str, buttons: &[u32], trim: Trim) -> MacroRecording {
    let mut recording = MacroRecording::new(
        name,
        "13.0.5",
        StartState::neutral(8, 0, 1.0),
        buttons.iter().copied().map(frame).collect(),
    ).unwrap();
    recording.trim = trim;
    recording
}

fn state(kind: i32) -> SavedState {
    let mut state = SavedState::never_saved();
    state.fighter_kind = kind;
    state
}

fn tick(root: &Path) {
    host::service_at(root, &[None, None, None, None, None], "2026-11-24", state(8), state(7));
}

fn buttons(machine: &mut Playback, role: Role, book: &Book) -> Vec<u32> {
    let mut out = Vec::new();
    for _ in 0..100 {
        match machine.step(role, book) {
            Some(played) => out.push(played.frame.buttons),
            None => return out,
        }
    }
    panic!("playback did not finish");
}

fn combo(input_only: bool, name: &str) -> ComboRecord {
    let link = if input_only {
        let mut step = LinkRec::INPUT;
        step.events[0] = Some(Event::press(InputButton::Jump, Aim::Neutral, 0));
        step
    } else {
        LinkRec::presses(Move::Aerial, Move::Aerial, Verdict::True(None), None, &[])
    };
    let mut combo = Combo::from_links([link]).unwrap();
    combo.input_only = input_only;
    ComboRecord::new(name.into(), combo, None)
}

fn open(screens: &mut RecordingScreens, group: u8, section: u8, name: &str) {
    let rows = screens.section(Face::Page, group, section).rows;
    let at = rows.iter().position(|row| row.name == name).expect("fixture row");
    screens.set_cursor(Face::Page, group, section, at);
    screens.handle_page(Button::A, true, group);
}

fn open_file(root: &Path, group: u8, name: &str, drill: bool) -> RecordingScreens {
    let mut screens = RecordingScreens::new();
    screens.set_catalog_bundle(rr_runtime::load_catalog(root).unwrap());
    if drill {
        open(&mut screens, COMBOS_SEQ, 1, "DRILLS >");
        open(&mut screens, COMBOS_SEQ, 1, name);
    } else {
        open(&mut screens, group, 1, name);
    }
    assert!(screens.file_open(), "fixture opens the file");
    screens
}

#[test]
fn trim_row_is_a_dropdown_of_start_end_both_off() {
    let row = ROWS.iter().find(|row| row.id == "recording_crop").unwrap();
    assert_eq!(row.title, "Trim trailing neutral");
    assert!(matches!(row.control, Control::Dropdown(options) if options == ["Start", "End", "Both", "Off"]));
    assert_eq!(row.help.lines().count(), 2);
}

#[test]
fn trim_cuts_only_the_sides_its_value_names() {
    for (input, windows) in [
        (vec![frame(0), frame(1), frame(2)], [1..3, 0..3, 1..3, 0..3]),
        (vec![frame(1), frame(2), frame(0)], [0..3, 0..2, 0..2, 0..3]),
        (vec![frame(0), frame(1), frame(2), frame(0)], [1..4, 0..3, 1..3, 0..4]),
    ] {
        for (trim, expected) in [Trim::Start, Trim::End, Trim::Both, Trim::Off].into_iter().zip(windows) {
            assert_eq!(trim.window(&input), expected, "{trim:?}");
            let mut applied = input.clone();
            trim.apply(&mut applied);
            assert_eq!(applied, input[expected].to_vec(), "{trim:?}");
        }
    }
}

#[test]
fn trim_all_neutral_take_keeps_nothing() {
    let input = [frame(0), frame(0x8000), InputFrame { lstick_x: 23, rstick_y: -23, ..frame(0) }];
    for trim in [Trim::Start, Trim::End, Trim::Both] {
        assert!(trim.window(&input).is_empty(), "{trim:?}");
        let mut frames = input.to_vec();
        trim.apply(&mut frames);
        assert!(frames.is_empty(), "{trim:?}");
    }
    assert_eq!(Trim::Off.window(&input), 0..input.len());
}

#[test]
fn trim_without_neutral_keeps_every_frame() {
    let input = [frame(1), InputFrame { lstick_x: 24, ..frame(0) }, frame(2)];
    for trim in [Trim::Start, Trim::End, Trim::Both, Trim::Off] {
        assert_eq!(trim.window(&input), 0..input.len(), "{trim:?}");
        let mut applied = input.to_vec();
        trim.apply(&mut applied);
        assert_eq!(applied, input);
    }
}

#[test]
fn trim_end_is_the_old_crop_on_one_table() {
    for input in [
        vec![],
        vec![frame(0), frame(0x8000)],
        vec![frame(1)],
        vec![frame(0), frame(1)],
        vec![frame(1), frame(0)],
        vec![frame(0), frame(1), frame(0x8000), frame(0)],
        vec![InputFrame { lstick_x: -24, ..frame(0) }, frame(0)],
    ] {
        let mut old = input.clone();
        while old.last().is_some_and(|frame| {
            let stick = |value: i8| value.unsigned_abs() < 24;
            (frame.buttons == 0 || frame.buttons == 0x8000)
                && stick(frame.lstick_x) && stick(frame.lstick_y)
                && stick(frame.rstick_x) && stick(frame.rstick_y)
        }) {
            old.pop();
        }
        let mut new = input.clone();
        Trim::End.apply(&mut new);
        assert_eq!(new, old);
        let mut crop = input.clone();
        crate::engine::macro_ui::macro_panel::crop_neutral_tail(&mut crop);
        assert_eq!(crop, old);
    }
}

#[test]
fn record_trims_by_the_setting() {
    let root = Scratch::new();
    for (row, expected) in [
        ([1, 0, 0, 0], vec![1, 0]),
        ([0, 1, 0, 0], vec![0, 1]),
        ([0, 0, 1, 0], vec![1]),
        ([0, 0, 0, 1], vec![0, 1, 0]),
    ] {
        let loaded = settings::parse(&format!(r#"{{"menu":{{"recording_crop":{row:?}}}}}"#));
        let trim = crate::model::recording::RecorderSettings::from_snapshot(&loaded.settings.menu).trim;
        let mut capture = rr_runtime::Capture::idle();
        rr_runtime::toggle_record(&mut capture, Role::Player);
        for input in [frame(0), frame(1), frame(0)] {
            rr_runtime::push_frame(&mut capture, Role::Player, input);
        }
        let stop = rr_runtime::finish_recording(&root.0, &capture, state(8), state(7), (0.0, 0.0), 1, trim).unwrap();
        let id = stop.cursor().unwrap();
        let saved = Library::new(root.0.join(MACROS_DIR)).load(id).unwrap();
        assert_eq!(saved.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), expected);
        let take = rr_runtime::save_drill_take(
            &root.0,
            vec![frame(0), frame(1), frame(0)],
            StartState::neutral(8, 0, 1.0),
            trim,
            1,
            &[],
        ).unwrap().unwrap();
        let saved = Library::new(root.0.join(MACROS_DIR)).load(&take).unwrap();
        assert_eq!(saved.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), expected);
    }
}

#[test]
fn recording_trim_migrates_on_to_end_and_off_to_off() {
    for (old, expected) in [([1, 0], [0, 1, 0, 0]), ([0, 1], [0, 0, 0, 1])] {
        let loaded = settings::parse(&format!(
            r#"{{"menu":{{"recording_crop":{old:?}}},"defaults_menu":{{"recording_crop":{old:?}}}}}"#
        ));
        assert_eq!(loaded.settings.menu.get("recording_crop"), Some(expected.as_slice()));
        assert_eq!(loaded.settings.defaults.get("recording_crop"), Some(expected.as_slice()));
        assert_eq!(Trim::migrate(&old), Some(expected));
        let saved: serde_json::Value = serde_json::from_str(&loaded.settings.to_json()).unwrap();
        assert_eq!(saved["menu"]["recording_crop"], serde_json::json!(expected));
        assert_eq!(saved["defaults_menu"]["recording_crop"], serde_json::json!(expected));
    }
    assert_eq!(Trim::migrate(&[0, 1, 0, 0]), None);
    assert_eq!(Trim::from_row(&[]), Trim::Off);
    assert_eq!(Trim::from_row(&[0, 1, 1, 0]), Trim::End);
    let invalid = settings::parse(r#"{"menu":{"recording_crop":[2,0]}}"#);
    assert!(invalid.issues.iter().any(|issue| issue.at == "menu.recording_crop"));
}

#[test]
fn trim_option_rows_decode_each_value() {
    for (row, expected) in [
        ([1, 0, 0, 0], Trim::Start),
        ([0, 1, 0, 0], Trim::End),
        ([0, 0, 1, 0], Trim::Both),
        ([0, 0, 0, 1], Trim::Off),
        ([0, 0, 0, 0], Trim::Off),
        ([1, 0, 1, 1], Trim::Start),
    ] {
        assert_eq!(Trim::from_row(&row), expected);
    }
}

#[test]
fn recording_trim_factory_is_end() {
    assert_eq!(Snapshot::factory().get("recording_crop"), Some(&[0, 1, 0, 0][..]));
    assert_eq!(crate::model::recording::RecorderSettings::factory().trim, Trim::End);
}

#[test]
fn old_files_without_trim_read_off_and_stay_unchanged() {
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let macro_id = macros.save_new(&recording("Old macro", &[1, 0], Trim::Off)).unwrap();
    let path = root.0.join(MACROS_DIR).join(format!("{macro_id}.json"));
    let before = std::fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains("\"trim\""));
    assert_eq!(macros.load(&macro_id).unwrap().trim, Trim::Off);
    assert_eq!(std::fs::read(path).unwrap(), before);

    let combos = ComboLibrary::new(root.0.join("combos"));
    for (drill, name) in [(false, "Old combo"), (true, "Old drill")] {
        let id = combos.save_new(&combo(drill, name)).unwrap();
        let path = root.0.join("combos").join(format!("{id}.json"));
        let before = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&before).contains("\"trim\""));
        assert_eq!(combos.load(&id).unwrap().trim, Trim::Off);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[test]
fn file_trim_round_trips_without_removing_frames() {
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let combos = ComboLibrary::new(root.0.join("combos"));
    for (trim, key) in [
        (Trim::Start, Some("start")),
        (Trim::End, Some("end")),
        (Trim::Both, Some("both")),
        (Trim::Off, None),
    ] {
        let name = format!("Macro {trim:?}");
        let id = macros.save_new(&recording(&name, &[0, 1, 0], trim)).unwrap();
        let text = std::fs::read_to_string(root.0.join(MACROS_DIR).join(format!("{id}.json"))).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json.get("trim").and_then(|value| value.as_str()), key);
        let loaded = macros.load(&id).unwrap();
        assert_eq!(loaded.trim, trim);
        assert_eq!(loaded.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), [0, 1, 0]);
        for input in [false, true] {
            let mut record = combo(input, &format!("Combo {trim:?} {input}"));
            record.trim = trim;
            let id = combos.save_new(&record).unwrap();
            let text = std::fs::read_to_string(root.0.join("combos").join(format!("{id}.json"))).unwrap();
            let json: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(json.get("trim").and_then(|value| value.as_str()), if input { key } else { None });
            assert_eq!(combos.load(&id).unwrap().trim, if input { trim } else { Trim::Off });
        }
    }
}

#[test]
fn macro_and_drill_screens_have_a_trim_row() {
    let root = Scratch::new();
    let macro_id = Library::new(root.0.join(MACROS_DIR)).save_new(&recording("Macro", &[1], Trim::Off)).unwrap();
    Store::<Chain>::new(root.0.join(MACROS_DIR)).save_new(&Chain::new("Chain", vec![macro_id]).unwrap()).unwrap();
    Store::<NamedSave>::new(root.0.join(SAVES_DIR)).save_new(&NamedSave::new("Save", state(8), state(7)).unwrap()).unwrap();
    let combos = ComboLibrary::new(root.0.join("combos"));
    let combo_id = combos.save_new(&combo(false, "Combo")).unwrap();
    combos.save_new(&combo(true, "Drill")).unwrap();
    Store::<Sequence>::new(root.0.join(SEQUENCES_DIR)).save_new(&Sequence::new("Sequence", vec![combo_id]).unwrap()).unwrap();
    for (group, name, drill) in [
        (SAVES_MACROS, "Macro", false),
        (COMBOS_SEQ, "Drill", true),
    ] {
        let screens = open_file(&root.0, group, name, drill);
        let rows = screens.file_rows();
        let crop = rows.iter().position(|row| row.title == "Crop").unwrap();
        assert!(crop > 0 && rows[crop - 1].title == "Trim", "{name}: {rows:?}");
        assert_eq!(rows[crop - 1].value, "Off");
    }
    for (group, section, name) in [
        (SAVES_MACROS, 1, "Chain"),
        (SAVES_MACROS, 0, "Save"),
        (COMBOS_SEQ, 0, "Sequence"),
    ] {
        let mut screens = RecordingScreens::new();
        screens.set_catalog_bundle(rr_runtime::load_catalog(&root.0).unwrap());
        open(&mut screens, group, section, name);
        assert!(screens.file_open());
        assert!(!screens.file_rows().iter().any(|row| row.title == "Trim"), "{name}");
    }
}

#[test]
fn a_hit_combo_has_no_trim_row() {
    let root = Scratch::new();
    let combos = ComboLibrary::new(root.0.join("combos"));
    let mut record = combo(false, "Combo");
    record.trim = Trim::Both;
    let original = record.combo;
    let id = combos.save_new(&record).unwrap();
    let path = root.0.join("combos").join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(json.get("trim").is_none());
    json["trim"] = serde_json::json!("both");
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let loaded = combos.load(&id).unwrap().combo;
    assert_eq!(loaded, original);
    let attempt = original.links().next().unwrap();
    assert_eq!(
        crate::model::combo::grade(loaded.links().next().unwrap(), attempt),
        crate::model::combo::grade(attempt, attempt)
    );
    let screens = open_file(&root.0, COMBOS_SEQ, "Combo", false);
    assert!(!screens.file_rows().iter().any(|row| row.title == "Trim"));
}

#[test]
fn file_trim_row_cycles_and_is_saved() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let macro_id = macros.save_new(&recording("Macro", &[0, 1, 0], Trim::Off)).unwrap();
    let take_id = macros.save_new(&recording("Drill take 0000000000000001", &[0, 1, 0], Trim::Off)).unwrap();
    let combos = ComboLibrary::new(root.0.join("combos"));
    let drill_id = combos.save_new(&combo(true, "Drill").with_take(&take_id)).unwrap();
    for (group, name, drill, id, kind) in [
        (SAVES_MACROS, "Macro", false, macro_id.as_str(), FileKind::Macro),
        (COMBOS_SEQ, "Drill", true, drill_id.as_str(), FileKind::Combo),
    ] {
        let mut screens = open_file(&root.0, group, name, drill);
        let row = screens.file_rows().iter().position(|row| row.title == "Trim");
        assert!(row.is_some(), "{name}: Trim row missing");
        screens.set_file_cursor(row.unwrap());
        for (button, expected, text) in [
            (Button::Right, Trim::Start, "Start"),
            (Button::A, Trim::End, "End"),
            (Button::Right, Trim::Both, "Both"),
            (Button::Right, Trim::Off, "Off"),
            (Button::Left, Trim::Both, "Both"),
        ] {
            screens.handle_page(button, true, group);
            assert_eq!(screens.file_rows()[row.unwrap()].value, text);
            let effect = screens.take_effect();
            assert_eq!(effect, Effect::SetTrim { id: id.into(), kind, trim: expected });
            if kind == FileKind::Macro { host::take_slots_dirty(); }
            host::post_effect(effect);
            tick(&root.0);
            let actual = if kind == FileKind::Macro { macros.load(id).unwrap().trim } else { combos.load(id).unwrap().trim };
            assert_eq!(actual, expected);
            if kind == FileKind::Macro { assert_eq!(host::take_slots_dirty(), Some(false)); }
            if drill {
                assert_eq!(macros.load(&take_id).unwrap().trim, Trim::Off, "the choice belongs to the drill");
            }
        }
        let reopened = open_file(&root.0, group, name, drill);
        assert_eq!(reopened.file_rows().iter().find(|row| row.title == "Trim").unwrap().value, "Both");
    }
}

#[test]
fn macro_plays_inside_its_trim_window() {
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some("m"), 1).unwrap();
    let mut book = Book::new(index);
    book.add_recording("m", recording("Macro", &[0, 1, 2, 0], Trim::Both));
    let mut machine = Playback::new();
    assert!(machine.play_chord(Role::Player, &book).started);
    assert_eq!(buttons(&mut machine, Role::Player, &book), [1, 2]);
    assert!(machine.play_file(Role::Player, &book, "m").started);
    assert_eq!(buttons(&mut machine, Role::Player, &book), [1, 2]);
}

#[test]
fn chain_steps_use_each_macros_trim() {
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some("chain"), 1).unwrap();
    index.set_loop(Role::Player, LoopCount::Finite(2));
    let mut book = Book::new(index);
    book.add_recording("a", recording("A", &[0, 1, 0], Trim::Start));
    book.add_recording("b", recording("B", &[0, 2, 0], Trim::End));
    book.add_chain("chain", Chain::new("Chain", vec!["a".into(), "b".into()]).unwrap());
    let mut machine = Playback::new();
    assert!(machine.play_chord(Role::Player, &book).started);
    assert_eq!(buttons(&mut machine, Role::Player, &book), [1, 0, 0, 2, 1, 0, 0, 2]);
}

#[test]
fn an_empty_trim_window_finishes_without_spinning() {
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some("m"), 1).unwrap();
    index.set_loop(Role::Player, LoopCount::Infinite);
    let mut book = Book::new(index);
    book.add_recording("m", recording("Idle", &[0, 0x8000], Trim::Both));
    let mut machine = Playback::new();
    machine.play_chord(Role::Player, &book);
    assert!(machine.step(Role::Player, &book).is_none());
    assert!(!machine.playing(Role::Player));
}

#[test]
fn playback_slot_copies_the_trim_window() {
    let _host = HostGuard::new();
    let mut input = vec![0; 100];
    input.extend([1; 500]);
    input.extend([0; 100]);
    let recording = recording("Long", &input, Trim::Both);
    let result = host::set_slot(2, Some(&recording));
    assert!(result.is_ok(), "600-frame limit applies to the trimmed window: {result:?}");
    let start = host::slot_start_mash(2, host::slot_take(2).unwrap(), 1).unwrap().unwrap();
    assert_eq!(start.frames, 500);
    assert_eq!(host::slot_control(false).unwrap().buttons, 1);
}

#[test]
fn crop_preview_addresses_the_whole_file() {
    let mut book = Book::new(RecordingIndex::fresh());
    book.add_recording("m", recording("Macro", &[0, 1, 2, 0], Trim::Both));
    let mut machine = Playback::new();
    assert!(machine.play_range(Role::Player, &book, "m", 0..4).started);
    assert_eq!(buttons(&mut machine, Role::Player, &book), [0, 1, 2, 0]);
}

fn saved_marked_capture(root: &Path, frames: &[InputFrame], spans: &[Span], trim: Trim) -> MacroRecording {
    let mut capture = rr_runtime::Capture::idle();
    rr_runtime::toggle_record(&mut capture, Role::Player);
    for &input in frames { rr_runtime::push_frame(&mut capture, Role::Player, input); }
    capture.set_spans(spans);
    let stop = rr_runtime::finish_recording(root, &capture, state(8), state(7), (0.0, 0.0), 1, trim).unwrap();
    Library::new(root.join(MACROS_DIR)).load(stop.cursor().unwrap()).unwrap()
}

fn marked_frames(include_open: bool) -> (Vec<InputFrame>, [Span; 2]) {
    let mut frames = vec![frame(0); 200];
    frames[10] = frame(1);
    if include_open { frames[50] = frame(1); }
    frames[90] = frame(2);
    let spans = [
        Span { start: 12, end: Some(40), mv: Some(Move::Jab), ..Span::default() },
        Span { start: 92, end: Some(150), mv: Some(Move::SideTilt), ..Span::default() },
    ];
    (frames, spans)
}

#[test]
fn record_time_trim_moves_the_marks_with_the_kept_frames() {
    let root = Scratch::new();
    let (frames, later_spans) = marked_frames(true);
    let spans = [Span { start: 2, end: Some(15), mv: Some(Move::UpTilt), ..Span::default() }, later_spans[0], later_spans[1]];
    for (trim, range, marks) in [
        (Trim::Start, 10..200, vec![(0, Some(30)), (40, None), (80, Some(140))]),
        (Trim::End, 0..91, vec![(2, Some(15)), (10, Some(40)), (50, None), (90, Some(91))]),
        (Trim::Both, 10..91, vec![(0, Some(30)), (40, None), (80, Some(81))]),
        (Trim::Off, 0..200, vec![(2, Some(15)), (10, Some(40)), (50, None), (90, Some(150))]),
    ] {
        let saved = saved_marked_capture(&root.0, &frames, &spans, trim);
        assert_eq!(saved.frames, frames[range]);
        assert_eq!(saved.actions.iter().map(|mark| (mark.start, mark.end)).collect::<Vec<_>>(), marks);
    }
}

#[test]
fn end_trim_gives_the_marks_the_editor_already_had() {
    let root = Scratch::new();
    let (frames, spans) = marked_frames(false);
    for (trim, range, marks) in [
        (Trim::End, 0..91, vec![(10, Some(40)), (90, Some(91))]),
        (Trim::Off, 0..200, vec![(10, Some(40)), (90, Some(150))]),
        (Trim::Start, 10..200, vec![(0, Some(30)), (80, Some(140))]),
    ] {
        let saved = saved_marked_capture(&root.0, &frames, &spans, trim);
        assert_eq!(saved.frames, frames[range]);
        assert_eq!(saved.actions.iter().map(|mark| (mark.start, mark.end)).collect::<Vec<_>>(), marks);
    }
}

#[test]
fn a_crop_preview_shows_the_playback_indicator_for_its_role() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let (frames, spans) = marked_frames(false);
    let saved = saved_marked_capture(&root.0, &frames, &spans, Trim::Start);
    assert_eq!(saved.actions[0].start, 0);
    assert_eq!(saved.actions[0].end, Some(30));
    let id = Library::new(root.0.join(MACROS_DIR))
        .save_new(&recording("Crop", &[0, 1, 2, 0], Trim::Both)).unwrap();
    for (role, expected) in [(Role::Player, [true, false]), (Role::Cpu, [false, true])] {
        host::request_preview(&id, role, 0, 4);
        tick(&root.0);
        assert_eq!(host::playback_roles(), expected);
        for _ in 0..8 {
            if role == Role::Player { host::take_player_frame(); } else { host::take_cpu_frame(); }
            tick(&root.0);
        }
        assert!(!host::preview_running());
        assert_eq!(host::playback_roles(), [false, false]);
    }
}

#[test]
fn a_crop_saved_as_a_new_macro_has_trim_off() {
    let root = Scratch::new();
    let library = Library::new(root.0.join(MACROS_DIR));
    let original = recording("Original", &[0, 1, 2, 0], Trim::Both);
    let original_id = library.save_new(&original).unwrap();
    let mut editor = ClipEditor::macro_file(library.load(&original_id).unwrap(), true).unwrap();
    let ClipEffect::Export(crop) = editor.on_action(ClipAction::Confirm) else { panic!("crop export") };
    let id = rr_runtime::save_clip_file(&root.0, rr_runtime::ClipFile::Macro(crop), "Crop", None).unwrap();
    let saved = library.load(&id).unwrap();
    assert_eq!(saved.trim, Trim::Off);
    assert_eq!(saved.frames, original.frames);
    assert_eq!(library.load(&original_id).unwrap().trim, Trim::Both);
}

#[test]
fn drill_preview_uses_the_takes_trim() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let id = macros.save_new(&recording("Drill take 0000000000000002", &[0, 1, 2, 0], Trim::Off)).unwrap();
    tick(&root.0);
    assert!(host::request_drill_preview(&id, 4, Trim::Both));
    let mut played = Vec::new();
    for _ in 0..5 {
        tick(&root.0);
        if let Some(frame) = host::take_player_frame() { played.push(frame.buttons); }
    }
    assert_eq!(played, [1, 2]);

    host::request_preview(&id, Role::Player, 0, 3);
    let mut partial = Vec::new();
    for _ in 0..5 {
        tick(&root.0);
        if let Some(frame) = host::take_player_frame() { partial.push(frame.buttons); }
    }
    assert_eq!(partial, [0, 1, 2]);

    let plain = macros.save_new(&recording("Ordinary macro", &[0, 1, 2, 0], Trim::Both)).unwrap();
    host::request_preview(&plain, Role::Player, 0, 4);
    let mut played = Vec::new();
    for _ in 0..5 {
        tick(&root.0);
        if let Some(frame) = host::take_player_frame() { played.push(frame.buttons); }
    }
    assert_eq!(played, [0, 1, 2, 0]);
}

fn played_preview(root: &Path) -> Vec<u32> {
    let mut out = Vec::new();
    for _ in 0..12 {
        tick(root);
        if let Some(frame) = host::take_player_frame() { out.push(frame.buttons); }
    }
    assert!(!host::preview_running());
    out
}

fn play_selected_drill(root: &Path, name: &str) -> Vec<u32> {
    let mut cache = rr_runtime::CatalogCache::new(root);
    cache.want(0);
    while cache.step() {}
    let mut screens = RecordingScreens::new();
    rr_runtime::apply_catalog(cache.ready(0).unwrap(), &RecordingIndex::load(root).unwrap(), &mut screens);
    open(&mut screens, COMBOS_SEQ, 1, "DRILLS >");
    let items = screens.section(Face::Page, COMBOS_SEQ, 1).rows;
    screens.set_cursor(Face::Page, COMBOS_SEQ, 1, items.iter().position(|row| row.name == name).unwrap());
    screens.handle_page(Button::X, true, COMBOS_SEQ);
    let Effect::PreviewDrill { source, frames, trim } = screens.take_effect() else { panic!("missing X preview") };
    assert!(host::request_drill_preview(&source, frames, trim));
    played_preview(root)
}

#[test]
fn rv_trim_shared_take_uses_the_selected_drills_setting() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let take = macros.save_new(&recording("Drill take 0000000000000002", &[0, 1, 2, 0], Trim::Off)).unwrap();
    let drills = ComboLibrary::new(root.0.join("combos"));
    let a = drills.save_new(&combo(true, "First").with_take(&take)).unwrap();
    let b = drills.save_new(&combo(true, "Second").with_take(&take)).unwrap();
    tick(&root.0);
    for (id, trim) in [(a, Trim::Start), (b, Trim::End)] {
        host::post_effect(Effect::SetTrim { id, kind: FileKind::Combo, trim });
        tick(&root.0);
    }
    assert_eq!(play_selected_drill(&root.0, "First"), [1, 2, 0]);
    assert_eq!(play_selected_drill(&root.0, "Second"), [0, 1, 2]);
    assert_eq!(macros.load(&take).unwrap().trim, Trim::Off);
}

#[test]
fn rv_trim_drill_does_not_modify_referenced_macro() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let macros = Library::new(root.0.join(MACROS_DIR));
    let take = macros.save_new(&recording("Ordinary macro", &[0, 1, 0], Trim::Off)).unwrap();
    let id = ComboLibrary::new(root.0.join("combos")).save_new(&combo(true, "Copied").with_take(&take)).unwrap();
    let path = root.0.join(MACROS_DIR).join(format!("{take}.json"));
    let before = std::fs::read(&path).unwrap();
    host::post_effect(Effect::SetTrim { id, kind: FileKind::Combo, trim: Trim::Both });
    tick(&root.0);
    assert!(std::fs::read(path).unwrap() == before, "changing a drill rewrote an unrelated macro");
}

#[test]
fn rv_trim_full_editor_preview_ignores_take_like_name() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let id = Library::new(root.0.join(MACROS_DIR))
        .save_new(&recording("Drill take 0000000000000002", &[0, 1, 2, 0], Trim::Both)).unwrap();
    tick(&root.0);
    host::request_preview(&id, Role::Player, 0, 4);
    assert_eq!(played_preview(&root.0), [0, 1, 2, 0]);
}

#[test]
fn playback_indicator_shows_the_playing_role() {
    for (playing, text, color) in [
        ([true, false], "PLAY P1", hud_row::PLAYER_COLOR),
        ([false, true], "PLAY CPU", hud_row::CPU_COLOR),
        ([true, true], "PLAY P1 + CPU", hud_row::CPU_COLOR),
    ] {
        let row = hud_row::hud_row(None, playing, "");
        assert!(row.is_some(), "missing {text}");
        let row = row.unwrap();
        assert_eq!(row.text.as_str(), text);
        assert_eq!((row.color, row.chip, row.playing), (color, color, playing));
        let rect = hud_row::hud_row_box(&row, false, false);
        let crate::model::panes::Kind::Text { font, .. } =
            crate::model::panes::spec(crate::model::panes::IDX_BTT + 3).kind else { panic!("text pane") };
        assert!(crate::model::notice_box::measure(text.encode_utf16(), font.0).0 <= 0.9 * (rect[2] - 64.0));
        for (drop, panels) in [
            (crate::model::panes::LOBBY_DROP, vec![[-960.0, 447.0, -543.0, 540.0], [588.0, 450.0, 937.0, 522.0], [697.0, -30.0, 930.0, 184.0]]),
            (0.0, vec![[685.0, 12.0, 925.0, 331.0]]),
        ] {
            let [x, y, w, h] = rect;
            let chip = [x, y - h / 2.0 - drop, x + w, y + h / 2.0 - drop];
            for panel in panels {
                assert!(!(chip[0] < panel[2] && panel[0] < chip[2] && chip[1] < panel[3] && panel[1] < chip[3]), "{text}: {chip:?}");
            }
        }
    }
}

#[test]
fn playback_indicator_shares_the_row_with_rec() {
    let row = hud_row::hud_row(Some(Take::Cpu), [true, true], "10/10").unwrap();
    assert_eq!(row.text.as_str(), "REC CPU · PLAY P1 + CPU · 10/10");
    assert_eq!((row.color, row.chip), (hud_row::CPU_COLOR, hud_row::CPU_COLOR));
    assert_eq!(row.playing, [true, true]);
    let longest = hud_row::hud_row(Some(Take::Cpu), [true, true], "").unwrap();
    let rect = hud_row::hud_row_box(&longest, false, false);
    let crate::model::panes::Kind::Text { font, .. } =
        crate::model::panes::spec(crate::model::panes::IDX_BTT + 3).kind else { panic!("text pane") };
    assert!(crate::model::notice_box::measure(longest.text.as_str().encode_utf16(), font.0).0 <= 0.9 * (rect[2] - 64.0));
}

#[test]
fn playback_indicator_is_on_by_default() {
    assert_eq!(Snapshot::factory().get("playback_indicator"), Some(&[1, 0][..]));
    assert!(crate::model::recording::RecorderSettings::factory().indicator);
    let crop = ROWS.iter().find(|row| row.id == "recording_crop").unwrap();
    let row = ROWS.iter().find(|row| row.id == "playback_indicator").unwrap();
    assert_eq!(row.group, crop.group);
    assert!(matches!(row.control, Control::Toggle));
}

#[test]
fn playback_indicator_off_shows_nothing() {
    let on = crate::model::recording::RecorderSettings::factory();
    assert!(on.indicator, "factory must admit the indicator");
    let off = settings::parse(r#"{"menu":{"playback_indicator":[0,1]}}"#);
    let option = crate::model::recording::RecorderSettings::from_snapshot(&off.settings.menu);
    assert!(!option.indicator);
    let playing = [true, true].map(|role| role && option.indicator);
    assert!(hud_row::hud_row(None, playing, "").is_none());
}

fn start_cpu_slot() {
    host::set_slot(2, Some(&recording("Slot", &[1, 2], Trim::Off))).unwrap();
    assert!(host::slot_start_mash(2, host::slot_take(2).unwrap(), 1).unwrap().is_some());
}

#[test]
fn playback_indicator_clears_when_playback_ends() {
    let _host = HostGuard::new();
    start_cpu_slot();
    assert_eq!(host::playback_roles(), [false, true]);
    assert!(host::slot_control(false).is_some());
    host::slot_post();
    assert!(host::slot_control(false).is_some());
    host::slot_post();
    assert_eq!(host::playback_roles(), [false, false]);
    start_cpu_slot();
    assert_eq!(host::playback_roles(), [false, true]);
    host::slot_stop();
    assert_eq!(host::playback_roles(), [false, false]);
}

#[test]
fn playback_roles_track_player_cpu_and_both() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let id = Library::new(root.0.join(MACROS_DIR))
        .save_new(&recording("Both", &[1, 2, 3], Trim::Off)).unwrap();
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some(&id), 1).unwrap();
    index.set_active(Role::Cpu, Some(&id), 1).unwrap();
    index.save(&root.0).unwrap();
    tick(&root.0);
    host::on_chords(false, true, false);
    tick(&root.0);
    assert!(host::player_playback_blocks_preview(), "the Player run reached its live tail");
    assert_eq!(host::playback_roles(), [true, false]);
    host::on_chords(false, false, true);
    tick(&root.0);
    assert!(host::cpu_playback(false).running, "the CPU run reached its live tail");
    assert_eq!(host::playback_roles(), [true, true]);
    while host::take_player_frame().is_some() {}
    assert_eq!(host::playback_roles(), [false, true]);
    while host::take_cpu_frame().is_some() {}
    assert_eq!(host::playback_roles(), [false, false]);
}

#[test]
fn playback_indicator_clears_on_a_training_reset() {
    let _host = HostGuard::new();
    let root = Scratch::new();
    let id = Library::new(root.0.join(MACROS_DIR)).save_new(&recording("CPU", &[1, 2, 3], Trim::Off)).unwrap();
    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Cpu, Some(&id), 1).unwrap();
    index.save(&root.0).unwrap();
    tick(&root.0);
    host::on_chords(false, false, true);
    tick(&root.0);
    assert!(host::cpu_playback(false).running, "the CPU run reached its live tail");
    assert_eq!(host::playback_roles(), [false, true]);
    host::clear_playback(crate::model::input_delay::Clear::TrainingReset);
    assert_eq!(host::playback_roles(), [false, false]);
    tick(&root.0);
    host::on_chords(false, false, true);
    tick(&root.0);
    assert_eq!(host::playback_roles(), [false, true]);
    host::note_load();
    assert_eq!(host::playback_roles(), [false, false]);
}

#[test]
fn playback_indicator_is_one_path_for_training_and_the_waiting_room() {
    let source = include_str!("../src/runtime/features/hud_draw.rs");
    let start = source.find("pub unsafe fn draw(").unwrap();
    let body = &source[start..start + source[start..].find("\n}\n").unwrap()];
    let row = body.find("hud_row::hud_row(").unwrap();
    assert!(row > body.find("if !waiting {").unwrap());
    assert!(body.contains("rr_host::playback_roles()"));
    assert!(body.contains("(!menu_open).then(|| hud_row::hud_row("));
}

#[test]
fn bridge_reports_the_indicator_the_hud_drew() {
    use crate::model::bridge_state::{playback_indicator, playback_indicator_stamp, snapshot, View};
    for (playing, expected) in [
        ([true, false], "player"),
        ([false, true], "cpu"),
        ([true, true], "both"),
        ([false, false], ""),
    ] {
        let drawn = hud_row::hud_row(None, playing, "").map_or([false; 2], |row| row.playing);
        let stamp = playback_indicator_stamp(100, drawn);
        assert_eq!(playback_indicator(stamp, 100, 50), expected);
        assert_eq!(playback_indicator(stamp, 151, 50), "");
        let mut text = String::new();
        let view = View { playback_indicator: playback_indicator(stamp, 100, 50), ..View::default() };
        assert!(snapshot(&view, &mut text));
        assert!(text.contains(&format!("playback.indicator=\"{expected}\"\n")));
    }
    let hidden = playback_indicator_stamp(100, [false; 2]);
    assert_eq!(playback_indicator(hidden, 100, 50), "");
    let mut text = String::new();
    assert!(snapshot(&View::default(), &mut text));
    assert!(text.contains("playback.indicator=\"\""));
    let worker = include_str!("../src/runtime/ui/worker.rs");
    assert!(worker.contains("recording_indicator: crate::runtime::bridge_state::recording_indicator(now),\n            playback_indicator: crate::runtime::bridge_state::playback_indicator(now),"));
    let runtime = include_str!("../src/runtime/bridge_state.rs");
    assert!(runtime.contains("m::playback_indicator(PLAYBACK_INDICATOR.load(Ordering::Relaxed), now_ms, SCENE_FRESH_MS)"));
}
