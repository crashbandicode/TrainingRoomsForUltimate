//! Native Mii resource setup for a bank-built one-member Mii group (WP-M2, WORKPLAN
//! §2.11 step 2): the value rules of the fingerprinted adapter that runs the canonical
//! loader's two Mii-setup calls before `sync_construct`. Copied values only; the Skyline
//! half (`src/swap_native/mii_setup.rs`) reads, calls and re-reads.
//!
//! FACTs this module encodes (13.0.5 main ELF `0789f0a4…`; listings
//! `plan2-mii-owner-loading01/native/{service-mii-rebuild,owner-record-bind,
//! owner-record-bind-tail,pool-build-publication}.s`, `refactor/wp-m2/re/faces_67ead0.asm`;
//! B124PRE-MII D-MII1; `redesign/astra/HUD.md` §4):
//! - Loader state 6, `0x65141c-0x651458`: when `D+0x94 == 0` it calls
//!   `0x67e7d0(manager = **(main+0x52b94f8), owner, kind, 0, &tokens)` then
//!   `0x67ead0(owner, kind, &tokens)`, `tokens` = the five FE halfwords `+0x46..0x50`
//!   (index, outfit, hat, colour, voice), and only then sets loader state 7.
//! - `0x67e7d0`: kind outside `0x48..=0x4a` → returns; `tokens[0]` is bounds-checked
//!   against the selector vector (`G+0x24d0..0x24d8`, 0x68-byte records) and the
//!   out-of-range path throws (`0x39c1330`); the target record's CharInfo must validate
//!   (`0x39c2930/0x39c2940`) or it returns; owner ≥ 8 throws; `R = *(*(main+0x5324680)
//!   + owner*8 + 0xe8)`; when `R+0x68 <= 9` the record at `R+0x68` has its pair zeroed
//!   and its control block released under G; the face handles `R+0x90/0x94` are
//!   released and zeroed; the tail is `0x3759500(G, tokens[0], slot_kind)`.
//! - `0x3759500`: locks G, builds the payload and control block, publishes the pair
//!   with `stp x28, x26, [record+0x58]` at `0x375a9ec`, releases the old pair, unlocks.
//! - `0x67ead0`: same kind/owner/R/index/CharInfo gates, and only when `R+0x90 == 0`
//!   builds the face handles `0x359f860(index, 0x100, 0x100, 0, 0, 0)` into `R+0x90/0x94`.
//! - A bank group exchange runs none of this (no Reload): the record keeps the
//!   published CharInfo and a zero pair, which M3 refused (`native Mii resource pair
//!   absent`, `b124pre-mii/PROGRESS.md:28`).
//!
//! The loader clause (MII-PHASEB, `refactor/mii-phaseb/re/`): neither call nor `0x3759500`
//! reads an entry loader byte or `R+0x4c`; `0x67e7d0` reads the entry only through the
//! manager (`FE+0x24`, slot kind, `0x67ea3c-0x67ea64`). Native runs them with its own loader
//! suspended at state 6 of a Reload (`+0x5920 == 3`, `+0x591c == 6`), after `R` is configured
//! and before any body is constructed. The bank's equivalent is its own parked change: the
//! commit `0x6581f0` (only at `+0x5920 == 6`, `0x65821c`) writes `+0x5920 = 3` (`0x658570`),
//! `+0x5934 = 1` (`0x658584`) and, with the bank flag, `+0x591c = 0xc` (`0x6585a4`),
//! `+0x5933 = 0` (`0x6585ac`), then `+0x591a = 0xff` (`0x6585d4`); `0x650010` does nothing for
//! `+0x591c > 0xb` (`0x650050`), so no native step runs for the entry until the bank
//! acknowledges. Plan time sees Ready (6/14), the only phase `switch` accepts. The clause is
//! therefore seam-specific ([`Seam`], [`EntryPhase`]); "idle" at Phase B could never hold.

/// `0x67e7d0`: bind the owner's native Mii resource for the record named by `tokens[0]`.
pub const BIND_MII_RESOURCE: usize = 0x67e7d0;
/// `0x67ead0`: build the owner's face texture handles for that record.
pub const SETUP_MII_FACES: usize = 0x67ead0;
/// `0x3759500`: the pool publication `0x67e7d0` tails into; stores the pair at `0x375a9ec`.
pub const PUBLISH_PAIR: usize = 0x3759500;
/// The `D+0x94` gate that guards the chain (`ldr x8,[x19,#0xf8]; ldrb w8,[x8,#0x94]; cbnz`).
pub const CHAIN_GATE: usize = 0x65141c;
/// `**(main+0x52b94f8)` is the first argument of `0x67e7d0`.
pub const MANAGER_SLOT: usize = 0x52b94f8;
/// Owner table: `R = *(*(main+0x5324680) + owner*8 + 0xe8)`.
pub const OWNER_TABLE: usize = 0x5324680;
pub const OWNER_TABLE_RESOURCE: usize = 0xe8;
/// Selector pool holder: `G = **(main+0x593b6a0)`, vector at `G+0x24d0..0x24d8`.
pub const POOL_HOLDER: usize = 0x593b6a0;
pub const VECTOR_BEGIN: usize = 0x24d0;
pub const VECTOR_END: usize = 0x24d8;
pub const RECORD_STRIDE: usize = 0x68;
pub const RECORD_PAIR: usize = 0x58;
pub const R_INDEX: usize = 0x68;
pub const R_FACE_HANDLES: usize = 0x90;
pub const D_GATE: usize = 0x94;
pub const FE_TOKENS: usize = 0x46;
/// Entry phase bytes: outer `+0x5920`, loader `+0x591c`, pending request `+0x591a`,
/// change in progress `+0x5934`, guard `+0x5935`, the bank's acknowledgement `+0x5933`.
pub const FE_OUTER: usize = 0x5920;
pub const FE_LOADER: usize = 0x591c;
pub const FE_PENDING: usize = 0x591a;
pub const FE_CHANGING: usize = 0x5934;
pub const FE_GUARD: usize = 0x5935;
pub const FE_ACKNOWLEDGED: usize = 0x5933;

/// Copied values of the entry's phase bytes (the `FE_*` offsets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryPhase { pub outer: u8, pub loader: u8, pub pending: u8, pub changing: u8, pub guard: u8, pub acknowledged: u8 }

impl EntryPhase {
    /// Ready: outer 6, loader 14, no pending request, no change, no guard. The only phase
    /// from which the bank accepts a member switch (`Members::ready_index`).
    #[cfg(test)]
    pub const READY: Self = Self { outer: 6, loader: 14, pending: 0xff, changing: 0, guard: 0, acknowledged: 1 };
    /// The bank's own in-window change as `0x6581f0` leaves it on the bank flag (outer 3,
    /// loader 0xc, request consumed, change marked, not yet acknowledged): `awaiting_completion`.
    #[cfg(test)]
    pub const BANK_PARKED: Self = Self { outer: 3, loader: 0xc, pending: 0xff, changing: 1, guard: 0, acknowledged: 0 };

    pub fn ready(&self) -> bool {
        self.outer == 6 && self.loader == 14 && self.pending == 0xff && self.changing == 0 && self.guard == 0
    }
    pub fn bank_change_parked(&self) -> bool {
        self.outer == 3 && self.loader == 0xc && self.pending == 0xff && self.changing == 1 && self.guard == 0
            && self.acknowledged == 0
    }
    pub fn admits(&self, seam: Seam) -> bool {
        match seam { Seam::Plan => self.ready(), Seam::PhaseB => self.bank_change_parked() }
    }
}

/// Where the preflight is evaluated. Every clause but the phase is the same at both seams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seam {
    /// The adapter's plan arm, before INSTALL and the switch request: the entry is Ready.
    Plan,
    /// `perform_swap`: the reason chain before any window mutation (identity predicted) and
    /// the re-proof after the identity install, both inside the bank's parked change.
    PhaseB,
}

impl Seam {
    pub const fn phase_reason(self) -> &'static str {
        match self {
            Seam::Plan => "Mii setup: entry not Ready (plan needs loader 6/14, no request)",
            Seam::PhaseB => "Mii setup: entry not parked in the bank's own change (3/0xc)",
        }
    }
    pub const fn label(self) -> &'static str { match self { Seam::Plan => "plan", Seam::PhaseB => "phase-b" } }
}

/// The first 64 bytes of every called function and of the call chain, as 13.0.5 words
/// (`refactor/wp-m2/re/fingerprint-words.txt`, extracted from the ELF).
pub const BIND_WORDS: [u32; 16] = [
    0xd10483ff, 0xa90c6ffc, 0xa90d67fa, 0xa90e5ff8, 0xa90f57f6, 0xa9104ff4, 0xa9117bfd, 0x910443fd,
    0x51012048, 0x7100091f, 0x54001008, 0xb00295f9, 0xf9435329, 0xf9400129, 0xf9526d2b, 0xf952692a,
];
pub const FACES_WORDS: [u32; 16] = [
    0xd10343ff, 0xa90b4ff4, 0xa90c7bfd, 0x910303fd, 0x51012028, 0x7100211f, 0x54000088, 0x528038e9,
    0x1ac82529, 0x370000a9, 0xa94c7bfd, 0xa94b4ff4, 0x910343ff, 0xd65f03c0, 0x93407c09, 0xf100213f,
];
pub const PUBLISH_WORDS: [u32; 16] = [
    0xd10743ff, 0xa9176ffc, 0xa91867fa, 0xa9195ff8, 0xa91a57f6, 0xa91b4ff4, 0xa91c7bfd, 0x910703fd,
    0x2a0203f3, 0x2a0103f4, 0xaa0003f6, 0x9409a125, 0x52849a08, 0x8b0802d7, 0xf9526ec9, 0xf9526ac8,
];
/// `0x65141c..0x651448`: the gate and the chain, up to and including `bl 0x67e7d0`.
pub const CHAIN_WORDS: [u32; 11] = [
    0xf9407e68, 0x39425108, 0x350001a8, 0x90026348, 0xf9427d08, 0xb9400321, 0xb9400282, 0x9100e3e4,
    0x2a1f03e3, 0xf9400100, 0x9400b4e3,
];

/// The native token block `0x67e7d0`/`0x67ead0` read through `x4`/`x2`: the five FE
/// halfwords in native order. Built from the identity the bank installed, never cast
/// from a Rust configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Tokens { pub index: u16, pub outfit: u16, pub hat: u16, pub colour: u16, pub voice: u16 }

impl Tokens {
    #[cfg(test)]
    pub fn from_entry_halfwords(halfwords: [u16; 5]) -> Self {
        Self { index: halfwords[0], outfit: halfwords[1], hat: halfwords[2], colour: halfwords[3], voice: halfwords[4] }
    }
}

/// Everything the adapter proves before the first native effect, as copied values.
/// `check` encodes the destructive preconditions of `0x67e7d0` (WORKPLAN §2.11 step 2
/// preflight): the retirement at `R+0x68` and the publication at `tokens[0]` must name
/// the same assigned owned record, and no other live owner may claim it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preflight {
    pub fingerprints: bool,
    pub owner: u32,
    pub kind: u32,
    pub tokens: Tokens,
    pub manager_present: bool,
    pub record_count: usize,
    pub r_present: bool,
    pub r_index: u16,
    pub fe_index: u16,
    pub d_gate: u8,
    /// `R+0x68` of every other owner whose resource owner currently holds a Mii class
    /// (`R+0x58` in `0x48..=0x4a`); `None` for no R or an ordinary body, whose zero
    /// index is not a Mii claim (`plan2-mii-owner-loading01/REPORT.md` visit 2).
    pub other_r_indices: [Option<u16>; 8],
    /// The entry's phase bytes and the seam they are judged at ([`EntryPhase::admits`]):
    /// Ready at plan time, the bank's parked change at Phase B. No native step for this
    /// entry can run in either phase without the bank (MII-PHASEB).
    pub phase: EntryPhase,
    pub seam: Seam,
    /// The record's CharInfo equals the published configuration's.
    pub charinfo_published: bool,
}

impl Preflight {
    pub fn check(&self) -> Result<(), &'static str> {
        if !self.fingerprints { return Err("Mii setup: native fingerprints differ"); }
        if !(0x48..=0x4a).contains(&self.kind) { return Err("Mii setup: kind is not a Mii class"); }
        if self.owner >= 8 { return Err("Mii setup: owner outside the native owner table"); }
        if !self.manager_present { return Err("Mii setup: manager singleton absent"); }
        if !self.r_present { return Err("Mii setup: resource owner absent"); }
        if usize::from(self.tokens.index) >= self.record_count { return Err("Mii setup: record index outside the selector vector"); }
        if !self.phase.admits(self.seam) { return Err(self.seam.phase_reason()); }
        if self.d_gate != 0 { return Err("Mii setup: D rebuild gate set; canonical path owns this owner"); }
        if self.r_index != self.tokens.index || self.fe_index != self.tokens.index {
            return Err("Mii setup: R+0x68, FE+0x46 and the target index differ");
        }
        if self.other_r_indices.iter().enumerate()
            .any(|(owner, index)| owner as u32 != self.owner && *index == Some(self.tokens.index)) {
            return Err("Mii setup: another owner's resource names the target record");
        }
        if !self.charinfo_published { return Err("Mii setup: record CharInfo is not the published configuration"); }
        Ok(())
    }
}

/// What the adapter reports after the two calls. `Refused` happened before any native
/// effect (a preflight clause); `Failed` after (a crossed effect: the recovery path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Prepared { pair: (usize, usize), faces: (u32, u32) },
    Refused(&'static str),
    Failed(&'static str),
}

/// After `0x67e7d0` → `0x67ead0`: the pair must be present and the published CharInfo
/// untouched (`0x3759500` copies it into the resource; it never rewrites the record
/// payload). Face handles are reported, not required: `0x67ead0` builds them only when
/// `R+0x90` was zero, and the HUD portrait is a separate helper render.
pub fn classify(pair_after: (usize, usize), faces_after: (u32, u32), charinfo_intact: bool) -> Outcome {
    if pair_after.0 == 0 || pair_after.1 == 0 { return Outcome::Failed("Mii setup: native resource pair absent after setup"); }
    if !charinfo_intact { return Outcome::Failed("Mii setup: record CharInfo changed under setup"); }
    Outcome::Prepared { pair: pair_after, faces: faces_after }
}

