//! Value-only policy shared by the native practice consumers and host probes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context { Training, Waiting }

pub fn context(native_training: bool, initialized: bool, melee: u8, mode: u32,
    waiting_active: bool) -> Option<Context> {
    if native_training { return Some(Context::Training); }
    (waiting_active && initialized && melee == 0x0d
        && (mode == 0x02010000 || crate::engine::mii::backend::Layout::quickplay_waiting_mode(mode)))
        .then_some(Context::Waiting)
}

impl Context {
    pub fn cpu_entry(self) -> i32 { if self == Self::Waiting { 4 } else { 1 } }
    pub fn role(self, entry: i32) -> Option<usize> {
        if entry == 0 { Some(0) } else if entry == self.cpu_entry() { Some(1) } else { None }
    }
}

/// Entry addresses are equality tokens only, never dereferenced after capture.
/// Death/rebirth can replace a body inside the same entry. Destruction retires
/// this binding before native free, even if the next entry reuses its address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entries { pub context: Context, pub player: usize, pub cpu: usize }
impl Entries {
    pub fn owns(self, entry: usize) -> bool { entry == self.player || entry == self.cpu }
}

#[derive(Default)]
pub struct EntryLease(Option<Entries>);
impl EntryLease {
    pub const fn new() -> Self { Self(None) }
    pub fn active(&self) -> bool { self.0.is_some() }
    pub fn acquire(&mut self, current: Entries) -> bool {
        if self.replaced(Some(current)) { return false; }
        self.0 = Some(current);
        true
    }
    pub fn replaced(&self, current: Option<Entries>) -> bool {
        self.0.is_some_and(|old| Some(old) != current)
    }
    pub fn owns(&self, entry: usize) -> bool { self.0.is_some_and(|old| old.owns(entry)) }
    pub fn retire(&mut self) -> bool { self.0.take().is_some() }
}

/// Authoritative scene/P1 lifetime departure, even without a save/record lease.
/// The native adapter releases its lease lock BEFORE clearing the consumers;
/// neither the old lease value nor a later CPU frame decides whether they run.
pub fn depart_session(retire_lease: impl FnOnce(), clear_consumers: impl FnOnce()) {
    retire_lease();
    clear_consumers();
}

