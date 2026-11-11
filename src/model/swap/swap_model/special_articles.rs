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

