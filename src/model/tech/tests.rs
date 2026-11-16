lib_unit_tests! {
use super::*;
use surface::{Kind, Line, Source, Surfaces, Walk};
use session::*;
use variables::*;
use super::super::tech_launch::{self as flight, FlightParams};

fn v(x: f32, y: f32) -> Vec2 { Vec2 { x, y } }
fn line(id: usize, a: Vec2, b: Vec2, n: Vec2) -> Line {
    Line { id, ends: [a,b], normal: n, floor: n.y >= 0.7, wall: n.x.abs() == 1.0, passable: false }
}
struct Stage { lines: Vec<Line> }
impl Stage {
    fn fixture() -> Self {
        Self { lines: vec![line(0,v(-40.,0.),v(40.,0.),v(0.,1.)), line(1,v(40.,0.),v(40.,-20.),v(1.,0.)),
            line(2,v(40.,-20.),v(-40.,-20.),v(0.,-1.)), line(3,v(-40.,-20.),v(-40.,0.),v(-1.,0.))] }
    }
    fn walk(&mut self) -> Surfaces {
        let mut s = Surfaces::EMPTY;
        s.begin(0,1);
        for _ in 0..4 { s.step(self); }
        s
    }
}
impl Source for Stage {
    fn read(&mut self, id: usize) -> Option<Line> { self.lines.get(id).copied() }
    fn next(&mut self, line: Line, _: usize) -> Option<(usize, usize)> { Some(((line.id+1)%self.lines.len(),1)) }
}
#[test]
fn surface_kind_floor_wall_ceiling_slopes() {
    let mut s = Stage::fixture();
    assert_eq!(s.lines.iter().map(|l| l.kind()).collect::<Vec<_>>(), vec![Some(Kind::Floor),Some(Kind::Wall),Some(Kind::Ceiling),Some(Kind::Wall)]);
    for (normal, floor, wall, want) in [(v(0.6,0.8),false,false,Some(Kind::Floor)), (v(0.8,0.6),false,true,Some(Kind::Wall)),
        (v(0.6,-0.8),false,false,Some(Kind::Ceiling)), (v(0.8,0.6),true,false,Some(Kind::Floor))] {
        let l = Line { normal, floor, wall, ..s.lines[0] };
        assert_eq!(l.kind(),want);
    }
    s.lines[0].passable = true;
    assert_eq!(s.lines[0].kind(), Some(Kind::Floor));
    assert_eq!(surface::crossing(v(0.,1.),v(0.,-1.),s.lines[0]),Some(0.5));
    assert_eq!(surface::crossing(v(0.,-1.),v(0.,1.),s.lines[0]),None);
}
#[test]
fn surface_walk_closes_the_outline() {
    let mut stage = Stage::fixture();
    let s = stage.walk();
    assert_eq!((s.state,s.lines().count()),(Walk::Closed,4));
    assert!(solver::inside(&s,v(0.,-10.)));
    assert!(!solver::inside(&s,v(0.,1.)));
    struct Endless;
    impl Source for Endless {
        fn read(&mut self,id:usize)->Option<Line>{ Some(line(id,v(0.,0.),v(20.,0.),v(0.,1.))) }
        fn next(&mut self,l:Line,_:usize)->Option<(usize,usize)>{ Some((l.id+1,1)) }
    }
    let mut full=Surfaces::EMPTY; full.begin(0,1);
    for _ in 0..64 { full.step(&mut Endless); }
    assert_eq!((full.state,full.lines().count()),(Walk::Full,64));
}
#[test]
fn unclassified_solid_line_keeps_outline_closed() {
    let mut stage=Stage::fixture();
    stage.lines[1]=line(1,v(40.,0.),v(50.,-20.),v(0.8944272,0.4472136));
    stage.lines[2].ends[0]=v(50.,-20.);
    let slope=stage.lines[1];
    assert_eq!(slope.kind(),None);
    let s=stage.walk();
    assert_eq!((s.state,s.lines().count()),(Walk::Closed,4));
    assert!(solver::inside(&s,v(44.,-10.)));
    assert!(solver::inside(&s,v(45.,-10.))); // boundary is retained geometry
    assert!(!solver::inside(&s,v(46.,-10.)));
    assert_eq!(surface::crossing(v(44.,-10.),v(46.,-10.),slope),Some(0.5));
    assert_ne!(s.nearest(v(45.,-10.),None).unwrap().0.id,slope.id);
    assert_eq!(slope.random_point(slope,0.5),None);
}
#[test]
fn point_snaps_to_nearest_surface() {
    let mut stage=Stage::fixture(); let s=stage.walk();
    assert_eq!(s.nearest(v(45.,-10.),None).map(|(_,p)|p),Some(v(40.,-10.)));
    assert_eq!(s.nearest(v(0.,-25.),None).map(|(_,p)|p),Some(v(0.,-20.)));
    let l=stage.lines[0];
    assert_eq!(l.random_point(l,0.),Some(v(-35.,0.)));
    assert_eq!(l.random_point(l,1.),Some(v(35.,0.)));
    assert_eq!(l.random_point(Line { ends:[v(-40.,1.),v(40.,1.)], ..l },0.5),None);
    assert_eq!(line(9,v(0.,0.),v(9.,0.),v(0.,1.)).random_point(line(9,v(0.,0.),v(9.,0.),v(0.,1.)),0.5),None);
}
fn aim<'a>(s:&'a Surfaces,l:Line,target:Vec2)->solver::Aim<'a>{
    solver::Aim { surfaces:s,line:l,target,knockback:100.,params:FlightParams {brake:0.051,gravity:0.06,fall_speed_cap:1.5},
        blast:solver::Box2 {min:v(-200.,-200.),max:v(200.,200.)},view:None }
}
#[test]
fn solver_hits_target_within_tolerance() {
    let mut stage=Stage::fixture();let s=stage.walk();
    for (i,target) in [(0,v(0.,0.)),(1,v(40.,-10.)),(2,v(0.,-20.)),(3,v(-40.,-10.))] {
        let a=aim(&s,stage.lines[i],target); let solved=a.solve(None,10).expect("surface solution");
        let initial=flight::launch_velocity(100.,flight::direction_degrees(solved.angle).unwrap()).unwrap();
        let p=add(solved.start,flight::flight(initial,a.params,10).unwrap().displacement);
        assert!((p.x-target.x).abs()<=0.00001 && (p.y-target.y).abs()<=0.00001);
    }
}
#[test]
fn solver_aims_upward_at_underside() {
    let mut stage=Stage::fixture();let s=stage.walk();let a=aim(&s,stage.lines[2],v(0.,-20.));
    let solved=a.solve(None,10).unwrap();assert_eq!(solved.angle,90.); assert!(solved.start.y < -20.);
    let end=flight::flight(v(0.,3.),a.params,10).unwrap(); assert!(end.damage_velocity.y-end.gravity_speed>0.);
}
#[test]
fn solver_start_is_inside_blast_zones_and_clear_of_geometry() {
    let mut stage=Stage::fixture();let s=stage.walk();let mut a=aim(&s,stage.lines[2],v(0.,-20.));
    assert!(a.solve(Some(90.),10).is_some());
    a.blast.min.y = -21.;assert!(a.solve(Some(90.),10).is_none());
    a.blast.min.y = -200.;a.view=Some(solver::Box2 {min:v(-1.,-21.),max:v(1.,1.)});assert!(a.solve(Some(90.),10).is_none());
    // A downward shot to the underside crosses the top floor first.
    a.view=None;assert!(a.solve(Some(270.),10).is_none());
}
#[test]
fn time_to_contact_stays_inside_hitstun() {
    let mut stage=Stage::fixture();let s=stage.walk();let a=aim(&s,stage.lines[0],v(0.,0.));
    assert!(a.solve(Some(270.),38).is_some());assert!(a.solve(Some(270.),39).is_none());
    let fast=solver::Aim {knockback:250.,..a};assert!(fast.solve(Some(270.),10).is_none());
}
#[test]
fn flight_matches_hand_computed_frames() {
    let p=FlightParams {brake:0.051,gravity:0.,fall_speed_cap:0.};
    let f=flight::flight(v(3.,0.),p,2).unwrap();assert_eq!(f.displacement.x,3.+(3.-0.051));assert_eq!(f.damage_velocity.x,(3.-0.051)-0.051);
}
#[test]
fn launch_speed_from_knockback() {
    assert_eq!(flight::launch_velocity(100.,v(1.,0.)),Some(v(3.,0.)));
    assert_eq!(flight::reaction_frames(100.,0.4,5.),Some(40.));
}
#[test]
fn random_variables_reroll_each_attempt() {
    let r=Variable {mode:Mode::Random,low:10.,high:90.,step:5.};let mut n=0;
    let mut draw=|bound|{let i=n%bound;n+=1;i};
    let all:Vec<_>=(0..34).map(|_|r.sample(&mut draw).unwrap()).collect();
    assert_ne!(all[0],all[1]);assert!(all.iter().all(|v|(10. ..=90.).contains(v)));assert_eq!(all[0],all[17]);
}
#[test]
fn fixed_variables_stay() {
    let r=Variable {mode:Mode::Set,low:60.,high:150.,step:5.};
    for _ in 0..20 {assert_eq!(r.sample(&mut |_|panic!("fixed must not draw")),Some(60.));}
    assert_eq!(Variable {mode:Mode::Auto,..r}.sample(&mut |_|panic!("auto must not draw")),None);
}
#[test]
fn di_off_writes_neutral_stick() {
    assert_eq!(Di::Off.sample(&mut |_|panic!("off must not draw")),Some(v(0.,0.)));
    assert_eq!(Di::Player.sample(&mut |_|panic!("player owns DI")),None);
}
fn decision()->Decision{ Decision {tech:None,count:254,previous:254,window:12,lockout:28,reaction:20.,damage_fall:false,enabled:true,press_frame_offset:0} }
#[test]
fn grade_each_offset() {
    for c in 0..12 { assert_eq!(grade(Decision{tech:Some(Direction::InPlace),count:c,..decision()}),Some(Grade::Perfect(c))); }
    for direction in [Direction::InPlace,Direction::RollLeft,Direction::RollRight,Direction::Wall,Direction::WallJump,Direction::Ceiling] {
        assert_eq!(grade(Decision{tech:Some(direction),count:4,..decision()}),Some(Grade::Perfect(4)));
    }
}
#[test]
fn early_by_counts_frames_past_window() {
    for c in 12..40 { assert_eq!(grade(Decision{count:c,..decision()}),Some(Grade::Early(c-11))); }
    assert_eq!(grade(Decision{count:40,..decision()}),None);
}
#[test]
fn lockout_grade() {
    assert_eq!(grade(Decision{count:3,previous:27,..decision()}),Some(Grade::Locked(27)));
    assert_eq!(grade(Decision{tech:Some(Direction::Wall),count:3,previous:28,..decision()}),Some(Grade::Perfect(3)));
}
#[test]
fn held_tech_is_untimed() {
    for d in [Decision{tech:Some(Direction::Wall),reaction:0.,..decision()},Decision{tech:Some(Direction::InPlace),damage_fall:true,..decision()}] {assert_eq!(grade(d),Some(Grade::Held));}
}
fn flying(length:Length)->Session{let mut s=Session::IDLE;s.start(length);s.placed(1);assert_eq!(s.tick(false,false),Action::Launch);s}
#[test]
fn untechable_not_counted() {let mut s=flying(Length::Attempts(10));s.contact(Decision{enabled:false,..decision()});assert_eq!(s.result,Some(Grade::Untechable));assert_eq!(s.summary.attempts,0);}
#[test]
fn late_by_counts_frames_after_contact() {
    for frame in [1,3,30,31] { let mut s=flying(Length::Free);s.contact(decision());for age in 1..=31{s.tick(false,age==frame);}
        assert_eq!(s.result,Some(if frame<=30{Grade::Late(frame)}else{Grade::Missed})); }
}
#[test]
fn reset_schedule_after_tech_and_miss() {
    for tech in [Some(Direction::InPlace),None] {let mut s=flying(Length::Free);s.contact(Decision{tech,count:12,..decision()});
        for _ in 1..45 {assert_eq!(s.tick(false,false),Action::None);}
        assert_eq!(s.tick(false,false),Action::Place{reroll:true});s.placed(10);
        for _ in 1..10{assert_eq!(s.tick(false,false),Action::None);}assert_eq!(s.tick(false,false),Action::Launch);
    }
}
#[test]
fn abandoned_attempt_relaunches_uncounted() {let mut s=flying(Length::Free);assert_eq!(s.abandon(),Action::Place{reroll:false});assert_eq!(s.summary.attempts,0);}
#[test]
fn session_attempts_end_after_n() {
    for n in [10,20] {let mut s=flying(Length::Attempts(n));
        for i in 0..n{s.contact(Decision{tech:Some(Direction::Wall),count:4,..decision()});for _ in 0..44{s.tick(false,false);}
            let action=s.tick(false,false);assert_eq!(action,if i+1==n{Action::Summary}else{Action::Place{reroll:true}});
            if i+1<n{s.placed(1);s.tick(false,false);}}
        assert_eq!(s.summary.attempts,u32::from(n));assert_eq!(s.phase,Phase::Done);
    }
}
#[test]
fn session_time_ends_and_finishes_the_flight() {let mut s=flying(Length::Frames(60));for _ in 0..100{s.tick(false,false);}assert_eq!(s.phase,Phase::Flight);
    s.contact(Decision{tech:Some(Direction::Ceiling),count:2,..decision()});for _ in 0..44{s.tick(false,false);}assert_eq!(s.tick(false,false),Action::Summary);assert_eq!(s.summary.attempts,1);
    let mut s=flying(Length::Frames(3600));let now=s.elapsed;for _ in 0..100{s.tick(true,false);}assert_eq!(s.elapsed,now);
}
#[test]
fn free_practice_never_ends() {let mut s=flying(Length::Free);for _ in 0..20000{s.tick(false,false);}assert_eq!(s.phase,Phase::Flight);assert_eq!(s.stop(),Action::Summary);}
#[test]
fn summary_counts_streak_and_average() {let mut s=Summary::default();for g in [Grade::Perfect(2),Grade::Perfect(6),Grade::Early(1),Grade::Held,Grade::Perfect(4)]{s.record(g);}
    assert_eq!((s.attempts,s.streak,s.best,s.average()),(5,2,2,Some(4.)));let mut text=String::new();s.write(&mut text).unwrap();assert!(text.len()<=96,"{text}");
}
#[test]
fn hit_damage_range_and_default() {assert_eq!(HIT_DAMAGE_DEFAULT,12.);assert_eq!(HIT_DAMAGE_RANGE,[1.,30.]);}
#[test]
fn presets_only_set_damage_and_height() {let mut l=Launch {start:v(2.,3.),target:v(4.,5.),angle:90.,knockback:100.,percent:60.,hit_damage:12.,height:HitHeight::Middle,hit_from_behind:false,facing:1.,di:Some(v(0.,0.))};let old=l;
    preset(&mut l,30.,HitHeight::High);assert_eq!(l,Launch {hit_damage:30.,height:HitHeight::High,..old});
}
}
