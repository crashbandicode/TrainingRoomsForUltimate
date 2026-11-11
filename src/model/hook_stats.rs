//! Hook activity counters: heap allocations made inside hooks, and how often each hook ran
//! (playbook "No heap in hooks"; review R5 of package 1).
//!
//! `CountingAllocator` is the plugin's `#[global_allocator]` (declared in `runtime::alloc`; a host test
//! declares its own). It forwards to the system allocator and counts the allocations a thread makes
//! while it is inside a `HookScope`. The scope also counts one invocation of the named hook.
//!
//! The install line reports the allocation count once at boot, which proves nothing about hooks that
//! have not run yet. `Observer` is the post-exercise half: the `diag` build samples the counters
//! sparsely after the hooks have worked and logs one line, `allocations=<since boot> delta=<since the
//! previous line> hooks=<name>:<n> …` (nonzero hooks only). The Eden lanes accept a hook lane only
//! when the hooks it needs show a nonzero count and every line's `delta` is 0 (`eden/hook_stats.py`).
//! The shipped build never samples or logs; its install line stays the only report.
//!
//! The pieces are pure and host-tested; `tests/hook_alloc.rs` proves the detector with a test stub that
//! really allocates inside a scope.

use core::fmt::{self, Display, Formatter};
use core::sync::atomic::{AtomicU64, Ordering};
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(not(target_os = "switch"))]
use std::cell::Cell;
#[cfg(target_os = "switch")]
use crate::runtime::hook_tls::{HOOK_DEPTH, LOAD_DEPTH, UNCOUNTED_DEPTH, CURRENT};

// Keep the enum, index order and report labels in one declaration.
macro_rules! hook_ids {
    ($( $(#[$meta:meta])* $variant:ident => $name:literal, )+) => {
        /// Every hook of the plugin that runs inside a HookScope, in report order.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(usize)]
        pub enum HookId { $( $(#[$meta])* $variant, )+ }

        /// Number of HookId values.
        pub const HOOK_COUNT: usize = HookId::Other as usize + 1;

        impl HookId {
            pub const ALL: [HookId; HOOK_COUNT] = [$(HookId::$variant),+];

            /// The name the report line uses (the Eden scripts match these).
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }
        }
    };
}

hook_ids! {
    InputCallerANote => "input_caller_a_note",
    InputCallerAApply => "input_caller_a_apply",
    InputCallerBNote => "input_caller_b_note",
    InputCallerBApply => "input_caller_b_apply",
    ResetCheck => "reset_check",
    HazardRule => "hazard_rule",
    ResetHub => "reset_hub",
    CpuFrame => "cpu_frame",
    CpuSdi => "cpu_sdi",
    CpuStickX => "cpu_stick_x",
    CpuStickY => "cpu_stick_y",
    CpuClatterStart => "cpu_clatter_start",
    CpuTransitionTerm => "cpu_transition_term",
    CpuMotionChange => "cpu_motion_change",
    CpuQuakePos => "cpu_quake_pos",
    CpuCheckButtonOn => "cpu_check_button_on",
    CpuCheckButtonOff => "cpu_check_button_off",
    CpuStickXNoClamp => "cpu_stick_x_no_clamp",
    CpuStickYNoClamp => "cpu_stick_y_no_clamp",
    CpuShieldParam => "cpu_shield_param",
    /// Package 5: the layout archive hook and the two UI anchors (R2-7).
    UiArchive => "ui_archive",
    UiPassCalculate => "ui_pass_calculate",
    UiRegisterCalculate => "ui_register_calculate",
    /// Package 4 S4-4: the live replay capture's hooks (the game's native replay recorder, the save actor, the fighter frame pair).
    LiveReplay => "live_replay",
    /// SPEC-core-06 visualizer hooks and per-frame pass.
    HitboxAttack => "hitbox_attack",
    HitboxCatch => "hitbox_catch",
    HitboxRebound => "hitbox_rebound",
    HitboxFrame => "hitbox_frame",
    HitboxOrbs => "hitbox_orbs",
    /// Package 6, nested inside the hooks above so an allocation is charged to the step that made it: the roster cycle's input
    /// step (P1 and the CPU operator), its per-frame steps (sources, Kirby hats, items, preload, the practice lease) and package
    /// 3's per-fighter state frame.
    RosterInput => "roster_input",
    RosterSources => "roster_sources",
    RosterKirby => "roster_kirby",
    RosterItems => "roster_items",
    RosterPreload => "roster_preload",
    PracticeLease => "practice_lease",
    StateFrame => "state_frame",
    /// Package 4 S4-4, nested the same way: the recorder's AI epilogue and P1 poll stages, the named layer's once-per-frame services
    /// (keyboard, character request, panel hold, clip reload) and its save-state restore follow-up.
    RecorderAi => "recorder_ai",
    RecorderPoll => "recorder_poll",
    NamedService => "named_service",
    NamedRestore => "named_restore",
    /// ui-screens, nested in the UI and input hooks so the menu's own allocations are charged to the step that made them (the lead's
    /// menu-open attribution, 2026-11-02): the menu's input adapter (open, close, presses), the tick's menu draw (panes, text), the stage
    /// grid's thumbnails, and any call a hook still makes into package 4's named-macro panel model (design C: none should remain).
    MenuInput => "menu_input",
    MenuDraw => "menu_draw",
    MenuThumbs => "menu_thumbs",
    MacroPanel => "macro_panel",
    DeveloperFloat => "developer_float",
    DeveloperBackshield => "developer_backshield",
    /// Phase 2 N11: the item spawner's switch test (Lobby Items), its one removal on Player 1's frame, and the timed
    /// generator's lottery test (#62).
    LobbyItems => "lobby_items",
    LobbyItemsRemove => "lobby_items_remove",
    LobbyItemsDrop => "lobby_items_drop",
    /// rc.6: the waiting room's practice panel, saved Training partners and behavior applied at entry, and the player's
    /// change saved (`lobby_persist`).
    LobbyPersistEntry => "lobby_persist_entry",
    LobbyPersistSave => "lobby_persist_save",
    /// rc.6 CPU level: the Training session clock (on the waiting-room poll) and the rank step on Player 1's frame.
    CpuLevelClock => "cpu_level_clock",
    CpuLevel => "cpu_level",
    /// rc.8 R2: the waiting area's class update (the 2/3 slot's pace and x1.5's extra scheduler pass, `lobby_speed`).
    LobbySpeed => "lobby_speed",
    /// Every hook that has no name of its own yet (`HookScope::enter()`): allocations inside them are
    /// still counted, their invocations are counted together.
    Other => "other",
}

#[cfg(not(target_os = "switch"))]
thread_local! {
    /// Host TLS oracle. Switch uses boot-reserved scalar SDK slots; its std TLS boxes allocate.
    static HOOK_DEPTH: Cell<u32> = const { Cell::new(0) };
    /// Nesting depth of per_load sections on this thread.
    static LOAD_DEPTH: Cell<u32> = const { Cell::new(0) };
    /// Nesting depth of uncounted sections on this thread.
    static UNCOUNTED_DEPTH: Cell<u32> = const { Cell::new(0) };
    /// The innermost hook scope on this thread (a `HookId` index): its steady-state allocations are charged to it.
    static CURRENT: Cell<u8> = const { Cell::new(HookId::Other as u8) };
}

static HOOK_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
/// Steady-state hook allocations per innermost scope (attribution of a nonzero R5 delta).
static ALLOCS: [AtomicU64; HOOK_COUNT] = [const { AtomicU64::new(0) }; HOOK_COUNT];
/// Allocations of the one-per-load kind inside a hook (per_load), counted apart from the steady-state ones.
static LOAD_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ENTRIES: [AtomicU64; HOOK_COUNT] = [const { AtomicU64::new(0) }; HOOK_COUNT];
/// Diag builds: the hook entered last on any thread, and the hook scopes open now (the stall watchdog's report).
#[cfg(feature = "diag")]
static LAST_ENTERED: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(HookId::Other as usize);
#[cfg(feature = "diag")]
static OPEN_SCOPES: core::sync::atomic::AtomicIsize = core::sync::atomic::AtomicIsize::new(0);

/// Every hook entry since boot, all hooks together (the stall watchdog's sample).
pub fn total_entries() -> u64 {
    ENTRIES.iter().map(|counter| counter.load(Ordering::Relaxed)).sum()
}

/// Diag builds: the hook entered last and the hook scopes open now, on all threads.
#[cfg(feature = "diag")]
pub fn last_entered() -> (&'static str, isize) {
    let id = LAST_ENTERED.load(Ordering::Relaxed).min(HOOK_COUNT - 1);
    (HookId::ALL[id].name(), OPEN_SCOPES.load(Ordering::Relaxed))
}

fn in_hook() -> bool {
    HOOK_DEPTH.try_with(|depth| depth.get() > 0).unwrap_or(false)
}

fn in_load() -> bool {
    LOAD_DEPTH.try_with(|depth| depth.get() > 0).unwrap_or(false)
}

/// Count one allocation made on this thread: nothing outside a hook; inside one, the steady-state counter (the lane gate
/// wants delta 0) or, inside per_load, the per-load counter.
fn count_allocation() {
    if UNCOUNTED_DEPTH.try_with(|depth| depth.get() > 0).unwrap_or(false) {
        return;
    }
    if in_hook() {
        if in_load() {
            LOAD_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        } else {
            HOOK_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            let id = CURRENT.try_with(|current| current.get()).map_or(HookId::Other as usize, usize::from);
            ALLOCS[id.min(HOOK_COUNT - 1)].fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// The byte meter of the diag heap features (`plugin-heap-meter`, `console-practice-trace`): this NRO's requested Rust allocation
/// sizes only (live, peak, total, call count, largest request); native game heaps, allocator metadata and other plugins are
/// excluded. It lives in the one global allocator of the plugin so that the diag features read these counters instead of each
/// declaring an allocator of their own (a crate has one `#[global_allocator]`). Counters are relaxed and a snapshot is not atomic
/// across fields; a realloc's temporary copy is outside the meter. The shipped build compiles the counters away: `grow`, `shrink`
/// and `moved` are empty and the allocator is byte-for-byte the hook counter it was.
#[cfg(any(test, feature = "plugin-heap-meter", feature = "console-practice-trace"))]
mod heap {
    use core::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::Relaxed};

    static LIVE: AtomicIsize = AtomicIsize::new(0);
    static PEAK: AtomicIsize = AtomicIsize::new(0);
    static TOTAL: AtomicUsize = AtomicUsize::new(0);
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static LARGEST: AtomicUsize = AtomicUsize::new(0);

    #[inline]
    pub(super) fn grow(size: usize) {
        let live = LIVE.fetch_add(size as isize, Relaxed) + size as isize;
        PEAK.fetch_max(live, Relaxed);
        TOTAL.fetch_add(size, Relaxed);
        CALLS.fetch_add(1, Relaxed);
        LARGEST.fetch_max(size, Relaxed);
    }

    #[inline]
    pub(super) fn shrink(size: usize) {
        LIVE.fetch_sub(size as isize, Relaxed);
    }

    /// A moved realloc briefly holds both blocks; counting only the new size understates that copy's instantaneous peak by the old
    /// size (acceptable for sparse profiling, as the fork's meter did).
    #[inline]
    pub(super) fn moved(old: usize, new: usize) {
        shrink(old);
        grow(new);
    }

    pub(super) fn snapshot() -> super::HeapBytes {
        super::HeapBytes {
            live: LIVE.load(Relaxed),
            peak: PEAK.load(Relaxed),
            total: TOTAL.load(Relaxed),
            calls: CALLS.load(Relaxed),
            largest: LARGEST.load(Relaxed),
        }
    }
}

#[cfg(not(any(test, feature = "plugin-heap-meter", feature = "console-practice-trace")))]
mod heap {
    #[inline(always)]
    pub(super) fn grow(_size: usize) {}
    #[inline(always)]
    pub(super) fn shrink(_size: usize) {}
    #[inline(always)]
    pub(super) fn moved(_old: usize, _new: usize) {}
}

/// What the byte meter has counted since boot.
#[cfg(any(test, feature = "plugin-heap-meter", feature = "console-practice-trace"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeapBytes {
    /// Bytes allocated and not yet freed.
    pub live: isize,
    /// The highest `live` seen.
    pub peak: isize,
    /// Every requested byte, freed or not.
    pub total: usize,
    /// Allocation calls (an allocation that moved on realloc counts once more).
    pub calls: usize,
    /// The largest single request.
    pub largest: usize,
}

/// Read the byte meter (the diag heap features and tests; a shipped build has no meter).
#[cfg(any(test, feature = "plugin-heap-meter", feature = "console-practice-trace"))]
pub fn heap_bytes() -> HeapBytes {
    heap::snapshot()
}

/// The counting allocator: forwards to the system allocator, counts the calls made inside a hook and (diag heap features only) the
/// bytes this NRO requested.
pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            heap::grow(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        let pointer = System.alloc_zeroed(layout);
        if !pointer.is_null() {
            heap::grow(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        heap::shrink(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        let moved = System.realloc(ptr, layout, new_size);
        if !moved.is_null() {
            heap::moved(layout.size(), new_size);
        }
        moved
    }
}

/// Marks the current thread as running hook code until dropped, and counts one invocation. Scopes nest: the innermost one is
/// charged with the allocations made while it is open (`ALLOCS`), and the enclosing scope is current again when it drops.
pub struct HookScope {
    previous: u8,
}

impl HookScope {
    /// A hook without a name of its own (counted under `HookId::Other`).
    pub fn enter() -> Self {
        Self::enter_hook(HookId::Other)
    }

    /// The hook `id`.
    pub fn enter_hook(id: HookId) -> Self {
        ENTRIES[id as usize].fetch_add(1, Ordering::Relaxed);
        let _ = HOOK_DEPTH.try_with(|depth| depth.set(depth.get() + 1));
        let previous = CURRENT.try_with(|current| current.replace(id as u8)).unwrap_or(HookId::Other as u8);
        #[cfg(feature = "diag")]
        {
            LAST_ENTERED.store(id as usize, Ordering::Relaxed);
            OPEN_SCOPES.fetch_add(1, Ordering::Relaxed);
        }
        #[cfg(feature = "diag")]
        if let Some(probe) = ENTRY_PROBE.get() {
            probe(id.name());
        }
        HookScope { previous }
    }
}

/// Diag builds: called with the hook's name at every `HookScope` entry (the runtime registers its stack-room probe,
/// `runtime::threads::headroom`; stream/stack). Absent from the shipped build.
#[cfg(feature = "diag")]
pub static ENTRY_PROBE: std::sync::OnceLock<fn(&'static str)> = std::sync::OnceLock::new();

impl Drop for HookScope {
    fn drop(&mut self) {
        #[cfg(feature = "diag")]
        OPEN_SCOPES.fetch_sub(1, Ordering::Relaxed);
        let _ = HOOK_DEPTH.try_with(|depth| depth.set(depth.get().saturating_sub(1)));
        let _ = CURRENT.try_with(|current| current.set(self.previous));
    }
}

/// Allocations made inside hooks since boot (the per-load ones of per_load are not in this number).
pub fn hook_allocations() -> u64 {
    HOOK_ALLOCATIONS.load(Ordering::Relaxed)
}

/// Allocations made inside per_load sections of hooks since boot.
pub fn load_allocations() -> u64 {
    LOAD_ALLOCATIONS.load(Ordering::Relaxed)
}

/// Run `work` without counting its allocations at all: diagnostic instrumentation that is not product code (the diag
/// build's stock-archive dump). Never for code the shipped build runs.
pub fn uncounted<R>(work: impl FnOnce() -> R) -> R {
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            let _ = UNCOUNTED_DEPTH.try_with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
    let _ = UNCOUNTED_DEPTH.try_with(|depth| depth.set(depth.get() + 1));
    let _leave = Leave;
    work()
}

/// Run work as the once-per-load part of a hook (the layout archive the game hands over once per Training load): its
/// allocations are counted apart (load_allocations), so the steady-state counter stays the proof that no hook allocates
/// per frame or per input. Allocations that can be made earlier (threads, queues) belong at boot, not here.
pub fn per_load<R>(work: impl FnOnce() -> R) -> R {
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            let _ = LOAD_DEPTH.try_with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }
    let _ = LOAD_DEPTH.try_with(|depth| depth.set(depth.get() + 1));
    let _leave = Leave;
    work()
}

/// The counters at one moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub allocations: u64,
    pub entries: [u64; HOOK_COUNT],
    /// Steady-state allocations charged to each innermost hook scope (they sum to `allocations`).
    pub allocs: [u64; HOOK_COUNT],
}

/// Read the live counters.
pub fn snapshot() -> Snapshot {
    let mut entries = [0u64; HOOK_COUNT];
    for (slot, counter) in entries.iter_mut().zip(&ENTRIES) {
        *slot = counter.load(Ordering::Relaxed);
    }
    let mut allocs = [0u64; HOOK_COUNT];
    for (slot, counter) in allocs.iter_mut().zip(&ALLOCS) {
        *slot = counter.load(Ordering::Relaxed);
    }
    Snapshot { allocations: hook_allocations(), entries, allocs }
}

/// One report: the counters now and what changed since the previous report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub now: Snapshot,
    /// Hook allocations since the previous observation (the lane gate wants 0).
    pub delta: u64,
    /// Hook invocations since the previous observation, all hooks.
    pub hook_calls: u64,
}

impl Observation {
    /// A hook that has run at least once since boot.
    pub fn covered(&self, id: HookId) -> bool {
        self.now.entries[id as usize] > 0
    }
}

impl Display for Observation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[TrainingRooms] hook_stats allocations={} delta={} hook_calls={} hooks=", self.now.allocations, self.delta, self.hook_calls)?;
        let mut any = false;
        for id in HookId::ALL {
            let n = self.now.entries[id as usize];
            if n > 0 {
                write!(f, "{}{}:{}", if any { "," } else { "" }, id.name(), n)?;
                any = true;
            }
        }
        if !any {
            f.write_str("none")?;
        }
        // Where the steady-state allocations happened (innermost scope), when there are any.
        if self.now.allocations > 0 {
            f.write_str(" allocs=")?;
            let mut first = true;
            for id in HookId::ALL {
                let n = self.now.allocs[id as usize];
                if n > 0 {
                    write!(f, "{}{}:{}", if first { "" } else { "," }, id.name(), n)?;
                    first = false;
                }
            }
        }
        Ok(())
    }
}

/// Turns samples of the counters into sparse report lines: a sample that shows no hook activity and no
/// allocation since the previous report produces nothing.
#[derive(Clone, Copy, Debug)]
pub struct Observer {
    last: Snapshot,
}

impl Default for Observer {
    fn default() -> Self {
        Self::new()
    }
}

impl Observer {
    pub const fn new() -> Self {
        Self { last: Snapshot { allocations: 0, entries: [0; HOOK_COUNT], allocs: [0; HOOK_COUNT] } }
    }

    /// Compare `now` with the previous report.
    pub fn observe(&mut self, now: Snapshot) -> Option<Observation> {
        let hook_calls: u64 = now.entries.iter().zip(&self.last.entries).map(|(n, l)| n.saturating_sub(*l)).sum();
        let delta = now.allocations.saturating_sub(self.last.allocations);
        if hook_calls == 0 && delta == 0 {
            return None;
        }
        self.last = now;
        Some(Observation { now, delta, hook_calls })
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn every_hook_has_a_unique_name_in_index_order() {
        for (i, id) in HookId::ALL.iter().enumerate() {
            assert_eq!(*id as usize, i);
            assert!(HookId::ALL.iter().filter(|other| other.name() == id.name()).count() == 1);
        }
    }

    #[test]
    fn a_per_load_allocation_is_counted_apart_from_the_steady_state_ones() {
        // The host test binary has no counting allocator, so drive the counting rule directly.
        let (hooks, loads) = (hook_allocations(), load_allocations());
        count_allocation();
        assert_eq!((hook_allocations(), load_allocations()), (hooks, loads), "outside a hook nothing is counted");
        let _scope = HookScope::enter_hook(HookId::UiArchive);
        per_load(count_allocation);
        assert_eq!((hook_allocations(), load_allocations()), (hooks, loads + 1));
        count_allocation();
        assert_eq!((hook_allocations(), load_allocations()), (hooks + 1, loads + 1), "a hook allocation outside per_load is the steady-state kind");
    }

    #[test]
    fn a_quiet_sample_reports_nothing_and_an_active_one_reports_once() {
        let mut observer = Observer::new();
        let quiet = Snapshot { allocations: 0, entries: [0; HOOK_COUNT], allocs: [0; HOOK_COUNT] };
        assert!(observer.observe(quiet).is_none());
        let mut busy = quiet;
        busy.entries[HookId::ResetCheck as usize] = 3;
        let first = observer.observe(busy).unwrap();
        assert_eq!((first.delta, first.hook_calls), (0, 3));
        assert!(first.covered(HookId::ResetCheck) && !first.covered(HookId::HazardRule));
        assert_eq!(first.to_string(), "[TrainingRooms] hook_stats allocations=0 delta=0 hook_calls=3 hooks=reset_check:3");
        assert!(observer.observe(busy).is_none(), "nothing new since the report");
    }
    }
}
