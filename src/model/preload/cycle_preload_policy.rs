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

