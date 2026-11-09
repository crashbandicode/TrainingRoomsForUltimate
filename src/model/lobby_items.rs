//! Lobby Items (Phase 2 N11, `docs/ROADMAP.md`): the waiting-room practice's item spawner, pure half.
//!
//! The runtime hooks the spawner's own switch test (`main+0x15d9a80`, `cbz w8` on the spawner control's `+0x30` byte) and the
//! timed generator's lottery test (`main+0x15499b4`, `tbz w0,#0`; the spawner's switch doesn't gate that generator, #62): in a
//! qualified waiting scene with the row Off each hands the native a zero, so the native takes its own "off" or "lottery
//! declined" path.
//! Nothing is written to the game and nothing is restored: On is "leave the native alone". Items already on the stage are removed
//! once per Off edge, through the native removal, from Player 1's frame (never per frame, never inside the item manager's update).

/// Whether the spawner's switch reads as off this frame: only in a qualified waiting scene, only with the row Off.
pub const fn suppress(waiting: bool, on: bool) -> bool {
    waiting && !on
}

/// One removal per Off edge: the row turned Off in a waiting scene, or a waiting scene began with it Off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Removal {
    /// The row as last seen in this waiting scene; `None` outside one.
    last_on: Option<bool>,
    owed: bool,
}

impl Removal {
    pub const fn new() -> Self {
        Self { last_on: None, owed: false }
    }

    /// True exactly when the caller removes the existing items now (and once). `allowed` (the native item gate and an idle
    /// roster) is asked only while a removal is owed; a refusal keeps it owed for a later frame. Turning On cancels it; leaving
    /// the waiting scene forgets it.
    pub fn step(&mut self, waiting: bool, on: bool, allowed: impl FnOnce() -> bool) -> bool {
        if !waiting {
            *self = Self::new();
            return false;
        }
        if on {
            self.owed = false;
        } else if self.last_on != Some(false) {
            self.owed = true;
        }
        self.last_on = Some(on);
        if self.owed && allowed() {
            self.owed = false;
            return true;
        }
        false
    }

    /// A removal is owed and waits for the gate or the roster.
    pub fn owed(&self) -> bool {
        self.owed
    }
}

/// The ids of at most `N` active items, collected before any is removed (removing shifts the native list). Zero ids (an empty
/// slot the native returned) are skipped.
pub fn collect<const N: usize>(count: usize, mut id_at: impl FnMut(usize) -> u32) -> ([u32; N], usize) {
    let mut ids = [0u32; N];
    let mut n = 0;
    for index in 0..count.min(N) {
        let id = id_at(index);
        if id != 0 {
            ids[n] = id;
            n += 1;
        }
    }
    (ids, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_waiting_scene_with_the_row_off_suppresses() {
        assert!(suppress(true, false));
        assert!(!suppress(true, true));
        assert!(!suppress(false, false), "Training, matches and online matches are untouched");
        assert!(!suppress(false, true));
    }

    #[test]
    fn on_never_removes_and_never_asks_the_gate() {
        let mut r = Removal::new();
        for _ in 0..100 {
            assert!(!r.step(true, true, || panic!("no gate read while On")));
        }
    }

    #[test]
    fn an_off_edge_removes_once_and_never_sweeps() {
        let mut r = Removal::new();
        assert!(!r.step(true, true, || true));
        assert!(r.step(true, false, || true), "the edge removes");
        for _ in 0..1000 {
            assert!(!r.step(true, false, || panic!("no gate read once paid")), "no per-frame sweep");
        }
        assert!(!r.step(true, true, || true));
        assert!(r.step(true, false, || true), "the next Off edge removes again");
    }

    #[test]
    fn a_scene_entered_with_the_row_off_removes_once() {
        let mut r = Removal::new();
        assert!(r.step(true, false, || true));
        assert!(!r.step(true, false, || true));
    }

    #[test]
    fn a_refused_gate_defers_without_losing_the_removal() {
        let mut r = Removal::new();
        assert!(!r.step(true, false, || false));
        assert!(r.owed());
        assert!(!r.step(true, false, || false));
        assert!(r.owed());
        assert!(r.step(true, false, || true), "paid as soon as the gate allows it");
        assert!(!r.owed());
        assert!(!r.step(true, false, || true));
    }

    #[test]
    fn turning_on_cancels_an_owed_removal() {
        let mut r = Removal::new();
        assert!(!r.step(true, false, || false));
        assert!(!r.step(true, true, || true));
        assert!(!r.owed());
        assert!(!r.step(true, true, || true));
    }

    #[test]
    fn leaving_the_waiting_scene_forgets_everything() {
        let mut r = Removal::new();
        assert!(!r.step(true, false, || false));
        assert!(!r.step(false, false, || panic!("no gate read outside a waiting scene")));
        assert!(!r.owed());
        assert_eq!(r, Removal::default());
        assert!(r.step(true, false, || true), "the next waiting scene, entered Off, removes once");
    }

    #[test]
    fn collect_takes_the_ids_first_skips_empty_slots_and_caps() {
        let (ids, n) = collect::<4>(3, |i| [7, 0, 9][i]);
        assert_eq!(&ids[..n], &[7, 9]);
        let (ids, n) = collect::<2>(5, |i| i as u32 + 1);
        assert_eq!(&ids[..n], &[1, 2]);
        assert_eq!(collect::<2>(0, |_| unreachable!()).1, 0);
    }
}
