//! Value-only speculative scheduling. A cache target owns no native resource.
//! Demand can replace every unleased target. Speculation may replace a lower
//! priority target to make room for an earlier missing target. Native-pair
//! priority reserves constrained Miis before ordinary targets; the global plan
//! order stays intact. Active leases can reduce usable capacity below the count.

/// RC keeps exact saved Miis on their existing native helper pair.
/// Creating exclusive portrait destinations is deferred, not a Cargo closure change.
pub const fn created_mii_sources() -> bool { false }

/// A helper loads the complete native group. Fighter predictions share one
/// canonical cache key; hat predictions retain their exact copy-ability kind.
pub fn fighter_group(kind: u32) -> u32 {
    let forms = crate::engine::swap::character_forms::Forms::starting(kind);
    *forms.kinds[..forms.count as usize].iter().min().unwrap()
}

pub fn merge<T: Copy + Eq, const N:usize>(lanes: [[Option<T>; N]; 4], slots: u8) -> [Option<T>; N] {
    let mut result = [None; N];
    let mut count = 0;
    for depth in 0..N {
        for lane in lanes {
            if count >= usize::from(slots).min(crate::engine::preload::cycle_preload_capacity::MAX_TARGETS).min(N) {
                return result;
            }
            if let Some(target) = lane[depth] {
                if !result.contains(&Some(target)) {
                    result[count] = Some(target);
                    count += 1;
                }
            }
        }
    }
    result
}

/// Lanes of `merge`, in its order: Player hats, CPU hats, Player fighters, CPU fighters.
pub const PLAYER_HATS: usize = 0;
pub const CPU_FIGHTERS: usize = 3;

/// `merge` for a Player Kirby whose cycle has hats: every Player hat look-ahead ranks above the
/// CPU's fighter look-ahead. Each CPU fighter that `merge` would place before the last Player hat
/// moves, in its own order, to just behind that hat; every other pair of targets keeps `merge`'s
/// order, and a target in both lanes keeps the earlier, hat position. The cost is the CPU's next
/// swap, which may then find no warm helper for its fighter. Without a Player hat this is `merge`.
pub fn merge_player_hats_first<T: Copy + Eq, const N: usize>(lanes: [[Option<T>; N]; 4], slots: u8) -> [Option<T>; N] {
    let Some(last_hat_depth) = lanes[PLAYER_HATS].iter().rposition(Option::is_some) else {
        return merge(lanes, slots);
    };
    // Visit the same three stable partitions directly. The last hat is lane 0,
    // so the head includes other lanes only at strictly earlier depths.
    let head = || (0..=last_hat_depth).flat_map(|depth| {
        lanes.iter().enumerate().filter_map(move |(lane, targets)| {
            (depth < last_hat_depth || lane == PLAYER_HATS)
                .then_some((lane, targets[depth]))
                .and_then(|(lane, target)| target.map(|target| (lane, target)))
        })
    });
    let order = head().filter(|(lane, _)| *lane != CPU_FIGHTERS)
        .chain(head().filter(|(lane, _)| *lane == CPU_FIGHTERS))
        .chain((last_hat_depth..N).flat_map(|depth| {
            lanes.iter().enumerate().filter_map(move |(lane, targets)| {
                (depth > last_hat_depth || lane != PLAYER_HATS)
                    .then_some((lane, targets[depth]))
                    .and_then(|(lane, target)| target.map(|target| (lane, target)))
            })
        }));
    let limit = usize::from(slots).min(crate::engine::preload::cycle_preload_capacity::MAX_TARGETS).min(N);
    let mut result = [None; N];
    let mut count = 0;
    for (_, target) in order {
        if count >= limit {
            break;
        }
        if !result.contains(&Some(target)) {
            result[count] = Some(target);
            count += 1;
        }
    }
    result
}

/// Sources include ready and in-flight entries. Protected/leased entries still
/// satisfy a target but can never be selected as a replacement by this policy.
pub fn missing<T: Copy + Eq, const N:usize>(
    targets: [Option<T>; N],
    sources: [Option<T>; 8],
    slot: usize,
    replaceable: bool,
    demand_pending: bool,
) -> Option<T> {
    if slot >= sources.len()
        || !replaceable
        || demand_pending
    {
        return None;
    }
    let (priority, target) = targets
        .iter()
        .copied()
        .enumerate()
        .find_map(|(priority, target)| target
            .filter(|target| !sources.contains(&Some(*target)))
            .map(|target| (priority, target)))?;
    let current_priority = sources[slot]
        .and_then(|old| targets.iter().position(|target| *target == Some(old)));
    current_priority.is_none_or(|current| priority < current).then_some(target)
}

/// [`missing`] for a parked F9 seat (entry 2 or 3 at fewer CPUs), whose one target is its own member. Only its own entry
/// can hold that member: the member on the other seat's entry doesn't count for it (primary inbox 2026-11-09 06:16Z, Eden
/// rc6-bg-pf4: after a CPU switch the two seats' members sat swapped, each entry idle, the notice at "Loading 4/6" and
/// the preload grace unasked until a P1 switch, 160 s).
pub fn seat_missing<T: Copy + Eq, const N:usize>(
    targets: [Option<T>; N],
    sources: [Option<T>; 8],
    slot: usize,
    replaceable: bool,
    demand_pending: bool,
) -> Option<T> {
    let mut own = [None; 8];
    if let (Some(cell), Some(source)) = (own.get_mut(slot), sources.get(slot)) {
        *cell = *source;
    }
    missing(targets, own, slot, replaceable, demand_pending)
}

/// An exact Mii fits only the native pair. A configuration is exact even when
/// the kind alone would look ordinary; Mii class kinds are exact either way.
pub fn exact_mii(kind: u32, configured: bool) -> bool {
    configured || (0x48..=0x4a).contains(&kind)
}

/// Slot-local view of an unchanged global plan. A scarce native-pair slot keeps
/// every target and stably moves exact Miis ahead of ordinary ones, so a later
/// Mii can replace an earlier ordinary resident. An ordinary slot keeps plan
/// order and cannot be assigned an exact Mii. Readiness continues to use the
/// caller's original plan, not this view.
pub fn schedulable<T: Copy, const N: usize>(
    targets: [Option<T>; N],
    scarce: bool,
    constrained: impl Fn(T) -> bool,
) -> [Option<T>; N] {
    let mut result = [None; N];
    let mut count = 0;
    if scarce {
        for target in targets.iter().copied().flatten() {
            if constrained(target) && count < N {
                result[count] = Some(target);
                count += 1;
            }
        }
    }
    for target in targets.iter().copied().flatten() {
        if !constrained(target) && count < N {
            result[count] = Some(target);
            count += 1;
        }
    }
    result
}

/// What the user should be told about the active speculative plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Preload is Off, the backend has no spare helper, or there is nothing to load.
    Hidden,
    Loading,
    /// Owed fighter loads are deferred by an accepted visible operation, native UI, or replay.
    /// The preloader's own helper and hat loads never pause it, and nothing pauses a swap the
    /// user still owes (`Waiting`).
    Paused,
    /// Nothing more will load, but a planned target failed: never reported Ready.
    Partial,
    /// Fighters only: no fighter load is owed, yet planned fighters have no window and wait for
    /// the swap of the role that builds it (`BuiltFighters::swap`).
    Waiting,
    /// Fighters only: no more background work. Every planned fighter is built or prefetched
    /// (reachable without a reload) and none failed, but a prefetched group is not built.
    Done,
    /// Every planned target is built. For fighters this is the C113 §6 rule: only built counts.
    Ready,
}

/// Counts are over the active plan only: the denominator is the number of planned
/// targets (never the configured slot count), failed ones included, and a resident
/// counts only when it matches a non-failed one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readiness {
    pub progress: Progress,
    pub ready: u8,
    pub planned: u8,
    pub configured: u8,
    /// Backend capacity: the most targets this scene can hold.
    pub limit: u8,
    /// Planned targets abandoned after a failed load; still part of `planned`.
    pub failed: u8,
}

/// Value receipts for the extra fighters retained in visible player entries.
/// Assets in helper entries never count here. Reservations are pending until
/// native callbacks prove that construction and ownership checks completed.
///
/// `planned == ready + prefetched + failed + pending + unstarted()`. `ready` is built (the next
/// switch is instant, ~50 ms), `prefetched` is a loaded group that is not built (reachable without
/// a reload; its first switch constructs the bodies: an ordinary group through the game's queued
/// construction with the fighter absent about a second while the game runs (Phase B′), a Mii,
/// native or pair group synchronously, ~0.35-1.0 s), `pending` is work still owed (a window building
/// or switching, a group planned or prefetching) and `unstarted()` the planned fighters whose
/// window does not exist yet, which the first swap builds. `swap` names the roles with such
/// fighters (`body_bank::SWAP_PLAYER`/`SWAP_CPU`, from `RoleCounts::swap_owed`): only that
/// role's swap builds them. `held` fighters are planned groups whose start waits for process
/// memory or free pool bodies (`hold` names which; memory first): retried, never `failed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuiltFighters {
    pub planned: u8,
    pub ready: u8,
    pub prefetched: u8,
    pub failed: u8,
    pub pending: u8,
    pub swap: u8,
    pub held: u8,
    pub hold: Option<crate::engine::swap::body_bank::Hold>,
}

impl BuiltFighters {
    pub fn unstarted(self) -> u8 {
        let accounted = u16::from(self.ready) + u16::from(self.prefetched)
            + u16::from(self.failed) + u16::from(self.pending) + u16::from(self.held);
        u16::from(self.planned).saturating_sub(accounted) as u8
    }

    /// Who the Waiting text asks to swap; empty when the receipts name no role.
    fn swap_who(self) -> &'static str {
        use crate::engine::swap::body_bank::{SWAP_CPU, SWAP_PLAYER};
        let player = (self.swap & SWAP_PLAYER) != 0;
        let cpu = (self.swap & SWAP_CPU) != 0;
        match (player, cpu) {
            (true, false) => "P1",
            (false, true) => "the CPU",
            (true, true) => "both",
            (false, false) => "",
        }
    }

    /// Fighters a switch reaches without a reload: built or prefetched.
    pub fn loaded(self) -> u8 {
        self.ready.saturating_add(self.prefetched)
    }
}

/// The notice describes built and prefetched fighters only. Asset progress remains diagnostic
/// information and cannot complete or retrigger the user-facing count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheStatus {
    pub assets: Readiness,
    pub fighters: BuiltFighters,
    /// An accepted visible operation, native UI, a save-state load or a replay holds the loader
    /// (`character_roster::notice_blocked` and the UI gates); never the preloader's own helper,
    /// hat or group-prefetch loads.
    pub blocked: bool,
}

/// A copied status formatter. Creating, displaying and debugging it allocate
/// nothing; Debug preserves the quoted text used by existing diagnostics.
#[derive(Clone, Copy)]
pub struct CacheText(CacheStatus);
impl std::fmt::Display for CacheText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fighters = self.0.fighters;
        let (loaded, planned, failed) = (fighters.loaded(), fighters.planned, fighters.failed);
        match self.0.progress() {
            Progress::Hidden => return Ok(()),
            Progress::Loading => write!(f, "Loading {}/{}", loaded, planned)?,
            Progress::Paused => write!(f, "Paused {}/{}", loaded, planned)?,
            Progress::Waiting if loaded == 0 && failed == 0 => return match fighters.swap_who() {
                "" => f.write_str("Swap once to load"),
                who => write!(f, "Swap {} once to load", who),
            },
            Progress::Waiting => match fighters.swap_who() {
                "" => write!(f, "Loaded {}/{} · swap to load {} more", loaded, planned, fighters.unstarted())?,
                who => write!(f, "Loaded {}/{} · swap {} to load {} more", loaded, planned, who, fighters.unstarted())?,
            },
            Progress::Partial => write!(f, "Loaded {}/{}", loaded, planned)?,
            Progress::Done => return write!(f, "All loaded ({}/{}, {} instant)", loaded, planned, fighters.ready),
            Progress::Ready => return write!(f, "Ready {}/{}", fighters.ready, planned),
        }
        use crate::engine::swap::body_bank::Hold;
        let held = fighters.held;
        match (failed > 0, fighters.hold.filter(|_| held > 0)) {
            (true, Some(hold)) => {
                let what = if hold == Hold::Memory { "memory" } else { "pool space" };
                // restriction: preload-failed-and-waiting
                write!(f, " ({} failed, {} waiting for {})", failed, held, what)?
            }
            (true, None) => write!(f, " ({} failed)", failed)?,
            // restriction: preload-wait-memory
            (false, Some(Hold::Memory)) => write!(f, " ({} waiting for memory)", held)?,
            // restriction: preload-wait-pool
            (false, Some(Hold::Pool)) => write!(f, " ({} waiting for pool space)", held)?,
            (false, None) => {}
        }
        Ok(())
    }
}
impl std::fmt::Debug for CacheText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // All words are fixed literals and all substitutions are integers.
        write!(f, "\"{}\"", self)
    }
}

impl CacheStatus {
    pub fn new(assets: Readiness, fighters: BuiltFighters, blocked: bool) -> Self {
        Self { assets, fighters, blocked }
    }

    /// Hidden: Off or nothing planned. Loading: fighter work is owed and runs; Paused: it is owed
    /// and a visible operation defers it. Waiting: no fighter work is owed, but a window is not
    /// built and only that role's swap builds it, so a blocked loader cannot pause it. Otherwise
    /// the plan is settled: Partial (a failure, or a group held for memory or pool space), Done (a prefetched group remains unbuilt), Ready
    /// (all built).
    pub fn progress(self) -> Progress {
        let fighters = self.fighters;
        if self.assets.configured == 0 || fighters.planned == 0 {
            return Progress::Hidden;
        }
        if fighters.pending > 0 {
            if self.blocked { Progress::Paused } else { Progress::Loading }
        }
        else if fighters.unstarted() > 0 { Progress::Waiting }
        else if fighters.failed > 0 || fighters.held > 0 { Progress::Partial }
        else if fighters.prefetched > 0 { Progress::Done }
        else { Progress::Ready }
    }

    /// Formatting is deferred into the logger/notice caller's fixed buffer.
    pub fn text(self) -> CacheText { CacheText(self) }

    /// A settled result is the one worth reading; a waiting hint needs a moment; transient
    /// progress is replaced by the next change.
    pub fn frames(self) -> u32 {
        match self.progress() {
            Progress::Ready | Progress::Done | Progress::Partial => 180,
            Progress::Waiting => 120,
            _ => 90,
        }
    }

    /// What the notice shows, without what changes afterwards for no reader's benefit: a settled
    /// result is the same whether a prefetched group has since been built, so that is no change.
    fn shown_key(self) -> (u8, u8, u8, u8, u8, u8) {
        let (class, swap) = match self.progress() {
            Progress::Hidden => (0, 0),
            Progress::Loading => (1, 0),
            Progress::Paused => (2, 0),
            // The Waiting text names who must swap, so a change of role is a change shown.
            Progress::Waiting => (3, self.fighters.swap),
            Progress::Partial | Progress::Done | Progress::Ready => (4, 0),
        };
        (class, self.fighters.loaded(), self.fighters.planned, self.fighters.failed, swap, self.fighters.held)
    }

    pub fn announce(self, shown: Option<Self>, paused_polls: u8) -> bool {
        self.progress() != Progress::Hidden
            && (self.progress() != Progress::Paused || paused_polls >= PAUSE_POLLS)
            && shown.is_none_or(|old| old.shown_key() != self.shown_key())
    }
}

/// `ready` lists current-epoch sources whose load has completed. A failed or
/// still-loading source, including one leased by a consumer, is not in it.
/// A failed target is never ready even if a source for it is resident.
pub fn readiness<T: Copy + Eq, const N: usize>(
    configured: u8,
    limit: u8,
    blocked: bool,
    plan: [Option<T>; N],
    failed: impl Fn(T) -> bool,
    ready: [Option<T>; 8],
) -> Readiness {
    let cap = usize::from(configured.min(limit));
    let (mut planned, mut loaded, mut lost) = (0u8, 0u8, 0u8);
    for target in plan.iter().take(cap).flatten().copied() {
        planned += 1;
        if failed(target) {
            lost += 1;
        } else {
            loaded += u8::from(ready.contains(&Some(target)));
        }
    }
    let progress = if planned == 0 {
        Progress::Hidden
    } else if loaded == planned {
        Progress::Ready
    } else if loaded + lost == planned {
        Progress::Partial
    } else if blocked {
        Progress::Paused
    } else {
        Progress::Loading
    };
    Readiness { progress, ready: loaded, planned, configured, limit, failed: lost }
}

/// Paused is shown only after this many consecutive blocked polls, so an
/// ordinary swap does not flash a notice.
pub const PAUSE_POLLS: u8 = 2;

impl Readiness {
    /// Backend capacity, not the plan size, is what held the plan below the setting.
    pub fn limited(self) -> bool {
        self.limit < self.configured && self.planned >= self.limit
    }

    #[cfg(test)]
    fn key(self) -> (Progress, u8, u8, u8, bool) {
        (self.progress, self.ready, self.planned, self.failed, self.limited())
    }

    #[cfg(test)]
    pub fn text(self) -> String {
        let word = match self.progress {
            Progress::Hidden => return String::new(),
            Progress::Loading => "Loading",
            Progress::Paused => "Paused",
            Progress::Partial => "Partial",
            // Asset readiness has neither: they are fighter states (`CacheStatus`).
            Progress::Waiting => "Waiting",
            Progress::Done => "Done",
            Progress::Ready => "Ready",
        };
        let mut text = format!("{word} {}/{}", self.ready, self.planned);
        if self.limited() {
            text.push_str(&format!(" (max {})", self.limit));
        }
        if self.failed > 0 {
            text.push_str(&format!(" ({} failed)", self.failed));
        }
        text
    }

    /// Notification lifetime in frames. A settled result is the one worth reading;
    /// transient progress is replaced by the next change.
    #[cfg(test)]
    pub fn frames(self) -> u32 {
        if matches!(self.progress, Progress::Ready | Progress::Partial) { 180 } else { 90 }
    }
}

/// Announce only a change in what is displayed, never on every poll.
#[cfg(test)]
pub fn announce(shown: Option<Readiness>, now: Readiness, paused_polls: u8) -> bool {
    now.progress != Progress::Hidden
        && (now.progress != Progress::Paused || paused_polls >= PAUSE_POLLS)
        && shown.map(Readiness::key) != Some(now.key())
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    
    const NONE: fn(i32) -> bool = |_| false;
    fn ready_of(kinds: &[i32]) -> [Option<i32>; 8] {
        let mut ready = [None; 8];
        for (i, kind) in kinds.iter().enumerate() {
            ready[i] = Some(*kind);
        }
        ready
    }
    /// Primary inbox 06:16Z (Eden rc6-bg-pf4): with one CPU, entries 2 and 3 are parked seats for the CPU rotation's next
    /// two members. After the CPU's switch to 0xd its seats wanted 0x5c (entry 2) and 0xb (entry 3), while entry 2 held
    /// 0xb and entry 3 held 0x5c. `missing` found each target on the other entry and both stayed idle. A seat counts only
    /// its own entry, so each loads its own member; an ordinary helper still finds a target anywhere.
    fn a_parked_seat_loads_its_member_even_when_the_other_seat_holds_it() {
        let (first, second) = (0x5c, 0x0b);
        let mut sources = [None; 8];
        sources[2] = Some(second);
        sources[3] = Some(first);
        let mut seat2 = [None; 1];
        seat2[0] = Some(first);
        let mut seat3 = [None; 1];
        seat3[0] = Some(second);
        assert_eq!(seat_missing(seat2, sources, 2, true, false), Some(first), "entry 2 loads its member");
        assert_eq!(seat_missing(seat3, sources, 3, true, false), Some(second), "entry 3 loads its member");
        assert_eq!(missing(seat2, sources, 2, true, false), None, "an ordinary helper finds it on entry 3");
        // A seat that holds its member stays; the seat rules for a protected or busy entry are unchanged.
        let mut own = sources;
        own[2] = Some(first);
        assert_eq!(seat_missing(seat2, own, 2, true, false), None);
        assert_eq!(seat_missing(seat3, sources, 3, false, false), None, "not replaceable");
        assert_eq!(seat_missing(seat3, sources, 3, true, true), None, "a demand pending");
    }

    fn warm_assets_never_claim_fighters_are_ready() {
        let assets = readiness(5, 2, false, [Some(1), Some(2)], NONE, ready_of(&[1, 2]));
        let planned = BuiltFighters { planned: 3, ..BuiltFighters::default() };
        // No window yet and no load running: not "Ready 0/3", the first swap loads them.
        let status = CacheStatus::new(assets, planned, false);
        assert_eq!(status.progress(), Progress::Waiting);
        assert_eq!(status.text().to_string(), "Swap once to load");
        let building = CacheStatus::new(assets, BuiltFighters { pending: 3, ..planned }, false);
        assert_eq!(building.progress(), Progress::Loading);
        assert_eq!(building.text().to_string(), "Loading 0/3");
        assert!(building.announce(Some(status), 0));
        let complete = CacheStatus::new(assets, BuiltFighters { ready: 3, ..planned }, false);
        assert_eq!(complete.text().to_string(), "Ready 3/3");
        assert_eq!(complete.progress(), Progress::Ready);
        assert!(complete.frames() > building.frames() && complete.frames() > status.frames());
        assert!(complete.announce(Some(status), 0));
        assert!(!complete.announce(Some(complete), 0));
        let loading_assets = CacheStatus::new(Readiness { ready: 0, ..assets },
            complete.fighters, false);
        assert_eq!(loading_assets.progress(), Progress::Ready);
        assert!(!loading_assets.announce(Some(complete), 0));
    }

    /// g03 frame 85 of tr-p23-hot-measure-1 read "Loaded 2/5 (3 failed)" for a group the memory admission
    /// refused at its start. A held group is named for what it waits for, never as a failure, and a change
    /// in the held count is a change shown.
    fn held_fighters_read_waiting_not_failed() {
        use crate::engine::swap::body_bank::Hold;
        let assets = readiness(5, 1, false, [Some(1)], NONE, ready_of(&[1]));
        let memory = BuiltFighters { planned: 5, ready: 2, held: 3, hold: Some(Hold::Memory), ..BuiltFighters::default() };
        let held = CacheStatus::new(assets, memory, false);
        assert_eq!(memory.unstarted(), 0);
        assert_eq!(held.progress(), Progress::Partial);
        assert_eq!(held.text().to_string(), "Loaded 2/5 (3 waiting for memory)");
        let pool = CacheStatus::new(assets, BuiltFighters { hold: Some(Hold::Pool), ..memory }, false);
        assert_eq!(pool.text().to_string(), "Loaded 2/5 (3 waiting for pool space)");
        let both = CacheStatus::new(assets, BuiltFighters { failed: 1, held: 2, ..memory }, false);
        assert_eq!(both.text().to_string(), "Loaded 2/5 (1 failed, 2 waiting for memory)");
        assert!(held.announce(Some(both), 0), "a change in what is held is shown");
        let loading = CacheStatus::new(assets, BuiltFighters { pending: 1, held: 2, ..memory }, false);
        assert_eq!(loading.text().to_string(), "Loading 2/5 (2 waiting for memory)");
        let admitted = CacheStatus::new(assets, BuiltFighters { ready: 5, held: 0, hold: None, ..memory }, false);
        assert_eq!(admitted.text().to_string(), "Ready 5/5");
    }

    fn fighters(planned: u8, ready: u8, prefetched: u8, failed: u8, pending: u8) -> BuiltFighters {
        BuiltFighters { planned, ready, prefetched, failed, pending, ..BuiltFighters::default() }
    }

    fn status(fighters: BuiltFighters) -> CacheStatus {
        CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])), fighters, false)
    }

    /// The two roles' receipts as `native_body_bank::progress` adds them, with each role's
    /// `RoleCounts::swap_owed` bit.
    fn both_roles(player: BuiltFighters, cpu: BuiltFighters) -> BuiltFighters {
        use crate::engine::swap::body_bank::RoleCounts;
        let mut total = BuiltFighters::default();
        for (role, f) in [player, cpu].iter().enumerate() {
            let counts = RoleCounts { planned: f.planned, ready: f.ready, prefetched: f.prefetched,
                failed: f.failed, pending: f.pending, held: f.held, hold: f.hold };
            assert_eq!(counts.unstarted(), f.unstarted());
            total = BuiltFighters { planned: total.planned + f.planned, ready: total.ready + f.ready,
                prefetched: total.prefetched + f.prefetched, failed: total.failed + f.failed,
                pending: total.pending + f.pending, swap: total.swap | counts.swap_owed(role),
                held: total.held + f.held, hold: crate::engine::swap::body_bank::merge_hold(total.hold, f.hold) };
        }
        total
    }

    fn the_waiting_text_names_the_role_that_must_swap() {
        // C116.2 console: P1 Kirby's window built (2), the CPU's next window not built (1).
        let kirby = fighters(2, 2, 0, 0, 0);
        let cpu = fighters(1, 0, 0, 0, 0);
        let total = both_roles(kirby, cpu);
        assert_eq!(total.swap, crate::engine::swap::body_bank::SWAP_CPU);
        for blocked in [false, true] {
            let now = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])), total, blocked);
            assert_eq!((now.progress(), now.text().to_string()),
                (Progress::Waiting, "Loaded 2/3 · swap the CPU to load 1 more".to_string()));
        }
        let player = status(both_roles(fighters(2, 0, 0, 0, 0), fighters(1, 1, 0, 0, 0)));
        assert_eq!(player.text().to_string(), "Loaded 1/3 · swap P1 to load 2 more");
        let both = status(both_roles(fighters(2, 0, 0, 0, 0), fighters(1, 0, 0, 0, 0)));
        assert_eq!(both.text().to_string(), "Swap both once to load");
        let first = status(both_roles(fighters(2, 0, 0, 0, 0), BuiltFighters::default()));
        assert_eq!(first.text().to_string(), "Swap P1 once to load");
        let failed = status(both_roles(fighters(3, 1, 0, 1, 0), fighters(1, 0, 0, 0, 0)));
        assert_eq!(failed.text().to_string(), "Loaded 1/4 · swap both to load 2 more (1 failed)");
        // Another role now owes the swap: the text changes, so it is shown again.
        assert!(player.announce(Some(status(total)), 0));
        assert!(!player.announce(Some(player), 0));
        // A role owing a swap while the other's load is still owed reads as that load.
        let mixed = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])),
            both_roles(fighters(2, 0, 0, 0, 2), fighters(1, 0, 0, 0, 0)), true);
        assert_eq!((mixed.progress(), mixed.text().to_string()), (Progress::Paused, "Paused 0/3".to_string()));
    }

    fn player_kirby_hats_rank_above_the_cpu_fighter_lookahead() {
        let (mario, sephiroth, steve, dk, ics, ridley) = (0, 0x59, 0x53, 1, 0x10, 0x2e);
        // C116.2 console: P1 Kirby cycling three hats, the CPU's next fighters DK and ICs, one P1
        // fighter beyond the Kirby window.
        let lanes = [
            [Some(sephiroth), Some(steve), Some(mario), None, None],
            [None; 5],
            [None, None, Some(ridley), None, None],
            [Some(dk), Some(ics), None, None, None],
        ];
        // Today's order gave DK the second of three slots.
        assert_eq!(merge(lanes, 3), [Some(sephiroth), Some(dk), Some(steve), None, None]);
        assert_eq!(merge_player_hats_first(lanes, 3), [Some(sephiroth), Some(steve), Some(mario), None, None]);
        // The CPU's fighters follow the last hat in their own order, still ahead of the later P1
        // fighter as in today's order.
        assert_eq!(merge_player_hats_first(lanes, 5), [Some(sephiroth), Some(steve), Some(mario), Some(dk), Some(ics)]);
        let all = merge_player_hats_first::<i32, 6>([
            [Some(sephiroth), Some(steve), Some(mario), None, None, None], [None; 6],
            [None, None, Some(ridley), None, None, None], [Some(dk), Some(ics), None, None, None, None]], 255);
        assert_eq!(all.iter().flatten().count(), crate::engine::preload::cycle_preload_capacity::MAX_TARGETS.min(6));
        assert_eq!(merge(lanes, 5), [Some(sephiroth), Some(dk), Some(steve), Some(ics), Some(mario)]);
        // Every other order is today's: P1 hats keep interleaving with CPU hats and P1 fighters.
        let mixed = [
            [Some(10), Some(11), None, None, None],
            [Some(20), None, None, None, None],
            [Some(30), Some(31), None, None, None],
            [Some(40), Some(41), None, None, None],
        ];
        assert_eq!(merge(mixed, 5), [Some(10), Some(20), Some(30), Some(40), Some(11)]);
        assert_eq!(merge_player_hats_first(mixed, 5), [Some(10), Some(20), Some(30), Some(11), Some(40)]);
        assert_eq!(merge_player_hats_first(mixed, 255), [Some(10), Some(20), Some(30), Some(11), Some(40)]);
        // A CPU fighter that is also a coming hat keeps the earlier, hat position.
        let shared = [
            [Some(mario), Some(steve), None, None, None],
            [None; 5],
            [None; 5],
            [Some(steve), Some(dk), None, None, None],
        ];
        assert_eq!(merge_player_hats_first(shared, 5), [Some(mario), Some(steve), Some(dk), None, None]);
        // No Player hat: exactly `merge`.
        let plain = [[None; 5], [Some(20), None, None, None, None], [Some(30), None, None, None, None],
            [Some(40), Some(41), None, None, None]];
        for slots in 0..=6 {
            assert_eq!(merge_player_hats_first(plain, slots), merge(plain, slots));
            assert_eq!(merge_player_hats_first(lanes, slots).iter().flatten().count(),
                merge(lanes, slots).iter().flatten().count());
        }
    }

    fn announces_changes_once_and_debounces_pause() {
        let plan = [Some(1), Some(2), Some(3)];
        let loading = readiness(3, 3, false, plan, NONE, ready_of(&[1]));
        let more = readiness(3, 3, false, plan, NONE, ready_of(&[1, 2]));
        let ready = readiness(3, 3, false, plan, NONE, ready_of(&[1, 2, 3]));
        let paused = readiness(3, 3, true, plan, NONE, ready_of(&[1]));
        assert!(announce(None, loading, 0));
        assert!(!announce(Some(loading), loading, 0));
        assert!(announce(Some(loading), more, 0));
        assert!(announce(Some(more), ready, 0));
        assert!(!announce(Some(ready), ready, 0));
        // Hidden clears the shown state so the same Ready announces again later.
        assert!(announce(None, ready, 0));
        assert!(!announce(Some(ready), readiness(0, 3, false, plan, NONE, [None; 8]), 0));
        assert!(!announce(Some(loading), paused, 1));
        assert!(announce(Some(loading), paused, PAUSE_POLLS));
        assert!(!announce(Some(paused), paused, PAUSE_POLLS));
        // Ready outranks a pause counter left over from earlier polls.
        assert!(announce(Some(paused), ready, PAUSE_POLLS));
        // The ready notice outlasts transient progress, which the next change replaces.
        assert!(ready.frames() > loading.frames() && ready.frames() > paused.frames());
        // A settled partial is announced once, then stays quiet rather than re-raising.
        let partial = readiness(3, 3, false, plan, |k| k == 3, ready_of(&[1, 2]));
        assert_eq!(partial.progress, Progress::Partial);
        assert!(announce(Some(more), partial, 0));
        assert!(!announce(Some(partial), partial, 0));
        assert_eq!(partial.frames(), ready.frames());
    }
    fn native_forms_share_one_cache_target() {
        assert_eq!(fighter_group(0x5a), fighter_group(0x5b));
        assert_eq!(fighter_group(0x24), fighter_group(0x25));
        assert_eq!(fighter_group(0x24), fighter_group(0x26));
        assert_ne!(fighter_group(0), fighter_group(1));
        assert_eq!(
            merge(
                [
                    [Some(fighter_group(0x5a)), None],
                    [Some(fighter_group(0x5b)), None],
                    [Some(0), None],
                    [None, None]
                ],
                2
            ),
            [Some(fighter_group(0x5a)), Some(0)]
        );
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Target {
        kind: u32,
        config: u16,
        epoch: u8,
    }
    fn constrained(target: Target) -> bool { exact_mii(target.kind, target.config != 0) }
    fn plan_of(items: [Target; 5]) -> [Option<Target>; 5] { items.map(Some) }
    const FALCON: Target = Target { kind: 0x0b, config: 0, epoch: 1 };
    const MARIO: Target = Target { kind: 0x00, config: 0, epoch: 1 };
    const SEPHIROTH: Target = Target { kind: 0x59, config: 0, epoch: 1 };
    const GAME_WATCH: Target = Target { kind: 0x1c, config: 0, epoch: 1 };
    const GUNNER: Target = Target { kind: 0x4a, config: 1, epoch: 1 };
    /// Global order stays ordinary-first. The exact Mii is last on purpose.
    fn kirby_plan() -> [Option<Target>; 5] {
        plan_of([FALCON, MARIO, SEPHIROTH, GAME_WATCH, GUNNER])
    }
    fn occupy(slots: &[(usize, Target)]) -> [Option<Target>; 8] {
        let mut sources = [None; 8];
        for (slot, target) in slots { sources[*slot] = Some(*target); }
        sources
    }
    fn a_failed_mii_stays_in_the_original_indicator_and_is_not_scheduled() {
        let plan = kirby_plan();
        let mut schedulable_plan = plan;
        schedulable_plan[4] = None;
        let view = schedulable(schedulable_plan, true, constrained);
        assert!(view.iter().flatten().all(|target| target.kind != GUNNER.kind));
        assert_eq!(view[0], Some(FALCON));
        let sources = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, GAME_WATCH)]);
        assert_eq!(missing(view, sources, 2, true, false), None);
        let ready = readiness(5, 5, false, plan, |target| target.kind == GUNNER.kind, sources);
        assert_eq!((ready.progress, ready.ready, ready.planned, ready.failed), (Progress::Partial, 4, 5, 1));
        assert_eq!(ready.text().to_string(), "Partial 4/5 (1 failed)");
        assert_eq!(plan[4], Some(GUNNER));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_parked_seat_loads_its_member_even_when_the_other_seat_holds_it", a_parked_seat_loads_its_member_even_when_the_other_seat_holds_it as fn()),
            ("warm_assets_never_claim_fighters_are_ready", warm_assets_never_claim_fighters_are_ready as fn()),
            ("held_fighters_read_waiting_not_failed", held_fighters_read_waiting_not_failed as fn()),
            ("the_waiting_text_names_the_role_that_must_swap", the_waiting_text_names_the_role_that_must_swap as fn()),
            ("player_kirby_hats_rank_above_the_cpu_fighter_lookahead", player_kirby_hats_rank_above_the_cpu_fighter_lookahead as fn()),
            ("announces_changes_once_and_debounces_pause", announces_changes_once_and_debounces_pause as fn()),
            ("native_forms_share_one_cache_target", native_forms_share_one_cache_target as fn()),
            ("a_failed_mii_stays_in_the_original_indicator_and_is_not_scheduled", a_failed_mii_stays_in_the_original_indicator_and_is_not_scheduled as fn()),
        ]);
    }
    }
}
