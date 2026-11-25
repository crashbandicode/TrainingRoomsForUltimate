lib_unit_tests! {
use super::*;
use crate::model::adapter::{bit, Adapter, Style};
use crate::model::frame::{self, Frame, Text};
use crate::model::screens::frame_of;
use crate::model::ui_events::{Event, Packet, Ring};

fn row(id: &str) -> usize {
    ROWS.iter().position(|r| r.id == id).unwrap()
}

fn menu(waiting_room: bool) -> Menu {
    Menu::new(Settings::factory(), Env { waiting_room, ..Env::default() })
}

fn open_slider(m: &mut Menu, id: &str, values: &[u8], upper: bool) {
    m.put("save_damage_cpu", &[0, 0, 1]);
    m.put("save_damage_player", &[0, 0, 1]);
    if let Some(scale) = super::super::tech_settings::scale(id) {
        m.put_one_hot(scale.mode, 1);
    }
    m.put(id, values);
    m.activate(row(id));
    if upper {
        m.press(Button::Down);
    }
    m.press(Button::A);
}

struct Input {
    adapter: Adapter,
    ring: Ring<8>,
}

impl Input {
    fn new() -> Self { Self { adapter: Adapter::new(), ring: Ring::new() } }
}

fn poll(m: &mut Menu, input: &mut Input, bits: u32) -> bool {
    input.adapter.set_jumps(m.takes_jumps());
    let step = input.adapter.poll(bits, Style::Pro, true, true, [Chord::NONE; 2]);
    let up = step.presses().any(|button| button == Button::Up);
    assert!(input.ring.push(input.ring.generation(), Event::Poll(Packet::from_step(&step))));
    let (_, Event::Poll(packet)) = input.ring.pop().unwrap() else { panic!("poll"); };
    for button in packet.presses(m) {
        m.press(button);
    }
    up
}

fn units(m: &Menu, id: &str, end: usize) -> u16 {
    let value = m.get(id);
    u16::from(value[end]) * 2 + u16::from(value.get(end + 2).copied().unwrap_or(0))
}

#[test]
fn slider_up_down_choose_min_or_max_and_a_edits() {
    for waiting_room in [false, true] {
        for (index, r) in ROWS.iter().enumerate().filter(|(_, r)| matches!(r.control, Control::Slider)) {
            let mut m = menu(waiting_room);
            m.put("save_damage_cpu", &[0, 0, 1]);
            m.put("save_damage_player", &[0, 0, 1]);
            if let Some(scale) = super::super::tech_settings::scale(r.id) {
                m.put_one_hot(scale.mode, 1);
            }
            assert_eq!(m.grey(index), None, "{}", r.id);
            m.activate(index);
            let state = |m: &Menu| match m.screen() {
                Some(Screen::Slider { state, .. }) => *state,
                other => panic!("{other:?}"),
            };
            assert_eq!(state(&m), SliderState::LowerHover, "{}", r.id);
            let mut frame = Frame::new();
            frame_of(&m, &mut frame, |_| "");
            assert_eq!(frame.hint.string_lossy(), "Up/Down Min or Max   A edit   B back");
            for (button, expected) in [
                (Button::Down, SliderState::UpperHover),
                (Button::Down, SliderState::LowerHover),
                (Button::Up, SliderState::UpperHover),
                (Button::Up, SliderState::LowerHover),
            ] {
                m.press(button);
                assert_eq!(state(&m), expected, "{}", r.id);
            }
            let before = m.get(r.id).to_vec();
            for button in [Button::Left, Button::Right] {
                assert!(!m.press_outcome(button));
                assert_eq!(state(&m), SliderState::LowerHover);
                assert_eq!(m.get(r.id), before.as_slice());
            }
            m.press(Button::A);
            assert_eq!(state(&m), SliderState::LowerSelected);
            frame_of(&m, &mut frame, |_| "");
            assert_eq!(frame.hint.string_lossy(), "Up/Down change   hold to speed up   A or B done");
            for button in [Button::Left, Button::Right] {
                assert!(!m.press_outcome(button));
                assert_eq!(state(&m), SliderState::LowerSelected);
                assert_eq!(m.get(r.id), before.as_slice());
            }
            m.press(Button::A);
            assert_eq!(state(&m), SliderState::LowerHover);
            m.press(Button::A);
            m.press(Button::B);
            assert_eq!(state(&m), SliderState::LowerHover);
            m.press(Button::B);
            assert!(m.screen().is_none());
        }
    }
}

#[test]
fn damage_range_tap_moves_half_a_percent() {
    for id in ["save_damage_limits_cpu", "save_damage_limits_player"] {
        let mut m = menu(false);
        open_slider(&mut m, id, &[0, 150], false);
        for expected in [&[0, 150, 1, 0][..], &[1, 150], &[1, 150, 1, 0]] {
            m.press(Button::Up);
            assert_eq!(m.get(id), expected, "{id}");
        }
        for expected in [&[1, 150][..], &[0, 150, 1, 0], &[0, 150]] {
            m.press(Button::Down);
            assert_eq!(m.get(id), expected, "{id}");
        }
    }
}

#[test]
fn damage_range_hold_quickens_through_the_adapter() {
    for (run, step) in [(0, 1), (1, 1), (2, 1), (3, 2), (6, 2), (7, 4), (12, 4),
        (13, 10), (22, 10), (23, 20), (255, 20)] {
        assert_eq!(slider_step(run), step, "run {run}");
    }
    let polls = [0, 21, 29, 37, 45, 53, 60, 65, 70, 75, 80, 85, 90, 95, 100, 105, 110, 115, 120,
        124, 128, 132, 136, 140, 144, 148, 152, 156, 160, 164, 168];
    let values = [1, 2, 4, 6, 8, 10, 12, 16, 20, 24, 28, 32, 40, 50, 60, 70, 80, 90, 100,
        110, 120, 130, 140, 160, 180, 200, 220, 240, 260, 280, 300];
    for waiting_room in [false, true] {
        let id = "save_damage_limits_cpu";
        let mut m = menu(waiting_room);
        open_slider(&mut m, id, &[0, 0], true);
        let mut adapter = Input::new();
        let mut expected = 0;
        let mut first_max = None;
        for tick in 0..=180 {
            if let Some(at) = polls.iter().position(|&p| p == tick) {
                expected = values[at];
            }
            poll(&mut m, &mut adapter, 1 << bit::DPAD_UP);
            assert_eq!(units(&m, id, 1), expected, "poll {tick}");
            if expected == 300 && first_max.is_none() {
                first_max = Some(tick);
            }
        }
        assert_eq!(first_max, Some(168));
        assert!(first_max.unwrap() <= 180);
        poll(&mut m, &mut adapter, 0);
        poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN);
        assert_eq!(units(&m, id, 1), 299);

        let mut m = menu(waiting_room);
        open_slider(&mut m, id, &[0, 0], true);
        let mut adapter = Input::new();
        for _ in 0..60 { poll(&mut m, &mut adapter, 1 << bit::DPAD_UP); }
        let before = units(&m, id, 1);
        poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN);
        assert_eq!(units(&m, id, 1), before - 1);

        let id = "save_damage_limits_player";
        let mut m = menu(waiting_room);
        open_slider(&mut m, id, &[0, 0], true);
        let mut adapter = Input::new();
        for tick in 0..=212 {
            poll(&mut m, &mut adapter, 1 << bit::DPAD_UP);
            if tick < 212 { assert!(units(&m, id, 1) < 510); }
        }
        assert_eq!(units(&m, id, 1), 510);
    }

    let id = "save_damage_limits_cpu";
    let mut m = menu(false);
    open_slider(&mut m, id, &[0, 0], true);
    let mut adapter = Input::new();
    let edges = [(1, 1), (2, 1), (3, 2), (6, 2), (7, 4), (12, 4),
        (13, 10), (22, 10), (23, 20), (255, 20)];
    let mut count = 0;
    for _ in 0..1200 {
        m.put(id, &[0, 0]);
        if poll(&mut m, &mut adapter, 1 << bit::DPAD_UP) {
            count += 1;
            if let Some((_, step)) = edges.iter().find(|(at, _)| *at == count) {
                assert_eq!(units(&m, id, 1), *step, "press {count}");
            }
            if count == 255 { break; }
        }
    }
    assert_eq!(count, 255);
}

#[test]
fn queued_jumps_and_popup_reset_the_slider_run() {
    let id = "save_damage_limits_cpu";
    let mut m = menu(false);
    open_slider(&mut m, id, &[0, 0], true);
    m.press(Button::Up);
    m.note_repeats(1);
    m.press(Button::Up);
    m.note_repeats(1);
    m.press(Button::JumpUp);
    assert_eq!(units(&m, id, 1), 4);
    m.on_request(crate::model::chords::Request::RecordingPopup);
    m.press(Button::Up);
    m.on_request(crate::model::chords::Request::RecordingPopup);
    m.note_repeats(1);
    m.press(Button::Up);
    assert_eq!(units(&m, id, 1), 5);
}

#[test]
fn slider_min_cannot_pass_max_and_ends_clamp() {
    let id = "save_damage_limits_cpu";
    let mut m = menu(false);
    open_slider(&mut m, id, &[0, 12, 0, 1], false);
    let mut adapter = Input::new();
    for _ in 0..400 { poll(&mut m, &mut adapter, 1 << bit::DPAD_UP); }
    assert_eq!(m.get(id), &[12, 12, 1, 1]);
    poll(&mut m, &mut adapter, 0);
    m.press(Button::A);
    m.press(Button::Down);
    m.press(Button::A);
    for _ in 0..400 { poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN); }
    assert_eq!(m.get(id), &[12, 12, 1, 1]);
    poll(&mut m, &mut adapter, 0);
    m.press(Button::A);
    m.press(Button::Up);
    m.press(Button::A);
    for _ in 0..400 { poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN); }
    assert_eq!(m.get(id), &[0, 12, 0, 1]);

    m.press(Button::A);
    m.press(Button::Down);
    m.press(Button::A);
    poll(&mut m, &mut adapter, 0);
    for _ in 0..400 { poll(&mut m, &mut adapter, 1 << bit::DPAD_DOWN); }
    assert_eq!(m.get(id), &[0, 0]);
    for (id, top) in [("save_damage_limits_cpu", 150), ("save_damage_limits_player", 255)] {
        let mut m = menu(false);
        open_slider(&mut m, id, &[0, 0], true);
        let mut adapter = Input::new();
        for _ in 0..400 { poll(&mut m, &mut adapter, 1 << bit::DPAD_UP); }
        assert_eq!(m.get(id), &[0, top]);
    }
}

#[test]
fn tech_sliders_keep_their_steps_and_quicken() {
    let id = "tech_damage_range";
    let mut m = menu(false);
    open_slider(&mut m, id, &[0, 0], true);
    m.press(Button::Up);
    assert_eq!(m.get(id), &[0, 1]);
    let mut frame = Frame::new();
    frame_of(&m, &mut frame, |_| "");
    assert_eq!(frame.rows[1].value.string_lossy(), "5%");
    for button in [Button::Left, Button::Right] {
        assert!(!m.press_outcome(button));
        assert_eq!(m.get(id), &[0, 1]);
    }
    m.put(id, &[0, 0]);
    let mut adapter = Input::new();
    let expected = [(0, 1), (21, 2), (29, 4), (37, 6), (45, 8), (53, 10), (60, 12),
        (65, 16), (70, 20), (75, 24), (80, 28), (85, 30)];
    let mut value = 0;
    for tick in 0..=180 {
        if let Some((_, next)) = expected.iter().find(|(at, _)| *at == tick) { value = *next; }
        poll(&mut m, &mut adapter, 1 << bit::DPAD_UP);
        assert_eq!(m.get(id), &[0, value], "poll {tick}");
    }
    frame_of(&m, &mut frame, |_| "");
    assert_eq!(frame.rows[1].value.string_lossy(), "150%");
}

#[test]
fn damage_range_shows_half_percent() {
    let id = "save_damage_limits_cpu";
    let mut m = menu(false);
    open_slider(&mut m, id, &[12, 150, 1, 0], false);
    assert_eq!(m.get(id), &[12, 150, 1, 0]);
    assert_eq!(m.value(row(id)), RowValue::Slider { lower: 25, upper: 300 });
    let mut frame = Frame::new();
    frame_of(&m, &mut frame, |_| "");
    assert_eq!(frame.rows[0].value.string_lossy(), "12.5%");
    assert_eq!(frame.rows[1].value.string_lossy(), "150%");
    let mut value = Text::<72>::new();
    frame::write_value(&m, row(id), &mut value);
    assert_eq!(value.string_lossy(), "12.5 - 150  >");
}
}
