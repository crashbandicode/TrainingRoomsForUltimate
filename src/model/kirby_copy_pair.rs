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
/// `[owner table] + 0x68`: the manager `main+0x6de830` reads (`ldr x22, [x22, #0x68]`).
pub const MANAGER_FIELD: usize = 0x68;
/// The first record and the record stride of `main+0x17f09d0`, and its record count.
pub const RECORDS_AT: usize = 0x98;
pub const RECORD_STRIDE: usize = 0x1748;
pub const RECORDS: usize = 20;
/// The pair inside a record: row = costume (`0x2e8` each), column = Kirby's entry (`0x10` each), then `+0x20`.
pub const ROW: usize = 0x2e8;
pub const COLUMN: usize = 0x10;
pub const PAIR_AT: usize = 0x20;
/// Kirby's fighter-entry byte that selects the row (`main+0xba440c`: `ldrb w21, [x8, #0x41]`).
pub const COSTUME_FIELD: usize = 0x41;

/// Whether the copy of `kind` reads the pair.
pub fn needs_pair(kind: u32) -> bool {
    PAIR_KINDS.contains(&kind)
}

/// Offset of record `index` from the manager, or None past the last record.
pub fn record_offset(index: usize) -> Option<usize> {
    (index < RECORDS).then(|| RECORDS_AT + index * RECORD_STRIDE)
}

/// Offset of the pair's object from its record, or None for a costume or entry the native never indexes (`entry` < 8,
/// checked at `main+0xba43d8`; eight costume rows fit one record).
pub fn pair_offset(costume: u8, entry: u32) -> Option<usize> {
    (costume < 8 && entry < 8).then(|| costume as usize * ROW + entry as usize * COLUMN + PAIR_AT)
}

/// The facts read from the game for one hat request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pair {
    /// The kind's copy does not read the pair.
    NotNeeded,
    /// The owner table, the manager or Kirby's entry could not be read.
    Unreadable,
    /// No record has this kind.
    NoRecord,
    /// The record exists; this is the object word of the pair (0 = released).
    Object(usize),
}

/// Whether the hat may be copied now. Anything but a live object for a pair kind waits (the request keeps its age and ends
/// with the existing "Kirby hat could not be loaded" if the pair never comes).
pub fn copy_ready(pair: Pair) -> bool {
    match pair {
        Pair::NotNeeded => true,
        Pair::Object(object) => object != 0,
        Pair::Unreadable | Pair::NoRecord => false,
    }
}

lib_unit_tests! {
mod tests {
    use super::*;

    #[test]
    fn the_pair_kinds_are_the_native_ones() {
        assert!(needs_pair(0x14) && needs_pair(0x1c) && needs_pair(0x32) && needs_pair(0x58) && needs_pair(0x5c));
        assert!(!needs_pair(0x00) && !needs_pair(0x09) && !needs_pair(0x15));
        // The mask at main+0x6de890..94: 0x0004_0000_1010_0000.
        let mask: u64 = 0x0004_0000_1010_0000;
        for kind in 0..=0x32u32 {
            assert_eq!(mask >> kind & 1 == 1, [0x14, 0x1c, 0x32].contains(&kind), "kind {kind:#x}");
        }
    }

    #[test]
    fn a_released_pair_is_not_ready() {
        // kh2-repro-xps-1 frame 43268: record found, pair {0, 0} -> the copy jumped to 0.
        assert!(!copy_ready(Pair::Object(0)));
        assert!(!copy_ready(Pair::NoRecord));
        assert!(!copy_ready(Pair::Unreadable));
        // frame 24604: pair object 0x11fcc99400 -> the hat applied.
        assert!(copy_ready(Pair::Object(0x11fc_c994_00)));
        assert!(copy_ready(Pair::NotNeeded));
    }

    #[test]
    fn the_offsets_match_the_disassembly() {
        assert_eq!(record_offset(0), Some(0x98));
        assert_eq!(record_offset(1), Some(0x17e0));
        assert_eq!(record_offset(2), Some(0x2f28));
        assert_eq!(record_offset(19), Some(0x1baf0));
        assert_eq!(record_offset(20), None);
        assert_eq!(pair_offset(0, 0), Some(0x20));
        assert_eq!(pair_offset(1, 2), Some(0x2e8 + 0x20 + 0x20));
        assert_eq!(pair_offset(7, 7), Some(7 * 0x2e8 + 7 * 0x10 + 0x20));
        assert!(pair_offset(7, 7).unwrap() + 16 <= RECORD_STRIDE);
        assert_eq!(pair_offset(8, 0), None);
        assert_eq!(pair_offset(0, 8), None);
    }
}
}
