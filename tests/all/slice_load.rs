use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState, MAX_FRAMES};
use crate::model::hook_stats::heap_bytes;
use std::{env, fs, path::PathBuf, process::Command, time::Instant};

#[test]
fn the_production_load_keeps_up_with_a_slice_of_the_same_file() {
    const KEY: &str = "TR_SLICE_LOAD_ROOT";
    const TEST: &str = "slice_load::the_production_load_keeps_up_with_a_slice_of_the_same_file";
    let Some(dir) = env::var_os(KEY).map(PathBuf::from) else {
        let scratch = crate::test_scratch::Scratch::new("tr-slice-load");
        let dir = scratch.join("root");
        let frames = (0..MAX_FRAMES).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
        let rec = MacroRecording::new("Timed", "13.0.5", StartState::neutral(8, 0, 1.0), frames).unwrap();
        assert_eq!(Library::new(&dir).save_new(&rec).unwrap(), "timed");
        let status = Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"]).env(KEY, &dir).status().unwrap();
        fs::remove_dir_all(dir).unwrap();
        assert!(status.success());
        return;
    };
    let library = Library::new(&dir);
    let path = dir.join("timed.json");
    let bytes = fs::metadata(&path).unwrap().len();
    let before = heap_bytes();
    let mut list = [0; 21];
    for sample in &mut list {
        let start = Instant::now();
        let listing = library.list().unwrap();
        *sample = start.elapsed().as_nanos();
        assert_eq!((listing.entries.len(), listing.entries[0].frame_count, listing.skipped.len()), (1, MAX_FRAMES, 0));
    }
    let list_peak = heap_bytes().peak - before.live;
    let mut prod = [0; 21];
    let mut baseline = [0; 21];
    for i in 0..21 {
        for production in [i % 2 == 0, i % 2 != 0] {
            let start = Instant::now();
            let rec = if production {
                library.load("timed").unwrap()
            } else {
                let buf = fs::read(&path).unwrap();
                let rec: MacroRecording = serde_json::from_slice(&buf).unwrap();
                rec.validate().unwrap();
                rec
            };
            (if production { &mut prod } else { &mut baseline })[i] = start.elapsed().as_nanos();
            assert_eq!((rec.frames.len(), rec.frames[MAX_FRAMES - 1].buttons), (MAX_FRAMES, ((MAX_FRAMES - 1) % 7 + 1) as u32));
        }
    }
    list.sort_unstable();
    prod.sort_unstable();
    baseline.sort_unstable();
    eprintln!("SLICE_LOAD frames={MAX_FRAMES} bytes={bytes} median_of=21 prod_us={} slice_us={} list_us={} list_peak={list_peak}",
        prod[10] / 1000, baseline[10] / 1000, list[10] / 1000);
    assert!(prod[10] * 10 <= baseline[10] * 13, "production load exceeds 1.3x the same-file slice baseline");
}
