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

#[cfg(test)]
mod tests {
    use super::*;

    /// Eden covrest-12 (2026-11-08): the CPU's next press was submitted in the frame the CPU's HUD portrait render was
    /// issued on helper 2; the press's snapshot refused the row mid-render, and the portrait's after-render proof then
    /// quarantined helper 2 for the scene, so presses 1-5 loaded nothing. A Mii owner's begin now waits while its
    /// role's portrait job holds a helper lease or has a render issued, and the press applies after it.
    #[test]
    fn a_press_during_a_portrait_render_waits_and_then_applies() {
        assert_eq!(Blocker::Portrait.reason(), "portrait render on a helper");
        let mut waiting = Waiting::new();
        let press = 7u32;
        assert_eq!(waiting.note(press, Some(Blocker::Portrait)), Some(Blocker::Portrait), "the press waits");
        assert_eq!(waiting.note(press, Some(Blocker::Portrait)), None, "noted once while it waits");
        assert_eq!(waiting.note(press, None), None, "the render ended: the press applies");
        let cpu = include_str!("../../runtime/mii/mii_ordinary_cpu.rs");
        assert!(cpu.contains("else if portrait_render_held() {Some(Blocker::Portrait)}"));
        assert!(cpu.contains("super::mii_hud_portrait::render_holds_helper(1)"));
        let player = include_str!("../../runtime/mii/mii_ordinary_player.rs");
        assert!(player.contains("super::mii_hud_portrait::render_holds_helper(0)"));
        let portrait = include_str!("../../runtime/mii/mii_hud_portrait.rs");
        assert!(portrait.contains("pub(crate) fn render_holds_helper(logical: u32) -> bool"));
    }

    /// M5c's rule (`character_sources.rs` `service_owned` before this change): a created entry never failed.
    fn m5c_verdict(load: Unfinished) -> Verdict {
        if load.native_done && !load.exact_mii && load.ticks >= STALL_TICKS && !load.departed && !load.created {
            Verdict::Fail
        } else if load.ticks == STALL_TICKS {
            Verdict::Retain
        } else {
            Verdict::Wait
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Holder {
        Source { slot: u32, kind: u32 },
        Cpu,
    }

    fn holder_text(holder: Option<Holder>) -> String {
        match holder {
            Some(Holder::Source { slot, kind }) => format!("source slot={} kind={:#x}", slot, kind),
            Some(Holder::Cpu) => "cpu reload".into(),
            None => "nothing".into(),
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Request(u32);

    #[derive(Clone, Copy)]
    struct Load {
        slot: u32,
        kind: u32,
        ticks: u16,
        native_done_at: u16,
        /// The whole-scene comparison of `capacity::loaded` (F1): another owner changed the scene.
        proof_holds: bool,
    }

    /// The shared install gate, one created-entry load and the CPU adapter's single request slot, stepped
    /// once per frame: the load's own callback, then the CPU entry's callback (`mii_ordinary_cpu::service`).
    struct World {
        rule: fn(Unfinished) -> Verdict,
        gate: Option<Holder>,
        load: Option<Load>,
        request: Option<Request>,
        waiting: Waiting<Request>,
        served: Vec<(u32, Request)>,
        lines: Vec<(u32, String)>,
        frame: u32,
        cpu_reload_frames: u32,
    }

    impl World {
        fn icp1d(rule: fn(Unfinished) -> Verdict, proof_holds: bool) -> Self {
            // #413: the replacement load of Richter on entry 7 takes INSTALL at frame 0.
            World::load(rule, 7, 0x44, 24, proof_holds)
        }

        fn load(rule: fn(Unfinished) -> Verdict, slot: u32, kind: u32, native_done_at: u16, proof_holds: bool) -> Self {
            World {
                rule,
                gate: Some(Holder::Source { slot, kind }),
                load: Some(Load { slot, kind, ticks: 0, native_done_at, proof_holds }),
                request: None,
                waiting: Waiting::new(),
                served: Vec::new(),
                lines: Vec::new(),
                frame: 0,
                cpu_reload_frames: 0,
            }
        }

        /// ZL+Left: `intercept` accepts when no request is pending (one line either way, as patched).
        fn press(&mut self, request: Request) {
            if self.request.is_none() {
                self.request = Some(request);
                self.lines.push((self.frame, format!("request accepted #{}", request.0)));
            } else {
                self.lines.push((self.frame, "request refused: owned CPU request busy".into()));
            }
        }

        fn step(&mut self) {
            self.frame += 1;
            if let Some(mut load) = self.load {
                load.ticks += 1;
                let native_done = load.ticks >= load.native_done_at;
                if native_done && load.proof_holds {
                    self.lines.push((self.frame, format!("ready slot={}", load.slot)));
                    self.load = None;
                    self.gate = None;
                } else {
                    let facts = Unfinished {
                        ticks: load.ticks, native_done, entry_ready: native_done, departed: false,
                        created: true, exact_mii: false,
                    };
                    match (self.rule)(facts) {
                        Verdict::Fail => {
                            self.lines.push((self.frame, format!("stalled slot={} kind={:#x}; load failed, INSTALL released", load.slot, load.kind)));
                            self.load = None;
                            self.gate = None;
                        }
                        Verdict::Retain => {
                            self.lines.push((self.frame, format!("stalled slot={} kind={:#x}; native ownership retained", load.slot, load.kind)));
                            self.load = Some(load);
                        }
                        Verdict::Wait => self.load = Some(load),
                    }
                }
            }
            if self.gate == Some(Holder::Cpu) {
                self.cpu_reload_frames += 1;
                if self.cpu_reload_frames == 40 {
                    self.gate = None;
                }
            }
            if let Some(request) = self.request {
                let blocker = self.gate.map(|_| Blocker::Install);
                if let Some(blocker) = self.waiting.note(request, blocker) {
                    self.lines.push((self.frame, format!("request waiting: {} by {}", blocker.reason(), holder_text(self.gate))));
                }
                if blocker.is_none() {
                    self.served.push((self.frame, request));
                    self.request = None;
                    self.gate = Some(Holder::Cpu);
                    self.cpu_reload_frames = 0;
                }
            }
        }

        fn run_to(&mut self, frame: u32) {
            while self.frame < frame {
                self.step();
            }
        }
    }

    // icp1d frames after the commit at Eden rel 139.17 s: step 8 at 143.32 s, the second press at 264.68 s.
    const STEP8: u32 = 249;
    const SECOND: u32 = 7_531;

    #[test]
    fn m5c_rule_keeps_the_created_load_and_its_gate_for_the_whole_scene() {
        let mut w = World::icp1d(m5c_verdict, false);
        w.run_to(STEP8);
        w.press(Request(8));
        w.run_to(SECOND);
        // #424: the one stall line, INSTALL kept; the step-8 request is never served.
        assert_eq!(w.gate, Some(Holder::Source { slot: 7, kind: 0x44 }));
        assert!(w.served.is_empty());
        let stalls: Vec<_> = w.lines.iter().filter(|(_, l)| l.contains("native ownership retained")).collect();
        assert_eq!(stalls.len(), 1);
        assert_eq!(stalls[0].0, u32::from(STALL_TICKS));
    }

    #[test]
    fn icp1d_native_completed_created_load_fails_at_the_verdict_and_the_cpu_request_is_served() {
        let mut w = World::icp1d(verdict, false);
        w.run_to(STEP8);
        w.press(Request(8));
        w.run_to(u32::from(STALL_TICKS) - 1);
        assert_eq!(w.gate, Some(Holder::Source { slot: 7, kind: 0x44 }));
        assert!(w.served.is_empty());
        w.run_to(u32::from(STALL_TICKS) + 1);
        assert_eq!(w.served, vec![(u32::from(STALL_TICKS), Request(8))]);
        assert!(w.lines.iter().any(|(f, l)| *f == u32::from(STALL_TICKS) && l.contains("load failed, INSTALL released")));
        assert!(!w.lines.iter().any(|(_, l)| l.contains("native ownership retained")));
        // The later press is a new request, served once the CPU's own reload is done.
        w.run_to(SECOND);
        w.press(Request(9));
        w.run_to(SECOND + 1);
        assert_eq!(w.served.last(), Some(&(SECOND + 1, Request(9))));
    }

    #[test]
    fn icp1d_every_consumed_press_has_a_line_and_the_wait_is_noted_once() {
        let mut w = World::icp1d(verdict, false);
        w.run_to(STEP8);
        w.press(Request(8));
        w.run_to(STEP8 + 300);
        w.press(Request(10));
        w.run_to(u32::from(STALL_TICKS) + 1);
        let after: Vec<_> = w.lines.iter().filter(|(f, _)| *f >= STEP8).map(|(f, l)| (*f, l.as_str())).collect();
        assert_eq!(after[0], (STEP8, "request accepted #8"));
        assert_eq!(after[1], (STEP8 + 1, "request waiting: INSTALL held by source slot=7 kind=0x44"));
        assert_eq!(after[2], (STEP8 + 300, "request refused: owned CPU request busy"));
        assert!(after[3].1.contains("load failed, INSTALL released"));
        assert_eq!(after.len(), 4);
    }

    #[test]
    fn a_load_whose_proof_holds_is_ready_as_before() {
        for rule in [verdict as fn(Unfinished) -> Verdict, m5c_verdict] {
            let mut w = World::icp1d(rule, true);
            w.run_to(STEP8);
            w.press(Request(8));
            w.run_to(STEP8 + 2);
            assert_eq!(w.lines[0], (24, "ready slot=7".into()));
            assert_eq!(w.served, vec![(STEP8 + 1, Request(8))]);
        }
    }

    #[test]
    fn ldn05_a_visible_form_change_during_the_load_now_ends_and_frees_the_gate() {
        // LDN05 window B: slot 7 kind 0x25 (PT), native helper full-ready, row 0 changed by P1's Aegis switch.
        let facts = |ticks| Unfinished { ticks, native_done: true, entry_ready: true, created: true, ..Unfinished::default() };
        assert_eq!(m5c_verdict(facts(STALL_TICKS)), Verdict::Retain);
        assert_eq!(m5c_verdict(facts(STALL_TICKS + 1)), Verdict::Wait);
        assert_eq!(verdict(facts(STALL_TICKS - 1)), Verdict::Wait);
        assert_eq!(verdict(facts(STALL_TICKS)), Verdict::Fail);
    }

    #[test]
    fn a_load_native_has_not_completed_keeps_its_gate() {
        for (native_done, entry_ready) in [(false, false), (false, true), (true, false)] {
            let load = Unfinished { native_done, entry_ready, created: true, ..Unfinished::default() };
            assert!(!load.native_completed());
            assert_eq!(verdict(Unfinished { ticks: STALL_TICKS, ..load }), Verdict::Retain);
            for ticks in [STALL_TICKS + 1, u16::MAX] {
                assert_eq!(verdict(Unfinished { ticks, ..load }), Verdict::Wait);
            }
        }
    }

    #[test]
    fn only_native_completed_created_loads_change_verdict() {
        let mut changed = 0;
        for bits in 0u8..32 {
            let flag = |i: u8| bits & (1 << i) != 0;
            for ticks in [0, 1, STALL_TICKS - 1, STALL_TICKS, STALL_TICKS + 1, u16::MAX] {
                let load = Unfinished {
                    ticks, native_done: flag(0), entry_ready: flag(1), departed: flag(2), created: flag(3), exact_mii: flag(4),
                };
                let (new, old) = (verdict(load), m5c_verdict(load));
                let differs = load.created && load.native_done && load.entry_ready && !load.departed
                    && !load.exact_mii && ticks >= STALL_TICKS;
                if differs {
                    assert_eq!(new, Verdict::Fail, "{:?}", load);
                    assert_ne!(old, Verdict::Fail, "{:?}", load);
                    changed += 1;
                } else {
                    assert_eq!(new, old, "{:?}", load);
                }
            }
        }
        assert_eq!(changed, 3);
    }

    #[test]
    fn departed_and_exact_mii_loads_are_never_failed() {
        for (departed, exact_mii) in [(true, false), (false, true), (true, true)] {
            for created in [false, true] {
                let load = Unfinished {
                    ticks: STALL_TICKS, native_done: true, entry_ready: true, departed, created, exact_mii,
                };
                assert_eq!(verdict(load), Verdict::Retain);
                assert_eq!(verdict(Unfinished { ticks: STALL_TICKS + 5, ..load }), Verdict::Wait);
            }
        }
    }

    #[test]
    fn the_waiting_note_is_once_per_request_and_blocker() {
        let mut waiting = Waiting::new();
        assert_eq!(waiting.note(Request(1), Some(Blocker::Install)), Some(Blocker::Install));
        for _ in 0..600 {
            assert_eq!(waiting.note(Request(1), Some(Blocker::Install)), None);
        }
        assert_eq!(waiting.note(Request(1), Some(Blocker::SaveState)), Some(Blocker::SaveState));
        assert_eq!(waiting.note(Request(2), Some(Blocker::SaveState)), Some(Blocker::SaveState));
        assert_eq!(waiting.note(Request(2), None), None);
        assert_eq!(waiting.note(Request(2), Some(Blocker::SaveState)), Some(Blocker::SaveState));
        assert_eq!(Waiting::<Request>::default(), Waiting::new());
    }

    #[test]
    fn every_blocker_has_a_distinct_reason() {
        let all = [
            Blocker::Backend, Blocker::ReadyGo, Blocker::Menu, Blocker::NativeUi, Blocker::Shortcut,
            Blocker::Install, Blocker::SaveState, Blocker::Recording, Blocker::ReplayPreview, Blocker::Roster,
        ];
        for (i, a) in all.iter().enumerate() {
            assert!(!a.reason().is_empty());
            for b in &all[i + 1..] {
                assert_ne!(a.reason(), b.reason());
            }
        }
    }

    // ---- Scene-proof follow-up: the waiting room's input member and the re-plan (l1, l1b on M5e). ----

    type Row = (usize, u8, i32, i32, i32);
    type Shared = (usize, usize, usize, usize, u8);
    type Own = (i32, usize, usize, u8, u8);
    type View = InputView<Row, Shared, Own>;

    const UNCLAIMED: Row = (0, 2, -1, -1, -1);
    const SHARED: Shared = (0x11e258fd30, 0x11dcc9c080, 0x11f61c8010, 0x1181103c00, 1);
    /// A created extra: no mapping, the native fallback controller, flag 0, CSS role 0 (`waiting_input`).
    const CREATED_OWN: Own = (-1, 0x856d3830, 0x856d37c0, 0, 0);

    /// The l1 room at the PT commit: P1 on entry 0 and the CPU on entry 4 active, everything else unclaimed.
    fn l1_rows() -> [Row; 16] {
        let mut rows = [UNCLAIMED; 16];
        rows[0] = (0x11dcfa7600, 0, 0, -1, -1);
        rows[4] = (0x1180e97a00, 1, 0, 4, 4);
        rows
    }

    fn view(slot: usize, rows: [Row; 16], shared: Shared, own: Own) -> View {
        InputView { slot, rows, shared, own }
    }

    /// The l1 load: `install slot=2 kind=0x24`. `loaded_input` expects only row 2's state 2 -> 0 (alternate bank).
    fn l1() -> (View, View, View, View) {
        let before = view(2, l1_rows(), SHARED, CREATED_OWN);
        let mut expected = before;
        expected.rows[2].1 = 0;
        let prior = view(7, l1_rows(), SHARED, CREATED_OWN);
        let current = view(7, expected.rows, SHARED, CREATED_OWN);
        (prior, current, before, expected)
    }

    /// M5e's `comparison_view`: the member is forgiven only when it equals the load's expected view (then the
    /// scene comparison needs the prior member to equal the load's preflight view).
    fn m5e_member_proven(prior: &View, current: &View, before: &View, expected: &View) -> bool {
        current == expected && prior == before
    }

    #[test]
    fn l1_entry7_member_sees_the_pt_load_s_own_route_and_only_the_new_rule_proves_it() {
        let (prior, current, before, expected) = l1();
        assert!(!m5e_member_proven(&prior, &current, &before, &expected));
        assert!(waiting_member_proven(&prior, &current, &before, &expected));
    }

    #[test]
    fn l1_replay_the_pt_load_is_ready_instead_of_failing_at_the_bound() {
        // #117 install slot=2 kind=0x24; the four PT modules were loaded about 0.6 s (36 frames) later.
        let (prior, current, before, expected) = l1();
        let new = waiting_member_proven(&prior, &current, &before, &expected);
        let old = m5e_member_proven(&prior, &current, &before, &expected);
        let mut w = World::load(verdict, 2, 0x24, 36, new);
        w.run_to(u32::from(STALL_TICKS) + 1);
        assert_eq!(w.lines, vec![(36, "ready slot=2".to_string())]);
        assert_eq!(w.gate, None);
        // M5e: #122 at the bound, the load failed and INSTALL released.
        let mut w = World::load(verdict, 2, 0x24, 36, old);
        w.run_to(u32::from(STALL_TICKS) + 1);
        assert_eq!(w.lines, vec![(u32::from(STALL_TICKS),
            "stalled slot=2 kind=0x24; load failed, INSTALL released".to_string())]);
    }

    #[test]
    fn a_change_the_load_did_not_make_is_still_caught() {
        let (prior, current, before, expected) = l1();
        // LDN05 shape: a visible owner's route row changed (P1's form switch).
        let mut visible = current;
        visible.rows[0] = (0x11dba60c00, 0, 0, -1, -1);
        assert!(!waiting_member_proven(&prior, &visible, &before, &expected));
        // A shared identity changed (the input service was replaced).
        let mut service = current;
        service.shared.0 += 0x100;
        assert!(!waiting_member_proven(&prior, &service, &before, &expected));
        // The member was not taken at the load's preflight.
        let mut stale = prior;
        stale.rows[3] = (0x11fe8ab200, 1, 0x48, 3, 3);
        assert!(!waiting_member_proven(&stale, &current, &before, &expected));
        // Another member than the one captured.
        let other = view(3, current.rows, SHARED, CREATED_OWN);
        assert!(!waiting_member_proven(&prior, &other, &before, &expected));
    }

    #[test]
    fn entry7_s_own_fields_belong_to_entry7_s_loads_only() {
        let (prior, current, before, expected) = l1();
        let mut mapped = current;
        mapped.own = (0, 0x856d3900, 0x856d3990, 1, 0);
        // Native reads entry 7's mapping/controller only when it builds entry 7's controls (0x6b6f10).
        assert!(waiting_member_proven(&prior, &mapped, &before, &expected));
        // A load on entry 7 still compares its own view whole, as before.
        let own_before = view(7, l1_rows(), SHARED, CREATED_OWN);
        let mut own_expected = own_before;
        own_expected.rows[7].1 = 0;
        let mut own_current = own_expected;
        assert!(waiting_member_proven(&own_before, &own_current, &own_before, &own_expected));
        own_current.own.3 = 1;
        assert!(!waiting_member_proven(&own_before, &own_current, &own_before, &own_expected));
    }

    #[test]
    fn entry7_loads_keep_the_m5e_rule() {
        let base = view(7, l1_rows(), SHARED, CREATED_OWN);
        let mut moved = base;
        moved.rows[7].1 = 0;
        let mut flagged = moved;
        flagged.own.3 = 1;
        let mut row0 = moved;
        row0.rows[0].1 = 1;
        for prior in [base, moved, flagged] {
            for current in [base, moved, flagged, row0] {
                for expected in [base, moved] {
                    assert_eq!(
                        waiting_member_proven(&prior, &current, &base, &expected),
                        m5e_member_proven(&prior, &current, &base, &expected),
                    );
                }
            }
        }
    }

    /// M5e: every `Fail` marked the kind failed for the scene.
    fn m5e_fails_kind(_: Unfinished) -> bool {
        true
    }

    /// The planner over one created slot: a failed attempt's kind is planned again unless the policy retires it.
    /// `proofs[i]` is whether attempt i's scene proof holds; native completes 36 frames (≈0.6 s) into an attempt.
    /// Returns (attempts started, frame of the ready completion).
    fn replan(fails_kind: fn(Unfinished) -> bool, proofs: &[bool], frames: u32) -> (usize, Option<u32>) {
        let (mut attempts, mut retired, mut loading, mut ticks) = (0usize, false, false, 0u16);
        for frame in 1..=frames {
            if !loading {
                if !retired {
                    loading = true;
                    ticks = 0;
                    attempts += 1;
                }
                continue;
            }
            ticks += 1;
            let native_done = ticks >= 36;
            if native_done && proofs.get(attempts - 1).copied().unwrap_or(true) {
                return (attempts, Some(frame));
            }
            let load = Unfinished { ticks, native_done, entry_ready: native_done, created: true, ..Unfinished::default() };
            if verdict(load) == Verdict::Fail {
                loading = false;
                retired = fails_kind(load);
            }
        }
        (attempts, None)
    }

    #[test]
    fn a_created_entry_s_failed_attempt_is_planned_again_and_a_native_helper_failure_retires_the_kind() {
        // l1's 2.5 minutes after the stall: 9,000 frames. First attempt's proof fails, the second's holds.
        assert_eq!(replan(m5e_fails_kind, &[false, true], 9_000), (1, None));
        assert_eq!(replan(Unfinished::fails_kind, &[false, true], 9_000),
            (2, Some(1 + u32::from(STALL_TICKS) + 1 + 36)));
        let created = Unfinished { ticks: STALL_TICKS, native_done: true, entry_ready: true, created: true, ..Unfinished::default() };
        assert_eq!(verdict(created), Verdict::Fail);
        assert!(!created.fails_kind());
        let helper = Unfinished { created: false, entry_ready: false, ..created };
        assert_eq!(verdict(helper), Verdict::Fail);
        assert!(helper.fails_kind());
    }
}
