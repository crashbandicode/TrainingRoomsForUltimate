//! Value-only end of an owned helper-source load, and the waiting note of a visible request queued
//! behind the shared install gate (FAIL 2: Yoga `icp1d`, M5c diag `756883e2…`, CPU step 8).
//!
//! FACTs (evidence in `refactor/fail2-mii-stall/evidence/`):
//! - `#413 install slot=7 kind=0x44` committed a speculative replacement load of the CPU original on the
//!   created entry 7. `#424 stalled … snapshot=Some(true) scene=Some(false) original_resources=Some(true)
//!   inputs=Some(false) ready=Some(true) native_done=true`: native finished the install, and the entry's own
//!   actors, controls and archives were ready (`capacity::ready`). Only the managed proof that nothing else
//!   in the scene changed during the load failed (`native_source_capacity.rs`, `stalled_checks`).
//! - The same signature on 2026-10-18 (LDN05 audit, slot 7 kind 0x25) came from another owner's legitimate
//!   input change during the load.
//! - `character_sources.rs` (`service_owned`) failed a native-finished load after 900 callbacks only on the
//!   two native helper entries. A created entry was excluded, kept its INSTALL token for the rest of the
//!   scene and logged once. The CPU's next cycle request was then accepted with no line
//!   (`mii_ordinary_cpu::intercept`) and its `begin` returned at `permitted()` (`!INSTALL.active()`) on every
//!   callback with no line, so it was never served nor refused.
//! - `SourceBook::fail` is the book state for "native completion is proven, but usable resources were not
//!   obtained", and a failed slot may be replaced in place.
//!
//! Rule: a load that native has completed and whose managed proof still fails after [`STALL_TICKS`]
//! callbacks is failed and its INSTALL token released, on every helper entry. For a created entry, native
//! completion means `native_done` **and** the entry's own actor/archive readiness. A load native has not
//! completed keeps its token and logs once (native keeps ownership, as before). So do departed loads and
//! exact Mii helper loads, which have their own completion protocol.
//!
//! Scene-proof follow-up (M5e diag, LDN waiting room, sessions l1/l1b): every alternate-bank load (PT, Aegis) on
//! a created extra other than entry 7 failed its scene proof, because the waiting snapshot's input member is
//! entry 7's view and saw the load's own expected route change ([`waiting_member_proven`]). A created entry's
//! failure is an attempt failure, not a kind failure, so the planner retries it ([`Unfinished::fails_kind`]).

/// Loader callbacks after which a native-completed load whose managed proof fails is ended.
pub const STALL_TICKS: u16 = 900;

/// One owned load at one of its own loader callbacks, after its completion proof failed this callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unfinished {
    /// Loader callbacks since the install was committed.
    pub ticks: u16,
    /// `native_source::parked_ready`: native's loader is idle on the entry with the requested bodies.
    pub native_done: bool,
    /// Created entries only: the entry's own actors, controls and archives are ready (`capacity::ready`).
    pub entry_ready: bool,
    /// The load's scene epoch is no longer current.
    pub departed: bool,
    /// A created (capacity) entry rather than one of the two native helper entries.
    pub created: bool,
    /// An exact Mii helper load; its selector transaction owns its completion.
    pub exact_mii: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Keep waiting for the completion proof.
    Wait,
    /// Native completed the load and the managed proof did not hold: `SourceBook::fail`, release INSTALL.
    Fail,
    /// Once, at [`STALL_TICKS`]: native has not completed; native keeps ownership and the token is retained.
    Retain,
}

impl Unfinished {
    /// Native completion: the loader is done, and on a created entry its own actors and archives are ready.
    pub fn native_completed(self) -> bool {
        self.native_done && (!self.created || self.entry_ready)
    }

    /// Whether a [`Verdict::Fail`] retires the kind for the rest of the scene (`failed` bit, demand cleared).
    /// A native helper fails only when native completed **without usable resources** (`native_source::ready`
    /// never held): a property of the kind, so retrying it in this scene cannot help. A created entry fails
    /// only when native completed **with** usable resources (`entry_ready`) and the scene proof did not hold:
    /// a property of that one attempt, so the slot is failed (replaceable in place) and the planner retries
    /// the kind. Scene-proof follow-up (l1, l1b): marking the kind left the waiting room without its PT
    /// helper for the rest of the visit.
    pub fn fails_kind(self) -> bool {
        !self.created
    }
}

pub fn verdict(load: Unfinished) -> Verdict {
    if load.native_completed() && !load.departed && !load.exact_mii && load.ticks >= STALL_TICKS {
        Verdict::Fail
    } else if load.ticks == STALL_TICKS {
        Verdict::Retain
    } else {
        Verdict::Wait
    }
}

/// One entry's input view as the scene proof sees it (`native_source_capacity::inputs_for(slot)`):
/// - `rows`: the 16 route records (pointer, state, key, owner, alias). They are global, identical in every
///   entry's view, and a load changes its own entry's route as `canonical::loaded_input` expects (an
///   alternate bank, PT and Aegis, sets its own row's state 2 -> 0);
/// - `shared`: the input service, data, records and owner-map identities and the CPU flag, also identical in
///   every view;
/// - `own`: this entry's mapping, mapped controller, its property and flag, and its row's CSS role. Native
///   reads these only when it builds this entry's own actor controls (`0x6b6f10` reads
///   `mapping[0x298 + 4*entry]`, the controller `main+0x5339860[map]` or the fallback `main+0x5339830`, and the
///   flag `*(controller+0x18)+8`), so they belong to this entry's loads only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputView<R, S, O> {
    pub slot: usize,
    pub rows: [R; 16],
    pub shared: S,
    pub own: O,
}

/// The waiting-room snapshot's input member (`Snapshot::ldn.input`) is the legacy entry-7 view
/// (`inputs()`), whatever entry is loading; the entry-7 probes read it as that. For a load on `expected.slot`,
/// it is proven, and compared as its prior value, when:
/// - it is the loading entry's own view, equals the load's expected view, and its prior value is the load's
///   preflight view (the rule before this change, which substituted `before` and then needed `prior == before`);
/// - or it is another entry's view, it was taken at the same preflight (rows and shared fields equal the load's
///   preflight view `before`), and its rows and shared fields now equal the load's expected view, which the
///   load's own input contract proves. That entry's `own` fields are not read by native for this load.
///
/// FACT (l1, l1b, M5e diag, LDN waiting room, host alone 2/2 boots): `install slot=2 kind=0x24`, then
/// `stalled … scene_diff_hex=Some(100000) input_rows_hex=Some(0) input_fields_equal=Some(true)`: bit 20 is
/// this member. Its row 2 went 2 -> 0 because the PT load set its own route, exactly as the input contract
/// expected, and the rule before this change accepted that only when entry 7 itself was loading.
pub fn waiting_member_proven<R: PartialEq, S: PartialEq, O: PartialEq>(
    prior: &InputView<R, S, O>,
    current: &InputView<R, S, O>,
    before: &InputView<R, S, O>,
    expected: &InputView<R, S, O>,
) -> bool {
    if current.slot == expected.slot {
        return current == expected && prior == before;
    }
    prior.slot == current.slot
        && prior.rows == before.rows
        && prior.shared == before.shared
        && current.rows == expected.rows
        && current.shared == expected.shared
}

/// What a visible Mii role adapter's `permitted()` waits on, in its own order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    Backend,
    ReadyGo,
    Menu,
    NativeUi,
    Shortcut,
    Install,
    SaveState,
    Recording,
    ReplayPreview,
    Roster,
    /// The role's HUD portrait job holds a helper lease or has its render issued (Eden covrest-12).
    Portrait,
}

impl Blocker {
    pub fn reason(self) -> &'static str {
        match self {
            Blocker::Backend => "Mii backend not ready",
            Blocker::ReadyGo => "match not at ready-go",
            Blocker::Menu => "menu captures input",
            Blocker::NativeUi => "native UI busy",
            Blocker::Shortcut => "another shortcut pending",
            Blocker::Install => "INSTALL held",
            Blocker::SaveState => "save-state restore",
            Blocker::Recording => "recording or playback",
            Blocker::ReplayPreview => "replay preview",
            Blocker::Roster => "roster transaction",
            Blocker::Portrait => "portrait render on a helper",
        }
    }
}

/// Once-per-change note for one queued request: the same request waiting on the same blocker on every
/// callback is noted once; a different blocker or request is noted again; no blocker re-arms it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Waiting<R> {
    last: Option<(R, Blocker)>,
}

impl<R: Copy + PartialEq> Waiting<R> {
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// `Some(blocker)` when this callback's (request, blocker) differs from the last one noted.
    pub fn note(&mut self, request: R, blocker: Option<Blocker>) -> Option<Blocker> {
        let now = blocker.map(|blocker| (request, blocker));
        if now == self.last {
            return None;
        }
        self.last = now;
        blocker
    }
}

impl<R: Copy + PartialEq> Default for Waiting<R> {
    fn default() -> Self {
        Self::new()
    }
}

