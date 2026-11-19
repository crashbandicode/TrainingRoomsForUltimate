#[path = "../src/model/lockon.rs"]
mod lockon;

use lockon::{ItemHost, Target, Tracker};

#[derive(Debug, PartialEq)]
enum Call {
    Create([f32; 2]),
    Move(u32, [f32; 2]),
    Remove(u32),
}

#[derive(Default)]
struct Field {
    calls: Vec<Call>,
    live: Vec<u32>,
    next: u32,
    full: bool,
}

impl ItemHost for Field {
    type Handle = u32;

    fn valid(&mut self, item: u32) -> bool {
        self.live.contains(&item)
    }

    fn create(&mut self, at: [f32; 2]) -> Option<u32> {
        self.calls.push(Call::Create(at));
        if self.full {
            return None;
        }
        self.next += 1;
        self.live.push(self.next);
        Some(self.next)
    }

    fn place(&mut self, item: u32, at: [f32; 2]) {
        assert!(self.live.contains(&item));
        self.calls.push(Call::Move(item, at));
    }

    fn remove(&mut self, item: u32) {
        assert!(self.live.contains(&item));
        self.live.retain(|&id| id != item);
        self.calls.push(Call::Remove(item));
    }
}

fn target(key: u64, at: [f32; 2]) -> Option<Target> {
    Some(Target { key, at })
}

#[test]
fn moving_egg_reuses_its_item_and_a_break_replaces_it_once() {
    let mut field = Field::default();
    let mut tracker = Tracker::new();
    tracker.update(1, [target(0, [24., 8.])], &mut field);
    tracker.update(1, [target(0, [25., 9.])], &mut field);
    tracker.update(1, [target(1, [30., 9.])], &mut field);
    tracker.update(1, [target(1, [30., 9.])], &mut field);
    assert_eq!(
        field.calls,
        [
            Call::Create([24., 8.]),
            Call::Move(1, [24., 8.]),
            Call::Move(1, [25., 9.]),
            Call::Remove(1),
            Call::Create([30., 9.]),
            Call::Move(2, [30., 9.]),
            Call::Move(2, [30., 9.]),
        ],
    );
    assert_eq!(field.live, [2]);
}

#[test]
fn clearing_a_btt_slot_leaves_other_targets_and_stock_items() {
    let mut field = Field {
        live: vec![80],
        ..Field::default()
    };
    let mut tracker = Tracker::new();
    let first = target(0, [22., 5.]);
    let second = target(0, [22., 16.]);
    tracker.update(3, [first, second], &mut field);
    field.calls.clear();
    tracker.update(3, [None, second], &mut field);
    assert_eq!(field.calls, [Call::Remove(1), Call::Move(2, [22., 16.])]);
    tracker.update(3, [None, None], &mut field); // Off or save-state load
    tracker.update(3, [None, None], &mut field);
    assert_eq!(
        field
            .calls
            .iter()
            .filter(|c| **c == Call::Remove(2))
            .count(),
        1
    );
    assert_eq!(field.live, [80]);
    assert!(tracker.is_empty());
}

#[test]
fn scene_change_never_dereferences_a_handle_from_the_departed_scene() {
    let mut field = Field::default();
    let mut tracker = Tracker::new();
    let spot = target(0, [0., 4.]);
    tracker.update(4, [spot], &mut field);
    field.calls.clear();
    // The same native ID may now belong to an unrelated object in the new match.
    tracker.update(5, [spot], &mut field);
    assert_eq!(
        field.calls,
        [Call::Create([0., 4.]), Call::Move(2, [0., 4.])]
    );
}

#[test]
fn invalid_handle_is_skipped_and_full_manager_retries_without_a_phantom_item() {
    let mut field = Field::default();
    let mut tracker = Tracker::new();
    let spot = target(7, [20., 6.]);
    tracker.update(1, [spot], &mut field);
    field.live.clear();
    field.calls.clear();
    field.full = true;
    tracker.update(1, [spot], &mut field);
    assert_eq!(field.calls, [Call::Create([20., 6.])]);
    field.full = false;
    tracker.update(1, [spot], &mut field);
    assert_eq!(
        field.calls,
        [
            Call::Create([20., 6.]),
            Call::Create([20., 6.]),
            Call::Move(2, [20., 6.]),
        ]
    );
}
