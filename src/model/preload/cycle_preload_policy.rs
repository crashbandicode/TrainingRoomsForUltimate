//! Value-only speculative scheduling. A cache target owns no native resource.
//! Demand can replace every unleased target. Speculation may replace a lower
//! priority target to make room for an earlier missing target. Native-pair
//! priority reserves constrained Miis before ordinary targets; the global plan
//! order stays intact. Active leases can reduce usable capacity below the count.

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
    let mut order: Vec<(usize, T)> = Vec::with_capacity(4 * N);
    for depth in 0..N {
        for (lane, targets) in lanes.iter().enumerate() {
            if let Some(target) = targets[depth] {
                order.push((lane, target));
            }
        }
    }
    if let Some(last_hat) = order.iter().rposition(|(lane, _)| *lane == PLAYER_HATS) {
        let (head, tail) = order.split_at(last_hat + 1);
        let mut ranked: Vec<(usize, T)> = head.iter().copied().filter(|(lane, _)| *lane != CPU_FIGHTERS).collect();
        ranked.extend(head.iter().copied().filter(|(lane, _)| *lane == CPU_FIGHTERS));
        ranked.extend_from_slice(tail);
        order = ranked;
    }
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
/// role's swap builds them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuiltFighters {
    pub planned: u8,
    pub ready: u8,
    pub prefetched: u8,
    pub failed: u8,
    pub pending: u8,
    pub swap: u8,
}

impl BuiltFighters {
    pub fn unstarted(self) -> u8 {
        let accounted = u16::from(self.ready) + u16::from(self.prefetched)
            + u16::from(self.failed) + u16::from(self.pending);
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

impl CacheStatus {
    pub fn new(assets: Readiness, fighters: BuiltFighters, blocked: bool) -> Self {
        Self { assets, fighters, blocked }
    }

    /// Hidden: Off or nothing planned. Loading: fighter work is owed and runs; Paused: it is owed
    /// and a visible operation defers it. Waiting: no fighter work is owed, but a window is not
    /// built and only that role's swap builds it, so a blocked loader cannot pause it. Otherwise
    /// the plan is settled: Partial (a failure), Done (a prefetched group remains unbuilt), Ready
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
        else if fighters.failed > 0 { Progress::Partial }
        else if fighters.prefetched > 0 { Progress::Done }
        else { Progress::Ready }
    }

    pub fn text(self) -> String {
        let fighters = self.fighters;
        let (loaded, planned, failed) = (fighters.loaded(), fighters.planned, fighters.failed);
        let mut text = match self.progress() {
            Progress::Hidden => return String::new(),
            Progress::Loading => format!("Loading {}/{}", loaded, planned),
            Progress::Paused => format!("Paused {}/{}", loaded, planned),
            Progress::Waiting if loaded == 0 && failed == 0 => return match fighters.swap_who() {
                "" => "Swap once to load".to_string(),
                who => format!("Swap {} once to load", who),
            },
            Progress::Waiting => match fighters.swap_who() {
                "" => format!("Loaded {}/{} · swap to load {} more", loaded, planned, fighters.unstarted()),
                who => format!("Loaded {}/{} · swap {} to load {} more", loaded, planned, who, fighters.unstarted()),
            },
            Progress::Partial => format!("Loaded {}/{}", loaded, planned),
            Progress::Done => return format!("All loaded ({}/{}, {} instant)", loaded, planned, fighters.ready),
            Progress::Ready => return format!("Ready {}/{}", fighters.ready, planned),
        };
        if failed > 0 {
            text.push_str(&format!(" ({} failed)", failed));
        }
        text
    }

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
    fn shown_key(self) -> (u8, u8, u8, u8, u8) {
        let (class, swap) = match self.progress() {
            Progress::Hidden => (0, 0),
            Progress::Loading => (1, 0),
            Progress::Paused => (2, 0),
            // The Waiting text names who must swap, so a change of role is a change shown.
            Progress::Waiting => (3, self.fighters.swap),
            Progress::Partial | Progress::Done | Progress::Ready => (4, 0),
        };
        (class, self.fighters.loaded(), self.fighters.planned, self.fighters.failed, swap)
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
    use super::*;
    use crate::engine::preload::cycle_preload_capacity::reusable_extras;
    const NONE: fn(i32) -> bool = |_| false;
    fn ready_of(kinds: &[i32]) -> [Option<i32>; 8] {
        let mut ready = [None; 8];
        for (i, kind) in kinds.iter().enumerate() {
            ready[i] = Some(*kind);
        }
        ready
    }
    #[test]
    fn warm_assets_never_claim_fighters_are_ready() {
        let assets = readiness(5, 2, false, [Some(1), Some(2)], NONE, ready_of(&[1, 2]));
        let planned = BuiltFighters { planned: 3, ..BuiltFighters::default() };
        // No window yet and no load running: not "Ready 0/3", the first swap loads them.
        let status = CacheStatus::new(assets, planned, false);
        assert_eq!(status.progress(), Progress::Waiting);
        assert_eq!(status.text(), "Swap once to load");
        let building = CacheStatus::new(assets, BuiltFighters { pending: 3, ..planned }, false);
        assert_eq!(building.progress(), Progress::Loading);
        assert_eq!(building.text(), "Loading 0/3");
        assert!(building.announce(Some(status), 0));
        let complete = CacheStatus::new(assets, BuiltFighters { ready: 3, ..planned }, false);
        assert_eq!(complete.text(), "Ready 3/3");
        assert_eq!(complete.progress(), Progress::Ready);
        assert!(complete.frames() > building.frames() && complete.frames() > status.frames());
        assert!(complete.announce(Some(status), 0));
        assert!(!complete.announce(Some(complete), 0));
        let loading_assets = CacheStatus::new(Readiness { ready: 0, ..assets },
            complete.fighters, false);
        assert_eq!(loading_assets.progress(), Progress::Ready);
        assert!(!loading_assets.announce(Some(complete), 0));
    }

    #[test]
    fn failure_busy_and_off_do_not_manufacture_readiness() {
        let assets = readiness(5, 1, false, [Some(1)], NONE, ready_of(&[1]));
        let fighters = BuiltFighters { planned: 4, ready: 2, failed: 2, ..BuiltFighters::default() };
        let failed = CacheStatus::new(assets, fighters, false);
        assert_eq!(failed.progress(), Progress::Partial);
        assert_eq!(failed.text(), "Loaded 2/4 (2 failed)");
        let busy = CacheStatus::new(assets, BuiltFighters { planned: 2, ready: 2,
            ..BuiltFighters::default() }, true);
        assert_eq!(busy.text(), "Ready 2/2");
        let off = CacheStatus::new(Readiness { configured: 0, ..assets }, fighters, false);
        assert_eq!(off.progress(), Progress::Hidden);
        assert_eq!(off.text(), "");
        assert!(!off.announce(Some(failed), 3));
        let empty = CacheStatus::new(readiness(5, 5, false, [None::<i32>], NONE, [None; 8]),
            BuiltFighters::default(), false);
        assert_eq!(empty.progress(), Progress::Hidden);
        // Only an owed load can be paused, and only after the debounce.
        let paused = CacheStatus::new(assets, BuiltFighters { planned: 2, pending: 2,
            ..BuiltFighters::default() }, true);
        assert_eq!(paused.progress(), Progress::Paused);
        assert!(!paused.announce(None, 1));
        assert!(paused.announce(None, PAUSE_POLLS));
        // Nothing owed, only a swap: a blocked loader defers nothing, so it stays Waiting.
        let unstarted = CacheStatus::new(assets, BuiltFighters { planned: 2,
            ..BuiltFighters::default() }, true);
        assert_eq!(unstarted.progress(), Progress::Waiting);
    }

    fn fighters(planned: u8, ready: u8, prefetched: u8, failed: u8, pending: u8) -> BuiltFighters {
        BuiltFighters { planned, ready, prefetched, failed, pending, swap: 0 }
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
                failed: f.failed, pending: f.pending };
            assert_eq!(counts.unstarted(), f.unstarted());
            total = BuiltFighters { planned: total.planned + f.planned, ready: total.ready + f.ready,
                prefetched: total.prefetched + f.prefetched, failed: total.failed + f.failed,
                pending: total.pending + f.pending, swap: total.swap | counts.swap_owed(role) };
        }
        total
    }

    #[test]
    fn done_is_its_own_state_and_ready_keeps_meaning_built() {
        // 5 planned, 2 built, 3 prefetched (reachable without a reload), nothing running.
        let done = status(fighters(5, 2, 3, 0, 0));
        assert_eq!(done.progress(), Progress::Done);
        assert_eq!(done.text(), "All loaded (5/5, 2 instant)");
        assert_eq!(done.frames(), 180);
        // Ready is only the all-built case (C113 §6): a prefetched group is never counted built.
        let built = status(fighters(5, 5, 0, 0, 0));
        assert_eq!((built.progress(), built.text()), (Progress::Ready, "Ready 5/5".to_string()));
        let only_prefetched = status(fighters(3, 0, 3, 0, 0));
        assert_eq!(only_prefetched.progress(), Progress::Done);
        assert_eq!(only_prefetched.text(), "All loaded (3/3, 0 instant)");
        // Work still owed is not done, however much is loaded.
        let running = status(fighters(5, 2, 2, 0, 1));
        assert_eq!((running.progress(), running.text()), (Progress::Loading, "Loading 4/5".to_string()));
        assert_eq!(running.frames(), 90);
        // A failure settles the plan but is never Done or Ready.
        let partial = status(fighters(5, 2, 2, 1, 0));
        assert_eq!((partial.progress(), partial.text()), (Progress::Partial, "Loaded 4/5 (1 failed)".to_string()));
        assert_eq!(partial.frames(), 180);
        // The Done notice outlasts transient progress.
        assert!(done.frames() > running.frames());
    }

    #[test]
    fn before_the_first_swap_the_notice_says_to_swap_not_ready_zero() {
        let fresh = status(fighters(3, 0, 0, 0, 0));
        assert_eq!((fresh.progress(), fresh.text()), (Progress::Waiting, "Swap once to load".to_string()));
        assert_eq!(fresh.frames(), 120);
        // One role loaded, the other's window not built yet: honest, and not Done.
        let half = status(fighters(5, 3, 0, 0, 0));
        assert_eq!((half.progress(), half.text()),
            (Progress::Waiting, "Loaded 3/5 · swap to load 2 more".to_string()));
        let half_prefetched = status(fighters(5, 1, 2, 0, 0));
        assert_eq!(half_prefetched.text(), "Loaded 3/5 · swap to load 2 more");
        // Windows being built read as progress, not as a hint.
        let building = status(fighters(5, 0, 0, 0, 2));
        assert_eq!((building.progress(), building.text()), (Progress::Loading, "Loading 0/5".to_string()));
        // A busy native UI does not pause the hint: nothing is deferred, the swap is still the user's
        // (C116.2 console: "Paused 2/3" stood for a CPU swap that nothing would ever resume).
        let blocked = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])),
            fighters(3, 0, 0, 0, 0), true);
        assert_eq!((blocked.progress(), blocked.text()), (Progress::Waiting, "Swap once to load".to_string()));
        // It does pause a window that is still building.
        let deferred = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])),
            fighters(3, 0, 0, 0, 2), true);
        assert_eq!((deferred.progress(), deferred.text()), (Progress::Paused, "Paused 0/3".to_string()));
    }

    #[test]
    fn the_waiting_text_names_the_role_that_must_swap() {
        // C116.2 console: P1 Kirby's window built (2), the CPU's next window not built (1).
        let kirby = fighters(2, 2, 0, 0, 0);
        let cpu = fighters(1, 0, 0, 0, 0);
        let total = both_roles(kirby, cpu);
        assert_eq!(total.swap, crate::engine::swap::body_bank::SWAP_CPU);
        for blocked in [false, true] {
            let now = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])), total, blocked);
            assert_eq!((now.progress(), now.text()),
                (Progress::Waiting, "Loaded 2/3 · swap the CPU to load 1 more".to_string()));
        }
        let player = status(both_roles(fighters(2, 0, 0, 0, 0), fighters(1, 1, 0, 0, 0)));
        assert_eq!(player.text(), "Loaded 1/3 · swap P1 to load 2 more");
        let both = status(both_roles(fighters(2, 0, 0, 0, 0), fighters(1, 0, 0, 0, 0)));
        assert_eq!(both.text(), "Swap both once to load");
        let first = status(both_roles(fighters(2, 0, 0, 0, 0), BuiltFighters::default()));
        assert_eq!(first.text(), "Swap P1 once to load");
        let failed = status(both_roles(fighters(3, 1, 0, 1, 0), fighters(1, 0, 0, 0, 0)));
        assert_eq!(failed.text(), "Loaded 1/4 · swap both to load 2 more (1 failed)");
        // Another role now owes the swap: the text changes, so it is shown again.
        assert!(player.announce(Some(status(total)), 0));
        assert!(!player.announce(Some(player), 0));
        // A role owing a swap while the other's load is still owed reads as that load.
        let mixed = CacheStatus::new(readiness(5, 5, false, [Some(1)], NONE, ready_of(&[1])),
            both_roles(fighters(2, 0, 0, 0, 2), fighters(1, 0, 0, 0, 0)), true);
        assert_eq!((mixed.progress(), mixed.text()), (Progress::Paused, "Paused 0/3".to_string()));
    }

    #[test]
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

    #[test]
    fn a_role_with_nothing_to_load_adds_nothing() {
        // A CPU on its Mii with no bankable cycle plans nothing: only the Player's targets count,
        // so the status settles as soon as the Player's are.
        let player = fighters(3, 1, 2, 0, 0);
        let cpu = BuiltFighters::default();
        let total = both_roles(player, cpu);
        assert_eq!(total, player);
        let done = status(total);
        assert_eq!((done.progress(), done.text()), (Progress::Done, "All loaded (3/3, 1 instant)".to_string()));
        // The same CPU with a cycle that has not been swapped yet is waiting for that swap.
        let waiting = status(both_roles(player, fighters(2, 0, 0, 0, 0)));
        assert_eq!(waiting.progress(), Progress::Waiting);
        assert_eq!(waiting.text(), "Loaded 3/5 · swap the CPU to load 2 more");
        // Nothing planned anywhere stays hidden.
        assert_eq!(status(both_roles(cpu, cpu)).progress(), Progress::Hidden);
    }

    #[test]
    fn receipts_partition_the_plan() {
        assert_eq!(fighters(5, 1, 1, 1, 1).unstarted(), 1);
        assert_eq!(fighters(5, 5, 0, 0, 0).unstarted(), 0);
        // A receipt that lags the plan never underflows.
        assert_eq!(fighters(2, 2, 2, 2, 2).unstarted(), 0);
        assert_eq!(fighters(200, 255, 255, 0, 0).unstarted(), 0);
        assert_eq!(fighters(5, 2, 3, 0, 0).loaded(), 5);
        assert_eq!(fighters(0, 200, 100, 0, 0).loaded(), 255);
    }

    #[test]
    fn a_settled_notice_is_not_raised_again_when_prefetched_groups_get_built() {
        let done = status(fighters(5, 2, 3, 0, 0));
        // A swap builds a prefetched group: the count of instant ones moves, the notice does not.
        assert!(!status(fighters(5, 3, 2, 0, 0)).announce(Some(done), 0));
        assert!(!status(fighters(5, 5, 0, 0, 0)).announce(Some(done), 0));
        assert!(!done.announce(Some(done), 0));
        // A new plan size, a failure, or work owed again is a change worth showing.
        assert!(status(fighters(6, 2, 4, 0, 0)).announce(Some(done), 0));
        assert!(status(fighters(5, 2, 2, 1, 0)).announce(Some(done), 0));
        assert!(status(fighters(6, 2, 3, 0, 1)).announce(Some(done), 0));
        // Progress while loading announces every step; leaving Waiting announces too.
        let first = status(fighters(5, 0, 1, 0, 4));
        let second = status(fighters(5, 0, 2, 0, 3));
        assert!(second.announce(Some(first), 0));
        assert!(!first.announce(Some(first), 0));
        assert!(first.announce(Some(status(fighters(5, 0, 0, 0, 0))), 0));
        // The first and last states of one lap are each announced once.
        assert!(done.announce(Some(second), 0));
    }

    #[test]
    fn original_reload_and_active_body_are_not_parked_reuse_targets() {
        // C111 physical red: R.O.B. / Lucario / original Mr. Game & Watch.
        assert_eq!(reusable_extras(&[0x2d, 0x2c, 0x1c], 0x2d, Some(0x1c)), 1);
        assert_eq!(reusable_extras(&[0x2d, 0x2c, 0x1c], 0x2c, Some(0x1c)), 1);
        assert_eq!(reusable_extras(&[0x0b, 0x1f, 0x2e], 0x0b, Some(0x1c)), 2);
        // Differently configured numbered Solo Popo is a separate reusable target.
        assert_eq!(reusable_extras(&[0x4b, 0, 1], 0, None), 2);
        let assets = readiness(5, 5, false, [Some(1), Some(2)], NONE, ready_of(&[1, 2]));
        let only_assets = CacheStatus::new(assets, BuiltFighters::default(), false);
        assert_eq!(only_assets.text(), "");
        assert_eq!(only_assets.progress(), Progress::Hidden);
    }

    #[test]
    fn denominator_is_the_actual_plan_not_the_configured_five() {
        // Five configured, only two distinct targets exist: Ready 2/2, never 2/5.
        let plan = [Some(1), Some(2), None, None, None];
        let r = readiness(5, 5, false, plan, NONE, ready_of(&[1, 2]));
        assert_eq!((r.progress, r.ready, r.planned, r.configured), (Progress::Ready, 2, 2, 5));
        assert_eq!(r.text(), "Ready 2/2");
        let r = readiness(5, 5, false, plan, NONE, ready_of(&[2]));
        assert_eq!((r.progress, r.ready, r.planned), (Progress::Loading, 1, 2));
        assert_eq!(r.text(), "Loading 1/2");
    }
    #[test]
    fn full_five_distinguishes_ready_two_from_ready_five() {
        let plan = [Some(1), Some(2), Some(3), Some(4), Some(5)];
        let two = readiness(5, 5, false, plan, NONE, ready_of(&[1, 2]));
        assert_eq!((two.progress, two.text()), (Progress::Loading, "Loading 2/5".to_string()));
        let five = readiness(5, 5, false, plan, NONE, ready_of(&[1, 2, 3, 4, 5]));
        assert_eq!((five.progress, five.text()), (Progress::Ready, "Ready 5/5".to_string()));
    }
    #[test]
    fn backend_capacity_truncates_and_is_named() {
        // Hardware holds two of the five configured: the plan beyond it is not owed.
        let plan = [Some(1), Some(2), Some(3), Some(4), Some(5)];
        let r = readiness(5, 2, false, plan, NONE, ready_of(&[1]));
        assert_eq!((r.planned, r.ready, r.limited()), (2, 1, true));
        assert_eq!(r.text(), "Loading 1/2 (max 2)");
        let r = readiness(5, 2, false, plan, NONE, ready_of(&[1, 2, 3]));
        assert_eq!((r.progress, r.text()), (Progress::Ready, "Ready 2/2 (max 2)".to_string()));
        // A short plan under spare capacity is not capacity-limited.
        assert!(!readiness(5, 4, false, [Some(1), Some(2), None, None, None], NONE, [None; 8]).limited());
        // Configuration at or under capacity is not limited either.
        assert!(!readiness(2, 2, false, [Some(1), Some(2)], NONE, [None; 8]).limited());
    }
    #[test]
    fn stale_or_unplanned_residents_do_not_count() {
        let plan = [Some(1), Some(2)];
        let r = readiness(2, 2, false, plan, NONE, ready_of(&[7, 8, 9]));
        assert_eq!((r.progress, r.ready, r.planned), (Progress::Loading, 0, 2));
        let r = readiness(2, 2, false, plan, NONE, ready_of(&[7, 2]));
        assert_eq!((r.progress, r.ready), (Progress::Loading, 1));
    }
    #[test]
    fn one_failed_target_stays_in_the_denominator_and_is_never_ready() {
        let plan = [Some(1), Some(2), Some(3)];
        // Target 2 failed even though a source for it is resident: not counted,
        // not dropped from the denominator, and never reported Ready.
        let r = readiness(3, 3, false, plan, |k| k == 2, ready_of(&[1, 2, 3]));
        assert_eq!((r.progress, r.ready, r.planned, r.failed), (Progress::Partial, 2, 3, 1));
        assert_ne!(r.progress, Progress::Ready);
        assert_eq!(r.text(), "Partial 2/3 (1 failed)");
        // Others still loading: keep Loading, with the failure visible.
        let r = readiness(3, 3, false, plan, |k| k == 2, ready_of(&[1]));
        assert_eq!((r.progress, r.ready, r.planned), (Progress::Loading, 1, 3));
        assert_eq!(r.text(), "Loading 1/3 (1 failed)");
        // A pause does not hide a settled failure.
        let r = readiness(3, 3, true, plan, |k| k == 2, ready_of(&[1, 3]));
        assert_eq!(r.progress, Progress::Partial);
    }
    #[test]
    fn all_failed_is_not_silent() {
        let plan = [Some(1), Some(2), Some(3)];
        let r = readiness(3, 3, false, plan, |_| true, ready_of(&[1, 2, 3]));
        assert_eq!((r.progress, r.ready, r.planned, r.failed), (Progress::Partial, 0, 3, 3));
        assert_eq!(r.text(), "Partial 0/3 (3 failed)");
        assert!(announce(None, r, 0));
        assert!(!announce(Some(r), r, 0));
        // A genuinely empty plan or Off is still hidden.
        assert_eq!(readiness(3, 3, false, [None::<i32>; 3], |_| true, [None; 8]).progress, Progress::Hidden);
        assert_eq!(readiness(0, 3, false, plan, |_| true, [None; 8]).progress, Progress::Hidden);
    }
    #[test]
    fn off_unavailable_and_empty_are_hidden() {
        let plan = [Some(1), Some(2)];
        assert_eq!(readiness(0, 5, false, plan, NONE, ready_of(&[1, 2])).progress, Progress::Hidden);
        // No qualified backend (actual match): capacity 0 even though configured.
        assert_eq!(readiness(5, 0, false, plan, NONE, ready_of(&[1, 2])).progress, Progress::Hidden);
        assert_eq!(readiness(5, 5, false, [None, None], NONE, [None; 8]).progress, Progress::Hidden);
        assert_eq!(Readiness { progress: Progress::Hidden, ..readiness(0, 0, false, plan, NONE, [None; 8]) }.text(), "");
    }
    #[test]
    fn blocked_pauses_incomplete_loads_but_never_unreadies() {
        let plan = [Some(1), Some(2), Some(3)];
        let r = readiness(3, 3, true, plan, NONE, ready_of(&[1]));
        assert_eq!((r.progress, r.text()), (Progress::Paused, "Paused 1/3".to_string()));
        let r = readiness(3, 3, true, plan, NONE, ready_of(&[1, 2, 3]));
        assert_eq!(r.progress, Progress::Ready);
    }
    #[test]
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
    #[test]
    fn next_hat_displaces_lower_priority_fighter_when_an_equipped_hat_owns_other_helper() {
        // Mario is leased by the current hat. Fox is a wanted fighter, but the
        // next Game & Watch hat has priority. The old policy pinned Fox and
        // forced an on-demand hat load even with the two-target budget.
        let (mario, game_watch, fox) = (0, 0x1c, 7);
        let targets = [Some(game_watch), Some(fox)];
        let mut sources = [None; 8];
        sources[2] = Some(fox);
        sources[3] = Some(mario);
        assert_eq!(missing(targets, sources, 3, false, false), None);
        assert_eq!(missing(targets, sources, 2, true, true), None);
        assert_eq!(missing(targets, sources, 2, true, false), Some(game_watch));
        sources[2] = Some(game_watch);
        // Never evict the higher-priority hat for Fox: no speculative churn.
        assert_eq!(missing(targets, sources, 2, true, false), None);
        assert_eq!(missing(targets, sources, 3, false, false), None);
        // Once Mario's actual native lease ends, both targets can be ready.
        assert_eq!(missing(targets, sources, 3, true, false), Some(fox));
        sources[3] = Some(fox);
        for slot in [2, 3] { assert_eq!(missing(targets, sources, slot, true, false), None); }
    }
    #[test]
    fn five_targets_fill_without_eviction_and_demand_still_wins() {
        let lane=[Some(10),Some(11),Some(12),Some(13),Some(14)];
        let lanes=[lane,[None;5],[None;5],[None;5]];
        for count in 0..=5 {
            let targets=merge(lanes,count);
            assert_eq!(targets.iter().flatten().count(),usize::from(count));
            let mut sources=[None;8];
            for slot in [2,3,7,6,5] {
                if let Some(target)=missing(targets,sources,slot,true,false) {sources[slot]=Some(target);}
            }
            assert_eq!(sources.iter().flatten().count(),usize::from(count));
            for slot in [2,3,7,6,5] {assert_eq!(missing(targets,sources,slot,true,false),None);}
        }
        assert_eq!(merge(lanes,255),lane);
        assert_eq!(missing(lane,[None;8],7,true,true),None);
        assert_eq!(missing(lane,[None;8],7,false,false),None);
    }
    #[test]
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
    #[test]
    fn zero_one_two_and_cross_lane_deduplication() {
        let lanes = [
            [Some(1), Some(2)],
            [Some(1), Some(3)],
            [None, None],
            [None, None],
        ];
        assert_eq!(merge(lanes, 0), [None, None]);
        assert_eq!(merge(lanes, 1), [Some(1), None]);
        assert_eq!(merge(lanes, 2), [Some(1), Some(2)]);
        assert_eq!(merge(lanes, 255), [Some(1), Some(2)]);
    }
    #[test]
    fn two_ready_sources_do_not_evict_each_other() {
        let mut sources = [None; 8];
        sources[2] = Some(10);
        sources[3] = Some(11);
        assert_eq!(missing([Some(10), Some(11)], sources, 2, true, false), None);
        assert_eq!(missing([Some(10), Some(11)], sources, 3, true, false), None);
        assert_eq!(
            missing([Some(11), Some(12)], sources, 2, true, false),
            Some(12)
        );
        assert_eq!(missing([Some(11), Some(12)], sources, 3, true, false), None);
    }
    #[test]
    fn demands_leases_and_inflight_sources_win() {
        let mut sources = [None; 8];
        sources[5] = Some(10);
        assert_eq!(
            missing([Some(10), Some(11)], sources, 6, true, false),
            Some(11)
        );
        assert_eq!(
            missing([Some(10), Some(11)], sources, 6, false, false),
            None
        );
        assert_eq!(missing([Some(10), Some(11)], sources, 6, true, true), None);
        assert_eq!(missing([Some(10), None], sources, 6, true, false), None);
        assert_eq!(missing([None, None], sources, 6, true, false), None);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Target {
        kind: u32,
        config: u16,
        epoch: u8,
    }
    fn plain(kind: u32) -> Target { Target { kind, config: 0, epoch: 1 } }
    fn exact(kind: u32, config: u16) -> Target { Target { kind, config, epoch: 1 } }
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
    /// `legacy` is the old shared walk: native slots see the global plan, extras
    /// see ordinary targets only. Scarce scheduling is the slot-local partition.
    fn sweep(plan: [Option<Target>; 5], sources: &mut [Option<Target>; 8], order: &[usize],
        native: &[usize], legacy: bool, demand: bool, locked: &[usize]) -> usize {
        let mut moves = 0;
        for &slot in order {
            let replaceable = !locked.contains(&slot);
            let view = if legacy && native.contains(&slot) {
                plan
            } else {
                schedulable(plan, native.contains(&slot), constrained)
            };
            let Some(next) = missing(view, *sources, slot, replaceable, demand) else { continue; };
            if sources[slot] == Some(next) { continue; }
            sources[slot] = Some(next);
            moves += 1;
        }
        moves
    }
    fn settle(plan: [Option<Target>; 5], sources: &mut [Option<Target>; 8], order: &[usize],
        native: &[usize], locked: &[usize]) -> usize {
        let mut sweeps = 0;
        loop {
            sweeps += 1;
            assert!(sweeps <= 4, "placement did not converge");
            if sweep(plan, sources, order, native, false, false, locked) == 0 { return sweeps; }
        }
    }
    fn residents(sources: impl IntoIterator<Item = Option<Target>>) -> Vec<Target> {
        let mut found: Vec<_> = sources.into_iter().flatten().collect();
        found.sort_by_key(|target| (target.kind, target.config, target.epoch));
        found
    }
    #[test]
    fn four_ready_ordinary_plan_then_converges_to_five_without_evicting_in_a_loop() {
        // The reported Training stall: bases 2/3 and extras 7/6 hold the four
        // earlier ordinary targets. Gunner is missing, extra 5 does not exist,
        // and the shared walk will not evict Falcon or Mario for it.
        let plan = kirby_plan();
        let order = [2usize, 3, 7, 6, 5];
        let native = [2usize, 3];
        let mut stalled = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, GAME_WATCH)]);
        assert_eq!(sweep(plan, &mut stalled, &order, &native, true, false, &[]), 0);
        assert_eq!(residents(stalled).len(), 4);
        assert!(!residents(stalled).contains(&GUNNER));
        let mut sources = stalled;
        let sweeps = settle(plan, &mut sources, &order, &native, &[]);
        assert!(sweeps > 1);
        assert_eq!(sweep(plan, &mut sources, &order, &native, false, false, &[]), 0);
        assert_eq!(residents(sources), residents(plan_of([FALCON, MARIO, SEPHIROTH, GAME_WATCH, GUNNER])));
        assert!(native.contains(&sources.iter().position(|source| *source == Some(GUNNER)).unwrap()));
        for slot in [7usize, 6, 5] { assert!(!sources[slot].is_some_and(constrained)); }
        // The indicator still reads the original ordinary-first plan.
        assert_eq!(plan[0], Some(FALCON));
        assert_eq!(plan[4], Some(GUNNER));
        let ready = readiness(5, 5, false, plan, |_| false, sources);
        assert_eq!((ready.progress, ready.ready, ready.planned, ready.failed), (Progress::Ready, 5, 5, 0));
    }
    #[test]
    fn ordinary_only_slot_view_matches_the_old_global_walk() {
        let plan = plan_of([plain(10), plain(11), plain(12), plain(13), plain(14)]);
        assert_eq!(schedulable(plan, true, constrained), plan);
        assert_eq!(schedulable(plan, false, constrained), plan);
        let mut sources = [None; 8];
        assert_eq!(settle(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], &[]), 2);
        assert_eq!(sources.iter().flatten().count(), 5);
        assert_eq!(sweep(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], false, false, &[]), 0);
        // A hat that is not an exact Mii still outranks a later fighter.
        let hats = [Some(plain(0x1c)), Some(plain(7))];
        assert_eq!(schedulable(hats, true, constrained), hats);
        let mut held = [None; 8];
        held[2] = Some(plain(7));
        assert_eq!(missing(schedulable(hats, true, constrained), held, 2, true, false), Some(plain(0x1c)));
    }
    #[test]
    fn demand_and_leases_protect_residents_while_the_other_native_slot_takes_the_mii() {
        let plan = kirby_plan();
        let order = [2usize, 3, 7, 6, 5];
        let native = [2usize, 3];
        let mut sources = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, GAME_WATCH)]);
        assert_eq!(sweep(plan, &mut sources, &order, &native, false, true, &[]), 0);
        assert_eq!(sources[2], Some(FALCON));
        // Slot 2 stays leased to Falcon. Slot 3 is free and is the other native helper.
        let sweeps = settle(plan, &mut sources, &order, &native, &[2]);
        assert!(sweeps > 1);
        assert_eq!(sources[2], Some(FALCON));
        assert_eq!(sources[3], Some(GUNNER));
        assert_eq!(sweep(plan, &mut sources, &order, &native, false, false, &[2]), 0);
        assert_eq!(residents(sources).len(), 5);
        // Both native helpers leased: the Mii stays missing and extras do not take it.
        let mut both = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, GAME_WATCH)]);
        assert_eq!(settle(plan, &mut both, &order, &native, &[2, 3]), 1);
        assert!(!residents(both).contains(&GUNNER));
        for slot in [7usize, 6, 5] { assert!(!both[slot].is_some_and(constrained)); }
    }
    #[test]
    fn two_distinct_mii_configurations_occupy_the_native_pair() {
        let sword = exact(0x49, 7);
        let gunner = exact(0x4a, 3);
        assert_ne!(sword, gunner);
        let plan = plan_of([FALCON, gunner, MARIO, sword, SEPHIROTH]);
        let mut sources = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, plain(0x58))]);
        // A same-kind source with the other configuration does not satisfy Gunner.
        sources[2] = Some(exact(0x4a, 9));
        let _ = settle(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], &[]);
        assert_eq!(sweep(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], false, false, &[]), 0);
        let native: Vec<Target> = [2usize, 3].iter().copied().filter_map(|slot| sources[slot]).collect();
        assert_eq!(residents([Some(native[0]), Some(native[1]), None, None, None, None, None, None]),
            residents([Some(gunner), Some(sword), None, None, None, None, None, None]));
        for slot in [7usize, 6, 5] { assert!(!sources[slot].is_some_and(constrained)); }
        let stale = Target { epoch: 0, ..gunner };
        let mut aged = [None; 8];
        aged[2] = Some(stale);
        let view = schedulable([Some(gunner), Some(FALCON), None, None, None], true, constrained);
        assert_eq!(missing(view, aged, 2, true, false), Some(gunner));
    }
    #[test]
    fn training_and_waiting_place_the_mii_on_their_own_native_pair() {
        let plan = kirby_plan();
        let mut training = occupy(&[(2, FALCON), (3, MARIO), (7, SEPHIROTH), (6, GAME_WATCH)]);
        settle(plan, &mut training, &[2, 3, 7, 6, 5], &[2, 3], &[]);
        assert!(training[2] == Some(GUNNER) || training[3] == Some(GUNNER));
        for slot in [7usize, 6, 5] { assert_ne!(training[slot], Some(GUNNER)); }
        // Waiting helpers are 5/6. Created extras are 7, then 3, then 2.
        let mut waiting = occupy(&[(5, FALCON), (6, MARIO), (7, SEPHIROTH), (3, GAME_WATCH)]);
        settle(plan, &mut waiting, &[5, 6, 7, 3, 2], &[5, 6], &[]);
        assert_eq!(sweep(plan, &mut waiting, &[5, 6, 7, 3, 2], &[5, 6], false, false, &[]), 0);
        assert!(waiting[5] == Some(GUNNER) || waiting[6] == Some(GUNNER));
        for slot in [7usize, 3, 2] { assert_ne!(waiting[slot], Some(GUNNER)); }
        assert_eq!(residents(training), residents(waiting));
    }
    #[test]
    fn a_third_exact_mii_remains_unscheduled_without_a_retry() {
        let first = exact(0x48, 1);
        let second = exact(0x49, 2);
        let third = exact(0x4a, 3);
        let plan = plan_of([first, MARIO, second, FALCON, third]);
        let mut sources = [None; 8];
        let sweeps = settle(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], &[]);
        assert!(sweeps > 1);
        assert_eq!(sweep(plan, &mut sources, &[2, 3, 7, 6, 5], &[2, 3], false, false, &[]), 0);
        assert_eq!(sources[2], Some(first));
        assert_eq!(sources[3], Some(second));
        assert!(!sources.contains(&Some(third)));
        for slot in [7usize, 6, 5] { assert!(!sources[slot].is_some_and(constrained)); }
        let ready = readiness(5, 5, false, plan, |_| false, sources);
        assert_eq!((ready.progress, ready.ready, ready.planned, ready.failed), (Progress::Loading, 4, 5, 0));
        assert_eq!(plan, plan_of([first, MARIO, second, FALCON, third]));
    }
    #[test]
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
        assert_eq!(ready.text(), "Partial 4/5 (1 failed)");
        assert_eq!(plan[4], Some(GUNNER));
    }
}
