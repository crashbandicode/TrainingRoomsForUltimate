//! Input-only references use the same event detector, matcher, attempt and store as hit combos.
lib_unit_tests! {
use super::*;
use crate::model::combo_record::{ComboLibrary, ComboRecord, UnitKind};

fn snap(frame: u32, buttons: u32) -> Snap { Snap { buttons, facing: 1, ..Snap::empty(frame) } }
fn capture(script: &[(u32,u32)]) -> Combo {
    let mut input = InputCapture::new(); for &(f,b) in script { input.step(snap(f,b)); } input.combo
}
fn run(helper: &mut Helper, until: u32, script: &[(u32,u32)]) {
    let mut buttons=0;
    for f in 0..=until { if let Some((_,b))=script.iter().find(|(at,_)| *at==f) { buttons = *b; }
        helper.settings = Settings::factory(); helper.step_inputs(snap(f,buttons), &mut Out::default());
    }
}
#[test]
fn input_idle_groups_and_continuations_share_combo_bounds() {
    assert_eq!(capture(&[(0,1),(20,0)]).len(),1);
    assert_eq!(capture(&[(0,1),(21,0)]).len(),2);
    let mut c=InputCapture::new();
    for f in 0..(LINKS*STEPS) { c.step(snap(f as u32, if f%2==0 {1} else {0})); }
    assert_eq!(c.combo.len(),LINKS); assert!(!c.overflow);
    c.step(snap((LINKS*STEPS) as u32,1)); assert!(c.overflow);
}
#[test]
fn input_attempt_counts_steps_and_drops_late_or_missing_releases() {
    let reference=capture(&[(0,1),(2,0),(23,2),(25,0)]);
    assert_eq!(reference.len(),2);
    let mut helper=Helper::new(); helper.set_target(Some(reference));
    run(&mut helper,27,&[(0,1),(2,0),(22,2),(24,0)]);
    assert!(helper.results().next().unwrap().clean); assert_eq!((helper.streak(),helper.best()),(1,2));
    assert!(helper.attempt().unwrap().grades().all(|g| !g.is_true() && g.input_only));
    helper.set_target(Some(reference)); run(&mut helper,27,&[(0,1),(2,0),(23,2)]);
    let result=helper.results().next().unwrap(); assert!(!result.clean); assert_eq!(result.fault,Some((1,Reason::MissingRelease(Button::Special))));
    assert_eq!((helper.streak(),helper.best()),(0,1));
    helper.set_target(Some(reference)); run(&mut helper,10,&[(0,1),(5,0)]);
    assert!(!helper.results().next().unwrap().clean);
    let want=reference.get(0).unwrap(); let actual=LinkRec { events: retain([Event {kind:Kind::Press(Button::Attack),dir:Dir::Neutral,offset:0}]), ..LinkRec::INPUT };
    assert!(grade(want,&actual).missing().is_none(),"hit-clipped release exemption remains");
    assert!(grade_mode(want,&actual,true).missing().is_some());
}
#[test]
fn input_schema_crop_and_hit_format_remain_distinct() {
    let combo=capture(&[(0,1),(2,0),(23,2),(25,0)]);
    let original=ComboRecord::new("Drill 1".into(),combo,Some("Mario".into())).with_made_from("macro-1");
    let value=serde_json::to_value(&original).unwrap();
    assert_eq!(value["schema_version"],2); assert_eq!(value["kind"],"input_drill"); assert!(value.get("links").is_none());
    let read:ComboRecord=serde_json::from_value(value).unwrap(); assert_eq!(read,original);
    assert_eq!((read.unit_kind(),read.unit_count()),(UnitKind::Step,2));
    let crop=read.crop_units(1,2).unwrap(); assert_eq!(crop.combo.get(0).unwrap().events().next().unwrap().offset,0);
    assert_eq!(crop.made_from.as_deref(),Some("macro-1")); assert_eq!(read.unit_count(),2);
    assert!(read.crop_units(1,1).is_err()); assert!(read.crop_units(0,3).is_err());
    let hit=ComboRecord::new("Combo 1".into(),Combo::from_links([LinkRec::INPUT]).unwrap(),None);
    let hit_value=serde_json::to_value(&hit).unwrap(); assert_eq!(hit_value["schema_version"],1); assert!(hit_value.get("kind").is_none()); assert!(hit_value.get("input_steps").is_none());
    let root=std::env::temp_dir().join(format!("tr-input-schema-{}",std::process::id())); std::fs::create_dir_all(&root).unwrap();
    let lib=ComboLibrary::new(&root); let id=lib.save_new(&read).unwrap(); let before=std::fs::read(root.join(format!("{id}.json"))).unwrap();
    lib.save_new(&crop).unwrap(); assert_eq!(std::fs::read(root.join(format!("{id}.json"))).unwrap(),before); std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn input_direction_and_airborne_facts_use_existing_semantics() {
    let mut c=InputCapture::new(); c.step(Snap { lstick_x:-127,facing:-1,..Snap::empty(0) });
    c.step(Snap { lstick_y:-100,airborne:true,facing:-1,..Snap::empty(1) });
    let events=c.combo.get(0).unwrap().events().collect::<Vec<_>>();
    assert_eq!((events[0].kind,events[0].dir),(Kind::Flick(Flick::Dash),Dir::Forward));
    assert!(events.iter().any(|e| e.kind==Kind::Flick(Flick::FastFall)));
}
}
