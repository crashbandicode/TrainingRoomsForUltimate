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

