//! Pointer-free native draft values. The native saved ID is deliberately zero.
use {crate::engine::mii::inventory::{self, MiiConfig}, crate::engine::mii::lifetime::Configuration, crate::engine::mii::saved};

pub type Row = [u8; saved::HEADER_LEN];

pub fn from_seed(seed: &MiiConfig) -> Result<Row, &'static str> {
    Configuration::from_saved(seed).map_err(|_| "Invalid editor appearance")?;
    let mut row = [0; saved::HEADER_LEN];
    let mut units = [0u16; 10];
    let mut len = 0;
    for ch in seed.name.chars() {
        let mut encoded = [0; 2];
        let next = ch.encode_utf16(&mut encoded);
        if len + next.len() > units.len() { break; }
        units[len..len + next.len()].copy_from_slice(next);
        len += next.len();
    }
    if len == 0 {
        for unit in "New Mii".encode_utf16() { units[len] = unit; len += 1; }
    }
    row[0x114..0x118].copy_from_slice(&(len as u32).to_le_bytes());
    for (i, unit) in units[..len].iter().enumerate() {
        row[0x118 + i * 2..0x11a + i * 2].copy_from_slice(&unit.to_le_bytes());
    }
    for (offset, tag) in [0x130, 0x138, 0x1a0].into_iter().zip(seed.raw_tags) {
        row[offset..offset + 8].copy_from_slice(&tag.0.to_le_bytes());
    }
    let appearance = seed.canonical.appearance;
    row[0x140] = appearance.colour;
    row[0x141] = appearance.voice;
    row[0x142..0x146].copy_from_slice(&appearance.specials);
    row[0x148..0x1a0].copy_from_slice(&seed.charinfo);
    let checked = configuration(&row)?;
    if checked.canonical() != seed.canonical { return Err("Editor seed differs from selected Mii"); }
    Ok(row)
}

pub fn parse(row: &Row) -> Result<(String, Configuration), &'static str> {
    let (raw, value) = validated(row)?;
    Ok((String::from_utf16_lossy(&raw.name_units[..raw.name_len as usize]), value))
}

/// Native handoffs need only the checked owned value, never a display name.
pub(crate) fn configuration(row: &Row) -> Result<Configuration, &'static str> {
    validated(row).map(|(_, value)| value)
}

fn validated(row: &Row) -> Result<(inventory::RawSlot, Configuration), &'static str> {
    let raw = saved::parse_header_value(0, 0, row).map_err(|_| "Invalid owned editor row")?;
    let canonical = inventory::classify_canonical(&raw).map_err(|_| "Invalid edited Mii")?;
    let value = Configuration::from_saved(&MiiConfig {
        slot_id: raw.slot_id, slot_index: raw.index, name: String::new(),
        charinfo: raw.charinfo, raw_tags: [raw.tag_outfit, raw.tag_hat, raw.tag_chara],
        canonical, copies: Vec::new(),
    }).map_err(|_| "Edited Mii resources differ")?;
    Ok((raw, value))
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::mii::inventory::{classify, RawSlot, MiiClass, TaggedId, CHARINFO_LEN, TAG_CHARA, TAG_HAT, TAG_OUTFIT};
    fn seed() -> MiiConfig {
        classify(&RawSlot { index: 0, slot_id: 2622, name_len: 1,
            name_units: [65, 0, 0, 0, 0, 0, 0, 0, 0, 0], colour: 1, voice: 37,
            specials: [2, 1, 0, 2], charinfo: [7; CHARINFO_LEN],
            tag_outfit: TaggedId((u64::from(TAG_OUTFIT) << 56) | (1 << 40) | 0x1bfef7a5f7),
            tag_hat: TaggedId((u64::from(TAG_HAT) << 56) | 0x0fa9efa5ea),
            tag_chara: TaggedId((u64::from(TAG_CHARA) << 56) | MiiClass::Swordfighter.chara_hash40()),
        }).unwrap()
    }
    fn owned_editor_row_drops_native_id_and_keeps_configuration() {
        let seed = seed(); let before = seed.clone();
        let row = from_seed(&seed).unwrap();
        assert_eq!(&row[..12], &[0; 12]);
        let (name, value) = parse(&row).unwrap();
        assert_eq!(name, seed.name);
        assert_eq!(value.canonical(), seed.canonical);
        assert_eq!(value.charinfo(), &seed.charinfo);
        assert_eq!(value.tokens().outfit, seed.outfit_hint());
        assert_eq!(value.tokens().hat, seed.hat_hint());
        assert_eq!(seed, before);
        let mut native_id = row;
        native_id[..4].copy_from_slice(&2622u32.to_le_bytes());
        assert!(parse(&native_id).is_err());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("owned_editor_row_drops_native_id_and_keeps_configuration", owned_editor_row_drops_native_id_and_keeps_configuration as fn()),
        ]);
    }
    }
}
