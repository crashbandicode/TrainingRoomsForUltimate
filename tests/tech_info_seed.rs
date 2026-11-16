#[path = "../src/runtime/features/tech_info_seed.rs"]
mod info;
use info::Seed;
#[test]
fn matched_high_fields_preserve_every_other_byte() {
    let seed = Seed {
        height: 2,
        speed: [f32::from_bits(0xc00612a4), f32::from_bits(0x3fd17f92)],
        angle: f32::from_bits(0x3f29c91f),
    };
    // Deliberately unaligned, with nonzero sentinels at all unapproved offsets.
    let mut storage = [0xa5u8; 0xa1];
    assert!(unsafe { seed.write(storage.as_mut_ptr().add(1), [0, 1, 2]) });
    let bytes: &[u8; 0xa0] = (&storage[1..]).try_into().unwrap();
    assert_eq!(Seed::from_info(bytes), seed);
    for (i, &byte) in bytes.iter().enumerate() {
        if !(8..12).contains(&i) && !(0x10..0x18).contains(&i) && !(0x40..0x44).contains(&i) {
            assert_eq!(byte, 0xa5, "offset {i:x}");
        }
    }
    assert_eq!(storage[0], 0xa5);
}
#[test]
fn bad_seed_writes_nothing() {
    for seed in [
        Seed {
            height: -1,
            speed: [1.0, 2.0],
            angle: 0.0,
        },
        Seed {
            height: 1,
            speed: [f32::NAN, 2.0],
            angle: 0.0,
        },
        Seed {
            height: 1,
            speed: [1.0, 2.0],
            angle: f32::INFINITY,
        },
    ] {
        let mut bytes = [0xa5; 0xa0];
        assert!(!unsafe { seed.write(bytes.as_mut_ptr(), [0, 1, 2]) });
        assert_eq!(bytes, [0xa5; 0xa0]);
    }
}
#[test]
fn angle_38_front_and_behind_matches_real_info_bits() {
    assert_eq!(info::relative_angle(38.0, true).to_bits(), 0x3f29c91f);
    assert_eq!(info::relative_angle(142.0, false).to_bits(), 0x3f29c91f);
}

#[test]
fn exact_recorded_kb_high_projection_reproduces_real_info() {
    // V5 R1 KB is 88.61502075195312, not the rounded display string88.615.
    // Owned native helper uses this parameter. These host arithmetic/trig inputs
    // reproduce the captured Info; they do not assert execution of Switch libm.
    let kb = f32::from_bits(0x42b13ae4);
    assert_ne!(kb.to_bits(), 88.615f32.to_bits());
    let magnitude = kb * f32::from_bits(0x3cf5c28f);
    assert_eq!(magnitude.to_bits(), 0x402a240e);
    let angle = info::relative_angle(38.0, true);
    let (sin, cos) = angle.sin_cos();
    assert_eq!([cos.to_bits(), sin.to_bits()], [0x3f49bb13, 0x3f1d9bfe]);
    for (degrees, sign, expected_x) in [(38.0, 1.0, 0x400612a4), (142.0, -1.0, 0xc00612a4)] {
        let seed = Seed::derived(2, magnitude, [sign * cos, sin], degrees);
        assert_eq!(seed.height, 2);
        assert_eq!(seed.speed[0].to_bits(), expected_x);
        assert_eq!(seed.speed[1].to_bits(), 0x3fd17f92);
        assert_eq!(seed.angle.to_bits(), 0x3f29c91f);
    }
}
