//! Owned 13.0.5 replay Mii values. No native pointers, file I/O or loading authority.
//!
//! The caller supplies exactly the inner CFMK header, beginning at its checksum word.
//! This parser neither locates nor authenticates an outer replay. In particular, outfit/hat
//! row hints are not stable identities: only qualified native getter results may resolve them.

use crate::engine::mii::inventory::{MiiClass, CHARINFO_LEN, LOW40};
use crate::engine::mii::lifetime::{Configuration, Tokens};
use crate::engine::mii::native_values::RowIds;

pub const HEADER_LEN: usize = 0x1bb0;
pub const VERSION: u32 = 0xd005;
pub const MAX_ENTRIES: usize = 8;
pub const MAX_ENCODED_LEN: usize = 2 + MAX_ENTRIES * 104;
const PLAYER_BLOCK: usize = 0x468;
const ENTRY_LEN: usize = 0x260;

/// An editor selection is temporary and belongs to one CPU lifetime and selection revision.
/// Matching class alone never certifies a replay configuration (including its unmasked bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub source: SelectionSource,
    pub generation: u64,
    pub revision: u64,
    pub target: Target,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionSource { Clip, NamedMacro }

/// One exact selection's terminal value result; stale/cross-source receipts do not apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionReceipt {
    pub selection: Selection,
    pub outcome: Result<(), &'static str>,
}
impl SelectionReceipt {
    pub fn for_selection(self, selection: Selection, generation: u64, revision: u64) -> Option<Result<(), &'static str>> {
        (self.selection == selection && selection.current(generation, revision)).then_some(self.outcome)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    kind: u32,
    configuration: Option<Configuration>,
}
impl Target {
    pub fn fighter(kind: u32) -> Self {
        Self {
            kind,
            configuration: None,
        }
    }
    pub fn mii(configuration: Configuration) -> Self {
        Self {
            kind: configuration.canonical().class as u32,
            configuration: Some(configuration),
        }
    }
    pub fn kind(self) -> u32 {
        self.kind
    }
    pub fn configuration(self) -> Option<Configuration> {
        self.configuration
    }
    pub fn ready(self, kind: u32, configuration: Option<Configuration>) -> bool {
        self.kind == kind && self.configuration == configuration
    }
}

impl Selection {
    pub fn current(self, generation: u64, revision: u64) -> bool {
        self.generation != 0
            && self.revision != 0
            && self.generation == generation
            && self.revision == revision
    }
    #[cfg(test)]
    pub fn ready(self, kind: u32, current: Option<Configuration>) -> bool {
        self.target.ready(kind, current)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    StoredEncoding,
    Length { actual: usize },
    Magic,
    Version { found: u32 },
    DeclaredLength { found: u16 },
    Tokens { entry: u8 },
    RowIdentity { entry: u8 },
    Configuration { entry: u8 },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// A source entry's immutable payload, not a native saved-file selector or current pool owner.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawMii {
    entry: u8,
    class: MiiClass,
    source_index: u16,
    charinfo: [u8; CHARINFO_LEN],
    tokens: Tokens,
}

impl core::fmt::Debug for RawMii {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawMii")
            .field("entry", &self.entry)
            .field("class", &self.class)
            .field("source_index", &self.source_index)
            .field("tokens", &self.tokens)
            .finish_non_exhaustive()
    }
}

impl RawMii {
    pub fn entry(&self) -> u8 {
        self.entry
    }
    pub fn class(&self) -> MiiClass {
        self.class
    }
    /// Informational only. Never use the old pool index to select a current role's record.
    pub fn source_index(&self) -> u16 {
        self.source_index
    }
    pub fn charinfo(&self) -> &[u8; CHARINFO_LEN] {
        &self.charinfo
    }
    pub fn tokens(&self) -> Tokens {
        self.tokens
    }

    /// The caller must obtain these exact tagged values at its qualified Training boundary.
    /// This pure check supplies no authority to call a native getter or publish a transaction.
    /// A hint cannot serve as a stable hash; wrong tag/index and zero hashes are refused.
    pub fn resolve(&self, rows: RowIds) -> Result<Configuration, Error> {
        let outfit = rows.outfit & LOW40;
        let hat = rows.hat & LOW40;
        rows.validate_tokens(self.tokens, outfit, hat)
            .map_err(|_| Error::RowIdentity { entry: self.entry })?;
        Configuration::from_observed(self.class, self.tokens, self.charinfo, rows)
            .map_err(|_| Error::Configuration { entry: self.entry })
    }
}

/// At most eight source entries, with no partial publication if any active Mii is malformed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct ReplayMiis {
    entries: [Option<RawMii>; MAX_ENTRIES],
}

impl ReplayMiis {
    pub fn parse_inner(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != HEADER_LEN {
            return Err(Error::Length {
                actual: bytes.len(),
            });
        }
        if &bytes[4..8] != b"CFMK" {
            return Err(Error::Magic);
        }
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(Error::Version { found: version });
        }
        // This native field is u16. The following two bytes are not part of its length.
        let length = half(bytes, 0x10);
        if usize::from(length) != HEADER_LEN {
            return Err(Error::DeclaredLength { found: length });
        }
        let mut entries = [None; MAX_ENTRIES];
        for (index, output) in entries.iter_mut().enumerate() {
            let row = &bytes[PLAYER_BLOCK + index * ENTRY_LEN..][..ENTRY_LEN];
            // Native's serializer skips +0xa==2 before the Mii CharInfo copy.
            if row[0xa] == 2 {
                continue;
            }
            let Some(class) = MiiClass::from_kind(u32::from(row[8])) else {
                continue;
            };
            let entry = index as u8;
            let tokens = Tokens {
                outfit: half(row, 0x1aa),
                hat: half(row, 0x1ac),
                colour: half(row, 0x1ae),
                voice: half(row, 0x1b0),
                specials: row[0x1b2..0x1b6].try_into().unwrap(),
            };
            if tokens.outfit == u16::MAX
                || tokens.hat == u16::MAX
                || tokens.colour > u16::from(u8::MAX)
                || tokens.voice > u16::from(u8::MAX)
                || tokens.specials.iter().any(|value| *value > 2)
            {
                return Err(Error::Tokens { entry });
            }
            *output = Some(RawMii {
                entry,
                class,
                source_index: half(row, 0x1a8),
                charinfo: row[0x150..0x150 + CHARINFO_LEN].try_into().unwrap(),
                tokens,
            });
        }
        Ok(Self { entries })
    }

    /// Compact owned companion payload. No outer replay offsets or native pointers are stored.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + self.entries().count() * 104);
        out.extend([1, self.entries().count() as u8]);
        for mii in self.entries() {
            out.extend([mii.entry, mii.class.kind() as u8]);
            out.extend(mii.source_index.to_le_bytes());
            out.extend(mii.charinfo);
            for token in [
                mii.tokens.outfit,
                mii.tokens.hat,
                mii.tokens.colour,
                mii.tokens.voice,
            ] {
                out.extend(token.to_le_bytes());
            }
            out.extend(mii.tokens.specials);
        }
        out
    }

    /// All restored values pass the same bounds as native parsing. Strict order prevents two
    /// encodings from ambiguously naming the same source entry; no partial result is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 2
            || bytes.len() > MAX_ENCODED_LEN
            || bytes[0] != 1
            || usize::from(bytes[1]) > MAX_ENTRIES
            || bytes.len() != 2 + usize::from(bytes[1]) * 104
        {
            return Err(Error::StoredEncoding);
        }
        let mut entries = [None; MAX_ENTRIES];
        let mut previous = None;
        // `as_chunks` (clippy 1.98 `chunks_exact_to_as_chunks`); the rows stay `&[u8]` below.
        for row in bytes[2..].as_chunks::<104>().0 {
            let row: &[u8] = row;
            let entry = row[0];
            if usize::from(entry) >= MAX_ENTRIES || previous.is_some_and(|p| p >= entry) {
                return Err(Error::StoredEncoding);
            }
            previous = Some(entry);
            let class = MiiClass::from_kind(u32::from(row[1])).ok_or(Error::StoredEncoding)?;
            let tokens = Tokens {
                outfit: half(row, 92),
                hat: half(row, 94),
                colour: half(row, 96),
                voice: half(row, 98),
                specials: row[100..104].try_into().unwrap(),
            };
            if tokens.outfit == u16::MAX
                || tokens.hat == u16::MAX
                || tokens.colour > u16::from(u8::MAX)
                || tokens.voice > u16::from(u8::MAX)
                || tokens.specials.iter().any(|v| *v > 2)
            {
                return Err(Error::Tokens { entry });
            }
            entries[usize::from(entry)] = Some(RawMii {
                entry,
                class,
                source_index: half(row, 2),
                charinfo: row[4..92].try_into().unwrap(),
                tokens,
            });
        }
        Ok(Self { entries })
    }

    pub fn entry(&self, entry: usize) -> Option<&RawMii> {
        self.entries.get(entry).and_then(Option::as_ref)
    }

    /// The observed fixture has one Mii of its class. Until repeated-class native entry
    /// mapping is qualified, class equality alone must not choose between those rows.
    pub fn for_track(&self, entry: usize, kind: u32) -> Result<&RawMii, &'static str> {
        let raw = self
            .entry(entry)
            .filter(|raw| raw.class() as u32 == kind)
            .ok_or("Capture entry has no matching Mii configuration")?;
        if self
            .entries()
            .filter(|other| other.class() == raw.class())
            .count()
            != 1
        {
            return Err("Capture has ambiguous same-class Mii entries");
        }
        Ok(raw)
    }

    /// Only the caller with the same immutable captured session can establish
    /// that its selected track is also unique. Native/live indices need not agree.
    pub fn unique_class(&self, kind: u32) -> Result<&RawMii, &'static str> {
        let mut matching = self.entries().filter(|raw| raw.class().kind() == kind);
        let raw = matching.next().ok_or("Capture has no Mii configuration for this class")?;
        if matching.next().is_some() { return Err("Capture has ambiguous same-class Mii entries"); }
        Ok(raw)
    }

    pub fn entries(&self) -> impl Iterator<Item = &RawMii> {
        self.entries.iter().flatten()
    }
}

impl From<ReplayMiis> for Vec<u8> {
    fn from(value: ReplayMiis) -> Self {
        value.to_bytes()
    }
}
impl TryFrom<Vec<u8>> for ReplayMiis {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::from_bytes(&value)
    }
}

fn half(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    

    fn header() -> Vec<u8> {
        let mut bytes = vec![0; HEADER_LEN];
        bytes[4..8].copy_from_slice(b"CFMK");
        bytes[8..12].copy_from_slice(&VERSION.to_le_bytes());
        bytes[0x10..0x12].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
        for index in 0..8 {
            bytes[0x468 + index * 0x260 + 0xa] = 2;
        }
        bytes
    }

    fn put_mii(bytes: &mut [u8], index: usize, kind: u8) {
        let at = 0x468 + index * 0x260;
        bytes[at + 8] = kind;
        bytes[at + 0xa] = 0;
        for n in 0..88 {
            bytes[at + 0x150 + n] = n as u8;
        }
        // Independent native window: index321,outfit1,hat0,colour4,voice37,2111.
        bytes[at + 0x1a8..at + 0x1b6]
            .copy_from_slice(&[65, 1, 1, 0, 0, 0, 4, 0, 37, 0, 1, 0, 0, 0]);
    }

    fn rows() -> RowIds {
        RowIds {
            outfit: 0x1300_011b_fef7_a5f7,
            hat: 0x5a00_000f_a9ef_a5ea,
        }
    }

    fn replay_selection_requires_exact_same_class_payload_and_current_lifetime() {
        let mut bytes = header();
        put_mii(&mut bytes, 0, 0x48);
        let original = ReplayMiis::parse_inner(&bytes)
            .unwrap()
            .entry(0)
            .unwrap()
            .resolve(rows())
            .unwrap();
        let selected = Selection {
            source: SelectionSource::Clip,
            generation: 7,
            revision: 11,
            target: Target::mii(original),
        };
        let refusal = SelectionReceipt { selection: selected, outcome: Err("CPU unchanged hat required") };
        assert_eq!(refusal.for_selection(selected, 7, 11), Some(Err("CPU unchanged hat required")));
        assert_eq!(refusal.for_selection(selected, 8, 11), None);
        assert_eq!(refusal.for_selection(selected, 7, 12), None);
        let named = Selection {source: SelectionSource::NamedMacro, ..selected};
        assert_eq!(refusal.for_selection(named, 7, 11), None);
        let ready = SelectionReceipt {selection: named, outcome: Ok(())};
        assert_eq!(ready.for_selection(named, 7, 11), Some(Ok(())));
        assert_eq!(ready.for_selection(selected, 7, 11), None);
        assert!(selected.current(7, 11));
        assert!(!selected.current(8, 11));
        assert!(!selected.current(7, 12));
        assert!(selected.ready(0x48, Some(original)));
        bytes[0x468 + 0x150] ^= 255;
        let same_canonical = ReplayMiis::parse_inner(&bytes)
            .unwrap()
            .entry(0)
            .unwrap()
            .resolve(rows())
            .unwrap();
        assert_eq!(original.canonical(), same_canonical.canonical());
        assert!(!selected.ready(0x48, Some(same_canonical)));
        bytes[0x468 + 0x1b2] = 2;
        let same_class = ReplayMiis::parse_inner(&bytes)
            .unwrap()
            .entry(0)
            .unwrap()
            .resolve(rows())
            .unwrap();
        assert!(!selected.ready(0x48, Some(same_class)));
        let request = crate::engine::mii::ordinary_cpu::Request::from_replay(selected).unwrap();
        assert_eq!(request.replay(), Some(selected));
        assert_eq!(request.choice(), None);
        assert!(crate::engine::mii::ordinary_cpu::Request::from_replay(Selection {
            revision: 0,
            ..selected
        })
        .is_none());
        assert!(crate::engine::mii::ordinary_cpu::Request::from_replay(Selection {
            target: Target::fighter(0x48),
            ..selected
        })
        .is_none());
        assert!(!Target::fighter(0).ready(0, Some(original)));
        assert!(Target::fighter(0).ready(0, None));
    }

    fn replay_header_refuses_unsupported_boundaries_and_any_malformed_active_mii() {
        let valid = header();
        for length in [0, HEADER_LEN - 1, HEADER_LEN + 1] {
            assert_eq!(
                ReplayMiis::parse_inner(&vec![0; length]).unwrap_err(),
                Error::Length { actual: length }
            );
        }
        let mut bad = valid.clone();
        bad[4] = 0;
        assert_eq!(ReplayMiis::parse_inner(&bad).unwrap_err(), Error::Magic);
        bad = valid.clone();
        bad[8] = 4;
        assert_eq!(
            ReplayMiis::parse_inner(&bad).unwrap_err(),
            Error::Version { found: 0xd004 }
        );
        bad = valid.clone();
        bad[0x10] -= 1;
        assert_eq!(
            ReplayMiis::parse_inner(&bad).unwrap_err(),
            Error::DeclaredLength { found: 0x1baf }
        );
        for (offset, value) in [
            (0x1aa, [255, 255]),
            (0x1ac, [255, 255]),
            (0x1ae, [0, 1]),
            (0x1b0, [0, 1]),
            (0x1b2, [3, 0]),
        ] {
            let mut bad = valid.clone();
            put_mii(&mut bad, 0, 0x48);
            put_mii(&mut bad, 7, 0x49);
            let at = 0x468 + 7 * 0x260 + offset;
            bad[at..at + 2].copy_from_slice(&value);
            assert_eq!(
                ReplayMiis::parse_inner(&bad).unwrap_err(),
                Error::Tokens { entry: 7 }
            );
        }
    }

    fn replay_companion_codec_retains_payload_and_rejects_stale_or_ambiguous_encoding() {
        let mut bytes = header();
        put_mii(&mut bytes, 0, 0x48);
        put_mii(&mut bytes, 7, 0x49);
        let raw = ReplayMiis::parse_inner(&bytes).unwrap();
        let encoded = raw.to_bytes();
        assert_eq!(encoded.len(), 210);
        assert_eq!(&encoded[..6], &[1, 2, 0, 0x48, 65, 1]);
        assert_eq!(ReplayMiis::from_bytes(&encoded).unwrap(), raw);
        let json = serde_json::to_vec(&raw).unwrap();
        assert_eq!(serde_json::from_slice::<ReplayMiis>(&json).unwrap(), raw);
        assert_eq!(
            ReplayMiis::parse_inner(&header()).unwrap().to_bytes(),
            [1, 0]
        );
        for mutate in [0, 1, 106, 107, 206] {
            let mut bad = encoded.clone();
            bad[mutate] = match mutate {
                0 => 2,
                1 => 9,
                106 => 0,
                107 => 0,
                206 => 3,
                _ => unreachable!(),
            };
            assert!(ReplayMiis::from_bytes(&bad).is_err());
            assert!(
                serde_json::from_slice::<ReplayMiis>(&serde_json::to_vec(&bad).unwrap()).is_err()
            );
        }
        assert!(ReplayMiis::from_bytes(&encoded[..209]).is_err());
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("replay_selection_requires_exact_same_class_payload_and_current_lifetime", replay_selection_requires_exact_same_class_payload_and_current_lifetime as fn()),
            ("replay_header_refuses_unsupported_boundaries_and_any_malformed_active_mii", replay_header_refuses_unsupported_boundaries_and_any_malformed_active_mii as fn()),
            ("replay_companion_codec_retains_payload_and_rejects_stale_or_ambiguous_encoding", replay_companion_codec_retains_payload_and_rejects_stale_or_ambiguous_encoding as fn()),
        ]);
    }
    }
}
