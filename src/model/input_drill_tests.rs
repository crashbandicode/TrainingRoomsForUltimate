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
fn input_idle_flag_does_not_start_a_drill_or_hide_jump() {
    // Native rc15-input-drill-1b134e51: neutral 0x8000 produced a phantom Jump
    // before four real dash inputs. Existing macro capture treats it as idle.
    assert!(capture(&[(0,FLICK_JUMP),(30,FLICK_JUMP)]).is_empty());
    let c=capture(&[(0,FLICK_JUMP),(30,FLICK_JUMP|0x4),(32,FLICK_JUMP)]);
    let events=c.get(0).unwrap().events().collect::<Vec<_>>();
    assert_eq!(events,vec![Event {kind:Kind::Press(Button::Jump),dir:Dir::Neutral,offset:0},Event {kind:Kind::Release(Button::Jump),dir:Dir::Neutral,offset:2}]);
    let first_jump=capture(&[(0,FLICK_JUMP|0x4),(2,FLICK_JUMP)]);
    assert_eq!(first_jump.get(0).unwrap().events().collect::<Vec<_>>(),events);
    let mut helper=Helper::new(); helper.set_target(Some(c));
    run(&mut helper,35,&[(0,FLICK_JUMP),(30,FLICK_JUMP|0x4),(32,FLICK_JUMP)]);
    assert!(helper.results().next().unwrap().clean); assert_eq!((helper.best(),helper.streak()),(1,1));
    let first=capture(&[(0,FLICK_JUMP|1),(1,FLICK_JUMP)]);
    assert_eq!(first.get(0).unwrap().events().next().unwrap().kind,Kind::Press(Button::Attack));
    assert_eq!(first.get(0).unwrap().events().count(),2);
    let mut tap=InputCapture::new(); tap.step(Snap {buttons:FLICK_JUMP,lstick_y:100,facing:1,..Snap::empty(0)});
    assert!(tap.combo.get(0).unwrap().events().any(|e| e.kind==Kind::Flick(Flick::TapJump)));
}
#[test]
fn four_dash_fixture_with_native_idle_flag_saves_and_grades_four_steps() {
    // Same four dash blocks as drill-move.edenmacro.json (0,46,91,137),
    // preceded by the 95 idle frames observed in the failed native take.
    let starts=[95,141,186,232];
    let snapshot=|frame| Snap {
        buttons:FLICK_JUMP,
        lstick_x:starts.iter().position(|start| (*start..*start+12).contains(&frame))
            .map_or(0,|i| if i%2==0 {127} else {-127}),
        facing:1,
        ..Snap::empty(frame)
    };
    let mut capture=InputCapture::new();
    for frame in 0..=250 { capture.step(snapshot(frame)); }
    let drill=capture.combo;
    assert_eq!(drill.len(),4);
    assert!(drill.links().all(|link| link.events().count()==1 && link.events().next().unwrap().kind==Kind::Flick(Flick::Dash)));
    assert_eq!(drill.links().map(|link| link.events().next().unwrap().offset).collect::<Vec<_>>(),vec![0,46,91,137]);
    let mut helper=Helper::new(); helper.set_target(Some(drill));
    for frame in 0..=250 {
        helper.step_inputs(Snap {frame:500+frame,..snapshot(frame)},&mut Out::default());
    }
    let result=helper.results().next().unwrap();
    assert!(result.clean); assert_eq!(result.graded,4);
    assert_eq!((helper.best(),helper.streak()),(4,1));
}

#[test]
fn input_idle_groups_and_continuations_share_combo_bounds() {
    assert_eq!(capture(&[(0,1),(20,0)]).len(),1);
    assert_eq!(capture(&[(0,1),(21,0)]).len(),2);
    let mut c=InputCapture::new();
    for f in 0..(LINKS*STEPS) { c.step(snap(f as u32, if f%2==0 {1} else {0})); }
    assert_eq!(c.combo.len(),LINKS); assert!(!c.combo.overflow);
    c.step(snap((LINKS*STEPS) as u32,1)); assert!(c.combo.overflow);
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
    assert!(grade_mode(want,&actual,true,false).missing().is_some());
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
#[test]
fn input_bar_names_steps_and_has_no_hit_verdict_or_window() {
    use crate::model::{combo_plan::Renderer,frame_strip_plan::Text};
    use crate::model::panes::strip_layout::C;
    let mut h=Helper::new(); h.set_target(Some(capture(&[(0,1),(2,0)]))); run(&mut h,4,&[(0,1),(2,0)]);
    let mut texts=Vec::new(); Renderer::new().draw(&h,&Text::EMPTY,None,true,(&C,false),|_,look,_| { if look.visible { texts.push(String::from_utf16_lossy(look.text.units())); } });
    assert!(texts.iter().any(|t| t=="Step 1")); assert!(texts.iter().any(|t| t=="Drill"));
    assert!(texts.iter().all(|t| !t.contains("TRUE") && !t.contains("DROP") && !t.contains("other>other")));
}

#[test]
fn capacity_layout_receipt() {
    use core::mem::size_of;
    println!("CAPACITY_LAYOUT units={} LinkRec={} Combo={} Grade={} Attempt={} Helper={} InputCapture={} triple_combo={}", LINKS, size_of::<LinkRec>(), size_of::<Combo>(),size_of::<Grade>(),size_of::<Attempt>(),size_of::<Helper>(),size_of::<InputCapture>(),size_of::<hos_lock::TripleBuffer<Combo>>());
    assert_eq!(STEPS,12);
}

#[test]
fn full_capacity_both_kinds_grade_and_seventeenth_refuses() {
    assert_eq!(LINKS,16);
    let hit = LinkRec { verdict: Verdict::True(None), ..LinkRec::INPUT };
    let combo = Combo::from_links([hit;LINKS]).unwrap();
    let mut helper=Helper::new(); helper.set_target(Some(combo));
    for i in 0..LINKS { helper.on_link(hit,i==0,1,&mut Out::default(),None); }
    assert_eq!(helper.results().next().unwrap().graded,LINKS as u8); assert_eq!((helper.best(),helper.streak()),(LINKS as u8,1));
    let script=(0..LINKS as u32).flat_map(|i| [(i*22,1),(i*22+1,0)]).collect::<Vec<_>>();
    let drill=capture(&script); assert_eq!(drill.len(),LINKS);
    helper.set_target(Some(drill)); run(&mut helper,(LINKS as u32-1)*22+4,&script);
    assert_eq!(helper.results().next().unwrap().graded,LINKS as u8); assert!(helper.results().next().unwrap().clean);
    assert_eq!((helper.best(),helper.streak()),(LINKS as u8,1));
    assert!(Combo::from_links([hit;LINKS+1]).is_none());
    let mut full=combo; full.push(hit); assert_eq!(full.len(),LINKS); assert!(full.overflow);
}

#[test]
fn full_capacity_file_roundtrip_receipt() {
    use crate::model::frame_strip::{Move,Verdict};
    let mut events=[None;STEPS];
    for (i,e) in events.iter_mut().enumerate() { *e=Some(Event { kind:if i%2==0 { Kind::Press(Button::Special) } else { Kind::Release(Button::Special) },dir:Dir::ForwardDown,offset:i as i32 }); }
    let link=LinkRec { from:Move::NeutralSpecial,to:Move::NeutralSpecial,verdict:Verdict::True(None),window:Some((0,4)),events,drift:Some(Dir::ForwardDown),..LinkRec::INPUT };
    let record=ComboRecord::new("Combo full".into(),Combo::from_links([link;LINKS]).unwrap(),None);
    let bytes=serde_json::to_vec_pretty(&record).unwrap(); println!("CAPACITY_JSON bytes={}",bytes.len());
    let root=std::env::temp_dir().join(format!("tr-capacity-file-{}",std::process::id())); std::fs::create_dir_all(&root).unwrap();
    let lib=ComboLibrary::new(&root); let id=lib.save_new(&record).unwrap();
    let saved_bytes=std::fs::metadata(root.join(format!("{id}.json"))).unwrap().len();
    println!("CAPACITY_FILE bytes={saved_bytes} max={}", crate::model::combo_record::MAX_FILE_BYTES);
    let loaded=lib.load(&id); let _=std::fs::remove_dir_all(root); assert!(loaded.is_ok(),"saved full-capacity file must reload: {loaded:?}");
    assert_eq!(loaded.unwrap().combo,record.combo);
}

#[test]
fn capacity_file_read_bound_preserves_refusal() {
    use crate::model::{combo_record::{ComboError,MAX_FILE_BYTES},store::StoreError};
    assert_eq!(MAX_FILE_BYTES,LINKS as u64 * 4096);
    let root=std::env::temp_dir().join(format!("tr-capacity-bound-{}",std::process::id()));
    let lib=ComboLibrary::new(&root);
    let record=ComboRecord::new("Boundary".into(),Combo::from_links([LinkRec::INPUT]).unwrap(),None);
    let id=lib.save_new(&record).unwrap(); let path=root.join(format!("{id}.json"));
    // Legal JSON whitespace reaches the exact cap without changing the saved record.
    let mut data=std::fs::read(&path).unwrap(); data.resize(MAX_FILE_BYTES as usize,b' ');
    std::fs::write(&path,&data).unwrap(); assert_eq!(lib.load(&id).unwrap().combo,record.combo);
    data.push(b' '); std::fs::write(&path,&data).unwrap();
    let refused=lib.load(&id); let _=std::fs::remove_dir_all(root);
    assert!(matches!(refused,Err(ComboError::Store(StoreError::FileTooLarge { bytes,max })) if bytes==MAX_FILE_BYTES+1 && max==MAX_FILE_BYTES),"over-cap file must still refuse: {refused:?}");
}

#[test]
fn same_frame_continuations_grade_and_each_kind_keeps_its_hot_five() {
    let reference=capture(&[(0,31),(1,0),(2,31)]); assert_eq!(reference.len(),2);
    assert_eq!(reference.get(0).unwrap().events().last().unwrap().offset,2);
    assert_eq!(reference.get(1).unwrap().events().next().unwrap().offset,2);
    let mut h=Helper::new(); h.set_target(Some(reference)); run(&mut h,3,&[(0,31),(1,0),(2,31)]);
    assert_eq!(h.best(),2); assert!(h.results().next().unwrap().clean);
    use crate::engine::library::{RecordingIndex,Kind,Use};
    let mut index=RecordingIndex::fresh();
    for i in 0..6 { index.note_use(Kind::Combo,&format!("drill-{i}"),Use::Recorded,i).unwrap(); index.note_use(Kind::Combo,&format!("combo-{i}"),Use::Recorded,10+i).unwrap(); }
    let drills=index.hot_where(Kind::Combo,|id|id.starts_with("drill-"));
    assert_eq!(drills.len(),5); assert_eq!(drills[0],"drill-5"); assert!(!drills.iter().any(|id|id=="drill-0"));
    let hits=index.hot_where(Kind::Combo,|id|id.starts_with("combo-")); assert_eq!(hits.len(),5); assert_eq!(hits[0],"combo-5");
}

}
