lib_unit_tests! {
use super::*;
use crate::model::combo_record::ComboRecord;
use crate::model::move_names::Namer;
use serde_json::{json, Value};

const FIVE: [(Move, &str); 5] = [
    (Move::Grab, "grab"), (Move::Throw, "throw"), (Move::LedgeAttack, "ledge_attack"),
    (Move::GetupAttack, "getup_attack"), (Move::TripAttack, "trip_attack"),
];

fn link_value(from: &str, to: &str) -> Value {
    json!({"from": from, "to": to, "events": [], "verdict": {"kind": "true", "link": null}})
}

fn file(links: Vec<Value>) -> Value {
    json!({"schema_version": 1, "name": "Shared", "links": links})
}

fn read(value: Value) -> ComboRecord { serde_json::from_value(value).unwrap() }

fn sided(key: &str, side: usize, named: bool, variant: u8) -> Value {
    let field = if side == 0 { "from" } else { "to" };
    let shown = if side == 0 { "from_shown" } else { "to_shown" };
    let mut link = link_value("jab", "jab");
    link["from_shown"] = json!({});
    link["to_shown"] = json!({});
    link[field] = json!(if named { key } else { "other" });
    if !named { link[shown]["move"] = json!(key); }
    if variant != 0 { link[shown]["variant"] = json!(variant); }
    link
}

fn desc(mv: Move, normal: u8, angle: Option<Angle>, of: Option<Move>) -> ActionDesc {
    ActionDesc { mv, of, angle, normal, parts: [None; 2] }
}

fn described(from: ActionDesc, to: ActionDesc) -> LinkRec {
    let mut link = LinkRec { from: from.mv, to: to.mv, verdict: Verdict::True(None), ..LinkRec::INPUT };
    link.set_descriptions([from, to]);
    link
}

fn plain(from: Move, to: Move) -> LinkRec {
    LinkRec { from, to, verdict: Verdict::True(None), ..LinkRec::INPUT }
}

#[test]
fn a_new_link_names_a_grab_a_throw_and_the_edge_attacks_and_round_trips() {
    let pair = described(desc(Move::Grab, 5, None, None), desc(Move::Throw, 7, None, None));
    let record = ComboRecord::new("Shared".into(), Combo::from_links([pair]).unwrap(), None);
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["links"][0]["from"], "grab");
    assert_eq!(value["links"][0]["to"], "throw");
    assert_eq!(value["links"][0]["from_shown"], json!({"variant": 5}));
    assert_eq!(value["links"][0]["to_shown"], json!({"variant": 7}));
    assert_eq!(read(value), record);
    for (mv, key) in FIVE {
        for side in 0..2 {
            let named = desc(mv, 2, None, None);
            let tilt = desc(Move::SideTilt, 0, Some(Angle::Up), None);
            let pair = if side == 0 { described(named, tilt) } else { described(tilt, named) };
            let record = ComboRecord::new("Shared".into(), Combo::from_links([pair]).unwrap(), None);
            let value = serde_json::to_value(&record).unwrap();
            let field = if side == 0 { "from" } else { "to" };
            let shown = if side == 0 { "from_shown" } else { "to_shown" };
            let other_shown = if side == 0 { "to_shown" } else { "from_shown" };
            assert_eq!(value["links"][0][field], key, "{key} side={side}");
            assert_eq!(value["links"][0][shown], json!({"variant": 2}), "{key} side={side}");
            assert_eq!(value["links"][0][other_shown], json!({"angle": "up"}), "{key} side={side}");
            let reopened = read(value);
            assert!(reopened.combo.get(0).unwrap().described(), "{key} side={side}");
            assert_eq!(reopened, record, "{key} side={side}");
        }
    }
}

#[test]
fn an_old_file_with_other_and_a_description_reads_as_before() {
    for (mv, key) in FIVE {
        for side in 0..2 {
            let source = file(vec![sided(key, side, false, 2)]);
            let old = read(source.clone());
            let link = old.combo.get(0).unwrap();
            assert_eq!(link.desc(side).mv, mv, "{key} side={side}");
            assert_eq!(link.desc(side).normal, 2, "{key} side={side}");
            assert!(link.described(), "{key} side={side}");
            let new = read(file(vec![sided(key, side, true, 2)]));
            assert_eq!(old.combo, new.combo, "{key} side={side}");
            assert_eq!(serde_json::to_value(&old).unwrap(), source, "{key} side={side}");
        }
    }
    let old = read(file(vec![link_value("jab", "other")]));
    let link = old.combo.get(0).unwrap();
    assert_eq!(link.to, Move::Other);
    assert!(!link.described());
}

#[test]
fn a_file_mixing_old_and_new_links_reads_both() {
    for (mv, key) in FIVE {
        let old = sided(key, 1, false, 0);
        let mut new = sided(key, 1, true, 0);
        new["from"] = json!("grab");
        let plain_old = link_value("jab", "other");
        let plain_new = link_value("throw", key);
        let record = read(file(vec![old, new, plain_old, plain_new]));
        for (i, (from, to, has_descriptions)) in [
            (Move::Jab, mv, true), (Move::Grab, mv, true),
            (Move::Jab, Move::Other, false), (Move::Throw, mv, false),
        ].into_iter().enumerate() {
            let link = record.combo.get(i).unwrap();
            assert_eq!((link.from, link.to, link.described()), (from, to, has_descriptions), "{key} side=to link={i}");
        }
        let crop = record.crop_units(0, 4).unwrap();
        let value = serde_json::to_value(&crop).unwrap();
        for i in 0..4 {
            assert_eq!(value["links"][i]["to"], if i == 2 { "other" } else { key }, "{key} side=to link={i}");
            if i < 2 {
                assert!(value["links"][i]["to_shown"].get("move").is_none(), "{key} side=to link={i}");
                assert!(value["links"][i]["from_shown"].get("move").is_none(), "{key} side=from link={i}");
            } else {
                assert!(value["links"][i].get("from_shown").is_none(), "{key} side=from link={i}");
                assert!(value["links"][i].get("to_shown").is_none(), "{key} side=to link={i}");
            }
        }
        assert_eq!(value["links"][1]["from"], "grab", "{key} side=from");
        assert_eq!(value["links"][3]["from"], "throw", "{key} side=from");
        assert_eq!(read(value), crop, "{key} side=to");
    }
}

#[test]
fn the_combo_and_drill_views_name_them_the_same() {
    for (_, key) in FIVE {
        let old = read(file(vec![sided(key, 1, false, 0)]));
        let new = read(file(vec![sided(key, 1, true, 0)]));
        assert_eq!(old.combo, new.combo, "{key} side=to");
        let old_line = crate::model::recording_menu::combo_link_lines(&old.combo, Namer::OFF)[0].clone();
        let new_line = crate::model::recording_menu::combo_link_lines(&new.combo, Namer::OFF)[0].clone();
        assert_eq!(old_line, new_line, "{key} side=to");
        let drill = read(json!({"schema_version": 2, "kind": "input_drill", "name": "Shared",
            "input_steps": [{"events": [{"kind": "press", "button": "attack", "dir": "neutral", "offset": 0}],
                "to_shown": {"move": key}}]}));
        let drill_line = crate::model::recording_menu::combo_link_lines(&drill.combo, Namer::OFF)[0].clone();
        assert_eq!(new_line.split_once("→ ").unwrap().1.strip_suffix(" TRUE").unwrap(),
            drill_line.strip_prefix("S1 ").unwrap(), "{key} side=to");
    }
}

#[test]
fn a_final_smash_is_still_refused_straight_in_from_or_to() {
    for side in 0..2 {
        let field = if side == 0 { "from" } else { "to" };
        let shown = if side == 0 { "from_shown" } else { "to_shown" };
        let mut bad = link_value("jab", "jab");
        bad[field] = json!("final_smash");
        let error = serde_json::from_value::<ComboRecord>(file(vec![bad])).unwrap_err();
        assert!(error.to_string().contains(field), "final_smash side={side}: {error}");
        let crop = read(file(vec![sided("final_smash", side, false, 0)])).crop_units(0, 1).unwrap();
        let value = serde_json::to_value(&crop).unwrap();
        assert_eq!(value["links"][0][field], "other", "final_smash side={side}");
        assert_eq!(value["links"][0][shown], json!({"move": "final_smash"}), "final_smash side={side}");
        let reopened = read(value);
        assert_eq!(reopened.combo.get(0).unwrap().desc(side).mv, Move::FinalSmash, "final_smash side={side}");
        assert_eq!(reopened, crop, "final_smash side={side}");
    }
}

#[test]
fn a_named_endpoint_takes_no_move_in_its_description() {
    for (_, key) in FIVE {
        for side in 0..2 {
            let shown = if side == 0 { "from_shown" } else { "to_shown" };
            let mut link = sided(key, side, true, 0);
            link[shown]["move"] = json!(key);
            let error = serde_json::from_value::<ComboRecord>(file(vec![link])).unwrap_err();
            assert!(error.to_string().contains(shown), "{key} side={side}: {error}");
        }
    }
}

#[test]
fn a_named_endpoint_without_descriptions_grades_by_its_name() {
    for (mv, key) in FIVE {
        let named = plain(Move::Jab, mv);
        let old = plain(Move::Jab, Move::Other);
        for (actual, actual_key) in FIVE.into_iter().chain([(Move::Other, "other"), (Move::Jab, "jab")]) {
            let actual_desc = desc(actual, 0, None, (actual == Move::Other).then_some(Move::UpSpecial));
            let attempt = described(desc(Move::Jab, 0, None, None), actual_desc);
            assert_eq!(grade(&named, &attempt).wrong_move, actual != mv, "{key} side=to actual={actual_key}");
            assert_eq!(grade(&old, &attempt).wrong_move, actual == Move::Jab, "{key} side=to old actual={actual_key}");
        }
        let other = FIVE.into_iter().find(|(candidate, _)| *candidate != mv).unwrap();
        for (reference, expected_other) in [(mv, false), (Move::Other, true)] {
            let target = Combo::from_links([plain(reference, Move::Jab)]).unwrap();
            for (actual, actual_key, expected) in [(mv, key, true), (other.0, other.1, expected_other)] {
                let mut helper = Helper::new();
                helper.set_target(Some(target));
                let rec = described(desc(actual, 0, None, None), desc(Move::Jab, 0, None, None));
                let mut out = Out::default();
                helper.on_link(rec, true, 1, &mut out);
                assert_eq!(helper.attempt().is_some(), expected, "{key} side=from actual={actual_key} reference={reference:?}");
            }
            for (actual, actual_key) in [(mv, key), (other.0, other.1)] {
                let mut helper = Helper::new();
                helper.continue_with(target);
                let rec = described(desc(Move::Jab, 0, None, None), desc(actual, 0, None, None));
                let mut out = Out::default();
                helper.on_link(rec, false, 0, &mut out);
                if reference == mv && actual != mv {
                    let result = out.resolved.expect("named continuation must stop");
                    assert_eq!((result.outcome, result.fault), (Outcome::Stopped, Some((0, Reason::WrongMove(actual)))),
                        "{key} side=from continuation={actual_key}");
                } else {
                    assert!(out.resolved.is_none(), "{key} side=from continuation={actual_key} reference={reference:?}");
                }
            }
        }
    }
}

#[test]
fn a_crop_of_an_old_file_writes_the_names() {
    for (mv, key) in FIVE {
        for side in 0..2 {
            let field = if side == 0 { "from" } else { "to" };
            let shown = if side == 0 { "from_shown" } else { "to_shown" };
            let crop = read(file(vec![sided(key, side, false, 0)])).crop_units(0, 1).unwrap();
            let value = serde_json::to_value(&crop).unwrap();
            assert_eq!(value["links"][0][field], key, "{key} side={side}");
            assert!(value["links"][0][shown].get("move").is_none(), "{key} side={side}");
            let reopened = read(value);
            assert_eq!(reopened.combo.get(0).unwrap().desc(side).mv, mv, "{key} side={side}");
            assert_eq!(reopened, crop, "{key} side={side}");
        }
    }
}

#[test]
fn the_format_docs_say_which_builds_read_a_named_link() {
    let guide = include_str!("../../docs/guide/combos.md");
    let moves = guide.split("**Moves**").nth(1).unwrap().split("### Reading rules").next().unwrap();
    let list = moves.split("These are kinds of").next().unwrap();
    let other = list.find("`other`").unwrap();
    for (_, key) in FIVE {
        assert!(list.find(&format!("`{key}`")).is_some_and(|at| at < other), "{key} side=from/to");
    }
    assert!(guide.contains("not backward compatible"));
    assert!(guide.contains("a945b0c1"));
    assert!(guide.contains("earlier shipped installs"));
    let design = include_str!("../../docs/design/action-model.md");
    assert!(design.contains("not backward compatible"));
    assert!(design.contains("a945b0c1"));
    assert!(design.contains("earlier shipped installs"));
    assert!(design.contains("refuses `final_smash` straight in `from` or `to`"));
}
}
