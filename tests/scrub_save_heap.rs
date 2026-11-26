use crate::engine::library::{Chain, Role, MACROS_DIR};
use crate::engine::macro_ui::{clip_panel::{ClipAction, ClipEditor, ClipEffect}, scrub_track::{Pos, TakeTrack, TAKE_CHUNK}};
use crate::engine::macros::{InputFrame, Library, ReplayFrame, ReplaySession, ReplayTrack, StartState, MAX_FRAMES};
use crate::engine::rr_runtime::{finish_recording, push_frame, toggle_record, Capture, RecordStop};
use crate::model::{hook_stats::heap_bytes, state::record::SavedState, trim::Trim};
use std::{env, fs, io::Write, process::Command, time::Instant};

#[test]
fn a_maximum_take_keeps_its_inputs_and_first_two_minutes_of_positions() {
    const TEST: &str = "scrub_save_heap::a_maximum_take_keeps_its_inputs_and_first_two_minutes_of_positions";
    const KEPT: usize = 7200;
    const KEY: &str = "TR_SCRUB_SAVE_HEAP_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        // The production allocator's peak is process-wide: run just this case in a fresh process.
        let root = env::temp_dir().join(format!("tr-scrub-save-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let run = |phase| Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"])
            .env(KEY, &root).env("TR_SCRUB_PHASE", phase).status().unwrap().success();
        let saved = run("save");
        let loaded = root.join("measured").is_file() && run("load");
        let listed = root.join("measured").is_file() && run("list");
        let catalog = root.join("measured").is_file() && run("catalog");
        fs::remove_dir_all(root).unwrap();
        assert!(saved && loaded && listed && catalog, "isolated checks: save={saved}, load={loaded}, list={listed}, catalog={catalog}");
        return;
    };

    let folder = root.join(MACROS_DIR);
    let phase = env::var("TR_SCRUB_PHASE").unwrap();
    if matches!(phase.as_str(), "list" | "catalog") {
        let id = fs::read_to_string(root.join("measured")).unwrap();
        let library = Library::new(&folder);
        let start = Instant::now();
        let before = heap_bytes();
        let count = if phase == "list" {
            let listing = library.list().unwrap();
            assert!(listing.skipped.is_empty());
            assert_eq!(listing.entries.len(), 1);
            listing.entries[0].frame_count
        } else {
            library.load_with_sibling::<Chain>(&id).unwrap().unwrap().unwrap().frames.len()
        };
        let elapsed = start.elapsed();
        let after = heap_bytes();
        let peak = after.peak - before.live;
        writeln!(std::io::stderr(), "SCRUB_LIST path={phase} count={count} peak_above_idle={peak} largest={} time_us={}", after.largest, elapsed.as_micros()).unwrap();
        assert_eq!(count, MAX_FRAMES);
        let file_bytes = fs::metadata(folder.join(format!("{id}.json"))).unwrap().len() as isize;
        assert!(peak < file_bytes + 1024 * 1024, "the directory needs one file buffer and its metadata, not a parsed JSON tree or position track");
        return;
    }
    if env::var("TR_SCRUB_PHASE").as_deref() == Ok("load") {
        let id = fs::read_to_string(root.join("measured")).unwrap();
        let library = Library::new(&folder);
        let before = heap_bytes();
        let mut rec = library.load(&id).unwrap();
        let load_peak = heap_bytes().peak - before.live;
        assert_eq!(rec.track.as_ref().unwrap().len(), KEPT);
        rec.name.push_str(" revised");
        let before = heap_bytes();
        library.overwrite(&id, &rec).unwrap();
        let overwrite_alloc = heap_bytes().total - before.total;
        assert_eq!(fs::read(folder.join(format!("{id}.json"))).unwrap(), serde_json::to_vec(&rec).unwrap());
        writeln!(std::io::stderr(), "SCRUB_LOAD frames={MAX_FRAMES} peak_above_idle={load_peak} overwrite_alloc={overwrite_alloc}").unwrap();
        return;
    }

    let baseline = heap_bytes();
    let take = TakeTrack::new();
    let mut capture = Capture::idle();
    toggle_record(&mut capture, Role::Player);
    let body = |kind, sign, i: usize| ReplayFrame { input: InputFrame::default(), start: StartState {
        situation: 2, status: 14, speed_x: sign * 0.8125, speed_y: -0.4375,
        ..StartState::neutral(kind, 2, sign)
    }, x: sign * (i % 1600) as f32 / 16.0, y: (i % 768) as f32 / 16.0 };
    for i in 0..MAX_FRAMES {
        if i % TAKE_CHUNK == 0 { take.service(true, i); }
        take.live(0, &body(8, -1.0, i));
        take.live(1, &body(18, 1.0, i));
        push_frame(&mut capture, Role::Player, InputFrame { buttons: if i + 1 == MAX_FRAMES { 7 } else { 513 },
            lstick_x: -85, ..InputFrame::default() });
        take.push(i, 0);
    }
    let track = take.take(MAX_FRAMES).unwrap();
    let track_bytes = track.capacity() * size_of::<[ReplayFrame; 2]>();
    capture.set_track(Some(track));
    let saved = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let stop = finish_recording(&root, &capture, saved(8), saved(18), (0.0, 0.0), 1, Trim::Off).unwrap();
    let peak = heap_bytes().peak - baseline.live;
    let RecordStop::Saved { id, .. } = stop else { panic!("maximum take was not saved") };

    // Reopen via the store, not just serde. Everything from here is outside the measured save peak.
    let rec = Library::new(&folder).load(&id).unwrap();
    assert_eq!(rec.frames.len(), MAX_FRAMES);
    assert_eq!(rec.frames[MAX_FRAMES - 1].buttons, 7);
    let stored = rec.track.as_ref().unwrap();
    assert_eq!(stored[KEPT - 1], [body(8, -1.0, KEPT - 1), body(18, 1.0, KEPT - 1)]);
    let bytes = fs::read(folder.join(format!("{id}.json"))).unwrap();
    let encoded = serde_json::to_vec(&rec).unwrap();
    assert_eq!(bytes.as_slice(), encoded.as_slice());
    let before = heap_bytes();
    serde_json::to_writer(std::io::sink(), &rec).unwrap();
    let scratch = heap_bytes().total - before.total;
    writeln!(std::io::stderr(), "SCRUB_SAVE frames={MAX_FRAMES} track={track_bytes} json={} json_capacity={} peak_above_idle={peak} serializer_scratch={scratch}",
        bytes.len(), encoded.capacity()).unwrap();
    fs::write(root.join("measured"), id).unwrap();
    assert_eq!(stored.len(), KEPT, "only new positions are capped");
    let mut editor = ClipEditor::macro_file(rec, true).unwrap();
    editor.on_action(ClipAction::SelectOut);
    assert_eq!(editor.scrub().1, Pos::End);
}

#[test]
fn vault_itself_is_uncapped_but_a_new_macro_keeps_two_minutes_from_in() {
    let tracks = (0..2).map(|entry| ReplayTrack { fighter: None, entry, quantized_analog: false,
        frames: (0..7210).map(|i| ReplayFrame { input: InputFrame { buttons: i, ..Default::default() },
            start: StartState { status: 14, ..StartState::neutral(18, 0, 1.0) },
            x: i as f32, y: entry as f32 }).collect() }).collect();
    let mut editor = ClipEditor::new(ReplaySession { tracks, ..Default::default() }, vec![14]).unwrap();
    editor.on_action(ClipAction::SelectOut);
    assert_eq!(editor.scrub().1, Pos::Held);
    editor.on_action(ClipAction::SelectIn);
    editor.on_action(ClipAction::Nudge(3));
    let ClipEffect::Export(rec) = editor.on_action(ClipAction::Confirm) else { panic!("expected capture export") };
    assert_eq!(rec.frames.len(), 7207);
    let track = rec.track.unwrap();
    assert_eq!([track[0][0].x, track[7199][1].x], [3.0, 7202.0]);
    assert_eq!(track.len(), 7200, "the capture export starts its own position prefix at In");
}
