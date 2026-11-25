lib_unit_tests! {
use super::*;
use crate::model::action::ActionDesc;
use crate::model::combo::{grade, Combo, Dir, Helper, InputCapture, Kind, LinkRec, Reason, Settings as ComboSettings};
use crate::model::combo_record::ComboRecord;

const NONE: MappedInputs = MappedInputs::NEUTRAL;
const A: MappedInputs = MappedInputs { buttons: 1, ..NONE };
const EDGE: [(Move, i32, i32, &str, &str); 3] = [
    (Move::LedgeAttack, 0x78, 0x77, "Ledge attack", "ledge_attack"),
    (Move::GetupAttack, 0x59, 0x53, "Getup attack", "getup_attack"),
    (Move::TripAttack, 0x8a, 0x88, "Trip attack", "trip_attack"),
];

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame, ..Sample::default() }
}

fn named(mv: Move, status: i32) -> Sample {
    Sample { named: Some(mv), status, ..Sample::default() }
}

fn waiting(status: i32) -> Sample {
    Sample { status, ..Sample::default() }
}

fn stun() -> Sample {
    Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() }
}

fn hit() -> Sample {
    Sample { stop: true, ..stun() }
}

fn hit_record(from: &str, to: &str, from_shown: Option<serde_json::Value>, to_shown: Option<serde_json::Value>) -> ComboRecord {
    let mut value = serde_json::json!({
        "schema_version": 1, "name": "Drill", "links": [{
            "from": from, "to": to, "verdict": {"kind": "true", "link": null},
            "window": null, "events": []
        }]
    });
    if let Some(shown) = from_shown { value["links"][0]["from_shown"] = shown; }
    if let Some(shown) = to_shown { value["links"][0]["to_shown"] = shown; }
    serde_json::from_value(value).unwrap()
}

fn edge_start(mv: Move, status: i32, wait: i32) -> Tracker {
    let mut tracker = Tracker::new();
    for frame in 1..=10 {
        tracker.step(waiting(wait), idle(), if frame == 8 { A } else { NONE });
    }
    tracker.step(named(mv, status), stun(), NONE);
    tracker
}

fn jab_to_edge(mv: Move, status: i32, wait: i32) -> (Tracker, Link) {
    let mut tracker = Tracker::new();
    tracker.step(attack(Move::Jab, 0x27, 0.0), hit(), NONE);
    for frame in 1..=4 {
        tracker.step(waiting(wait), stun(), if frame == 3 { A } else { NONE });
    }
    tracker.step(Sample { active: true, ..named(mv, status) }, hit(), NONE);
    let link = tracker.latest_link().expect("jab to edge attack");
    (tracker, link)
}

fn edge_to_jab(mv: Move, status: i32, wait: i32) -> (Tracker, Link) {
    let mut tracker = edge_start(mv, status, wait);
    tracker.step(named(mv, status), stun(), NONE);
    tracker.step(Sample { active: true, ..named(mv, status) }, hit(), NONE);
    for _ in 0..6 { tracker.step(named(mv, status), stun(), NONE); }
    for _ in 0..2 { tracker.step(idle(), stun(), NONE); }
    tracker.step(attack(Move::Jab, 0x27, 0.0), hit(), NONE);
    let link = tracker.latest_link().expect("edge attack to jab");
    (tracker, link)
}

fn check_link_text(tracker: &Tracker, link: Link, from: &str, to: &str, mv: Move) -> Combo {
    use crate::model::combo_plan::Renderer;
    use crate::model::frame_strip_plan::{link_moves, Text, TEXT_UNITS};
    use crate::model::panes::strip_layout::C;

    assert_eq!(String::from_utf16(link_moves(&link, TEXT_UNITS).units()).unwrap(), format!("{from} > {to}"), "{mv:?}");
    let rec = LinkRec::of(&link, tracker, tracker.exchange().1, true);
    let combo = Combo::from_links([rec]).unwrap();
    assert_eq!(crate::model::recording_menu::combo_link_lines(&combo)[0],
        format!("L1 {} → {} TRUE", from.to_ascii_lowercase(), to.to_ascii_lowercase()), "{mv:?}");
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let mut texts = Vec::new();
    Renderer::new().draw(&helper, &Text::EMPTY, None, true, (&C, false), |_, look, _| {
        if look.visible { texts.push(String::from_utf16_lossy(look.text.units())); }
    });
    assert!(texts.iter().any(|text| text == &format!("1 {from}>{to}")), "{mv:?}: {texts:?}");
    combo
}

#[test]
fn the_edge_attack_statuses_are_the_games() {
    use super::ids::{Table, C};
    let table = Table::with(&[(C::CliffAttack, 0x78), (C::DownStandAttack, 0x59),
        (C::SlipStandAttack, 0x8a), (C::Final, 0x1e0)]);
    for (mv, status, _, name, _) in EDGE {
        assert_eq!(table.named_move(status), Some(mv), "{mv:?}");
        assert_eq!(table.attack(status), None, "{mv:?}");
        assert_eq!(mv.name(), name, "{mv:?}");
        assert_eq!(mv.legacy(), Move::Other, "{mv:?}");
    }
    for status in [0x77, 0x79, 0x53, 0x57, 0x88, 0x89] {
        assert_eq!(table.named_move(status), None, "{status:#x}");
    }
    for (mv, constant) in [
        (Move::LedgeAttack, "FIGHTER_STATUS_KIND_CLIFF_ATTACK"),
        (Move::GetupAttack, "FIGHTER_STATUS_KIND_DOWN_STAND_ATTACK"),
        (Move::TripAttack, "FIGHTER_STATUS_KIND_SLIP_STAND_ATTACK"),
    ] {
        assert!(include_str!("frame_strip.rs").contains(&format!("\"{constant}\"")), "{mv:?}");
    }
}

#[test]
fn an_edge_attack_opens_its_own_action_at_its_press() {
    for (mv, status, wait, _, _) in EDGE {
        for actionable in [false, true] {
            let mut tracker = Tracker::new();
            for frame in 1..=10 {
                tracker.step(Sample { actionable, ..waiting(wait) }, idle(), if frame == 8 { A } else { NONE });
            }
            let id = tracker.action.id;
            tracker.step(named(mv, status), idle(), NONE);
            assert_eq!(tracker.action.mv, mv, "{mv:?}, actionable={actionable}");
            assert_eq!(tracker.action.start, 11, "{mv:?}, actionable={actionable}");
            assert_eq!(tracker.origin(), Some(8), "{mv:?}, actionable={actionable}");
            assert_eq!(tracker.late().open, Some((Some(8), None)), "{mv:?}, actionable={actionable}");
            assert_ne!(tracker.action.id, id, "{mv:?}, actionable={actionable}");
        }

        let mut tracker = Tracker::new();
        for frame in 0..4 { tracker.step(attack(Move::UpSpecial, 0x1de, frame as f32), idle(), NONE); }
        for frame in 0..4 { tracker.step(attack(Move::Other, 0x1e1, frame as f32), idle(), NONE); }
        assert_eq!(tracker.action.shown.of, Some(Move::UpSpecial), "{mv:?}");
        for frame in 1..=4 { tracker.step(waiting(wait), idle(), if frame == 3 { A } else { NONE }); }
        let id = tracker.action.id;
        tracker.step(named(mv, status), idle(), NONE);
        assert_eq!(tracker.action.mv, mv, "{mv:?}");
        assert_eq!(tracker.action.shown, Shown::default(), "{mv:?}");
        assert_ne!(tracker.action.id, id, "{mv:?}");
        assert_eq!(tracker.origin(), Some(11), "{mv:?}");
        assert_eq!(tracker.late().open, Some((Some(11), None)), "{mv:?}");
    }
}

#[test]
fn an_edge_attack_keeps_its_move_one_frame_past_its_end() {
    for (mv, status, wait, _, _) in EDGE {
        let mut tracker = edge_start(mv, status, wait);
        tracker.step(named(mv, status), stun(), NONE);
        tracker.step(Sample { active: true, ..named(mv, status) }, hit(), NONE);
        for _ in 0..6 { tracker.step(named(mv, status), stun(), NONE); }
        for _ in 0..2 {
            tracker.step(idle(), stun(), NONE);
            assert_eq!(tracker.action.mv, mv, "{mv:?}");
            assert_eq!(tracker.late().open, None, "{mv:?}");
            assert_eq!(tracker.origin(), Some(8), "{mv:?}");
        }
        tracker.step(attack(Move::Jab, 0x27, 0.0), hit(), NONE);
        let link = tracker.latest_link().expect("edge attack to jab");
        assert_eq!((link.from, link.to), (mv, Move::Jab), "{mv:?}");
    }
}

#[test]
fn an_edge_attack_links_on_both_sides_and_reads_alike() {
    for (mv, status, wait, name, _) in EDGE {
        let (tracker, link) = jab_to_edge(mv, status, wait);
        assert_eq!((link.from, link.to), (Move::Jab, mv), "{mv:?}");
        check_link_text(&tracker, link, "Jab", name, mv);

        let (tracker, link) = edge_to_jab(mv, status, wait);
        assert_eq!((link.from, link.to), (mv, Move::Jab), "{mv:?}");
        check_link_text(&tracker, link, name, "Jab", mv);
    }
}

#[test]
fn the_grader_tells_an_edge_attack_from_another_move() {
    let plain = *hit_record("jab", "other", Some(serde_json::json!({})), Some(serde_json::json!({})))
        .combo.get(0).unwrap();
    let jab = *hit_record("jab", "jab", Some(serde_json::json!({})), Some(serde_json::json!({})))
        .combo.get(0).unwrap();
    for (mv, _, _, _, key) in EDGE {
        let reference = *hit_record("jab", "other", Some(serde_json::json!({})),
            Some(serde_json::json!({"move": key}))).combo.get(0).unwrap();
        assert!(!grade(&reference, &reference).wrong_move, "{mv:?}");
        assert!(grade(&reference, &plain).wrong_move, "{mv:?}");
        assert!(grade(&reference, &jab).wrong_move, "{mv:?}");
        for (other, _, _, _, other_key) in EDGE {
            let attempt = *hit_record("jab", "other", Some(serde_json::json!({})),
                Some(serde_json::json!({"move": other_key}))).combo.get(0).unwrap();
            assert_eq!(grade(&reference, &attempt).wrong_move, mv != other, "{mv:?} vs {other:?}");
        }
    }

    let (tracker, link) = jab_to_edge(Move::LedgeAttack, 0x78, 0x77);
    let target = Combo::from_links([LinkRec::of(&link, &tracker, tracker.exchange().1, true)]).unwrap();
    for (mv, status, wait, _, _) in [EDGE[0], EDGE[1]] {
        let mut tracker = Tracker::new();
        let mut helper = Helper::new();
        helper.set_target(Some(target));
        tracker.step(attack(Move::Jab, 0x27, 0.0), hit(), NONE);
        helper.step(&tracker, ComboSettings::factory());
        for frame in 1..=4 {
            tracker.step(waiting(wait), stun(), if frame == 3 { A } else { NONE });
            helper.step(&tracker, ComboSettings::factory());
        }
        tracker.step(Sample { active: true, ..named(mv, status) }, hit(), NONE);
        helper.step(&tracker, ComboSettings::factory());
        let result = helper.results().next().expect("edge attack result");
        if mv == Move::LedgeAttack {
            assert!(result.clean, "{mv:?}: {result:?}");
        } else {
            assert_eq!(result.fault, Some((0, Reason::WrongMove(Move::GetupAttack))), "{mv:?}");
        }
    }
}

#[test]
fn edge_attacks_round_trip_in_files_and_in_the_packed_link() {
    for (index, (mv, _, _, _, key)) in EDGE.into_iter().enumerate() {
        let parsed = hit_record("jab", "other", Some(serde_json::json!({})),
            Some(serde_json::json!({"move": key})));
        assert_eq!(parsed.combo.get(0).unwrap().to, mv, "{mv:?}");
        let record = ComboRecord::new("Drill".into(), parsed.combo, None);
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value["links"][0]["to"], "other", "{mv:?}");
        assert_eq!(value["links"][0]["to_shown"], serde_json::json!({"move": key}), "{mv:?}");
        assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record, "{mv:?}");

        let desc = ActionDesc { mv: Move::Other, of: Some(mv), angle: None, normal: 26 + index as u8, parts: [None; 2] };
        let mut link = LinkRec::INPUT;
        link.set_descriptions([desc, desc]);
        for side in 0..2 {
            assert_eq!((link.desc(side).of, link.desc(side).normal), (Some(mv), 26 + index as u8), "{mv:?}");
        }
    }
    let parsed = hit_record("other", "other", Some(serde_json::json!({"move": "ledge_attack"})),
        Some(serde_json::json!({"move": "getup_attack"})));
    let link = parsed.combo.get(0).unwrap();
    assert_eq!((link.from, link.to), (Move::LedgeAttack, Move::GetupAttack));
    assert!(link.described());
    let record = ComboRecord::new("Drill".into(), parsed.combo, None);
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!((value["links"][0]["from"].as_str(), value["links"][0]["to"].as_str()), (Some("other"), Some("other")));
    assert_eq!(value["links"][0]["from_shown"], serde_json::json!({"move": "ledge_attack"}));
    assert_eq!(value["links"][0]["to_shown"], serde_json::json!({"move": "getup_attack"}));
    assert_eq!(serde_json::from_value::<ComboRecord>(value.clone()).unwrap(), record);
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);

    let parsed = hit_record("jab", "other", Some(serde_json::json!({})),
        Some(serde_json::json!({"of": "trip_attack"})));
    assert_eq!(parsed.combo.get(0).unwrap().desc(1).of, Some(Move::TripAttack));
    let record = ComboRecord::new("Drill".into(), parsed.combo, None);
    let encoded = serde_json::to_value(&record).unwrap();
    assert_eq!(encoded["links"][0]["to_shown"], serde_json::json!({"of": "trip_attack"}));
    assert_eq!(serde_json::from_value::<ComboRecord>(encoded).unwrap(), record);

    let mut invalid = value;
    invalid["links"][0]["to_shown"] = serde_json::json!({"move": "ledge"});
    assert!(serde_json::from_value::<ComboRecord>(invalid.clone()).unwrap_err().to_string().contains("to_shown"));
    invalid["links"][0]["to"] = serde_json::json!("jab");
    invalid["links"][0]["to_shown"] = serde_json::json!({"move": "ledge_attack"});
    assert!(serde_json::from_value::<ComboRecord>(invalid).unwrap_err().to_string().contains("to_shown"));

    for (source, table) in [(include_str!("combo.rs"), "const DESC_MOVES:"),
        (include_str!("combo_record.rs"), "const MOVES:")] {
        let table = source.split(table).nth(1).unwrap().split("];").next().unwrap();
        let positions = ["Move::FinalSmash", "Move::LedgeAttack", "Move::GetupAttack", "Move::TripAttack"]
            .map(|name| table.find(name).unwrap());
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{table}");
    }
}

#[test]
fn an_old_file_grades_an_edge_attack_as_before() {
    let old = hit_record("jab", "other", None, None);
    let value = serde_json::to_value(ComboRecord::new("Drill".into(), old.combo, None)).unwrap();
    assert!(value["links"][0].get("from_shown").is_none());
    assert!(value["links"][0].get("to_shown").is_none());
    let drill: ComboRecord = serde_json::from_str(r#"{"input_steps":[{"events":[{"button":"attack","dir":"neutral","kind":"press","offset":0},{"button":"attack","dir":"neutral","kind":"release","offset":2}]}],"kind":"input_drill","name":"Drill 1","schema_version":2}"#).unwrap();
    for (mv, status, wait, _, key) in EDGE {
        let described = hit_record("jab", "other", Some(serde_json::json!({})),
            Some(serde_json::json!({"move": key})));
        assert!(!grade(old.combo.get(0).unwrap(), described.combo.get(0).unwrap()).wrong_move, "{mv:?}");
        let mut tracker = Tracker::new();
        let mut helper = Helper::new();
        helper.set_target(Some(drill.combo));
        for frame in 1..=20 {
            let p1 = if frame <= 7 { waiting(wait) } else { named(mv, status) };
            tracker.step(p1, idle(), if (6..=7).contains(&frame) { A } else { NONE });
            helper.step(&tracker, ComboSettings::factory());
        }
        let result = helper.results().next().expect("old drill result");
        assert!(result.clean, "{mv:?}: {result:?}");
        assert_eq!(result.graded, 1, "{mv:?}");
    }
}

#[test]
fn an_edge_attack_step_is_decided_when_its_action_opens() {
    for (mv, status, wait, _, _) in EDGE {
        let mut tracker = Tracker::new();
        let mut input = InputCapture::new();
        for frame in 1..=20 {
            let p1 = if frame <= 7 { waiting(wait) } else { named(mv, status) };
            let inputs = MappedInputs {
                buttons: u32::from((6..=7).contains(&frame)),
                lstick_x: if frame >= 6 { 60 } else { 0 },
                ..NONE
            };
            tracker.step(p1, idle(), inputs);
            input.step_late(tracker.snaps().last().unwrap(), tracker.late());
        }
        assert_eq!(input.combo.len(), 1, "{mv:?}");
        let events = input.combo.get(0).unwrap().events().collect::<Vec<_>>();
        assert_eq!(events.len(), 2, "{mv:?}");
        assert_eq!((events[0].kind, events[0].dir, events[0].offset),
            (Kind::Press(Button::Attack), Dir::Forward, 0), "{mv:?}");
        assert_eq!((events[1].kind, events[1].offset), (Kind::Release(Button::Attack), 2), "{mv:?}");

        let mut tracker = Tracker::new();
        let mut helper = Helper::new();
        helper.set_target(Some(input.combo));
        for frame in 1..=12 {
            let p1 = if frame <= 7 { waiting(wait) } else { named(mv, status) };
            let inputs = MappedInputs {
                buttons: u32::from((6..=7).contains(&frame)),
                lstick_x: if frame >= 6 { 60 } else { 0 },
                ..NONE
            };
            tracker.step(p1, idle(), inputs);
            helper.step(&tracker, ComboSettings::factory());
        }
        let result = helper.results().next().expect("edge direction resolved by frame 12");
        assert!(result.clean, "{mv:?}: {result:?}");
    }
}
}
