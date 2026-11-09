//! The fingerprinted native Mii-setup adapter (WP-M2, WORKPLAN §2.11 step 2; Astra's
//! seam, `redesign/astra/HUD.md` §4). A bank-built one-member Mii group replays the two
//! calls the state-6 loader makes for a Mii before its body is constructed:
//! `0x67e7d0(manager, owner, kind, 0, &tokens)` then `0x67ead0(owner, kind, &tokens)`
//! (`service-mii-rebuild.s` `0x651428-0x651454`). Without them the record keeps a zero
//! resource pair and the exact settle refuses (B124PRE D-MII1). The value rules, the
//! constants and the 13.0.5 words live in `crate::engine::mii::mii_setup`; this
//! file only reads, calls and re-reads, with every lock dropped (`0x67e7d0` and
//! `0x3759500` take G themselves) and under the transition's INSTALL/service authority.
//!
//! Never written here: `record+0x58/+0x60` (native owns the pair), `D+0x94`, `R+0x4c`.
//!
//! The loader clause is seam-specific (MII-PHASEB, `rules::Seam`): plan time needs the
//! entry Ready (6/14), Phase B needs the bank's own parked change (3/0xc, `0x6581f0`), the
//! bank's equivalent of the state-6 point where native makes these calls (`0x651444`).
use crate::engine::seams::native_fields as fields;
use crate::engine::swap::body_bank::MiiIdentity;
use crate::engine::mii::lifetime::Configuration;
use crate::engine::mii::mii_setup::{self as rules, EntryPhase, Outcome, Preflight, Seam, Tokens};

macro_rules! trace {
    ($($arg:tt)*) => { crate::engine::compat::logging::diagnostic(format_args!($($arg)*)) };
}

fn now_us() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// Everything the preflight read, kept for the `[MiiSetup]` lines and the post-read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reads {
    pub manager: usize,
    pub r: usize,
    pub record: usize,
    pub record_count: usize,
    pub tokens: Tokens,
    pub pair: (usize, usize),
    pub faces: (u32, u32),
    pub d_gate: u8,
    pub preflight: Preflight,
}

unsafe fn fingerprints(base: usize) -> bool {
    let check = |offset: usize, words: &[u32]| words.iter().enumerate()
        .all(|(i, w)| fields::read::<u32>(base, offset + i * 4) == Some(*w));
    check(rules::BIND_MII_RESOURCE, &rules::BIND_WORDS)
        && check(rules::SETUP_MII_FACES, &rules::FACES_WORDS)
        && check(rules::PUBLISH_PAIR, &rules::PUBLISH_WORDS)
        && check(rules::CHAIN_GATE, &rules::CHAIN_WORDS)
}

/// The entry's phase bytes (copied values).
unsafe fn read_phase(entry: usize) -> EntryPhase {
    let b = |offset: usize| fields::read::<u8>(entry, offset).unwrap_or(0xfe);
    EntryPhase { outer: b(rules::FE_OUTER), loader: b(rules::FE_LOADER), pending: b(rules::FE_PENDING),
        changing: b(rules::FE_CHANGING), guard: b(rules::FE_GUARD), acknowledged: b(rules::FE_ACKNOWLEDGED) }
}

/// Where a preflight runs: the seam (the entry phase it needs) and whether the bank has
/// installed the target identity yet (`R+0x68`/`FE+0x46` read live, else predicted).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum At {
    /// The adapter's plan arm (entry Ready).
    Plan,
    /// `perform_swap`'s reason chain, inside the parked change, before any window mutation.
    BeforeDetach,
    /// After the identity install, right before the two calls.
    IdentityInstalled,
    /// rc.6 item 3 (SPECIAL-GROUPS §1 (a)): a background build's tick, the identity installed for that one tick while
    /// the window stays live, so the entry is Ready (the plan seam; MII-PHASEB: neither call reads a loader byte or
    /// `R+0x4c`).
    Background,
}
impl At {
    fn seam(self) -> Seam { if matches!(self, At::Plan | At::Background) { Seam::Plan } else { Seam::PhaseB } }
    fn installed(self) -> bool { matches!(self, At::IdentityInstalled | At::Background) }
    fn label(self) -> &'static str {
        match self {
            At::Plan => "plan", At::BeforeDetach => "before-detach", At::IdentityInstalled => "identity-installed",
            At::Background => "background",
        }
    }
}

/// Copied values of every preflight input, in one pass, no lock held. `alias` is the
/// resource owner the bank module holds for `owner`; it must be the owner table's `R`.
/// `tokens` come from the identity the bank installs (`write_identity`), which is also
/// what `FE+0x46..0x50` holds once Phase B has written it; before that (plan time, and
/// Phase B's reason chain before any window mutation) the entry still shows the window's
/// halfwords, so `at` says which to use and which entry phase the seam needs.
pub(crate) unsafe fn read_preflight(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity,
    c: &Configuration, at: At) -> Option<Reads> {
    let (installed, seam) = (at.installed(), at.seam());
    let kind = identity.kinds[0];
    let tokens = Tokens { index: identity.index, outfit: identity.tokens[0], hat: identity.tokens[1],
        colour: identity.tokens[2], voice: identity.tokens[3] };
    let manager = fields::read::<usize>(base, rules::MANAGER_SLOT).and_then(|slot| fields::read::<usize>(slot, 0)).unwrap_or(0);
    let table = fields::read::<usize>(base, rules::OWNER_TABLE).unwrap_or(0);
    let r_of = |o: u32| -> usize { if table == 0 { 0 } else { fields::read::<usize>(table, rules::OWNER_TABLE_RESOURCE + o as usize * 8).unwrap_or(0) } };
    let r = r_of(owner);
    let mut other_r_indices = [None; 8];
    for (o, slot) in other_r_indices.iter_mut().enumerate() {
        if o as u32 == owner { continue; }
        let r_o = r_of(o as u32);
        if r_o == 0 { continue; }
        // Only a resource owner whose current kind is a Mii class claims a record.
        let claims = fields::read::<u32>(r_o, 0x58).is_some_and(|k| (0x48..=0x4a).contains(&k));
        if claims { *slot = fields::read::<u16>(r_o, rules::R_INDEX); }
    }
    let g = fields::read::<usize>(base, rules::POOL_HOLDER).and_then(|holder| fields::read::<usize>(holder, 0)).unwrap_or(0);
    let (begin, end) = if g == 0 { (0, 0) } else {
        (fields::read::<usize>(g, rules::VECTOR_BEGIN).unwrap_or(0), fields::read::<usize>(g, rules::VECTOR_END).unwrap_or(0))
    };
    let record_count = if begin != 0 && end >= begin { (end - begin) / rules::RECORD_STRIDE } else { 0 };
    let record = if usize::from(tokens.index) < record_count { begin + usize::from(tokens.index) * rules::RECORD_STRIDE } else { 0 };
    let (pair, charinfo_published) = if record == 0 { ((0, 0), false) } else {
        let pair = (fields::read::<usize>(record, rules::RECORD_PAIR).unwrap_or(0), fields::read::<usize>(record, rules::RECORD_PAIR + 8).unwrap_or(0));
        let payload = fields::read::<[u8; 88]>(record, 0);
        (pair, payload.is_some_and(|p| p == *c.charinfo()))
    };
    let d = fields::child(entry, 0xf8).unwrap_or(0);
    let d_gate = if d == 0 { u8::MAX } else { fields::read::<u8>(d, rules::D_GATE).unwrap_or(u8::MAX) };
    let fe_index = if installed { fields::read::<u16>(entry, rules::FE_TOKENS).unwrap_or(u16::MAX) } else { tokens.index };
    let r_index = if installed && r != 0 { fields::read::<u16>(r, rules::R_INDEX).unwrap_or(u16::MAX) } else { tokens.index };
    let faces = if r == 0 { (0, 0) } else { (fields::read::<u32>(r, rules::R_FACE_HANDLES).unwrap_or(0), fields::read::<u32>(r, rules::R_FACE_HANDLES + 4).unwrap_or(0)) };
    let phase = read_phase(entry);
    let preflight = Preflight {
        fingerprints: fingerprints(base), owner, kind, tokens, manager_present: manager != 0, record_count,
        r_present: r != 0 && r == alias, r_index, fe_index, d_gate, other_r_indices, phase, seam, charinfo_published,
    };
    Some(Reads { manager, r, record, record_count, tokens, pair, faces, d_gate, preflight })
}

fn trace_refused(reads: &Reads, at: At, reason: &str) {
    let p = &reads.preflight;
    trace!("[MiiSetup] refused owner={} kind={:#x} record={} R={:#x} r_index={} fe_index={} d_gate={} pair={:#x}/{:#x} records={} seam={} at={} phase=outer:{}/loader:{:#x}/pending:{:#x}/changing:{}/guard:{}/ack:{} reason={}",
        p.owner, p.kind, reads.tokens.index, reads.r, p.r_index, p.fe_index, reads.d_gate, reads.pair.0, reads.pair.1,
        reads.record_count, p.seam.label(), at.label(), p.phase.outer, p.phase.loader, p.phase.pending, p.phase.changing,
        p.phase.guard, p.phase.acknowledged, reason);
}

/// Plan-time admission (the interim canonical-before-mutation rule, WORKPLAN §2.11
/// step 4): the preflight evaluated with the identity the bank would install, the entry
/// Ready. Every clause but the phase is the one Phase B re-proves (`rules::Seam`).
pub(crate) unsafe fn admitted(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity, c: &Configuration)
    -> Result<(), &'static str> {
    let reads = read_preflight(base, entry, owner, alias, identity, c, At::Plan).ok_or("Mii setup: preflight reads failed")?;
    reads.preflight.check()
}

/// Phase B before any window mutation (`perform_swap`'s reason chain): the same preflight
/// with the identity the build will install and the entry in the bank's parked change. A
/// refusal here leaves the window attached (the chain's fallback activates the committed
/// in-window member); nothing is detached or rolled back.
pub(crate) unsafe fn phase_b_admitted(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity,
    c: &Configuration) -> Result<(), &'static str> {
    let reads = read_preflight(base, entry, owner, alias, identity, c, At::BeforeDetach)
        .ok_or("Mii setup: preflight reads failed")?;
    reads.preflight.check().inspect_err(|reason| trace_refused(&reads, At::BeforeDetach, reason))
}

/// Phase B: after the target's R/FE/E/D identity is installed and before
/// `sync_construct`. Re-proves the preflight on the live words (`R+0x68` and
/// `FE+0x46` must now be the target index), runs the two calls, re-reads the pair,
/// the face handles and the payload, and classifies. `Refused` means nothing native
/// ran; `Failed` means a native effect crossed (the recovery path).
pub(crate) unsafe fn prepare_mii_group(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity,
    c: &Configuration) -> Outcome {
    prepare_at(base, entry, owner, alias, identity, c, At::IdentityInstalled).0
}

/// rc.6 item 3: what a background setup changed in the group's selector record: the whole record before and after
/// (`0x67e7d0` publishes its pair `+0x58..+0x68` through `0x3759500`; the CharInfo stays). The owner adapters, idle
/// meanwhile, adopt it into their snapshots (`adopt_background_record`), as a switch-time build's settle adopts it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RecordChange {
    pub index: u16,
    pub before: [u8; crate::engine::mii::native_values::RECORD_LEN],
    pub after: [u8; crate::engine::mii::native_values::RECORD_LEN],
}

/// rc.6 item 3: the same two calls in a background build's tick (`At::Background`), the identity installed for that
/// tick and the window's face handles held aside (`native_body_bank::start_background`), with the record change.
pub(crate) unsafe fn prepare_mii_background(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity,
    c: &Configuration) -> (Outcome, Option<RecordChange>) {
    prepare_at(base, entry, owner, alias, identity, c, At::Background)
}

unsafe fn prepare_at(base: usize, entry: usize, owner: u32, alias: usize, identity: &MiiIdentity, c: &Configuration,
    at: At) -> (Outcome, Option<RecordChange>) {
    use crate::engine::mii::native_values::RECORD_LEN;
    let Some(reads) = read_preflight(base, entry, owner, alias, identity, c, at) else {
        return (Outcome::Refused("Mii setup: preflight reads failed"), None);
    };
    if let Err(reason) = reads.preflight.check() {
        trace_refused(&reads, at, reason);
        return (Outcome::Refused(reason), None);
    }
    let record_before = fields::read::<[u8; RECORD_LEN]>(reads.record, 0);
    let started = now_us();
    let tokens = reads.tokens;
    let bind: unsafe extern "C" fn(usize, u32, u32, u32, *const Tokens) = std::mem::transmute(base + rules::BIND_MII_RESOURCE);
    let faces: unsafe extern "C" fn(u32, u32, *const Tokens) = std::mem::transmute(base + rules::SETUP_MII_FACES);
    bind(reads.manager, owner, reads.preflight.kind, 0, &tokens);
    faces(owner, reads.preflight.kind, &tokens);
    let elapsed = now_us().saturating_sub(started);
    let pair_after = (fields::read::<usize>(reads.record, rules::RECORD_PAIR).unwrap_or(0),
        fields::read::<usize>(reads.record, rules::RECORD_PAIR + 8).unwrap_or(0));
    let faces_after = (fields::read::<u32>(reads.r, rules::R_FACE_HANDLES).unwrap_or(0),
        fields::read::<u32>(reads.r, rules::R_FACE_HANDLES + 4).unwrap_or(0));
    let intact = fields::read::<[u8; 88]>(reads.record, 0).is_some_and(|p| p == *c.charinfo());
    let outcome = rules::classify(pair_after, faces_after, intact);
    match outcome {
        Outcome::Prepared { .. } => trace!("[MiiSetup] prepared owner={} kind={:#x} record={} R={:#x} pair={:#x}/{:#x}->{:#x}/{:#x} faces={:#x}/{:#x}->{:#x}/{:#x} d_gate={} us={}",
            owner, reads.preflight.kind, tokens.index, reads.r, reads.pair.0, reads.pair.1, pair_after.0, pair_after.1,
            reads.faces.0, reads.faces.1, faces_after.0, faces_after.1, reads.d_gate, elapsed),
        Outcome::Failed(reason) | Outcome::Refused(reason) => trace!("[MiiSetup] failed owner={} kind={:#x} record={} R={:#x} pair={:#x}/{:#x}->{:#x}/{:#x} faces={:#x}/{:#x} intact={} us={} reason={}",
            owner, reads.preflight.kind, tokens.index, reads.r, reads.pair.0, reads.pair.1, pair_after.0, pair_after.1,
            faces_after.0, faces_after.1, intact, elapsed, reason),
    }
    let change = record_before.zip(fields::read::<[u8; RECORD_LEN]>(reads.record, 0))
        .map(|(before, after)| RecordChange { index: tokens.index, before, after });
    (outcome, change)
}
