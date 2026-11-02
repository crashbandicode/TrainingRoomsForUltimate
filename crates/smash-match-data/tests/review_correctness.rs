//! Shared selection regression from the operator's data-correctness review.
//! Every fixture goes through Recorder and validates without even warnings.
use smash_match_data::*;

fn clock(n: u64) -> Clocks {
    Clocks::tick(n).with_record(n)
}

fn identity(n: u8) -> ActorIdentity {
    ActorIdentity {
        entry: Some(n),
        controller: Some(n),
        control: ControlKind::Human,
        ..Default::default()
    }
}

fn recorder(actors: u8) -> Recorder {
    let axis = AxisLayout {
        encoding: AxisEncoding::F32,
        min: Some(-1.0),
        max: Some(1.0),
        neutral: Some(0.0),
        float_scale: Some(1.0),
        note: None,
    };
    let header = CaptureHeader {
        capture_id: "review-correctness".into(),
        recorder: ToolVersion::new("review-fixture", "1"),
        game: GameBuild::ultimate("13.0.5"),
        source: Source::live_match(),
        channels: ChannelSupport::all_full(),
        clocks: ClockSupport {
            supplied: vec![ClockKind::Tick, ClockKind::Record],
            relations: vec![],
        },
        rules: Rules {
            target: RulesetTarget::Competitive,
            actual: ActualRules {
                items: Some(ItemSetting::Off),
                stage_hazards: Some(false),
                ..Default::default()
            },
        },
        input_layouts: ["a", "b"]
            .into_iter()
            .map(|id| InputLayout {
                id: id.into(),
                description: None,
                buttons: ButtonLayout {
                    width_bits: 32,
                    names: Default::default(),
                },
                left_stick: axis.clone(),
                right_stick: axis.clone(),
                raw_encoding: None,
            })
            .collect(),
        ..Default::default()
    };
    let mut r = Recorder::in_memory(header).unwrap();
    r.boundary(clock(0), BoundaryKind::RecordingStart).unwrap();
    for n in 0..actors {
        r.spawn(
            clock(0),
            ActorId(u32::from(n) + 1),
            identity(n),
            SpawnCause::MatchStart,
        )
        .unwrap();
    }
    r
}

fn state(y: f32) -> FighterSample {
    FighterSample::unsupported()
        .with_kinematics(Kinematics {
            position: Vec3::new(0.0, y, 0.0),
            facing: 1.0,
            velocity: Some(Vec3::new(0.0, 1.25, 0.0)),
        })
        .with_situation(Situation {
            situation_kind: 1,
            status_kind: 1,
            grounded: Some(false),
            hitstop_frames: Some(0.0),
        })
        .with_animation(Animation {
            motion: MotionRef::new(1),
            frame: 1.25,
            rate: 1.0,
            root_rotation: None,
            root_scale: None,
            secondary: None,
        })
        .with_vitals(Vitals {
            damage: 0.0,
            stocks: Some(3),
            shield: None,
        })
}

fn input(actor: u32, x: f32, bits: u32) -> InputEvent {
    InputEvent {
        controller: Some((actor - 1) as u8),
        entry: Some((actor - 1) as u8),
        actor: Some(ActorId(actor)),
        origin: InputOrigin::Physical,
        layout: "a".into(),
        buttons: Channel::Observed(Buttons::new(bits)),
        left_stick: Channel::Observed(Stick::F32 { x, y: -0.0 }),
        right_stick: Channel::Observed(Stick::F32 { x: -x, y: 0.125 }),
        raw: None,
    }
}

fn finish(r: Recorder, tick: u64) -> Capture {
    let f = r.finish(clock(tick), Completion::complete()).unwrap();
    assert!(f.warnings.is_empty(), "{:?}", f.warnings);
    let c = f.into_capture().unwrap();
    assert!(c.validate().is_empty(), "{:?}", c.validate());
    c
}

#[test]
fn selecting_second_fighter_uses_its_current_before_state_and_matching_input_range() {
    let mut r = recorder(2);
    let mut selected_start = 0;
    let mut selected_input = 0;
    for tick in 1..=3 {
        for actor in 1..=2 {
            let y = if tick == 2 { 9.0 } else { tick as f32 };
            let before = r
                .state(clock(tick), ActorId(actor), Phase::Before, state(y))
                .unwrap();
            let applied = r
                .input(clock(tick), input(actor, 0.25, tick as u32))
                .unwrap();
            r.state(clock(tick), ActorId(actor), Phase::After, state(y + 0.5))
                .unwrap();
            if tick == 2 && actor == 2 {
                selected_start = before;
                selected_input = applied;
            }
        }
    }
    let c = finish(r, 3);
    for (start, end) in [
        (SelectAt::Tick(2), SelectAt::Tick(3)),
        (SelectAt::Record(2), SelectAt::Record(3)),
        (SelectAt::Seq(selected_start), SelectAt::Seq(selected_input)),
    ] {
        let context = c.clip_context_for(start, ActorId(2)).unwrap();
        let selected = context.actor(ActorId(2)).unwrap();
        match &selected.sample.kinematics {
            Resolved::Observed {
                value,
                phase,
                value_seq,
                ..
            } => {
                assert_eq!(value.position.y, 9.0);
                assert_eq!(*phase, Some(Phase::Before));
                assert_eq!(*value_seq, selected_start);
            }
            v => panic!("bad selected context {v:?}"),
        }
        let events = c.clip_events_for(start, end, ActorId(2)).unwrap();
        assert_eq!(events.first().unwrap().seq, selected_input);
        let inputs: Vec<_> = events
            .iter()
            .filter_map(|event| {
                matches!(
                    &event.body,
                    EventBody::Input(input) if input.actor == Some(ActorId(2))
                )
                .then_some(event.seq)
            })
            .collect();
        assert_eq!(inputs, vec![selected_input]);
    }
}
