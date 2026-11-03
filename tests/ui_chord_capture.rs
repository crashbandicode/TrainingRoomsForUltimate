//! N7 original physical-input regression: never injects Menu::chord_entry.
//! Delivery uses the same adapter/whole-poll ring/capture boundary as the worker.
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::adapter::{bit, Adapter, Style};
use model::menu::chord::Chord;
use model::menu::input::Button;
use model::menu::table::ROWS;
use model::menu::{Entry, Env, Menu, Screen};
use model::ui_events::{Event, Packet, Ring};

struct Allocator;
thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
// Host-thread attribution only; no TLS or allocator is added to the plugin.
unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        TRACK.with(|track| if track.get() { ALLOCS.with(|count| count.set(count.get() + 1)); });
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, ptr, layout); }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn shortcut_editor() -> Menu {
    let mut menu = Menu::new(model::settings::Settings::factory(), Env::default());
    let row = ROWS.iter().position(|row| row.title == "Menu shortcut 1").unwrap();
    for _ in 0..6 {
        if menu.page() == ROWS[row].page() { break; }
        menu.press(Button::R);
    }
    assert_eq!(menu.page(), ROWS[row].page());
    for _ in 0..menu.entry_count(menu.page()) {
        if menu.focus() == Some(Entry::Row(row)) { break; }
        menu.press(Button::Down);
    }
    assert_eq!(menu.focus(), Some(Entry::Row(row)));
    menu.press(Button::A);
    assert!(matches!(menu.screen(), Some(Screen::ChordEntry { .. })));
    menu
}

fn physical_poll(menu: &mut Menu, adapter: &mut Adapter, raw: u32) {
    physical_poll_style(menu, adapter, raw, Style::Pro);
}

fn physical_poll_style(menu: &mut Menu, adapter: &mut Adapter, raw: u32, style: Style) {
    let step = adapter.poll(raw, style, true, false, menu.env().shortcuts);
    let ring = Ring::<4>::new();
    assert!(ring.push(ring.generation(), Event::Poll(Packet::from_step(&step))));
    let (generation, Event::Poll(packet)) = ring.pop().unwrap() else { panic!("wrong event") };
    assert_eq!(generation, ring.generation());
    for button in packet.presses(menu) { menu.press(button); }
}

#[test]
fn n7_physical_zl_up_is_captured_without_direct_model_injection() {
    let mut menu = shortcut_editor();
    let mut adapter = Adapter::new();
    for raw in [1 << bit::X, 0, 1 << bit::ZL,
            (1 << bit::ZL) | (1 << bit::DPAD_UP), 0] {
        physical_poll(&mut menu, &mut adapter, raw);
    }
    // Independent spec wire-order oracle: logical ZL bit6, D-pad Up bit8.
    let expected = Chord((1 << 6) | (1 << 8));
    assert!(matches!(menu.screen(), Some(Screen::ChordEntry { captured, .. }) if *captured == expected),
        "physical polls must replace the preview, not just navigate: {:?}", menu.screen());
    assert_eq!(menu.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1,
        "capture alone must not confirm an edit");
    physical_poll(&mut menu, &mut adapter, 1 << bit::A);
    assert_eq!(menu.env().shortcuts[0], expected);
    assert_eq!(menu.take_outbox().button_config, Some([expected, Chord::NONE]));
}

fn captured(menu: &Menu) -> Chord {
    let Some(Screen::ChordEntry { captured, .. }) = menu.screen() else { panic!("not an editor") };
    *captured
}

fn capture(menu: &mut Menu, adapter: &mut Adapter, raw: u32, style: Style) {
    for buttons in [1 << bit::X, 0, raw, raw, 0] {
        physical_poll_style(menu, adapter, buttons, style);
    }
}

#[test]
fn n7_all_sixteen_logical_buttons_are_data_not_preview_controls() {
    // Independent SPEC-input-04 wire map in chord declaration order.
    let pro = [5, 6, 4, 7, 8, 9, 10, 11, 0, 2, 3, 1, 18, 19, 16, 17];
    for (logical, raw) in pro.into_iter().enumerate() {
        let mut menu = shortcut_editor();
        let mut adapter = Adapter::new();
        capture(&mut menu, &mut adapter, 1 << raw, Style::Pro);
        assert_eq!(captured(&menu), Chord(1 << logical), "logical button {logical}");
        assert!(menu.take_button_config().is_none(), "capture did not confirm");
        physical_poll(&mut menu, &mut adapter, 1 << bit::A);
        assert_eq!(menu.env().shortcuts[0], Chord(1 << logical));
        let wanted = menu.take_button_config();
        if logical == 13 { assert!(wanted.is_none(), "Minus was unchanged"); }
        else { assert_eq!(wanted, Some([Chord(1 << logical), Chord::NONE])); }
        assert!(menu.take_button_config().is_none(), "exactly once");
        assert!(!menu.is_closed(), "editor confirm does not close the whole menu");
    }
}

#[test]
fn n7_arming_packet_consumes_buttons_already_held_and_all_releases() {
    let mut menu = shortcut_editor();
    let mut adapter = Adapter::new();
    for raw in [(1 << bit::X) | (1 << bit::A) | (1 << bit::B), 1 << bit::A, 0] {
        physical_poll(&mut menu, &mut adapter, raw);
        assert!(menu.take_button_config().is_none());
    }
    assert_eq!(captured(&menu), Chord::DEFAULT_SHORTCUT_1);
    for raw in [1 << bit::ZL, (1 << bit::ZL) | (1 << bit::DPAD_RIGHT), 1 << bit::ZL, 0] {
        physical_poll(&mut menu, &mut adapter, raw);
    }
    assert_eq!(captured(&menu), Chord((1 << 6) | (1 << 11)));
    assert_eq!(menu.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1);
    physical_poll(&mut menu, &mut adapter, 1 << bit::B);
    assert!(menu.screen().is_none());
    assert!(menu.take_button_config().is_none(), "preview cancellation never publishes");
}

#[test]
fn n7_controller_styles_keep_the_spec_logical_mapping_and_unmapped_buttons_absent() {
    for (style, raw, expected) in [
        (Style::Handheld, (1 << 12) | (1 << 13), (1 << 6) | (1 << 7)),
        (Style::DualJoyCon, (1 << 14) | (1 << 15), (1 << 6) | (1 << 7)),
        (Style::GameCube, (1 << 28) | (1 << 29) | (1 << 11), (1 << 6) | (1 << 7) | (1 << 5)),
        (Style::GameCube, (1 << 8) | (1 << 9), (1 << 6) | (1 << 7)),
        (Style::GameCube, (1 << 19) | (1 << 5), 1), // no physical Minus mapping
    ] {
        let mut menu = shortcut_editor();
        let mut adapter = Adapter::new();
        capture(&mut menu, &mut adapter, raw, style);
        assert_eq!(captured(&menu), Chord(expected));
    }
}

#[test]
fn n7_existing_collision_rules_refuse_without_losing_the_preview_or_originals() {
    for (raw, other, why) in [
        ((1 << bit::B) | (1 << bit::DPAD_UP), Chord::NONE, model::ini_files::EditRefusal::Permanent),
        (1 << bit::Y, Chord(1 << 3), model::ini_files::EditRefusal::SameAsOther),
    ] {
        let mut menu = shortcut_editor();
        let settings = menu.settings().clone();
        menu.reopen(settings, Env { shortcuts: [Chord::DEFAULT_SHORTCUT_1, other], ..Env::default() });
        // The saved page is not a cursor selection; use the same actual row route again.
        let row = ROWS.iter().position(|r| r.title == "Menu shortcut 1").unwrap();
        while menu.page() != ROWS[row].page() { menu.press(Button::R); }
        while menu.focus() != Some(Entry::Row(row)) { menu.press(Button::Down); }
        menu.press(Button::A);
        let mut adapter = Adapter::new();
        capture(&mut menu, &mut adapter, raw, Style::Pro);
        physical_poll(&mut menu, &mut adapter, 1 << bit::A);
        assert_eq!(menu.chord_refusal(), Some(why));
        assert!(menu.screen().is_some());
        assert_eq!(menu.env().shortcuts, [Chord::DEFAULT_SHORTCUT_1, other]);
        assert!(menu.take_button_config().is_none());
    }
}

#[test]
fn n7_reopen_cancels_unconfirmed_capture_and_y_disables_only_on_confirm() {
    let mut menu = shortcut_editor();
    let mut adapter = Adapter::new();
    capture(&mut menu, &mut adapter, 1 << bit::R, Style::Pro);
    let settings = menu.settings().clone();
    menu.reopen(settings, Env::default());
    assert!(menu.screen().is_none());
    assert_eq!(menu.chord_capture(), model::menu::ChordCapture::Preview);
    assert_eq!(menu.env().shortcuts[0], Chord::DEFAULT_SHORTCUT_1);
    assert!(menu.take_button_config().is_none());
    menu.press(Button::A);
    assert!(matches!(menu.screen(), Some(Screen::ChordEntry { .. })));
    menu.press(Button::Y);
    assert_eq!(captured(&menu), Chord::NONE);
    assert!(menu.take_button_config().is_none());
    menu.press(Button::A);
    assert_eq!(menu.take_button_config(), Some([Chord::NONE, Chord::NONE]));
    assert!(menu.take_outbox().button_config.is_none(), "close cannot rewrite the consumed effect");
}

#[test]
fn n7_ring_full_and_session_retirement_cannot_deliver_old_confirmation() {
    let ring = Ring::<2>::new();
    let old = ring.generation();
    let mut adapter = Adapter::new();
    let step = adapter.poll(1 << bit::A, Style::Pro, true, false, [Chord::NONE; 2]);
    let confirm = Event::Poll(Packet::from_step(&step));
    assert!(ring.push(old, Event::Open));
    assert!(ring.push(old, confirm));
    assert!(!ring.push(old, confirm), "full packet is rejected atomically");
    ring.reset(); // cancellation never needs a free slot
    assert_ne!(ring.generation(), old);
    assert!(!ring.push(old, confirm), "an in-flight old producer is stale too");
    assert_eq!(ring.pop(), Some((old, Event::Open)));
    assert_eq!(ring.pop(), Some((old, confirm)));
    assert!(ring.pop().is_none());
    let current = ring.generation();
    assert!(ring.push(current, Event::Open));
    assert_eq!(ring.pop(), Some((current, Event::Open)));
    for _ in 0..100 { // physical ring wraps without reusing any old generation
        assert!(ring.push(current, confirm));
        assert_eq!(ring.pop(), Some((current, confirm)));
    }
}

#[test]
fn n7_ordered_ring_keeps_complete_poll_fields_across_real_host_threads() {
    let ring = Ring::<8>::new();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut adapter = Adapter::new();
            for i in 0..2048 {
                let raw = 1 << if i % 2 == 0 { bit::A } else { bit::B };
                let step = adapter.poll(raw, Style::Pro, true, false, [Chord::NONE; 2]);
                let event = Event::Poll(Packet::from_step(&step));
                while !ring.push(0, event) { std::thread::yield_now(); }
            }
        });
        for i in 0..2048 {
            let event = loop {
                if let Some(event) = ring.pop() { break event; }
                std::thread::yield_now();
            };
            let (generation, Event::Poll(packet)) = event else { panic!("wrong event") };
            let expected = if i % 2 == 0 { 1 } else { 2 }; // spec logical A/B bits
            assert_eq!((generation, packet.held, packet.pressed), (0, expected, expected));
        }
    });
    assert!(ring.pop().is_none());
}

#[test]
fn n7_adapter_and_ring_first_and_repeated_hook_operations_allocate_nothing() {
    let ring = Ring::<4>::new();
    let mut adapter = Adapter::new();
    TRACK.with(|v| v.set(false));
    ALLOCS.with(|v| v.set(0));
    struct End;
    impl Drop for End { fn drop(&mut self) { TRACK.with(|v| v.set(false)); } }
    TRACK.with(|v| v.set(true));
    let end = End;
    for raw in [1 << bit::X, 0, 1 << bit::A, 0, 1 << bit::PLUS, 0, 1 << bit::STICK_R] {
        let step = adapter.poll(raw, Style::Pro, true, false, [Chord::NONE; 2]);
        assert!(ring.push(ring.generation(), Event::Poll(Packet::from_step(&step))));
        assert!(ring.pop().is_some());
    }
    ring.reset();
    drop(end);
    assert_eq!(ALLOCS.with(|v| v.get()), 0);
}

#[test]
fn n7_runtime_wiring_uses_whole_polls_raw_plus_and_confirmation_not_close() {
    let input = include_str!("../src/runtime/ui/input.rs");
    let worker = include_str!("../src/runtime/ui/worker.rs");
    let delay = include_str!("../src/runtime/features/input_delay.rs");
    let state = include_str!("../src/runtime/ui/state.rs");
    assert!(input.contains("Packet::from_step(&step)"));
    assert!(input.contains("step.changed || step.presses().next().is_some()"));
    assert!(!input.contains("push_press("));
    assert!(worker.contains("let buttons = packet.presses(menu)"));
    assert!(worker.contains("if generation == session.input_generation"));
    assert!(state.contains("worker::push_reset()"));
    let note = delay.split("fn note(").nth(1).unwrap().split("/// Caller A").next().unwrap();
    assert!(note.find("ui::input::on_p1_poll(state)").unwrap() < note.find("chords::on_poll(controller)").unwrap());
    assert_eq!(delay.matches("ui::input::on_p1_poll(").count(), 1);
    let close = worker.split("fn close(").nth(1).unwrap();
    assert!(!close.contains("outbox.button_config"), "whole close must not own an immediate INI effect");
    assert!(close.contains("INPUT.reset()"), "close invalidates polls queued before the next opening");
    assert!(worker.contains("menu.take_button_config()"));
    assert!(worker.contains("chords::commit_shortcut(slot, chord)"));
}

#[test]
fn n7_confirmed_ini_survives_a_fresh_process_and_preserves_unrelated_text() {
    use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
    const MODE: &str = "TR_UI_N7_PROCESS_MODE";
    const ROOT: &str = "TR_UI_N7_PROCESS_ROOT";
    let expected = [Chord((1 << 6) | (1 << 8)), Chord::NONE];
    if let Ok(mode) = std::env::var(MODE) {
        let root = std::path::PathBuf::from(std::env::var_os(ROOT).unwrap());
        let path = root.join("button_config.ini");
        match mode.as_str() {
            "save" => {
                let mut menu = shortcut_editor();
                let mut adapter = Adapter::new();
                capture(&mut menu, &mut adapter, (1 << bit::ZL) | (1 << bit::DPAD_UP), Style::Pro);
                assert!(menu.take_button_config().is_none());
                physical_poll(&mut menu, &mut adapter, 1 << bit::A);
                let slots = menu.take_button_config().expect("actual physical confirmation");
                assert_eq!(slots, expected);
                assert!(!menu.is_closed(), "persist before whole-menu close");
                model::ini_files::write_shortcuts(&path, model::menu_shortcuts::MenuShortcuts::new(slots[0], slots[1])).unwrap();
                fs::write(root.join("saver.json"), format!("{{\"pid\":{},\"saved_after_physical_confirm\":true,\"time_ns\":{}}}",
                    std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())).unwrap();
            }
            "restore" => {
                let (loaded, error) = model::ini_files::load_shortcuts(&path);
                assert!(error.is_none());
                assert_eq!(loaded.slots(), expected);
                let text = fs::read_to_string(&path).unwrap();
                assert!(text.contains("# private note\n") && text.contains("other_key=untouched\n")
                    && text.contains("[foreign]\ncolour=blue\n"));
                let mut adapter = Adapter::new();
                assert!(adapter.poll((1 << bit::ZL) | (1 << bit::DPAD_UP), Style::Pro, false, true, loaded.slots()).open);
                assert!(!Adapter::new().poll(1 << bit::MINUS, Style::Pro, false, true, loaded.slots()).open);
                assert!(Adapter::new().poll((1 << bit::B) | (1 << bit::DPAD_UP), Style::Pro, false, true, loaded.slots()).open);
                fs::write(root.join("restorer.json"), format!("{{\"pid\":{},\"nondefault_slots_restored\":true,\"time_ns\":{}}}",
                    std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos())).unwrap();
            }
            _ => panic!("unknown process mode"),
        }
        return;
    }
    let root = std::env::temp_dir().join(format!("tr-ui-n7-continuation-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("button_config.ini"), "# private note\n[menu]\nshortcut_1=Minus\nshortcut_2=disabled\nother_key=untouched\n[foreign]\ncolour=blue\n").unwrap();
    let exe = std::env::current_exe().unwrap();
    let run = |mode| Command::new(&exe).args(["--exact", "n7_confirmed_ini_survives_a_fresh_process_and_preserves_unrelated_text", "--nocapture"])
        .env(MODE, mode).env(ROOT, &root).output().unwrap();
    let saved = run("save");
    assert!(saved.status.success(), "saver failed: {}", String::from_utf8_lossy(&saved.stderr));
    let saver_exit = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    // This launch is after output()/successful saver exit, not a same-process round trip.
    let restore_launch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let restored = run("restore");
    assert!(restored.status.success(), "restorer failed: {}", String::from_utf8_lossy(&restored.stderr));
    println!("N7_CONTINUATION saver_exit_ns={saver_exit} restore_launch_ns={restore_launch} saver={} restorer={}",
        fs::read_to_string(root.join("saver.json")).unwrap(), fs::read_to_string(root.join("restorer.json")).unwrap());
    assert!(saver_exit <= restore_launch);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn n7_failed_ini_write_keeps_confirmed_session_value_and_old_file() {
    use std::fs;
    let root = std::env::temp_dir().join(format!("tr-ui-n7-write-failure-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("button_config.ini");
    let before = "# keep me\n[menu]\nshortcut_1=Minus\nshortcut_2=disabled\nforeign=42\n";
    fs::write(&path, before).unwrap();
    // Occupy the actual transaction's temporary path: no permissions/root assumption.
    fs::create_dir(root.join("button_config.ini.tmp")).unwrap();
    let mut menu = shortcut_editor();
    let mut adapter = Adapter::new();
    capture(&mut menu, &mut adapter, 1 << bit::R, Style::Pro);
    physical_poll(&mut menu, &mut adapter, 1 << bit::A);
    let slots = menu.take_button_config().unwrap();
    assert!(model::ini_files::write_shortcuts(&path, model::menu_shortcuts::MenuShortcuts::new(slots[0], slots[1])).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    assert_eq!(menu.env().shortcuts, slots, "writer failure does not silently undo session publication");
    assert!(!menu.is_closed());
    fs::remove_dir_all(root).unwrap();
}
