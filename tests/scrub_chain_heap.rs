use crate::engine::library::{Chain, LoopCount, RecordingIndex, Role, MACROS_DIR};
use crate::engine::macros::{InputFrame, Library, MacroRecording, ReplayFrame, StartState};
use crate::engine::rr_host as host;
use crate::model::{hook_stats::heap_bytes, state::record::SavedState, store::Store};
use std::{env, fs, io::Write, process::Command};

const STEPS: usize = 6;
const FRAMES: usize = 7_200;

/// tr-primary-2 2026-11-26 02:30:57Z and 03:06:35Z: playback never needs positions, so a chain's start must not
/// read its steps' positions files. Six distinct tracked steps of two minutes each, started through the host's own service tick.
#[test]
fn a_chain_start_does_not_build_the_steps_position_tracks() {
    const TEST: &str = "scrub_chain_heap::a_chain_start_does_not_build_the_steps_position_tracks";
    const KEY: &str = "TR_SCRUB_CHAIN_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        // The allocator's peak is process-wide: write the files in one fresh process and start the chain in another.
        let root = crate::test_scratch::Scratch::new("tr-scrub-chain");
        let run = |phase| Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"])
            .env(KEY, root.path()).env("TR_SCRUB_CHAIN_PHASE", phase).status().unwrap().success();
        let made = run("make");
        let started = made && run("start");
        fs::remove_dir_all(root).unwrap();
        assert!(made && started, "isolated checks: make={made}, start={started}");
        return;
    };
    let library = Library::new(root.join(MACROS_DIR));
    let tracks = (STEPS * FRAMES * size_of::<[ReplayFrame; 2]>()) as isize;
    if env::var("TR_SCRUB_CHAIN_PHASE").as_deref() == Ok("make") {
        let body = |kind, i: usize| ReplayFrame { pose: crate::engine::macros::PackedPose::NONE, input: InputFrame::default(),
            start: StartState { status: 14, ..StartState::neutral(kind, 0, 1.0) }, x: (i % 1600) as f32 / 16.0, y: 0.0 };
        let mut ids = Vec::new();
        for step in 0..STEPS {
            let frames = (0..FRAMES).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
            let mut rec = MacroRecording::new(format!("Step {step}"), "13.0.5", StartState::neutral(8, 0, 1.0), frames).unwrap();
            rec.set_track(Some((0..FRAMES).map(|i| [body(8, i), body(7, i)]).collect()));
            ids.push(library.save_new(&rec).unwrap());
        }
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Chain", ids.clone()).unwrap()).unwrap();
        crate::engine::rr_runtime::migrate_at_boot(&root, &[None, None, None, None, None], "2026-11-26").unwrap();
        let mut index = RecordingIndex::load(&root).unwrap();
        index.set_active(Role::Player, Some(&chain), 1).unwrap();
        index.set_loop(Role::Player, LoopCount::Finite(1));
        index.save(&root).unwrap();
        fs::write(root.join("last-step"), &ids[STEPS - 1]).unwrap();
        return;
    }
    let state = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let before = heap_bytes();
    let reads = crate::engine::macros::positions::reads();
    host::reset_host();
    let tick = || host::service_at(&root, &[None, None, None, None, None], "2026-11-26", state(8), state(7));
    tick();
    host::on_chords(false, true, false);
    tick();
    let after = heap_bytes();
    let peak = after.peak - before.live;
    let started = host::playback_roles()[0];
    let first = host::take_player_frame().map(|frame| frame.buttons);
    writeln!(std::io::stderr(), "SCRUB_CHAIN steps={STEPS} frames={FRAMES} tracks={tracks} peak_above_idle={peak} largest={} started={started} first={first:?}",
        after.largest).unwrap();
    assert!(started || first.is_some(), "the chain did not start");
    assert!(peak < tracks / 2, "the chain start built position tracks: peak {peak} B against {tracks} B of tracks");
    assert_eq!(crate::engine::macros::positions::reads(), reads, "a chain start opens no positions file");
    // The positions are beside the macros: playback's load has none, the editor's load reads them.
    let last = fs::read_to_string(root.join("last-step")).unwrap();
    assert_eq!(library.load(&last).map(|rec| (rec.frames.len(), rec.track.is_none())).unwrap(), (FRAMES, true));
    assert_eq!(library.load_edited(&last).unwrap().track.map(|track| track.len()), Some(FRAMES));
    host::reset_host();
}
