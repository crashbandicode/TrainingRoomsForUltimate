use crate::engine::library::{Role, MACROS_DIR};
use crate::engine::macro_ui::scrub_track::{TakeTrack, TAKE_CHUNK};
use crate::engine::macros::{InputFrame, Library, ReplayFrame, StartState, MAX_FRAMES};
use crate::engine::rr_runtime::{finish_recording, push_frame, toggle_record, Capture, RecordStop};
use crate::model::{hook_stats::heap_bytes, state::record::SavedState, trim::Trim};
use std::{env, fs, io::Write, process::Command};

#[test]
fn a_maximum_take_serializes_without_a_second_track() {
    const TEST: &str = "scrub_save_heap::a_maximum_take_serializes_without_a_second_track";
    const KEY: &str = "TR_SCRUB_SAVE_HEAP_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        // The production allocator's peak is process-wide: run just this case in a fresh process.
        let root = env::temp_dir().join(format!("tr-scrub-save-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let status = Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"])
            .env(KEY, &root).status();
        let completed = root.join("measured").is_file();
        fs::remove_dir_all(root).unwrap();
        assert!(status.unwrap().success() && completed, "isolated save measurement did not pass");
        return;
    };

    let baseline = heap_bytes();
    let take = TakeTrack::new();
    let mut inputs = Vec::with_capacity(MAX_FRAMES);
    let body = |kind, sign, i: usize| ReplayFrame { input: InputFrame::default(), start: StartState {
        situation: 2, status: 14, speed_x: sign * 0.8125, speed_y: -0.4375,
        ..StartState::neutral(kind, 2, sign)
    }, x: sign * (i % 1600) as f32 / 16.0, y: (i % 768) as f32 / 16.0 };
    for i in 0..MAX_FRAMES {
        if i % TAKE_CHUNK == 0 { take.service(true, i); }
        take.live(0, &body(8, -1.0, i));
        take.live(1, &body(18, 1.0, i));
        inputs.push(InputFrame { buttons: 513, lstick_x: -85, ..InputFrame::default() });
        take.push(i, 0);
    }
    let track = take.take(MAX_FRAMES).unwrap();
    assert_eq!(track.len(), MAX_FRAMES);
    let track_bytes = track.capacity() * size_of::<[ReplayFrame; 2]>();
    let mut capture = Capture::idle();
    toggle_record(&mut capture, Role::Player);
    capture.set_track(Some(track));
    for input in inputs { push_frame(&mut capture, Role::Player, input); }
    let saved = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let stop = finish_recording(&root, &capture, saved(8), saved(18), (0.0, 0.0), 1, Trim::Off).unwrap();
    let peak = heap_bytes().peak - baseline.live;
    let RecordStop::Saved { id, .. } = stop else { panic!("maximum take was not saved") };

    // Reopen via the store, not just serde. Everything from here is outside the measured save peak.
    let folder = root.join(MACROS_DIR);
    let rec = Library::new(&folder).load(&id).unwrap();
    assert_eq!(rec.frames.len(), MAX_FRAMES);
    let stored = rec.track.as_ref().unwrap();
    assert_eq!(stored.len(), MAX_FRAMES);
    assert_eq!(stored[MAX_FRAMES - 1], [body(8, -1.0, MAX_FRAMES - 1), body(18, 1.0, MAX_FRAMES - 1)]);
    let bytes = fs::read(folder.join(format!("{id}.json"))).unwrap();
    let encoded = serde_json::to_vec(&rec).unwrap();
    assert_eq!(bytes.as_slice(), encoded.as_slice());
    let before = heap_bytes();
    serde_json::to_writer(std::io::sink(), &rec).unwrap();
    let scratch = heap_bytes().total - before.total;
    writeln!(std::io::stderr(), "SCRUB_SAVE frames={MAX_FRAMES} track={track_bytes} json={} json_capacity={} peak_above_idle={peak} serializer_scratch={scratch}",
        bytes.len(), encoded.capacity()).unwrap();
    assert!(scratch < 4096, "serializing to a nonallocating writer requested {scratch} bytes");
    fs::write(root.join("measured"), b"ok").unwrap();
}
