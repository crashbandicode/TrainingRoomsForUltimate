use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState, MAX_FRAMES};
use crate::model::hook_stats::heap_bytes;
use std::{env, fs, path::PathBuf, process::Command};

#[test]
fn a_full_input_only_macro_streams_its_save_and_loads_from_one_buffer() {
    const TEST: &str = "input_json::a_full_input_only_macro_streams_its_save_and_loads_from_one_buffer";
    const ROOT: &str = "TR_INPUT_JSON_TEST_DIR";
    let Some(dir) = env::var_os(ROOT).map(PathBuf::from) else {
        let dir = env::temp_dir().join(format!("tr-input-json-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let run = |phase| Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"])
            .env(ROOT, &dir).env("TR_INPUT_JSON_PHASE", phase).status().unwrap().success();
        let saved = run("save");
        let loaded = saved && run("load");
        fs::remove_dir_all(&dir).unwrap();
        assert!(saved && loaded, "save={saved} load={loaded}");
        return;
    };
    let library = Library::new(&dir);
    let phase = env::var("TR_INPUT_JSON_PHASE").unwrap();
    let id = "long-inputs";
    let path = dir.join(format!("{id}.json"));
    let input = |i: usize| InputFrame { buttons: (i % 17 + 1) as u32, lstick_x: -81, rstick_y: 52, ..InputFrame::default() };
    if phase == "save" {
        let rec = MacroRecording::new("Long inputs", "13.0.5", StartState::neutral(8, 0, 1.0),
            (0..MAX_FRAMES).map(input).collect()).unwrap();
        let before = heap_bytes();
        assert_eq!(library.save_new(&rec).unwrap(), id);
        let after = heap_bytes();
        let bytes = fs::read(path).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&rec).unwrap());
        eprintln!("INPUT_JSON save frames={MAX_FRAMES} file={} peak_above_live={} allocated={}", bytes.len(),
            after.peak - before.live, after.total - before.total);
        assert!(after.total - before.total < 256 * 1024, "buffered a maximum input-only macro during save");
    } else {
        assert_eq!(phase, "load");
        let before = heap_bytes();
        let rec = library.load(id).unwrap();
        let peak = heap_bytes().peak - before.live;
        assert_eq!(rec.frames.len(), MAX_FRAMES);
        assert!(rec.frames.iter().copied().eq((0..MAX_FRAMES).map(input)));
        eprintln!("INPUT_JSON load frames={} peak_above_live={peak}", rec.frames.len());
        assert!(peak < fs::metadata(path).unwrap().len() as isize + 1024 * 1024, "load retained more than one JSON buffer");
    }
}
