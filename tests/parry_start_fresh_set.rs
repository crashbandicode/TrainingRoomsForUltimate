//! Exercise the production Parry state machine through the practice Start/Stop entry points.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/runtime/features/parry.rs"]
#[allow(dead_code)]
mod parry;
#[path = "../src/runtime/features/practice.rs"]
#[allow(dead_code)]
mod practice;

mod engine {
    pub mod rr_host {
        pub fn set_parry(_on: bool) {}
        pub fn set_parry_open(_open: bool) {}
        pub fn cpu_run_active() -> bool { true }
    }
}

mod runtime {
    pub mod diag_log {
        pub fn diagnostic(_message: core::fmt::Arguments<'_>) {}
    }
}

mod hitbox {
    pub fn practice_started() {}
    pub fn eggs_start() {}
    pub fn eggs_running() -> bool { false }
}

mod btt {
    pub fn start(_place: bool) {}
    pub fn running() -> bool { false }
    pub fn starting() -> bool { false }
}

mod mashout {
    pub fn start() {}
}

mod tech {
    pub fn start(_point: bool) {}
    pub fn open_wizard() {}
    pub fn running() -> bool { false }
    pub fn starting() -> bool { false }
}

mod notices {
    use std::sync::Mutex;

    pub static DONE: Mutex<Option<String>> = Mutex::new(None);

    pub fn clear(id: u8) {
        assert_eq!(id, 0x33);
        *DONE.lock().unwrap() = None;
    }

    pub fn upsert(_id: u8, _header: &'static str, body: &str, _length: u16, _color: [u8; 4]) {
        *DONE.lock().unwrap() = Some(body.to_owned());
    }
}

use model::{parry::Frame, practice_games::Game};

fn step(frame: Frame, clock: &mut u32) {
    *clock += 1;
    parry::step(frame, Some(0), *clock);
}

fn attempts(count: usize, perfect: bool, clock: &mut u32) {
    for _ in 0..count {
        step(Frame {
            stop: true,
            guard_damage: perfect,
            gained: !perfect,
            just_shield: perfect.then_some(true),
            ..Frame::default()
        }, clock);
        step(Frame::default(), clock);
    }
}

fn assert_readout(set: &str, summary: &str) {
    let view = parry::view().expect("Parry must be active");
    assert_eq!(String::from_utf16_lossy(view.set.units()), set);
    assert_eq!(String::from_utf16_lossy(view.set_line.units()), summary);
}

#[test]
fn explicit_start_clears_the_set_and_preserves_lifetime_results() {
    let mut clock = 0;
    practice::start(Game::Parry, practice::How::default());
    step(Frame::default(), &mut clock);
    assert_readout("0/10", "streak 0, perfect 0/0");

    attempts(3, true, &mut clock);
    assert_readout("3/10", "streak 3, perfect 3/3");
    let earlier_grades = parry::view().unwrap().last;
    assert_eq!(earlier_grades.iter().flatten().count(), 3);

    // L+R+A and state Load both use reset. An open attempt is dropped without counting it.
    step(Frame {
        stop: true,
        guard_damage: true,
        just_shield: Some(false),
        shield_held: true,
        ..Frame::default()
    }, &mut clock);
    parry::reset();
    step(Frame::default(), &mut clock);
    assert_readout("3/10", "streak 3, perfect 3/3");

    assert_eq!(parry::view().unwrap().last, earlier_grades);
    practice::start(Game::Parry, practice::How { resume_parry: true, ..Default::default() });
    step(Frame::default(), &mut clock);
    assert_readout("3/10", "streak 3, perfect 3/3");
    assert_eq!(parry::view().unwrap().last, earlier_grades);

    practice::stop();
    assert!(parry::view().is_none());
    practice::start(Game::Parry, practice::How::default());
    let before_first_sample = parry::view().map(|view| String::from_utf16_lossy(view.set.units()));
    // The first resumed frame can still show the previous run's guard hitstop.
    step(Frame {
        stop: true,
        guard_damage: true,
        just_shield: Some(true),
        ..Frame::default()
    }, &mut clock);
    assert_readout("0/10", "streak 0, perfect 3/3");
    assert_eq!((before_first_sample, parry::view().unwrap().last), (None, [None; 4]));
    step(Frame::default(), &mut clock);

    attempts(9, false, &mut clock);
    assert_readout("9/10", "streak 0, perfect 3/12");
    assert!(notices::DONE.lock().unwrap().is_none());
    attempts(1, false, &mut clock);
    assert_readout("0/10", "streak 0, perfect 3/13");
    let done = notices::DONE.lock().unwrap().clone().expect("set completion");
    assert!(done.starts_with("Done! 10: 0 Perfect"), "{done}");
    assert!(done.ends_with("total 13 · fighter best 3"), "{done}");

    practice::stop();
    practice::start(Game::Parry, practice::How::default());
    step(Frame::default(), &mut clock);
    assert_readout("0/10", "streak 0, perfect 3/13");
    assert!(notices::DONE.lock().unwrap().is_none());
}
