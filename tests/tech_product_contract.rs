#[allow(dead_code)]
#[path = "../src/model/tech_launch.rs"]
mod launch;
use launch::{Attempt, AttemptState, ContactReceipt, HitHeight, Launch, Surface, Vec2};
#[path = "common/mod.rs"]
mod common;
#[path = "../src/runtime/features/tech_decision_guards.rs"]
mod decision_guards;
fn request() -> Launch {
    Launch {
        start: Vec2 { x: -30.0, y: 20.0 },
        target: Vec2 { x: 0.0, y: 0.0 },
        angle: 315.0,
        knockback: 100.0,
        percent: 60.0,
        hit_damage: 12.0,
        height: HitHeight::Middle,
        facing: 1.0,
        hit_from_behind: false,
        grounded_hit: false,
        di: Some(Vec2::default()),
    }
}
#[test]
fn fighter_control_reader_accepts_the_native_constructed_address_point() {
    let Some(main) = common::owned_main("fighter ControlModule address point") else {
        return;
    };
    let bytes = main.window(0x6b8598);
    let word = |n: usize| u32::from_le_bytes(bytes[n..n + 4].try_into().unwrap());
    // Native Fighter Control constructor materializes its vptr and stores it to this.
    assert_eq!(word(0), 0xb0024669); // ADRP x9
    assert_eq!(word(4), 0x91378129); // ADD x9,x9,#de0
    assert_eq!(word(20), 0xf9000009); // STR x9,[x0]
    let instruction = word(0);
    let pages = (((instruction >> 5) & 0x7ffff) << 2) | ((instruction >> 29) & 3);
    let pages = ((pages << 11) as i32 >> 11) as isize;
    let constructed = (0x6b8000isize + (pages << 12)) as usize
        + ((word(4) >> 10) & 0xfff) as usize;
    assert_eq!(
        decision_guards::CONTROL_VTABLE_OFFSET,
        constructed,
        "The guarded reader must accept the Fighter instance, whose native slots are checked separately"
    );
    let u64_at = |at: usize| u64::from_le_bytes(main.window(at)[..8].try_into().unwrap()) as usize;
    let module = u32::from_le_bytes(main.window(0)[4..8].try_into().unwrap()) as usize;
    assert_eq!(&main.window(module)[..4], b"MOD0");
    let delta = i32::from_le_bytes(main.window(module)[4..8].try_into().unwrap());
    let mut entry = module.wrapping_add_signed(delta as isize);
    let mut dynamic = std::collections::BTreeMap::new();
    while u64_at(entry) != 0 {
        dynamic.insert(u64_at(entry), u64_at(entry + 8));
        entry += 16;
    }
    let mut targets = std::collections::BTreeMap::new();
    for at in (dynamic[&7]..dynamic[&7] + dynamic[&8]).step_by(24) {
        if u64_at(at + 8) & 0xffff_ffff == 1027 && (constructed..constructed + 0x258).contains(&u64_at(at)) {
            targets.insert(u64_at(at), u64_at(at + 16));
        }
    }
    for (slot, target) in [(0x178, 0x3f84a0), (0x220, 0x3f8610), (0x228, 0x3f8620), (0x230, 0x3f8630), (0x250, 0x3f8670)] {
        assert_eq!(targets.get(&(constructed + slot)), Some(&target));
    }
}

#[test]
fn native_press_counter_zeroes_only_on_the_native_positive_edge() {
    let Some(main) = common::owned_main("native tech press counter update") else { return; };
    let bytes = main.window(0x3f9544);
    let words: Vec<u32> = bytes.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
    assert_eq!(&words[..11], &[
        0xb868696c, 0x3940014d, 0x6a09019f, 0x54000080,
        0x2a1f03ec, 0x381ff14d, 0x14000004, 0x7103f5bf,
        0x54000068, 0x110005ac, 0x3900014c,
    ]); // Edge copies old count to previous and writes zero; only the other branch increments.
}
#[test]
fn product_request_rejects_nonfinite_fields_before_native_mutation() {
    let good = request();
    assert!(good.valid());
    for bad in [
        Launch {
            start: Vec2 {
                x: f32::NAN,
                y: 0.0,
            },
            ..good
        },
        Launch {
            target: Vec2 {
                x: 0.0,
                y: f32::INFINITY,
            },
            ..good
        },
        Launch {
            angle: f32::NAN,
            ..good
        },
        Launch {
            knockback: 0.0,
            ..good
        },
        Launch {
            percent: -1.0,
            ..good
        },
        Launch {
            hit_damage: f32::NAN,
            ..good
        },
        Launch {
            facing: 0.0,
            ..good
        },
        Launch {
            di: Some(Vec2 {
                x: f32::INFINITY,
                y: 0.0,
            }),
            ..good
        },
    ] {
        assert!(!bad.valid(), "{bad:?}");
    }
}
#[test]
fn hit_side_is_independent_of_world_angle() {
    for angle in [38.0, 142.0, 218.0, 322.0] {
        for (facing, behind, reserve, away_right) in [
            (1.0, false, 1.0, false),
            (1.0, true, -1.0, true),
            (-1.0, false, -1.0, true),
            (-1.0, true, 1.0, false),
        ] {
            let r = Launch {
                angle,
                facing,
                hit_from_behind: behind,
                ..request()
            };
            assert!(r.valid());
            assert_eq!(r.reserve_damage_lr(), reserve);
            assert_eq!(r.away_from_hit_toward_right(), away_right);
            assert_eq!(r.angle.to_bits(), angle.to_bits());
        }
    }
}
#[test]
fn neutral_di_and_player_di_are_distinct_requests() {
    let neutral = request();
    let player = Launch {
        di: None,
        ..neutral
    };
    assert!(neutral.valid() && player.valid());
    assert_ne!(neutral.di, player.di);
    assert!(!Launch {
        di: Some(Vec2 { x: 1.01, y: 0.0 }),
        ..neutral
    }
    .valid());
    assert_eq!(neutral.percent, 60.0);
    assert_eq!(neutral.hit_damage, 12.0);
}

#[test]
fn airborne_start_never_requests_ground_to_air() {
    for angle in [38.0, 142.0, 218.0, 322.0] {
        for facing in [-1.0, 1.0] {
            let air = Launch {
                angle,
                facing,
                grounded_hit: false,
                ..request()
            };
            assert!(!air.ground_to_air());
            assert!(Launch {
                grounded_hit: true,
                ..air
            }
            .ground_to_air());
        }
    }
    let source = include_str!("../src/runtime/features/tech_adapter.rs");
    assert!(source.contains("ground_to_air: owned.launch.ground_to_air()"));
    assert!(!source.contains("ground_to_air: owned.saved.grounded"));
}

fn contact(passive_enabled: bool) -> ContactReceipt {
    ContactReceipt {
        surface: Surface::Wall,
        requested_status: 42,
        passive_enabled,
        position: Vec2 { x: 20.0, y: 30.0 },
    }
}

#[test]
fn attempt_observations_do_not_invent_contact() {
    let mut attempt = Attempt::held(1);
    attempt.observe_stop(false);
    attempt.record_contact(contact(true));
    assert_eq!(attempt.state, AttemptState::Held);
    assert_eq!(attempt.contact, None);
    attempt.launched();
    assert_eq!(attempt.state, AttemptState::Stopped);
    attempt.observe_stop(true);
    assert_eq!(attempt.state, AttemptState::Stopped);
    attempt.observe_stop(false);
    assert_eq!(attempt.state, AttemptState::Flying);
    assert_eq!(attempt.contact, None);
    attempt.mark_abandoned();
    attempt.record_contact(contact(true));
    assert_eq!(attempt.state, AttemptState::Abandoned);
    assert_eq!(attempt.contact, None);
    assert_eq!(Attempt::held(2).contact, None);
}

#[test]
fn contact_receipt_is_first_native_decision() {
    for stopped in [true, false] {
        let mut attempt = Attempt::held(3);
        attempt.launched();
        attempt.observe_stop(stopped);
        let mut invalid = contact(true);
        invalid.position.x = f32::NAN;
        attempt.record_contact(invalid);
        assert_eq!(attempt.contact, None);
        attempt.record_contact(contact(false));
        assert_eq!(attempt.state, AttemptState::Contact);
        attempt.record_contact(contact(true));
        assert_eq!(attempt.contact, Some(contact(false)));
        attempt.mark_abandoned();
        assert_eq!(attempt.state, AttemptState::Abandoned);
        assert_eq!(attempt.contact, Some(contact(false)));
    }
}

#[test]
fn decision_readers_match_owned_native_byte_and_float_abis() {
    let Some(owned) = common::owned_main("decision_readers_match_owned_native_byte_and_float_abis")
    else {
        return;
    };
    for &(offset, bytes) in decision_guards::GUARDS {
        assert_eq!(
            bytes,
            owned.window(offset),
            "full64-byte concrete reader guard at {offset:x}"
        );
    }
    let words = |offset| {
        let bytes = owned.window(offset);
        let (chunks, _) = bytes.as_chunks::<4>();
        chunks
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect::<Vec<_>>()
    };
    // Both thunks load ControlModule from accessor+48 then dispatch slots228/230.
    for (offset, slot_word, load_word) in [
        (0x20013c0, 0xf9411502, 0x39424500),
        (0x20013d0, 0xf9411902, 0x39424100),
    ] {
        assert_eq!(
            &words(offset)[..4],
            &[0xf9402400, 0xf9400008, slot_word, 0xd61f0040]
        );
        let concrete = if offset == 0x20013c0 {
            0x3f8620
        } else {
            0x3f8630
        };
        // uxtb button, read one byte, return zero-extended value. No u16 narrowing.
        assert_eq!(
            &words(concrete)[..4],
            &[0xf9408808, 0x8b210508, load_word, 0xd65f03c0]
        );
    }
    assert_eq!(
        &words(0x2001260)[..4],
        &[0xf9402400, 0xf9400008, 0xf940bd01, 0xd61f0020]
    );
    assert_eq!(
        &words(0x3f84a0)[..7],
        &[0xf9408809, 0xbd401120, 0xb9412008, 0x7100011f, 0x1e214001, 0x1e210c00, 0xd65f03c0]
    );
}

#[test]
fn rejected_decision_delivery_retains_exact_native_snapshot_until_matching_ack() {
    use launch::{DecisionDelivery, DecisionSnapshot};
    let mut native = DecisionSnapshot {
        attempt: 17,
        contact: contact(false),
        source_status: Some(81),
        entered_status: None,
        guard_count: Some(12),
        previous_guard_gap: Some(254),
        guard_event: None,
        window_base: Some(12),
        window_multiplier: Some(f32::from_bits(0x3f855555)),
        lockout: Some(28),
        reaction: Some(f32::from_bits(0x41880001)),
        damage_fall: Some(false),
        stick_x: Some(-0.75),
        press_frame_offset: None,
    };
    let mut delivery = DecisionDelivery::EMPTY;
    assert!(delivery.capture_first(native));
    // Later native state cannot replace the exact decision while the mailbox is busy.
    assert!(!delivery.capture_first(DecisionSnapshot {
        guard_count: Some(13),
        reaction: Some(0.0),
        ..native
    }));
    assert_eq!(delivery.confirmed_decision(), None);
    assert!(delivery.observe_entered_status(native.attempt, Some(90), &[90]));
    native.entered_status = Some(90);
    for accepted in [false, false, true] {
        let sent = delivery
            .confirmed_decision()
            .expect("busy must retain decision");
        assert_eq!(sent, native);
        assert_eq!(sent.window_multiplier.unwrap().to_bits(), 0x3f855555);
        assert_eq!(sent.reaction.unwrap().to_bits(), 0x41880001);
        assert!(
            !delivery.acknowledge_decision(18),
            "stale/wrong attempt cannot clear it"
        );
        if accepted {
            assert!(delivery.acknowledge_decision(sent.attempt));
        }
    }
    assert_eq!(delivery.pending_decision(), None);
    assert!(!delivery.acknowledge_decision(17));
    assert_eq!(
        DecisionDelivery::EMPTY.pending_decision(),
        None,
        "fresh placement has no old decision"
    );
}

#[test]
fn unavailable_guard_event_survives_delivery_distinct_from_no_press() {
    use launch::{DecisionDelivery, DecisionSnapshot};
    let sample = DecisionSnapshot {
        attempt: 41,
        contact: contact(false),
        source_status: None,
        entered_status: None,
        guard_count: None,
        previous_guard_gap: None,
        guard_event: None,
        window_base: None,
        window_multiplier: None,
        lockout: None,
        reaction: None,
        damage_fall: None,
        stick_x: None,
        press_frame_offset: None,
    };
    let mut observed = Vec::new();
    for event in [None, Some(false), Some(true)] {
        let mut delivery = DecisionDelivery::EMPTY;
        assert!(delivery.capture_first(DecisionSnapshot {
            guard_event: event,
            ..sample
        }));
        for _ in 0..2 {
            assert_eq!(delivery.pending_decision().unwrap().guard_event, event);
        }
        observed.push(delivery.pending_decision().unwrap().guard_event);
        assert!(delivery.acknowledge_decision(sample.attempt));
    }
    assert_eq!(observed, [None, Some(false), Some(true)]);
    assert_ne!(observed[0], observed[1]);
}

fn unconfirmed_native_decision() -> launch::DecisionSnapshot {
    launch::DecisionSnapshot {
        attempt: 51, contact: contact(false), source_status: Some(81), entered_status: None,
        guard_count: Some(12), previous_guard_gap: Some(254), guard_event: None,
        window_base: Some(12), window_multiplier: Some(f32::from_bits(0x3f855555)),
        lockout: Some(28), reaction: Some(f32::from_bits(0x41880001)),
        damage_fall: Some(false), stick_x: Some(-0.75), press_frame_offset: None,
    }
}

#[test]
fn native_entry_requires_observed_transition_not_request() {
    use launch::DecisionDelivery;
    let sample = unconfirmed_native_decision();
    let statuses = [sample.contact.requested_status, 81, 90, 91];
    for source in [None, Some(81)] {
        let mut delivery = DecisionDelivery::EMPTY;
        // Even a supplied pre-request outcome cannot bypass native confirmation.
        assert!(delivery.capture_first(launch::DecisionSnapshot {
            source_status: source, entered_status: Some(90), ..sample
        }));
        assert_eq!(delivery.pending_decision().unwrap().entered_status, None);
        assert_eq!(delivery.confirmed_decision(), None);
        for actual in [None, Some(81), Some(999)] {
            assert!(!delivery.observe_entered_status(sample.attempt, actual, &statuses));
            assert_eq!(delivery.confirmed_decision(), None);
        }
        assert_eq!(delivery.observe_entered_status(sample.attempt, Some(90), &statuses), source.is_some());
        assert_eq!(delivery.confirmed_decision().is_some(), source.is_some());
    }
}

#[test]
fn native_entry_confirmation_retains_predecision_and_first_actual_bits() {
    use launch::DecisionDelivery;
    let sample = unconfirmed_native_decision();
    for actual in [90, 91] { // Actual tech or miss is retained even when different from request.
        assert_ne!(actual, sample.contact.requested_status);
        let mut delivery = DecisionDelivery::EMPTY;
        assert!(delivery.capture_first(sample));
        assert!(delivery.observe_entered_status(sample.attempt, Some(actual), &[90, 91]));
        let expected = launch::DecisionSnapshot { entered_status: Some(actual), ..sample };
        for later in [None, Some(81), Some(90), Some(91)] {
            assert!(!delivery.observe_entered_status(sample.attempt, later, &[90, 91]));
            assert!(!delivery.capture_first(unconfirmed_native_decision()));
            let sent = delivery.confirmed_decision().unwrap();
            assert_eq!(sent, expected); // Busy observer retries identical predecision and actual bytes.
            assert_eq!(sent.reaction.unwrap().to_bits(), 0x41880001);
            assert_eq!(sent.window_multiplier.unwrap().to_bits(), 0x3f855555);
            assert!(!delivery.acknowledge_decision(sample.attempt + 1));
        }
        assert!(delivery.acknowledge_decision(sample.attempt));
        assert_eq!(delivery.confirmed_decision(), None);
    }
}

#[test]
fn native_entry_rejects_stale_attempt_and_reset() {
    use launch::DecisionDelivery;
    let sample = unconfirmed_native_decision();
    let mut delivery = DecisionDelivery::EMPTY;
    assert!(delivery.capture_first(sample));
    assert!(!delivery.observe_entered_status(sample.attempt + 1, Some(90), &[90]));
    assert_eq!(delivery.confirmed_decision(), None);
    // Owned drop/reset discards its mailbox. A new placement has a unique attempt.
    delivery = DecisionDelivery::EMPTY;
    assert!(!delivery.observe_entered_status(sample.attempt, Some(90), &[90]));
    assert!(delivery.capture_first(launch::DecisionSnapshot { attempt: sample.attempt + 1, ..sample }));
    assert!(!delivery.observe_entered_status(sample.attempt, Some(90), &[90]));
    assert_eq!(delivery.confirmed_decision(), None);
    assert!(delivery.observe_entered_status(sample.attempt + 1, Some(90), &[90]));
}

#[test]
fn raw_contact_delivery_boundary_preserves_confirmed_and_unavailable_cases() {
    use launch::DecisionDelivery;
    // Resolved native IDs are supplied by the caller; no numeric game IDs guessed.
    let [passive, roll, down] = [200, 201, 202];
    for (requested, actual, stick) in [(down, passive, 0.0), (passive, roll, -1.0), (passive, roll, 1.0), (passive, down, 0.0)] {
        let mut raw = unconfirmed_native_decision();
        raw.contact.requested_status = requested;
        raw.stick_x = Some(stick);
        let mut delivery = DecisionDelivery::EMPTY;
        delivery.capture_first(raw);
        assert_eq!(delivery.confirmed_decision(), None);
        assert!(delivery.observe_entered_status(raw.attempt, Some(actual), &[passive, roll, down]));
        let delivered = delivery.confirmed_decision().unwrap();
        assert_eq!(delivered.entered_status, Some(actual));
        assert_eq!(delivered.contact.requested_status, requested);
        assert_ne!(actual, requested);
        assert_eq!(delivered.stick_x, Some(stick)); // Both roll directions preserved for UI mapping.
        assert_eq!(delivery.pending_decision(), Some(delivered)); // Read-only query does not consume.
        assert_eq!(delivery.confirmed_decision(), Some(delivered));
    }
    let raw = unconfirmed_native_decision();
    // Ungranted status getter, unknown source, source still current, non-contact, no transition.
    for (source, actual) in [(Some(81), None), (None, Some(passive)), (Some(passive), Some(passive)), (Some(81), Some(999)), (Some(81), Some(81))] {
        let raw = launch::DecisionSnapshot { source_status: source, ..raw };
        let mut delivery = DecisionDelivery::EMPTY;
        delivery.capture_first(raw);
        for _ in 0..45 {
            assert!(!delivery.observe_entered_status(raw.attempt, actual, &[passive, roll, down]));
            assert_eq!(delivery.confirmed_decision(), None);
            assert_eq!(delivery.pending_decision(), Some(raw)); // UI can age exact contact to its existing hold.
        }
        assert_eq!(delivery.pending_decision().unwrap().entered_status, None);
    }
}

#[test]
fn converted_base_damage_is_positive_without_reapplying_visible_ui_range() {
    for visible in [1.0f32,30.0] {
        let base_damage=visible/1.2f32;
        assert!(Launch {hit_damage:base_damage,..request()}.valid());
    }
    for invalid in [0.0,-1.0,f32::NAN,f32::INFINITY] {
        assert!(!Launch {hit_damage:invalid,..request()}.valid());
    }
}
