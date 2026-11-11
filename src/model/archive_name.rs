//! Borrowed SARC directory lookup for the layout-allocation callback.
//!
//! Identifying a layout needs its directory, not owned copies of every texture
//! and animation. All reads stay within the caller's slice; no pointer escapes,
//! no payload is copied, and malformed directories return `None`.

use std::convert::TryInto;

pub fn contains(bytes: &[u8], expected: &[u8]) -> Option<bool> {
    if bytes.get(..4)? != b"SARC" { return None; }
    let little = match bytes.get(6..8)? {
        [0xff, 0xfe] => true,
        [0xfe, 0xff] => false,
        _ => return None,
    };
    let u16_at = |at: usize| -> Option<usize> {
        let b: [u8; 2] = bytes.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if little { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) } as usize)
    };
    let u32_at = |at: usize| -> Option<usize> {
        let b: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if little { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) } as usize)
    };
    let header = u16_at(4)?;
    let size = u32_at(8)?;
    let payload = u32_at(12)?;
    if header < 20 || size > bytes.len() || payload > size { return None; }
    if bytes.get(header..header.checked_add(4)?)? != b"SFAT" { return None; }
    let fat_size = u16_at(header.checked_add(4)?)?;
    let count = u16_at(header.checked_add(6)?)?;
    if fat_size < 12 { return None; }
    let nodes = header.checked_add(fat_size)?;
    let names_header = nodes.checked_add(count.checked_mul(16)?)?;
    if bytes.get(names_header..names_header.checked_add(4)?)? != b"SFNT" { return None; }
    let names_size = u16_at(names_header.checked_add(4)?)?;
    if names_size < 8 { return None; }
    let names_start = names_header.checked_add(names_size)?;
    let names = bytes.get(names_start..payload)?;
    let mut found = false;
    for i in 0..count {
        let node = nodes + i * 16; // bounded by names_header above
        let attrs = u32_at(node + 4)?;
        let start = u32_at(node + 8)?;
        let end = u32_at(node + 12)?;
        if start > end || end > size - payload { return None; }
        if attrs & 0x0100_0000 != 0 {
            let offset = (attrs & 0x00ff_ffff).checked_mul(4)?;
            let tail = names.get(offset..)?;
            let len = tail.iter().position(|&b| b == 0)?;
            found |= &tail[..len] == expected;
        }
    }
    Some(found)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn fixture(little: bool, name: Option<&[u8]>, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 64 + payload.len()];
        bytes[..4].copy_from_slice(b"SARC");
        let put16 = |b: &mut [u8], at, n: u16| {
            b[at..at+2].copy_from_slice(&if little { n.to_le_bytes() } else { n.to_be_bytes() });
        };
        let put32 = |b: &mut [u8], at, n: u32| {
            b[at..at+4].copy_from_slice(&if little { n.to_le_bytes() } else { n.to_be_bytes() });
        };
        put16(&mut bytes, 4, 20);
        put16(&mut bytes, 6, 0xfeff);
        put32(&mut bytes, 8, (64 + payload.len()) as u32);
        put32(&mut bytes, 12, 64);
        put16(&mut bytes, 16, 0x100);
        bytes[20..24].copy_from_slice(b"SFAT");
        put16(&mut bytes, 24, 12);
        put16(&mut bytes, 26, 1);
        put32(&mut bytes, 28, 0x65);
        put32(&mut bytes, 36, if name.is_some() { 0x0100_0000 } else { 0 });
        put32(&mut bytes, 44, payload.len() as u32);
        bytes[48..52].copy_from_slice(b"SFNT");
        put16(&mut bytes, 52, 8);
        if let Some(name) = name { bytes[56..56+name.len()].copy_from_slice(name); }
        bytes[64..].copy_from_slice(payload);
        bytes
    }

    #[test]
    fn reads_both_endians_without_searching_payload_or_unnamed_entries() {
        for little in [false, true] {
            assert_eq!(contains(&fixture(little, Some(b"target"), b"data"), b"target"), Some(true));
            assert_eq!(contains(&fixture(little, Some(b"other"), b"target\0"), b"target"), Some(false));
            assert_eq!(contains(&fixture(little, None, b"target\0"), b"target"), Some(false));
            assert_eq!(contains(&fixture(little, Some(b"target"), b"data"), b"targ"), Some(false));
        }
    }

    #[test]
    fn truncated_or_invalid_directory_never_matches() {
        let good = fixture(true, Some(b"target"), b"data");
        for end in 0..good.len() { assert_eq!(contains(&good[..end], b"target"), None); }
        for offset in [0, 4, 6, 8, 12, 20, 24, 26, 36, 40, 44, 48, 52] {
            let mut bad = good.clone();
            bad[offset] ^= 0x80;
            assert_eq!(contains(&bad, b"target"), None, "offset {offset}");
        }
        let mut bad = good;
        bad[56..64].fill(b'x');
        assert_eq!(contains(&bad, b"target"), None);
    }

    #[test]
    fn non_utf8_names_do_not_match_valid_layout_names() {
        assert_eq!(contains(&fixture(true, Some(b"\xff"), b"data"), b"target"), Some(false));
    }
    }
}
