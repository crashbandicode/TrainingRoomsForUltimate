use crate::engine::library::{Chain, LoopCount, RecordingIndex, Role, MACROS_DIR};
use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState};
use crate::engine::rr_host as host;
use crate::model::{state::record::SavedState, store::Store};
use std::{env, fs, path::Path, process::Command, time::{Duration, Instant}};

const RUNS: usize = 21;
const SIZES: [usize; 3] = [180, 1_800, 36_000];
const CPU: &str = "3";
const SIBLING: &str = "cpu7 ";

/// The operator's question (2026-11-26 03:02:02Z): is Play slower with positions? Host timings of the Play chord's
/// service tick up to the first injected input, of a selection load and of a chain start, for a 3 s, a 30 s and a
/// 36,000-input macro. The numbers are read from its output.
/// A trackless, generation-armed Play also reports its first cold sample separately from twenty warm samples.
///
/// The same file runs on `main` (tr-primary-2 04:35:21Z, 04:58:43Z): there the lines marked head-only are dropped
/// and the lines marked main-only are switched on, so both sides have the same files in the folder (a positions
/// file beside each macro, which `main` ignores), flush them, and wait the same time before the timed loop. Each
/// size is timed in a fresh process pinned to one core; each prints the host's load and that core's sibling.
#[test]
fn play_start_timings() {
    const TEST: &str = "scrub_play_timing::play_start_timings";
    const KEY: &str = "TR_SCRUB_TIMING_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        let exe = env::current_exe().unwrap();
        let pinned = Path::new("/usr/bin/taskset").exists();
        for frames in SIZES {
            let root = crate::test_scratch::Scratch::new(&format!("tr-scrub-timing-{frames}"));
            let mut child = if pinned { let mut c = Command::new("/usr/bin/taskset"); c.args(["-c", CPU]).arg(&exe); c } else { Command::new(&exe) };
            let ran = child.args(["--exact", TEST, "--nocapture"]).env(KEY, root.path()).env("TR_SCRUB_TIMING_FRAMES", frames.to_string())
                .env("TR_SCRUB_TIMING_PINNED", if pinned { "cpu3" } else { "no" }).status().unwrap().success();
            fs::remove_dir_all(root).unwrap();
            assert!(ran, "the timed process for {frames} frames failed");
        }
        return;
    };
    let frames: usize = env::var("TR_SCRUB_TIMING_FRAMES").unwrap().parse().unwrap();
    host_state("start", frames);
    let library = Library::new(root.join(MACROS_DIR));
    crate::engine::rr_runtime::migrate_at_boot(&root, &[None, None, None, None, None], "2026-11-26").unwrap();
    let make = |name: String| {
        let inputs = (0..frames).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
        #[allow(unused_mut)]
        let mut rec = MacroRecording::new(name, "13.0.5", StartState::neutral(8, 0, 1.0), inputs).unwrap();
        rec.set_track(Some(vec![[crate::engine::macros::ReplayFrame { pose: crate::engine::macros::PackedPose::NONE, input: InputFrame::default(), start: StartState::neutral(8, 0, 1.0), x: 1.0, y: 2.0 }; 2]; frames])); // head-only
        let id = library.save_new(&rec).unwrap();
        // main-only: positions_file(&root.join(MACROS_DIR), &id, frames);
        id
    };
    let ids: Vec<String> = (0..3).map(|step| make(format!("Timing {frames} {step}"))).collect();
    let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new(format!("Chain {frames}"), ids.clone()).unwrap()).unwrap();
    let inputs = (0..frames).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
    let plain = MacroRecording::new(format!("Armed {frames}"), "13.0.5", StartState::neutral(8, 0, 1.0), inputs).unwrap();
    let armed_id = library.save_new(&plain).unwrap();
    assert!(!root.join(MACROS_DIR).join(format!("{armed_id}.pos")).exists());
    let armed_json = fs::metadata(root.join(MACROS_DIR).join(format!("{armed_id}.json"))).unwrap().len();
    let json = fs::metadata(root.join(MACROS_DIR).join(format!("{}.json", ids[0]))).unwrap().len();
    let pos = fs::metadata(root.join(MACROS_DIR).join(format!("{}.pos", ids[0]))).unwrap().len();
    assert_eq!(pos, 40 + 72 * frames as u64, "both sides have the same positions file beside each macro");
    // Everything written so far is flushed, and the host settles, before anything is timed.
    assert!(Command::new("sync").status().unwrap().success());
    std::thread::sleep(Duration::from_millis(500));
    report("load", frames, json, || { assert_eq!(library.load(&ids[0]).unwrap().frames.len(), frames); });
    report("play", frames, json, || start(&root, &ids[0]));
    report("chain3", frames, json, || start(&root, &chain));
    assert!(!host::play_allocated(), "no sampler table before the first eligible Play"); // head-only
    let mut times = Vec::with_capacity(RUNS);
    let mut armed = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let began = Instant::now();
        TIMED.with(|timed| timed.set(None));
        start_armed(&root, &armed_id);
        times.push(TIMED.with(|timed| timed.take()).unwrap_or(began).elapsed().as_micros());
        armed.push(ARMED.with(|armed| armed.get()));
    }
    let cold = times[0];
    eprintln!("PLAY_TIMING what=play_armed_cold frames={frames} json_bytes={armed_json} runs=1 pinned={} median_us={cold} min_us={cold} max_us={cold} first_us={cold}",
        env::var("TR_SCRUB_TIMING_PINNED").unwrap_or_default());
    let first = times[1];
    let mut warm = times[1..].to_vec();
    warm.sort_unstable();
    let median = (warm[warm.len() / 2 - 1] + warm[warm.len() / 2]) / 2;
    eprintln!("PLAY_TIMING what=play_armed frames={frames} json_bytes={armed_json} runs={} pinned={} median_us={} min_us={} max_us={} first_us={first}",
        warm.len(), env::var("TR_SCRUB_TIMING_PINNED").unwrap_or_default(), median, warm[0], warm[warm.len() - 1]);
    assert!(armed.iter().all(|armed| *armed) && host::play_sampling(), "every sample armed generation"); // head-only
    host::on_chords(false, true, false);
    let state = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    for _ in 0..2 {
        host::play_track_service(&root); // head-only
        host::service_at(&root, &[None, None, None, None, None], "2026-11-26", state(8), state(7));
    }
    let armed_inputs = library.load(&armed_id).unwrap().frames;
    assert_eq!(armed_inputs.len(), frames);
    assert_eq!(crate::engine::macros::positions::source(&root.join(MACROS_DIR), &armed_id, &armed_inputs), Some(crate::engine::macros::positions::Source::Generated)); // head-only
    assert_eq!(crate::engine::macros::positions::read(&root.join(MACROS_DIR), &armed_id, &armed_inputs).map(|rows| rows.len()), Some(1)); // head-only
    host_state("end", frames);
}

// main-only: /// What the head's library writes beside a macro with positions: 40 + 72 B a frame, through a temporary name, synced.
// main-only: fn positions_file(dir: &Path, id: &str, frames: usize) {
// main-only:     use std::io::Write;
// main-only:     let (tmp, path) = (dir.join(format!("{id}.pos.tmp")), dir.join(format!("{id}.pos")));
// main-only:     let mut file = fs::File::create(&tmp).unwrap();
// main-only:     file.write_all(&vec![0u8; 40 + 72 * frames]).unwrap();
// main-only:     file.sync_all().unwrap();
// main-only:     fs::rename(tmp, path).unwrap();
// main-only: }
// main-only:
/// The host's load average, and the busy time so far of the pinned core's sibling thread (clock ticks).
fn host_state(when: &str, frames: usize) {
    let load = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let sibling = fs::read_to_string("/proc/stat").unwrap_or_default().lines().find(|line| line.starts_with(SIBLING))
        .map(|line| line.split_whitespace().skip(1).take(3).filter_map(|ticks| ticks.parse::<u64>().ok()).sum::<u64>());
    eprintln!("PLAY_HOST when={when} frames={frames} loadavg={} sibling_busy_ticks={sibling:?}", load.split_whitespace().take(3).collect::<Vec<_>>().join("/"));
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
    TIMED.with(|timed| timed.set(Some(Instant::now())));
    host::on_chords(false, true, false);
    tick();
    assert_eq!(host::take_player_frame().map(|frame| frame.buttons), Some(1), "the first input after the chord");
}

fn start_armed(root: &Path, id: &str) {
    let mut index = RecordingIndex::load(root).unwrap();
    index.set_active(Role::Player, Some(id), 1).unwrap();
    index.set_loop(Role::Player, LoopCount::Finite(1));
    index.save(root).unwrap();
    host::reset_host();
    host::set_role_fighter(Role::Player, 8);
    let state = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let tick = || host::service_at(root, &[None, None, None, None, None], "2026-11-26", state(8), state(7));
    tick();
    TIMED.with(|timed| timed.set(Some(Instant::now())));
    host::on_chords(false, true, false);
    tick();
    ARMED.with(|armed| armed.set(host::play_sampling())); // head-only
    assert_eq!(host::take_player_frame().map(|frame| frame.buttons), Some(1), "the first input after the chord");
}

thread_local! {
    static TIMED: core::cell::Cell<Option<Instant>> = const { core::cell::Cell::new(None) };
    static ARMED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// Run `what` `RUNS` times and print the median and the fastest, in microseconds. `start` restarts the clock at its
/// chord, so its setup (the index write, the reset, the warm tick) is not counted.
fn report(name: &str, frames: usize, json: u64, mut what: impl FnMut()) {
    let mut times: Vec<u128> = (0..RUNS).map(|_| {
        let began = Instant::now();
        TIMED.with(|timed| timed.set(None));
        what();
        TIMED.with(|timed| timed.take()).unwrap_or(began).elapsed().as_micros()
    }).collect();
    let first = times[0];
    times.sort_unstable();
    eprintln!("PLAY_TIMING what={name} frames={frames} json_bytes={json} runs={RUNS} pinned={} median_us={} min_us={} max_us={} first_us={first}",
        env::var("TR_SCRUB_TIMING_PINNED").unwrap_or_default(), times[RUNS / 2], times[0], times[RUNS - 1]);
}
