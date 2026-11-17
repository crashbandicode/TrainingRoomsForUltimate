//! Shared DamageInfo seed and native stop-frame invariant. Offsets pinned against the owned 13.0.5 ELF.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub height: i32,
    pub speed: [f32; 2],
    pub angle: f32,
}
impl Seed {
    /// Magnitude and trig are supplied by the guarded native functions in the hook.
    #[cfg(any(test, feature = "diag"))]
    pub fn derived(height: i32, magnitude: f32, direction: [f32; 2], world_degrees: f32) -> Self {
        Self {
            height,
            speed: [magnitude * direction[0], magnitude * direction[1]],
            angle: relative_angle(world_degrees, direction[0] >= 0.0),
        }
    }
    #[cfg(any(test, feature = "diag"))]
    pub fn from_info(info: &[u8; 0xa0]) -> Self {
        let word = |offset| u32::from_le_bytes(info[offset..offset + 4].try_into().unwrap());
        Self {
            height: word(8) as i32,
            speed: [f32::from_bits(word(0x10)), f32::from_bits(word(0x14))],
            angle: f32::from_bits(word(0x40)),
        }
    }
    pub fn valid(self, heights: [i32; 3]) -> bool {
        heights.contains(&self.height)
            && self.speed.iter().all(|v| v.is_finite())
            && self.angle.is_finite()
    }
    /// Caller supplies the guarded inline Info pointer and SDK-resolved height constants.
    /// Validate the complete seed before any mutation; write exactly five 32-bit fields.
    pub unsafe fn write(self, info: *mut u8, heights: [i32; 3], frames: i32) -> bool {
        if info.is_null() || !self.valid(heights) || frames <= 0 {
            return false;
        }
        (info.add(0x08) as *mut i32).write_unaligned(self.height);
        (info.add(0x10) as *mut f32).write_unaligned(self.speed[0]);
        (info.add(0x14) as *mut f32).write_unaligned(self.speed[1]);
        (info.add(0x40) as *mut f32).write_unaligned(self.angle);
        (info.add(0x4c) as *mut i32).write_unaligned(frames);
        true
    }
}
/// Facing-normalized launch angle. Past-vertical mapping remains diagnostic/unverified.
pub fn relative_angle(world_degrees: f32, toward_right: bool) -> f32 {
    let degrees = if toward_right {
        world_degrees
    } else {
        180.0 - world_degrees
    };
    degrees * (std::f32::consts::PI / 180.0)
}

/// Native transactor input retained in Info, not the number of observed callbacks.
#[cfg(any(test, feature = "diag"))]
pub fn hitstop_frames(info: &[u8; 0xa0]) -> i32 {
    i32::from_le_bytes([info[0x4c], info[0x4d], info[0x4e], info[0x4f]])
}
