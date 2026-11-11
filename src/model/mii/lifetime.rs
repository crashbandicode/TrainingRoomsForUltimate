//! Owned Mii override transactions. No native calls, pointers, locks, files or admission wiring.
//! A numeric Binding is comparison identity, never dereference authority. The adapter must supply
//! a fresh, fully validated observation on the existing native callback/INSTALL boundary.

use crate::engine::mii::inventory::{
    Appearance, Canonical, MiiClass, MiiConfig, TaggedId, CHARINFO_LEN, CHARINFO_MASK_END,
    LOW40, TAG_CHARA, TAG_HAT, TAG_OUTFIT,
};
use crate::engine::mii::selection::{Choice, Role};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub outfit: u16,
    pub hat: u16,
    pub colour: u16,
    pub voice: u16,
    pub specials: [u8; 4],
}

/// Exact unmasked record bytes are separate from the catalog's masked canonical identity.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "StoredConfiguration", into = "StoredConfiguration")]
pub struct Configuration {
    class: MiiClass,
    hashes: [u64; 3],
    charinfo: [u8; CHARINFO_LEN],
    tokens: Tokens,
}

// Same value validation as the existing preset library. Native row hints remain
// hints; the runtime still revalidates them against the saved stable IDs.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConfiguration {
    canonical: Canonical,
    charinfo: Vec<u8>,
    tags: [u64; 3],
}
impl From<Configuration> for StoredConfiguration {
    fn from(value: Configuration) -> Self {
        let c = value.canonical();
        let tag = |kind, row, hash| (u64::from(kind) << 56) | (u64::from(row) << 40) | hash;
        Self { canonical: c, charinfo: value.charinfo.to_vec(), tags: [
            tag(TAG_OUTFIT, value.tokens.outfit, c.appearance.outfit),
            tag(TAG_HAT, value.tokens.hat, c.appearance.hat),
            tag(TAG_CHARA, 0u16, c.appearance.chara),
        ] }
    }
}
impl TryFrom<StoredConfiguration> for Configuration {
    type Error = &'static str;
    fn try_from(value: StoredConfiguration) -> Result<Self, Self::Error> {
        let charinfo = value.charinfo.try_into().map_err(|_| "Invalid Mii appearance length")?;
        Self::from_saved(&MiiConfig { slot_id: 0, slot_index: 0, name: String::new(),
            charinfo, raw_tags: value.tags.map(crate::engine::mii::inventory::TaggedId),
            canonical: value.canonical, copies: Vec::new() })
            .map_err(|_| "Mii appearance or stable resource identity differs")
    }
}

impl core::fmt::Debug for Configuration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Configuration")
            .field("class", &self.class)
            .field("canonical_hash", &self.canonical().hash())
            .field("tokens", &self.tokens)
            .finish_non_exhaustive()
    }
}

impl Configuration {
    /// Own observed values without inventing a saved-file identity. The caller must
    /// qualify the native row getters and revalidate its source after those calls.
    /// This constructor supplies no native read, write, or lifetime authority.
    pub fn from_observed(
        class: MiiClass,
        tokens: Tokens,
        charinfo: [u8; CHARINFO_LEN],
        rows: crate::engine::mii::native_values::RowIds,
    ) -> Result<Self, Error> {
        let outfit = rows.outfit & LOW40;
        let hat = rows.hat & LOW40;
        rows.validate_tokens(tokens, outfit, hat)
            .map_err(|_| Error::InvalidConfiguration)?;
        let colour = u8::try_from(tokens.colour).map_err(|_| Error::InvalidConfiguration)?;
        let voice = u8::try_from(tokens.voice).map_err(|_| Error::InvalidConfiguration)?;
        let mut masked = charinfo;
        masked[..CHARINFO_MASK_END].fill(0);
        Self::from_saved(&MiiConfig {
            slot_id: 0,
            slot_index: 0,
            name: String::new(),
            charinfo,
            raw_tags: [
                TaggedId(rows.outfit),
                TaggedId(rows.hat),
                TaggedId((u64::from(TAG_CHARA) << 56) | class.chara_hash40()),
            ],
            canonical: Canonical {
                class,
                appearance: Appearance {
                    charinfo_masked: masked,
                    specials: tokens.specials,
                    colour,
                    voice,
                    outfit,
                    hat,
                    chara: class.chara_hash40(),
                },
            },
            copies: Vec::new(),
        })
    }

    pub fn from_saved(config: &MiiConfig) -> Result<Self, Error> {
        let c = config.canonical;
        let [outfit, hat, chara] = config.raw_tags;
        let mut masked = config.charinfo;
        masked[..CHARINFO_MASK_END].fill(0);
        if Canonical::from_bytes(&c.to_bytes()) != Ok(c)
            || masked != c.appearance.charinfo_masked
            || outfit.tag() != TAG_OUTFIT
            || hat.tag() != TAG_HAT
            || chara.tag() != TAG_CHARA
            || outfit.hash40() != c.appearance.outfit
            || hat.hash40() != c.appearance.hat
            || chara.hash40() != c.appearance.chara
            || outfit.row_hint() == u16::MAX
            || hat.row_hint() == u16::MAX
        {
            return Err(Error::InvalidConfiguration);
        }
        Ok(Self {
            class: c.class,
            hashes: [c.appearance.outfit, c.appearance.hat, c.appearance.chara],
            charinfo: config.charinfo,
            tokens: Tokens {
                outfit: outfit.row_hint(),
                hat: hat.row_hint(),
                colour: u16::from(c.appearance.colour),
                voice: u16::from(c.appearance.voice),
                specials: c.appearance.specials,
            },
        })
    }

    pub fn canonical(&self) -> Canonical {
        let mut masked = self.charinfo;
        masked[..CHARINFO_MASK_END].fill(0);
        Canonical {
            class: self.class,
            appearance: Appearance {
                charinfo_masked: masked,
                specials: self.tokens.specials,
                colour: self.tokens.colour as u8,
                voice: self.tokens.voice as u8,
                outfit: self.hashes[0],
                hat: self.hashes[1],
                chara: self.hashes[2],
            },
        }
    }
    pub fn charinfo(&self) -> &[u8; CHARINFO_LEN] {
        &self.charinfo
    }
    pub fn tokens(&self) -> Tokens {
        self.tokens
    }
}


/// Values copied from the current native binding. These do not pin native objects or prove
/// exclusivity. All owners, mirrors and record contents still need fresh adapter validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub epoch: u64,
    pub owner: u32,
    pub entry: usize,
    pub row: usize,
    pub holder: usize,
    pub manager: usize,
    pub vector_begin: usize,
    pub vector_end: usize,
    pub record_index: u16,
}

impl Binding {
    fn valid(self) -> bool {
        let aligned = |p: usize| p != 0 && p.is_multiple_of(8);
        [
            self.entry,
            self.row,
            self.holder,
            self.manager,
            self.vector_begin,
        ]
        .into_iter()
        .all(aligned)
            && self.vector_end > self.vector_begin
            && (self.vector_end - self.vector_begin).is_multiple_of(0x68)
            && usize::from(self.record_index) < (self.vector_end - self.vector_begin) / 0x68
    }
}

/// The adapter constructs this only after exact live row/record/mirror comparison, including
/// rederivation and compare inside the native pool lock immediately before a write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    pub binding: Binding,
    pub configuration: Configuration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedOwner,
    InvalidBinding,
    InvalidConfiguration,
    BindingChanged,
    UnexpectedConfiguration,
    ChoiceMismatch,
    DifferentClass,
    DifferentHat,
    OriginRequiresRestore,
    Busy,
    Refused,
    Departed,
    StaleTicket,
    WrongPhase,
    RestoreRequired,
    SequenceExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Prepared,
    Loading,
    Refused,
    Departed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Apply,
    Restore,
}

/// Separate from native InstallToken. It authorizes no native access or gate release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    binding: Binding,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transaction {
    ticket: Ticket,
    before: Configuration,
    after: Configuration,
    operation: Operation,
}

impl Transaction {
    pub fn ticket(self) -> Ticket {
        self.ticket
    }
    pub fn binding(self) -> Binding {
        self.ticket.binding
    }
    pub fn before(self) -> Configuration {
        self.before
    }
    pub fn after(self) -> Configuration {
        self.after
    }
    pub fn operation(self) -> Operation {
        self.operation
    }
}

/// One value per native owner lifetime; deliberately not Clone. Capture is allowed only from a
/// freshly proven existing CSS Mii in the qualified native Training role/index. This pure API does not enable native Mii admission.
pub struct MiiOverride {
    binding: Binding,
    original: Configuration,
    expected: Configuration,
    phase: Phase,
    next_sequence: u64,
    pending: Option<Transaction>,
}

impl MiiOverride {
    pub fn capture(role: Role, original: Observed) -> Result<Self, Error> {
        let owner = match role {
            Role::Player => 0,
            Role::Cpu => 1,
        };
        if original.binding.owner != owner || original.binding.record_index != owner as u16 {
            return Err(Error::UnsupportedOwner);
        }
        if !original.binding.valid() {
            return Err(Error::InvalidBinding);
        }
        Ok(Self {
            binding: original.binding,
            original: original.configuration,
            expected: original.configuration,
            phase: Phase::Ready,
            next_sequence: 1,
            pending: None,
        })
    }

    pub fn original(&self) -> Configuration {
        self.original
    }
    pub fn expected(&self) -> Configuration {
        self.expected
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn pending_ticket(&self) -> Option<Ticket> {
        self.pending.map(|p| p.ticket)
    }

    fn ready(&self, live: Observed) -> Result<(), Error> {
        match self.phase {
            Phase::Ready => {}
            Phase::Prepared | Phase::Loading => return Err(Error::Busy),
            Phase::Refused => return Err(Error::Refused),
            Phase::Departed => return Err(Error::Departed),
        }
        self.matches(live, self.expected)
    }

    fn matches(&self, live: Observed, expected: Configuration) -> Result<(), Error> {
        if live.binding != self.binding {
            return Err(Error::BindingChanged);
        }
        if live.configuration != expected {
            return Err(Error::UnexpectedConfiguration);
        }
        Ok(())
    }

    fn prepare(
        &mut self,
        after: Configuration,
        operation: Operation,
    ) -> Result<Transaction, Error> {
        let sequence = self.next_sequence;
        self.next_sequence = sequence.checked_add(1).ok_or(Error::SequenceExhausted)?;
        let transaction = Transaction {
            ticket: Ticket {
                binding: self.binding,
                sequence,
            },
            before: self.expected,
            after,
            operation,
        };
        self.pending = Some(transaction);
        self.phase = Phase::Prepared;
        Ok(transaction)
    }

    /// None means the selected canonical configuration is already current, after exact live
    /// validation. Changing representative saved-file metadata alone does not cause a reload.
    pub fn apply(
        &mut self,
        live: Observed,
        choice: Choice,
        target: Configuration,
    ) -> Result<Option<Transaction>, Error> {
        self.ready(live)?;
        let canonical = target.canonical();
        if choice.canonical != canonical {
            return Err(Error::ChoiceMismatch);
        }
        if target.class != self.original.class {
            return Err(Error::DifferentClass);
        }
        if target.tokens.hat != self.original.tokens.hat
            || target.hashes[1] != self.original.hashes[1]
        {
            return Err(Error::DifferentHat);
        }
        if canonical == self.original.canonical() {
            return Err(Error::OriginRequiresRestore);
        }
        if canonical == self.expected.canonical() {
            return Ok(None);
        }
        self.prepare(target, Operation::Apply).map(Some)
    }

    /// Slot0 or a pending kind change must restore this exact captured original while the old
    /// same-class binding still exists. No original is reconstructed from current preferences.
    pub fn restore(&mut self, live: Observed) -> Result<Option<Transaction>, Error> {
        self.ready(live)?;
        if self.expected == self.original {
            return Ok(None);
        }
        self.prepare(self.original, Operation::Restore).map(Some)
    }

    pub fn can_change_kind_or_return_slot0(&self, live: Observed) -> Result<(), Error> {
        self.ready(live)?;
        if self.expected != self.original {
            return Err(Error::RestoreRequired);
        }
        Ok(())
    }

    fn transaction(&self, ticket: Ticket) -> Result<Transaction, Error> {
        self.pending
            .filter(|p| p.ticket == ticket)
            .ok_or(Error::StaleTicket)
    }

    /// Called after obtaining the existing InstallToken, before releasing Rust state and calling
    /// native code. The adapter must revalidate again inside the native lock before writing.
    pub fn submit(&mut self, ticket: Ticket, live: Observed) -> Result<Transaction, Error> {
        let transaction = self.transaction(ticket)?;
        if self.phase != Phase::Prepared {
            return Err(Error::WrongPhase);
        }
        self.matches(live, transaction.before)?;
        self.phase = Phase::Loading;
        Ok(transaction)
    }

    /// Valid only before submit, when no native write/call could have happened.
    pub fn cancel_prepared(&mut self, ticket: Ticket) -> Result<(), Error> {
        self.transaction(ticket)?;
        if self.phase != Phase::Prepared {
            return Err(Error::WrongPhase);
        }
        self.pending = None;
        self.phase = Phase::Ready;
        Ok(())
    }

    /// The native adapter has proved that submit copied NO native byte and issued NO INSTALL.
    /// Recover only that unwritten transaction; expected/original are unchanged. Any committed
    /// byte or native install forbids this API and requires completion or actual destruction.
    pub fn abandon_unwritten(&mut self, ticket: Ticket) -> Result<(), Error> {
        self.transaction(ticket)?;
        if self.phase != Phase::Loading {
            return Err(Error::WrongPhase);
        }
        self.pending = None;
        self.phase = Phase::Ready;
        Ok(())
    }

    /// Full readiness and fresh exact target bytes/mirrors must already be established. A bad
    /// completion retains the transaction and disables writes; no timeout API exists.
    pub fn complete(&mut self, ticket: Ticket, live: Observed) -> Result<Operation, Error> {
        let transaction = self.transaction(ticket)?;
        if self.phase != Phase::Loading {
            return Err(Error::WrongPhase);
        }
        if let Err(error) = self.matches(live, transaction.after) {
            self.phase = Phase::Refused;
            return Err(error);
        }
        self.expected = transaction.after;
        self.pending = None;
        self.phase = Phase::Ready;
        Ok(transaction.operation)
    }

    /// Native adapter refusal never discards a possibly active installation.
    pub fn refuse(&mut self, ticket: Ticket) -> Result<(), Error> {
        self.transaction(ticket)?;
        if self.phase == Phase::Departed {
            return Err(Error::Departed);
        }
        self.phase = Phase::Refused;
        Ok(())
    }

    /// Actual scene departure invalidates all intent. No restoration command, token release or
    /// native access occurs. A pending ticket stays available for exact completion/destruction.
    pub fn depart(&mut self) {
        self.phase = Phase::Departed;
    }

    /// Receipt bookkeeping only. The adapter may call this only after actual native completion
    /// or destruction (or proof submit never wrote/called); never because time elapsed.
    /// Finishing the separate native InstallToken remains the adapter's responsibility.
    pub fn settle_terminal(&mut self, ticket: Ticket) -> Result<(), Error> {
        self.transaction(ticket)?;
        if !matches!(self.phase, Phase::Departed | Phase::Refused) {
            return Err(Error::WrongPhase);
        }
        self.pending = None;
        Ok(())
    }
}

