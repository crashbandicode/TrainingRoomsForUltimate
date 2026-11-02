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

#[cfg(test)]
mod tests {
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
    #[test]
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
    #[test]
    fn without_the_retire_the_detached_trainer_stays_drawn() {
        let mut pt = Holder::built(PT_FLAGS);
        pt.activate();
        pt.attached = false; // the M5f detach: window saved, pool untouched
        assert!(pt.trainer_drawn());
    }

    /// Over every sequence of group exchanges (PT, Aegis, ordinary, a rollback that re-attaches the outgoing
    /// window), a PT trainer is drawn exactly when its window is attached and activated. The pool is rebuilt only
    /// after a retire, never on top of a live one, and Aegis and ordinary windows are never touched.
    #[test]
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

    #[test]
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
}
