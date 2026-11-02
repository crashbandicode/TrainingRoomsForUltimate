//! Core library tests: bit-exact float round trip, lifecycle and missingness
//! rules, archive truncation status, and clip-start context resolution.

use std::fs;
use std::path::PathBuf;

use smash_match_data::archive::{open_timeline, read_manifest};
use smash_match_data::clip::{ActorStatus, MissingReason};
use smash_match_data::validate::{competitive_conformance, Conformance};
use smash_match_data::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("smd-core-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn layout(id: &str, encoding: AxisEncoding) -> InputLayout {
    let axis = AxisLayout {
        encoding,
        min: None,
        max: None,
        neutral: None,
        float_scale: None,
        note: None,
    };
    InputLayout {
        id: id.into(),
        description: None,
        buttons: ButtonLayout {
            width_bits: 32,
            names: [(0u8, "a".to_string()), (1, "b".to_string())]
                .into_iter()
                .collect(),
        },
        left_stick: axis.clone(),
        right_stick: axis,
        raw_encoding: Some("test_raw".into()),
    }
}

fn live_header() -> CaptureHeader {
    CaptureHeader {
        capture_id: "test-capture".into(),
        created_utc: None,
        recorder: ToolVersion::new("core-tests", "0"),
        game: GameBuild::ultimate("13.0.5"),
        source: Source::live_match(),
        rules: Rules {
            target: RulesetTarget::Competitive,
            actual: ActualRules {
                mode: Some(MatchMode::Stock),
                stocks: Some(3),
                items: Some(ItemSetting::Off),
                stage_hazards: Some(false),
                ..Default::default()
            },
        },
        stage: Stage::default(),
        controllers: vec![ControllerInfo {
            slot: 0,
            kind: Some("pro".into()),
            human: Some(true),
            label: None,
        }],
        entries: vec![
            FighterEntry {
                control: ControlKind::Human,
                controller: Some(0),
                fighter_kind: Some(8),
                ..FighterEntry::new(0)
            },
            FighterEntry {
                control: ControlKind::Cpu { level: Some(9) },
                fighter_kind: Some(12),
                ..FighterEntry::new(1)
            },
        ],
        input_layouts: vec![layout("f32", AxisEncoding::F32)],
        channels: ChannelSupport::all_full(),
        clocks: ClockSupport {
            supplied: vec![ClockKind::Tick, ClockKind::InputSeq],
            relations: vec![],
        },
        notes: vec![],
    }
}

fn observed(x: f32, vx: f32) -> FighterSample {
    FighterSample {
        kinematics: Channel::Observed(Kinematics {
            position: Vec3::new(x, 0.0, 0.0),
            facing: 1.0,
            velocity: Some(Vec3::new(vx, 0.0, 0.0)),
        }),
        situation: Channel::Observed(Situation {
            situation_kind: 0,
            status_kind: 0,
            grounded: Some(true),
            hitstop_frames: None,
        }),
        animation: Channel::Observed(Animation {
            motion: MotionRef::labelled(0x1234, "wait1"),
            frame: 3.5,
            rate: 1.0,
            root_rotation: None,
            root_scale: None,
            secondary: None,
        }),
        vitals: Channel::Observed(Vitals {
            damage: 0.0,
            stocks: Some(3),
            shield: None,
        }),
    }
}

fn input(actor: ActorId, x: f32) -> InputEvent {
    InputEvent {
        controller: Some(0),
        entry: Some(0),
        actor: Some(actor),
        origin: InputOrigin::Physical,
        layout: "f32".into(),
        buttons: Channel::Observed(Buttons::new(1)),
        left_stick: Channel::Observed(Stick::F32 { x, y: -0.0 }),
        right_stick: Channel::Observed(Stick::F32 { x: 0.0, y: 0.0 }),
        raw: Some(RawBytes::new("test_raw", &[1, 2, 3])),
    }
}

fn identity(entry: u8, kind: i32) -> ActorIdentity {
    ActorIdentity {
        fighter_kind: Some(kind),
        entry: Some(entry),
        controller: Some(entry),
        control: ControlKind::Human,
        native_object_id: Some(0x1000_0000 + u32::from(entry)),
        ..Default::default()
    }
}

fn invalid_code(result: Result<u64, CaptureError>) -> IssueCode {
    match result {
        Err(CaptureError::Invalid(issue)) => issue.code,
        other => panic!("expected an invalid-event error, got {other:?}"),
    }
}

fn errors(issues: &[Issue]) -> Vec<&Issue> {
    issues.iter().filter(|i| i.is_error()).collect()
}

/// A short two-actor match recorded through the public producer API.
fn record_small_match(recorder: &mut Recorder) -> Result<(), CaptureError> {
    let a = ActorId(1);
    let b = ActorId(2);
    recorder.boundary(Clocks::tick(0), BoundaryKind::RecordingStart)?;
    recorder.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::MatchStart)?;
    recorder.spawn(Clocks::tick(0), b, identity(1, 12), SpawnCause::MatchStart)?;
    recorder.boundary(Clocks::tick(0), BoundaryKind::FirstPlayable)?;
    recorder.pace(Clocks::tick(0), Pace::normal())?;
    for tick in 1..=3u64 {
        let clocks = Clocks::tick(tick).with_input_seq(tick);
        for actor in [a, b] {
            recorder.state(clocks, actor, Phase::Before, observed(tick as f32, 0.5))?;
            recorder.input(clocks, input(actor, 0.1))?;
            recorder.state(clocks, actor, Phase::After, FighterSample::unchanged())?;
        }
    }
    recorder.death(Clocks::tick(3), a, Some("blast_zone_left".into()))?;
    recorder.gap(Clocks::tick(4), Some(b), Some("hook skipped".into()))?;
    recorder.respawn(Clocks::tick(5), a)?;
    recorder.object_spawn(
        Clocks::tick(5),
        ObjectId(1),
        ObjectIdentity {
            class: ObjectClass::Article,
            native_kind: Some(3),
            name: Some("turnip".into()),
            owner: Some(a),
            native_object_id: Some(0x2000_0001),
        },
        Channel::Observed(Transform {
            position: Vec3::new(1.0, 2.0, 0.0),
            rotation: None,
            scale: None,
            velocity: None,
        }),
        Channel::Missing,
    )?;
    recorder.random(
        Clocks::tick(5),
        RandomEvent {
            actor: Some(a),
            basis: OutcomeBasis::Observed,
            outcome: RandomOutcome::ItemPull {
                item_kind: Some(3),
                name: Some("turnip".into()),
                object: Some(ObjectId(1)),
            },
            note: None,
        },
    )?;
    recorder.object_despawn(Clocks::tick(6), ObjectId(1), None)?;
    recorder.boundary(Clocks::tick(6), BoundaryKind::MatchEnd)?;
    recorder.despawn(Clocks::tick(6), a, DespawnCause::MatchEnd)?;
    recorder.despawn(Clocks::tick(6), b, DespawnCause::MatchEnd)?;
    Ok(())
}

fn write_small_archive(dir: &PathBuf) -> Capture {
    let mut recorder = Recorder::create(
        dir,
        live_header(),
        RecorderOptions {
            keep_events: true,
            ..Default::default()
        },
    )
    .unwrap();
    record_small_match(&mut recorder).unwrap();
    recorder
        .finish(Clocks::tick(6), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap()
}

// ---------------------------------------------------------------------------
// Round trip
// ---------------------------------------------------------------------------

fn interesting_bits() -> Vec<u32> {
    let mut bits = vec![
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x8000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f80_0000,
        0xbf80_0000,
        0x3dcc_cccd,
        0x3eaa_aaab,
        0x3f35_04f3,
        0x3f7f_ffff,
        0x4b80_0001,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x3400_0000,
    ];
    let mut state = 0x9e37_79b9u32;
    for _ in 0..4000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        bits.push(state);
    }
    bits.retain(|b| f32::from_bits(*b).is_finite());
    bits
}

#[test]
fn f32_values_round_trip_bit_exactly_through_event_json() {
    let bits = interesting_bits();
    for chunk in bits.chunks_exact(4) {
        let [x, y, z, facing] = [chunk[0], chunk[1], chunk[2], chunk[3]].map(f32::from_bits);
        let event = Event {
            seq: 7,
            clocks: Clocks::tick(3).with_host_ns(u64::MAX),
            body: EventBody::State {
                actor: ActorId(1),
                phase: Phase::After,
                sample: FighterSample::unsupported().with_kinematics(Kinematics {
                    position: Vec3::new(x, y, z),
                    facing,
                    velocity: Some(Vec3::new(facing, z, y)),
                }),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        let EventBody::State { sample, .. } = &back.body else {
            panic!("kind changed");
        };
        let k = sample.kinematics.observed().unwrap();
        assert_eq!(
            [k.position.x, k.position.y, k.position.z, k.facing].map(f32::to_bits),
            [chunk[0], chunk[1], chunk[2], chunk[3]],
            "{json}"
        );
        let v = k.velocity.unwrap();
        assert_eq!(
            [v.x, v.y, v.z].map(f32::to_bits),
            [chunk[3], chunk[2], chunk[1]]
        );
        assert_eq!(back.clocks.host_ns, Some(u64::MAX));
        assert!(matches!(sample.situation, Channel::Unsupported));
    }
    for chunk in bits.chunks_exact(2) {
        let event = Event {
            seq: 0,
            clocks: Clocks::NONE,
            body: EventBody::Input(InputEvent {
                left_stick: Channel::Observed(Stick::F32 {
                    x: f32::from_bits(chunk[0]),
                    y: f32::from_bits(chunk[1]),
                }),
                ..input(ActorId(1), 0.0)
            }),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        let EventBody::Input(i) = back.body else {
            panic!("kind changed")
        };
        let Some(Stick::F32 { x, y }) = i.left_stick.observed().copied() else {
            panic!("stick lost")
        };
        assert_eq!([x.to_bits(), y.to_bits()], [chunk[0], chunk[1]], "{json}");
        assert_eq!(i.raw.unwrap().bytes().unwrap(), vec![1, 2, 3]);
    }
}

#[test]
fn signed_zero_and_markers_are_distinct_in_json() {
    let json = serde_json::to_string(&Vec3::new(-0.0, 0.0, 1.0)).unwrap();
    assert_eq!(json, r#"{"x":-0.0,"y":0.0,"z":1.0}"#);
    let sample: FighterSample = serde_json::from_str(
        r#"{"kinematics":"missing","situation":"unchanged","animation":"unsupported",
            "vitals":{"damage":12.5,"stocks":2}}"#,
    )
    .unwrap();
    assert!(matches!(sample.kinematics, Channel::Missing));
    assert!(matches!(sample.situation, Channel::Unchanged));
    assert!(matches!(sample.animation, Channel::Unsupported));
    assert_eq!(sample.vitals.observed().unwrap().stocks, Some(2));
    let null = serde_json::from_str::<FighterSample>(
        r#"{"kinematics":null,"situation":"unchanged","animation":"unsupported","vitals":"missing"}"#,
    );
    assert!(
        null.is_err(),
        "null must not be accepted as a channel value"
    );
    // NaN cannot be written as a number; the recorder rejects it before that.
    let mut recorder = Recorder::in_memory(live_header()).unwrap();
    recorder
        .spawn(
            Clocks::tick(0),
            ActorId(1),
            identity(0, 8),
            SpawnCause::MatchStart,
        )
        .unwrap();
    let nan = FighterSample::unsupported().with_kinematics(Kinematics {
        position: Vec3::new(0.0, f32::NAN, 0.0),
        facing: 1.0,
        velocity: None,
    });
    assert_eq!(
        invalid_code(recorder.state(Clocks::tick(0), ActorId(1), Phase::Before, nan)),
        IssueCode::NonFinite
    );
}

// ---------------------------------------------------------------------------
// Archive
// ---------------------------------------------------------------------------

#[test]
fn recorder_archive_round_trips_and_reports_complete() {
    let dir = temp_dir("complete");
    let recorded = write_small_archive(&dir);
    let loaded = Capture::load(&dir).unwrap();

    assert_eq!(loaded.completeness, Completeness::Complete);
    assert_eq!(loaded.events, recorded.events);
    assert_eq!(loaded.manifest, recorded.manifest);
    assert!(
        matches!(loaded.events.last().unwrap().body, EventBody::End { events } if events + 1 == loaded.events.len() as u64)
    );
    assert_eq!(loaded.manifest.completion.state, CompletionState::Complete);
    assert_eq!(
        loaded.manifest.completion.events,
        Some(loaded.events.len() as u64)
    );
    assert_eq!(loaded.manifest.boundaries.len(), 3);
    assert_eq!(loaded.manifest.actors.len(), 2);
    let a = &loaded.manifest.actors[0];
    assert_eq!((a.actor, a.deaths, a.respawns), (ActorId(1), 1, 1));
    assert!(a.despawn_seq.is_some());
    let issues = loaded.validate();
    assert!(issues.is_empty(), "{issues:?}");

    // Streaming read gives the same events and the same verdict.
    let manifest = read_manifest(&dir).unwrap();
    let mut reader = open_timeline(&dir, &manifest).unwrap();
    assert!(reader.completeness().is_none());
    let mut count = 0;
    while let Some(event) = reader.next_event().unwrap() {
        assert_eq!(event, loaded.events[count]);
        count += 1;
    }
    assert_eq!(count, loaded.events.len());
    assert_eq!(reader.completeness(), Some(&Completeness::Complete));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn truncated_timeline_is_reported_not_accepted() {
    let dir = temp_dir("truncated");
    let full = write_small_archive(&dir);
    let timeline = dir.join(TIMELINE_FILE);
    let bytes = fs::read(&timeline).unwrap();

    // Cut in the middle of the last line.
    fs::write(&timeline, &bytes[..bytes.len() - 20]).unwrap();
    let cut = Capture::load(&dir).unwrap();
    let Completeness::Incomplete { problems } = &cut.completeness else {
        panic!("partial file accepted as {:?}", cut.completeness);
    };
    assert!(
        problems.contains(&TruncationProblem::MissingEnd),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| matches!(p, TruncationProblem::PartialTrailingLine { .. })),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| matches!(p, TruncationProblem::ManifestCountMismatch { .. })),
        "{problems:?}"
    );
    assert_eq!(cut.events.len(), full.events.len() - 1);
    assert_eq!(cut.events, full.events[..cut.events.len()]);
    let issues = cut.validate();
    assert!(errors(&issues).is_empty(), "{issues:?}");
    assert!(issues
        .iter()
        .any(|i| i.code == IssueCode::Completeness && i.severity == Severity::Warning));

    // Cut exactly at a line boundary: no partial line, still not complete.
    let last_newline = bytes[..bytes.len() - 1]
        .iter()
        .rposition(|b| *b == b'\n')
        .unwrap();
    fs::write(&timeline, &bytes[..=last_newline]).unwrap();
    let cut = Capture::load(&dir).unwrap();
    let Completeness::Incomplete { problems } = &cut.completeness else {
        panic!("cut file accepted as {:?}", cut.completeness);
    };
    assert!(problems.contains(&TruncationProblem::MissingEnd));
    assert!(!problems
        .iter()
        .any(|p| matches!(p, TruncationProblem::PartialTrailingLine { .. })));

    // Re-saving an incomplete capture produces a consistent, declared-truncated one.
    let resaved = temp_dir("resaved");
    cut.save(&resaved, &WriteOptions::default()).unwrap();
    let again = Capture::load(&resaved).unwrap();
    let Completeness::DeclaredTruncated {
        reason: Some(reason),
    } = &again.completeness
    else {
        panic!("{:?}", again.completeness);
    };
    assert!(reason.contains("re-saved"), "{reason}");
    assert_eq!(again.events.len(), cut.events.len() + 1);
    assert!(errors(&again.validate()).is_empty());
    fs::remove_dir_all(&dir).unwrap();
    fs::remove_dir_all(&resaved).unwrap();
}

#[test]
fn unfinished_recorder_leaves_manifest_open_and_is_reported() {
    let dir = temp_dir("open");
    {
        let mut recorder =
            Recorder::create(&dir, live_header(), RecorderOptions::default()).unwrap();
        record_small_match(&mut recorder).unwrap();
        // Dropped without finish: the process died.
    }
    let capture = Capture::load(&dir).unwrap();
    let Completeness::Incomplete { problems } = &capture.completeness else {
        panic!("{:?}", capture.completeness);
    };
    assert_eq!(problems[0], TruncationProblem::ManifestOpen);
    assert!(problems.contains(&TruncationProblem::MissingEnd));
    assert_eq!(capture.manifest.completion.state, CompletionState::Open);
    assert!(
        capture.events.len() > 20,
        "events were flushed as they were written"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn declared_truncation_is_distinct_from_damage() {
    let dir = temp_dir("aborted");
    let mut recorder = Recorder::create(&dir, live_header(), RecorderOptions::default()).unwrap();
    record_small_match(&mut recorder).unwrap();
    let finished = recorder.abort(Clocks::tick(6), "hook detached").unwrap();
    assert!(finished.events.is_none());
    assert_eq!(
        finished.completeness,
        Completeness::DeclaredTruncated {
            reason: Some("hook detached".into())
        }
    );
    let capture = Capture::load(&dir).unwrap();
    assert_eq!(capture.completeness, finished.completeness);
    assert_eq!(capture.manifest, finished.manifest);
    let issues = capture.validate();
    assert!(errors(&issues).is_empty(), "{issues:?}");
    assert!(
        issues.iter().all(|i| i.severity == Severity::Info),
        "{issues:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn in_memory_capture_saves_and_reloads() {
    let mut recorder = Recorder::in_memory(live_header()).unwrap();
    record_small_match(&mut recorder).unwrap();
    let capture = recorder
        .finish(Clocks::tick(6), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap();
    let dir = temp_dir("saved");
    let manifest = capture.save(&dir, &WriteOptions::default()).unwrap();
    assert_eq!(manifest.completion.state, CompletionState::Complete);
    let loaded = Capture::load(&dir).unwrap();
    assert_eq!(loaded.completeness, Completeness::Complete);
    assert_eq!(loaded.events, capture.events);
    assert_eq!(loaded.manifest.actors, capture.manifest.actors);
    assert!(matches!(
        capture.save(&dir, &WriteOptions::default()),
        Err(ArchiveError::AlreadyExists { .. })
    ));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn format_name_and_major_version_are_gated() {
    let dir = temp_dir("version");
    write_small_archive(&dir);
    let manifest_path = dir.join(MANIFEST_FILE);
    let original = fs::read_to_string(&manifest_path).unwrap();
    assert!(original.contains("\"major\": 1"));
    fs::write(
        &manifest_path,
        original.replace("\"major\": 1", "\"major\": 2"),
    )
    .unwrap();
    assert!(matches!(
        Capture::load(&dir),
        Err(ArchiveError::Version { .. })
    ));
    fs::write(
        &manifest_path,
        original.replace(FORMAT_NAME, "other-format"),
    )
    .unwrap();
    assert!(matches!(
        Capture::load(&dir),
        Err(ArchiveError::Format { .. })
    ));
    fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------
// Lifecycle and declaration rules
// ---------------------------------------------------------------------------

#[test]
fn lifecycle_and_declaration_rules_are_enforced() {
    let a = ActorId(1);
    let mut rec = Recorder::in_memory(live_header()).unwrap();
    assert_eq!(
        invalid_code(rec.state(Clocks::tick(0), a, Phase::Before, observed(0.0, 0.0))),
        IssueCode::UnknownActor
    );
    assert_eq!(rec.next_seq(), 0, "rejected events do not consume seq");
    rec.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::MatchStart)
        .unwrap();
    assert_eq!(
        invalid_code(rec.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::Unknown)),
        IssueCode::ActorReused
    );
    assert_eq!(
        invalid_code(rec.respawn(Clocks::tick(0), a)),
        IssueCode::LifecycleOrder
    );
    rec.death(Clocks::tick(1), a, None).unwrap();
    assert_eq!(
        invalid_code(rec.death(Clocks::tick(1), a, None)),
        IssueCode::LifecycleOrder
    );
    rec.state(Clocks::tick(1), a, Phase::After, FighterSample::missing())
        .unwrap();
    rec.respawn(Clocks::tick(2), a).unwrap();
    assert_eq!(
        invalid_code(rec.input(
            Clocks::tick(2),
            InputEvent {
                layout: "nope".into(),
                ..input(a, 0.0)
            }
        )),
        IssueCode::UnknownInputLayout
    );
    assert_eq!(
        invalid_code(rec.input(
            Clocks::tick(2),
            InputEvent {
                raw: Some(RawBytes {
                    encoding: "test_raw".into(),
                    hex: "abc".into()
                }),
                ..input(a, 0.0)
            }
        )),
        IssueCode::InvalidRawBytes
    );
    rec.input(Clocks::tick(5), input(a, 0.0)).unwrap();
    assert_eq!(
        invalid_code(rec.input(Clocks::tick(4), input(a, 0.0))),
        IssueCode::ClockRegression
    );
    // Stick encoding and raw encoding must match the declared layout.
    assert_eq!(
        invalid_code(rec.input(
            Clocks::tick(5),
            InputEvent {
                left_stick: Channel::Observed(Stick::I8 { x: 0, y: 0 }),
                ..input(a, 0.0)
            }
        )),
        IssueCode::LayoutMismatch
    );
    assert_eq!(
        invalid_code(rec.input(
            Clocks::tick(5),
            InputEvent {
                raw: Some(RawBytes::new("other_encoding", &[1])),
                ..input(a, 0.0)
            }
        )),
        IssueCode::LayoutMismatch
    );
    // Within one tick an actor's `before` sample cannot follow its `after` sample.
    rec.state(Clocks::tick(5), a, Phase::After, observed(5.0, 0.0))
        .unwrap();
    assert_eq!(
        invalid_code(rec.state(Clocks::tick(5), a, Phase::Before, observed(5.0, 0.0))),
        IssueCode::PhaseOrder
    );
    rec.boundary(Clocks::tick(5), BoundaryKind::MatchEnd)
        .unwrap();
    assert_eq!(
        invalid_code(rec.boundary(Clocks::tick(5), BoundaryKind::FirstPlayable)),
        IssueCode::BoundaryOrder
    );
    rec.despawn(Clocks::tick(6), a, DespawnCause::MatchEnd)
        .unwrap();
    assert_eq!(
        invalid_code(rec.state(Clocks::tick(6), a, Phase::Before, observed(0.0, 0.0))),
        IssueCode::LifecycleOrder
    );
    assert_eq!(
        invalid_code(rec.gap(Clocks::tick(6), Some(ActorId(9)), None)),
        IssueCode::UnknownActor
    );
    assert_eq!(
        rec.next_seq(),
        8,
        "only the eight accepted events consumed seq numbers"
    );
    assert!(rec.warnings().is_empty(), "{:?}", rec.warnings());

    // Declared-unsupported channels reject observations.
    let mut header = live_header();
    header.channels.random = Supply::unsupported();
    header.channels.animation = Supply::unsupported();
    let mut rec = Recorder::in_memory(header).unwrap();
    rec.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::MatchStart)
        .unwrap();
    assert_eq!(
        invalid_code(rec.random(
            Clocks::tick(0),
            RandomEvent {
                actor: Some(a),
                basis: OutcomeBasis::Observed,
                outcome: RandomOutcome::Judge { number: 9 },
                note: None,
            }
        )),
        IssueCode::UnsupportedChannelObserved
    );
    assert_eq!(
        invalid_code(rec.state(Clocks::tick(0), a, Phase::Before, observed(0.0, 0.0))),
        IssueCode::UnsupportedChannelObserved
    );
    let mut honest = observed(0.0, 0.0);
    honest.animation = Channel::Unsupported;
    rec.state(Clocks::tick(0), a, Phase::Before, honest)
        .unwrap();
    assert!(matches!(
        rec.finish(Clocks::tick(0), Completion::open()),
        Err(CaptureError::OpenCompletion)
    ));

    // Header rules.
    let mut header = live_header();
    header.input_layouts.push(layout("f32", AxisEncoding::F32));
    assert!(matches!(
        Recorder::in_memory(header),
        Err(CaptureError::Invalid(Issue {
            code: IssueCode::DuplicateInputLayout,
            ..
        }))
    ));
    let mut header = live_header();
    header.capture_id = "  ".into();
    assert!(matches!(
        Recorder::in_memory(header),
        Err(CaptureError::Invalid(Issue {
            code: IssueCode::EmptyCaptureId,
            ..
        }))
    ));
}

#[test]
fn competitive_target_is_separate_from_actual_rules() {
    assert_eq!(
        competitive_conformance(&ActualRules::default()),
        Conformance::Undetermined {
            unknown: vec!["items".into(), "stage_hazards".into()]
        }
    );
    assert_eq!(
        competitive_conformance(&live_header().rules.actual),
        Conformance::Conforms
    );
    let on = ActualRules {
        items: Some(ItemSetting::On),
        stage_hazards: Some(false),
        spirits: Some(true),
        ..Default::default()
    };
    assert_eq!(
        competitive_conformance(&on),
        Conformance::Deviates {
            reasons: vec!["match items are on".into()]
        }
    );
    let mut header = live_header();
    header.rules.actual = on;
    let mut rec = Recorder::in_memory(header).unwrap();
    rec.boundary(Clocks::tick(0), BoundaryKind::RecordingStart)
        .unwrap();
    let capture = rec
        .finish(Clocks::tick(0), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap();
    let issues = capture.validate();
    assert!(issues
        .iter()
        .any(|i| i.code == IssueCode::RulesTarget && i.severity == Severity::Warning));
    assert_eq!(capture.header().rules.target, RulesetTarget::Competitive);
}

// ---------------------------------------------------------------------------
// Clip context
// ---------------------------------------------------------------------------

struct Marks {
    before: Vec<u64>,
    after: Vec<u64>,
    death: u64,
    gap: u64,
}

fn clip_capture() -> (Capture, Marks) {
    let a = ActorId(1);
    let mut rec = Recorder::in_memory(live_header()).unwrap();
    let mut marks = Marks {
        before: vec![0; 10],
        after: vec![0; 10],
        death: 0,
        gap: 0,
    };
    rec.boundary(Clocks::tick(0), BoundaryKind::RecordingStart)
        .unwrap();
    rec.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::MatchStart)
        .unwrap();
    rec.boundary(Clocks::tick(0), BoundaryKind::FirstPlayable)
        .unwrap();
    let mut tick =
        |rec: &mut Recorder, t: u64, before: FighterSample, after: Option<FighterSample>| {
            let clocks = Clocks::tick(t).with_input_seq(t);
            marks.before[t as usize] = rec.state(clocks, a, Phase::Before, before).unwrap();
            rec.input(clocks, input(a, t as f32)).unwrap();
            if let Some(after) = after {
                marks.after[t as usize] = rec.state(clocks, a, Phase::After, after).unwrap();
            }
        };
    tick(
        &mut rec,
        1,
        observed(1.0, 0.0),
        Some(FighterSample::unchanged()),
    );
    tick(
        &mut rec,
        2,
        FighterSample::unchanged(),
        Some(observed(2.0, 1.0)),
    );
    tick(
        &mut rec,
        3,
        FighterSample::missing(),
        Some(FighterSample::unchanged()),
    );
    tick(
        &mut rec,
        4,
        observed(4.0, 2.0),
        Some(FighterSample::unchanged()),
    );
    marks.death = rec.death(Clocks::tick(4), a, None).unwrap();
    tick(&mut rec, 5, FighterSample::unchanged(), None);
    rec.respawn(Clocks::tick(6), a).unwrap();
    tick(
        &mut rec,
        7,
        observed(0.0, 0.0),
        Some(FighterSample::unchanged()),
    );
    marks.gap = rec
        .gap(Clocks::tick(8), Some(a), Some("dropped".into()))
        .unwrap();
    tick(&mut rec, 9, FighterSample::unchanged(), None);
    let capture = rec
        .finish(Clocks::tick(9), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap();
    (capture, marks)
}

fn kinematics_at(capture: &Capture, at: SelectAt) -> Resolved<Kinematics> {
    capture
        .clip_context(at)
        .unwrap_or_else(|| panic!("{at:?} not found"))
        .actor(ActorId(1))
        .expect("actor resolved")
        .sample
        .kinematics
        .clone()
}

fn observed_x(resolved: &Resolved<Kinematics>) -> (f32, u64, u64) {
    match resolved {
        Resolved::Observed {
            value,
            value_seq,
            latest_seq,
            ..
        } => (value.position.x, *value_seq, *latest_seq),
        other => panic!("expected an observed value, got {other:?}"),
    }
}

#[test]
fn clip_start_uses_before_state_and_same_segment_predecessors_only() {
    let (capture, m) = clip_capture();
    let a = ActorId(1);

    // Tick selection resolves from the `before` sample, not the previous tick's `after`.
    let r = kinematics_at(&capture, SelectAt::Tick(1));
    assert_eq!(observed_x(&r), (1.0, m.before[1], m.before[1]));
    assert!(!r.is_carried());
    let ctx = capture.clip_context(SelectAt::Tick(1)).unwrap();
    assert_eq!(ctx.seq, Some(m.before[1]));
    assert_eq!(
        ctx.boundaries,
        vec![BoundaryKind::RecordingStart, BoundaryKind::FirstPlayable]
    );
    assert_eq!(ctx.actor(a).unwrap().status, ActorStatus::Alive);
    assert_eq!(ctx.actor(a).unwrap().segment.boundary, LifecycleKind::Spawn);

    // `unchanged` walks back through `unchanged` to the real observation.
    let r = kinematics_at(&capture, SelectAt::Tick(2));
    assert_eq!(observed_x(&r), (1.0, m.before[1], m.before[2]));
    assert!(r.is_carried());
    assert_eq!(
        observed_x(&kinematics_at(&capture, SelectAt::Seq(m.after[2]))),
        (2.0, m.after[2], m.after[2])
    );

    // A missing sample is missing; `unchanged` after it has no valid predecessor.
    assert_eq!(
        kinematics_at(&capture, SelectAt::Tick(3)),
        Resolved::Missing(MissingReason::SampleMissing { seq: m.before[3] })
    );
    assert_eq!(
        kinematics_at(&capture, SelectAt::Seq(m.after[3])),
        Resolved::Missing(MissingReason::UnchangedWithoutPredecessor { seq: m.after[3] })
    );
    assert_eq!(
        observed_x(&kinematics_at(&capture, SelectAt::Tick(4))).0,
        4.0
    );

    // Death starts a new segment: nothing carries across it.
    let ctx = capture.clip_context(SelectAt::Seq(m.death)).unwrap();
    let dead = ctx.actor(a).unwrap();
    assert_eq!(dead.status, ActorStatus::Dead);
    assert_eq!(
        dead.segment,
        clip::Segment {
            start_seq: m.death,
            boundary: LifecycleKind::Death
        }
    );
    assert_eq!(dead.sample.kinematics, Resolved::NoSample);
    assert_eq!(dead.sample.vitals, Resolved::NoSample);
    assert_eq!(
        kinematics_at(&capture, SelectAt::Tick(5)),
        Resolved::Missing(MissingReason::UnchangedWithoutPredecessor { seq: m.before[5] })
    );

    // Respawn: alive again, fresh observation.
    let ctx = capture.clip_context(SelectAt::Tick(7)).unwrap();
    assert_eq!(ctx.actor(a).unwrap().status, ActorStatus::Alive);
    assert_eq!(
        ctx.actor(a).unwrap().segment.boundary,
        LifecycleKind::Respawn
    );
    assert_eq!(
        observed_x(&kinematics_at(&capture, SelectAt::Tick(7))).0,
        0.0
    );

    // Gaps break both directions.
    assert_eq!(
        kinematics_at(&capture, SelectAt::Seq(m.gap)),
        Resolved::Missing(MissingReason::GapAfterLatestSample { gap_seq: m.gap })
    );
    assert_eq!(
        kinematics_at(&capture, SelectAt::Tick(9)),
        Resolved::Missing(MissingReason::GapBeforePredecessor { gap_seq: m.gap })
    );

    // Tick without input includes the whole tick; unknown tick is not found.
    let ctx = capture.clip_context(SelectAt::Tick(0)).unwrap();
    assert_eq!(ctx.actor(a).unwrap().sample.kinematics, Resolved::NoSample);
    assert_eq!(ctx.included, 3);
    assert!(capture.clip_context(SelectAt::Tick(42)).is_none());
    assert!(capture.clip_context(SelectAt::Record(0)).is_none());
    // A reversed selection is invalid, never a panic or a wrapped range.
    assert!(capture
        .clip_events(SelectAt::Tick(4), SelectAt::Tick(2))
        .is_none());
    assert!(capture
        .clip_events(SelectAt::Tick(2), SelectAt::Tick(42))
        .is_none());

    // Clip events start at the first input of the start tick and end with the
    // `before` samples of the end tick.
    let clip = capture
        .clip_events(SelectAt::Tick(2), SelectAt::Tick(4))
        .unwrap();
    assert!(matches!(clip.first().unwrap().body, EventBody::Input(_)));
    assert_eq!(clip.first().unwrap().clocks.tick, Some(2));
    assert_eq!(clip.last().unwrap().seq, m.before[4]);
}

#[test]
fn objects_pace_and_unsupported_channels_in_context() {
    let a = ActorId(1);
    let mut header = live_header();
    header.channels.animation = Supply::unsupported();
    let mut rec = Recorder::in_memory(header).unwrap();
    rec.spawn(Clocks::tick(0), a, identity(0, 8), SpawnCause::MatchStart)
        .unwrap();
    let transform = Transform {
        position: Vec3::new(5.0, 1.0, 0.0),
        rotation: Some(Rotation::Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }),
        scale: None,
        velocity: Some(Vec3::new(0.0, -0.5, 0.0)),
    };
    let spawn = rec
        .object_spawn(
            Clocks::tick(2),
            ObjectId(7),
            ObjectIdentity {
                class: ObjectClass::Projectile,
                owner: Some(a),
                ..Default::default()
            },
            Channel::Observed(transform),
            Channel::Unsupported,
        )
        .unwrap();
    let state = rec
        .object_state(
            Clocks::tick(3),
            ObjectId(7),
            Channel::Unchanged,
            Channel::Unsupported,
        )
        .unwrap();
    let paused = rec.pace(Clocks::tick(3), Pace::paused()).unwrap();
    let mut honest = observed(1.0, 0.0);
    honest.animation = Channel::Unsupported;
    let sample_seq = rec.state(Clocks::tick(3), a, Phase::After, honest).unwrap();
    rec.object_despawn(Clocks::tick(4), ObjectId(7), Some("hit".into()))
        .unwrap();
    rec.pace(Clocks::tick(4), Pace::normal()).unwrap();
    let capture = rec
        .finish(Clocks::tick(4), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap();

    let ctx = capture.clip_context(SelectAt::Seq(sample_seq)).unwrap();
    assert_eq!(
        ctx.pace.as_ref().map(|(p, seq)| (p.paused, *seq)),
        Some((true, paused))
    );
    let object = ctx.object(ObjectId(7)).expect("object live at tick 3");
    assert_eq!(object.identity.owner, Some(a));
    assert_eq!(object.spawn_seq, spawn);
    match &object.transform {
        Resolved::Observed {
            value,
            value_seq,
            latest_seq,
            ..
        } => {
            assert_eq!(value.position.x, 5.0);
            assert_eq!((*value_seq, *latest_seq), (spawn, state));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(object.animation, Resolved::Unsupported);
    assert_eq!(
        ctx.actor(a).unwrap().sample.animation,
        Resolved::Unsupported
    );
    assert_eq!(observed_x(&ctx.actor(a).unwrap().sample.kinematics).0, 1.0);

    let later = capture.clip_context(SelectAt::Tick(4)).unwrap();
    assert!(later.objects.is_empty(), "despawned objects are not live");
    assert_eq!(later.pace.as_ref().map(|(p, _)| p.paused), Some(false));
    assert!(capture.validate().iter().all(|i| !i.is_error()));
}

#[test]
fn decoded_replay_records_carry_only_the_record_clock() {
    let replay_bytes = b"not a real replay file";
    let header = CaptureHeader {
        capture_id: "replay-import-test".into(),
        recorder: ToolVersion::this_crate(),
        game: GameBuild::ultimate("13.0.5"),
        source: Source::file_decode(
            ReplayAssociation::from_bytes(replay_bytes, Some("match.bin".into())),
            ToolVersion::new("smash-replay-core", "0.1.0"),
        ),
        input_layouts: vec![InputLayout {
            raw_encoding: Some("fram_packet_13_0_5".into()),
            ..layout("fram_i8", AxisEncoding::I8)
        }],
        channels: ChannelSupport {
            inputs: Supply::partial("sparse packets; absent packets are missing"),
            ..Default::default()
        },
        clocks: ClockSupport {
            supplied: vec![ClockKind::Record],
            relations: vec![],
        },
        ..Default::default()
    };
    assert_eq!(
        header.source.replay.as_ref().unwrap().sha256,
        sha256_hex(replay_bytes)
    );

    let mut rec = Recorder::in_memory(header).unwrap();
    for record in 0..3u64 {
        for slot in 0..2u8 {
            let packet = [0x01, 0x80, 0x00, 0x00, 0x7f, 0x81, 0x00, 0xff];
            rec.input(
                Clocks::record(record),
                InputEvent {
                    controller: Some(slot),
                    entry: None,
                    actor: None,
                    origin: InputOrigin::ReplayDecoded,
                    layout: "fram_i8".into(),
                    buttons: Channel::Observed(Buttons::new(0x8001)),
                    left_stick: Channel::Observed(Stick::I8 { x: 127, y: -127 }),
                    right_stick: if slot == 0 {
                        Channel::Observed(Stick::I8 { x: 0, y: -1 })
                    } else {
                        Channel::Missing
                    },
                    raw: Some(RawBytes::new("fram_packet_13_0_5", &packet)),
                },
            )
            .unwrap();
        }
    }
    let capture = rec
        .finish(Clocks::record(2), Completion::complete())
        .unwrap()
        .into_capture()
        .unwrap();
    assert!(capture.events.iter().all(|e| e.clocks.tick.is_none()));
    assert!(capture.clip_context(SelectAt::Tick(0)).is_none());
    let ctx = capture.clip_context(SelectAt::Record(1)).unwrap();
    assert_eq!(
        ctx.clocks.record,
        Some(0),
        "record 1 starts with an input, so the context ends with record 0"
    );
    assert!(ctx.actors.is_empty());
    let issues = capture.validate();
    assert!(errors(&issues).is_empty(), "{issues:?}");
    assert!(issues
        .iter()
        .any(|i| i.code == IssueCode::RulesTarget && i.severity == Severity::Info));

    let dir = temp_dir("replay-like");
    capture.save(&dir, &WriteOptions::default()).unwrap();
    let loaded = Capture::load(&dir).unwrap();
    assert_eq!(loaded.events, capture.events);
    let EventBody::Input(first) = &loaded.events[0].body else {
        panic!()
    };
    assert_eq!(
        first.left_stick.observed(),
        Some(&Stick::I8 { x: 127, y: -127 })
    );
    assert_eq!(first.raw.as_ref().unwrap().bytes().unwrap()[1], 0x80);
    fs::remove_dir_all(&dir).unwrap();
}
