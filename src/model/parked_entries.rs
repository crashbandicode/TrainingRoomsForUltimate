//! F9 operator decision B (2026-11-02): delay the native count store while a
//! newly included parked seat still belongs to the helper loader. This model
//! owns intent only. Cancelling it must never cancel an already-started reload.

pub const PARKED: [u32; 2] = [2, 3];

/// Fresh observations, not retained authority to dereference a native address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation<E> {
    pub epoch: E,
    pub menu: usize,
    pub count: u32,
    pub open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Request<E> {
    before: Observation<E>,
    desired: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setter {
    /// Let the complete native feedback/store/pane path execute once.
    Continue,
    /// Return through the native epilogue before any of those effects.
    Defer,
}

/// One coalesced request; no heap, timer, retained pointer or polling loop.
pub struct Pending<E> {
    request: Option<Request<E>>,
}

impl<E: Copy + Eq> Default for Pending<E> {
    fn default() -> Self { Self::new() }
}

impl<E: Copy + Eq> Pending<E> {
    pub const fn new() -> Self { Self { request: None } }

    /// Called at the pre-store gate with the native clamped desired count.
    /// `occupied` includes helper loads and native restores until their receipts.
    pub fn setter(&mut self, now: Observation<E>, desired: u32, occupied: u8) -> Setter {
        self.request = None;
        if !now.open || now.menu == 0 || !(1..=3).contains(&now.count)
            || !(1..=3).contains(&desired) || desired <= now.count
            || seats(now.count, desired) & occupied == 0
        {
            return Setter::Continue;
        }
        self.request = Some(Request { before: now, desired });
        Setter::Defer
    }

    /// Seats the cleanup service should return. A stale observation cancels
    /// intent immediately; ownership of any in-flight restore lives elsewhere.
    pub fn returning(&mut self, now: Observation<E>) -> u8 {
        let Some(request) = self.current(now) else { return 0; };
        seats(request.before.count, request.desired)
    }

    /// Consume only after every newly included seat is native-ready. The caller
    /// rechecks native identity and invokes the complete setter with this value.
    /// Clearing before replay makes re-entry one-shot.
    pub fn replay(&mut self, now: Observation<E>, occupied: u8) -> Option<u32> {
        let request = self.current(now)?;
        if seats(now.count, request.desired) & occupied != 0 { return None; }
        self.request = None;
        Some(request.desired)
    }

    pub fn cancel(&mut self) { self.request = None; }

    fn current(&mut self, now: Observation<E>) -> Option<Request<E>> {
        if self.request.is_some_and(|request| request.before != now) {
            self.cancel();
        }
        self.request
    }
}

fn seats(old: u32, desired: u32) -> u8 {
    PARKED.into_iter().filter(|slot| *slot > old && *slot <= desired)
        .fold(0, |mask, slot| mask | (1 << slot))
}

#[cfg(test)]
mod tests {
    use super::*;
    const MENU: Observation<u64> = Observation { epoch: 7, menu: 0x1000, count: 1, open: true };

    #[test]
    fn leased_loading_and_restoring_seats_all_hold_the_count_until_receipt() {
        let mut pending = Pending::new();
        assert_eq!(pending.setter(MENU, 3, 0b1100), Setter::Defer);
        // Hat lease drains, helper load finishes, restore begins: still occupied.
        for occupied in [0b1100, 0b1100, 0b1000, 0b1000] {
            assert_eq!(pending.returning(MENU), 0b1100);
            assert_eq!(pending.replay(MENU, occupied), None);
        }
        assert_eq!(pending.replay(MENU, 0), Some(3));
        assert_eq!(pending.replay(MENU, 0), None);
        assert_eq!(pending.setter(MENU, 3, 0), Setter::Continue);
    }

    #[test]
    fn latest_click_coalesces_and_decrease_cancels() {
        let mut pending = Pending::new();
        assert_eq!(pending.setter(MENU, 2, 0b1100), Setter::Defer);
        assert_eq!(pending.returning(MENU), 0b0100);
        assert_eq!(pending.setter(MENU, 3, 0b1100), Setter::Defer);
        assert_eq!(pending.returning(MENU), 0b1100);
        assert_eq!(pending.setter(MENU, 2, 0b1000), Setter::Continue);
        assert_eq!(pending.replay(MENU, 0), None);
        assert_eq!(pending.setter(MENU, 3, 0b1100), Setter::Defer);
        assert_eq!(pending.setter(MENU, 1, 0b1100), Setter::Continue);
        assert_eq!(pending.returning(MENU), 0);
    }

    #[test]
    fn early_close_reopen_scene_root_or_count_change_never_replays_old_intent() {
        for changed in [Observation { open: false, ..MENU }, Observation { epoch: 8, ..MENU },
            Observation { menu: 0x2000, ..MENU }, Observation { count: 2, ..MENU }]
        {
            let mut pending = Pending::new();
            assert_eq!(pending.setter(MENU, 3, 0b1100), Setter::Defer);
            assert_eq!(pending.returning(changed), 0);
            assert_eq!(pending.replay(MENU, 0), None, "reopening cannot resurrect a cancelled click");
        }
    }

    #[test]
    fn only_newly_live_native_cpu_seats_delay_and_reverse_is_immediate() {
        let mut pending = Pending::new();
        assert_eq!(pending.setter(MENU, 2, 0b1000_1000), Setter::Continue);
        let two = Observation { count: 2, ..MENU };
        assert_eq!(pending.setter(two, 3, 0b1000), Setter::Defer);
        assert_eq!(pending.returning(two), 0b1000);
        assert_eq!(pending.setter(two, 1, 0b1100), Setter::Continue);
        assert_eq!(pending.setter(Observation { count: 3, ..MENU }, 2, 0b1100), Setter::Continue);
    }
}
