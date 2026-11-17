//! SOL-R23: the native copy precondition of a Kirby hat (`FighterSpecializer_Kirby::copy_setup`).
//!
//! For the kinds below, `copy_setup` → `main+0xba4090` → `main+0x6de830` reads the copied kind's hat resource pair for this
//! Kirby from the owner table and passes its object to a virtual call with no null check (`main+0x491e1c`). The pair lives at
//! `record(kind) + costume * 0x2e8 + kirby_entry * 0x10 + 0x20`. `record(kind)` is `main+0x17f09d0`: twenty records of
//! `0x1748` bytes from `[[main+0x5324680] + 0x68] + 0x98`, each starting with its kind. The costume is the byte `+0x41` of
//! Kirby's fighter entry, and the entry is Kirby's `FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID`. When the quiet preload has released
//! that pair (repro kh2-repro-xps-1: frame 43268, pair `{0, 0}`), the copy jumps to address 0. A hat request for these kinds
//! waits until the pair is there.

/// The kinds whose copy reads the pair: the mask at `main+0x6de88c..9c` (Falco, Mr. Game & Watch, Wii Fit Trainer) and the
/// compares at `+0x6de8a0..ac` (Steve, Kazuya).
pub const PAIR_KINDS: [u32; 5] = [0x14, 0x1c, 0x32, 0x58, 0x5c];
/// `[owner table] + 0x68`: the manager `main+0x6de830` reads; the number of `main+0x17f09d0` records; Kirby's fighter-entry
/// byte that selects the row (`main+0xba440c`: `ldrb w21, [x8, #0x41]`).
pub const MANAGER_FIELD: usize = 0x68;
pub const RECORDS: usize = 20;
pub const COSTUME_FIELD: usize = 0x41;

/// Offset of record `index` (< `RECORDS`) from the manager.
pub fn record_offset(index: usize) -> usize {
    0x98 + index * 0x1748
}

/// Offset of the pair's object from its record, or None for a costume or entry the native never indexes (`entry` < 8,
/// checked at `main+0xba43d8`; eight costume rows fit one record).
pub fn pair_offset(costume: u8, entry: u32) -> Option<usize> {
    (costume < 8 && entry < 8).then(|| costume as usize * 0x2e8 + entry as usize * 0x10 + 0x20)
}

/// Whether the hat of `kind` may be copied now, from the pair's object word read from the game (None: unreadable, or no
/// record has the kind). A pair kind without a live object waits; the existing timeout ends a request whose pair never comes.
pub fn copy_ready(kind: u32, object: Option<usize>) -> bool {
    !PAIR_KINDS.contains(&kind) || object.is_some_and(|object| object != 0)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn the_pair_kinds_are_the_native_ones() {
        // The mask at main+0x6de890..94: 0x0004_0000_1010_0000, then the compares with 0x5c and 0x58.
        let mask: u64 = 0x0004_0000_1010_0000;
        for kind in 0..=0x32u32 {
            assert_eq!(mask >> kind & 1 == 1, [0x14, 0x1c, 0x32].contains(&kind), "kind {kind:#x}");
        }
        assert!(PAIR_KINDS.contains(&0x58) && PAIR_KINDS.contains(&0x5c));
    }

    #[test]
    fn a_released_pair_waits_and_other_kinds_do_not() {
        // kh2-repro-xps-1 frame 43268: Falco, record found, pair {0, 0} -> the copy jumped to 0.
        assert!(!copy_ready(0x14, Some(0)));
        assert!(!copy_ready(0x1c, None));
        // frame 24604: pair object 0x11fcc99400 -> the hat applied.
        assert!(copy_ready(0x14, Some(0x11fc_c994_00)));
        // Mario and Luigi never read the pair.
        assert!(copy_ready(0x00, None) && copy_ready(0x09, Some(0)));
    }

    #[test]
    fn the_offsets_match_the_disassembly() {
        assert_eq!([record_offset(0), record_offset(1), record_offset(2), record_offset(19)], [0x98, 0x17e0, 0x2f28, 0x1baf0]);
        assert_eq!(pair_offset(0, 0), Some(0x20));
        assert_eq!(pair_offset(1, 2), Some(0x2e8 + 0x20 + 0x20));
        assert!(pair_offset(7, 7).unwrap() + 16 <= 0x1748);
        assert_eq!((pair_offset(8, 0), pair_offset(0, 8)), (None, None));
    }
    }
}
