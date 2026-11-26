use crate::engine::library::{Chain, LoopCount, RecordingIndex, Role, MACROS_DIR};
use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState};
use crate::engine::rr_host as host;
use crate::model::{state::record::SavedState, store::Store};
use std::{env, fs, path::Path, process::Command, time::Instant};

const RUNS: usize = 21;

/// The operator's question (2026-11-26 03:02:02Z): is Play slower with positions? Host timings of the Play chord's
/// service tick up to the first injected input, of a selection load and of a chain start, for a 3 s, a 30 s and a
/// 36,000-input macro. The same file runs on `main` without the two lines marked head-only. It asserts only that
/// each run starts; the numbers are read from its output.
#[test]
fn play_start_timings() {
    const TEST: &str = "scrub_play_timing::play_start_timings";
    const KEY: &str = "TR_SCRUB_TIMING_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        // The host's playback state is process-wide: time it in a fresh process.
        let root = env::temp_dir().join(format!("tr-scrub-timing-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        // One core for the whole timed process, the same on every run (tr-primary-2 2026-11-26 04:35:21Z): the host
        // otherwise gives a process one of two speeds for its life. Without taskset the run is not pinned and says so.
        let exe = env::current_exe().unwrap();
        let pinned = Path::new("/usr/bin/taskset").exists();
        let mut child = if pinned { let mut c = Command::new("/usr/bin/taskset"); c.args(["-c", "3"]).arg(&exe); c } else { Command::new(&exe) };
        let ran = child.args(["--exact", TEST, "--nocapture"]).env(KEY, &root).env("TR_SCRUB_TIMING_PINNED", if pinned { "cpu3" } else { "no" }).status().unwrap().success();
        fs::remove_dir_all(root).unwrap();
        assert!(ran, "the timed process failed");
        return;
    };
    let library = Library::new(root.join(MACROS_DIR));
    crate::engine::rr_runtime::migrate_at_boot(&root, &[None, None, None, None, None], "2026-11-26").unwrap();
    for frames in [180usize, 1_800, 36_000] {
        let make = |name: String| {
            let inputs = (0..frames).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
            #[allow(unused_mut)]
            let mut rec = MacroRecording::new(name, "13.0.5", StartState::neutral(8, 0, 1.0), inputs).unwrap();
            rec.set_track(Some(vec![[crate::engine::macros::ReplayFrame { input: InputFrame::default(), start: StartState::neutral(8, 0, 1.0), x: 1.0, y: 2.0 }; 2]; frames])); // head-only
            library.save_new(&rec).unwrap()
        };
        let ids: Vec<String> = (0..3).map(|step| make(format!("Timing {frames} {step}"))).collect();
        let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new(format!("Chain {frames}"), ids.clone()).unwrap()).unwrap();
        let json = fs::metadata(root.join(MACROS_DIR).join(format!("{}.json", ids[0]))).unwrap().len();
        report("load", frames, json, || { assert_eq!(library.load(&ids[0]).unwrap().frames.len(), frames); });
        report("play", frames, json, || start(&root, &ids[0]));
        report("chain3", frames, json, || start(&root, &chain));
    }
}

/// Make `id` Player 1's active file, then time the chord's service tick up to the first input it hands out.
fn start(root: &Path, id: &str) {
    let mut index = RecordingIndex::load(root).unwrap();
    index.set_active(Role::Player, Some(id), 1).unwrap();
    index.set_loop(Role::Player, LoopCount::Finite(1));
    index.save(root).unwrap();
    host::reset_host();
    let state = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let tick = || host::service_at(root, &[None, None, None, None, None], "2026-11-26", state(8), state(7));
    tick();
    warm();
    TIMED.with(|timed| timed.set(Some(Instant::now())));
    host::on_chords(false, true, false);
    tick();
    assert_eq!(host::take_player_frame().map(|frame| frame.buttons), Some(1), "the first input after the chord");
}

/// Keep the core busy for 5 ms just before a timed span. The spans are a fraction of a millisecond and follow synced
/// file writes (the fixture, the index save); a core that has just slept through one starts the span slower, and
/// by a different amount on each run. A Play press in a game finds a busy core.
fn warm() {
    let began = Instant::now();
    while began.elapsed().as_micros() < 5_000 {
        std::hint::spin_loop();
    }
}

thread_local! {
    static TIMED: core::cell::Cell<Option<Instant>> = const { core::cell::Cell::new(None) };
}

/// Run `what` `RUNS` times and print the median and the fastest, in microseconds. `start` restarts the clock at its
/// chord, so its setup (the index write, the reset, the warm tick) is not counted.
fn report(name: &str, frames: usize, json: u64, mut what: impl FnMut()) {
    let mut times: Vec<u128> = (0..RUNS).map(|_| {
        warm();
        let began = Instant::now();
        TIMED.with(|timed| timed.set(None));
        what();
        TIMED.with(|timed| timed.take()).unwrap_or(began).elapsed().as_micros()
    }).collect();
    times.sort_unstable();
    eprintln!("PLAY_TIMING what={name} frames={frames} json_bytes={json} runs={RUNS} pinned={} median_us={} min_us={} max_us={}", env::var("TR_SCRUB_TIMING_PINNED").unwrap_or_default(),
        times[RUNS / 2], times[0], times[RUNS - 1]);
}
