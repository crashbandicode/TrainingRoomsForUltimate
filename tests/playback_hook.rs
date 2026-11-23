//! The playback machine allocates nothing inside a hook. This crate installs the plugin's counting allocator.
#![allow(unused)]

use crate::model;

use crate::engine;

use model::hook_stats::{hook_allocations, HookId, HookScope};

#[test]
fn f4_cpu_role_live_input_does_not_depend_on_facing_or_dial() {
    use model::input_delay::MappedInputs;
    use model::recording::{CpuRoleDrive, Phase, FLICK_JUMP, LOCKOUT_FRAMES};

    for hold in [80, -80] {
        let mut drive = CpuRoleDrive::idle();
        drive.begin_placed();
        let input = MappedInputs { buttons: 42, lstick_x: hold, ..MappedInputs::NEUTRAL };
        let mut player = input;
        assert_eq!(drive.poll(&mut player), Some(input));
        assert_eq!(player, MappedInputs::NEUTRAL);
        let write = drive.control().write.unwrap();
        assert_eq!(write.stick_x, f32::from(hold) / 120.0);
        assert_eq!(write.clamped_x, f32::from(hold) / 120.0);
        assert_eq!(write.buttons, 42);
    }

    let mut drive = CpuRoleDrive::idle();
    drive.begin();
    for _ in 0..LOCKOUT_FRAMES {
        let mut player = MappedInputs { buttons: 42, ..MappedInputs::NEUTRAL };
        assert_eq!(drive.poll(&mut player), None);
        assert_eq!(player, MappedInputs::NEUTRAL);
        let out = drive.control();
        assert!(out.write.is_none());
        assert!(out.hold_cliff_timer);
    }
    assert_eq!(drive.phase(), Phase::Standby);
    let mut player = MappedInputs { buttons: FLICK_JUMP, lstick_x: 20, ..MappedInputs::NEUTRAL };
    assert_eq!(drive.poll(&mut player), None);
    assert_eq!(player, MappedInputs::NEUTRAL);
    let out = drive.control();
    assert!(out.hold_cliff_timer);
    let write = out.write.unwrap();
    assert_eq!(write.stick_x, 20.0 / 120.0);
    assert_eq!(write.clamped_x, 0.0);
    assert_eq!(write.buttons, FLICK_JUMP);
}

#[test]
fn f4_cpu_role_held_stick_turns_only_on_direction_change() {
    use model::input_delay::MappedInputs;
    use model::recording::CpuRoleDrive;

    let mut drive = CpuRoleDrive::idle();
    drive.begin_placed();
    let mut facing = -1;
    let mut turns = 0;
    let mut mismatches = 0;
    for hold in [80, -80] {
        for _ in 0..60 {
            let mut player = MappedInputs { lstick_x: hold, ..MappedInputs::NEUTRAL };
            assert!(drive.poll(&mut player).is_some());
            let write = drive.control().write.unwrap();
            if write.stick_x != f32::from(hold) / 120.0 {
                mismatches += 1;
            }
            let next = if write.stick_x > 0.0 { 1 } else { -1 };
            if next != facing {
                turns += 1;
                facing = next;
            }
        }
    }
    assert_eq!(turns, 2);
    assert_eq!(mismatches, 0);
}

#[test]
fn placed_cpu_recording_has_no_lockout_or_standby() {
    use model::input_delay::MappedInputs;
    use model::recording::{CpuRoleDrive, Phase};
    let mut drive = CpuRoleDrive::idle();
    drive.begin_placed();
    assert_eq!(drive.phase(), Phase::Recording);
    let first = MappedInputs { buttons: 42, lstick_x: 60, ..MappedInputs::NEUTRAL };
    let mut player = first;
    assert_eq!(drive.poll(&mut player), Some(first));
    assert_eq!(player, MappedInputs::NEUTRAL);
    let cpu = drive.control();
    assert_eq!(cpu.write.unwrap().buttons, 42);
    assert!(!cpu.hold_cliff_timer);
    drive.begin_placed();
    let mut neutral = MappedInputs::NEUTRAL;
    assert_eq!(drive.poll(&mut neutral), Some(MappedInputs::NEUTRAL));
}

#[test]
fn playback_slot_start_frames_post_and_stop_allocate_nothing() {
    use engine::macros::{InputFrame, MacroRecording, StartState};
    use engine::rr_host;

    let frames = vec![InputFrame { buttons: 1, ..InputFrame::default() }; 2];
    let recording = MacroRecording::new("Slot", "13.0.5", StartState::neutral(7, 0, -1.0), frames).unwrap();
    rr_host::set_slot(0, Some(&recording)).unwrap();
    let _turn = crate::allocation::hook_counter_turn();
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::RecorderAi);
        assert!(rr_host::slot_start_mash(0, rr_host::slot_take(0).unwrap(), 1).unwrap().is_some());
        assert_eq!(rr_host::slot_control(false).unwrap().buttons, 1);
        assert_eq!(rr_host::slot_post(), None);
        assert_eq!(rr_host::slot_control(false).unwrap().buttons, 1);
        assert_eq!(rr_host::slot_post(), Some(0));
        assert!(rr_host::slot_start_mash(0, rr_host::slot_take(0).unwrap(), 1).unwrap().is_some());
        assert!(rr_host::slot_stop());
    }
    assert_eq!(hook_allocations(), before, "slot start, frames, post or stop allocated inside the hook");
}

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

    let _turn = crate::allocation::hook_counter_turn();
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
    use model::recording::{inject_values, CpuRoleDrive, Phase, FLICK_JUMP, LOCKOUT_FRAMES};

    let mut drive = CpuRoleDrive::idle();
    drive.begin();
    let _turn = crate::allocation::hook_counter_turn();
    let before = hook_allocations();
    {
        let _poll = HookScope::enter_hook(HookId::RecorderPoll);
        let _ai = HookScope::enter_hook(HookId::RecorderAi);
        for _ in 0..LOCKOUT_FRAMES {
            let mut out = MappedInputs { buttons: 1, ..MappedInputs::NEUTRAL };
            std::hint::black_box(drive.poll(&mut out));
            std::hint::black_box(drive.control().write);
        }
        let mut baseline = MappedInputs { buttons: FLICK_JUMP, ..MappedInputs::NEUTRAL };
        std::hint::black_box(drive.poll(&mut baseline));
        std::hint::black_box(drive.control().write);
        let jab = [36_872u32, 36_872, 36_872, 33_281];
        for buttons in jab {
            let mut out = MappedInputs { buttons, ..MappedInputs::NEUTRAL };
            let stored = drive.poll(&mut out).unwrap_or(MappedInputs::NEUTRAL);
            let write = drive.control().write;
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

    let _turn = crate::allocation::hook_counter_turn();
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::RecorderAi);
        engine::rr_host::cpu_role_after_chord();
        engine::rr_host::set_record_role(Role::Cpu);
        engine::rr_host::on_chords(true, false, false);
        engine::rr_host::cpu_role_after_chord();
        std::hint::black_box(engine::rr_host::cpu_role_control());
        std::hint::black_box(engine::rr_host::cpu_role_phase());
        engine::rr_host::on_chords(true, false, false);
        engine::rr_host::cpu_role_after_chord();
        engine::rr_host::set_record_role(Role::Player);
    }
    assert_eq!(hook_allocations(), before, "the AI hook's CPU-role lock allocated");
}
