//! Review of selection boundaries and the operator's explicit competitive target.
use smash_match_data_oracle::clip::{clip_events, SelectAt};
use smash_match_data_oracle::model::*;
use smash_match_data_oracle::validate::{competitive_conformance, Conformance};

#[test]
fn reversed_clip_is_invalid_while_equal_endpoints_are_empty() {
    let events: Vec<_> = (0..3)
        .map(|seq| Event {
            seq,
            clocks: Clocks::tick(seq),
            body: EventBody::Note {
                severity: Severity::Info,
                text: format!("sample {seq}"),
            },
        })
        .collect();
    assert!(clip_events(&events, SelectAt::Seq(2), SelectAt::Seq(0)).is_none());
    assert!(clip_events(&events, SelectAt::Seq(1), SelectAt::Seq(1))
        .unwrap()
        .is_empty());
    assert_eq!(
        clip_events(&events, SelectAt::Seq(0), SelectAt::Seq(2)).unwrap(),
        &events[1..3]
    );
}

#[test]
fn competitive_target_checks_the_two_agreed_settings() {
    let mut actual = ActualRules {
        items: Some(ItemSetting::Off),
        stage_hazards: Some(false),
        spirits: Some(true),
        final_smash_meter: Some(true),
        stage_morph: Some(true),
        underdog_boost: Some(true),
        ..Default::default()
    };
    assert_eq!(competitive_conformance(&actual), Conformance::Conforms);
    actual.stage_hazards = Some(true);
    assert!(matches!(
        competitive_conformance(&actual),
        Conformance::Deviates { .. }
    ));
    actual.stage_hazards = None;
    assert!(matches!(
        competitive_conformance(&actual),
        Conformance::Undetermined { .. }
    ));
}
