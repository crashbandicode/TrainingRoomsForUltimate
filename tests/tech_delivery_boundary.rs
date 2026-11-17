//! Core retained receipt -> owner-supplied production UI grading/session boundary.
macro_rules! lib_unit_tests { ($($body:tt)*) => {}; }
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
use model::tech::{controller::{Controller, Event, Setup}, session::{self, Direction, Grade, Length, NativeOutcome, Phase}};
use model::tech_launch::{Attempt, ContactReceipt, DecisionDelivery, DecisionSnapshot, HitHeight, Launch, Launcher, Surface, Vec2};
struct Fake { attempt: Attempt, placements: u64 }
impl Launcher for Fake {
    type Error = ();
    fn place(&mut self, _: Launch) -> Result<(), ()> {
        self.placements += 1;
        self.attempt = Attempt::held(self.placements);
        Ok(())
    }
    fn launch(&mut self) -> Result<(), ()> { self.attempt.launched(); Ok(()) }
    fn attempt(&self) -> Result<Attempt, ()> { Ok(self.attempt) }
    fn restore(&mut self) { self.attempt.mark_abandoned(); }
}
fn setup() -> Setup {
    Setup { delay: 1, launch: Launch {
        start: Vec2 { x: -30., y: 20. }, target: Vec2::default(), angle: 315.,
        knockback: 100., percent: 60., hit_damage: 12., height: HitHeight::Middle,
        facing: 1., hit_from_behind: false, grounded_hit: false, di: None,
    }}
}
fn contact() -> (Controller, Fake, DecisionSnapshot) {
    let mut c = Controller::IDLE;
    let mut f = Fake { attempt: Attempt::IDLE, placements: 0 };
    c.start(Length::Free);
    c.place(&mut f, setup()).unwrap();
    assert_eq!(c.tick(&mut f, false, None), Ok(Event::Launched));
    let contact = ContactReceipt { surface: Surface::Floor, requested_status: 999, passive_enabled: true, position: Vec2::default() };
    f.attempt.record_contact(contact);
    assert_eq!(c.tick(&mut f, false, None), Ok(Event::None));
    assert!(c.awaiting_outcome());
    let snapshot = DecisionSnapshot {
        attempt: f.attempt.id, contact, source_status: Some(81), entered_status: None,
        guard_count: Some(254), previous_guard_gap: Some(254), guard_event: None,
        window_base: Some(12), window_multiplier: Some(1.), lockout: Some(28),
        reaction: Some(20.), damage_fall: Some(false), stick_x: None, press_frame_offset: None,
    };
    (c, f, snapshot)
}
const IDS: [i32; 11] = [200,201,202,203,204,205,206,207,208,209,210];
#[test]
fn retained_delivery_grades_actual_tech_roll_both_inputs_and_miss() {
    for (actual, stick, expected) in [
        (IDS[0], 0., NativeOutcome::Tech(Some(Direction::InPlace))),
        (IDS[1], -1., NativeOutcome::Tech(None)),
        (IDS[1], 1., NativeOutcome::Tech(None)),
        (IDS[2], 0., NativeOutcome::Miss),
    ] {
        let (mut c, mut f, mut raw) = contact();
        raw.stick_x = Some(stick);
        assert_ne!(raw.contact.requested_status, actual);
        let mut delivery = DecisionDelivery::EMPTY;
        delivery.capture_first(raw);
        assert_eq!(delivery.confirmed_decision(), None);
        assert_eq!(c.snapshot(raw, session::accepted_status(raw, IDS)), Event::None);
        assert!(delivery.observe_entered_status(raw.attempt, Some(actual), &IDS));
        let sent = delivery.confirmed_decision().unwrap();
        assert_eq!(session::accepted_status(sent, IDS), Some(expected));
        // Recorder backpressure retains the same actual and all predecision bytes.
        for _ in 0..3 { assert_eq!(delivery.confirmed_decision(), Some(sent)); }
        assert_eq!(c.snapshot_at(sent, session::accepted_status(sent, IDS), 0), Event::Graded);
        assert!(delivery.acknowledge_decision(sent.attempt));
        assert!(!c.awaiting_outcome());
        if expected == NativeOutcome::Miss {
            assert_eq!(c.session.result, None);
            c.tick(&mut f, false, Some(false)).unwrap(); // Decision frame; no late sample.
            for _ in 0..session::LATE { c.tick(&mut f, false, Some(false)).unwrap(); }
            assert_eq!(c.session.result, Some(Grade::Missed));
        } else {
            assert_eq!(c.session.result, Some(Grade::Perfect(None)));
            // Predecision stick does not prove the native roll direction.
            assert_eq!(c.direction, if actual == IDS[0] { Some(Direction::InPlace) } else { None });
        }
        assert_eq!(c.session.summary.attempts, 1);
    }
}
#[test]
fn retained_unavailable_contact_respawns_at_existing_hold_uncounted_and_continues() {
    for (source, actual) in [(Some(81), None), (None, Some(IDS[0])), (Some(IDS[0]), Some(IDS[0])), (Some(81), Some(999)), (Some(81), Some(81))] {
        let (mut c, mut f, mut raw) = contact();
        raw.source_status = source;
        let mut delivery = DecisionDelivery::EMPTY;
        delivery.capture_first(raw);
        for age in 0..session::HOLD {
            assert!(!delivery.observe_entered_status(raw.attempt, actual, &IDS));
            let queried = delivery.pending_decision().unwrap();
            assert_eq!(queried, raw);
            assert_eq!(queried.entered_status, None);
            assert_eq!(c.snapshot(queried, session::accepted_status(queried, IDS)), Event::None);
            assert_eq!(c.session.summary.attempts, 0);
            assert_eq!(c.session.phase, Phase::Flight);
            assert_eq!(c.tick(&mut f, false, None), Ok(Event::None));
            if age == 7 {
                let elapsed = c.session.elapsed;
                for _ in 0..5 { assert_eq!(c.tick(&mut f, true, None), Ok(Event::None)); }
                assert_eq!(c.session.elapsed, elapsed);
            }
        }
        let queried = delivery.pending_decision().unwrap();
        assert_eq!(c.snapshot_unavailable(queried, session::HOLD), Event::Graded);
        assert_eq!(c.session.result, Some(Grade::Unavailable));
        assert_eq!(c.session.summary.attempts, 0);
        assert_eq!(c.session.summary.missed, 0);
        assert!(!c.awaiting_outcome());
        assert_eq!(c.tick(&mut f, false, None), Ok(Event::NeedSetup { reroll: false }));
        assert_eq!(c.session.phase, Phase::Place);
        c.place(&mut f, setup()).unwrap();
        assert_eq!(f.placements, 2);
        assert_eq!(c.tick(&mut f, false, None), Ok(Event::Launched));
        assert_eq!(c.session.summary.attempts, 0);
        assert!(!c.accepts_snapshot(raw.attempt));
    }
}
