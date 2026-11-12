//! The game's path hash (`hash40`): the path's byte length in the top byte pair and the IEEE CRC-32 of the
//! bytes below it, `(len << 32) | crc32(path)`. ARCropolis and the game's archive address files by it
//! (SPEC-core-07 §4.5). The path is the archive-relative one, lower case, without a mount prefix.

/// `crc32` (IEEE 802.3, reflected, polynomial 0xedb88320).
pub const fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i] as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            bit += 1;
        }
        i += 1;
    }
    !crc
}

pub const fn hash40(path: &str) -> u64 {
    ((path.len() as u64) << 32) | crc32(path.as_bytes()) as u64
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    /// The values the ARCropolis audit lists (`artifacts/enhancements/arc-stage-assets/REPORT.md`).
    fn the_audited_stage_and_message_paths_hash_as_listed() {
        assert_eq!(hash40("ui/replace/stage/stage_1/stage_1_battlefield.bntx"), 0x31c1_6bfb09);
        assert_eq!(hash40("ui/replace/stage/stage_1/stage_1_end.bntx"), 0x294c_d46fe9);
        assert_eq!(hash40("ui/replace/stage/stage_0/stage_0_end.bntx"), 0x294c_44b236);
        assert_eq!(hash40("ui/message/msg_name.msbt"), 0x1839_3b2070);
    }

    fn the_effect_file_hashes_as_the_trace_says() {
        assert_eq!(hash40("effect/system/common/ef_common.eff"), 0x222d_35b012);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_audited_stage_and_message_paths_hash_as_listed", the_audited_stage_and_message_paths_hash_as_listed as fn()),
            ("the_effect_file_hashes_as_the_trace_says", the_effect_file_hashes_as_the_trace_says as fn()),
        ]);
    }
    }
}