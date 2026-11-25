use crate::engine::library::{Chain, LoopCount, NamedSave, RecordingIndex, Role, MACROS_DIR, SAVES_DIR};
use crate::engine::macros::{InputFrame, Library, MacroRecording, StartState};
use crate::engine::playback::{Book, Playback};
use crate::engine::rr_host as host;
use crate::model::menu::table::{Control, ROWS};
use crate::model::recording::RecorderSettings;
use crate::model::settings::{self, Snapshot};
use crate::model::state::record::SavedState;
use crate::model::store::{self, Store};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

static HOST: Mutex<()> = Mutex::new(());
static NEXT: AtomicUsize = AtomicUsize::new(0);
const SESSION_FRAMES: [usize; 22] = [
    5, 47, 49, 49, 75, 75, 88, 115, 136, 138, 138, 146, 149, 149, 149, 150, 150, 152, 154, 198, 201, 214,
];

struct HostGuard(MutexGuard<'static, ()>);

impl HostGuard {
    fn new() -> Self {
        let guard = HOST.lock().unwrap_or_else(|error| error.into_inner());
        host::reset_host();
        Self(guard)
    }
}

impl Drop for HostGuard {
    fn drop(&mut self) {
        host::reset_host();
        let _ = &self.0;
    }
}

fn scratch() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tr-playback-immediate-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

fn state(kind: i32) -> SavedState {
    let mut state = SavedState::never_saved();
    state.fighter_kind = kind;
    state
}

fn recording(name: &str, len: usize) -> MacroRecording {
    let frames = (0..len).map(|i| InputFrame { buttons: (i % 7 + 1) as u32, ..InputFrame::default() }).collect();
    MacroRecording::new(name, "13.0.5", StartState::neutral(8, 0, 1.0), frames).unwrap()
}

struct Fixture {
    root: PathBuf,
    active: String,
    ids: Vec<String>,
    save: String,
    pair: [SavedState; 2],
}

fn fixture(with_save: bool, loops: LoopCount, frame_counts: &[usize]) -> Fixture {
    let root = scratch();
    let saves = Store::<NamedSave>::new(root.join(SAVES_DIR));
    let pair = [state(8), state(7)];
    let save = saves.save_new(&NamedSave::new("Save 0", pair[0].clone(), pair[1].clone()).unwrap()).unwrap();
    for i in 1..10 {
        saves.save_new(&NamedSave::new(format!("Save {i}"), state(8), state(7)).unwrap()).unwrap();
    }
    let library = Library::new(root.join(MACROS_DIR));
    let mut ids = Vec::new();
    for (i, &len) in frame_counts.iter().enumerate() {
        let mut rec = recording(&format!("Macro {i}"), len);
        if i == 0 && with_save { rec.set_start_save(Some(&save)).unwrap(); }
        ids.push(library.save_new(&rec).unwrap());
    }
    crate::engine::rr_runtime::migrate_at_boot(&root, &[None, None, None, None, None], "2026-11-25").unwrap();
    let mut index = RecordingIndex::load(&root).unwrap();
    index.set_active(Role::Player, Some(&ids[0]), 1).unwrap();
    index.set_active(Role::Cpu, Some(&ids[0]), 1).unwrap();
    index.set_loop(Role::Player, loops);
    index.set_loop(Role::Cpu, loops);
    index.save(&root).unwrap();
    Fixture { root, active: ids[0].clone(), ids, save, pair }
}

fn small(with_save: bool) -> Fixture {
    fixture(with_save, LoopCount::Finite(1), &[24])
}

fn session(with_save: bool) -> Fixture {
    fixture(with_save, LoopCount::Finite(1), &[24; 22])
}

fn tick(root: &Path) -> host::Tick {
    host::service_at(root, &[None, None, None, None, None], "2026-11-25", state(8), state(7))
}

fn chord(role: Role) {
    host::on_chords(false, role == Role::Player, role == Role::Cpu);
}

fn poll(role: Role) -> Option<InputFrame> {
    if role == Role::Player { host::take_player_frame() } else { host::take_cpu_frame() }
}

fn warm(f: &Fixture, with_save: bool) {
    tick(&f.root);
    chord(Role::Player);
    let start = tick(&f.root);
    if with_save {
        assert_eq!(start.warp.as_deref(), Some(f.save.as_str()));
        host::warp_landed();
        tick(&f.root);
    }
    chord(Role::Player);
    tick(&f.root);
}

fn counts() -> (usize, usize) {
    (store::file_reads() + store::index_reads(), store::directory_reads())
}

fn difference(before: (usize, usize), after: (usize, usize)) -> (usize, usize) {
    (after.0 - before.0, after.1 - before.1)
}

fn delayed_chord(f: &Fixture, role: Role, delay: u16, warp: bool) {
    host::set_playback_delay(delay);
    chord(role);
    let started = tick(&f.root);
    if warp {
        assert_eq!(started.warp.as_deref(), Some(f.save.as_str()));
        assert_eq!(poll(role), None);
        host::warp_landed();
        tick(&f.root);
    } else {
        assert!(started.warp.is_none());
    }
}

fn assert_delay(role: Role, frames: u16) {
    for at in 0..frames {
        assert_eq!(host::playback_roles(), [role == Role::Player, role == Role::Cpu], "indicator during poll {at}");
        assert_eq!(poll(role), None, "role {role:?}, delay {frames}, poll {at}");
    }
    assert_eq!(poll(role).map(|f| f.buttons), Some(1), "first frame after delay {frames}");
}

#[test]
fn a_play_start_reads_only_what_it_starts() {
    let _guard = HostGuard::new();
    let f = session(true);
    warm(&f, true);
    let before = counts();
    chord(Role::Player);
    let start = tick(&f.root);
    let after_chord = counts();
    assert_eq!(start.warp.as_deref(), Some(f.save.as_str()));
    host::warp_landed();
    tick(&f.root);
    let after_restore = counts();
    let mut passes = vec![difference(before, after_chord), difference(after_chord, after_restore)];
    for _ in 0..3 {
        assert!(poll(Role::Player).is_some());
        let before = counts();
        tick(&f.root);
        passes.push(difference(before, counts()));
    }
    assert_eq!(passes, vec![(3, 0), (0, 0), (0, 0), (0, 0), (0, 0)], "file reads, listings: chord, after restore, running passes");
}

#[test]
fn an_idle_pass_reads_nothing_from_the_card() {
    let _guard = HostGuard::new();
    let f = small(false);
    // The first tick runs migration and is not measured.
    tick(&f.root);
    let before = counts();
    tick(&f.root);
    assert_eq!(difference(before, counts()), (0, 0), "idle before Play");
    chord(Role::Player);
    tick(&f.root);
    while poll(Role::Player).is_some() {}
    tick(&f.root);
    let before = counts();
    tick(&f.root);
    assert_eq!(difference(before, counts()), (0, 0), "idle after Play");
}

#[test]
fn the_reads_at_a_start_do_not_grow_with_the_library() {
    let _guard = HostGuard::new();
    let mut results = Vec::new();
    for f in [small(true), session(true)] {
        host::reset_host();
        warm(&f, true);
        let before = counts();
        chord(Role::Player);
        tick(&f.root);
        results.push(difference(before, counts()));
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn a_playing_run_reads_nothing_from_the_card_per_pass() {
    let _guard = HostGuard::new();
    let f = session(false);
    warm(&f, false);
    chord(Role::Cpu);
    tick(&f.root);
    let mut passes = Vec::new();
    for _ in 0..4 {
        assert!(poll(Role::Cpu).is_some());
        let before = counts();
        tick(&f.root);
        passes.push(difference(before, counts()));
    }
    assert_eq!(passes, vec![(0, 0); 4], "file reads, listings per CPU running pass");
}

#[test]
fn play_with_a_start_save_feeds_on_the_pass_after_the_restore() {
    let _guard = HostGuard::new();
    let f = small(true);
    tick(&f.root);
    chord(Role::Player);
    assert_eq!(poll(Role::Player), None);
    let start = tick(&f.root);
    assert_eq!(start.warp.as_deref(), Some(f.save.as_str()));
    for _ in 0..3 {
        assert_eq!(poll(Role::Player), None);
        tick(&f.root);
    }
    host::warp_landed();
    assert_eq!(poll(Role::Player), None);
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    writeln!(std::io::stdout(), "start-save timing: chord pass, restore, one worker pass, first poll").unwrap();
}

#[test]
fn play_without_a_start_save_feeds_on_the_first_pass() {
    let _guard = HostGuard::new();
    let f = small(false);
    tick(&f.root);
    chord(Role::Player);
    assert_eq!(poll(Role::Player), None);
    assert!(tick(&f.root).warp.is_none());
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    writeln!(std::io::stdout(), "no-save timing: chord pass, first poll").unwrap();
}

#[test]
fn the_start_save_is_posted_without_a_card_read() {
    let _guard = HostGuard::new();
    let f = session(true);
    warm(&f, true);
    let before = counts();
    chord(Role::Player);
    let started = tick(&f.root);
    let reads = difference(before, counts());
    assert_eq!(started.warp.as_deref(), Some(f.save.as_str()));
    assert_eq!(started.warp_pair, Some(f.pair), "the worker supplies the restored pair; card reads/listings {reads:?}");
    assert_eq!(reads, (3, 0));
    let source = include_str!("../src/runtime/features/recording_host.rs");
    assert!(source.contains("tick.warp_pair"));
    assert!(source.contains("request_named_load"));
}

#[test]
fn a_library_write_while_a_macro_plays_is_read_on_the_next_pass() {
    let _guard = HostGuard::new();
    let f = session(false);
    warm(&f, false);
    chord(Role::Player);
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(1));
    let library = Library::new(f.root.join(MACROS_DIR));
    library.save_new(&recording("Added", 7)).unwrap();
    let before_catalog = counts();
    tick(&f.root);
    let catalog_reads = difference(before_catalog, counts());
    assert!(catalog_reads.0 > 0 && catalog_reads.1 == 0, "catalog change must reload held files: {catalog_reads:?}");
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(2));
    let before_stable = counts();
    tick(&f.root);
    assert_eq!(difference(before_stable, counts()), (0, 0));
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(3));
}

#[test]
fn a_same_size_rewrite_outside_the_mod_is_played_by_the_next_start() {
    let _guard = HostGuard::new();
    let f = small(true);
    warm(&f, true);
    let versions = (store::catalog_generation(), store::index_generation());
    let macro_path = f.root.join(MACROS_DIR).join(format!("{}.json", f.active));
    let mut bytes = std::fs::read(&macro_path).unwrap();
    let macro_len = bytes.len();
    let needle = b"\"buttons\":1";
    let at = bytes.windows(needle.len()).position(|part| part == needle).unwrap();
    bytes[at + needle.len() - 1] = b'9';
    std::fs::write(&macro_path, &bytes).unwrap();
    assert_eq!(std::fs::metadata(&macro_path).unwrap().len(), macro_len as u64);
    let save_path = f.root.join(SAVES_DIR).join(format!("{}.json", f.save));
    let old = std::fs::read(&save_path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&old).unwrap();
    value["player"]["x"] = serde_json::json!(9.0);
    let changed = serde_json::to_vec(&value).unwrap();
    assert_eq!(old.len(), changed.len());
    assert_eq!(old.iter().zip(&changed).filter(|(a, b)| a != b).count(), 1);
    std::fs::write(&save_path, &changed).unwrap();
    assert_eq!((store::catalog_generation(), store::index_generation()), versions);
    chord(Role::Player);
    let start = tick(&f.root);
    let mut pair = f.pair.clone();
    pair[0].x = 9.0;
    assert_eq!(start.warp_pair, Some(pair));
    host::warp_landed();
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(9));
}

#[test]
fn an_index_replaced_outside_the_mod_is_followed_by_the_next_start() {
    let _guard = HostGuard::new();
    let f = small(false);
    let mut other = recording("Other", 2);
    other.frames[0].buttons = 67;
    let id = Library::new(f.root.join(MACROS_DIR)).save_new(&other).unwrap();
    warm(&f, false);
    let mut index = RecordingIndex::load(&f.root).unwrap();
    index.set_active(Role::Player, Some(&id), 4).unwrap();
    let versions = (store::catalog_generation(), store::index_generation());
    std::fs::write(f.root.join(crate::engine::library::INDEX_FILE), serde_json::to_vec(&index).unwrap()).unwrap();
    assert_eq!((store::catalog_generation(), store::index_generation()), versions);
    chord(Role::Player);
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(67));
}

#[test]
fn a_rewritten_active_start_save_is_used_by_the_next_play() {
    let _guard = HostGuard::new();
    let f = session(true);
    warm(&f, true);
    let saves = Store::<NamedSave>::new(f.root.join(SAVES_DIR));
    let mut changed = saves.load(&f.save).unwrap();
    changed.player.x = 42.0;
    changed.cpu.x = -21.0;
    saves.overwrite(&f.save, &changed).unwrap();
    chord(Role::Player);
    let next = tick(&f.root);
    assert_eq!(next.warp.as_deref(), Some(f.save.as_str()));
    assert_eq!(next.warp_pair, Some([changed.player, changed.cpu]));
}

#[test]
fn a_deleted_active_start_save_is_not_warped_to_on_the_next_play() {
    let _guard = HostGuard::new();
    let f = session(true);
    warm(&f, true);
    Store::<NamedSave>::new(f.root.join(SAVES_DIR)).delete(&f.save).unwrap();
    chord(Role::Player);
    let next = tick(&f.root);
    assert!(next.warp.is_none());
    assert!(next.warp_pair.is_none());
    assert!(next.notices.contains(&crate::engine::rr_runtime::MISSING_START_SAVE));
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(1));
}

#[test]
fn rewritten_active_macro_frames_are_used_by_the_next_play() {
    let _guard = HostGuard::new();
    let f = session(false);
    warm(&f, false);
    let library = Library::new(f.root.join(MACROS_DIR));
    let mut changed = library.load(&f.active).unwrap();
    changed.frames[0].buttons = 91;
    changed.frames[1].buttons = 92;
    library.overwrite(&f.active, &changed).unwrap();
    chord(Role::Player);
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(91));
    assert_eq!(poll(Role::Player).map(|frame| frame.buttons), Some(92));
}

#[test]
fn changed_active_and_loop_are_used_by_the_next_play() {
    let _guard = HostGuard::new();
    let f = session(false);
    warm(&f, false);
    let mut changed = recording("Other", 2);
    changed.frames[0].buttons = 71;
    changed.frames[1].buttons = 72;
    let id = Library::new(f.root.join(MACROS_DIR)).save_new(&changed).unwrap();
    let mut index = RecordingIndex::load(&f.root).unwrap();
    index.set_active(Role::Player, Some(&id), 3).unwrap();
    index.set_loop(Role::Player, LoopCount::Finite(2));
    index.save(&f.root).unwrap();
    chord(Role::Player);
    tick(&f.root);
    let frames: Vec<_> = (0..4).map(|_| poll(Role::Player).unwrap().buttons).collect();
    assert_eq!(frames, [71, 72, 71, 72]);
}

#[test]
fn an_active_chain_reads_its_steps_and_first_start_save() {
    let _guard = HostGuard::new();
    let f = small(true);
    let chain = Chain::new("Chain", vec![f.active.clone()]).unwrap();
    let id = Store::<Chain>::new(f.root.join(MACROS_DIR)).save_new(&chain).unwrap();
    let mut index = RecordingIndex::load(&f.root).unwrap();
    index.set_active(Role::Player, Some(&id), 3).unwrap();
    index.save(&f.root).unwrap();
    warm(&f, true);
    let before = counts();
    chord(Role::Player);
    let next = tick(&f.root);
    assert_eq!(next.warp.as_deref(), Some(f.save.as_str()));
    assert_eq!(next.warp_pair, Some(f.pair));
    assert_eq!(difference(before, counts()), (5, 0));
}

#[test]
fn a_chain_switch_decides_its_warp_not_macro_ones_own_switch() {
    let _guard = HostGuard::new();
    let f = small(true);
    warm(&f, true);
    let library = Library::new(f.root.join(MACROS_DIR));
    let mut recording = library.load(&f.active).unwrap();
    recording.set_start_save_on(false);
    library.overwrite(&f.active, &recording).unwrap();
    chord(Role::Player);
    assert!(tick(&f.root).warp.is_none());
    chord(Role::Player);
    tick(&f.root);
    let chains = Store::<Chain>::new(f.root.join(MACROS_DIR));
    let mut chain = Chain::new("Chain", vec![f.active.clone()]).unwrap();
    let id = chains.save_new(&chain).unwrap();
    let mut index = RecordingIndex::load(&f.root).unwrap();
    index.set_active(Role::Player, Some(&id), 3).unwrap();
    index.save(&f.root).unwrap();
    chord(Role::Player);
    let start = tick(&f.root);
    assert_eq!(start.warp.as_deref(), Some(f.save.as_str()));
    assert_eq!(start.warp_pair, Some(f.pair.clone()));
    host::warp_landed();
    let before = counts();
    tick(&f.root);
    assert_eq!(difference(before, counts()), (0, 0));
    chord(Role::Player);
    tick(&f.root);
    chain.set_start_save_on(false);
    chains.overwrite(&id, &chain).unwrap();
    chord(Role::Player);
    assert!(tick(&f.root).warp.is_none());
}

#[test]
fn held_files_play_the_same_frames_as_a_fresh_read() {
    let _guard = HostGuard::new();
    let f = small(false);
    let rec = Library::new(f.root.join(MACROS_DIR)).load(&f.active).unwrap();
    let mut book = Book::new(RecordingIndex::load(&f.root).unwrap());
    book.add_recording(f.active.clone(), rec.clone());
    let mut fresh = Playback::new();
    assert!(fresh.play_file(Role::Player, &book, &f.active).started);
    let expected: Vec<_> = (0..rec.frames.len()).map(|_| fresh.step(Role::Player, &book).unwrap().frame).collect();
    warm(&f, false);
    chord(Role::Player);
    tick(&f.root);
    let played: Vec<_> = (0..rec.frames.len()).map(|_| poll(Role::Player).unwrap()).collect();
    assert_eq!(played, expected);
}

#[test]
fn a_vault_playback_and_a_preview_read_only_what_they_start() {
    let _guard = HostGuard::new();
    let f = session(false);
    warm(&f, false);
    let before = counts();
    host::request_clip_playback(recording("Clip", 4), Role::Cpu);
    tick(&f.root);
    let clip_start = counts();
    host::warp_landed();
    tick(&f.root);
    let clip_resume = counts();
    assert_eq!(poll(Role::Cpu).map(|f| f.buttons), Some(1));
    host::stop_preview();
    tick(&f.root);
    host::warp_landed();
    tick(&f.root);
    let before_preview = counts();
    host::request_preview(&f.active, Role::Player, 0, 4);
    tick(&f.root);
    let preview_start = counts();
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    assert_eq!([
        difference(before, clip_start), difference(clip_start, clip_resume), difference(before_preview, preview_start),
    ], [(1, 0), (0, 0), (2, 0)], "file reads, listings: Vault start/resume and editor preview");
}

#[test]
fn playback_delay_row_lists_its_frames() {
    let row = ROWS.iter().find(|row| row.id == "playback_delay");
    assert!(row.is_some(), "Playback delay must exist");
    let row = row.unwrap();
    let options = ROWS.iter().find(|other| other.id == "recording_crop").unwrap().group;
    assert_eq!((row.title, row.group), ("Playback delay", options));
    assert!(matches!(row.control, Control::Dropdown(options) if options == [
        "None", "10 frames", "20 frames", "30 frames", "45 frames", "60 frames", "90 frames", "120 frames"
    ]));
    assert_eq!(row.help.lines().count(), 2);
    assert!(row.help.contains("60 frames is one second"));
    assert_eq!(ROWS.iter().filter(|other| other.group == options).last().unwrap().id, "playback_delay");
    for (at, frames) in [0, 10, 20, 30, 45, 60, 90, 120].into_iter().enumerate() {
        let mut snapshot = Snapshot::factory();
        let mut selected = [0; 8];
        selected[at] = 1;
        snapshot.set("playback_delay", &selected).unwrap();
        assert_eq!(RecorderSettings::from_snapshot(&snapshot).delay, frames);
    }
}

#[test]
fn playback_delay_holds_the_first_input_for_its_frames() {
    let _guard = HostGuard::new();
    for delay in [0, 10, 60] {
        host::reset_host();
        let f = small(false);
        tick(&f.root);
        delayed_chord(&f, Role::Player, delay, false);
        assert_delay(Role::Player, delay);
    }
}

#[test]
fn playback_delay_applies_to_player_cpu_and_vault_playback() {
    let _guard = HostGuard::new();
    for role in [Role::Player, Role::Cpu] {
        for delay in [0, 10, 60] {
            host::reset_host();
            let f = small(false);
            tick(&f.root);
            delayed_chord(&f, role, delay, false);
            assert_delay(role, delay);
        }
    }
    for delay in [0, 10, 60] {
        host::reset_host();
        let f = small(false);
        tick(&f.root);
        host::set_playback_delay(delay);
        host::request_clip_playback(recording("Vault", 4), Role::Cpu);
        assert!(matches!(tick(&f.root).home, Some(host::HomeWarp::Start { .. })));
        host::warp_landed();
        tick(&f.root);
        assert_delay(Role::Cpu, delay);
    }
    host::reset_host();
    let f = small(false);
    let chain = Chain::new("Chain", vec![f.active.clone()]).unwrap();
    let chain_id = Store::<Chain>::new(f.root.join(MACROS_DIR)).save_new(&chain).unwrap();
    let mut index = RecordingIndex::load(&f.root).unwrap();
    index.set_active(Role::Player, Some(&chain_id), 2).unwrap();
    index.save(&f.root).unwrap();
    tick(&f.root);
    delayed_chord(&f, Role::Player, 10, false);
    assert_delay(Role::Player, 10);

    host::reset_host();
    let f = small(false);
    tick(&f.root);
    host::set_playback_delay(10);
    host::request_make(host::MakeAsk { source: f.active.clone(), role: Role::Player, drill: None });
    tick(&f.root);
    assert_delay(Role::Player, 10);

    host::reset_host();
    let f = small(false);
    tick(&f.root);
    host::set_playback_delay(10);
    chord(Role::Player);
    assert_eq!(host::playback_roles(), [false, false]);
    tick(&f.root);
    assert_eq!(host::playback_roles(), [true, false], "indicator starts with the worker pass");
    let source = include_str!("../src/runtime/macros/replay_capture.rs");
    assert!(source.contains("request_clip_playback("), "Vault Play back must carry its origin through the runtime");
}

#[test]
fn playback_delay_counts_after_the_restore() {
    let _guard = HostGuard::new();
    let f = small(true);
    tick(&f.root);
    host::set_playback_delay(10);
    chord(Role::Player);
    assert!(tick(&f.root).warp.is_some());
    for _ in 0..15 {
        assert_eq!(poll(Role::Player), None);
        tick(&f.root);
    }
    host::warp_landed();
    tick(&f.root);
    assert_delay(Role::Player, 10);
}

#[test]
fn playback_delay_is_owed_once_not_per_loop_pass() {
    let _guard = HostGuard::new();
    let f = fixture(false, LoopCount::Finite(2), &[2]);
    tick(&f.root);
    delayed_chord(&f, Role::Player, 10, false);
    assert_delay(Role::Player, 10);
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(2));
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(2));
}

#[test]
fn a_preview_and_a_playback_slot_are_never_delayed() {
    let _guard = HostGuard::new();
    let f = small(false);
    tick(&f.root);
    delayed_chord(&f, Role::Player, 10, false);
    assert_eq!(poll(Role::Player), None, "configured delay is active for an ordinary Play");
    chord(Role::Player);
    tick(&f.root);
    host::request_preview(&f.active, Role::Player, 0, 4);
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    host::stop_preview();
    tick(&f.root);
    assert!(host::request_drill_preview(&f.active, 4, crate::model::trim::Trim::Off));
    tick(&f.root);
    assert_eq!(poll(Role::Player).map(|f| f.buttons), Some(1));
    host::stop_preview();
    tick(&f.root);
    host::request_clip_preview(recording("Editor clip", 4), Role::Cpu);
    assert!(matches!(tick(&f.root).home, Some(host::HomeWarp::Start { .. })));
    host::warp_landed();
    tick(&f.root);
    assert_eq!(poll(Role::Cpu).map(|f| f.buttons), Some(1));
    host::stop_preview();
    tick(&f.root);
    host::warp_landed();
    tick(&f.root);
    host::set_slot(0, Some(&recording("Slot", 4))).unwrap();
    assert!(host::slot_start_mash(0, host::slot_take(0).unwrap(), 1).unwrap().is_some());
    assert_eq!(host::slot_control(false).unwrap().buttons, 1);
}

#[test]
fn a_stop_during_the_delay_cancels_the_run() {
    let _guard = HostGuard::new();
    for (cause, role) in [("stop", Role::Player), ("mash", Role::Cpu), ("load", Role::Player), ("reset", Role::Cpu)] {
        host::reset_host();
        let f = small(false);
        tick(&f.root);
        delayed_chord(&f, role, 10, false);
        assert_eq!(poll(role), None, "the run is delayed before {cause}");
        match cause {
            "stop" => chord(role),
            "mash" => host::note_mash(),
            "load" => host::note_load(),
            "reset" => host::clear_playback(crate::model::input_delay::Clear::TrainingReset),
            _ => unreachable!(),
        }
        tick(&f.root);
        for _ in 0..12 { assert_eq!(poll(role), None, "{cause}"); }
        assert_eq!(host::playback_roles(), [false, false], "{cause}");
        host::set_playback_delay(0);
        chord(role);
        tick(&f.root);
        assert_eq!(poll(role).map(|f| f.buttons), Some(1), "new run after {cause}");
    }
}

#[test]
fn playback_delay_is_none_by_default() {
    assert_eq!(RecorderSettings::factory().delay, 0);
    assert_eq!(Snapshot::factory().get("playback_delay"), Some(&[1, 0, 0, 0, 0, 0, 0, 0][..]));
}

#[test]
fn an_old_settings_file_reads_no_playback_delay() {
    let old = settings::parse(r#"{"menu":{"recording_crop":[0,1,0,0]}}"#);
    assert_eq!(RecorderSettings::from_snapshot(&old.settings.menu).delay, 0);
}

#[test]
fn playback_delay_is_one_path_for_training_and_the_waiting_room() {
    let host_source = include_str!("../src/model/rr_host.rs");
    let runtime = include_str!("../src/runtime/features/recording_host.rs");
    let recorder = include_str!("../src/runtime/features/recorder.rs");
    assert!(runtime.contains("rr_host::set_playback_delay(super::recorder::settings().delay)"));
    assert!(host_source.contains("fn pop_frame(role: Role)"));
    assert!(recorder.contains("rr_host::cpu_controller"));
    assert!(recorder.contains("if !(native_training || scene::lobby_term())"));
    assert!(recorder.contains("recording_host::on_player_poll(out)"));
    assert!(host_source.contains("pub unsafe fn on_player_poll"));
}

#[test]
fn the_held_files_heap_is_reported() {
    let _guard = HostGuard::new();
    for (both, shared) in [(false, false), (true, false), (true, true)] {
        host::reset_host();
        let f = fixture(true, LoopCount::Finite(1), &SESSION_FRAMES);
        let player = &f.ids[21];
        let cpu = if shared { player } else { &f.ids[20] };
        let library = Library::new(f.root.join(MACROS_DIR));
        for id in [player, cpu] {
            let mut recording = library.load(id).unwrap();
            recording.set_start_save(Some(&f.save)).unwrap();
            library.overwrite(id, &recording).unwrap();
        }
        let mut index = RecordingIndex::load(&f.root).unwrap();
        index.set_active(Role::Player, Some(player), 30).unwrap();
        index.set_active(Role::Cpu, Some(cpu), 31).unwrap();
        index.save(&f.root).unwrap();
        tick(&f.root);
        chord(Role::Player);
        tick(&f.root);
        host::warp_landed();
        tick(&f.root);
        if both {
            chord(Role::Cpu);
            tick(&f.root);
            host::warp_landed();
            tick(&f.root);
        }
        let live = host::playback_roles();
        assert_eq!(live, [true, both]);
        let measured = crate::cache_drop_bytes(host::drop_cached_book);
        assert!(measured.is_some(), "a held book exists");
        let (heap, inline) = measured.unwrap();
        let bytes = heap + inline;
        writeln!(std::io::stdout(), "held files: {heap} heap + {inline} inline = {bytes} bytes; player {player} 214 frames, cpu {} {}, roles {live:?}", if both { cpu } else { "-" }, if both { if shared { 214 } else { 201 } } else { 0 }).unwrap();
        assert!(bytes < 256 * 1024);
    }
}

#[test]
fn cpu_controller_holds_neutral_until_a_buffered_short_macro_starts() {
    let _guard = HostGuard::new();
    let f = fixture(false, LoopCount::Finite(1), &[2]);
    tick(&f.root);
    let controller = || host::cpu_controller(host::take_cpu_frame);
    assert_eq!(controller(), None);
    delayed_chord(&f, Role::Cpu, 10, false);
    for poll in 0..10 {
        assert_eq!(controller(), Some(crate::model::recording::Inject::NEUTRAL), "held CPU poll {poll}");
    }
    assert_eq!(controller().map(|pad| pad.buttons), Some(1));
    assert_eq!(controller().map(|pad| pad.buttons), Some(2));
    assert_eq!(controller(), None, "after the tail, the stock CPU regains control");
}

#[test]
fn player_one_is_neutral_during_its_own_delay() {
    let _guard = HostGuard::new();
    let f = small(false);
    tick(&f.root);
    delayed_chord(&f, Role::Player, 10, false);
    let pressed = crate::model::input_delay::MappedInputs { buttons: 3, lstick_x: 80, ..Default::default() };
    for _ in 0..10 {
        let mut out = pressed;
        assert!(!unsafe { host::on_player_poll(&mut out) });
        assert_eq!(out, crate::model::input_delay::MappedInputs::default());
    }
    let mut out = pressed;
    assert!(unsafe { host::on_player_poll(&mut out) });
    assert_eq!(out.buttons, 1);
    host::reset_host();
    let f = small(false);
    tick(&f.root);
    delayed_chord(&f, Role::Cpu, 10, false);
    for _ in 0..10 {
        let mut out = pressed;
        assert!(!unsafe { host::on_player_poll(&mut out) });
        assert_eq!(out, pressed);
        assert_eq!(poll(Role::Cpu), None);
    }
}

#[test]
fn the_start_speed_waits_for_the_first_input_after_the_delay() {
    let _guard = HostGuard::new();
    let f = small(true);
    let library = Library::new(f.root.join(MACROS_DIR));
    let mut rec = library.load(&f.active).unwrap();
    rec.start.speed_x = 1.25;
    rec.start.speed_y = -0.5;
    library.overwrite(&f.active, &rec).unwrap();
    tick(&f.root);
    delayed_chord(&f, Role::Player, 10, true);
    let mut out = crate::model::input_delay::MappedInputs::default();
    for _ in 0..10 {
        assert!(!unsafe { host::on_player_poll(&mut out) });
    }
    assert!(unsafe { host::on_player_poll(&mut out) });
    assert_eq!(host::take_start_speed(Role::Player).map(|speed| speed.speed), Some((1.25, -0.5)));
    let source = include_str!("../src/runtime/features/recording_host.rs");
    let player = source.split("pub unsafe fn on_player_poll(").nth(1).unwrap().split("\n}").next().unwrap();
    assert!(player.contains("if rr_host::on_player_poll(out)") && player.contains("apply_start_speed("));
}
