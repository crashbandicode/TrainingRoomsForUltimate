//! Allocation-free source lifetime bookkeeping. The caller serializes access and
//! verifies native ownership/readiness; this module never selects native slots.
//! Keep one book alive across scene changes. Tokens belong to that book, must not
//! cross book instances, and must be retained by native callbacks before loading.
//! Cancelling UI/queue intent only discards its epoch/token, never ends a load.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceKey {
    pub kind: u32,
    pub costume: u32,
}

/// Numeric native identity, not a pointer or proof of slot ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeEntry(pub u64);

/// Numeric native lifetime captured only after full load completion.
/// No saved pointer is dereferenced through this value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeIdentity {
    pub slot: u32,
    pub entry: usize,
    pub kind: u32,
    pub costume: u32,
    pub body_id: u32,
    /// Exact completed native lifetime, including every grouped body. Numeric
    /// identities only; consumers must reacquire pointers in their callback.
    pub bodies: [(usize, u32); 3],
    pub descriptor: usize,
    pub resource: usize,
}

/// Existing native Training reset dispatch covers all four visible owners.
/// Managed seats still require their current publication in the adapter.
pub fn training_portrait_owner(owner: u64, training: bool) -> bool {
    training && owner <= 3
}

/// Caller-defined category and identity (copy body, live item, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Holder {
    pub category: u32,
    pub id: u64,
}

/// Stamp pending intent and check it before performing any consumer operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneEpoch(u64);

/// Retain this full identity for destruction, including at a reused native entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceToken {
    slot: usize,
    entry: NativeEntry,
    generation: u64,
    epoch: SceneEpoch,
}

impl SourceToken {
    pub fn slot(self) -> usize { self.slot }
    pub fn entry(self) -> NativeEntry { self.entry }
    pub fn epoch(self) -> SceneEpoch { self.epoch }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadToken(SourceToken);

impl LoadToken {
    pub fn source(self) -> SourceToken { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadyToken(SourceToken);

impl ReadyToken {
    pub fn source(self) -> SourceToken { self.0 }
}

/// Retained-body ownership transfer, distinct from the native resource lifetime.
/// A fresh serial prevents an old deactivation callback from readopting a later
/// promotion of the same body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromotionToken {
    source: SourceToken,
    serial: u64,
}

impl PromotionToken {
    pub fn source(self) -> SourceToken { self.source }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseToken {
    index: usize,
    serial: u64,
    source: SourceToken,
    holder: Holder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Empty, Loading, Ready, Failed, Departed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSlot,
    Occupied,
    EntryAlreadyRegistered,
    Pinned,
    StaleToken,
    NotReady,
    Departed,
    LeaseCapacity,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    generation: u64,
    source: Option<SourceToken>,
    key: Option<SourceKey>,
    phase: Phase,
    promoted: bool,
    promotion_serial: u64,
    presentation: Option<u64>,
}

impl Slot {
    const EMPTY: Self = Self {
        generation: 0, source: None, key: None, phase: Phase::Empty,
        promoted: false, promotion_serial: 0, presentation: None,
    };
}

/// Fixed bookkeeping capacity, not permission to use any native entry number.
pub struct SourceBook<const SOURCES: usize, const LEASES: usize> {
    slots: [Slot; SOURCES],
    leases: [Option<LeaseToken>; LEASES],
    epoch: SceneEpoch,
    lease_serial: u64,
    presentation_serial: u64,
}

impl<const SOURCES: usize, const LEASES: usize> SourceBook<SOURCES, LEASES> {
    pub const fn new() -> Self {
        Self {
            slots: [Slot::EMPTY; SOURCES], leases: [None; LEASES],
            epoch: SceneEpoch(0), lease_serial: 0, presentation_serial: 0,
        }
    }

    pub fn epoch(&self) -> SceneEpoch { self.epoch }

    pub fn is_current(&self, epoch: SceneEpoch) -> bool { self.epoch == epoch }

    pub fn phase(&self, slot: usize) -> Result<Phase, Error> {
        Ok(self.slot(slot)?.phase)
    }

    pub fn key(&self, slot: usize) -> Result<Option<SourceKey>, Error> {
        Ok(self.slot(slot)?.key)
    }

    /// Lookup only; loading/departed entries cannot supply ready tokens.
    pub fn ready(&self, slot: usize) -> Result<ReadyToken, Error> {
        let state = self.slot(slot)?;
        if state.phase != Phase::Ready || state.promoted { return Err(Error::NotReady); }
        Ok(ReadyToken(state.source.ok_or(Error::StaleToken)?))
    }

    /// Read-only advisory; begin_install rechecks before changing anything.
    pub fn can_replace(&self, slot: usize) -> Result<bool, Error> {
        let state = self.slot(slot)?;
        if state.promoted { return Ok(false); }
        Ok(match state.phase {
            Phase::Empty => true,
            Phase::Ready | Phase::Failed => !self.pinned(slot),
            Phase::Loading | Phase::Departed => false,
        })
    }

    /// Register a freshly verified, already-ready source in an empty book slot.
    /// Caller must verify the entry/key in this epoch; never imports old tokens.
    pub fn register_ready(
        &mut self, slot: usize, epoch: SceneEpoch, entry: NativeEntry, key: SourceKey,
    ) -> Result<ReadyToken, Error> {
        if self.slot(slot)?.phase != Phase::Empty { return Err(Error::Occupied); }
        self.admit(slot, epoch, entry, key, Phase::Ready).map(ReadyToken)
    }

    /// Record an actual new native load on the caller's verified slot. Call under
    /// the same serialization as native initiation; retain its token for callbacks.
    /// Loading and departed slots stay occupied regardless of consumer count.
    pub fn begin_install(
        &mut self, slot: usize, epoch: SceneEpoch, entry: NativeEntry, key: SourceKey,
    ) -> Result<LoadToken, Error> {
        if self.slot(slot)?.promoted { return Err(Error::Occupied); }
        match self.slot(slot)?.phase {
            Phase::Loading | Phase::Departed => return Err(Error::Occupied),
            _ => {}
        }
        if self.pinned(slot) { return Err(Error::Pinned); }
        if self.slot(slot)?.source.is_some_and(|source| source.entry != entry) {
            return Err(Error::Occupied);
        }
        self.admit(slot, epoch, entry, key, Phase::Loading).map(LoadToken)
    }

    /// Replace a currently promoted lifetime through a real native Reload.
    /// The adapter must prove the exact active native identity and own INSTALL
    /// before committing. This is not readoption: no parked-ready interval or
    /// helper lease is published between the visible source and its new load.
    pub fn begin_promoted_reload(&mut self, promotion: PromotionToken, key: SourceKey)
        -> Result<LoadToken, Error> {
        self.promoted_ready(promotion)?;
        let source = promotion.source;
        if self.pinned(source.slot) { return Err(Error::Pinned); }
        self.admit(source.slot, source.epoch, source.entry, key, Phase::Loading).map(LoadToken)
    }

    pub fn can_reload_promoted(&self, promotion: PromotionToken) -> bool {
        self.promoted_ready(promotion).is_ok() && !self.pinned(promotion.source.slot)
    }

    /// Only the exact current load can publish readiness. A late departed load
    /// stays occupied; use native_destroyed after verified native cleanup.
    pub fn complete(&mut self, load: LoadToken) -> Result<ReadyToken, Error> {
        let state = self.exact(load.0)?;
        if state.phase == Phase::Departed { return Err(Error::Departed); }
        if !self.is_current(load.0.epoch) { return Err(Error::StaleToken); }
        if state.phase != Phase::Loading { return Err(Error::NotReady); }
        self.slots[load.0.slot].phase = Phase::Ready;
        Ok(ReadyToken(load.0))
    }

    /// Native completion is proven, but usable resources were not obtained.
    /// Keep the native lifetime occupied and non-ready; permit an in-place retry.
    /// Never call this merely because an in-flight load exceeded a timeout.
    pub fn fail(&mut self, load: LoadToken) -> Result<(), Error> {
        let state = self.exact(load.0)?;
        if state.phase == Phase::Departed { return Err(Error::Departed); }
        if !self.is_current(load.0.epoch) { return Err(Error::StaleToken); }
        if state.phase != Phase::Loading { return Err(Error::NotReady); }
        self.slots[load.0.slot].phase = Phase::Failed;
        Ok(())
    }

    /// Complete an explicit return-to-native load without publishing it as a
    /// plugin resource. Caller has verified the original descriptor/body ready
    /// on this exact entry's callback. This is not a destructor notification.
    pub fn returned_native(&mut self, load: LoadToken) -> Result<(), Error> {
        let state = self.exact(load.0)?;
        if state.phase == Phase::Departed { return Err(Error::Departed); }
        if !self.is_current(load.0.epoch) { return Err(Error::StaleToken); }
        if state.phase != Phase::Loading { return Err(Error::NotReady); }
        if self.pinned(load.0.slot) { return Err(Error::Pinned); }
        let generation = state.generation;
        self.slots[load.0.slot] = Slot { generation, ..Slot::EMPTY };
        Ok(())
    }

    /// Transfer a verified ready helper to native visible ownership without
    /// destroying its body or invalidating leases held by existing consumers.
    /// Caller must establish the native activation boundary and preserve those
    /// resources. Loading is not completion: this accepts only a ready token.
    pub fn promote(&mut self, ready: ReadyToken) -> Result<PromotionToken, Error> {
        self.validate_ready(ready)?;
        let state = self.exact(ready.0)?;
        if state.promoted { return Err(Error::Occupied); }
        let serial = state.promotion_serial.checked_add(1).ok_or(Error::CounterExhausted)?;
        let state = &mut self.slots[ready.0.slot];
        state.promotion_serial = serial;
        state.promoted = true;
        Ok(PromotionToken { source: ready.0, serial })
    }

    /// Reserve every managed seat before native count activation. Validate the
    /// entire transaction first: a later stale/loading seat must not reserve an
    /// earlier one. Retrying the same reservation returns the same serials.
    /// Reservation protects resources; it does not certify visible activation.
    pub fn reserve_promotions<const N: usize>(
        &mut self, sources: [Option<SourceToken>; N],
    ) -> Result<[Option<PromotionToken>; N], Error> {
        for (i, source) in sources.iter().copied().enumerate() {
            let Some(source) = source else { continue; };
            if sources[..i].contains(&Some(source)) { return Err(Error::Occupied); }
            self.validate_ready(ReadyToken(source))?;
            let state = self.exact(source)?;
            if !state.promoted && state.promotion_serial == u64::MAX {
                return Err(Error::CounterExhausted);
            }
        }
        Ok(sources.map(|source| source.map(|source| {
            let state = &mut self.slots[source.slot];
            if !state.promoted {
                state.promotion_serial += 1;
                state.promoted = true;
            }
            PromotionToken { source, serial: state.promotion_serial }
        })))
    }

    /// Re-adopt only after the caller proves native deactivation has completed
    /// and the exact retained entry/body is parked ready. No reload is implied.
    pub fn readopt(&mut self, promotion: PromotionToken) -> Result<ReadyToken, Error> {
        let ready = ReadyToken(promotion.source);
        self.validate_ready(ready)?;
        let state = self.exact(promotion.source)?;
        if !state.promoted || state.promotion_serial != promotion.serial {
            return Err(Error::StaleToken);
        }
        self.slots[promotion.source.slot].promoted = false;
        self.slots[promotion.source.slot].presentation = None;
        Ok(ready)
    }

    /// Current reservation only; native activation completion is a separate
    /// proof. Readoption, a later reservation, destruction or departure refuses.
    pub fn promoted_ready(&self, promotion:PromotionToken)->Result<ReadyToken,Error> {
        let ready=ReadyToken(promotion.source);
        self.validate_ready(ready)?;
        let state=self.exact(promotion.source)?;
        if !state.promoted || state.promotion_serial!=promotion.serial {return Err(Error::NotReady);}
        Ok(ready)
    }

    /// Commit only after the adapter proves actual native activation, the exact
    /// retained body identity and (for Miis) the full current payload. A reserved
    /// promotion alone is deliberately insufficient to call this method.
    /// This book-wide serial is independent of the CPU1 selection generation and
    /// is never reused after readoption, destruction, replacement or departure.
    pub fn publish_promotion(&mut self, promotion: PromotionToken) -> Result<u64, Error> {
        self.promoted_ready(promotion)?;
        if let Some(serial) = self.slots[promotion.source.slot].presentation {
            return Ok(serial);
        }
        let serial = self.presentation_serial.checked_add(1).ok_or(Error::CounterExhausted)?;
        self.presentation_serial = serial;
        self.slots[promotion.source.slot].presentation = Some(serial);
        Ok(serial)
    }

    /// A publication receipt, never an activation proof by itself. Consumers
    /// also reacquire and validate their current native pointers before use.
    pub fn published_promotion(&self, promotion: PromotionToken) -> Result<Option<u64>, Error> {
        self.promoted_ready(promotion)?;
        Ok(self.slots[promotion.source.slot].presentation)
    }

    /// Validate a published receipt against a freshly read full native identity.
    /// The adapter selects the completed transaction's readiness contract and
    /// rechecks its source snapshot before calling this; this never reads native
    /// memory or upgrades an unpublished reservation to a presentation.
    pub fn published_identity(&self, promotion: PromotionToken,
        sealed: Option<NativeIdentity>, current: Option<NativeIdentity>, installing: bool,
    ) -> Option<u64> {
        if installing { return None; }
        let sealed = sealed?;
        if current != Some(sealed) || sealed.slot as usize != promotion.source.slot
            || sealed.entry as u64 != promotion.source.entry.0 { return None; }
        let state = self.exact(promotion.source).ok()?;
        if state.key != Some(SourceKey { kind: sealed.kind, costume: sealed.costume }) { return None; }
        self.published_promotion(promotion).ok().flatten()
    }

    /// Every acquire owns a distinct lease, even when holder values are equal.
    /// Keep it until the actual consumer ends, not merely until the UI finishes.
    pub fn acquire(&mut self, ready: ReadyToken, holder: Holder) -> Result<LeaseToken, Error> {
        self.validate_ready(ready)?;
        if self.exact(ready.0)?.promoted { return Err(Error::NotReady); }
        let index = self.leases.iter().position(Option::is_none).ok_or(Error::LeaseCapacity)?;
        let serial = self.lease_serial.checked_add(1).ok_or(Error::CounterExhausted)?;
        let token = LeaseToken { index, serial, source: ready.0, holder };
        self.leases[index] = Some(token);
        self.lease_serial = serial;
        Ok(token)
    }

    /// Existing consumers retain their exact resource proof during promotion;
    /// this cannot grant a new lease or select another helper's ready token.
    pub fn leased_ready(&self, lease: LeaseToken, holder: Holder) -> Option<ReadyToken> {
        let ready = ReadyToken(lease.source);
        (lease.holder == holder
            && self.leases.get(lease.index) == Some(&Some(lease))
            && self.validate_ready(ready).is_ok()).then_some(ready)
    }

    pub fn validate_lease(&self, lease: LeaseToken, holder: Holder) -> bool {
        self.leased_ready(lease, holder).is_some()
    }

    /// Idempotent: false means absent/stale and leaves every other lease intact.
    pub fn release(&mut self, lease: LeaseToken) -> bool {
        if !self.validate_lease(lease, lease.holder) { return false; }
        self.leases[lease.index] = None;
        true
    }

    /// Invalidates intent and leases immediately, but retains native occupancy.
    /// No native operation is performed. Counter exhaustion is mutation-free.
    pub fn depart_scene(&mut self) -> Result<SceneEpoch, Error> {
        let next = self.epoch.0.checked_add(1).ok_or(Error::CounterExhausted)?;
        self.epoch = SceneEpoch(next);
        self.leases = [None; LEASES];
        for state in &mut self.slots {
            if state.source.is_some() { state.phase = Phase::Departed; }
        }
        Ok(self.epoch)
    }

    /// Acknowledge actual native destruction/cleanup using the identity retained
    /// when that native lifetime began. Address-only destruction is insufficient.
    pub fn native_destroyed(&mut self, source: SourceToken) -> Result<usize, Error> {
        self.exact(source)?;
        let mut released = 0;
        for lease in &mut self.leases {
            if lease.map(|token| token.source == source).unwrap_or(false) {
                *lease = None;
                released += 1;
            }
        }
        let generation = self.slots[source.slot].generation;
        self.slots[source.slot] = Slot { generation, ..Slot::EMPTY };
        Ok(released)
    }

    fn slot(&self, slot: usize) -> Result<&Slot, Error> {
        self.slots.get(slot).ok_or(Error::InvalidSlot)
    }

    fn exact(&self, source: SourceToken) -> Result<&Slot, Error> {
        let state = self.slot(source.slot)?;
        if state.source != Some(source) { return Err(Error::StaleToken); }
        Ok(state)
    }

    fn validate_ready(&self, ready: ReadyToken) -> Result<(), Error> {
        let state = self.exact(ready.0)?;
        if !self.is_current(ready.0.epoch) { return Err(Error::StaleToken); }
        if state.phase != Phase::Ready { return Err(Error::NotReady); }
        Ok(())
    }

    fn pinned(&self, slot: usize) -> bool {
        self.leases.iter().flatten().any(|lease| lease.source.slot == slot)
    }

    fn admit(
        &mut self, slot: usize, epoch: SceneEpoch, entry: NativeEntry,
        key: SourceKey, phase: Phase,
    ) -> Result<SourceToken, Error> {
        if !self.is_current(epoch) { return Err(Error::StaleToken); }
        if self.slots.iter().enumerate().any(|(index, state)| {
            index != slot && state.source.map(|source| source.entry == entry).unwrap_or(false)
        }) { return Err(Error::EntryAlreadyRegistered); }
        let generation = self.slot(slot)?.generation.checked_add(1).ok_or(Error::CounterExhausted)?;
        let source = SourceToken { slot, entry, generation, epoch };
        self.slots[slot] = Slot { generation, source: Some(source), key: Some(key), phase,
            promoted: false, promotion_serial: 0, presentation: None };
        Ok(source)
    }
}

impl<const SOURCES: usize, const LEASES: usize> Default for SourceBook<SOURCES, LEASES> {
    fn default() -> Self { Self::new() }
}

