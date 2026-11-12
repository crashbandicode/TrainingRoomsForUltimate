//! PT-TRAINER-PARK: the articles of a native group's object-record holder (`entry+0x5a88`) across group
//! exchanges. The Pokémon Trainer's trainer is one of them, and it stayed on stage after P1 swapped PT → Aegis
//! (agent-online `sp1` `m-075849`, `l1b` `m-063539`; `u18/u19/pt-trainer-park/PROGRESS.md`).
//!
//! FACTs (owned 13.0.5 ELF `0789f0a4…`; listings in `u18/u19/pt-trainer-park/re/`,
//! `special-groups/re/`):
//! - The PT branch of the window constructor `0x658af0` registers the trainer table with G68
//!   (`0x17eeaa0(G68, 0x72, E+0x41, main+0x52b9488, 7, 0)` at `0x658d04`). It allocates the 0x238-byte holder
//!   (vtable `main+0x4f626e8`) at `+0x5a88`. It then fills the holder's article pool with
//!   `0x3a6270(holder, main+0x52b9488, 0, -1, E+0xe0, 0)` (`0x658db8`).
//! - Activation `0x656ee0` (PT branch: `+0x591f` bit 0, `0x657018`) claims the trainer from that pool with
//!   `0x3a6ce0(holder, 0, active body, 0)` whenever `+0x5a90` is null, and stores it at `+0x5a90` (`0x65727c`,
//!   `0x657350–0x657358`). The trainer is a weapon battle object. It is updated and drawn on its own, whatever
//!   the entry's window shows.
//! - Every native path that takes the PT window off the entry first writes `+0x5a90 = 0` and destroys the pool
//!   with `0x3a6b80(holder)`:
//!   - finalize `0x652f60–0x652f64`;
//!   - loader state 1 `0x65128c–0x651290`;
//!   - `0x653118–0x65311c`.
//! - `0x3a6b80` removes each article and ends with `+0x10 = 0`, `+0x228 = 0` (`0x3a6cb4`). The holder is then
//!   reusable.
//! - Native also rebuilds a live PT pool in place, on the same holder, in `0x65a650` (PT branch, `+0x591f` bit 0):
//!   `+0x5a90 = 0`, `0x3a6b80(holder)`, then `0x3a6270(holder, main+0x52b9488, 0, -1, E+0xe0, 0)`. That is the
//!   pair this module uses.
//! - The Aegis branch (`0x658e38–0x658f10`) builds its holder with no G68 registration. Its article files are
//!   served from `R`'s `0x75` keys, which the bank returns right after the build (D-SG1). An Aegis pool rebuilt
//!   on a later attach could name files that are no longer resident (WP-R §3.4 class). Native has no in-place
//!   Aegis rebuild, so the Aegis holder is left as it is.
//!
//! **Root cause.** The bank's detach (`clear_window` / `restore_window` at the acknowledgement seam) moves the PT
//! holder into the group's saved window but leaves its pool alive. The claimed trainer stays in the world,
//! drawn, while another group is the entry's window. Native has no parked PT window. The rule below restores
//! native's invariant: **a PT holder has articles only while its window is attached.**
//! - Retire on detach: `0x3a6b80`, with the saved `+0x5a90` cleared.
//! - Rebuild on attach: `0x3a6270`, the build's own arguments, before activation.
//!
//! Activation then claims the trainer exactly as it does after a Phase B build (`#205 records=[…, 0, …]`, then
//! claimed by the time of `#235`).

/// `entry+0x591f` bit 0: the PT window flag (`654dc0` writes 5 for a PT window).
pub const PT_WINDOW_FLAG: u8 = 0x01;

/// What the bank does with a holder's article pool at an exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArticleStep {
    /// `+0x5a90 = 0` (the saved window's record), then `0x3a6b80(holder)`.
    Retire,
    /// `0x3a6270(holder, main+0x52b9488, 0, -1, E+0xe0, 0)`, before the activation that claims the trainer.
    Rebuild,
}

/// A window leaves the entry (detached into its group record, or replaced by a rollback). `flags` is the
/// window's `+0x591f`, `holder` whether `+0x5a88` holds a valid holder, and `articles` the holder's pool count
/// (`holder+0x10`).
pub fn on_detach(flags: u8, holder: bool, articles: u64) -> Option<ArticleStep> {
    (flags & PT_WINDOW_FLAG != 0 && holder && articles != 0).then_some(ArticleStep::Retire)
}

/// A window becomes the entry's window again (warm attach, or the rollback's restore). The pool is rebuilt only
/// when a detach retired it (count 0). A fresh Phase B build already filled it inside `0x658af0`.
pub fn on_attach(flags: u8, holder: bool, articles: u64) -> Option<ArticleStep> {
    (flags & PT_WINDOW_FLAG != 0 && holder && articles == 0).then_some(ArticleStep::Rebuild)
}

/// Inbox 02:00Z (the PT exit's 5-frame gap): a detach's [`ArticleStep::Retire`], split across game-thread ticks.
///
/// FACTs (owned 13.0.5 ELF; Eden `rc6-bg-ptx1`/`ptx2` guest trace points, `rc6-eden/bg-pt-evidence/`):
/// - `0x3a6b80` runs two passes. Pass 1 (`0x3a6bb0-0x3a6c04`) deactivates each article: the trainer's
///   `vt+0x20` thunk `0x33a8a80` reaches the weapon's `vt+0x48` = `0x33a1c90`, which clears `bo+0x61` and switches its
///   modules off (0.1-0.7 ms). Pass 2 (`0x3a6c5c-0x3a6cb0`) destroys each object by id (`0x3aeb50`, after
///   `0x3ac560` and a one-time static init), then `+0x10 = 0`, `+0x228 = 0`.
/// - All of the exit's 28-57 ms is pass 2: the trainer's own teardown (`vt+0x50` = `0x33a1fc0`, 15-33 ms) and its own
///   article pool, which that teardown releases through `0x3d3a90` at `[[bo+0x20]+0x98]+0x240` (eight sub-weapons,
///   13-26 ms; the holder class `0x4f626e8` the article module's constructor stores there, `0x33b98d4`).
/// - `0x3ac560(id)` returns null for an id whose slot no longer holds it; `0x3aeb50(id)` does not check.
///
/// The exit's tick runs [`RetireStage::Deactivate`] only; each later tick runs the next stage. The trainer leaves the
/// world at once (native's pass 1), and nothing else touches its pool until the stages finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetireStage {
    /// Pass 1 on the exit's tick.
    Deactivate,
    /// The deactivated trainer's own article pools through `0x3a6b80` (what its teardown would release first).
    SubPools,
    /// Pass 2 and the tail: each object destroyed by id when `0x3ac560` still finds it.
    Destroy,
}

impl RetireStage {
    /// The stage the next tick runs, `None` once the pool is retired.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Deactivate => Some(Self::SubPools),
            Self::SubPools => Some(Self::Destroy),
            Self::Destroy => None,
        }
    }
}

/// The stages still due after `done`, in order: a teardown, a drain or a switch to the retiring group runs them at
/// once (the pool must be empty before anything else reads or rebuilds it).
pub fn remaining(done: RetireStage) -> impl Iterator<Item = RetireStage> {
    std::iter::successors(done.next(), |stage| stage.next())
}

/// Inbox 02:42Z: a detached PT group's pool is built again on the task worker, ahead of its attach, when it is a built
/// `trainer` group whose pool (`pool` articles) is empty, it holds its object kind again (`held`: the files resident,
/// body_bank's item 1) and the pool is not still `retiring`. Its attach then finds the pool filled ([`on_attach`]: no
/// rebuild on the game thread) and claims the trainer, as after a background build.
pub fn prepare_due(trainer: bool, built: bool, pool: u64, held: bool, retiring: bool) -> bool {
    trainer && built && pool == 0 && held && !retiring
}

/// A group whose pool is still retiring is not attached warm: its pool is neither live (deactivated) nor empty (the
/// rebuild needs count 0), so the activation would claim a deactivated trainer.
pub fn attachable(retiring: Option<RetireStage>) -> bool {
    retiring.is_none()
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const PT_FLAGS: u8 = 0x05;
    const AEGIS_FLAGS: u8 = 0x64;
    const ORDINARY_FLAGS: u8 = 0x04;
    /// The pool size `0x3a6270` builds from the PT table (any non-zero count; the model only needs "some").
    const PT_POOL: u64 = 2;

    /// One native group's holder and the world it draws into.
    #[derive(Clone, Copy, Debug, Default)]
    struct Holder {
        flags: u8,
        present: bool,
        /// `holder+0x10`.
        articles: u64,
        /// `+0x5a90` (the claimed trainer), live or saved with the window.
        claimed: bool,
        /// The window is the entry's window.
        attached: bool,
    }

    impl Holder {
        fn built(flags: u8) -> Self {
            // `0x658af0`: the holder and its pool; `+0x5a90` is null right after the build (`#205`).
            Holder { flags, present: true, articles: if flags & PT_WINDOW_FLAG != 0 { PT_POOL } else { 0 }, claimed: false, attached: true }
        }
        /// Activation `0x656ee0`'s PT branch: claim when `+0x5a90` is null and the pool can serve.
        fn activate(&mut self) {
            if self.attached && self.flags & PT_WINDOW_FLAG != 0 && !self.claimed && self.articles != 0 { self.claimed = true; }
        }
        fn apply(&mut self, step: Option<ArticleStep>) {
            match step {
                Some(ArticleStep::Retire) => { self.claimed = false; self.articles = 0; }
                Some(ArticleStep::Rebuild) => { self.articles = PT_POOL; }
                None => {}
            }
        }
        fn detach(&mut self) {
            self.apply(on_detach(self.flags, self.present, self.articles));
            self.attached = false;
        }
        fn attach(&mut self) {
            self.attached = true;
            self.apply(on_attach(self.flags, self.present, self.articles));
            self.activate();
        }
        /// What a viewer sees: a claimed article of this holder is drawn, attached or not (the defect).
        fn trainer_drawn(&self) -> bool { self.claimed && self.articles != 0 }
    }

    /// The `sp1`/`l1b` lane: PT built, activated (trainer claimed), then P1 → Aegis. Before the fix the detach
    /// kept the pool, so the claimed trainer stayed drawn. Now it is retired, and a warm return draws it again.
    fn a_detached_pt_draws_no_trainer_and_a_warm_return_claims_a_new_one() {
        let mut pt = Holder::built(PT_FLAGS);
        assert!(!pt.trainer_drawn(), "#205: +0x5a90 is null right after the build");
        pt.activate();
        assert!(pt.trainer_drawn(), "m-075840: the trainer is claimed once PT activates");
        let mut aegis = Holder::built(AEGIS_FLAGS);
        pt.detach();
        aegis.activate();
        assert!(!pt.trainer_drawn(), "m-075849: no trainer while Aegis is the window");
        assert_eq!(pt.articles, 0, "0x3a6b80 emptied the pool");
        aegis.detach();
        pt.attach();
        assert!(pt.trainer_drawn(), "the return PT swap claims a rebuilt trainer");
    }

    /// The defect itself: without the retire, the detached holder still draws its trainer.
    fn without_the_retire_the_detached_trainer_stays_drawn() {
        let mut pt = Holder::built(PT_FLAGS);
        pt.activate();
        pt.attached = false; // the M5f detach: window saved, pool untouched
        assert!(pt.trainer_drawn());
    }

    /// Over every sequence of group exchanges (PT, Aegis, ordinary, a rollback that re-attaches the outgoing
    /// window), a PT trainer is drawn exactly when its window is attached and activated. The pool is rebuilt only
    /// after a retire, never on top of a live one, and Aegis and ordinary windows are never touched.
    fn a_pt_trainer_is_drawn_only_while_its_window_is_attached() {
        let mut runs = 0;
        for seed in 0u64..300 {
            let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
            let mut next = |n: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % n };
            let mut groups = [Holder::built(PT_FLAGS), Holder::built(AEGIS_FLAGS), Holder::built(ORDINARY_FLAGS)];
            groups[2].present = false;
            for g in groups.iter_mut().skip(1) { g.attached = false; }
            let mut window = 0usize;
            groups[0].activate();
            for _ in 0..12 {
                let target = next(3) as usize;
                if target == window { continue; }
                let before = groups;
                groups[window].detach();
                let rollback = next(5) == 0;
                if rollback {
                    // The rollback restores the outgoing window and native activates it again.
                    groups[window].attach();
                } else {
                    groups[target].attach();
                    window = target;
                }
                for (i, g) in groups.iter().enumerate() {
                    assert_eq!(g.trainer_drawn(), g.attached && g.flags & PT_WINDOW_FLAG != 0,
                        "seed {} group {} {:?}", seed, i, g);
                    if g.flags & PT_WINDOW_FLAG == 0 {
                        assert_eq!(g.articles, before[i].articles, "seed {}: a non-PT holder is never touched", seed);
                    }
                }
                runs += 1;
            }
        }
        assert!(runs > 1500, "{}", runs);
    }

    /// Inbox 02:00Z: leaving a PT group took 114.9-118.5 ms with a 5-frame gap (worker A, `rc6-xc2`/`xc3`); the exit's
    /// setup tick held `0x3a6b80` whole. Eden `rc6-bg-ptx1`/`ptx2` split it: pass 1 0.12-0.66 ms; pass 2 is the
    /// trainer's teardown, 15.3-32.6 ms of its own plus 12.9-26.1 ms for its sub-pools. The exit's tick now runs pass 1
    /// only and every later tick one stage, so no tick carries more than one of those parts.
    fn a_pt_exit_retires_its_pool_one_stage_per_tick() {
        assert_eq!(on_detach(PT_FLAGS, true, PT_POOL), Some(ArticleStep::Retire));
        let mut stage = RetireStage::Deactivate;
        let mut ticks = vec![stage];
        while let Some(next) = stage.next() {
            stage = next;
            ticks.push(stage);
        }
        assert_eq!(ticks, [RetireStage::Deactivate, RetireStage::SubPools, RetireStage::Destroy],
            "the trainer's own pools before its teardown, which then finds them empty");
        // The probe's worst (first exit of a boot) game-thread costs in microseconds, on top of a tick's own frame.
        const FRAME_US: u64 = 16_667;
        const SETUP_US: u64 = 19_100; // the exit's setup tick without the release (rc6-bg-pt7: 16-19 ms)
        let cost = |s: RetireStage| match s { RetireStage::Deactivate => 660, RetireStage::SubPools => 26_100, RetireStage::Destroy => 32_600 };
        let frames = |us: u64| us.div_ceil(FRAME_US);
        assert!(frames(SETUP_US + cost(RetireStage::Deactivate)) <= 2, "the exit's own tick");
        for s in [RetireStage::SubPools, RetireStage::Destroy] {
            assert!(frames(FRAME_US + cost(s)) <= 3, "{s:?}");
        }
        let whole: u64 = ticks.iter().map(|s| cost(*s)).sum();
        assert!(frames(SETUP_US + whole) >= 5, "one tick: the 5-frame exit this replaces");
    }

    /// The trainer is undrawn from the exit's tick (pass 1); a retiring group is not attached (its pool is neither live
    /// nor empty); a teardown, a drain or a switch to it first runs every stage still due, in order.
    fn a_retiring_pool_is_finished_before_anything_else_reads_it() {
        assert!(attachable(None));
        for s in [RetireStage::Deactivate, RetireStage::SubPools, RetireStage::Destroy] {
            assert!(!attachable(Some(s)), "{s:?}");
        }
        assert_eq!(remaining(RetireStage::Deactivate).collect::<Vec<_>>(), [RetireStage::SubPools, RetireStage::Destroy]);
        assert_eq!(remaining(RetireStage::SubPools).collect::<Vec<_>>(), [RetireStage::Destroy]);
        assert_eq!(remaining(RetireStage::Destroy).count(), 0);
        // Aegis and ordinary windows have nothing to retire, so nothing is staged.
        assert_eq!(on_detach(AEGIS_FLAGS, true, 3), None);
        assert_eq!(on_detach(ORDINARY_FLAGS, false, 0), None);
    }

    /// Inbox 02:42Z: the attach's own rebuild (`0x3a6270` inside the switch's setup tick) took 209 ms the boot's first
    /// time and 43-63 ms after it (Eden rc6-bg-ptx3; hot-slots rc6-hot-xps22: 382 ms, then 62 ms). A detached PT group's
    /// retired pool is built again on the task worker once the group holds its object kind again (item 1), so the attach
    /// finds it filled, as after a background build, and claims it without a rebuild.
    fn a_retired_pt_pool_is_built_again_on_the_worker_before_its_attach() {
        assert!(prepare_due(true, true, 0, true, false));
        assert!(!prepare_due(true, true, 0, false, false), "the files are not resident yet: item 1's re-acquisition first");
        assert!(!prepare_due(true, true, 0, true, true), "still retiring");
        assert!(!prepare_due(true, true, PT_POOL, true, false), "already filled (a background build's, or prepared)");
        assert!(!prepare_due(false, true, 0, true, false), "Aegis and ordinary holders are never retired");
        assert!(!prepare_due(true, false, 0, true, false), "only a built group");
        let mut pt = Holder::built(PT_FLAGS);
        pt.activate();
        pt.detach();
        assert_eq!(pt.articles, 0, "retired at the exit");
        pt.articles = PT_POOL; // the worker's 0x3a6270 on the detached holder
        assert!(!pt.trainer_drawn(), "a prepared pool draws nothing until its attach claims it");
        assert_eq!(on_attach(PT_FLAGS, true, pt.articles), None, "the attach rebuilds nothing");
        pt.attach();
        assert!(pt.trainer_drawn(), "and claims the prepared trainer");
    }

    fn the_rule_s_edges() {
        assert_eq!(on_detach(PT_FLAGS, true, PT_POOL), Some(ArticleStep::Retire));
        assert_eq!(on_detach(PT_FLAGS, true, 0), None, "already retired");
        assert_eq!(on_detach(PT_FLAGS, false, PT_POOL), None, "no valid holder");
        assert_eq!(on_detach(AEGIS_FLAGS, true, 3), None, "Aegis: no in-place native rebuild, files served by R");
        assert_eq!(on_attach(PT_FLAGS, true, 0), Some(ArticleStep::Rebuild));
        assert_eq!(on_attach(PT_FLAGS, true, PT_POOL), None, "a fresh build already filled the pool");
        assert_eq!(on_attach(AEGIS_FLAGS, true, 0), None);
        assert_eq!(on_attach(ORDINARY_FLAGS, false, 0), None);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_detached_pt_draws_no_trainer_and_a_warm_return_claims_a_new_one", a_detached_pt_draws_no_trainer_and_a_warm_return_claims_a_new_one as fn()),
            ("without_the_retire_the_detached_trainer_stays_drawn", without_the_retire_the_detached_trainer_stays_drawn as fn()),
            ("a_pt_trainer_is_drawn_only_while_its_window_is_attached", a_pt_trainer_is_drawn_only_while_its_window_is_attached as fn()),
            ("a_pt_exit_retires_its_pool_one_stage_per_tick", a_pt_exit_retires_its_pool_one_stage_per_tick as fn()),
            ("a_retiring_pool_is_finished_before_anything_else_reads_it", a_retiring_pool_is_finished_before_anything_else_reads_it as fn()),
            ("a_retired_pt_pool_is_built_again_on_the_worker_before_its_attach", a_retired_pt_pool_is_built_again_on_the_worker_before_its_attach as fn()),
            ("the_rule_s_edges", the_rule_s_edges as fn()),
        ]);
    }
    }
}
