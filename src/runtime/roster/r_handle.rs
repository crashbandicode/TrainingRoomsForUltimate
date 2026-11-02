//! The only caller of the resource owner's acquisition and release primitives, and the only writer
//! of `R+0x4c` bit 0 and `R+0x50` (WORKPLAN §2.5; SYNTHESIS agree-3/-5). Every native key effect is
//! decided first by the role's per-R [`Ledger`] under this module's lock, then issued outside it:
//! `0x17e64d0(R, kind)` only after `Ledger::acquire` minted a token (for a group prefetch the kind is
//! first published to the per-body identity view, `identity_view::publish`, WP-R I2); `0x17e2fe0(R, key)` for the
//! orphan keys and `0x17e8610(R, kind)` in that order only for a token `Ledger::release` handed
//! back as calls (D-SG1 §5: orphans first so the native reset tail still fires); a release the live
//! consumers refuse keeps the token as a retained obligation (`[Ledger] retained …`), never a
//! destructor (WORKPLAN §5.4). `R+0x50` is written only under a `ResetBatchPermit` (Astra §4.2:
//! no other batch's keys on R when the pair rule may reset it); `R+0x4c |= 1` through
//! [`request_bit`] (GROUP-SWAP-IMPL §14.2), held for the construction only and ended by
//! [`end_request`] (U19).
//!
//! Lock discipline (module doc of `native_body_bank.rs`): the `LEDGERS` lock is never held across a
//! native call; the bank's `BANKS` lock may be held while this lock is taken, never the reverse.
//! Addresses are identity tokens: `r` is the alias the caller proved with `resource(base, owner)` in
//! the current callback. Tokens the ledger mints are held here per group (`usize::MAX` = the window).
//!
//! Lines (interface for the R-L rows): `[Ledger] acquire owner={} kind={:#x} keys={:x?} holder={:?}
//! recipe={:?} batch={}`, `[Ledger] release owner={} kind={:#x} calls={:x?} reason={}`,
//! `[Ledger] retained owner={} kind={:#x} reason={}`, `[Ledger] reset deferred owner={} batch={}
//! others={:?}`, `[Ledger] reconcile owner={} phase={} predicted={:x?} actual={:x?} residual={:x?}
//! missing={:x?}` (WP2's format).

use super::observe;
use crate::engine::seams::native_fields as fields;
use std::sync::{Mutex, MutexGuard};
use crate::engine::swap::swap_model::request_bit as bit0;
use crate::engine::swap::swap_model::ledger::{
    AcqToken, BatchId, Holder, Ledger, LedgerRefusal, LiveConsumers, NativeCall, NativeRequest, RFlags,
    ReleaseRecipe, Role,
};

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

const ACQUIRE_KIND: usize = 0x17e64d0;
const RELEASE_KIND: usize = 0x17e8610;
const RELEASE_KEY: usize = 0x17e2fe0;
/// Group index that stands for the window's own acquisitions.
pub(crate) const WINDOW: usize = usize::MAX;

/// One role's ledger, bound to the R it mirrors.
pub(crate) struct RHandle {
    r: usize,
    owner: u32,
    ledger: Ledger,
    /// Tokens the ledger minted for us, by group (`WINDOW` for the window build).
    held: Vec<(usize, AcqToken)>,
    batches: Vec<(usize, BatchId)>,
}

static LEDGERS: [Mutex<Option<RHandle>>; 2] = [Mutex::new(None), Mutex::new(None)];

fn lock(role: Role) -> MutexGuard<'static, Option<RHandle>> {
    LEDGERS[role.index()].lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl RHandle {
    fn batch(&mut self, group: usize) -> BatchId {
        if let Some((_, batch)) = self.batches.iter().find(|(g, _)| *g == group) { return *batch; }
        let batch = self.ledger.new_batch();
        self.batches.push((group, batch));
        batch
    }
}

/// Bind the role's ledger to `r`: a new R (first Ready, canonical replacement) starts an empty
/// ledger with `flags` (`R+0x48/0x7d/0x7f` as read); the same R keeps its tokens.
pub(crate) fn bind(role: Role, owner: u32, r: usize, flags: RFlags) {
    let mut slot = lock(role);
    match slot.as_mut() {
        Some(handle) if handle.r == r && handle.owner == owner => handle.ledger.set_flags(flags),
        _ => {
            *slot = Some(RHandle { r, owner, ledger: Ledger::new(flags), held: Vec::new(), batches: Vec::new() });
        }
    }
}

/// Forget the role's ledger (the scene departed or native reset R at a canonical replacement, which
/// clears the key map itself). Retained obligations are reported before they are dropped.
pub(crate) fn reset(role: Role, reason: &str) {
    let mut slot = lock(role);
    if let Some(handle) = slot.take() {
        for retained in handle.ledger.retained() {
            trace!("[Ledger] retained owner={} kind={:#x} keys={:x?} dropped with the ledger reason={}",
                handle.owner, retained.kind, retained.keys, reason);
        }
        // `AcqToken` has no `Drop` (WORKPLAN §5.4: no native work in a destructor); the reset discards
        // the tokens on purpose, after the retained-obligation lines above.
        for (_, token) in handle.held { drop(token); }
    }
}

/// Record the requests native's own `0x17e5bc0` makes for a window descriptor (never issued).
pub(crate) fn record_native(role: Role, requests: &[NativeRequest]) {
    let mut slot = lock(role);
    let Some(handle) = slot.as_mut() else { return; };
    let batch = handle.batch(WINDOW);
    for request in requests {
        let token = handle.ledger.record_native(*request, batch);
        handle.held.push((WINDOW, token));
    }
}

/// Acquire `kind` for `group` (`WINDOW` for the hook's supplemental request): the ledger decides
/// under the lock and the native request runs outside it. `Err` = nothing was issued.
///
/// # Safety
/// `base` is the main image base; the role's ledger is bound to the R the caller proved in this
/// callback (`bind`).
pub(crate) unsafe fn acquire(base: usize, role: Role, group: usize, kind: u32, holder: Holder, recipe: ReleaseRecipe,
    occupancy: &dyn LiveConsumers) -> Result<(), LedgerRefusal> {
    let (r, owner, keys, batch, call) = {
        let mut slot = lock(role);
        let Some(handle) = slot.as_mut() else { return Err(LedgerRefusal::UnknownToken); };
        let batch = handle.batch(group);
        let (token, call) = handle.ledger.acquire(kind, holder, recipe, batch, occupancy, role)?;
        let keys = token.keys();
        handle.held.push((group, token));
        (handle.r, handle.owner, keys, batch, call)
    };
    if let Some(NativeCall::Acquire { kind }) = call {
        // WP-R (identity view, I2): a detached group's kind becomes a body of `r` here, so it is
        // published for this R before its first native call (what the bank's `acquire_kind` did on
        // M2). Only for a group prefetch: the window's own kinds are `R+0x58` header kinds, which the
        // view keys without publication, and they are requested on the resource worker, where
        // `publish` (game thread under INSTALL, I4) must not run. Nothing is published for a refused
        // acquisition (nothing is issued).
        if matches!(holder, Holder::Prefetch(_)) { super::identity_view::publish(r, kind); }
        let acquire: unsafe extern "C" fn(*mut u8, u32) = std::mem::transmute(base + ACQUIRE_KIND);
        acquire(r as *mut u8, kind);
    }
    trace!("[Ledger] acquire owner={} kind={:#x} keys={:x?} holder={:?} recipe={:?} batch={} issued={}",
        owner, kind, keys.iter(kind).map(|k| k.value()).collect::<Vec<_>>(), holder, recipe, batch.0, call.is_some());
    Ok(())
}

#[derive(Debug, Default)]
pub(crate) struct ReleaseReceipt {
    pub released: usize,
    /// `(kind, why)` of every token the live consumers refused; kept as retained obligations.
    pub kept: Vec<(u32, &'static str)>,
    /// Per-key releases issued (the orphans, e.g. `0x1338`).
    pub keys: Vec<u32>,
}

fn refusal_text(refusal: &LedgerRefusal) -> &'static str {
    match refusal {
        LedgerRefusal::Conflict(crate::engine::swap::swap_model::ledger::Conflict::LiveCopySharesRegistry) => "live Kirby copies this kind",
        LedgerRefusal::Conflict(crate::engine::swap::swap_model::ledger::Conflict::PrefetchDebtUnderLiveKirby) => "prefetch debt under a live Kirby",
        LedgerRefusal::Conflict(crate::engine::swap::swap_model::ledger::Conflict::HatSourceLive) => "hat source live",
        LedgerRefusal::Conflict(crate::engine::swap::swap_model::ledger::Conflict::OtherRoleLive) => "other role live",
        LedgerRefusal::AlreadyHeld { .. } => "already held",
        LedgerRefusal::ResetNotExclusive(_) => "reset not exclusive",
        LedgerRefusal::UnknownToken => "unknown token",
    }
}

/// Release the tokens held for `group` (`None` = every group) whose kind is in `kinds` (`None` =
/// all): per token the ledger's calls, in order, outside the lock; a refused token is retained and
/// reported in the receipt (the caller logs the `key kept` line the E4 rows read).
///
/// # Safety
/// As for [`acquire`].
pub(crate) unsafe fn release(base: usize, role: Role, group: Option<usize>, kinds: Option<&[u32]>,
    occupancy: &dyn LiveConsumers, reason: &str) -> ReleaseReceipt {
    let mut receipt = ReleaseReceipt::default();
    let (r, owner, plan) = {
        let mut slot = lock(role);
        let Some(handle) = slot.as_mut() else { return receipt; };
        let selected: Vec<usize> = handle.held.iter().enumerate()
            .filter(|(_, (g, token))| group.is_none_or(|x| *g == x) && kinds.is_none_or(|ks| ks.contains(&token.kind())))
            .map(|(i, _)| i).collect();
        let mut plan: Vec<(u32, Vec<NativeCall>)> = Vec::new();
        for i in selected.into_iter().rev() {
            let (_, token) = handle.held.remove(i);
            let kind = token.kind();
            match handle.ledger.release(token, occupancy, role) {
                Ok(calls) => plan.push((kind, calls)),
                Err((token, why)) => {
                    let text = refusal_text(&why);
                    handle.ledger.retain_dropped(token, text);
                    receipt.kept.push((kind, text));
                }
            }
        }
        plan.reverse();
        (handle.r, handle.owner, plan)
    };
    for (kind, calls) in &plan {
        execute(base, r, calls);
        for call in calls {
            if let NativeCall::ReleaseKey(key) = call { receipt.keys.push(key.value()); }
        }
        receipt.released += 1;
        trace!("[Ledger] release owner={} kind={:#x} calls={:x?} reason={}", owner, kind,
            calls.iter().map(|c| match c { NativeCall::ReleaseKey(k) => k.value(), NativeCall::ReleaseKind { kind } => *kind, _ => 0 }).collect::<Vec<_>>(), reason);
    }
    for (kind, text) in &receipt.kept {
        trace!("[Ledger] retained owner={} kind={:#x} reason={} ({})", owner, kind, text, reason);
    }
    receipt
}

/// The bodies of `kinds` in `group` finished construction: their tails released the keys (or took
/// the pair rule). The matching tokens are settled, no native call.
pub(crate) fn construction_settled(role: Role, group: usize, kinds: &[u32]) {
    let mut slot = lock(role);
    let Some(handle) = slot.as_mut() else { return; };
    let batch = handle.batch(group);
    for kind in kinds {
        if let Some(i) = handle.held.iter().position(|(g, t)| *g == group && t.kind() == *kind) {
            let (_, token) = handle.held.remove(i);
            drop(token); // no `Drop` on `AcqToken`; the ledger settles the construction below
        }
        handle.ledger.construction_settled(*kind, batch);
    }
}

/// Settle, with no native call, the tokens held for `group` whose kind is in `kinds`, because the
/// owner's state proves native already freed their keys (the caller read `R+0x18 == 0`). Returns
/// the settled kinds.
pub(crate) fn settle(role: Role, group: usize, kinds: &[u32], reason: &str) -> Vec<u32> {
    let mut slot = lock(role);
    let Some(handle) = slot.as_mut() else { return Vec::new(); };
    let mut settled = Vec::new();
    while let Some(i) = handle.held.iter().position(|(g, t)| *g == group && kinds.contains(&t.kind())) {
        let (_, token) = handle.held.remove(i);
        let kind = token.kind();
        if handle.ledger.settle(token) { settled.push(kind); }
    }
    for kind in &settled {
        trace!("[Ledger] settled owner={} group={} kind={:#x} reason={}", handle.owner, group, kind, reason);
    }
    settled
}

/// Native picked this R as free for a configure (`0x17e4700`: `R+0x18 == 0 && R+0x4c0 == 0`;
/// the caller read the empty map before `0x17e5bc0` registered anything). Every token the ledger
/// still holds for it predicts keys native does not have, so each is settled and reported: one
/// line per token, which a correct bank never prints (IC-PAIR-STALL). Returns how many.
pub(crate) fn settle_stale_on_empty(role: Role, reason: &str) -> usize {
    let mut slot = lock(role);
    let Some(handle) = slot.as_mut() else { return 0; };
    let stale: Vec<(usize, AcqToken)> = std::mem::take(&mut handle.held);
    let count = stale.len();
    for (group, token) in stale {
        let (kind, batch) = (token.kind(), token.batch());
        if handle.ledger.settle(token) {
            trace!("[Ledger] stale owner={} group={} kind={:#x} batch={} settled reason={}", handle.owner,
                if group == WINDOW { -1 } else { group as i64 }, kind, batch.0, reason);
        }
    }
    count
}

/// `R+0x4c |= 1` before a synchronous build when bit 0 is clear (GROUP-SWAP-IMPL §14.2), as a
/// lease the build ends with [`end_request`] once its construction is over (U19,
/// `swap_model::request_bit`). Returns the lease and the byte after the write.
///
/// # Safety
/// As for [`acquire`]; `r` is the proved alias.
pub(crate) unsafe fn request_bit(role: Role, r: usize) -> (bit0::Lease, u8) {
    let (lease, write) = bit0::begin(fields::read_unchecked::<u8>(r, 0x4c));
    if let Some(byte) = write { fields::write_unchecked(r, 0x4c, byte); }
    if let Some(handle) = lock(role).as_mut() { handle.ledger.request_bit = true; }
    (lease, fields::read_unchecked::<u8>(r, 0x4c))
}

/// End the build's lease: clear bit 0 when the build set it and the byte is still exactly what it
/// wrote. Native ends the bit with the reset when the last construction tail empties the map
/// (`0x17e8848–0x17e8880`); on the bank's R other batches' keys keep the map non-empty, and a bit
/// that outlived the build lets a later load completion `0x17e8440` take G68 copy keys for the
/// header members that the per-kind releases never return (U19). Returns `(how, byte before the
/// end, byte after)`.
///
/// # Safety
/// As for [`request_bit`], in the same callback, after the synchronous construction returned.
pub(crate) unsafe fn end_request(role: Role, r: usize, lease: bit0::Lease) -> (bit0::End, u8, u8) {
    let now = fields::read_unchecked::<u8>(r, 0x4c);
    let (how, write) = bit0::end(lease, now);
    if let Some(byte) = write { fields::write_unchecked(r, 0x4c, byte); }
    let after = fields::read_unchecked::<u8>(r, 0x4c);
    if let Some(handle) = lock(role).as_mut() { handle.ledger.request_bit = after & bit0::REQUEST != 0; }
    (how, now, after)
}

/// Astra §4.2 check only: may a pair-rule build of `group` reset R, i.e. does no other batch hold
/// temporary keys on it? `Err` names the other batches and logs `[Ledger] reset deferred …`.
pub(crate) fn reset_exclusive(role: Role, group: usize) -> Result<(), Vec<BatchId>> {
    let mut slot = lock(role);
    let Some(handle) = slot.as_mut() else { return Ok(()); };
    let batch = handle.batch(group);
    match handle.ledger.reset_batch_permit(batch) {
        Ok(_) => Ok(()),
        Err(others) => {
            trace!("[Ledger] reset deferred owner={} batch={} others={:?}", handle.owner, batch.0, others.iter().map(|b| b.0).collect::<Vec<_>>());
            Err(others)
        }
    }
}

/// `R+0x50 = references` for a pair-rule build of `group`, only under a reset-batch permit: no other
/// batch's temporary keys on R (Astra §4.2), or the tails' whole-R reset would discard them. `Err`
/// names the other batches and writes nothing (`[Ledger] reset deferred …`).
///
/// # Safety
/// As for [`request_bit`].
pub(crate) unsafe fn set_pair_refs(role: Role, r: usize, group: usize, references: u8) -> Result<u32, Vec<BatchId>> {
    let (owner, batch, permit) = {
        let mut slot = lock(role);
        let Some(handle) = slot.as_mut() else { return Err(Vec::new()); };
        let batch = handle.batch(group);
        (handle.owner, batch, handle.ledger.reset_batch_permit(batch))
    };
    match permit {
        Ok(_permit) => {
            let before = fields::read_unchecked::<u32>(r, 0x50);
            fields::write_unchecked(r, 0x50, u32::from(references));
            if let Some(handle) = lock(role).as_mut() { handle.ledger.pair_refs = Some(references); }
            Ok(before)
        }
        Err(others) => {
            trace!("[Ledger] reset deferred owner={} batch={} others={:?}", owner, batch.0, others.iter().map(|b| b.0).collect::<Vec<_>>());
            Err(others)
        }
    }
}

/// Nothing held by us and nothing recorded for native: a canonical Reload may be issued.
pub(crate) fn expect_free(role: Role) -> Result<(), Vec<u32>> {
    match lock(role).as_ref() {
        Some(handle) => handle.ledger.expect_free().map_err(|keys| keys.iter().map(|k| k.value()).collect()),
        None => Ok(()),
    }
}

/// Compare the ledger's prediction with the owner's actual `(key, refs)` pairs (WP2's observer walk)
/// and log one `[Ledger] reconcile` line; diagnostics only.
///
/// # Safety
/// As for `observe::walk_map`: the owner's task idle, the current callback.
pub(crate) unsafe fn reconcile(role: Role, phase: &str) {
    let (r, owner, predicted) = match lock(role).as_ref() {
        Some(handle) => (handle.r, handle.owner, handle.ledger.predicted()),
        None => return,
    };
    let map = fields::read::<u64>(r, 0x18).unwrap_or(0);
    let task_pending = fields::child(r, 0xa8).and_then(|task| fields::read::<u32>(task, 0x10)).unwrap_or(0);
    if task_pending != 0 {
        trace!("[Ledger] reconcile owner={} phase={} map={} task_pending={} predicted={:x?} walk=skipped", owner, phase, map, task_pending, predicted);
        return;
    }
    match observe::walk_map(r) {
        Some(nodes) => {
            let actual: Vec<(u32, u32)> = nodes.iter().map(|n| (n.key, n.refs)).collect();
            let result = match lock(role).as_ref() { Some(handle) => handle.ledger.reconcile(&actual), None => return };
            trace!("[Ledger] reconcile owner={} phase={} map={} task_pending=0 predicted={:x?} actual={:x?} residual={:x?} missing={:x?}",
                owner, phase, map, predicted, actual, result.residual, result.missing);
        }
        None => trace!("[Ledger] reconcile owner={} phase={} map={} task_pending=0 predicted={:x?} walk=invalid", owner, phase, map, predicted),
    }
}

/// Issue the ledger's calls in order. `Acquire` is issued by [`acquire`] only; `WritePairRefs` and
/// `SetRequestBit` by their own writers.
unsafe fn execute(base: usize, r: usize, calls: &[NativeCall]) {
    for call in calls {
        match call {
            NativeCall::ReleaseKey(key) => {
                let release: unsafe extern "C" fn(*mut u8, u32) = std::mem::transmute(base + RELEASE_KEY);
                release(r as *mut u8, key.value());
            }
            NativeCall::ReleaseKind { kind } => {
                let release: unsafe extern "C" fn(*mut u8, u32) = std::mem::transmute(base + RELEASE_KIND);
                release(r as *mut u8, *kind);
            }
            NativeCall::Acquire { .. } | NativeCall::WritePairRefs(_) | NativeCall::SetRequestBit => {}
        }
    }
}
