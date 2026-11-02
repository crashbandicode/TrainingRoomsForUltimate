//! Independent review checks for the public recording format's precision contract.
use smash_match_data::model::*;

fn input_event(left_stick: Channel<Stick>) -> Event {
    Event {
        seq: 4,
        clocks: Clocks::tick(12).with_input_seq(9),
        body: EventBody::Input(InputEvent {
            controller: Some(1),
            entry: Some(0),
            actor: Some(ActorId(42)),
            origin: InputOrigin::CpuGenerated,
            layout: "review-native-f32".into(),
            buttons: Channel::Observed(Buttons::new(0x8000_0001)),
            left_stick,
            right_stick: Channel::Observed(Stick::F32 { x: -0.0, y: 0.0 }),
            raw: None,
        }),
    }
}

#[test]
fn cpu_float_bits_survive_the_actual_nested_json_event() {
    // Native unit-range values that signed-byte macro conversion would change,
    // plus signed zero. The shared format must never perform that conversion.
    let values = [
        0x0000_0000_u32,
        0x8000_0000,
        0x3dcc_cccd,
        0xbdcc_cccd,
        0x3f35_04f3,
        0x3eaa_aaab,
        0x3f7f_ffff,
        0x3f80_0000,
        0xbf80_0000,
    ];
    for bits in values {
        let value = f32::from_bits(bits);
        let original = input_event(Channel::Observed(Stick::F32 { x: value, y: value }));
        let json = serde_json::to_vec(&original).unwrap();
        let decoded: Event = serde_json::from_slice(&json).unwrap();
        let EventBody::Input(input) = decoded.body else {
            panic!("input event changed kind");
        };
        let Channel::Observed(Stick::F32 { x, y }) = input.left_stick else {
            panic!("native floats were converted or lost");
        };
        assert_eq!(x.to_bits(), bits);
        assert_eq!(y.to_bits(), bits);
        let Channel::Observed(Stick::F32 { x, y }) = input.right_stick else {
            panic!("second stick was lost");
        };
        assert_eq!(x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(y.to_bits(), 0.0_f32.to_bits());
        assert_eq!(input.actor, Some(ActorId(42)));
        assert_eq!(input.controller, Some(1));
        assert_eq!(input.buttons, Channel::Observed(Buttons::new(0x8000_0001)));
    }
}

#[test]
fn neutral_and_each_missingness_marker_remain_distinct_in_json() {
    let values = [
        Channel::Observed(Stick::F32 { x: 0.0, y: 0.0 }),
        Channel::Missing,
        Channel::Unsupported,
        Channel::Unchanged,
    ];
    let mut encodings = Vec::new();
    for expected in values {
        let json = serde_json::to_string(&input_event(expected)).unwrap();
        let decoded: Event = serde_json::from_str(&json).unwrap();
        let EventBody::Input(input) = decoded.body else {
            panic!("input event changed kind");
        };
        assert_eq!(input.left_stick, expected);
        assert!(!encodings.contains(&json));
        encodings.push(json);
    }
}
