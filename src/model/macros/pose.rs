/// The animation's 40-bit name and its frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose { pub motion: u64, pub frame: f32 }

impl Pose {
    /// The name is not 0 and below 2^40; the frame is finite and from 0 to 65,535.
    pub fn is_valid(self) -> bool {
        self.motion != 0 && self.motion < (1 << 40)
            && self.frame.is_finite() && (0.0..=65_535.0).contains(&self.frame)
    }
}

/// 8 bytes, alignment 4. The name's low 32 bits and its top 8 bits with a 24.8 frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackedPose(pub [u32; 2]);

impl PackedPose {
    pub const NONE: Self = Self([0, 0]);

    pub fn pack(pose: Pose) -> Self {
        if !pose.is_valid() {
            return Self::NONE;
        }
        let frame = (pose.frame * 256.0).round().min(0xFF_FFFF as f32) as u32;
        Self([pose.motion as u32, ((pose.motion >> 32) as u32) | (frame << 8)])
    }

    pub fn get(self) -> Option<Pose> {
        let pose = Pose {
            motion: self.0[0] as u64 | (((self.0[1] & 0xFF) as u64) << 32),
            frame: (self.0[1] >> 8) as f32 / 256.0,
        };
        pose.is_valid().then_some(pose)
    }

    /// What a sampler stores: no pose unless both reads were granted (a refused read is not a frame of 0).
    pub fn sampled(motion: Option<u64>, frame: Option<f32>) -> Self {
        match (motion, frame) { (Some(motion), Some(frame)) => Self::pack(Pose { motion, frame }), _ => Self::NONE }
    }

    pub fn to_le_bytes(self) -> [u8; 8] {
        let first = self.0[0].to_le_bytes();
        let second = self.0[1].to_le_bytes();
        [first[0], first[1], first[2], first[3], second[0], second[1], second[2], second[3]]
    }

    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self([u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])])
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn a_pose_packs_into_two_words_and_back() {
        let pose = Pose { motion: 0x12_3456_789A, frame: 12.5 };
        let packed = PackedPose([0x3456_789A, 0x000C_8012]);
        assert_eq!(PackedPose::pack(pose), packed);
        assert_eq!(packed.get(), Some(pose));
        let high = Pose { motion: 1, frame: 65_535.0 };
        let high_packed = PackedPose([1, 0xFFFF_0000]);
        assert_eq!(PackedPose::pack(high), high_packed);
        assert_eq!(high_packed.get(), Some(high));
        let bytes = [0x9A, 0x78, 0x56, 0x34, 0x12, 0x80, 0x0C, 0x00];
        assert_eq!(packed.to_le_bytes(), bytes);
        assert_eq!(PackedPose::from_le_bytes(bytes), packed);
    }

    fn the_frame_rounds_to_a_256th() {
        assert_eq!(PackedPose::pack(Pose { motion: 1, frame: 0.3 }).get().unwrap().frame, 77.0 / 256.0);
        assert_eq!(PackedPose::pack(Pose { motion: 1, frame: 1.0 / 512.0 }).get().unwrap().frame, 1.0 / 256.0);
    }

    fn a_pose_that_is_not_valid_packs_as_none() {
        for pose in [
            Pose { motion: 0, frame: 0.0 }, Pose { motion: 1 << 40, frame: 0.0 },
            Pose { motion: 1, frame: f32::NAN }, Pose { motion: 1, frame: f32::INFINITY },
            Pose { motion: 1, frame: -0.5 }, Pose { motion: 1, frame: 65_535.5 },
        ] {
            assert_eq!(PackedPose::pack(pose), PackedPose::NONE);
        }
    }

    fn none_and_foreign_words_give_no_pose() {
        assert_eq!(PackedPose::NONE.get(), None);
        assert_eq!(PackedPose([0, 0x0000_0500]).get(), None);
        assert_eq!(PackedPose([1, 0xFFFF_FF00]).get(), None);
        assert_eq!(PackedPose::default(), PackedPose::NONE);
    }

    fn a_refused_read_gives_no_pose_and_frame_zero_is_a_pose() {
        assert_eq!(PackedPose::sampled(Some(7), None), PackedPose::NONE);
        assert_eq!(PackedPose::sampled(None, Some(1.0)), PackedPose::NONE);
        assert_eq!(PackedPose::sampled(None, None), PackedPose::NONE);
        let zero = PackedPose::sampled(Some(7), Some(0.0));
        assert_eq!(zero, PackedPose([7, 0]));
        assert_eq!(zero.get(), Some(Pose { motion: 7, frame: 0.0 }));
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_pose_packs_into_two_words_and_back", a_pose_packs_into_two_words_and_back as fn()),
            ("the_frame_rounds_to_a_256th", the_frame_rounds_to_a_256th as fn()),
            ("a_pose_that_is_not_valid_packs_as_none", a_pose_that_is_not_valid_packs_as_none as fn()),
            ("none_and_foreign_words_give_no_pose", none_and_foreign_words_give_no_pose as fn()),
            ("a_refused_read_gives_no_pose_and_frame_zero_is_a_pose", a_refused_read_gives_no_pose_and_frame_zero_is_a_pose as fn()),
        ]);
    }
    }
}
