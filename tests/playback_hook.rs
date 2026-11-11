//! The playback machine allocates nothing inside a hook. This crate installs the plugin's counting allocator.
#![allow(unused)]

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[path = "support/playback_engine.rs"]
#[allow(dead_code)]
pub mod engine;

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
        let started = machine.play_file(Role::Player, &book, "macro-a");
        std::hint::black_box(started.started);
        if let Some(played) = machine.step(Role::Player, &book) {
            std::hint::black_box(played.frame.buttons);
        }
        let mut window = engine::macros::FrameWindow::new(8).unwrap();
        std::hint::black_box(window.set_in(2));
        std::hint::black_box(window.set_out(6));
    }
    assert_eq!(hook_allocations(), before, "play, step, load, auto-load and mash allocated inside the hook");
}

#[test]
fn cpu_role_record_and_playback_frames_allocate_nothing() {
    use model::input_delay::MappedInputs;
    use model::recording::{inject_values, ControlCtx, CpuRoleDrive, Phase, FLICK_JUMP, LOCKOUT_FRAMES};

    let mut drive = CpuRoleDrive::idle();
    drive.begin(1);
    let ctx = ControlCtx { facing: 1, shulk_dial: false };
    let before = hook_allocations();
    {
        let _poll = HookScope::enter_hook(HookId::RecorderPoll);
        let _ai = HookScope::enter_hook(HookId::RecorderAi);
        for _ in 0..LOCKOUT_FRAMES {
            let mut out = MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL };
            std::hint::black_box(drive.poll(&mut out));
            std::hint::black_box(drive.control(&ctx).write);
        }
        let mut baseline = MappedInputs { buttons: FLICK_JUMP, ..MappedInputs::NEUTRAL };
        std::hint::black_box(drive.poll(&mut baseline));
        std::hint::black_box(drive.control(&ctx).write);
        let jab = [36_872u32, 36_872, 36_872, 33_281];
        for buttons in jab {
            let mut out = MappedInputs { buttons, ..MappedInputs::NEUTRAL };
            let stored = drive.poll(&mut out).unwrap_or(MappedInputs::NEUTRAL);
            let write = drive.control(&ctx).write;
            let playback = inject_values(stored, 1);
            std::hint::black_box((write, playback.buttons));
        }
    }
    assert_eq!(hook_allocations(), before, "cpu-role record and playback frames allocated inside the hook");
    assert_eq!(drive.phase(), Phase::Recording);
}

/// The AI epilogue locks the CPU-role drive on the first training frame, before any worker.
/// That lock is `hos_lock`: a `std::sync::Mutex` allocates one OnceBox on Switch and the hook
/// counter charges it to `recorder_ai`.
#[test]
fn the_cpu_role_drive_lock_allocates_nothing_in_recorder_ai() {
    use engine::library::Role;
    use model::recording::ControlCtx;

    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::RecorderAi);
        engine::rr_host::cpu_role_after_chord(1);
        engine::rr_host::set_record_role(Role::Cpu);
        engine::rr_host::on_chords(true, false, false);
        engine::rr_host::cpu_role_after_chord(1);
        let ctx = ControlCtx { facing: 1, shulk_dial: false };
        std::hint::black_box(engine::rr_host::cpu_role_control(&ctx));
        std::hint::black_box(engine::rr_host::cpu_role_phase());
        engine::rr_host::on_chords(true, false, false);
        engine::rr_host::cpu_role_after_chord(1);
        engine::rr_host::set_record_role(Role::Player);
    }
    assert_eq!(hook_allocations(), before, "the AI hook's CPU-role lock allocated");
}
