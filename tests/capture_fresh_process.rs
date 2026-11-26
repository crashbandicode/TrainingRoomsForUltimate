use crate::engine::macros::{CaptureArchive, InputFrame, PackedPose, Pose, ReplayFrame, ReplaySession, ReplayTrack, StartState};
use std::{env, fs, path::Path, process::Command};

fn file_id(id: &str) -> String {
    id.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect()
}

fn session() -> ReplaySession {
    let frame = |n: u32, kind| ReplayFrame {
        input: InputFrame { buttons: n, ..InputFrame::default() },
        start: StartState::neutral(kind, 0, 1.0),
        x: n as f32,
        y: -(n as f32),
        pose: PackedPose::pack(Pose { motion: 100 + u64::from(n), frame: n as f32 + 0.5 }),
    };
    ReplaySession {
        tracks: vec![
            ReplayTrack { fighter: None, entry: 0, frames: vec![frame(1, 5), frame(2, 5), frame(3, 5)], quantized_analog: false },
            ReplayTrack { fighter: None, entry: 3, frames: vec![frame(4, 9)], quantized_analog: false },
        ],
        truncated: false,
        breaks: Vec::new(),
    }
}

fn save_legacy(root: &Path, session: &ReplaySession) {
    let header = serde_json::json!({
        "schema_version": 1, "replay_id": "fresh-old", "replay_name": "Old",
        "game_version": "13.0.5", "truncated": false,
        "tracks": [
            {"entry": 0, "frames": 3, "quantized_analog": false},
            {"entry": 3, "frames": 1, "quantized_analog": false}
        ]
    });
    let header = serde_json::to_vec(&header).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"TMRCAP01");
    bytes.extend_from_slice(&(header.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&header);
    for frame in session.tracks.iter().flat_map(|track| &track.frames) {
        let mut record = [0u8; 44];
        record[0..4].copy_from_slice(&frame.input.buttons.to_le_bytes());
        record[4] = frame.input.lstick_x as u8;
        record[5] = frame.input.lstick_y as u8;
        record[6] = frame.input.rstick_x as u8;
        record[7] = frame.input.rstick_y as u8;
        record[8..12].copy_from_slice(&frame.start.fighter_kind.to_le_bytes());
        record[12..16].copy_from_slice(&frame.start.costume.to_le_bytes());
        record[16..20].copy_from_slice(&frame.start.facing.to_le_bytes());
        record[20..24].copy_from_slice(&frame.start.situation.to_le_bytes());
        record[24..28].copy_from_slice(&frame.start.status.to_le_bytes());
        record[28..32].copy_from_slice(&frame.start.speed_x.to_le_bytes());
        record[32..36].copy_from_slice(&frame.start.speed_y.to_le_bytes());
        record[36..40].copy_from_slice(&frame.x.to_le_bytes());
        record[40..44].copy_from_slice(&frame.y.to_le_bytes());
        bytes.extend_from_slice(&record);
    }
    fs::write(root.join(format!("{}.tmrc", file_id("fresh-old"))), bytes).unwrap();
}

#[test]
fn a_fresh_process_lists_and_loads_both_capture_schemas() {
    const TEST: &str = "capture_fresh_process::a_fresh_process_lists_and_loads_both_capture_schemas";
    const ROOT: &str = "TR_CAPTURE_FRESH_ROOT";
    const PHASE: &str = "TR_CAPTURE_FRESH_PHASE";
    let Some(root) = env::var_os(ROOT).map(std::path::PathBuf::from) else {
        let root = crate::test_scratch::Scratch::new("tr-capture-fresh");
        for phase in ["save", "read"] {
            let status = Command::new(env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env(ROOT, root.path())
                .env(PHASE, phase)
                .status()
                .unwrap_or_else(|error| panic!("{phase} phase could not start: {error}"));
            assert!(status.success(), "{phase} phase failed: {status}");
            eprintln!("CAPTURE_FRESH phase={phase} exited_successfully");
        }
        fs::remove_dir_all(root.path()).unwrap();
        return;
    };

    match env::var(PHASE).unwrap().as_str() {
        "save" => {
            let session = session();
            let outcome = CaptureArchive::new(&root)
                .save_with_miis("fresh-new", "New", "13.0.5", &session, None).unwrap();
            assert!(outcome.created);
            save_legacy(&root, &session);
            eprintln!("CAPTURE_FRESH phase=save pid={} wrote_both", std::process::id());
        }
        "read" => {
            let archive = CaptureArchive::new(&root);
            let listing = archive.list().unwrap();
            assert!(listing.skipped.is_empty(), "{:?}", listing.skipped);
            assert_eq!(listing.entries.len(), 2);
            for entry in &listing.entries {
                assert_eq!(entry.sides, [(0, 5), (3, 9)]);
                assert_eq!(entry.frames, 3);
            }
            let expected = session();
            for (id, has_pose) in [("fresh-new", true), ("fresh-old", false)] {
                let id = file_id(id);
                assert!(listing.entries.iter().any(|entry| entry.id == id));
                let loaded = archive.load(&id).unwrap();
                assert_eq!(loaded.session.tracks.len(), expected.tracks.len());
                for expected_track in &expected.tracks {
                    let track = loaded.session.tracks.iter().find(|track| track.entry == expected_track.entry).unwrap();
                    assert_eq!(track.frames.len(), expected_track.frames.len());
                    for (actual, expected) in track.frames.iter().zip(&expected_track.frames) {
                        assert_eq!(actual.start.fighter_kind, expected.start.fighter_kind);
                        assert_eq!(actual.x, expected.x);
                        assert_eq!(actual.y, expected.y);
                        assert_eq!(actual.input.buttons, expected.input.buttons);
                        assert_eq!(actual.pose, if has_pose { expected.pose } else { PackedPose::NONE });
                    }
                }
            }
            eprintln!("CAPTURE_FRESH phase=read pid={} listed_and_loaded_both", std::process::id());
        }
        phase => panic!("unknown capture phase: {phase}"),
    }
}
