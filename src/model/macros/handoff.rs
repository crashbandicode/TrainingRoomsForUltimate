//! Worker archive ownership over the existing portable replay engines.
//! Kept in engine::macros so standalone model test crates stay engine-independent.

/// Copied Vault observations; native pointers never cross to the worker.
#[derive(Clone, Copy, Debug)]
pub enum VaultEvent {
    Begin { epoch: u64 },
    Sample { epoch: u64, entry: i32, sample: crate::engine::macros::ReplayFrame, quantized: bool },
    End { epoch: u64, truncated: bool },
}

/// rc.8 R4: the slots of each replay mailbox (`live_replay::EVENTS`, `replay_capture::VAULT_EVENTS`) and of each drain's heap
/// scratch, from 4096. A game hook posts at most one `Sample` per fighter entry per game frame (8 entries at 60 frames a
/// second: 480 a second), and every post wakes the menu worker, which drains them all. The rc.8 pass's highest
/// `[MailboxPeak]` was 4 waiting (`final-surface-vs-9d6c7e02`, a 5,818-frame VS match with capture on; 1 or 2 elsewhere).
/// 1024 slots hold 2.1 s of the 8-entry rate, the diag stall watchdog's `STALL_MS`: a worker that long without a pass is
/// stalled in its own right.
///
/// A full mailbox drops the post and counts it (`[MacroHandoff] live_events_dropped=` / `vault_events_dropped=`):
/// - live: that fighter's track is sealed, and the worker then clears every live capture not yet saved, so that match's
///   saved replay gets no capture. The game's own replay is unaffected; the next match captures again.
/// - vault: sampling of that replay playback stops, and its timeline keeps what arrived, marked truncated (never complete).
pub const REPLAY_EVENT_SLOTS: usize = 1024;

/// Worker-owned archive of the latest and previous native Vault timelines.
pub struct VaultArchive {
    epoch: u64,
    pub session: crate::engine::macros::ReplaySession,
    pub previous: Option<crate::engine::macros::ReplaySession>,
}
impl Default for VaultArchive { fn default() -> Self { Self::new() } }
impl VaultArchive {
    pub const fn new() -> Self {
        Self { epoch: 0, session: crate::engine::macros::ReplaySession { tracks: Vec::new(), truncated: false, breaks: Vec::new() }, previous: None }
    }
    pub fn note_loss(&mut self, last_lost_epoch: u64) {
        if last_lost_epoch != 0 && self.epoch <= last_lost_epoch { self.session.truncated = true; }
    }
    pub fn apply(&mut self, event: VaultEvent, last_lost_epoch: u64) {
        use crate::engine::macros::{ReplayTrack, MAX_FRAMES};
        let epoch = match event {
            VaultEvent::Begin { epoch } | VaultEvent::Sample { epoch, .. } | VaultEvent::End { epoch, .. } => epoch,
        };
        if epoch == 0 || epoch < self.epoch { return; }
        if epoch != self.epoch {
            let previous = std::mem::take(&mut self.session);
            if previous.tracks.iter().any(|track| !track.frames.is_empty()) { self.previous = Some(previous); }
            self.epoch = epoch;
            self.session.truncated = !matches!(event, VaultEvent::Begin { .. });
        }
        // A failed post ends hook sampling immediately. Treat queued prefixes
        // predating a known loss conservatively; never label one complete.
        self.note_loss(last_lost_epoch);
        match event {
            VaultEvent::Begin { .. } => {},
            VaultEvent::End { truncated, .. } => self.session.truncated |= truncated,
            VaultEvent::Sample { entry, sample, quantized, .. } => {
                if !(0..8).contains(&entry) { self.session.truncated = true; return; }
                let tracks = &mut self.session.tracks;
                let index = match tracks.iter().position(|track| track.entry == entry) {
                    Some(index) => index,
                    None => {
                        tracks.push(ReplayTrack { entry, frames: Vec::new(), quantized_analog: false });
                        tracks.len() - 1
                    },
                };
                let track = &mut tracks[index];
                track.quantized_analog |= quantized;
                if track.frames.len() < MAX_FRAMES { track.frames.push(sample); }
                else { self.session.truncated = true; }
            },
        }
    }
}

const _: () = assert!(crate::engine::macros::MAX_RETAINED_CAPTURES == 4);

/// Game-owned record identities only. Frame storage belongs to LiveArchive.
pub struct LiveTokens {
    generation: u64,
    keys: [Option<crate::engine::macros::NativeRecordKey>; 4],
    active: Option<crate::engine::macros::NativeRecordKey>,
    sampled_keys: [bool; 4],
    last_tick: [Option<u32>; 8],
    sealed: [bool; 8],
}
impl Default for LiveTokens { fn default() -> Self { Self::new() } }
impl LiveTokens {
    pub const fn new() -> Self {
        Self { generation: 0, keys: [None; 4], active: None, sampled_keys: [false; 4], last_tick: [None; 8], sealed: [false; 8] }
    }
    pub fn clear(&mut self) { self.keys = [None; 4]; self.sampled_keys = [false; 4]; self.active = None; }
    pub fn active_key(&self) -> Option<crate::engine::macros::NativeRecordKey> { self.active }
    pub fn key_for_index(&self, index: u32) -> Option<crate::engine::macros::NativeRecordKey> {
        self.keys.iter().flatten().copied().find(|key| key.index == index)
    }
    pub fn invalidate_record(&mut self, index: u32) {
        if self.active.is_some_and(|key| key.index == index) { self.active = None; }
        let mut kept = [None; 4];
        let mut sampled = [false; 4];
        let mut at = 0;
        for (slot, value) in self.keys.iter().enumerate() {
            if let Some(key) = value.filter(|key| key.index != index) {
                kept[at] = Some(key); sampled[at] = self.sampled_keys[slot]; at += 1;
            }
        }
        self.keys = kept; self.sampled_keys = sampled;
    }
    pub fn start_record(&mut self, index: u32) -> crate::engine::macros::NativeRecordKey {
        self.finish();
        self.invalidate_record(index);
        self.generation += 1;
        let key = crate::engine::macros::NativeRecordKey { generation: self.generation, index };
        let at = self.keys.iter().position(Option::is_none).unwrap_or_else(|| {
            self.keys.rotate_left(1); self.sampled_keys.rotate_left(1); 3
        });
        self.keys[at] = Some(key);
        self.sampled_keys[at] = false;
        self.active = Some(key);
        self.last_tick = [None; 8]; self.sealed = [false; 8];
        key
    }
    pub fn has_samples(&self, key: crate::engine::macros::NativeRecordKey) -> bool {
        self.keys.iter().position(|value| *value == Some(key)).is_some_and(|slot| self.sampled_keys[slot])
    }
    pub fn finish(&mut self) {
        if let Some(key) = self.active {
            if !self.has_samples(key) { self.invalidate_record(key.index); }
        }
        self.active = None;
    }
    pub fn wants_sample(&self, key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32) -> bool {
        self.active == Some(key) && (0..8).contains(&entry)
            && !self.sealed[entry as usize] && self.last_tick[entry as usize] != Some(tick)
    }
    pub fn sampled(&mut self, key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32) {
        if self.active == Some(key) && (0..8).contains(&entry) {
            self.last_tick[entry as usize] = Some(tick);
            if let Some(slot) = self.keys.iter().position(|value| *value == Some(key)) { self.sampled_keys[slot] = true; }
        }
    }
    pub fn seal(&mut self, key: crate::engine::macros::NativeRecordKey, entry: i32) {
        if self.active == Some(key) && (0..8).contains(&entry) { self.sealed[entry as usize] = true; }
    }
}

#[derive(Clone, Copy)]
pub struct FixedReplayId { bytes: [u8; 96], len: u8 }
impl FixedReplayId {
    pub fn new(text: &str) -> Option<Self> {
        if text.is_empty() || text.len() > 96 || text.chars().any(|c| c.is_control() || c == '/' || c == char::from(92u8)) { return None; }
        let mut value = Self { bytes: [0; 96], len: text.len() as u8 };
        value.bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(value)
    }
    fn text(&self) -> &str { std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap() }
}
#[derive(Clone, Copy)]
pub struct FixedReplayTitle { units: [u16; 40], len: u8 }
impl FixedReplayTitle {
    pub fn new(units: &[u16]) -> Option<Self> {
        if units.is_empty() || units.len() > 40
            || char::decode_utf16(units.iter().copied()).any(|c| c.is_err() || c.is_ok_and(char::is_control)) { return None; }
        let mut value = Self { units: [0; 40], len: units.len() as u8 };
        value.units[..units.len()].copy_from_slice(units);
        Some(value)
    }
    fn text(&self) -> String { String::from_utf16(&self.units[..usize::from(self.len)]).unwrap() }
}
/// ReplayMiis is a fixed value but intentionally has a non-Copy public API.
/// Copy its proven entries for the mailbox; rebuild its portable codec on the worker.
#[derive(Clone, Copy)]
pub struct FixedReplayMiis([Option<crate::engine::mii::replay::RawMii>; 8]);
impl FixedReplayMiis {
    pub fn copied(value: &crate::engine::mii::replay::ReplayMiis) -> Self {
        Self(std::array::from_fn(|entry| value.entry(entry).copied()))
    }
    pub fn owned(&self) -> Option<crate::engine::mii::replay::ReplayMiis> {
        let mut bytes = vec![1, self.0.iter().flatten().count() as u8];
        for mii in self.0.iter().flatten() {
            bytes.extend([mii.entry(), mii.class().kind() as u8]);
            bytes.extend(mii.source_index().to_le_bytes());
            bytes.extend(mii.charinfo());
            let tokens = mii.tokens();
            for token in [tokens.outfit, tokens.hat, tokens.colour, tokens.voice] { bytes.extend(token.to_le_bytes()); }
            bytes.extend(tokens.specials);
        }
        crate::engine::mii::replay::ReplayMiis::from_bytes(&bytes).ok()
    }
}
#[derive(Clone, Copy)]
pub enum LiveEvent {
    Clear,
    Start { key: crate::engine::macros::NativeRecordKey },
    Invalidate { index: u32 },
    Finish { partial: bool },
    Seal { key: crate::engine::macros::NativeRecordKey, entry: i32 },
    Sample { key: crate::engine::macros::NativeRecordKey, entry: i32, tick: u32, sample: crate::engine::macros::LiveFrame },
    Bind { save: u64, key: crate::engine::macros::NativeRecordKey },
    Forget { save: u64 },
    Commit { save: u64, id: FixedReplayId, title: FixedReplayTitle, miis: bool },
}
pub struct LiveCommit {
    pub save: u64,
    pub needs_miis: bool,
    pub id: String,
    pub title: String,
    pub session: std::sync::Arc<crate::engine::macros::LiveSession>,
    pub miis: Option<crate::engine::mii::replay::ReplayMiis>,
}
pub struct LiveArchive {
    pool: crate::engine::macros::LiveCapturePool,
    bindings: std::collections::VecDeque<(u64, std::sync::Arc<crate::engine::macros::LiveSession>)>,
    last_loss: u64,
}
impl Default for LiveArchive { fn default() -> Self { Self::new() } }
impl LiveArchive {
    pub const fn new() -> Self {
        Self { pool: crate::engine::macros::LiveCapturePool::new(), bindings: std::collections::VecDeque::new(), last_loss: 0 }
    }
    #[cfg(any(feature = "diag", test))]
    pub fn take_first_seal(&mut self) -> Option<crate::engine::macros::FirstSeal> {
        self.pool.take_first_seal()
    }

    /// A missing lifecycle/sample event invalidates uncertain associations. A later
    /// Start is independent; no suffix or reused native slot is rebound as complete.
    pub fn apply(&mut self, serial: u64, event: LiveEvent, last_lost_serial: u64) -> Option<LiveCommit> {
        if last_lost_serial > self.last_loss {
            self.pool.clear(); self.bindings.clear(); self.last_loss = last_lost_serial;
        }
        if serial <= self.last_loss { return None; }
        match event {
            LiveEvent::Clear => self.pool.clear(),
            LiveEvent::Start { key } => { self.pool.start_record_owned(key); },
            LiveEvent::Invalidate { index } => self.pool.invalidate_record(index),
            LiveEvent::Finish { partial } => if partial { self.pool.finish_partial(); } else { self.pool.finish_record(); },
            LiveEvent::Seal { key, entry } => self.pool.seal_entry(key, entry),
            LiveEvent::Sample { key, entry, tick, sample } => { self.pool.push(key, entry, tick, sample); },
            LiveEvent::Bind { save, key } => {
                self.bindings.retain(|(id, _)| *id != save);
                if let Some(session) = self.pool.snapshot(key) {
                    while self.bindings.len() >= crate::engine::macros::MAX_RETAINED_CAPTURES { self.bindings.pop_front(); }
                    self.bindings.push_back((save, session));
                }
            },
            LiveEvent::Forget { save } => self.bindings.retain(|(id, _)| *id != save),
            LiveEvent::Commit { save, id, title, miis } => {
                let at = self.bindings.iter().position(|(id, _)| *id == save)?;
                let (_, session) = self.bindings.remove(at)?;
                return Some(LiveCommit { save, needs_miis: miis, id: id.text().to_owned(), title: title.text(), session, miis: None });
            },
        }
        None
    }
}

