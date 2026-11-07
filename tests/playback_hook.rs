//! The playback machine allocates nothing inside a hook. This crate installs the plugin's counting allocator.
#![allow(unused)]

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[path = "support/playback_engine.rs"]
#[allow(dead_code)]
mod engine;

use model::hook_stats::{hook_allocations, CountingAllocator, HookId, HookScope};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn playback_makes_no_hook_allocations() {
    use engine::library::{LoopCount, RecordingIndex, Role};
    use engine::macros::{InputFrame, MacroRecording, StartState};
    use engine::playback::{Book, Playback};
    use model::state::settings::StateSettings;

    let mut index = RecordingIndex::fresh();
    index.set_active(Role::Player, Some("chain-a"), 1).unwrap();
    index.set_loop(Role::Player, LoopCount::Finite(2));
    index.bind_save(Some("save-a"), 1).unwrap();
    let mut book = Book::new(index);
    for (id, name, button) in [("macro-a", "A", 1u32), ("macro-b", "B", 2), ("macro-c", "C", 3)] {
        let frame = InputFrame { buttons: button, ..InputFrame::default() };
        book.add_recording(id, MacroRecording::new(name, "13.0.5", StartState::neutral(1, 0, 1.0), vec![frame]).unwrap());
    }
    book.add_chain("chain-a", engine::library::Chain::new("Chain", vec!["macro-a".into(), "macro-b".into(), "macro-c".into()]).unwrap());
    book.add_save("save-a");
    let settings = StateSettings { autoload: true, ..StateSettings::default() };
    let mut machine = Playback::new();

    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::RecorderPoll);
        let started = machine.play_chord(Role::Player, &book);
        std::hint::black_box(started.warp);
        assert!(started.started);
        let mut frames = 0u32;
        let mut buttons = [0u32; 6];
        while let Some(played) = machine.step(Role::Player, &book) {
            std::hint::black_box(played.recording_id);
            buttons[frames as usize] = played.frame.buttons;
            frames += 1;
            assert!(frames <= 6, "two passes of three steps");
        }
        assert_eq!(frames, 6, "a three-step chain played two loop passes");
        assert_eq!(&buttons, &[1, 2, 3, 1, 2, 3]);
        let loaded = machine.load(&book, &settings).unwrap();
        std::hint::black_box(loaded.save_id);
        assert_eq!(loaded.save_id, "save-a");
        let auto = machine.auto_load_on_death(&book, &settings).unwrap();
        std::hint::black_box(auto.save_id);
        assert_eq!(auto.save_id, "save-a");
        assert!(!machine.playing(Role::Player));
        machine.play_chord(Role::Player, &book);
        assert!(machine.mash(Role::Player, true));
        assert!(!machine.playing(Role::Player));
        assert!(machine.step(Role::Player, &book).is_none());
    }
    assert_eq!(hook_allocations(), before, "play, step, load, auto-load and mash allocated inside the hook");
}
