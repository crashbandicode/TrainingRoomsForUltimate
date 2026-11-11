//! Value-only checks for saved-original same-class live adapters.
//!
//! Inputs are owned copies, never Rust references into native-owned memory. These checks do not
//! acquire a lock, pin G/FE, establish a callback lifetime, or authorize a native write. Native glue
//! must bind the current FE and rederive holder/G/vector/index inside its qualified pool lock.

use crate::engine::mii::inventory::{MiiClass, CHARINFO_LEN};
use crate::engine::mii::lifetime::{Binding, Configuration, Tokens, Transaction};

pub const RECORD_LEN: usize = 0x68;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Scope,
    OtherMii,
    Binding,
    Vector,
    SavedRow,
    FighterMirror,
    InformationMirror,
    OwnerMirror,
    RowIdentity,
    HatChanged,
    RecordChanged,
}

/// Actual u64 results of the fingerprinted native getters, not asset field/index accessors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowIds {
    pub outfit: u64,
    pub hat: u64,
}

impl RowIds {
    pub fn validate(self, configuration: Configuration) -> Result<(), Error> {
        let tokens = configuration.tokens();
        let appearance = configuration.canonical().appearance;
        self.validate_tokens(tokens, appearance.outfit, appearance.hat)
    }

    pub fn validate_tokens(
        self,
        tokens: Tokens,
        outfit_hash: u64,
        hat_hash: u64,
    ) -> Result<(), Error> {
        for (tag, hint, hash, actual) in [
            (0x13u64, tokens.outfit, outfit_hash, self.outfit),
            (0x5au64, tokens.hat, hat_hash, self.hat),
        ] {
            if hint == u16::MAX
                || hash == 0
                || hash & !0xff_ffff_ffff != 0
                || actual != ((tag << 56) | (u64::from(hint) << 40) | hash)
            {
                return Err(Error::RowIdentity);
            }
        }
        Ok(())
    }
}

/// E, FE, D and O copied values. D has FOUR halfword mirrors; D+0x7a is not voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mirrors {
    pub index: u16,
    pub tokens: Tokens,
    pub fe: [u16; 5],
    pub d: [u16; 4],
    pub d_specials: [u32; 4],
    pub owner_index: u16,
    pub owner: [u32; 4],
}

impl Mirrors {
    /// Exact owned byte windows: E+1a8..1b6, FE+46..50, D+72..7a, D+390..3a0,
    /// O+68..7c. O+6a padding is ignored, not reinterpreted as a token.
    pub fn decode(
        e: [u8; 14],
        fe: [u8; 10],
        d: [u8; 8],
        d_specials: [u8; 16],
        owner: [u8; 20],
    ) -> Self {
        Self {
            index: half(&e, 0),
            tokens: Tokens {
                outfit: half(&e, 2),
                hat: half(&e, 4),
                colour: half(&e, 6),
                voice: half(&e, 8),
                specials: [e[10], e[11], e[12], e[13]],
            },
            fe: core::array::from_fn(|i| half(&fe, i * 2)),
            d: core::array::from_fn(|i| half(&d, i * 2)),
            d_specials: core::array::from_fn(|i| word(&d_specials, i * 4)),
            owner_index: half(&owner, 0),
            owner: core::array::from_fn(|i| word(&owner, 4 + i * 4)),
        }
    }

    pub fn validate(self, index: u16, expected: Tokens) -> Result<(), Error> {
        if self.index != index
            || self.tokens != expected
            || expected.specials.iter().any(|s| *s > 2)
        {
            return Err(Error::SavedRow);
        }
        if self.fe
            != [
                index,
                expected.outfit,
                expected.hat,
                expected.colour,
                expected.voice,
            ]
        {
            return Err(Error::FighterMirror);
        }
        if self.d != [index, expected.outfit, expected.hat, expected.colour]
            || self.d_specials != expected.specials.map(u32::from)
        {
            return Err(Error::InformationMirror);
        }
        if self.owner_index != index
            || self.owner
                != [
                    expected.outfit,
                    expected.hat,
                    expected.colour,
                    expected.voice,
                ]
                .map(u32::from)
        {
            return Err(Error::OwnerMirror);
        }
        Ok(())
    }
}

fn half(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Exact expected identity, including pool endpoints, not merely the old record count. The
/// returned address is a numeric value for a native adapter; it is NOT dereference authority.
pub fn record_address(expected: Binding, fresh: Binding) -> Result<usize, Error> {
    if fresh != expected {
        return Err(Error::Binding);
    }
    let aligned = |p: usize| p != 0 && p.is_multiple_of(8);
    if ![
        fresh.entry,
        fresh.row,
        fresh.holder,
        fresh.manager,
        fresh.vector_begin,
    ]
    .into_iter()
    .all(aligned)
    {
        return Err(Error::Vector);
    }
    let length = fresh
        .vector_end
        .checked_sub(fresh.vector_begin)
        .ok_or(Error::Vector)?;
    if !length.is_multiple_of(RECORD_LEN) || usize::from(fresh.record_index) >= length / RECORD_LEN
    {
        return Err(Error::Vector);
    }
    fresh
        .vector_begin
        .checked_add(usize::from(fresh.record_index) * RECORD_LEN)
        .ok_or(Error::Vector)
}

/// A freshly copied current owner map is required, including inactive native helpers. None
/// means a positively absent entry, not an unreadable or unchecked manager lookup.
pub fn qualified_scope(
    fresh: Binding,
    saved_row_kind: u32,
    owner_kinds: [Option<u32>; 8],
) -> Result<(), Error> {
    if saved_row_kind != 0x49 { return Err(Error::Scope); }
    qualified_saved_scope(fresh, saved_row_kind, owner_kinds)
}

/// Same-class saved-original scope; no ordinary-origin or second Mii admission.
pub fn qualified_saved_scope(
    fresh: Binding, saved_row_kind: u32, owner_kinds: [Option<u32>; 8],
) -> Result<(), Error> {
    if fresh.owner != 0 || MiiClass::from_kind(saved_row_kind).is_none()
        || owner_kinds[0] != Some(saved_row_kind) {
        return Err(Error::Scope);
    }
    if owner_kinds[1..]
        .iter()
        .flatten()
        .any(|kind| (0x48..=0x4a).contains(kind))
    {
        return Err(Error::OtherMii);
    }
    Ok(())
}

/// Copied current associations for the narrow CPU diagnostic. Addresses are identities only.
/// This is not native dereference authority or production Mii admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuOwner {
    pub owner: u32,
    pub kind: u32,
    pub selector: u8,
    pub index: u16,
    pub entry: usize,
    pub row: usize,
    pub information: usize,
    pub object: usize,
    pub alias: u8,
}

/// Origin01 #284/#297/#298: saved Sword CPU1 and native inactive Swords2/3,
/// distinct current objects and indices. P1 is the explicitly qualified ordinary Luigi fixture.
/// Ordinary P1's zero Mii index is observed, not treated as a claim on record0.
pub fn qualified_cpu_scope(
    owners: &[CpuOwner; 4],
    other_entries_present: bool,
) -> Result<(), Error> {
    qualified_saved_cpu_scope(owners, other_entries_present, 0x49)
}

/// Retains exact owner/index/alias uniqueness and Luigi P1; all three CPU/helper kinds
/// must equal the captured saved class. Inactive helpers are never available records.
pub fn qualified_saved_cpu_scope(
    owners: &[CpuOwner; 4], other_entries_present: bool, kind: u32,
) -> Result<(), Error> {
    if MiiClass::from_kind(kind).is_none() || other_entries_present {
        return Err(Error::Scope);
    }
    for (id, owner) in owners.iter().enumerate() {
        if owner.owner != id as u32
            || owner.selector != id as u8
            || owner.kind != if id == 0 { 9 } else { kind }
            || (id != 0 && owner.index != id as u16)
            || owner.alias != 0
            || [owner.entry, owner.row, owner.information, owner.object]
                .iter()
                .any(|p| *p == 0 || !p.is_multiple_of(8))
        {
            return Err(Error::Scope);
        }
        for previous in &owners[..id] {
            if owner.entry == previous.entry
                || owner.row == previous.row
                || owner.information == previous.information
                || owner.object == previous.object
            {
                return Err(Error::Binding);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuHelper {
    pub owner: CpuOwner,
    pub mirrors: Mirrors,
    pub record: [u8; RECORD_LEN],
    pub body: usize,
    pub body_id: u32,
}

/// Helper configuration and numeric ownership are invariant across the target's reload.
/// Shared-pointer words are compared as bytes, never dereferenced or reference-counted here.
pub fn preserved_cpu_helpers(
    expected: &[CpuHelper; 2],
    current: &[CpuHelper; 2],
) -> Result<(), Error> {
    if expected != current {
        return Err(Error::RecordChanged);
    }
    Ok(())
}

/// An owned update payload. The only mutation method below modifies an owned record COPY.
/// In particular, it cannot lock G, write native tokens, call INSTALL or complete a transaction.
#[derive(Clone, Copy)]
pub struct RecordUpdate {
    before: [u8; CHARINFO_LEN],
    after: [u8; CHARINFO_LEN],
    tokens: Tokens,
}

impl RecordUpdate {
    /// Called with freshly copied values from the qualified critical section. All validation
    /// precedes publication. Native code must perform the same checks on its current identities
    /// and exact record immediately before copying 0x58 bytes, while still holding that lock.
    pub fn prepare(
        transaction: Transaction,
        fresh: Binding,
        mirrors: Mirrors,
        rows: [RowIds; 2],
        record: &[u8; RECORD_LEN],
    ) -> Result<Self, Error> {
        if transaction.before().canonical().class != MiiClass::Swordfighter {
            return Err(Error::Scope);
        }
        Self::prepare_for_role(crate::engine::mii::selection::Role::Player, transaction, fresh, mirrors, rows, record)
    }

    /// Value kernel for the role adapter after its complete scope/preservation validation.
    /// The public legacy entry point above retains its original P1-only scope.
    pub(crate) fn prepare_for_role(
        role: crate::engine::mii::selection::Role,
        transaction: Transaction,
        fresh: Binding,
        mirrors: Mirrors,
        rows: [RowIds; 2],
        record: &[u8; RECORD_LEN],
    ) -> Result<Self, Error> {
        let before = transaction.before();
        let after = transaction.after();
        let owner = match role { crate::engine::mii::selection::Role::Player => 0, crate::engine::mii::selection::Role::Cpu => 1 };
        if fresh.owner != owner || fresh.record_index != owner as u16
            || before.canonical().class != after.canonical().class
        {
            return Err(Error::Scope);
        }
        record_address(transaction.binding(), fresh)?;
        mirrors.validate(fresh.record_index, before.tokens())?;
        rows[0].validate(before)?;
        rows[1].validate(after)?;
        if before.tokens().hat != after.tokens().hat
            || before.canonical().appearance.hat != after.canonical().appearance.hat
        {
            return Err(Error::HatChanged);
        }
        Self::compare_record(
            *before.charinfo(),
            *after.charinfo(),
            after.tokens(),
            record,
        )
    }

    /// Exact copied-record kernel shared by the transaction adapter and the existing diagnostic.
    /// This alone does not establish scope, row identity, mirrors, a lock or native write authority.
    pub fn compare_record(
        before: [u8; CHARINFO_LEN],
        after: [u8; CHARINFO_LEN],
        tokens: Tokens,
        record: &[u8; RECORD_LEN],
    ) -> Result<Self, Error> {
        if record[..CHARINFO_LEN] != before {
            return Err(Error::RecordChanged);
        }
        Ok(Self {
            before,
            after,
            tokens,
        })
    }

    pub fn tokens(self) -> Tokens {
        self.tokens
    }
    pub fn charinfo(&self) -> &[u8; CHARINFO_LEN] {
        &self.after
    }

    /// Fault-atomic value operation for host validation. Never create this mutable Rust reference
    /// from concurrent native memory. The shared_ptr data/control bytes +58..68 are untouched.
    pub fn apply_to_copy(self, record: &mut [u8; RECORD_LEN]) -> Result<(), Error> {
        if record[..CHARINFO_LEN] != self.before {
            return Err(Error::RecordChanged);
        }
        record[..CHARINFO_LEN].copy_from_slice(&self.after);
        Ok(())
    }
}

