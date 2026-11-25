//! EXPECTED RED on a4f2087f: a buffered grab/Final Smash and a throw's variant.
//! GUARDS: pummels, aim, own-status names, rendering/grading, encoding and status mapping.

lib_unit_tests! {
use super::*;
use crate::model::action::ActionDesc;
use crate::model::combo::{grade, Combo, Helper, LinkRec};
use crate::model::combo_record::ComboRecord;

const NONE: MappedInputs = MappedInputs::NEUTRAL;
const ATTACK: i32 = 0x27;
const ATTACK_DASH: i32 = 0x29;
const ATTACK_AIR: i32 = 0x36;
const CATCH: i32 = 0x37;
const CATCH_DASH: i32 = 0x39;
const CATCH_WAIT: i32 = 0x3c;
const CATCH_ATTACK: i32 = 0x3d;
const CATCH_CUT: i32 = 0x3e;
const CATCH_JUMP: i32 = 0x3f;
const THROW: i32 = 0x40;
const THROW_KIRBY: i32 = 0xe3;
const FINAL: i32 = 0x1e0;
const CATCH_MOTION: u64 = 0x05_c56b_7b64;
const THROW_F_MOTION: u64 = 0x07_49f7_09a8;

fn idle() -> Sample {
    Sample { actionable: true, ..Sample::default() }
}

fn attack(mv: Move, status: i32, motion_frame: f32) -> Sample {
    Sample { attack: Some(mv), status, motion_frame, ..Sample::default() }
}

fn named(mv: Move, status: i32) -> Sample {
    Sample { named: Some(mv), status, ..Sample::default() }
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

#[test]
fn a_buffered_grab_is_its_own_action() {
    let mut tracker = Tracker::new();
    tracker.step(attack(Move::DashAttack, ATTACK_DASH, 0.0), hit(), NONE);
    tracker.step(attack(Move::DashAttack, ATTACK_DASH, 1.0), stun(), NONE);
    tracker.step(named(Move::Grab, CATCH_DASH), stun(), NONE);
    tracker.step(named(Move::Grab, CATCH_WAIT), stun(), NONE);
    tracker.step(named(Move::Throw, THROW), hit(), NONE);
    assert!(tracker.latest_link().is_some(), "the buffered grab opens an action for the throw");
    let link = tracker.latest_link().unwrap();
    assert_eq!((link.from, link.to), (Move::DashAttack, Move::Throw));

    let mut tracker = Tracker::new();
    tracker.step(attack(Move::Jab, ATTACK, 0.0), hit(), NONE);
    tracker.step(attack(Move::Jab, ATTACK, 1.0), stun(), NONE);
    tracker.step(named(Move::FinalSmash, FINAL), stun(), NONE);
    tracker.step(named(Move::FinalSmash, FINAL), hit(), NONE);
    assert!(tracker.latest_link().is_some(), "the buffered Final Smash opens an action");
    let link = tracker.latest_link().unwrap();
    assert_eq!((link.from, link.to), (Move::Jab, Move::FinalSmash));
}

#[test]
fn a_numbered_jab_becoming_rapid_names_the_hit_without_opening_another_action() {
    use crate::model::hash40::hash40;
    use crate::model::move_names::Namer;

    for (motions, expected) in [
        (&["attack_100_start"][..], "Rapid jab"),
        (&["attack_100"][..], "Rapid jab"),
        (&["attack_100", "attack_100_end"][..], "Rapid jab finisher"),
    ] {
        let mut tracker = Tracker::new();
        tracker.step(attack(Move::DashAttack, ATTACK_DASH, 0.0), hit(), NONE);
        tracker.step(idle(), stun(), NONE);
        tracker.step(Sample { motion: hash40("attack_12"), ..attack(Move::Jab, ATTACK, 8.0) }, stun(), NONE);
        let id = tracker.action.id;
        for (frame, motion) in motions.iter().enumerate() {
            let cpu = if frame + 1 == motions.len() { hit() } else { stun() };
            tracker.step(Sample { motion: hash40(motion), ..attack(Move::Jab, 0x28, frame as f32) }, cpu, NONE);
            assert_eq!(tracker.action.id, id, "a change of name is not a new action");
        }
        let link = tracker.latest_link().expect("dash attack into the rapid hit");
        let desc = ActionDesc::of(link.to, link.shown[1], link.variants[1]);
        let mut name = String::new();
        desc.write(&mut name, false, Namer::OFF).unwrap();
        assert_eq!(name, "Jab");
        name.clear();
        desc.write(&mut name, false, Namer::new(true, None, None)).unwrap();
        assert_eq!(name, expected, "hit on {motions:?}");
    }
}

#[test]
fn a_throw_takes_its_variant_when_it_begins() {
    let mut tracker = Tracker::new();
    tracker.step(attack(Move::Jab, ATTACK, 0.0), hit(), NONE);
    tracker.step(idle(), stun(), NONE);
    tracker.step(Sample { motion: CATCH_MOTION, ..named(Move::Grab, CATCH) }, stun(), NONE);
    tracker.step(Sample { motion: CATCH_MOTION, ..named(Move::Grab, CATCH_WAIT) }, stun(), NONE);
    assert_eq!(tracker.action.variant, 18, "the held grab keeps its variant");
    tracker.step(Sample { motion: THROW_F_MOTION, ..named(Move::Throw, THROW) }, hit(), NONE);
    let link = tracker.latest_link().expect("jab to throw");
    assert_eq!((link.from, link.to), (Move::Jab, Move::Throw));
    assert_eq!(link.variants[1], 22, "the throw takes its own variant");
    tracker.step(idle(), stun(), NONE);
    tracker.step(attack(Move::Jab, ATTACK, 0.0), hit(), NONE);
    let link = tracker.latest_link().expect("throw to jab");
    assert_eq!((link.from, link.to), (Move::Throw, Move::Jab));
    assert_eq!(link.variants[0], 22, "the following link retains the throw's variant");
}

#[test]
fn a_pummel_stays_in_the_action_of_its_grab() {
    let mut tracker = Tracker::new();
    tracker.step(named(Move::Grab, CATCH), stun(), NONE);
    let id = tracker.action.id;
    for (status, cpu) in [
        (CATCH_WAIT, stun()), (CATCH_ATTACK, hit()), (CATCH_WAIT, stun()),
        (CATCH_ATTACK, hit()), (CATCH_WAIT, stun()),
    ] {
        tracker.step(named(Move::Grab, status), cpu, NONE);
        assert_eq!(tracker.action.id, id);
        assert_eq!(tracker.action.mv, Move::Grab);
        assert!(tracker.latest_link().is_none());
    }
    tracker.step(named(Move::Throw, THROW), hit(), NONE);
    assert_eq!(tracker.action.id, id);
    assert_eq!(tracker.action.mv, Move::Throw);
    assert!(tracker.latest_link().is_none());
}

#[test]
fn a_final_smash_opens_no_aim_parts() {
    let mut tracker = Tracker::new();
    let stick = MappedInputs { lstick_x: -100, lstick_y: 100, ..NONE };
    for motion in 0..12 {
        tracker.step(Sample { pos: Some((0.0, 0.0)), motion_frame: motion as f32,
            ..named(Move::FinalSmash, FINAL) }, idle(), stick);
    }
    for motion in 0..4 {
        tracker.step(Sample { pos: Some((-4.0 * (motion + 1) as f32, 4.0 * (motion + 1) as f32)),
            ..attack(Move::Other, FINAL + 1, motion as f32) }, idle(), stick);
    }
    assert_eq!(tracker.aims().count(), 0);
    assert!(tracker.snaps().all(|snap| snap.aim.part == 0 && !snap.aim.inside));
}

#[test]
fn own_statuses_after_a_final_smash_or_a_throw_keep_its_name() {
    for (mv, name) in [(Move::FinalSmash, "Final Smash"), (Move::Throw, "Throw")] {
        let mut tracker = Tracker::new();
        tracker.step(attack(Move::Jab, ATTACK, 0.0), hit(), NONE);
        tracker.step(idle(), stun(), NONE);
        if mv == Move::Throw {
            tracker.step(named(Move::Grab, CATCH), stun(), NONE);
            tracker.step(named(Move::Throw, THROW), stun(), NONE);
        } else {
            tracker.step(named(Move::FinalSmash, FINAL), stun(), NONE);
            tracker.step(named(Move::FinalSmash, FINAL), stun(), NONE);
        }
        let status = if mv == Move::Throw { FINAL + 2 } else { FINAL + 1 };
        tracker.step(attack(Move::Other, status, 0.0), hit(), NONE);
        let link = tracker.latest_link().expect("own status follows the jab");
        assert_eq!(link.to, Move::Other);
        let desc = ActionDesc::of(link.to, link.shown[1], link.variants[1]);
        assert_eq!(desc.of, Some(mv));
        let mut text = String::new();
        desc.write(&mut text, false, crate::model::move_names::Namer::OFF).unwrap();
        assert_eq!(text, name);
    }
}

#[test]
fn the_strip_the_bar_and_the_menu_name_a_grab_a_throw_and_a_final_smash() {
    use crate::model::combo_plan::Renderer;
    use crate::model::frame_strip_plan::{link_moves, Text, TEXT_UNITS};
    use crate::model::panes::strip_layout::C;

    let mut tracker = Tracker::new();
    tracker.step(attack(Move::Jab, ATTACK, 0.0), hit(), NONE);
    tracker.step(idle(), stun(), NONE);
    tracker.step(named(Move::Grab, CATCH), stun(), NONE);
    tracker.step(named(Move::Throw, THROW), hit(), NONE);
    let link = tracker.latest_link().unwrap();
    assert_eq!(String::from_utf16(link_moves(&link, TEXT_UNITS).units()).unwrap(), "Jab > Throw");
    let rec = LinkRec::of(&link, &tracker, tracker.exchange().1, true);
    let combo = Combo::from_links([rec]).unwrap();
    assert!(crate::model::recording_menu::combo_link_lines(&combo, crate::model::move_names::Namer::OFF)[0].starts_with("L1 jab → throw TRUE"));
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    let mut texts = Vec::new();
    Renderer::new().draw(&helper, &Text::EMPTY, None, true, (&C, false), |_, look, _| {
        if look.visible { texts.push(String::from_utf16_lossy(look.text.units())); }
    });
    assert!(texts.iter().any(|text| text == "1 Jab>Throw"), "{texts:?}");
    assert_eq!((Move::Grab.name(), Move::Throw.name(), Move::FinalSmash.name()),
        ("Grab", "Throw", "Final Smash"));

    let throw = *hit_record("jab", "other", Some(serde_json::json!({})),
        Some(serde_json::json!({"move": "throw"}))).combo.get(0).unwrap();
    let grab = *hit_record("jab", "other", Some(serde_json::json!({})),
        Some(serde_json::json!({"move": "grab"}))).combo.get(0).unwrap();
    assert!(grade(&throw, &grab).wrong_move);
    assert!(grade(&grab, &throw).wrong_move);
    assert!(!grade(&throw, &throw).wrong_move);
    assert!(!grade(&grab, &grab).wrong_move);
}

#[test]
fn the_three_moves_round_trip_in_files_and_in_the_packed_link() {
    for (key, mv) in [("grab", Move::Grab), ("throw", Move::Throw), ("final_smash", Move::FinalSmash)] {
        let parsed = hit_record("jab", "other", Some(serde_json::json!({})),
            Some(serde_json::json!({"move": key})));
        assert_eq!(parsed.combo.get(0).unwrap().to, mv);
        let record = ComboRecord::new("Drill".into(), parsed.combo, None);
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value["links"][0]["to"], "other");
        assert_eq!(value["links"][0]["to_shown"], serde_json::json!({"move": key}));
        assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record);

        let parsed = hit_record("jab", "other", Some(serde_json::json!({})),
            Some(serde_json::json!({"of": key})));
        assert_eq!(parsed.combo.get(0).unwrap().desc(1).of, Some(mv));
        let record = ComboRecord::new("Drill".into(), parsed.combo, None);
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value["links"][0]["to"], "other");
        assert_eq!(value["links"][0]["to_shown"], serde_json::json!({"of": key}));
        assert_eq!(serde_json::from_value::<ComboRecord>(value).unwrap(), record);

        let mut rec = LinkRec::INPUT;
        let desc = ActionDesc { mv: Move::Other, of: Some(mv), angle: Some(Angle::Down), normal: 31, parts: [None; 2] };
        rec.set_descriptions([desc, desc]);
        for side in 0..2 {
            let actual = rec.desc(side);
            assert_eq!((actual.of, actual.angle, actual.normal), (Some(mv), Some(Angle::Down), 31));
        }
    }
    assert_eq!(core::mem::size_of::<LinkRec>(), 124);
    let old = hit_record("jab", "other", None, None);
    assert!(!old.combo.get(0).unwrap().described());
    let value = serde_json::to_value(ComboRecord::new("Drill".into(), old.combo, None)).unwrap();
    assert!(value["links"][0].get("from_shown").is_none());
    assert!(value["links"][0].get("to_shown").is_none());

    for (source, table) in [(include_str!("combo.rs"), "const DESC_MOVES:"),
        (include_str!("combo_record.rs"), "const MOVES:")] {
        let table = source.split(table).nth(1).unwrap().split("];").next().unwrap();
        let positions = [Move::Other, Move::Grab, Move::Throw, Move::FinalSmash]
            .map(|mv| table.find(match mv {
                Move::Other => "Move::Other", Move::Grab => "Move::Grab",
                Move::Throw => "Move::Throw", Move::FinalSmash => "Move::FinalSmash", _ => unreachable!(),
            }).unwrap());
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn a_grab_a_throw_and_a_final_smash_belong_to_descriptions_not_bare_link_endpoints() {
    for (mv, key) in [(Move::Grab, "grab"), (Move::Throw, "throw"), (Move::FinalSmash, "final_smash")] {
        for (field, shown, side) in [("from", "from_shown", 0), ("to", "to_shown", 1)] {
            let mut file = serde_json::json!({"name":"Shared", "schema_version":1, "links":[{
                "from":"jab", "to":"jab", "events":[], "verdict":{"kind":"true","link":null}
            }]});
            file["links"][0][field] = serde_json::json!(key);
            let err = serde_json::from_value::<ComboRecord>(file.clone()).unwrap_err();
            assert!(err.to_string().contains(field), "{field}={key}: {err}");

            file["links"][0][field] = serde_json::json!("other");
            file["links"][0]["from_shown"] = serde_json::json!({});
            file["links"][0]["to_shown"] = serde_json::json!({});
            file["links"][0][shown] = serde_json::json!({"move": key});
            let crop = serde_json::from_value::<ComboRecord>(file).unwrap().crop_units(0, 1).unwrap();
            let saved = serde_json::to_value(&crop).unwrap();
            assert_eq!(saved["links"][0][field], "other");
            assert_eq!(saved["links"][0][shown], serde_json::json!({"move": key}));
            let reopened: ComboRecord = serde_json::from_value(saved).unwrap();
            assert_eq!(reopened.combo.get(0).unwrap().desc(side).mv, mv);
            assert_eq!(reopened, crop);
        }
    }
}

#[test]
fn the_grab_throw_and_final_statuses_are_the_games() {
    use super::ids::{Table, C};
    let table = Table::with(&[(C::Catch, CATCH), (C::CatchAttack, CATCH_ATTACK),
        (C::Throw, THROW), (C::Final, FINAL)]);
    for status in CATCH..=CATCH_ATTACK {
        assert_eq!(table.named_move(status), Some(Move::Grab), "{status:#x}");
    }
    for status in [ATTACK_AIR, CATCH_CUT, CATCH_JUMP, THROW_KIRBY, FINAL + 1] {
        assert_eq!(table.named_move(status), None, "{status:#x}");
    }
    assert_eq!(table.named_move(THROW), Some(Move::Throw));
    assert_eq!(table.named_move(FINAL), Some(Move::FinalSmash));
    assert_eq!(table.attack(FINAL), None);
    assert_eq!(table.attack(FINAL + 1), Some(Move::Other));
    assert_eq!(ids::NAMES[C::Catch as usize], "FIGHTER_STATUS_KIND_CATCH");
    assert_eq!(ids::NAMES[C::CatchAttack as usize], "FIGHTER_STATUS_KIND_CATCH_ATTACK");
    assert_eq!(ids::NAMES[C::Throw as usize], "FIGHTER_STATUS_KIND_THROW");
    assert_eq!(ids::NAMES[C::Final as usize], "FIGHTER_STATUS_KIND_FINAL");
}
}
