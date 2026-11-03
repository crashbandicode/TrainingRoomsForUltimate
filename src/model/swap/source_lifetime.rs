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

#[cfg(test)]
mod tests {
    use super::*;

    const A: SourceKey = SourceKey { kind: 7, costume: 2 };
    const B: SourceKey = SourceKey { kind: 9, costume: 0 };
    const COPY_P1: Holder = Holder { category: 1, id: 100 };
    const COPY_CPU: Holder = Holder { category: 1, id: 200 };
    const ITEM: Holder = Holder { category: 2, id: 300 };

    #[test]
    fn count_promotion_hud_uses_completed_purpose_and_full_published_identity() {
        use crate::engine::swap::character_forms::EntryState;
        // s6-12 #294 retained Samus identity; s6-11 #308 parked marker shape.
        let native = NativeIdentity { slot: 2, entry: 77577412608, kind: 3,
            costume: 3, body_id: 131073, bodies: [(75895878416, 131073), (0, 0), (0, 0)],
            descriptor: 77577412880, resource: 77044595200 };
        let shape = EntryState { count: 1, kinds: [3, u32::MAX, u32::MAX],
            actor_count: 1, installed: 1, index: 0, previous_or_next: 0, pending: 255,
            flags: 0, outer: 6, loader: 14, changing: 1, guard: 0 };
        let mut book = SourceBook::<4, 1>::new();
        let ready = book.register_ready(2, book.epoch(), NativeEntry(native.entry as u64),
            SourceKey { kind: 3, costume: 3 }).unwrap();
        let promotion = book.promote(ready).unwrap();
        assert_eq!(book.published_identity(promotion, Some(native), Some(native), false), None);
        let generation = book.publish_promotion(promotion).unwrap();
        assert!(shape.ready().is_none(), "ordinary Reload stays strict");
        assert_eq!(book.published_identity(promotion, Some(native),
            shape.ready_count_promoted(false).map(|_| native), false), None);
        assert_eq!(book.published_identity(promotion, Some(native),
            shape.ready_count_promoted(true).map(|_| native), false), Some(generation));
        let reset = EntryState { changing: 0, ..shape };
        assert_eq!(book.published_identity(promotion, Some(native),
            reset.ready_count_promoted(true).map(|_| native), false), Some(generation));
        assert_eq!(book.published_identity(promotion, Some(native), Some(native), true), None);
        assert_eq!(book.published_identity(promotion, None, Some(native), false), None);
        assert_eq!(book.published_identity(promotion, Some(native), None, false), None);
        for changed in [
            NativeIdentity { slot: 3, ..native },
            NativeIdentity { entry: native.entry + 8, ..native },
            NativeIdentity { kind: 2, ..native },
            NativeIdentity { costume: 4, ..native },
            NativeIdentity { body_id: 131074, ..native },
            NativeIdentity { bodies: [(75895878424, 131073), (0, 0), (0, 0)], ..native },
            NativeIdentity { bodies: [(75895878416, 131074), (0, 0), (0, 0)], ..native },
            NativeIdentity { bodies: [(75895878416, 131073), (8, 2), (0, 0)], ..native },
            NativeIdentity { descriptor: native.descriptor + 8, ..native },
            NativeIdentity { resource: native.resource + 8, ..native },
        ] {
            assert_eq!(book.published_identity(promotion, Some(native), Some(changed), false), None);
        }
        // Even two matching identities cannot name a foreign source/key.
        for foreign in [NativeIdentity { slot: 1, ..native },
            NativeIdentity { entry: 17, ..native }, NativeIdentity { kind: 4, ..native },
            NativeIdentity { costume: 0, ..native }] {
            assert_eq!(book.published_identity(promotion, Some(foreign), Some(foreign), false), None);
        }
        assert_eq!(book.published_identity(promotion, Some(native), Some(native), false), Some(generation));
    }

    #[test]
    fn count_promotion_hud_rejects_readopt_reload_reused_entry_and_old_epoch() {
        let native = NativeIdentity { slot: 2, entry: 17, kind: A.kind, costume: A.costume,
            body_id: 3, bodies: [(24, 3), (0, 0), (0, 0)], descriptor: 32, resource: 40 };
        let mut book = SourceBook::<4, 1>::new();
        let ready = book.register_ready(2, book.epoch(), NativeEntry(17), A).unwrap();
        let old = book.promote(ready).unwrap();
        let generation = book.publish_promotion(old).unwrap();
        book.readopt(old).unwrap();
        assert_eq!(book.published_identity(old, Some(native), Some(native), false), None);
        let next = book.promote(ready).unwrap();
        assert_eq!(book.published_identity(next, Some(native), Some(native), false), None);
        let newer = book.publish_promotion(next).unwrap();
        assert!(newer > generation);
        assert_eq!(book.published_identity(old, Some(native), Some(native), false), None);
        assert_eq!(book.published_identity(next, Some(native), Some(native), false), Some(newer));
        let load = book.begin_promoted_reload(next, A).unwrap();
        assert_eq!(book.published_identity(next, Some(native), Some(native), false), None);
        let ready = book.complete(load).unwrap();
        let reloaded = book.promote(ready).unwrap();
        book.publish_promotion(reloaded).unwrap();
        book.native_destroyed(ready.source()).unwrap();
        let reused = book.register_ready(2, book.epoch(), NativeEntry(17), A).unwrap();
        let replacement = book.promote(reused).unwrap();
        book.publish_promotion(replacement).unwrap();
        assert_eq!(book.published_identity(reloaded, Some(native), Some(native), false), None);
        assert!(book.published_identity(replacement, Some(native), Some(native), false).is_some());
        book.depart_scene().unwrap();
        assert_eq!(book.published_identity(replacement, Some(native), Some(native), false), None);
    }

    #[test]
    fn count_promotion_reset_dispatch_reaches_four_training_owners_only() {
        for owner in 0..=3 { assert!(training_portrait_owner(owner, true)); }
        for owner in [4, 5, 6, 7, 8, u64::MAX] { assert!(!training_portrait_owner(owner, true)); }
        for owner in 0..=8 { assert!(!training_portrait_owner(owner, false)); }
    }

    #[test]
    fn visible_cutover_requires_real_consumer_release_and_a_new_completed_generation() {
        let mut book = SourceBook::<2, 4>::new();
        let old = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let other = book.register_ready(1, book.epoch(), NativeEntry(18), B).unwrap();
        let hat = book.acquire(old, COPY_P1).unwrap();
        let item = book.acquire(old, ITEM).unwrap();
        let promotion = book.promote(old).unwrap();
        let shown = book.publish_promotion(promotion).unwrap();
        book.release(hat);
        assert!(!book.can_reload_promoted(promotion));
        assert_eq!(book.begin_promoted_reload(promotion, B), Err(Error::Pinned));
        assert_eq!(book.published_promotion(promotion), Ok(Some(shown)));
        assert!(book.validate_lease(item, ITEM));
        book.release(item);
        assert!(book.can_reload_promoted(promotion));
        let load = book.begin_promoted_reload(promotion, B).unwrap();
        assert_ne!(load.source(), old.source());
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        assert!(book.ready(0).is_err());
        assert!(book.published_promotion(promotion).is_err());
        assert!(!book.can_reload_promoted(promotion));
        assert_eq!(book.ready(1), Ok(other));
        let new = book.complete(load).unwrap();
        let next = book.promote(new).unwrap();
        assert!(book.publish_promotion(next).unwrap() > shown);
        assert_eq!(book.begin_promoted_reload(promotion, A), Err(Error::StaleToken));
        book.depart_scene().unwrap();
        assert_eq!(book.begin_promoted_reload(next, A), Err(Error::StaleToken));
    }

    #[test]
    fn presentation_requires_publication_and_never_reuses_a_visible_generation() {
        let mut book = SourceBook::<3, 4>::new();
        let cpu1 = book.register_ready(0, book.epoch(), NativeEntry(16), A).unwrap();
        let cpu2 = book.register_ready(1, book.epoch(), NativeEntry(17), A).unwrap();
        let cpu3 = book.register_ready(2, book.epoch(), NativeEntry(18), B).unwrap();
        let lease = book.acquire(cpu2, COPY_P1).unwrap();
        let second = book.promote(cpu2).unwrap();
        let third = book.promote(cpu3).unwrap();
        assert_eq!(book.published_promotion(second), Ok(None));
        assert_eq!(book.published_promotion(third), Ok(None));
        // The adapter supplies actual native completion separately for each seat.
        let s2 = book.publish_promotion(second).unwrap();
        assert_eq!(book.published_promotion(third), Ok(None));
        let s3 = book.publish_promotion(third).unwrap();
        assert_ne!(s2, s3);
        for _ in 0..128 {
            assert_eq!(book.publish_promotion(second), Ok(s2));
            assert_eq!(book.publish_promotion(third), Ok(s3));
            assert_eq!(book.ready(0), Ok(cpu1));
            assert_eq!(book.leased_ready(lease, COPY_P1), Some(cpu2));
        }
        book.readopt(third).unwrap();
        assert!(book.published_promotion(third).is_err());
        let third_again = book.promote(cpu3).unwrap();
        assert_eq!(book.published_promotion(third_again), Ok(None));
        let s3_again = book.publish_promotion(third_again).unwrap();
        assert!(s3_again > s3);
        assert!(book.publish_promotion(third).is_err());
        book.native_destroyed(cpu3.source()).unwrap();
        let replacement = book.register_ready(2, book.epoch(), NativeEntry(18), B).unwrap();
        let replacement_promotion = book.promote(replacement).unwrap();
        assert!(book.publish_promotion(replacement_promotion).unwrap() > s3_again);
        assert!(book.published_promotion(third_again).is_err());
        book.depart_scene().unwrap();
        assert!(book.published_promotion(second).is_err());
        assert!(book.published_promotion(replacement_promotion).is_err());
    }

    #[test]
    fn exhausted_presentation_generation_does_not_publish_or_revoke_existing_receipts() {
        let mut book = SourceBook::<2, 1>::new();
        let first = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let second = book.register_ready(1, book.epoch(), NativeEntry(18), B).unwrap();
        let first = book.promote(first).unwrap();
        let second = book.promote(second).unwrap();
        book.presentation_serial = u64::MAX - 1;
        assert_eq!(book.publish_promotion(first), Ok(u64::MAX));
        assert_eq!(book.publish_promotion(second), Err(Error::CounterExhausted));
        assert_eq!(book.published_promotion(second), Ok(None));
        assert_eq!(book.publish_promotion(first), Ok(u64::MAX));
        assert!(book.promoted_ready(second).is_ok());
    }

    #[test]
    fn count_reservation_is_atomic_and_retries_preserve_leases_and_serials() {
        let mut book = SourceBook::<3, 4>::new();
        let first = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let loading = book.begin_install(1, book.epoch(), NativeEntry(18), B).unwrap();
        let lease = book.acquire(first, COPY_P1).unwrap();
        let seats = [Some(first.source()), Some(loading.source())];
        assert_eq!(book.reserve_promotions(seats), Err(Error::NotReady));
        assert_eq!(book.ready(0), Ok(first));
        let second = book.complete(loading).unwrap();
        let reserved = book.reserve_promotions(seats).unwrap();
        for _ in 0..128 {
            assert_eq!(book.reserve_promotions(seats), Ok(reserved));
            assert_eq!(book.leased_ready(lease, COPY_P1), Some(first));
            assert_eq!(book.can_replace(0), Ok(false));
            assert_eq!(book.can_replace(1), Ok(false));
        }
        // A native abort/deactivation is acknowledged separately, only after
        // the adapter proves both exact bodies parked. Reservation alone never
        // asserts that either body activated.
        assert_eq!(book.readopt(reserved[1].unwrap()), Ok(second));
        assert_eq!(book.readopt(reserved[0].unwrap()), Ok(first));
        let next = book.reserve_promotions(seats).unwrap();
        assert_ne!(next, reserved);
        assert_eq!(book.readopt(reserved[0].unwrap()), Err(Error::StaleToken));
        assert!(book.validate_lease(lease, COPY_P1));
    }

    #[test]
    fn count_reservation_rejects_partial_failure_and_scene_reuse_without_mutation() {
        let mut book = SourceBook::<2, 4>::new();
        let first = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let second = book.register_ready(1, book.epoch(), NativeEntry(18), B).unwrap();
        let seats = [Some(first.source()), Some(second.source())];
        book.slots[1].promotion_serial = u64::MAX;
        assert_eq!(book.reserve_promotions(seats), Err(Error::CounterExhausted));
        assert_eq!(book.ready(0), Ok(first));
        assert_eq!(book.slots[0].promotion_serial, 0);
        assert_eq!(book.reserve_promotions([seats[0], seats[0]]), Err(Error::Occupied));
        assert_eq!(book.ready(0), Ok(first));
        book.depart_scene().unwrap();
        assert_eq!(book.reserve_promotions(seats), Err(Error::StaleToken));
        book.native_destroyed(first.source()).unwrap();
        let replacement = book.register_ready(0, book.epoch(), NativeEntry(17), B).unwrap();
        assert_eq!(book.reserve_promotions(seats), Err(Error::StaleToken));
        assert_eq!(book.ready(0), Ok(replacement));
    }

    #[test]
    fn promotion_keeps_existing_resources_but_removes_helper_admission() {
        let mut book = SourceBook::<1, 3>::new();
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let hat = book.acquire(ready, COPY_P1).unwrap();
        let item = book.acquire(ready, ITEM).unwrap();
        let promoted = book.promote(ready).unwrap();
        assert_eq!(promoted.source(), ready.source());
        assert_eq!(book.promoted_ready(promoted),Ok(ready));
        assert_eq!(book.phase(0), Ok(Phase::Ready));
        assert_eq!(book.key(0), Ok(Some(A)));
        assert_eq!(book.ready(0), Err(Error::NotReady));
        assert_eq!(book.acquire(ready, COPY_CPU), Err(Error::NotReady));
        assert_eq!(book.can_replace(0), Ok(false));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), B), Err(Error::Occupied));
        assert!(book.validate_lease(hat, COPY_P1));
        assert_eq!(book.leased_ready(hat, COPY_P1), Some(ready));
        assert_eq!(book.leased_ready(hat, ITEM), None);
        assert!(book.validate_lease(item, ITEM));
        assert!(book.release(hat));
        assert_eq!(book.leased_ready(hat, COPY_P1), None);
        assert_eq!(book.readopt(promoted), Ok(ready));
        assert!(book.promoted_ready(promoted).is_err());
        assert!(book.validate_lease(item, ITEM));
        assert_eq!(book.can_replace(0), Ok(false));
        assert!(book.release(item));
        assert_eq!(book.can_replace(0), Ok(true));
        assert!(book.acquire(ready, COPY_CPU).is_ok());
    }

    #[test]
    fn promotion_waits_for_real_load_completion_and_rejects_stale_return() {
        let mut book = SourceBook::<1, 2>::new();
        let load = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        assert_eq!(book.promote(ReadyToken(load.source())), Err(Error::NotReady));
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        let ready = book.complete(load).unwrap();
        let first = book.promote(ready).unwrap();
        assert_eq!(book.promote(ready), Err(Error::Occupied));
        assert_eq!(book.readopt(first), Ok(ready));
        let second = book.promote(ready).unwrap();
        assert_ne!(first, second);
        assert_eq!(book.readopt(first), Err(Error::StaleToken));
        assert_eq!(book.ready(0), Err(Error::NotReady));
        assert_eq!(book.readopt(second), Ok(ready));
        let third = book.promote(ready).unwrap();
        book.depart_scene().unwrap();
        assert_eq!(book.readopt(third), Err(Error::StaleToken));
        book.native_destroyed(ready.source()).unwrap();
        let next = book.register_ready(0, book.epoch(), NativeEntry(17), B).unwrap();
        let fourth = book.promote(next).unwrap();
        assert_eq!(book.readopt(third), Err(Error::StaleToken));
        assert_eq!(book.readopt(fourth), Ok(next));
    }

    #[test]
    fn native_destruction_retires_promoted_consumers_and_serial_exhaustion_is_atomic() {
        let mut book = SourceBook::<1, 2>::new();
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let item = book.acquire(ready, ITEM).unwrap();
        book.slots[0].promotion_serial = u64::MAX;
        let before = book.slots[0];
        assert_eq!(book.promote(ready), Err(Error::CounterExhausted));
        assert_eq!(book.slots[0], before);
        assert!(book.validate_lease(item, ITEM));
        book.slots[0].promotion_serial = 0;
        let promoted = book.promote(ready).unwrap();
        assert_eq!(book.native_destroyed(promoted.source()), Ok(1));
        assert!(!book.validate_lease(item, ITEM));
        assert!(!book.release(item));
        assert_eq!(book.readopt(promoted), Err(Error::StaleToken));
        assert_eq!(book.phase(0), Ok(Phase::Empty));
    }

    #[test]
    fn returning_native_requires_drained_leases_and_never_publishes_a_source() {
        let mut book = SourceBook::<1, 2>::new();
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let hat = book.acquire(ready, COPY_P1).unwrap();
        let item = book.acquire(ready, ITEM).unwrap();
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), B), Err(Error::Pinned));
        assert!(book.release(hat));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), B), Err(Error::Pinned));
        assert!(book.release(item));
        let restore = book.begin_install(0, book.epoch(), NativeEntry(17), B).unwrap();
        assert_eq!(book.ready(0), Err(Error::NotReady));
        // Native-ready receipt arrives on a later callback, not a timeout.
        assert_eq!(book.returned_native(restore), Ok(()));
        assert_eq!(book.phase(0), Ok(Phase::Empty));
        assert_eq!(book.key(0), Ok(None));
        assert_eq!(book.acquire(ready, COPY_P1), Err(Error::StaleToken));
        assert_eq!(book.returned_native(restore), Err(Error::StaleToken));
        let reuse = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        assert_ne!(restore, reuse);
        assert_eq!(book.returned_native(restore), Err(Error::StaleToken));
        book.depart_scene().unwrap();
        assert_eq!(book.returned_native(reuse), Err(Error::Departed));
        assert_eq!(book.phase(0), Ok(Phase::Departed));
        assert_eq!(book.native_destroyed(reuse.source()), Ok(0));
    }

    #[test]
    fn normal_load_ready_lease_release_reuse() {
        let mut book = SourceBook::<2, 4>::new();
        let load = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        assert_eq!(book.ready(0), Err(Error::NotReady));
        assert_eq!(book.can_replace(0), Ok(false));
        let ready = book.complete(load).unwrap();
        let lease = book.acquire(ready, COPY_P1).unwrap();
        assert!(book.validate_lease(lease, COPY_P1));
        assert!(book.release(lease));
        assert_eq!(book.can_replace(0), Ok(true));
        let next = book.begin_install(0, book.epoch(), NativeEntry(17), B).unwrap();
        assert_ne!(next.source(), ready.source());
        assert_eq!(book.acquire(ready, COPY_CPU), Err(Error::StaleToken));
        assert!(book.complete(next).is_ok());
        assert_eq!(book.key(0), Ok(Some(B)));
    }

    #[test]
    fn simultaneous_hat_and_item_pin_source_independently() {
        let mut book = SourceBook::<1, 4>::new();
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let hat = book.acquire(ready, COPY_P1).unwrap();
        let item = book.acquire(ready, ITEM).unwrap();
        assert!(!book.validate_lease(hat, ITEM));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(18), B), Err(Error::Pinned));
        assert!(book.release(hat));
        assert!(book.validate_lease(item, ITEM));
        assert_eq!(book.can_replace(0), Ok(false));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), B), Err(Error::Pinned));
        assert!(book.release(item));
        assert_eq!(book.can_replace(0), Ok(true));
    }

    #[test]
    fn full_source_and_lease_capacity_preserve_existing_dependencies() {
        let mut book = SourceBook::<2, 2>::new();
        let first = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let second = book.register_ready(1, book.epoch(), NativeEntry(18), B).unwrap();
        let hat = book.acquire(first, COPY_P1).unwrap();
        let item = book.acquire(second, ITEM).unwrap();
        assert_eq!(book.acquire(first, COPY_CPU), Err(Error::LeaseCapacity));
        assert_eq!(book.begin_install(2, book.epoch(), NativeEntry(19), A), Err(Error::InvalidSlot));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(19), A), Err(Error::Pinned));
        assert_eq!(book.begin_install(1, book.epoch(), NativeEntry(19), A), Err(Error::Pinned));
        assert!(book.validate_lease(hat, COPY_P1));
        assert!(book.validate_lease(item, ITEM));
        assert_eq!(book.ready(0), Ok(first));
        assert_eq!(book.ready(1), Ok(second));
    }

    #[test]
    fn late_completion_after_same_address_reuse_cannot_commit() {
        let mut book = SourceBook::<1, 1>::new();
        let old = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        book.native_destroyed(old.source()).unwrap();
        let new = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        assert_ne!(old, new);
        assert_eq!(book.complete(old), Err(Error::StaleToken));
        assert_eq!(book.native_destroyed(old.source()), Err(Error::StaleToken));
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        let ready = book.complete(new).unwrap();
        assert_eq!(book.complete(new), Err(Error::NotReady));
        assert_eq!(book.ready(0), Ok(ready));
    }

    #[test]
    fn stale_and_double_release_cannot_release_reused_lease_cell() {
        let mut book = SourceBook::<1, 2>::new();
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        let old = book.acquire(ready, COPY_P1).unwrap();
        assert!(book.release(old));
        assert!(!book.release(old));
        let new = book.acquire(ready, COPY_P1).unwrap();
        let other = book.acquire(ready, COPY_P1).unwrap();
        assert_ne!(old, new);
        assert_ne!(new, other);
        assert!(!book.release(old));
        assert!(book.validate_lease(new, COPY_P1));
        book.native_destroyed(ready.source()).unwrap();
        assert!(!book.validate_lease(new, COPY_P1));
        assert!(!book.release(other));
        let next = book.register_ready(0, book.epoch(), NativeEntry(18), A).unwrap();
        let fresh = book.acquire(next, COPY_CPU).unwrap();
        assert!(!book.release(new));
        assert!(book.validate_lease(fresh, COPY_CPU));
    }

    #[test]
    fn departure_during_load_retains_occupancy_until_exact_cleanup() {
        let mut book = SourceBook::<2, 2>::new();
        let epoch = book.epoch();
        let load = book.begin_install(0, epoch, NativeEntry(17), A).unwrap();
        let ready = book.register_ready(1, epoch, NativeEntry(18), B).unwrap();
        let lease = book.acquire(ready, ITEM).unwrap();
        book.depart_scene().unwrap();
        assert!(!book.is_current(epoch));
        assert!(!book.validate_lease(lease, ITEM));
        assert!(!book.release(lease));
        for slot in 0..2 {
            assert_eq!(book.phase(slot), Ok(Phase::Departed));
            assert_eq!(book.can_replace(slot), Ok(false));
            assert_eq!(book.begin_install(slot, book.epoch(), NativeEntry(19), A), Err(Error::Occupied));
        }
        assert_eq!(book.complete(load), Err(Error::Departed));
        assert_eq!(book.phase(0), Ok(Phase::Departed));
        book.native_destroyed(load.source()).unwrap();
        assert_eq!(book.phase(0), Ok(Phase::Empty));
        assert_eq!(book.phase(1), Ok(Phase::Departed));
        assert_eq!(book.begin_install(0, epoch, NativeEntry(17), A), Err(Error::StaleToken));
        assert!(book.begin_install(0, book.epoch(), NativeEntry(17), A).is_ok());
    }

    #[test]
    fn two_scene_reconstruction_cycles_require_fresh_tokens() {
        let mut book = SourceBook::<1, 2>::new();
        let mut ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        for _ in 0..2 {
            let hat = book.acquire(ready, COPY_P1).unwrap();
            let item = book.acquire(ready, ITEM).unwrap();
            let epoch = book.epoch();
            book.depart_scene().unwrap();
            assert!(!book.is_current(epoch));
            assert_eq!(book.acquire(ready, COPY_CPU), Err(Error::StaleToken));
            book.native_destroyed(ready.source()).unwrap();
            let load = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
            let next = book.complete(load).unwrap();
            assert_ne!(ready, next);
            assert!(!book.release(hat));
            assert!(!book.validate_lease(item, ITEM));
            ready = next;
        }
        assert!(book.acquire(ready, COPY_CPU).is_ok());
    }

    #[test]
    fn caller_cancellation_does_not_finish_native_install() {
        let mut book = SourceBook::<1, 1>::new();
        let load = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        // UI drops its intent; native callback still owns the load token.
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), B), Err(Error::Occupied));
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        let ready = book.complete(load).unwrap();
        assert_eq!(book.can_replace(0), Ok(true));
        assert!(book.acquire(ready, ITEM).is_ok());
    }

    #[test]
    fn duplicate_native_identity_and_stale_destruction_are_rejected() {
        let mut book = SourceBook::<2, 1>::new();
        let old = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        assert_eq!(book.register_ready(1, book.epoch(), NativeEntry(17), B), Err(Error::EntryAlreadyRegistered));
        assert_eq!(book.phase(1), Ok(Phase::Empty));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(18), B), Err(Error::Occupied));
        assert_eq!(book.ready(0), Ok(old));
        let next = book.begin_install(0, book.epoch(), NativeEntry(17), B).unwrap();
        assert_eq!(book.native_destroyed(old.source()), Err(Error::StaleToken));
        assert_eq!(book.phase(0), Ok(Phase::Loading));
        assert!(book.complete(next).is_ok());
    }

    #[test]
    fn counter_exhaustion_and_zero_capacity_fail_without_mutation() {
        let mut empty = SourceBook::<0, 0>::new();
        assert_eq!(empty.begin_install(0, empty.epoch(), NativeEntry(17), A), Err(Error::InvalidSlot));
        let mut book = SourceBook::<1, 1>::new();
        book.slots[0].generation = u64::MAX;
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(17), A), Err(Error::CounterExhausted));
        assert_eq!(book.phase(0), Ok(Phase::Empty));
        book.slots[0].generation = 0;
        let ready = book.register_ready(0, book.epoch(), NativeEntry(17), A).unwrap();
        book.lease_serial = u64::MAX;
        assert_eq!(book.acquire(ready, ITEM), Err(Error::CounterExhausted));
        book.lease_serial = 0;
        book.epoch = SceneEpoch(u64::MAX);
        let before = book.slots[0];
        assert_eq!(book.depart_scene(), Err(Error::CounterExhausted));
        assert_eq!(book.slots[0], before);
    }

    #[test]
    fn completed_unusable_load_can_retry_only_on_same_native_entry() {
        let mut book = SourceBook::<1, 1>::new();
        let load = book.begin_install(0, book.epoch(), NativeEntry(17), A).unwrap();
        book.fail(load).unwrap();
        assert_eq!(book.phase(0), Ok(Phase::Failed));
        assert_eq!(book.ready(0), Err(Error::NotReady));
        assert_eq!(book.complete(load), Err(Error::NotReady));
        assert_eq!(book.can_replace(0), Ok(true));
        assert_eq!(book.begin_install(0, book.epoch(), NativeEntry(18), B), Err(Error::Occupied));
        let retry = book.begin_install(0, book.epoch(), NativeEntry(17), B).unwrap();
        assert_eq!(book.fail(load), Err(Error::StaleToken));
        let ready = book.complete(retry).unwrap();
        book.acquire(ready, ITEM).unwrap();
        assert_eq!(book.native_destroyed(ready.source()), Ok(1));
    }
}
