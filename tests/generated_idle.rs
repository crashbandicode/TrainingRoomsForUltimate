use crate::engine::library::{Chain, RecordingIndex, Role, MACROS_DIR};
use crate::engine::macros::{positions::{self, Source}, InputFrame, Library, MacroRecording, StartState};
use crate::engine::rr_host as host;
use crate::model::{state::record::SavedState, store::Store};
use std::{env, fs, process::Command};

#[test]
fn no_positions_sampler_until_the_first_eligible_play() {
    const TEST: &str = "generated_idle::no_positions_sampler_until_the_first_eligible_play";
    const KEY: &str = "TR_GENERATED_IDLE_ROOT";
    let Some(root) = env::var_os(KEY).map(std::path::PathBuf::from) else {
        let root = crate::test_scratch::Scratch::new("tr-generated-idle");
        let ran = Command::new(env::current_exe().unwrap()).args(["--exact", TEST, "--nocapture"]).env(KEY, root.path()).status().unwrap().success();
        fs::remove_dir_all(root).unwrap();
        assert!(ran, "the fresh process failed");
        return;
    };
    assert!(!host::play_allocated());
    let state = |kind| { let mut s = SavedState::never_saved(); s.fighter_kind = kind; s };
    let tick = || {
        host::play_track_service(&root);
        host::service_at(&root, &[None, None, None, None, None], "2026-11-26", state(8), state(7))
    };
    for _ in 0..3 {
        tick();
        assert!(!host::play_allocated());
    }
    let library = Library::new(root.join(MACROS_DIR));
    let inputs = (1..=5).map(|buttons| InputFrame { buttons, ..InputFrame::default() }).collect();
    let recording = MacroRecording::new("Idle", "13.0.5", StartState::neutral(8, 0, 1.0), inputs).unwrap();
    let id = library.save_new(&recording).unwrap();
    let chain = Store::<Chain>::new(root.join(MACROS_DIR)).save_new(&Chain::new("Chain", vec![id.clone()]).unwrap()).unwrap();
    let mut index = RecordingIndex::load(&root).unwrap();
    index.set_active(Role::Player, Some(&chain), 1).unwrap();
    index.save(&root).unwrap();
    host::set_role_fighter(Role::Player, 8);
    host::on_chords(false, true, false);
    tick();
    assert!(!host::play_allocated() && !host::play_sampling());
    for input in &recording.frames { assert_eq!(host::take_player_frame(), Some(*input)); }
    tick();
    assert!(!host::play_allocated());

    let mut index = RecordingIndex::load(&root).unwrap();
    index.set_active(Role::Player, Some(&id), 1).unwrap();
    index.save(&root).unwrap();
    host::set_role_fighter(Role::Player, 7);
    host::publish_fighter_prompt(false);
    host::on_chords(false, true, false);
    tick();
    assert!(!host::play_allocated() && !host::play_sampling());
    for input in &recording.frames { assert_eq!(host::take_player_frame(), Some(*input)); }
    tick();
    assert!(!host::play_allocated());

    host::set_role_fighter(Role::Player, 8);
    host::request_preview(&id, Role::Player, 1, 4);
    tick();
    assert!(!host::play_allocated() && !host::play_sampling());
    for input in &recording.frames[1..4] { assert_eq!(host::take_player_frame(), Some(*input)); }
    tick();
    assert!(!host::play_allocated());

    host::on_chords(false, true, false);
    tick();
    assert!(host::play_allocated() && host::play_sampling());
    assert_eq!(host::take_player_frame(), Some(recording.frames[0]));
    host::on_chords(false, true, false);
    tick();
    tick();
    let dir = root.join(MACROS_DIR);
    assert_eq!(positions::source(&dir, &id, &recording.frames), Some(Source::Generated));
    assert_eq!(positions::read(&dir, &id, &recording.frames).map(|rows| rows.len()), Some(1));
    assert!(host::play_allocated());
}
